use super::Parser;
use crate::ast::*;
use crate::lexer::TokKind;

impl Parser {
    pub(super) fn parse_stmt(&mut self) -> Option<Stmt> {
        if let TokKind::Ident(word) = self.peek().kind.clone() {
            match word.as_str() {
                "let" => return self.parse_let(),
                "var" => return self.parse_var(),
                "if" => return Some(self.parse_if()),
                "for" => return self.parse_for(),
                "kill" => return self.parse_kill(),
                _ => {
                    if self.is_assignment_start() {
                        return self.parse_assign();
                    }
                    let sp = self.peek().span;
                    self.error(sp, format!("expected a statement, found `{word}`"));
                    return None;
                }
            }
        }
        let sp = self.peek().span;
        let found = self.peek().kind.describe();
        self.error(sp, format!("expected a statement, found {found}"));
        None
    }

    fn parse_let(&mut self) -> Option<Stmt> {
        let start = self.advance().span; // "let"
        let (name, _) = self.expect_ident("a name after `let`")?;
        self.expect(TokKind::Eq, "=");
        let value = self.parse_expr();
        let end = if self.expect(TokKind::Semicolon, ";") {
            self.prev_span()
        } else {
            value.span()
        };
        Some(Stmt::Let {
            name,
            value,
            span: start.join(end),
        })
    }

    fn parse_var(&mut self) -> Option<Stmt> {
        let start = self.advance().span; // "var"
        let (name, _) = self.expect_ident("a name after `var`")?;
        self.expect(TokKind::Eq, "=");
        let value = self.parse_expr();
        let end = if self.expect(TokKind::Semicolon, ";") {
            self.prev_span()
        } else {
            value.span()
        };
        Some(Stmt::Var {
            name,
            value,
            span: start.join(end),
        })
    }

    fn parse_assign(&mut self) -> Option<Stmt> {
        let (target, target_span) = self.expect_ident("an assignment target")?;
        let op = match self.peek().kind.clone() {
            TokKind::Eq => {
                self.advance();
                None
            }
            TokKind::PlusEq => {
                self.advance();
                Some(BinOp::Add)
            }
            TokKind::MinusEq => {
                self.advance();
                Some(BinOp::Sub)
            }
            TokKind::StarEq => {
                self.advance();
                Some(BinOp::Mul)
            }
            TokKind::SlashEq => {
                self.advance();
                Some(BinOp::Div)
            }
            _ => {
                let sp = self.peek().span;
                self.error(sp, "expected `=`, `+=`, `-=`, `*=`, or `/=`");
                return None;
            }
        };
        let value = self.parse_expr();
        let end = if self.expect(TokKind::Semicolon, ";") {
            self.prev_span()
        } else {
            value.span()
        };
        Some(Stmt::Assign {
            target,
            op,
            value,
            span: target_span.join(end),
        })
    }

    fn parse_if(&mut self) -> Stmt {
        let start = self.advance().span; // "if"
        let cond = self.parse_expr();
        let (then, mut end) = self.parse_brace_stmts();
        let mut els = Vec::new();

        if self.at_ident("else") {
            self.advance();
            if self.at_ident("if") {
                let nested = self.parse_if();
                end = nested.span();
                els = vec![nested];
            } else {
                let (else_stmts, else_span) = self.parse_brace_stmts();
                end = else_span;
                els = else_stmts;
            }
        }

        Stmt::If {
            cond,
            then,
            els,
            span: start.join(end),
        }
    }

    fn parse_for(&mut self) -> Option<Stmt> {
        let start = self.advance().span; // "for"
        let (var, _) = self.expect_ident("a loop variable name")?;

        if self.at_ident("in") {
            self.advance();
        } else {
            let sp = self.peek().span;
            let found = self.peek().kind.describe();
            self.error(sp, format!("expected `in`, found {found}"));
        }

        let range_start = self.parse_int_bound();
        self.expect(TokKind::DotDot, "..");
        let range_end = self.parse_int_bound();

        let (body, body_span) = self.parse_brace_stmts();
        Some(Stmt::For {
            var,
            start: range_start,
            end: range_end,
            body,
            span: start.join(body_span),
        })
    }

    fn parse_kill(&mut self) -> Option<Stmt> {
        let start = self.advance().span; // "kill"
        self.expect(TokKind::LParen, "(");
        self.expect(TokKind::RParen, ")");
        self.expect(TokKind::Semicolon, ";");
        let end = self.prev_span();
        Some(Stmt::Kill {
            span: start.join(end),
        })
    }

    // -- expressions -------------------------------------------------------

    pub(super) fn parse_expr(&mut self) -> Expr {
        self.parse_binary(1)
    }

    fn peek_binop(&self) -> Option<BinOp> {
        Some(match self.peek().kind {
            TokKind::Plus => BinOp::Add,
            TokKind::Minus => BinOp::Sub,
            TokKind::Star => BinOp::Mul,
            TokKind::Slash => BinOp::Div,
            TokKind::Percent => BinOp::Rem,
            TokKind::Lt => BinOp::Lt,
            TokKind::Le => BinOp::Le,
            TokKind::Gt => BinOp::Gt,
            TokKind::Ge => BinOp::Ge,
            TokKind::EqEq => BinOp::Eq,
            TokKind::Ne => BinOp::Ne,
            TokKind::AmpAmp => BinOp::And,
            TokKind::PipePipe => BinOp::Or,
            _ => return None,
        })
    }

    fn parse_binary(&mut self, min_prec: u8) -> Expr {
        let mut lhs = self.parse_unary();
        while let Some(op) = self.peek_binop() {
            if op.precedence() < min_prec {
                break;
            }
            self.advance();
            let rhs = self.parse_binary(op.precedence() + 1);
            let span = lhs.span().join(rhs.span());
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        lhs
    }

    fn parse_unary(&mut self) -> Expr {
        match self.peek().kind {
            TokKind::Minus => {
                let start = self.advance().span;
                let value = self.parse_unary();
                let span = start.join(value.span());
                Expr::Unary {
                    op: UnOp::Neg,
                    value: Box::new(value),
                    span,
                }
            }
            TokKind::Bang => {
                let start = self.advance().span;
                let value = self.parse_unary();
                let span = start.join(value.span());
                Expr::Unary {
                    op: UnOp::Not,
                    value: Box::new(value),
                    span,
                }
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        loop {
            if self.peek().kind != TokKind::Dot {
                break;
            }
            self.advance();
            match self.peek().kind.clone() {
                TokKind::Ident(components) => {
                    let comp_span = self.advance().span;
                    let span = expr.span().join(comp_span);
                    expr = Expr::Swizzle {
                        value: Box::new(expr),
                        components,
                        span,
                    };
                }
                _ => {
                    let sp = self.peek().span;
                    let found = self.peek().kind.describe();
                    self.error(
                        sp,
                        format!("expected swizzle components after `.`, found {found}"),
                    );
                    break;
                }
            }
        }
        expr
    }

    /// Parses a primary expression (parenthesized expression, literal, identifier, or unary op).
    fn parse_primary(&mut self) -> Expr {
        match self.peek().kind.clone() {
            TokKind::LParen => {
                self.advance();
                let inner = self.parse_expr();
                self.expect(TokKind::RParen, ")");
                inner
            }
            TokKind::Ident(name) => {
                let span = self.peek().span;
                if name == "true" {
                    self.advance();
                    return Expr::Lit {
                        value: Lit::Bool(true),
                        span,
                    };
                }
                if name == "false" {
                    self.advance();
                    return Expr::Lit {
                        value: Lit::Bool(false),
                        span,
                    };
                }
                self.advance();
                if self.peek().kind == TokKind::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek().kind != TokKind::RParen {
                        loop {
                            args.push(self.parse_expr());
                            if self.peek().kind == TokKind::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    let close = if self.expect(TokKind::RParen, ")") {
                        self.prev_span()
                    } else {
                        self.peek().span
                    };
                    Expr::Call {
                        name,
                        args,
                        span: span.join(close),
                    }
                } else {
                    if name == "id" {
                        self.error_with_hint(
                            span,
                            "there is no `id`",
                            "element identity is `seed` — a slot index stops being an \
                             identity once compaction moves elements",
                        );
                    }
                    Expr::Ident { name, span }
                }
            }
            TokKind::Int(v) => {
                let span = self.advance().span;
                Expr::Lit {
                    value: Lit::Int(v as i32),
                    span,
                }
            }
            TokKind::Uint(v) => {
                let span = self.advance().span;
                Expr::Lit {
                    value: Lit::Uint(v as u32),
                    span,
                }
            }
            TokKind::Float(v) => {
                let span = self.advance().span;
                Expr::Lit {
                    value: Lit::Float(v as f32),
                    span,
                }
            }
            _ => {
                let sp = self.peek().span;
                let found = self.peek().kind.describe();
                self.error(sp, format!("expected an expression, found {found}"));
                if !self.is_stopper(&self.peek().kind) {
                    self.advance();
                }
                Expr::Lit {
                    value: Lit::Int(0),
                    span: sp,
                }
            }
        }
    }
}
