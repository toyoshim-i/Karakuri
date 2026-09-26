//! Expression evaluation, identifier resolution, operators, and swizzling.

use super::*;

impl<'a> Checker<'a> {
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

    pub(super) fn check_ident(&mut self, name: &str, span: Span) -> Option<TExpr> {
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
                // A `spawn` block reads only what this procedure emits.
                Some(BlockKind::Field) => false,
                Some(BlockKind::Spawn) => self.emit.contains(&attr),
                Some(BlockKind::Element) => {
                    self.emit.contains(&attr) || self.consumes.contains(&attr)
                }
                // A `deform` block reads upstream consumed attributes and emitted attributes.
                Some(BlockKind::Deform) => {
                    self.emit.contains(&attr) || self.consumes.contains(&attr)
                }
                // A `mask` block reads consumed upstream attributes only.
                Some(BlockKind::Mask) => self.consumes.contains(&attr),
                Some(BlockKind::Vertex) | Some(BlockKind::Fragment) => {
                    self.consumes.contains(&attr)
                }
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
            // `source` is ambiguous in a procedure that declares a paired geometry slot.
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
            // Dedicated errors for fullscreen passes accessing per-element ambients.
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
            // Diagnostically distinct error hints for L5 ambient restrictions.
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
        // A geometry slot name cannot be evaluated as an rvalue.
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
        // Field slots cannot be evaluated as bare values without spatial coordinates.
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
        // A texture slot name cannot be evaluated as an rvalue.
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
        // `held` requires `retains` in frame blocks.
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
        // A Source slot evaluates to the geometry instance's uint identifier.
        if self.sources.contains(&name) {
            return Some(TExpr::new(
                Ty::Uint,
                span,
                TExprKind::Source {
                    slot: name.to_string(),
                },
            ));
        }
        // Camera slots cannot be evaluated as bare values; member access is required.
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

        // Unresolved identifier: signal bus is not directly readable from IR.
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

    pub(super) fn desugar_compound(
        &mut self,
        op: BinOp,
        lhs: TExpr,
        rhs: TExpr,
        span: Span,
    ) -> Option<TExpr> {
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

    pub(super) fn check_swizzle(
        &mut self,
        value: &Expr,
        components: &str,
        span: Span,
    ) -> Option<TExpr> {
        // Delegate slot member access (`<geometry>.<attr>` or `<camera>.<member>`).
        if let Expr::Ident { name, .. } = value {
            if self.uses == Some(name.as_str()) {
                return self.check_far(name, components, span);
            }
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
