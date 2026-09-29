use crate::ast::*;
use crate::token::{Span, Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Значения `const`-констант, извлечённые при обходе верхнего уровня.
    /// Нужны, чтобы размер массива мог быть задан именем константы:
    /// `const STACK_SIZE u64 = 16384` + `var stack [STACK_SIZE]u8`.
    const_values: std::collections::HashMap<String, i128>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            const_values: std::collections::HashMap::new(),
        }
    }

    fn peek(&self) -> &Token {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            &self.tokens[self.tokens.len() - 1]
        }
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn peek_next(&self) -> Option<&Token> {
        if self.pos + 1 < self.tokens.len() {
            Some(&self.tokens[self.pos + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Token {
        let tok = self.peek().clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek_kind() == kind
    }

    fn match_tok(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume(&mut self, kind: &TokenKind, err_msg: &str) -> Result<Token, String> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            let tok = self.peek();
            Err(format!("{} at {} (got {:?})", err_msg, tok.span, tok.kind))
        }
    }

    fn consume_ident(&mut self, err_msg: &str) -> Result<(String, Span), String> {
        let tok = self.peek().clone();
        if let TokenKind::Ident(ref s) = tok.kind {
            self.advance();
            Ok((s.clone(), tok.span))
        } else {
            Err(format!("{} at {} (got {:?})", err_msg, tok.span, tok.kind))
        }
    }

    fn skip_semicolons(&mut self) {
        while self.match_tok(&TokenKind::Semicolon) {}
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
     self.skip_semicolons();

     // 0. Предпросмотр: значения целочисленных констант, чтобы размер
     // массива мог быть задан именем — `const STACK_SIZE u64 = 16384`.
     for i in 0..self.tokens.len() {
         if !matches!(self.tokens[i].kind, TokenKind::Const) {
             continue;
         }
         let name = match self.tokens.get(i + 1) {
             Some(Token {
                 kind: TokenKind::Ident(s),
                 ..
             }) => s.clone(),
             _ => continue,
         };
         // Пропускаем необязательный тип и ищем '=' до конца объявления.
         for j in (i + 2)..self.tokens.len() {
             match &self.tokens[j].kind {
                 TokenKind::Semicolon | TokenKind::Eof => break,
                 TokenKind::Assign => {
                     if let Some(Token {
                         kind: TokenKind::IntLit(n),
                         ..
                     }) = self.tokens.get(j + 1)
                     {
                         self.const_values.insert(name, *n);
                     }
                     break;
                 }
                 _ => {}
             }
         }
     }

        // 1. package <name>
        self.consume(&TokenKind::Package, "Expected 'package' declaration at start of file")?;
        let (package_name, _) = self.consume_ident("Expected package name")?;
        self.skip_semicolons();

        // 2. imports
        let mut imports = Vec::new();
        while self.check(&TokenKind::Import) {
            self.advance();
            if self.match_tok(&TokenKind::LParen) {
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    self.skip_semicolons();
                    if self.check(&TokenKind::RParen) {
                        break;
                    }
                    let tok = self.advance();
                    if let TokenKind::StringLit(s) = tok.kind {
                        imports.push(s);
                    } else {
                        return Err(format!("Expected string literal import path at {}", tok.span));
                    }
                    self.skip_semicolons();
                }
                self.consume(&TokenKind::RParen, "Expected ')' after import list")?;
            } else {
                let tok = self.advance();
                if let TokenKind::StringLit(s) = tok.kind {
                    imports.push(s);
                } else {
                    return Err(format!("Expected string literal import path at {}", tok.span));
                }
            }
            self.skip_semicolons();
        }

        // 3. declarations
        let mut decls = Vec::new();
        while !self.check(&TokenKind::Eof) {
            self.skip_semicolons();
            if self.check(&TokenKind::Eof) {
                break;
            }
            let decl = self.parse_top_level_decl()?;
            decls.push(decl);
            self.skip_semicolons();
        }

        Ok(Program {
            package_name,
            imports,
            decls,
        })
    }

    fn parse_top_level_decl(&mut self) -> Result<Decl, String> {
        let mut attributes = self.parse_attributes()?;

        if self.match_tok(&TokenKind::Extern) {
            // Extern function
            self.consume(&TokenKind::Func, "Expected 'func' after 'extern'")?;
            let func_decl = self.parse_func_decl(None, attributes, true)?;
            return Ok(Decl::Func(func_decl));
        }

        if self.check(&TokenKind::Func) {
            self.advance();
            // Receiver? func (r *Type) MethodName(...)
            let receiver = if self.check(&TokenKind::LParen) {
                Some(self.parse_receiver()?)
            } else {
                None
            };
            let func_decl = self.parse_func_decl(receiver, attributes, false)?;
            return Ok(Decl::Func(func_decl));
        }

        if self.match_tok(&TokenKind::Type) {
            let (name, span) = self.consume_ident("Expected type name")?;
            if self.match_tok(&TokenKind::Struct) {
                let struct_attrs = self.parse_attributes()?;
                self.consume(&TokenKind::LBrace, "Expected '{' after 'struct'")?;
                let mut fields = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                    self.skip_semicolons();
                    if self.check(&TokenKind::RBrace) {
                        break;
                    }
                    let (field_name, field_span) = self.consume_ident("Expected field name in struct")?;
                    let field_ty = self.parse_type()?;
                    fields.push(StructField {
                        name: field_name,
                        ty: field_ty,
                        span: field_span,
                    });
                    self.skip_semicolons();
                }
                self.consume(&TokenKind::RBrace, "Expected '}' after struct body")?;
                let mut all_attrs = attributes;
                all_attrs.extend(struct_attrs);
                return Ok(Decl::TypeDecl {
                    name,
                    def: TypeDef::Struct {
                        fields,
                        attributes: all_attrs,
                    },
                    span,
                });
            } else {
                // Type alias / newtype: type ID u16
                let ty = self.parse_type()?;
                return Ok(Decl::TypeDecl {
                    name,
                    def: TypeDef::Alias(ty),
                    span,
                });
            }
        }

        if self.match_tok(&TokenKind::Var) {
            let (name, span) = self.consume_ident("Expected variable name after 'var'")?;
            let ty = if !self.check(&TokenKind::Assign) && !self.check(&TokenKind::Semicolon) && !self.check(&TokenKind::AtSection) && !self.check(&TokenKind::AtAlign) {
                Some(self.parse_type()?)
            } else {
                None
            };

            let trailing_attrs = self.parse_attributes()?;
            attributes.extend(trailing_attrs);

            let init = if self.match_tok(&TokenKind::Assign) {
                Some(self.parse_expr()?)
            } else {
                None
            };

            let post_attrs = self.parse_attributes()?;
            attributes.extend(post_attrs);

            return Ok(Decl::GlobalVar {
                name,
                ty,
                init,
                attributes,
                span,
            });
        }

        if self.match_tok(&TokenKind::Const) {
            let (name, span) = self.consume_ident("Expected constant name after 'const'")?;
            let ty = if !self.check(&TokenKind::Assign) {
                Some(self.parse_type()?)
            } else {
                None
            };
            self.consume(&TokenKind::Assign, "Expected '=' in const declaration")?;
            let init = self.parse_expr()?;
            return Ok(Decl::Const {
                name,
                ty,
                init,
                span,
            });
        }

        let tok = self.peek();
        Err(format!("Unexpected token in top-level declaration at {}: {:?}", tok.span, tok.kind))
    }

    fn parse_receiver(&mut self) -> Result<Receiver, String> {
        self.consume(&TokenKind::LParen, "Expected '(' for method receiver")?;
        let (name, span) = self.consume_ident("Expected receiver parameter name")?;
        let ty = self.parse_type()?;
        self.consume(&TokenKind::RParen, "Expected ')' after method receiver")?;
        Ok(Receiver { name, ty, span })
    }

    fn parse_func_decl(
        &mut self,
        receiver: Option<Receiver>,
        mut attributes: Vec<Attribute>,
        is_extern: bool,
    ) -> Result<FuncDecl, String> {
        let (name, span) = self.consume_ident("Expected function name")?;

        self.consume(&TokenKind::LParen, "Expected '(' after function name")?;
        let mut params = Vec::new();
        let mut is_variadic = false;
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            if self.match_tok(&TokenKind::Ellipsis) {
                is_variadic = true;
                break;
            }
            let (param_name, param_span) = self.consume_ident("Expected parameter name")?;
            let param_ty = self.parse_type()?;
            params.push(Param {
                name: param_name,
                ty: param_ty,
                span: param_span,
            });
            if !self.match_tok(&TokenKind::Comma) {
                break;
            }
        }
        self.consume(&TokenKind::RParen, "Expected ')' after function parameters")?;

        // Return type?
        let ret_type = if !self.check(&TokenKind::LBrace)
            && !self.check(&TokenKind::Semicolon)
            && !self.check(&TokenKind::AtNaked)
            && !self.check(&TokenKind::AtSection)
            && !self.check(&TokenKind::AtExport)
            && !self.check(&TokenKind::AtInterrupt)
        {
            Some(self.parse_type()?)
        } else {
            None
        };

        let more_attrs = self.parse_attributes()?;
        attributes.extend(more_attrs);

        let body = if is_extern || self.check(&TokenKind::Semicolon) && !self.check(&TokenKind::LBrace) {
            None
        } else {
            Some(self.parse_block()?)
        };

        Ok(FuncDecl {
            receiver,
            name,
            params,
            ret_type,
            body,
            attributes,
            is_extern,
            is_variadic,
            span,
        })
    }

    fn parse_attributes(&mut self) -> Result<Vec<Attribute>, String> {
        let mut attrs = Vec::new();
        loop {
            if self.match_tok(&TokenKind::AtPacked) {
                attrs.push(Attribute::Packed);
            } else if self.match_tok(&TokenKind::AtNaked) {
                attrs.push(Attribute::Naked);
            } else if self.match_tok(&TokenKind::AtInterrupt) {
                attrs.push(Attribute::Interrupt);
            } else if self.match_tok(&TokenKind::AtAlign) {
                self.consume(&TokenKind::LParen, "Expected '(' after @align")?;
                let tok = self.advance();
                if let TokenKind::IntLit(val) = tok.kind {
                    self.consume(&TokenKind::RParen, "Expected ')' after @align value")?;
                    attrs.push(Attribute::Align(val as usize));
                } else {
                    return Err(format!("Expected integer literal in @align at {}", tok.span));
                }
            } else if self.match_tok(&TokenKind::AtSection) {
                self.consume(&TokenKind::LParen, "Expected '(' after @section")?;
                let tok = self.advance();
                if let TokenKind::StringLit(val) = tok.kind {
                    self.consume(&TokenKind::RParen, "Expected ')' after @section value")?;
                    attrs.push(Attribute::Section(val));
                } else {
                    return Err(format!("Expected string literal in @section at {}", tok.span));
                }
            } else if self.match_tok(&TokenKind::AtExport) {
                self.consume(&TokenKind::LParen, "Expected '(' after @export")?;
                let tok = self.advance();
                if let TokenKind::StringLit(val) = tok.kind {
                    self.consume(&TokenKind::RParen, "Expected ')' after @export value")?;
                    attrs.push(Attribute::Export(val));
                } else {
                    return Err(format!("Expected string literal in @export at {}", tok.span));
                }
            } else {
                break;
            }
        }
        Ok(attrs)
    }

    pub fn parse_type(&mut self) -> Result<TypeNode, String> {
        let tok = self.peek().clone();
        if self.match_tok(&TokenKind::Star) {
            let inner = self.parse_type()?;
            Ok(TypeNode::Pointer(Box::new(inner), tok.span))
        } else if self.match_tok(&TokenKind::LBracket) {
            if self.match_tok(&TokenKind::RBracket) {
                // Slice: []T
                let inner = self.parse_type()?;
                Ok(TypeNode::Slice(Box::new(inner), tok.span))
            } else {
                // Array: [N]T — N может быть целочисленным литералом или
                // именем константы (const STACK_SIZE u64 = 16384).
                let size_tok = self.advance();
                let size = match size_tok.kind {
                    TokenKind::IntLit(n) => n as usize,
                    TokenKind::Ident(ref name) => {
                        self.const_values.get(name).copied().ok_or_else(|| {
                            format!("Expected integer array size at {}: unknown const '{}'", size_tok.span, name)
                        })? as usize
                    }
                    _ => {
                        return Err(format!("Expected integer array size at {}", size_tok.span));
                    }
                };
                self.consume(&TokenKind::RBracket, "Expected ']' after array size")?;
                let inner = self.parse_type()?;
                Ok(TypeNode::Array(Box::new(inner), size, tok.span))
            }
        } else if let TokenKind::Ident(ref name) = tok.kind {
            self.advance();
            Ok(TypeNode::Named(name.clone(), tok.span))
        } else {
            Err(format!("Expected type name at {}: {:?}", tok.span, tok.kind))
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.consume(&TokenKind::LBrace, "Expected '{' to start block")?;
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            self.skip_semicolons();
            if self.check(&TokenKind::RBrace) {
                break;
            }
            let stmt = self.parse_stmt()?;
            stmts.push(stmt);
            self.skip_semicolons();
        }
        self.consume(&TokenKind::RBrace, "Expected '}' to end block")?;
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, String> {
        let tok = self.peek().clone();

        if self.match_tok(&TokenKind::Var) {
            let (name, span) = self.consume_ident("Expected variable name after 'var'")?;
            let ty = if !self.check(&TokenKind::Assign) && !self.check(&TokenKind::Semicolon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            let init = if self.match_tok(&TokenKind::Assign) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            return Ok(Stmt::VarDecl {
                name,
                ty,
                init,
                attributes: Vec::new(),
                span,
            });
        }

        if self.match_tok(&TokenKind::Const) {
            let (name, span) = self.consume_ident("Expected constant name after 'const'")?;
            let ty = if !self.check(&TokenKind::Assign) {
                Some(self.parse_type()?)
            } else {
                None
            };
            self.consume(&TokenKind::Assign, "Expected '=' in const declaration")?;
            let init = self.parse_expr()?;
            return Ok(Stmt::ConstDecl {
                name,
                ty,
                init,
                span,
            });
        }

        if self.match_tok(&TokenKind::Return) {
            let value = if !self.check(&TokenKind::Semicolon) && !self.check(&TokenKind::RBrace) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            return Ok(Stmt::Return {
                value,
                span: tok.span,
            });
        }

        if self.match_tok(&TokenKind::Defer) {
            let call = self.parse_expr()?;
            return Ok(Stmt::Defer {
                call,
                span: tok.span,
            });
        }

        if self.match_tok(&TokenKind::Break) {
            return Ok(Stmt::Break(tok.span));
        }

        if self.match_tok(&TokenKind::Continue) {
            return Ok(Stmt::Continue(tok.span));
        }

        if self.match_tok(&TokenKind::If) {
            return self.parse_if_stmt(tok.span);
        }

        if self.match_tok(&TokenKind::For) {
            return self.parse_for_stmt(tok.span);
        }

        if self.match_tok(&TokenKind::Switch) {
            return self.parse_switch_stmt(tok.span);
        }

        if self.check(&TokenKind::LBrace) {
            let block = self.parse_block()?;
            return Ok(Stmt::Block(block, tok.span));
        }

        // Short var decl: `ident := expr`
        if let TokenKind::Ident(ref name) = tok.kind {
            if let Some(next) = self.peek_next() {
                if next.kind == TokenKind::ColonAssign {
                    let name = name.clone();
                    let span = tok.span;
                    self.advance(); // consume ident
                    self.advance(); // consume :=
                    let init = self.parse_expr()?;
                    return Ok(Stmt::ShortVarDecl { name, init, span });
                }
            }
        }

        // Expression statement or Assignment
        let expr = self.parse_expr()?;

        // Assignment operators
        let assign_op = match self.peek_kind() {
            TokenKind::Assign => Some(AssignOp::Assign),
            TokenKind::PlusAssign => Some(AssignOp::AddAssign),
            TokenKind::MinusAssign => Some(AssignOp::SubAssign),
            TokenKind::StarAssign => Some(AssignOp::MulAssign),
            TokenKind::SlashAssign => Some(AssignOp::DivAssign),
            TokenKind::PercentAssign => Some(AssignOp::ModAssign),
            TokenKind::AmpAssign => Some(AssignOp::AndAssign),
            TokenKind::PipeAssign => Some(AssignOp::OrAssign),
            TokenKind::CaretAssign => Some(AssignOp::XorAssign),
            TokenKind::ShlAssign => Some(AssignOp::ShlAssign),
            TokenKind::ShrAssign => Some(AssignOp::ShrAssign),
            _ => None,
        };

        if let Some(op) = assign_op {
            self.advance(); // consume operator
            let value = self.parse_expr()?;
            return Ok(Stmt::Assign {
                target: expr,
                op,
                value,
                span: tok.span,
            });
        }

        Ok(Stmt::Expr(expr))
    }

    fn parse_if_stmt(&mut self, span: Span) -> Result<Stmt, String> {
        // Go syntax: `if [init;] cond { ... } else { ... }`
        let first = self.parse_simple_stmt_or_expr()?;

        let (init, cond) = if self.match_tok(&TokenKind::Semicolon) {
            let cond = self.parse_expr()?;
            (Some(Box::new(first)), cond)
        } else {
            // No init statement, first is the condition expression
            let cond_expr = match first {
                Stmt::Expr(e) => e,
                _ => return Err(format!("Expected boolean condition at {}", span)),
            };
            (None, cond_expr)
        };

        let then_block = self.parse_block()?;
        let else_block = if self.match_tok(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                let else_if_tok = self.advance();
                let else_if_stmt = self.parse_if_stmt(else_if_tok.span)?;
                Some(vec![else_if_stmt])
            } else {
                Some(self.parse_block()?)
            }
        } else {
            None
        };

        Ok(Stmt::If {
            init,
            cond,
            then_block,
            else_block,
            span,
        })
    }

    fn parse_for_stmt(&mut self, span: Span) -> Result<Stmt, String> {
        // 1. Infinite loop: `for { ... }`
        if self.check(&TokenKind::LBrace) {
            let body = self.parse_block()?;
            return Ok(Stmt::For {
                init: None,
                cond: None,
                post: None,
                body,
                span,
            });
        }

        // 2. Three-clause or condition-only:
        // `for init; cond; post { ... }` or `for cond { ... }`
        let first = self.parse_simple_stmt_or_expr()?;

        if self.match_tok(&TokenKind::Semicolon) {
            // Three-clause for loop: `for init; cond; post { ... }`
            let init = Some(Box::new(first));
            let cond = if !self.check(&TokenKind::Semicolon) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.consume(&TokenKind::Semicolon, "Expected ';' after for condition")?;
            let post = if !self.check(&TokenKind::LBrace) {
                Some(Box::new(self.parse_simple_stmt_or_expr()?))
            } else {
                None
            };
            let body = self.parse_block()?;
            Ok(Stmt::For {
                init,
                cond,
                post,
                body,
                span,
            })
        } else {
            // Condition-only: `for cond { ... }`
            let cond_expr = match first {
                Stmt::Expr(e) => e,
                _ => return Err(format!("Expected boolean condition in for loop at {}", span)),
            };
            let body = self.parse_block()?;
            Ok(Stmt::For {
                init: None,
                cond: Some(cond_expr),
                post: None,
                body,
                span,
            })
        }
    }

    fn parse_simple_stmt_or_expr(&mut self) -> Result<Stmt, String> {
        let tok = self.peek().clone();

        // Short var decl: `ident := expr`
        if let TokenKind::Ident(ref name) = tok.kind {
            if let Some(next) = self.peek_next() {
                if next.kind == TokenKind::ColonAssign {
                    let name = name.clone();
                    let span = tok.span;
                    self.advance(); // consume ident
                    self.advance(); // consume :=
                    let init = self.parse_expr()?;
                    return Ok(Stmt::ShortVarDecl { name, init, span });
                }
            }
        }

        if self.match_tok(&TokenKind::Var) {
            let (name, span) = self.consume_ident("Expected variable name after 'var'")?;
            let ty = if !self.check(&TokenKind::Assign) && !self.check(&TokenKind::Semicolon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            let init = if self.match_tok(&TokenKind::Assign) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            return Ok(Stmt::VarDecl {
                name,
                ty,
                init,
                attributes: Vec::new(),
                span,
            });
        }

        let expr = self.parse_expr()?;
        let assign_op = match self.peek_kind() {
            TokenKind::Assign => Some(AssignOp::Assign),
            TokenKind::PlusAssign => Some(AssignOp::AddAssign),
            TokenKind::MinusAssign => Some(AssignOp::SubAssign),
            TokenKind::StarAssign => Some(AssignOp::MulAssign),
            TokenKind::SlashAssign => Some(AssignOp::DivAssign),
            TokenKind::PercentAssign => Some(AssignOp::ModAssign),
            TokenKind::AmpAssign => Some(AssignOp::AndAssign),
            TokenKind::PipeAssign => Some(AssignOp::OrAssign),
            TokenKind::CaretAssign => Some(AssignOp::XorAssign),
            TokenKind::ShlAssign => Some(AssignOp::ShlAssign),
            TokenKind::ShrAssign => Some(AssignOp::ShrAssign),
            _ => None,
        };

        if let Some(op) = assign_op {
            self.advance();
            let value = self.parse_expr()?;
            return Ok(Stmt::Assign {
                target: expr,
                op,
                value,
                span: tok.span,
            });
        }

        Ok(Stmt::Expr(expr))
    }

    fn parse_switch_stmt(&mut self, span: Span) -> Result<Stmt, String> {
        let expr = if !self.check(&TokenKind::LBrace) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.consume(&TokenKind::LBrace, "Expected '{' to start switch body")?;
        let mut cases = Vec::new();
        let mut default_case = None;

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            self.skip_semicolons();
            if self.check(&TokenKind::RBrace) {
                break;
            }
            if self.match_tok(&TokenKind::Case) {
                let case_span = self.peek().span;
                let mut values = Vec::new();
                loop {
                    values.push(self.parse_expr()?);
                    if !self.match_tok(&TokenKind::Comma) {
                        break;
                    }
                }
                self.consume(&TokenKind::Colon, "Expected ':' after case values")?;
                let mut body = Vec::new();
                while !self.check(&TokenKind::Case)
                    && !self.check(&TokenKind::Default)
                    && !self.check(&TokenKind::RBrace)
                    && !self.check(&TokenKind::Eof)
                {
                    self.skip_semicolons();
                    if self.check(&TokenKind::Case) || self.check(&TokenKind::Default) || self.check(&TokenKind::RBrace) {
                        break;
                    }
                    body.push(self.parse_stmt()?);
                    self.skip_semicolons();
                }
                cases.push(SwitchCase {
                    values,
                    body,
                    span: case_span,
                });
            } else if self.match_tok(&TokenKind::Default) {
                self.consume(&TokenKind::Colon, "Expected ':' after default")?;
                let mut body = Vec::new();
                while !self.check(&TokenKind::Case)
                    && !self.check(&TokenKind::Default)
                    && !self.check(&TokenKind::RBrace)
                    && !self.check(&TokenKind::Eof)
                {
                    self.skip_semicolons();
                    if self.check(&TokenKind::Case) || self.check(&TokenKind::Default) || self.check(&TokenKind::RBrace) {
                        break;
                    }
                    body.push(self.parse_stmt()?);
                    self.skip_semicolons();
                }
                default_case = Some(body);
            } else {
                let tok = self.peek();
                return Err(format!("Expected 'case' or 'default' in switch at {}", tok.span));
            }
        }
        self.consume(&TokenKind::RBrace, "Expected '}' after switch body")?;

        Ok(Stmt::Switch {
            expr,
            cases,
            default_case,
            span,
        })
    }

    // ==========================================
    // Pratt Parser for Expressions
    // ==========================================

    pub fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_expr_precedence(0)
    }

    fn parse_expr_precedence(&mut self, min_prec: u8) -> Result<Expr, String> {
        let mut lhs = self.parse_unary_or_primary()?;

        loop {
            let (op, prec) = match self.peek_kind() {
                TokenKind::Or => (BinaryOp::LogicalOr, 1),
                TokenKind::And => (BinaryOp::LogicalAnd, 2),
                TokenKind::Eq => (BinaryOp::Eq, 3),
                TokenKind::NotEq => (BinaryOp::NotEq, 3),
                TokenKind::Lt => (BinaryOp::Lt, 3),
                TokenKind::LtEq => (BinaryOp::LtEq, 3),
                TokenKind::Gt => (BinaryOp::Gt, 3),
                TokenKind::GtEq => (BinaryOp::GtEq, 3),
                TokenKind::Pipe => (BinaryOp::BitOr, 4),
                TokenKind::Caret => (BinaryOp::BitXor, 5),
                TokenKind::Amp => (BinaryOp::BitAnd, 6),
                TokenKind::AmpCaret => (BinaryOp::BitClear, 6),
                TokenKind::Shl => (BinaryOp::Shl, 7),
                TokenKind::Shr => (BinaryOp::Shr, 7),
                TokenKind::Plus => (BinaryOp::Add, 8),
                TokenKind::Minus => (BinaryOp::Sub, 8),
                TokenKind::Star => (BinaryOp::Mul, 9),
                TokenKind::Slash => (BinaryOp::Div, 9),
                TokenKind::Percent => (BinaryOp::Mod, 9),
                _ => break,
            };

            if prec < min_prec {
                break;
            }

            self.advance();
            let rhs = self.parse_expr_precedence(prec + 1)?;
            let span = lhs.span();
            lhs = Expr::Binary(Box::new(lhs), op, Box::new(rhs), span);
        }

        Ok(lhs)
    }

    fn parse_unary_or_primary(&mut self) -> Result<Expr, String> {
        let tok = self.peek().clone();

        match tok.kind {
            TokenKind::Minus => {
                self.advance();
                let inner = self.parse_unary_or_primary()?;
                Ok(Expr::Unary(UnaryOp::Neg, Box::new(inner), tok.span))
            }
            TokenKind::Bang => {
                self.advance();
                let inner = self.parse_unary_or_primary()?;
                Ok(Expr::Unary(UnaryOp::Not, Box::new(inner), tok.span))
            }
            TokenKind::Caret => {
                self.advance();
                let inner = self.parse_unary_or_primary()?;
                Ok(Expr::Unary(UnaryOp::BitNot, Box::new(inner), tok.span))
            }
            TokenKind::Amp => {
                self.advance();
                let inner = self.parse_unary_or_primary()?;
                Ok(Expr::Unary(UnaryOp::AddrOf, Box::new(inner), tok.span))
            }
            TokenKind::Star => {
                self.advance();
                let inner = self.parse_unary_or_primary()?;
                Ok(Expr::Unary(UnaryOp::Deref, Box::new(inner), tok.span))
            }
            TokenKind::AtVolatileLoad => {
                self.advance();
                self.consume(&TokenKind::LParen, "Expected '(' after @volatile_load")?;
                let ptr = self.parse_expr()?;
                self.consume(&TokenKind::RParen, "Expected ')' after @volatile_load pointer")?;
                Ok(Expr::VolatileLoad {
                    ptr: Box::new(ptr),
                    span: tok.span,
                })
            }
            TokenKind::AtVolatileStore => {
                self.advance();
                self.consume(&TokenKind::LParen, "Expected '(' after @volatile_store")?;
                let ptr = self.parse_expr()?;
                self.consume(&TokenKind::Comma, "Expected ',' between pointer and value in @volatile_store")?;
                let val = self.parse_expr()?;
                self.consume(&TokenKind::RParen, "Expected ')' after @volatile_store value")?;
                Ok(Expr::VolatileStore {
                    ptr: Box::new(ptr),
                    val: Box::new(val),
                    span: tok.span,
                })
            }
            TokenKind::InlineAsm => {
                self.advance();
                self.parse_inline_asm(tok.span)
            }
            _ => self.parse_primary_and_postfix(),
        }
    }

    fn parse_inline_asm(&mut self, span: Span) -> Result<Expr, String> {
        self.consume(&TokenKind::LParen, "Expected '(' after 'inline_asm'")?;
        let asm_tok = self.advance();
        let asm_string = match asm_tok.kind {
            TokenKind::StringLit(s) => s,
            _ => return Err(format!("Expected string literal for inline assembly at {}", asm_tok.span)),
        };

        let mut outputs = Vec::new();
        let mut inputs = Vec::new();
        let mut clobbers = Vec::new();

        if self.match_tok(&TokenKind::Colon) {
            // Output operands
            while !self.check(&TokenKind::Colon) && !self.check(&TokenKind::RParen) {
                let op = self.parse_asm_operand()?;
                outputs.push(op);
                if !self.match_tok(&TokenKind::Comma) {
                    break;
                }
            }

            if self.match_tok(&TokenKind::Colon) {
                // Input operands
                while !self.check(&TokenKind::Colon) && !self.check(&TokenKind::RParen) {
                    let op = self.parse_asm_operand()?;
                    inputs.push(op);
                    if !self.match_tok(&TokenKind::Comma) {
                        break;
                    }
                }

                if self.match_tok(&TokenKind::Colon) {
                    // Clobbers
                    while !self.check(&TokenKind::RParen) {
                        let clobber_tok = self.advance();
                        if let TokenKind::StringLit(s) = clobber_tok.kind {
                            clobbers.push(s);
                        } else {
                            return Err(format!("Expected string literal clobber at {}", clobber_tok.span));
                        }
                        if !self.match_tok(&TokenKind::Comma) {
                            break;
                        }
                    }
                }
            }
        }

        self.consume(&TokenKind::RParen, "Expected ')' after inline assembly")?;
        Ok(Expr::InlineAsm {
            asm_string,
            outputs,
            inputs,
            clobbers,
            span,
        })
    }

    fn parse_asm_operand(&mut self) -> Result<AsmOperand, String> {
        let constr_tok = self.advance();
        let constraint = match constr_tok.kind {
            TokenKind::StringLit(s) => s,
            _ => return Err(format!("Expected constraint string at {}", constr_tok.span)),
        };
        self.consume(&TokenKind::LParen, "Expected '(' around asm operand expression")?;
        let expr = self.parse_expr()?;
        self.consume(&TokenKind::RParen, "Expected ')' around asm operand expression")?;
        Ok(AsmOperand { constraint, expr })
    }

    fn parse_primary_and_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_tok(&TokenKind::LParen) {
                // Call: expr(...)
                let mut args = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    args.push(self.parse_expr()?);
                    if !self.match_tok(&TokenKind::Comma) {
                        break;
                    }
                }
                self.consume(&TokenKind::RParen, "Expected ')' after function arguments")?;
                let span = expr.span();
                expr = Expr::Call(Box::new(expr), args, span);
            } else if self.match_tok(&TokenKind::Dot) {
                // Member access: expr.field
                let (field_name, _) = self.consume_ident("Expected member name after '.'")?;
                let span = expr.span();
                expr = Expr::MemberAccess(Box::new(expr), field_name, span);
            } else if self.match_tok(&TokenKind::LBracket) {
                // Index or Slice: expr[i] or expr[low:high] or expr[:high] or expr[low:]
                let span = expr.span();
                if self.match_tok(&TokenKind::Colon) || self.match_tok(&TokenKind::DotDot) {
                    // expr[:high] or expr[:]
                    let high = if !self.check(&TokenKind::RBracket) {
                        Some(Box::new(self.parse_expr()?))
                    } else {
                        None
                    };
                    self.consume(&TokenKind::RBracket, "Expected ']' after slice")?;
                    expr = Expr::Slice {
                        expr: Box::new(expr),
                        low: None,
                        high,
                        span,
                    };
                } else {
                    let first_expr = self.parse_expr()?;
                    if self.match_tok(&TokenKind::Colon) || self.match_tok(&TokenKind::DotDot) {
                        // expr[low:high] or expr[low:]
                        let high = if !self.check(&TokenKind::RBracket) {
                            Some(Box::new(self.parse_expr()?))
                        } else {
                            None
                        };
                        self.consume(&TokenKind::RBracket, "Expected ']' after slice")?;
                        expr = Expr::Slice {
                            expr: Box::new(expr),
                            low: Some(Box::new(first_expr)),
                            high,
                            span,
                        };
                    } else {
                        // Index: expr[idx]
                        self.consume(&TokenKind::RBracket, "Expected ']' after index")?;
                        expr = Expr::Index(Box::new(expr), Box::new(first_expr), span);
                    }
                }
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let tok = self.advance();

        match tok.kind {
            TokenKind::IntLit(val) => Ok(Expr::IntLit(val, tok.span)),
            TokenKind::FloatLit(val) => Ok(Expr::FloatLit(val, tok.span)),
            TokenKind::StringLit(val) => Ok(Expr::StringLit(val, tok.span)),
            TokenKind::CharLit(val) => Ok(Expr::CharLit(val, tok.span)),
            TokenKind::Ident(ref s) if s == "true" => Ok(Expr::BoolLit(true, tok.span)),
            TokenKind::Ident(ref s) if s == "false" => Ok(Expr::BoolLit(false, tok.span)),
            TokenKind::Ident(ref s) => {
                let name = s.clone();
                // Check if this is a struct literal: TypeName{ field: val, ... }
                // Only if followed by `{` and inside the brace is `ident:` or empty `}`
                if self.check(&TokenKind::LBrace) && self.looks_like_struct_literal() {
                    self.advance(); // consume {
                    let mut fields = Vec::new();
                    while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                        self.skip_semicolons();
                        if self.check(&TokenKind::RBrace) {
                            break;
                        }
                        let (field_name, f_span) = self.consume_ident("Expected field name in struct literal")?;
                        self.consume(&TokenKind::Colon, "Expected ':' after field name in struct literal")?;
                        let val = self.parse_expr()?;
                        fields.push(NamedFieldInit {
                            name: field_name,
                            value: val,
                            span: f_span,
                        });
                        self.match_tok(&TokenKind::Comma);
                        self.skip_semicolons();
                    }
                    self.consume(&TokenKind::RBrace, "Expected '}' after struct literal")?;
                    Ok(Expr::StructLiteral {
                        ty: TypeNode::Named(name, tok.span),
                        fields,
                        span: tok.span,
                    })
                } else {
                    Ok(Expr::Ident(name, tok.span))
                }
            }
            TokenKind::LParen => {
                let inner = self.parse_expr()?;
                self.consume(&TokenKind::RParen, "Expected ')' after expression")?;
                Ok(inner)
            }
            _ => Err(format!("Unexpected token in expression at {}: {:?}", tok.span, tok.kind)),
        }
    }

    fn looks_like_struct_literal(&self) -> bool {
        // We are at `{`. Look ahead to see if it's `{ ident :` or `{}`
        if self.pos < self.tokens.len() && self.tokens[self.pos].kind == TokenKind::LBrace {
            if self.pos + 1 < self.tokens.len() {
                let next = &self.tokens[self.pos + 1];
                if next.kind == TokenKind::RBrace {
                    return true;
                }
                if matches!(next.kind, TokenKind::Ident(_)) {
                    if self.pos + 2 < self.tokens.len() && self.tokens[self.pos + 2].kind == TokenKind::Colon {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_parse_basic_program() {
        let src = r#"
            package main

            import "core/fmt"

            type Point struct {
                x int
                y int
            }

            func (p *Point) Move(dx int, dy int) {
                p.x += dx
                p.y += dy
            }

            func main() int {
                var p Point = Point{ x: 10, y: 20 }
                p.Move(5, 5)
                return p.x + p.y
            }
        "#;
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();
        assert_eq!(prog.package_name, "main");
        assert_eq!(prog.imports.len(), 1);
        assert_eq!(prog.decls.len(), 3);
    }

    #[test]
    fn test_parse_osdev_attributes() {
        let src = r#"
            package main

            type IDTEntry struct @packed {
                offset_low u16
                selector   u16
            }

            var gdt [3]u64 @section(".boot") @align(4096)

            func handler() @naked {
                inline_asm("cli; hlt")
            }
        "#;
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize_all().unwrap();
        let mut parser = Parser::new(tokens);
        let prog = parser.parse_program().unwrap();
        assert_eq!(prog.decls.len(), 3);
    }
}
