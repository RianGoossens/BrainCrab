#[derive(Debug, Clone)]
pub enum Expression<'a> {
    Constant(u8),
    Variable(&'a str),
    Add(Box<Expression<'a>>, Box<Expression<'a>>),
    Sub(Box<Expression<'a>>, Box<Expression<'a>>),
    Not(Box<Expression<'a>>),
    And(Box<Expression<'a>>, Box<Expression<'a>>),
    Or(Box<Expression<'a>>, Box<Expression<'a>>),
}

impl<'a> Expression<'a> {
    pub fn constant(value: u8) -> Self {
        Self::Constant(value)
    }
    pub fn variable(name: &'a str) -> Self {
        Self::Variable(name)
    }
    pub fn add(a: Expression<'a>, b: Expression<'a>) -> Self {
        Self::Add(Box::new(a), Box::new(b))
    }
    pub fn sub(a: Expression<'a>, b: Expression<'a>) -> Self {
        Self::Sub(Box::new(a), Box::new(b))
    }
    pub fn not(a: Expression<'a>) -> Self {
        Self::Not(Box::new(a))
    }
    pub fn and(a: Expression<'a>, b: Expression<'a>) -> Self {
        Self::And(Box::new(a), Box::new(b))
    }
    pub fn or(a: Expression<'a>, b: Expression<'a>) -> Self {
        Self::Or(Box::new(a), Box::new(b))
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
        predicate: Expression<'a>,
        body: Vec<Instruction<'a>>,
    },
    IfThenElse {
        predicate: Expression<'a>,
        if_body: Vec<Instruction<'a>>,
        else_body: Vec<Instruction<'a>>,
    },
}

pub struct Program<'a> {
    pub instructions: Vec<Instruction<'a>>,
}
