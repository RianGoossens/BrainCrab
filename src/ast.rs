#[derive(Debug, Clone)]
pub enum Expression<'a> {
    Constant(u8),
    Variable(&'a str),

    Add(Box<Expression<'a>>, Box<Expression<'a>>),
    Sub(Box<Expression<'a>>, Box<Expression<'a>>),

    Not(Box<Expression<'a>>),
    And(Box<Expression<'a>>, Box<Expression<'a>>),
    Or(Box<Expression<'a>>, Box<Expression<'a>>),

    Equals(Box<Expression<'a>>, Box<Expression<'a>>),
    NotEquals(Box<Expression<'a>>, Box<Expression<'a>>),
    LessThanEquals(Box<Expression<'a>>, Box<Expression<'a>>),
    GreaterThanEquals(Box<Expression<'a>>, Box<Expression<'a>>),
    LessThan(Box<Expression<'a>>, Box<Expression<'a>>),
    GreaterThan(Box<Expression<'a>>, Box<Expression<'a>>),
}

impl<'a> Expression<'a> {
    pub fn constant(value: u8) -> Self {
        Self::Constant(value)
    }
    pub fn variable(name: &'a str) -> Self {
        Self::Variable(name)
    }
    pub fn new_add(a: Expression<'a>, b: Expression<'a>) -> Self {
        Self::Add(Box::new(a), Box::new(b))
    }
    pub fn new_sub(a: Expression<'a>, b: Expression<'a>) -> Self {
        Self::Sub(Box::new(a), Box::new(b))
    }
    pub fn new_not(a: Expression<'a>) -> Self {
        Self::Not(Box::new(a))
    }
    pub fn new_and(a: Expression<'a>, b: Expression<'a>) -> Self {
        Self::And(Box::new(a), Box::new(b))
    }
    pub fn new_or(a: Expression<'a>, b: Expression<'a>) -> Self {
        Self::Or(Box::new(a), Box::new(b))
    }
    pub fn new_equals(a: Expression<'a>, b: Expression<'a>) -> Self {
        Self::Equals(Box::new(a), Box::new(b))
    }
    pub fn new_not_equals(a: Expression<'a>, b: Expression<'a>) -> Self {
        Self::NotEquals(Box::new(a), Box::new(b))
    }
    pub fn new_less_than_equals(a: Expression<'a>, b: Expression<'a>) -> Self {
        Self::LessThanEquals(Box::new(a), Box::new(b))
    }
    pub fn new_greater_than_equals(a: Expression<'a>, b: Expression<'a>) -> Self {
        Self::GreaterThanEquals(Box::new(a), Box::new(b))
    }
    pub fn new_less_than(a: Expression<'a>, b: Expression<'a>) -> Self {
        Self::LessThan(Box::new(a), Box::new(b))
    }
    pub fn new_greater_than(a: Expression<'a>, b: Expression<'a>) -> Self {
        Self::GreaterThan(Box::new(a), Box::new(b))
    }
}

impl<'a> From<u8> for Expression<'a> {
    fn from(value: u8) -> Self {
        Expression::constant(value)
    }
}

impl<'a> From<&'a str> for Expression<'a> {
    fn from(value: &'a str) -> Self {
        Expression::variable(value)
    }
}

#[derive(Debug)]
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
    Print {
        string: String,
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

#[derive(Debug)]
pub struct Program<'a> {
    pub instructions: Vec<Instruction<'a>>,
}
