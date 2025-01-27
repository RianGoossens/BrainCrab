use braincrab::new_abf::*;

fn main() {
    let mut program_builder = ABFProgramBuilder::new();
    let a = program_builder.new_address(1);
    let b = program_builder.read();
    program_builder.add(b, -2);
    let d = program_builder.read();
    program_builder.while_loop(b, |program_builder| {
        let c = program_builder.new_address(1);
        program_builder.add(b, -1);
        program_builder.while_loop(c, |program_builder: &mut ABFProgramBuilder| {
            program_builder.add(a, 1);
            program_builder.add(c, -1);
            program_builder.write(d);
        });
    });
    program_builder.write(a);
    program_builder.write(b);

    let mut program = program_builder.program();
    println!("{:}", program);
    ABFCompiler::optimize_frees(&mut program);
    println!("Adding frees:\n{:}", program);
    let program = ABFCompiler::optimize(&program);
    println!("Simplifying:\n{:}", program);
}
