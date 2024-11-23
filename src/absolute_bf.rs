use bf_core::{BFProgram, BFTree};

pub enum ABFTree {
    MoveTo(u16),
    Add(u8),
    Write,
    Read,
    While(u16, Vec<ABFTree>),
}

impl ABFTree {
    fn calculate_path_impl(&self, output: &mut Vec<u16>) {
        match self {
            ABFTree::MoveTo(x) => output.push(*x),
            ABFTree::While(predicate, body) => {
                output.push(*predicate);
                for tree in body {
                    tree.calculate_path_impl(output);
                }
                output.push(*predicate);
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
            ABFTree::While(predicate, body) => {
                ABFTree::MoveTo(*predicate).to_bf_impl(pointer, output);

                let mut body_bf = vec![];
                for tree in body {
                    tree.to_bf_impl(pointer, &mut body_bf);
                    ABFTree::MoveTo(*predicate).to_bf_impl(pointer, &mut body_bf);
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
