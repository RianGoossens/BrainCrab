use std::fs;
use std::io::{self, stdin, BufRead};
use std::path::PathBuf;
use std::time::Instant;

use bf_core::{BFInterpreter, BFProgram};
use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::{Parser, Subcommand};

use crate::abf::{ABFCompiler, ABFOptimizer};
use crate::compiler::BrainCrabCompiler;
use crate::parser::BrainCrabParser;

fn get_cli_style() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default())
        .usage(AnsiColor::Green.on_default())
        .literal(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::Green.on_default())
}

#[derive(Parser)]
#[command(version, about, long_about, styles=get_cli_style())]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a BrainCrab script to Brainfuck.
    Compile {
        path: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Run a BrainCrab script as Brainfuck.
    Run { path: PathBuf },

    /// BF Commands
    #[command(subcommand)]
    BF(BFCommands),
}

#[derive(Subcommand)]
enum BFCommands {
    /// Run a Brainfuck file.
    Run { path: PathBuf },

    /// Provides an interactive environment for executing Brainfuck code line-by-line.
    ///
    /// Type any Brainfuck code directly, pressing Enter after each line.
    /// Once the code completes execution, the command prompt will return to the repl mode, allowing further inputs.
    Repl,
}

impl Cli {
    pub fn start(self) -> io::Result<()> {
        match self.command {
            Commands::Run { path } => Self::run(path),
            Commands::Compile { path, output } => Self::compile(path, output),
            Commands::BF(BFCommands::Run { path }) => Self::bf_run(path),
            Commands::BF(BFCommands::Repl) => Self::bf_repl(),
        }
    }

    fn run(path: PathBuf) -> io::Result<()> {
        let script = fs::read_to_string(&path)?;
        let mut parser = BrainCrabParser::new();
        let parse_result = parser.parse_program(&script);

        match parse_result {
            Ok(parsed) => {
                let program = parsed.value;
                let start_time = Instant::now();
                println!("Compiling ABF...");
                let compiled_abf = BrainCrabCompiler::compile_abf(program);
                match compiled_abf {
                    Ok(compiled_abf) => {
                        println!("Optimizing ABF...");
                        let mut compiled_abf = ABFOptimizer::optimize_abf(&compiled_abf);
                        compiled_abf.clear_unused_variables();
                        compiled_abf.insert_frees();

                        println!("Compiling to BF...");
                        let bf = ABFCompiler::compile_to_bf(&compiled_abf);
                        println!("Compile time: {:?}", start_time.elapsed());
                        println!("Running BF...");
                        let mut interpreter = BFInterpreter::new();
                        interpreter.run(&bf);
                    }
                    Err(error) => {
                        eprintln!("Encountered error while compiling {path:?}:");
                        eprintln!("{error:?}");
                    }
                }
            }
            Err(error) => {
                eprintln!("Encountered error while parsing {path:?}:");
                eprintln!("{error}")
            }
        }
        Ok(())
    }

    fn compile(path: PathBuf, output: Option<PathBuf>) -> io::Result<()> {
        let script = fs::read_to_string(&path)?;
        let mut parser = BrainCrabParser::new();
        let parse_result = parser.parse_program(&script);

        match parse_result {
            Ok(parsed) => {
                let program = parsed.value;
                println!("Compiling ABF...");
                let compiled_abf = BrainCrabCompiler::compile_abf(program);
                match compiled_abf {
                    Ok(compiled_abf) => {
                        println!("Optimizing ABF...");
                        let mut compiled_abf = ABFOptimizer::optimize_abf(&compiled_abf);
                        compiled_abf.clear_unused_variables();
                        compiled_abf.insert_frees();

                        println!("Compiling to BF...");
                        let bf = ABFCompiler::compile_to_bf(&compiled_abf).to_string();
                        if let Some(output_path) = output {
                            fs::write(output_path, bf)?;
                        } else {
                            println!("{bf}");
                        }
                    }
                    Err(error) => {
                        eprintln!("Encountered error while compiling {path:?}:");
                        eprintln!("{error:?}");
                    }
                }
            }
            Err(error) => {
                eprintln!("Encountered error while parsing {path:?}:");
                eprintln!("{error}")
            }
        }
        Ok(())
    }

    fn bf_run(path: PathBuf) -> io::Result<()> {
        let script = std::fs::read_to_string(path)?;
        let program = BFProgram::parse(&script).expect("Invalid program");
        let mut interpreter = BFInterpreter::new();
        interpreter.run(&program);
        Ok(())
    }

    fn bf_repl() -> io::Result<()> {
        let mut interpreter = BFInterpreter::new();
        loop {
            let mut buffer = String::new();

            {
                let mut stdin = stdin().lock();
                stdin
                    .read_line(&mut buffer)
                    .expect("Could not read line from stdin.");
            }

            match BFProgram::parse(&buffer) {
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
