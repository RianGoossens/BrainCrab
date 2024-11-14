use std::{
    fmt,
    io::{stdin, Read},
};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BFToken {
    Left,
    Right,
    Inc,
    Dec,
    Write,
    Read,
    BeginLoop,
    EndLoop,
}

impl BFToken {
    pub fn to_char(&self) -> char {
        match self {
            BFToken::Left => '<',
            BFToken::Right => '>',
            BFToken::Inc => '+',
            BFToken::Dec => '-',
            BFToken::Write => '.',
            BFToken::Read => ',',
            BFToken::BeginLoop => '[',
            BFToken::EndLoop => ']',
        }
    }
    pub fn from_char(character: char) -> Option<Self> {
        match character {
            '<' => Some(BFToken::Left),
            '>' => Some(BFToken::Right),
            '+' => Some(BFToken::Inc),
            '-' => Some(BFToken::Dec),
            '.' => Some(BFToken::Write),
            ',' => Some(BFToken::Read),
            '[' => Some(BFToken::BeginLoop),
            ']' => Some(BFToken::EndLoop),
            _ => None,
        }
    }
}

pub fn tokenize_bf(text: &str) -> Vec<BFToken> {
    text.chars().flat_map(BFToken::from_char).collect()
}

pub fn stringify_bf_tokens(tokens: &[BFToken]) -> String {
    tokens.iter().map(BFToken::to_char).collect()
}

#[derive(Debug, Clone)]
pub enum BFTree {
    Move(isize),
    Add(u8),
    Write,
    Read,
    Loop(Vec<BFTree>),
}

impl BFTree {
    fn to_tokens_impl(&self, result: &mut Vec<BFToken>) {
        match self {
            BFTree::Move(amount) => result.extend(if *amount < 0 {
                [BFToken::Left].repeat(-amount as usize)
            } else {
                [BFToken::Right].repeat(*amount as usize)
            }),
            BFTree::Add(amount) => result.extend(if *amount > 127 {
                [BFToken::Dec].repeat((255 - amount + 1) as usize)
            } else {
                [BFToken::Inc].repeat(*amount as usize)
            }),
            BFTree::Write => result.push(BFToken::Write),
            BFTree::Read => result.push(BFToken::Read),
            BFTree::Loop(vec) => {
                result.push(BFToken::BeginLoop);
                vec.iter().for_each(|tree| tree.to_tokens_impl(result));
                result.push(BFToken::EndLoop);
            }
        }
    }
    pub fn to_tokens(&self) -> Vec<BFToken> {
        let mut result = vec![];
        self.to_tokens_impl(&mut result);
        result
    }
}

#[derive(Debug, Clone)]
pub struct BFProgram(pub Vec<BFTree>);

impl BFProgram {
    pub fn new() -> Self {
        BFProgram(vec![])
    }

    pub fn append(&mut self, mut rhs: BFProgram) {
        self.0.append(&mut rhs.0);
    }

    fn parse_bf_tokens_impl(tokens: &[BFToken], index: &mut usize) -> Vec<BFTree> {
        let mut result = vec![];

        while *index < tokens.len() {
            match tokens[*index] {
                BFToken::Left => {
                    if let Some(BFTree::Move(movement)) = result.last_mut() {
                        *movement -= 1;
                    } else {
                        result.push(BFTree::Move(-1));
                    }
                }
                BFToken::Right => {
                    if let Some(BFTree::Move(movement)) = result.last_mut() {
                        *movement += 1;
                    } else {
                        result.push(BFTree::Move(1));
                    }
                }
                BFToken::Inc => {
                    if let Some(BFTree::Add(addition)) = result.last_mut() {
                        *addition = addition.wrapping_add(1);
                    } else {
                        result.push(BFTree::Add(1));
                    }
                }
                BFToken::Dec => {
                    if let Some(BFTree::Add(addition)) = result.last_mut() {
                        *addition = addition.wrapping_sub(1);
                    } else {
                        result.push(BFTree::Add(255));
                    }
                }
                BFToken::Write => result.push(BFTree::Write),
                BFToken::Read => result.push(BFTree::Read),
                BFToken::BeginLoop => {
                    *index += 1;
                    let loop_body = Self::parse_bf_tokens_impl(tokens, index);
                    result.push(BFTree::Loop(loop_body));
                }
                BFToken::EndLoop => {
                    break;
                }
            }
            *index += 1;
        }

        result
    }

    pub fn parse_bf_tokens(tokens: &[BFToken]) -> Result<Self, BFParseError> {
        let mut index = 0;
        let result = Self::parse_bf_tokens_impl(tokens, &mut index);
        if index != tokens.len() {
            Err(BFParseError::UnmatchedBrackets)
        } else {
            Ok(Self(result))
        }
    }

    pub fn parse(script: &str) -> Result<Self, BFParseError> {
        Self::parse_bf_tokens(&tokenize_bf(script))
    }
    fn to_bf_tokens_impl(&self, result: &mut Vec<BFToken>) {
        self.0.iter().for_each(|tree| tree.to_tokens_impl(result));
    }
    pub fn to_bf_tokens(&self) -> Vec<BFToken> {
        let mut result = vec![];
        self.to_bf_tokens_impl(&mut result);
        result
    }
    pub fn to_string(&self) -> String {
        let tokens = self.to_bf_tokens();
        stringify_bf_tokens(&tokens)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BFParseError {
    UnmatchedBrackets,
}

impl fmt::Display for BFParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unmatched brackets")
    }
}

pub struct BFInterpreter {
    tape: [u8; 30000],
    pointer: usize,
}

impl Default for BFInterpreter {
    fn default() -> Self {
        Self {
            tape: [0; 30000],
            pointer: 0,
        }
    }
}

impl BFInterpreter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run_instructions(&mut self, instructions: &[BFTree]) {
        for tree in instructions {
            match tree {
                BFTree::Move(amount) => self.pointer = ((self.pointer as isize) + amount) as usize,
                BFTree::Add(amount) => {
                    self.tape[self.pointer] = self.tape[self.pointer].wrapping_add(*amount)
                }
                BFTree::Write => print!("{}", self.tape[self.pointer] as char),
                BFTree::Read => {
                    let mut byte = [0_u8];
                    {
                        let mut stdin_handle = stdin().lock();
                        stdin_handle.read_exact(&mut byte).unwrap();
                        if byte[0] == 13 {
                            stdin_handle.read_exact(&mut byte).unwrap();
                        }
                    }
                    self.tape[self.pointer] = byte[0];
                }
                BFTree::Loop(instructions) => loop {
                    if self.tape[self.pointer] == 0 {
                        break;
                    }
                    self.run_instructions(instructions);
                },
            }
        }
    }

    pub fn run(&mut self, program: &BFProgram) {
        self.run_instructions(&program.0);
    }
}
