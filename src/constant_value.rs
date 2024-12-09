use crate::{
    compiler_error::{CompileResult, CompilerError},
    types::Type,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstantValue {
    U8(u8),
    Bool(bool),
}

impl From<u8> for ConstantValue {
    fn from(value: u8) -> Self {
        ConstantValue::U8(value)
    }
}

impl From<bool> for ConstantValue {
    fn from(value: bool) -> Self {
        ConstantValue::Bool(value)
    }
}

impl ConstantValue {
    pub fn value_type(&self) -> Type {
        match self {
            ConstantValue::U8(_) => Type::U8,
            ConstantValue::Bool(_) => Type::Bool,
        }
    }

    pub fn get_u8(&self) -> CompileResult<u8> {
        match self {
            ConstantValue::U8(value) => Ok(*value),
            _ => Err(CompilerError::TypeError {
                expected: Type::U8,
                actual: self.value_type(),
            }),
        }
    }

    pub fn get_bool(&self) -> CompileResult<bool> {
        match self {
            ConstantValue::Bool(value) => Ok(*value),
            _ => Err(CompilerError::TypeError {
                expected: Type::Bool,
                actual: self.value_type(),
            }),
        }
    }
}
