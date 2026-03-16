// KLIK Language - High-performance Lexer
// Tokenizes UTF-8 source code with full source location tracking

mod token;

pub use token::{Token, TokenKind};

use klik_ast::{Position, Span};
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum LexerError {
    #[error("{message} at {span}")]
    Error { message: String, span: Span },
}

impl LexerError {
    pub fn span(&self) -> &Span {
        match self {
            LexerError::Error { span, .. } => span,
        }
    }
}

pub struct Lexer {
    source: Vec<char>,
    file: String,
    pos: usize,
    line: usize,
    column: usize,
    tokens: Vec<Token>,
    errors: Vec<LexerError>,
}

impl Lexer {
    pub fn new(source: &str, file: impl Into<String>) -> Self {
        Self {
            source: source.chars().collect(),
            file: file.into(),
            pos: 0,
            line: 0,
            column: 0,
            tokens: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, Vec<LexerError>> {
        while !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.is_at_end() {
                break;
            }
            match self.scan_token() {
                Ok(token) => self.tokens.push(token),
                Err(e) => self.errors.push(e),
            }
        }

        self.tokens.push(Token {
            kind: TokenKind::Eof,
            lexeme: String::new(),
            span: self.make_span_here(),
        });

        if self.errors.is_empty() {
            Ok(self.tokens)
        } else {
            Err(self.errors)
        }
    }

    fn scan_token(&mut self) -> Result<Token, LexerError> {
        let start = self.position();
        let ch = self.advance();

        match ch {
            '(' => Ok(self.make_token(TokenKind::LeftParen, start)),
            ')' => Ok(self.make_token(TokenKind::RightParen, start)),
            '{' => Ok(self.make_token(TokenKind::LeftBrace, start)),
            '}' => Ok(self.make_token(TokenKind::RightBrace, start)),
            '[' => Ok(self.make_token(TokenKind::LeftBracket, start)),
            ']' => Ok(self.make_token(TokenKind::RightBracket, start)),
            ',' => Ok(self.make_token(TokenKind::Comma, start)),
            ';' => Ok(self.make_token(TokenKind::Semicolon, start)),
            ':' => {
                if self.match_char(':') {
                    Ok(self.make_token(TokenKind::ColonColon, start))
                } else {
                    Ok(self.make_token(TokenKind::Colon, start))
                }
            }
            '.' => {
                if self.match_char('.') {
                    if self.match_char('=') {
                        Ok(self.make_token(TokenKind::DotDotEq, start))
                    } else {
                        Ok(self.make_token(TokenKind::DotDot, start))
                    }
                } else {
                    Ok(self.make_token(TokenKind::Dot, start))
                }
            }
            '+' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::PlusEq, start))
                } else {
                    Ok(self.make_token(TokenKind::Plus, start))
                }
            }
            '-' => {
                if self.match_char('>') {
                    Ok(self.make_token(TokenKind::Arrow, start))
                } else if self.match_char('=') {
                    Ok(self.make_token(TokenKind::MinusEq, start))
                } else {
                    Ok(self.make_token(TokenKind::Minus, start))
                }
            }
            '*' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::StarEq, start))
                } else {
                    Ok(self.make_token(TokenKind::Star, start))
                }
            }
            '/' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::SlashEq, start))
                } else {
                    Ok(self.make_token(TokenKind::Slash, start))
                }
            }
            '%' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::PercentEq, start))
                } else {
                    Ok(self.make_token(TokenKind::Percent, start))
                }
            }
            '!' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::BangEq, start))
                } else {
                    Ok(self.make_token(TokenKind::Bang, start))
                }
            }
            '=' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::EqEq, start))
                } else if self.match_char('>') {
                    Ok(self.make_token(TokenKind::FatArrow, start))
                } else {
                    Ok(self.make_token(TokenKind::Eq, start))
                }
            }
            '<' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::LessEq, start))
                } else if self.match_char('<') {
                    Ok(self.make_token(TokenKind::Shl, start))
                } else {
                    Ok(self.make_token(TokenKind::Less, start))
                }
            }
            '>' => {
                if self.match_char('=') {
                    Ok(self.make_token(TokenKind::GreaterEq, start))
                } else if self.match_char('>') {
                    Ok(self.make_token(TokenKind::Shr, start))
                } else {
                    Ok(self.make_token(TokenKind::Greater, start))
                }
            }
            '&' => {
                if self.match_char('&') {
                    Ok(self.make_token(TokenKind::AmpAmp, start))
                } else {
                    Ok(self.make_token(TokenKind::Amp, start))
                }
            }
            '|' => {
                if self.match_char('|') {
                    Ok(self.make_token(TokenKind::PipePipe, start))
                } else if self.match_char('>') {
                    Ok(self.make_token(TokenKind::PipeArrow, start))
                } else {
                    Ok(self.make_token(TokenKind::Pipe, start))
                }
            }
            '^' => Ok(self.make_token(TokenKind::Caret, start)),
            '~' => Ok(self.make_token(TokenKind::Tilde, start)),
            '?' => Ok(self.make_token(TokenKind::Question, start)),
            '#' => Ok(self.make_token(TokenKind::Hash, start)),
            '@' => Ok(self.make_token(TokenKind::At, start)),
            '"' => self.scan_string(start),
            '\'' => self.scan_char(start),
            _ if ch.is_ascii_digit() => self.scan_number(ch, start),
            _ if is_ident_start(ch) => Ok(self.scan_identifier(ch, start)),
            _ => Err(LexerError::Error {
                message: format!("unexpected character '{ch}'"),
                span: self.make_span(start),
            }),
        }
    }

    fn scan_string(&mut self, start: Position) -> Result<Token, LexerError> {
        let mut value = String::new();
        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(LexerError::Error {
                        message: "unterminated string escape".into(),
                        span: self.make_span(start),
                    });
                }
                let escaped = self.advance();
                match escaped {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    '0' => value.push('\0'),
                    'u' => {
                        if self.match_char('{') {
                            let mut hex = String::new();
                            while !self.is_at_end() && self.peek() != '}' {
                                hex.push(self.advance());
                            }
                            if !self.match_char('}') {
                                return Err(LexerError::Error {
                                    message: "unterminated unicode escape".into(),
                                    span: self.make_span(start),
                                });
                            }
                            let code =
                                u32::from_str_radix(&hex, 16).map_err(|_| LexerError::Error {
                                    message: format!("invalid unicode escape: \\u{{{hex}}}"),
                                    span: self.make_span(start.clone()),
                                })?;
                            let c = char::from_u32(code).ok_or_else(|| LexerError::Error {
                                message: format!("invalid unicode codepoint: {code}"),
                                span: self.make_span(start.clone()),
                            })?;
                            value.push(c);
                        }
                    }
                    _ => {
                        return Err(LexerError::Error {
                            message: format!("unknown escape sequence: \\{escaped}"),
                            span: self.make_span(start),
                        });
                    }
                }
            } else {
                if self.peek() == '\n' {
                    self.line += 1;
                    self.column = 0;
                }
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(LexerError::Error {
                message: "unterminated string literal".into(),
                span: self.make_span(start),
            });
        }

        self.advance(); // closing "
        let span = self.make_span(start);
        Ok(Token {
            kind: TokenKind::StringLiteral(value),
            lexeme: span.file.clone(), // store for debugging
            span,
        })
    }

    fn scan_char(&mut self, start: Position) -> Result<Token, LexerError> {
        if self.is_at_end() {
            return Err(LexerError::Error {
                message: "unterminated character literal".into(),
                span: self.make_span(start),
            });
        }

        let ch = if self.peek() == '\\' {
            self.advance();
            let escaped = self.advance();
            match escaped {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                '\\' => '\\',
                '\'' => '\'',
                '0' => '\0',
                _ => {
                    return Err(LexerError::Error {
                        message: format!("unknown escape in char: \\{escaped}"),
                        span: self.make_span(start),
                    });
                }
            }
        } else {
            self.advance()
        };

        if !self.match_char('\'') {
            return Err(LexerError::Error {
                message: "unterminated character literal".into(),
                span: self.make_span(start),
            });
        }

        Ok(Token {
            kind: TokenKind::CharLiteral(ch),
            lexeme: format!("'{ch}'"),
            span: self.make_span(start),
        })
    }

    fn scan_number(&mut self, first: char, start: Position) -> Result<Token, LexerError> {
        let mut num = String::new();
        num.push(first);
        let mut is_float = false;

        // Check for hex, binary, octal
        if first == '0' && !self.is_at_end() {
            match self.peek() {
                'x' | 'X' => {
                    num.push(self.advance());
                    while !self.is_at_end()
                        && (self.peek().is_ascii_hexdigit() || self.peek() == '_')
                    {
                        let c = self.advance();
                        if c != '_' {
                            num.push(c);
                        }
                    }
                    let val =
                        i64::from_str_radix(&num[2..], 16).map_err(|_| LexerError::Error {
                            message: format!("invalid hex literal: {num}"),
                            span: self.make_span(start.clone()),
                        })?;
                    return Ok(Token {
                        kind: TokenKind::IntLiteral(val),
                        lexeme: num,
                        span: self.make_span(start),
                    });
                }
                'b' | 'B' => {
                    num.push(self.advance());
                    while !self.is_at_end()
                        && (self.peek() == '0' || self.peek() == '1' || self.peek() == '_')
                    {
                        let c = self.advance();
                        if c != '_' {
                            num.push(c);
                        }
                    }
                    let val = i64::from_str_radix(&num[2..], 2).map_err(|_| LexerError::Error {
                        message: format!("invalid binary literal: {num}"),
                        span: self.make_span(start.clone()),
                    })?;
                    return Ok(Token {
                        kind: TokenKind::IntLiteral(val),
                        lexeme: num,
                        span: self.make_span(start),
                    });
                }
                'o' | 'O' => {
                    num.push(self.advance());
                    while !self.is_at_end()
                        && ((self.peek() >= '0' && self.peek() <= '7') || self.peek() == '_')
                    {
                        let c = self.advance();
                        if c != '_' {
                            num.push(c);
                        }
                    }
                    let val = i64::from_str_radix(&num[2..], 8).map_err(|_| LexerError::Error {
                        message: format!("invalid octal literal: {num}"),
                        span: self.make_span(start.clone()),
                    })?;
                    return Ok(Token {
                        kind: TokenKind::IntLiteral(val),
                        lexeme: num,
                        span: self.make_span(start),
                    });
                }
                _ => {}
            }
        }

        while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
            let c = self.advance();
            if c != '_' {
                num.push(c);
            }
        }

        if !self.is_at_end()
            && self.peek() == '.'
            && self.peek_next().is_some_and(|c| c.is_ascii_digit())
        {
            is_float = true;
            num.push(self.advance()); // '.'
            while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '_') {
                let c = self.advance();
                if c != '_' {
                    num.push(c);
                }
            }
        }

        // Scientific notation
        if !self.is_at_end() && (self.peek() == 'e' || self.peek() == 'E') {
            is_float = true;
            num.push(self.advance());
            if !self.is_at_end() && (self.peek() == '+' || self.peek() == '-') {
                num.push(self.advance());
            }
            while !self.is_at_end() && self.peek().is_ascii_digit() {
                num.push(self.advance());
            }
        }

        let span = self.make_span(start);
        if is_float {
            let val: f64 = num.parse().map_err(|_| LexerError::Error {
                message: format!("invalid float literal: {num}"),
                span: span.clone(),
            })?;
            Ok(Token {
                kind: TokenKind::FloatLiteral(val),
                lexeme: num,
                span,
            })
        } else {
            let val: i64 = num.parse().map_err(|_| LexerError::Error {
                message: format!("invalid integer literal: {num}"),
                span: span.clone(),
            })?;
            Ok(Token {
                kind: TokenKind::IntLiteral(val),
                lexeme: num,
                span,
            })
        }
    }

    fn scan_identifier(&mut self, first: char, start: Position) -> Token {
        let mut ident = String::new();
        ident.push(first);
        while !self.is_at_end() && is_ident_continue(self.peek()) {
            ident.push(self.advance());
        }

        let kind = match ident.as_str() {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "return" => TokenKind::Return,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "trait" => TokenKind::Trait,
            "impl" => TokenKind::Impl,
            "import" => TokenKind::Import,
            "pub" => TokenKind::Pub,
            "match" => TokenKind::Match,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "none" => TokenKind::None,
            "as" => TokenKind::As,
            "type" => TokenKind::Type,
            "const" => TokenKind::Const,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "mod" => TokenKind::Mod,
            "test" => TokenKind::Test,
            "assert" => TokenKind::Assert,
            "self" => TokenKind::SelfKw,
            "spawn" => TokenKind::Spawn,
            _ => TokenKind::Identifier(ident.clone()),
        };

        Token {
            kind,
            lexeme: ident,
            span: self.make_span(start),
        }
    }

    // ========================================================================
    // Helper methods
    // ========================================================================

    fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn peek(&self) -> char {
        self.source[self.pos]
    }

    fn peek_next(&self) -> Option<char> {
        if self.pos + 1 < self.source.len() {
            Some(self.source[self.pos + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) -> char {
        let ch = self.source[self.pos];
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }
        ch
    }

    fn match_char(&mut self, expected: char) -> bool {
        if !self.is_at_end() && self.source[self.pos] == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                }
                '/' if self.peek_next() == Some('/') => {
                    // Line comment
                    while !self.is_at_end() && self.peek() != '\n' {
                        self.advance();
                    }
                }
                '/' if self.peek_next() == Some('*') => {
                    // Block comment (nestable)
                    self.advance(); // /
                    self.advance(); // *
                    let mut depth = 1;
                    while !self.is_at_end() && depth > 0 {
                        if self.peek() == '/' && self.peek_next() == Some('*') {
                            self.advance();
                            self.advance();
                            depth += 1;
                        } else if self.peek() == '*' && self.peek_next() == Some('/') {
                            self.advance();
                            self.advance();
                            depth -= 1;
                        } else {
                            self.advance();
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn position(&self) -> Position {
        Position {
            line: self.line,
            column: self.column,
            offset: self.pos,
        }
    }

    fn make_span_here(&self) -> Span {
        let pos = self.position();
        Span::new(&self.file, pos.clone(), pos)
    }

    fn make_span(&self, start: Position) -> Span {
        Span::new(&self.file, start, self.position())
    }

    fn make_token(&self, kind: TokenKind, start: Position) -> Token {
        let span = self.make_span(start);
        Token {
            lexeme: self.source[span.start.offset..span.end.offset]
                .iter()
                .collect(),
            kind,
            span,
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let lexer = Lexer::new("fn main { }", "<test>");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Fn);
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(_)));
        assert_eq!(tokens[2].kind, TokenKind::LeftBrace);
        assert_eq!(tokens[3].kind, TokenKind::RightBrace);
        assert_eq!(tokens[4].kind, TokenKind::Eof);
    }

    #[test]
    fn test_number_literals() {
        let lexer = Lexer::new("42 3.14 0xFF 0b1010", "<test>");
        let tokens = lexer.tokenize().unwrap();
        let expected_float = "3.14".parse::<f64>().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::IntLiteral(42));
        assert_eq!(tokens[1].kind, TokenKind::FloatLiteral(expected_float));
        assert_eq!(tokens[2].kind, TokenKind::IntLiteral(255));
        assert_eq!(tokens[3].kind, TokenKind::IntLiteral(10));
    }

    #[test]
    fn test_string_literal() {
        let lexer = Lexer::new(r#""hello\nworld""#, "<test>");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::StringLiteral(s) if s == "hello\nworld"));
    }

    #[test]
    fn test_operators() {
        let lexer = Lexer::new("+ - * / == != <= >= && || |>", "<test>");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Plus);
        assert_eq!(tokens[1].kind, TokenKind::Minus);
        assert_eq!(tokens[2].kind, TokenKind::Star);
        assert_eq!(tokens[3].kind, TokenKind::Slash);
        assert_eq!(tokens[4].kind, TokenKind::EqEq);
        assert_eq!(tokens[5].kind, TokenKind::BangEq);
        assert_eq!(tokens[6].kind, TokenKind::LessEq);
        assert_eq!(tokens[7].kind, TokenKind::GreaterEq);
        assert_eq!(tokens[8].kind, TokenKind::AmpAmp);
        assert_eq!(tokens[9].kind, TokenKind::PipePipe);
        assert_eq!(tokens[10].kind, TokenKind::PipeArrow);
    }
}
