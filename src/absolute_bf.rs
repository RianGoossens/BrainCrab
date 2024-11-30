use abf_optimizer::path_optimize;
use bf_core::{BFProgram, BFTree};

#[derive(Debug)]
pub enum ABFTree {
    MoveTo(u16),
    Add(u16, i8),
    Write(u16),
    Read(u16),
    While(u16, Vec<ABFTree>),
}

impl ABFTree {
    fn calculate_path_impl(&self, output: &mut Vec<u16>) {
        match self {
            ABFTree::MoveTo(x) => {
                if output.last().unwrap() != x {
                    output.push(*x);
                }
            }
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
            ABFTree::MoveTo(position) => {
                let offset = (*position as i16) - (*pointer as i16);
                output.push_instruction(BFTree::Move(offset));
                *pointer = *position;
            }
            ABFTree::Add(position, value) => {
                let offset = (*position as i16) - (*pointer as i16);
                output.push_instruction(BFTree::Move(offset));
                output.push_instruction(BFTree::Add(*value as u8));
            }
            ABFTree::Write(position) => {
                let offset = (*position as i16) - (*pointer as i16);
                output.push_instruction(BFTree::Move(offset));
                output.push_instruction(BFTree::Write);
            }
            ABFTree::Read(position) => {
                let offset = (*position as i16) - (*pointer as i16);
                output.push_instruction(BFTree::Move(offset));
                output.push_instruction(BFTree::Read);
            }
            ABFTree::While(position, body) => {
                let offset = (*position as i16) - (*pointer as i16);
                output.push_instruction(BFTree::Move(offset));

                let mut body_bf = BFProgram::new();
                for tree in body {
                    tree.to_bf_impl(pointer, &mut body_bf);
                }
                ABFTree::MoveTo(*position).to_bf_impl(pointer, &mut body_bf);

                output.push_instruction(BFTree::Loop(body_bf.0))
            }
        }
    }
    fn remap_addresses(&mut self, address_map: &[u16]) {
        match self {
            ABFTree::MoveTo(address) => *address = address_map[*address as usize],
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
            (ABFTree::MoveTo(destination), Some(ABFTree::MoveTo(previous_destination))) => {
                *previous_destination = *destination
            }
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
        self.body.insert(0, ABFTree::MoveTo(0));

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
}

impl Default for ABFProgram {
    fn default() -> Self {
        Self::new()
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
