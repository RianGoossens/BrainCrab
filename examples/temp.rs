use std::io;

use bf_core::BFInterpreter;
use braincrab::ast::{Expression, Instruction, Program};
use braincrab::compiler::BrainCrabCompiler;
use braincrab::parser::BrainCrabParser;

fn main() -> io::Result<()> {
    let _program = Program {
        instructions: vec![
            Instruction::Print {
                string: "Hello World!\n".into(),
            },
            Instruction::Define {
                name: "x",
                mutable: true,
                value: Expression::constant(b'H'),
            },
            Instruction::Define {
                name: "y",
                mutable: true,
                value: Expression::constant(b'i'),
            },
            Instruction::Write { name: "x" },
            Instruction::Write { name: "y" },
            Instruction::Define {
                name: "z",
                mutable: true,
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
                mutable: true,
                value: Expression::constant(128),
            },
            Instruction::Define {
                name: "lol",
                mutable: true,
                value: Expression::constant(b'X'),
            },
            Instruction::While {
                predicate: Expression::variable("abc"),
                body: vec![
                    Instruction::Define {
                        name: "lol",
                        mutable: true,
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
                mutable: true,
                value: Expression::new_add(Expression::constant(1), Expression::constant(2)),
            },
            Instruction::Assign {
                name: "x",
                value: Expression::new_add(Expression::variable("x"), Expression::variable("x")),
            },
            Instruction::Print {
                string: "The detected value was ".into(),
            },
            Instruction::IfThenElse {
                predicate: Expression::variable("x"),
                if_body: vec![Instruction::Print {
                    string: "true".into(),
                }],
                else_body: vec![Instruction::Print {
                    string: "false".into(),
                }],
            },
            Instruction::Print {
                string: "!\n".into(),
            },
        ],
    };
    let _program = Program {
        instructions: vec![
            Instruction::Define {
                name: "x",
                mutable: true,
                value: Expression::constant(10),
            },
            Instruction::While {
                predicate: Expression::new_sub(Expression::variable("x"), Expression::constant(4)),
                body: vec![
                    Instruction::Print {
                        string: "The detected value was ".into(),
                    },
                    Instruction::IfThenElse {
                        predicate: Expression::new_or(
                            Expression::new_equals(
                                Expression::variable("x"),
                                Expression::constant(5),
                            ),
                            Expression::new_equals(
                                Expression::variable("x"),
                                Expression::constant(8),
                            ),
                        ),
                        else_body: vec![Instruction::IfThenElse {
                            predicate: Expression::new_sub(
                                Expression::variable("x"),
                                Expression::constant(6),
                            ),
                            if_body: vec![Instruction::IfThenElse {
                                predicate: Expression::new_sub(
                                    Expression::variable("x"),
                                    Expression::constant(7),
                                ),
                                if_body: vec![Instruction::Print {
                                    string: "OTHER".into(),
                                }],
                                else_body: vec![Instruction::Print { string: "7".into() }],
                            }],
                            else_body: vec![Instruction::Print {
                                string: "SIX".into(),
                            }],
                        }],
                        if_body: vec![Instruction::Print {
                            string: "FIVE OR EIGHT".into(),
                        }],
                    },
                    Instruction::SubAssign {
                        name: "x",
                        value: Expression::constant(1),
                    },
                    Instruction::Print {
                        string: "!\n".into(),
                    },
                ],
            },
        ],
    };
    let _program = Program {
        instructions: vec![
            Instruction::Define {
                name: "x",
                mutable: true,
                value: 10.into(),
            },
            Instruction::Define {
                name: "y",
                mutable: true,
                value: 5.into(),
            },
            Instruction::Define {
                name: "x<=y",
                mutable: true,
                value: 0.into(),
            },
            Instruction::Define {
                name: "loopvar",
                mutable: true,
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
                mutable: true,
                value: 10.into(),
            },
            Instruction::Define {
                name: "y",
                mutable: true,
                value: 15.into(),
            },
            Instruction::Define {
                name: "x<=y",
                mutable: true,
                value: Expression::new_less_than("x".into(), "y".into()),
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

    let mut parser = BrainCrabParser::new();
    let number = parser.parse_definition("let this_is_a_variable = (!('X' + 3 == 2) - 2 < 3);");
    match number {
        Ok(number) => println!("Parsed: {:#?} {}", number.value, number.span),
        Err(error) => println!("Error: {error}"),
    }
    /*
    let cli = Cli::parse();

    cli.start()?;
    */
    Ok(())
}
