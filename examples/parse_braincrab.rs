use std::fs;
use std::io::Result;

use bf_core::BFInterpreter;
use braincrab::absolute_bf::ABFOptimizer;
use braincrab::compiler::BrainCrabCompiler;
use braincrab::parser::BrainCrabParser;

pub fn main() -> Result<()> {
    let script = fs::read_to_string("examples/braincrab_files/count.bc")?;
    println!("Parsing:\n\n{}\n", &script);

    let mut parser = BrainCrabParser::new();

    let parsed = parser.parse_program(&script);

    if let Err(error) = parsed {
        panic!("{error}")
    }
    let parsed = parsed.unwrap().value;

    println!("{parsed:?}");

    println!("\nCompiling:\n");

    let mut compiled_abf =
        BrainCrabCompiler::compile_abf(parsed).expect("could not compile program");

    compiled_abf = compiled_abf.without_dead_loops();
    compiled_abf.disentangle_addresses();
    compiled_abf.optimize_addresses(10000);
    let test = ABFOptimizer::analyze_abf(&compiled_abf);
    println!("{test}");

    println!("{compiled_abf:?}");
    println!("\n{}", compiled_abf.dot_dependency_graph());

    let compiled_bf = compiled_abf.to_bf();

    println!("{}", compiled_bf.to_string());
    return Ok(());

    println!("\nRunning:\n");

    let mut interpreter = BFInterpreter::new();

    interpreter.run(&compiled_bf);

    Ok(())
}
