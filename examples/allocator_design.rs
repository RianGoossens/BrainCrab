use std::fmt;

pub struct Allocator {
    tape: Vec<bool>,
}

impl Allocator {
    pub fn new(len: usize) -> Self {
        Self {
            tape: vec![false; len],
        }
    }

    pub fn allocate(&mut self, size: usize) -> Option<usize> {
        println!("Allocating {size}");

        let mut check_index = size - 1;

        loop {
            if check_index >= self.tape.len() {
                break;
            }
            let mut found_spot = true;
            for i in 0..size {
                if self.tape[check_index - i] {
                    check_index += size - i;
                    found_spot = false;
                    break;
                }
            }
            if found_spot {
                for i in 0..size {
                    self.tape[check_index - i] = true;
                }
                return Some(check_index + 1 - size);
            }
        }
        None
    }

    pub fn deallocate(&mut self, index: usize, size: usize) {
        println!("Deallocating {index} len {size}");
        for i in index..index + size {
            self.tape[i] = false;
        }
    }
}

impl Default for Allocator {
    fn default() -> Self {
        Self::new(100)
    }
}

impl fmt::Display for Allocator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}",
            self.tape
                .iter()
                .map(|x| if *x { 1 } else { 0 })
                .collect::<Vec<_>>()
        )
    }
}

fn main() {
    let mut allocator = Allocator::new(20);

    println!("{allocator}");
    allocator.allocate(2);
    println!("{allocator}");
    allocator.allocate(2);
    println!("{allocator}");
    allocator.allocate(2);
    println!("{allocator}");
    allocator.deallocate(2, 3);
    println!("{allocator}");
    allocator.allocate(4);
    println!("{allocator}");
    allocator.allocate(3);
    println!("{allocator}");
}
