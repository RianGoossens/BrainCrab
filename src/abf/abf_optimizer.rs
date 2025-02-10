use std::collections::BTreeMap;

use super::{ABFInstruction, ABFProgram, ABFProgramBuilder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ABFValue {
    Runtime,
    CompileTime(u8),
}

impl From<u8> for ABFValue {
    fn from(value: u8) -> Self {
        ABFValue::CompileTime(value)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ABFState {
    pub values: Vec<ABFValue>,
}

impl ABFState {
    pub fn new() -> Self {
        Self {
            values: vec![0.into(); 30000],
        }
    }

    pub fn get_cell(&mut self, address: u16) -> ABFValue {
        self.values[address as usize]
    }

    pub fn get_cell_mut(&mut self, address: u16) -> &mut ABFValue {
        self.values.get_mut(address as usize).unwrap()
    }

    pub fn set_value(&mut self, address: u16, value: impl Into<ABFValue>) {
        let cell = self.get_cell_mut(address);
        *cell = value.into();
    }

    pub fn free(&mut self, _address: u16) {
        //No op
    }
}

#[derive(Clone)]
pub struct ABFOptimizer {
    state: ABFState,
    address_map: BTreeMap<u16, u16>,
    builder: ABFProgramBuilder,
}

impl ABFOptimizer {
    fn new() -> Self {
        Self {
            state: ABFState::new(),
            address_map: BTreeMap::new(),
            builder: ABFProgramBuilder::new(),
        }
    }

    fn set_mapped_address(&mut self, source: u16, destination: u16) {
        self.address_map.insert(source, destination);
    }

    fn get_mapped_address(&self, source: u16) -> u16 {
        *self.address_map.get(&source).unwrap()
    }

    fn optimize_abf_impl(&mut self, abf: &ABFProgram) {
        for instruction in &abf.instructions {
            match instruction {
                ABFInstruction::New(address, value) => {
                    self.state.set_value(*address, *value);
                }
                ABFInstruction::Read(address) => {
                    self.state.set_value(*address, ABFValue::Runtime);
                    let destination_address = self.builder.read();
                    self.set_mapped_address(*address, destination_address);
                }
                ABFInstruction::Free(_address) => {
                    // Do nothing
                }
                ABFInstruction::Write(address) => {
                    let cell = self.state.get_cell_mut(*address);
                    match cell {
                        ABFValue::CompileTime(value) => {
                            let destination_address = self.builder.new_address(*value);
                            self.builder.write(destination_address);
                        }
                        ABFValue::Runtime => {
                            let destination_address = self.get_mapped_address(*address);
                            self.builder.write(destination_address);
                        }
                    }
                }
                ABFInstruction::Add(address, amount) => {
                    let cell = self.state.get_cell_mut(*address);
                    match cell {
                        ABFValue::CompileTime(value) => {
                            *value = value.wrapping_add(*amount as u8);
                        }
                        ABFValue::Runtime => {
                            let destination_address = self.get_mapped_address(*address);
                            self.builder.add(destination_address, *amount);
                        }
                    }
                }
                ABFInstruction::While(address, body) => {
                    let cell = self.state.get_cell(*address);
                    if cell == ABFValue::CompileTime(0) {
                        continue;
                    }
                    let old_optimizer = self.clone();

                    let mut unrolled_successfully = false;
                    for _ in 0..10000 {
                        let cell = self.state.get_cell(*address);
                        if cell == ABFValue::CompileTime(0) {
                            unrolled_successfully = true;
                            break;
                        }
                        if cell == ABFValue::Runtime {
                            unrolled_successfully = false;
                            break;
                        }

                        self.optimize_abf_impl(body);
                    }

                    if !unrolled_successfully {
                        *self = old_optimizer;
                        // Since we don't know how this loop will run, any modified addresses
                        // in this loop become unknown
                        let modified_addresses = body.modified_addresses();
                        for modified_address in &modified_addresses {
                            let cell = self.state.get_cell(*modified_address);
                            if let ABFValue::CompileTime(x) = cell {
                                let destination_address = self.builder.new_address(x);
                                self.set_mapped_address(*modified_address, destination_address);
                            }
                            self.state.set_value(*modified_address, ABFValue::Runtime);
                        }

                        let destination_address = self.get_mapped_address(*address);
                        self.builder.start_loop();
                        self.optimize_abf_impl(body);
                        self.builder.end_loop(destination_address);

                        // We need to make sure that all modified addresses are still marked as
                        // runtime after the loop, since there is no way to guarantee if the loop
                        // will even run.
                        for modified_address in modified_addresses {
                            let cell = self.state.get_cell_mut(modified_address);
                            *cell = ABFValue::Runtime;
                        }
                    }
                    self.state.set_value(*address, 0);
                }
            }
        }
    }

    pub fn optimize_abf(program: &ABFProgram) -> ABFProgram {
        let mut optimizer = Self::new();
        optimizer.optimize_abf_impl(program);
        optimizer.builder.program().unwrap()
    }
}
