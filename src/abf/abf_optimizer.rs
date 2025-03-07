use std::{collections::BTreeMap, mem::swap};

use super::{ABFInstruction, ABFProgram, ABFProgramBuilder};

#[derive(Debug, Clone)]
pub enum AnalyzedABFInstruction {
    New(u16, u8),
    Read(u16),
    Write(u16),
    Add(u16, i8),
    While(u16, AnalyzedABFProgram),
}

#[derive(Debug, Clone)]
pub struct AnalyzedABFProgram {
    pub instructions: Vec<AnalyzedABFInstruction>,
    pub modified_addresses: Vec<u16>,
}

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
struct ABFState {
    values: Vec<ABFValue>,
    used: Vec<bool>,
}

impl ABFState {
    fn new() -> Self {
        Self {
            values: vec![],
            used: vec![],
        }
    }

    fn is_used(&self, address: u16) -> bool {
        *self.used.get(address as usize).unwrap()
    }

    fn get_value(&self, address: u16) -> ABFValue {
        if let Some(value) = self.values.get(address as usize) {
            *value
        } else {
            0.into()
        }
    }

    fn set_value(&mut self, address: u16, value: impl Into<ABFValue>) {
        if address as usize >= self.values.len() {
            self.values.resize(address as usize + 1, 0.into());
            self.used.resize(address as usize + 1, false);
        }
        self.values[address as usize] = value.into();
        self.used[address as usize] = true;
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

    fn analyze_abf_program(program: &ABFProgram) -> AnalyzedABFProgram {
        let mut modified_addresses = vec![];
        let mut analyzed_instructions = vec![];
        for instruction in &program.instructions {
            match instruction {
                ABFInstruction::New(address, value) => {
                    modified_addresses.push(*address);
                    analyzed_instructions.push(AnalyzedABFInstruction::New(*address, *value));
                }
                ABFInstruction::Read(address) => {
                    modified_addresses.push(*address);
                    analyzed_instructions.push(AnalyzedABFInstruction::Read(*address));
                }
                ABFInstruction::Free(_) => {}
                ABFInstruction::Write(address) => {
                    analyzed_instructions.push(AnalyzedABFInstruction::Write(*address));
                }
                ABFInstruction::Add(address, value) => {
                    modified_addresses.push(*address);
                    analyzed_instructions.push(AnalyzedABFInstruction::Add(*address, *value));
                }
                ABFInstruction::While(predicate, body) => {
                    let analyzed_body = Self::analyze_abf_program(body);
                    modified_addresses.push(*predicate);
                    modified_addresses.extend_from_slice(&analyzed_body.modified_addresses);
                    analyzed_instructions
                        .push(AnalyzedABFInstruction::While(*predicate, analyzed_body));
                }
            }
        }
        modified_addresses.sort();
        modified_addresses.dedup();
        AnalyzedABFProgram {
            instructions: analyzed_instructions,
            modified_addresses,
        }
    }

    fn create_child(&self) -> Self {
        Self {
            state: self.state.clone(),
            address_map: self.address_map.clone(),
            builder: self.builder.create_child(),
        }
    }

    fn merge_child(&mut self, rhs: Self) {
        self.state = rhs.state;
        self.address_map = rhs.address_map;
        self.builder.merge_child(rhs.builder);
    }

    fn set_value(&mut self, address: u16, value: impl Into<ABFValue>) {
        self.state.set_value(address, value);
    }

    fn get_value(&self, address: u16) -> ABFValue {
        self.state.get_value(address)
    }

    fn address_is_used(&self, address: u16) -> bool {
        self.state.is_used(address)
    }

    fn set_mapped_address(&mut self, source: u16, destination: u16) {
        self.address_map.insert(source, destination);
    }

    fn get_mapped_address(&self, source: u16) -> u16 {
        *self.address_map.get(&source).unwrap()
    }

    fn create_or_reuse_mapped_address(&mut self, address: u16) -> u16 {
        if let Some(destination) = self.address_map.get(&address) {
            *destination
        } else {
            let value = self.get_value(address);
            match value {
                ABFValue::CompileTime(value) => {
                    let destination = self.builder.new_address(value);
                    self.set_mapped_address(address, destination);
                    destination
                }
                ABFValue::Runtime => panic!("Runtime value that has no mapped address: {address}"),
            }
        }
    }

    fn optimize_abf_impl(&mut self, abf: &AnalyzedABFProgram) {
        for instruction in &abf.instructions {
            match instruction {
                AnalyzedABFInstruction::New(address, value) => {
                    self.set_value(*address, *value);
                }
                AnalyzedABFInstruction::Read(address) => {
                    self.set_value(*address, ABFValue::Runtime);
                    let destination_address = self.builder.read();
                    self.set_mapped_address(*address, destination_address);
                }
                AnalyzedABFInstruction::Write(address) => {
                    let value = self.get_value(*address);
                    let destination_address = match value {
                        ABFValue::CompileTime(value) => self.builder.new_address(value),
                        ABFValue::Runtime => self.get_mapped_address(*address),
                    };
                    self.set_mapped_address(*address, destination_address);
                    self.builder.write(destination_address);
                }
                AnalyzedABFInstruction::Add(address, amount) => {
                    if let ABFValue::CompileTime(value) = self.get_value(*address) {
                        self.set_value(*address, value.wrapping_add(*amount as u8));
                    } else {
                        let destination_address = self.get_mapped_address(*address);
                        self.builder.add(destination_address, *amount);
                    }
                }
                AnalyzedABFInstruction::While(address, body) => {
                    let predicate = self.get_value(*address);
                    if predicate == ABFValue::CompileTime(0) {
                        continue;
                    }
                    let mut unrolled_successfully = false;
                    let modified_addresses = &body.modified_addresses;

                    // We first try to unroll this loop unless it's infinite or runtime dependent.
                    if modified_addresses.contains(address) && predicate != ABFValue::Runtime {
                        let mut child_optimizer = self.create_child();

                        for _ in 0..255 * 255 {
                            let predicate = child_optimizer.get_value(*address);
                            match predicate {
                                ABFValue::CompileTime(0) => {
                                    unrolled_successfully = true;
                                    break;
                                }
                                ABFValue::Runtime => {
                                    break;
                                }
                                _ => {}
                            }

                            child_optimizer.optimize_abf_impl(body);
                        }
                        if unrolled_successfully {
                            self.merge_child(child_optimizer);
                        }
                    }

                    if !unrolled_successfully {
                        // Since we don't know how this loop will run, any modified addresses
                        // in this loop that are defined outside become unknown.
                        for modified_address in modified_addresses {
                            if self.address_is_used(*modified_address) {
                                let _ = self.create_or_reuse_mapped_address(*modified_address);
                                self.set_value(*modified_address, ABFValue::Runtime);
                            }
                        }

                        let destination_address = self.create_or_reuse_mapped_address(*address);

                        let mut body_builder = self.builder.start_loop();
                        swap(&mut body_builder, &mut self.builder);
                        self.optimize_abf_impl(body);
                        swap(&mut body_builder, &mut self.builder);
                        self.builder.end_loop(destination_address, body_builder);

                        // We need to make sure that all modified addresses are still marked as
                        // runtime after the loop, since there is no way to guarantee if the loop
                        // will even run.
                        for modified_address in modified_addresses {
                            self.set_value(*modified_address, ABFValue::Runtime);
                        }
                    }
                    self.set_value(*address, 0);
                }
            }
        }
    }

    pub fn optimize_abf(program: &ABFProgram) -> ABFProgram {
        let mut optimizer = Self::new();
        let analyzed_program = Self::analyze_abf_program(program);
        optimizer.optimize_abf_impl(&analyzed_program);
        optimizer.builder.build()
    }
}
