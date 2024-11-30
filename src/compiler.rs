use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    absolute_bf::{ABFProgram, ABFTree},
    allocator::BrainCrabAllocator,
    ast::{Expression, Instruction, Program},
    value::{Owned, Value, Variable},
};

pub type AddressPool = Rc<RefCell<Vec<u16>>>;

#[derive(Debug)]
pub enum CompilerError {
    UndefinedVariable(String),
    AlreadyDefinedVariable(String),
    NoFreeAddresses,
    UnclosedLoop,
    NonAsciiString(String),
}

pub type CompileResult<A> = Result<A, CompilerError>;

pub struct ScopedVariableMap<'a> {
    pub variable_map_stack: Vec<HashMap<&'a str, Owned>>,
}

impl<'a> Default for ScopedVariableMap<'a> {
    fn default() -> Self {
        Self {
            variable_map_stack: vec![HashMap::new()],
        }
    }
}

impl<'a> ScopedVariableMap<'a> {
    pub fn borrow_variable(&self, name: &'a str) -> Option<Variable> {
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

    pub fn register(&mut self, name: &'a str, value: Owned) {
        self.variable_map_stack
            .last_mut()
            .unwrap()
            .insert(name, value);
    }

    pub fn start_scope(&mut self) {
        self.variable_map_stack.push(HashMap::new());
    }

    pub fn end_scope(&mut self) -> Vec<Owned> {
        let last_variable_map = self.variable_map_stack.pop().unwrap();
        last_variable_map.into_values().collect()
    }
}

pub struct BrainCrabCompiler<'a> {
    pub program_stack: Vec<ABFProgram>,
    pub variable_map: ScopedVariableMap<'a>,
    pub address_pool: AddressPool,
    pub pointer: u16,
}

impl<'a> Default for BrainCrabCompiler<'a> {
    fn default() -> Self {
        Self {
            program_stack: vec![ABFProgram::new()],
            variable_map: Default::default(),
            address_pool: Rc::new(RefCell::new(BrainCrabAllocator::new_allocator())),
            pointer: 0,
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

    pub fn allocate(&mut self) -> CompileResult<Owned> {
        if let Some(address) = self.address_pool.borrow_mut().allocate(self.pointer) {
            Ok(Owned {
                address,
                address_pool: self.address_pool.clone(),
                dirty: false,
            })
        } else {
            Err(CompilerError::NoFreeAddresses)
        }
    }

    pub fn new_variable(&mut self, name: &'a str, value: Value) -> CompileResult<u16> {
        if self.variable_map.defined_in_current_scope(name) {
            Err(CompilerError::AlreadyDefinedVariable(name.into()))
        } else {
            let owned = self.allocate()?;
            let address = owned.address;
            self.variable_map.register(name, owned);

            self.n_times(value, |compiler| {
                compiler.move_pointer_to(address);
                compiler.inc_current();
                Ok(())
            })?;
            Ok(address)
        }
    }

    pub fn get_variable(&self, name: &'a str) -> CompileResult<Variable> {
        if let Some(variable) = self.variable_map.borrow_variable(name) {
            Ok(variable)
        } else {
            Err(CompilerError::UndefinedVariable(name.into()))
        }
    }

    pub fn get_address(&self, name: &'a str) -> CompileResult<u16> {
        let variable = self.get_variable(name)?;
        Ok(variable.address())
    }

    pub fn new_owned<V: Into<Value>>(&mut self, value: V) -> CompileResult<Owned> {
        let value: Value = value.into();
        match value {
            Value::Constant(_) | Value::Variable(Variable::Borrow(_)) => {
                let owned = self.allocate()?;
                self.add_assign(owned.address, value)?;
                Ok(owned)
            }
            Value::Variable(Variable::Owned(owned)) => Ok(owned),
        }
    }

    // Primitives

    pub fn move_pointer_to(&mut self, address: u16) {
        self.pointer = address;
        self.push_instruction(ABFTree::MoveTo(address));
    }

    pub fn inc_current(&mut self) {
        self.push_instruction(ABFTree::Add(self.pointer, 1));
    }

    pub fn dec_current(&mut self) {
        self.push_instruction(ABFTree::Add(self.pointer, -1));
    }

    pub fn write_current(&mut self) {
        self.push_instruction(ABFTree::Write(self.pointer));
    }

    pub fn read_current(&mut self) {
        self.push_instruction(ABFTree::Read(self.pointer));
    }

    pub fn scoped<F: FnOnce(&mut Self) -> CompileResult<()>>(&mut self, f: F) -> CompileResult<()> {
        self.variable_map.start_scope();
        f(self)?;
        let last_pointer_position = self.pointer;
        let scope = self.variable_map.end_scope();
        for owned in scope {
            self.zero(owned.address);
        }
        self.move_pointer_to(last_pointer_position);
        Ok(())
    }

    pub fn loop_while<F: FnOnce(&mut Self) -> CompileResult<()>>(
        &mut self,
        predicate: u16,
        f: F,
    ) -> CompileResult<()> {
        self.move_pointer_to(predicate);

        self.program_stack.push(ABFProgram::new());
        self.scoped(|compiler| {
            f(compiler)?;
            compiler.move_pointer_to(predicate);
            Ok(())
        })?;

        let loop_program = self.program_stack.pop().unwrap();
        self.push_instruction(ABFTree::While(self.pointer, loop_program.body));
        Ok(())
    }

    pub fn if_then<I: FnOnce(&mut Self) -> CompileResult<()>>(
        &mut self,
        predicate: Value,
        body: I,
    ) -> CompileResult<()> {
        match predicate {
            Value::Constant(value) => {
                if value > 0 {
                    body(self)
                } else {
                    Ok(())
                }
            }
            Value::Variable(variable) => {
                let if_check = self.new_owned(variable)?;
                self.loop_while(if_check.address, |compiler| {
                    body(compiler)?;
                    compiler.zero(if_check.address);
                    Ok(())
                })
            }
        }
    }
    // Utilities
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
                if value > 0 {
                    if_case(self)
                } else {
                    else_case(self)
                }
            }
            Value::Variable(variable) => {
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

    pub fn n_times<F: Fn(&mut Self) -> CompileResult<()>>(
        &mut self,
        n: Value,
        f: F,
    ) -> CompileResult<()> {
        match n {
            Value::Constant(n) => {
                for _ in 0..n {
                    self.scoped(|compiler| f(compiler))?
                }
            }
            Value::Variable(variable) => match variable {
                Variable::Owned(temp) => {
                    self.loop_while(temp.address, |compiler| {
                        compiler.dec_current();
                        f(compiler)?;
                        Ok(())
                    })?;
                }
                _ => {
                    let address = variable.address();
                    let temp = self.new_owned(0)?;
                    self.loop_while(address, |compiler| {
                        compiler.dec_current();
                        compiler.move_pointer_to(temp.address);
                        compiler.inc_current();
                        f(compiler)?;
                        Ok(())
                    })?;
                    self.loop_while(temp.address, |compiler| {
                        compiler.dec_current();
                        compiler.move_pointer_to(address);
                        compiler.inc_current();
                        Ok(())
                    })?;
                }
            },
        }
        Ok(())
    }

    pub fn zero(&mut self, address: u16) {
        self.move_pointer_to(address);
        self.program().append(ABFProgram {
            body: vec![ABFTree::While(address, vec![ABFTree::Add(address, -1)])],
        });
    }

    pub fn add_assign(&mut self, destination: u16, value: Value) -> CompileResult<()> {
        if let Value::Variable(variable) = &value {
            let value_address = variable.address();
            if value_address == destination {
                assert!(!variable.is_owned(), "Attempting to add a temp onto itself, which is not allowed as it's already consumed");
                let temp = self.new_owned(0)?;
                self.copy_on_top_of_cells(value, &[temp.address])?;
                self.copy_on_top_of_cells(Value::owned(temp), &[value_address])?;
                return Ok(());
            }
        }
        self.copy_on_top_of_cells(value, &[destination])
    }

    pub fn sub_assign(&mut self, destination: u16, value: Value) -> CompileResult<()> {
        if let Value::Variable(variable) = &value {
            let value_address = variable.address();
            if value_address == destination {
                assert!(!variable.is_owned(), "Attempting to sub a temp from itself, which is not allowed as it's already consumed");
                self.zero(destination);
                return Ok(());
            }
        }
        self.n_times(value, |compiler| {
            compiler.move_pointer_to(destination);
            compiler.dec_current();
            Ok(())
        })
    }

    pub fn mul_assign(&mut self, destination: u16, value: Value) -> CompileResult<()> {
        let result = self.new_owned(0)?;
        self.n_times(value, move |compiler| {
            compiler.add_assign(result.address, Value::new_borrow(destination))?;
            Ok(())
        })?;
        self.assign(destination, result.into())
    }

    pub fn div_assign(&mut self, destination: u16, value: Value) -> CompileResult<()> {
        if let Value::Variable(variable) = &value {
            let value_address = variable.address();
            if value_address == destination {
                assert!(!variable.is_owned(), "Attempting to div a temp from itself, which is not allowed as it's already consumed");
                self.zero(destination);
                self.add_assign(destination, 1.into())?;
                return Ok(());
            }
        }
        let result = self.new_owned(0)?;

        self.loop_while(destination, |compiler| {
            let predicate =
                compiler.eval_less_than_equals(value.borrow(), Value::new_borrow(destination))?;
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
            Value::new_borrow(destination),
            |_| Ok(()),
            |compiler| {
                compiler.if_then(value, |compiler| compiler.add_assign(destination, 1.into()))
            },
        )
    }

    pub fn assign(&mut self, destination: u16, value: Value) -> CompileResult<()> {
        if let Value::Variable(variable) = &value {
            let value_address = variable.address();
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
        source: Variable,
        destinations: &[u16],
    ) -> CompileResult<()> {
        self.loop_while(source.address(), |compiler| {
            compiler.move_pointer_to(source.address());
            compiler.dec_current();
            for destination in destinations {
                compiler.move_pointer_to(*destination);
                compiler.inc_current();
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
                compiler.move_pointer_to(*destination);
                compiler.inc_current();
            }
            Ok(())
        })?;
        Ok(())
    }

    pub fn print_string(&mut self, string: &str) -> CompileResult<()> {
        if string.is_ascii() {
            let temp = self.new_owned(0)?;
            self.move_pointer_to(temp.address);
            let mut current_value = 0u8;
            for char in string.chars() {
                let new_value = char as u8;
                let offset = new_value.wrapping_sub(current_value);
                self.add_assign(temp.address, Value::constant(offset))?;
                self.write_current();
                current_value = new_value;
            }
            self.sub_assign(temp.address, Value::constant(current_value))?;

            Ok(())
        } else {
            Err(CompilerError::NonAsciiString(string.to_owned()))
        }
    }

    // Expressions

    fn eval_add(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => Ok(Value::Constant(a.wrapping_add(b))),
            (Value::Variable(Variable::Owned(a)), b) => {
                self.add_assign(a.address, b)?;
                Ok(Value::owned(a))
            }
            (a, Value::Variable(Variable::Owned(b))) => {
                self.add_assign(b.address, a)?;
                Ok(Value::owned(b))
            }
            (a, b) => {
                let temp = self.new_owned(a)?;
                self.add_assign(temp.address, b)?;

                Ok(Value::owned(temp))
            }
        }
    }

    fn eval_mul(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => Ok(Value::Constant(a.wrapping_mul(b))),
            (Value::Variable(Variable::Owned(a)), b) => {
                self.mul_assign(a.address, b)?;
                Ok(Value::owned(a))
            }
            (a, Value::Variable(Variable::Owned(b))) => {
                self.mul_assign(b.address, a)?;
                Ok(Value::owned(b))
            }
            (a, b) => {
                let temp = self.new_owned(a)?;
                self.mul_assign(temp.address, b)?;

                Ok(Value::owned(temp))
            }
        }
    }

    fn eval_sub(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => Ok(Value::Constant(a.wrapping_sub(b))),
            (Value::Variable(Variable::Owned(a)), b) => {
                self.sub_assign(a.address, b)?;
                Ok(Value::owned(a))
            }
            (a, b) => {
                let temp = self.new_owned(a)?;
                self.sub_assign(temp.address, b)?;

                Ok(Value::owned(temp))
            }
        }
    }

    fn eval_div(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => Ok(Value::Constant(a.wrapping_div(b))),
            (Value::Variable(Variable::Owned(a)), b) => {
                self.div_assign(a.address, b)?;
                Ok(a.into())
            }
            (a, b) => {
                let temp = self.new_owned(a)?;
                self.div_assign(temp.address, b)?;

                Ok(temp.into())
            }
        }
    }

    fn eval_not(&mut self, inner: Value) -> CompileResult<Value> {
        match inner {
            Value::Constant(value) => {
                if value > 0 {
                    Ok(0.into())
                } else {
                    Ok(1.into())
                }
            }
            Value::Variable(Variable::Owned(owned)) => {
                self.not_assign(owned.address, owned.borrow().into())?;
                Ok(owned.into())
            }
            Value::Variable(Variable::Borrow(borrowed)) => {
                let result = self.new_owned(0)?;
                self.not_assign(result.address, Value::new_borrow(borrowed))?;
                Ok(result.into())
            }
        }
    }

    fn eval_and(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => {
                Ok(Value::Constant(((a != 0) && (b != 0)) as u8))
            }
            (Value::Variable(Variable::Owned(a)), b) => {
                self.and_assign(a.address, b)?;
                Ok(Value::owned(a))
            }
            (a, Value::Variable(Variable::Owned(b))) => {
                self.and_assign(b.address, a)?;
                Ok(Value::owned(b))
            }
            (a, b) => {
                let temp = self.new_owned(a)?;
                self.and_assign(temp.address, b)?;

                Ok(Value::owned(temp))
            }
        }
    }

    fn eval_or(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => {
                Ok(Value::Constant(((a != 0) || (b != 0)) as u8))
            }
            (Value::Variable(Variable::Owned(a)), b) => {
                self.or_assign(a.address, b)?;
                Ok(Value::owned(a))
            }
            (a, Value::Variable(Variable::Owned(b))) => {
                self.or_assign(b.address, a)?;
                Ok(Value::owned(b))
            }
            (a, b) => {
                let temp = self.new_owned(a)?;
                self.or_assign(temp.address, b)?;

                Ok(Value::owned(temp))
            }
        }
    }

    fn eval_not_equals(&mut self, a: Value, b: Value) -> CompileResult<Value> {
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => {
                Ok(Value::Constant(if a != b { 1 } else { 0 }))
            }
            (Value::Variable(Variable::Owned(a)), b) => {
                self.sub_assign(a.address, b)?;
                Ok(Value::owned(a))
            }
            (a, Value::Variable(Variable::Owned(b))) => {
                self.sub_assign(b.address, a)?;
                Ok(Value::owned(b))
            }
            (a, b) => {
                let temp = self.new_owned(a)?;
                self.sub_assign(temp.address, b)?;

                Ok(Value::owned(temp))
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
                Ok(Value::Constant(if a <= b { 1 } else { 0 }))
            }
            (a, b) => {
                let a_temp = self.new_owned(a)?;
                let b_temp = self.new_owned(b)?;
                let result = self.new_owned(0)?;
                let loop_value = self.new_owned(1)?;
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

    pub fn eval_expression(&mut self, expression: Expression<'a>) -> CompileResult<Value> {
        match expression {
            Expression::Constant(value) => Ok(Value::constant(value)),
            Expression::Variable(name) => {
                let variable = self.get_variable(name)?;
                Ok(variable.into())
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
                if predicate > 0 {
                    // Infinite loop
                    let temp = self.new_owned(1)?;
                    self.loop_while(temp.address, body)
                } else {
                    // Nothing to do here
                    Ok(())
                }
            }
            Expression::Variable(variable) => {
                let predicate = self.get_address(variable)?;
                self.loop_while(predicate, body)
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
}

/// Instruction compiling
impl<'a> BrainCrabCompiler<'a> {
    fn compile_instructions(&mut self, instructions: Vec<Instruction<'a>>) -> CompileResult<()> {
        for instruction in instructions {
            match instruction {
                Instruction::Define { name, value } => {
                    let value = self.eval_expression(value)?;
                    self.new_variable(name, value)?;
                }
                Instruction::Assign { name, value } => {
                    let destination = self.get_address(name)?;
                    let value = self.eval_expression(value)?;
                    self.assign(destination, value)?;
                }
                Instruction::AddAssign { name, value } => {
                    let destination = self.get_address(name)?;
                    let value = self.eval_expression(value)?;
                    self.add_assign(destination, value)?;
                }
                Instruction::SubAssign { name, value } => {
                    let destination = self.get_address(name)?;
                    let value = self.eval_expression(value)?;
                    self.sub_assign(destination, value)?;
                }
                Instruction::Write { name } => {
                    let address = self.get_address(name)?;
                    self.move_pointer_to(address);
                    self.write_current();
                }
                Instruction::Read { name } => {
                    let address = self.get_address(name)?;
                    self.move_pointer_to(address);
                    self.zero(address);
                    self.read_current();
                }
                Instruction::Print { string } => {
                    self.print_string(&string)?;
                }
                Instruction::While { predicate, body } => {
                    self.loop_while_expression(predicate, |compiler| {
                        compiler.compile_instructions(body)
                    })?;
                }
                Instruction::Scope { body } => {
                    self.scoped(|compiler| compiler.compile_instructions(body))?;
                }
                Instruction::IfThenElse {
                    predicate,
                    if_body,
                    else_body,
                } => {
                    let predicate = self.eval_expression(predicate)?;
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
