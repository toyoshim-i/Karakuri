use super::*;

pub mod strip;
pub mod transition;

pub use strip::*;
pub(crate) use strip::{next, next_shape, residency, wipe_kind};
pub use transition::*;

/// Where a slot sits between compiled and composited, which is the mock's
/// `.tally` and `karakuri_engine`'s `Residency` — three states and no fourth.
///
/// The words are the mock's abbreviations and the variants are the engine's
/// names, because each document owns one end: `live`, `prim` and `alloc` are
/// what fits in a strip 53 wide, and `Live`, `Priming` and `Allocated` are what
/// `Deck::residency` answers.
///
/// The mock drew a fourth and it was not a residency. Its `.strip.empty` read
/// `empty` through a `.tally.off` rule, and a `Deck` has no such slot: every
/// one of its `slot_count` slots holds a `HotSwap`. An empty strip is a *track
/// on the page nothing fills*, and this bay draws nothing at all in one rather
/// than a strip full of dashes — see [`mixer`].
///
/// The page has since agreed: `39f1e6b` took that strip out under ADR-0178, and
pub use super::widgets::chip::{centre_galley, tally_job, Tally};

// ---------------------------------------------------------------------------
// The Mixer bay
// ---------------------------------------------------------------------------

/// The word at the head of the Mixer bay, in the source's own capitalisation
/// for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and that is done at
/// paint time so the word a reader searches for is the word in the source.
pub(super) const MIXER_TITLE: &str = "Mixer";

/// The mixer's strips, laid out: the values, and where each one goes.
///
/// # It borrows the values rather than carrying a copy
///
/// [`TransportRow`] carries the [`Transport`] it was measured from, for
/// [`Picture`]'s reason: whoever measured the type and whoever paints it are
/// then one statement, so a row laid out for one value and painted with another
/// cannot be written by accident. A [`Strip`] carries a name, so it is not
/// `Copy` and a copy per frame would be a `String` allocated per strip per
/// frame. The borrow says the same thing for nothing — these boxes were
/// measured from *these* strips — and the compiler holds it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mixer<'a> {
    /// The values these rectangles were measured from, in slot order.
    pub strips: &'a [Strip],
    /// Where each of them is. `None` past the last strip: a track on the page no
    /// deck fills, which is drawn as nothing at all.
    boxes: [Option<StripBox>; DECKS],
}

impl<'a> Mixer<'a> {
    /// How many strips there are — the deck's slot count, as far as one page of
    /// this bay reaches.
    pub fn count(&self) -> usize {
        self.boxes.iter().filter(|at| at.is_some()).count()
    }

    /// One strip's furniture. Panics on a strip this bay has not got, which is a
    /// caller having invented a slot — the rule [`TransportRow::dot`] states about
    /// a dot of a grid.
    pub fn strip(&self, index: usize) -> StripBox {
        let count = self.count();
        self.boxes
            .get(index)
            .copied()
            .flatten()
            .unwrap_or_else(|| panic!("strip {index} of a mixer of {count}"))
    }

    /// What a press at `p` takes hold of, or `None` where there is nothing under it
    /// that a hand can move.
    ///
    /// # It is the knob, and the track is deliberately not a target
    ///
    /// A press on the track, off the knob, does nothing. A fader at 0.3 whose top
    /// is clicked would jump to 1.0 — a change to the mix nobody asked for, made on
    /// stage — and this bay's controls are played during a performance. So the
    /// answer is `None` there, and it is a decision rather than a hit test that
    /// stops at the knob by accident.
    ///
    /// It also decides who the *event* belongs to, since [`crate::input::claim`]'s
    /// rule 3 asks this: the panel claims what it acts on, so a press on a track
    /// goes to `egui`, which owns no widget there and does nothing with it — which
    /// is the same nothing, arrived at without the panel claiming a press it would
    /// throw away.
    ///
    /// # One derivation, asked twice
    ///
    /// [`crate::input::claim`] asks this and so does the caller that acts on the
    /// press, exactly as [`Outputs::op`] is asked after [`Outputs::hit`]
    /// (ADR-0176). Two copies of *where the knob is* is a control drawn where it
    /// cannot be grabbed, with nothing on screen saying so.
    ///
    /// The value is part of the geometry here, which the Outputs chip does not have
    /// to deal with: the knob sits on the fill's moving edge, so where it is
    /// depends on what the deck said this frame. That is the same [`Strip`] the bay
    /// was laid out from, so the knob a hand sees and the knob it grabs are the
    /// same one.
    pub fn grab(&self, p: karakuri_layout::Point) -> Option<Grab> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                let at = at?;
                // The manual's *deck* is the code's *slot*, and a deck holds
                // `MAX_SLOTS` of them — `DECKS`, which is 4 — so the index is
                // a `u8` with room to spare. See `Knob::operation`.
                let deck = index as u8;
                grabbed(at.trim_at(strip.gain), Knob::Trim { deck }, p)
                    .or_else(|| grabbed(at.fader_at(strip.opacity), Knob::Fader { deck }, p))
            })
    }

    /// What a press at `p` asks the blend to become, or `None` where there is no
    /// blend chip under it.
    ///
    /// # The chip cycles, and the operation names where it arrived
    ///
    /// Click it and the deck's blend moves to the next of [`BlendMode::ALL`],
    /// wrapping from the last back to the first. What comes out is
    /// [`Operation::SetBlendMode`] naming the destination — never a step, because
    /// there is no step in the vocabulary to name.
    ///
    /// That is the affordance P-0090 leaves to whoever draws the control rather
    /// than an exception to it: a toggle is built over operations by whoever draws
    /// them, and a mini that cycles the blend is one control emitting three — the
    /// operator sees a toggle and the vocabulary never does. The cycle is
    /// [`after`], which is four lines in this file and nothing at all in
    /// `karakuri-operation`.
    ///
    /// What a MIDI map is offered is the three values, not the cycle —
    /// [ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md),
    /// which is the decision this affordance forces and the reason it is recorded
    /// at all.
    ///
    /// # One derivation, asked twice, and the whole chip is the target
    ///
    /// [`crate::input::claim`]'s rule 3 asks this and so does the caller that acts
    /// on the press — the arrangement [`Outputs::op`] and [`Mixer::grab`] both
    /// have, where the derivation that claims a press is asked again rather than
    /// copied. [`StripBox::blend`] is the chip's own rectangle, the one the word is
    /// painted into, so a chip a hand sees and a chip it clicks are the same one.
    ///
    /// The whole chip is the target and not the glyphs in it, which is
    /// [`Outputs::sink`]'s rule one bay along: a 15px word is not something a hand
    /// finds, and `.mini`'s padding is what makes it one.
    ///
    /// # It names the strip's own deck
    ///
    /// The slot index, cast the way [`Mixer::grab`] casts it — the manual's *deck*
    /// is the code's *slot* (ADR-0180), and a deck holds `MAX_SLOTS` of them, so
    /// the index is a `u8` with room to spare.
    pub fn blend(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.blend.contains(p))
                    .map(|_| Operation::SetBlendMode {
                        deck: index as u8,
                        blend: after(strip.blend),
                    })
            })
    }

    /// What a press at `p` asks the deck's solo state to become, or `None` where
    /// there is no solo button under it.
    pub fn solo(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (_, at))| {
                at.filter(|at| at.solo.contains(p))
                    .map(|_| Operation::ToggleSolo { deck: index as u8 })
            })
    }

    /// What a press at `p` asks the deck's mute state to become, or `None` where
    /// there is no mute button under it.
    pub fn mute(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (_, at))| {
                at.filter(|at| at.mute.contains(p))
                    .map(|_| Operation::ToggleMute { deck: index as u8 })
            })
    }

    /// What a press at `p` asks the residency to become, or `None` where there is
    /// no tally chip under it.
    ///
    /// # The chip cycles, and it cycles from what was *requested*
    ///
    /// Click it and the deck is asked for the next of [`Tally::ALL`] — `live`,
    /// `prim`, `alloc`, wrapping — and what comes out is
    /// [`Operation::SetResidency`] naming that destination. The affordance is the
    /// blend chip's ([`Mixer::blend`], ADR-0187) and so is the division it rests
    /// on: the cycle is [`next`] here and nothing at all in `karakuri-operation`,
    /// which is P-0090's division: a toggle is an affordance, built over operations
    /// by whoever draws the control.
    ///
    /// The step is taken from [`Strip::requested`] and not from [`Strip::tally`],
    /// and that is the decision rather than a detail. The two disagree exactly
    /// while a request has not landed, and cycling from the request is what makes
    /// that case come out right with no case in the code for it: a parked slot's
    /// request is `Priming`, so the next is `Allocated` — which *is* the withdrawal
    /// of the prime request, said by the ordinary arithmetic. Cycling from the
    /// effective residency would answer `Live` there, and a press meant to take a
    /// request back would put the deck on air.
    ///
    /// That is what
    /// [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// permits a surface: *"a press on a control whose transition is pending may
    /// ask for the withdrawal"* — a control choosing which destination a press
    /// names, arrived at out of one rule rather than a branch, and not a lock. The
    /// chip refuses nothing; every request is handed over and the engine decides.
    ///
    /// It is also what `karakuri-cli`'s `w` already does: `toggle_priming` reads
    /// `Deck::requested_residency` to choose its direction, so a parked slot's `w`
    /// withdraws rather than re-asking. Two surfaces reading different halves of
    /// the pair would disagree about what a press means on exactly the slots where
    /// it matters.
    ///
    /// # One derivation, asked twice, and the whole chip is the target
    ///
    /// [`crate::input::claim`]'s rule 3 asks this and so does the caller that acts
    /// on the press — [`Outputs::op`], [`Mixer::grab`] and [`Mixer::blend`] are the
    /// same arrangement. [`StripBox::tally`] is the capsule the word is painted
    /// into, and it is the widest of the three words whatever it is showing
    /// ([`mixer`]), so this target does not move when the deck moves under it and
    /// does not move while a word is rolling through it — which the blend chip,
    /// sized to the word it shows, cannot say.
    ///
    /// # It names the strip's own deck
    ///
    /// The slot index, cast the way [`Mixer::grab`] and [`Mixer::blend`] cast it —
    /// the manual's *deck* is the code's *slot* (ADR-0180), and a deck holds
    /// `MAX_SLOTS` of them, so the index is a `u8` with room to spare.
    pub fn tally(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.tally.contains(p))
                    .map(|_| Operation::SetResidency {
                        deck: index as u8,
                        residency: residency(next(strip.requested)),
                    })
            })
    }

    /// What a press at `p` asks the mask's shape to become, or `None` where there
    /// is no mask mini under it.
    ///
    /// # The chip cycles, and the operation names where it arrived
    ///
    /// Click it and the deck's mask moves to the next shape — none, linear, radial,
    /// wrapping — and what comes out is [`Operation::SetMaskShape`] naming that
    /// destination. The affordance is the blend chip's ([`Mixer::blend`], ADR-0187)
    /// and the tally's (ADR-0195), and so is the division under it: the cycle is
    /// [`next_shape`] here and nothing at all in `karakuri-operation`, which is
    /// P-0090's division: a toggle is an affordance, built over operations by
    /// whoever draws the control.
    ///
    /// The order starts at `None`, which is
    /// `karakuri_engine::deck::MaskKind::ALL`'s and is written down there: *"`None`
    /// first, because it is the default and a cycle should start where a slot
    /// starts."* This crate has no engine (ADR-0156), so the order is restated in
    /// [`next_shape`] rather than read from it.
    ///
    /// # It carries the angle it does not control, and that is the decision
    ///
    /// [`Operation::SetMaskShape`] is a shape and an angle, and this chip names
    /// only the shape — *"`.mini` is a chip that says which shape, and three
    /// numbers about that shape are the inspector's row, not this one."* So the
    /// angle handed back is [`Strip::mask_angle`], the one the slot is already
    /// wearing: a press chooses a shape and changes nothing else. Sending `0.0`
    /// would make choosing a shape straighten a diagonal front, which is a surface
    /// asking for a change nobody made — see
    /// [ADR-0203](../../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md).
    ///
    /// The position and the softness are not here at all. The operation does not
    /// name them, and the record that does is filled in from a reading of the
    /// running mask where the record is written (ADR-0201). A surface carrying a
    /// value no operation asks for would be this crate keeping half a deck.
    ///
    /// # One derivation, asked twice, and the whole chip is the target
    ///
    /// [`crate::input::claim`]'s rule 3 asks this and so does the caller that acts
    /// on the press — [`Outputs::op`], [`Mixer::grab`], [`Mixer::blend`] and
    /// [`Mixer::tally`] are the same arrangement. [`StripBox::mask`] is the chip's
    /// own rectangle, the one the mark is drawn into, and it is [`size::MINI_SIZE`]
    /// wide inside `.mini`'s padding whichever shape it is showing — so this
    /// target, like the tally's capsule and unlike the blend chip's, does not move
    /// under the value it draws.
    ///
    /// # It names the strip's own deck
    ///
    /// The slot index, cast the way [`Mixer::grab`], [`Mixer::blend`] and
    /// [`Mixer::tally`] cast it — the manual's *deck* is the code's *slot*
    /// (ADR-0180), and a deck holds `MAX_SLOTS` of them, so the index is a `u8`
    /// with room to spare.
    pub fn mask(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.mask.contains(p))
                    .map(|_| Operation::SetMaskShape {
                        deck: index as u8,
                        kind: wipe_kind(next_shape(strip.mask)),
                        angle: strip.mask_angle,
                    })
            })
    }

    /// Which deck a press at `p` selects, or `None` where no strip is under it.
    ///
    /// # The strip is the control, and it is the last question this bay asks
    ///
    /// A strip's rectangle contains the trim, the fader, the two chips and the mask
    /// mini, so this would answer for a press on any of them if it were asked
    /// first. It is asked last: whoever routes a press tries the four questions
    /// that name something inside the column, and this is what is left over — a
    /// press on the strip's name, on its number, on the ground between its rows.
    /// That is the affordance `console.html` states — *"a press anywhere on a strip
    /// that no knob under the pointer claimed"* — and it is why the bay needs no
    /// sixth control drawn to carry it.
    ///
    /// It is one of rule 4's probes like every other control here, and it is asked
    /// twice for the same reason they are: once by [`crate::input::claim`] to
    /// decide the press is the panel's, and once by whoever acts on it. It went
    /// eight days claimed by nobody — `input::on_strip` asked the four inside the
    /// column and not this one, so the press reached `egui`, which owns no widget
    /// on the console and did nothing with it. `tests/mixer.rs` is what says
    /// otherwise now.
    ///
    /// [`Operation::SelectDeck`] writes no record and is the console's own pointer,
    /// so whoever emits it performs it: there is nothing on the deck for it to
    /// move. What it moves is [`View::selection`], which the ring above and the
    /// Library bay's pill both read.
    pub fn select(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.deck_at(p).map(|deck| Operation::SelectDeck { deck })
    }

    /// Which deck a carry let go at `p` lands on, or `None` where no strip is under
    /// it — which is a drop that asks for nothing at all
    /// ([`crate::panel::Released::Nowhere`]).
    ///
    /// # It is a release where every other question in this bay is a press
    ///
    /// [`Mixer::grab`] and the four beside it answer *what does a press here ask
    /// for*, and they are asked twice — once by [`crate::input::claim`] to decide
    /// whose event it is, and once by whoever acts on it. Nothing asks this one
    /// first: a carry already holds the pointer under `claim`'s rule 1, so there is
    /// no claim to decide, and what is left is the second ask on its own. What that
    /// buys is the destination resolved against the geometry the frame *last drew*
    /// rather than against the geometry the press was made on — the strips can have
    /// moved under a carry that took a boundary with it on the way, and where the
    /// row lands is where the strip is now.
    ///
    /// The whole strip, where [`Mixer::select`] is what is left over. The two
    /// answer off one derivation ([`Mixer::deck_at`]) and differ in nothing else,
    /// and that is the point: a press is offered the strip only after the knobs and
    /// chips inside it have declined, because a press on a knob is a press on that
    /// knob — but a Set let go over a knob is a Set let go over that deck, since
    /// none of the five controls is a place a Set could go instead. So this is
    /// asked of no other question first.
    ///
    /// It reads no residency and no reading of any kind. A drop on a live deck asks
    /// for the load and the instrument decides
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md),
    /// and `console.html`'s *"Nothing refuses it"*), so [`Strip::tally`] is not
    /// consulted here and a strip's values reach this only as the rectangle they
    /// were laid out into.
    ///
    /// It is one of two answers now, and never both. The four deck preview cells
    /// take the drop as well ([`ProgramBay::dropped`]), and the two bays are in two
    /// regions of the panel — so a point is inside one set of rectangles or the
    /// other or neither, and *at most one rectangle is marked* is a fact about
    /// where the pointer is rather than a rule either of them enforces
    /// ([ADR-0273](../../../../docs/adr/0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md)).
    pub fn dropped(&self, p: karakuri_layout::Point) -> Option<u8> {
        self.deck_at(p)
    }

    /// Which strip's rectangle `p` is inside, as a deck.
    ///
    /// Private, and the one derivation [`Mixer::select`] and [`Mixer::dropped`]
    /// both read: *which column is the pointer in* is one question, and two
    /// spellings of it would be a press that selects one deck and a drop that loads
    /// another from the same point. The manual's *deck* is the code's *slot* and a
    /// deck holds `MAX_SLOTS` of them, so the index is a `u8` with room to spare —
    /// [`Mixer::grab`]'s note.
    fn deck_at(&self, p: karakuri_layout::Point) -> Option<u8> {
        let p = Pos2::new(p.x, p.y);
        self.boxes
            .iter()
            .enumerate()
            .find_map(|(index, at)| at.filter(|at| at.rect.contains(p)).map(|_| index as u8))
    }

    /// Where the deck selection's ring goes, and `None` for a selection this bay
    /// has no strip for.
    ///
    /// The one box in this bay that is the strip's *whole* rectangle rather than
    /// something inside it, because what the selection addresses is the deck and
    /// not any one of the six readings in the column. Answered from the same
    /// `boxes` every other rectangle here comes off, so the ring is painted round
    /// the strip a press on that track would select ([`crate::input::claim`]) and
    /// never round a neighbour.
    pub fn selected(&self, deck: u8) -> Option<Rect> {
        self.boxes
            .get(usize::from(deck))
            .copied()
            .flatten()
            .map(|at| at.rect)
    }

    /// Every strip and its box, in slot order.
    pub fn placed(&self) -> impl Iterator<Item = (&'a Strip, StripBox)> {
        let boxes = self.boxes;
        self.strips
            .iter()
            .zip(boxes)
            .filter_map(|(strip, at)| at.map(|at| (strip, at)))
    }
}

/// The Mixer bay's strips, derived: one per slot the deck has, and none at
/// all where there is no deck.
///
/// # The page has [`DECKS`] tracks whatever the deck holds
///
/// `.mixer-strips` is `grid-template-columns: repeat(4, 1fr)`, and that four
/// is the same four [`DECKS`] is — `MAX_SLOTS` is 4, so no deck can fill a
/// fifth. A strip is one of four tracks wide even where there is one
/// strip, and that is read off the arrangement rather than chosen here: the
/// right pane's minimum width is written as *"four mixer strips still side by
/// side … four of them with three 4px gaps inside `.mixer-strips`' 6 + 6 is
/// 172"*. Tracks that followed the strip *count* would make a one-slot deck's
/// strip 244 wide in a pane sized for four 61-wide ones, and would re-derive
/// that minimum every time a slot was installed.
///
/// So a track with no strip in it draws nothing at all — not an empty
/// strip. That is the opposite of what [`preview`] does with a cell that has
/// no slot behind it, and the two are not in tension: a preview cell is the
/// region's own face and its caption says `no slot`, where a strip is six
/// readings and an empty one is six readings nobody took. The mock drew such a
/// strip — `.strip.empty`, with `—` for a name, `empty` for a tally and both
/// tracks bare — until `39f1e6b` took it out under ADR-0178; inventing one
/// here would be ADR-0177's row of zeroes with a different glyph, which is why
/// the strip going does not make this rule the mock's rather than the
/// record's.
///
/// The example's deck has one slot, so it draws one strip, and that is the
/// example rather than a gap in it — exactly as three of its preview cells
/// read `no slot`.
///
/// # What is in the mock's bay and is deliberately not here
///
/// - The head's `3 of 3 · page 1`. *"The strip is a paged list whose
///   length is a number, the header says how long the list is, and it says
///   which page you are on"* — so both halves of that pill are about paging,
///   and there is none. Every strip the deck has is drawn, on the one page, so
///   the pill could only ever read `n of n · page 1`: two numbers that are
///   always equal and a third that is always 1. That is [`Kind::Bay`]'s own
///   rule about a pill stating a value the console does not have, and the
///   number that *is* known — how many
///   strips there are — is on the face of the bay already. It arrives with
///   paging.
/// - `.xfade`, the transition row under the strips. It is drawn now, and
///   it is [`transition`] rather than this: the three settings are the
///   console's own pointer and not six readings a slot, so the row survives a
///   console with no deck behind it where these strips do not. The A/B
///   track that used to sit above it is not a row this console owes: the
///   mixer has no crossfader, and `console.html`'s *The mixer has no
///   crossfader* is the argument. The block is 37.5 of the mock's bay
///   ([`size::XFADE_H`], which is what this console now carves) and the bay
///   still reserves 61 for it — the 16.5 row and the 7 gap the crossfader
///   took with it. That height has still not been re-derived, and
///   `tests/mixer.rs` now states the leftover in the row's own terms rather
///   than as a subtraction.
/// - `.wfocus`, which is the second of the mock's two focuses: keyboard
///   focus, transient, wherever tab lands. The mock draws it as a dashed sun
///   outline and the deck selection as a solid lavender ring, on purpose,
///   because *"Drawing them the same way would erase which of the two a reader
///   is looking at"*. The selection exists here now and is drawn —
///   [`View::selection`], and [`mixer_into`] for the ring — which is the
///   sentence ADR-0219 recorded as owed. Keyboard focus does not: nothing in
///   this console takes it, so a dashed outline would be drawn around a state
///   that is not kept.
/// - Every tooltip. Four of this bay's controls carry one, and a tooltip
///   needs `egui` to own a widget — the sentence [`outputs`] writes about a
///   control, one bay along.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. `ctx` is asked for the type, because the tally's capsule and the
/// blend's mini are as wide as the words in them.
pub fn mixer<'a>(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    strips: &'a [Strip],
) -> Option<Mixer<'a>> {
    // **No deck behind the console, so there are no strips.** Every test in
    // this crate is here, and so is the whole of `cargo test -p
    // karakuri-console`. Drawing four empty strips would be inventing six
    // readings a slot; this is `View::picture`'s rule, one bay along.
    if strips.is_empty() {
        return None;
    }
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // `outputs` and `transport`.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let region = to_egui(layout.rect(layout.find("mixer")?));
    let row = strips_row(region)?;
    let width = |job: LayoutJob| ctx.fonts_mut(|f| f.layout_job(job).size().x);
    let label = width(span_at(
        TRIM_LABEL,
        size::TRIM_LABEL_SIZE,
        Color32::PLACEHOLDER,
    ));
    // **The chip is as wide as the widest residency, not as the one it is
    // showing.** Measured once for the bay rather than per strip: it is the
    // same three words in every strip, so a per-strip measurement would be
    // three galley layouts a strip for one answer.
    //
    // It read `strip.tally` alone until the tally learned to say that a
    // request had not landed, and that was a defect rather than a
    // simplification: a chip sized to the current word is a capsule that
    // changes width when the deck moves under it, and one that changes width
    // *while a word is rolling through it* would be a control resizing on its
    // own animation. What the mock does not have is the reason it survived —
    // a still page draws each chip once, so shrink-to-fit and this are the
    // same picture there and only one of them is the same picture over time.
    let tally = Tally::ALL
        .into_iter()
        .map(|tally| width(tally_job(tally, Color32::PLACEHOLDER)))
        .fold(0.0f32, f32::max);
    let mut boxes = [None; DECKS];
    for (index, strip) in strips.iter().take(DECKS).enumerate() {
        let blend = width(span_at(
            strip.blend.name(),
            size::MINI_SIZE,
            Color32::PLACEHOLDER,
        ));
        boxes[index] = strip_box(
            track(row, DECKS, index, size::STRIP_GAP, Axis::Row),
            label,
            tally,
            blend,
        );
    }
    Some(Mixer { strips, boxes })
}

/// The Mixer bay's strips, painted.
///
/// Where everything goes is [`mixer`]'s, so this paints and derives nothing.
///
/// `marked` is the strip a carried Set would land on, and it is handed in
/// rather than asked here for the reason `selection` is: it is a *pointer*, and
/// one of the two the panel holds. [`View::draw`] resolves it off
/// [`Mixer::dropped`] — the derivation the release asks — so the ring is
/// painted round the strip that release would name and never round a neighbour.
pub(super) fn mixer_into(
    ui: &Ui,
    pal: &Palette,
    mixer: &Mixer,
    phase: Phase,
    selection: u8,
    marked: Option<u8>,
) {
    for (strip, at) in mixer.placed() {
        strip_into(ui, pal, strip, at, phase);
    }

    // `.strip.drop` — `outline: 2px solid var(--c-text)` at `outline-offset:
    // 0`, the rectangle a Set in hand lands on if it is let go here. Drawn
    // **before** the selection below and outside the strip's own edge where
    // that one is inset, which is how one strip wears both at once — the mock
    // draws deck A wearing exactly that pair.
    if let Some(rect) = marked.and_then(|deck| mixer.selected(deck)) {
        drop_ring(ui, pal, rect, size::STRIP_RADIUS);
    }
    // `.strip.focus` — `box-shadow: inset 0 0 0 2px var(--c-lav)`, the deck
    // selection, drawn **after every strip** because an inset shadow is over a
    // strip's contents and not under them. It is a solid ring where the mock's
    // keyboard focus is a dashed outline, which is `console.html`'s *Two
    // focuses, and they do not look alike*: drawn the same way, a reader could
    // not tell which of the two they were looking at. Nothing draws the dashed
    // one — this console takes no keyboard focus.
    //
    // `None` where the selection names a slot this bay is not drawing, which
    // [`View::select`] refuses at the source and this answers again because a
    // rectangle is what it has: a bay one frame behind a deck that lost a slot
    // would otherwise ring a track no strip is in.
    if let Some(rect) = mixer.selected(selection) {
        ui.painter().with_clip_rect(rect).rect_stroke(
            rect,
            CornerRadius::same(size::STRIP_RADIUS as u8),
            Stroke::new(size::STRIP_FOCUS_RING, pal.lav),
            StrokeKind::Inside,
        );
    }
}

impl View {
    /// What the next fade, crossfade or wipe means — see [`View::transition`] the
    /// field, which is where the argument is.
    ///
    /// Read back bare, where [`View::cursor_row`] and [`View::scope`] are both
    /// answered against a listing: there is no listing under this one to fall out
    /// of, because the three cycles are the console's own and
    /// [`View::set_transition`] refuses anything that is not on them.
    pub fn transition(&self) -> TransitionSettings {
        self.transition
    }

    /// Take one of the three settings, and answer whether that moved anything.
    ///
    /// A setting no pill can draw is refused, which is [`View::select`]'s rule and
    /// the same reasoning: the row is three capsules and each reads the word its
    /// cycle gives the value it is on, so a shape or a beat count off the cycle
    /// would be a pill with nothing to say — and, once it was there, a press on it
    /// stepping from wherever the fallback in [`TransitionSettings::shape_at`]
    /// landed rather than from what the pill reads. The cycles are [`WIPE_SHAPES`],
    /// [`QUANTA`] and [`FADE_BEATS`], and each is a *curation*: an angle between
    /// two of them is a value the operation can carry and this row cannot show
    /// (`karakuri_operation::WipeKind`'s own *"a dial with nowhere to show its
    /// value is a control an operator cannot read"*).
    ///
    /// It refuses rather than clamping, for [`View::select`]'s reason: the nearest
    /// curated angle is not the angle that was asked for, and a wipe that ran at it
    /// would be a move nobody asked for made on stage.
    ///
    /// This is not a lock, and P-0090 is what says so. What may be asked of the
    /// *instrument* is decided where the record is applied; nothing is applied here
    /// at all. The console is the model of record for these three
    /// ([`View::transition`] the field), and a model of record refusing a value it
    /// cannot hold is not a surface deciding what may be asked.
    ///
    /// The `bool` is [`View::select`]'s: a caller repaints on a move and not on a
    /// press, so a press that changed nothing costs no frame (P-0091).
    pub fn set_transition(&mut self, setting: TransitionSetting) -> bool {
        self.transition.take(setting)
    }

    /// What the mixer bay declares: the roll's staleness while anything in it is
    /// pending *and* the bay is laid out, and nothing otherwise — with the deadline
    /// it asks for taken from where the roll has got to rather than from the fact
    /// that something is pending.
    ///
    /// Three pending things, one rate — see [`View::declares`], which carries the
    /// whole argument.
    ///
    /// # Pending is when it declares; moving is when it asks for a frame
    ///
    /// The three presentations in this bay are one curve ([`roll_at`]) off one
    /// [`Phase`], and that curve is exactly zero for the 600 ms of every
    /// [`ROLL_PERIOD`] that is not [`ROLL_TRAVEL`]. So *is anything pending* is the
    /// right question for whether this region is live — it is what separates a
    /// strip that has somewhere to go from one that has arrived — and it is the
    /// wrong question for whether a frame is owed thirty milliseconds from now.
    /// [`roll_moves_in`] answers the second one, and
    /// [ADR-0283](../../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)
    /// is where the two are separated.
    ///
    /// The region stays in both sums throughout, rest included: a parked slot is a
    /// live region for as long as it is parked, and a schedule admitted on the 40%
    /// of the period that moves would be a schedule that could not afford the thing
    /// it admitted.
    ///
    /// # The level meter is in this bay and is not in this declaration
    ///
    /// [`Strip::level`] is the one thing on a strip that is a *measurement* rather
    /// than a setting, and it moves on every frame the engine renders — so *nothing
    /// in this bay changes while the roll rests* is a sentence about the roll and
    /// not about the bay. It declares nothing anyway, and the reason is where the
    /// reading comes from: `Deck::level` moves inside `Deck::begin_frame`, which is
    /// the only caller of `Meters::collect`, and a caller calls it once per
    /// composed frame. The meter's picture is therefore a function of the frames
    /// this panel is drawn on rather than of wall time, and there is no moment
    /// between two frames at which what is on screen is not the newest reading
    /// taken — because no reading is taken between two frames.
    ///
    /// That is what separates it from [`roll_at`], which is different at 400 ms
    /// from what it was at 399 whether or not anybody drew anything and can
    /// therefore say when it next moves. It is also what separates it from the
    /// beat, which moves per composed frame as well and declares anyway on P-0094's
    /// forced clause: *something is moving continuously while the console is live*.
    /// There is no such clause for a meter, and a deadline for one would be asking
    /// for the frames that produce the readings it would then draw — measured in
    /// [ADR-0290](../../../../docs/adr/0290-the-level-meter-moves-only-when-a-frame-is-drawn-so-it-declares-nothing.md),
    /// which is where the alternatives are, and held in `tests/metered.rs`.
    ///
    /// What would change it is ballistics. A peak that is held and decays, or a
    /// fill that falls at a rate rather than following the reading, is a function
    /// of the clock exactly as the roll is: it would move between two frames, and
    /// it would then have to declare. `tests/metered.rs` asserts that premise — the
    /// meter's geometry is a function of the reading and of nothing else — so the
    /// day a meter grows a fall time is a test failure rather than a readout that
    /// freezes with nothing saying so.
    pub(super) fn mixer_declares(&self, layout: &karakuri_layout::Layout) -> Option<Declared> {
        let bay = layout.find("mixer").is_some_and(|id| layout.visible(id));
        if !bay {
            return None;
        }
        if self.mixer_dirty {
            return Some(Declared {
                region: "mixer",
                cost: PANEL_PASS,
                staleness: ROLL_STALENESS,
                moves_in: Duration::ZERO,
            });
        }
        let moving = self.mixer.iter().any(|strip| {
            strip.pending().is_some()
                || strip.gain_pending().is_some()
                || strip.opacity_pending().is_some()
        });
        moving.then_some(Declared {
            region: "mixer",
            cost: PANEL_PASS,
            staleness: ROLL_STALENESS,
            moves_in: roll_moves_in(self.phase),
        })
    }
}
