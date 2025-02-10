use super::{ABFInstruction, ABFProgram};

#[derive(Clone)]
pub struct ABFProgramBuilder {
    program: ABFProgram,
    value_counter: u16,
}

impl ABFProgramBuilder {
    pub fn new() -> Self {
        Self {
            program: ABFProgram::new(vec![]),
            value_counter: 0,
        }
    }

    pub fn build(self) -> ABFProgram {
        self.program
    }

    fn add_instruction(&mut self, instruction: ABFInstruction) {
        self.program.add_instruction(instruction);
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

    pub fn create_child(&self) -> Self {
        Self {
            program: ABFProgram::new(vec![]),
            value_counter: self.value_counter,
        }
    }

    pub fn merge_child(&mut self, rhs: Self) {
        self.value_counter = rhs.value_counter;
        self.program.merge(rhs.build());
    }

    pub fn start_loop(&mut self) -> Self {
        self.create_child()
    }

    pub fn end_loop(&mut self, address: u16, body_builder: ABFProgramBuilder) {
        self.value_counter = body_builder.value_counter;
        self.add_instruction(ABFInstruction::While(address, body_builder.program));
    }

    pub fn while_loop(&mut self, address: u16, body_function: impl FnOnce(&mut ABFProgramBuilder)) {
        let mut body_builder = self.start_loop();
        body_function(&mut body_builder);
        self.end_loop(address, body_builder);
    }

    // Utility functions
    pub fn zero(&mut self, address: u16) {
        self.while_loop(address, |builder| {
            builder.add(address, -1);
        });
    }
}

impl Default for ABFProgramBuilder {
    fn default() -> Self {
        Self::new()
    }
}
