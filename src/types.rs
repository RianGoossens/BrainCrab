#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    U8,
    Bool,
    Array { element_type: Box<Type>, len: u16 },
}

impl Type {
    pub fn size(&self) -> u16 {
        match self {
            Type::U8 => 1,
            Type::Bool => 1,
            Type::Array { element_type, len } => element_type.size() * len,
        }
    }
}
