use std::io;

use braincrab::cli::Cli;
use clap::Parser;

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    cli.start()
}
