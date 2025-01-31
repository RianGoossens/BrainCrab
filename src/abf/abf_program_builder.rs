use super::{ABFInstruction, ABFProgram};

#[derive(Clone)]
pub struct ABFProgramBuilder {
    program_stack: Vec<ABFProgram>,
    value_counter: u16,
}

impl ABFProgramBuilder {
    pub fn new() -> Self {
        Self {
            program_stack: vec![ABFProgram::new(vec![])],
            value_counter: 0,
        }
    }

    pub fn program(mut self) -> ABFProgram {
        assert!(self.program_stack.len() == 1);
        self.program_stack.pop().unwrap()
    }

    fn add_instruction(&mut self, instruction: ABFInstruction) {
        self.program_stack
            .last_mut()
            .unwrap()
            .add_instruction(instruction);
    }

    pub fn new_address(&mut self, value: u8) -> u16 {
        let address = self.value_counter;
        self.value_counter += 1;
        self.add_instruction(ABFInstruction::New(address, value));
        address
    }

    pub fn read(&mut self) -> u16 {
        let address = self.value_counter;
        self.value_counter += 1;
        self.add_instruction(ABFInstruction::Read(address));
        address
    }

    pub fn write(&mut self, address: u16) {
        self.add_instruction(ABFInstruction::Write(address));
    }

    pub fn add(&mut self, address: u16, amount: i8) {
        self.add_instruction(ABFInstruction::Add(address, amount));
    }

    pub fn while_loop(&mut self, address: u16, body_function: impl FnOnce(&mut ABFProgramBuilder)) {
        self.program_stack.push(ABFProgram::new(vec![]));
        body_function(self);
        let body = self.program_stack.pop().unwrap();

        self.add_instruction(ABFInstruction::While(address, body));
    }
}

impl Default for ABFProgramBuilder {
    fn default() -> Self {
        Self::new()
    }
}
