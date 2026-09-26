//! Conservative lower-bound estimation for `point_rate` output expressions.
//!
//! Uses interval arithmetic over parameter declarations and literals to establish
//! a guaranteed minimum primitive size for renderer procedures.

use std::collections::{BTreeSet, HashMap};

use crate::ast::{BinOp, BlockKind, Lit, Output, Ty, UnOp};
use crate::builtin::Builtin;
use crate::span::Span;
use crate::typed::{Checked, TExpr, TExprKind, TStmt, Target};

/// Lower-bound estimate for `point_rate` emitted by a procedure.
#[derive(Debug, Clone, PartialEq)]
pub struct RateBound {
    /// The procedure the vertex block was read from.
    pub procedure: String,
    /// What was proved.
    pub bound: Bound,
}

/// The result of `point_rate_bound` analysis.
#[derive(Debug, Clone, PartialEq)]
pub enum Bound {
    /// The procedure has no `vertex` block and emits no primitive.
    NoPrimitive,
    /// Minimum guaranteed `point_rate` over all parameter states.
    AtLeast {
        /// Lower bound rate (strictly greater than zero).
        rate: f32,
        /// Parameter names upon whose declarations this bound depends.
        over: Vec<String>,
    },
    /// No positive lower bound could be statically proved.
    Unbounded {
        /// Location of the `point_rate` assignment that could not be bounded.
        at: Span,
        /// Computed lower bound at the failure point.
        lower: f32,
    },
}

impl RateBound {
    /// Returns the bounded rate if positive, or `None` if unbounded or non-primitive.
    pub fn rate(&self) -> Option<f32> {
        match self.bound {
            Bound::AtLeast { rate, .. } => Some(rate),
            _ => None,
        }
    }
}

/// Bounds the minimum `point_rate` `proc` can emit across its parameter ranges.
pub fn point_rate_bound(proc: &Checked) -> RateBound {
    point_rate_bound_at(proc, &HashMap::new())
}

/// Bounds the minimum `point_rate` `proc` can emit with given parameters pinned to held values.
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

/// Computes the minimal lower bound across all emitted assignments.
fn settle(emitted: &[(Span, Value)], block: Span) -> Bound {
    if emitted.is_empty() {
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
    // Round down into f32 so that the conservative lower bound is preserved.
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

/// Closed interval `[lo, hi]` containing all components of a value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub lo: f64,
    pub hi: f64,
}

impl Range {
    /// Unbounded range representing an unknown value.
    pub const WHOLE: Range = Range {
        lo: f64::NEG_INFINITY,
        hi: f64::INFINITY,
    };

    /// Returns a validated interval, or [`Range::WHOLE`] if endpoints are invalid or inverted.
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

    /// Default interval bounding values of type `ty`.
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

    /// Evaluates `%` according to whether `ty` is float (`mod`) or integer (remainder).
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

/// Multiplies interval endpoints with the convention `0 × ∞ = 0`.
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

/// An evaluated interval range along with the parameter declarations it depends upon.
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
                // Join environments from both branches without evaluating runtime conditions.
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
                // Locals assigned inside loop bodies are conservative WHOLE intervals.
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
                // Join with state before loop in case loop runs 0 iterations.
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
            // Attributes, ambients, field samples, and texture fetches have unbounded dynamic ranges.
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
                    // Matrix products are conservatively unbounded.
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

/// Transforms an interval under type conversion rules.
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

/// Evaluates interval bounds for builtin functions given argument ranges.
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

/// Evaluates interval bounds for `pow(x, y)` at box domain corners.
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
mod tests;
