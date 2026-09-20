//! Call resolution, builtins, constructors, texture sampling, and slot access.

use super::*;

impl<'a> Checker<'a> {
    /// What the header declared, before the table the language ships.
    ///
    /// A field is reached through a slot, so the name at a call site is the
    /// procedure's own — and this branch is the whole of that. It is first for the
    /// reason it is a branch at all: a call resolves against what the file said it
    /// takes, and asking the builtin table first would make the language's
    /// vocabulary quietly outrank the header. Nothing is hidden by the order, since
    /// a slot named after a builtin is refused where it is declared — see
    /// `check_slot_names`.
    pub(super) fn check_call(&mut self, name: &str, args: &[Expr], span: Span) -> Option<TExpr> {
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
    pub(super) fn check_far(&mut self, slot: &str, name: &str, span: Span) -> Option<TExpr> {
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
    pub(super) fn check_camera_member(
        &mut self,
        slot: &str,
        name: &str,
        span: Span,
    ) -> Option<TExpr> {
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
}
