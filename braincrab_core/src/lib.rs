use std::{
    fmt,
    io::{stdin, Read},
};

use proc_macro2::TokenStream;
use quote::quote;
use quote::ToTokens;

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
    Move(isize),
    Add(u8),
    Write,
    Read,
    Loop(Vec<BFTree>),
}

#[derive(Debug, Clone)]
pub struct BFProgram(pub Vec<BFTree>);

impl ToTokens for BFTree {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let tokenized = match self {
            BFTree::Move(offset) => quote! { BFTree::Move(#offset) },
            BFTree::Add(value) => quote! { BFTree::Add(#value) },
            BFTree::Write => quote! { BFTree::Write },
            BFTree::Read => quote! { BFTree::Read },
            BFTree::Loop(subtrees) => {
                let subtrees_tokens = subtrees.iter().map(|subtree| {
                    let mut subtree_tokens = TokenStream::new();
                    subtree.to_tokens(&mut subtree_tokens);
                    subtree_tokens
                });
                quote! { BFTree::Loop(vec![#(#subtrees_tokens),*]) }
            }
        };

        tokens.extend(tokenized);
    }
}

impl ToTokens for BFProgram {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let bftrees_tokens = self.0.iter().map(|tree| {
            let mut tree_tokens = TokenStream::new();
            tree.to_tokens(&mut tree_tokens);
            tree_tokens
        });

        tokens.extend(quote! {
            BFProgram(vec![#(#bftrees_tokens),*])
        });
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
                let loop_body = parse_bf_tokens_impl(tokens, index);
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

pub fn parse_bf_tokens(tokens: &[BFToken]) -> Result<BFProgram, BFParseError> {
    let mut index = 0;
    let result = parse_bf_tokens_impl(tokens, &mut index);
    if index != tokens.len() {
        Err(BFParseError::UnmatchedBrackets)
    } else {
        Ok(BFProgram(result))
    }
}

pub fn parse_bf(script: &str) -> Result<BFProgram, BFParseError> {
    parse_bf_tokens(&tokenize_bf(script))
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
