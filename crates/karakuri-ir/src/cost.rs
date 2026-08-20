//! Stage 4: cost estimation.
//!
//! Produces a per-element figure, never a total. `capacity` belongs to the Set,
//! so an artifact has no total cost to be judged on; rejection here is against
//! a per-element ceiling only, and the frame-budget decision belongs to the
//! probe in stage 7 where the real capacity and the real parameters are known.
//!
//! ## Method
//!
//! `ops_per_element` is a static instruction count, multiplied out through loop
//! bounds (constant, per the grammar, so this terminates and needs no actual
//! iteration — a loop's cost is its body's cost times its trip count, computed
//! once and multiplied, never executed `n` times by this pass). Nested loops
//! multiply into the running multiplier, so nesting multiplies the estimate
//! rather than adding to it, per the spec. An `if` charges the condition, one
//! unit for the test itself, and the *more expensive* of its two arms — not
//! both — because both are usually executed on a GPU control-flow-divergent
//! lane anyway, so the arm that is skipped in scalar code is not free here.
//!
//! Every leaf read (a literal, a local, a param, an attribute, an ambient) and
//! every operator or statement costs one unit — a stand-in for "one scalar ALU
//! instruction, roughly." Builtins are weighted relative to that baseline in
//! [`builtin_weight`]; see the comment there for where the numbers come from.
//! **All of this is ordinal, not measured.** Nothing here has been run through
//! a profiler or a WGSL compiler; the numbers encode "this is roughly N times
//! more expensive than a multiply," not a nanosecond figure. Replacing them
//! with real numbers needs GPU timing of each builtin in isolation (a
//! microbenchmark shader per function, on representative hardware) — exactly
//! the kind of measurement stage 7's probe does for a whole procedure, just
//! decomposed per builtin instead.
//!
//! `bytes_per_element` is simpler and not ordinal: it follows directly from
//! the WGSL lowering section. Every emitted attribute gets a pair of storage
//! buffers (prev/next), 16-byte aligned; `seed`, the alive flag, and the birth
//! fraction are always allocated whether or not the procedure names them,
//! because none of the three is nameable from IR.

use crate::ast::{BlockKind, Lit, Ty};
use crate::builtin::Builtin;
use crate::error::{IrError, IrResult};
use crate::span::Span;
use crate::typed::{Checked, Cost, TExpr, TExprKind, TStmt};

/// Per-element ceiling on [`Cost::ops_per_element`]. Anything estimated above
/// this is rejected at stage 4.
///
/// The number is ordinal, chosen rather than measured, and lives here as one
/// tunable constant precisely because it will need retuning once real probe
/// data (stage 7) exists to compare against. It was calibrated against the two
/// worked examples in `docs/ir-spec.md`: `drift_shell` (spawn + element,
/// including a `curl` call) estimates at a little under 200 ops/element under
/// this module's weights, and the `curl`-in-a-4-iteration-loop snippet under
/// [Statements and expressions](../../../docs/ir-spec.md#statements-and-expressions)
/// at a little under 500. `4096` leaves roughly an order of magnitude of
/// headroom above both — enough for a procedure with a few small loops of
/// noise calls — while still catching the pattern the spec calls out
/// explicitly as dangerous: loops nested a few levels deep, each multiplying
/// the estimate rather than adding to it.
pub const MAX_OPS_PER_ELEMENT: u64 = 4096;

/// Ceiling on [`Cost::ops_per_spawn`]. Looser than the per-frame figure because
/// spawning happens once in an element's life and then never again: the work
/// per frame is `spawn_rate * dt` elements' worth, which at any sane rate is a
/// small fraction of what the population costs. Charging spawn against the
/// per-frame ceiling would reject a procedure that seeds an expensive initial
/// state and then coasts, which is a shape worth allowing.
pub const MAX_OPS_PER_SPAWN: u64 = 16_384;

/// Ceiling on [`Cost::ops_per_fragment`] for a **per-element** renderer.
/// Tighter than either of the above, because a fragment is evaluated far more
/// often than an element: one soft sprite covers tens of pixels, they overlap,
/// and there are `capacity` of them. Like the others this is ordinal and
/// untested — stage 7's probe is what actually knows.
pub const MAX_OPS_PER_FRAGMENT: u64 = 512;

/// Ceiling on [`Cost::ops_per_fragment`] for a **fullscreen** renderer.
///
/// **Higher because the fragment count is known here and unknown there**, which
/// is the whole reason the two differ. The number above is a stand-in for an
/// unbounded quantity: `capacity` sprites times their area times whatever they
/// overlap. A fullscreen procedure covers the canvas exactly once and nothing
/// overdraws, so the stand-in has nothing to stand in for — and applying it
/// anyway forbids the one thing the mode exists for. A raymarch is thirty-odd
/// iterations of a distance function by construction; at 512 there is no
/// marcher that fits, which is a ceiling saying no to the feature rather than
/// to an excess.
///
/// Set to [`MAX_OPS_PER_ELEMENT`] deliberately: a fullscreen fragment is the
/// analogue of an element — one evaluation per thing drawn — so it is priced
/// like one rather than given a number of its own to drift.
pub const MAX_OPS_PER_FULLSCREEN_FRAGMENT: u64 = MAX_OPS_PER_ELEMENT;

/// Which fragment ceiling this procedure is held to. See
/// [`MAX_OPS_PER_FULLSCREEN_FRAGMENT`] for why there are two.
fn fragment_ceiling(checked: &Checked) -> u64 {
    if checked.topology == Some(crate::ast::Topology::Fullscreen) {
        MAX_OPS_PER_FULLSCREEN_FRAGMENT
    } else {
        MAX_OPS_PER_FRAGMENT
    }
}

/// Every attribute buffer is 16-byte aligned per the WGSL lowering section.
///
/// **This model is stale and reports about 85% too much.** An element slot took
/// its attribute's own width at WGSL's own offsets in `697557b`; this still pads
/// every one to sixteen bytes and charges the alive flag thirty-two. For
/// `drift_shell` it says 192 bytes an element where the buffers are 104.
///
/// **It is a second copy of an arithmetic that lives in
/// `karakuri-codegen::layout`, and it drifted the moment that one moved** —
/// which is the defect this repository has now paid for five times. The copy
/// exists because `karakuri-codegen` depends on `karakuri-ir` and not the other
/// way round, so the honest fix is to move the placement rules *down* into this
/// crate and have the lowering read them, rather than to correct the numbers
/// here and leave two copies that agree for a while.
///
/// Nothing gates on the figure today — it is printed, and `docs/ir-spec.md`
/// writes it into a `perf` record that nothing yet reads — which is why the
/// drift went unnoticed. M4's metadata file is what starts reading it. See
/// `docs/roadmap.md`, "The cost estimator models a layout that no longer
/// exists".
const BUFFER_ALIGN: u32 = 16;

const fn align16(bytes: u32) -> u32 {
    bytes.div_ceil(BUFFER_ALIGN) * BUFFER_ALIGN
}

/// `seed`, the alive flag, and the birth fraction are always allocated per the
/// lowering section, regardless of whether the procedure names them — none of
/// the three is nameable from IR. Each is 4 bytes natively (a `uint`, a flag,
/// and a `float`), padded to the buffer alignment, and double buffered like
/// any emitted attribute.
const ALWAYS_ALLOCATED_BYTES: u32 = align16(4) * 2 * 3;

fn native_bytes(ty: Ty) -> u32 {
    match ty {
        Ty::Float | Ty::Int | Ty::Uint | Ty::Bool => 4,
        Ty::Vec2 => 8,
        Ty::Vec3 => 12,
        Ty::Vec4 => 16,
        Ty::Mat3 => 36,
        Ty::Mat4 => 64,
    }
}

fn storage_bytes(checked: &Checked) -> u32 {
    // **A field has no element and therefore no per-element storage.** Without
    // this it reported the unconditional slots every element carries, which is
    // a number about a thing it does not have.
    if checked.kind == crate::ast::Kind::Field {
        return 0;
    }
    checked
        .emit
        .iter()
        .fold(ALWAYS_ALLOCATED_BYTES, |total, attr| {
            total + align16(native_bytes(attr.ty())) * 2
        })
}

// ---------------------------------------------------------------------------
// Builtin weights
// ---------------------------------------------------------------------------
//
// Ordinal, not measured — see the module doc. The rough tiers, from cheapest
// to most expensive:
//
// - `1`: componentwise selects/compares (`abs`, `floor`, `min`, `step`, ...) —
//   about one ALU instruction.
// - `2`-`4`: a handful of multiply-adds (`dot`, `cross`) or a single
//   reciprocal-square-root-shaped op (`sqrt`).
// - `8`-`10`: transcendentals (`exp`, `log`, `pow`, trig) — GPUs implement
//   these via range reduction plus a polynomial or rational approximation,
//   commonly cited as several times the cost of a multiply-add.
// - low double digits: integer hashes (a handful of bit-mixing ops each,
//   `hash3` costing roughly three `hash1`s) and the noise functions built on
//   them (`value_noise`, `perlin`, `simplex` — multiple lattice-corner hashes
//   plus interpolation).
// - `curl`: "several noise evaluations" per the task brief — a numerical curl
//   needs the underlying field sampled at multiple offset points (or an
//   analytic gradient with a comparable number of terms), so it is weighted
//   as six `perlin` calls.
// - `fbm`: exactly `octaves` `perlin`-equivalent evaluations plus a per-octave
//   combine, since that is literally what the lowering unrolls it into.
// - SDF primitives, rotations, and distributions: composed from the above
//   (a couple of trig calls plus vector arithmetic for a rotation, a `dot` or
//   `length` plus a few ALU ops for an SDF primitive), weighted accordingly.

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

const W_HSV_RGB: u64 = 10; // piecewise-branchy conversion
const W_SRGB_LINEAR: u64 = W_TRANSCENDENTAL + 2; // a pow-shaped curve

/// Weight of one call to `func`. `args` is only consulted for `fbm`, whose
/// cost depends on its compile-time octave count.
fn builtin_weight(func: Builtin, args: &[TExpr]) -> u64 {
    match func {
        // **Zero here, and counted instead.** What one evaluation costs is the
        // field's own figure, which lives in another file and is not in hand
        // until the Set is built. Giving it a stand-in weight would be a
        // number that is wrong for every field, and giving it the largest
        // plausible one would refuse callers that are fine.
        Builtin::Field => 0,
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
    // `octaves` is `const_args[0]` on `fbm`'s signature (see builtin.rs), so
    // the check pass guarantees this is a constant int literal by the time
    // cost estimation runs — it has to be, since lowering unrolls it. Falling
    // back to a single octave if that invariant is ever violated is a
    // defensive underestimate, not an expected path.
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
    calls: &mut u64,
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
    calls: &mut u64,
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

/// Trip count of `start..end`, per the `for i in <start>..<end>` grammar.
/// Both bounds are integer literals, so this is exact and needs no runtime
/// value; an empty or backwards range costs nothing.
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
    calls: &mut u64,
) -> u64 {
    match &expr.kind {
        TExprKind::Lit(_)
        | TExprKind::Local(_)
        | TExprKind::Param(_)
        | TExprKind::Attr(_)
        | TExprKind::Far(_)
        | TExprKind::Ambient(_) => 1,
        TExprKind::Unary { value, .. } => {
            1u64.saturating_add(expr_cost(value, mult, block, hot, calls))
        }
        TExprKind::Binary { lhs, rhs, .. } => 1u64
            .saturating_add(expr_cost(lhs, mult, block, hot, calls))
            .saturating_add(expr_cost(rhs, mult, block, hot, calls)),
        TExprKind::Builtin { func, args } => {
            // **`mult`, not one.** A `field(p)` inside `for i in 0..48` is
            // forty-eight evaluations, and the number the Set multiplies has to
            // be the number that actually happens.
            if *func == Builtin::Field {
                *calls = calls.saturating_add(mult);
            }
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
        // charges is decided by the block it is in: a `field(p)` in a `vertex`
        // is once per element and one in a `fragment` is once per covered
        // pixel, and the two are not comparable numbers.
        let mut block_calls = 0u64;
        let block_cost = stmts_cost(&block.stmts, 1, block.kind, &mut hot, &mut block_calls);
        // **Every block, before the match below decides which ceiling it is
        // charged to.** Two of them are charged to none — a `camera` scales with
        // nothing, and a `field` is charged to its callers — and a call in
        // either is still a call.
        field_calls.total = field_calls.total.saturating_add(block_calls);
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
                field_calls.per_element = field_calls.per_element.saturating_add(block_calls);
            }
            BlockKind::Spawn => {
                ops_per_spawn = ops_per_spawn.saturating_add(block_cost);
                field_calls.per_spawn = field_calls.per_spawn.saturating_add(block_calls);
            }
            BlockKind::Fragment => {
                ops_per_fragment = ops_per_fragment.saturating_add(block_cost);
                field_calls.per_fragment = field_calls.per_fragment.saturating_add(block_calls);
            }
            // **Charged to nothing, because it scales with nothing.** A
            // `camera` block runs once per frame, in one invocation, whatever
            // the capacity and whatever the frame size — the only block in this
            // language of which that is true. Every ceiling here is a rate
            // against a quantity that multiplies, so there is no ceiling this
            // could exceed: a thousand operations once a frame is free next to
            // one operation per element.
            //
            // It is still *costed* above, so `block_totals` names it in a
            // rejection message about some other block, and a future ceiling
            // has a number to use.
            BlockKind::Camera => {}
            // **Charged per *evaluation*, on its own axis.** A field runs
            // wherever it is called and as often as the caller calls it — once
            // per element in a `vertex`, forty-eight times in a march loop — so
            // it scales with nothing the caller does not decide. Its figure is
            // what a caller multiplies, and the multiplication happens where
            // the Set is built, which is the first point holding both.
            //
            // Not charged to `ops_per_element`: that would be a rate against a
            // quantity a field does not have, and would put a ceiling on a
            // field that says nothing about what evaluating it costs anybody.
            BlockKind::Field => ops_per_evaluation = ops_per_evaluation.saturating_add(block_cost),
        }
    }

    // **Amplification is a product, and it is charged here rather than
    // anywhere downstream.** A `deform` in a node of factor `n` runs `n` times
    // for each element that reaches it, so its per-element figure is `n` times
    // its block cost — which is what makes the ceiling mean the same thing for
    // an amplifying stage as for any other, and what makes a stage of factor 64
    // running an expensive body get refused for the reason it deserves rather
    // than sailing through at a sixty-fourth of its true cost.
    //
    // `bytes_per_element` multiplies for the same reason and stays as partial a
    // figure as it always was — `storage_bytes` counts this node's own `emit`
    // and doubles it for the two buffers an L1 has, and an amplifying L2 has
    // neither property: three synthetic slots rather than two, and one buffer
    // rather than two. It is wrong in both directions and nothing reads it,
    // which is why it is left alone rather than half-corrected here: the figure
    // wants an owner, and that owner is the memory budget a deck's residency
    // question needs, not this function.
    let amplify = u64::from(checked.amplify.unwrap_or(1));
    let ops_per_element = ops_per_element.saturating_mul(amplify);

    let cost = Cost {
        ops_per_element,
        ops_per_spawn,
        ops_per_fragment,
        ops_per_evaluation,
        field_calls,
        bytes_per_element: storage_bytes(checked).saturating_mul(checked.amplify.unwrap_or(1)),
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

/// **Re-check a caller with the field it evaluates multiplied in.**
///
/// A `field(p)` weighs nothing where the caller is estimated, because what one
/// evaluation costs lives in another file. So the ceiling a caller passed was a
/// ceiling applied to an incomplete figure, and this is where it is completed —
/// at the Set, which is the first point holding both procedures.
///
/// **Not a nicety.** `examples/field_march.kir` marches forty-eight steps; a
/// field of 48 ops/evaluation adds 2304 to a 4096 fragment ceiling. A Set that
/// skipped this would run a shader nobody had costed, and the number it is over
/// by would be invisible.
pub fn check_with_field(caller: &Checked, per_evaluation: u64) -> IrResult<()> {
    // **Estimated, not read.** `Checked::cost` is never filled by anything —
    // see its own doc — so taking it from there made this function a no-op that
    // reported success.
    let cost = estimate(caller)?;
    let calls = cost.field_calls;
    for (own, count, ceiling, unit) in [
        (
            cost.ops_per_element,
            calls.per_element,
            MAX_OPS_PER_ELEMENT,
            "ops/element",
        ),
        (
            cost.ops_per_spawn,
            calls.per_spawn,
            MAX_OPS_PER_SPAWN,
            "ops/spawn",
        ),
        (
            cost.ops_per_fragment,
            calls.per_fragment,
            fragment_ceiling(caller),
            "ops/fragment",
        ),
    ] {
        if count == 0 {
            continue;
        }
        let total = own.saturating_add(count.saturating_mul(per_evaluation));
        if total > ceiling {
            return Err(vec![IrError::new(
                crate::error::Stage::Cost,
                caller.span,
                format!(
                    "{total} {unit} with the field multiplied in exceeds the {ceiling} {unit} \
                     ceiling ({own} of its own, plus {count} evaluations at {per_evaluation})"
                ),
            )
            .with_hint(
                "a field is inlined at every call site, so evaluating one in a loop costs the \
                 loop's count — cut the field, cut the evaluations, or cut the loop",
            )]);
        }
    }
    Ok(())
}

/// Build the rejection diagnostic. Per the validation pipeline section, a cost
/// rejection has to carry the estimate and the ceiling as actual numbers, and
/// should say what dominated so a regeneration has something to aim at —
/// "over budget" tells a repair prompt nothing about how much to cut.
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
