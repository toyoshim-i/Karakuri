use super::super::*;
use super::*;

// ---------------------------------------------------------------------------
// The Inspector's deck head, name and targets
// ---------------------------------------------------------------------------

/// The word on the sync chip, which is `karakuri_operation::Sync::name` and not
/// a second spelling: *free*, *tempo*, *beat* are the manual's three and the
/// mock's three.
fn sync_word(sync: Sync) -> &'static str {
    sync.name()
}

/// Every sync mode, in the order the deck head's chip walks them — the
/// vocabulary's own declaration order, which is `Sync::ALL`'s in
/// `karakuri_engine::transport` and the mock's *free, tempo, beat*.
///
/// [`AUTHORITIES`]' array one head along, and it exists for the same two
/// reasons: [`DeckHead::mode`] steps *positions* in this list, which a `match`
/// cannot express once the step may be refused, and `tests/deck_head.rs` walks
/// it to assert the cycle reaches every mode. A fourth mode is still caught at
/// build time, by [`sync_word`]'s `match` on the way to a word and by
/// [`anchor_letter`]'s on the way to a letter.
///
/// Public because [`Pane::allows`] is stated in this order, and whoever fills
/// that field is in another crate: an order a caller has to match and cannot
/// name is an order two crates would each write down.
pub const SYNCS: [Sync; 3] = [Sync::Free, Sync::Tempo, Sync::Beat];

/// The next mode the chip can actually get to, walking [`SYNCS`] from the one
/// after `at` and taking the first this deck's material can honour.
///
/// # It skips rather than offering, and that is the mock's own sentence
///
/// *"Click to cycle, and the cycle skips a mode this material cannot honour
/// instead of offering it"*. Whether a mode is honourable is
/// `karakuri_engine::deck::Deck::sync_allowed`'s and arrives here as
/// [`Pane::allows`] — the surface owns the affordance and never the authority
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
/// so this chooses which destination to name and refuses nothing.
///
/// # Landing back on `at` is a state and not a failure
///
/// The walk starts at `at + 1`, so it comes back to `at` only when every other
/// mode is refused — and the operation that comes out then names the mode the
/// deck is in, which re-anchors. That is the one re-anchor a cycle can reach
/// and it is the one where re-anchoring does nothing: the only material that
/// refuses two modes is material that accumulates *and* reads the beat, where
/// the mode left is `Free` and `Free` reads no anchor. The control that can ask
/// for it deliberately is [`DeckHead::anchor`]
/// ([ADR-0218](../../../../docs/adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)).
///
/// `Free` is refused by nothing, so the loop always finds something and there
/// is no `None` to answer.
pub(crate) fn next_sync(at: Sync, allows: [bool; SYNCS.len()]) -> Sync {
    let from = SYNCS.iter().position(|s| *s == at).unwrap_or(0);
    (1..=SYNCS.len())
        .map(|step| (from + step) % SYNCS.len())
        .find(|index| allows[*index])
        .map(|index| SYNCS[index])
        .unwrap_or(at)
}

/// The letter the anchor readout leads with: the mock's `T128` under tempo sync
/// and `B128 +0.25` under beat, and nothing at all under free — *"a free deck
/// shows neither, because free is the absence of a transport rather than a
/// setting, and a column reading free on every deck would be four words of
/// nothing."*
fn anchor_letter(sync: Sync) -> Option<&'static str> {
    match sync {
        Sync::Free => None,
        Sync::Tempo => Some("T"),
        Sync::Beat => Some("B"),
    }
}

/// What the anchor reads, and `None` under free sync.
///
/// The mock's own two spellings: `T128` where a deck is tempo-synced, and `B128
/// +0.25` where it is beat-synced and sitting a quarter beat ahead of the room.
/// The tempo is whole because the mock writes it whole — `T<b>128</b>` — and
/// the offset carries two places and a sign because the mock's `<em>+0.25</em>`
/// does. The offset is in beats and the transport row's is in milliseconds, so
/// neither is ever drawn without knowing which it is; here that is the `B` in
/// front of it.
pub(crate) fn anchor_text(pane: &Pane) -> Option<String> {
    let letter = anchor_letter(pane.sync)?;
    Some(match pane.sync {
        Sync::Beat => format!("{letter}{:.0} {:+.2}", pane.anchor_bpm, pane.scrub_beats),
        _ => format!("{letter}{:.0}", pane.anchor_bpm),
    })
}

/// How far one press of the scrub moves a deck, in beats.
///
/// A quarter beat, which is the vocabulary's own figure —
/// [`Operation::ScrubDeck`] writes it at the field (*"How far, in beats. A
/// quarter beat is what a key press asks for"*) and the row it fills is titled
/// *Scrub a deck a quarter beat*. The console page says the same of this
/// control: *"a quarter beat a press, into that same offset"*.
///
/// Signed at the call site and not here. The left arrow asks for minus this and
/// the right for plus it, so the amount is one number and the direction is
/// which chip was pressed — see [`DeckHead::scrub`], and the manual's *"the one
/// control here meant to go backwards"*.
pub const SCRUB_BEATS: f64 = 0.25;

/// What the `re-salt` capsule reads.
pub const RE_SALT_LABEL: &str = "re-salt";

/// Where a press on the capacity chip arrives: the next number up the ladder,
/// and off the top back to the bottom — or `None` where the ladder is empty and
/// there is nothing to ask for.
///
/// # The next one *above* what is running, rather than the next one along
///
/// `candidates` is [`Aimed::capacities`], ascending, and the running value is
/// not necessarily one of them: a procedure may declare `capacity [4096,
/// 1048576] = 81920`, and a Set loaded from a file may be running at whatever
/// that file recorded. Asking for the first candidate *greater than* where the
/// slot is answers both cases in one line — the next power of two from a value
/// that is on the ladder, and the next power of two up from one that is not —
/// where a `position` lookup would have fallen back to the bottom of the range
/// and taken a slot from 81920 to 4096 on a press that reads as *one step*.
///
/// The wrap goes through the bottom and not through unset, which is where this
/// differs from the Library's two filter fields (ADR-0262): a filter has a
/// state that is *not narrowed* and a capacity has no such state — every
/// geometry is running at some number — so the end of the ladder is the
/// beginning of it.
fn stepped_capacity(candidates: &[u32], at: u32) -> Option<u32> {
    candidates
        .iter()
        .find(|candidate| **candidate > at)
        .or_else(|| candidates.first())
        .copied()
}

/// The deck head's controls, laid out: the chip that names the mode, the anchor
/// that re-asks for it, the two arrows that scrub, the two chips that ask for a
/// different build, and the fold at the right.
///
/// # One derivation, for [`Outputs`]' reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. Two copies of a flex row's running sum
/// is an arrow that lights under a pointer that cannot move the deck.
///
/// # Six rectangles and one type, because they are one row
///
/// [`LookRow`]'s reason one bay over: each one's place is measured from the
/// last, which is what a flex row is, and splitting them into six functions
/// would mean measuring the chip before each of them again to find out where it
/// starts. The fold is in here for the same reason, and it is a control too —
/// see [`DeckHead::composite`], which was the one rectangle here that claimed
/// nothing until 2026-09-09.
///
/// The two before the fold are measured from the right, because that is what
/// `.sep`'s `flex: 1` does to everything after it: the fold sits against the
/// row's right-hand padding, the `re-salt` capsule one gap before it and the
/// capacity chip one gap before that, and what says the row fits is that the
/// arrows end before the leftmost of the three begins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeckHead {
    /// The sync chip, which is what a press has to land in to move the mode on.
    /// `.mini`'s box round [`sync_word`], at the left of the row.
    pub mode: Rect,
    /// The anchor, or `None` under [`Sync::Free`], where there is nothing to read
    /// and nothing to re-anchor — see [`anchor_text`].
    ///
    /// The text run grown to a chip's height, which is [`LookRow::grip`]'s
    /// treatment of a 5px track and its argument: `.anchor` is a bare span at
    /// [`size::ANCHOR_SIZE`] with no padding of its own, and 13.5 pixels of type is
    /// not a target a hand finds. It is grown to [`size::MINI_H`], so it is the
    /// same 15.5 as the chips either side of it and sits in the same
    /// [`size::DECK_HEAD_PAD_Y`] the row gives them. No wider than the words,
    /// because the row is a flex row and a target that reached past its own text
    /// would take the arrows' places with it.
    pub anchor: Option<Rect>,
    /// A quarter beat back. One `.scrub i`.
    pub back: Rect,
    /// A quarter beat forward. The other.
    pub forward: Rect,
    /// The fold at the right of the row, and the fourth control on it —
    /// [`DeckHead::compositing`] is what a press on it asks for.
    ///
    /// It was not a control until 2026-09-09, on the argument [`Pane::composite`]
    /// carries and corrects: layering is a *build* decision, so a press is a
    /// rebuild rather than a write. That is true and is the reason this works
    /// rather than the reason it could not — a rebuild off the render thread,
    /// judged against the budget and rolled back on its own, is what this
    /// instrument does to change what a slot is running, and the layering is one
    /// field of the aim a watcher is pointed at (ADR-0314).
    ///
    /// It is here because the row is laid out to it: it is `.sep`'s `flex: 1`
    /// pushing it against the right-hand padding, and what says the controls on the
    /// left fit is that they end before it.
    pub composite: Rect,
    /// The capacity chip and the `re-salt` capsule, or `None` on a deck with
    /// nothing to size and nothing to seed — see [`AimChips`].
    ///
    /// One field for two chips because they are one reading: both are drawn exactly
    /// when [`Pane::aimed`] is, and a state where one of them was there and the
    /// other was not is not a state this row has.
    pub aim: Option<AimChips>,
    /// Which deck this head belongs to, as [`Operation::SetSync`],
    /// [`Operation::ScrubDeck`] and [`Operation::SetCompositing`] each name one —
    /// [`Pane::deck`], carried so that a press answers with the deck it was
    /// measured for.
    pub deck: usize,
    /// What the fold chip is showing, and what a press names the other of —
    /// [`Pane::composite`] as it was read, carried for [`DeckHead::locked`]'s
    /// reason one field down: whoever measured this row and whoever acts on a press
    /// in it are one statement, so a chip cannot name a destination computed from a
    /// state some later frame read.
    pub composited: bool,
    /// What the mode chip is showing, and what re-anchoring re-asks for. Carried
    /// for [`LookRow::values`]' reason: whoever measured this row and whoever acts
    /// on a press in it are one statement.
    pub locked: Sync,
    /// What this deck's material can honour, [`Pane::allows`] as it was read — the
    /// whole of what the cycle skips on.
    pub allows: [bool; SYNCS.len()],
}

/// The deck head's two build chips, laid out and with what a press on each one
/// asks for — the capacity the slot's geometries run at, and the salt its
/// randomness comes from.
///
/// # The destinations are carried, for [`DeckHead::composited`]'s reason
///
/// Whoever measured this row and whoever acts on a press in it are one
/// statement. The step is arithmetic over [`Aimed::capacities`] and the salt is
/// a number the host handed in, and both are worked out once, on the frame that
/// laid the chips out — so a chip a hand pressed and the operation that leaves
/// this crate cannot be about two different readings of the slot.
///
/// # Neither says *step* and neither says *again*
///
/// What crosses into the vocabulary is
/// [`Operation::SetProperty`](karakuri_operation::Operation::SetProperty)
/// naming a number: `Property::Capacity` carries the element count the step
/// arrived at and `Property::Seed` carries the salt. The affordance — *press it
/// and it moves on* — is the surface's, which is [`DeckHead::sync`]'s division
/// and [`Mixer::blend`]'s
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimChips {
    /// The capacity chip, reading [`Aimed::capacity`]. `.mini`'s box round the
    /// number, one gap left of [`AimChips::salt`].
    pub size: Rect,
    /// Where a press on [`AimChips::size`] arrives, or `None` where there is
    /// nothing to step to — see [`stepped_capacity`] and [`Aimed::capacities`].
    ///
    /// `None` is drawn and not claimed, which is `input`'s *a control claims what
    /// it acts on and no more* and the arrangement an inert scrub is already in:
    /// the number is still worth reading on a deck whose geometries share no range,
    /// and a press on it has nothing to ask for.
    pub resize: Option<u32>,
    /// The `re-salt` capsule, one gap left of [`DeckHead::composite`].
    pub salt: Rect,
    /// The salt a press on it asks for — [`Aimed::salt`], carried.
    ///
    /// There is no state in which this chip is drawn and inert: a slot with a
    /// geometry has randomness to re-seed, and a slot without one draws neither of
    /// these two.
    pub re_salt: u32,
}

impl DeckHead {
    /// Whether `p` is on the sync chip.
    pub fn hit_mode(&self, p: karakuri_layout::Point) -> bool {
        self.mode.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is on the anchor, which a free deck does not draw.
    pub fn hit_anchor(&self, p: karakuri_layout::Point) -> bool {
        self.anchor
            .is_some_and(|at| at.contains(Pos2::new(p.x, p.y)))
    }

    /// Whether `p` is on the fold at the right of the row.
    pub fn hit_composite(&self, p: karakuri_layout::Point) -> bool {
        self.composite.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is on the capacity chip *and* the chip has somewhere to step,
    /// which is [`DeckHead::arrow`]'s arrangement written for a chip: a deck whose
    /// geometries share no declared range draws the number and claims nothing.
    pub fn hit_size(&self, p: karakuri_layout::Point) -> bool {
        self.aim
            .is_some_and(|aim| aim.resize.is_some() && aim.size.contains(Pos2::new(p.x, p.y)))
    }

    /// Whether `p` is on the `re-salt` capsule, which is claimed wherever it is
    /// drawn.
    pub fn hit_salt(&self, p: karakuri_layout::Point) -> bool {
        self.aim
            .is_some_and(|aim| aim.salt.contains(Pos2::new(p.x, p.y)))
    }

    /// Which arrow `p` is on, as the amount it asks for — or `None` off both, and
    /// `None` on either while the scrub is inert.
    ///
    /// Inert is not claimed, which is [`crate::input`]'s *a control claims what it
    /// acts on and no more*, and it is why this answers for liveness as well as for
    /// position. The arrows are drawn on every deck, because a scrub is *"an offset
    /// added to the room's position under beat sync"* and a pair of chips that
    /// vanished on two modes out of three would move the rest of the row under the
    /// hand every time the chip beside them was pressed. Drawn and not claimed is
    /// the arrangement a fader's track is already in.
    fn arrow(&self, p: karakuri_layout::Point) -> Option<f64> {
        if self.locked != Sync::Beat {
            return None;
        }
        let p = Pos2::new(p.x, p.y);
        match (self.back.contains(p), self.forward.contains(p)) {
            (true, _) => Some(-SCRUB_BEATS),
            (_, true) => Some(SCRUB_BEATS),
            _ => None,
        }
    }

    /// Whether `p` is on any of the six, which is what [`crate::input::claim`]
    /// asks. The fold is one of them since 2026-09-09, and it is the only one of
    /// the six that is claimed on every deck: a sync chip is always live, an anchor
    /// is not drawn on a free deck, an arrow is not claimed off beat sync, and the
    /// two build chips are drawn only where the deck has a geometry — where a
    /// layering is a state every slot is in.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.hit_mode(p)
            || self.hit_anchor(p)
            || self.arrow(p).is_some()
            || self.hit_size(p)
            || self.hit_salt(p)
            || self.hit_composite(p)
    }

    /// What a press at `p` asks this deck's clock to become, or `None` where there
    /// is no sync chip under it.
    ///
    /// # The chip cycles, the operation names where it arrived, and the cycle skips
    ///
    /// Click it and the deck is asked for the next of [`SYNCS`] its material can
    /// honour — *free*, *tempo*, *beat*, wrapping — and what comes out is
    /// [`Operation::SetSync`] naming the destination, never a step, because there
    /// is no step in the vocabulary to name. The affordance is [`Mixer::blend`]'s
    /// and [`LookRow::tonemap`]'s exactly
    /// ([ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)),
    /// and so is the division it rests on: the cycle is [`next_sync`] here and
    /// nothing at all in `karakuri-operation`, which is P-0090's division: a toggle
    /// is an affordance, built over operations by whoever draws the control.
    ///
    /// The skip is the one thing this cycle has that the other two do not, and it
    /// is not a refusal: what may be asked for is the engine's
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// and this chip is choosing which of the destinations *it* offers to name, out
    /// of a reading somebody else took. A mode this material cannot honour is
    /// passed over rather than handed on to be refused, which is the mock's own
    /// *"skips a mode this material cannot honour instead of offering it"*.
    ///
    /// What a map is offered is the three modes, not the cycle — the sentence
    /// ADR-0187 wrote about three blend modes, and the reason the anchor beside
    /// this chip can be a fourth way to say one of them.
    pub fn sync(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_mode(p).then(|| Operation::SetSync {
            deck: self.deck as u8,
            sync: next_sync(self.locked, self.allows),
        })
    }

    /// What a press on the anchor asks for: [`Operation::SetSync`] naming the mode
    /// this deck is already in, which re-anchors it — or `None` off the anchor, and
    /// `None` on a free deck, which draws none.
    ///
    /// # It is one operation asked for from two ends, and that is the record
    ///
    /// `karakuri_engine::transport::Transport::engage`'s own documentation is where
    /// this comes from: *"Re-engaging the mode a slot is already in re-anchors it,
    /// which is how an operator says 'call this the reference tempo' without a
    /// second control."* `Transport::engaged` recomputes the anchor from the
    /// session tempo every time and clears the scrub with it, so naming the mode
    /// that is running is a real move rather than a press that does nothing.
    ///
    /// A cycle structurally cannot ask for it, which is why this is a second target
    /// on the row rather than a second press on the first: [`next_sync`] starts at
    /// the mode *after* the one the deck is in, so the one state it can never
    /// arrive at is the state it is in. That is a fault of the affordance and not a
    /// gap in the vocabulary, and it is why there is no `ReAnchor` variant here to
    /// name — an operation whose meaning is *again* is the shape P-0090 rules out
    /// ([ADR-0218](../../../../docs/adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)).
    ///
    /// A free deck has no anchor and loses nothing. `Free` is the absence of a
    /// transport rather than a setting and reads no anchor at all, so there is
    /// nothing for a press to re-ask for — [`anchor_text`] draws nothing there and
    /// this answers `None` off the same `Option`.
    pub fn reanchor(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_anchor(p).then_some(Operation::SetSync {
            deck: self.deck as u8,
            sync: self.locked,
        })
    }

    /// What a press on an arrow asks for: [`Operation::ScrubDeck`] moving this deck
    /// a [`SCRUB_BEATS`] back or forward — or `None` off both, and `None` wherever
    /// the scrub is inert.
    ///
    /// # The one control on this panel that moves by an amount
    ///
    /// Every other control here names a destination, which is P-0090's rule, and
    /// the vocabulary says at the variant why this one does not: *"it is relative
    /// because nothing in this instrument can set a position"*. Scrubbing moves
    /// closed-form material by an amount; accumulating material cannot be moved to
    /// a position at all, so an absolute `at_beat` would be an operation that does
    /// not exist for two thirds of the material. So this is not the exception to
    /// P-0090 it looks like — there is no destination in the language for it to
    /// name.
    ///
    /// Signed and unbounded, which is the deck head's own spelling: the offset this
    /// writes is drawn in the anchor beside it, in beats and for one deck, where
    /// the transport row's offset is in milliseconds and is the whole instrument's.
    /// Nothing clamps it here and there is nothing to clamp it to.
    ///
    /// Inert under anything but beat sync, because that is the only mode that reads
    /// the offset. The arrows keep their shape and the console page says why rather
    /// than greying them out: *"Nothing here re-runs a deck's history to place
    /// it."*
    ///
    /// # One press is one operation
    ///
    ///
    /// [ADR-0207](../../../../docs/adr/0207-a-continuous-control-says-one-thing-per-frame.md)
    /// coalesces a swept MIDI fader to one operation a frame; neither half of it
    /// applies to a press, and a second press in the same frame is a second quarter
    /// beat an operator asked for. That is the whole reason the amount is a
    /// constant rather than a distance along anything.
    pub fn scrub(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.arrow(p).map(|beats| Operation::ScrubDeck {
            deck: self.deck as u8,
            beats,
        })
    }

    /// What a press on the fold asks for: [`Operation::SetCompositing`] naming the
    /// layering this deck is not in — or `None` off the chip.
    ///
    /// # A destination and not a flip, on a chip that reads as a toggle
    ///
    /// [`Mixer::blend`]'s division and [`DeckHead::sync`]'s: the affordance is
    /// *press it and it changes*, and what leaves this crate is the state being
    /// asked for. Nothing in `karakuri-operation` says *toggle*, because two
    /// surfaces stepping one control disagree about where they are
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// and the destination is computed from [`DeckHead::composited`] — the state
    /// the frame that laid this row out drew — so the chip a hand pressed and the
    /// operation that leaves are one statement.
    ///
    /// # The press is a rebuild, and that is the mechanism rather than a cost
    ///
    /// `Set::merge` is written at `Set::build` and nothing moves it afterwards,
    /// which read for a year as *the engine has no setter for this, so the control
    /// is blocked*. What it actually means is that the control is not a write at
    /// all: the layering is one field of the description a slot's watcher is
    /// pointed at, so the window that acts on this operation restates the rest of
    /// that description with this field changed and sends it, and the worker
    /// rebuilds the slot off the render thread. That build lands at a frame
    /// boundary and is judged there and then, on what one frame of that Set was
    /// measured to cost, exactly as an edited file and a library load are, and
    /// rolls itself back if it cannot hold the frame — which is the point rather
    /// than the price, because compositing costs a frame-sized target per renderer
    /// ([P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md),
    /// [P-0085](../../../../docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md),
    /// [ADR-0314](../../../../docs/adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)).
    ///
    /// Nothing here knows any of that, and this crate could not: it names a
    /// destination and a deck, and where the rebuild happens is the window's
    /// (ADR-0156). What it does owe is that the chip goes on reading what *landed*
    /// rather than what was asked for, which is [`Pane::composite`]'s own note.
    pub fn compositing(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_composite(p).then_some(Operation::SetCompositing {
            deck: self.deck as u8,
            compositing: !self.composited,
        })
    }

    /// What a press on the capacity chip asks for: [`Operation::SetProperty`]
    /// naming the element count the step arrived at — or `None` off the chip, and
    /// `None` on a chip with nowhere to step.
    ///
    /// # A number a hand should not drag, so the control steps
    ///
    /// A capacity is a number in a declared range, which everywhere else on this
    /// console is [`Param`]'s track — and the mock's own reading of a Set says so,
    /// *"capacity included, because a procedure declares one the same way it
    /// declares a knob"*. It is refused here by what a drag is: [`ParamGrip`] turns
    /// a pointer into a value every frame it moves, which for a parameter is a
    /// uniform write and for a capacity is a full rebuild of the slot with every
    /// element buffer in it reallocated — dozens of them across one gesture, which
    /// is
    /// [P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)
    /// at its widest. So the affordance is the Library filters' one bay over
    /// ([ADR-0262](../../../../docs/adr/0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md)):
    /// the field steps a closed list and the operation names where it arrived.
    ///
    /// What it steps is not this console's list. The powers of two inside a
    /// declared range belong to the material in the slot, so they arrive as
    /// [`Aimed::capacities`] the way `holds` arrives as [`View::holds`], and what
    /// is offered is a reading somebody else took (P-0090).
    ///
    /// # The press is a rebuild, and it is the fold's mechanism exactly
    ///
    /// The capacity is one field of the description this slot's watcher is pointed
    /// at, so the window restates the rest and sends it and the worker recompiles
    /// the slot off the render thread, judged at a frame boundary against what one
    /// frame of that Set was measured to cost — see [`DeckHead::compositing`],
    /// where the argument is written out, and `docs/adr/0328-…`. Nothing here knows
    /// any of that and this crate could not: it names a number and a deck.
    pub fn resized(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let aim = self.aim?;
        let elements = aim.resize?;
        aim.size
            .contains(Pos2::new(p.x, p.y))
            .then_some(Operation::SetProperty {
                deck: self.deck as u8,
                property: karakuri_operation::Property::Capacity { elements },
            })
    }

    /// What a press on the `re-salt` capsule asks for: [`Operation::SetProperty`]
    /// naming the salt this slot's randomness is to come from — or `None` off the
    /// capsule.
    ///
    /// # The number is handed in, and that is the whole of the decision
    ///
    /// A salt is the one payload on this row that could plausibly be *made up*, and
    /// a console that made one up would be a surface producing a picture no later
    /// run could produce again
    /// ([P-0092](../../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
    /// [`Aimed::salt`] is the next value of the slot's own deterministic sequence,
    /// derived by whoever read the Set from the salt the slot is actually running —
    /// so this names a destination like every other control here, the same press
    /// twice from the same place lands on the same two pictures, and a Set kept
    /// afterwards records the salts it was running at.
    ///
    /// Nothing is refused here. Asking for the salt a slot is already on is not a
    /// state this capsule can produce — the sequence goes forward — and a re-seed
    /// changes the picture, so unlike the fold beside it there is no press that
    /// buys a recompile and moves nothing.
    pub fn re_salted(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let aim = self.aim?;
        aim.salt
            .contains(Pos2::new(p.x, p.y))
            .then_some(Operation::SetProperty {
                deck: self.deck as u8,
                property: karakuri_operation::Property::Seed { salt: aim.re_salt },
            })
    }
}

/// The deck head's controls, derived — the chips in
/// [`InspectorPane::deck_head`], laid along it the way `.deck-head`'s flex row
/// lays them.
///
/// # Measured off a laid-out pane rather than off the layout
///
/// [`inspector`] answers where the pane is and this answers where the chips in
/// its second row are: two questions about one laid-out pane, which is exactly
/// what the mixer bay's four controls are about one laid-out strip. A second
/// derivation from the layout would be a second answer that could disagree with
/// the one the frame drew.
///
/// # What it costs to ask
///
/// Five galley lookups per pane: the mode's word, the anchor's two numbers, the
/// capacity's digits, the `re-salt` capsule's word and the fold's. The two
/// arrows cost none — they are marks rather than words, which is
/// [`Mixer::mask`]'s own saving one bay over — and nothing here asks after the
/// node groups below.
///
/// It is asked twice on a frame, once here and once for the galleys
/// [`deck_head_into`] paints, which is [`look`]'s honest cost written down one
/// bay along: the derivation that draws a control is the one that hit-tests it,
/// so a control cannot be painted anywhere a press cannot reach. Two panes at
/// three lookups is on the order of ten allocations a frame against the panel
/// pass's measured median of 1518 (`crates/karakuri`'s `WRITTEN_ALLOCS`, taken
/// 2026-08-31), which is inside the factor of two that file quotes a figure
/// across.
///
/// # `None` is a row that cannot hold its own controls
///
/// [`look`]'s rule and [`arrangement`]'s: *a control that does not fit in the
/// row it is drawn in is no control at all, rather than half of one*. The fold
/// is pushed against the right-hand padding and the three controls run from the
/// left, so what says the row fits is that the arrows end before the fold
/// begins. A pane narrow enough to fail that draws its two heads' words and no
/// chips at all, where it used to draw chips cut in half by
/// [`inspector_into`]'s clip rectangle — which is a picture of a control that
/// cannot be pressed
/// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
pub fn deck_head(ctx: &egui::Context, at: &InspectorPane, pane: &Pane) -> Option<DeckHead> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`transport`], [`outputs`] and [`mixer`] — and on the frame before the
    // first one there is nothing drawn here to press.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let row = at.deck_head;
    let mid = row.center().y;
    let width = |text: &str, size: f32| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    // `.mini`'s box, which is `mini_word`'s arithmetic: the padding either
    // side of the word, with no border counted, because that is what this row
    // has always been drawn with and this is the same chip.
    let mini = |text: &str, x: f32| {
        Rect::from_min_size(
            Pos2::new(x, mid - size::MINI_H * 0.5),
            egui::vec2(
                width(text, size::MINI_SIZE) + size::MINI_PAD_X * 2.0,
                size::MINI_H,
            ),
        )
    };

    let mode = mini(sync_word(pane.sync), row.min.x + size::DECK_HEAD_PAD_X);
    let anchor = anchor_text(pane).map(|text| {
        Rect::from_min_size(
            Pos2::new(mode.max.x + size::DECK_HEAD_GAP, mid - size::MINI_H * 0.5),
            egui::vec2(width(&text, size::ANCHOR_SIZE), size::MINI_H),
        )
    });
    // One `.deck-head` gap after whichever of the two came last — a free deck
    // draws no anchor, and a flex row closes up rather than leaving a hole
    // where one would have been.
    let arrows = anchor.map_or(mode.max.x, |at| at.max.x) + size::DECK_HEAD_GAP;
    // `.scrub i`'s box: the mark is as wide as the glyph it stands in for,
    // which is [`Mixer::mask`]'s rule, inside its own padding and its border.
    let arrow_w = size::SCRUB_SIZE + size::SCRUB_PAD_X * 2.0 + size::HAIRLINE * 2.0;
    let arrow = |x: f32| {
        Rect::from_min_size(
            Pos2::new(x, mid - size::SCRUB_H * 0.5),
            egui::vec2(arrow_w, size::SCRUB_H),
        )
    };
    let back = arrow(arrows);
    let forward = arrow(back.max.x + size::SCRUB_GAP);

    // `.sep`'s `flex: 1` puts the fold hard against the right of the row, and
    // the two build chips are measured leftwards from it — a flex row's running
    // sum taken from the other end, which is what everything after the `.sep`
    // is.
    let fold = mini(COMPOSITE_LABEL, row.min.x);
    let composite = mini(
        COMPOSITE_LABEL,
        row.max.x - size::DECK_HEAD_PAD_X - fold.width(),
    );
    // **Both or neither**, which is [`DeckHead::aim`]'s own sentence: they come
    // from one reading, so a pane with no [`Pane::aimed`] draws the row it drew
    // before this control existed.
    //
    // **And neither where the row cannot hold them**, which is the one place
    // this row's *a control that does not fit is no control at all* is answered
    // by dropping part of the row rather than all of it. The reason is a
    // measurement: an Inspector pane at the console's declared minimum window
    // is 237 pixels wide and the five chips that were here already come to
    // within a couple of dozen of that, so a row that took all seven or none
    // would answer *none* at the width this arrangement claims to work at —
    // trading two controls that were never there for four that were. So the two
    // build chips are dropped first and the row goes on drawing what it drew
    // before them, and the page says so rather than leaving an operator to
    // discover it by dragging
    // ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    let aim = pane.aimed.as_ref().and_then(|aimed| {
        let word = aimed.capacity.to_string();
        let width_of = |text: &str| mini(text, row.min.x).width();
        let salt = mini(
            RE_SALT_LABEL,
            composite.min.x - size::DECK_HEAD_GAP - width_of(RE_SALT_LABEL),
        );
        let size_at = mini(&word, salt.min.x - size::DECK_HEAD_GAP - width_of(&word));
        let fits =
            row.contains_rect(size_at) && forward.max.x + size::DECK_HEAD_GAP <= size_at.min.x;
        fits.then_some(AimChips {
            size: size_at,
            resize: stepped_capacity(&aimed.capacities, aimed.capacity),
            salt,
            re_salt: aimed.salt,
        })
    });

    if !row.contains_rect(mode)
        || !row.contains_rect(composite)
        || forward.max.x + size::DECK_HEAD_GAP > composite.min.x
    {
        return None;
    }

    Some(DeckHead {
        mode,
        anchor,
        back,
        forward,
        composite,
        aim,
        deck: pane.deck,
        composited: pane.composite,
        locked: pane.sync,
        allows: pane.allows,
    })
}

/// The `keep` pill in a pane's head, laid out — the capsule at the right of
/// `.half-head`, and the one control in this bay that performs rather than
/// sets.
///
/// # It keeps the pane's deck, and `k` keeps the selection
///
/// The mock draws one of these per pane and the tooltip names the pane's own
/// deck: *"Keep deck A as a Set, exactly as it is on screen."* So this carries
/// [`Pane::deck`] the way [`DeckHead::deck`] does, and a press answers with the
/// deck the pill was measured for — a pill in the second pane keeps that pane's
/// deck while the selection stays where the operator put it. The key `k` keeps
/// *the selected deck*, because a bare key press cannot say which, and the two
/// are one operation asked for from two ends
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
///
/// # What it files it under
///
/// [`Operation::SaveSet`] with no id, which is the same call the key makes and
/// is a decision rather than an omission
/// ([ADR-0287](../../../../docs/adr/0287-the-keep-pill-files-under-a-stamp-because-the-consoles-one-letter-taking-flow-is-an-arrangements-name.md)).
/// What the store does with a `None` is `karakuri_environment::accepted_save`'s
/// convention — a stamp, because *"an operator looks for the time they saved
/// it"*.
///
/// The reason has changed and the decision has not. ADR-0287 argued the `None`
/// from there being one letter-taking flow on this console and it being an
/// arrangement's; there are two now, and the second is the name in the head
/// beside this capsule
/// ([ADR-0292](../../../../docs/adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md)).
/// What holds the capsule at `None` from here on is
/// [ADR-0128](../../../../docs/adr/0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md)
/// rather than the absence of a field: *"an operator's own act gets the name it
/// asked for; a key press cannot type one and takes a stamp"*. This is the
/// press that types nothing, so this is the one that takes the stamp — see
/// [`DeckName`] for the one that does not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeepPill {
    /// The capsule, which is what a press has to land in. The mock gives the whole
    /// pill the click and so does this — [`ArrangementPill::pill`]'s own reading.
    pub pill: Rect,
    /// Which deck this pill keeps, as [`Pane::deck`] — carried so that a press
    /// answers with the deck it was measured for, which is [`DeckHead::deck`]'s
    /// reason one row down.
    pub deck: usize,
}

impl KeepPill {
    /// Whether `p` is on the capsule, which is the whole of what this control owns:
    /// there is no menu under it and no second target beside it.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// What a press at `p` asks for, or `None` off the capsule.
    ///
    /// The same derivation [`crate::input::claim`] hit-tests, asked a second time
    /// rather than copied — [`DeckHead::sync`]'s arrangement, and the reason is the
    /// same one row up: the pill that claims a press and the pill that acts on it
    /// cannot come apart.
    ///
    /// It refuses nothing. What a keep costs and whether the store will take it are
    /// the instrument's answers rather than this surface's, and the operation is
    /// *"on a worker"* on the page it is specified on — the press leaves and the
    /// answer arrives later.
    pub fn keep(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit(p).then_some(Operation::SaveSet {
            deck: self.deck as u8,
            // **`None`, and it is the payload saying so rather than this
            // control inventing a stamp** — the sentence
            // `crates/karakuri/src/main.rs` already writes over the `k` arm,
            // and the same one: *"a caller that can type a name is not made to
            // take a timestamp"*, and this control is not one of them.
            id: None,
        })
    }
}

/// The pane head's pill, derived — [`inspector`] answers where the head is and
/// this answers where the capsule in it is, which is [`deck_head`]'s division
/// one row down.
///
/// `.sep`'s `flex: 1` puts it hard against the head's right-hand padding, and
/// one padding down from the top rather than centred in the row: the rule at
/// the bottom is inside `.half-head`, so the row's middle is half a pixel below
/// the middle of its content box — which is the scope row's own note one bay
/// along, on a row built the same way.
///
/// # What it costs to ask
///
/// One galley lookup per pane, for the word in the capsule, on a pointer event
/// and on a frame — [`deck_head`]'s three beside it, and paid the same way.
///
/// # `None` is a head that cannot hold it
///
/// [`deck_head`]'s rule and [`look`]'s: *a control that does not fit in the row
/// it is drawn in is no control at all, rather than half of one*. The words to
/// its left are a readout and are clipped; the pill is a target and is not
/// drawn where it would be cut.
pub fn keep_pill(ctx: &egui::Context, at: &InspectorPane, pane: &Pane) -> Option<KeepPill> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`deck_head`] — and on the frame before the first one there is nothing
    // drawn here to press.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let head = at.head;
    let w = pill_width(ctx, KEEP_LABEL);
    let pill = Rect::from_min_size(
        Pos2::new(
            head.max.x - size::HALF_HEAD_PAD_X - w,
            head.min.y + size::HALF_HEAD_PAD_Y,
        ),
        egui::vec2(w, size::PILL_H),
    );
    // **Measured against `.half-head`'s content box and not against the row**,
    // because a flex item cannot be laid out inside its parent's padding: the
    // capsule is placed from the right-hand padding, so what it runs off is
    // the left one, and a head with less room between its two paddings than
    // the word needs draws none. [`positive`] is what says the head is a row
    // at all — a pane with no height has one that is not.
    let room = head.width() - size::HALF_HEAD_PAD_X * 2.0;
    (positive(head) && pill.width() <= room).then_some(KeepPill {
        pill,
        deck: pane.deck,
    })
}

/// The slot's MCP policy pill in a pane's head, laid out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotMcpPill {
    pub pill: Rect,
    pub deck: usize,
    pub policy: SlotPolicy,
}

impl SlotMcpPill {
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }
}

pub fn slot_mcp_pill(
    ctx: &egui::Context,
    at: &InspectorPane,
    pane: &Pane,
    policy: SlotPolicy,
) -> Option<SlotMcpPill> {
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let head = at.head;
    let right = match keep_pill(ctx, at, pane) {
        Some(keep) => keep.pill.min.x - size::PILL_GAP,
        None => head.max.x - size::HALF_HEAD_PAD_X,
    };
    let w = pill_width(ctx, policy.pill_word());
    let pill = Rect::from_min_size(
        Pos2::new(right - w, head.min.y + size::HALF_HEAD_PAD_Y),
        egui::vec2(w, size::PILL_H),
    );
    let min_left = head.min.x + size::HALF_HEAD_PAD_X;
    (positive(head) && pill.min.x >= min_left).then_some(SlotMcpPill {
        pill,
        deck: pane.deck,
        policy,
    })
}

/// The word in the capsule, which is the mock's own and is the row's name in
/// the panel column of [every
/// operation](../../../../docs/manual/operations.html).
pub(crate) const KEEP_LABEL: &str = "keep";

/// The letter of the deck a pane is pointed at, or `?` for a pane pointed past
/// the end of [`DECK_LETTERS`] — which is a caller's error and not a state, and
/// is drawn rather than panicked for [`showing_text`]'s reason: a head is a
/// readout and a readout does not stop a frame.
fn deck_letter(pane: &Pane) -> &'static str {
    DECK_LETTERS.get(pane.deck).copied().unwrap_or("?")
}

/// What the pane head reads: the mock's `deck A · drift_night`.
pub(crate) fn showing_text(pane: &Pane) -> String {
    format!("deck {} · {}", deck_letter(pane), pane.material)
}

/// What that same run reads while the head is taking letters — `deck A ·
/// glass_sh▏`, with [`CARET`] after it as the arrangement's field has.
///
/// The deck stays and the material goes. What is being typed is the name this
/// deck's material will be filed under, so the run says which deck is being
/// filed for the whole of the gesture — and the half of it that is replaced is
/// exactly the half a name is. A field that had cleared the run would take the
/// one word that says *whose* name this is off the screen at the moment an
/// operator is looking hardest at it.
pub(crate) fn naming_text_in_head(pane: &Pane, typed: &str) -> String {
    format!("deck {} · {typed}{CARET}", deck_letter(pane))
}

/// The word the mock puts in front of it.
const SHOWING_LABEL: &str = "showing";

/// The word in front of the run while the head is taking letters, where
/// [`SHOWING_LABEL`] is the word in front of it the rest of the time.
///
/// The row stops being a readout the moment letters are going into it, and the
/// label is the only thing that can say what they are *for*: they name the Set
/// the capsule at the other end of the same row files. It is [`KEEP_LABEL`]'s
/// own word rather than a new one, which is [`SAVE_ITEM_ASKING`]'s arrangement
/// three bays along — the thing that asks for something says so in the verb it
/// is about to perform.
const NAMING_LABEL: &str = "keep as";

/// The word this head has in front of its run, which is the one thing about the
/// row that says whether it is reading or asking.
pub(crate) fn head_label(naming: Option<&str>) -> &'static str {
    match naming {
        Some(_) => NAMING_LABEL,
        None => SHOWING_LABEL,
    }
}

/// The name in a pane head, laid out — the mock's `.what`, and this console's
/// second letter-taking flow
/// ([ADR-0292](../../../../docs/adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md)).
///
/// # A press on it names the Set, and the capsule beside it goes on stamping
///
/// [ADR-0128](../../../../docs/adr/0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md)
/// is what puts two routes on one row: *"an operator's own act gets the name it
/// asked for; a key press cannot type one and takes a stamp"*. The `keep`
/// capsule is the second of those and is unchanged — [`KeepPill::keep`] emits
/// `id: None` exactly as ADR-0287 decided — and this is the first: a press here
/// puts the head into [`Naming`], and the commit is [`View::named_set`]'s `id:
/// Some(typed)`.
///
/// # What it does not claim, and that is the whole of its right-hand edge
///
/// The mock's head is `showing`, the name, `▾`, `.sep`, `keep`. The `▾` is the
/// chooser — *point this pane at another deck* — which is [`Pane::deck`]'s
/// per-pane pointer and is still not a control this console has (ADR-0200). It
/// is not drawn, and this derivation reserves [`DeckName::chevron`] for it
/// anyway: the target is the run's own ink and stops there, so the day the
/// chooser lands it takes the rectangle beside the name rather than taking it
/// *back*. A name target that had run to the capsule would have swallowed the
/// chooser's place before anybody drew it, and a press meant for the caret
/// would be a press that re-points the pane.
///
/// # The run is one target and is deliberately not two
///
/// `deck A · drift_night` is one `.what` in the mock and one galley here.
/// Claiming the material and leaving `deck A ·` a readout would be a boundary
/// inside a run of text with nothing on screen drawing it, which is the
/// opposite of *a control claims what it acts on and no more*: what this acts
/// on is the name display, and the name display is the whole run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeckName {
    /// The run as it is painted, clipped to what the head has room for — see
    /// [`deck_name`]. A press has to land in this and nowhere else.
    pub name: Rect,
    /// Where the mock's `▾` goes, one `.half-head` gap after the run. Drawn by
    /// nobody and claimed by nobody: it is the chooser's place, held so that this
    /// control's edge is a measured thing rather than a comment.
    pub chevron: Rect,
    /// Which deck this head names, as [`Pane::deck`] — carried for
    /// [`KeepPill::deck`]'s reason one capsule along.
    pub deck: usize,
}

impl DeckName {
    /// Whether `p` is on the run, which is the whole of what this control owns: the
    /// label to its left is a readout, and the rectangle to its right is the
    /// chooser's.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.name.contains(Pos2::new(p.x, p.y))
    }
}

/// What the count at the right of a pane head reads — `n of m`, the node groups
/// this pane is showing whole out of the ones the deck has.
///
/// The Library foot's `5 of 27` counted on this bay's items rather than on that
/// one's rows, which is what
/// [ADR-0259](../../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
/// says a pane's items are: *"Items are its panes; a pane's controls are its
/// deck head and its node groups"*. A percentage would be a number about a
/// rectangle, and what an operator counts is groups.
pub fn count_text(at: &InspectorPane, pane: &Pane) -> String {
    format!("{} of {}", at.shown, pane.nodes.len())
}

/// The count in a pane head, derived — where the run goes, or `None` for a head
/// with no room for it between the label and the capsule.
///
/// It is a readout: nothing hit-tests it, it names no operation, and it carries
/// no row on [every operation](../../../../docs/manual/operations.html) — the
/// mixer head's `3 of 3 · page 1` one bay along, and the reason is the same one
/// that keeps the Library's cursor off that page. What it is *for* is rule 04 —
/// *"A list that showed you part of itself says so and says how much"* — which
/// is the whole of what a scrolled pane owes a reader, and is why this is
/// derived beside the two controls in the row rather than painted wherever
/// there happened to be space.
///
/// # Where it sits, and what gives way to what
///
/// `.half-head` is a flex row: the label and the run are at the left, `.sep`
/// takes what is over, and the capsule is hard against the right-hand padding.
/// This goes one [`size::HALF_HEAD_GAP`] to the left of the capsule — the mixer
/// head's order, where the readout is left of the pill — and the run to its
/// left is what gives way when the pane is narrowed, because the run is the one
/// thing in the row that is clipped rather than dropped.
///
/// `None` is a head that cannot hold it, which is [`keep_pill`]'s rule read on
/// a readout: measured against the room between the label and the capsule, so a
/// head that would have to draw this over the words draws none of it. A pane at
/// the declared minimum of 208 has room for all three
/// ([ADR-0279](../../../../docs/adr/0279-the-centre-is-two-parameter-rows-wide-because-a-pane-that-cannot-draw-a-fader-is-not-a-minimum.md)),
/// so this `None` is a pane below what the arrangement admits rather than a
/// state rule 04 is broken in.
pub fn pane_count(
    ctx: &egui::Context,
    at: &InspectorPane,
    pane: &Pane,
    naming: Option<&str>,
    mcp: Option<Rect>,
) -> Option<Rect> {
    // Fonts are not valid until `egui` has run a pass — [`keep_pill`]'s guard,
    // and before the first one there is no head painted to read.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let head = at.head;
    if !positive(head) {
        return None;
    }
    let run = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    let right = match mcp {
        Some(mcp_rect) => mcp_rect.min.x - size::HALF_HEAD_GAP,
        None => match keep_pill(ctx, at, pane) {
            Some(pill) => pill.pill.min.x - size::HALF_HEAD_GAP,
            None => head.max.x - size::HALF_HEAD_PAD_X,
        },
    };
    // Where the run beside it would start: the label inside the left padding,
    // and one gap. This is measured against that rather than against the
    // head's edge so that a head narrow enough to want the room for its words
    // keeps it — the words are what says *which deck*, and a count of groups
    // on a pane whose deck has gone unnamed is a number about nothing.
    let left = head.min.x + size::HALF_HEAD_PAD_X + run(head_label(naming)) + size::HALF_HEAD_GAP;
    let w = run(&count_text(at, pane));
    let top = head.min.y + size::HALF_HEAD_PAD_Y;
    (right - w >= left).then(|| {
        Rect::from_min_max(
            Pos2::new(right - w, top),
            Pos2::new(right, top + size::PILL_H),
        )
    })
}

/// The pane head's name, derived — [`inspector`] answers where the head is and
/// this answers where the run in it is, which is [`keep_pill`]'s division along
/// the same row.
///
/// `naming` is what the head is taking letters into, or `None` for a head that
/// is reading — and it is a parameter rather than a field of [`Pane`] because a
/// pane is rewritten whenever a Set lands ([`View::inspector`]) and a buffer
/// kept there would be a name that vanished mid-word. It lives in
/// [`View::naming_set`], which is [`Arrangement::menu`]'s argument on a second
/// control: what a *control* is doing is this crate's, and it is not part of
/// anything a host hands in.
///
/// # What it costs to ask
///
/// Three galley lookups per pane — the label, the run, and [`keep_pill`]'s
/// word, because where the run may be painted to is where the capsule starts.
/// The capsule is derived here rather than passed in for [`crate::input`]'s own
/// reason one bay along, where [`on_pill`](crate::input) derives the tracker
/// group as well: one derivation asked twice cannot come apart, and two
/// arguments that a caller could fill from two frames can.
///
/// # `None` is a head with no ink to press
///
/// [`keep_pill`]'s rule read on a readout instead of on a capsule. The run is
/// clipped where the words are clipped — one `.half-head` gap short of the
/// capsule — so a head narrow enough that the label alone fills it leaves no
/// name on screen, and a target over ink nobody can see is a press that lands
/// on nothing an operator could have aimed at.
pub fn deck_name(
    ctx: &egui::Context,
    at: &InspectorPane,
    pane: &Pane,
    naming: Option<&str>,
    mcp: Option<Rect>,
) -> Option<DeckName> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`keep_pill`] — and on the frame before the first one there is nothing
    // drawn here to press.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let head = at.head;
    if !positive(head) {
        return None;
    }
    let run = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };
    // **The same clip [`inspector_into`] paints the words inside**, written
    // once here and read there: everything up to whatever is next along the
    // row, one `.half-head` gap short of it. That is the count where the head
    // has room for one ([`pane_count`]), the mcp pill where it has one, the
    // capsule where it has not, and the head's own edge where it has neither.
    let limit = match pane_count(ctx, at, pane, naming, mcp) {
        Some(count) => count.min.x - size::HALF_HEAD_GAP,
        None => match mcp {
            Some(pill) => pill.min.x - size::HALF_HEAD_GAP,
            None => match keep_pill(ctx, at, pane) {
                Some(pill) => pill.pill.min.x - size::HALF_HEAD_GAP,
                None => head.max.x,
            },
        },
    };
    // **And the chooser's own room comes off it**, which is the half of
    // ADR-0292's *the chooser is boxed in* that the chooser landing makes
    // real: `.half-head` is `showing`, the run, `▾`, `.sep`, the count and the
    // capsule, so the `▾` sits **between** the run and everything else in the
    // row. The run is the one thing here that is clipped rather than dropped
    // (`pane_count`'s own note), so it is the run that gives way and never the
    // control. Before this the chevron was reserved and unpainted, and its
    // rectangle could sit on top of the count in a narrow head — which cost
    // nothing while nobody drew it and would be a target over another
    // control's ink now that somebody does.
    let limit = limit - (CHEVRON_W + size::HALF_HEAD_GAP);
    let left = head.min.x + size::HALF_HEAD_PAD_X + run(head_label(naming)) + size::HALF_HEAD_GAP;
    let text = match naming {
        Some(typed) => naming_text_in_head(pane, typed),
        None => showing_text(pane),
    };
    let right = (left + run(&text)).min(limit);
    if right <= left {
        return None;
    }
    let top = head.min.y + size::HALF_HEAD_PAD_Y;
    let name = Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, top + size::PILL_H));
    Some(DeckName {
        // **The chooser**, one gap after the run and at the glyph's own
        // measure — [`CHEVRON_W`], which is the arrangement pill's `▾` three
        // bays along. It was reserved and drawn by nobody until 2026-09-10
        // (ADR-0292's *the chooser is boxed in*), and it is a control now:
        // [`pane_target`] is what paints and hit-tests it, off this
        // rectangle. What has not changed is that [`DeckName::name`] stops
        // before it — the run is one target and the mark beside it is
        // another.
        chevron: Rect::from_min_size(
            Pos2::new(
                name.max.x + size::HALF_HEAD_GAP,
                name.center().y - CHEVRON_H * 0.5,
            ),
            egui::vec2(CHEVRON_W, CHEVRON_H),
        ),
        name,
        deck: pane.deck,
    })
}

/// The pulldown on a pane head, and the card it brings down — *point this pane
/// at another deck*.
///
/// # It is the pane's own pointer and it is not the deck selection
///
/// A pick moves this pane and nothing else: not the deck the keys are addressed
/// to ([`View::selection`]), not the pane next door, and not the Library bay's
/// load target ([`View::target_deck`]). That is the whole of why the mark
/// exists — a pane can show a deck the keys are not on — and it is [`Load`]'s
/// argument one bay along (`docs/adr/0305-…`, `docs/adr/0338-…`, decision 5).
///
/// # A pulldown and not a flip
///
/// The maintainer's choice, and
/// [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// underneath it: a flip is a *step*, so two panes stepping cannot both be
/// aimed without knowing where they started, and a key, a map line or a model
/// would have to count presses to say *deck C*. Every row of this card names a
/// destination.
///
/// # What it offers is what the mixer is drawing
///
/// [`Target::decks`]' count read a second time and not a second rule: a deck
/// the mixer draws no strip for is not in the list, which is [`View::select`]'s
/// own refusal met from one more direction.
///
/// # The card hangs down, as the `uses` line's does
///
/// It is inside a pane's body's own bay rather than in a foot, so what is under
/// the head is the pane — [`UsesLine::list`]'s division, and it is held inside
/// the viewport for that method's reason.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaneTarget {
    /// The `▾` after the run — [`DeckName::chevron`], made live. A press on it puts
    /// the card down; a press on it while the card is down is the host's to read as
    /// *shut it*, which is [`Load`]'s arrangement.
    pub chevron: Rect,
    /// Which pane this head belongs to, as an index into [`PANE_NAMES`] — what
    /// [`Operation::PointPane`]'s `pane` is spelled from, and what says which of
    /// [`View::pane_deck`]'s entries a pick moves.
    pub pane: usize,
    /// How many decks the card offers, which is how many strips the mixer is
    /// drawing while it is down and zero while it is shut — [`Load::rows`]' shape
    /// and its reason: [`PaneTarget::row`] cannot hand out a rectangle for a card
    /// nobody opened.
    pub rows: usize,
}

impl PaneTarget {
    /// Whether `p` is on the mark, which is the whole of what the shut control
    /// owns: the run to its left is [`DeckName`]'s and the count to its right is a
    /// readout.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.chevron.contains(Pos2::new(p.x, p.y))
    }

    /// The card under the mark, or `None` while it is shut — and `None` for a
    /// console with no strip to offer, which is every test in this crate that hands
    /// no mixer in.
    pub fn list(&self, viewport: Rect) -> Option<Rect> {
        if self.rows == 0 {
            return None;
        }
        let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * self.rows as f32;
        let width = size::LIB_ROW_H + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0;
        Some(held_inside(
            &viewport,
            self.chevron.min.x,
            self.chevron.max.y + size::PILL_GAP,
            width,
            height,
        ))
    }

    /// Where the `index`th deck's row is, from the top of `card` — the decks in
    /// [`DECK_LETTERS`] order, which is [`Load::row`]'s own reading.
    ///
    /// Panics on a row this card has not got, which is that method's rule: a caller
    /// has invented a deck.
    pub fn row(&self, card: Rect, index: usize) -> Rect {
        assert!(index < self.rows, "deck {index} of a list of {}", self.rows);
        Rect::from_min_size(
            Pos2::new(
                card.min.x + size::LIB_LIST_PAD,
                card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32,
            ),
            egui::vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        )
    }

    /// What a press at `p` on the card asks for, or `None` off every row.
    ///
    /// The pane is named by [`PANE_NAMES`], which is the arrangement's own handle
    /// for it — `karakuri-operation` has no dependencies and cannot hold one, which
    /// is [`Operation::FoldPane`]'s spelling and its reason.
    pub fn picked(&self, viewport: Rect, p: karakuri_layout::Point) -> Option<Operation> {
        let card = self.list(viewport)?;
        let at = Pos2::new(p.x, p.y);
        let deck = (0..self.rows).find(|index| self.row(card, *index).contains(at))?;
        Some(Operation::PointPane {
            pane: PANE_NAMES.get(self.pane)?.to_string(),
            deck: deck as u8,
        })
    }
}

/// The pulldown on one pane head, derived — [`deck_name`] answers where the run
/// is and this answers where the mark after it is, which is [`keep_pill`]'s
/// division along the same row.
///
/// `None` is a head with no run drawn in it, which is [`deck_name`]'s own
/// refusal: the mark sits one gap after the run, so a head too narrow to paint
/// any of the name has nowhere to put it. The run is clipped short of this mark
/// rather than over it — see [`deck_name`], where that is one line.
pub fn pane_target(
    ctx: &egui::Context,
    at: &InspectorPane,
    pane: &Pane,
    index: usize,
    naming: Option<&str>,
    decks: usize,
    open: bool,
    mcp: Option<Rect>,
) -> Option<PaneTarget> {
    let named = deck_name(ctx, at, pane, naming, mcp)?;
    Some(PaneTarget {
        chevron: named.chevron,
        pane: index,
        // **Zero while it is shut**, which is what stops [`PaneTarget::row`]
        // handing out a rectangle for a card nobody opened — [`Load`]'s own
        // field.
        rows: match open {
            true => decks.min(DECK_LETTERS.len()),
            false => 0,
        },
    })
}

/// A pane head taking letters, and the whole of the console's second
/// letter-taking flow's state.
///
/// # One at a time, and it carries which head it is in
///
/// [`Menu::Naming`] is the first flow and it is one because a menu is one; this
/// is one because the keyboard is one. Whoever holds the keys takes them whole
/// while a name is being asked for — `s` is an `s` in a name and not a solo —
/// so two open fields would be two places one keystroke could go, with nothing
/// on the panel saying which. So this is an `Option` on the console and not a
/// field per pane, and it names the pane the field is drawn in.
///
/// # The buffer is a `String` this crate owns and does not check
///
/// [`Menu::Naming`]'s rule, unchanged and for its reason: a name that is not
/// one path component is refused where the file is written, in one sentence, by
/// whoever writes it — the surface owns the affordance and never the authority
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
/// A head that quietly dropped the characters it did not like would be a rule
/// an operator could only find by experiment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Naming {
    /// Which pane head the field is in, as an index into [`View::inspector`] and so
    /// into [`PANE_NAMES`].
    pub pane: usize,
    pub(crate) typed: String,
}

impl Naming {
    /// What has been typed so far. The caret is drawn after it and there is no
    /// selection: this is a name, not a document — [`Arrangement::naming`]'s own
    /// sentence.
    pub fn typed(&self) -> &str {
        &self.typed
    }
}

/// The word on the fold chip, which is the mock's own and is drawn whether or
/// not it changes anything: *"A deck publishing a single renderer draws the
/// chip anyway and says that it changes nothing either way, because a deck that
/// grows a second one needs the control already where it was."*
const COMPOSITE_LABEL: &str = "composite";

/// The deck head, painted: the sync chip, the anchor beside it, the two
/// scrub arrows and the fold at the right.
///
/// Where everything goes is [`deck_head`]'s, so this paints and derives
/// nothing — which is the change this pass made to it: the row used to be a
/// running sum here and a press had nowhere to ask what it had landed on.
///
/// Term for term from `style.css`:
///
/// - `.mini` — the mode chip and the fold, through [`mini_into`], selected
///   because a mode is always one of three and the fold is on or off.
/// - `.anchor` — `font-size: 9px` in `--c-faint`, the run [`anchor_text`]
///   writes.
/// - `.scrub i` — `padding: 0 4px; border-radius: 999px; border: 1px solid
///   var(--c-line)` with `--c-dim` inside it, and `.scrub.idle i`'s
///   `--c-faint` over `--c-hair` where the deck is not beat-synced. Never
///   grey without a reason, which is the stylesheet's own note on this pair:
///   an inert scrub keeps its shape, and what says why is the page.
///
/// The arrows are drawn rather than typed, which is [`CHEVRON_W`]'s reason
/// three bays along: whether a black left-pointing small triangle is in
/// `egui`'s default face is a question with no good answer, and a triangle is
/// the same mark either way.
pub(crate) fn deck_head_into(ui: &Ui, pal: &Palette, at: &DeckHead, pane: &Pane) {
    let painter = ui.painter().with_clip_rect(at.mode.union(at.composite));
    let word = |rect: Rect, text: &str, sel: bool| {
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::MINI_SIZE, FontFamily::Proportional),
            pal.faint,
        );
        mini_into(&painter, pal, rect, sel, |painter, colour| {
            painter.galley(
                Pos2::new(
                    rect.min.x + size::MINI_PAD_X,
                    rect.center().y - galley.size().y * 0.5,
                ),
                galley,
                colour,
            );
        });
    };
    word(at.mode, sync_word(pane.sync), true);

    if let (Some(rect), Some(text)) = (at.anchor, anchor_text(pane)) {
        let galley = painter.layout_job(span_at(&text, size::ANCHOR_SIZE, pal.faint));
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            pal.faint,
        );
    }

    // Live is the one mode that reads the offset; the other two keep the
    // chips and lose the ink, which is `.scrub.idle`.
    let live = pane.sync == Sync::Beat;
    let (ink, edge) = match live {
        true => (pal.dim, pal.line),
        false => (pal.faint, pal.hair),
    };
    for (rect, back) in [(at.back, true), (at.forward, false)] {
        painter.rect_stroke(
            rect,
            CornerRadius::same((size::SCRUB_H * 0.5) as u8),
            Stroke::new(size::HAIRLINE, edge),
            StrokeKind::Inside,
        );
        arrow_mark(&painter, rect.center(), size::SCRUB_SIZE, ink, back);
    }

    if let (Some(chips), Some(aimed)) = (at.aim, pane.aimed.as_ref()) {
        // **Lit says somebody asked for this number**, and unlit says it is
        // what the material declares for itself — which is what `.mini.sel`
        // already means on this row for the fold beside it: the chip's two
        // states answer *who chose this* rather than restating the number.
        word(chips.size, &aimed.capacity.to_string(), aimed.stated);
        // **Never lit**, because a capsule that performs has no state to be in
        // — the `keep` pill's arrangement two rows up.
        word(chips.salt, RE_SALT_LABEL, false);
    }

    word(at.composite, COMPOSITE_LABEL, pane.composite);
}

/// A pane head's deck list, painted — [`deck_list_into`]'s card one bay along,
/// with the deck the pane is *showing* in the panel's own text colour and the
/// rest dim.
///
/// Where everything goes is [`PaneTarget`]'s, so this paints and derives
/// nothing, which is [`deck_list_into`]'s own sentence.
///
/// The marked row is what this pane is pointed at and never the deck selection,
/// which is the whole of what this mark is: a pane showing deck C while the
/// keys are on deck A draws `C` in the text colour here and the ring stays on
/// A's strip, one bay over.
pub fn pane_list_into(ui: &Ui, pal: &Palette, target: &PaneTarget, showing: usize, card: Rect) {
    let painter = ui.painter();
    popup_card(painter, pal, card);
    // `take` rather than a range, because the rows are the letters — see
    // [`deck_list_into`], and `View::point_pane` is what stops a deck this
    // crate has no letter for being asked for.
    for (index, letter) in DECK_LETTERS.iter().enumerate().take(target.rows) {
        let row = target.row(card, index);
        let ink = match index == showing {
            true => pal.text,
            false => pal.dim,
        };
        let galley = painter.layout_no_wrap(
            (*letter).to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            ink,
        );
        painter.galley(
            Pos2::new(
                row.min.x + size::LIB_ROW_PAD_X,
                row.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
    }
}
