use crate::{
    compiler_error::{CompileResult, CompilerError},
    types::Type,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstantValue {
    U8(u8),
    Bool(bool),
    Array(Vec<ConstantValue>),
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
            ConstantValue::Array(vec) => match vec.first() {
                Some(x) => Type::Array {
                    element_type: Box::new(x.value_type()),
                    len: vec.len() as u16,
                },
                None => panic!("array of size 0"),
            },
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
