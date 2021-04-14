use std::fmt::{self, Display};
use std::iter::Peekable;
use std::rc::Rc;

use crate::data_types::Type;
use crate::scanner::*;
use crate::treewalk::ast::*;

macro_rules! consume_expected_token_with_action {
    ($tokens:expr, $expected:pat, $transform_token:expr, $required_element:expr) => {
        match $tokens.peek().map(|t| &t.token) {
            Some($expected) => {
                let _ = $tokens.next();
                Ok($transform_token)
            }
            Some(_) => {
                let token = $tokens.next().unwrap();
                Err(ParseError::Missing(
                    $required_element,
                    token.lexeme.clone(),
                    token.position,
                ))
            }
            None => Err(ParseError::UnexpectedEndOfFile),
        }
    };
}

macro_rules! consume_expected_token {
    ($tokens:expr, $expected:pat, $required_element:expr) => {
        consume_expected_token_with_action!($tokens, $expected, (), $required_element)
    };
}

#[derive(Debug)]
pub enum RequiredElements {
    StringLiteral,
    Struct,
    LeftBrace,
    Identifier,
    Colon,
}

impl Display for RequiredElements {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RequiredElements::StringLiteral => write!(f, "StringLiteral"),
            RequiredElements::Struct => write!(f, "Struct"),
            RequiredElements::LeftBrace => write!(f, "LeftBrace"),
            RequiredElements::Identifier => write!(f, "Identifier"),
            RequiredElements::Colon => write!(f, "Colon"),
        }
    }
}
#[derive(Debug)]
pub enum ParseError {
    UnexpectedEndOfFile,
    UnknownError,
    Missing(RequiredElements, Lexeme, Position),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::UnexpectedEndOfFile => write!(f, "Unexpected End Of file"),
            ParseError::UnknownError => write!(
                f,
                "We have encountered an unknown error. This is likely a bug with the library."
            ),
            ParseError::Missing(token, lexeme, Position { line, column, .. }) => {
                write!(
                    f,
                    "Expected {} but found `{}` at line {} column {}",
                    token, lexeme, line, column
                )
            }
        }
    }
}

pub fn parse(tokens: &[TokenWithContext]) -> Result<Vec<GoStruct>, Vec<String>> {
    let mut statements = Vec::new();
    let mut errors = Vec::new();
    let mut peekable_tokens = tokens.iter().peekable();
    loop {
        let result = parse_declaration(&mut peekable_tokens);
        match result {
            Ok(statement) => {
                statements.push(statement);
            }
            Err(ParseError::UnexpectedEndOfFile) => {
                break;
            }
            Err(error) => {
                errors.push(format!("{}", error));
            }
        }
    }
    if errors.is_empty() {
        Ok(statements)
    } else {
        Err(errors)
    }
}

fn parse_declaration<'a, I>(tokens: &mut Peekable<I>) -> Result<GoStruct, ParseError>
where
    I: Iterator<Item = &'a TokenWithContext>,
{
    match tokens.peek().map(|t| &t.token) {
        Some(&Token::Type) => {
            let _ = tokens.next();
            parse_struct_declaration(tokens)
        }
        Some(Token::Identifier(key)) => {
            let _ = tokens.next();
            parse_identifier(key.to_string(), tokens)
        }
        Some(&Token::LeftBrace) => {
            let _ = tokens.next();
            parse_block(tokens)
        }
        Some(&Token::Json) => {
            let _ = tokens.next();
            parse_json(tokens)
        }
        Some(&Token::Binding) => {
            let _ = tokens.next();
            parse_binding(tokens)
        }
        Some(_) => {
            let _ = tokens.next();
            Ok(GoStruct::Unknown)
        }
        None => Err(ParseError::UnexpectedEndOfFile),
    }
}

fn parse_struct_declaration<'a, I>(tokens: &mut Peekable<I>) -> Result<GoStruct, ParseError>
where
    I: Iterator<Item = &'a TokenWithContext>,
{
    let identifier = consume_expected_identifier(tokens)?;
    consume_expected_token!(tokens, &Token::Struct, RequiredElements::Struct)?;
    consume_expected_token!(tokens, &Token::LeftBrace, RequiredElements::LeftBrace)?;
    let block = match parse_block(tokens) {
        Ok(block) => block,
        err => return err,
    };
    Ok(GoStruct::StructDefinition(Rc::new(StructDefinition {
        name: identifier,
        body: block,
    })))
}

fn consume_expected_identifier<'a, I>(tokens: &mut Peekable<I>) -> Result<String, ParseError>
where
    I: Iterator<Item = &'a TokenWithContext>,
{
    consume_expected_token_with_action!(
        tokens,
        &Token::Identifier(ref identifier),
        identifier.to_string(),
        RequiredElements::Identifier
    )
}

fn parse_block<'a, I>(tokens: &mut Peekable<I>) -> Result<GoStruct, ParseError>
where
    I: Iterator<Item = &'a TokenWithContext>,
{
    let mut statements = Vec::new();
    fn is_block_end(t: Option<&&TokenWithContext>) -> bool {
        matches!(
            t,
            Some(&TokenWithContext {
                token: Token::RightBrace,
                ..
            })
        )
    };
    while !is_block_end(tokens.peek()) {
        match parse_declaration(tokens) {
            Ok(statement) => statements.push(statement),
            Err(error) => return Err(error),
        }
    }
    if is_block_end(tokens.peek()) {
        let _ = tokens.next();
        Ok(GoStruct::Block(Box::new(Block { statements })))
    } else {
        Err(ParseError::UnexpectedEndOfFile)
    }
}

fn parse_identifier<'a, I>(
    identifier: String,
    tokens: &mut Peekable<I>,
) -> Result<GoStruct, ParseError>
where
    I: Iterator<Item = &'a TokenWithContext>,
{
    let item = match tokens.peek().map(|t| &t.token) {
        Some(&Token::DataType(typ)) => {
            let _ = tokens.next();
            Ok(GoStruct::FieldNameWithTypeOnly(identifier, typ))
        }
        Some(Token::Identifier(literal)) => {
            let _ = tokens.next();
            Ok(GoStruct::FieldWithIdentifierTypeOnly(
                identifier,
                literal.to_string(),
            ))
        }
        Some(&Token::NextLine) => {
            let _ = tokens.next();
            Ok(GoStruct::FieldNameOnly(identifier))
        }
        Some(&Token::Graveaccent) => {
            let vec = Vec::new();
            Ok(GoStruct::FieldWithJSONTags(identifier, Type::Any, vec))
        }
        Some(&Token::LeftBracket) => {
            let _ = tokens.next();
            Ok(GoStruct::FieldWithList(identifier))
        }
        Some(_) => Err(ParseError::UnknownError),
        None => Err(ParseError::UnexpectedEndOfFile),
    };

    parse_identifier_to_backticks(item, tokens)
}

fn parse_identifier_to_backticks<'a, I>(
    prev_item: Result<GoStruct, ParseError>,
    tokens: &mut Peekable<I>,
) -> Result<GoStruct, ParseError>
where
    I: Iterator<Item = &'a TokenWithContext>,
{
    let item = match (tokens.peek().map(|t| &t.token), prev_item) {
        (Some(&Token::Graveaccent), Ok(GoStruct::FieldWithJSONTags(name, typ, _))) => {
            let _ = tokens.next();
            let res = parse_backtick_block(tokens);
            match res {
                Ok(GoStruct::Block(b)) => Ok(GoStruct::FieldWithJSONTags(name, typ, b.statements)),
                _ => res,
            }
        }
        (Some(&Token::Graveaccent), Ok(GoStruct::FieldNameWithTypeOnly(name, typ))) => {
            let _ = tokens.next();
            let res = parse_backtick_block(tokens);
            match res {
                Ok(GoStruct::Block(b)) => Ok(GoStruct::FieldWithJSONTags(name, typ, b.statements)),
                res => res,
            }
        }
        (Some(&Token::Graveaccent), Ok(GoStruct::FieldWithIdentifierTypeOnly(name, literal))) => {
            let _ = tokens.next();
            let res = parse_backtick_block(tokens);
            match res {
                Ok(GoStruct::Block(b)) => Ok(GoStruct::FieldWithIdentifierAndJSONTags(
                    name,
                    literal,
                    b.statements,
                )),
                _ => res,
            }
        }
        (Some(&Token::RightBracket), Ok(GoStruct::FieldWithList(identifier))) => {
            let _ = tokens.next();
            let item_type = match tokens.peek().map(|t| &t.token) {
                Some(&Token::DataType(typ)) => {
                    let _ = tokens.next();
                    Ok(GoStruct::FieldWithListAndType(identifier, typ))
                }
                Some(Token::Identifier(customtype)) => {
                    let _ = tokens.next();
                    Ok(GoStruct::FieldWithCustomListIdentifier(
                        identifier,
                        customtype.to_string(),
                    ))
                }
                Some(_) => Err(ParseError::UnknownError),
                None => {
                    let _ = tokens.next();
                    Err(ParseError::UnexpectedEndOfFile)
                }
            };
            match (item_type, tokens.peek().map(|t| &t.token)) {
                (
                    Ok(GoStruct::FieldWithListAndType(identifier, typ)),
                    Some(&Token::Graveaccent),
                ) => {
                    let _ = tokens.next();
                    let res = parse_backtick_block(tokens);
                    match res {
                        Ok(GoStruct::Block(b)) => Ok(GoStruct::FieldWithListTypeAndJSONTags(
                            identifier,
                            typ,
                            b.statements,
                        )),
                        _ => res,
                    }
                }
                (
                    Ok(GoStruct::FieldWithCustomListIdentifier(identifier, customtype)),
                    Some(&Token::Graveaccent),
                ) => {
                    let _ = tokens.next();
                    let res = parse_backtick_block(tokens);
                    match res {
                        Ok(GoStruct::Block(b)) => {
                            Ok(GoStruct::FieldWithCustomListIdentifierAndJSONTags(
                                identifier,
                                customtype,
                                b.statements,
                            ))
                        }
                        _ => res,
                    }
                }
                (p, Some(&Token::NextLine)) => {
                    let _ = tokens.peek();
                    p
                }
                _ => {
                    let _ = tokens.peek();
                    Err(ParseError::UnexpectedEndOfFile)
                }
            }
        }
        (_, p) => p,
    };
    item
}

fn parse_backtick_block<'a, I>(tokens: &mut Peekable<I>) -> Result<GoStruct, ParseError>
where
    I: Iterator<Item = &'a TokenWithContext>,
{
    let mut statements = Vec::new();
    fn is_block_end(t: Option<&&TokenWithContext>) -> bool {
        matches!(
            t,
            Some(&TokenWithContext {
                token: Token::Graveaccent,
                ..
            })
        )
    };
    while !is_block_end(tokens.peek()) {
        match parse_declaration(tokens) {
            Ok(statement) => statements.push(statement),
            other => return other,
        }
    }
    if is_block_end(tokens.peek()) {
        let _ = tokens.next();
        Ok(GoStruct::Block(Box::new(Block { statements })))
    } else {
        Err(ParseError::UnexpectedEndOfFile)
    }
}

fn parse_json<'a, I>(tokens: &mut Peekable<I>) -> Result<GoStruct, ParseError>
where
    I: Iterator<Item = &'a TokenWithContext>,
{
    consume_expected_token!(tokens, &Token::Colon, RequiredElements::Colon)?;

    let str_literal = consume_expected_token_with_action!(
        tokens,
        &Token::StringLiteral(ref literal),
        literal.to_string(),
        RequiredElements::StringLiteral
    )?;
    if str_literal.as_str() == "-" {
        return Ok(GoStruct::IgnoreField);
    }
    Ok(GoStruct::JSONName(str_literal))
}

fn parse_binding<'a, I>(tokens: &mut Peekable<I>) -> Result<GoStruct, ParseError>
where
    I: Iterator<Item = &'a TokenWithContext>,
{
    consume_expected_token!(tokens, &Token::Colon, RequiredElements::Colon)?;

    consume_expected_token_with_action!(
        tokens,
        &Token::StringLiteral(ref literal),
        literal.to_string(),
        RequiredElements::StringLiteral
    )?;
    Ok(GoStruct::Binding)
}
