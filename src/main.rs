mod cli;

use std::{cell::RefCell, collections::HashMap, io, rc::Rc};

use ast::{Expression, Instruction, Program};
use bf_core::{BFInterpreter, BFProgram, BFTree};
use bf_macros::bf;
use clap::Parser;
use cli::Cli;
use compiler::BrainCrabCompiler;
use value::{Temp, Value, Variable};

mod ast;
mod compiler;
mod value;

fn main() -> io::Result<()> {
    let program = Program {
        instructions: vec![
            Instruction::WriteString {
                string: "Hello World!\n",
            },
            Instruction::Define {
                name: "x",
                value: Value::constant(b'H'),
            },
            Instruction::Define {
                name: "y",
                value: Value::constant(b'i'),
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
                value: Value::Constant(0),
            },
            Instruction::Write { name: "z" },
            Instruction::Define {
                name: "abc",
                value: Value::constant(128),
            },
            Instruction::Define {
                name: "lol",
                value: Value::constant(b'X'),
            },
            Instruction::While {
                predicate: "abc",
                body: vec![
                    Instruction::Define {
                        name: "lol",
                        value: Value::constant(b'Y'),
                    },
                    Instruction::Write { name: "abc" },
                    Instruction::SubAssign {
                        name: "abc",
                        value: Value::Constant(1),
                    },
                    Instruction::Write { name: "lol" },
                ],
            },
            Instruction::Write { name: "lol" },
        ],
    };
    let program = Program {
        instructions: vec![
            Instruction::Define {
                name: "x",
                value: Value::Constant(1),
            },
            Instruction::WriteString {
                string: "The detected value was ",
            },
            Instruction::IfThenElse {
                predicate: "x",
                if_body: vec![Instruction::WriteString { string: "true" }],
                else_body: vec![Instruction::WriteString { string: "false" }],
            },
            Instruction::WriteString { string: "!\n" },
        ],
    };
    let bf_program = BrainCrabCompiler::compile(program).expect("could not compile program");
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
