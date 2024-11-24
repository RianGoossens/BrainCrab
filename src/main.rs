mod absolute_bf;
mod allocator;
mod ast;
mod cli;
mod compiler;
mod parser;
mod value;

use std::io;

use ast::{Expression, Instruction, Program};
use bf_core::BFInterpreter;
use compiler::BrainCrabCompiler;
use parser::Parser;

fn main() -> io::Result<()> {
    let _program = Program {
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
                predicate: Expression::variable("abc"),
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
    let _program = Program {
        instructions: vec![
            Instruction::Define {
                name: "x",
                value: Expression::add(Expression::constant(1), Expression::constant(2)),
            },
            Instruction::Assign {
                name: "x",
                value: Expression::add(Expression::variable("x"), Expression::variable("x")),
            },
            Instruction::WriteString {
                string: "The detected value was ",
            },
            Instruction::IfThenElse {
                predicate: Expression::variable("x"),
                if_body: vec![Instruction::WriteString { string: "true" }],
                else_body: vec![Instruction::WriteString { string: "false" }],
            },
            Instruction::WriteString { string: "!\n" },
        ],
    };
    let _program = Program {
        instructions: vec![
            Instruction::Define {
                name: "x",
                value: Expression::constant(10),
            },
            Instruction::While {
                predicate: Expression::sub(Expression::variable("x"), Expression::constant(4)),
                body: vec![
                    Instruction::WriteString {
                        string: "The detected value was ",
                    },
                    Instruction::IfThenElse {
                        predicate: Expression::or(
                            Expression::equals(Expression::variable("x"), Expression::constant(5)),
                            Expression::equals(Expression::variable("x"), Expression::constant(8)),
                        ),
                        else_body: vec![Instruction::IfThenElse {
                            predicate: Expression::sub(
                                Expression::variable("x"),
                                Expression::constant(6),
                            ),
                            if_body: vec![Instruction::IfThenElse {
                                predicate: Expression::sub(
                                    Expression::variable("x"),
                                    Expression::constant(7),
                                ),
                                if_body: vec![Instruction::WriteString { string: "OTHER" }],
                                else_body: vec![Instruction::WriteString { string: "7" }],
                            }],
                            else_body: vec![Instruction::WriteString { string: "SIX" }],
                        }],
                        if_body: vec![Instruction::WriteString {
                            string: "FIVE OR EIGHT",
                        }],
                    },
                    Instruction::SubAssign {
                        name: "x",
                        value: Expression::constant(1),
                    },
                    Instruction::WriteString { string: "!\n" },
                ],
            },
        ],
    };
    let program = Program {
        instructions: vec![
            Instruction::Define {
                name: "x",
                value: 10.into(),
            },
            Instruction::Define {
                name: "y",
                value: 5.into(),
            },
            Instruction::Define {
                name: "x<=y",
                value: 0.into(),
            },
            Instruction::Define {
                name: "loopvar",
                value: 1.into(),
            },
            Instruction::While {
                predicate: "loopvar".into(),
                body: vec![
                    Instruction::SubAssign {
                        name: "x",
                        value: 1.into(),
                    },
                    Instruction::SubAssign {
                        name: "y",
                        value: 1.into(),
                    },
                    Instruction::IfThenElse {
                        predicate: "x".into(),
                        if_body: vec![Instruction::IfThenElse {
                            predicate: "y".into(),
                            if_body: vec![],
                            else_body: vec![Instruction::SubAssign {
                                name: "loopvar",
                                value: 1.into(),
                            }],
                        }],
                        else_body: vec![
                            Instruction::AddAssign {
                                name: "x<=y",
                                value: 1.into(),
                            },
                            Instruction::SubAssign {
                                name: "loopvar",
                                value: 1.into(),
                            },
                        ],
                    },
                ],
            },
        ],
    };
    let program = Program {
        instructions: vec![
            Instruction::Define {
                name: "x",
                value: 10.into(),
            },
            Instruction::Define {
                name: "y",
                value: 15.into(),
            },
            Instruction::Define {
                name: "x<=y",
                value: Expression::less_than("x".into(), "y".into()),
            },
        ],
    };

    let mut abf_program =
        BrainCrabCompiler::compile_abf(program).expect("could not compile program");
    let bf_program = abf_program.to_bf();
    let bf_program_string = bf_program.to_string();
    println!(
        "{}\nLength:{}\n",
        bf_program_string,
        bf_program_string.len()
    );

    abf_program.optimize_addresses(1000);
    let bf_program = abf_program.to_bf();
    let bf_program_string = bf_program.to_string();
    println!("{}\nLength:{}", bf_program_string, bf_program_string.len());

    let mut interpreter = BFInterpreter::new();
    interpreter.run(&bf_program);
    println!("\n{:?}", interpreter.tape()[..10].to_owned());

    let mut parser = Parser::new("'\n'").unwrap();
    let number = parser.parse_constant().unwrap();
    println!("Parsed a number: {:?}", number);
    /*
    let cli = Cli::parse();

    cli.start()?;
    */
    Ok(())
}
