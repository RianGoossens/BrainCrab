pub enum Expression<'a> {
    Constant(u8),
    Variable(&'a str),
    Add(Box<Expression<'a>>, Box<Expression<'a>>),
}

impl<'a> Expression<'a> {
    pub fn constant(value: u8) -> Self {
        Self::Constant(value)
    }
    pub fn variable(name: &'a str) -> Self {
        Self::Variable(name)
    }
}

pub enum Instruction<'a> {
    Define {
        name: &'a str,
        value: Expression<'a>,
    },
    Assign {
        name: &'a str,
        value: Expression<'a>,
    },
    AddAssign {
        name: &'a str,
        value: Expression<'a>,
    },
    SubAssign {
        name: &'a str,
        value: Expression<'a>,
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
