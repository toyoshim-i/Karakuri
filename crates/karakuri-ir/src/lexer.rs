//! `.kir` source to a flat token stream.
//!
//! Hand written on purpose: the grammar is small, and a table-driven or
//! regex-based lexer would be more machinery than the language needs. Numbers
//! carry enough precision to survive round-tripping (`i64`/`u64`/`f64`) even
//! though the AST narrows them to `i32`/`u32`/`f32` — narrowing happens in the
//! parser, where a span is available to report a value that does not fit.
//!
//! The lexer never hard-fails. An unrecognized character is one diagnostic and
//! one skipped byte, not a stop — the parser, and whatever repair prompt reads
//! its output, wants every problem in the file, not just the first.

use crate::error::IrError;
use crate::span::Span;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TokKind {
    Ident(String),
    Int(i64),
    Uint(u64),
    Float(f64),

    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,

    Comma,
    Colon,
    Semicolon,
    Dot,
    DotDot,

    Eq,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,

    AmpAmp,
    PipePipe,
    Bang,

    Eof,
}

impl TokKind {
    /// Human-readable description for diagnostics: `` `}` `` or `end of file`.
    pub(crate) fn describe(&self) -> String {
        match self {
            TokKind::Ident(s) => format!("`{s}`"),
            TokKind::Int(v) => format!("`{v}`"),
            TokKind::Uint(v) => format!("`{v}u`"),
            TokKind::Float(v) => format!("`{v}`"),
            TokKind::LBrace => "`{`".to_string(),
            TokKind::RBrace => "`}`".to_string(),
            TokKind::LParen => "`(`".to_string(),
            TokKind::RParen => "`)`".to_string(),
            TokKind::LBracket => "`[`".to_string(),
            TokKind::RBracket => "`]`".to_string(),
            TokKind::Comma => "`,`".to_string(),
            TokKind::Colon => "`:`".to_string(),
            TokKind::Semicolon => "`;`".to_string(),
            TokKind::Dot => "`.`".to_string(),
            TokKind::DotDot => "`..`".to_string(),
            TokKind::Eq => "`=`".to_string(),
            TokKind::EqEq => "`==`".to_string(),
            TokKind::Ne => "`!=`".to_string(),
            TokKind::Lt => "`<`".to_string(),
            TokKind::Le => "`<=`".to_string(),
            TokKind::Gt => "`>`".to_string(),
            TokKind::Ge => "`>=`".to_string(),
            TokKind::Plus => "`+`".to_string(),
            TokKind::Minus => "`-`".to_string(),
            TokKind::Star => "`*`".to_string(),
            TokKind::Slash => "`/`".to_string(),
            TokKind::Percent => "`%`".to_string(),
            TokKind::PlusEq => "`+=`".to_string(),
            TokKind::MinusEq => "`-=`".to_string(),
            TokKind::StarEq => "`*=`".to_string(),
            TokKind::SlashEq => "`/=`".to_string(),
            TokKind::AmpAmp => "`&&`".to_string(),
            TokKind::PipePipe => "`||`".to_string(),
            TokKind::Bang => "`!`".to_string(),
            TokKind::Eof => "end of file".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Token {
    pub kind: TokKind,
    pub span: Span,
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek_char_at(&self, ahead: usize) -> Option<char> {
        self.src[self.pos..].chars().nth(ahead)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn lex_number(&mut self) -> TokKind {
        let start = self.pos;
        while matches!(self.peek_char(), Some(c) if c.is_ascii_digit()) {
            self.bump();
        }
        let mut is_float = false;
        if self.peek_char() == Some('.')
            && matches!(self.peek_char_at(1), Some(c) if c.is_ascii_digit())
        {
            is_float = true;
            self.bump();
            while matches!(self.peek_char(), Some(c) if c.is_ascii_digit()) {
                self.bump();
            }
        }
        if matches!(self.peek_char(), Some('e') | Some('E')) {
            let mut ahead = 1;
            if matches!(self.peek_char_at(ahead), Some('+') | Some('-')) {
                ahead += 1;
            }
            if matches!(self.peek_char_at(ahead), Some(c) if c.is_ascii_digit()) {
                is_float = true;
                self.bump();
                if matches!(self.peek_char(), Some('+') | Some('-')) {
                    self.bump();
                }
                while matches!(self.peek_char(), Some(c) if c.is_ascii_digit()) {
                    self.bump();
                }
            }
        }
        let text = &self.src[start..self.pos];
        if is_float {
            TokKind::Float(text.parse().unwrap_or(0.0))
        } else if self.peek_char() == Some('u')
            && !matches!(self.peek_char_at(1), Some(c) if c.is_alphanumeric() || c == '_')
        {
            self.bump();
            TokKind::Uint(text.parse().unwrap_or(0))
        } else {
            TokKind::Int(text.parse().unwrap_or(0))
        }
    }

    fn lex_ident(&mut self) -> TokKind {
        let start = self.pos;
        while matches!(self.peek_char(), Some(c) if c.is_alphanumeric() || c == '_') {
            self.bump();
        }
        TokKind::Ident(self.src[start..self.pos].to_string())
    }
}

/// Tokenize `src`. Always returns a token stream ending in `Eof`, plus every
/// lexical error found — an unrecognized character does not stop the scan.
pub(crate) fn lex(src: &str) -> (Vec<Token>, Vec<IrError>) {
    let mut lx = Lexer { src, pos: 0 };
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    loop {
        loop {
            match lx.peek_char() {
                Some(c) if c.is_whitespace() => {
                    lx.bump();
                }
                Some('/') if lx.peek_char_at(1) == Some('/') => {
                    while let Some(c) = lx.peek_char() {
                        if c == '\n' {
                            break;
                        }
                        lx.bump();
                    }
                }
                _ => break,
            }
        }

        let start = lx.pos as u32;
        let c = match lx.peek_char() {
            Some(c) => c,
            None => {
                tokens.push(Token {
                    kind: TokKind::Eof,
                    span: Span::new(start, start),
                });
                break;
            }
        };

        let kind: Option<TokKind> = if c.is_ascii_digit() {
            Some(lx.lex_number())
        } else if c.is_alphabetic() || c == '_' {
            Some(lx.lex_ident())
        } else {
            lx.bump();
            match c {
                '{' => Some(TokKind::LBrace),
                '}' => Some(TokKind::RBrace),
                '(' => Some(TokKind::LParen),
                ')' => Some(TokKind::RParen),
                '[' => Some(TokKind::LBracket),
                ']' => Some(TokKind::RBracket),
                ',' => Some(TokKind::Comma),
                ':' => Some(TokKind::Colon),
                ';' => Some(TokKind::Semicolon),
                '.' => {
                    if lx.peek_char() == Some('.') {
                        lx.bump();
                        Some(TokKind::DotDot)
                    } else {
                        Some(TokKind::Dot)
                    }
                }
                '=' => {
                    if lx.peek_char() == Some('=') {
                        lx.bump();
                        Some(TokKind::EqEq)
                    } else {
                        Some(TokKind::Eq)
                    }
                }
                '!' => {
                    if lx.peek_char() == Some('=') {
                        lx.bump();
                        Some(TokKind::Ne)
                    } else {
                        Some(TokKind::Bang)
                    }
                }
                '<' => {
                    if lx.peek_char() == Some('=') {
                        lx.bump();
                        Some(TokKind::Le)
                    } else {
                        Some(TokKind::Lt)
                    }
                }
                '>' => {
                    if lx.peek_char() == Some('=') {
                        lx.bump();
                        Some(TokKind::Ge)
                    } else {
                        Some(TokKind::Gt)
                    }
                }
                '+' => {
                    if lx.peek_char() == Some('=') {
                        lx.bump();
                        Some(TokKind::PlusEq)
                    } else {
                        Some(TokKind::Plus)
                    }
                }
                '-' => {
                    if lx.peek_char() == Some('=') {
                        lx.bump();
                        Some(TokKind::MinusEq)
                    } else {
                        Some(TokKind::Minus)
                    }
                }
                '*' => {
                    if lx.peek_char() == Some('=') {
                        lx.bump();
                        Some(TokKind::StarEq)
                    } else {
                        Some(TokKind::Star)
                    }
                }
                '/' => {
                    if lx.peek_char() == Some('=') {
                        lx.bump();
                        Some(TokKind::SlashEq)
                    } else {
                        Some(TokKind::Slash)
                    }
                }
                '%' => Some(TokKind::Percent),
                '&' => {
                    if lx.peek_char() == Some('&') {
                        lx.bump();
                        Some(TokKind::AmpAmp)
                    } else {
                        let end = lx.pos as u32;
                        errors.push(IrError::parse(
                            Span::new(start, end),
                            "unexpected character `&` (did you mean `&&`?)",
                        ));
                        None
                    }
                }
                '|' => {
                    if lx.peek_char() == Some('|') {
                        lx.bump();
                        Some(TokKind::PipePipe)
                    } else {
                        let end = lx.pos as u32;
                        errors.push(IrError::parse(
                            Span::new(start, end),
                            "unexpected character `|` (did you mean `||`?)",
                        ));
                        None
                    }
                }
                other => {
                    let end = lx.pos as u32;
                    errors.push(IrError::parse(
                        Span::new(start, end),
                        format!("unexpected character `{other}`"),
                    ));
                    None
                }
            }
        };

        if let Some(kind) = kind {
            let end = lx.pos as u32;
            tokens.push(Token {
                kind,
                span: Span::new(start, end),
            });
        }
    }

    (tokens, errors)
}
