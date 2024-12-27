use crate::{
    compiler::AddressPool,
    compiler_error::{CompileResult, CompilerError},
    constant_value::ConstantValue,
    types::Type,
};

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
    pub fn address(&self) -> u16 {
        self.address
    }
    pub fn borrow(&self) -> Self {
        Self {
            address: self.address,
            value_type: self.value_type.clone(),
            mutable: self.mutable,
            address_pool: None,
        }
    }
    pub fn is_mutable(&self) -> bool {
        self.mutable
    }
    pub fn value_type(&self) -> Type {
        self.value_type.clone()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Constant(ConstantValue),
    LValue(LValue),
}

impl Value {
    pub fn constant<A: Into<ConstantValue>>(value: A) -> Self {
        Self::Constant(value.into())
    }

    pub fn new_borrow(address: u16, value_type: Type) -> Self {
        Self::LValue(LValue {
            address,
            value_type,
            mutable: false,
            address_pool: None,
        })
    }

    pub fn lvalue(lvalue: LValue) -> Self {
        Self::LValue(lvalue)
    }

    pub fn borrow(&self) -> Self {
        match self {
            Value::Constant(x) => x.clone().into(),
            Value::LValue(lvalue) => Value::LValue(lvalue.borrow()),
        }
    }

    pub fn is_mutable(&self) -> bool {
        match self {
            Value::Constant(_) => false,
            Value::LValue(variable) => variable.is_mutable(),
        }
    }

    pub fn value_type(&self) -> CompileResult<Type> {
        match self {
            Value::Constant(value) => value.value_type(),
            Value::LValue(variable) => Ok(variable.value_type()),
        }
    }

    pub fn type_check(&self, expected: Type) -> CompileResult<()> {
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
