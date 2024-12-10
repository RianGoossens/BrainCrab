use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub struct BrainCrabAllocator {
    tape: [bool; 30000],
}

impl BrainCrabAllocator {
    pub fn new() -> Self {
        Self {
            tape: [false; 30000],
        }
    }

    pub fn allocate(&mut self, size: u16) -> Option<u16> {
        let mut check_index = size - 1;

        loop {
            if check_index as usize >= self.tape.len() {
                break;
            }
            let mut found_spot = true;
            for i in 0..size {
                if self.tape[(check_index - i) as usize] {
                    check_index += size - i;
                    found_spot = false;
                    break;
                }
            }
            if found_spot {
                for i in 0..size {
                    self.tape[(check_index - i) as usize] = true;
                }
                return Some(check_index + 1 - size);
            }
        }
        None
    }

    pub fn deallocate(&mut self, index: u16, size: u16) {
        for i in index..index + size {
            self.tape[i as usize] = false;
        }
    }
}

impl Default for BrainCrabAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for BrainCrabAllocator {
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
