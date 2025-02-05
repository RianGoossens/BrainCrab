use std::fs;
use std::io::Result;

use bf_core::BFInterpreter;
use braincrab::abf::ABFCompiler;
use braincrab::compiler::BrainCrabCompiler;
use braincrab::parser::BrainCrabParser;

pub fn main() -> Result<()> {
    let script = fs::read_to_string("examples/braincrab_files/number_testing.bc")?;
    println!("Parsing:\n\n{}\n", &script);

    let mut parser = BrainCrabParser::new();

    let parsed = parser.parse_program(&script);

    if let Err(error) = parsed {
        panic!("{error}")
    }
    let parsed = parsed.unwrap().value;

    println!("{parsed:?}");

    println!("\nCompiling:\n");

    let compiled_abf = BrainCrabCompiler::compile_abf(parsed).expect("could not compile program");

    //compiled_abf.optimize_addresses(10000);

    let compiled_bf = ABFCompiler::compile_to_bf(&compiled_abf);

    println!("{}", compiled_bf.to_string());

    println!("\nRunning:\n");

    let mut interpreter = BFInterpreter::new();

    interpreter.run(&compiled_bf);

    println!("program length: {}", compiled_bf.to_string().len());

    Ok(())
}
