use crate::{
    compiler::AddressPool,
    compiler_error::{CompileResult, CompilerError},
    constant_value::ConstantValue,
    types::Type,
};

#[derive(Debug, PartialEq, Eq)]
pub struct Owned {
    pub address: u16,
    pub value_type: Type,
    pub mutable: bool,
    pub address_pool: AddressPool,
}

impl Owned {
    pub fn borrow(&self) -> Variable {
        Variable::Borrow {
            address: self.address,
            value_type: self.value_type.clone(),
            mutable: self.mutable,
        }
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        self.address_pool
            .borrow_mut()
            .deallocate(self.address, self.value_type.size());
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Variable {
    Owned(Owned),
    Borrow {
        address: u16,
        value_type: Type,
        mutable: bool,
    },
}

impl Variable {
    pub fn is_owned(&self) -> bool {
        matches!(self, Variable::Owned(_))
    }
    pub fn address(&self) -> u16 {
        match self {
            Variable::Owned(owned) => owned.address,
            Variable::Borrow { address, .. } => *address,
        }
    }
    pub fn borrow(&self) -> Self {
        match self {
            Variable::Owned(owned) => owned.borrow(),
            Variable::Borrow {
                address,
                value_type,
                mutable,
            } => Variable::Borrow {
                address: *address,
                value_type: value_type.clone(),
                mutable: *mutable,
            },
        }
    }
    pub fn is_mutable(&self) -> bool {
        match self {
            Variable::Owned(owned) => owned.mutable,
            Variable::Borrow { mutable, .. } => *mutable,
        }
    }
    pub fn value_type(&self) -> Type {
        match self {
            Variable::Owned(owned) => owned.value_type.clone(),
            Variable::Borrow { value_type, .. } => value_type.clone(),
        }
    }
}

impl From<Owned> for Variable {
    fn from(value: Owned) -> Self {
        Variable::Owned(value)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Value {
    Constant(ConstantValue),
    Variable(Variable),
}

impl Value {
    pub fn constant<A: Into<ConstantValue>>(value: A) -> Self {
        Self::Constant(value.into())
    }

    pub fn new_borrow(address: u16, value_type: Type) -> Self {
        Self::Variable(Variable::Borrow {
            address,
            value_type,
            mutable: false,
        })
    }

    pub fn owned(owned: Owned) -> Self {
        Self::Variable(Variable::Owned(owned))
    }

    pub fn borrow(&self) -> Self {
        match self {
            Value::Constant(x) => x.clone().into(),
            Value::Variable(variable) => Value::Variable(variable.borrow()),
        }
    }

    pub fn is_mutable(&self) -> bool {
        match self {
            Value::Constant(_) => false,
            Value::Variable(variable) => variable.is_mutable(),
        }
    }

    pub fn value_type(&self) -> CompileResult<Type> {
        match self {
            Value::Constant(value) => value.value_type(),
            Value::Variable(variable) => Ok(variable.value_type()),
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

impl From<Variable> for Value {
    fn from(variable: Variable) -> Self {
        Self::Variable(variable)
    }
}

impl From<Owned> for Value {
    fn from(owned: Owned) -> Self {
        Self::Variable(owned.into())
    }
}

impl<A: Into<ConstantValue>> From<A> for Value {
    fn from(value: A) -> Self {
        Self::Constant(value.into())
    }
}
