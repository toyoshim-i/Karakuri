//! Statement type-checking and validation.

use super::*;

impl<'a> Checker<'a> {
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
        // Prevent shadowing reserved frame textures `src` and `held`.
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
                // Field blocks have no element attributes.
                Some(BlockKind::Field) => false,
                Some(BlockKind::Spawn) | Some(BlockKind::Element) => self.emit.contains(&attr),
                // A `deform` block may rewrite consumed attributes or write emitted ones.
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
        // In a `camera` block, `eye` resolves as output rather than ambient.
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
            // Validate that the output belongs to the current block.
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
        // Provide migration hint for legacy point size spelling.
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
            // L2 deforms do not compact or remove elements; kill is restricted to L1 element blocks.
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
}
