use std::collections::BTreeMap;

use bf_core::{BFProgram, BFTree};

use crate::abf::ABFInstruction;

use super::ABFProgram;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BFValue {
    Runtime,
    CompileTime(u8),
}

impl From<u8> for BFValue {
    fn from(value: u8) -> Self {
        BFValue::CompileTime(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BFCell {
    value: BFValue,
    used: bool,
}

impl BFCell {
    pub fn new(value: impl Into<BFValue>, used: bool) -> Self {
        Self {
            value: value.into(),
            used,
        }
    }
}

/// This `Builder` can build BF programs using only absolute positioning.
/// It tracks positions itself and allows efficient and automatic reuse of values.
struct BFProgramBuilder {
    program_stack: Vec<BFProgram>,
    current_position: u16,
}

impl BFProgramBuilder {
    fn new() -> Self {
        Self {
            program_stack: vec![BFProgram::new()],
            current_position: 0,
        }
    }

    fn current_program(&mut self) -> &mut BFProgram {
        self.program_stack.last_mut().unwrap()
    }

    fn build_program(self) -> BFProgram {
        let mut program_stack = self.program_stack;
        assert!(program_stack.len() == 1);
        program_stack.pop().unwrap()
    }

    fn in_loop(&self) -> bool {
        self.program_stack.len() > 1
    }

    fn add_instruction(&mut self, instruction: BFTree) {
        self.current_program().push_instruction(instruction);
    }

    fn move_to(&mut self, new_position: u16) {
        let offset = new_position as i16 - self.current_position as i16;
        self.add_instruction(BFTree::Move(offset));
        self.current_position = new_position;
    }

    fn zero(&mut self) {
        self.add_instruction(BFTree::Loop(vec![BFTree::Add(255)]));
    }

    fn add(&mut self, amount: u8) {
        self.add_instruction(BFTree::Add(amount));
    }

    fn read(&mut self) {
        self.add_instruction(BFTree::Read);
    }

    fn write(&mut self) {
        self.add_instruction(BFTree::Write);
    }

    fn while_loop(&mut self, address: u16, body_function: impl FnOnce(&mut BFProgramBuilder)) {
        self.move_to(address);
        self.program_stack.push(BFProgram::new());
        body_function(self);
        self.move_to(address);
        let body = self.program_stack.pop().unwrap();
        self.add_instruction(BFTree::Loop(body.0));
    }
}

impl Default for BFProgramBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ABFCompiler {
    address_map: BTreeMap<u16, u16>,
    cells: Vec<BFCell>,
    current_position: u16,
}

impl ABFCompiler {
    fn new() -> Self {
        Self {
            address_map: BTreeMap::new(),
            cells: vec![BFCell::new(0, false); 30000],
            current_position: 0,
        }
    }

    fn find_address(&mut self, expected: Option<u8>) -> u16 {
        let mut best_address = u16::MAX;
        let mut best_distance = u16::MAX;
        for (i, cell) in self.cells.iter().enumerate() {
            if !cell.used {
                let address_distance = self.current_position.abs_diff(i as u16);
                let value_distance = if let Some(expected) = expected {
                    if let BFValue::CompileTime(actual) = cell.value {
                        actual.abs_diff(expected)
                    } else {
                        255
                    }
                } else {
                    0
                };
                let distance = address_distance + value_distance as u16;
                if distance < best_distance {
                    best_address = i as u16;
                    best_distance = distance;
                }
                if best_distance == 0 {
                    break;
                }
            }
        }
        best_address
    }

    fn get_value(&self, index: u16) -> BFValue {
        self.cells[index as usize].value
    }

    fn get_cell_mut(&mut self, index: u16) -> &mut BFCell {
        &mut self.cells[index as usize]
    }

    fn set_value(&mut self, index: u16, value: impl Into<BFValue>) {
        let cell = self.get_cell_mut(index);
        cell.used = true;
        cell.value = value.into();
        self.current_position = index;
    }

    fn free(&mut self, index: u16) {
        let cell = self.get_cell_mut(index);
        cell.used = false;
    }

    pub fn compile_to_bf(program: &ABFProgram) -> BFProgram {
        fn compile_impl(
            compiler: &mut ABFCompiler,
            program: &ABFProgram,
            builder: &mut BFProgramBuilder,
        ) {
            for instruction in &program.instructions {
                match instruction {
                    ABFInstruction::New(address, value) => {
                        let expected_value = if builder.in_loop() {
                            None
                        } else {
                            Some(*value)
                        };
                        let bf_address = compiler.find_address(expected_value);
                        compiler.address_map.insert(*address, bf_address);

                        builder.move_to(bf_address);
                        if !builder.in_loop() {
                            let current_value = compiler.get_value(bf_address);
                            if let BFValue::CompileTime(current_value) = current_value {
                                let value_offset = value.wrapping_sub(current_value);
                                builder.add(value_offset);
                            } else {
                                builder.zero();
                            }
                        } else {
                            builder.zero();
                        }
                        compiler.set_value(bf_address, *value);
                    }
                    ABFInstruction::Read(address) => {
                        let bf_address = compiler.find_address(None);
                        compiler.address_map.insert(*address, bf_address);

                        builder.move_to(bf_address);
                        builder.read();
                        compiler.set_value(bf_address, BFValue::Runtime);
                    }
                    ABFInstruction::Free(address) => {
                        let bf_address = *compiler.address_map.get(address).unwrap();
                        compiler.free(bf_address);
                    }
                    ABFInstruction::Write(address) => {
                        let bf_address = *compiler.address_map.get(address).unwrap();
                        builder.move_to(bf_address);
                        builder.write();
                        compiler.current_position = bf_address;
                    }
                    ABFInstruction::Add(address, amount) => {
                        let bf_address = *compiler.address_map.get(address).unwrap();
                        builder.move_to(bf_address);
                        builder.add(*amount as u8);
                        compiler.current_position = bf_address;
                    }
                    ABFInstruction::While(address, body) => {
                        let bf_address = *compiler.address_map.get(address).unwrap();
                        builder.move_to(bf_address);

                        let modified_addresses = body.modified_addresses();
                        for modified_address in &modified_addresses {
                            if let Some(modified_bf_address) =
                                compiler.address_map.get(modified_address)
                            {
                                compiler.set_value(*modified_bf_address, BFValue::Runtime);
                            }
                        }

                        builder.while_loop(bf_address, |builder| {
                            compile_impl(compiler, body, builder);
                        });

                        for modified_address in modified_addresses {
                            if let Some(modified_bf_address) =
                                compiler.address_map.get(&modified_address)
                            {
                                compiler.set_value(*modified_bf_address, BFValue::Runtime);
                            }
                        }

                        compiler.set_value(bf_address, 0);
                    }
                }
            }
        }
        let mut compiler = Self::new();
        let mut builder = BFProgramBuilder::new();
        compile_impl(&mut compiler, program, &mut builder);
        builder.build_program()
    }
}
