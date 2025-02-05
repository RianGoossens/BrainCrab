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

impl From<char> for ConstantValue {
    fn from(value: char) -> Self {
        (value as u8).into()
    }
}

impl<'a> From<&'a str> for ConstantValue {
    fn from(value: &'a str) -> Self {
        let chars: Vec<_> = value.chars().collect();
        chars.into()
    }
}

impl From<String> for ConstantValue {
    fn from(value: String) -> Self {
        let value: &str = &value;
        value.into()
    }
}

impl<A: Into<ConstantValue>> From<Vec<A>> for ConstantValue {
    fn from(value: Vec<A>) -> Self {
        ConstantValue::Array(value.into_iter().map(|x| x.into()).collect())
    }
}

impl ConstantValue {
    pub fn data(&self) -> Vec<u8> {
        fn data_impl(source: &ConstantValue, result: &mut Vec<u8>) {
            match source {
                ConstantValue::U8(value) => result.push(*value),
                ConstantValue::Bool(value) => result.push(if *value { 1 } else { 0 }),
                ConstantValue::Array(vec) => vec.iter().for_each(|x| data_impl(x, result)),
            }
        }
        let mut result = vec![];
        data_impl(self, &mut result);
        result
    }

    pub fn value_type<'a>(&self) -> CompileResult<'a, Type> {
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
                        len: vec.len() as u8,
                    })
                }
                None => panic!("array of size 0"),
            },
        }
    }

    pub fn get_u8<'a>(&self) -> CompileResult<'a, u8> {
        match self {
            ConstantValue::U8(value) => Ok(*value),
            _ => Err(CompilerError::TypeError {
                expected: Type::U8,
                actual: self.value_type()?,
            }),
        }
    }

    pub fn get_bool<'a>(&self) -> CompileResult<'a, bool> {
        match self {
            ConstantValue::Bool(value) => Ok(*value),
            _ => Err(CompilerError::TypeError {
                expected: Type::Bool,
                actual: self.value_type()?,
            }),
        }
    }
}
