use crate::{allocator::BrainCrabAllocator, compiler::AddressPool};

#[derive(PartialEq, Eq)]
pub struct Owned {
    pub address: u16,
    pub address_pool: AddressPool,
    pub dirty: bool,
}

impl Owned {
    pub fn borrow(&self) -> Variable {
        Variable::Borrow(self.address)
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
    Borrow(u16),
}

impl Variable {
    pub fn is_owned(&self) -> bool {
        matches!(self, Variable::Owned(_))
    }
    pub fn address(&self) -> u16 {
        match self {
            Variable::Owned(owned) => owned.address,
            Variable::Borrow(address) => *address,
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

    pub fn borrow(address: u16) -> Self {
        Self::Variable(Variable::Borrow(address))
    }

    pub fn owned(owned: Owned) -> Self {
        Self::Variable(Variable::Owned(owned))
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
