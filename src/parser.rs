use crate::ast::Program;

#[derive(Debug)]
pub enum ParseError {
    NonAsciiProgram,
    UnexpectedEnd,
    FilterFailed,
    NotFound(&'static str),
}

pub type ParseResult<A> = Result<A, ParseError>;

pub struct Parser {
    characters: Vec<char>,
    char_pointer: usize,
}

impl Parser {
    pub fn new(script: &str) -> ParseResult<Self> {
        if !script.is_ascii() {
            Err(ParseError::NonAsciiProgram)
        } else {
            Ok(Parser {
                characters: script.chars().collect(),
                char_pointer: 0,
            })
        }
    }

    fn optional<A, P: Fn(&mut Parser) -> ParseResult<A>>(
        &mut self,
        parse_function: &P,
    ) -> Option<A> {
        let current_char_pointer = self.char_pointer;
        if let Ok(result) = parse_function(self) {
            Some(result)
        } else {
            self.char_pointer = current_char_pointer;
            None
        }
    }

    fn repeat<A, P: Fn(&mut Parser) -> ParseResult<A>>(&mut self, parse_function: &P) -> Vec<A> {
        let mut result = vec![];
        while let Some(element) = self.optional(parse_function) {
            result.push(element);
        }
        result
    }

    fn one_or_more<A, P: Fn(&mut Parser) -> ParseResult<A>>(
        &mut self,
        parse_function: &P,
    ) -> ParseResult<Vec<A>> {
        let mut result = vec![parse_function(self)?];
        while let Some(element) = self.optional(parse_function) {
            result.push(element);
        }
        Ok(result)
    }

    fn char(&mut self) -> ParseResult<char> {
        if self.char_pointer < self.characters.len() {
            let result = self.characters[self.char_pointer];
            self.char_pointer += 1;
            Ok(result)
        } else {
            Err(ParseError::UnexpectedEnd)
        }
    }

    fn filter<A, P: Fn(&mut Self) -> ParseResult<A>, F: Fn(&A) -> bool>(
        &mut self,
        parser_function: P,
        filter_function: F,
    ) -> ParseResult<A> {
        let parsed = parser_function(self)?;
        if filter_function(&parsed) {
            Ok(parsed)
        } else {
            Err(ParseError::FilterFailed)
        }
    }

    fn digit(&mut self) -> ParseResult<u8> {
        let result = self.filter(Parser::char, |char| char.is_ascii_digit())?;
        Ok(result.to_digit(10).unwrap() as u8)
    }

    fn literal(&mut self, literal: &'static str) -> ParseResult<&'static str> {
        if !literal.is_ascii() {
            Err(ParseError::NonAsciiProgram)
        } else {
            let literal_characters = literal.chars();
            for char in literal_characters {
                self.filter(|parser| parser.char(), |value| *value == char)?;
            }
            Ok(literal)
        }
    }

    fn whitespace(&mut self) -> ParseResult<()> {
        self.one_or_more(&|parser| {
            parser.filter(|parser| parser.char(), |char| char.is_whitespace())
        })?;
        Ok(())
    }

    pub fn parse_u8(&mut self) -> ParseResult<u8> {
        let mut number = self.digit()?;
        if let Some(digit) = self.optional(&Self::digit) {
            number *= 10;
            number += digit;
        }
        if let Some(digit) = self.optional(&Self::digit) {
            number *= 10;
            number += digit;
        }
        Ok(number)
    }

    pub fn parse_program<'a>(&mut self) -> ParseResult<Program<'a>> {
        todo!()
    }
}
