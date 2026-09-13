use crate::token::{Span, Token, TokenKind};

pub struct Lexer<'a> {
    source: &'a str,
    chars: Vec<(usize, char)>,
    pos: usize,
    line: usize,
    col: usize,
    prev_kind: Option<TokenKind>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        let chars: Vec<(usize, char)> = source.char_indices().collect();
        Self {
            source,
            chars,
            pos: 0,
            line: 1,
            col: 1,
            prev_kind: None,
        }
    }

    fn peek(&self) -> Option<char> {
        if self.pos < self.chars.len() {
            Some(self.chars[self.pos].1)
        } else {
            None
        }
    }

    fn peek_next(&self) -> Option<char> {
        if self.pos + 1 < self.chars.len() {
            Some(self.chars[self.pos + 1].1)
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<char> {
        if self.pos < self.chars.len() {
            let ch = self.chars[self.pos].1;
            self.pos += 1;
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            Some(ch)
        } else {
            None
        }
    }

    fn current_span(&self) -> Span {
        Span::new(self.line, self.col)
    }

    pub fn next_token(&mut self) -> Result<Token, String> {
        loop {
            // Check for EOF
            if self.pos >= self.chars.len() {
                if let Some(ref prev) = self.prev_kind {
                    if prev.can_precede_semicolon() {
                        self.prev_kind = Some(TokenKind::Semicolon);
                        return Ok(Token::new(TokenKind::Semicolon, self.current_span()));
                    }
                }
                return Ok(Token::new(TokenKind::Eof, self.current_span()));
            }

            let start_span = self.current_span();
            let ch = self.peek().unwrap();

            // Whitespace (excluding newline)
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
                continue;
            }

            // Newline: check Go-style Automatic Semicolon Insertion (ASI)
            if ch == '\n' {
                self.advance();
                if let Some(ref prev) = self.prev_kind {
                    if prev.can_precede_semicolon() {
                        self.prev_kind = Some(TokenKind::Semicolon);
                        return Ok(Token::new(TokenKind::Semicolon, start_span));
                    }
                }
                continue;
            }

            // Comments
            if ch == '/' {
                if self.peek_next() == Some('/') {
                    // Line comment
                    self.advance(); // skip /
                    self.advance(); // skip /
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                    continue;
                } else if self.peek_next() == Some('*') {
                    // Block comment
                    self.advance(); // skip /
                    self.advance(); // skip *
                    let mut closed = false;
                    while let Some(c) = self.advance() {
                        if c == '*' && self.peek() == Some('/') {
                            self.advance(); // skip /
                            closed = true;
                            break;
                        }
                    }
                    if !closed {
                        return Err(format!("Unclosed block comment starting at {}", start_span));
                    }
                    continue;
                }
            }

            // String literals
            if ch == '"' {
                let token = self.lex_string(start_span)?;
                self.prev_kind = Some(token.kind.clone());
                return Ok(token);
            }

            // Raw string literals (backtick)
            if ch == '`' {
                let token = self.lex_raw_string(start_span)?;
                self.prev_kind = Some(token.kind.clone());
                return Ok(token);
            }

            // Character literals
            if ch == '\'' {
                let token = self.lex_char(start_span)?;
                self.prev_kind = Some(token.kind.clone());
                return Ok(token);
            }

            // Attributes starting with '@'
            if ch == '@' {
                let token = self.lex_attribute(start_span)?;
                self.prev_kind = Some(token.kind.clone());
                return Ok(token);
            }

            // Numbers
            if ch.is_ascii_digit() {
                let token = self.lex_number(start_span)?;
                self.prev_kind = Some(token.kind.clone());
                return Ok(token);
            }

            // Identifiers and Keywords
            if ch.is_alphabetic() || ch == '_' {
                let token = self.lex_identifier(start_span);
                self.prev_kind = Some(token.kind.clone());
                return Ok(token);
            }

            // Operators & Delimiters
            self.advance();
            let kind = match ch {
                '+' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::PlusAssign
                    } else {
                        TokenKind::Plus
                    }
                }
                '-' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::MinusAssign
                    } else {
                        TokenKind::Minus
                    }
                }
                '*' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::StarAssign
                    } else {
                        TokenKind::Star
                    }
                }
                '/' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::SlashAssign
                    } else {
                        TokenKind::Slash
                    }
                }
                '%' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::PercentAssign
                    } else {
                        TokenKind::Percent
                    }
                }
                '=' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::Eq
                    } else {
                        TokenKind::Assign
                    }
                }
                '!' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::NotEq
                    } else {
                        TokenKind::Bang
                    }
                }
                '<' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::LtEq
                    } else if self.peek() == Some('<') {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::ShlAssign
                        } else {
                            TokenKind::Shl
                        }
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::GtEq
                    } else if self.peek() == Some('>') {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::ShrAssign
                        } else {
                            TokenKind::Shr
                        }
                    } else {
                        TokenKind::Gt
                    }
                }
                '&' => {
                    if self.peek() == Some('&') {
                        self.advance();
                        TokenKind::And
                    } else if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::AmpAssign
                    } else if self.peek() == Some('^') {
                        self.advance();
                        TokenKind::AmpCaret
                    } else {
                        TokenKind::Amp
                    }
                }
                '|' => {
                    if self.peek() == Some('|') {
                        self.advance();
                        TokenKind::Or
                    } else if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::PipeAssign
                    } else {
                        TokenKind::Pipe
                    }
                }
                '^' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::CaretAssign
                    } else {
                        TokenKind::Caret
                    }
                }
                ':' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::ColonAssign
                    } else {
                        TokenKind::Colon
                    }
                }
                '.' => {
                    if self.peek() == Some('.') {
                        self.advance();
                        if self.peek() == Some('.') {
                            self.advance();
                            TokenKind::Ellipsis
                        } else {
                            TokenKind::DotDot
                        }
                    } else {
                        TokenKind::Dot
                    }
                }
                ';' => TokenKind::Semicolon,
                ',' => TokenKind::Comma,
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                '[' => TokenKind::LBracket,
                ']' => TokenKind::RBracket,
                '{' => TokenKind::LBrace,
                '}' => TokenKind::RBrace,
                unknown => return Err(format!("Unexpected character '{}' at {}", unknown, start_span)),
            };

            let token = Token::new(kind, start_span);
            self.prev_kind = Some(token.kind.clone());
            return Ok(token);
        }
    }

    fn lex_identifier(&mut self, start_span: Span) -> Token {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let kind = match s.as_str() {
            "package" => TokenKind::Package,
            "import" => TokenKind::Import,
            "func" | "fn" => TokenKind::Func,
            "type" => TokenKind::Type,
            "struct" => TokenKind::Struct,
            "var" => TokenKind::Var,
            "const" => TokenKind::Const,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "switch" => TokenKind::Switch,
            "case" => TokenKind::Case,
            "default" => TokenKind::Default,
            "return" => TokenKind::Return,
            "defer" => TokenKind::Defer,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "extern" => TokenKind::Extern,
            "inline_asm" => TokenKind::InlineAsm,
            _ => TokenKind::Ident(s),
        };

        Token::new(kind, start_span)
    }

    fn lex_attribute(&mut self, start_span: Span) -> Result<Token, String> {
        self.advance(); // skip '@'
        let mut name = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphabetic() || ch == '_' {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let kind = match name.as_str() {
            "packed" => TokenKind::AtPacked,
            "align" => TokenKind::AtAlign,
            "section" => TokenKind::AtSection,
            "naked" => TokenKind::AtNaked,
            "interrupt" => TokenKind::AtInterrupt,
            "export" => TokenKind::AtExport,
            "volatile_load" => TokenKind::AtVolatileLoad,
            "volatile_store" => TokenKind::AtVolatileStore,
            _ => return Err(format!("Unknown attribute '@{}' at {}", name, start_span)),
        };

        Ok(Token::new(kind, start_span))
    }

    fn lex_string(&mut self, start_span: Span) -> Result<Token, String> {
        self.advance(); // skip opening quote
        let mut s = String::new();
        while let Some(ch) = self.advance() {
            if ch == '"' {
                return Ok(Token::new(TokenKind::StringLit(s), start_span));
            } else if ch == '\\' {
                match self.advance() {
                    Some('n') => s.push('\n'),
                    Some('r') => s.push('\r'),
                    Some('t') => s.push('\t'),
                    Some('\\') => s.push('\\'),
                    Some('"') => s.push('"'),
                    Some('0') => s.push('\0'),
                    Some('x') => {
                        let mut hex = String::new();
                        for _ in 0..2 {
                            if let Some(h) = self.advance() {
                                hex.push(h);
                            }
                        }
                        if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                            s.push(byte as char);
                        } else {
                            return Err(format!("Invalid hex escape '\\x{}' at {}", hex, start_span));
                        }
                    }
                    Some(c) => s.push(c),
                    None => return Err(format!("Unterminated escape at {}", start_span)),
                }
            } else {
                s.push(ch);
            }
        }
        Err(format!("Unterminated string literal starting at {}", start_span))
    }

    fn lex_raw_string(&mut self, start_span: Span) -> Result<Token, String> {
        self.advance(); // skip backtick
        let mut s = String::new();
        while let Some(ch) = self.advance() {
            if ch == '`' {
                return Ok(Token::new(TokenKind::StringLit(s), start_span));
            } else {
                s.push(ch);
            }
        }
        Err(format!("Unterminated raw string literal starting at {}", start_span))
    }

    fn lex_char(&mut self, start_span: Span) -> Result<Token, String> {
        self.advance(); // skip opening single quote
        let ch = match self.advance() {
            Some('\\') => match self.advance() {
                Some('n') => '\n',
                Some('r') => '\r',
                Some('t') => '\t',
                Some('\\') => '\\',
                Some('\'') => '\'',
                Some('0') => '\0',
                Some(c) => c,
                None => return Err(format!("Unterminated char escape at {}", start_span)),
            },
            Some('\'') => return Err(format!("Empty character literal at {}", start_span)),
            Some(c) => c,
            None => return Err(format!("Unterminated character literal at {}", start_span)),
        };

        if self.advance() != Some('\'') {
            return Err(format!("Multi-character char literal or missing closing quote at {}", start_span));
        }

        Ok(Token::new(TokenKind::CharLit(ch), start_span))
    }

    fn lex_number(&mut self, start_span: Span) -> Result<Token, String> {
        let mut num_str = String::new();
        let first = self.advance().unwrap();
        num_str.push(first);

        if first == '0' {
            if let Some(next) = self.peek() {
                if next == 'x' || next == 'X' {
                    num_str.push(self.advance().unwrap());
                    while let Some(c) = self.peek() {
                        if c.is_ascii_hexdigit() || c == '_' {
                            if c != '_' {
                                num_str.push(c);
                            }
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let hex_clean = &num_str[2..];
                    let val = i128::from_str_radix(hex_clean, 16)
                        .map_err(|e| format!("Invalid hex literal '{}': {} at {}", num_str, e, start_span))?;
                    return Ok(Token::new(TokenKind::IntLit(val), start_span));
                } else if next == 'b' || next == 'B' {
                    num_str.push(self.advance().unwrap());
                    while let Some(c) = self.peek() {
                        if c == '0' || c == '1' || c == '_' {
                            if c != '_' {
                                num_str.push(c);
                            }
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let bin_clean = &num_str[2..];
                    let val = i128::from_str_radix(bin_clean, 2)
                        .map_err(|e| format!("Invalid binary literal '{}': {} at {}", num_str, e, start_span))?;
                    return Ok(Token::new(TokenKind::IntLit(val), start_span));
                }
            }
        }

        let mut is_float = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '_' {
                if c != '_' {
                    num_str.push(c);
                }
                self.advance();
            } else if c == '.' && !is_float && self.peek_next() != Some('.') {
                // Period followed by digit or non-period: float
                is_float = true;
                num_str.push(self.advance().unwrap());
            } else {
                break;
            }
        }

        if is_float {
            let val = num_str.parse::<f64>()
                .map_err(|e| format!("Invalid float literal '{}': {} at {}", num_str, e, start_span))?;
            Ok(Token::new(TokenKind::FloatLit(val), start_span))
        } else {
            let val = num_str.parse::<i128>()
                .map_err(|e| format!("Invalid integer literal '{}': {} at {}", num_str, e, start_span))?;
            Ok(Token::new(TokenKind::IntLit(val), start_span))
        }
    }

    pub fn tokenize_all(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_asi_semicolons() {
        let src = "package main\nvar x = 10\nfunc main() {\nreturn\n}";
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Semicolon));
    }

    #[test]
    fn test_hex_binary() {
        let src = "0xdead_beef 0b1010_0101";
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::IntLit(0xdeadbeef));
        assert_eq!(tokens[1].kind, TokenKind::IntLit(0b10100101));
    }

    #[test]
    fn test_attributes() {
        let src = "@packed @naked @section(\".boot\")";
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::AtPacked);
        assert_eq!(tokens[1].kind, TokenKind::AtNaked);
        assert_eq!(tokens[2].kind, TokenKind::AtSection);
    }
}
