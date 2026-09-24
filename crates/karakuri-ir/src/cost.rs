//! Stage 4: static cost estimation for compute budget bounds.

use crate::ast::{BlockKind, Kind, Lit};
use crate::builtin::Builtin;
use crate::error::{IrError, IrResult};
use crate::span::Span;
use crate::typed::{Checked, Cost, TExpr, TExprKind, TStmt};

/// Maximum estimated ops per element per frame in L1 or L2 stages.
pub const MAX_OPS_PER_ELEMENT: u64 = 4096;

/// Maximum estimated ops per element in the spawn block.
pub const MAX_OPS_PER_SPAWN: u64 = 16_384;

/// Maximum estimated ops per fragment for per-element renderers.
pub const MAX_OPS_PER_FRAGMENT: u64 = 512;

/// Maximum estimated ops per fragment for fullscreen or post-processing passes.
pub const MAX_OPS_PER_FULLSCREEN_FRAGMENT: u64 = MAX_OPS_PER_ELEMENT;

/// Returns the fragment stage cost ceiling applicable to `checked`.
fn fragment_ceiling(checked: &Checked) -> u64 {
    if checked.kind == Kind::L5 || checked.topology == Some(crate::ast::Topology::Fullscreen) {
        MAX_OPS_PER_FULLSCREEN_FRAGMENT
    } else {
        MAX_OPS_PER_FRAGMENT
    }
}

// Relative ordinal weights for builtins (ALU/selects: 1, vectors/roots: 2-4,
// transcendentals: 8-10, hashes/noise: low double digits, composite/unrolled: scaled).

const W_CHEAP: u64 = 1; // abs/floor/ceil/round/fract/sign/min/max/step/mod
const W_SELECT3: u64 = 2; // clamp/mix: two selects/lerps worth of blending
const W_SMOOTHSTEP: u64 = 3; // clamp + a cubic Hermite blend
const W_SQRT: u64 = 4;
const W_TRANSCENDENTAL: u64 = 8; // exp/log/exp2/log2
const W_POW: u64 = 10; // often lowered as exp(log(x) * y)
const W_TRIG: u64 = 8; // sin/cos
const W_TRIG_HARDER: u64 = 9; // asin/acos/atan: extra branching for domain edges
const W_TRIG_HARDEST: u64 = 10; // tan (division on top), atan2 (quadrant select)
const W_DOT: u64 = 2;
const W_CROSS: u64 = 4;
const W_LENGTH: u64 = W_DOT + W_SQRT; // 6
const W_DISTANCE: u64 = W_LENGTH + 1; // 7: subtract, then length
const W_NORMALIZE: u64 = W_LENGTH + 2; // 8: length, then a componentwise divide
const W_REFLECT: u64 = W_DOT + 3; // I - 2*dot(I,N)*N
const W_REFRACT: u64 = W_REFLECT + W_SQRT + 2; // + a discriminant and a branch

const W_HASH1: u64 = 3;
const W_HASH2: u64 = 5;
const W_HASH3: u64 = 7;
const W_VALUE_NOISE: u64 = 12; // 8 lattice-corner hashes + trilinear blend
const W_PERLIN: u64 = 16; // 8 corner gradients, dot products, and a blend
const W_SIMPLEX: u64 = 14; // fewer corners than a cubic lattice (4 in 3D)
const W_CURL: u64 = 6 * W_PERLIN; // several field evaluations; see doc above

const W_SD_SPHERE: u64 = W_LENGTH + 1;
const W_SD_BOX: u64 = 10; // abs + max + length across components
const W_SD_TORUS: u64 = 9;
const W_SD_PLANE: u64 = W_DOT + 2;
const W_OP_UNION: u64 = W_CHEAP;
const W_OP_SUBTRACT: u64 = W_CHEAP;
const W_OP_INTERSECT: u64 = W_CHEAP;
const W_OP_SMOOTH_UNION: u64 = 8; // smin: a clamp plus a cubic blend

const W_ROT: u64 = 2 * W_TRIG + 4; // build sin/cos, then a handful of madds
const W_ROT_AXIS: u64 = 2 * W_TRIG + 10; // Rodrigues' formula: more vector terms

const W_SPHERE_POINT: u64 = 2 * W_TRIG + W_SQRT + 4;
const W_DISC_POINT: u64 = W_TRIG + W_SQRT + 2;

// Relative weight estimations for texture sampling.
const W_TEXEL: u64 = 2; // address arithmetic and an unfiltered load
const W_TAP: u64 = 4; // the same, plus the filter the texture unit does
const W_FRAME_STEP: u64 = 2; // a divide and a multiply against the viewport

const W_HSV_RGB: u64 = 10; // piecewise-branchy conversion
const W_SRGB_LINEAR: u64 = W_TRANSCENDENTAL + 2; // a pow-shaped curve

/// Weight of one call to `func`. `args` is only consulted for `fbm`, whose cost
/// depends on its compile-time octave count.
fn builtin_weight(func: Builtin, args: &[TExpr]) -> u64 {
    match func {
        Builtin::Abs
        | Builtin::Floor
        | Builtin::Ceil
        | Builtin::Round
        | Builtin::Fract
        | Builtin::Mod
        | Builtin::Min
        | Builtin::Max
        | Builtin::Step
        | Builtin::Sign => W_CHEAP,
        Builtin::Clamp | Builtin::Mix => W_SELECT3,
        Builtin::Smoothstep => W_SMOOTHSTEP,
        Builtin::Sqrt => W_SQRT,
        Builtin::Pow => W_POW,
        Builtin::Exp | Builtin::Log | Builtin::Exp2 | Builtin::Log2 => W_TRANSCENDENTAL,

        Builtin::Sin | Builtin::Cos => W_TRIG,
        Builtin::Asin | Builtin::Acos | Builtin::Atan => W_TRIG_HARDER,
        Builtin::Tan | Builtin::Atan2 => W_TRIG_HARDEST,

        Builtin::Length => W_LENGTH,
        Builtin::Distance => W_DISTANCE,
        Builtin::Normalize => W_NORMALIZE,
        Builtin::Dot => W_DOT,
        Builtin::Cross => W_CROSS,
        Builtin::Reflect => W_REFLECT,
        Builtin::Refract => W_REFRACT,

        Builtin::Hash1 => W_HASH1,
        Builtin::Hash2 => W_HASH2,
        Builtin::Hash3 => W_HASH3,
        Builtin::ValueNoise => W_VALUE_NOISE,
        Builtin::Perlin => W_PERLIN,
        Builtin::Simplex => W_SIMPLEX,
        Builtin::Fbm => fbm_weight(args),
        Builtin::Curl => W_CURL,

        Builtin::SdSphere => W_SD_SPHERE,
        Builtin::SdBox => W_SD_BOX,
        Builtin::SdTorus => W_SD_TORUS,
        Builtin::SdPlane => W_SD_PLANE,
        Builtin::OpUnion => W_OP_UNION,
        Builtin::OpSmoothUnion => W_OP_SMOOTH_UNION,
        Builtin::OpSubtract => W_OP_SUBTRACT,
        Builtin::OpIntersect => W_OP_INTERSECT,

        Builtin::RotX | Builtin::RotY | Builtin::RotZ => W_ROT,
        Builtin::RotAxis => W_ROT_AXIS,

        Builtin::Texel => W_TEXEL,
        Builtin::Tap => W_TAP,
        Builtin::FrameStep => W_FRAME_STEP,

        Builtin::SpherePoint => W_SPHERE_POINT,
        Builtin::DiscPoint => W_DISC_POINT,

        Builtin::HsvToRgb | Builtin::RgbToHsv => W_HSV_RGB,
        Builtin::SrgbToLinear | Builtin::LinearToSrgb => W_SRGB_LINEAR,
    }
}

/// `fbm(position, octaves)` unrolls to `octaves` noise evaluations at lowering
/// time (see WGSL lowering notes), plus a per-octave amplitude/frequency
/// combine, so its cost is exactly that multiplication rather than a fixed
/// weight.
fn fbm_weight(args: &[TExpr]) -> u64 {
    // `octaves` is validated as a constant integer literal during type checking.
    let octaves = match args.get(1).map(|a| &a.kind) {
        Some(TExprKind::Lit(Lit::Int(n))) => (*n).max(0) as u64,
        _ => 1,
    };
    (W_PERLIN + 1).saturating_mul(octaves)
}

// ---------------------------------------------------------------------------
// Static op counting
// ---------------------------------------------------------------------------

/// The most expensive single builtin call found, scaled by how many times its
/// enclosing loops replay it. Recorded so a rejection can name what dominated
/// the estimate instead of just reporting a number.
struct HotSpot {
    builtin: Builtin,
    /// `weight * (product of enclosing loop trip counts)`.
    contribution: u64,
    block: BlockKind,
    span: Span,
}

fn stmts_cost(
    stmts: &[TStmt],
    mult: u64,
    block: BlockKind,
    hot: &mut Option<HotSpot>,
    calls: &mut Vec<(String, u64)>,
) -> u64 {
    stmts.iter().fold(0u64, |total, s| {
        total.saturating_add(stmt_cost(s, mult, block, hot, calls))
    })
}

fn stmt_cost(
    stmt: &TStmt,
    mult: u64,
    block: BlockKind,
    hot: &mut Option<HotSpot>,
    calls: &mut Vec<(String, u64)>,
) -> u64 {
    match stmt {
        TStmt::Let { value, .. } | TStmt::Var { value, .. } | TStmt::Assign { value, .. } => {
            1u64.saturating_add(expr_cost(value, mult, block, hot, calls))
        }
        TStmt::If {
            cond, then, els, ..
        } => {
            let cond_cost = expr_cost(cond, mult, block, hot, calls);
            let then_cost = stmts_cost(then, mult, block, hot, calls);
            let els_cost = stmts_cost(els, mult, block, hot, calls);
            // Both arms are charged for hot-spot tracking (both were walked
            // above), but the total only counts the pricier one plus the
            // test: on a GPU both are usually executed by every lane anyway,
            // so the skipped arm in scalar code is not free here.
            cond_cost
                .saturating_add(1)
                .saturating_add(then_cost.max(els_cost))
        }
        TStmt::For {
            start, end, body, ..
        } => {
            let iterations = loop_iterations(*start, *end);
            // The multiplier that reaches any builtin *inside* this loop
            // grows for hot-spot tracking, but the body is walked once — its
            // cost is computed, not executed `iterations` times — which is
            // what keeps a huge literal bound cheap to estimate.
            let inner_mult = mult.saturating_mul(iterations);
            let body_cost = stmts_cost(body, inner_mult, block, hot, calls);
            // +1 per iteration for the loop counter's increment/compare.
            iterations.saturating_mul(body_cost.saturating_add(1))
        }
        TStmt::Kill { .. } => 1,
    }
}

/// Trip count of `start..end`, per the `for i in <start>..<end>` grammar. Both
/// bounds are integer literals, so this is exact and needs no runtime value; an
/// empty or backwards range costs nothing.
fn loop_iterations(start: i32, end: i32) -> u64 {
    let n = i64::from(end) - i64::from(start);
    if n <= 0 {
        0
    } else {
        n as u64
    }
}

fn expr_cost(
    expr: &TExpr,
    mult: u64,
    block: BlockKind,
    hot: &mut Option<HotSpot>,
    calls: &mut Vec<(String, u64)>,
) -> u64 {
    match &expr.kind {
        // A Source slot's read is a uniform load, which is what an ambient's
        // and a param's are — one, on the same terms.
        TExprKind::Lit(_)
        | TExprKind::Local(_)
        | TExprKind::Param(_)
        | TExprKind::Attr(_)
        | TExprKind::Far(_)
        | TExprKind::Source { .. }
        | TExprKind::Ambient(_) => 1,
        TExprKind::Unary { value, .. } => {
            1u64.saturating_add(expr_cost(value, mult, block, hot, calls))
        }
        TExprKind::Binary { lhs, rhs, .. } => 1u64
            .saturating_add(expr_cost(lhs, mult, block, hot, calls))
            .saturating_add(expr_cost(rhs, mult, block, hot, calls)),
        TExprKind::Builtin { func, args } => {
            let weight = builtin_weight(*func, args);
            let contribution = weight.saturating_mul(mult);
            let is_new_max = hot.as_ref().is_none_or(|h| contribution > h.contribution);
            if is_new_max {
                *hot = Some(HotSpot {
                    builtin: *func,
                    contribution,
                    block,
                    span: expr.span,
                });
            }
            let args_cost = args.iter().fold(0u64, |total, a| {
                total.saturating_add(expr_cost(a, mult, block, hot, calls))
            });
            weight.saturating_add(args_cost)
        }
        // Field calls are counted scaled by the loop multiplier rather than
        // weighted statically; actual costs are multiplied during Set composition.
        TExprKind::Field { slot, point } => {
            match calls.iter_mut().find(|(name, _)| name == slot) {
                Some((_, n)) => *n = n.saturating_add(mult),
                None => calls.push((slot.clone(), mult)),
            }
            expr_cost(point, mult, block, hot, calls)
        }
        // Texture fetches are weighted by the sampling builtin plus coordinate calculation cost.
        TExprKind::Sample {
            func,
            texture: _,
            at,
        } => {
            let weight = builtin_weight(*func, &[]);
            let contribution = weight.saturating_mul(mult);
            let is_new_max = hot.as_ref().is_none_or(|h| contribution > h.contribution);
            if is_new_max {
                *hot = Some(HotSpot {
                    builtin: *func,
                    contribution,
                    block,
                    span: expr.span,
                });
            }
            let at_cost = at
                .as_ref()
                .map_or(0, |a| expr_cost(a, mult, block, hot, calls));
            weight.saturating_add(at_cost)
        }
        TExprKind::Construct { args } => 1u64.saturating_add(args.iter().fold(0u64, |total, a| {
            total.saturating_add(expr_cost(a, mult, block, hot, calls))
        })),
        TExprKind::Swizzle { value, .. } => {
            1u64.saturating_add(expr_cost(value, mult, block, hot, calls))
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Estimate, and reject anything above the per-element ceiling.
pub fn estimate(checked: &Checked) -> IrResult<Cost> {
    let mut hot: Option<HotSpot> = None;
    let mut block_totals: Vec<(BlockKind, u64)> = Vec::with_capacity(checked.blocks.len());
    let mut ops_per_element = 0u64;

    let mut ops_per_spawn = 0u64;
    let mut ops_per_fragment = 0u64;
    let mut ops_per_evaluation = 0u64;
    let mut field_calls = crate::typed::FieldCalls::default();

    for block in &checked.blocks {
        // **Counted per block**, because which of the three ceilings a call
        // charges is decided by the block it is in: a field call in a `vertex`
        // is once per element and one in a `fragment` is once per covered
        // pixel, and the two are not comparable numbers.
        let mut block_calls: Vec<(String, u64)> = Vec::new();
        let block_cost = stmts_cost(&block.stmts, 1, block.kind, &mut hot, &mut block_calls);
        // **Every block, before the match below decides which ceiling it is
        // charged to.** Two of them are charged to none — a `camera` scales with
        // nothing, and a `field` is charged to its callers — and a call in
        // either is still a call.
        for (slot, n) in &block_calls {
            let entry = field_calls.entry(slot);
            entry.total = entry.total.saturating_add(*n);
        }
        block_totals.push((block.kind, block_cost));
        // Each block's cost is charged to the quantity it actually scales
        // with. These are not summed: see `Cost`.
        match block.kind {
            // A `deform` runs once per live element, exactly as `element` and
            // `vertex` do — the layer differs, what it scales with does not.
            // A `mask` runs once per live element beside the `deform` it
            // gates, so it scales with exactly what that does.
            BlockKind::Element | BlockKind::Deform | BlockKind::Mask | BlockKind::Vertex => {
                ops_per_element = ops_per_element.saturating_add(block_cost);
                for (slot, n) in &block_calls {
                    let entry = field_calls.entry(slot);
                    entry.per_element = entry.per_element.saturating_add(*n);
                }
            }
            BlockKind::Spawn => {
                ops_per_spawn = ops_per_spawn.saturating_add(block_cost);
                for (slot, n) in &block_calls {
                    let entry = field_calls.entry(slot);
                    entry.per_spawn = entry.per_spawn.saturating_add(*n);
                }
            }
            // **The same axis, and the same rate against the same quantity.**
            // Fragment and frame blocks scale with rasterized fragment/texel counts.
            BlockKind::Fragment | BlockKind::Frame => {
                ops_per_fragment = ops_per_fragment.saturating_add(block_cost);
                for (slot, n) in &block_calls {
                    let entry = field_calls.entry(slot);
                    entry.per_fragment = entry.per_fragment.saturating_add(*n);
                }
            }
            // Camera blocks run once per frame and are not subject to per-element/fragment ceilings.
            BlockKind::Camera => {}
            // Field procedures are charged per evaluation.
            BlockKind::Field => ops_per_evaluation = ops_per_evaluation.saturating_add(block_cost),
        }
    }

    // Amplification multiplies deformation cost per input element.
    let amplify = u64::from(checked.amplify.unwrap_or(1));
    let ops_per_element = ops_per_element.saturating_mul(amplify);

    let cost = Cost {
        ops_per_element,
        ops_per_spawn,
        ops_per_fragment,
        ops_per_evaluation,
        field_calls,
    };

    for (measured, ceiling, unit) in [
        (ops_per_element, MAX_OPS_PER_ELEMENT, "ops/element"),
        (ops_per_spawn, MAX_OPS_PER_SPAWN, "ops/spawn"),
        (ops_per_fragment, fragment_ceiling(checked), "ops/fragment"),
    ] {
        if measured > ceiling {
            return Err(vec![reject(
                checked,
                measured,
                ceiling,
                unit,
                &block_totals,
                hot,
            )]);
        }
    }

    Ok(cost)
}

/// Error report when a procedure combined with field evaluation exceeds budget.
pub struct OverBudget {
    pub slot: String,
    pub errors: Vec<IrError>,
}

/// Evaluates static budget compliance for `caller` including dynamically spliced field costs.
pub fn check_with_field(
    caller: &Checked,
    per_evaluation: &dyn Fn(&str) -> u64,
) -> Result<(), OverBudget> {
    let cost = match estimate(caller) {
        Ok(cost) => cost,
        // Over on its own terms, before any field is multiplied in. There is no
        // slot to blame, so the first declared one carries the report rather
        // than the sentence claiming a field it cannot name.
        Err(errors) => {
            return Err(OverBudget {
                slot: caller
                    .field_slots()
                    .first()
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
                errors,
            })
        }
    };
    for (own, ceiling, unit, count_of) in [
        (
            cost.ops_per_element,
            MAX_OPS_PER_ELEMENT,
            "ops/element",
            (|s: &crate::typed::SlotCalls| s.per_element) as fn(&crate::typed::SlotCalls) -> u64,
        ),
        (
            cost.ops_per_spawn,
            MAX_OPS_PER_SPAWN,
            "ops/spawn",
            (|s: &crate::typed::SlotCalls| s.per_spawn) as fn(&crate::typed::SlotCalls) -> u64,
        ),
        (
            cost.ops_per_fragment,
            fragment_ceiling(caller),
            "ops/fragment",
            (|s: &crate::typed::SlotCalls| s.per_fragment) as fn(&crate::typed::SlotCalls) -> u64,
        ),
    ] {
        // What each slot adds on this axis, and what all of them add together.
        let charged: Vec<(&str, u64, u64)> = cost
            .field_calls
            .slots
            .iter()
            .map(|s| (s.slot.as_str(), count_of(s), per_evaluation(&s.slot)))
            .filter(|(_, count, _)| *count > 0)
            .collect();
        let added = charged.iter().fold(0u64, |sum, (_, count, each)| {
            sum.saturating_add(count.saturating_mul(*each))
        });
        if added == 0 {
            continue;
        }
        let total = own.saturating_add(added);
        if total > ceiling {
            // The one worth cutting first, which is the one the sentence names.
            let (slot, count, each) = charged
                .iter()
                .max_by_key(|(_, count, each)| count.saturating_mul(*each))
                .expect("`added` is non-zero, so something is charged");
            return Err(OverBudget {
                slot: (*slot).to_string(),
                errors: vec![IrError::new(
                    crate::error::Stage::Cost,
                    caller.span,
                    format!(
                        "{total} {unit} with its fields multiplied in exceeds the {ceiling} \
                         {unit} ceiling ({own} of its own, plus {count} evaluations of \
                         `{slot}` at {each})"
                    ),
                )
                .with_hint(
                    "a field is inlined at every call site, so evaluating one in a loop costs \
                     the loop's count — cut the field, cut the evaluations, or cut the loop",
                )],
            });
        }
    }
    Ok(())
}

/// Build the rejection diagnostic. Per the validation pipeline section, a cost
/// rejection has to carry the estimate and the ceiling as actual numbers, and
/// should say what dominated so a regeneration has something to aim at — "over
/// budget" tells a repair prompt nothing about how much to cut.
fn reject(
    checked: &Checked,
    ops: u64,
    ceiling: u64,
    unit: &str,
    block_totals: &[(BlockKind, u64)],
    hot: Option<HotSpot>,
) -> IrError {
    if let Some(hot) = hot {
        let percent = hot.contribution.saturating_mul(100) / ops;
        let message = format!(
            "{ops} {unit} exceeds the {ceiling} {unit} ceiling \
             (dominated by `{}` in `{}`: ~{} of {ops} ops, {percent}%)",
            hot.builtin.name(),
            hot.block.name(),
            hot.contribution,
        );
        let hint = format!(
            "cut or cheapen the `{}` call in the `{}` block first — it alone accounts for \
             about {percent}% of the estimate. Fewer loop iterations around it, a lower `fbm` \
             octave count, or a cheaper builtin all reduce it directly.",
            hot.builtin.name(),
            hot.block.name(),
        );
        IrError::cost(hot.span, message).with_hint(hint)
    } else {
        // No single builtin call stands out — the estimate is dominated by
        // plain statements and arithmetic, most likely under deep loop
        // nesting. Point at the costliest block instead.
        let (dom_block, dom_cost) = block_totals
            .iter()
            .copied()
            .max_by_key(|&(_, c)| c)
            .unwrap_or((BlockKind::Element, ops));
        let message = format!(
            "{ops} {unit} exceeds the {ceiling} {unit} ceiling \
             (the `{}` block accounts for {dom_cost} of {ops} ops)",
            dom_block.name(),
        );
        let hint = format!(
            "cut statements or loop iterations in the `{}` block — the estimate is dominated \
             by plain arithmetic under loop nesting there, not by a single builtin call.",
            dom_block.name(),
        );
        IrError::cost(checked.span, message).with_hint(hint)
    }
}
