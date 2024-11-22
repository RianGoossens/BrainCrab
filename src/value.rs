use crate::compiler::AddressPool;

#[derive(PartialEq, Eq)]
pub struct Temp {
    pub address: u16,
    pub address_pool: AddressPool,
}

impl Drop for Temp {
    fn drop(&mut self) {
        self.address_pool.borrow_mut().push(self.address);
    }
}

#[derive(PartialEq, Eq)]
pub enum Variable<'a> {
    Named(&'a str),
    Borrow(u16),
    Temp(Temp),
}

impl<'a> Variable<'a> {
    pub fn is_temp(&self) -> bool {
        matches!(self, Variable::Temp(_))
    }
    pub fn is_borrowed(&self) -> bool {
        matches!(self, Variable::Borrow(_))
    }
    pub fn is_named(&self) -> bool {
        matches!(self, Variable::Named(_))
    }
}

#[derive(PartialEq, Eq)]
pub enum Value<'a> {
    Constant(u8),
    Variable(Variable<'a>),
}

impl<'a> Value<'a> {
    pub fn constant(value: u8) -> Self {
        Self::Constant(value)
    }

    pub fn named(name: &'a str) -> Self {
        Self::Variable(Variable::Named(name))
    }

    pub fn borrow(address: u16) -> Self {
        Self::Variable(Variable::Borrow(address))
    }

    pub fn temp(temp: Temp) -> Self {
        Self::Variable(Variable::Temp(temp))
    }
}
