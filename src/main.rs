use std::fmt;
use std::io::{self, stdin, BufRead, Read};
use std::path::PathBuf;

use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::{Parser, Subcommand};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BFToken {
    Left,
    Right,
    Inc,
    Dec,
    Write,
    Read,
    BeginLoop,
    EndLoop,
}

pub fn tokenize_bf(text: &str) -> Vec<BFToken> {
    let mut result = vec![];
    for char in text.chars() {
        match char {
            '<' => result.push(BFToken::Left),
            '>' => result.push(BFToken::Right),
            '+' => result.push(BFToken::Inc),
            '-' => result.push(BFToken::Dec),
            '.' => result.push(BFToken::Write),
            ',' => result.push(BFToken::Read),
            '[' => result.push(BFToken::BeginLoop),
            ']' => result.push(BFToken::EndLoop),
            _ => {}
        }
    }
    result
}

#[derive(Debug, Clone)]
pub enum BFTree {
    Move(isize),
    Add(u8),
    Write,
    Read,
    Loop(Vec<BFTree>),
}

#[derive(Debug, Clone)]
pub struct BFProgram(pub Vec<BFTree>);

#[derive(Debug, Clone, Copy)]
pub enum BFParseError {
    UnmatchedBrackets,
}

impl fmt::Display for BFParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unmatched brackets")
    }
}

fn parse_bf_impl(tokens: &[BFToken], index: &mut usize) -> Vec<BFTree> {
    let mut result = vec![];

    while *index < tokens.len() {
        match tokens[*index] {
            BFToken::Left => {
                if let Some(BFTree::Move(movement)) = result.last_mut() {
                    *movement -= 1;
                } else {
                    result.push(BFTree::Move(-1));
                }
            }
            BFToken::Right => {
                if let Some(BFTree::Move(movement)) = result.last_mut() {
                    *movement += 1;
                } else {
                    result.push(BFTree::Move(1));
                }
            }
            BFToken::Inc => {
                if let Some(BFTree::Add(addition)) = result.last_mut() {
                    *addition = addition.wrapping_add(1);
                } else {
                    result.push(BFTree::Add(1));
                }
            }
            BFToken::Dec => {
                if let Some(BFTree::Add(addition)) = result.last_mut() {
                    *addition = addition.wrapping_sub(1);
                } else {
                    result.push(BFTree::Add(255));
                }
            }
            BFToken::Write => result.push(BFTree::Write),
            BFToken::Read => result.push(BFTree::Read),
            BFToken::BeginLoop => {
                *index += 1;
                let loop_body = parse_bf_impl(tokens, index);
                result.push(BFTree::Loop(loop_body));
            }
            BFToken::EndLoop => {
                break;
            }
        }
        *index += 1;
    }

    result
}

pub fn parse_bf(tokens: &[BFToken]) -> Result<BFProgram, BFParseError> {
    let mut index = 0;
    let result = parse_bf_impl(tokens, &mut index);
    if index != tokens.len() {
        Err(BFParseError::UnmatchedBrackets)
    } else {
        Ok(BFProgram(result))
    }
}

pub struct BFInterpreter {
    tape: [u8; 30000],
    pointer: usize,
}

impl Default for BFInterpreter {
    fn default() -> Self {
        Self {
            tape: [0; 30000],
            pointer: 0,
        }
    }
}

impl BFInterpreter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run_instructions(&mut self, instructions: &[BFTree]) {
        for tree in instructions {
            match tree {
                BFTree::Move(amount) => self.pointer = ((self.pointer as isize) + amount) as usize,
                BFTree::Add(amount) => {
                    self.tape[self.pointer] = self.tape[self.pointer].wrapping_add(*amount)
                }
                BFTree::Write => print!("{}", self.tape[self.pointer] as char),
                BFTree::Read => {
                    let mut byte = [0_u8];
                    {
                        let mut stdin_handle = stdin().lock();
                        stdin_handle.read_exact(&mut byte).unwrap();
                        if byte[0] == 13 {
                            stdin_handle.read_exact(&mut byte).unwrap();
                        }
                    }
                    self.tape[self.pointer] = byte[0];
                }
                BFTree::Loop(instructions) => loop {
                    if self.tape[self.pointer] == 0 {
                        break;
                    }
                    self.run_instructions(instructions);
                },
            }
        }
    }

    pub fn run(&mut self, program: &BFProgram) {
        self.run_instructions(&program.0);
    }
}

fn get_cli_style() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default())
        .usage(AnsiColor::Green.on_default())
        .literal(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::Green.on_default())
}

#[derive(Parser)]
#[command(version, about, long_about, styles=get_cli_style())]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a Brainfuck file.
    Run { path: PathBuf },

    /// Provides an interactive environment for executing Brainfuck code line-by-line.
    ///
    /// Type any Brainfuck code directly, pressing Enter after each line.
    /// Once the code completes execution, the command prompt will return to the repl mode, allowing further inputs.
    Repl,
}

impl Cli {
    fn start(self) -> io::Result<()> {
        match self.command {
            Commands::Run { path } => Self::run(path),
            Commands::Repl => Self::repl(),
        }
    }

    fn run(path: PathBuf) -> io::Result<()> {
        let script = std::fs::read_to_string(path)?;
        let tokens = tokenize_bf(&script);
        let program = parse_bf(&tokens).expect("Invalid program");
        let mut interpreter = BFInterpreter::new();
        interpreter.run(&program);
        Ok(())
    }

    fn repl() -> io::Result<()> {
        let mut interpreter = BFInterpreter::new();
        loop {
            let mut buffer = String::new();

            {
                let mut stdin = stdin().lock();
                stdin
                    .read_line(&mut buffer)
                    .expect("Could not read line from stdin.");
            }

            match parse_bf(&tokenize_bf(&buffer)) {
                Ok(program) => {
                    if program.0.is_empty() {
                        return Ok(());
                    } else {
                        interpreter.run(&program);
                        println!();
                    }
                }
                Err(error) => println!("{error}"),
            }
        }
    }
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    cli.start()
}
