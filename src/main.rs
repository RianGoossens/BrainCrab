mod cli;

use std::{cell::RefCell, collections::HashMap, io, rc::Rc};

use bf_core::{BFInterpreter, BFProgram, BFTree};
use bf_macros::bf;
use clap::Parser;
use cli::Cli;

#[derive(Clone, Copy)]
pub enum Value<'a> {
    Cell(u16),
    Literal(u8),
    Variable(&'a str),
}

pub enum Instruction<'a> {
    Define { name: &'a str, value: Value<'a> },
    Copy { name: &'a str, value: &'a str },
    Write { name: &'a str },
    Read { name: &'a str },
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

            match value {
                Value::Cell(address) => self.copy_cell(address, &[address])?,
                Value::Literal(value) => {
                    self.move_pointer_to(address);
                    self.program().push(BFTree::Add(value));
                }
                Value::Variable(name) => {
                    let address = self.get_variable(name)?;
                    self.copy_cell(address, &[address])?;
                }
            }
            Ok(address)
        }
    }

    pub fn get_variable(&mut self, name: &'a str) -> CompileResult<u16> {
        if let Some(address) = self.variable_map.get(name) {
            Ok(*address)
        } else {
            Err(CompilerError::UndefinedVariable(name.into()))
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
        self.program().push(BFTree::Move(amount));
    }

    pub fn move_pointer_to(&mut self, address: u16) {
        let offset = address as i16 - self.pointer as i16;
        if offset != 0 {
            self.move_pointer(offset);
        }
    }

    pub fn inc(&mut self) {
        self.program().push(BFTree::Add(1));
    }

    pub fn dec(&mut self) {
        self.program().push(BFTree::Add(255));
    }

    pub fn write(&mut self) {
        self.program().push(BFTree::Write);
    }

    pub fn read(&mut self) {
        self.program().push(BFTree::Read);
    }

    fn start_loop(&mut self) {
        self.program_stack.push(BFProgram::new());
    }

    fn end_loop(&mut self) -> CompileResult<()> {
        if self.program_stack.len() == 1 {
            Err(CompilerError::ClosingNonExistantLoop)
        } else {
            let loop_program = self.program_stack.pop().unwrap();
            self.program().push(BFTree::Loop(loop_program.0));
            Ok(())
        }
    }

    pub fn loop_until_zero<F: FnOnce(&mut Self) -> CompileResult<()>>(
        &mut self,
        f: F,
    ) -> CompileResult<()> {
        self.start_loop();
        f(self)?;
        self.end_loop()?;
        Ok(())
    }

    // Utilities

    pub fn zero(&mut self) {
        self.program().append(bf!("[-]"));
    }

    pub fn set(&mut self, value: u8) {
        self.zero();
        self.program().push(BFTree::Add(value));
    }

    pub fn move_cell(&mut self, source: u16, destinations: &[u16]) {
        self.move_pointer_to(source);
        self.loop_until_zero(|builder| {
            builder.dec();
            for destination in destinations {
                builder.move_pointer_to(*destination);
                builder.inc();
            }
            builder.move_pointer_to(source);
            Ok(())
        })
        .unwrap();
    }

    pub fn copy_cell(&mut self, source: u16, destinations: &[u16]) -> CompileResult<()> {
        let temp = self.new_temp()?;
        let mut destinations = destinations.to_vec();
        destinations.push(temp.address);
        self.move_cell(source, &destinations);
        self.move_cell(temp.address, &[source]);
        Ok(())
    }
}

pub fn compile(program: &Program) -> CompileResult<BFProgram> {
    let mut builder = BFProgramBuilder::new();

    for instruction in &program.instructions {
        match instruction {
            Instruction::Define { name, value } => {
                builder.new_variable(name, *value)?;
            }
            Instruction::Copy { name, value } => {
                let destination = builder.get_variable(name)?;
                let source = builder.get_variable(value)?;
                builder.copy_cell(source, &[destination])?;
            }
            Instruction::Write { name } => {
                let address = builder.get_variable(name)?;
                builder.move_pointer_to(address);
                builder.write();
            }
            Instruction::Read { name } => {
                let address = builder.get_variable(name)?;
                builder.move_pointer_to(address);
                builder.read();
            }
        }
    }

    builder.build()
}

fn main() -> io::Result<()> {
    let program = Program {
        instructions: vec![
            Instruction::Define {
                name: "x",
                value: Value::Literal(b'H'),
            },
            Instruction::Define {
                name: "y",
                value: Value::Literal(b'i'),
            },
            Instruction::Write { name: "x" },
            Instruction::Write { name: "y" },
            Instruction::Define {
                name: "z",
                value: Value::Variable("y"),
            },
            Instruction::Write { name: "z" },
        ],
    };
    let bf_program = compile(&program).expect("could not compile program");
    println!("{}", bf_program.to_string());
    let mut interpreter = BFInterpreter::new();
    interpreter.run(&bf_program);

    /*
    let cli = Cli::parse();

    cli.start()*/
    Ok(())
}
