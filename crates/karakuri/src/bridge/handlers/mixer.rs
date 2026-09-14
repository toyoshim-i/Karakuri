use super::*;

/// What a scheduled move on this control is taking it to, or `None` for a
/// control nothing is moving.
///
/// The engine's `Transition` carries the instant it starts, its length in
/// beats and its curve as well, and none of the three crosses this seam: the
/// console has no beat count, so what it could draw out of them is nothing.
/// See `view::Strip::gain_to`.
pub(crate) fn destination(deck: &Deck, slot: EngineSlot, control: Control) -> Option<f32> {
    deck.transitions_on(slot)
        .find(|t| t.control() == control)
        .map(|t| t.to())
}

/// What each preview cell's risk badge reads, from the pass that decided it —
/// one entry per deck slot, in slot order.
///
/// `Decision::budgeted_ms` is the number the governor spent and
/// `Decision::basis` says which of its two numbers that is (ADR-0296). The
/// console reads the number into five bands and carries the basis undrawn
/// (ADR-0298), so both halves cross and neither is spent twice.
///
/// `Basis::Unbudgetable` is written as `None`, and that is the whole of what
/// this function decides. It is a slot nothing measured and nothing estimated,
/// it is not a zero, and a zero here would draw a green dot.
///
/// A deck with more slots than the row has cells contributes nothing past the
/// fourth, which is the reading `View::select` refuses a key on.
pub(crate) fn costs(governed: &Report) -> [Option<Budgeted>; DECKS] {
    let mut out = [None; DECKS];
    for decision in &governed.decisions {
        let Some(cell) = out.get_mut(decision.slot) else {
            continue;
        };
        *cell = match (decision.budgeted_ms, decision.basis) {
            (Some(ms), Spent::Estimated) => Some(Budgeted {
                ms,
                basis: Basis::Estimated,
            }),
            (Some(ms), Spent::Measured) => Some(Budgeted {
                ms,
                basis: Basis::Measured,
            }),
            // `Unbudgetable`, and a number arriving without a basis — which
            // cannot happen, and is not worth inventing a band for if it does.
            _ => None,
        };
    }
    out
}

/// What the mixer strips read this frame: one per slot the deck has, out
/// of the six things a `Deck` will say about a slot.
///
/// Everything here is reachable from a `Deck` and nothing reaches around one:
/// `slot_count`, `residency`, `gain`, `opacity`, `blend`, `mask` and `level`
/// are its own, and this takes a `&Deck` because it is on the side of the seam
/// that is allowed one — what crosses into the console is a name, a word and
/// four numbers (ADR-0156).
///
/// # Where each one comes from, and the two that are not the deck's
///
/// - The tally is `Deck::residency`, which is the effective residency
///   the frame loop reads and not `requested_residency`. The governor moves a
///   slot down without anybody asking, and a tally showing the request would
///   be describing a slot that is doing something else.
/// - And the request beside it — `Deck::requested_residency`, which is the
///   other half of the same pair. Both are handed over and neither side
///   computes `Deck::is_parked`: the engine has the predicate and the
///   console derives its own from the two values (`view::Strip::pending`), so
///   what crosses the seam stays a residency and a residency rather than
///   becoming a bit whose meaning is written down in only one of the two
///   crates. It is the same reading as the four numbers below — the deck says
///   what it is doing, and the surface decides what that looks like.
/// - The trim and the fader are `gain` and `opacity`, which are two
///   controls and not one — *"opacity at zero silences under every blend mode,
///   gain at zero does not silence `over`"* — and the bay draws them as two.
/// - The blend is [`blend_mode`]: the engine's `Blend` turned into the
///   vocabulary's `BlendMode`, because the chip is a control now and a control
///   has to know which of the three it is on to say what the next one is
///   (ADR-0187). This is where a fourth engine mode with no operation
///   variant stops the build, which is the failure worth having — the
///   alternative is a word drawn on a chip no map can ask for.
/// - The mask is `Deck::mask(slot).kind()` for the mark, and its angle
///   beside it — `view::Strip::mask_angle`, which is read to build the
///   press's operation and drawn nowhere
///   ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
///   The position and the softness are left behind: the strip's `.mini` says
///   *which shape*, three numbers about that shape are an inspector row, and
///   the two the record needs are read where the record is written rather
///   than carried across this seam.
/// - The level is `Deck::level`, which is already `None` for every case
///   where a held reading would be about a different image. Its
///   `frames_behind` is not passed on — `view::Level` is where that argument
///   is written out, and the short of it is that the number means different
///   things on different loops and this loop is a `Fifo` one that never stops
///   asking for frames while anything is live.
/// - The name is [`material`], and it is this file's because a `Set` has
///   none. See `view::Strip::name`.
///
/// # Written into the `Vec` the view already holds
///
/// `out` is grown to the deck's slot count and then every field of every strip
/// is written, so nothing a `push` left behind is ever read. The name is the
/// one field that owns anything, and it is rewritten only when it differs —
/// which keeps this off the frame's allocation budget (ADR-0164) rather than
/// putting a `String` per strip on it every frame.
pub(crate) fn mixer(deck: &Deck, names: &[String], out: &mut Vec<view::Strip>) {
    out.truncate(deck.slot_count());
    while out.len() < deck.slot_count() {
        out.push(view::Strip {
            name: String::new(),
            tally: view::Tally::Allocated,
            requested: view::Tally::Allocated,
            gain: 0.0,
            gain_to: None,
            opacity: 0.0,
            opacity_to: None,
            blend: BlendMode::Add,
            mask: view::Mask::None,
            mask_angle: 0.0,
            level: None,
        });
    }
    for (slot, strip) in out.iter_mut().enumerate() {
        // **One name per slot, because a load moves one slot.** It was one
        // name for the whole deck while every slot ran the same pair and
        // nothing could change any of them; a library Set loaded into deck
        // B would then have left every strip reading the pair this program was
        // launched with, which is a readout that is wrong and says nothing
        // (P-0094). Empty for a slot nobody named, which draws no name at all
        // rather than somebody else's.
        let name = names.get(slot).map_or("", String::as_str);
        if strip.name != name {
            strip.name.clear();
            strip.name.push_str(name);
        }
        let slot = EngineSlot(slot as u8);
        strip.tally = tally(deck.residency(slot));
        strip.requested = tally(deck.requested_residency(slot));
        strip.gain = deck.gain(slot);
        strip.opacity = deck.opacity(slot);
        // **Where a scheduled move is taking each fader**, which is
        // `Deck::transitions_on` — *"what is moving on this slot, for a status
        // line"* — read for a surface instead. `karakuri-cli`'s line prints
        // `o>0.80` off the same call and for the same reason: with the default
        // quantum a fade is armed up to a bar before it is due, and a control
        // that changes something invisible is indistinguishable from one that
        // is broken.
        //
        // **At most one per control**, because `Deck::schedule` cancels
        // whatever was moving that pair before it pushes — so `find` is the
        // whole answer rather than the first of several.
        strip.gain_to = destination(deck, slot, Control::Gain);
        strip.opacity_to = destination(deck, slot, Control::Opacity);
        strip.blend = blend_mode(deck.blend(slot));
        strip.mask = masked(deck.mask(slot).kind());
        // **Read for the press and painted nowhere**, which is what
        // `view::Strip::mask_angle` is for: the chip asks for a shape and the
        // operation carries an angle, so the angle it carries is the one the
        // slot already has (ADR-0203). A harness that left this at zero would
        // make every press straighten a diagonal front, and nothing on the
        // panel would show it having happened.
        strip.mask_angle = deck.mask(slot).angle();
        strip.level = deck.level(slot).map(|level| view::Level {
            mean: level.mean,
            peak: level.peak,
        });
    }
}

/// Which of the four cells is showing a still, in slot order —
/// `view::View::overloaded`, which the caption reads.
///
/// A slot the watchdog stopped holds the frame it last drew and is not stepped
/// or drawn (ADR-0316), and a held frame of good material is indistinguishable
/// from material: the word under the cell is what says which it is (ADR-0269).
///
/// `false` past `slot_count`, not a panic. The row is `view::DECKS` cells
/// whatever the deck holds, so the fourth cell of a three-slot deck asks about
/// a slot that is not there — and *there is no slot* is the caption's own word
/// rather than a state of one (`view::PREVIEW_NO_SLOT`).
///
/// A function rather than four lines in the frame loop, so that the bound is
/// written once and can be named from a test.
pub(crate) fn stopped_slots(deck: &Deck) -> [bool; view::DECKS] {
    std::array::from_fn(|slot| slot < deck.slot_count() && deck.overloaded(EngineSlot(slot as u8)))
}

/// The engine's residency, as the console's word for it — and, like
/// [`blend_mode`], a `match` so that a fourth `Residency` stops the build here
/// rather than drawing a chip nothing can read.
///
/// One function for both halves of the pair. It was written inline for the
/// effective residency alone; the request needs exactly the same three arms,
/// and a second copy of them is a translation that can start disagreeing with
/// itself about what `Priming` is called.
pub(crate) fn tally(residency: Residency) -> view::Tally {
    match residency {
        Residency::Live => view::Tally::Live,
        Residency::Priming => view::Tally::Priming,
        Residency::Allocated => view::Tally::Allocated,
    }
}

/// The engine's blend mode, as the vocabulary's — and the one place the two
/// lists are made to agree.
///
/// `karakuri-operation` owns its own copy of every list a destination is drawn
/// from, which is the cost P-0090 says the vocabulary pays: *"The two rules —
/// be engine-neutral, and have no toggles — are not jointly satisfiable unless
/// the vocabulary owns the lists."* A copy needs somewhere the two meet, and
/// this is that place for this list, on the harness side of the seam — the same
/// side [`mixer`] reads a `Deck` from (ADR-0156).
///
/// A match, so the day a fourth mode lands in `karakuri_engine::deck::Blend`
/// this stops compiling. That is the whole reason `view::Strip::blend` is a
/// `BlendMode` and not the engine's word: a `&str` handed through would draw
/// the new mode's name on a chip, and the chip would cycle three ways past a
/// state no operation can name and no MIDI map can reach, with nothing saying
/// so. Failing here is the loud failure P-0094 asks for.
///
/// It stays this program's, and that is now settled rather than pending. The
/// console cannot depend on the engine (ADR-0156), and
/// `karakuri-operation-record` cannot either — it is the vocabulary and the
/// records and nothing else, by charter. So the two lists meet on the harness
/// side of the seam, wherever a harness holds both, and ADR-0180's *"one `From`
/// impl per list in `karakuri-cli`"* cannot be written at all: neither
/// [`Blend`] nor [`BlendMode`] is that package's, and the orphan rule refuses
/// it
/// ([ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).
pub(crate) fn blend_mode(blend: Blend) -> BlendMode {
    match blend {
        Blend::Add => BlendMode::Add,
        Blend::Over => BlendMode::Over,
        Blend::Max => BlendMode::Max,
    }
}

// **The blend cycle is `karakuri_console::view::after` and is not here.**
//
// `after_blend` stood here until 2026-09-10 and said the same three modes in
// the same order as the console's own cycle, with a test each. The key that
// asked it was `m`; under the grammar `space` on an addressed blend chip asks
// the console, and the console has always had the cycle because the *chip*
// needs it (ADR-0187, P-0090 — a toggle is an affordance built over operations
// by whoever draws the control). **So there is one cycle where there were
// two**, and `karakuri-console/tests/blend.rs` is what holds it: it walks
// `BlendMode::ALL` through the chip and asserts the wrap, which is exactly
// what the test deleted beside this function asserted from the other side
// (ADR-0333).
//
// `blend_mode` stays, and is what `holding` reads the deck through: the
// engine's three and the vocabulary's three are two crates' words for the same
// states, and a `match` is where they are made to agree.

/// What the deck is holding on `at`, for the three chips whose next state the
/// console names — or `None` where this deck has no slot there.
///
/// [`held`] is the guard, for its own reason: `Deck::blend` indexes its slots
/// and a panic reachable from an event handler aborts this process rather than
/// unwinding.
///
/// Read at the press and never off `view::Strip`, which is what the three mix
/// keys this replaces already said: a strip is this same reading copied once a
/// frame, and a scheduled fade landing between the frame and the press would
/// leave the cycle counting from a state the deck has left behind.
pub(crate) fn holding(deck: &Deck, at: u8) -> Option<focus::Held> {
    let slot = held(deck, at)?;
    Some(focus::Held {
        // **The residency the deck was last *asked* for**, which is what the
        // tally cycles from — `view::Mixer::tally`'s decision, and the one
        // that makes a parked slot's press a withdrawal rather than a
        // re-request.
        requested: tally(deck.requested_residency(slot)),
        blend: blend_mode(deck.blend(slot)),
        mask: masked(deck.mask(slot).kind()),
        // The angle the slot is already wearing, carried through unchanged
        // (ADR-0203).
        mask_angle: deck.mask(slot).angle(),
    })
}

/// The engine's mask shape as the console's, and the mirror image of
/// `karakuri_console::view::wipe_kind` on the way back out.
///
/// One function and two callers — [`mixer`] builds a strip from it every frame
/// and [`holding`] reads it at a press — because two copies of a three-arm
/// translation is exactly the shape that goes wrong the day a fourth shape
/// lands: a `match` with no wildcard stops the build in one place instead of
/// two.
pub(crate) fn masked(kind: MaskKind) -> view::Mask {
    match kind {
        MaskKind::None => view::Mask::None,
        MaskKind::Linear => view::Mask::Linear,
        MaskKind::Radial => view::Mask::Radial,
    }
}

/// One press of a gain key. Linear and additive, because a fader is: the same
/// press means the same amount wherever the trim is standing, rather than a
/// proportion of wherever it happens to be.
///
/// A tenth, because that is what the other keyboard steps by.
/// `docs/manual/operations.html` names the keys and says nothing about how far
/// a press goes, so the size comes from `karakuri-cli`'s own `GAIN_STEP` —
/// which `docs/manual.md` documents as *"focused slot gain down / up"* — and it
/// is copied rather than shared because neither binary may depend on the other
/// (ADR-0214). A page that decides otherwise moves this constant.
pub(crate) const GAIN_STEP: f32 = 0.1;

/// One press of an opacity key, and [`GAIN_STEP`]'s sentence one control along:
/// `karakuri-cli`'s `OPACITY_STEP`, which is the same tenth, and the console's
/// page is silent about this one too.
pub(crate) const OPACITY_STEP: f32 = 0.1;

/// Where a press takes the trim it is standing on.
///
/// [`offset_step`]'s shape one bay along, with the grammar's own word for a
/// direction in place of a letter: which way each press goes is a value this
/// file can be asked about without a window.
///
/// # What the page does not say, and where each answer comes from
///
/// The row names the keys and stops. So the size of a step and the destination
/// [`Step::Default`] names are `karakuri-cli`'s `'['`, `']'` and `'\\'` —
/// *"focused slot gain down / up / back to 1.0"* in `docs/manual.md` — taken
/// whole rather than invented here, because two keyboards that disagree about
/// how far one press goes is the one mistake an operator makes in the dark and
/// cannot see. The letters were this keyboard's too until 2026-09-10, and what
/// survived them is the arithmetic rather than the spelling.
///
/// # Floored and not ceilinged, and the clamp is the surface's
///
/// A negative gain would subtract one slot's light from another's, which is a
/// blend mode rather than a level; above 1.0 is ordinary, because the pipeline
/// is HDR
/// ([P-0064](../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md)).
/// It is clamped here rather than left to `Deck::set_gain` for `karakuri-cli`'s
/// `clamp_gain` reason: this decides what the *record* says, so a session
/// replays the value that took effect rather than one the engine quietly
/// corrected.
///
/// So [`Step::Default`] is a destination and the other two are steps, and all
/// three leave as the same absolute [`Operation::SetGain`] — an absolute value
/// can express every step and a step cannot express a setting.
///
/// It took the letter and takes the step since 2026-09-10. `[`, `]` and `\` are
/// unbound: the trim is reached by addressing it — `space` on the Mixer's strip
/// — and the arrows step it (ADR-0259, ADR-0333). The pair of directions and
/// the tenth between them are unchanged and are still `karakuri-cli`'s, which
/// is what the paragraphs above are about; what went is the letter that named
/// each one.
pub(crate) fn gain_key(step: Step, from: f32) -> f32 {
    let asked = match step {
        Step::Down => from - GAIN_STEP,
        Step::Up => from + GAIN_STEP,
        Step::Default => 1.0,
    };
    asked.max(0.0)
}

/// Where a press takes the fader it is standing on — [`gain_key`]'s function on
/// the other control.
///
/// The pair and the tenth between them are `karakuri-cli`'s, for the reason
/// written at [`gain_key`]: the page names the pair and not the direction, and
/// `;` down and `'` up were the letters until 2026-09-10.
///
/// The fader gains a default here and did not have one. The trim's `\` had no
/// partner on this control, so `space` on an addressed fader is the first way
/// back to unity it has ever had — ADR-0259's *"on a level, the one state worth
/// naming is the value it was declared at"*, which is the clause that record
/// buys with an argument rather than finds.
///
/// Held inside `[0, 1]` where the gain is only floored, which is the difference
/// the vocabulary already draws between the two: opacity is a proportion of a
/// blend and there is no such thing as 1.4 of one, where gain is a level into
/// an HDR mix. The clamp is this surface's for [`gain_key`]'s reason — it
/// decides what the record says.
pub(crate) fn opacity_key(step: Step, from: f32) -> f32 {
    let asked = match step {
        Step::Down => from - OPACITY_STEP,
        Step::Up => from + OPACITY_STEP,
        Step::Default => 1.0,
    };
    asked.clamp(0.0, 1.0)
}

/// Where a press takes the master out — [`gain_key`]'s function one bay down,
/// and the three answers are the same three.
///
/// A tenth, and the same tenth: the trim, the fader and this are one gesture on
/// three controls, and a keyboard that stepped each of them by a different
/// amount would be three keyboards. Held inside `[0, 1]` where the gain is only
/// floored, which is [`opacity_key`]'s distinction met on the level the whole
/// programme leaves through: `Knob::Out` drags over exactly that range, so a
/// key and a hand can reach the same values and no others.
///
/// The default is unity, which is where a run starts and what the row reads
/// before anybody has touched it.
pub(crate) fn out_key(step: Step, from: f32) -> f32 {
    let asked = match step {
        Step::Down => from - OPACITY_STEP,
        Step::Up => from + OPACITY_STEP,
        Step::Default => 1.0,
    };
    asked.clamp(0.0, 1.0)
}

/// The slot a press or a record names, as an index this deck has, or `None`
/// where it has not got one.
///
/// `Deck::gain` and `Deck::set_gain` index their slots, and a panic reachable
/// from an event handler aborts this process rather than unwinding (see the
/// module documentation), so every route from a letter or a record to the deck
/// asks this first. `mix::change`'s whole reason for taking a `slot_count` is
/// that a stream may name a slot that is not there.
///
/// Nothing in this file can produce one: the strips are the deck's own count,
/// and `View::select` refuses a deck the mixer draws no strip for — so this is
/// the guard rather than the message, and the real sentence is `karakuri-cli`'s
/// `no_such_slot`.
///
/// One derivation and not one per caller, which is what makes the key arms and
/// [`apply`] refuse the same slot: a press reads the deck before it names a
/// destination and the record writes it afterwards, and a guard on only the
/// second of the two would be a read that panicked on its way to a refusal.
pub(crate) fn held(deck: &Deck, slot: u8) -> Option<EngineSlot> {
    EngineSlot::new(slot, deck.slot_count())
}
