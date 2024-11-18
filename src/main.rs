mod cli;

use std::{cell::RefCell, collections::HashMap, io, rc::Rc};

use bf_core::{BFInterpreter, BFProgram, BFTree};
use bf_macros::bf;
use clap::Parser;
use cli::Cli;

pub enum Identifier<'a> {
    Named(&'a str),
    Temp(Temp),
}

impl<'a> Identifier<'a> {
    pub fn is_temp(&self) -> bool {
        matches!(self, Identifier::Temp(_))
    }
    pub fn is_named(&self) -> bool {
        matches!(self, Identifier::Named(_))
    }
}

pub enum Value<'a> {
    Literal(u8),
    Identifier(Identifier<'a>),
}

impl<'a> Value<'a> {
    pub fn literal(value: u8) -> Self {
        Self::Literal(value)
    }

    pub fn named(name: &'a str) -> Self {
        Self::Identifier(Identifier::Named(name))
    }

    pub fn temp(temp: Temp) -> Self {
        Self::Identifier(Identifier::Temp(temp))
    }
}

pub enum Expression<'a> {
    Value(Value<'a>),
    Add(Box<Expression<'a>>, Box<Expression<'a>>),
}

pub enum Instruction<'a> {
    Define { name: &'a str, value: Value<'a> },
    Assign { name: &'a str, value: Value<'a> },
    AddAssign { name: &'a str, value: Value<'a> },
    SubAssign { name: &'a str, value: Value<'a> },
    Write { name: &'a str },
    Read { name: &'a str },
    WriteString { string: &'a str },
}

pub struct Program<'a> {
    pub instructions: Vec<Instruction<'a>>,
}

#[derive(Debug)]
pub enum CompilerError {
    UndefinedVariable(String),
    AlreadyDefinedVariable(String),
    NoFreeAddresses,
    ClosingNonExistantLoop,
    UnclosedLoop,
    NonAsciiString(String),
}

pub struct Temp {
    pub address: u16,
    address_pool: AddressPool,
}

impl Drop for Temp {
    fn drop(&mut self) {
        self.address_pool.borrow_mut().push(self.address);
    }
}

pub type AddressPool = Rc<RefCell<Vec<u16>>>;

pub struct BFProgramBuilder<'a> {
    pub program_stack: Vec<BFProgram>,
    pub variable_map: HashMap<&'a str, u16>,
    pub address_pool: AddressPool,
    pub pointer: u16,
}

impl<'a> Default for BFProgramBuilder<'a> {
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

pub type CompileResult<A> = Result<A, CompilerError>;

impl<'a> BFProgramBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn program(&mut self) -> &mut BFProgram {
        self.program_stack.last_mut().unwrap()
    }

    pub fn push_instruction(&mut self, instruction: BFTree) {
        self.program().push_instruction(instruction);
    }

    pub fn build(mut self) -> CompileResult<BFProgram> {
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
        if self.variable_map.contains_key(name) {
            Err(CompilerError::AlreadyDefinedVariable(name.into()))
        } else {
            let address = self.new_address()?;
            self.variable_map.insert(name, address);

            self.n_times(value, |builder| {
                builder.move_pointer_to(address);
                builder.inc_current();
                Ok(())
            })?;
            Ok(address)
        }
    }

    pub fn get_variable(&self, name: &'a str) -> CompileResult<u16> {
        if let Some(address) = self.variable_map.get(name) {
            Ok(*address)
        } else {
            Err(CompilerError::UndefinedVariable(name.into()))
        }
    }

    pub fn identifier_address(&self, identifier: &Identifier<'a>) -> CompileResult<u16> {
        match identifier {
            Identifier::Named(name) => self.get_variable(name),
            Identifier::Temp(temp) => Ok(temp.address),
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

    fn start_loop(&mut self) {
        self.program_stack.push(BFProgram::new());
    }

    fn end_loop(&mut self) -> CompileResult<()> {
        if self.program_stack.len() == 1 {
            Err(CompilerError::ClosingNonExistantLoop)
        } else {
            let loop_program = self.program_stack.pop().unwrap();
            self.program()
                .push_instruction(BFTree::Loop(loop_program.0));
            Ok(())
        }
    }

    pub fn loop_while<F: FnOnce(&mut Self) -> CompileResult<()>>(
        &mut self,
        predicate: u16,
        f: F,
    ) -> CompileResult<()> {
        self.move_pointer_to(predicate);
        self.start_loop();
        f(self)?;
        self.move_pointer_to(predicate);
        self.end_loop()?;
        Ok(())
    }

    // Utilities

    pub fn n_times<F: Fn(&mut Self) -> CompileResult<()>>(
        &mut self,
        n: Value<'a>,
        f: F,
    ) -> CompileResult<()> {
        match n {
            Value::Literal(n) => {
                for _ in 0..n {
                    f(self)?;
                }
            }
            Value::Identifier(identifier) => match identifier {
                Identifier::Named(name) => {
                    let address = self.get_variable(name)?;
                    let temp = self.new_temp()?;
                    self.loop_while(address, |builder| {
                        builder.dec_current();
                        builder.move_pointer_to(temp.address);
                        builder.inc_current();
                        f(builder)?;
                        Ok(())
                    })?;
                    self.loop_while(temp.address, |builder| {
                        builder.dec_current();
                        builder.move_pointer_to(address);
                        builder.inc_current();
                        Ok(())
                    })?;
                }
                Identifier::Temp(temp) => {
                    self.loop_while(temp.address, |builder| {
                        builder.dec_current();
                        f(builder)?;
                        Ok(())
                    })?;
                }
            },
        }
        Ok(())
    }

    pub fn add_assign(&mut self, destination: u16, value: Value<'a>) -> CompileResult<()> {
        if let Value::Identifier(identifier) = &value {
            let value_address = self.identifier_address(identifier)?;
            if value_address == destination {
                assert!(!identifier.is_temp(), "Attempting to add a temp onto itself, which is not allowed as it's already consumed");
                let temp = self.new_temp()?;
                self.copy_on_top_of_cells(value, &[temp.address])?;
                self.copy_on_top_of_cells(Value::temp(temp), &[value_address])?;
                return Ok(());
            }
        }
        self.copy_on_top_of_cells(value, &[destination])
    }

    pub fn sub_assign(&mut self, destination: u16, value: Value<'a>) -> CompileResult<()> {
        if let Value::Identifier(identifier) = &value {
            let value_address = self.identifier_address(identifier)?;
            if value_address == destination {
                assert!(!identifier.is_temp(), "Attempting to sub a temp from itself, which is not allowed as it's already consumed");
                self.zero(destination);
                return Ok(());
            }
        }
        self.n_times(value, |builder| {
            builder.move_pointer_to(destination);
            builder.dec_current();
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
        self.n_times(source, |builder| {
            for destination in destinations {
                builder.move_pointer_to(*destination);
                builder.inc_current();
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
                self.add_assign(temp.address, Value::literal(offset))?;
                self.write_current();
                current_value = new_value;
            }
            self.sub_assign(temp.address, Value::literal(current_value))?;

            Ok(())
        } else {
            Err(CompilerError::NonAsciiString(string.to_owned()))
        }
    }

    // Expressions

    pub fn zero_if_temp(&mut self, value: &Value<'a>) {
        if let Value::Identifier(Identifier::Temp(temp)) = value {
            self.zero(temp.address);
        }
    }

    pub fn eval_expression(&mut self, expression: Expression<'a>) -> CompileResult<Value<'a>> {
        match expression {
            Expression::Value(value) => Ok(value),
            Expression::Add(a, b) => {
                let a = self.eval_expression(*a)?;
                let b = self.eval_expression(*b)?;
                match (&a, &b) {
                    (Value::Literal(a), Value::Literal(b)) => {
                        Ok(Value::Literal(a.wrapping_add(*b)))
                    }
                    (_, _) => {
                        let temp = self.new_temp()?;
                        self.add_assign(temp.address, a)?;
                        self.add_assign(temp.address, b)?;

                        Ok(Value::temp(temp))
                    }
                }
            }
        }
    }
}

/// Instruction compiling
impl<'a> BFProgramBuilder<'a> {
    pub fn compile(&mut self, program: Program<'a>) -> CompileResult<()> {
        for instruction in program.instructions {
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
            }
        }
        Ok(())
    }
}

pub fn compile(program: Program) -> CompileResult<BFProgram> {
    let mut builder = BFProgramBuilder::new();
    builder.compile(program)?;
    builder.build()
}

fn main() -> io::Result<()> {
    let program = Program {
        instructions: vec![
            Instruction::WriteString {
                string: "Hello World!\n",
            },
            Instruction::Define {
                name: "x",
                value: Value::literal(b'H'),
            },
            Instruction::Define {
                name: "y",
                value: Value::literal(b'i'),
            },
            Instruction::Write { name: "x" },
            Instruction::Write { name: "y" },
            Instruction::Define {
                name: "z",
                value: Value::named("y"),
            },
            Instruction::AddAssign {
                name: "z",
                value: Value::named("z"),
            },
            Instruction::SubAssign {
                name: "z",
                value: Value::Literal(0),
            },
            Instruction::Write { name: "z" },
        ],
    };
    let bf_program = compile(program).expect("could not compile program");
    println!("{}", bf_program.to_string());
    let mut interpreter = BFInterpreter::new();
    interpreter.run(&bf_program);
    println!("\n{:?}", interpreter.tape()[..10].to_owned());

    /*
    let cli = Cli::parse();

    cli.start()?;
    */
    Ok(())
}
