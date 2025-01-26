use braincrab::new_abf::*;

fn main() {
    let program = ABFProgram::new(vec![
        ABFInstruction::New(0, 1),
        ABFInstruction::Read(1),
        ABFInstruction::Add(1, -2),
        ABFInstruction::Read(3),
        ABFInstruction::While(
            1,
            ABFProgram::new(vec![
                ABFInstruction::New(2, 1),
                ABFInstruction::Add(1, -1),
                ABFInstruction::While(
                    2,
                    ABFProgram::new(vec![
                        ABFInstruction::Add(0, 1),
                        ABFInstruction::Add(2, -1),
                        ABFInstruction::Write(3),
                    ]),
                ),
            ]),
        ),
        ABFInstruction::Write(0),
        ABFInstruction::Write(1),
    ]);
    println!("{:}", program);
    let program = ABFOptimizer::optimize(&program);
    println!("Simplifying:\n{:}", program);
}
