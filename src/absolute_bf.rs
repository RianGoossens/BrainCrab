use std::collections::{HashMap, HashSet};
use std::fmt::{self, Write};
use std::ops::{Index, IndexMut};

use abf_optimizer::path_optimize;
use bf_core::{BFProgram, BFTree};

#[derive(Debug, Clone)]
pub enum ABFTree {
    Add(u16, i8),
    Write(u16),
    Read(u16),
    While(u16, Vec<ABFTree>),
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dependency {
    from: String,
    to: String,
}

impl ABFTree {
    fn calculate_path_impl(&self, output: &mut Vec<u16>) {
        match self {
            ABFTree::Add(x, _) => {
                if output.last().unwrap() != x {
                    output.push(*x);
                }
            }
            ABFTree::Read(x) => {
                if output.last().unwrap() != x {
                    output.push(*x);
                }
            }
            ABFTree::Write(x) => {
                if output.last().unwrap() != x {
                    output.push(*x);
                }
            }
            ABFTree::While(x, body) => {
                if output.last().unwrap() != x {
                    output.push(*x);
                }
                for tree in body {
                    tree.calculate_path_impl(output);
                }
                if *output.last().unwrap() != *x {
                    output.push(*x);
                }
            }
        }
    }
    fn to_bf_impl(&self, pointer: &mut u16, output: &mut BFProgram) {
        match self {
            ABFTree::Add(position, value) => {
                let offset = (*position as i16) - (*pointer as i16);
                output.push_instruction(BFTree::Move(offset));
                output.push_instruction(BFTree::Add(*value as u8));
                *pointer = *position;
            }
            ABFTree::Write(position) => {
                let offset = (*position as i16) - (*pointer as i16);
                output.push_instruction(BFTree::Move(offset));
                output.push_instruction(BFTree::Write);
                *pointer = *position;
            }
            ABFTree::Read(position) => {
                let offset = (*position as i16) - (*pointer as i16);
                output.push_instruction(BFTree::Move(offset));
                output.push_instruction(BFTree::Read);
                *pointer = *position;
            }
            ABFTree::While(position, body) => {
                let offset = (*position as i16) - (*pointer as i16);
                output.push_instruction(BFTree::Move(offset));
                *pointer = *position;

                let mut body_bf = BFProgram::new();
                for tree in body {
                    tree.to_bf_impl(pointer, &mut body_bf);
                }
                let offset = (*position as i16) - (*pointer as i16);
                body_bf.push_instruction(BFTree::Move(offset));

                output.push_instruction(BFTree::Loop(body_bf.0));
                *pointer = *position;
            }
        }
    }
    fn remap_addresses(&mut self, address_map: &[u16]) {
        match self {
            ABFTree::Add(address, _) => *address = address_map[*address as usize],
            ABFTree::Read(address) => *address = address_map[*address as usize],
            ABFTree::Write(address) => *address = address_map[*address as usize],
            ABFTree::While(address, body) => {
                *address = address_map[*address as usize];
                body.iter_mut().for_each(|tree| {
                    tree.remap_addresses(address_map);
                })
            }
        }
    }
    fn collect_variables(&self, addresses: &mut HashSet<u16>) {
        match self {
            ABFTree::Add(address, _) | ABFTree::Write(address) | ABFTree::Read(address) => {
                addresses.insert(*address);
            }
            ABFTree::While(address, body) => {
                addresses.insert(*address);
                for tree in body {
                    tree.collect_variables(addresses);
                }
            }
        }
    }
    fn build_dot_dependency_graph(&self, dependencies: &mut HashSet<Dependency>) {
        match self {
            ABFTree::Write(address) => {
                dependencies.insert(Dependency {
                    from: format!("{address}"),
                    to: "out".into(),
                });
            }
            ABFTree::Read(address) => {
                dependencies.insert(Dependency {
                    from: "in".into(),
                    to: format!("{address}"),
                });
            }
            ABFTree::While(address, body) => {
                let mut addresses = HashSet::new();
                self.collect_variables(&mut addresses);
                addresses.remove(address);
                for destination in addresses {
                    dependencies.insert(Dependency {
                        from: format!("{address}"),
                        to: format!("{destination}"),
                    });
                }
                for tree in body {
                    tree.build_dot_dependency_graph(dependencies);
                }
            }
            _ => {}
        }
    }

    fn disentangle_addresses(
        &mut self,
        address_map: &mut HashMap<u16, u16>,
        address_counter: &mut u16,
        in_loop: bool,
    ) {
        match self {
            ABFTree::Add(address, _) | ABFTree::Write(address) | ABFTree::Read(address) => {
                if let Some(mapped) = address_map.get(address) {
                    *address = *mapped;
                } else {
                    address_map.insert(*address, *address_counter);
                    *address = *address_counter;
                    *address_counter += 1;
                }
            }
            ABFTree::While(address, body) => {
                let original_address = *address;

                if let Some(mapped) = address_map.get(address) {
                    *address = *mapped;
                } else {
                    address_map.insert(*address, *address_counter);
                    *address = *address_counter;
                    *address_counter += 1;
                }

                for tree in body {
                    tree.disentangle_addresses(address_map, address_counter, true);
                }

                if !in_loop {
                    address_map.insert(original_address, *address_counter);
                    *address_counter += 1;
                }
            }
        }
    }

    fn analyze_relative_updates(&self, relative_updates: &mut HashMap<u16, Option<i8>>) {
        match self {
            ABFTree::Add(address, amount) => match relative_updates.get_mut(address) {
                Some(Some(x)) => *x = x.wrapping_add(*amount),
                Some(None) => {}
                None => {
                    relative_updates.insert(*address, Some(*amount));
                }
            },
            ABFTree::Write(_) => {}
            ABFTree::Read(address) => {
                relative_updates.insert(*address, None);
            }
            ABFTree::While(_, body) => {
                let mut body_relative_updates = HashMap::new();
                for tree in body {
                    tree.analyze_relative_updates(&mut body_relative_updates);
                }
                for (key, value) in body_relative_updates {
                    match value {
                        Some(0) => {}
                        _ => {
                            relative_updates.insert(key, None);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct ABFProgram {
    pub body: Vec<ABFTree>,
}

impl ABFProgram {
    pub fn new() -> Self {
        ABFProgram { body: vec![] }
    }
    pub fn push_instruction(&mut self, instruction: ABFTree) {
        match (&instruction, self.body.last_mut()) {
            (ABFTree::Add(location_a, a), Some(ABFTree::Add(location_b, b)))
                if location_a == location_b =>
            {
                *b = b.wrapping_add(*a)
            }
            _ => self.body.push(instruction),
        }
    }

    pub fn append(&mut self, rhs: ABFProgram) {
        for instruction in rhs.body {
            self.push_instruction(instruction);
        }
    }
    pub fn calculate_path(&self) -> Vec<u16> {
        let mut path = vec![0];
        for tree in &self.body {
            tree.calculate_path_impl(&mut path);
        }
        path
    }
    fn remap_addresses(&mut self, address_map: &[u16]) {
        for tree in &mut self.body {
            tree.remap_addresses(address_map);
        }
    }
    pub fn optimize_addresses(&mut self, max_iterations: u32) {
        let current_path = self.calculate_path();
        let address_map = path_optimize(&current_path, max_iterations);
        self.remap_addresses(&address_map);
    }

    pub fn to_bf(&self) -> BFProgram {
        let mut bf_program = BFProgram::new();
        let mut pointer = 0;
        for tree in &self.body {
            tree.to_bf_impl(&mut pointer, &mut bf_program);
        }
        bf_program
    }

    pub fn dot_dependency_graph(&self) -> String {
        let mut dependencies = HashSet::new();
        for tree in &self.body {
            tree.build_dot_dependency_graph(&mut dependencies);
        }

        let mut result = String::new();
        writeln!(result, "digraph G {{").expect("Could not write to dot graph.");
        for Dependency { from, to } in dependencies {
            writeln!(result, "{from} -> {to};").expect("Could not write to dot graph.");
        }
        writeln!(result, "}}").expect("Could not write to dot graph.");
        result
    }

    pub fn disentangle_addresses(&mut self) {
        let mut address_map = HashMap::new();
        let mut address_counter = 0;
        for tree in &mut self.body {
            tree.disentangle_addresses(&mut address_map, &mut address_counter, false);
        }
    }

    pub fn without_dead_loops(&self) -> Self {
        let mut current_state: Vec<Option<u8>> = (0..30000).map(|_| Some(0)).collect();

        let mut result = ABFProgram::new();
        for tree in &self.body {
            match tree {
                ABFTree::Add(address, value) => {
                    if let Some(x) = &mut current_state[*address as usize] {
                        *x = x.wrapping_add(*value as u8);
                    }
                    result.push_instruction(tree.clone());
                }
                ABFTree::Write(_) => {
                    result.push_instruction(tree.clone());
                }
                ABFTree::Read(address) => {
                    current_state[*address as usize] = None;
                    result.push_instruction(tree.clone());
                }
                ABFTree::While(address, body) => {
                    // Only does something if the cell at address is not zero
                    if current_state[*address as usize] != Some(0) {
                        let mut relative_updates = HashMap::new();
                        for tree in body {
                            tree.analyze_relative_updates(&mut relative_updates);
                        }
                        for (address, update) in relative_updates {
                            match update {
                                Some(0) => {}
                                _ => current_state[address as usize] = None,
                            }
                        }
                        current_state[*address as usize] = Some(0);
                        result.push_instruction(tree.clone());
                    }
                }
            }
        }

        result
    }
}

impl Default for ABFProgram {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellAnalysis {
    Unknown,
    Absolute(u8),
    Relative(i8),
}

impl fmt::Display for CellAnalysis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellAnalysis::Unknown => write!(f, "?"),
            CellAnalysis::Absolute(x) => write!(f, "{x}"),
            CellAnalysis::Relative(x) => {
                if *x >= 0 {
                    write!(f, "+{x}")
                } else {
                    write!(f, "{x}")
                }
            }
        }
    }
}

pub struct TapeAnalysis {
    tape: [CellAnalysis; 30000],
}

impl fmt::Display for TapeAnalysis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in 0..10 {
            write!(f, "{i}:{}\t", &self.tape[i])?;
        }
        Ok(())
    }
}

impl TapeAnalysis {
    fn new() -> Self {
        Self {
            tape: [CellAnalysis::Absolute(0); 30000],
        }
    }

    fn new_relative() -> Self {
        Self {
            tape: [CellAnalysis::Relative(0); 30000],
        }
    }

    fn iterate_effects(&mut self, amount: u8) {
        if amount == 0 {
            for cell in &mut self.tape {
                *cell = CellAnalysis::Relative(0)
            }
        } else {
            for cell in &mut self.tape {
                if let CellAnalysis::Relative(x) = cell {
                    *x *= amount as i8
                }
            }
        }
    }

    fn iterate_effects_unknown_times(&mut self) {
        for cell in &mut self.tape {
            match cell {
                CellAnalysis::Relative(x) if *x > 0 => *cell = CellAnalysis::Unknown,
                _ => {}
            }
        }
    }

    fn merge_with(&mut self, other: Self) {
        self.tape
            .iter_mut()
            .zip(other.tape)
            .for_each(|(current, new)| match (&current, new) {
                (_, CellAnalysis::Unknown) => *current = new,
                (_, CellAnalysis::Absolute(_)) => *current = new,
                (CellAnalysis::Unknown, _) => {}
                (CellAnalysis::Absolute(x), CellAnalysis::Relative(i)) => {
                    *current = CellAnalysis::Absolute((*x as i8 + i) as u8)
                }
                (CellAnalysis::Relative(a), CellAnalysis::Relative(b)) => {
                    *current = CellAnalysis::Relative(a.wrapping_add(b))
                }
            });
    }
}

impl Index<u16> for TapeAnalysis {
    type Output = CellAnalysis;
    fn index(&self, index: u16) -> &Self::Output {
        &self.tape[index as usize]
    }
}

impl IndexMut<u16> for TapeAnalysis {
    fn index_mut(&mut self, index: u16) -> &mut Self::Output {
        &mut self.tape[index as usize]
    }
}

mod util {
    pub(crate) fn steps_to_zero(start: u8, step: u8) -> Option<u8> {
        if step == 0 {
            return if start == 0 { Some(0) } else { None }; // No solution
        }

        let target = (256 - start as u16) % 256;
        let gcd = gcd(step as u16, 256);

        if target % gcd != 0 {
            return None; // No solution
        }

        // Modular inverse of step modulo 256/gcd
        let step_mod = step / gcd as u8;
        let target_mod = target / gcd;
        let m = 256 / gcd; // Reduce modulo space
        let step_inverse = mod_inverse(step_mod, m as u8)?; // Find inverse if it exists

        let n = (target_mod * step_inverse as u16) % m;
        Some(n as u8)
    }

    // Helper functions: gcd and mod_inverse
    fn gcd(a: u16, b: u16) -> u16 {
        if b == 0 {
            a
        } else {
            gcd(b, a % b)
        }
    }

    fn mod_inverse(a: u8, m: u8) -> Option<u8> {
        (1..m).find(|&x| (a as u16 * x as u16) % m as u16 == 1) // No inverse
    }
}

pub struct ABFOptimizer;

impl ABFOptimizer {
    fn analyze_abf_tree(tree: &ABFTree, state: &mut TapeAnalysis) {
        match tree {
            ABFTree::Add(address, value) => match &mut state[*address] {
                CellAnalysis::Unknown => {}
                CellAnalysis::Absolute(current) => *current = (*current as i8 + *value) as u8,
                CellAnalysis::Relative(current) => *current += *value,
            },
            ABFTree::Write(_) => {}
            ABFTree::Read(address) => state[*address] = CellAnalysis::Unknown,
            ABFTree::While(address, body) => {
                if state[*address] != CellAnalysis::Absolute(0) {
                    let mut body_analysis = TapeAnalysis::new_relative();
                    for tree in body {
                        Self::analyze_abf_tree(tree, &mut body_analysis);
                    }
                    match (state[*address], body_analysis[*address]) {
                        (_, CellAnalysis::Unknown) => body_analysis.iterate_effects_unknown_times(),
                        (CellAnalysis::Unknown, _) => body_analysis.iterate_effects_unknown_times(),
                        (_, CellAnalysis::Absolute(0)) => body_analysis.iterate_effects(1),
                        (_, CellAnalysis::Absolute(_)) => {
                            body_analysis.iterate_effects_unknown_times()
                        }
                        (CellAnalysis::Absolute(start), CellAnalysis::Relative(step)) => {
                            use util::*;
                            if let Some(iterations) = steps_to_zero(start, step as u8) {
                                body_analysis.iterate_effects(iterations);
                            } else {
                                body_analysis.iterate_effects_unknown_times();
                            }
                        }
                        (CellAnalysis::Relative(_), _) => {
                            body_analysis.iterate_effects_unknown_times()
                        }
                    }
                    state.merge_with(body_analysis);

                    state[*address] = CellAnalysis::Absolute(0);
                }
            }
        }
        println!("{tree:?}");
        println!("{state}");
    }
    pub fn analyze_abf(program: &ABFProgram) -> TapeAnalysis {
        let mut analysis = TapeAnalysis::new();
        for tree in &program.body {
            Self::analyze_abf_tree(tree, &mut analysis);
        }
        analysis
    }
}

mod abf_optimizer {
    use rand::{thread_rng, Rng};

    fn path_score(path: &[u16]) -> u32 {
        path.windows(2)
            .map(|window| (window[0] as i32 - window[1] as i32).unsigned_abs())
            .sum::<u32>()
            + path[0] as u32
    }
    fn remap_path(path: &[u16], map: &[u16]) -> Vec<u16> {
        let mut result = Vec::with_capacity(path.len());

        for i in path {
            result.push(map[*i as usize]);
        }

        result
    }
    fn mutate_map(map: &[u16], max_mutations: u8) -> Vec<u16> {
        let mut result = map.to_vec();
        let number_of_mutations = thread_rng().gen_range(1..=max_mutations);
        for _i in 0..number_of_mutations {
            let index_a = thread_rng().gen_range(0..map.len());
            let index_b = thread_rng().gen_range(0..map.len());
            result.swap(index_a, index_b);
        }
        result
    }
    pub fn path_optimize(path: &[u16], max_iterations: u32) -> Vec<u16> {
        let mut best_score = path_score(path);

        let mut best_map: Vec<_> = (0..=*path.iter().max().unwrap()).collect();
        for _i in 0..max_iterations {
            let mutation = mutate_map(&best_map, 5);
            let current_path = remap_path(path, &mutation);
            let current_score = path_score(&current_path);
            if current_score < best_score {
                best_score = current_score;
                best_map = mutation;
            }
        }
        best_map
    }
}
