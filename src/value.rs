use crate::{allocator::BrainCrabAllocator, compiler::AddressPool};

#[derive(PartialEq, Eq)]
pub struct Owned {
    pub address: u16,
    pub address_pool: AddressPool,
    pub mutable: bool,
}

impl Owned {
    pub fn borrow(&self) -> Variable {
        Variable::Borrow {
            address: self.address,
            mutable: self.mutable,
        }
    }
}

impl Drop for Owned {
    fn drop(&mut self) {
        self.address_pool.borrow_mut().deallocate(self.address);
    }
}

#[derive(PartialEq, Eq)]
pub enum Variable {
    Owned(Owned),
    Borrow { address: u16, mutable: bool },
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
            Variable::Borrow { address, mutable } => Variable::Borrow {
                address: *address,
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
}

impl From<Owned> for Variable {
    fn from(value: Owned) -> Self {
        Variable::Owned(value)
    }
}

#[derive(PartialEq, Eq)]
pub enum Value {
    Constant(u8),
    Variable(Variable),
}

impl Value {
    pub fn constant(value: u8) -> Self {
        Self::Constant(value)
    }

    pub fn new_borrow(address: u16) -> Self {
        Self::Variable(Variable::Borrow {
            address,
            mutable: false,
        })
    }

    pub fn owned(owned: Owned) -> Self {
        Self::Variable(Variable::Owned(owned))
    }

    pub fn borrow(&self) -> Self {
        match self {
            Value::Constant(x) => (*x).into(),
            Value::Variable(variable) => Value::Variable(variable.borrow()),
        }
    }

    pub fn is_mutable(&self) -> bool {
        match self {
            Value::Constant(_) => false,
            Value::Variable(variable) => variable.is_mutable(),
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

impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Self::Constant(value)
    }
}
