use std::{cell::RefCell, collections::HashMap, rc::Rc};

use bf_core::{BFProgram, BFTree};
use bf_macros::bf;

use crate::{
    ast::{Expression, Instruction, Program},
    value::{Temp, Value, Variable},
};

pub type AddressPool = Rc<RefCell<Vec<u16>>>;

#[derive(Debug)]
pub enum CompilerError {
    UndefinedVariable(String),
    AlreadyDefinedVariable(String),
    NoFreeAddresses,
    ClosingNonExistantLoop,
    UnclosedLoop,
    NonAsciiString(String),
}

pub type CompileResult<A> = Result<A, CompilerError>;

pub struct ScopedVariableMap<'a> {
    pub variable_map_stack: Vec<HashMap<&'a str, u16>>,
}

impl<'a> Default for ScopedVariableMap<'a> {
    fn default() -> Self {
        Self {
            variable_map_stack: vec![HashMap::new()],
        }
    }
}

impl<'a> ScopedVariableMap<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_address(&self, name: &'a str) -> Option<u16> {
        for variable_map in self.variable_map_stack.iter().rev() {
            if let Some(result) = variable_map.get(name) {
                return Some(*result);
            }
        }
        None
    }

    pub fn defined_in_current_scope(&mut self, name: &'a str) -> bool {
        self.variable_map_stack.last().unwrap().contains_key(name)
    }

    pub fn register(&mut self, name: &'a str, value: u16) {
        self.variable_map_stack
            .last_mut()
            .unwrap()
            .insert(name, value);
    }

    pub fn start_scope(&mut self) {
        self.variable_map_stack.push(HashMap::new());
    }

    pub fn end_scope(&mut self) -> Vec<u16> {
        let last_variable_map = self.variable_map_stack.pop().unwrap();
        last_variable_map.into_values().collect()
    }
}

pub struct BrainCrabCompiler<'a> {
    pub program_stack: Vec<BFProgram>,
    pub variable_map: ScopedVariableMap<'a>,
    pub address_pool: AddressPool,
    pub pointer: u16,
}

impl<'a> Default for BrainCrabCompiler<'a> {
    fn default() -> Self {
        let mut free_addresses = vec![];
        for x in (0..30000).rev() {
            free_addresses.push(x);
        }
        Self {
            program_stack: vec![BFProgram::new()],
            variable_map: Default::default(),
            address_pool: Rc::new(RefCell::new(free_addresses)),
            pointer: 0,
        }
    }
}

impl<'a> BrainCrabCompiler<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn program(&mut self) -> &mut BFProgram {
        self.program_stack.last_mut().unwrap()
    }

    pub fn push_instruction(&mut self, instruction: BFTree) {
        self.program().push_instruction(instruction);
    }

    pub fn get_result(mut self) -> CompileResult<BFProgram> {
        if self.program_stack.len() != 1 {
            Err(CompilerError::UnclosedLoop)
        } else {
            Ok(self.program_stack.pop().unwrap())
        }
    }

    // Memory management

    pub fn new_address(&mut self) -> CompileResult<u16> {
        if let Some(address) = self.address_pool.borrow_mut().pop() {
            Ok(address)
        } else {
            Err(CompilerError::NoFreeAddresses)
        }
    }

    pub fn free_address(&mut self, address: u16) {
        self.address_pool.borrow_mut().push(address);
    }

    pub fn new_variable(&mut self, name: &'a str, value: Value<'a>) -> CompileResult<u16> {
        if self.variable_map.defined_in_current_scope(name) {
            Err(CompilerError::AlreadyDefinedVariable(name.into()))
        } else {
            let address = self.new_address()?;
            self.variable_map.register(name, address);

            self.n_times(value, |compiler| {
                compiler.move_pointer_to(address);
                compiler.inc_current();
                Ok(())
            })?;
            Ok(address)
        }
    }

    pub fn get_variable(&self, name: &'a str) -> CompileResult<u16> {
        if let Some(address) = self.variable_map.get_address(name) {
            Ok(address)
        } else {
            Err(CompilerError::UndefinedVariable(name.into()))
        }
    }

    pub fn get_address(&self, variable: &Variable<'a>) -> CompileResult<u16> {
        match variable {
            Variable::Named(name) => self.get_variable(name),
            Variable::Borrow(address) => Ok(*address),
            Variable::Temp(temp) => Ok(temp.address),
        }
    }

    pub fn new_temp(&mut self) -> CompileResult<Temp> {
        let address = self.new_address()?;
        Ok(Temp {
            address,
            address_pool: self.address_pool.clone(),
        })
    }

    // Primitives

    pub fn move_pointer(&mut self, amount: i16) {
        self.pointer = ((self.pointer as i16) + amount) as u16;
        self.push_instruction(BFTree::Move(amount));
    }

    pub fn move_pointer_to(&mut self, address: u16) {
        let offset = address as i16 - self.pointer as i16;
        if offset != 0 {
            self.move_pointer(offset);
        }
    }

    pub fn inc_current(&mut self) {
        self.push_instruction(BFTree::Add(1));
    }

    pub fn dec_current(&mut self) {
        self.push_instruction(BFTree::Add(255));
    }

    pub fn write_current(&mut self) {
        self.push_instruction(BFTree::Write);
    }

    pub fn read_current(&mut self) {
        self.push_instruction(BFTree::Read);
    }

    pub fn scoped<F: FnOnce(&mut Self) -> CompileResult<()>>(&mut self, f: F) -> CompileResult<()> {
        self.variable_map.start_scope();
        f(self)?;
        let last_pointer_position = self.pointer;
        let scope = self.variable_map.end_scope();
        for address in scope {
            self.zero(address);
            self.free_address(address);
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

        self.program_stack.push(BFProgram::new());
        self.scoped(|compiler| {
            f(compiler)?;
            compiler.move_pointer_to(predicate);
            Ok(())
        })?;

        let loop_program = self.program_stack.pop().unwrap();
        self.push_instruction(BFTree::Loop(loop_program.0));
        Ok(())
    }

    // Utilities
    pub fn if_then_else<
        I: FnOnce(&mut Self) -> CompileResult<()>,
        E: FnOnce(&mut Self) -> CompileResult<()>,
    >(
        &mut self,
        predicate: u16,
        if_case: I,
        else_case: E,
    ) -> CompileResult<()> {
        let else_check = self.new_temp()?;
        self.add_assign(else_check.address, Value::constant(1))?;
        let if_check = self.new_temp()?;
        self.add_assign(if_check.address, Value::borrow(predicate))?;
        self.loop_while(if_check.address, |compiler| {
            if_case(compiler)?;
            compiler.sub_assign(else_check.address, Value::constant(1))?;
            compiler.zero(if_check.address);
            Ok(())
        })?;
        self.loop_while(else_check.address, |compiler| {
            else_case(compiler)?;
            compiler.sub_assign(else_check.address, Value::constant(1))?;
            Ok(())
        })?;

        Ok(())
    }

    pub fn n_times<F: Fn(&mut Self) -> CompileResult<()>>(
        &mut self,
        n: Value<'a>,
        f: F,
    ) -> CompileResult<()> {
        match n {
            Value::Constant(n) => {
                for _ in 0..n {
                    self.scoped(|compiler| f(compiler))?
                }
            }
            Value::Variable(variable) => match variable {
                Variable::Temp(temp) => {
                    self.loop_while(temp.address, |compiler| {
                        compiler.dec_current();
                        f(compiler)?;
                        Ok(())
                    })?;
                }
                _ => {
                    let address = self.get_address(&variable)?;
                    let temp = self.new_temp()?;
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

    pub fn add_assign(&mut self, destination: u16, value: Value<'a>) -> CompileResult<()> {
        if let Value::Variable(variable) = &value {
            let value_address = self.get_address(variable)?;
            if value_address == destination {
                assert!(!variable.is_temp(), "Attempting to add a temp onto itself, which is not allowed as it's already consumed");
                let temp = self.new_temp()?;
                self.copy_on_top_of_cells(value, &[temp.address])?;
                self.copy_on_top_of_cells(Value::temp(temp), &[value_address])?;
                return Ok(());
            }
        }
        self.copy_on_top_of_cells(value, &[destination])
    }

    pub fn sub_assign(&mut self, destination: u16, value: Value<'a>) -> CompileResult<()> {
        if let Value::Variable(variable) = &value {
            let value_address = self.get_address(variable)?;
            if value_address == destination {
                assert!(!variable.is_temp(), "Attempting to sub a temp from itself, which is not allowed as it's already consumed");
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

    pub fn zero(&mut self, address: u16) {
        self.move_pointer_to(address);
        self.program().append(bf!("[-]"));
    }

    pub fn assign(&mut self, destination: u16, value: Value<'a>) -> CompileResult<()> {
        self.zero(destination);
        self.add_assign(destination, value)?;
        Ok(())
    }

    pub fn copy_on_top_of_cells(
        &mut self,
        source: Value<'a>,
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

    pub fn write_string(&mut self, string: &str) -> CompileResult<()> {
        if string.is_ascii() {
            let temp = self.new_temp()?;
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

    pub fn zero_if_temp(&mut self, value: &Value<'a>) {
        if let Value::Variable(Variable::Temp(temp)) = value {
            self.zero(temp.address);
        }
    }

    fn eval_add(&mut self, a: Value<'a>, b: Value<'a>) -> CompileResult<Value<'a>> {
        match (a, b) {
            (Value::Constant(a), Value::Constant(b)) => Ok(Value::Constant(a.wrapping_add(b))),
            (Value::Variable(Variable::Temp(a)), b) => {
                self.add_assign(a.address, b)?;
                Ok(Value::temp(a))
            }
            (a, Value::Variable(Variable::Temp(b))) => {
                self.add_assign(b.address, a)?;
                Ok(Value::temp(b))
            }
            (a, b) => {
                let temp = self.new_temp()?;
                self.add_assign(temp.address, a)?;
                self.add_assign(temp.address, b)?;

                Ok(Value::temp(temp))
            }
        }
    }

    pub fn eval_expression(&mut self, expression: Expression<'a>) -> CompileResult<Value<'a>> {
        match expression {
            Expression::Value(value) => Ok(value),
            Expression::Add(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                self.eval_add(a, b)
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
                    self.new_variable(name, value)?;
                }
                Instruction::Assign { name, value } => {
                    let destination = self.get_variable(name)?;
                    self.assign(destination, value)?;
                }
                Instruction::AddAssign { name, value } => {
                    let destination = self.get_variable(name)?;
                    self.add_assign(destination, value)?;
                }
                Instruction::SubAssign { name, value } => {
                    let destination = self.get_variable(name)?;
                    self.sub_assign(destination, value)?;
                }
                Instruction::Write { name } => {
                    let address = self.get_variable(name)?;
                    self.move_pointer_to(address);
                    self.write_current();
                }
                Instruction::Read { name } => {
                    let address = self.get_variable(name)?;
                    self.move_pointer_to(address);
                    self.read_current();
                }
                Instruction::WriteString { string } => {
                    self.write_string(string)?;
                }
                Instruction::While { predicate, body } => {
                    let address = self.get_variable(predicate)?;
                    self.loop_while(address, |compiler| compiler.compile_instructions(body))?;
                }
                Instruction::Scope { body } => {
                    self.scoped(|compiler| compiler.compile_instructions(body))?;
                }
                Instruction::IfThenElse {
                    predicate,
                    if_body,
                    else_body,
                } => {
                    let address = self.get_variable(predicate)?;
                    self.if_then_else(
                        address,
                        |compiler| compiler.compile_instructions(if_body),
                        |compiler| compiler.compile_instructions(else_body),
                    )?;
                }
            }
        }
        Ok(())
    }
    pub fn compile(program: Program) -> CompileResult<BFProgram> {
        let mut compiler = BrainCrabCompiler::new();
        compiler.compile_instructions(program.instructions)?;
        compiler.get_result()
    }
}
