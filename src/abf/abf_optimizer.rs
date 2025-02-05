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

pub struct ABFOptimizer;

impl ABFOptimizer {
    fn optimize_abf_impl(
        abf: &ABFProgram,
        state: &mut ABFState,
        address_map: &mut BTreeMap<u16, u16>,
        program_builder: &mut ABFProgramBuilder,
    ) {
        for instruction in &abf.instructions {
            match instruction {
                ABFInstruction::New(address, value) => {
                    state.set_value(*address, *value);
                }
                ABFInstruction::Read(address) => {
                    state.set_value(*address, ABFValue::Runtime);
                    let destination_address = program_builder.read();
                    address_map.insert(*address, destination_address);
                }
                ABFInstruction::Free(_address) => {
                    // Do nothing
                }
                ABFInstruction::Write(address) => {
                    let cell = state.get_cell_mut(*address);
                    assert!(cell.used);
                    match cell.value {
                        ABFValue::CompileTime(value) => {
                            let destination_address = program_builder.new_address(value);
                            program_builder.write(destination_address);
                        }
                        ABFValue::Runtime => {
                            let destination_address = *address_map.get(address).unwrap();
                            program_builder.write(destination_address);
                        }
                    }
                }
                ABFInstruction::Add(address, amount) => {
                    let cell = state.get_cell_mut(*address);
                    assert!(cell.used);
                    match &mut cell.value {
                        ABFValue::CompileTime(value) => {
                            *value = value.wrapping_add(*amount as u8);
                        }
                        ABFValue::Runtime => {
                            let destination_address = *address_map.get(address).unwrap();
                            program_builder.add(destination_address, *amount);
                        }
                    }
                }
                ABFInstruction::While(address, body) => {
                    let cell = state.get_cell(*address);
                    assert!(cell.used);
                    let mut new_state = state.clone();
                    let mut new_address_map = address_map.clone();
                    let mut new_program_builder = program_builder.clone();

                    let mut unrolled_successfully = false;
                    for _ in 0..10000 {
                        let cell = new_state.get_cell(*address);
                        if cell.value == ABFValue::CompileTime(0) {
                            unrolled_successfully = true;
                            break;
                        }
                        if cell.value == ABFValue::Runtime {
                            unrolled_successfully = false;
                            break;
                        }

                        Self::optimize_abf_impl(
                            body,
                            &mut new_state,
                            &mut new_address_map,
                            &mut new_program_builder,
                        );
                    }

                    if unrolled_successfully {
                        *state = new_state;
                        *address_map = new_address_map;
                        *program_builder = new_program_builder;
                    } else {
                        // Since we don't know how this loop will run, any modified addresses
                        // in this loop become unknown
                        let modified_addresses = body.modified_addresses();
                        for modified_address in &modified_addresses {
                            let cell = state.get_cell_mut(*modified_address);
                            if cell.used {
                                if let ABFValue::CompileTime(x) = cell.value {
                                    let destination_address = program_builder.new_address(x);
                                    address_map.insert(*modified_address, destination_address);
                                }
                                cell.value = ABFValue::Runtime;
                            }
                        }

                        let destination_address = *address_map.get(address).unwrap();
                        program_builder.while_loop(destination_address, |program_builder| {
                            Self::optimize_abf_impl(body, state, address_map, program_builder);
                        });

                        // We need to make sure that all modified addresses are still marked as
                        // runtime after the loop, since there is no way to guarantee if the loop
                        // will even run.
                        for modified_address in modified_addresses {
                            let cell = state.get_cell_mut(modified_address);
                            if cell.used {
                                cell.value = ABFValue::Runtime;
                            }
                        }
                    }
                    state.set_value(*address, 0);
                }
            }
        }
    }

    pub fn optimize_abf(program: &ABFProgram) -> ABFProgram {
        let mut state = ABFState::new();
        let mut address_map = BTreeMap::new();
        let mut program_builder = ABFProgramBuilder::new();
        Self::optimize_abf_impl(program, &mut state, &mut address_map, &mut program_builder);
        program_builder.program().unwrap()
    }
}
