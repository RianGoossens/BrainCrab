mod cli;

use std::io;

use clap::Parser;
use cli::Cli;

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    cli.start()
}
