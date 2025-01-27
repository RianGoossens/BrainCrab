use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

#[derive(Debug, Clone)]
pub enum ABFInstruction {
    New(u16, u8),
    Read(u16),
    Free(u16),
    Write(u16),
    WriteConst(u8),
    Add(u16, i8),
    While(u16, ABFProgram),
}

impl Display for ABFInstruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn fmt_impl(
            instruction: &ABFInstruction,
            f: &mut std::fmt::Formatter<'_>,
            indents: u8,
        ) -> std::fmt::Result {
            for _ in 0..indents {
                write!(f, "    ")?;
            }
            match instruction {
                ABFInstruction::New(address, value) => writeln!(f, "&{address} = {value};"),
                ABFInstruction::Read(address) => writeln!(f, "&{address} = read();"),
                ABFInstruction::Free(address) => writeln!(f, "free(&{address});"),
                ABFInstruction::Write(address) => writeln!(f, "write(&{address});"),
                ABFInstruction::WriteConst(value) => writeln!(f, "write({value});"),
                ABFInstruction::Add(address, amount) => writeln!(f, "&{address} += {amount};"),
                ABFInstruction::While(address, body) => {
                    writeln!(f, "while &{address} {{")?;
                    for instruction in &body.instructions {
                        fmt_impl(instruction, f, indents + 1)?;
                    }
                    for _ in 0..indents {
                        write!(f, "    ")?;
                    }
                    writeln!(f, "}}")
                }
            }
        }
        fmt_impl(self, f, 0)
    }
}

impl ABFInstruction {
    pub fn relevant_address(&self) -> Option<u16> {
        match self {
            ABFInstruction::New(x, _) => Some(*x),
            ABFInstruction::Read(x) => Some(*x),
            ABFInstruction::Free(x) => Some(*x),
            ABFInstruction::Write(x) => Some(*x),
            ABFInstruction::WriteConst(_) => None,
            ABFInstruction::Add(x, _) => Some(*x),
            ABFInstruction::While(x, _) => Some(*x),
        }
    }
    fn collect_modified_addresses(&self, addresses: &mut BTreeSet<u16>) {
        match self {
            ABFInstruction::Read(address) | ABFInstruction::Add(address, _) => {
                addresses.insert(*address);
            }
            ABFInstruction::While(address, body) => {
                addresses.insert(*address);
                for instruction in &body.instructions {
                    instruction.collect_modified_addresses(addresses);
                }
            }
            _ => {}
        };
    }
    fn collect_used_addresses(&self, addresses: &mut BTreeSet<u16>) {
        match self {
            ABFInstruction::New(address, _)
            | ABFInstruction::Read(address)
            | ABFInstruction::Add(address, _)
            | ABFInstruction::Free(address)
            | ABFInstruction::Write(address) => {
                addresses.insert(*address);
            }
            ABFInstruction::While(address, body) => {
                addresses.insert(*address);
                for instruction in &body.instructions {
                    instruction.collect_used_addresses(addresses);
                }
            }
            _ => {}
        };
    }
}

#[derive(Debug, Clone)]
pub struct ABFProgram {
    pub instructions: Vec<ABFInstruction>,
}

impl Display for ABFProgram {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for instruction in &self.instructions {
            instruction.fmt(f)?;
        }
        Ok(())
    }
}

impl ABFProgram {
    pub fn new(instructions: Vec<ABFInstruction>) -> Self {
        Self { instructions }
    }
    pub fn add_instruction(&mut self, instruction: ABFInstruction) {
        self.instructions.push(instruction);
    }
    pub fn used_addresses(&self) -> BTreeSet<u16> {
        let mut result = BTreeSet::new();
        for instruction in &self.instructions {
            instruction.collect_used_addresses(&mut result);
        }
        result
    }
    pub fn modified_addresses(&self) -> BTreeSet<u16> {
        let mut result = BTreeSet::new();
        for instruction in &self.instructions {
            instruction.collect_modified_addresses(&mut result);
        }
        result
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ABFCell {
    value: ABFValue,
    used: bool,
}

impl ABFCell {
    pub fn new(value: ABFValue, used: bool) -> Self {
        Self { value, used }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ABFState {
    pub values: Vec<ABFCell>,
}

impl ABFState {
    pub fn new() -> Self {
        Self {
            values: vec![
                ABFCell {
                    value: 0.into(),
                    used: false
                };
                30000
            ],
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

pub struct ABFProgramBuilder {
    program: ABFProgram,
    state: ABFState,
}

impl ABFProgramBuilder {
    pub fn new() -> Self {
        Self {
            program: ABFProgram::new(vec![]),
            state: ABFState::new(),
        }
    }

    fn add_instruction(&mut self, instruction: ABFInstruction) {
        self.program.add_instruction(instruction);
    }

    pub fn new_address(&mut self, value: u8) -> u16 {
        todo!()
    }

    pub fn read(&mut self) -> u16 {
        todo!()
    }

    pub fn free(&mut self, address: u16) {
        todo!()
    }

    pub fn write(&mut self, address: u16) {
        todo!()
    }

    pub fn add(&mut self, address: u16, amount: i8) {
        todo!()
    }

    pub fn while_loop(&mut self, address: u16, body: impl Fn(&mut ABFProgramBuilder)) {
        todo!()
    }
}

impl Default for ABFProgramBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ABFCompiler;

impl ABFCompiler {
    pub fn optimize_frees(program: &mut ABFProgram) {
        // First clear out any existing frees, we can do better.
        program
            .instructions
            .retain(|x| !matches!(x, &ABFInstruction::Free(_)));

        // Now we detect usage of all declared addresses. Undeclared addresses are handled by parent scopes.
        // We also optimize bodies of While loops here.
        let mut last_address_usage = BTreeMap::new();

        for (index, instruction) in program.instructions.iter_mut().enumerate() {
            match instruction {
                ABFInstruction::New(address, _) | ABFInstruction::Read(address) => {
                    last_address_usage.insert(*address, index);
                }
                ABFInstruction::Write(address) | ABFInstruction::Add(address, _) => {
                    if last_address_usage.contains_key(address) {
                        last_address_usage.insert(*address, index);
                    }
                }
                ABFInstruction::Free(_) => panic!("There should not be any frees at this point."),
                ABFInstruction::WriteConst(_) => {}
                ABFInstruction::While(address, body) => {
                    Self::optimize_frees(body);
                    if last_address_usage.contains_key(address) {
                        last_address_usage.insert(*address, index);
                    }
                    for address in body.used_addresses() {
                        if last_address_usage.contains_key(&address) {
                            last_address_usage.insert(address, index);
                        }
                    }
                }
            }
        }

        // Sort last address usages by usage, from most recent to least recent
        let mut last_address_usage: Vec<_> = last_address_usage.into_iter().collect();
        last_address_usage.sort_by(|a, b| b.1.cmp(&a.1));

        // Insert frees at their optimal location
        for (address, last_usage) in last_address_usage.into_iter() {
            program
                .instructions
                .insert(last_usage + 1, ABFInstruction::Free(address));
        }
    }

    fn optimize_impl(abf: &ABFProgram, state: &mut ABFState, output: &mut ABFProgram) {
        for instruction in &abf.instructions {
            match instruction {
                ABFInstruction::New(address, value) => {
                    state.set_value(*address, *value);
                }
                ABFInstruction::Read(address) => {
                    output.add_instruction(ABFInstruction::Read(*address));
                    state.set_value(*address, ABFValue::Runtime);
                }
                ABFInstruction::Free(address) => {
                    let cell = state.get_cell_mut(*address);
                    assert!(cell.used);
                    if cell.value == ABFValue::Runtime {
                        output.add_instruction(ABFInstruction::Free(*address));
                    }
                    state.free(*address);
                }
                ABFInstruction::Write(address) => {
                    let cell = state.get_cell_mut(*address);
                    assert!(cell.used);
                    match cell.value {
                        ABFValue::CompileTime(value) => {
                            output.add_instruction(ABFInstruction::WriteConst(value));
                        }
                        ABFValue::Runtime => {
                            output.add_instruction(ABFInstruction::Write(*address))
                        }
                    }
                }
                ABFInstruction::WriteConst(value) => {
                    output.add_instruction(ABFInstruction::WriteConst(*value))
                }
                ABFInstruction::Add(address, amount) => {
                    let cell = state.get_cell_mut(*address);
                    assert!(cell.used);
                    match &mut cell.value {
                        ABFValue::CompileTime(value) => {
                            *value = value.wrapping_add(*amount as u8);
                        }
                        ABFValue::Runtime => {
                            output.add_instruction(ABFInstruction::Add(*address, *amount))
                        }
                    }
                }
                ABFInstruction::While(address, body) => {
                    let cell = state.get_cell(*address);
                    assert!(cell.used);
                    let mut new_state = state.clone();
                    let mut new_output = output.clone();

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

                        Self::optimize_impl(body, &mut new_state, &mut new_output);
                    }

                    if unrolled_successfully {
                        *state = new_state;
                        *output = new_output;
                    } else {
                        let mut new_body = ABFProgram::new(vec![]);

                        // Since we don't know how this loop will run, any modified addresses
                        // in this loop become unknown
                        let modified_addresses = body.modified_addresses();
                        for modified_address in &modified_addresses {
                            let cell = state.get_cell_mut(*modified_address);
                            if cell.used {
                                if let ABFValue::CompileTime(x) = cell.value {
                                    output
                                        .add_instruction(ABFInstruction::New(*modified_address, x));
                                }
                                cell.value = ABFValue::Runtime;
                            }
                        }

                        Self::optimize_impl(body, state, &mut new_body);

                        // We need to make sure that all modified addresses are still marked as
                        // runtime after the loop, since there is no way to guarantee if the loop
                        // will even run.
                        for modified_address in modified_addresses {
                            let cell = state.get_cell_mut(modified_address);
                            if cell.used {
                                cell.value = ABFValue::Runtime;
                            }
                        }
                        output.add_instruction(ABFInstruction::While(*address, new_body));
                    }
                    state.set_value(*address, 0);
                }
            }
        }
    }

    pub fn optimize(program: &ABFProgram) -> ABFProgram {
        let mut state = ABFState::new();
        let mut output = ABFProgram::new(vec![]);
        Self::optimize_impl(program, &mut state, &mut output);
        output
    }
}
