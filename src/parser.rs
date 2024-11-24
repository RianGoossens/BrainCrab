use crate::ast::{Expression, Program};

#[derive(Debug)]
pub enum ParseErrorMessage {
    NonAsciiProgram,
    UnexpectedEnd,
    FilterFailed,
    Expected(&'static str),
}

pub type ParseResult<A> = Result<A, ParseErrorMessage>;

pub struct Parser {
    characters: Vec<char>,
    char_pointer: usize,
}

impl Parser {
    pub fn new(script: &str) -> ParseResult<Self> {
        if !script.is_ascii() {
            Err(ParseErrorMessage::NonAsciiProgram)
        } else {
            Ok(Parser {
                characters: script.chars().collect(),
                char_pointer: 0,
            })
        }
    }

    pub fn error<A>(&self, message: ParseErrorMessage) -> ParseResult<A> {
        Err(message)
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
            self.error(ParseErrorMessage::UnexpectedEnd)
        }
    }

    fn digit(&mut self) -> ParseResult<u8> {
        let result = self.char()?;
        if let Some(digit) = result.to_digit(10) {
            Ok(digit as u8)
        } else {
            self.error(ParseErrorMessage::Expected("digit"))
        }
    }

    fn literal(&mut self, literal: &'static str) -> ParseResult<&'static str> {
        if !literal.is_ascii() {
            Err(ParseErrorMessage::NonAsciiProgram)
        } else {
            let literal_characters = literal.chars();
            for char in literal_characters {
                let parsed_char = self.char()?;
                if parsed_char != char {
                    return self.error(ParseErrorMessage::Expected(literal));
                }
            }
            Ok(literal)
        }
    }

    fn whitespace(&mut self) -> ParseResult<Vec<char>> {
        self.one_or_more(&|parser| {
            let char = parser.char()?;
            if char.is_whitespace() {
                Ok(char)
            } else {
                parser.error(ParseErrorMessage::Expected("whitespace"))
            }
        })
    }

    pub fn parse_u8_constant(&mut self) -> ParseResult<u8> {
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

    pub fn parse_char_constant(&mut self) -> ParseResult<u8> {
        self.literal("'")?;
        let char = self.char()?;
        self.literal("'")?;
        Ok(char as u8)
    }

    pub fn parse_constant<'a>(&mut self) -> ParseResult<Expression<'a>> {
        let u8_constant = self.optional(&Self::parse_u8_constant);
        if let Some(value) = u8_constant {
            Ok(value.into())
        } else {
            let char_constant = self.optional(&Self::parse_char_constant);
            if let Some(value) = char_constant {
                Ok(value.into())
            } else {
                self.error(ParseErrorMessage::Expected("constant value"))
            }
        }
    }

    pub fn parse_program<'a>(&mut self) -> ParseResult<Program<'a>> {
        todo!()
    }
}
