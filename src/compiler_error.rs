use crate::types::Type;

#[derive(Debug)]
pub enum CompilerError {
    UndefinedVariable(String),
    AlreadyDefinedVariable(String),
    NoFreeAddresses,
    UnclosedLoop,
    NonAsciiString(String),
    MutableBorrowOfImmutableVariable(String),
    CantRegisterBorrowedValues(String),
    TypeError { expected: Type, actual: Type },
    InvalidReinterpretCast { original: Type, new: Type },
}

pub type CompileResult<A> = Result<A, CompilerError>;
