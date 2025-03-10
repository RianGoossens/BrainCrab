use std::fs;
use std::io::{self, stdin, BufRead};
use std::path::PathBuf;
use std::time::Instant;

use bf_core::{BFInterpreter, BFProgram};
use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OptimizeMode {
    None,
    Speed,
}

#[derive(Args)]
#[group()]
struct CompileArgs {
    #[arg(short, long, default_value = "false", default_missing_value = "true", num_args=0..=1, action=ArgAction::Set)]
    verbose: bool,
    #[arg(short, long, default_value = "speed")]
    optimize: OptimizeMode,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a BrainCrab script to Brainfuck.
    Compile {
        path: PathBuf,
        #[arg(long)]
        output: Option<PathBuf>,
        #[group(flatten)]
        compile_args: CompileArgs,
    },

    /// Run a BrainCrab script as Brainfuck.
    Run {
        path: PathBuf,
        #[group(flatten)]
        compile_args: CompileArgs,
    },

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
            Commands::Run { path, compile_args } => Self::run(path, compile_args),
            Commands::Compile {
                path,
                output,
                compile_args,
            } => Self::compile(path, output, compile_args),
            Commands::BF(BFCommands::Run { path }) => Self::bf_run(path),
            Commands::BF(BFCommands::Repl) => Self::bf_repl(),
        }
    }

    fn create_bf(path: PathBuf, compile_args: CompileArgs) -> io::Result<BFProgram> {
        let verbose = compile_args.verbose;
        let script = fs::read_to_string(&path)?;
        let mut parser = BrainCrabParser::new();
        let parse_result = parser.parse_program(&script);

        match parse_result {
            Ok(parsed) => {
                let program = parsed.value;
                let start_time = Instant::now();
                if verbose {
                    println!("Compiling ABF...");
                }
                let compiled_abf = BrainCrabCompiler::compile_abf(program);
                match compiled_abf {
                    Ok(mut compiled_abf) => {
                        if compile_args.optimize == OptimizeMode::Speed {
                            if verbose {
                                println!("Optimizing ABF...");
                            }
                            compiled_abf = ABFOptimizer::optimize_abf(&compiled_abf);
                            compiled_abf.clear_unused_variables();
                            compiled_abf.insert_frees();
                        }
                        // println!("{compiled_abf}");

                        if verbose {
                            println!("Compiling to BF...");
                        }
                        let bf = ABFCompiler::compile_to_bf(&compiled_abf);
                        if verbose {
                            println!("Compile time: {:?}", start_time.elapsed());
                            println!("Size: {:?}", bf.to_string().len());
                        }
                        Ok(bf)
                    }
                    Err(error) => {
                        eprintln!("Encountered error while compiling {path:?}:");
                        panic!("{error:?}");
                    }
                }
            }
            Err(error) => {
                eprintln!("Encountered error while parsing {path:?}:");
                panic!("{error}");
            }
        }
    }

    fn run(path: PathBuf, compile_args: CompileArgs) -> io::Result<()> {
        let verbose = compile_args.verbose;
        let bf = Self::create_bf(path, compile_args)?;
        if verbose {
            println!("Running BF...");
        }
        let mut interpreter = BFInterpreter::new();
        interpreter.run(&bf);
        Ok(())
    }

    fn compile(
        path: PathBuf,
        output: Option<PathBuf>,
        compile_args: CompileArgs,
    ) -> io::Result<()> {
        let bf = Self::create_bf(path, compile_args)?;
        let bf_string = bf.to_string();
        if let Some(output_path) = output {
            fs::write(output_path, bf_string)?;
        } else {
            println!("{bf_string}");
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
