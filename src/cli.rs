use std::io::{self, stdin, BufRead};
use std::path::PathBuf;

use bf_core::{parse_bf, BFInterpreter};
use clap::builder::styling::AnsiColor;
use clap::builder::Styles;
use clap::{Parser, Subcommand};

fn get_cli_style() -> Styles {
    Styles::styled()
        .header(AnsiColor::Yellow.on_default())
        .usage(AnsiColor::Green.on_default())
        .literal(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::Green.on_default())
}

#[derive(Parser)]
#[command(version, about, long_about, styles=get_cli_style())]
pub(crate) struct Cli {
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
    pub(crate) fn start(self) -> io::Result<()> {
        match self.command {
            Commands::Run { path } => Self::run(path),
            Commands::Repl => Self::repl(),
        }
    }

    fn run(path: PathBuf) -> io::Result<()> {
        let script = std::fs::read_to_string(path)?;
        let program = parse_bf(&script).expect("Invalid program");
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

            match parse_bf(&buffer) {
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
