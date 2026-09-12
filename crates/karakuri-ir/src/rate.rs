//! How small a primitive this renderer can draw, read off the expression rather
//! than measured.
//!
//! [`Output::PointRate`] is a fraction of the render target's height, so a
//! primitive is `rate × height` pixels across and stops being a pixel at `1 /
//! rate` rows.
//! `docs/adr/0245-the-sub-pixel-compensation-is-paid-in-the-colour-because-alpha-is-coverage.md`
//! says what happens under that height — the primitive is drawn at one pixel
//! and dimmed in the colour — which is a different picture rather than a
//! smaller one, and is why anything extrapolating a cost from a small draw has
//! to know the height it stops being allowed to go below.
//!
//! The rate is a per-element vertex expression. Nothing outside the shader
//! evaluates it, and nothing can enumerate the elements it would be evaluated
//! over. What *can* be done is to bound it: every leaf is a literal, a param
//! with a declared range, an integer whose width is a range, or something with
//! no range at all — and interval arithmetic over those carries a bound to the
//! root.
//!
//! ## Which end, and why it is that end
//!
//! The smallest rate the procedure can emit is what is wanted, so this computes
//! a *lower* bound and an under-estimate is the safe error. The floor is `1 /
//! rate`: a bound *below* the true smallest rate yields a floor *above* the
//! true floor, which places a probe's rungs higher than they had to be and at
//! worst refuses to place them at all. A bound *above* it yields a floor
//! *below* the true one, which certifies a draw whose primitives are being
//! rounded up to a pixel — the reading that makes a cost model under-state,
//! which is the one direction ADR-0245's consumer forbids. So every rule below
//! rounds outward, and anything it cannot bound is [`Bound::Unbounded`] rather
//! than a guess.
//!
//! The upper end is computed and is not published. It exists because interval
//! arithmetic needs both ends to carry either — `1 - x` needs `x`'s upper end
//! to bound its lower one — and it is not offered because nothing asks how
//! *large* a primitive can be.
//!
//! ## What a param contributes
//!
//! The declared range, never the value the Set is holding. A param is a fader's
//! range, an agent's search range and a signal's normalisation basis
//! ([`crate::ast::Param`]), so its value moves while the material is on air,
//! and a floor read off the value would be wrong the moment somebody turned a
//! knob. The declared range is a property of the *file*, so a bound taken from
//! it holds for as long as that file is the material.
//!
//! Nothing clamps a write to the declared range — `karakuri_engine::Set`'s
//! `carry_moved_from` says so outright — so a value written outside a
//! declaration is outside what this bound covers. [`Bound::AtLeast::over`]
//! names every declaration the bound leaned on, which is what lets a caller
//! that can see the held values check them rather than assume them.
//!
//! ## What it does not attempt
//!
//! Correlation. Two reads of one param are two intervals here, so `p - p`
//! bounds to `[min - max, max - min]` rather than to zero. That is sound in
//! this direction and loses nothing the corpus needed.
//!
//! Attributes, ambients and field evaluations have no declared range and are
//! [`Range::WHOLE`] — except at the integer widths, where the *type* is a range
//! and `float(source % 3u) + 1.0` bounds because of it. `max(size, 1.0)` and
//! `clamp(x, 0.001, 0.03)` are how the shipped material gets a bound anyway,
//! and that is not a coincidence: an author who wants a primitive that stays
//! visible writes the same guard.
//!
//! A matrix product, which is a sum of products rather than a product and would
//! need a rule of its own. Nothing worth bounding is written through one.
//!
//! A second pass over a loop. A local a loop body assigns to is unknown for the
//! whole body, because the value it holds at the top of an iteration is the one
//! the iteration before left and this walks the body once.

use std::collections::{BTreeSet, HashMap};

use crate::ast::{BinOp, BlockKind, Lit, Output, Ty, UnOp};
use crate::builtin::Builtin;
use crate::span::Span;
use crate::typed::{Checked, TExpr, TExprKind, TStmt, Target};

/// What can be proved about the smallest `point_rate` a procedure emits, and
/// what that proof rests on.
///
/// Carried with the procedure's name because a Set draws through several
/// renderers and its floor is the least of theirs: a bound with no name on it
/// cannot say which file is holding the floor down. That is `P-0095` at the
/// same remove `karakuri_engine::estimate::Estimate` reads it at — a number
/// travels with what it is a number *of*.
#[derive(Debug, Clone, PartialEq)]
pub struct RateBound {
    /// The procedure the vertex block was read from.
    pub procedure: String,
    /// What was proved.
    pub bound: Bound,
}

/// The three answers [`point_rate_bound`] can give.
#[derive(Debug, Clone, PartialEq)]
pub enum Bound {
    /// The procedure has no `vertex` block, so it emits no rate and draws no
    /// primitive — [`crate::ast::Topology::Fullscreen`], which [`crate::check`]
    /// infers from exactly this absence.
    ///
    /// Not a bound of zero and not a refusal: there is nothing that can fall under
    /// a pixel, so no height is out of bounds for it.
    NoPrimitive,
    /// Every rate this vertex block can emit is at least `rate`, over every value
    /// its params can take inside their declarations and over every value of
    /// everything the analysis could not see.
    AtLeast {
        /// Finite and greater than zero, always. A bound of zero is no bound — there is
        /// no height at which a zero-rate primitive is a pixel across — so it is
        /// reported as [`Bound::Unbounded`] instead.
        rate: f32,
        /// Every param declaration the bound leaned on, by name, sorted. Empty where
        /// the rate is a constant expression.
        ///
        /// This is the *what it is a bound of*: the bound holds while these
        /// declarations hold, and nothing in the engine clamps a write to a declared
        /// range. A caller holding the values can check them; a caller that cannot is
        /// reading a bound over the file rather than over the run, and this list is how
        /// it knows the difference.
        over: Vec<String>,
    },
    /// No positive lower bound could be proved, and where it was lost.
    ///
    /// A refusal rather than a degraded answer: a floor guessed too low certifies a
    /// draw whose primitives are floored, which makes a cost model under-state —
    /// see the module doc on which end is safe.
    Unbounded {
        /// The `point_rate` assignment that defeated the analysis. Where a vertex block
        /// assigns on several paths this is the first one that could not be bounded,
        /// which is not necessarily the smallest.
        at: Span,
        /// What was proved there anyway, so a reader can tell the two failures apart: a
        /// finite number at or below zero is an expression that genuinely reaches zero,
        /// and `f32::NEG_INFINITY` is one nothing bounded below at all.
        lower: f32,
    },
}

impl RateBound {
    /// The proved rate, or [`None`] where there is no primitive or no bound.
    ///
    /// A convenience for a caller that has already decided what it does about the
    /// other two, and never a way to skip that decision: [`Bound::NoPrimitive`] and
    /// [`Bound::Unbounded`] mean opposite things and both answer `None`.
    pub fn rate(&self) -> Option<f32> {
        match self.bound {
            Bound::AtLeast { rate, .. } => Some(rate),
            _ => None,
        }
    }
}

/// Bound the smallest `point_rate` `proc` can emit, from its vertex block and
/// its params' declared ranges.
///
/// Pure — no device, no Set, no values. The answer is a property of the file,
/// which is what makes it usable before anything is drawn.
///
/// Asked of an L4, which is the only kind with a `vertex` block to write a rate
/// in. Any other kind answers [`Bound::NoPrimitive`] by the same route a
/// fullscreen renderer does, and truthfully: it draws no primitive either.
///
/// Over the whole declared range of every param. That is the bound over the
/// *file*, and it is the widest one there is. A caller holding the state a Set
/// is actually in wants [`point_rate_bound_at`] — see there for why the state
/// and not the file is what
/// `docs/adr/0282-a-rebuild-inherits-the-values-somebody-moved-and-reads-the-rest-from-the-code.md`
/// makes the reading.
pub fn point_rate_bound(proc: &Checked) -> RateBound {
    point_rate_bound_at(proc, &HashMap::new())
}

/// [`point_rate_bound`], with named params pinned to the values they are
/// holding.
///
/// `held` is read by the param's bare declared name; a name it does not carry
/// keeps its declared range, and a name that is not a param of `proc` is
/// ignored. A pinned param is the single point `[v, v]`, so the interval that
/// comes back is over *this* state rather than over every state the file
/// permits.
///
/// Which is the reading ADR-0282 settled. A declared value is the value in the
/// untouched state — what the code says when nobody has moved anything — and
/// where somebody moved one, the held value *is* the value. So a caller holding
/// a Set's values passes all of them and gets the bound over the state as it
/// stands; the whole-range bound above is what is left when nothing is known
/// about the state at all.
///
/// It goes stale when a fader moves, and that is the design. The narrower bound
/// is a claim about a state, so a write invalidates it and the estimate
/// standing on it is taken again — rather than the wider bound's bargain, which
/// is to stay true by covering states nobody is in.
///
/// A vector param is not pinned. The analysis carries one interval per
/// declaration and a Set holds a value per *component key*, so pinning `glow`
/// from `glow.x` would be claiming a bound over a value this map does not name.
/// Nothing shipped writes a rate through a vector param; a caller that does
/// gets the declared range and no worse.
pub fn point_rate_bound_at(proc: &Checked, held: &HashMap<String, f32>) -> RateBound {
    let bound = match proc.block(BlockKind::Vertex) {
        // A procedure with no vertex block emits no rate. `check` infers
        // `Fullscreen` from the same absence, so the two facts are one fact.
        None => Bound::NoPrimitive,
        Some(block) => {
            let mut a = Analysis {
                params: proc
                    .params
                    .iter()
                    .map(|p| {
                        let declared =
                            Range::new(f64::from(p.min.min(p.max)), f64::from(p.max.max(p.min)));
                        let range = match held.get(&p.name) {
                            Some(v) if v.is_finite() => Range::exact(f64::from(*v)),
                            _ => declared,
                        };
                        (p.name.as_str(), range)
                    })
                    .collect(),
                env: HashMap::new(),
                emitted: Vec::new(),
            };
            a.stmts(&block.stmts);
            settle(&a.emitted, block.span)
        }
    };
    RateBound {
        procedure: proc.name.clone(),
        bound,
    }
}

/// The least of what every assignment can emit, or the first one that could not
/// be bounded.
///
/// Every assignment counts, whatever path it is on. Which branch a given
/// element takes is not decidable here and does not need to be: the emitted
/// rate is one of these, so the least of their lower bounds is a lower bound on
/// all of them.
fn settle(emitted: &[(Span, Value)], block: Span) -> Bound {
    if emitted.is_empty() {
        // `check` requires `point_rate` on every path of a vertex block, so a
        // checked procedure cannot reach this. Refused rather than asserted:
        // an analysis that found no assignment has not proved a floor, and
        // that is the whole of what it is asked.
        return Bound::Unbounded {
            at: block,
            lower: f32::NEG_INFINITY,
        };
    }
    let mut least = f64::INFINITY;
    let mut over = BTreeSet::new();
    for (at, value) in emitted {
        if !value.range.lo.is_finite() || value.range.lo <= 0.0 {
            return Bound::Unbounded {
                at: *at,
                lower: value.range.lo as f32,
            };
        }
        least = least.min(value.range.lo);
        over.extend(value.over.iter().cloned());
    }
    // **Rounded down into `f32`, never across.** The bound is about to be
    // handed to a reciprocal, and a value rounded *up* shortens the floor —
    // the one direction this whole module refuses. `as f32` rounds to nearest,
    // so a result that landed above the `f64` answer is stepped back one
    // representable value, which for a positive float is one off the bit
    // pattern.
    let mut rate = least as f32;
    if f64::from(rate) > least {
        rate = f32::from_bits(rate.to_bits() - 1);
    }
    if !rate.is_finite() || rate <= 0.0 {
        return Bound::Unbounded {
            at: emitted[0].0,
            lower: rate,
        };
    }
    Bound::AtLeast {
        rate,
        over: over.into_iter().collect(),
    }
}

/// A closed interval containing every component of a value, with either end
/// allowed to be infinite.
///
/// One interval for a `vec3` rather than three, because every consumer here
/// wants a scalar in the end and a swizzle out of a vector must not be able to
/// escape the bound its source carried.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub lo: f64,
    pub hi: f64,
}

impl Range {
    /// Nothing is known.
    pub const WHOLE: Range = Range {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
    };

    /// The interval, or [`Range::WHOLE`] where the arithmetic that produced it did
    /// not yield one — a `NaN` end, or an end pair that crossed.
    ///
    /// Every construction goes through here, so an operation that overflows into
    /// `NaN` degrades to "not known" rather than propagating a comparison that is
    /// false both ways.
    pub fn new(lo: f64, hi: f64) -> Range {
        if lo.is_nan() || hi.is_nan() || lo > hi {
            Range::WHOLE
        } else {
            Range { lo, hi }
        }
    }

    fn exact(v: f64) -> Range {
        Range::new(v, v)
    }

    /// What a value of `ty` is known to be worth with nothing else said about it.
    /// The integer widths are the only real answers: a `uint` is non-negative and
    /// bounded by construction, which is the whole of why `float(source % 3u) +
    /// 1.0` has a floor.
    fn of(ty: Ty) -> Range {
        match ty {
            Ty::Int => Range::new(f64::from(i32::MIN), f64::from(i32::MAX)),
            Ty::Uint => Range::new(0.0, f64::from(u32::MAX)),
            Ty::Bool => Range::new(0.0, 1.0),
            Ty::Float | Ty::Vec2 | Ty::Vec3 | Ty::Vec4 | Ty::Mat3 | Ty::Mat4 => Range::WHOLE,
        }
    }

    fn join(self, o: Range) -> Range {
        Range::new(self.lo.min(o.lo), self.hi.max(o.hi))
    }

    fn add(self, o: Range) -> Range {
        Range::new(self.lo + o.lo, self.hi + o.hi)
    }

    fn neg(self) -> Range {
        Range::new(-self.hi, -self.lo)
    }

    fn sub(self, o: Range) -> Range {
        self.add(o.neg())
    }

    fn mul(self, o: Range) -> Range {
        let c = [
            times(self.lo, o.lo),
            times(self.lo, o.hi),
            times(self.hi, o.lo),
            times(self.hi, o.hi),
        ];
        Range::new(least(&c), most(&c))
    }

    fn div(self, o: Range) -> Range {
        // A divisor whose interval touches zero divides by zero somewhere in
        // it, and the quotient is then unbounded in at least one direction.
        if o.lo <= 0.0 && o.hi >= 0.0 {
            return Range::WHOLE;
        }
        self.mul(Range::new(1.0 / o.hi, 1.0 / o.lo))
    }

    /// `%`, at whichever family `ty` is.
    ///
    /// `float` takes the sign of the divisor — the IR's `%` lowers to `a - b *
    /// floor(a / b)`, which is `mod` and not WGSL's remainder — so a positive
    /// divisor makes the result non-negative whatever the dividend is. The integer
    /// widths keep the native remainder, whose sign is the *dividend*'s, so a
    /// `uint` is non-negative and an `int` is not.
    fn rem(self, o: Range, ty: Ty) -> Range {
        let magnitude = o.lo.abs().max(o.hi.abs());
        if !magnitude.is_finite() || magnitude == 0.0 {
            return Range::WHOLE;
        }
        match ty {
            Ty::Int | Ty::Uint => {
                // `|a % b| <= |b| - 1` on integers, and the sign follows `a`.
                if o.lo <= 0.0 && o.hi >= 0.0 {
                    return Range::WHOLE;
                }
                let top = magnitude - 1.0;
                Range::new(if self.lo >= 0.0 { 0.0 } else { -top }, top)
            }
            _ => {
                if o.lo > 0.0 {
                    Range::new(0.0, o.hi)
                } else if o.hi < 0.0 {
                    Range::new(o.lo, 0.0)
                } else {
                    Range::new(-magnitude, magnitude)
                }
            }
        }
    }

    fn min(self, o: Range) -> Range {
        Range::new(self.lo.min(o.lo), self.hi.min(o.hi))
    }

    fn max(self, o: Range) -> Range {
        Range::new(self.lo.max(o.lo), self.hi.max(o.hi))
    }

    fn abs(self) -> Range {
        if self.lo >= 0.0 {
            self
        } else if self.hi <= 0.0 {
            self.neg()
        } else {
            Range::new(0.0, self.lo.abs().max(self.hi.abs()))
        }
    }

    /// `f` applied at both ends, for an `f` that never decreases.
    fn monotone(self, f: impl Fn(f64) -> f64) -> Range {
        Range::new(f(self.lo), f(self.hi))
    }
}

/// One end times another, with `0 × ∞` read as `0`.
///
/// The convention is what makes the products of an interval touching zero and
/// one reaching infinity come out right: `[0, 5] × [1, ∞)` is `[0, ∞)`, and
/// `NaN` would have thrown the whole bound away.
fn times(a: f64, b: f64) -> f64 {
    if a == 0.0 || b == 0.0 {
        0.0
    } else {
        a * b
    }
}

fn least(xs: &[f64]) -> f64 {
    xs.iter().copied().fold(f64::INFINITY, f64::min)
}

fn most(xs: &[f64]) -> f64 {
    xs.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

/// An interval and which declarations it rests on.
///
/// The two travel together because a bound taken over a param's declaration is
/// only worth what the declaration is worth, and a caller cannot check a
/// declaration it was not told about.
#[derive(Debug, Clone, PartialEq)]
struct Value {
    range: Range,
    over: BTreeSet<String>,
}

impl Value {
    fn of(range: Range) -> Value {
        Value {
            range,
            over: BTreeSet::new(),
        }
    }

    fn join(&self, o: &Value) -> Value {
        Value {
            range: self.range.join(o.range),
            over: self.over.union(&o.over).cloned().collect(),
        }
    }
}

struct Analysis<'a> {
    params: HashMap<&'a str, Range>,
    env: HashMap<String, Value>,
    /// Every `point_rate` assignment reached, in source order.
    emitted: Vec<(Span, Value)>,
}

impl Analysis<'_> {
    fn stmts(&mut self, stmts: &[TStmt]) {
        for stmt in stmts {
            self.stmt(stmt);
        }
    }

    fn stmt(&mut self, stmt: &TStmt) {
        match stmt {
            TStmt::Let { name, value, .. } | TStmt::Var { name, value, .. } => {
                let v = self.eval(value);
                self.env.insert(name.clone(), v);
            }
            TStmt::Assign { target, value, .. } => {
                let v = self.eval(value);
                match target {
                    Target::Local(name) => {
                        self.env.insert(name.clone(), v);
                    }
                    Target::Output(Output::PointRate) => {
                        self.emitted.push((value.span, v));
                    }
                    // Everything else a vertex block writes is somebody else's
                    // question: `clip` and `clip_b` decide where the primitive
                    // is, not how big it is.
                    Target::Output(_) | Target::Attr(_) => {}
                }
            }
            TStmt::If { then, els, .. } => {
                // **Both arms are walked and their environments joined**, and
                // the condition is not read at all: which arm an element takes
                // is per-element and undecidable here, so a `point_rate`
                // written in either is one this Set can emit.
                let saved = self.env.clone();
                self.stmts(then);
                let after_then = std::mem::replace(&mut self.env, saved.clone());
                self.stmts(els);
                let after_else = std::mem::take(&mut self.env);
                // Only the names that existed before the branch survive it: a
                // local declared inside an arm is scoped to that arm.
                self.env = saved
                    .into_keys()
                    .map(|name| {
                        let v = match (after_then.get(&name), after_else.get(&name)) {
                            (Some(a), Some(b)) => a.join(b),
                            (Some(a), None) | (None, Some(a)) => a.clone(),
                            (None, None) => Value::of(Range::WHOLE),
                        };
                        (name, v)
                    })
                    .collect();
            }
            TStmt::For {
                var,
                start,
                end,
                body,
                ..
            } => {
                // **Anything the body assigns is unknown for the whole of it.**
                // The value a local holds at the top of iteration `n` is the one
                // iteration `n-1` left, and this pass walks the body once —
                // so the only sound reading of a mutated local is that it could
                // be anything. A local *declared* in the body is rebound on
                // every pass and keeps its real interval.
                let saved = self.env.clone();
                for name in assigned(body) {
                    self.env.insert(name, Value::of(Range::WHOLE));
                }
                self.env.insert(
                    var.clone(),
                    Value::of(Range::new(f64::from(*start), f64::from(*end) - 1.0)),
                );
                self.stmts(body);
                let after = std::mem::take(&mut self.env);
                // **Joined with the state before the loop**, because the bounds
                // are constants and `start >= end` is a body that never runs.
                self.env = saved
                    .into_iter()
                    .map(|(name, before)| {
                        let v = match after.get(&name) {
                            Some(a) => a.join(&before),
                            None => before,
                        };
                        (name, v)
                    })
                    .collect();
            }
            TStmt::Kill { .. } => {}
        }
    }

    fn eval(&self, e: &TExpr) -> Value {
        match &e.kind {
            TExprKind::Lit(v) => Value::of(Range::exact(match v {
                Lit::Float(x) => f64::from(*x),
                Lit::Int(x) => f64::from(*x),
                Lit::Uint(x) => f64::from(*x),
                Lit::Bool(x) => f64::from(u8::from(*x)),
            })),
            TExprKind::Local(name) => self
                .env
                .get(name)
                .cloned()
                .unwrap_or_else(|| Value::of(Range::of(e.ty))),
            TExprKind::Param(name) => Value {
                range: self
                    .params
                    .get(name.as_str())
                    .copied()
                    .unwrap_or_else(|| Range::of(e.ty)),
                over: BTreeSet::from([name.clone()]),
            },
            // **No declared range anywhere in here.** An attribute is whatever
            // the simulation left in it, an ambient is the frame's, and a field
            // is another file's arithmetic. `Range::of` still answers for the
            // integer widths, which is not nothing: a `Source` identity is a
            // `uint`, so `source % 3u` is bounded and `float` of it stays that
            // way.
            // **And a texture fetch is whatever was drawn into it**, which is
            // the frame's own arithmetic several layers up and unbounded in
            // linear HDR by construction — the pipeline runs unclamped and a
            // texel above 1.0 is light the display cannot show rather than a
            // mistake.
            TExprKind::Attr(_)
            | TExprKind::Far(_)
            | TExprKind::Ambient(_)
            | TExprKind::Field { .. }
            | TExprKind::Sample { .. }
            | TExprKind::Source { .. } => Value::of(Range::of(e.ty)),
            TExprKind::Unary { op, value } => {
                let v = self.eval(value);
                Value {
                    range: match op {
                        UnOp::Neg => v.range.neg(),
                        UnOp::Not => Range::new(0.0, 1.0),
                    },
                    over: v.over,
                }
            }
            TExprKind::Binary { op, lhs, rhs } => {
                let (a, b) = (self.eval(lhs), self.eval(rhs));
                let range = match op {
                    BinOp::Add => a.range.add(b.range),
                    BinOp::Sub => a.range.sub(b.range),
                    // **A matrix product is a sum of products and not a
                    // product**, so an interval over the operands' components
                    // does not bound the result's. `camera * vec4(position,
                    // 1.0)` is the only shape the corpus writes and its matrix
                    // is an ambient with no range anyway, so nothing is lost by
                    // giving the whole line rather than a rule for it.
                    BinOp::Mul if matches!(lhs.ty, Ty::Mat3 | Ty::Mat4) => Range::WHOLE,
                    BinOp::Mul if matches!(rhs.ty, Ty::Mat3 | Ty::Mat4) => Range::WHOLE,
                    BinOp::Mul => a.range.mul(b.range),
                    BinOp::Div => a.range.div(b.range),
                    BinOp::Rem => a.range.rem(b.range, lhs.ty),
                    // A comparison or a connective is a `bool`, and the
                    // language has no implicit conversion that would let one
                    // reach arithmetic.
                    _ => Range::new(0.0, 1.0),
                };
                Value {
                    range,
                    over: a.over.union(&b.over).cloned().collect(),
                }
            }
            TExprKind::Builtin { func, args } => {
                let args: Vec<Value> = args.iter().map(|a| self.eval(a)).collect();
                let ranges: Vec<Range> = args.iter().map(|a| a.range).collect();
                Value {
                    range: builtin_range(*func, &ranges),
                    over: args.into_iter().flat_map(|a| a.over).collect(),
                }
            }
            // `vec3(a, b, c)` is bounded by the hull of its arguments, which is
            // what makes a swizzle out of it safe to bound by the same
            // interval. `float(i)` is the same rule at width one.
            TExprKind::Construct { args } => {
                let args: Vec<Value> = args.iter().map(|a| self.eval(a)).collect();
                let hull = args
                    .iter()
                    .map(|a| a.range)
                    .reduce(Range::join)
                    .unwrap_or(Range::WHOLE);
                Value {
                    range: convert(hull, e.ty),
                    over: args.into_iter().flat_map(|a| a.over).collect(),
                }
            }
            // The source's interval bounds every one of its components, so it
            // bounds any selection of them.
            TExprKind::Swizzle { value, .. } => self.eval(value),
        }
    }
}

/// What a constructor does to the interval it was handed.
///
/// Only the narrowing conversions change it: `int` and `uint` truncate toward
/// zero, which never decreases, so both ends move under the same function. A
/// negative float converted to `uint` is not defined by WGSL, and the whole
/// interval is given up rather than half of it kept.
fn convert(from: Range, to: Ty) -> Range {
    match to {
        Ty::Int => from.monotone(f64::trunc),
        Ty::Uint => {
            if from.lo < 0.0 {
                Range::of(Ty::Uint)
            } else {
                from.monotone(f64::trunc)
            }
        }
        _ => from,
    }
}

/// What a builtin's result is worth, given its arguments'.
///
/// Everything not named here is [`Range::WHOLE`], which is the honest answer
/// for a function whose range this file has no record of. The ones that are
/// named are named from a record: `docs/ir-spec.md`'s built-in table states
/// `hash1` at `0..1` and `value_noise` at `-1..1`, and the rest are the
/// definitions of the functions themselves.
///
/// `perlin`, `simplex`, `fbm` and `curl` are deliberately absent. They are
/// bounded in practice and the specification does not say by what, and a bound
/// this file invented would be a claim no other file is holding to.
fn builtin_range(func: Builtin, args: &[Range]) -> Range {
    let arg = |i: usize| args.get(i).copied().unwrap_or(Range::WHOLE);
    match func {
        Builtin::Abs => arg(0).abs(),
        Builtin::Floor => arg(0).monotone(f64::floor),
        Builtin::Ceil => arg(0).monotone(f64::ceil),
        Builtin::Round => arg(0).monotone(f64::round),
        // `x - floor(x)`, so `[0, 1)` — closed at 1 here, which is the outward
        // rounding this module owes.
        Builtin::Fract => Range::new(0.0, 1.0),
        Builtin::Mod => arg(0).rem(arg(1), Ty::Float),
        Builtin::Min => arg(0).min(arg(1)),
        Builtin::Max => arg(0).max(arg(1)),
        // `clamp(x, lo, hi)` is `min(max(x, lo), hi)`, and composing it that
        // way is what makes the constant-bounded form answer: the result is
        // inside `[lo, hi]` however little is known about `x`.
        Builtin::Clamp => arg(0).max(arg(1)).min(arg(2)),
        // `a + (b - a) * t`, which is the definition and needs no rule of its
        // own — an unbounded `t` correctly gives an unbounded result.
        Builtin::Mix => arg(0).add(arg(1).sub(arg(0)).mul(arg(2))),
        Builtin::Step => Range::new(0.0, 1.0),
        Builtin::Smoothstep => Range::new(0.0, 1.0),
        Builtin::Sign => Range::new(-1.0, 1.0),
        Builtin::Sqrt => {
            let x = arg(0);
            if x.lo >= 0.0 {
                x.monotone(f64::sqrt)
            } else {
                Range::WHOLE
            }
        }
        Builtin::Pow => power(arg(0), arg(1)),
        Builtin::Exp => arg(0).monotone(f64::exp),
        Builtin::Exp2 => arg(0).monotone(f64::exp2),
        Builtin::Log | Builtin::Log2 => {
            let x = arg(0);
            if x.lo > 0.0 {
                x.monotone(if func == Builtin::Log {
                    f64::ln
                } else {
                    f64::log2
                })
            } else {
                Range::WHOLE
            }
        }
        Builtin::Sin | Builtin::Cos => Range::new(-1.0, 1.0),
        Builtin::Asin => Range::new(-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2),
        Builtin::Acos => Range::new(0.0, std::f64::consts::PI),
        Builtin::Atan => Range::new(-std::f64::consts::FRAC_PI_2, std::f64::consts::FRAC_PI_2),
        Builtin::Atan2 => Range::new(-std::f64::consts::PI, std::f64::consts::PI),
        // A norm is non-negative, whatever it is a norm of.
        Builtin::Length | Builtin::Distance => Range::new(0.0, f64::INFINITY),
        // Unit vectors, componentwise.
        Builtin::Normalize | Builtin::SpherePoint | Builtin::DiscPoint => Range::new(-1.0, 1.0),
        // `docs/ir-spec.md`, *Hash and noise*.
        Builtin::Hash1 | Builtin::Hash2 | Builtin::Hash3 => Range::new(0.0, 1.0),
        Builtin::ValueNoise => Range::new(-1.0, 1.0),
        _ => Range::WHOLE,
    }
}

/// `pow(x, y)`, at the corners.
///
/// `pow` is monotone in each argument separately — in `x` by the sign of `y`,
/// in `y` by whether `x` is above or below one — so over a box its extremes are
/// at the corners whenever the box stays in `x >= 0`, which is where WGSL
/// defines it at all. A negative base or an infinite end gives the whole line
/// rather than a corner that happens to evaluate.
fn power(x: Range, y: Range) -> Range {
    if x.lo < 0.0 || !x.hi.is_finite() || !y.lo.is_finite() || !y.hi.is_finite() {
        return Range::WHOLE;
    }
    let c = [
        x.lo.powf(y.lo),
        x.lo.powf(y.hi),
        x.hi.powf(y.lo),
        x.hi.powf(y.hi),
    ];
    if c.iter().any(|v| v.is_nan()) {
        return Range::WHOLE;
    }
    Range::new(least(&c), most(&c))
}

/// Every local a statement list assigns to, at any depth.
///
/// What a loop body does to a local it did not declare is the one thing this
/// pass cannot walk its way to, so it asks the question directly instead.
fn assigned(stmts: &[TStmt]) -> Vec<String> {
    let mut found = Vec::new();
    fn walk(stmts: &[TStmt], found: &mut Vec<String>) {
        for stmt in stmts {
            match stmt {
                TStmt::Assign {
                    target: Target::Local(name),
                    ..
                } => found.push(name.clone()),
                TStmt::If { then, els, .. } => {
                    walk(then, found);
                    walk(els, found);
                }
                TStmt::For { body, .. } => walk(body, found),
                _ => {}
            }
        }
    }
    walk(stmts, &mut found);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One L4 around `body`, checked, so every case below asks the analysis about a
    /// procedure the checker has already accepted — a `point_rate` expression that
    /// does not type is not a case this module owes an answer for.
    fn renderer(params: &str, body: &str) -> Checked {
        let src = source(params, body);
        let proc = crate::parse(&src).expect("parse");
        crate::check::check(&proc).expect("check")
    }

    fn source(params: &str, body: &str) -> String {
        format!(
            r#"
proc bounded {{
  kind  L4
  blend additive

  consumes position, size

{params}

  vertex {{
    let cp = camera * vec4(position, 1.0);
    clip = cp;
{body}
  }}

  fragment {{
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }}
}}
"#
        )
    }

    fn bound(params: &str, body: &str) -> Bound {
        point_rate_bound(&renderer(params, body)).bound
    }

    fn at_least(params: &str, body: &str) -> f32 {
        match bound(params, body) {
            Bound::AtLeast { rate, .. } => rate,
            other => panic!("expected a bound, got {other:?}"),
        }
    }

    fn over(params: &str, body: &str) -> Vec<String> {
        match bound(params, body) {
            Bound::AtLeast { over, .. } => over,
            other => panic!("expected a bound, got {other:?}"),
        }
    }

    /// A rate that is a literal is its own bound, exactly.
    #[test]
    fn a_constant_rate_bounds_to_itself() {
        assert_eq!(at_least("", "point_rate = 0.004;"), 0.004);
    }

    /// The declared minimum, not the default. A param is a fader's range, so the
    /// value the file opens at says nothing about where it will be a minute later —
    /// and a floor read off the default would be wrong the first time anybody
    /// turned the knob down.
    #[test]
    fn a_param_bounds_at_its_declaration_rather_than_its_default() {
        let rate = at_least(
            "  param point_scale : float [0.00069, 0.0556] = 0.00556",
            "point_rate = point_scale;",
        );
        assert_eq!(rate, 0.00069);
    }

    /// A `clamp` with constant ends bounds whatever it is given, which is how
    /// `sheet_shade` states a floor over a division by a clip-space `w` nothing
    /// here can bound.
    #[test]
    fn a_clamp_with_constant_ends_bounds_an_unbounded_expression() {
        let rate = at_least(
            "  param point_scale : float [0.00069, 0.0556] = 0.00417",
            "point_rate = clamp(point_scale * 4.0 / max(cp.w, 0.5), 0.00139, 0.0333);",
        );
        assert_eq!(rate, 0.00139);
    }

    /// `max` recovers a bound from an unbounded operand, which is what
    /// `star_flares` does with an attribute: `max(size, 1.0)` is at least one
    /// however the simulation left `size`.
    #[test]
    fn max_against_a_constant_recovers_a_bound_from_an_attribute() {
        let rate = at_least(
            "  param point_scale : float [0.0014, 0.0417] = 0.0076",
            "point_rate = point_scale * max(size, 1.0);",
        );
        assert_eq!(rate, 0.0014);
    }

    /// An attribute has no declared range, and the answer is a refusal. `hard_dots`
    /// ships exactly this — `dot_scale * size` — so this is the shipped case rather
    /// than a constructed one.
    #[test]
    fn an_attribute_with_nothing_guarding_it_is_refused() {
        match bound(
            "  param dot_scale : float [0.00069, 0.0833] = 0.0125",
            "point_rate = dot_scale * size;",
        ) {
            Bound::Unbounded { lower, .. } => {
                assert_eq!(lower, f32::NEG_INFINITY, "nothing bounded it below");
            }
            other => panic!("an attribute is not bounded, got {other:?}"),
        }
    }

    /// A rate that reaches zero is refused, and the reason is distinguishable from
    /// the one above. There is no height at which a zero-rate primitive is a pixel
    /// across, so this is not a floor that is merely large.
    #[test]
    fn a_rate_that_reaches_zero_is_refused_and_says_it_proved_zero() {
        match bound(
            "  param point_scale : float [0.00069, 0.0556] = 0.0111\n  \
             param width_var   : float [0.0, 1.0]  = 0.45",
            "point_rate = point_scale * (1.0 - width_var + width_var * hash1(seed) * 2.0);",
        ) {
            Bound::Unbounded { lower, .. } => assert_eq!(lower, 0.0),
            other => panic!("this rate reaches zero, got {other:?}"),
        }
    }

    /// The refusal points at the assignment that defeated it, not at the first one
    /// or at the block. A vertex block writing three rates and failing on the third
    /// has to say which, or the reader is left grepping.
    #[test]
    fn the_refusal_names_the_assignment_that_defeated_it() {
        let params = "  param width_var : float [0.0, 1.0] = 0.45";
        let body = "if seed < 100u {\n      point_rate = 0.01;\n    } else {\n      \
                    point_rate = 0.02 * (1.0 - width_var);\n    }";
        let checked = renderer(params, body);
        // The source `renderer` built, so the span can be read back against it.
        let src = source(params, body);
        match point_rate_bound(&checked).bound {
            Bound::Unbounded { at, lower } => {
                assert_eq!(lower, 0.0);
                // The parser's span for a parenthesised operand stops short of
                // the closing bracket, which is not this module's to fix.
                assert_eq!(at.text(&src), "0.02 * (1.0 - width_var");
            }
            other => panic!("the second arm reaches zero, got {other:?}"),
        }
    }

    /// Every path counts and the least of them wins. Which arm an element takes is
    /// per-element and undecidable here, so the rate the Set can emit is either —
    /// and the floor has to hold for both.
    #[test]
    fn both_arms_of_a_branch_are_bounded_and_the_smaller_holds() {
        let rate = at_least(
            "",
            "if seed < 100u {\n      point_rate = 0.01;\n    } else {\n      \
             point_rate = 0.002;\n    }",
        );
        assert_eq!(rate, 0.002);
    }

    /// A bound says which declarations it rests on, and names no others. Nothing
    /// clamps a write to a declared range, so a caller holding the values needs to
    /// know which ones the floor depends on — `P-0095` at the remove a static bound
    /// sits at.
    #[test]
    fn a_bound_names_the_declarations_it_rests_on() {
        let names = over(
            "  param point_scale : float [0.0014, 0.0417] = 0.0076\n  \
             param exposure    : float [0.0, 8.0] = 1.0",
            "point_rate = point_scale * max(size, 1.0);",
        );
        assert_eq!(names, vec!["point_scale".to_string()]);
        assert!(over("", "point_rate = 0.004;").is_empty());
    }

    /// A procedure with no `vertex` block emits no rate, which is not a bound of
    /// zero: there is no primitive that can fall under a pixel. It is the same
    /// absence `check` infers `Topology::Fullscreen` from.
    #[test]
    fn a_fullscreen_procedure_has_no_primitive_rather_than_no_bound() {
        let src = r#"
proc wash {
  kind  L4
  blend additive

  param level : float [0.0, 1.0] = 0.2

  fragment {
    color = vec4(level, level, level, 1.0);
  }
}
"#;
        let proc = crate::parse(src).expect("parse");
        let checked = crate::check::check(&proc).expect("check");
        assert_eq!(point_rate_bound(&checked).bound, Bound::NoPrimitive);
        assert_eq!(checked.topology, Some(crate::ast::Topology::Fullscreen));
    }

    /// A `uint` is bounded by being a `uint`, and that is the whole of why this
    /// expression has a floor: `source` is an identity nothing declares a range
    /// for, `% 3u` puts it in `0..=2`, and the `+ 1.0` lifts it clear of zero.
    ///
    /// The expression is the corpus's, verbatim from `SOURCE_L4` in
    /// `crates/karakuri-ir/tests/check.rs` — the one computed `point_rate` in that
    /// file, every other being a literal.
    #[test]
    fn an_integer_remainder_bounds_a_value_nothing_declared() {
        let src = r#"
proc lit
{
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = float(source % 3u) + 1.0;
  }

  fragment {
    color = vec4(float(source % 5u) * 0.2, 1.0, 1.0, 1.0);
  }
}
"#;
        let proc = crate::parse(src).expect("parse");
        let checked = crate::check::check(&proc).expect("check");
        match point_rate_bound(&checked).bound {
            Bound::AtLeast { rate, over } => {
                assert_eq!(rate, 1.0);
                assert!(over.is_empty(), "no param is involved");
            }
            other => panic!("expected a bound, got {other:?}"),
        }
    }

    /// A local a loop assigns to is unknown for the whole loop. This pass walks a
    /// body once, and the value a local holds at the top of an iteration is the one
    /// the iteration before left — so the only sound reading is that it could be
    /// anything, and a rate built from it is refused rather than bounded to what
    /// one pass happened to produce.
    #[test]
    fn a_local_a_loop_mutates_is_not_bounded_by_one_pass() {
        match bound(
            "",
            "var w = 0.01;\n    for i in 0..4 {\n      w = w * 0.5;\n    }\n    \
             point_rate = w;",
        ) {
            Bound::Unbounded { lower, .. } => assert_eq!(lower, f32::NEG_INFINITY),
            other => panic!("one pass over a loop proves nothing, got {other:?}"),
        }
    }

    /// A local a loop only reads keeps its bound, so the widening above is about
    /// mutation rather than about loops.
    #[test]
    fn a_loop_that_mutates_nothing_leaves_the_bound_alone() {
        let rate = at_least(
            "  param point_scale : float [0.002, 0.05] = 0.01",
            "var w = point_scale;\n    for i in 0..4 {\n      let unused = float(i);\n    }\n    \
             point_rate = w;",
        );
        assert_eq!(rate, 0.002);
    }

    /// `%` on a float takes the sign of its divisor, which the generated WGSL
    /// spells `a - b * floor(a / b)` — so a positive divisor makes the result
    /// non-negative whatever the dividend was, and `+ 0.001` is then a floor over
    /// an ambient nothing declares.
    #[test]
    fn a_float_remainder_by_a_positive_divisor_is_non_negative() {
        let rate = at_least("", "point_rate = (t % 0.5) + 0.001;");
        assert_eq!(rate, 0.001);
    }
}
