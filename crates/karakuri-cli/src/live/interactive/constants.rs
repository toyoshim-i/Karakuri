use super::super::*;

/// One press of a gain key. Linear and additive, because a fader is: the
/// operator wants the same physical move to mean the same amount everywhere,
/// not a proportion of wherever the slot happens to be.
pub(crate) const GAIN_STEP: f32 = 0.1;

/// Quantization intervals for scheduled fades, represented as beat counts and labels.
pub(crate) const QUANTA: [(f64, &str); 3] =
    [(4.0, "the next bar"), (1.0, "the next beat"), (0.0, "now")];

/// How long a scheduled fade lasts, in beats. A bar, half a bar, two bars, and
/// a cut — the four an operator reaches for, in the order they are reached for.
pub(crate) const FADE_BEATS: [f64; 4] = [4.0, 2.0, 8.0, 0.0];

/// The shapes a wipe can take, in cycle order, with the angle each runs at and
/// what to call it.
///
/// `None` first, so that a deck nobody has touched wipes with nothing and says
/// so rather than doing something. The rest are the four directions and the
/// iris — the ones a hand reaches for. An arbitrary angle is a dial, and a dial
/// with nowhere to show its value is a control an operator cannot read.
pub(crate) const MASK_SHAPES: [(MaskKind, f32, &str); 6] = [
    (MaskKind::None, 0.0, "off — `c` needs a shape"),
    (MaskKind::Linear, 0.0, "linear, left to right"),
    (
        MaskKind::Linear,
        std::f32::consts::FRAC_PI_2,
        "linear, bottom to top",
    ),
    (
        MaskKind::Linear,
        std::f32::consts::FRAC_PI_4,
        "linear, diagonal",
    ),
    (
        MaskKind::Linear,
        -std::f32::consts::FRAC_PI_4,
        "linear, the other diagonal",
    ),
    (MaskKind::Radial, 0.0, "an iris"),
];

/// One press of an opacity key. Additive for the same reason as [`GAIN_STEP`],
/// and clamped to `[0, 1]` where gain is not: opacity is a proportion of a
/// blend and there is no such thing as 1.4 of one, while gain is a level into
/// an HDR mix and values above 1.0 are ordinary.
pub(crate) const OPACITY_STEP: f32 = 0.1;

/// One press of an exposure key, as a factor. Multiplicative, because exposure
/// is: a stop is a ratio, and an additive step would be enormous at 0.1 and
/// invisible at 8.0. This is a quarter of a stop, near enough.
pub(crate) const EXPOSURE_STEP: f32 = 1.189_207;

/// The interactive bounds, and only the interactive ones. `-` and `=` nudge
/// inside them, because a key that steps has to stop somewhere and a stop it
/// cannot see is worse than one it can. `--exposure` is not held to them, which
/// is deliberate and is `exposure_positive_is_accepted_unclamped`'s own
/// sentence: a batch render asks for something extreme on purpose, and a flag
/// is read once by somebody who typed it rather than nudged into a corner. What
/// the flag refuses is what has no meaning at all — see [`clamp_exposure`] for
/// why zero and negative are neither clamped nor accepted anywhere.
///
/// This comment said the two shared a range and they never have. It was written
/// beside a constant pair pulled out so the bound would not drift, and the
/// drift was the sentence rather than the numbers.
pub(crate) const EXPOSURE_MIN: f32 = 1.0 / 64.0;
pub(crate) const EXPOSURE_MAX: f32 = 64.0;

/// Exposure is a multiplier before the tone map; zero is degenerate (always
/// black) and negative inverts an otherwise-positive HDR value into one no tone
/// mapper is specified for. Pulled out as a pure function so the bound is one
/// piece of logic instead of two copies that could drift, and so it is testable
/// without a `Live` or a GPU.
pub(crate) fn clamp_exposure(exposure: f32) -> f32 {
    exposure.clamp(EXPOSURE_MIN, EXPOSURE_MAX)
}

/// One press of the scrub keys, in beats. A quarter beat — a sixteenth of a bar
/// in four — which is small enough to place a hit by ear and large enough to
/// hear one press.
pub(crate) const SCRUB_BEATS: f64 = 0.25;

/// A level floor: a negative gain would subtract one slot's light from
/// another's, which is a blend mode rather than a level. Not ceilinged — the
/// pipeline is HDR and values above 1.0 are expected. Pure for the same reason
/// as [`clamp_exposure`].
///
/// `Deck::set_gain` floors too, and the two are not a duplicate. This one
/// decides what the record says, so a session replays the value that took
/// effect rather than one the engine quietly corrected; that one guards the
/// engine against every record it did not write, which is the whole of a
/// replay. Deleting either leaves a real hole.
pub(crate) fn clamp_gain(gain: f32) -> f32 {
    gain.max(0.0)
}

/// The tone map cycle `t` steps through. Pure so the cycle — and that it
/// returns to where it started — is checkable without a window.
pub(crate) fn next_tonemap(op: TonemapOp) -> TonemapOp {
    match op {
        TonemapOp::Clamp => TonemapOp::Reinhard,
        TonemapOp::Reinhard => TonemapOp::Aces,
        TonemapOp::Aces => TonemapOp::AgX,
        TonemapOp::AgX => TonemapOp::Clamp,
    }
}

/// What this program does with an operation: the records it writes, or the
/// sentence saying it did nothing.
///
/// [`Live::performed`] takes the readings and this decides, so that the answer
/// a model is handed and the line a terminal is given are one string built
/// once. It is a free function rather than a method for the same reason
/// [`rewired`] is: it needs no `Live`, and a test can hold it against a real
/// call over the socket without a window or a GPU.
///
/// An operation that writes no record writes nothing here. This surface
/// performs an operation by converting it to records and reading them back —
/// there is no second arm — so `Written::Silent`, `Written::Owed` and
/// `Written::Refused` all mean
/// nothing on this run changed, and a caller that reported success for one
/// would be reporting a change it did not make
/// ([P-0094](../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md):
/// a silently wrong answer loses to a loud failure). That is the whole of why
/// this is a `Result`: `Live::operate`'s printed line reaches an operator who
/// is at the terminal, and a model on `--mcp` is not.
///
/// The refusal names the operation and where it is answered
/// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)),
/// and it is the performer's rather than the gate's — which is
/// [ADR-0341](../../../docs/adr/0341-a-route-that-answers-is-built-and-a-send-that-ends-in-a-dialog-is-gap.md)'s
/// *a route that answers is a built route* read on the surface that has no
/// performer instead of the one that has one. `Silent::why` and `Owed::why` are
/// the reasons in the words the crate that decided them says them in, so
/// nothing is written down twice.
///
/// And the sentence they are carried in is that crate's too —
/// `karakuri_operation_record::not_performed`, which the instrument's MCP drain
/// answers a refusal and a gap in as well. A model reaching `--mcp` and a model
/// reaching `cargo run -p karakuri` made the same mistake and were answered in
/// two different sentences until it was stated once (ADR-0131), and the tail
/// below is the one thing that is true of this program alone: it has no control
/// for a `Silent` and the instrument does, so the instrument performs one and
/// says so.
pub(crate) fn answered(operation: &Operation, current: &Current) -> Result<Vec<Record>, String> {
    let title = operation.title();
    match karakuri_operation_record::written(operation, current) {
        Written::Records(records) => Ok(records),
        Written::Silent(silent) => Err(format!(
            "{}, and this program performs an operation by writing the records it converts \
             to. `karakuri-cli` has no control for this one — the instrument, `cargo run -p \
             karakuri`, is the surface that answers it.",
            not_performed(title, silent.why())
        )),
        // **The reading, and not the control.** `Owed` says a record is owed
        // and could not be made here — a deck the run does not hold, or a
        // vocabulary question nobody has settled — so the sentence is that
        // reason and not *this program has no control for it*, which would be
        // false of an operation whose key is on this keyboard.
        Written::Owed(owed) => Err(not_performed(title, owed.why())),
        // **A decision and not a gap**, in the words the crate that took it
        // says them in: a scheduled move on a fader a lane of the armed
        // pattern holds writes no record, here and on every other surface
        // (ADR-0323). No lane can hold anything on this program — it runs no
        // sequencer — and the arm is here because the sentence is one sentence
        // wherever it is met.
        Written::Refused(refusal) => Err(not_performed(title, &refusal.why())),
    }
}

/// The answer an operation that was performed goes back with.
///
/// One string, so that [`Live::run_operations`] and the test that drives it
/// over a socket say the same thing — and so that the sentence which says
/// *where a later answer lands* is beside the one that says nothing landed.
pub(crate) fn performed_at_the_frame(title: &str) -> String {
    format!(
        "`{title}` was performed on the frame it arrived on, where the same operation from a \
         key or a mapped control is performed. Anything it started rather than finished is \
         reported where it lands: ask `swap_outcome` for a rebuild, and a scheduled move \
         arrives on the grid."
    )
}

/// Whether `at` names a renderer of a slot that draws with `count` of them, and
/// the sentence if it does not. [`slot_in_range`]'s companion, returning the
/// refusal rather than a bool because both callers print it.
pub(crate) fn renderer_in_range(slot: usize, at: usize, count: usize) -> Result<(), String> {
    if at < count {
        Ok(())
    } else {
        Err(no_such_renderer(slot, at, count))
    }
}

/// Status label for a watchdog-stopped overloaded slot (ADR-0316).
/// Returns `"overloaded "` if true, or empty string otherwise.
pub(crate) fn stopped_tag(overloaded: bool) -> &'static str {
    match overloaded {
        true => "overloaded ",
        false => "",
    }
}

/// The status line's four-column form of the same thing [`residency_name`]
/// spells out. Fixed width, so the columns after it do not move.
///
/// Pulled out beside `residency_name` for the reason `slot_in_range` was: the
/// distinction it carries is the one thing about it that can be wrong, and
/// checking it should not need a GPU, a window, or a `Deck`.
pub(crate) fn residency_tag(residency: Residency, parked: bool) -> &'static str {
    match residency {
        Residency::Live => "LIVE",
        Residency::Priming => "prim",
        Residency::Allocated if parked => "park",
        Residency::Allocated => "off ",
    }
}

/// `parked` is [`Deck::is_parked`] for the same slot: Allocated with a standing
/// request to prime. It is not a residency of its own, which is exactly why it
/// has to be passed in — the residency alone cannot tell a deferred request
/// from no request.
///
/// A parked slot and one nobody asked about are the same [`Residency`] and
/// opposite situations, and a surface that names them alike tells the operator
/// their request was discarded when it is being reconsidered every pass.
pub(crate) fn residency_name(residency: Residency, parked: bool) -> &'static str {
    match residency {
        Residency::Live => "live",
        Residency::Priming => "priming (warming, off air)",
        Residency::Allocated if parked => "parked (asked to prime, waiting for room)",
        Residency::Allocated => "allocated (off air)",
    }
}

/// Whether `slot` names one this deck actually has. Pulled out of
/// [`Live::focus_slot`] so the boundary — the neighbour of the off-by-one the
/// digit keys used to have, where a digit equal to or past the slot count must
/// be rejected rather than wrap or panic — is checkable without a window, a
/// GPU, or a `Deck`.
///
/// A digit key names no deck member yet, so this takes the raw `usize` a press
/// or an MCP argument is rather than a [`karakuri_store::record::DeckSlot`] —
/// there is no address here to validate at construction, only a number to check
/// before one can be made. It checks through
/// [`karakuri_store::record::DeckSlot::new`] rather than `slot < slot_count`
/// again, which is the same question `crates/karakuri-environment/src/mix.rs`'s
/// `mix::change` answers for a stream's own `slot` field.
pub(crate) fn slot_in_range(slot: usize, slot_count: usize) -> bool {
    u8::try_from(slot)
        .ok()
        .is_some_and(|slot| DeckSlot::new(slot, slot_count).is_some())
}
