use std::{cell::RefCell, collections::BTreeMap, mem::swap, rc::Rc};

use crate::{
    abf::{ABFProgram, ABFProgramBuilder},
    allocator::BrainCrabAllocator,
    ast::{Expression, Instruction, LValueExpression, Program},
    compiler_error::{CompileResult, CompilerError},
    constant_value::ConstantValue,
    types::Type,
    value::Value,
};

pub type AddressPool = Rc<RefCell<BrainCrabAllocator>>;

pub struct ScopedVariableMap<'a> {
    pub variable_map_stack: Vec<BTreeMap<&'a str, Value>>,
}

impl<'a> Default for ScopedVariableMap<'a> {
    fn default() -> Self {
        Self {
            variable_map_stack: vec![BTreeMap::new()],
        }
    }
}

impl<'a> ScopedVariableMap<'a> {
    pub fn borrow_variable(&self, name: &'a str) -> Option<Value> {
        for variable_map in self.variable_map_stack.iter().rev() {
            if let Some(result) = variable_map.get(name) {
                return Some(result.borrow());
            }
        }
        None
    }

    pub fn defined_in_current_scope(&mut self, name: &'a str) -> bool {
        self.variable_map_stack.last().unwrap().contains_key(name)
    }

    pub fn register(&mut self, name: &'a str, value: Value) {
        self.variable_map_stack
            .last_mut()
            .unwrap()
            .insert(name, value);
    }

    pub fn start_scope(&mut self) {
        self.variable_map_stack.push(BTreeMap::new());
    }

    pub fn end_scope(&mut self) {
        self.variable_map_stack.pop().unwrap();
    }
}

enum Accessor {
    Index(Value),
}

struct AccessedValue {
    source: Value,
    accessors: Vec<Accessor>,
}

impl AccessedValue {
    fn new(source: impl Into<Value>, accessors: Vec<Accessor>) -> Self {
        Self {
            source: source.into(),
            accessors,
        }
    }
    fn unit(source: impl Into<Value>) -> Self {
        Self::new(source, Vec::new())
    }
    fn value_type<'a>(&self) -> CompileResult<'a, Type> {
        fn value_type_impl<'a>(
            source_type: &Type,
            accessors: &[Accessor],
        ) -> CompileResult<'a, Type> {
            match accessors {
                [] => Ok(source_type.clone()),
                [Accessor::Index(_), tail @ ..] => {
                    if let Type::Array { element_type, .. } = source_type {
                        value_type_impl(element_type, tail)
                    } else {
                        Err(CompilerError::NotAnArray(source_type.clone()))
                    }
                }
            }
        }
        value_type_impl(&self.source.value_type, &self.accessors)
    }
}

pub struct BrainCrabCompiler<'a> {
    pub variable_map: ScopedVariableMap<'a>,
    pub old_address_pool: AddressPool,
    pub builder: ABFProgramBuilder,
}

impl<'a> Default for BrainCrabCompiler<'a> {
    fn default() -> Self {
        Self {
            variable_map: Default::default(),
            old_address_pool: Rc::new(RefCell::new(BrainCrabAllocator::new())),
            builder: ABFProgramBuilder::new(),
        }
    }
}

impl<'a> BrainCrabCompiler<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_result(self) -> CompileResult<'a, ABFProgram> {
        Ok(self.builder.build())
    }

    // Memory management

    pub fn allocate(&mut self, value_type: Type) -> Value {
        let addresses: Vec<_> = (0..value_type.size())
            .map(|_| self.builder.new_address(0))
            .collect();
        Value::new(addresses, value_type, true)
    }

    pub fn register_variable(&mut self, name: &'a str, value: Value) -> CompileResult<'a, Value> {
        if self.variable_map.defined_in_current_scope(name) {
            Err(CompilerError::AlreadyDefinedVariable(name))
        } else {
            let borrowed = value.borrow();
            self.variable_map.register(name, value);
            Ok(borrowed)
        }
    }

    pub fn new_variable(
        &mut self,
        name: &'a str,
        value: Value,
        mutable: bool,
    ) -> CompileResult<'a, Value> {
        if mutable {
            let mut owned = self.new_owned(value)?;
            owned.mutable = mutable;
            let borrow = owned.borrow();
            self.register_variable(name, owned)?;
            Ok(borrow)
        } else {
            self.register_variable(name, value)
        }
    }

    pub fn borrow_immutable(&self, name: &'a str) -> CompileResult<'a, Value> {
        if let Some(variable) = self.variable_map.borrow_variable(name) {
            Ok(variable)
        } else {
            Err(CompilerError::UndefinedVariable(name))
        }
    }

    pub fn borrow_mutable(&self, name: &'a str) -> CompileResult<'a, Value> {
        let result = self.borrow_immutable(name)?;

        if result.mutable {
            Ok(result)
        } else {
            Err(CompilerError::MutableBorrowOfImmutableVariable(result))
        }
    }

    pub fn new_owned(&mut self, value: impl Into<Value>) -> CompileResult<'a, Value> {
        let value: Value = value.into();
        if value.is_owned() {
            Ok(value)
        } else {
            let owned = self.allocate(value.value_type.clone());
            self.copy_and_add_values(value, &[owned.borrow()])?;
            Ok(owned)
        }
    }

    pub fn value_from_const(&mut self, value: impl Into<ConstantValue>) -> Value {
        let value: ConstantValue = value.into();
        let value_type = value.value_type().unwrap();
        let data = value.data();
        let addresses: Vec<_> = data.iter().map(|x| self.builder.new_address(*x)).collect();
        Value::new(addresses, value_type, true)
    }

    pub fn reinterpret_cast(&self, mut value: Value, new_type: Type) -> CompileResult<'a, Value> {
        if value.value_type.size() != new_type.size() {
            Err(CompilerError::InvalidReinterpretCast {
                original: value.value_type.clone(),
                new: new_type,
            })
        } else {
            value.value_type = new_type;
            Ok(value)
        }
    }

    // Primitives

    pub fn add_to(&mut self, address: u16, value: i8) {
        self.builder.add(address, value);
    }

    pub fn write(&mut self, address: u16) {
        self.builder.write(address);
    }

    pub fn read(&mut self) -> Value {
        let address = self.builder.read();
        Value::new(vec![address], Type::U8, true)
    }

    pub fn scoped(
        &mut self,
        f: impl FnOnce(&mut Self) -> CompileResult<'a, ()>,
    ) -> CompileResult<'a, ()> {
        self.variable_map.start_scope();
        f(self)?;
        self.variable_map.end_scope();
        Ok(())
    }

    pub fn loop_while(
        &mut self,
        predicate: u16,
        f: impl FnOnce(&mut Self) -> CompileResult<'a, ()>,
    ) -> CompileResult<'a, ()> {
        self.scoped(|compiler| {
            let mut body_builder = compiler.builder.start_loop();
            swap(&mut body_builder, &mut compiler.builder);
            f(compiler)?;
            swap(&mut body_builder, &mut compiler.builder);
            compiler.builder.end_loop(predicate, body_builder);
            Ok(())
        })
    }

    // Utilities
    pub fn write_value(&mut self, value: Value) {
        for address in value.addresses {
            self.write(address);
        }
    }

    pub fn zero(&mut self, value: Value) {
        for address in value.addresses {
            self.builder.zero(address);
        }
    }

    pub fn move_and_add_values(
        &mut self,
        source: Value,
        destinations: &[Value],
    ) -> CompileResult<'a, ()> {
        for destination in destinations {
            assert!(destination.size() == source.size());
        }
        for (i, source_address) in source.addresses.into_iter().enumerate() {
            self.loop_while(source_address, |compiler| {
                compiler.add_to(source_address, -1);
                for destination in destinations {
                    let destination_address = destination.addresses[i];
                    assert!(destination_address != source_address);
                    compiler.add_to(destination.addresses[i], 1);
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    pub fn copy_and_add_values(
        &mut self,
        source: Value,
        destinations: &[Value],
    ) -> CompileResult<'a, ()> {
        // If source is owned then we are throwing it away anyway, so we can move instead
        if source.is_owned() {
            self.move_and_add_values(source, destinations)
        } else {
            let temp = self.allocate(source.value_type.clone());
            let mut new_destinations = vec![temp.borrow()];
            for destination in destinations {
                new_destinations.push(destination.borrow());
            }
            self.move_and_add_values(source.borrow(), &new_destinations)?;
            self.move_and_add_values(temp, &[source])
        }
    }

    pub fn if_then<I: FnOnce(&mut Self) -> CompileResult<'a, ()>>(
        &mut self,
        predicate: Value,
        body: I,
    ) -> CompileResult<'a, ()> {
        let if_check = self.new_owned(predicate)?;
        self.loop_while(if_check.address(), |compiler| {
            body(compiler)?;
            compiler.zero(if_check);
            Ok(())
        })
    }

    pub fn if_then_else<
        I: FnOnce(&mut Self) -> CompileResult<'a, ()>,
        E: FnOnce(&mut Self) -> CompileResult<'a, ()>,
    >(
        &mut self,
        predicate: Value,
        if_case: I,
        else_case: E,
    ) -> CompileResult<'a, ()> {
        let else_check = self.value_from_const(1);
        let if_check = self.new_owned(predicate)?;
        self.loop_while(if_check.address(), |compiler| {
            if_case(compiler)?;
            compiler.add_to(else_check.address(), -1);
            compiler.zero(if_check);
            Ok(())
        })?;
        self.loop_while(else_check.address(), |compiler| {
            else_case(compiler)?;
            compiler.add_to(else_check.address(), -1);
            Ok(())
        })
    }

    pub fn n_times(
        &mut self,
        n: Value,
        f: impl Fn(&mut Self) -> CompileResult<'a, ()>,
    ) -> CompileResult<'a, ()> {
        assert!(
            n.value_type.size() == 1,
            "n times with a constant value which is not a bool or u8. Instead got {n:?}"
        );
        if n.is_owned() {
            self.loop_while(n.address(), |compiler| {
                compiler.add_to(n.address(), -1);
                f(compiler)?;
                Ok(())
            })?;
        } else {
            let address = n.address();
            let temp = self.value_from_const(0);
            self.loop_while(address, |compiler| {
                compiler.add_to(address, -1);
                compiler.add_to(temp.address(), 1);
                f(compiler)?;
                Ok(())
            })?;
            self.loop_while(temp.address(), |compiler| {
                compiler.add_to(temp.address(), -1);
                compiler.add_to(address, 1);
                Ok(())
            })?;
        }
        Ok(())
    }

    pub fn add_assign(&mut self, destination: Value, value: Value) -> CompileResult<'a, ()> {
        if destination.address() == value.address() {
            let temp = self.value_from_const(0);
            self.copy_and_add_values(destination.borrow(), &[temp.borrow()])?;
            self.move_and_add_values(temp, &[destination])
        } else {
            self.copy_and_add_values(value, &[destination])
        }
    }

    pub fn sub_assign(&mut self, destination: Value, value: Value) -> CompileResult<'a, ()> {
        if destination.address() == value.address() {
            self.zero(destination);
            Ok(())
        } else {
            self.n_times(value, |compiler| {
                compiler.add_to(destination.address(), -1);
                Ok(())
            })
        }
    }

    pub fn mul_assign(&mut self, destination: Value, value: Value) -> CompileResult<'a, ()> {
        let result = self.value_from_const(0);
        self.n_times(value, |compiler| {
            compiler.add_assign(result.borrow(), destination.borrow())
        })?;
        self.assign(destination, result.borrow())
    }

    pub fn div_assign(&mut self, destination: Value, value: Value) -> CompileResult<'a, ()> {
        if destination.address() == value.address() {
            self.zero(destination.borrow());
            self.add_to(destination.address(), 1);
            Ok(())
        } else {
            let result = self.value_from_const(0);

            self.loop_while(destination.address(), |compiler| {
                let predicate =
                    compiler.eval_less_than_equals(value.borrow(), destination.borrow())?;
                compiler.if_then_else(
                    predicate,
                    |compiler| {
                        compiler.sub_assign(destination.borrow(), value.borrow())?;
                        compiler.add_to(result.address(), 1);
                        Ok(())
                    },
                    |compiler| {
                        compiler.zero(destination.borrow());
                        Ok(())
                    },
                )
            })?;
            self.move_and_add_values(result, &[destination])
        }
    }

    pub fn mod_assign(&mut self, destination: Value, value: Value) -> CompileResult<'a, ()> {
        if destination.address() == value.address() {
            self.zero(destination);
            Ok(())
        } else {
            let predicate = self.eval_greater_than_equals(destination.borrow(), value.borrow())?;
            self.loop_while(predicate.address(), |compiler| {
                compiler.sub_assign(destination.borrow(), value.borrow())?;
                let new_predicate =
                    compiler.eval_greater_than_equals(destination.borrow(), value.borrow())?;
                compiler.assign(predicate, new_predicate)
            })
        }
    }

    pub fn not_assign(&mut self, value: Value) -> CompileResult<'a, ()> {
        self.if_then_else(
            value.borrow(),
            |compiler| {
                compiler.zero(value.borrow());
                Ok(())
            },
            |compiler| {
                compiler.add_to(value.address(), 1);
                Ok(())
            },
        )
    }
    pub fn and_assign(&mut self, destination: Value, value: Value) -> CompileResult<'a, ()> {
        self.if_then_else(
            value,
            |_| Ok(()),
            |compiler| {
                compiler.zero(destination);
                Ok(())
            },
        )
    }
    pub fn or_assign(&mut self, destination: Value, value: Value) -> CompileResult<'a, ()> {
        self.if_then_else(
            destination.borrow(),
            |_| Ok(()),
            |compiler| compiler.assign(destination, value),
        )
    }

    pub fn assign(&mut self, destination: Value, value: Value) -> CompileResult<'a, ()> {
        assert!(destination.size() == value.size());
        if destination.addresses != value.addresses {
            self.zero(destination.borrow());
            self.copy_and_add_values(value, &[destination])?;
        }
        Ok(())
    }

    pub fn print_string(&mut self, string: String) -> CompileResult<'a, ()> {
        if string.is_ascii() {
            for char in string.chars() {
                let new_value = self.value_from_const(char as u8);
                self.write_value(new_value);
            }

            Ok(())
        } else {
            Err(CompilerError::NonAsciiString(string.into()))
        }
    }

    // Expressions

    fn eval_add(&mut self, a: Value, b: Value) -> CompileResult<'a, Value> {
        a.type_check(&Type::U8)?;
        b.type_check(&Type::U8)?;
        if a.is_owned() {
            self.add_assign(a.borrow(), b)?;
            Ok(a)
        } else {
            let result = self.new_owned(b)?;
            self.add_assign(result.borrow(), a)?;
            Ok(result)
        }
    }

    fn eval_mul(&mut self, a: Value, b: Value) -> CompileResult<'a, Value> {
        a.type_check(&Type::U8)?;
        b.type_check(&Type::U8)?;
        if b.is_owned() {
            self.mul_assign(b.borrow(), a)?;
            Ok(b)
        } else {
            let result = self.new_owned(a)?;
            self.mul_assign(result.borrow(), b)?;

            Ok(result)
        }
    }

    fn eval_sub(&mut self, a: Value, b: Value) -> CompileResult<'a, Value> {
        a.type_check(&Type::U8)?;
        b.type_check(&Type::U8)?;
        let result = self.new_owned(a)?;
        self.sub_assign(result.borrow(), b)?;

        Ok(result)
    }

    fn eval_div(&mut self, a: Value, b: Value) -> CompileResult<'a, Value> {
        a.type_check(&Type::U8)?;
        b.type_check(&Type::U8)?;
        let result = self.new_owned(a)?;
        self.div_assign(result.borrow(), b)?;

        Ok(result)
    }

    fn eval_mod(&mut self, a: Value, b: Value) -> CompileResult<'a, Value> {
        a.type_check(&Type::U8)?;
        b.type_check(&Type::U8)?;
        let result = self.new_owned(a)?;
        self.mod_assign(result.borrow(), b)?;

        Ok(result)
    }

    fn eval_not(&mut self, value: Value) -> CompileResult<'a, Value> {
        value.type_check(&Type::Bool)?;
        let result = self.new_owned(value)?;
        self.not_assign(result.borrow())?;
        Ok(result)
    }

    fn eval_and(&mut self, a: Value, b: Value) -> CompileResult<'a, Value> {
        a.type_check(&Type::Bool)?;
        b.type_check(&Type::Bool)?;
        if b.is_owned() {
            self.and_assign(b.borrow(), a)?;
            Ok(b)
        } else {
            let result = self.new_owned(a)?;
            self.and_assign(result.borrow(), b)?;

            Ok(result)
        }
    }

    fn eval_or(&mut self, a: Value, b: Value) -> CompileResult<'a, Value> {
        a.type_check(&Type::Bool)?;
        b.type_check(&Type::Bool)?;
        if b.is_owned() {
            self.or_assign(b.borrow(), a)?;
            Ok(b)
        } else {
            let result = self.new_owned(a)?;
            self.or_assign(result.borrow(), b)?;

            Ok(result)
        }
    }

    fn eval_not_equals(&mut self, a: Value, b: Value) -> CompileResult<'a, Value> {
        a.type_check(&Type::U8)?;
        b.type_check(&Type::U8)?;
        if b.is_owned() {
            self.sub_assign(b.borrow(), a)?;
            let result = self.reinterpret_cast(b, Type::Bool)?;
            Ok(result)
        } else {
            let mut result = self.new_owned(a)?;
            self.sub_assign(result.borrow(), b)?;
            result = self.reinterpret_cast(result, Type::Bool)?;

            Ok(result)
        }
    }

    fn eval_equals(&mut self, a: Value, b: Value) -> CompileResult<'a, Value> {
        let not_equals = self.eval_not_equals(a, b)?;
        self.eval_not(not_equals)
    }

    fn eval_less_than_equals(&mut self, a: Value, b: Value) -> CompileResult<'a, Value> {
        let a_temp = self.new_owned(a)?;
        let b_temp = self.new_owned(b)?;
        let result = self.value_from_const(false);
        let loop_value = self.value_from_const(true);
        self.loop_while(loop_value.address(), |compiler| {
            compiler.if_then_else(
                a_temp.borrow(),
                |compiler| {
                    compiler.if_then_else(
                        b_temp.borrow(),
                        |compiler| {
                            compiler.add_to(a_temp.address(), -1);
                            compiler.add_to(b_temp.address(), -1);
                            Ok(())
                        },
                        |compiler| {
                            compiler.zero(a_temp.borrow());
                            compiler.add_to(loop_value.address(), -1);
                            Ok(())
                        },
                    )
                },
                |compiler| {
                    compiler.zero(b_temp.borrow());
                    compiler.add_to(result.address(), 1);
                    compiler.add_to(loop_value.address(), 1);
                    Ok(())
                },
            )?;
            Ok(())
        })?;

        Ok(result)
    }

    fn eval_greater_than_equals(&mut self, a: Value, b: Value) -> CompileResult<'a, Value> {
        self.eval_less_than_equals(b, a)
    }

    fn eval_less_than(&mut self, a: Value, b: Value) -> CompileResult<'a, Value> {
        let opposite = self.eval_greater_than_equals(a, b)?;
        self.eval_not(opposite)
    }

    fn eval_greater_than(&mut self, a: Value, b: Value) -> CompileResult<'a, Value> {
        let opposite = self.eval_less_than_equals(a, b)?;
        self.eval_not(opposite)
    }

    fn eval_const_index(array: &Value, index: u8) -> CompileResult<'a, Value> {
        match &array.value_type {
            Type::Array { element_type, .. } => {
                let start_index = index as u16 * element_type.size();
                let end_index = start_index + element_type.size();
                Ok(array.borrow_slice(start_index, end_index, element_type.as_ref().clone()))
            }
            _ => Err(CompilerError::NotAnArray(array.value_type.clone())),
        }
    }

    fn eval_accessors(
        &mut self,
        accessed_value: AccessedValue,
        f: impl Fn(&mut Self, Value) -> CompileResult<'a, ()>,
    ) -> CompileResult<'a, ()> {
        fn eval_accessors_impl<'a>(
            compiler: &mut BrainCrabCompiler<'a>,
            source: Value,
            accessors: &[Accessor],
            f: &impl Fn(&mut BrainCrabCompiler<'a>, Value) -> CompileResult<'a, ()>,
        ) -> CompileResult<'a, ()> {
            match accessors {
                [] => f(compiler, source),
                [accessor, tail @ ..] => match accessor {
                    Accessor::Index(index) => {
                        let array_type = &source.value_type;
                        if let Type::Array { len, .. } = array_type {
                            for i in 0..*len {
                                let array = source.borrow();
                                compiler.scoped(|compiler| {
                                    let i_value = compiler.value_from_const(i);
                                    let predicate =
                                        compiler.eval_equals(i_value, index.borrow())?;
                                    compiler.if_then(predicate, |compiler| {
                                        let indexed_value =
                                            BrainCrabCompiler::eval_const_index(&array, i)?;
                                        eval_accessors_impl(compiler, indexed_value, tail, f)
                                    })
                                })?;
                            }
                            Ok(())
                        } else {
                            Err(CompilerError::NotAnArray(array_type.clone()))
                        }
                    }
                },
            }
        }
        eval_accessors_impl(self, accessed_value.source, &accessed_value.accessors, &f)
    }

    fn eval_lvalue_expression(
        &mut self,
        expression: LValueExpression<'a>,
    ) -> CompileResult<'a, AccessedValue> {
        match expression {
            LValueExpression::Variable(name) => {
                self.borrow_immutable(name).map(AccessedValue::unit)
            }
            LValueExpression::Index(name, indices) => {
                let array = self.borrow_immutable(name)?;
                let mut accessors = vec![];
                for index_expression in indices {
                    accessors.push(Accessor::Index(self.eval_expression(index_expression)?));
                }
                Ok(AccessedValue::new(array, accessors))
            }
        }
    }

    pub fn eval_expression(&mut self, expression: Expression<'a>) -> CompileResult<'a, Value> {
        match expression {
            Expression::Constant(constant_value) => Ok(self.value_from_const(constant_value)),
            Expression::LValue(expression) => {
                let accessed_value = self.eval_lvalue_expression(expression)?;
                let accessed_value_type = accessed_value.value_type()?;
                let temp = self.allocate(accessed_value_type);
                self.eval_accessors(accessed_value, |compiler, value| {
                    compiler.copy_and_add_values(value, &[temp.borrow()])
                })?;
                Ok(temp)
            }
            Expression::Read => Ok(self.read()),
            Expression::Add(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                self.eval_add(a, b)
            }
            Expression::Sub(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                self.eval_sub(a, b)
            }
            Expression::Mul(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                self.eval_mul(a, b)
            }
            Expression::Div(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                self.eval_div(a, b)
            }
            Expression::Mod(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                self.eval_mod(a, b)
            }
            Expression::Not(inner) => {
                let inner = self.eval_expression(*inner)?;
                self.eval_not(inner)
            }
            Expression::And(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                self.eval_and(a, b)
            }
            Expression::Or(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                self.eval_or(a, b)
            }
            Expression::Equals(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                self.eval_equals(a, b)
            }
            Expression::NotEquals(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                self.eval_not_equals(a, b)
            }
            Expression::LessThanEquals(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                self.eval_less_than_equals(a, b)
            }
            Expression::GreaterThanEquals(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                self.eval_greater_than_equals(a, b)
            }
            Expression::LessThan(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                self.eval_less_than(a, b)
            }
            Expression::GreaterThan(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                self.eval_greater_than(a, b)
            }
        }
    }

    pub fn loop_while_expression<F: FnOnce(&mut Self) -> CompileResult<'a, ()>>(
        &mut self,
        predicate: Expression<'a>,
        body: F,
    ) -> CompileResult<'a, ()> {
        match predicate {
            Expression::Constant(predicate) => {
                if predicate.get_bool()? {
                    // Infinite loop
                    let temp = self.value_from_const(1);
                    self.loop_while(temp.address(), body)
                } else {
                    // Nothing to do here
                    Ok(())
                }
            }
            Expression::LValue(LValueExpression::Variable(variable)) => {
                let predicate = self.borrow_immutable(variable)?;
                self.loop_while(predicate.address(), body)
            }
            _ => {
                let predicate_value = self.eval_expression(predicate.clone())?;
                let temp = self.new_owned(predicate_value)?;
                self.loop_while(temp.address(), |compiler| {
                    body(compiler)?;
                    let predicate_value = compiler.eval_expression(predicate)?;
                    compiler.assign(temp.borrow(), predicate_value)
                })
            }
        }
    }

    fn for_each<F>(&mut self, array: Value, function: F) -> CompileResult<'a, ()>
    where
        F: Fn(&mut Self, Value) -> CompileResult<'a, ()>,
    {
        if let Type::Array { len, .. } = &array.value_type {
            for i in 0..*len {
                self.scoped(|compiler| {
                    let element = Self::eval_const_index(&array.borrow(), i)?;
                    function(compiler, element)
                })?;
            }
            Ok(())
        } else {
            Err(CompilerError::NotAnArray(array.value_type))
        }
    }

    fn for_each_expression(
        &mut self,
        loop_variable: &'a str,
        array_expression: Expression<'a>,
        body: Vec<Instruction<'a>>,
    ) -> CompileResult<'a, ()> {
        let array = self.eval_expression(array_expression)?;

        self.for_each(array, |compiler, value| {
            compiler.register_variable(loop_variable, value)?;
            compiler.compile_instructions(body.clone())
        })
    }
}

/// Instruction compiling
impl<'a> BrainCrabCompiler<'a> {
    fn compile_instructions(
        &mut self,
        instructions: Vec<Instruction<'a>>,
    ) -> CompileResult<'a, ()> {
        // TODO, make this work with a slice of instructions
        for instruction in instructions {
            match instruction {
                Instruction::Define {
                    name,
                    value_type,
                    mutable,
                    value,
                } => {
                    let value = self.eval_expression(value)?;
                    if let Some(value_type) = value_type {
                        value.type_check(&value_type)?;
                    }
                    self.new_variable(name, value, mutable)?;
                }
                Instruction::Assign { name, value } => {
                    let destination = self.eval_lvalue_expression(name)?;
                    let value = self.eval_expression(value)?;
                    self.eval_accessors(destination, |compiler, destination| {
                        compiler.assign(destination.borrow(), value.borrow())
                    })?;
                }
                Instruction::AddAssign { name, value } => {
                    let destination = self.borrow_mutable(name)?;
                    let value = self.eval_expression(value)?;
                    self.add_assign(destination, value)?;
                }
                Instruction::SubAssign { name, value } => {
                    let destination = self.borrow_mutable(name)?;
                    let value = self.eval_expression(value)?;
                    self.sub_assign(destination, value)?;
                }
                Instruction::Write { expression } => {
                    let value = self.eval_expression(expression)?;
                    self.write_value(value);
                }
                Instruction::Print { string } => {
                    self.print_string(string)?;
                }
                Instruction::Scope { body } => {
                    self.scoped(|compiler| compiler.compile_instructions(body))?;
                }
                Instruction::While { predicate, body } => {
                    self.loop_while_expression(predicate, |compiler| {
                        compiler.compile_instructions(body)
                    })?;
                }
                Instruction::IfThenElse {
                    predicate,
                    if_body,
                    else_body,
                } => {
                    let predicate = self.eval_expression(predicate)?;
                    predicate.type_check(&Type::Bool)?;
                    if else_body.is_empty() {
                        self.if_then(predicate, |compiler| compiler.compile_instructions(if_body))?;
                    } else {
                        self.if_then_else(
                            predicate,
                            |compiler| compiler.compile_instructions(if_body),
                            |compiler| compiler.compile_instructions(else_body),
                        )?;
                    }
                }
                Instruction::ForEach {
                    loop_variable,
                    array,
                    body,
                } => self.for_each_expression(loop_variable, array, body)?,
            }
        }
        Ok(())
    }
    pub fn compile_abf(program: Program) -> CompileResult<ABFProgram> {
        let mut compiler = BrainCrabCompiler::new();
        compiler.compile_instructions(program.instructions)?;
        compiler.get_result()
    }
}
