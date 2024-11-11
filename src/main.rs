use core::fmt;

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

pub fn tokenize_bf(text: &str) -> Vec<BFToken> {
    let mut result = vec![];
    for char in text.chars() {
        match char {
            '<' => result.push(BFToken::Left),
            '>' => result.push(BFToken::Right),
            '+' => result.push(BFToken::Inc),
            '-' => result.push(BFToken::Dec),
            '.' => result.push(BFToken::Write),
            ',' => result.push(BFToken::Read),
            '[' => result.push(BFToken::BeginLoop),
            ']' => result.push(BFToken::EndLoop),
            _ => {}
        }
    }
    result
}

#[derive(Debug, Clone)]
pub enum BFTree {
    Left,
    Right,
    Inc,
    Dec,
    Write,
    Read,
    Loop(Vec<BFTree>),
}

#[derive(Debug, Clone)]
pub struct BFProgram(pub Vec<BFTree>);

#[derive(Debug, Clone, Copy)]
pub enum BFParseError {
    UnmatchedBrackets,
}

impl fmt::Display for BFParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unmatched brackets")
    }
}

fn parse_bf_impl(tokens: &[BFToken], index: &mut usize) -> Vec<BFTree> {
    let mut result = vec![];

    while *index < tokens.len() {
        match tokens[*index] {
            BFToken::Left => result.push(BFTree::Left),
            BFToken::Right => result.push(BFTree::Right),
            BFToken::Inc => result.push(BFTree::Inc),
            BFToken::Dec => result.push(BFTree::Dec),
            BFToken::Write => result.push(BFTree::Write),
            BFToken::Read => result.push(BFTree::Read),
            BFToken::BeginLoop => {
                *index += 1;
                let loop_body = parse_bf_impl(tokens, index);
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

pub fn parse_bf(tokens: &[BFToken]) -> Result<BFProgram, BFParseError> {
    let mut index = 0;
    let result = parse_bf_impl(tokens, &mut index);
    if index != tokens.len() {
        Err(BFParseError::UnmatchedBrackets)
    } else {
        Ok(BFProgram(result))
    }
}

fn main() {
    let test_program = ">>>>+++-..,.<><<[fdsfasd+)_-[[]+>]]";
    let tokens = tokenize_bf(test_program);
    let program = parse_bf(&tokens).expect("Invalid program");
    println!("{program:?}");
}
