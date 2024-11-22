mod cli;

use std::io;

use ast::{Expression, Instruction, Program};
use bf_core::BFInterpreter;
use compiler::BrainCrabCompiler;

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
                value: Expression::constant(b'H'),
            },
            Instruction::Define {
                name: "y",
                value: Expression::constant(b'i'),
            },
            Instruction::Write { name: "x" },
            Instruction::Write { name: "y" },
            Instruction::Define {
                name: "z",
                value: Expression::variable("y"),
            },
            Instruction::AddAssign {
                name: "z",
                value: Expression::variable("z"),
            },
            Instruction::SubAssign {
                name: "z",
                value: Expression::constant(0),
            },
            Instruction::Write { name: "z" },
            Instruction::Define {
                name: "abc",
                value: Expression::constant(128),
            },
            Instruction::Define {
                name: "lol",
                value: Expression::constant(b'X'),
            },
            Instruction::While {
                predicate: "abc",
                body: vec![
                    Instruction::Define {
                        name: "lol",
                        value: Expression::constant(b'Y'),
                    },
                    Instruction::Write { name: "abc" },
                    Instruction::SubAssign {
                        name: "abc",
                        value: Expression::constant(1),
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
                value: Expression::add(Expression::constant(1), Expression::constant(2)),
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
