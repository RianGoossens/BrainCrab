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
    pub last_address: u16,
}

impl ABFState {
    pub fn new() -> Self {
        Self {
            values: vec![ABFCell::new(0, false); 30000],
            last_address: 0,
        }
    }

    pub fn find_address(&mut self, expected: Option<u8>) -> u16 {
        let mut best_address = u16::MAX;
        let mut best_distance = u16::MAX;
        for (i, cell) in self.values.iter().enumerate() {
            if !cell.used {
                let address_distance = self.last_address.abs_diff(i as u16);
                let value_distance = if let Some(expected) = expected {
                    if let ABFValue::CompileTime(actual) = cell.value {
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
        self.last_address = address;
    }

    pub fn free(&mut self, address: u16) {
        let cell = self.get_cell_mut(address);
        cell.used = false;
    }
}

pub struct ABFProgramBuilder {
    program_stack: Vec<ABFProgram>,
    state: ABFState,
}

impl ABFProgramBuilder {
    pub fn new() -> Self {
        Self {
            program_stack: vec![ABFProgram::new(vec![])],
            state: ABFState::new(),
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
        let address = self.state.find_address(Some(value));
        self.state.set_value(address, value);
        self.add_instruction(ABFInstruction::New(address, value));
        address
    }

    pub fn read(&mut self) -> u16 {
        let address = self.state.find_address(None);
        self.state.set_value(address, ABFValue::Runtime);
        self.add_instruction(ABFInstruction::Read(address));
        address
    }

    pub fn free(&mut self, address: u16) {
        self.state.free(address);
        // Since we are tracking state ourselves, no need to add any frees anymore?
        self.add_instruction(ABFInstruction::Free(address));
    }

    pub fn write(&mut self, address: u16) {
        self.add_instruction(ABFInstruction::Write(address));
    }

    pub fn add(&mut self, address: u16, amount: i8) {
        let cell = self.state.get_cell_mut(address);
        assert!(cell.used);
        if let ABFValue::CompileTime(x) = &mut cell.value {
            *x = x.wrapping_add(amount as u8);
        }
        self.add_instruction(ABFInstruction::Add(address, amount));
    }

    pub fn while_loop(&mut self, address: u16, body_function: impl FnOnce(&mut ABFProgramBuilder)) {
        self.program_stack.push(ABFProgram::new(vec![]));
        body_function(self);
        let body = self.program_stack.pop().unwrap();

        // Every value that was modified inside the while loop is now unknown at compile time
        for modified_address in body.modified_addresses() {
            let cell = self.state.get_cell_mut(modified_address);
            cell.value = ABFValue::Runtime;
        }

        // After a loop the predicate address is always zero
        self.state.set_value(address, 0);

        self.add_instruction(ABFInstruction::While(address, body));
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
