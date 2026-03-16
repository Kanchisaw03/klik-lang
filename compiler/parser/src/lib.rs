// KLIK Language - Recursive Descent Parser with Pratt Precedence
// Parses token stream into a strongly-typed AST

use klik_ast::*;
use klik_lexer::{Lexer, LexerError, Token, TokenKind};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("parse error: {message} at {span}")]
    Error { message: String, span: Span },
    #[error("lexer errors")]
    LexerErrors(Vec<LexerError>),
}

impl ParseError {
    pub fn span(&self) -> Option<&Span> {
        match self {
            ParseError::Error { span, .. } => Some(span),
            ParseError::LexerErrors(_) => None,
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    file: String,
    errors: Vec<ParseError>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, file: impl Into<String>) -> Self {
        Self {
            tokens,
            pos: 0,
            file: file.into(),
            errors: Vec::new(),
        }
    }

    pub fn from_source(source: &str, file: &str) -> Result<Self, ParseError> {
        let lexer = Lexer::new(source, file);
        let tokens = lexer.tokenize().map_err(ParseError::LexerErrors)?;
        Ok(Self::new(tokens, file))
    }

    pub fn parse_program(&mut self) -> Result<Program, Vec<ParseError>> {
        let start = self.current_span();
        let mut items = Vec::new();

        while !self.is_at_end() {
            match self.parse_item() {
                Ok(item) => items.push(item),
                Err(e) => {
                    self.errors.push(e);
                    self.synchronize();
                }
            }
        }

        let span = start.merge(&self.current_span());
        let module = Module {
            name: self.file.clone(),
            items,
            span: span.clone(),
        };

        if self.errors.is_empty() {
            Ok(Program {
                modules: vec![module],
                span,
            })
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    // ========================================================================
    // Item parsing
    // ========================================================================

    fn parse_item(&mut self) -> Result<Item, ParseError> {
        let is_pub = self.check(&TokenKind::Pub);
        if is_pub {
            self.advance();
        }

        match self.peek_kind() {
            TokenKind::Fn => self.parse_function(is_pub, false).map(Item::Function),
            TokenKind::Async => {
                self.advance();
                self.parse_function(is_pub, true).map(Item::Function)
            }
            TokenKind::Struct => self.parse_struct(is_pub).map(Item::Struct),
            TokenKind::Enum => self.parse_enum(is_pub).map(Item::Enum),
            TokenKind::Trait => self.parse_trait(is_pub).map(Item::Trait),
            TokenKind::Impl => self.parse_impl().map(Item::Impl),
            TokenKind::Import => self.parse_import().map(Item::Import),
            TokenKind::Const => self.parse_const(is_pub).map(Item::Const),
            TokenKind::Type => self.parse_type_alias(is_pub).map(Item::TypeAlias),
            TokenKind::Mod => self.parse_module().map(Item::Module),
            TokenKind::Test => self.parse_test().map(Item::Test),
            _ => Err(self.error(&format!(
                "expected item declaration, found '{}'",
                self.peek_kind()
            ))),
        }
    }

    fn parse_function(&mut self, is_pub: bool, is_async: bool) -> Result<Function, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::Fn)?;
        let name = self.expect_identifier()?;

        let generic_params = if self.check(&TokenKind::Less) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        let params = if self.check(&TokenKind::LeftParen) {
            self.parse_params()?
        } else {
            Vec::new()
        };

        let return_type = if self.check(&TokenKind::Arrow) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(Function {
            name,
            generic_params,
            params,
            return_type,
            body,
            is_async,
            is_pub,
            span: start.merge(&self.prev_span()),
        })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, ParseError> {
        self.expect(&TokenKind::LeftParen)?;
        let mut params = Vec::new();

        while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
            let start = self.current_span();

            // Handle `self` parameter (for impl methods)
            if self.check(&TokenKind::SelfKw) {
                self.advance();
                params.push(Param {
                    name: "self".to_string(),
                    type_expr: TypeExpr::Named {
                        name: "Self".to_string(),
                        generic_args: Vec::new(),
                        span: start.merge(&self.prev_span()),
                    },
                    default: None,
                    span: start.merge(&self.prev_span()),
                });
                if !self.check(&TokenKind::RightParen) {
                    self.expect(&TokenKind::Comma)?;
                }
                continue;
            }

            let name = self.expect_identifier()?;
            self.expect(&TokenKind::Colon)?;
            let type_expr = self.parse_type_expr()?;

            let default = if self.check(&TokenKind::Eq) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };

            params.push(Param {
                name,
                type_expr,
                default,
                span: start.merge(&self.prev_span()),
            });

            if !self.check(&TokenKind::RightParen) {
                self.expect(&TokenKind::Comma)?;
            }
        }

        self.expect(&TokenKind::RightParen)?;
        Ok(params)
    }

    fn parse_generic_params(&mut self) -> Result<Vec<GenericParam>, ParseError> {
        self.expect(&TokenKind::Less)?;
        let mut params = Vec::new();

        while !self.check(&TokenKind::Greater) && !self.is_at_end() {
            let start = self.current_span();
            let name = self.expect_identifier()?;

            let mut bounds = Vec::new();
            if self.check(&TokenKind::Colon) {
                self.advance();
                bounds.push(self.parse_type_expr()?);
                while self.check(&TokenKind::Plus) {
                    self.advance();
                    bounds.push(self.parse_type_expr()?);
                }
            }

            let default = if self.check(&TokenKind::Eq) {
                self.advance();
                Some(self.parse_type_expr()?)
            } else {
                None
            };

            params.push(GenericParam {
                name,
                bounds,
                default,
                span: start.merge(&self.prev_span()),
            });

            if !self.check(&TokenKind::Greater) {
                self.expect(&TokenKind::Comma)?;
            }
        }

        self.expect(&TokenKind::Greater)?;
        Ok(params)
    }

    fn parse_struct(&mut self, is_pub: bool) -> Result<StructDef, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::Struct)?;
        let name = self.expect_identifier()?;

        let generic_params = if self.check(&TokenKind::Less) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        self.expect(&TokenKind::LeftBrace)?;
        let mut fields = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let field_start = self.current_span();
            let field_pub = if self.check(&TokenKind::Pub) {
                self.advance();
                true
            } else {
                false
            };
            let field_name = self.expect_identifier()?;
            self.expect(&TokenKind::Colon)?;
            let field_type = self.parse_type_expr()?;

            fields.push(FieldDef {
                name: field_name,
                type_expr: field_type,
                is_pub: field_pub,
                span: field_start.merge(&self.prev_span()),
            });

            // Optional comma/newline separator
            self.match_token(&TokenKind::Comma);
        }

        self.expect(&TokenKind::RightBrace)?;

        Ok(StructDef {
            name,
            generic_params,
            fields,
            is_pub,
            span: start.merge(&self.prev_span()),
        })
    }

    fn parse_enum(&mut self, is_pub: bool) -> Result<EnumDef, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::Enum)?;
        let name = self.expect_identifier()?;

        let generic_params = if self.check(&TokenKind::Less) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        self.expect(&TokenKind::LeftBrace)?;
        let mut variants = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let var_start = self.current_span();
            let var_name = self.expect_identifier()?;

            let fields = if self.check(&TokenKind::LeftParen) {
                self.advance();
                let mut f = Vec::new();
                while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
                    f.push(self.parse_type_expr()?);
                    if !self.check(&TokenKind::RightParen) {
                        self.expect(&TokenKind::Comma)?;
                    }
                }
                self.expect(&TokenKind::RightParen)?;
                f
            } else {
                Vec::new()
            };

            variants.push(EnumVariant {
                name: var_name,
                fields,
                span: var_start.merge(&self.prev_span()),
            });

            self.match_token(&TokenKind::Comma);
        }

        self.expect(&TokenKind::RightBrace)?;

        Ok(EnumDef {
            name,
            generic_params,
            variants,
            is_pub,
            span: start.merge(&self.prev_span()),
        })
    }

    fn parse_trait(&mut self, is_pub: bool) -> Result<TraitDef, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::Trait)?;
        let name = self.expect_identifier()?;

        let generic_params = if self.check(&TokenKind::Less) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        self.expect(&TokenKind::LeftBrace)?;
        let mut methods = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let m_start = self.current_span();
            self.expect(&TokenKind::Fn)?;
            let m_name = self.expect_identifier()?;
            let params = if self.check(&TokenKind::LeftParen) {
                self.parse_params()?
            } else {
                Vec::new()
            };

            let return_type = if self.check(&TokenKind::Arrow) {
                self.advance();
                Some(self.parse_type_expr()?)
            } else {
                None
            };

            let default_body = if self.check(&TokenKind::LeftBrace) {
                Some(self.parse_block()?)
            } else {
                None
            };

            methods.push(TraitMethod {
                name: m_name,
                params,
                return_type,
                default_body,
                span: m_start.merge(&self.prev_span()),
            });
        }

        self.expect(&TokenKind::RightBrace)?;

        Ok(TraitDef {
            name,
            generic_params,
            methods,
            is_pub,
            span: start.merge(&self.prev_span()),
        })
    }

    fn parse_impl(&mut self) -> Result<ImplBlock, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::Impl)?;

        let generic_params = if self.check(&TokenKind::Less) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        let first_name = self.expect_identifier()?;

        let (type_name, trait_name) = if self.check(&TokenKind::For) {
            self.advance();
            let tn = self.expect_identifier()?;
            (tn, Some(first_name))
        } else {
            (first_name, None)
        };

        self.expect(&TokenKind::LeftBrace)?;
        let mut methods = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let is_pub = if self.check(&TokenKind::Pub) {
                self.advance();
                true
            } else {
                false
            };
            methods.push(self.parse_function(is_pub, false)?);
        }

        self.expect(&TokenKind::RightBrace)?;

        Ok(ImplBlock {
            type_name,
            generic_params,
            trait_name,
            methods,
            span: start.merge(&self.prev_span()),
        })
    }

    fn parse_import(&mut self) -> Result<ImportDecl, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::Import)?;

        let mut path = Vec::new();
        path.push(self.expect_identifier()?);
        while self.check(&TokenKind::ColonColon) {
            self.advance();
            if self.check(&TokenKind::LeftBrace) {
                // import foo::bar::{baz, qux}
                self.advance();
                let mut items = Vec::new();
                while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
                    items.push(self.expect_identifier()?);
                    if !self.check(&TokenKind::RightBrace) {
                        self.expect(&TokenKind::Comma)?;
                    }
                }
                self.expect(&TokenKind::RightBrace)?;
                return Ok(ImportDecl {
                    path,
                    alias: None,
                    items: Some(items),
                    span: start.merge(&self.prev_span()),
                });
            }
            path.push(self.expect_identifier()?);
        }

        let alias = if self.check(&TokenKind::As) {
            self.advance();
            Some(self.expect_identifier()?)
        } else {
            None
        };

        Ok(ImportDecl {
            path,
            alias,
            items: None,
            span: start.merge(&self.prev_span()),
        })
    }

    fn parse_const(&mut self, is_pub: bool) -> Result<ConstDecl, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::Const)?;
        let name = self.expect_identifier()?;

        let type_expr = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        self.expect(&TokenKind::Eq)?;
        let value = self.parse_expr()?;

        Ok(ConstDecl {
            name,
            type_expr,
            value,
            is_pub,
            span: start.merge(&self.prev_span()),
        })
    }

    fn parse_type_alias(&mut self, is_pub: bool) -> Result<TypeAlias, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::Type)?;
        let name = self.expect_identifier()?;

        let generic_params = if self.check(&TokenKind::Less) {
            self.parse_generic_params()?
        } else {
            Vec::new()
        };

        self.expect(&TokenKind::Eq)?;
        let type_expr = self.parse_type_expr()?;

        Ok(TypeAlias {
            name,
            generic_params,
            type_expr,
            is_pub,
            span: start.merge(&self.prev_span()),
        })
    }

    fn parse_module(&mut self) -> Result<Module, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::Mod)?;
        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LeftBrace)?;

        let mut items = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            items.push(self.parse_item()?);
        }

        self.expect(&TokenKind::RightBrace)?;

        Ok(Module {
            name,
            items,
            span: start.merge(&self.prev_span()),
        })
    }

    fn parse_test(&mut self) -> Result<TestDecl, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::Test)?;
        let name = match self.peek_kind() {
            TokenKind::StringLiteral(s) => {
                let n = s.clone();
                self.advance();
                n
            }
            _ => return Err(self.error("expected test name string")),
        };
        let body = self.parse_block()?;

        Ok(TestDecl {
            name,
            body,
            span: start.merge(&self.prev_span()),
        })
    }

    // ========================================================================
    // Type expression parsing
    // ========================================================================

    fn parse_type_expr(&mut self) -> Result<TypeExpr, ParseError> {
        let start = self.current_span();

        // Check for reference types
        if self.check(&TokenKind::Amp) {
            self.advance();
            let mutable = self.match_token(&TokenKind::Mut);
            let inner = self.parse_type_expr()?;
            return Ok(TypeExpr::Reference {
                inner: Box::new(inner),
                mutable,
                span: start.merge(&self.prev_span()),
            });
        }

        // Array types
        if self.check(&TokenKind::LeftBracket) {
            self.advance();
            let element = self.parse_type_expr()?;
            let size = if self.check(&TokenKind::Semicolon) {
                self.advance();
                Some(Box::new(self.parse_expr()?))
            } else {
                None
            };
            self.expect(&TokenKind::RightBracket)?;
            return Ok(TypeExpr::Array {
                element: Box::new(element),
                size,
                span: start.merge(&self.prev_span()),
            });
        }

        // Tuple types
        if self.check(&TokenKind::LeftParen) {
            self.advance();
            let mut elements = Vec::new();
            while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
                elements.push(self.parse_type_expr()?);
                if !self.check(&TokenKind::RightParen) {
                    self.expect(&TokenKind::Comma)?;
                }
            }
            self.expect(&TokenKind::RightParen)?;
            return Ok(TypeExpr::Tuple {
                elements,
                span: start.merge(&self.prev_span()),
            });
        }

        // Function type: fn(T, U) -> V
        if self.check(&TokenKind::Fn) {
            self.advance();
            self.expect(&TokenKind::LeftParen)?;
            let mut params = Vec::new();
            while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
                params.push(self.parse_type_expr()?);
                if !self.check(&TokenKind::RightParen) {
                    self.expect(&TokenKind::Comma)?;
                }
            }
            self.expect(&TokenKind::RightParen)?;
            self.expect(&TokenKind::Arrow)?;
            let return_type = self.parse_type_expr()?;
            return Ok(TypeExpr::Function {
                params,
                return_type: Box::new(return_type),
                span: start.merge(&self.prev_span()),
            });
        }

        // Named type
        let name = self.expect_identifier()?;
        let mut generic_args = Vec::new();

        if self.check(&TokenKind::Less) {
            self.advance();
            while !self.check(&TokenKind::Greater) && !self.is_at_end() {
                generic_args.push(self.parse_type_expr()?);
                if !self.check(&TokenKind::Greater) {
                    self.expect(&TokenKind::Comma)?;
                }
            }
            self.expect(&TokenKind::Greater)?;
        }

        let mut ty = TypeExpr::Named {
            name,
            generic_args,
            span: start.merge(&self.prev_span()),
        };

        // Optional type suffix: T?
        if self.check(&TokenKind::Question) {
            let q_span = self.current_span();
            self.advance();
            ty = TypeExpr::Optional {
                inner: Box::new(ty),
                span: start.merge(&q_span),
            };
        }

        Ok(ty)
    }

    // ========================================================================
    // Block and statement parsing
    // ========================================================================

    fn parse_block(&mut self) -> Result<Block, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::LeftBrace)?;

        let mut stmts = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            match self.parse_stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(e) => {
                    self.errors.push(e);
                    self.synchronize();
                }
            }
        }

        self.expect(&TokenKind::RightBrace)?;
        Ok(Block {
            stmts,
            span: start.merge(&self.prev_span()),
        })
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.peek_kind() {
            TokenKind::Let => self.parse_let_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::Break => {
                let span = self.current_span();
                self.advance();
                Ok(Stmt::Break(span))
            }
            TokenKind::Continue => {
                let span = self.current_span();
                self.advance();
                Ok(Stmt::Continue(span))
            }
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Fn
            | TokenKind::Struct
            | TokenKind::Enum
            | TokenKind::Trait
            | TokenKind::Impl
            | TokenKind::Import
            | TokenKind::Const
            | TokenKind::Type
            | TokenKind::Mod
            | TokenKind::Pub => self.parse_item().map(Stmt::Item),
            _ => {
                let expr = self.parse_expr()?;

                // Check for assignment
                if self.check(&TokenKind::Eq) {
                    let start = expr.span().clone();
                    self.advance();
                    let value = self.parse_expr()?;
                    return Ok(Stmt::Assign(AssignStmt {
                        span: start.merge(value.span()),
                        target: expr,
                        value,
                        op: None,
                    }));
                }

                // Check for compound assignment
                let compound_op = match self.peek_kind() {
                    TokenKind::PlusEq => Some(BinaryOp::Add),
                    TokenKind::MinusEq => Some(BinaryOp::Sub),
                    TokenKind::StarEq => Some(BinaryOp::Mul),
                    TokenKind::SlashEq => Some(BinaryOp::Div),
                    TokenKind::PercentEq => Some(BinaryOp::Mod),
                    _ => None,
                };

                if let Some(op) = compound_op {
                    let start = expr.span().clone();
                    self.advance();
                    let value = self.parse_expr()?;
                    return Ok(Stmt::Assign(AssignStmt {
                        span: start.merge(value.span()),
                        target: expr,
                        value,
                        op: Some(op),
                    }));
                }

                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::Let)?;
        let mutable = self.match_token(&TokenKind::Mut);
        let name = self.expect_identifier()?;

        let type_expr = if self.check(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        let value = if self.check(&TokenKind::Eq) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(Stmt::Let(LetStmt {
            name,
            type_expr,
            value,
            mutable,
            span: start.merge(&self.prev_span()),
        }))
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::Return)?;
        let value = if !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(Stmt::Return(ReturnStmt {
            value,
            span: start.merge(&self.prev_span()),
        }))
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::While)?;
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;

        Ok(Stmt::While(WhileStmt {
            condition,
            body,
            span: start.merge(&self.prev_span()),
        }))
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::For)?;
        let variable = self.expect_identifier()?;
        self.expect(&TokenKind::In)?;
        let iterator = self.parse_expr()?;
        let body = self.parse_block()?;

        Ok(Stmt::For(ForStmt {
            variable,
            iterator,
            body,
            span: start.merge(&self.prev_span()),
        }))
    }

    // ========================================================================
    // Expression parsing (Pratt parser)
    // ========================================================================

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_prefix()?;

        loop {
            if self.is_at_end() {
                break;
            }

            // Check for postfix operations
            match self.peek_kind() {
                TokenKind::Dot => {
                    self.advance();
                    let field = self.expect_identifier()?;
                    if self.check(&TokenKind::LeftParen) {
                        // Method call
                        self.advance();
                        let args = self.parse_call_args()?;
                        let span = lhs.span().merge(&self.prev_span());
                        lhs = Expr::MethodCall(MethodCallExpr {
                            receiver: Box::new(lhs),
                            method: field,
                            args,
                            generic_args: Vec::new(),
                            span,
                        });
                        continue;
                    } else {
                        let span = lhs.span().merge(&self.prev_span());
                        lhs = Expr::FieldAccess(FieldAccessExpr {
                            object: Box::new(lhs),
                            field,
                            span,
                        });
                        continue;
                    }
                }
                TokenKind::LeftBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(&TokenKind::RightBracket)?;
                    let span = lhs.span().merge(&self.prev_span());
                    lhs = Expr::Index(IndexExpr {
                        object: Box::new(lhs),
                        index: Box::new(index),
                        span,
                    });
                    continue;
                }
                TokenKind::LeftParen => {
                    self.advance();
                    let args = self.parse_call_args()?;
                    let span = lhs.span().merge(&self.prev_span());
                    lhs = Expr::Call(CallExpr {
                        callee: Box::new(lhs),
                        args,
                        generic_args: Vec::new(),
                        span,
                    });
                    continue;
                }
                TokenKind::As => {
                    self.advance();
                    let type_expr = self.parse_type_expr()?;
                    let span = lhs.span().merge(type_expr.span());
                    lhs = Expr::Cast(CastExpr {
                        expr: Box::new(lhs),
                        type_expr,
                        span,
                    });
                    continue;
                }
                _ => {}
            }

            let op = match self.peek_kind_to_binop() {
                Some(op) => op,
                None => break,
            };

            let (l_bp, r_bp) = infix_binding_power(op);
            if l_bp < min_bp {
                break;
            }

            self.advance();
            let rhs = self.parse_expr_bp(r_bp)?;
            let span = lhs.span().merge(rhs.span());
            lhs = Expr::Binary(BinaryExpr {
                left: Box::new(lhs),
                op,
                right: Box::new(rhs),
                span,
            });
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let start = self.current_span();

        match self.peek_kind() {
            TokenKind::IntLiteral(v) => {
                let val = v;
                self.advance();
                Ok(Expr::Literal(Literal {
                    kind: LiteralKind::Int(val),
                    span: start,
                }))
            }
            TokenKind::FloatLiteral(v) => {
                let val = v;
                self.advance();
                Ok(Expr::Literal(Literal {
                    kind: LiteralKind::Float(val),
                    span: start,
                }))
            }
            TokenKind::StringLiteral(s) => {
                let val = s.clone();
                self.advance();
                Ok(Expr::Literal(Literal {
                    kind: LiteralKind::String(val),
                    span: start,
                }))
            }
            TokenKind::CharLiteral(c) => {
                let val = c;
                self.advance();
                Ok(Expr::Literal(Literal {
                    kind: LiteralKind::Char(val),
                    span: start,
                }))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Literal(Literal {
                    kind: LiteralKind::Bool(true),
                    span: start,
                }))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Literal(Literal {
                    kind: LiteralKind::Bool(false),
                    span: start,
                }))
            }
            TokenKind::None => {
                self.advance();
                Ok(Expr::Literal(Literal {
                    kind: LiteralKind::None,
                    span: start,
                }))
            }
            TokenKind::SelfKw => {
                self.advance();
                Ok(Expr::Identifier(Identifier {
                    name: "self".to_string(),
                    span: start,
                }))
            }
            TokenKind::Identifier(_) => {
                let name = self.expect_identifier()?;
                // Check for path expression: Name::Variant or Name::Variant(args)
                if self.check(&TokenKind::ColonColon) {
                    self.advance();
                    let variant = self.expect_identifier()?;
                    let path_name = format!("{}::{}", name, variant);
                    if self.check(&TokenKind::LeftParen) {
                        // Name::Variant(args)
                        self.advance();
                        let args = self.parse_call_args()?;
                        Ok(Expr::Call(CallExpr {
                            callee: Box::new(Expr::Identifier(Identifier {
                                name: path_name,
                                span: start.merge(&self.prev_span()),
                            })),
                            args,
                            generic_args: Vec::new(),
                            span: start.merge(&self.prev_span()),
                        }))
                    } else {
                        // Name::Variant (unit variant)
                        Ok(Expr::Identifier(Identifier {
                            name: path_name,
                            span: start.merge(&self.prev_span()),
                        }))
                    }
                } else if self.check(&TokenKind::LeftBrace)
                    && name.chars().next().is_some_and(|c| c.is_uppercase())
                {
                    self.advance();
                    let mut fields = Vec::new();
                    while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
                        let fname = self.expect_identifier()?;
                        self.expect(&TokenKind::Colon)?;
                        let fval = self.parse_expr()?;
                        fields.push((fname, fval));
                        if !self.check(&TokenKind::RightBrace) {
                            self.expect(&TokenKind::Comma)?;
                        }
                    }
                    self.expect(&TokenKind::RightBrace)?;
                    Ok(Expr::StructInit(StructInitExpr {
                        name,
                        fields,
                        span: start.merge(&self.prev_span()),
                    }))
                } else {
                    Ok(Expr::Identifier(Identifier {
                        name,
                        span: start.merge(&self.prev_span()),
                    }))
                }
            }
            TokenKind::Minus => {
                self.advance();
                let operand = self.parse_expr_bp(prefix_binding_power())?;
                Ok(Expr::Unary(UnaryExpr {
                    op: UnaryOp::Neg,
                    span: start.merge(operand.span()),
                    operand: Box::new(operand),
                }))
            }
            TokenKind::Bang => {
                self.advance();
                let operand = self.parse_expr_bp(prefix_binding_power())?;
                Ok(Expr::Unary(UnaryExpr {
                    op: UnaryOp::Not,
                    span: start.merge(operand.span()),
                    operand: Box::new(operand),
                }))
            }
            TokenKind::Tilde => {
                self.advance();
                let operand = self.parse_expr_bp(prefix_binding_power())?;
                Ok(Expr::Unary(UnaryExpr {
                    op: UnaryOp::BitNot,
                    span: start.merge(operand.span()),
                    operand: Box::new(operand),
                }))
            }
            TokenKind::Amp => {
                self.advance();
                let mutable = self.match_token(&TokenKind::Mut);
                let operand = self.parse_expr_bp(prefix_binding_power())?;
                Ok(Expr::Unary(UnaryExpr {
                    op: if mutable {
                        UnaryOp::RefMut
                    } else {
                        UnaryOp::Ref
                    },
                    span: start.merge(operand.span()),
                    operand: Box::new(operand),
                }))
            }
            TokenKind::Star => {
                self.advance();
                let operand = self.parse_expr_bp(prefix_binding_power())?;
                Ok(Expr::Unary(UnaryExpr {
                    op: UnaryOp::Deref,
                    span: start.merge(operand.span()),
                    operand: Box::new(operand),
                }))
            }
            TokenKind::LeftParen => {
                self.advance();
                let mut exprs = Vec::new();
                while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
                    exprs.push(self.parse_expr()?);
                    if !self.check(&TokenKind::RightParen) {
                        self.expect(&TokenKind::Comma)?;
                    }
                }
                self.expect(&TokenKind::RightParen)?;
                if exprs.len() == 1 {
                    Ok(exprs.into_iter().next().unwrap())
                } else {
                    Ok(Expr::Tuple(TupleExpr {
                        elements: exprs,
                        span: start.merge(&self.prev_span()),
                    }))
                }
            }
            TokenKind::LeftBracket => {
                self.advance();
                let mut elements = Vec::new();
                while !self.check(&TokenKind::RightBracket) && !self.is_at_end() {
                    elements.push(self.parse_expr()?);
                    if !self.check(&TokenKind::RightBracket) {
                        self.expect(&TokenKind::Comma)?;
                    }
                }
                self.expect(&TokenKind::RightBracket)?;
                Ok(Expr::Array(ArrayExpr {
                    elements,
                    span: start.merge(&self.prev_span()),
                }))
            }
            TokenKind::LeftBrace => {
                let block = self.parse_block()?;
                Ok(Expr::Block(block))
            }
            TokenKind::If => self.parse_if_expr(),
            TokenKind::Match => self.parse_match_expr(),
            TokenKind::Pipe => self.parse_lambda(),
            TokenKind::Await => {
                self.advance();
                let expr = self.parse_expr()?;
                Ok(Expr::Await(AwaitExpr {
                    span: start.merge(expr.span()),
                    expr: Box::new(expr),
                }))
            }
            _ => Err(self.error(&format!(
                "expected expression, found '{}'",
                self.peek_kind()
            ))),
        }
    }

    fn parse_if_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::If)?;
        let condition = self.parse_expr()?;
        let then_block = self.parse_block()?;

        let else_block = if self.check(&TokenKind::Else) {
            self.advance();
            if self.check(&TokenKind::If) {
                Some(Box::new(self.parse_if_expr()?))
            } else {
                let block = self.parse_block()?;
                Some(Box::new(Expr::Block(block)))
            }
        } else {
            None
        };

        Ok(Expr::If(IfExpr {
            condition: Box::new(condition),
            then_block,
            else_block,
            span: start.merge(&self.prev_span()),
        }))
    }

    fn parse_match_expr(&mut self) -> Result<Expr, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::Match)?;
        let subject = self.parse_expr()?;
        self.expect(&TokenKind::LeftBrace)?;

        let mut arms = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let arm_start = self.current_span();
            let pattern = self.parse_pattern()?;

            let guard = if self.check(&TokenKind::If) {
                self.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };

            self.expect(&TokenKind::FatArrow)?;
            let body = self.parse_expr()?;

            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span: arm_start.merge(&self.prev_span()),
            });

            self.match_token(&TokenKind::Comma);
        }

        self.expect(&TokenKind::RightBrace)?;

        Ok(Expr::Match(MatchExpr {
            subject: Box::new(subject),
            arms,
            span: start.merge(&self.prev_span()),
        }))
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        let start = self.current_span();

        match self.peek_kind() {
            TokenKind::IntLiteral(v) => {
                let val = v;
                self.advance();
                Ok(Pattern::Literal(Literal {
                    kind: LiteralKind::Int(val),
                    span: start,
                }))
            }
            TokenKind::StringLiteral(s) => {
                let val = s.clone();
                self.advance();
                Ok(Pattern::Literal(Literal {
                    kind: LiteralKind::String(val),
                    span: start,
                }))
            }
            TokenKind::True => {
                self.advance();
                Ok(Pattern::Literal(Literal {
                    kind: LiteralKind::Bool(true),
                    span: start,
                }))
            }
            TokenKind::False => {
                self.advance();
                Ok(Pattern::Literal(Literal {
                    kind: LiteralKind::Bool(false),
                    span: start,
                }))
            }
            TokenKind::Identifier(name) if name == "_" => {
                self.advance();
                Ok(Pattern::Wildcard(start))
            }
            TokenKind::Identifier(_) => {
                let name = self.expect_identifier()?;
                if self.check(&TokenKind::ColonColon) {
                    // Enum pattern: Name::Variant(fields)
                    self.advance();
                    let variant = self.expect_identifier()?;
                    let mut fields = Vec::new();
                    if self.check(&TokenKind::LeftParen) {
                        self.advance();
                        while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
                            fields.push(self.parse_pattern()?);
                            if !self.check(&TokenKind::RightParen) {
                                self.expect(&TokenKind::Comma)?;
                            }
                        }
                        self.expect(&TokenKind::RightParen)?;
                    }
                    Ok(Pattern::Enum {
                        name,
                        variant,
                        fields,
                        span: start.merge(&self.prev_span()),
                    })
                } else {
                    Ok(Pattern::Identifier(name, start.merge(&self.prev_span())))
                }
            }
            TokenKind::LeftParen => {
                self.advance();
                let mut patterns = Vec::new();
                while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
                    patterns.push(self.parse_pattern()?);
                    if !self.check(&TokenKind::RightParen) {
                        self.expect(&TokenKind::Comma)?;
                    }
                }
                self.expect(&TokenKind::RightParen)?;
                Ok(Pattern::Tuple(patterns, start.merge(&self.prev_span())))
            }
            _ => Err(self.error(&format!("expected pattern, found '{}'", self.peek_kind()))),
        }
    }

    fn parse_lambda(&mut self) -> Result<Expr, ParseError> {
        let start = self.current_span();
        self.expect(&TokenKind::Pipe)?;

        let mut params = Vec::new();
        while !self.check(&TokenKind::Pipe) && !self.is_at_end() {
            let p_start = self.current_span();
            let name = self.expect_identifier()?;
            let type_expr = if self.check(&TokenKind::Colon) {
                self.advance();
                self.parse_type_expr()?
            } else {
                TypeExpr::Named {
                    name: "_".into(),
                    generic_args: Vec::new(),
                    span: p_start.clone(),
                }
            };
            params.push(Param {
                name,
                type_expr,
                default: None,
                span: p_start.merge(&self.prev_span()),
            });
            if !self.check(&TokenKind::Pipe) {
                self.expect(&TokenKind::Comma)?;
            }
        }
        self.expect(&TokenKind::Pipe)?;

        let body = self.parse_expr()?;

        Ok(Expr::Lambda(LambdaExpr {
            params,
            body: Box::new(body),
            span: start.merge(&self.prev_span()),
        }))
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
            args.push(self.parse_expr()?);
            if !self.check(&TokenKind::RightParen) {
                self.expect(&TokenKind::Comma)?;
            }
        }
        self.expect(&TokenKind::RightParen)?;
        Ok(args)
    }

    // ========================================================================
    // Helper methods
    // ========================================================================

    fn peek_kind_to_binop(&self) -> Option<BinaryOp> {
        match self.peek_kind() {
            TokenKind::Plus => Some(BinaryOp::Add),
            TokenKind::Minus => Some(BinaryOp::Sub),
            TokenKind::Star => Some(BinaryOp::Mul),
            TokenKind::Slash => Some(BinaryOp::Div),
            TokenKind::Percent => Some(BinaryOp::Mod),
            TokenKind::EqEq => Some(BinaryOp::Eq),
            TokenKind::BangEq => Some(BinaryOp::Neq),
            TokenKind::Less => Some(BinaryOp::Lt),
            TokenKind::Greater => Some(BinaryOp::Gt),
            TokenKind::LessEq => Some(BinaryOp::Lte),
            TokenKind::GreaterEq => Some(BinaryOp::Gte),
            TokenKind::AmpAmp => Some(BinaryOp::And),
            TokenKind::PipePipe => Some(BinaryOp::Or),
            TokenKind::Amp => Some(BinaryOp::BitAnd),
            TokenKind::Pipe => Some(BinaryOp::BitOr),
            TokenKind::Caret => Some(BinaryOp::BitXor),
            TokenKind::Shl => Some(BinaryOp::Shl),
            TokenKind::Shr => Some(BinaryOp::Shr),
            TokenKind::PipeArrow => Some(BinaryOp::Pipe),
            _ => None,
        }
    }

    fn is_at_end(&self) -> bool {
        self.peek_kind() == TokenKind::Eof
    }

    fn peek_kind(&self) -> TokenKind {
        self.tokens
            .get(self.pos)
            .map(|t| t.kind.clone())
            .unwrap_or(TokenKind::Eof)
    }

    fn current_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span.clone())
            .unwrap_or_else(Span::dummy)
    }

    fn prev_span(&self) -> Span {
        if self.pos > 0 {
            self.tokens[self.pos - 1].span.clone()
        } else {
            Span::dummy()
        }
    }

    fn advance(&mut self) -> &Token {
        let token = &self.tokens[self.pos];
        self.pos += 1;
        token
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek_kind()) == std::mem::discriminant(kind)
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<&Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.error(&format!(
                "expected '{}', found '{}'",
                kind,
                self.peek_kind()
            )))
        }
    }

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        match self.peek_kind() {
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(name)
            }
            _ => Err(self.error(&format!(
                "expected identifier, found '{}'",
                self.peek_kind()
            ))),
        }
    }

    fn error(&self, message: &str) -> ParseError {
        ParseError::Error {
            message: message.to_string(),
            span: self.current_span(),
        }
    }

    fn synchronize(&mut self) {
        while !self.is_at_end() {
            match self.peek_kind() {
                TokenKind::Fn
                | TokenKind::Let
                | TokenKind::Struct
                | TokenKind::Enum
                | TokenKind::Trait
                | TokenKind::Impl
                | TokenKind::Import
                | TokenKind::Return
                | TokenKind::If
                | TokenKind::While
                | TokenKind::For
                | TokenKind::Match => return,
                TokenKind::RightBrace => {
                    self.advance();
                    return;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }
}

/// Binding power for binary operators (left, right)
fn infix_binding_power(op: BinaryOp) -> (u8, u8) {
    match op {
        BinaryOp::Pipe => (1, 2),
        BinaryOp::Or => (3, 4),
        BinaryOp::And => (5, 6),
        BinaryOp::BitOr => (7, 8),
        BinaryOp::BitXor => (9, 10),
        BinaryOp::BitAnd => (11, 12),
        BinaryOp::Eq | BinaryOp::Neq => (13, 14),
        BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Lte | BinaryOp::Gte => (15, 16),
        BinaryOp::Shl | BinaryOp::Shr => (17, 18),
        BinaryOp::Add | BinaryOp::Sub => (19, 20),
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => (21, 22),
    }
}

fn prefix_binding_power() -> u8 {
    23
}

/// Convenience function: parse source directly to AST
pub fn parse(source: &str, file: &str) -> Result<Program, Vec<ParseError>> {
    let mut parser = Parser::from_source(source, file).map_err(|e| vec![e])?;
    parser.parse_program()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hello() {
        let src = r#"
fn main {
    let x = 10
    let y = 20
    print(x + y)
}
"#;
        let program = parse(src, "<test>").unwrap();
        assert_eq!(program.modules.len(), 1);
        assert_eq!(program.modules[0].items.len(), 1);
    }

    #[test]
    fn test_parse_struct() {
        let src = r#"
struct Point {
    x: f64
    y: f64
}
"#;
        let program = parse(src, "<test>").unwrap();
        match &program.modules[0].items[0] {
            Item::Struct(s) => {
                assert_eq!(s.name, "Point");
                assert_eq!(s.fields.len(), 2);
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn test_parse_if_else() {
        let src = r#"
fn test_fn {
    if x > 0 {
        print(x)
    } else {
        print(0)
    }
}
"#;
        let program = parse(src, "<test>").unwrap();
        assert_eq!(program.modules[0].items.len(), 1);
    }
}
