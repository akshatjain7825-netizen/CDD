use std::collections::HashMap;
use wasmc_diagnostics::{Diagnostic, DiagnosticSink, SourceSpan};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub u32);

#[derive(Debug, Default, Clone)]
pub struct SymbolInterner {
    strings: Vec<String>,
    lookup: HashMap<String, SymbolId>,
}

impl SymbolInterner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, s: &str) -> SymbolId {
        if let Some(&id) = self.lookup.get(s) {
            return id;
        }
        let id = SymbolId(self.strings.len() as u32);
        self.strings.push(s.to_string());
        self.lookup.insert(s.to_string(), id);
        id
    }

    pub fn resolve(&self, id: SymbolId) -> Option<&str> {
        self.strings.get(id.0 as usize).map(|s| s.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TokenKind {
    // Keywords
    Import,
    Export,
    Struct,
    Global,
    Mut,
    Fn,
    Let,
    If,
    Else,
    While,
    Return,
    Tail,
    Break,
    Continue,
    Try,
    Catch,
    Throw,

    // Types
    Void,
    Bool,
    Char,
    I32,
    I64,
    U32,
    U64,
    F32,
    F64,

    // Literals
    Integer(i64),
    Float(f64),
    BoolLiteral(bool),
    CharLiteral(char),
    StringLiteral(SymbolId),

    // Identifiers
    Identifier(SymbolId),

    // Symbols & Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Ampersand,
    Pipe,
    Caret,
    Tilde,
    Bang,
    Eq,
    EqEq,
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    Shl,
    Shr,
    AmpAmp,
    PipePipe,
    Arrow, // ->

    // Delimiters & Punctuation
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Semicolon,
    Colon,
    Comma,
    Dot,

    // Error token
    Error(String),
    Eof,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub kind: TokenKind,
    pub span: SourceSpan,
}

pub struct Lexer<'a> {
    source: &'a [u8],
    source_str: &'a str,
    cursor: usize,
    line: u32,
    col: u32,
    file_id: u32,
    interner: &'a mut SymbolInterner,
}

impl<'a> Lexer<'a> {
    pub fn new(source_str: &'a str, file_id: u32, interner: &'a mut SymbolInterner) -> Self {
        Self {
            source: source_str.as_bytes(),
            source_str,
            cursor: 0,
            line: 1,
            col: 1,
            file_id,
            interner,
        }
    }

    fn is_eof(&self) -> bool {
        self.cursor >= self.source.len()
    }

    fn peek(&self) -> Option<u8> {
        if self.is_eof() {
            None
        } else {
            Some(self.source[self.cursor])
        }
    }

    fn peek_next(&self) -> Option<u8> {
        if self.cursor + 1 >= self.source.len() {
            None
        } else {
            Some(self.source[self.cursor + 1])
        }
    }

    fn advance(&mut self) -> Option<u8> {
        if self.is_eof() {
            return None;
        }
        let b = self.source[self.cursor];
        self.cursor += 1;
        if b == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(b)
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_eof() {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r' | b'\n') => {
                    self.advance();
                }
                Some(b'/') if self.peek_next() == Some(b'/') => {
                    // Line comment
                    self.advance();
                    self.advance();
                    while let Some(c) = self.peek() {
                        if c == b'\n' {
                            self.advance();
                            break;
                        }
                        self.advance();
                    }
                }
                Some(b'/') if self.peek_next() == Some(b'*') => {
                    // Block comment
                    self.advance();
                    self.advance();
                    while !self.is_eof() {
                        if self.peek() == Some(b'*') && self.peek_next() == Some(b'/') {
                            self.advance();
                            self.advance();
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    pub fn next_token(&mut self, sink: &mut DiagnosticSink) -> Token {
        self.skip_whitespace_and_comments();

        let start_byte = self.cursor as u32;
        let start_line = self.line;
        let start_col = self.col;

        if self.is_eof() {
            return Token {
                kind: TokenKind::Eof,
                span: SourceSpan {
                    file_id: self.file_id,
                    start_byte,
                    end_byte: start_byte,
                    line: start_line,
                    column: start_col,
                },
            };
        }

        let b = self.advance().unwrap();

        let kind = match b {
            b'(' => TokenKind::OpenParen,
            b')' => TokenKind::CloseParen,
            b'{' => TokenKind::OpenBrace,
            b'}' => TokenKind::CloseBrace,
            b'[' => TokenKind::OpenBracket,
            b']' => TokenKind::CloseBracket,
            b';' => TokenKind::Semicolon,
            b':' => TokenKind::Colon,
            b',' => TokenKind::Comma,
            b'.' => TokenKind::Dot,
            b'+' => TokenKind::Plus,
            b'-' => {
                if self.peek() == Some(b'>') {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            b'*' => TokenKind::Star,
            b'/' => TokenKind::Slash,
            b'%' => TokenKind::Percent,
            b'^' => TokenKind::Caret,
            b'~' => TokenKind::Tilde,
            b'=' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::EqEq
                } else {
                    TokenKind::Eq
                }
            }
            b'!' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::BangEq
                } else {
                    TokenKind::Bang
                }
            }
            b'<' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::LtEq
                } else if self.peek() == Some(b'<') {
                    self.advance();
                    TokenKind::Shl
                } else {
                    TokenKind::Lt
                }
            }
            b'>' => {
                if self.peek() == Some(b'=') {
                    self.advance();
                    TokenKind::GtEq
                } else if self.peek() == Some(b'>') {
                    self.advance();
                    TokenKind::Shr
                } else {
                    TokenKind::Gt
                }
            }
            b'&' => {
                if self.peek() == Some(b'&') {
                    self.advance();
                    TokenKind::AmpAmp
                } else {
                    TokenKind::Ampersand
                }
            }
            b'|' => {
                if self.peek() == Some(b'|') {
                    self.advance();
                    TokenKind::PipePipe
                } else {
                    TokenKind::Pipe
                }
            }
            b'"' => self.lex_string(start_byte, start_line, start_col, sink),
            b'\'' => self.lex_char(start_byte, start_line, start_col, sink),
            b'0'..=b'9' => self.lex_number(b, start_byte, start_line, start_col, sink),
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                self.lex_identifier_or_keyword(b, start_byte)
            }
            _ => {
                // Illegal character recovery per FE-LEX-005 / FR-002
                let span = SourceSpan {
                    file_id: self.file_id,
                    start_byte,
                    end_byte: self.cursor as u32,
                    line: start_line,
                    column: start_col,
                };
                sink.push(Diagnostic::error(
                    "E1001",
                    format!("Illegal or unrecognized character: '{}'", b as char),
                    span,
                ));
                TokenKind::Error(format!("Illegal character: '{}'", b as char))
            }
        };

        Token {
            kind,
            span: SourceSpan {
                file_id: self.file_id,
                start_byte,
                end_byte: self.cursor as u32,
                line: start_line,
                column: start_col,
            },
        }
    }

    fn lex_identifier_or_keyword(&mut self, _first_byte: u8, start_byte: u32) -> TokenKind {
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == b'_' {
                self.advance();
            } else {
                break;
            }
        }
        let text = &self.source_str[start_byte as usize..self.cursor];
        match text {
            "import" => TokenKind::Import,
            "export" => TokenKind::Export,
            "struct" => TokenKind::Struct,
            "global" => TokenKind::Global,
            "mut" => TokenKind::Mut,
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "return" => TokenKind::Return,
            "tail" => TokenKind::Tail,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "throw" => TokenKind::Throw,
            "void" => TokenKind::Void,
            "bool" => TokenKind::Bool,
            "char" => TokenKind::Char,
            "i32" => TokenKind::I32,
            "i64" => TokenKind::I64,
            "u32" => TokenKind::U32,
            "u64" => TokenKind::U64,
            "f32" => TokenKind::F32,
            "f64" => TokenKind::F64,
            "true" => TokenKind::BoolLiteral(true),
            "false" => TokenKind::BoolLiteral(false),
            _ => {
                let sym = self.interner.intern(text);
                TokenKind::Identifier(sym)
            }
        }
    }

    fn lex_number(
        &mut self,
        first_byte: u8,
        start_byte: u32,
        _start_line: u32,
        _start_col: u32,
        _sink: &mut DiagnosticSink,
    ) -> TokenKind {
        let mut is_hex = false;
        let mut is_float = false;

        if first_byte == b'0' && (self.peek() == Some(b'x') || self.peek() == Some(b'X')) {
            is_hex = true;
            self.advance(); // skip 'x'
            while let Some(c) = self.peek() {
                if c.is_ascii_hexdigit() {
                    self.advance();
                } else {
                    break;
                }
            }
        } else {
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else if c == b'.' && self.peek_next().map_or(false, |n| n.is_ascii_digit()) {
                    is_float = true;
                    self.advance(); // skip '.'
                    while let Some(d) = self.peek() {
                        if d.is_ascii_digit() {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
            if self.peek() == Some(b'e') || self.peek() == Some(b'E') {
                is_float = true;
                self.advance();
                if self.peek() == Some(b'+') || self.peek() == Some(b'-') {
                    self.advance();
                }
                while let Some(d) = self.peek() {
                    if d.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        let slice = &self.source_str[start_byte as usize..self.cursor];
        if is_hex {
            let num = i64::from_str_radix(&slice[2..], 16).unwrap_or(0);
            TokenKind::Integer(num)
        } else if is_float {
            let val = slice.parse::<f64>().unwrap_or(0.0);
            TokenKind::Float(val)
        } else {
            let num = slice.parse::<i64>().unwrap_or(0);
            TokenKind::Integer(num)
        }
    }

    fn lex_string(
        &mut self,
        start_byte: u32,
        start_line: u32,
        start_col: u32,
        sink: &mut DiagnosticSink,
    ) -> TokenKind {
        let mut buf = String::new();
        let mut terminated = false;

        while !self.is_eof() {
            let b = self.advance().unwrap();
            if b == b'"' {
                terminated = true;
                break;
            } else if b == b'\\' {
                if let Some(esc) = self.advance() {
                    match esc {
                        b'n' => buf.push('\n'),
                        b'r' => buf.push('\r'),
                        b't' => buf.push('\t'),
                        b'\\' => buf.push('\\'),
                        b'"' => buf.push('"'),
                        b'0' => buf.push('\0'),
                        _ => buf.push(esc as char),
                    }
                }
            } else {
                buf.push(b as char);
            }
        }

        if !terminated {
            sink.push(Diagnostic::error(
                "E1002",
                "Unterminated string literal",
                SourceSpan {
                    file_id: self.file_id,
                    start_byte,
                    end_byte: self.cursor as u32,
                    line: start_line,
                    column: start_col,
                },
            ));
        }

        let sym = self.interner.intern(&buf);
        TokenKind::StringLiteral(sym)
    }

    fn lex_char(
        &mut self,
        start_byte: u32,
        start_line: u32,
        start_col: u32,
        sink: &mut DiagnosticSink,
    ) -> TokenKind {
        let ch = if let Some(b) = self.advance() {
            if b == b'\\' {
                if let Some(esc) = self.advance() {
                    match esc {
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'0' => '\0',
                        b'\'' => '\'',
                        b'\\' => '\\',
                        _ => esc as char,
                    }
                } else {
                    '\0'
                }
            } else {
                b as char
            }
        } else {
            '\0'
        };

        if self.advance() != Some(b'\'') {
            sink.push(Diagnostic::error(
                "E1003",
                "Unterminated character literal",
                SourceSpan {
                    file_id: self.file_id,
                    start_byte,
                    end_byte: self.cursor as u32,
                    line: start_line,
                    column: start_col,
                },
            ));
        }

        TokenKind::CharLiteral(ch)
    }

    pub fn tokenize_all(&mut self, sink: &mut DiagnosticSink) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token(sink);
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }
}
