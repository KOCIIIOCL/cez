use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // End of file
    Eof,

    // Identifiers and Literals
    Ident(String),
    IntLit(i128),
    FloatLit(f64),
    StringLit(String),
    CharLit(char),

    // Keywords
    Package,
    Import,
    Func,
    Type,
    Struct,
    Var,
    Const,
    If,
    Else,
    For,
    Switch,
    Case,
    Default,
    Return,
    Defer,
    Break,
    Continue,
    Extern,
    InlineAsm,

    // Attributes (OSDev / low-level)
    AtPacked,
    AtAlign,
    AtSection,
    AtNaked,
    AtInterrupt,
    AtExport,
    AtVolatileLoad,
    AtVolatileStore,

    // Operators
    Plus,          // +
    Minus,         // -
    Star,          // *
    Slash,         // /
    Percent,       // %
    Amp,           // &
    Pipe,          // |
    Caret,         // ^
    Shl,           // <<
    Shr,           // >>
    AmpCaret,      // &^ (bit clear)

    // Assignment & Compound
    Assign,        // =
    ColonAssign,   // :=
    PlusAssign,    // +=
    MinusAssign,   // -=
    StarAssign,    // *=
    SlashAssign,   // /=
    PercentAssign, // %=
    AmpAssign,     // &=
    PipeAssign,    // |=
    CaretAssign,   // ^=
    ShlAssign,     // <<=
    ShrAssign,     // >>=

    // Comparison & Logical
    Eq,            // ==
    NotEq,         // !=
    Lt,            // <
    LtEq,          // <=
    Gt,            // >
    GtEq,          // >=
    And,           // &&
    Or,            // ||
    Bang,          // !

    // Delimiters
    LParen,        // (
    RParen,        // )
    LBracket,      // [
    RBracket,      // ]
    LBrace,        // {
    RBrace,        // }
    Comma,         // ,
    Dot,           // .
    Colon,         // :
    Semicolon,     // ;
    DotDot,        // ..
    Ellipsis,      // ...
}

impl TokenKind {
    pub fn is_literal_or_ident(&self) -> bool {
        matches!(
            self,
            TokenKind::Ident(_)
                | TokenKind::IntLit(_)
                | TokenKind::FloatLit(_)
                | TokenKind::StringLit(_)
                | TokenKind::CharLit(_)
        )
    }

    /// Determines if a semicolon should be automatically inserted after this token
    /// when a newline is encountered (Go-style Automatic Semicolon Insertion).
    pub fn can_precede_semicolon(&self) -> bool {
        matches!(
            self,
            TokenKind::Ident(_)
                | TokenKind::IntLit(_)
                | TokenKind::FloatLit(_)
                | TokenKind::StringLit(_)
                | TokenKind::CharLit(_)
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Return
                | TokenKind::RParen
                | TokenKind::RBracket
                | TokenKind::RBrace
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
