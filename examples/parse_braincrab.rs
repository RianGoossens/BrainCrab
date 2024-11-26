use std::fs;
use std::io::Result;

use braincrab::parser::Parser;

pub fn main() -> Result<()> {
    let script = fs::read_to_string("examples/braincrab_files/parse_test.bc")?;
    println!("Parsing:\n\n{}\n", &script);

    let mut parser = Parser::new();

    let parsed = parser.parse_definition(&script);

    match parsed {
        Ok(value) => println!("{:#?}", value.value),
        Err(error) => eprintln!("{error}"),
    }

    Ok(())
}
