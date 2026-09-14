use super::*;

/// Which of the grammar's four keys a press is, or `None` for a key that is not
/// one of them.
///
/// # Why the digits are a guard and not ten arms
///
/// `1` in the Mixer is deck A's strip and `1` in the Library is its first row,
/// so ten arms naming ten literals would say the digits are bound and say
/// nothing about what they reach. The dispatch table is what says that —
/// `karakuri_console::focus::BUILT` — so the digit is a guard here rather than
/// ten characters (ADR-0333).
///
/// `key_column::bound` no longer reads this file's text at all — since the ten
/// literal keys `window_event` binds outside this guard moved into
/// `KEY_BINDINGS`, there is nothing left in this file's source for a scan to
/// find that a declared fact could not say instead. What this function still
/// binds — `space`, `enter`, the four arrows and the digit — is declared in
/// `key_column::bound` alongside `KEY_BINDINGS`' own keys rather than scanned
/// for, the same as the digit always was.
pub(crate) fn grammar(key: &Key<&str>) -> Option<focus::Press> {
    match key {
        Key::Named(NamedKey::Space) => Some(focus::Press::Space),
        Key::Named(NamedKey::Enter) => Some(focus::Press::Enter),
        Key::Named(NamedKey::ArrowUp) => Some(focus::Press::Arrow(focus::Arrow::Up)),
        Key::Named(NamedKey::ArrowDown) => Some(focus::Press::Arrow(focus::Arrow::Down)),
        Key::Named(NamedKey::ArrowLeft) => Some(focus::Press::Arrow(focus::Arrow::Left)),
        Key::Named(NamedKey::ArrowRight) => Some(focus::Press::Arrow(focus::Arrow::Right)),
        Key::Character(text) => digit(text).map(focus::Press::Digit),
        _ => None,
    }
}

/// One digit, `0` to `9`, or `None` for anything else a `Key::Character` can
/// be.
///
/// A `Key::Character` is *text* and may be more than one character — a dead key
/// resolving, an IME committing a run — which is why the length is checked
/// rather than the first character taken. `char::to_digit` at radix ten accepts
/// the ASCII ten and nothing else, so a digit from another script is not one
/// here: the digits count what a bay drew and the number row is what a
/// performer finds without looking.
pub(crate) fn digit(text: &str) -> Option<usize> {
    let mut chars = text.chars();
    let one = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    one.to_digit(10).map(|digit| digit as usize)
}

/// Where a press takes the exposure — a quarter stop, which is the step the
/// console's own track was built for.
///
/// `karakuri_console::view::EXPOSURE_TRACK_W`'s documentation is where that
/// number comes from and it says the whole argument: *"one pixel a press … a
/// pointer on this track can ask for any of the 48 positions along it and a
/// keyboard stepping a quarter stop at a time can ask for any of the 48 values
/// between the ends, so neither surface can reach a value the other cannot"*.
/// So the step is taken on the track's axis and converted back, rather than as
/// a multiplier written here — the two surfaces then land on the same 48 values
/// by construction.
///
/// Clamped by the conversion rather than here: `unit_of` holds a value past
/// either end at that end and `exposure_at` runs over `[0, 1]`, which is where
/// `--exposure 200` is allowed to be unclamped and a press is not.
///
/// The default is 1.0, which is the middle of the track and the level a run
/// starts at — ADR-0259's *"`space` returns it to 1.0 — today's `` ` ``"*.
pub(crate) fn exposure_key(step: Step, from: f32) -> f32 {
    let at = view::unit_of(from);
    match step {
        Step::Down => view::exposure_at(at - 1.0 / view::EXPOSURE_TRACK_W),
        Step::Up => view::exposure_at(at + 1.0 / view::EXPOSURE_TRACK_W),
        Step::Default => 1.0,
    }
}

/// One press of a tempo key: one beat a minute, which is the smallest change the
/// figure draws as a different whole number.
///
/// A difference and not a ratio, so the same press means the same amount
/// wherever the grid is standing. The figure's own band is
/// `karakuri_console::view::TEMPO_BAND`, ±15% of the tempo at the press, and one
/// step is inside it at every tempo this instrument runs at
/// ([ADR-0350](../../../docs/adr/0350-the-transports-two-cards-are-walked-and-the-tempo-figure-steps-by-a-beat-a-minute.md)).
///
/// No second keyboard steps a tempo: `karakuri-cli`'s `--bpm N` names one
/// outright and binds no key for it.
pub(crate) const TEMPO_STEP_BPM: f32 = 1.0;

/// Where a press takes the latency offset — five milliseconds, which is the
/// page's own step and the one `karakuri-cli`'s `o` and `p` use.
///
/// The sign is the half that gets read wrong at two in the morning, and
/// `docs/manual/console.html` says so: *"Negative and the picture waits for the
/// music, positive and it leads."* So `Step::Down` is the picture waiting, and
/// this function is where a test can ask which way each direction goes — a pair
/// wired the wrong way round reads correct and points backwards.
///
/// `o` and `p` were this keyboard's letters until 2026-09-10, and what survived
/// them is the step rather than the spelling: the constant is
/// `karakuri_environment::audio`'s, which is what the command line steps by,
/// and this program does not keep a second copy of it.
///
/// Not clamped here, which is the one place this differs from [`gain_key`] and
/// [`opacity_key`]: the offset's range is `karakuri_environment::audio`'s and
/// the session holds a press at the end of its travel and says so
/// ([`offset_said`]). A second clamp here would decide the same thing twice.
///
/// The default is zero, which is the value the offset is declared at: a session
/// nobody has nudged runs at no offset at all.
/// Where a press takes the free-run tempo — one beat a minute, which is
/// [`TEMPO_STEP_BPM`].
///
/// `from` is the tempo the grid is running, read off the oscillator at the press.
///
/// Floored at one beat a minute, which is `karakuri_signal`'s own floor: a grid
/// at zero has no beat to run, and this decides what the record says.
///
/// [`Step::Default`] is the tempo unchanged. The figure has no value it was
/// declared at, so `space` declines on it in `karakuri_console::focus` and never
/// reaches this.
pub(crate) fn tempo_key(step: Step, from: f32) -> f32 {
    let asked = match step {
        Step::Down => from - TEMPO_STEP_BPM,
        Step::Up => from + TEMPO_STEP_BPM,
        Step::Default => from,
    };
    asked.max(1.0)
}

pub(crate) fn offset_key(step: Step, from: f32) -> f32 {
    match step {
        Step::Down => from - audio::LATENCY_OFFSET_STEP_MS,
        Step::Up => from + audio::LATENCY_OFFSET_STEP_MS,
        Step::Default => 0.0,
    }
}

/// The engine's look, as the console reads it — [`blend_mode`]'s function one
/// row up, on the value every sink is drawn under.
///
/// Two fields of three: `white_point` is Reinhard's parameter, it is on no
/// surface, and a console field for it would be a reading no control names —
/// see `karakuri_console::view::Look`. The operator goes through
/// [`mix::tonemap`], which is the match that makes the engine's list and the
/// vocabulary's agree and stops compiling the day a fifth operator lands on one
/// side only.
pub(crate) fn look(look: &Look) -> view::Look {
    view::Look {
        tonemap: mix::tonemap(look.op),
        exposure: look.exposure,
    }
}
