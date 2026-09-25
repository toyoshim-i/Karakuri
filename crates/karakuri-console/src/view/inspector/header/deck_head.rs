use super::super::*;

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

/// The letter the anchor readout leads with (`T` for tempo sync, `B` for beat sync, none for free).
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
    /// The anchor, or `None` under [`Sync::Free`]. Sized to [`size::MINI_H`].
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
