use bf_core::{BFProgram, BFTree};

pub enum ABFTree {
    MoveTo(u16),
    Add(u8),
    Write,
    Read,
    While(Vec<ABFTree>),
}

impl ABFTree {
    fn calculate_path_impl(&self, output: &mut Vec<u16>) {
        match self {
            ABFTree::MoveTo(x) => output.push(*x),
            ABFTree::While(body) => {
                let start_point = *output.last().unwrap();
                for tree in body {
                    tree.calculate_path_impl(output);
                }
                output.push(start_point);
            }
            _ => {}
        }
    }
    fn to_bf_impl(&self, pointer: &mut u16, output: &mut Vec<BFTree>) {
        match self {
            ABFTree::MoveTo(position) => {
                let offset = (*position as i16) - (*pointer as i16);
                output.push(BFTree::Move(offset));
                *pointer = *position;
            }
            ABFTree::Add(value) => {
                output.push(BFTree::Add(*value));
            }
            ABFTree::Write => output.push(BFTree::Write),
            ABFTree::Read => output.push(BFTree::Read),
            ABFTree::While(body) => {
                let start_pointer = *pointer;

                let mut body_bf = vec![];
                for tree in body {
                    tree.to_bf_impl(pointer, &mut body_bf);
                    ABFTree::MoveTo(start_pointer).to_bf_impl(pointer, &mut body_bf);
                }

                output.push(BFTree::Loop(body_bf))
            }
        }
    }
}

pub struct ABFProgram {
    body: Vec<ABFTree>,
}

impl ABFProgram {
    pub fn push_instruction(&mut self, instruction: ABFTree) {
        match (&instruction, self.body.last_mut()) {
            (ABFTree::MoveTo(destination), Some(ABFTree::MoveTo(previous_destination))) => {
                *previous_destination = *destination
            }
            (ABFTree::Add(a), Some(ABFTree::Add(b))) => *b = b.wrapping_add(*a),
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
    pub fn to_bf(&self) -> BFProgram {
        let mut bf_instructions = vec![];
        let mut pointer = 0;
        for tree in &self.body {
            tree.to_bf_impl(&mut pointer, &mut bf_instructions);
        }
        BFProgram(bf_instructions)
    }
}
