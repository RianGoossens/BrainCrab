use crate::value::Value;

pub enum Expression<'a> {
    Value(Value<'a>),
    Add(Box<Expression<'a>>, Box<Expression<'a>>),
}

pub enum Instruction<'a> {
    Define {
        name: &'a str,
        value: Value<'a>,
    },
    Assign {
        name: &'a str,
        value: Value<'a>,
    },
    AddAssign {
        name: &'a str,
        value: Value<'a>,
    },
    SubAssign {
        name: &'a str,
        value: Value<'a>,
    },
    Write {
        name: &'a str,
    },
    Read {
        name: &'a str,
    },
    WriteString {
        string: &'a str,
    },
    Scope {
        body: Vec<Instruction<'a>>,
    },
    While {
        predicate: &'a str,
        body: Vec<Instruction<'a>>,
    },
    IfThenElse {
        predicate: &'a str,
        if_body: Vec<Instruction<'a>>,
        else_body: Vec<Instruction<'a>>,
    },
}

pub struct Program<'a> {
    pub instructions: Vec<Instruction<'a>>,
}
