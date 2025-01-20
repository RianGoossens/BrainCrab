use std::borrow::Cow;

use crate::{types::Type, value::Value};

#[derive(Debug)]
pub enum CompilerError<'a> {
    UndefinedVariable(&'a str),
    AlreadyDefinedVariable(&'a str),
    NoFreeAddresses,
    UnclosedLoop,
    NonAsciiString(Cow<'a, str>),
    MutableBorrowOfImmutableVariable(Value),
    CantRegisterBorrowedValues(&'a str),
    TypeError {
        expected: Type,
        actual: Type,
    },
    InvalidReinterpretCast {
        original: Type,
        new: Type,
    },
    ArrayHasDifferentTypes {
        expected: Type,
        index: u16,
        actual: Type,
    },
    NotAnArray(Type),
}

pub type CompileResult<'a, A> = Result<A, CompilerError<'a>>;
