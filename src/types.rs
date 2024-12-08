#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    U8,
    Bool,
}

impl Type {
    pub fn size(&self) -> u8 {
        1
    }
}
