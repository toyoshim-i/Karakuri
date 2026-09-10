//! `.kir` source to [`Proc`].
//!
//! Syntax only. Undefined names, type errors, and contract violations are not
//! this pass's business — it should accept anything shaped like the grammar and
//! leave meaning to the check pass, so that a single malformed expression does
//! not mask every later diagnostic.
//!
//! Two exceptions, both required by the calling convention rather than by the
//! grammar: the value of `kind`/`topology`/`blend` has to resolve to an AST
//! enum variant right here (there is nowhere else to put "unknown kind `L9`"),
//! and the `id` / signal-bus-name mistakes are common enough that a hint at
//! parse time saves a full round trip through type checking. See the module
//! docs on those two call sites below for more.
//!
//! Recovery happens at two granularities: a broken header declaration or
//! block is skipped up to the next recognized keyword (or `}`), and a broken
//! statement is skipped up to the next `;` or `}`. Everything else — a
//! missing token inside an otherwise-recognizable construct — is patched with
//! a placeholder so a single mistake doesn't swallow the rest of the file.

use crate::ast::{
    AmplifyDecl, Attr, BinOp, Blend, Block, BlockKind, CapacityDecl, Expr, Kind, Lit, Param, Proc,
    RetainsDecl, SlotTy, Stmt, Topology, Ty, UnOp, UsesDecl,
};
use crate::error::IrError;
use crate::error::IrResult;
use crate::lexer::{self, TokKind, Token};
use crate::span::Span;

/// Parse one `.kir` file.
///
/// Returns every syntax error found, not just the first: a generated procedure
/// with four mistakes should produce four diagnostics so a repair prompt can fix
/// them in one pass.
pub fn parse(src: &str) -> IrResult<Proc> {
    let (tokens, lex_errors) = lexer::lex(src);
    let mut parser = Parser {
        tokens,
        pos: 0,
        errors: lex_errors,
    };
    let proc = parser.parse_proc();
    if parser.errors.is_empty() {
        Ok(proc)
    } else {
        Err(parser.errors)
    }
}

/// Header/block keywords. Declaration recovery scans forward to the next one
/// of these (or `}`), so one bad declaration does not eat the rest of the file.
const DECL_KEYWORDS: [&str; 19] = [
    "kind", "topology", "capacity", "amplify", "retains", "uses", "param", "emit", "consumes",
    "blend", "spawn", "element", "deform", "mask", "camera", "field", "vertex", "fragment",
    "frame",
];

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    errors: Vec<IrError>,
}

impl Parser {
    // -- token stream primitives -------------------------------------------

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn peek_at(&self, ahead: usize) -> &Token {
        let idx = (self.pos + ahead).min(self.tokens.len() - 1);
        &self.tokens[idx]
    }

    fn at_end(&self) -> bool {
        matches!(self.peek().kind, TokKind::Eof)
    }

    /// Advance and return the consumed token. A no-op at `Eof`, so callers
    /// can always advance speculatively without checking first.
    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if !matches!(tok.kind, TokKind::Eof) {
            self.pos += 1;
        }
        tok
    }

    /// Span of the most recently consumed token. Used to close out a span
    /// after an `expect` that may or may not have consumed anything.
    fn prev_span(&self) -> Span {
        if self.pos == 0 {
            Span::EMPTY
        } else {
            self.tokens[self.pos - 1].span
        }
    }

    fn at_ident(&self, text: &str) -> bool {
        matches!(&self.peek().kind, TokKind::Ident(s) if s == text)
    }

    fn is_assignment_start(&self) -> bool {
        matches!(
            self.peek_at(1).kind,
            TokKind::Eq | TokKind::PlusEq | TokKind::MinusEq | TokKind::StarEq | TokKind::SlashEq
        )
    }

    /// Tokens a best-effort literal/expression recovery should never eat,
    /// because they close a structure some enclosing parser still needs.
    fn is_stopper(&self, kind: &TokKind) -> bool {
        matches!(
            kind,
            TokKind::RBrace
                | TokKind::RBracket
                | TokKind::RParen
                | TokKind::Comma
                | TokKind::Semicolon
                | TokKind::Eof
                | TokKind::LBrace
        )
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.errors.push(IrError::parse(span, message));
    }

    fn error_with_hint(&mut self, span: Span, message: impl Into<String>, hint: impl Into<String>) {
        self.errors
            .push(IrError::parse(span, message).with_hint(hint));
    }

    /// Consume `want` if present; otherwise report it missing and leave the
    /// stream untouched, so the caller can decide what to try next.
    fn expect(&mut self, want: TokKind, desc: &str) -> bool {
        if self.peek().kind == want {
            self.advance();
            true
        } else {
            let sp = self.peek().span;
            let found = self.peek().kind.describe();
            self.error(sp, format!("expected `{desc}`, found {found}"));
            false
        }
    }

    fn expect_ident(&mut self, what: &str) -> Option<(String, Span)> {
        if let TokKind::Ident(name) = self.peek().kind.clone() {
            let span = self.advance().span;
            Some((name, span))
        } else {
            let sp = self.peek().span;
            let found = self.peek().kind.describe();
            self.error(sp, format!("expected {what}, found {found}"));
            None
        }
    }

    fn synchronize_decl(&mut self) {
        loop {
            match &self.peek().kind {
                TokKind::RBrace | TokKind::Eof => return,
                TokKind::Ident(s) if DECL_KEYWORDS.contains(&s.as_str()) => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    fn synchronize_stmt(&mut self) {
        loop {
            match self.peek().kind {
                TokKind::Semicolon => {
                    self.advance();
                    return;
                }
                TokKind::RBrace | TokKind::Eof => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    // -- literals used outside full expressions ------------------------------

    fn parse_f32_literal(&mut self) -> f32 {
        let neg = if self.peek().kind == TokKind::Minus {
            self.advance();
            true
        } else {
            false
        };
        let v: f64 = match self.peek().kind.clone() {
            TokKind::Int(v) => {
                self.advance();
                v as f64
            }
            TokKind::Uint(v) => {
                self.advance();
                v as f64
            }
            TokKind::Float(v) => {
                self.advance();
                v
            }
            _ => {
                let sp = self.peek().span;
                let found = self.peek().kind.describe();
                self.error(sp, format!("expected a number, found {found}"));
                if !self.is_stopper(&self.peek().kind) {
                    self.advance();
                }
                0.0
            }
        };
        (if neg { -v } else { v }) as f32
    }

    fn parse_u32_literal(&mut self) -> u32 {
        let neg_span = if self.peek().kind == TokKind::Minus {
            Some(self.advance().span)
        } else {
            None
        };
        let (val, span): (i64, Span) = match self.peek().kind.clone() {
            TokKind::Int(v) => (v, self.advance().span),
            TokKind::Uint(v) => (v as i64, self.advance().span),
            _ => {
                let sp = self.peek().span;
                let found = self.peek().kind.describe();
                self.error(sp, format!("expected an integer literal, found {found}"));
                if !self.is_stopper(&self.peek().kind) {
                    self.advance();
                }
                return 0;
            }
        };
        if let Some(neg_span) = neg_span {
            self.error(neg_span.join(span), "this value must not be negative");
            return 0;
        }
        if val > u32::MAX as i64 {
            self.error(span, "integer literal out of range for a 32-bit value");
            return 0;
        }
        val as u32
    }

    /// `for` bounds are stored as plain `i32` in the AST (see `Stmt::For`),
    /// so anything other than a literal integer — a param, an ambient, an
    /// arithmetic expression — cannot be represented and is a parse error
    /// here rather than a later "not a constant" check.
    fn parse_int_bound(&mut self) -> i32 {
        let neg = if self.peek().kind == TokKind::Minus {
            self.advance();
            true
        } else {
            false
        };
        match self.peek().kind.clone() {
            TokKind::Int(v) => {
                self.advance();
                let v = if neg { -v } else { v };
                v as i32
            }
            TokKind::Uint(v) => {
                self.advance();
                let v = if neg { -(v as i64) } else { v as i64 };
                v as i32
            }
            _ => {
                let sp = self.peek().span;
                let found = self.peek().kind.describe();
                self.error_with_hint(
                    sp,
                    format!("`for` bounds must be constant integers, found {found}"),
                    "loop bounds cannot reference a param, an ambient, or any other \
                     expression — cost estimation needs them fixed at parse time",
                );
                if !self.is_stopper(&self.peek().kind) {
                    self.advance();
                }
                0
            }
        }
    }

    // -- top level -------------------------------------------------------------

    fn parse_proc(&mut self) -> Proc {
        let start_span = self.peek().span;

        if !self.at_ident("proc") {
            let sp = self.peek().span;
            let found = self.peek().kind.describe();
            self.error(sp, format!("expected `proc`, found {found}"));
            return empty_proc(sp);
        }
        self.advance();

        let name = match self.expect_ident("a procedure name") {
            Some((n, _)) => n,
            None => String::new(),
        };

        if !self.expect(TokKind::LBrace, "{") {
            let sp = start_span.join(self.prev_span());
            return empty_proc(sp);
        }

        let mut kind = None;
        let mut topology = None;
        let mut capacity = None;
        let mut amplify = None;
        let mut retains = None;
        let mut uses = Vec::new();
        let mut blend = None;
        let mut params = Vec::new();
        let mut emit = Vec::new();
        let mut consumes = Vec::new();
        let mut blocks = Vec::new();

        loop {
            if self.at_end() {
                let sp = self.peek().span;
                self.error(sp, "unterminated `proc`: expected `}` before end of file");
                break;
            }
            if self.peek().kind == TokKind::RBrace {
                break;
            }

            let before = self.pos;
            if let TokKind::Ident(word) = self.peek().kind.clone() {
                match word.as_str() {
                    "kind" => kind = Some(self.parse_kind()),
                    "topology" => {
                        if let Some(t) = self.parse_topology() {
                            topology = Some(t);
                        }
                    }
                    "capacity" => capacity = Some(self.parse_capacity()),
                    "amplify" => amplify = Some(self.parse_amplify()),
                    // **A bare word and nothing after it**, so there is no
                    // operand to parse and no way for one to be wrong. A second
                    // `retains` overwrites the first with the same value, which
                    // is what a repeated declaration of a flag is.
                    "retains" => {
                        retains = Some(RetainsDecl {
                            span: self.advance().span,
                        })
                    }
                    "uses" => {
                        if let Some(u) = self.parse_uses() {
                            uses.push(u);
                        }
                    }
                    "param" => {
                        if let Some(p) = self.parse_param() {
                            params.push(p);
                        }
                    }
                    "emit" => {
                        self.advance();
                        emit.extend(self.parse_attr_list());
                    }
                    "consumes" => {
                        self.advance();
                        consumes.extend(self.parse_attr_list());
                    }
                    "blend" => {
                        if let Some(b) = self.parse_blend() {
                            blend = Some(b);
                        }
                    }
                    "spawn" => blocks.push(self.parse_block(BlockKind::Spawn)),
                    "element" => blocks.push(self.parse_block(BlockKind::Element)),
                    "deform" => blocks.push(self.parse_block(BlockKind::Deform)),
                    "mask" => blocks.push(self.parse_block(BlockKind::Mask)),
                    "camera" => blocks.push(self.parse_block(BlockKind::Camera)),
                    "field" => blocks.push(self.parse_block(BlockKind::Field)),
                    "vertex" => blocks.push(self.parse_block(BlockKind::Vertex)),
                    "fragment" => blocks.push(self.parse_block(BlockKind::Fragment)),
                    "frame" => blocks.push(self.parse_block(BlockKind::Frame)),
                    _ => {
                        let sp = self.peek().span;
                        self.error(
                            sp,
                            format!("expected a header declaration or block, found `{word}`"),
                        );
                        self.synchronize_decl();
                    }
                }
            } else {
                let sp = self.peek().span;
                let found = self.peek().kind.describe();
                self.error(
                    sp,
                    format!("expected a header declaration or block, found {found}"),
                );
                self.synchronize_decl();
            }

            if self.pos == before {
                // Nothing above made progress (e.g. a stray token that is
                // also a sync point) — force one token so the loop cannot spin.
                self.advance();
            }
        }

        let close_span = if self.peek().kind == TokKind::RBrace {
            self.advance().span
        } else {
            self.prev_span()
        };
        let proc_span = start_span.join(close_span);

        if !self.at_end() {
            let sp = self.peek().span;
            self.error(
                sp,
                "unexpected content after the closing `}` of `proc` — a `.kir` file \
                 contains exactly one `proc`",
            );
        }

        let kind = match kind {
            Some(k) => k,
            None => {
                self.error(proc_span, "missing `kind` declaration");
                Kind::L1
            }
        };

        Proc {
            name,
            kind,
            topology,
            capacity,
            amplify,
            retains,
            uses,
            blend,
            params,
            emit,
            consumes,
            blocks,
            span: proc_span,
        }
    }

    // -- header declarations -----------------------------------------------

    /// Always returns a `Kind`, even on failure: an invalid or missing value
    /// is already reported here, and the caller treating `kind` as present
    /// would otherwise cascade into a redundant "missing `kind`" diagnostic
    /// at the end of `parse_proc`.
    fn parse_kind(&mut self) -> Kind {
        self.advance(); // "kind"
        match self.expect_ident("`L1`, `L2`, `L3`, `L4`, `L5` or `Field`") {
            Some((name, span)) => match name.as_str() {
                "L1" => Kind::L1,
                "L2" => Kind::L2,
                "L3" => Kind::L3,
                "L4" => Kind::L4,
                "L5" => Kind::L5,
                "Field" => Kind::Field,
                _ => {
                    self.error_with_hint(
                        span,
                        format!("unknown kind `{name}`"),
                        "this compiler builds `L1` (geometry), `L2` (geometry modulation), \
                         `L3` (the camera), `L4` (rendering), `L5` (a frame effect over the \
                         picture handed to it) and `Field` (a signed distance at a point, \
                         spliced into whoever evaluates it)",
                    );
                    Kind::L1
                }
            },
            None => Kind::L1,
        }
    }

    fn parse_topology(&mut self) -> Option<Topology> {
        self.advance(); // "topology"
        let (name, span) = self.expect_ident("`points` or `lines`")?;
        match name.as_str() {
            "points" => Some(Topology::Points),
            "lines" => Some(Topology::Lines),
            // Parsed so that the *contract* check can say why it is wrong here
            // rather than the parser saying the word does not exist. It does
            // exist; it is not something geometry can be.
            "fullscreen" => Some(Topology::Fullscreen),
            _ => {
                self.error_with_hint(
                    span,
                    format!("unknown topology `{name}`"),
                    "the topologies are `points` and `lines` — `lines` makes each element \
                     one segment, whose far end the paired L4 writes to `clip_b`",
                );
                None
            }
        }
    }

    fn parse_blend(&mut self) -> Option<Blend> {
        self.advance(); // "blend"
        let (name, span) = self.expect_ident("`additive` or `weighted`")?;
        match name.as_str() {
            "additive" => Some(Blend::Additive),
            "weighted" => Some(Blend::Weighted),
            _ => {
                self.error_with_hint(
                    span,
                    format!("unknown blend mode `{name}`"),
                    "the blend modes are `additive`, which sums colour and occludes nothing, \
                     and `weighted`, which is order-independent transparency and reads \
                     `color`'s alpha as opacity",
                );
                None
            }
        }
    }

    /// `amplify <factor>` — a bare literal, unlike `capacity`'s range.
    ///
    /// **There is no range because there is nothing to override it with.** A
    /// Set turns `capacity` because how much material to make is the operator's
    /// question; how many copies a kaleidoscope has is the procedure's own, and
    /// making it adjustable would resize a buffer from a fader.
    fn parse_amplify(&mut self) -> AmplifyDecl {
        let start = self.advance().span; // "amplify"
        let factor = self.parse_u32_literal();
        AmplifyDecl {
            factor,
            span: start.join(self.prev_span()),
        }
    }

    /// `uses <name> : Geometry` / `uses <name> : Field` / `uses <name> :
    /// Camera` — one named input this node takes.
    ///
    /// **The name is the procedure's and the binding is the Set's.** So this
    /// declaration says what the file needs and never which node supplies it:
    /// a `.kir` that named a node would be a procedure coupled to one Set, and
    /// it would stop being a library part. See [`UsesDecl`].
    ///
    /// The type is carried rather than checked and dropped, because every rule
    /// downstream is about *which* one — an L3 refuses a geometry slot because
    /// an L3 makes no geometry, and accepts a Field slot because evaluating a
    /// field is not making geometry. See [`SlotTy`].
    fn parse_uses(&mut self) -> Option<UsesDecl> {
        let start = self.advance().span; // "uses"
        let (name, name_span) = self.expect_ident("a name for the input this procedure takes")?;
        self.expect(TokKind::Colon, ":");
        // **A refused type recovers as `Geometry`**, and a missing one too.
        // Both have already reported, so neither reaches the check pass;
        // carrying on with one of the types there are lets the rest of the
        // header be parsed and its own mistakes reported in the same run.
        let ty = match self.expect_ident("`Geometry`, `Field`, `Camera`, `Source` or `Texture`") {
            Some((spelling, ty_span)) => SlotTy::from_name(&spelling).unwrap_or_else(|| {
                self.error_with_hint(
                    ty_span,
                    format!("unknown input type `{spelling}`"),
                    "a `uses` slot is `Geometry` — the elements of an L1, read beside the ones \
                     this node runs over — `Field`, a `kind Field` procedure this one \
                     evaluates, `Camera`, a viewpoint this one draws from, `Source`, the \
                     identity of one geometry for `source` to be compared against, or \
                     `Texture`, a picture an L5 folds in",
                );
                SlotTy::Geometry
            }),
            None => SlotTy::Geometry,
        };
        Some(UsesDecl {
            name,
            name_span,
            ty,
            span: start.join(self.prev_span()),
        })
    }

    fn parse_capacity(&mut self) -> CapacityDecl {
        let start = self.advance().span; // "capacity"
        self.expect(TokKind::LBracket, "[");
        let min = self.parse_u32_literal();
        self.expect(TokKind::Comma, ",");
        let max = self.parse_u32_literal();
        self.expect(TokKind::RBracket, "]");
        self.expect(TokKind::Eq, "=");
        let default = self.parse_u32_literal();
        let end = self.prev_span();
        CapacityDecl {
            min,
            max,
            default,
            span: start.join(end),
        }
    }

    /// `param <name> : <type> [<min>, <max>] = <default>`.
    ///
    /// The range is mandatory per spec, but a missing `[` is a mistake real
    /// generations make, so this recovers instead of losing the rest of the
    /// declaration: if `[` is absent it looks straight for `=` and parses
    /// the default with a placeholder `0.0..0.0` range, reporting exactly one
    /// error rather than cascading into the default-value expression.
    fn parse_param(&mut self) -> Option<Param> {
        let start = self.advance().span; // "param"
        let (name, _) = self.expect_ident("a parameter name")?;
        self.expect(TokKind::Colon, ":");
        let ty = match self.expect_ident("a type") {
            Some((type_name, type_span)) => match Ty::from_name(&type_name) {
                Some(t) => t,
                None => {
                    self.error(type_span, format!("unknown type `{type_name}`"));
                    Ty::Float
                }
            },
            None => Ty::Float,
        };

        if self.peek().kind != TokKind::LBracket {
            let sp = self.peek().span;
            self.error_with_hint(
                sp,
                format!("param `{name}` has no range"),
                "the range is mandatory — it is the fader range, the agent's search \
                 range, and the normalization basis for signal binding: \
                 `param name : type [min, max] = default`",
            );
            if self.peek().kind == TokKind::Eq {
                self.advance();
            }
            let default = self.parse_expr();
            let end = default.span();
            return Some(Param {
                name,
                ty,
                min: 0.0,
                max: 0.0,
                default,
                span: start.join(end),
            });
        }
        self.advance(); // "["

        let min = self.parse_f32_literal();
        self.expect(TokKind::Comma, ",");
        let max = self.parse_f32_literal();
        self.expect(TokKind::RBracket, "]");
        self.expect(TokKind::Eq, "=");
        let default = self.parse_expr();
        let end = default.span();
        Some(Param {
            name,
            ty,
            min,
            max,
            default,
            span: start.join(end),
        })
    }

    fn parse_attr_list(&mut self) -> Vec<(Attr, Span)> {
        let mut out = Vec::new();
        loop {
            match self.peek().kind.clone() {
                TokKind::Ident(name) => {
                    let span = self.advance().span;
                    if name == "seed" {
                        self.error_with_hint(
                            span,
                            "`seed` must not be declared here",
                            "`seed` is implicit and always readable — carried automatically, \
                             never named in `emit` or `consumes`",
                        );
                    } else if name == "id" {
                        self.error_with_hint(
                            span,
                            "there is no `id`",
                            "element identity is `seed` — a slot index stops being an \
                             identity once compaction moves elements",
                        );
                    } else if let Some(attr) = Attr::from_name(&name) {
                        out.push((attr, span));
                    } else {
                        self.error(span, format!("unknown attribute `{name}`"));
                    }
                }
                _ => {
                    let sp = self.peek().span;
                    let found = self.peek().kind.describe();
                    self.error(sp, format!("expected an attribute name, found {found}"));
                    break;
                }
            }
            if self.peek().kind == TokKind::Comma {
                self.advance();
            } else {
                break;
            }
        }
        out
    }

    // -- blocks --------------------------------------------------------------

    fn parse_block(&mut self, kind: BlockKind) -> Block {
        let start = self.advance().span; // block-kind ident
        let (stmts, body_span) = self.parse_brace_stmts();
        Block {
            kind,
            stmts,
            span: start.join(body_span),
        }
    }

    /// Parses `{ <stmt>* }`. On a missing `}` before end of file this is
    /// where "unterminated block" is reported.
    fn parse_brace_stmts(&mut self) -> (Vec<Stmt>, Span) {
        let open_span = self.peek().span;
        if !self.expect(TokKind::LBrace, "{") {
            return (Vec::new(), open_span);
        }

        let mut stmts = Vec::new();
        loop {
            if self.at_end() {
                let sp = self.peek().span;
                self.error(sp, "unterminated block: expected `}` before end of file");
                break;
            }
            if self.peek().kind == TokKind::RBrace {
                break;
            }

            let before = self.pos;
            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            } else {
                self.synchronize_stmt();
            }
            if self.pos == before {
                self.advance();
            }
        }

        let close_span = if self.peek().kind == TokKind::RBrace {
            self.advance().span
        } else {
            self.prev_span()
        };
        (stmts, open_span.join(close_span))
    }

    // -- statements ------------------------------------------------------------

    fn parse_stmt(&mut self) -> Option<Stmt> {
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

    fn parse_expr(&mut self) -> Expr {
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

    /// `id` is a reserved word rather than an undefined name, so rejecting it
    /// here is a lexical matter and a repair prompt gets the hint on the first
    /// pass. Signal-bus names are not reserved and cannot be judged here: a
    /// procedure may legally declare `param energy`, and whether a bare
    /// `energy` resolves is a question about declarations, which is name
    /// resolution's job.
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

fn empty_proc(span: Span) -> Proc {
    Proc {
        name: String::new(),
        kind: Kind::L1,
        topology: None,
        capacity: None,
        amplify: None,
        retains: None,
        uses: Vec::new(),
        blend: None,
        params: Vec::new(),
        emit: Vec::new(),
        consumes: Vec::new(),
        blocks: Vec::new(),
        span,
    }
}
