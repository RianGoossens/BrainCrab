use std::fs;
use std::io::Result;

use bf_core::BFInterpreter;
use braincrab::compiler::BrainCrabCompiler;
use braincrab::parser::Parser;

pub fn main() -> Result<()> {
    let script = fs::read_to_string("examples/braincrab_files/parse_test.bc")?;
    println!("Parsing:\n\n{}\n", &script);

    let mut parser = Parser::new();

    let parsed = parser.parse_program(&script);

    if let Err(error) = parsed {
        panic!("{error}")
    }
    let parsed = parsed.unwrap().value;

    println!("{parsed:?}");

    println!("\nCompiling:\n");

    let compiled_abf = BrainCrabCompiler::compile_abf(parsed).expect("could not compile program");

    let compiled_bf = compiled_abf.to_bf();

    println!("{}", compiled_bf.to_string());

    println!("\nRunning:\n");

    let mut interpreter = BFInterpreter::new();

    interpreter.run(&compiled_bf);

    Ok(())
}
