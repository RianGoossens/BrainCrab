use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use crate::{
    absolute_bf::{ABFProgram, ABFTree},
    allocator::BrainCrabAllocator,
    ast::{Expression, Instruction, LValueExpression, Program},
    compiler_error::{CompileResult, CompilerError},
    constant_value::ConstantValue,
    types::Type,
    value::{LValue, Value},
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

    pub fn end_scope(&mut self) -> Vec<LValue> {
        let last_variable_map = self.variable_map_stack.pop().unwrap();
        last_variable_map
            .into_values()
            .filter_map(|x| {
                if let Value::LValue(lvalue) = x {
                    if lvalue.is_owned() {
                        Some(lvalue)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
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
    fn value_type(&self) -> CompileResult<Type> {
        fn value_type_impl(source_type: &Type, accessors: &[Accessor]) -> CompileResult<Type> {
            match accessors {
                [] => Ok(source_type.clone()),
                [Accessor::Index(_), tail @ ..] => {
                    if let Type::Array { element_type, .. } = source_type {
                        value_type_impl(element_type, tail)
                    } else {
                        Err(CompilerError::NotAnArray)
                    }
                }
            }
        }
        value_type_impl(&self.source.value_type()?, &self.accessors)
    }
}

pub struct BrainCrabCompiler<'a> {
    pub program_stack: Vec<ABFProgram>,
    pub variable_map: ScopedVariableMap<'a>,
    pub address_pool: AddressPool,
}

impl<'a> Default for BrainCrabCompiler<'a> {
    fn default() -> Self {
        Self {
            program_stack: vec![ABFProgram::new()],
            variable_map: Default::default(),
            address_pool: Rc::new(RefCell::new(BrainCrabAllocator::new())),
        }
    }
}

impl<'a> BrainCrabCompiler<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn program(&mut self) -> &mut ABFProgram {
        self.program_stack.last_mut().unwrap()
    }

    pub fn push_instruction(&mut self, instruction: ABFTree) {
        self.program().push_instruction(instruction);
    }

    pub fn get_result(mut self) -> CompileResult<ABFProgram> {
        if self.program_stack.len() != 1 {
            Err(CompilerError::UnclosedLoop)
        } else {
            Ok(self.program_stack.pop().unwrap())
        }
    }

    // Memory management

    pub fn allocate(&mut self, value_type: Type) -> CompileResult<LValue> {
        if let Some(address) = self.address_pool.borrow_mut().allocate(value_type.size()) {
            Ok(LValue {
                address,
                value_type,
                address_pool: Some(self.address_pool.clone()),
                mutable: true,
            })
        } else {
            Err(CompilerError::NoFreeAddresses)
        }
    }

    pub fn register_variable(&mut self, name: &'a str, value: Value) -> CompileResult<Value> {
        if self.variable_map.defined_in_current_scope(name) {
            Err(CompilerError::AlreadyDefinedVariable(name.into()))
        } else {
            match &value {
                Value::LValue(lvalue) if lvalue.is_borrowed() => {
                    Err(CompilerError::CantRegisterBorrowedValues(name.into()))
                }
                _ => {
                    let borrowed = value.borrow();
                    self.variable_map.register(name, value);
                    Ok(borrowed)
                }
            }
        }
    }

    pub fn new_variable(
        &mut self,
        name: &'a str,
        value: Value,
        mutable: bool,
    ) -> CompileResult<Value> {
        if mutable || matches!(value, Value::LValue(_)) {
            let mut owned = self.new_owned(value)?;
            owned.mutable = mutable;
            let borrow = owned.borrow();
            self.register_variable(name, owned.into())?;
            Ok(borrow.into())
        } else {
            self.register_variable(name, value)
        }
    }

    pub fn borrow_immutable(&self, name: &'a str) -> CompileResult<Value> {
        if let Some(variable) = self.variable_map.borrow_variable(name) {
            Ok(variable)
        } else {
            Err(CompilerError::UndefinedVariable(name.into()))
        }
    }

    pub fn borrow_mutable(&self, name: &'a str) -> CompileResult<LValue> {
        let result = self.borrow_immutable(name)?;

        match result {
            Value::LValue(lvalue) if lvalue.mutable => Ok(lvalue),
            _ => Err(CompilerError::MutableBorrowOfImmutableVariable(name.into())),
        }
    }

    pub fn new_owned(&mut self, value: impl Into<Value>) -> CompileResult<LValue> {
        let value: Value = value.into();
        match value {
            Value::LValue(lvalue) if lvalue.is_owned() => Ok(lvalue),
            _ => {
                let owned = self.allocate(value.value_type()?)?;
                assert!(
                    value.value_type()?.size() == 1,
                    "bigger types not yet supported"
                );
                self.add_assign(owned.address, value)?;
                Ok(owned)
            }
        }
    }

    pub fn reinterpret_cast(&self, mut value: LValue, new_type: Type) -> CompileResult<LValue> {
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
        self.push_instruction(ABFTree::Add(address, value));
    }

    pub fn write(&mut self, address: u16) {
        self.push_instruction(ABFTree::Write(address));
    }

    pub fn read(&mut self, address: u16) {
        self.push_instruction(ABFTree::Read(address));
    }

    pub fn scoped(&mut self, f: impl FnOnce(&mut Self) -> CompileResult<()>) -> CompileResult<()> {
        self.variable_map.start_scope();
        f(self)?;
        let scope = self.variable_map.end_scope();
        for owned in scope {
            self.zero(owned.address);
        }
        Ok(())
    }

    pub fn loop_while<F: FnOnce(&mut Self) -> CompileResult<()>>(
        &mut self,
        predicate: u16,
        f: F,
    ) -> CompileResult<()> {
        self.program_stack.push(ABFProgram::new());
        self.scoped(|compiler| {
            f(compiler)?;
            Ok(())
        })?;

        let loop_program = self.program_stack.pop().unwrap();
        self.push_instruction(ABFTree::While(predicate, loop_program.body));
        Ok(())
    }

    // Utilities
    pub fn if_then<I: FnOnce(&mut Self) -> CompileResult<()>>(
        &mut self,
        predicate: Value,
        body: I,
    ) -> CompileResult<()> {
        match predicate {
            Value::Constant(value) => {
                if value.get_bool()? {
                    body(self)
                } else {
                    Ok(())
                }
            }
            Value::LValue(variable) => {
                let if_check = self.new_owned(variable)?;
                self.loop_while(if_check.address, |compiler| {
                    body(compiler)?;
                    compiler.zero(if_check.address);
                    Ok(())
                })
            }
        }
    }

    pub fn if_then_else<
        I: FnOnce(&mut Self) -> CompileResult<()>,
        E: FnOnce(&mut Self) -> CompileResult<()>,
    >(
        &mut self,
        predicate: Value,
        if_case: I,
        else_case: E,
    ) -> CompileResult<()> {
        match predicate {
            Value::Constant(value) => {
                if value.get_bool()? {
                    if_case(self)
                } else {
                    else_case(self)
                }
            }
            Value::LValue(variable) => {
                let else_check = self.new_owned(1)?;
                let if_check = self.new_owned(variable)?;
                self.loop_while(if_check.address, |compiler| {
                    if_case(compiler)?;
                    compiler.sub_assign(else_check.address, 1.into())?;
                    compiler.zero(if_check.address);
                    Ok(())
                })?;
                self.loop_while(else_check.address, |compiler| {
                    else_case(compiler)?;
                    compiler.sub_assign(else_check.address, 1.into())?;
                    Ok(())
                })
            }
        }
    }

    pub fn n_times(
        &mut self,
        n: Value,
        f: impl Fn(&mut Self) -> CompileResult<()>,
    ) -> CompileResult<()> {
        match n {
            Value::Constant(n) => match n {
                ConstantValue::U8(n) => {
                    for _ in 0..n {
                        self.scoped(|compiler| f(compiler))?
                    }
                }
                ConstantValue::Bool(n) => {
                    if n {
                        self.scoped(|compiler| f(compiler))?
                    }
                }
                _ => {
                    panic!("n times with a constant value which is not a bool or u8. Instead got {n:?}")
                }
            },
            Value::LValue(lvalue) => {
                if lvalue.is_owned() {
                    self.loop_while(lvalue.address, |compiler| {
                        compiler.add_to(lvalue.address, -1);
                        f(compiler)?;
                        Ok(())
                    })?;
                } else {
                    let address = lvalue.address;
                    let temp = self.new_owned(0)?;
                    self.loop_while(address, |compiler| {
                        compiler.add_to(address, -1);
                        compiler.add_to(temp.address, 1);
                        f(compiler)?;
                        Ok(())
                    })?;
                    self.loop_while(temp.address, |compiler| {
                        compiler.add_to(temp.address, -1);
                        compiler.add_to(address, 1);
                        Ok(())
                    })?;
                }
            }
        }
        Ok(())
    }

    pub fn write_value(&mut self, value: Value) -> CompileResult<()> {
        match &value {
            Value::LValue(lvalue) if lvalue.is_borrowed() => {
                self.write(lvalue.address);
                Ok(())
            }
            _ => {
                let owned = self.new_owned(value)?;
                self.write(owned.address);
                self.zero(owned.address);
                Ok(())
            }
        }
    }

    pub fn zero(&mut self, address: u16) {
        self.program().append(ABFProgram {
            body: vec![ABFTree::While(address, vec![ABFTree::Add(address, -1)])],
        });
    }

    pub fn add_assign(&mut self, destination: u16, value: Value) -> CompileResult<()> {
        if let Value::LValue(variable) = &value {
            let value_address = variable.address;
            if value_address == destination {
                assert!(!variable.is_owned(), "Attempting to add a temp onto itself, which is not allowed as it's already consumed");
                let temp = self.new_owned(0)?;
                self.copy_on_top_of_cells(value, &[temp.address])?;
                self.copy_on_top_of_cells(temp.into(), &[value_address])?;
                return Ok(());
            }
        }
        self.copy_on_top_of_cells(value, &[destination])
    }

    pub fn sub_assign(&mut self, destination: u16, value: Value) -> CompileResult<()> {
        if let Value::LValue(variable) = &value {
            let value_address = variable.address;
            if value_address == destination {
                assert!(!variable.is_owned(), "Attempting to sub a temp from itself, which is not allowed as it's already consumed");
                self.zero(destination);
                return Ok(());
            }
        }
        self.n_times(value, |compiler| {
            compiler.add_to(destination, -1);
            Ok(())
        })
    }

    pub fn mul_assign(&mut self, destination: u16, value: Value) -> CompileResult<()> {
        let result = self.new_owned(0)?;
        self.n_times(value, move |compiler| {
            compiler.add_assign(result.address, Value::new_borrow(destination, Type::U8))?;
            Ok(())
        })?;
        self.assign(destination, result.into())
    }

    pub fn div_assign(&mut self, destination: u16, value: Value) -> CompileResult<()> {
        if let Value::LValue(variable) = &value {
            let value_address = variable.address;
            if value_address == destination {
                assert!(!variable.is_owned(), "Attempting to div a temp from itself, which is not allowed as it's already consumed");
                self.zero(destination);
                self.add_assign(destination, 1.into())?;
                return Ok(());
            }
        }
        let result = self.new_owned(0)?;

        self.loop_while(destination, |compiler| {
            let predicate = compiler
                .eval_less_than_equals(value.borrow(), Value::new_borrow(destination, Type::U8))?;
            compiler.if_then_else(
                predicate,
                |compiler| {
                    compiler.sub_assign(destination, value)?;
                    compiler.add_assign(result.address, 1.into())
                },
                |compiler| {
                    compiler.zero(destination);
                    Ok(())
                },
            )
        })?;
        self.copy_on_top_of_cells(result.into(), &[destination])
    }

    pub fn mod_assign(&mut self, destination: u16, value: Value) -> CompileResult<()> {
        if let Value::LValue(variable) = &value {
            let value_address = variable.address;
            if value_address == destination {
                assert!(!variable.is_owned(), "Attempting to mod a temp from itself, which is not allowed as it's already consumed");
                self.zero(destination);
                return Ok(());
            }
        }

        let predicate = self
            .eval_greater_than_equals(Value::new_borrow(destination, Type::U8), value.borrow())?;
        let predicate = self.new_owned(predicate)?;
        self.loop_while(predicate.address, |compiler| {
            compiler.sub_assign(destination, value.borrow())?;
            let new_predicate = compiler.eval_greater_than_equals(
                Value::new_borrow(destination, Type::U8),
                value.borrow(),
            )?;
            compiler.assign(predicate.address, new_predicate)
        })
    }

    pub fn not_assign(&mut self, destination: u16, value: Value) -> CompileResult<()> {
        self.if_then_else(
            value,
            |compiler| {
                compiler.zero(destination);
                Ok(())
            },
            |compiler| compiler.add_assign(destination, 1.into()),
        )
    }
    pub fn and_assign(&mut self, destination: u16, value: Value) -> CompileResult<()> {
        self.if_then_else(
            value,
            |_| Ok(()),
            |compiler| {
                compiler.zero(destination);
                Ok(())
            },
        )
    }
    pub fn or_assign(&mut self, destination: u16, value: Value) -> CompileResult<()> {
        self.if_then_else(
            Value::new_borrow(destination, Type::Bool),
            |_| Ok(()),
            |compiler| {
                compiler.if_then(value, |compiler| compiler.add_assign(destination, 1.into()))
            },
        )
    }

    pub fn assign(&mut self, destination: u16, value: Value) -> CompileResult<()> {
        if let Value::LValue(variable) = &value {
            let value_address = variable.address;
            if value_address == destination {
                // assigning to self is a no-op
                return Ok(());
            }
        }
        self.zero(destination);
        self.add_assign(destination, value)?;
        Ok(())
    }

    pub fn move_on_top_of_cells(
        &mut self,
        source: LValue,
        destinations: &[u16],
    ) -> CompileResult<()> {
        self.loop_while(source.address, |compiler| {
            compiler.add_to(source.address, -1);
            for destination in destinations {
                compiler.add_to(*destination, 1);
            }
            Ok(())
        })
    }

    pub fn copy_on_top_of_cells(
        &mut self,
        source: Value,
        destinations: &[u16],
    ) -> CompileResult<()> {
        self.n_times(source, |compiler| {
            for destination in destinations {
                compiler.add_to(*destination, 1);
            }
            Ok(())
        })?;
        Ok(())
    }

    pub fn print_string(&mut self, string: &str) -> CompileResult<()> {
        if string.is_ascii() {
            let temp = self.new_owned(0)?;
            let mut current_value = 0u8;
            for char in string.chars() {
                let new_value = char as u8;
                let offset = new_value.wrapping_sub(current_value);
                self.add_assign(temp.address, offset.into())?;
                self.write(temp.address);
                current_value = new_value;
            }
            self.sub_assign(temp.address, current_value.into())?;

            Ok(())
        } else {
            Err(CompilerError::NonAsciiString(string.to_owned()))
        }
    }

    // Expressions

    fn eval_add(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        a.type_check(Type::U8)?;
        b.type_check(Type::U8)?;
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => {
                let a = a.get_u8()?;
                let b = b.get_u8()?;
                Ok(a.wrapping_add(b).into())
            }
            (a, Value::LValue(b)) if b.is_owned() => {
                self.add_assign(b.address, a)?;
                Ok(b.into())
            }
            (a, b) => {
                let temp = self.new_owned(a)?;
                self.add_assign(temp.address, b)?;

                Ok(temp.into())
            }
        }
    }

    fn eval_mul(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        a.type_check(Type::U8)?;
        b.type_check(Type::U8)?;
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => {
                let a = a.get_u8()?;
                let b = b.get_u8()?;
                Ok(a.wrapping_mul(b).into())
            }
            (a, Value::LValue(b)) if b.is_owned() => {
                self.mul_assign(b.address, a)?;
                Ok(b.into())
            }
            (a, b) => {
                let temp = self.new_owned(a)?;
                self.mul_assign(temp.address, b)?;

                Ok(temp.into())
            }
        }
    }

    fn eval_sub(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        a.type_check(Type::U8)?;
        b.type_check(Type::U8)?;
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => {
                let a = a.get_u8()?;
                let b = b.get_u8()?;
                Ok(a.wrapping_sub(b).into())
            }
            (a, b) => {
                let temp = self.new_owned(a)?;
                self.sub_assign(temp.address, b)?;

                Ok(temp.into())
            }
        }
    }

    fn eval_div(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        a.type_check(Type::U8)?;
        b.type_check(Type::U8)?;
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => {
                let a = a.get_u8()?;
                let b = b.get_u8()?;
                Ok(a.wrapping_div(b).into())
            }
            (a, b) => {
                let temp = self.new_owned(a)?;
                self.div_assign(temp.address, b)?;

                Ok(temp.into())
            }
        }
    }

    fn eval_mod(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        a.type_check(Type::U8)?;
        b.type_check(Type::U8)?;
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => {
                let a = a.get_u8()?;
                let b = b.get_u8()?;
                Ok((a % b).into())
            }
            (a, b) => {
                let temp = self.new_owned(a)?;
                self.mod_assign(temp.address, b)?;

                Ok(temp.into())
            }
        }
    }

    fn eval_not(&mut self, inner: Value) -> CompileResult<Value> {
        inner.type_check(Type::Bool)?;
        match inner {
            Value::Constant(value) => {
                if value.get_bool()? {
                    Ok(false.into())
                } else {
                    Ok(true.into())
                }
            }
            Value::LValue(lvalue) => {
                if lvalue.is_owned() {
                    self.not_assign(lvalue.address, lvalue.borrow().into())?;
                    Ok(lvalue.into())
                } else {
                    let result = self.new_owned(false)?;
                    self.not_assign(
                        result.address,
                        Value::new_borrow(lvalue.address, Type::Bool),
                    )?;
                    Ok(result.into())
                }
            }
        }
    }

    fn eval_and(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        a.type_check(Type::Bool)?;
        b.type_check(Type::Bool)?;
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => {
                let a = a.get_bool()?;
                let b = b.get_bool()?;
                Ok((a && b).into())
            }
            (Value::LValue(a), b) if a.is_owned() => {
                self.and_assign(a.address, b)?;
                Ok(a.into())
            }
            (a, Value::LValue(b)) if b.is_owned() => {
                self.and_assign(b.address, a)?;
                Ok(b.into())
            }
            (a, b) => {
                let temp = self.new_owned(a)?;
                self.and_assign(temp.address, b)?;

                Ok(temp.into())
            }
        }
    }

    fn eval_or(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        a.type_check(Type::Bool)?;
        b.type_check(Type::Bool)?;
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => {
                let a = a.get_bool()?;
                let b = b.get_bool()?;
                Ok((a || b).into())
            }
            (Value::LValue(a), b) if a.is_owned() => {
                self.or_assign(a.address, b)?;
                Ok(a.into())
            }
            (a, Value::LValue(b)) if b.is_owned() => {
                self.or_assign(b.address, a)?;
                Ok(b.into())
            }
            (a, b) => {
                let temp = self.new_owned(a)?;
                self.or_assign(temp.address, b)?;

                Ok(temp.into())
            }
        }
    }

    fn eval_not_equals(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        a.type_check(Type::U8)?;
        b.type_check(Type::U8)?;
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => {
                let a = a.get_u8()?;
                let b = b.get_u8()?;
                Ok((a != b).into())
            }
            (a, Value::LValue(mut b)) if b.is_owned() => {
                self.sub_assign(b.address, a)?;
                b = self.reinterpret_cast(b, Type::Bool)?;
                Ok(b.into())
            }
            (a, b) => {
                let mut temp = self.new_owned(a)?;
                self.sub_assign(temp.address, b)?;
                temp = self.reinterpret_cast(temp, Type::Bool)?;

                Ok(temp.into())
            }
        }
    }

    fn eval_equals(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        let not_equals = self.eval_not_equals(a, b)?;
        self.eval_not(not_equals)
    }

    fn eval_less_than_equals(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => {
                let a = a.get_u8()?;
                let b = b.get_u8()?;
                Ok((a <= b).into())
            }
            (a, b) => {
                let a_temp = self.new_owned(a)?;
                let b_temp = self.new_owned(b)?;
                let result = self.new_owned(false)?;
                let loop_value = self.new_owned(true)?;
                self.loop_while(loop_value.address, |compiler| {
                    compiler.if_then_else(
                        a_temp.borrow().into(),
                        |compiler| {
                            compiler.if_then_else(
                                b_temp.borrow().into(),
                                |compiler| {
                                    compiler.sub_assign(a_temp.address, 1.into())?;
                                    compiler.sub_assign(b_temp.address, 1.into())
                                },
                                |compiler| {
                                    compiler.zero(a_temp.address);
                                    compiler.sub_assign(loop_value.address, 1.into())
                                },
                            )
                        },
                        |compiler| {
                            compiler.zero(b_temp.address);
                            compiler.add_assign(result.address, 1.into())?;
                            compiler.sub_assign(loop_value.address, 1.into())
                        },
                    )?;
                    Ok(())
                })?;

                Ok(result.into())
            }
        }
    }

    fn eval_greater_than_equals(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        self.eval_less_than_equals(b, a)
    }

    fn eval_less_than(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        let opposite = self.eval_greater_than_equals(a, b)?;
        self.eval_not(opposite)
    }

    fn eval_greater_than(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        let opposite = self.eval_less_than_equals(a, b)?;
        self.eval_not(opposite)
    }

    fn eval_const_index(array: Value, index: u8) -> CompileResult<Value> {
        if let Value::Constant(ConstantValue::Array(array)) = array {
            Ok(array[index as usize].clone().into())
        } else {
            Err(CompilerError::NotAnArray)
        }
    }
    fn eval_const_accessors(source: Value, accessors: &[Accessor]) -> CompileResult<Option<Value>> {
        match accessors {
            [] => Ok(Some(source)),
            [Accessor::Index(Value::Constant(index)), tail @ ..] => {
                let indexed_value = Self::eval_const_index(source, index.get_u8()?)?;
                Self::eval_const_accessors(indexed_value, tail)
            }
            _ => Ok(None),
        }
    }
    fn eval_accessors(
        &mut self,
        source: Value,
        accessors: &[Accessor],
        f: &impl Fn(&mut Self, Value) -> CompileResult<()>,
    ) -> CompileResult<()> {
        match accessors {
            [] => f(self, source),
            [accessor, tail @ ..] => match accessor {
                Accessor::Index(index) => match index {
                    Value::Constant(index) => {
                        let indexed_value = Self::eval_const_index(source, index.get_u8()?)?;
                        self.eval_accessors(indexed_value, tail, f)
                    }
                    Value::LValue(index) => {
                        if let Type::Array { len, .. } = source.value_type()? {
                            for i in 0..len {
                                let array = source.borrow();
                                self.scoped(|compiler| {
                                    let predicate =
                                        compiler.eval_equals(i.into(), index.borrow().into())?;
                                    compiler.if_then(predicate, |compiler| {
                                        let indexed_value = Self::eval_const_index(array, i)?;
                                        compiler.eval_accessors(indexed_value, tail, f)
                                    })
                                })?;
                            }
                            Ok(())
                        } else {
                            Err(CompilerError::NotAnArray)
                        }
                    }
                },
            },
        }
    }

    fn eval_lvalue_expression(
        &mut self,
        expression: LValueExpression<'a>,
    ) -> CompileResult<AccessedValue> {
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

    pub fn eval_expression(&mut self, expression: Expression<'a>) -> CompileResult<Value> {
        match expression {
            Expression::Constant(constant_value) => Ok(constant_value.into()),
            Expression::LValue(expression) => {
                let accessed_value = self.eval_lvalue_expression(expression)?;
                if let Some(value) = Self::eval_const_accessors(
                    accessed_value.source.borrow(),
                    &accessed_value.accessors,
                )? {
                    Ok(value)
                } else {
                    let accessed_value_type = accessed_value.value_type()?;
                    let temp = self.allocate(accessed_value_type)?;
                    self.eval_accessors(
                        accessed_value.source,
                        &accessed_value.accessors,
                        &|compiler, value| compiler.copy_on_top_of_cells(value, &[temp.address]),
                    )?;
                    Ok(temp.into())
                }
            }
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

    pub fn loop_while_expression<F: FnOnce(&mut Self) -> CompileResult<()>>(
        &mut self,
        predicate: Expression<'a>,
        body: F,
    ) -> CompileResult<()> {
        match predicate {
            Expression::Constant(predicate) => {
                if predicate.get_bool()? {
                    // Infinite loop
                    let temp = self.new_owned(1)?;
                    self.loop_while(temp.address, body)
                } else {
                    // Nothing to do here
                    Ok(())
                }
            }
            Expression::LValue(LValueExpression::Variable(variable)) => {
                let predicate = self.borrow_immutable(variable)?;
                match predicate {
                    Value::Constant(predicate) => {
                        if predicate.get_bool()? {
                            // Infinite loop
                            let temp = self.new_owned(1)?;
                            self.loop_while(temp.address, body)
                        } else {
                            // Nothing to do here
                            Ok(())
                        }
                    }
                    Value::LValue(predicate) => self.loop_while(predicate.address, body),
                }
            }
            _ => {
                let predicate_value = self.eval_expression(predicate.clone())?;
                let temp = self.new_owned(predicate_value)?;
                self.loop_while(temp.address, |compiler| {
                    body(compiler)?;
                    let predicate_value = compiler.eval_expression(predicate)?;
                    compiler.assign(temp.address, predicate_value)?;
                    Ok(())
                })
            }
        }
    }

    fn for_each<F>(&mut self, array: Value, function: F) -> CompileResult<()>
    where
        F: Fn(&mut Self, Value) -> CompileResult<()>,
    {
        if let Type::Array { len, .. } = array.value_type()? {
            for i in 0..len {
                self.scoped(|compiler| {
                    let element = Self::eval_const_index(array.borrow(), i)?;
                    function(compiler, element)
                })?;
            }
            Ok(())
        } else {
            Err(CompilerError::NotAnArray)
        }
    }

    fn for_each_expression(
        &mut self,
        loop_variable: &'a str,
        array_expression: Expression<'a>,
        body: Vec<Instruction<'a>>,
    ) -> CompileResult<()> {
        let array = self.eval_expression(array_expression)?;

        self.for_each(array, |compiler, value| {
            compiler.register_variable(loop_variable, value)?;
            compiler.compile_instructions(body.clone())
        })
    }
}

/// Instruction compiling
impl<'a> BrainCrabCompiler<'a> {
    fn compile_instructions(&mut self, instructions: Vec<Instruction<'a>>) -> CompileResult<()> {
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
                        value.type_check(value_type)?;
                    }
                    self.new_variable(name, value, mutable)?;
                }
                Instruction::Assign { name, value } => {
                    let destination = self.borrow_mutable(name)?;
                    let value = self.eval_expression(value)?;
                    self.assign(destination.address, value)?;
                }
                Instruction::AddAssign { name, value } => {
                    let destination = self.borrow_mutable(name)?;
                    let value = self.eval_expression(value)?;
                    self.add_assign(destination.address, value)?;
                }
                Instruction::SubAssign { name, value } => {
                    let destination = self.borrow_mutable(name)?;
                    let value = self.eval_expression(value)?;
                    self.sub_assign(destination.address, value)?;
                }
                Instruction::Write { expression } => {
                    let value = self.eval_expression(expression)?;
                    self.write_value(value)?;
                }
                Instruction::Read { name } => {
                    let destination = self.borrow_mutable(name)?;
                    self.read(destination.address);
                }
                Instruction::Print { string } => {
                    self.print_string(&string)?;
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
                    predicate.type_check(Type::Bool)?;
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
