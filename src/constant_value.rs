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
    pub fn value_type(&self) -> CompileResult<Type> {
        match self {
            ConstantValue::U8(_) => Ok(Type::U8),
            ConstantValue::Bool(_) => Ok(Type::Bool),
            ConstantValue::Array(vec) => match vec.first() {
                Some(x) => {
                    let expected = x.value_type()?;
                    for (index, element) in vec.iter().enumerate() {
                        let actual = element.value_type()?;
                        if actual != expected {
                            return Err(CompilerError::ArrayHasDifferentTypes {
                                expected,
                                index: index as u16,
                                actual,
                            });
                        }
                    }
                    Ok(Type::Array {
                        element_type: Box::new(x.value_type()?),
                        len: vec.len() as u16,
                    })
                }
                None => panic!("array of size 0"),
            },
        }
    }

    pub fn get_u8(&self) -> CompileResult<u8> {
        match self {
            ConstantValue::U8(value) => Ok(*value),
            _ => Err(CompilerError::TypeError {
                expected: Type::U8,
                actual: self.value_type()?,
            }),
        }
    }

    pub fn get_bool(&self) -> CompileResult<bool> {
        match self {
            ConstantValue::Bool(value) => Ok(*value),
            _ => Err(CompilerError::TypeError {
                expected: Type::Bool,
                actual: self.value_type()?,
            }),
        }
    }
}
