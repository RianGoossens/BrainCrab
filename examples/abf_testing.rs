use bf_core::BFInterpreter;
use braincrab::abf::*;

fn main() {
    let mut program_builder = ABFProgramBuilder::new();
    let zero = program_builder.new_address(0);
    program_builder.write(zero);
    let a = program_builder.new_address(1);
    let b = program_builder.read();
    let d = program_builder.read();
    let e = program_builder.new_address(15);
    program_builder.while_loop(b, |program_builder| {
        let c = program_builder.new_address(1);
        program_builder.while_loop(c, |program_builder: &mut ABFProgramBuilder| {
            program_builder.add(b, -1);
            program_builder.add(a, 1);
            program_builder.add(c, -1);
            program_builder.add(e, 20);
            program_builder.write(d);
        });
    });
    program_builder.write(a);
    program_builder.write(b);
    let zero = program_builder.new_address(0);
    program_builder.write(zero);

    let program = program_builder.program();
    println!("{:}", program);
    let mut program = ABFOptimizer::optimize_abf(&program);
    println!("Simplifying:\n{:}", program);
    program.clear_unused_variables();
    program.optimize_frees();
    println!("Adding frees and removing unused variables:\n{:}", program);

    let bf_program = ABFCompiler::compile_to_bf(&program);

    println!("{}", bf_program.to_string());
    let mut interpreter = BFInterpreter::new();
    interpreter.run(&bf_program);
}
