use super::*;

// ---------------------------------------------------------------------------
// The Mixer bay's transition row
// ---------------------------------------------------------------------------

/// What the next scheduled move means: the shape a wipe's front takes and which
/// way it runs, the musical grid the move starts on, and how long it lasts.
///
/// # It is the console's own model of record, and that is not a convenience
///
/// Every other value this bay draws is a reading handed in by whoever owns the
/// deck. These three are not, and [`Operation::SetTransition`] is why: it
/// *"changes nothing you can see and writes nothing to the stream"*, it is
/// `Written::Silent(Silent::Surface)`, and no record in `karakuri-store`
/// carries any of the three. So there is nothing downstream that could be asked
/// what the quantum is, and a host that kept a copy would be keeping the
/// console's state on its behalf — which is [`View::selection`]'s argument
/// arriving at a fourth pointer.
///
/// What reads it is the wipe. `karakuri_operation_record::Current::transition`
/// wants the quantum, the length and the front's shape together, and
/// [`Operation::Wipe`] is converted against them — so this is what a host hands
/// that conversion, and `karakuri-cli` holds the same four numbers as
/// `quantum`, `fade_beats`, `mask_kind` and `mask_angle` for the same reason.
///
/// # One value and not four fields on [`View`]
///
/// [`Operation::SetTransition`] is one operation with a three-armed payload,
/// and the manual has the row as one heading — *"This row is three operations
/// and the manual has it as one"*, which is `TransitionSetting`'s own sentence.
/// Four loose fields would be that sum taken apart in the one crate that draws
/// the row it is a sum for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionSettings {
    /// The shape the next wipe's front takes. [`WipeKind::None`] is *no shape*,
    /// under which `karakuri-cli`'s `c` is refused — which is the vocabulary's own
    /// sentence at that variant, not a rule this row holds.
    pub kind: WipeKind,
    /// Which way a linear front runs, in radians.
    ///
    /// Its own field beside the kind, because the operation carries both together:
    /// `TransitionSetting::WipeShape { kind, angle }` is one setting, and a shape
    /// chosen without an angle would be a surface straightening a diagonal front
    /// nobody touched — [`Mixer::mask`]'s argument (ADR-0203) one row down. Here
    /// the two move together because the pill's cycle names both, which is what
    /// [`WIPE_SHAPES`] is.
    pub angle: f32,
    /// The grid the next scheduled move starts on, in beats: 4 for the next bar, 1
    /// for the next beat, 0 for now.
    pub quantum: f64,
    /// How long the next scheduled move lasts, in beats. Zero is a cut.
    pub length: f64,
}

/// The shapes the row's first pill offers, in cycle order, with the angle each
/// runs at and the word the pill reads.
///
/// # The vocabulary has no list, so a surface curates one
///
/// `karakuri_operation::TransitionSetting` has no `ALL` and no `name`, and
/// `WipeKind` has an `ALL` for nobody: a wipe shape is a kind and an angle, so
/// the thing a control cycles is a set of *pairs* and no enumeration of a kind
/// can be it. `WipeKind`'s own documentation says so — *"an arbitrary angle is
/// a dial, and a dial with nowhere to show its value is a control an operator
/// cannot read … the curation is a keyboard's compromise, not the operation"* —
/// and names `karakuri-cli`'s `MASK_SHAPES` as the surface that curates one.
/// This is the second, and the six pairs are that one's exactly, so two
/// surfaces stepping this setting arrive at the same six places.
///
/// The words are not that one's, and they are the only half that differs:
/// `MASK_SHAPES` writes a sentence into a status line (*"linear, left to
/// right"*) where this writes into a capsule the width of its own word.
/// `docs/manual/console.html` draws the row's shape pill reading `iris`, so the
/// register is the mock's.
///
/// `WipeKind::None` reads `no shape` and never `off`, which is not a
/// preference: `off` is a *residency* word on this console — the one a deck
/// preview's caption said until ADR-0240 — and `tests/preview_caption.rs`
/// asserts it is painted nowhere on the panel, because it is a state the
/// program cannot be in. `no shape` is `MASK_SHAPES`' own sentence for the same
/// entry (*"`c` needs a shape"*) in this console's register, beside `no slot`.
///
/// `None` first, which is `MASK_SHAPES`' own reason and
/// `karakuri_engine::deck::MaskKind::ALL`'s before it: *"a deck nobody has
/// touched wipes with nothing and says so rather than doing something."* It is
/// also [`TransitionSettings::START`], so a console nobody has pressed anything
/// on and a program nobody has pressed anything on begin in the same place.
///
/// A table rather than a `match`, where [`after`], [`next`] and [`next_shape`]
/// are all matches. Those cycle a *vocabulary* enumeration, and a match is what
/// stops a fourth variant compiling until somebody says what follows it. This
/// cycles a curation — six of an unbounded set of pairs — and there is no
/// enumeration for the compiler to hold it against, so a table is the honest
/// shape: the list is the decision.
const WIPE_SHAPES: [(WipeKind, f32, &str); 6] = [
    (WipeKind::None, 0.0, "no shape"),
    (WipeKind::Linear, 0.0, "left"),
    (WipeKind::Linear, std::f32::consts::FRAC_PI_2, "up"),
    (WipeKind::Linear, std::f32::consts::FRAC_PI_4, "diagonal"),
    (
        WipeKind::Linear,
        -std::f32::consts::FRAC_PI_4,
        "back diagonal",
    ),
    (WipeKind::Radial, 0.0, "iris"),
];

/// The grids the row's second pill offers, in cycle order, with the word the
/// pill reads.
///
/// The three are `karakuri-cli`'s `QUANTA`, values and order, for
/// [`WIPE_SHAPES`]' reason: two surfaces stepping one setting arrive at the
/// same places. A bar is four beats here, which is that constant's own
/// assumption rather than a measurement — nothing in the signal bus knows a
/// time signature.
///
/// The words are shortened to the mock's own, which draws this pill reading
/// `next bar`.
const QUANTA: [(f64, &str); 3] = [(4.0, "next bar"), (1.0, "next beat"), (0.0, "now")];

/// The lengths the row's third pill offers, in cycle order, with the word the
/// pill reads.
///
/// `karakuri-cli`'s `FADE_BEATS`, values and order: *"a bar, half a bar, two
/// bars, and a cut — the four an operator reaches for, in the order they are
/// reached for."* The mock draws this pill reading `8 beats`, which is the
/// third of them.
///
/// Zero reads `cut` rather than `0 beats`, because that is what a fade of no
/// length is and is the word `karakuri-cli` prints for it.
const FADE_BEATS: [(f64, &str); 4] = [
    (4.0, "4 beats"),
    (2.0, "2 beats"),
    (8.0, "8 beats"),
    (0.0, "cut"),
];

impl TransitionSettings {
    /// Where a run starts: no shape, the next bar, four beats — the first entry of
    /// each of the three cycles.
    ///
    /// It is `karakuri-cli`'s own opening state (`MASK_SHAPES[0]`, `QUANTA[0]` and
    /// `FADE_BEATS[0]` at `Live::new`), so two surfaces that step the same three
    /// settings also begin at the same three values. A console that opened on the
    /// mock's `iris · next bar · 8 beats` would be starting a run somewhere a hand
    /// had to have put it.
    pub const START: TransitionSettings = TransitionSettings {
        kind: WIPE_SHAPES[0].0,
        angle: WIPE_SHAPES[0].1,
        quantum: QUANTA[0].0,
        length: FADE_BEATS[0].0,
    };

    /// Where this shape sits in [`WIPE_SHAPES`].
    ///
    /// The fallback is unreachable while [`View::set_transition`] is the only way
    /// in, because that setter refuses a value no entry of the table names — see it
    /// for why. It falls back rather than panicking for the reason nothing on the
    /// frame path panics: the cost of being wrong here is one pill reading the
    /// wrong word, and the cost of being right about it is a console that stops
    /// drawing.
    fn shape_at(&self) -> usize {
        WIPE_SHAPES
            .iter()
            .position(|(kind, angle, _)| *kind == self.kind && *angle == self.angle)
            .unwrap_or(0)
    }

    fn quantum_at(&self) -> usize {
        QUANTA
            .iter()
            .position(|(beats, _)| *beats == self.quantum)
            .unwrap_or(0)
    }

    fn length_at(&self) -> usize {
        FADE_BEATS
            .iter()
            .position(|(beats, _)| *beats == self.length)
            .unwrap_or(0)
    }

    /// What the shape pill reads.
    pub fn shape_word(&self) -> &'static str {
        WIPE_SHAPES[self.shape_at()].2
    }

    /// What the quantum pill reads.
    pub fn quantum_word(&self) -> &'static str {
        QUANTA[self.quantum_at()].1
    }

    /// What the length pill reads.
    pub fn length_word(&self) -> &'static str {
        FADE_BEATS[self.length_at()].1
    }

    /// Whether a shape is chosen at all, which is what draws the pill armed:
    /// `.pill.armed` is *armed, bound, live in the good sense*, and a row whose
    /// shape reads `no shape` has nothing armed to say.
    ///
    /// It is not a rule about what a wipe may do. `WipeKind::None` refuses a wipe
    /// where the record is applied and not here — every way in meets the same wall
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    pub fn armed(&self) -> bool {
        self.kind != WipeKind::None
    }

    /// The next shape round the cycle, as the setting an operation carries.
    pub(crate) fn next_wipe_shape(&self) -> TransitionSetting {
        let (kind, angle, _) = WIPE_SHAPES[(self.shape_at() + 1) % WIPE_SHAPES.len()];
        TransitionSetting::WipeShape { kind, angle }
    }

    /// The next quantum round the cycle, as the setting an operation carries.
    pub(crate) fn next_quantum(&self) -> TransitionSetting {
        TransitionSetting::Quantum {
            beats: QUANTA[(self.quantum_at() + 1) % QUANTA.len()].0,
        }
    }

    /// The next length round the cycle, as the setting an operation carries.
    pub(crate) fn next_length(&self) -> TransitionSetting {
        TransitionSetting::Length {
            beats: FADE_BEATS[(self.length_at() + 1) % FADE_BEATS.len()].0,
        }
    }

    /// Take a setting, and answer whether anything moved — the whole of what
    /// [`View::set_transition`] does, kept beside the three tables that decide it.
    ///
    /// `false` for a setting no entry of the cycles names, and for one that names
    /// where the row already is.
    pub(crate) fn take(&mut self, setting: TransitionSetting) -> bool {
        let was = *self;
        match setting {
            TransitionSetting::WipeShape { kind, angle } => {
                if !WIPE_SHAPES
                    .iter()
                    .any(|(k, a, _)| *k == kind && *a == angle)
                {
                    return false;
                }
                self.kind = kind;
                self.angle = angle;
            }
            TransitionSetting::Quantum { beats } => {
                if !QUANTA.iter().any(|(b, _)| *b == beats) {
                    return false;
                }
                self.quantum = beats;
            }
            TransitionSetting::Length { beats } => {
                if !FADE_BEATS.iter().any(|(b, _)| *b == beats) {
                    return false;
                }
                self.length = beats;
            }
        }
        *self != was
    }
}

/// The Mixer bay's transition row, laid out: `.xfade` under the strips,
/// the three setting pills in it, and the `go` capsule at the right end.
///
/// # It carries the settings it was measured from
///
/// [`Mixer`] borrows the strips for [`Picture`]'s reason and this carries a
/// copy for the same one: a pill is as wide as the word in it and the word is
/// the setting's, so whoever measured the row and whoever paints it are one
/// statement. [`TransitionSettings`] is four scalars and `Copy`, so there is
/// no borrow to take.
///
/// # The `go` pill is drawn now, and what changed is not this crate
///
/// It was left out while [`Operation::Wipe`] could not be converted anywhere:
/// a wipe is written against `karakuri_operation_record::Current::transition`
/// and `Current::mix`, `crates/karakuri/src/main.rs` answered `None` for
/// both, and a capsule the mock lights `on` that a press does nothing with is
/// the scaffolding this module refuses. That window supplies both readings
/// now, so the capsule is a control and is drawn.
///
/// `.sep` is honoured rather than drawn. It is `flex: 1` and paints
/// nothing at all; what it does is push the `go` capsule to the right end of
/// the row, which is where [`transition`] puts it — measured in from
/// `.xfade`'s own padding, the way the shape pill is measured in from the
/// other side.
///
/// # What the mock draws here and this does not
///
/// - The `wipe` pill beside the shape. The mock draws the shape as two
///   spans — an armed `wipe` and the shape's own word — and that is one
///   statement about one setting: *a wipe shape is armed, and it is an iris*.
///   There is one setting in `TransitionSetting` for it, so there is one
///   control here, and the armed treatment the mock puts on the first span is
///   carried by the pill that names the shape
///   ([`TransitionSettings::armed`]).
/// - The row's tooltip. Every control on this console is a painted shape
///   and a tooltip needs `egui` to own a widget — the sentence [`outputs`]
///   writes about a control, three bays along.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionRow {
    /// `.xfade` itself: the block under the strips, the full width of the bay, with
    /// its rule along the top edge.
    pub rect: Rect,
    /// The shape pill, which is the capsule a press acts on and not only the box a
    /// word is painted into — [`TransitionRow::shape`] hit-tests exactly this
    /// rectangle, the way [`StripBox::blend`] is. As wide as the word in it, inside
    /// `.pill`'s padding and border.
    pub shape: Rect,
    /// The quantum pill, on the same terms.
    pub quantum: Rect,
    /// The length pill, on the same terms.
    pub length: Rect,
    /// The `go` capsule, at the right end of the row with `.sep`'s `flex: 1`
    /// between it and the length pill. On the same terms as the three: it is the
    /// rectangle a press acts on, and [`TransitionRow::go`] hit-tests exactly it.
    pub go: Rect,
    /// The settings these rectangles were measured from.
    pub settings: TransitionSettings,
}

/// What a press on the `go` capsule comes to: a wipe, or the reason there is
/// not one.
///
/// # A refusal is an answer this control has and the record layer does not
///
/// [`Operation::Wipe`] says *"Refused with no shape chosen"* at its own
/// definition and `karakuri-operation-record`'s arm says the same from the
/// other side: `Written` has three answers and none of them is a refusal,
/// because the shape is a surface's own setting and a surface is the only thing
/// that can see it is unset. `karakuri-cli`'s `c` is the precedent for both of
/// these arms — it turns a one-slot deck and a chosen-nothing shape away before
/// it asks — and this is that key's two refusals as a value, because this crate
/// has nowhere to print.
///
/// It says which refusal and not what to do about it, which is where the seam
/// between this crate and the window that runs it falls: the sentence an
/// operator reads is `crates/karakuri/src/main.rs`'s, and
/// [P-0083](../../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)
/// is what that sentence owes — the shape pill is two capsules to the left, and
/// a second deck is a `--set` away.
///
/// Not `Option<Operation>`, which would make a press on a row with no shape
/// chosen indistinguishable from a press on the card beside it. A control that
/// claims a press has acted on it ([`crate::input::claim`]'s rule 4), and the
/// act here is the refusal.
#[derive(Debug, Clone, PartialEq)]
pub enum Go {
    /// Run it: [`Operation::Wipe`] naming the deck being covered and the deck
    /// arriving over it. See [`TransitionRow::go`] for which is which.
    Wipe(Operation),
    /// There is nowhere for the wipe to come from: the mixer draws fewer than two
    /// strips, so the deck the selection is on is the only deck there is.
    /// `karakuri-cli`'s *"a wipe needs somewhere to come from — this deck holds one
    /// slot"*.
    NoOtherDeck,
    /// No shape is chosen, so there is nothing for the front to be. The shape pill
    /// on this row is where one is picked, and [`TransitionSettings::armed`] is the
    /// same fact drawn.
    NoShape,
}

impl TransitionRow {
    /// What a press at `p` asks the wipe shape to become, or `None` where there is
    /// no shape pill under it.
    ///
    /// # The pill cycles, and the operation names where it arrived
    ///
    /// Click it and the shape moves to the next of [`WIPE_SHAPES`], wrapping from
    /// the last back to the first, and what comes out is
    /// [`Operation::SetTransition`] naming the destination — never a step, because
    /// there is no step in the vocabulary to name.
    ///
    /// That is the affordance
    /// [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// leaves to whoever draws the control, and [`Mixer::blend`] is the precedent
    /// (ADR-0187): a pill that cycles is one control emitting six, the operator
    /// sees a toggle and the vocabulary never does. What P-0090 forbids is an
    /// operation that says *step*, and this emits none.
    ///
    /// The cycle is curated here because the vocabulary has no list to cycle — see
    /// [`WIPE_SHAPES`], where that is the whole argument, and `karakuri-cli`'s
    /// `MASK_SHAPES`, which is the precedent for a surface curating one.
    ///
    /// # One derivation, asked twice, and the whole pill is the target
    ///
    /// [`crate::input::claim`]'s rule 4 asks this and so does the caller that acts
    /// on the press — the arrangement [`Mixer::blend`], [`Mixer::mask`] and
    /// [`Outputs::op`] are all in. [`TransitionRow::shape`] is the pill's own
    /// rectangle, the one the word is painted into, so a pill a hand sees and a
    /// pill it clicks are the same one, and the padding is what makes a word a hand
    /// can find ([`Outputs::sink`]'s rule).
    ///
    /// # It names no deck, and there is nothing missing
    ///
    /// The settings decide what the *next* move means wherever it lands, so
    /// [`Operation::SetTransition`] carries a setting and no slot — the one row of
    /// this bay that does, for [`Operation::SetMasterOut`]'s reason one bay down.
    pub fn shape(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.pressed(self.shape, p, self.settings.next_wipe_shape())
    }

    /// What a press at `p` asks the quantum to become, or `None` where there is no
    /// quantum pill under it. [`TransitionRow::shape`]'s affordance over
    /// [`QUANTA`].
    pub fn quantum(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.pressed(self.quantum, p, self.settings.next_quantum())
    }

    /// What a press at `p` asks the length to become, or `None` where there is no
    /// length pill under it. [`TransitionRow::shape`]'s affordance over
    /// [`FADE_BEATS`].
    pub fn length(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.pressed(self.length, p, self.settings.next_length())
    }

    /// What a press at `p` on the `go` capsule comes to, or `None` where there is
    /// no capsule under it.
    ///
    /// # Which two decks a wipe names
    ///
    /// [`Operation::Wipe`] carries *the deck being covered* and *the deck arriving
    /// over it*, and this row names them the way `karakuri-cli`'s `c` does: the
    /// deck the selection is on is covered, and the next one round arrives over it.
    /// *The next deck* is the surface's translation and never the operation — the
    /// vocabulary's own sentence at that variant — and here the addressed deck is
    /// [`View::selection`], which is the ring this bay draws round a strip and the
    /// letter the library's `load` pill reads. `decks` is how many strips the mixer
    /// has, which is [`View::mixer`]'s length and the same count [`View::select`]
    /// refuses a selection against, so the wrap cannot name a deck with no strip.
    ///
    /// Neither deck is decided here beyond that. What the wipe *does* to them — the
    /// mask at 0, the put-on-air, whether `over` is written at all — is the
    /// conversion's and the deck's
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// # The two refusals are this control's, and they are its own
    ///
    /// A one-deck mixer and a shape reading `no shape` are both turned away here,
    /// in `karakuri-cli`'s order, and see [`Go`] for why a refusal is a value
    /// rather than a `None`.
    pub fn go(&self, p: karakuri_layout::Point, selection: u8, decks: usize) -> Option<Go> {
        if !self.go.contains(Pos2::new(p.x, p.y)) {
            return None;
        }
        if decks < 2 {
            return Some(Go::NoOtherDeck);
        }
        if !self.settings.armed() {
            return Some(Go::NoShape);
        }
        let from = usize::from(selection).min(decks - 1);
        Some(Go::Wipe(Operation::Wipe {
            from: from as u8,
            to: ((from + 1) % decks) as u8,
        }))
    }

    /// Whether a press on the `go` capsule would run a wipe, which is what draws it
    /// lit: `.pill.on` is the mock's *this is the press that does the thing*, and a
    /// capsule lit over a refusal would be the row saying it can do something it
    /// cannot.
    ///
    /// It is [`TransitionSettings::armed`] with the deck count beside it — exactly
    /// the two conditions [`TransitionRow::go`] refuses on, asked again rather than
    /// copied, so the pill a hand sees lit and the press that runs cannot come
    /// apart.
    pub fn runs(&self, decks: usize) -> bool {
        decks >= 2 && self.settings.armed()
    }

    /// One pill's hit test, written once because the three differ only in which
    /// rectangle and which cycle — the shape [`Mixer`]'s five share by being five
    /// questions about one laid-out strip.
    fn pressed(
        &self,
        pill: Rect,
        p: karakuri_layout::Point,
        setting: TransitionSetting,
    ) -> Option<Operation> {
        pill.contains(Pos2::new(p.x, p.y))
            .then_some(Operation::SetTransition { setting })
    }

    /// Whether a press at `p` is on any of the row's four capsules, which is what
    /// [`crate::input::claim`] asks: the panel claims what it acts on, and the
    /// row's own ground between two pills is not something it acts on.
    ///
    /// [`MasterRow::owns`]'s shape one bay up, and asked of the four derivations
    /// rather than of [`TransitionRow::rect`] — a press on the card either side of
    /// the pills reaches nothing, so claiming it would be taking an event to throw
    /// away. `.sep` is ground and not a control, for exactly that reason: it is the
    /// gap the `go` capsule is pushed to the end by, and there is nothing there to
    /// press.
    ///
    /// The `go` capsule is claimed whether or not a wipe would run, which is the
    /// one place this differs from the three pills: a press that is refused *is*
    /// acted on — the window says why — so claiming it is the rule met rather than
    /// bent.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.shape(p).is_some()
            || self.quantum(p).is_some()
            || self.length(p).is_some()
            || self.go.contains(Pos2::new(p.x, p.y))
    }
}

/// Where the transition row goes: the `.xfade` block inside a mixer region.
///
/// Directly under `.mixer-strips`, which is [`strips_row`] plus the
/// [`size::STRIPS_PAD`] below it, and exactly [`size::XFADE_H`] tall rather
/// than whatever the bay has left — the bay is taller than its contents by
/// design, and what is under this block is the 23.5 the crossfader took with it
/// when the mixer was decided to have none. It is the full width of the bay,
/// because `.xfade` is a child of `.bay` and its own padding is
/// [`size::XFADE_PAD_X`]; the rule along its top spans the card the way a bay
/// head's does.
///
/// `None` where the region cannot hold it — folded away, soloed away, or a
/// window too small — which is [`strips_row`]'s rule stated on the row under
/// it.
fn xfade_row(region: Rect) -> Option<Rect> {
    let block = Rect::from_min_size(
        Pos2::new(
            region.min.x,
            region.min.y + size::HEAD_H + size::STRIPS_PAD * 2.0 + size::STRIP_H,
        ),
        egui::vec2(region.width(), size::XFADE_H),
    );
    match block.width() > 0.0 && region.contains_rect(block) {
        true => Some(block),
        false => None,
    }
}

/// The Mixer bay's transition row, derived: the block under the strips, the
/// three setting pills laid end to end from its left padding, and the `go`
/// capsule against its right one.
///
/// # It is drawn with or without a deck, and that is not the strips' rule
/// broken
///
/// [`mixer`] answers `None` for a console with no deck behind it, because six
/// readings a slot with no slot to read is ADR-0177's row of zeroes. These
/// three are not readings. They are the console's own pointer
/// ([`TransitionSettings`]) and it always has a value — the same thing that is
/// true of [`View::selection`] and of the arrangement pill, which is drawn with
/// no store behind it. A row blanked for want of a deck would be a setting an
/// operator cannot make until something else has happened.
///
/// The `go` capsule is drawn there too, and what it says about a deckless
/// console is said in the paint and in the answer rather than by leaving it
/// out: it is not lit ([`TransitionRow::runs`]) and a press on it is
/// [`Go::NoOtherDeck`]. A capsule that vanished with the strips would be the
/// one control on this row an operator has to discover.
///
/// # All four capsules or none
///
/// The three settings are laid from the left of the block's padding, one
/// [`size::XROW_GAP`] apart; `go` is measured back from the padding on the
/// other side, which is `.sep`'s `flex: 1` — the separator takes whatever is
/// between them and paints nothing. If the three would reach it the whole row
/// answers `None`. Not the chips-that-fit rule `.scopes` and `.rend-row` are
/// drawn under: those are a *list* whose length is a value, and what is dropped
/// off the end is one more of the same question. This is three different
/// settings and the press that runs them, and one of them silently missing is a
/// control with nothing on screen saying where it went. The bay's track is a
/// fixed 400 wide (`lib.rs`), so the case is a window below the arrangement's
/// own minimum rather than an ordinary narrow console.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. `ctx` is asked for the type, because a pill is as wide as the word in
/// it.
pub fn transition(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    settings: TransitionSettings,
) -> Option<TransitionRow> {
    // Fonts are not valid until `egui` has run a pass, exactly as in `mixer`,
    // `outputs` and `transport`.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let region = to_egui(layout.rect(layout.find("mixer")?));
    let block = xfade_row(region)?;
    let mut x = block.min.x + size::XFADE_PAD_X;
    let top = block.min.y + size::HAIRLINE + size::XFADE_PAD_TOP;
    let right = block.max.x - size::XFADE_PAD_X;
    let mut pill = |text: &str| {
        // `.pill`'s padding either side of the word, and its own border,
        // which `pill_width` does not count — see `size::XPILL_H`, where the
        // two pixels are argued.
        let w = pill_width(ctx, text) + size::HAIRLINE * 2.0;
        let at = Rect::from_min_size(Pos2::new(x, top), egui::vec2(w, size::XPILL_H));
        x = at.max.x + size::XROW_GAP;
        at
    };
    let shape = pill(settings.shape_word());
    let quantum = pill(settings.quantum_word());
    let length = pill(settings.length_word());
    // **`go` from the other end**, which is what `.sep`'s `flex: 1` puts it:
    // the separator absorbs whatever is left between the length pill and this
    // one, so the capsule's place is measured off the block's right padding
    // and never off the words to its left.
    let go_w = pill_width(ctx, GO) + size::HAIRLINE * 2.0;
    let go = Rect::from_min_size(
        Pos2::new(right - go_w, top),
        egui::vec2(go_w, size::XPILL_H),
    );
    // The separator is `flex: 1` and so is never negative: where the three
    // settings would reach the capsule there is no row, for the reason the
    // header gives. One `.xrow` gap is the least `.sep` can be and still be a
    // gap between two pills rather than two capsules touching.
    if length.max.x + size::XROW_GAP > go.min.x {
        return None;
    }
    Some(TransitionRow {
        rect: block,
        shape,
        quantum,
        length,
        go,
        settings,
    })
}

/// What the `go` capsule reads, which is the mock's own word and the only one
/// on this row that is not a setting's.
///
/// It is a *verb* where the three beside it are values —
/// `docs/manual/console.html` draws `iris · next bar · 8 beats · go` — so it
/// has no cycle behind it and no table to come out of. Written here rather than
/// inline because [`transition`] measures the capsule from it and
/// [`transition_into`] paints it from it, which is this module's rule about
/// every word it draws.
const GO: &str = "go";

/// Paints the mixer transition row controls and status indicators.
pub fn transition_into(ui: &Ui, pal: &Palette, row: &TransitionRow, decks: usize) {
    // The rule is inside the block rather than above it, which is what keeps
    // the pills where `transition` put them: `size::XFADE_H` counts the
    // hairline as the first pixel of the block.
    let rule = row.rect.min.y + size::HAIRLINE * 0.5;
    ui.painter().line_segment(
        [
            Pos2::new(row.rect.min.x, rule),
            Pos2::new(row.rect.max.x, rule),
        ],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
    pill_into(
        ui,
        pal,
        row.shape,
        row.settings.shape_word(),
        row.settings.armed(),
    );
    pill_into(ui, pal, row.quantum, row.settings.quantum_word(), false);
    pill_into(ui, pal, row.length, row.settings.length_word(), false);
    match row.runs(decks) {
        true => on_pill_at(ui, pal, row.go, GO),
        false => pill_at(ui, pal, row.go, GO),
    }
}
