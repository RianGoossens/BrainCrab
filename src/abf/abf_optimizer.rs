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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ABFCell {
    pub value: ABFValue,
    pub used: bool,
}

impl ABFCell {
    pub fn new(value: impl Into<ABFValue>, used: bool) -> Self {
        Self {
            value: value.into(),
            used,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ABFState {
    pub values: Vec<ABFCell>,
}

impl ABFState {
    pub fn new() -> Self {
        Self {
            values: vec![ABFCell::new(0, false); 30000],
        }
    }

    pub fn get_cell(&mut self, address: u16) -> ABFCell {
        self.values[address as usize]
    }

    pub fn get_cell_mut(&mut self, address: u16) -> &mut ABFCell {
        self.values.get_mut(address as usize).unwrap()
    }

    pub fn set_value(&mut self, address: u16, value: impl Into<ABFValue>) {
        let cell = self.get_cell_mut(address);
        cell.value = value.into();
        cell.used = true;
    }

    pub fn free(&mut self, address: u16) {
        let cell = self.get_cell_mut(address);
        cell.used = false;
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
                    assert!(cell.used);
                    match cell.value {
                        ABFValue::CompileTime(value) => {
                            let destination_address = self.builder.new_address(value);
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
                    assert!(cell.used);
                    match &mut cell.value {
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
                    assert!(cell.used);
                    if cell.value == ABFValue::CompileTime(0) {
                        continue;
                    }
                    let mut new_optimizer = self.clone();

                    let mut unrolled_successfully = false;
                    for _ in 0..10000 {
                        let cell = new_optimizer.state.get_cell(*address);
                        if cell.value == ABFValue::CompileTime(0) {
                            unrolled_successfully = true;
                            break;
                        }
                        if cell.value == ABFValue::Runtime {
                            unrolled_successfully = false;
                            break;
                        }

                        new_optimizer.optimize_abf_impl(body);
                    }

                    if unrolled_successfully {
                        *self = new_optimizer;
                    } else {
                        // Since we don't know how this loop will run, any modified addresses
                        // in this loop become unknown
                        let modified_addresses = body.modified_addresses();
                        for modified_address in &modified_addresses {
                            let cell = self.state.get_cell(*modified_address);
                            if cell.used {
                                if let ABFValue::CompileTime(x) = cell.value {
                                    let destination_address = self.builder.new_address(x);
                                    self.set_mapped_address(*modified_address, destination_address);
                                }
                                self.state.set_value(*modified_address, ABFValue::Runtime);
                            }
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
                            if cell.used {
                                cell.value = ABFValue::Runtime;
                            }
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
