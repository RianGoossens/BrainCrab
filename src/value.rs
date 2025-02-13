use crate::{
    compiler_error::{CompileResult, CompilerError},
    types::Type,
};

#[derive(Debug, PartialEq, Eq)]
pub struct Value {
    pub addresses: Vec<u16>,
    pub value_type: Type,
    owned: bool,
    pub mutable: bool,
}

impl Value {
    pub fn new(addresses: Vec<u16>, value_type: Type, mutable: bool) -> Self {
        assert!(addresses.len() == value_type.size() as usize);
        Self {
            addresses,
            value_type,
            owned: true,
            mutable,
        }
    }

    pub fn borrow(&self) -> Self {
        Self {
            addresses: self.addresses.clone(),
            value_type: self.value_type.clone(),
            owned: false,
            mutable: self.mutable,
        }
    }

    pub fn is_owned(&self) -> bool {
        self.owned
    }

    pub fn is_borrowed(&self) -> bool {
        !self.is_owned()
    }

    pub fn borrow_slice(&self, start_index: u16, end_index: u16, slice_type: Type) -> Self {
        let length = end_index - start_index;
        assert!(length == slice_type.size());
        Self {
            addresses: self.addresses[start_index as usize..end_index as usize].into(),
            value_type: slice_type,
            owned: false,
            mutable: self.mutable,
        }
    }

    pub fn type_check<'a>(&self, expected: &Type) -> CompileResult<'a, ()> {
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

    pub fn data(&self) -> Vec<Value> {
        todo!()
    }

    pub fn address(&self) -> u16 {
        assert!(self.addresses.len() == 1);
        self.addresses[0]
    }

    pub fn size(&self) -> u16 {
        self.value_type.size()
    }
}
