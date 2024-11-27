use std::{collections::BTreeSet, fmt::Display};

use crate::ast::{Expression, Instruction, Program};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParseErrorMessage {
    NonAsciiProgram,
    UnexpectedEnd,
    Expected(&'static str),
    IgnoreError,
}

impl Display for ParseErrorMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseErrorMessage::NonAsciiProgram => write!(f, "Not a valid ASCII program."),
            ParseErrorMessage::UnexpectedEnd => write!(f, "Unexpected EOF."),
            ParseErrorMessage::Expected(expected) => write!(f, "Expected {expected}"),
            ParseErrorMessage::IgnoreError => write!(
                f,
                "This triggered an error that will be shown from somewhere else."
            ),
        }
    }
}

#[derive(Debug)]
pub struct ParseError<'a> {
    messages: Vec<ParseErrorMessage>,
    string: &'a str,
    index: usize,
}

impl<'a> Display for ParseError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut line_start = 0;
        let mut line_end = usize::MAX;
        for (i, c) in self.string.char_indices() {
            if c == '\n' {
                if i <= self.index {
                    line_start = i;
                } else {
                    line_end = i;
                    break;
                }
            }
        }
        if line_end == usize::MAX {
            line_end = self.string.len();
        }
        let index_on_line = self.index - line_start;

        writeln!(f, "{}", &self.string[line_start..line_end])?;
        for _ in 0..index_on_line - 1 {
            write!(f, " ")?;
        }
        writeln!(f, "╥")?;
        let unique_messages: BTreeSet<_> = self.messages.iter().collect();
        for (i, message) in unique_messages.iter().enumerate() {
            for _ in 0..index_on_line - 1 {
                write!(f, " ")?;
            }
            if i < unique_messages.len() - 1 {
                writeln!(f, "╠═► {}", message)?;
            } else {
                writeln!(f, "╚═► {}", message)?;
            }
        }
        Ok(())
    }
}

pub struct Parsed<'a, A> {
    pub value: A,
    pub span: &'a str,
    pub start: usize,
    pub len: usize,
}

impl<'a, A> Parsed<'a, A> {
    pub fn map<B, F: Fn(A) -> B>(self, f: F) -> Parsed<'a, B> {
        Parsed {
            value: f(self.value),
            span: self.span,
            start: self.start,
            len: self.len,
        }
    }
    pub fn with<B>(self, value: B) -> Parsed<'a, B> {
        Parsed {
            value,
            span: self.span,
            start: self.start,
            len: self.len,
        }
    }
}

#[derive(Debug)]
pub enum BinaryOperator {
    Add,
    Sub,
    And,
    Or,
    Eq,
    Neq,
    Lt,
    Gt,
    Leq,
    Geq,
}

impl BinaryOperator {
    pub fn create_expression<'a>(&self, a: Expression<'a>, b: Expression<'a>) -> Expression<'a> {
        match self {
            BinaryOperator::Add => Expression::add(a, b),
            BinaryOperator::Sub => Expression::sub(a, b),
            BinaryOperator::And => Expression::and(a, b),
            BinaryOperator::Or => Expression::or(a, b),
            BinaryOperator::Eq => Expression::equals(a, b),
            BinaryOperator::Neq => Expression::not_equals(a, b),
            BinaryOperator::Lt => Expression::less_than(a, b),
            BinaryOperator::Gt => Expression::greater_than(a, b),
            BinaryOperator::Leq => Expression::less_than_equals(a, b),
            BinaryOperator::Geq => Expression::greater_than_equals(a, b),
        }
    }

    pub fn precedence(&self) -> u8 {
        match self {
            BinaryOperator::Add => 4,
            BinaryOperator::Sub => 4,
            BinaryOperator::Lt => 6,
            BinaryOperator::Gt => 6,
            BinaryOperator::Leq => 6,
            BinaryOperator::Geq => 6,
            BinaryOperator::Eq => 7,
            BinaryOperator::Neq => 7,
            BinaryOperator::And => 8,
            BinaryOperator::Or => 9,
        }
    }
}

#[derive(Debug)]
pub enum ExpressionParseTree<'a> {
    Leaf(Expression<'a>),
    Branch(
        BinaryOperator,
        Box<ExpressionParseTree<'a>>,
        Box<ExpressionParseTree<'a>>,
    ),
}

impl<'a> ExpressionParseTree<'a> {
    pub fn leaf(expression: Expression<'a>) -> Self {
        Self::Leaf(expression)
    }
    pub fn branch(
        operator: BinaryOperator,
        a: ExpressionParseTree<'a>,
        b: ExpressionParseTree<'a>,
    ) -> Self {
        Self::Branch(operator, Box::new(a), Box::new(b))
    }
    pub fn extend(self, new_operator: BinaryOperator, rhs: Expression<'a>) -> Self {
        let rhs = Self::leaf(rhs);
        match self {
            ExpressionParseTree::Leaf(_) => Self::branch(new_operator, self, rhs),
            ExpressionParseTree::Branch(current_operator, a, b) => {
                if new_operator.precedence() > current_operator.precedence() {
                    Self::branch(new_operator, Self::branch(current_operator, *a, *b), rhs)
                } else {
                    Self::branch(current_operator, *a, Self::branch(new_operator, *b, rhs))
                }
            }
        }
    }
    pub fn into_expression(self) -> Expression<'a> {
        match self {
            ExpressionParseTree::Leaf(expression) => expression,
            ExpressionParseTree::Branch(binary_operator, a, b) => {
                let a = a.into_expression();
                let b = b.into_expression();
                binary_operator.create_expression(a, b)
            }
        }
    }
}

pub type ParseResult<'a, A> = Result<Parsed<'a, A>, ParseError<'a>>;

pub struct Parser {
    index: usize,
    longest_parse: usize,
    longest_parse_error: Vec<ParseErrorMessage>,
}

type SubParser<'a, A> = dyn Fn(&mut Parser, &'a str) -> ParseResult<'a, A>;

impl Parser {
    pub fn new() -> Self {
        Self {
            index: 0,
            longest_parse: 0,
            longest_parse_error: vec![],
        }
    }

    pub fn success<'a, A>(
        &mut self,
        string: &'a str,
        value: A,
        start: usize,
        len: usize,
    ) -> ParseResult<'a, A> {
        let span = &string[start..start + len];
        self.index = start + len;
        Ok(Parsed {
            value,
            span,
            start,
            len,
        })
    }

    pub fn error<'a, A>(
        &mut self,
        string: &'a str,
        message: ParseErrorMessage,
    ) -> ParseResult<'a, A> {
        if message != ParseErrorMessage::IgnoreError {
            match self.index.cmp(&self.longest_parse) {
                std::cmp::Ordering::Equal => self.longest_parse_error.push(message),
                std::cmp::Ordering::Greater => {
                    self.longest_parse = self.index;
                    self.longest_parse_error = vec![message];
                }
                _ => {}
            }
        }
        Err(ParseError {
            messages: self.longest_parse_error.clone(),
            string,
            index: self.longest_parse,
        })
    }

    fn optional<'a, A, P: Fn(&mut Self, &'a str) -> ParseResult<'a, A>>(
        &mut self,
        string: &'a str,
        parse_function: P,
    ) -> ParseResult<'a, Option<A>> {
        let start_location = self.index;
        if let Ok(result) = parse_function(self, string) {
            Ok(result.map(|x| Some(x)))
        } else {
            self.success(string, None, start_location, 0)
        }
    }

    fn filter<'a, A, P, F>(
        &mut self,
        string: &'a str,
        parse_function: P,
        filter_function: F,
        error_message: ParseErrorMessage,
    ) -> ParseResult<'a, A>
    where
        P: Fn(&mut Self, &'a str) -> ParseResult<'a, A>,
        F: Fn(&A) -> bool,
    {
        let start_location = self.index;
        let value = parse_function(self, string)?.value;
        if filter_function(&value) {
            self.success(string, value, start_location, self.index - start_location)
        } else {
            self.index = start_location;
            self.error(string, error_message)
        }
    }

    fn eof<'a>(&mut self, string: &'a str) -> ParseResult<'a, ()> {
        if self.index == string.len() {
            self.success(string, (), self.index, 0)
        } else {
            self.error(string, ParseErrorMessage::Expected("EOF"))
        }
    }

    fn repeat<'a, A, P: Fn(&mut Self, &'a str) -> ParseResult<'a, A>>(
        &mut self,
        string: &'a str,
        parse_function: P,
    ) -> ParseResult<'a, Vec<A>> {
        let start_location = self.index;
        let mut result = vec![];
        while let Parsed {
            value: Some(element),
            ..
        } = self.optional(string, &parse_function)?
        {
            result.push(element);
        }
        self.success(string, result, start_location, self.index - start_location)
    }

    fn one_or_more<'a, A, P: Fn(&mut Self, &'a str) -> ParseResult<'a, A>>(
        &mut self,
        string: &'a str,
        parse_function: P,
    ) -> ParseResult<'a, Vec<A>> {
        let start_location = self.index;
        let first_value = parse_function(self, string)?;
        let mut result = vec![first_value.value];
        while let Parsed {
            value: Some(element),
            ..
        } = self.optional(string, &parse_function)?
        {
            result.push(element);
        }
        self.success(string, result, start_location, self.index - start_location)
    }

    pub fn one_of<'a, A>(
        &mut self,
        string: &'a str,
        parsers: &[&SubParser<'a, A>],
    ) -> ParseResult<'a, A> {
        let start_index = self.index;
        for parser in parsers {
            if let Some(result) = self.optional(string, parser)?.value {
                return self.success(string, result, start_index, self.index - start_index);
            }

            self.index = start_index;
        }
        self.error(string, ParseErrorMessage::IgnoreError)
    }

    fn char<'a>(&mut self, string: &'a str) -> ParseResult<'a, char> {
        let start_index = self.index;
        if start_index < string.len() {
            let result = string.as_bytes()[start_index] as char;
            self.success(string, result, start_index, 1)
        } else {
            self.error(string, ParseErrorMessage::UnexpectedEnd)
        }
    }

    fn digit<'a>(&mut self, string: &'a str) -> ParseResult<'a, u8> {
        let start_location = self.index;
        let result = self
            .filter(
                string,
                Self::char,
                |x| x.is_ascii_digit(),
                ParseErrorMessage::Expected("digit"),
            )?
            .value;
        let digit = result
            .to_digit(10)
            .expect("character should be a digit since we used a filter parser.");
        self.success(
            string,
            digit as u8,
            start_location,
            self.index - start_location,
        )
    }

    fn escaped_char<'a>(&mut self, string: &'a str) -> ParseResult<'a, char> {
        let start_location = self.index;
        self.literal(string, "\\")?;
        let result = self.char(string)?.value;
        let result = match result {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            _ => result,
        };
        self.success(string, result, start_location, self.index - start_location)
    }

    fn literal<'a>(
        &mut self,
        string: &'a str,
        literal: &'static str,
    ) -> ParseResult<'a, &'static str> {
        assert!(
            literal.is_ascii(),
            "Literal provided for parsing is not ascii: \"{literal}\""
        );
        let start_location = self.index;
        if string[start_location..].starts_with(literal) {
            self.success(string, literal, start_location, literal.len())
        } else {
            self.error(string, ParseErrorMessage::Expected(literal))
        }
    }

    fn whitespace<'a>(&mut self, string: &'a str) -> ParseResult<'a, Vec<char>> {
        self.one_or_more(string, |parser, string| {
            parser.filter(
                string,
                Self::char,
                |x| x.is_whitespace(),
                ParseErrorMessage::Expected("whitespace"),
            )
        })
    }

    pub fn parse_u8_constant<'a>(&mut self, string: &'a str) -> ParseResult<'a, u8> {
        let start_location = self.index;
        let mut number = self.digit(string)?.value;
        let mut len = 1;
        if let Some(digit) = self.optional(string, Self::digit)?.value {
            number *= 10;
            number += digit;
            len += 1;
        }
        if let Some(digit) = self.optional(string, Self::digit)?.value {
            number *= 10;
            number += digit;
            len += 1;
        }
        self.success(string, number, start_location, len)
    }

    pub fn parse_char_constant<'a>(&mut self, string: &'a str) -> ParseResult<'a, u8> {
        let start_location = self.index;
        self.literal(string, "'")?;
        let char = self
            .one_of(string, &[&Self::escaped_char, &Self::char])?
            .value;
        self.literal(string, "'")?;
        self.success(
            string,
            char as u8,
            start_location,
            self.index - start_location,
        )
    }

    pub fn parse_constant<'a>(&mut self, string: &'a str) -> ParseResult<'a, Expression<'a>> {
        self.one_of(
            string,
            &[&Self::parse_u8_constant, &Self::parse_char_constant],
        )
        .map(|x| x.map(|x| x.into()))
    }

    pub fn parse_variable_name<'a>(&mut self, string: &'a str) -> ParseResult<'a, &'a str> {
        let start_index = self.index;
        self.one_or_more(string, |p, s| {
            p.filter(
                s,
                Self::char,
                |x| x.is_ascii_alphabetic() || *x == '_',
                ParseErrorMessage::Expected("variable name (alphabetic or _ ascii characters)"),
            )
        })?;
        let value = &string[start_index..self.index];
        self.success(string, value, start_index, self.index - start_index)
    }

    pub fn parse_variable<'a>(&mut self, string: &'a str) -> ParseResult<'a, Expression<'a>> {
        let result = self.parse_variable_name(string)?;
        Ok(result.map(|x| x.into()))
    }

    pub fn parse_parens<'a>(&mut self, string: &'a str) -> ParseResult<'a, Expression<'a>> {
        let start_index = self.index;
        self.literal(string, "(")?;
        self.optional(string, Self::whitespace)?;
        let result = self.parse_expression(string)?.value;
        self.optional(string, Self::whitespace)?;
        self.literal(string, ")")?;
        self.success(string, result, start_index, self.index - start_index)
    }

    pub fn parse_leaf_expression<'a>(
        &mut self,
        string: &'a str,
    ) -> ParseResult<'a, Expression<'a>> {
        self.one_of(
            string,
            &[
                &Self::parse_constant,
                &Self::parse_variable,
                &Self::parse_parens,
                &Self::parse_not_expression,
            ],
        )
    }

    pub fn parse_not_expression<'a>(&mut self, string: &'a str) -> ParseResult<'a, Expression<'a>> {
        let start_index = self.index;
        self.literal(string, "!")?;
        self.optional(string, Self::whitespace)?;
        let inner = self.parse_leaf_expression(string)?.value;
        let result = Expression::not(inner);
        self.success(string, result, start_index, self.index - start_index)
    }

    pub fn parse_binary_operator<'a>(
        &mut self,
        string: &'a str,
    ) -> ParseResult<'a, BinaryOperator> {
        self.one_of(
            string,
            &[
                &|p, s| Ok(p.literal(s, "+")?.with(BinaryOperator::Add)),
                &|p, s| Ok(p.literal(s, "-")?.with(BinaryOperator::Sub)),
                &|p, s| Ok(p.literal(s, "&")?.with(BinaryOperator::And)),
                &|p, s| Ok(p.literal(s, "|")?.with(BinaryOperator::Or)),
                &|p, s| Ok(p.literal(s, "==")?.with(BinaryOperator::Eq)),
                &|p, s| Ok(p.literal(s, "!=")?.with(BinaryOperator::Neq)),
                &|p, s| Ok(p.literal(s, "<=")?.with(BinaryOperator::Leq)),
                &|p, s| Ok(p.literal(s, ">=")?.with(BinaryOperator::Geq)),
                &|p, s| Ok(p.literal(s, "<")?.with(BinaryOperator::Lt)),
                &|p, s| Ok(p.literal(s, ">")?.with(BinaryOperator::Gt)),
            ],
        )
    }

    pub fn parse_binary_expression<'a>(
        &mut self,
        string: &'a str,
    ) -> ParseResult<'a, Expression<'a>> {
        let start_index = self.index;
        let first_expression = self.parse_leaf_expression(string)?.value;
        let mut parse_tree = ExpressionParseTree::leaf(first_expression);
        while let Some((operator, next_expression)) = self
            .optional(string, |p, s| {
                let start_index = p.index;
                p.optional(s, Self::whitespace)?;
                let operator = p.parse_binary_operator(s)?.value;
                p.optional(s, Self::whitespace)?;
                let next_expression = p.parse_leaf_expression(string)?.value;
                p.success(
                    string,
                    (operator, next_expression),
                    start_index,
                    p.index - start_index,
                )
            })?
            .value
        {
            parse_tree = parse_tree.extend(operator, next_expression);
        }
        let result = parse_tree.into_expression();
        self.success(string, result, start_index, self.index - start_index)
    }

    pub fn parse_expression<'a>(&mut self, string: &'a str) -> ParseResult<'a, Expression<'a>> {
        self.parse_binary_expression(string)
    }

    pub fn parse_definition<'a>(&mut self, string: &'a str) -> ParseResult<'a, Instruction<'a>> {
        let start_location = self.index;
        self.literal(string, "let")?;
        self.whitespace(string)?;
        let name = self.parse_variable_name(string)?.value;
        self.optional(string, Self::whitespace)?;
        self.literal(string, "=")?;
        self.optional(string, Self::whitespace)?;
        let expression = self.parse_expression(string)?.value;
        self.optional(string, Self::whitespace)?;
        self.literal(string, ";")?;
        let result = Instruction::Define {
            name,
            value: expression,
        };
        self.success(string, result, start_location, self.index - start_location)
    }

    pub fn parse_assignment<'a>(&mut self, string: &'a str) -> ParseResult<'a, Instruction<'a>> {
        let start_location = self.index;
        let name = self.parse_variable_name(string)?.value;
        self.optional(string, Self::whitespace)?;
        self.literal(string, "=")?;
        self.optional(string, Self::whitespace)?;
        let expression = self.parse_expression(string)?.value;
        self.optional(string, Self::whitespace)?;
        self.literal(string, ";")?;
        let result = Instruction::Assign {
            name,
            value: expression,
        };
        self.success(string, result, start_location, self.index - start_location)
    }

    pub fn parse_add_assignment<'a>(
        &mut self,
        string: &'a str,
    ) -> ParseResult<'a, Instruction<'a>> {
        let start_location = self.index;
        let name = self.parse_variable_name(string)?.value;
        self.optional(string, Self::whitespace)?;
        self.literal(string, "+=")?;
        self.optional(string, Self::whitespace)?;
        let expression = self.parse_expression(string)?.value;
        self.optional(string, Self::whitespace)?;
        self.literal(string, ";")?;
        let result = Instruction::AddAssign {
            name,
            value: expression,
        };
        self.success(string, result, start_location, self.index - start_location)
    }

    pub fn parse_sub_assignment<'a>(
        &mut self,
        string: &'a str,
    ) -> ParseResult<'a, Instruction<'a>> {
        let start_location = self.index;
        let name = self.parse_variable_name(string)?.value;
        self.optional(string, Self::whitespace)?;
        self.literal(string, "-=")?;
        self.optional(string, Self::whitespace)?;
        let expression = self.parse_expression(string)?.value;
        self.optional(string, Self::whitespace)?;
        self.literal(string, ";")?;
        let result = Instruction::SubAssign {
            name,
            value: expression,
        };
        self.success(string, result, start_location, self.index - start_location)
    }

    pub fn parse_read<'a>(&mut self, string: &'a str) -> ParseResult<'a, Instruction<'a>> {
        let start_location = self.index;
        self.literal(string, "read")?;
        self.optional(string, Self::whitespace)?;
        self.literal(string, "(")?;
        self.optional(string, Self::whitespace)?;
        let variable_name = self.parse_variable_name(string)?.value;
        self.optional(string, Self::whitespace)?;
        self.literal(string, ")")?;
        self.optional(string, Self::whitespace)?;
        self.literal(string, ";")?;
        let result = Instruction::Read {
            name: variable_name,
        };
        self.success(string, result, start_location, self.index - start_location)
    }

    pub fn parse_write<'a>(&mut self, string: &'a str) -> ParseResult<'a, Instruction<'a>> {
        let start_location = self.index;
        self.literal(string, "write")?;
        self.optional(string, Self::whitespace)?;
        self.literal(string, "(")?;
        self.optional(string, Self::whitespace)?;
        let variable_name = self.parse_variable_name(string)?.value;
        self.optional(string, Self::whitespace)?;
        self.literal(string, ")")?;
        self.optional(string, Self::whitespace)?;
        self.literal(string, ";")?;
        let result = Instruction::Write {
            name: variable_name,
        };
        self.success(string, result, start_location, self.index - start_location)
    }

    pub fn parse_scope<'a>(&mut self, string: &'a str) -> ParseResult<'a, Instruction<'a>> {
        let start_index = self.index;
        self.literal(string, "{")?;
        let instructions = self.parse_instructions(string)?.value;
        self.literal(string, "}")?;
        let result = Instruction::Scope { body: instructions };
        self.success(string, result, start_index, self.index - start_index)
    }

    pub fn parse_while<'a>(&mut self, _string: &'a str) -> ParseResult<'a, Instruction<'a>> {
        todo!()
    }

    pub fn parse_if_else<'a>(&mut self, _string: &'a str) -> ParseResult<'a, Instruction<'a>> {
        todo!()
    }

    pub fn parse_instruction<'a>(&mut self, string: &'a str) -> ParseResult<'a, Instruction<'a>> {
        self.one_of(
            string,
            &[
                &Self::parse_definition,
                &Self::parse_assignment,
                &Self::parse_add_assignment,
                &Self::parse_sub_assignment,
                &Self::parse_read,
                &Self::parse_write,
                &Self::parse_scope,
            ],
        )
    }

    pub fn parse_instructions<'a>(
        &mut self,
        string: &'a str,
    ) -> ParseResult<'a, Vec<Instruction<'a>>> {
        let start_index = self.index;
        let instructions = self
            .repeat(string, |p, s| {
                p.optional(s, Self::whitespace)?;
                p.parse_instruction(s)
            })?
            .value;
        self.optional(string, Self::whitespace)?;
        self.success(string, instructions, start_index, self.index - start_index)
    }

    pub fn parse_program<'a>(&mut self, string: &'a str) -> ParseResult<'a, Program<'a>> {
        let start_index = self.index;
        let instructions = self.parse_instructions(string)?.value;
        let program = Program { instructions };
        self.eof(string)?;

        self.success(string, program, start_index, self.index - start_index)
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}
