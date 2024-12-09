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
}

pub type CompileResult<A> = Result<A, CompilerError>;
