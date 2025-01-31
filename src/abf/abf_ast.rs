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
    fn collect_mentioned_addresses(&self, addresses: &mut BTreeSet<u16>) {
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
                    instruction.collect_mentioned_addresses(addresses);
                }
            }
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

    pub fn mentioned_addresses(&self) -> BTreeSet<u16> {
        let mut result = BTreeSet::new();
        for instruction in &self.instructions {
            instruction.collect_mentioned_addresses(&mut result);
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

    pub fn optimize_frees(&mut self) {
        // First clear out any existing frees, we can do better.
        self.instructions
            .retain(|x| !matches!(x, &ABFInstruction::Free(_)));

        // Now we detect usage of all declared addresses. Undeclared addresses are handled by parent scopes.
        // We also optimize bodies of While loops here.
        let mut last_address_mention = BTreeMap::new();

        for (index, instruction) in self.instructions.iter_mut().enumerate() {
            match instruction {
                ABFInstruction::New(address, _) | ABFInstruction::Read(address) => {
                    last_address_mention.insert(*address, index);
                }
                ABFInstruction::Write(address) | ABFInstruction::Add(address, _) => {
                    if last_address_mention.contains_key(address) {
                        last_address_mention.insert(*address, index);
                    }
                }
                ABFInstruction::Free(_) => panic!("There should not be any frees at this point."),
                ABFInstruction::While(address, body) => {
                    Self::optimize_frees(body);
                    if last_address_mention.contains_key(address) {
                        last_address_mention.insert(*address, index);
                    }
                    for address in body.mentioned_addresses() {
                        if last_address_mention.contains_key(&address) {
                            last_address_mention.insert(address, index);
                        }
                    }
                }
            }
        }

        // Sort last address usages by usage, from most recent to least recent
        let mut last_address_mention: Vec<_> = last_address_mention.into_iter().collect();
        last_address_mention.sort_by(|a, b| b.1.cmp(&a.1));

        // Insert frees at their optimal location
        for (address, last_usage) in last_address_mention.into_iter() {
            self.instructions
                .insert(last_usage + 1, ABFInstruction::Free(address));
        }
    }

    pub fn clear_unused_variables(&mut self) {
        fn analyze_variable_usage(program: &ABFProgram, variable_usage: &mut BTreeMap<u16, bool>) {
            for instruction in &program.instructions {
                match instruction {
                    ABFInstruction::New(address, _) => {
                        variable_usage.insert(*address, false);
                    }
                    ABFInstruction::Read(address) | ABFInstruction::Write(address) => {
                        variable_usage.insert(*address, true);
                    }
                    ABFInstruction::While(address, body) => {
                        variable_usage.insert(*address, true);
                        analyze_variable_usage(body, variable_usage);
                    }
                    ABFInstruction::Free(_) | ABFInstruction::Add(_, _) => {
                        // Do nothing
                    }
                }
            }
        }

        fn keep_used_variables(program: &ABFProgram, used_variables: &BTreeSet<u16>) -> ABFProgram {
            let mut output = ABFProgram::new(vec![]);

            for instruction in &program.instructions {
                match instruction {
                    ABFInstruction::New(address, _)
                    | ABFInstruction::Read(address)
                    | ABFInstruction::Free(address)
                    | ABFInstruction::Write(address)
                    | ABFInstruction::Add(address, _) => {
                        if used_variables.contains(address) {
                            output.add_instruction(instruction.clone());
                        }
                    }
                    ABFInstruction::While(address, body) => {
                        if used_variables.contains(address) {
                            let new_body = keep_used_variables(body, used_variables);
                            output.add_instruction(ABFInstruction::While(*address, new_body));
                        }
                    }
                }
            }

            output
        }

        let mut variable_usage = BTreeMap::new();
        analyze_variable_usage(self, &mut variable_usage);

        let used_variables = variable_usage
            .into_iter()
            .filter_map(|(address, used)| if used { Some(address) } else { None })
            .collect::<BTreeSet<_>>();

        *self = keep_used_variables(self, &used_variables);
    }
}
