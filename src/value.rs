use std::ops::Range;

use crate::{
    compiler::AddressPool,
    compiler_error::{CompileResult, CompilerError},
    constant_value::ConstantValue,
    types::Type,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemorySlice {
    pub address: u16,
    pub len: u16,
}

impl MemorySlice {
    pub fn new(address: u16, len: u16) -> Self {
        Self { address, len }
    }
    pub fn range(&self) -> Range<u16> {
        self.address..self.address + self.len
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct LValue {
    pub address: u16,
    pub value_type: Type,
    pub mutable: bool,
    pub address_pool: Option<AddressPool>,
}

impl Drop for LValue {
    fn drop(&mut self) {
        if let Some(address_pool) = &self.address_pool {
            address_pool
                .borrow_mut()
                .deallocate(self.address, self.value_type.size());
        }
    }
}

impl LValue {
    pub fn is_owned(&self) -> bool {
        self.address_pool.is_some()
    }
    pub fn is_borrowed(&self) -> bool {
        !self.is_owned()
    }
    pub fn borrow(&self) -> Self {
        Self {
            address: self.address,
            value_type: self.value_type.clone(),
            mutable: self.mutable,
            address_pool: None,
        }
    }
    pub fn type_check(&self, expected: &Type) -> CompileResult<()> {
        let actual = &self.value_type;
        if actual == expected {
            Ok(())
        } else {
            Err(CompilerError::TypeError {
                expected: expected.clone(),
                actual: actual.clone(),
            })
        }
    }
    pub fn memory_slice(&self) -> MemorySlice {
        MemorySlice {
            address: self.address,
            len: self.value_type.size(),
        }
    }
    pub fn data(&self) -> Vec<LValue> {
        let mut result = vec![];
        for address in self.memory_slice().range() {
            result.push(LValue {
                address,
                value_type: Type::U8,
                mutable: self.mutable,
                address_pool: None,
            });
        }
        result
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Constant(ConstantValue),
    LValue(LValue),
}

impl Value {
    pub fn new_borrow(address: u16, value_type: Type) -> Self {
        Self::LValue(LValue {
            address,
            value_type,
            mutable: false,
            address_pool: None,
        })
    }

    pub fn borrow(&self) -> Self {
        match self {
            Value::Constant(x) => x.clone().into(),
            Value::LValue(lvalue) => lvalue.borrow().into(),
        }
    }

    pub fn is_owned(&self) -> bool {
        if let Value::LValue(x) = self {
            x.is_owned()
        } else {
            false
        }
    }

    pub fn mutable<'a>(&self) -> CompileResult<'a, LValue> {
        match self {
            Value::Constant(_) => Err(CompilerError::MutableBorrowOfImmutableVariable(
                self.borrow(),
            )),
            Value::LValue(variable) => {
                if variable.mutable {
                    Ok(variable.borrow())
                } else {
                    Err(CompilerError::MutableBorrowOfImmutableVariable(
                        self.borrow(),
                    ))
                }
            }
        }
    }

    pub fn data(&self) -> Vec<Value> {
        match self {
            Value::Constant(constant_value) => constant_value
                .data()
                .into_iter()
                .map(|x| x.into())
                .collect(),
            Value::LValue(lvalue) => lvalue.data().into_iter().map(|x| x.into()).collect(),
        }
    }

    pub fn value_type<'a>(&self) -> CompileResult<'a, Type> {
        match self {
            Value::Constant(value) => value.value_type(),
            Value::LValue(variable) => Ok(variable.value_type.clone()),
        }
    }

    pub fn type_check<'a>(&self, expected: Type) -> CompileResult<'a, ()> {
        let actual = self.value_type()?;
        if actual == expected {
            Ok(())
        } else {
            Err(CompilerError::TypeError { expected, actual })
        }
    }
}

impl From<LValue> for Value {
    fn from(lvalue: LValue) -> Self {
        Self::LValue(lvalue)
    }
}

impl<A: Into<ConstantValue>> From<A> for Value {
    fn from(value: A) -> Self {
        Self::Constant(value.into())
    }
}
