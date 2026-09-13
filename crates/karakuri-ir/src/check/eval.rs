//! Expression and statement type-checking and semantic evaluation.

use super::*;
use crate::ast::{
    Ambient, Attr, BinOp, Expr, Kind, Lit, Output, Stmt, Ty, UnOp, TEXTURE_HELD, TEXTURE_SRC,
};
use crate::builtin::{Builtin, Domain, Shape};
use crate::error::Stage;
use crate::span::Span;
use crate::typed::{TExpr, TExprKind, TStmt, Target, TexRef};

/// The spelling [`Output::PointRate`] had before its unit stopped being pixels.
///
/// Kept as a name this pass still recognises so that a `.kir` written against
/// the old language is refused with the new spelling rather than with "never
/// declared". It is not a reserved word: nothing stops an author declaring a
/// local or a param called `point_size`, and if one does the declaration wins —
/// this only catches the name when nothing else claims it.
const OLD_POINT_SIZE: &str = "point_size";

/// What to do about it, in one sentence.
///
/// The division is deliberately not given a number. `point_rate` is a fraction
/// of the render target's height, and which height a file's old pixel values
/// were authored against is a fact about that file rather than about the
/// language. Naming one here would make it an anchor.
const POINT_RATE_HINT: &str = concat!(
    "the output is now `point_rate`, a fraction of the render target's height ",
    "rather than a count of pixels — rename it and divide the old pixel value ",
    "by the height it was authored against"
);

impl<'a> Checker<'a> {
    // -- statements ---------------------------------------------------------

    pub(crate) fn check_stmts(&mut self, stmts: &[Stmt]) -> Vec<TStmt> {
        stmts.iter().filter_map(|s| self.check_stmt(s)).collect()
    }

    pub(crate) fn check_stmt(&mut self, s: &Stmt) -> Option<TStmt> {
        match s {
            Stmt::Let { name, value, span } => self.check_let(name, value, *span, false),
            Stmt::Var { name, value, span } => self.check_let(name, value, *span, true),
            Stmt::Assign {
                target,
                op,
                value,
                span,
            } => self.check_assign(target, *op, value, *span),
            Stmt::If {
                cond,
                then,
                els,
                span,
            } => self.check_if(cond, then, els, *span),
            Stmt::For {
                var,
                start,
                end,
                body,
                span,
            } => self.check_for(var, *start, *end, body, *span),
            Stmt::Kill { span } => self.check_kill(*span),
        }
    }

    fn check_let(&mut self, name: &str, value: &Expr, span: Span, mutable: bool) -> Option<TStmt> {
        let value_t = self.check_expr(value);
        self.check_declarable_name(name, span);
        let value_t = value_t?;
        self.scope.declare(
            name.to_string(),
            LocalInfo {
                ty: value_t.ty,
                mutable,
            },
        );
        if mutable {
            Some(TStmt::Var {
                name: name.to_string(),
                value: value_t,
                span,
            })
        } else {
            Some(TStmt::Let {
                name: name.to_string(),
                value: value_t,
                span,
            })
        }
    }

    /// `let`, `var`, and the loop variable may not shadow `id`, an attribute, an
    /// ambient, a stage output, a param, or another local — see the module docs on
    /// why this is stricter than the task checklist's paraphrase.
    fn check_declarable_name(&mut self, name: &str, span: Span) {
        if name == "id" {
            self.err_hint(
                Stage::Contract,
                span,
                "`id` is reserved and cannot be used as a name",
                "there is no `id` — element identity is `seed`",
            );
            return;
        }
        if Attr::from_name(name).is_some() {
            self.err(
                Stage::Contract,
                span,
                format!("`{name}` shadows an attribute name"),
            );
            return;
        }
        if self.uses == Some(name) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is the geometry this procedure uses"),
                "a slot and a local share one scope — rename one of them",
            );
            return;
        }
        if self.fields.contains(&name) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is a field this procedure uses"),
                "a slot and a local share one scope — rename one of them",
            );
            return;
        }
        if self.camera == Some(name) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is the camera this procedure draws from"),
                "a slot and a local share one scope — rename one of them",
            );
            return;
        }
        if self.textures.contains(&name) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is a picture this procedure folds in"),
                "a slot and a local share one scope — rename one of them",
            );
            return;
        }
        // **The two reserved names of a `frame` block**, on the terms `color`
        // is reserved in a `fragment` one: a local called `src` would make one
        // spelling mean two things — the incoming frame in one line and a
        // binding in the next — and `held` the same under `retains`. Refused
        // only where they name something, so an L4 local called `src` stays
        // legal.
        if self.block == Some(BlockKind::Frame)
            && (name == TEXTURE_SRC || (name == TEXTURE_HELD && self.retains))
        {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is a texture this procedure is handed"),
                format!(
                    "`{name}` names a picture in a `frame` block and cannot also name a local \
                     — rename the local. Fetch from the texture with `texel({name})` or \
                     `tap({name}, uv)`"
                ),
            );
            return;
        }
        if Ambient::from_name(name).is_some() {
            self.err(
                Stage::Contract,
                span,
                format!("`{name}` shadows an ambient value"),
            );
            return;
        }
        if shadows_output(name, self.kind) {
            self.err(
                Stage::Contract,
                span,
                format!("`{name}` shadows a stage output name"),
            );
            return;
        }
        if self.params.contains_key(name) {
            self.err(Stage::Contract, span, format!("`{name}` shadows a param"));
            return;
        }
        if self.scope.lookup(name).is_some() {
            self.err(
                Stage::Contract,
                span,
                format!("`{name}` shadows another local"),
            );
        }
    }

    fn resolve_target(&mut self, name: &str, span: Span) -> TargetRes {
        if let Some(info) = self.scope.lookup(name) {
            return TargetRes::Local(name.to_string(), info.ty, info.mutable);
        }
        if self.params.contains_key(name) {
            self.err(
                Stage::Type,
                span,
                format!("cannot assign to param `{name}`; params are read-only"),
            );
            return TargetRes::Invalid;
        }
        if let Some(attr) = Attr::from_name(name) {
            let available = match self.block {
                // **A field has no element**, so no attribute is writable in
                // one. It is a function of space, and the material that happens
                // to be at a point is not something it is given.
                Some(BlockKind::Field) => false,
                Some(BlockKind::Spawn) | Some(BlockKind::Element) => self.emit.contains(&attr),
                // **A `deform` writes what it consumes as well as what it
                // emits**, and rewriting is the more common of the two:
                // `position = position + …` is what a modulator is for. An L1
                // has one buffer and one list; an L2 has an input edge and an
                // output one, and it may write anything that reaches the output
                // — which is everything it was given, plus everything it adds.
                Some(BlockKind::Deform) => {
                    self.emit.contains(&attr) || self.consumes.contains(&attr)
                }
                // An L5 is handed a picture, and a texel has no element
                // behind it to write an attribute onto.
                Some(BlockKind::Vertex)
                | Some(BlockKind::Fragment)
                | Some(BlockKind::Camera)
                | Some(BlockKind::Mask)
                | Some(BlockKind::Frame)
                | None => false,
            };
            if !available {
                let hint = match self.block {
                    Some(BlockKind::Vertex) | Some(BlockKind::Fragment) => {
                        "attributes are read-only in vertex/fragment blocks — assign a stage \
                         output instead"
                            .to_string()
                    }
                    Some(BlockKind::Deform) => format!(
                        "add `{name}` to `consumes` to rewrite what reaches this node, or to \
                         `emit` to add it to what leaves"
                    ),
                    Some(BlockKind::Mask) => "a mask says where the deformation applies and \
                         writes nothing but `strength` — rewrite the attribute in `deform`"
                        .to_string(),
                    _ => format!("add `{name}` to `emit` to make it writable here"),
                };
                self.err_hint(
                    Stage::Contract,
                    span,
                    format!("attribute `{name}` is not writable here"),
                    hint,
                );
                return TargetRes::Invalid;
            }
            return TargetRes::Attr(attr);
        }
        // **In a `camera` block, `eye` is the output rather than the ambient.**
        // The two name one thing — where the camera is — written here and read
        // in a marching fragment stage, the same way `position` is written by
        // an L1 and read by an L4. Ambients are tried first below, which is
        // right everywhere else and wrong in exactly this block: without this,
        // the one word an L3 author writes most reports "cannot assign to an
        // ambient value".
        if self.block == Some(BlockKind::Camera) {
            if let Some(output) = Output::from_name(name) {
                if output.block() == BlockKind::Camera {
                    return TargetRes::Output(output);
                }
            }
        }
        if Ambient::from_name(name).is_some() {
            self.err(
                Stage::Type,
                span,
                format!("cannot assign to `{name}`; it is an ambient value"),
            );
            return TargetRes::Invalid;
        }
        if let Some(output) = Output::from_name(name) {
            // **The block that writes it, asked of the kind** — `color` is a
            // `fragment` block's on an L4 and a `frame` block's on an L5. See
            // [`output_block`].
            let owner = output_block(output, self.kind);
            if self.block != Some(owner) {
                self.err_hint(
                    Stage::Contract,
                    span,
                    format!("`{name}` belongs to the `{}` block", owner.name()),
                    format!("write `{name}` inside `{}`, not here", owner.name()),
                );
                return TargetRes::Invalid;
            }
            return TargetRes::Output(output);
        }
        // **The renamed output gets its own refusal rather than the generic
        // one.** `point_rate` was the spelling until the unit changed from
        // pixels to a fraction of the target's height, and a file still
        // written against the old one is not a typo and not an undeclared
        // name: it is a file that needs one edit and one division. Reporting
        // it as "never declared" would leave the author guessing at both.
        if name == OLD_POINT_SIZE {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{OLD_POINT_SIZE}` is now spelled `point_rate`"),
                POINT_RATE_HINT.to_string(),
            );
            return TargetRes::Invalid;
        }
        self.err_hint(
            Stage::Type,
            span,
            format!("assigning to `{name}`, which was never declared"),
            "assigning to a name that was never declared is an error, not a declaration — \
             declare it first with `let` or `var`",
        );
        TargetRes::Invalid
    }

    fn check_assign(
        &mut self,
        target: &str,
        op: Option<BinOp>,
        value: &Expr,
        span: Span,
    ) -> Option<TStmt> {
        let value_t = self.check_expr(value);
        let res = self.resolve_target(target, span);

        if op.is_some() {
            match res {
                TargetRes::Attr(_) => {
                    self.err_hint(
                        Stage::Type,
                        span,
                        format!("compound assignment is not allowed on attribute `{target}`"),
                        format!(
                            "on an attribute this reads like accumulation but actually re-reads \
                             the previous frame's value every time — write `let v = ...;` then \
                             `{target} = v;`"
                        ),
                    );
                    return None;
                }
                TargetRes::Output(_) => {
                    self.err(
                        Stage::Type,
                        span,
                        format!("compound assignment is not allowed on stage output `{target}`"),
                    );
                    return None;
                }
                _ => {}
            }
        }

        match res {
            TargetRes::Invalid => None,
            TargetRes::Local(name, ty, mutable) => {
                if !mutable {
                    self.err_hint(
                        Stage::Type,
                        span,
                        format!("cannot assign to `{name}`; `let` bindings are immutable"),
                        "declare it with `var` instead if it needs to change",
                    );
                    return None;
                }
                let value_t = value_t?;
                let final_v = match op {
                    None => value_t,
                    Some(binop) => {
                        let lhs = TExpr::new(ty, span, TExprKind::Local(name.clone()));
                        self.desugar_compound(binop, lhs, value_t, span)?
                    }
                };
                if final_v.ty != ty {
                    self.err(
                        Stage::Type,
                        span,
                        format!(
                            "cannot assign `{}` to `{name}`, which has type `{}`",
                            final_v.ty.name(),
                            ty.name()
                        ),
                    );
                    return None;
                }
                Some(TStmt::Assign {
                    target: Target::Local(name),
                    value: final_v,
                    span,
                })
            }
            TargetRes::Attr(attr) => {
                let value_t = value_t?;
                if value_t.ty != attr.ty() {
                    self.err(
                        Stage::Type,
                        span,
                        format!(
                            "cannot assign `{}` to attribute `{}`, which has type `{}`",
                            value_t.ty.name(),
                            attr.name(),
                            attr.ty().name()
                        ),
                    );
                    return None;
                }
                Some(TStmt::Assign {
                    target: Target::Attr(attr),
                    value: value_t,
                    span,
                })
            }
            TargetRes::Output(output) => {
                let value_t = value_t?;
                if value_t.ty != output.ty() {
                    self.err(
                        Stage::Type,
                        span,
                        format!(
                            "cannot assign `{}` to `{}`, which has type `{}`",
                            value_t.ty.name(),
                            output.name(),
                            output.ty().name()
                        ),
                    );
                    return None;
                }
                Some(TStmt::Assign {
                    target: Target::Output(output),
                    value: value_t,
                    span,
                })
            }
        }
    }

    fn check_if(&mut self, cond: &Expr, then: &[Stmt], els: &[Stmt], span: Span) -> Option<TStmt> {
        let cond_t = self.check_expr(cond);
        if let Some(c) = &cond_t {
            if c.ty != Ty::Bool {
                self.err(
                    Stage::Type,
                    c.span,
                    format!("`if` condition must be `bool`, found `{}`", c.ty.name()),
                );
            }
        }
        self.scope.push();
        let then_t = self.check_stmts(then);
        self.scope.pop();
        self.scope.push();
        let els_t = self.check_stmts(els);
        self.scope.pop();

        let cond_t = cond_t.unwrap_or_else(|| poison(Ty::Bool, cond.span()));
        Some(TStmt::If {
            cond: cond_t,
            then: then_t,
            els: els_t,
            span,
        })
    }

    fn check_for(
        &mut self,
        var: &str,
        start: i32,
        end: i32,
        body: &[Stmt],
        span: Span,
    ) -> Option<TStmt> {
        self.scope.push();
        self.check_declarable_name(var, span);
        self.scope.declare(
            var.to_string(),
            LocalInfo {
                ty: Ty::Int,
                mutable: false,
            },
        );
        let body_t = self.check_stmts(body);
        self.scope.pop();
        Some(TStmt::For {
            var: var.to_string(),
            start,
            end,
            body: body_t,
            span,
        })
    }

    fn check_kill(&mut self, span: Span) -> Option<TStmt> {
        if !(self.kind == Kind::L1 && self.block == Some(BlockKind::Element)) {
            // **An L2 gets its own reason**, because "move it into the `element`
            // block" names a block an L2 does not have and would send an author
            // looking for one. The rule there is structural: compaction runs
            // once, after L1, and nothing downstream of a deformation
            // reconsiders liveness — so a `kill()` here would remove an element
            // from a buffer whose live range had already been decided.
            let hint = if self.kind == Kind::L2 {
                "an L2 rewrites elements and never removes them: compaction runs once, after L1, \
                so liveness is settled before a `deform` sees anything. Fade it out instead — \
                write `size` or `color`'s alpha — or kill it in the L1"
            } else {
                "remove it, or move this logic into the `element` block"
            };
            self.err_hint(
                Stage::Contract,
                span,
                "`kill()` is only legal in an L1 `element` block",
                hint,
            );
        }
        Some(TStmt::Kill { span })
    }

    // -- expressions ----------------------------------------------------------

    pub(crate) fn check_expr(&mut self, e: &Expr) -> Option<TExpr> {
        match e {
            Expr::Lit { value, span } => {
                Some(TExpr::new(value.ty(), *span, TExprKind::Lit(*value)))
            }
            Expr::Ident { name, span } => self.check_ident(name, *span),
            Expr::Unary { op, value, span } => {
                let v = self.check_expr(value)?;
                let ty = self.check_unary(*op, v.ty, *span)?;
                Some(TExpr::new(
                    ty,
                    *span,
                    TExprKind::Unary {
                        op: *op,
                        value: Box::new(v),
                    },
                ))
            }
            Expr::Binary { op, lhs, rhs, span } => {
                let l = self.check_expr(lhs);
                let r = self.check_expr(rhs);
                let (l, r) = (l?, r?);
                self.desugar_compound(*op, l, r, *span)
            }
            Expr::Call { name, args, span } => self.check_call(name, args, *span),
            Expr::Swizzle {
                value,
                components,
                span,
            } => self.check_swizzle(value, components, *span),
        }
    }

    fn check_ident(&mut self, name: &str, span: Span) -> Option<TExpr> {
        if let Some(info) = self.scope.lookup(name) {
            return Some(TExpr::new(
                info.ty,
                span,
                TExprKind::Local(name.to_string()),
            ));
        }
        if let Some(ty) = self.params.get(name) {
            return Some(TExpr::new(*ty, span, TExprKind::Param(name.to_string())));
        }
        if let Some(attr) = Attr::from_name(name) {
            let available = match self.block {
                // **A `spawn` block reads only what this procedure emits.**
                // An attribute it merely `consumes` is one the engine derives,
                // and every rule reads state an element does not have yet: the
                // spawn instant is written after this block runs, and last
                // step's position is a step this element has not lived. Read
                // here, both would be whatever the slot held for the element
                // that last occupied it.
                //
                // Refused rather than substituted, because there is no value to
                // substitute. It is also refused rather than left to the
                // lowering, which had no field to name and produced WGSL naga
                // rejects — a `.kir` that checked clean and took the process
                // down, which is the one shape this pass exists to prevent.
                // Read side of the same rule: no element, no attributes.
                Some(BlockKind::Field) => false,
                Some(BlockKind::Spawn) => self.emit.contains(&attr),
                Some(BlockKind::Element) => {
                    self.emit.contains(&attr) || self.consumes.contains(&attr)
                }
                // **Both lists, and they mean different things here.** A
                // `deform` reads what it `consumes` from upstream and reads
                // back what it `emit`s, because an L2 that widens the element —
                // adding a `tint` nothing produced — has to be able to read the
                // field it is writing. Same shape as an `element` block, for a
                // different reason: there the two lists are one buffer, here
                // they are the input edge and the output one.
                Some(BlockKind::Deform) => {
                    self.emit.contains(&attr) || self.consumes.contains(&attr)
                }
                // **`consumes` only, unlike the `deform` beside it.** A mask
                // decides where the deformation applies, which is a question
                // about what *reaches* this node — and an `emit`ted attribute
                // has not been written yet when the mask runs, so reading one
                // here would read the zero the pass-through left rather than a
                // value. Refusing it is better than a rule an author has to
                // remember.
                Some(BlockKind::Mask) => self.consumes.contains(&attr),
                Some(BlockKind::Vertex) | Some(BlockKind::Fragment) => {
                    self.consumes.contains(&attr)
                }
                // An L3 has no element in hand — see `check_header`. Neither
                // has an L5: what it is handed is the picture the elements
                // already drew.
                Some(BlockKind::Camera) | Some(BlockKind::Frame) | None => false,
            };
            if available {
                return Some(TExpr::new(attr.ty(), span, TExprKind::Attr(attr)));
            }
            let hint = match self.block {
                Some(BlockKind::Field) => format!(
                    "a field is a function of space: it is handed `point` and nothing else, \
                     so `{name}` — a property of an element — has no meaning here"
                ),
                Some(BlockKind::Vertex) | Some(BlockKind::Fragment) => {
                    format!("add `{name}` to `consumes` to read it here")
                }
                Some(BlockKind::Spawn) if attr.is_derivable() && self.consumes.contains(&attr) => {
                    format!(
                        "`{name}` is derived, and a derivation reads state an element being \
                         spawned does not have yet — add `{name}` to `emit` and write it here, \
                         or read it in `element`"
                    )
                }
                Some(BlockKind::Spawn) | Some(BlockKind::Element) => {
                    format!("add `{name}` to `emit` to read it here")
                }
                Some(BlockKind::Deform) => format!(
                    "add `{name}` to `consumes` to read what reaches this node, or to \
                     `emit` to add it to what leaves"
                ),
                Some(BlockKind::Mask) => format!(
                    "add `{name}` to `consumes` — a mask reads what reaches this node, and an \
                     `emit`ted attribute has not been written when it runs"
                ),
                Some(BlockKind::Camera) => "a camera reads the clock and its params, not \
                     elements — pointing one at geometry means naming a reduction or element \
                     zero, which `docs/ir-spec.md` specifies and nothing builds yet"
                    .to_string(),
                Some(BlockKind::Frame) => format!(
                    "an L5 is handed a picture rather than the material that made it, so \
                     `{name}` — a property of an element — has nothing here to be a property \
                     of. Read the frame with `texel(src)` and `tap(src, uv)`"
                ),
                None => "attributes are not available in a header expression".to_string(),
            };
            self.err_hint(
                Stage::Contract,
                span,
                format!("attribute `{name}` is not available here"),
                hint,
            );
            return None;
        }
        if let Some(ambient) = Ambient::from_name(name) {
            // **`source` and a geometry slot cannot both be in one
            // procedure**, and the read is where it is caught because the read
            // is what is ambiguous.
            //
            // A pairing Set is *one* source made of two simulations: the far
            // one feeds the slot and shares the near one's uniform, so there is
            // one `seed_salt` for two geometries and `source` here would
            // silently mean the near one. Refused rather than defined as the
            // near side, because a value that quietly answers for one of two
            // things is the shape this whole notation exists to remove — and
            // the sentence names the slot, so the author is told *which*
            // reading was ambiguous rather than left to infer a rule.
            if ambient == Ambient::Source {
                if let Some(far) = self.paired {
                    self.err_hint(
                        Stage::Contract,
                        span,
                        format!(
                            "`source` is ambiguous in a procedure that declares `{far} : Geometry`"
                        ),
                        format!(
                            "this node has two geometries in hand and one identity to answer \
                             with: the far one feeds `{far}` and shares this node's uniform, so \
                             `source` would mean the near one and say nothing about it. Compare \
                             against a `uses <name> : Source` slot in a node that takes no \
                             second geometry"
                        ),
                    );
                    return None;
                }
            }
            let available = match self.block {
                Some(block) => {
                    ambient.available_in(self.kind, block)
                        && self.marching_only(ambient)
                        && self.element_only(ambient)
                }
                None => false,
            };
            if available {
                return Some(TExpr::new(ambient.ty(), span, TExprKind::Ambient(ambient)));
            }
            // A marcher's values get their own sentence, because "not available
            // in this block" would send an author looking at the block when what
            // is wrong is that the procedure has a `vertex` block at all.
            // The converse sentence, for the converse mistake — and it names
            // the `vertex` block too, because that is the thing to add rather
            // than the thing to remove.
            if matches!(ambient, Ambient::Seed | Ambient::Copy) && self.fullscreen {
                self.err_hint(
                    Stage::Contract,
                    span,
                    format!("`{name}` is per element, and this procedure draws the whole frame"),
                    "an L4 with no `vertex` block covers the frame and has no element to have \
                     an identity. Add a `vertex` block to draw elements, or drive the \
                     picture from `eye`, `ray` and `point_coord`, which are what a \
                     marcher has",
                );
                return None;
            }
            // The same shape once more, from the third side. A field has no
            // element for a reason an author cannot see from the block: it is
            // not a node, so "which element" has no answer at the point the
            // splice lands.
            if ambient == Ambient::Seed && self.kind == Kind::Field {
                self.err_hint(
                    Stage::Contract,
                    span,
                    "`seed` is per element, and a field has none",
                    "a field is a function of space — it is handed `point` and nothing else, \
                     and it is spliced into every procedure that declares a slot for it, \
                     including fragment stages that have no element at all. Vary it with `t`, `beats` \
                     or a `param` instead",
                );
                return None;
            }
            if matches!(ambient, Ambient::Eye | Ambient::Ray) && self.kind == Kind::L4 {
                self.err_hint(
                    Stage::Contract,
                    span,
                    format!("`{name}` is only available to a procedure that draws the whole frame"),
                    "a `vertex` block is what makes an L4 per-element, and a ray through a \
                     fragment is not something a sprite has. Remove the `vertex` block to \
                     march, or read the attributes this procedure `consumes` instead",
                );
                return None;
            }
            // **The eight an L5 refuses, each by name and each with its own
            // sentence** — which is the whole of P-0083 at this layer: a
            // refusal that names the fix beats one that names the rule.
            //
            // Before the three sentences below it, because every one of those
            // is about a kind an L5 is not: `seed` here is not a fullscreen
            // L4's `seed`, and `source` here is not a field's.
            if self.kind == Kind::L5 {
                let hint = match ambient {
                    Ambient::Seed => {
                        "`seed` is not available to an L5: a fullscreen pass has no element. \
                         Vary the picture with `point_coord`, `t`, `beats` or a `param` instead"
                    }
                    Ambient::Copy => {
                        "`copy` is which copy of its parent an element is, and an L5 has no \
                         element: the frame in front of it was drawn by every copy at once. \
                         Drive the effect from a `param`"
                    }
                    Ambient::Source => {
                        "`source` names which geometry a chain instance runs over, and the \
                         frame an L5 is handed may hold several decks\u{2019} material folded \
                         together. Mask upstream, in a node that runs over one geometry"
                    }
                    Ambient::Point => {
                        "`point` is a `field` block\u{2019}s one input and nothing else\u{2019}s. An L5 \
                         has a frame coordinate — read `point_coord`, which is 0..1 across \
                         the frame and is the coordinate `tap` takes"
                    }
                    Ambient::Capacity => {
                        "`capacity` is how much material an L1 was built for, and an L5 makes \
                         no material: it covers the frame exactly once whatever is in it"
                    }
                    Ambient::Camera => {
                        "`camera` projects an element, and an L5 has none — the frame in front \
                         of it may hold several decks\u{2019} material seen from several cameras, \
                         so there is no one viewpoint for it to name"
                    }
                    Ambient::Eye | Ambient::Ray => {
                        "`eye` and `ray` are what a marcher looks along, and an L5 is standing \
                         on the picture a marcher already drew — there is no one viewpoint it \
                         could be. March in a fullscreen L4 and hand this pass the result"
                    }
                    // The four an L5 does read, and none of them reaches here.
                    Ambient::T | Ambient::Beats | Ambient::Dt | Ambient::PointCoord => {
                        unreachable!("`{}` is available in a `frame` block", ambient.name())
                    }
                };
                self.err_hint(
                    Stage::Contract,
                    span,
                    format!("`{name}` is not available to an L5"),
                    hint,
                );
                return None;
            }
            // The fourth such sentence, for the two kinds that have no chain
            // instance to be running over. "Not available in this block" would
            // send an author looking at the block, where what is wrong is the
            // `kind` — a camera and a field are not run per geometry at all.
            if ambient == Ambient::Source && matches!(self.kind, Kind::L3 | Kind::Field) {
                let (what, hint) = match self.kind {
                    Kind::L3 => (
                        "a camera",
                        "an L3 runs once a frame over nothing, and the identity `source` \
                         names is a property of a Set's geometry — which a camera is not. \
                         Read the clock and this procedure's params instead",
                    ),
                    _ => (
                        "a field",
                        "a field is a function of space: it is spliced into whichever \
                         procedures evaluate it, including fragment stages that run over no \
                         geometry at all, so there is no chain instance for it to belong to. \
                         Mask in the caller instead, where there is one",
                    ),
                };
                self.err_hint(
                    Stage::Contract,
                    span,
                    format!("`source` is per geometry, and {what} runs over none"),
                    hint,
                );
                return None;
            }
            self.err(
                Stage::Contract,
                span,
                format!("`{name}` is not available in this block"),
            );
            return None;
        }
        // **A geometry is not a value.** `far` on its own is the whole second
        // source, which this language has no type for and no way to pass — the
        // one thing that can be said about it is what one of its elements
        // holds.
        //
        // **Before the stage outputs**, and that is not an ordering
        // convenience: `near` and `far` are the camera's clip planes, so they
        // are output names on an L3 and ordinary names everywhere else — which
        // is exactly what `shadows_output` already says by asking about the
        // kind. Asked in the other order, an L2 slot called `far` would be
        // declarable and unreadable.
        if self.uses == Some(name) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is a geometry, not a value"),
                format!(
                    "read an attribute of the element it is paired with — `{name}.position` \
                     — which is the whole of what a used geometry offers"
                ),
            );
            return None;
        }
        // **A field is not a value either**, and for the mirror reason: it is a
        // function of space, so what can be had from it is its value *at* a
        // point. The language has no function type and nothing to pass one to.
        if self.fields.contains(&name) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is a field, not a value"),
                format!(
                    "evaluate it at a point — `{name}(p)` — which is the whole of what a \
                     field offers"
                ),
            );
            return None;
        }
        // **A texture is not a value, and it is the one slot type that can
        // never become one.** A geometry has no type for a whole source, a
        // field has none until it is evaluated, a camera is six numbers — and
        // each of those sentences is about a value this language could in
        // principle have. This one is not: what a texture offers is a *fetch*,
        // at this fragment or at a coordinate, and both of those are the two
        // builtins rather than a value with parts.
        //
        // **Before the ambient arm and before the undefined fallthrough**, so
        // that `src` reads as what it is rather than as a name nobody declared
        // — which is the sentence an author of a `frame` block would find
        // hardest to act on.
        if let Some(tex) = self.texture(name) {
            let what = match tex {
                TexRef::Src => "the incoming frame".to_string(),
                TexRef::Held => "the retained frame".to_string(),
                TexRef::Slot(_) => {
                    format!("a picture this procedure folds in, from `uses {name} : Texture`")
                }
            };
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is a texture, not a value"),
                format!(
                    "fetch from it — `texel({name})` for this fragment's own texel, unfiltered, \
                     or `tap({name}, uv)` for a filtered sample at a frame coordinate. `{name}` \
                     is {what}, and a picture is not something the language can hold"
                ),
            );
            return None;
        }
        // **`held` named where nothing retains a frame.** Refused with a
        // sentence about the *declaration* rather than about the name, because
        // the name is right and the header is what is missing — and refused
        // only in a `frame` block, since `held` is an ordinary word everywhere
        // else and an author who calls a local that is not shadowing anything.
        if name == TEXTURE_HELD && self.block == Some(BlockKind::Frame) && !self.retains {
            self.err_hint(
                Stage::Contract,
                span,
                "`held` is available only under `retains`",
                "add a bare `retains` to the header: it says this procedure reads a retained \
                 cut of the previous frame, and it is what makes `held` a texture here. Which \
                 cut is held — `mix` or `exit` — is answered where the procedure is placed, \
                 not in the file",
            );
            return None;
        }
        // **A Source slot *is* a value, and it is the one that is.** The three
        // refusals around it say the language has no type for a whole source,
        // no function type and nothing for six numbers — all true, and none of
        // them about this: what a Source slot binds is the assigned `uint` that
        // identifies one geometry, and `uint` is a type the language has.
        //
        // **Before the camera arm and after the field one**, which is only
        // where it reads best: the four slot names are checked against four
        // disjoint lists, so the order between them decides nothing.
        if self.sources.contains(&name) {
            return Some(TExpr::new(
                Ty::Uint,
                span,
                TExprKind::Source {
                    slot: name.to_string(),
                },
            ));
        }
        // **A camera is not a value either.** It is six numbers and three
        // derivations of them, and the language has no type for any of that —
        // what can be had is one of the parts, which is what the members are.
        if self.camera == Some(name) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` is a camera, not a value"),
                format!(
                    "read one of its parts — `{name}.clip`, `{name}.eye`, `{name}.ray` — \
                     which is the whole of what a camera offers a renderer"
                ),
            );
            return None;
        }
        if Output::from_name(name).is_some() {
            self.err_hint(
                Stage::Contract,
                span,
                format!("reading `{name}` is not allowed; stage outputs are write-only"),
                "bind a `let` to the value you need before writing it, and read the `let` instead",
            );
            return None;
        }
        // The same rename, met in an expression rather than as a target. It is
        // still a stale spelling and still needs the same sentence, even
        // though reading the output was never allowed under either name.
        if name == OLD_POINT_SIZE {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{OLD_POINT_SIZE}` is now spelled `point_rate`"),
                POINT_RATE_HINT.to_string(),
            );
            return None;
        }

        // Truly unresolved. The signal bus accepts any name (see the module
        // docs), so this is the only diagnosis available: either the author
        // meant to read a signal directly, which IR cannot do, or made a
        // typo, which the hint below will not fit as well but does not
        // actively mislead either.
        if self.block.is_some() {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{name}` does not resolve to a local, a param, an attribute, or an ambient value"),
                format!(
                    "the signal bus is not readable from IR — declare `param {name}` and attach \
                     a `bind` record to it in the Set file"
                ),
            );
        } else {
            self.err(Stage::Type, span, format!("`{name}` is undefined"));
        }
        None
    }

    fn check_unary(&mut self, op: UnOp, ty: Ty, span: Span) -> Option<Ty> {
        match op {
            UnOp::Neg => {
                if matches!(ty, Ty::Float | Ty::Int | Ty::Vec2 | Ty::Vec3 | Ty::Vec4) {
                    Some(ty)
                } else {
                    self.err(
                        Stage::Type,
                        span,
                        format!("`-` is not defined for `{}`", ty.name()),
                    );
                    None
                }
            }
            UnOp::Not => {
                if ty == Ty::Bool {
                    Some(Ty::Bool)
                } else {
                    self.err(
                        Stage::Type,
                        span,
                        format!("`!` requires `bool`, found `{}`", ty.name()),
                    );
                    None
                }
            }
        }
    }

    fn desugar_compound(&mut self, op: BinOp, lhs: TExpr, rhs: TExpr, span: Span) -> Option<TExpr> {
        let ty = self.binary_ty(op, lhs.ty, rhs.ty, span)?;
        Some(TExpr::new(
            ty,
            span,
            TExprKind::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
        ))
    }

    fn binary_ty(&mut self, op: BinOp, lhs: Ty, rhs: Ty, span: Span) -> Option<Ty> {
        if op.is_logical() {
            if lhs == Ty::Bool && rhs == Ty::Bool {
                return Some(Ty::Bool);
            }
            self.err(
                Stage::Type,
                span,
                format!(
                    "`{}` requires `bool` operands, found `{}` and `{}`",
                    op_symbol(op),
                    lhs.name(),
                    rhs.name()
                ),
            );
            return None;
        }
        if op.is_comparison() {
            let ok = if matches!(op, BinOp::Eq | BinOp::Ne) {
                lhs == rhs
            } else {
                lhs == rhs && matches!(lhs, Ty::Float | Ty::Int | Ty::Uint)
            };
            if ok {
                return Some(Ty::Bool);
            }
            self.err(
                Stage::Type,
                span,
                format!(
                    "`{}` is not defined for `{}` and `{}`",
                    op_symbol(op),
                    lhs.name(),
                    rhs.name()
                ),
            );
            return None;
        }

        // Arithmetic: same type, or `float`/vector broadcast, or the
        // `camera * vecN` special case (matrices have no other operation).
        if lhs == rhs
            && matches!(
                lhs,
                Ty::Float | Ty::Int | Ty::Uint | Ty::Vec2 | Ty::Vec3 | Ty::Vec4
            )
        {
            return Some(lhs);
        }
        match (lhs, rhs) {
            (v, Ty::Float) | (Ty::Float, v) if matches!(v, Ty::Vec2 | Ty::Vec3 | Ty::Vec4) => {
                return Some(v);
            }
            (Ty::Mat4, Ty::Vec4) if op == BinOp::Mul => return Some(Ty::Vec4),
            (Ty::Mat3, Ty::Vec3) if op == BinOp::Mul => return Some(Ty::Vec3),
            _ => {}
        }
        self.err(
            Stage::Type,
            span,
            format!(
                "`{}` is not defined for `{}` and `{}` — there are no implicit conversions",
                op_symbol(op),
                lhs.name(),
                rhs.name()
            ),
        );
        None
    }

    /// What the header declared, before the table the language ships.
    ///
    /// A field is reached through a slot, so the name at a call site is the
    /// procedure's own — and this branch is the whole of that. It is first for the
    /// reason it is a branch at all: a call resolves against what the file said it
    /// takes, and asking the builtin table first would make the language's
    /// vocabulary quietly outrank the header. Nothing is hidden by the order, since
    /// a slot named after a builtin is refused where it is declared — see
    /// `check_slot_names`.
    fn check_call(&mut self, name: &str, args: &[Expr], span: Span) -> Option<TExpr> {
        if self.fields.contains(&name) {
            return self.check_field_call(name, args, span);
        }
        if let Some(builtin) = Builtin::from_name(name) {
            return self.check_builtin_call(builtin, args, span);
        }
        if let Some(ty) = Ty::from_name(name) {
            return self.check_constructor(ty, args, span);
        }
        // **`field` is an ordinary name now**, and this is the sentence the
        // person migrating a file reads. It was the reserved word a field was
        // reached through, and reserving one is exactly what capped a procedure
        // at a single input — so it is gone rather than kept as an alias for
        // "the one field, if there is exactly one", which is the rule the slot
        // exists to remove.
        let hint = (name == "field").then_some(
            "`field(p)` is no longer the way to evaluate one: declare which field this \
             procedure takes — `uses shape : Field` — and call it by that name, `shape(p)`. \
             The Set binds it with `--edge <node>.shape=<field>`",
        );
        match hint {
            Some(hint) => self.err_hint(
                Stage::Type,
                span,
                format!("`{name}` is not a builtin function or a type constructor"),
                hint,
            ),
            None => self.err(
                Stage::Type,
                span,
                format!("`{name}` is not a builtin function or a type constructor"),
            ),
        }
        for a in args {
            self.check_expr(a);
        }
        None
    }

    /// A field's signature is the field block's: one `vec3` in, a `float` out. It
    /// is not read off the bound procedure, because no single file holds one —
    /// every `kind Field` has exactly this shape, which is what makes a slot
    /// bindable at all.
    fn check_field_call(&mut self, slot: &str, args: &[Expr], span: Span) -> Option<TExpr> {
        if args.len() != 1 {
            self.err_hint(
                Stage::Type,
                span,
                format!("`{slot}` takes 1 argument, found {}", args.len()),
                "a field is handed a position and returns the distance at it",
            );
            for a in args {
                self.check_expr(a);
            }
            return None;
        }
        let point = self.check_expr(&args[0])?;
        if point.ty != Ty::Vec3 {
            self.err(
                Stage::Type,
                point.span,
                format!("`{slot}` expects `vec3`, found `{}`", point.ty.name()),
            );
            return None;
        }
        Some(TExpr::new(
            Ty::Float,
            span,
            TExprKind::Field {
                slot: slot.to_string(),
                point: Box::new(point),
            },
        ))
    }

    /// A fetch from a named texture — `texel(src)`, `tap(held, uv)`.
    ///
    /// The texture position takes a bare name and nothing else: not a local holding
    /// one, because there is no type to hold it in, and not an expression, because
    /// there is nothing to compute. Every other position is checked the ordinary
    /// way, which today is `tap`'s `vec2`.
    fn check_texture_call(
        &mut self,
        b: Builtin,
        tex_at: usize,
        args_ast: &[Expr],
        span: Span,
    ) -> Option<TExpr> {
        let sig = b.signature();
        if args_ast.len() != sig.args.len() {
            let hint = if b == Builtin::Texel {
                "`texel` takes the texture and no coordinate, deliberately: a coordinate is an \
                 invitation to resample, and a centre tap that resamples makes a pass at an \
                 amount just above zero differ from one that did not run by what a filter did \
                 rather than by what the effect is. `tap(<texture>, uv)` is the filtered read"
            } else {
                "`tap` takes the texture and a frame coordinate — `tap(src, point_coord)`, \
                 where the coordinate runs 0..1 across the frame and `frame_step` is what \
                 turns a distance into one"
            };
            self.err_hint(
                Stage::Type,
                span,
                format!(
                    "`{}` takes {} argument(s), found {}",
                    b.name(),
                    sig.args.len(),
                    args_ast.len()
                ),
                hint,
            );
            for a in args_ast {
                self.check_expr(a);
            }
            return None;
        }

        // Resolved first, so that a mistake in the coordinate does not hide a
        // mistake in the texture — both are reported in one pass.
        let texture = match &args_ast[tex_at] {
            Expr::Ident { name, .. } => match self.texture(name) {
                Some(tex) => Some(tex),
                None => {
                    // `check_ident` owns every sentence about why a name is not
                    // a texture — `held` without `retains`, an undeclared slot,
                    // a param — so it is asked rather than second-guessed here.
                    // It always reports, since a name that resolved to
                    // something else is not a texture either.
                    match self.check_ident(name, args_ast[tex_at].span()) {
                        Some(other) => {
                            self.err_hint(
                                Stage::Type,
                                other.span,
                                format!(
                                    "`{}` expects a texture, found a `{}`",
                                    b.name(),
                                    other.ty.name()
                                ),
                                "the first argument names a texture this procedure is \
                                 handed — `src`, `held` under `retains`, or a slot from \
                                 `uses <name> : Texture`",
                            );
                            None
                        }
                        None => None,
                    }
                }
            },
            other => {
                self.err_hint(
                    Stage::Type,
                    other.span(),
                    format!("`{}` expects a texture name here", b.name()),
                    "a texture is named rather than computed: there is no value of one to \
                     build, pass or bind to a `let`. Write `src`, `held` under `retains`, or \
                     the name of a `uses <name> : Texture` slot",
                );
                self.check_expr(other);
                None
            }
        };

        let mut at: Option<Box<TExpr>> = None;
        let mut ok = texture.is_some();
        for (i, arg) in args_ast.iter().enumerate() {
            if i == tex_at {
                continue;
            }
            let Some(t) = self.check_expr(arg) else {
                ok = false;
                continue;
            };
            if t.ty != Ty::Vec2 {
                self.err_hint(
                    Stage::Type,
                    t.span,
                    format!("`{}` expects `vec2`, found `{}`", b.name(), t.ty.name()),
                    "a frame coordinate runs 0..1 across the frame, x to the right and y \
                     down — `point_coord`, and `frame_step` for a distance to add to it",
                );
                ok = false;
                continue;
            }
            at = Some(Box::new(t));
        }
        if !ok {
            return None;
        }

        Some(TExpr::new(
            Ty::Vec4,
            span,
            TExprKind::Sample {
                func: b,
                texture: texture.expect("checked above"),
                at,
            },
        ))
    }

    fn check_builtin_call(&mut self, b: Builtin, args_ast: &[Expr], span: Span) -> Option<TExpr> {
        let sig = b.signature();
        // **Refused by kind before it is refused by shape**, because the shape
        // is not what is wrong: `frame_step(0.02)` types perfectly well in an
        // L4 and lowers to a read of a `viewport` field no other module carries
        // — which is a `.kir` that checks clean and comes up short at stage 5
        // ([ADR-0032](../../../docs/adr/0032-nothing-checks-clean-and-comes-up-short-at-runtime.md)).
        // Its two neighbours refuse themselves for want of a texture name, and
        // they get this sentence anyway so that all three are one rule.
        if b.is_frame_effect() && self.kind != Kind::L5 {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{}` is an L5 builtin", b.name()),
                format!(
                    "`texel`, `tap` and `frame_step` read and measure the frame a pass is \
                     handed, and only a `kind L5` procedure is handed one. Change `kind` to \
                     `L5`, or drop the `{}` call",
                    b.name()
                ),
            );
            for a in args_ast {
                self.check_expr(a);
            }
            return None;
        }
        // **A texture argument is a name and not an expression**, so it is
        // resolved here rather than by `check_expr` below — see
        // [`Shape::Texture`]. What comes back is which binding to fetch from,
        // which is what the tree carries.
        if let Some(at) = b.texture_arg() {
            return self.check_texture_call(b, at, args_ast, span);
        }
        if args_ast.len() != sig.args.len() {
            self.err(
                Stage::Type,
                span,
                format!(
                    "`{}` takes {} argument(s), found {}",
                    b.name(),
                    sig.args.len(),
                    args_ast.len()
                ),
            );
            for a in args_ast {
                self.check_expr(a);
            }
            return None;
        }

        for &i in sig.const_args {
            if let Some(a) = args_ast.get(i) {
                if !matches!(
                    a,
                    Expr::Lit {
                        value: Lit::Int(_),
                        ..
                    }
                ) {
                    self.err_hint(
                        Stage::Type,
                        a.span(),
                        format!(
                            "argument {} to `{}` must be a literal integer",
                            i + 1,
                            b.name()
                        ),
                        "octave counts are unrolled at lowering time, so they cannot be a \
                         runtime value",
                    );
                }
            }
        }

        let mut checked_args = Vec::with_capacity(args_ast.len());
        let mut ok = true;
        for a in args_ast {
            match self.check_expr(a) {
                Some(t) => checked_args.push(t),
                None => ok = false,
            }
        }
        if !ok {
            return None;
        }

        let mut same_ty: Option<Ty> = None;
        for (shape, actual) in sig.args.iter().zip(checked_args.iter()) {
            match shape {
                Shape::Exact(t) => {
                    if actual.ty != *t {
                        self.err(
                            Stage::Type,
                            actual.span,
                            format!(
                                "`{}` expects `{}`, found `{}`",
                                b.name(),
                                t.name(),
                                actual.ty.name()
                            ),
                        );
                        ok = false;
                    }
                }
                Shape::Same => {
                    if !domain_allows(sig.domain, actual.ty) {
                        self.err(
                            Stage::Type,
                            actual.span,
                            format!("`{}` does not accept `{}`", b.name(), actual.ty.name()),
                        );
                        ok = false;
                    } else {
                        match same_ty {
                            None => same_ty = Some(actual.ty),
                            Some(t) if t == actual.ty => {}
                            Some(t) => {
                                self.err(
                                    Stage::Type,
                                    actual.span,
                                    format!(
                                        "`{}` expects every argument to be the same type; found \
                                         `{}` and `{}`",
                                        b.name(),
                                        t.name(),
                                        actual.ty.name()
                                    ),
                                );
                                ok = false;
                            }
                        }
                    }
                }
                Shape::Scalar => {
                    // No builtin puts `Scalar` in an argument position today
                    // (it only ever appears as `ret`); nothing to unify.
                }
                // Diverted to `check_texture_call` above, which is what
                // `Builtin::texture_arg` is asked for — a texture position is a
                // name rather than an expression, so there is nothing here that
                // could have been checked.
                Shape::Texture => {
                    unreachable!("a texture argument is resolved by name, not unified as a type")
                }
            }
        }
        if !ok {
            return None;
        }

        let ret_ty = match sig.ret {
            Shape::Exact(t) => t,
            Shape::Same => same_ty.unwrap_or(Ty::Float),
            Shape::Scalar => Ty::Float,
            // Nothing *returns* a texture, and nothing will: a builtin that
            // produced one would be a value of a type the language does not
            // have.
            Shape::Texture => unreachable!("no builtin returns a texture"),
        };
        Some(TExpr::new(
            ret_ty,
            span,
            TExprKind::Builtin {
                func: b,
                args: checked_args,
            },
        ))
    }

    fn check_constructor(&mut self, ty: Ty, args_ast: &[Expr], span: Span) -> Option<TExpr> {
        match ty {
            Ty::Float | Ty::Int | Ty::Uint => {
                if args_ast.len() != 1 {
                    self.err(
                        Stage::Type,
                        span,
                        format!("`{}(...)` takes exactly one argument", ty.name()),
                    );
                    for a in args_ast {
                        self.check_expr(a);
                    }
                    return None;
                }
                let a = self.check_expr(&args_ast[0])?;
                if !matches!(a.ty, Ty::Float | Ty::Int | Ty::Uint) {
                    self.err(
                        Stage::Type,
                        a.span,
                        format!(
                            "`{}(...)` converts a scalar number, found `{}`",
                            ty.name(),
                            a.ty.name()
                        ),
                    );
                    return None;
                }
                Some(TExpr::new(ty, span, TExprKind::Construct { args: vec![a] }))
            }
            Ty::Vec2 | Ty::Vec3 | Ty::Vec4 => {
                let n = ty.components().expect("vector has components") as usize;

                // Single-scalar broadcast: `vec3(0.0)`. Checked as its own
                // case because it is the one shape the general component-sum
                // rule below cannot express (a lone `float` sums to 1, not
                // `n`, for every `n` this branch handles).
                if args_ast.len() == 1 {
                    if let Some(a) = self.check_expr(&args_ast[0]) {
                        if a.ty == Ty::Float {
                            return Some(TExpr::new(
                                ty,
                                span,
                                TExprKind::Construct { args: vec![a] },
                            ));
                        }
                        // Not a scalar: fall through to the general rule,
                        // which will accept it if it happens to already be
                        // exactly `n` components (e.g. a redundant
                        // `vec3(some_vec3)`) and reject it otherwise.
                        return self.check_constructor_concat(ty, n, vec![a], span);
                    }
                    return None;
                }

                // General rule: any mix of `float`/`vec2`/`vec3`/`vec4`
                // arguments whose component counts sum to exactly `n` —
                // e.g. `vec4(position, 1.0)` concatenates a `vec3` and a
                // `float`. This is GLSL's actual vector-constructor rule;
                // the spec's prose ("one scalar per component, or a single
                // scalar to broadcast") undersells it — the spec's own
                // `soft_points` example uses `vec4(position, 1.0)`, which
                // only this broader rule accepts. See the module docs.
                let mut checked = Vec::with_capacity(args_ast.len());
                let mut ok = true;
                for a in args_ast {
                    match self.check_expr(a) {
                        Some(t) => checked.push(t),
                        None => ok = false,
                    }
                }
                if !ok {
                    return None;
                }
                self.check_constructor_concat(ty, n, checked, span)
            }
            Ty::Bool | Ty::Mat3 | Ty::Mat4 => {
                self.err(
                    Stage::Type,
                    span,
                    format!("`{}` cannot be constructed", ty.name()),
                );
                for a in args_ast {
                    self.check_expr(a);
                }
                None
            }
        }
    }

    /// The shared tail of vector construction: every argument must be
    /// `float`/`vec2`/`vec3`/`vec4`, and their component counts must sum to exactly
    /// `n`.
    fn check_constructor_concat(
        &mut self,
        ty: Ty,
        n: usize,
        args: Vec<TExpr>,
        span: Span,
    ) -> Option<TExpr> {
        let mut total = 0usize;
        let mut ok = true;
        for a in &args {
            match a.ty.components() {
                Some(c) if matches!(a.ty, Ty::Float | Ty::Vec2 | Ty::Vec3 | Ty::Vec4) => {
                    total += c as usize
                }
                _ => {
                    self.err(
                        Stage::Type,
                        a.span,
                        format!(
                            "`{}(...)` arguments must be `float` or a vector, found `{}`",
                            ty.name(),
                            a.ty.name()
                        ),
                    );
                    ok = false;
                }
            }
        }
        if !ok {
            return None;
        }
        if total != n {
            self.err_hint(
                Stage::Type,
                span,
                format!(
                    "`{}(...)` arguments have {} component(s) total, expected {}",
                    ty.name(),
                    total,
                    n
                ),
                format!(
                    "e.g. `{0}(1.0, 0.0, 0.0)`, `{0}(0.0)` to broadcast, or `{0}(v, 1.0)` to \
                     extend a smaller vector",
                    ty.name()
                ),
            );
            return None;
        }
        Some(TExpr::new(ty, span, TExprKind::Construct { args }))
    }

    /// `<slot>.<attr>` — an attribute of the far element, from the geometry bound
    /// to the slot this procedure declared.
    ///
    /// Only for something the node consumes. The far geometry is an input edge:
    /// this node reads it and writes its own output, so what is readable there is
    /// what the node declared it takes.
    ///
    /// `slot` is the name the header gave it, carried in only so the diagnostics
    /// are written in the author's own spelling. Which node fills it is not decided
    /// here and never can be — it is the Set's answer.
    fn check_far(&mut self, slot: &str, name: &str, span: Span) -> Option<TExpr> {
        let Some(attr) = Attr::from_name(name) else {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{slot}.{name}` is not an attribute"),
                format!(
                    "`{slot}` is a geometry, so it has the attributes an element has — \
                     `{slot}.position`, `{slot}.tint` — and nothing else"
                ),
            );
            return None;
        };
        if !matches!(self.block, Some(BlockKind::Deform) | Some(BlockKind::Mask)) {
            self.err(
                Stage::Contract,
                span,
                format!("`{slot}` is readable in a `deform` or a `mask`, and nowhere else"),
            );
            return None;
        }
        if !self.consumes.contains(&attr) {
            self.err_hint(
                Stage::Contract,
                span,
                format!("`{slot}.{name}` is not consumed"),
                format!(
                    "add `{name}` to `consumes`: a used geometry is an input edge, and one \
                     `consumes` covers both sides of it — a node reads the same attribute \
                     from each"
                ),
            );
            return None;
        }
        Some(TExpr::new(attr.ty(), span, TExprKind::Far(attr)))
    }

    /// `<slot>.<member>` — one part of the camera bound to the slot this procedure
    /// declared.
    ///
    /// The second resolution path, and the reason a Camera slot cost the checker
    /// anything at all. `far.position` resolves against the attribute table,
    /// because a geometry's parts *are* attributes; a camera's are not parts of
    /// anything else, so this asks the slot's *type* what it has. Three members,
    /// and each is a value an L4 could already read — which is what makes this a
    /// new spelling rather than a new capability, and why the L3's other five stay
    /// unreadable
    /// (`docs/adr/0153-a-renderer-reads-three-camera-members-and-the-l3s-five-stay-unreadable.md`):
    /// the unnamed forms are [`Ambient::Camera`], [`Ambient::Eye`] and
    /// [`Ambient::Ray`], and they mean the Set's camera where no slot was declared.
    ///
    /// So the stage rules are the ambients' own, asked rather than restated:
    /// `.clip` is readable wherever an L4 projects, and `.eye` and `.ray` only in
    /// the fragment stage of a procedure that draws the whole frame — because they
    /// are defined by the ray prologue such a stage opens with and by nothing else.
    /// A second copy of those rules here would be a second place for them to be
    /// wrong, and this one would be the copy nobody looks at.
    fn check_camera_member(&mut self, slot: &str, name: &str, span: Span) -> Option<TExpr> {
        let amb = match name {
            "clip" => Ambient::Camera,
            "eye" => Ambient::Eye,
            "ray" => Ambient::Ray,
            _ => {
                self.err_hint(
                    Stage::Contract,
                    span,
                    format!("`{slot}.{name}` is not part of a camera"),
                    format!(
                        "a camera offers a renderer three things — `{slot}.clip`, the \
                         projection to multiply a position by; `{slot}.eye`, where it is; \
                         and `{slot}.ray`, the direction through this fragment. Its `target`, \
                         `up`, `fov_y`, `near` and `far` are the L3's to write and no \
                         renderer can read them yet"
                    ),
                );
                return None;
            }
        };
        let available = match self.block {
            Some(block) => amb.available_in(self.kind, block) && self.marching_only(amb),
            None => false,
        };
        if available {
            return Some(TExpr::new(amb.ty(), span, TExprKind::Ambient(amb)));
        }
        // The same sentence the bare ambient gets, because it is the same
        // mistake: a `vertex` block is what makes an L4 per element, and a ray
        // through a fragment is not something a sprite has.
        if matches!(amb, Ambient::Eye | Ambient::Ray) && !self.fullscreen {
            self.err_hint(
                Stage::Contract,
                span,
                format!(
                    "`{slot}.{name}` is only available to a procedure that draws the whole frame"
                ),
                format!(
                    "a `vertex` block is what makes an L4 per-element, and a ray through a \
                     fragment is not something a sprite has. Remove the `vertex` block to \
                     march, or project with `{slot}.clip` instead"
                ),
            );
            return None;
        }
        self.err_hint(
            Stage::Contract,
            span,
            format!("`{slot}.{name}` is not available in this block"),
            format!(
                "`{slot}.eye` and `{slot}.ray` are the fragment stage's — they are built by \
                 the ray prologue it opens with, and a vertex stage has no fragment to send \
                 one through"
            ),
        );
        None
    }

    fn check_swizzle(&mut self, value: &Expr, components: &str, span: Span) -> Option<TExpr> {
        // **`far.position` is not a swizzle**, and it arrives here because it
        // is *shaped* like one — `expr . ident` is the grammar, and the parser
        // is right not to decide which it is. Deciding here costs no new
        // syntactic category, which is the whole reason a read of the second
        // geometry is spelled this way: `uses` adds one header declaration and
        // one name, and nothing else in the language moves.
        //
        // **The base name is the procedure's own**, so what reaches here is a
        // comparison against what the header declared rather than against a
        // reserved word. That is the difference the whole notation is: a file
        // that had to spell it `other` could only ever have one.
        if let Expr::Ident { name, .. } = value {
            if self.uses == Some(name.as_str()) {
                return self.check_far(name, components, span);
            }
            // **The same shape a third time, resolved a second way.** A
            // geometry slot's members are attributes and a camera slot's are
            // not, so this cannot go through `check_far`: what decides which
            // members exist is the *type* the header declared, which is the
            // whole of what the type on a slot is for.
            if self.camera == Some(name.as_str()) {
                return self.check_camera_member(name, components, span);
            }
        }
        let v = self.check_expr(value)?;
        let width = match v.ty {
            Ty::Vec2 => 2u8,
            Ty::Vec3 => 3,
            Ty::Vec4 => 4,
            other => {
                self.err(
                    Stage::Type,
                    span,
                    format!(
                        "cannot swizzle `{}`; only vectors have components",
                        other.name()
                    ),
                );
                return None;
            }
        };
        if components.is_empty() || components.len() > 4 {
            self.err(
                Stage::Type,
                span,
                format!(
                    "swizzle must have 1 to 4 components, found {}",
                    components.len()
                ),
            );
            return None;
        }

        let mut idx = Vec::with_capacity(components.len());
        let mut ok = true;
        for c in components.chars() {
            let i = match c {
                'x' => 0u8,
                'y' => 1,
                'z' => 2,
                'w' => 3,
                _ => {
                    self.err(
                        Stage::Type,
                        span,
                        format!(
                            "`{c}` is not a valid swizzle component; use `x`, `y`, `z`, or `w`"
                        ),
                    );
                    ok = false;
                    continue;
                }
            };
            if i >= width {
                self.err_hint(
                    Stage::Type,
                    span,
                    format!("`.{c}` is out of range for a {width}-component vector"),
                    format!(
                        "`{}` only has components `{}`",
                        v.ty.name(),
                        &"xyzw"[..width as usize]
                    ),
                );
                ok = false;
                continue;
            }
            idx.push(i);
        }
        if !ok {
            return None;
        }

        let ty = match idx.len() {
            1 => Ty::Float,
            2 => Ty::Vec2,
            3 => Ty::Vec3,
            4 => Ty::Vec4,
            _ => unreachable!("checked above"),
        };
        Some(TExpr::new(
            ty,
            span,
            TExprKind::Swizzle {
                value: Box::new(v),
                components: idx,
            },
        ))
    }
}

pub(crate) fn domain_allows(d: Domain, ty: Ty) -> bool {
    match d {
        Domain::None => false,
        Domain::FloatOrVector => matches!(ty, Ty::Float | Ty::Vec2 | Ty::Vec3 | Ty::Vec4),
        Domain::VectorOnly => matches!(ty, Ty::Vec2 | Ty::Vec3 | Ty::Vec4),
    }
}

pub(crate) fn op_symbol(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::And => "&&",
        BinOp::Or => "||",
    }
}

/// A well-typed placeholder used only for continued error recovery. The tree it
/// lands in is discarded whenever any error was recorded — `check` never
/// returns `Ok` otherwise — so this only has to keep `check_stmts` from
/// panicking, not represent anything meaningful.
pub(crate) fn poison(ty: Ty, span: Span) -> TExpr {
    match ty {
        Ty::Float => TExpr::new(ty, span, TExprKind::Lit(Lit::Float(0.0))),
        Ty::Int => TExpr::new(ty, span, TExprKind::Lit(Lit::Int(0))),
        Ty::Uint => TExpr::new(ty, span, TExprKind::Lit(Lit::Uint(0))),
        Ty::Bool => TExpr::new(ty, span, TExprKind::Lit(Lit::Bool(false))),
        Ty::Vec2 | Ty::Vec3 | Ty::Vec4 => {
            let zero = TExpr::new(Ty::Float, span, TExprKind::Lit(Lit::Float(0.0)));
            TExpr::new(ty, span, TExprKind::Construct { args: vec![zero] })
        }
        Ty::Mat3 | Ty::Mat4 => TExpr::new(ty, span, TExprKind::Lit(Lit::Float(0.0))),
    }
}
