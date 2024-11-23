pub trait BrainCrabAllocator {
    fn new_allocator() -> Self;
    fn allocate(&mut self, near: u16) -> Option<u16>;
    fn deallocate(&mut self, address: u16);
}

impl BrainCrabAllocator for Vec<u16> {
    fn new_allocator() -> Self {
        (0..30000).rev().collect()
    }
    fn allocate(&mut self, _near: u16) -> Option<u16> {
        // if let Some(address) = address {
        //     println!("allocating {address}");
        // }
        self.pop()
    }

    fn deallocate(&mut self, address: u16) {
        // println!("Deallocating {address}");
        self.push(address);
    }
}
