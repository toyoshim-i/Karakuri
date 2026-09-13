use std::time::Duration;

use super::super::*;
use super::audio_in::Rec;
use super::*;

// ---------------------------------------------------------------------------
// The Transport bay
// ---------------------------------------------------------------------------

/// What the transport row reads this frame: the session's tempo and position,
/// and what the frame before this one cost.
///
/// # The same seam as [`View::picture`], and it is not crossed
///
/// Every field is a number and none of them is a `Deck`, an `Instant` or a
/// `Duration` that means *now*. This crate takes no device, no window and no
/// clock (ADR-0156), and this row is made of a clock and an engine — so whoever
/// owns those reads them and writes this per frame, exactly as whoever owns the
/// device registers a texture and writes [`View::picture`]. A `Deck` here would
/// put the engine in this crate's dependencies; an `Instant` here would put a
/// clock in it, and then the row would be reading wall time in a repository
/// whose first principle is that nothing does
/// ([P-0092](../../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
///
/// It carries what cannot be derived and nothing that can. The beat within the
/// bar and the bar number are arithmetic on [`Transport::beats`] and are
/// [`Transport::beat`] and [`Transport::bar`] rather than two more fields:
/// three numbers for one position is three chances for the lit dot and the bar
/// beside it to come from different arithmetic and disagree.
///
/// No repaint arm. [`crate::repaint::Change`] is one list of everything that
/// can change what the console shows, and this is not on it: the values move
/// when the engine draws a frame, and a caller that is drawing engine frames is
/// already asking for frames for the picture beside this row. A panel with
/// nothing live on it is a panel where the oscillator is not advancing either,
/// so the row is right to be still — see [`Transport::fps`], which is the one
/// field that would go stale there and is the one the program leaves `None`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transport {
    /// The tempo, drawn in `.bpm`'s treatment. `karakuri_signal`'s
    /// `Oscillator::bpm` is an `f32` and so is this.
    ///
    /// Not derivable from [`Transport::beats`], which is why it is its own field:
    /// the oscillator's position is an accumulator precisely so that a tempo
    /// correction changes the rate from now on without moving a beat that has
    /// already happened, so the tempo is not the slope of the position and cannot
    /// be read off it.
    pub bpm: f32,
    /// Musical position, unbounded and monotone — `Oscillator::beats`, which counts
    /// from the start of the session and never wraps.
    ///
    /// `f64` because that is the type the oscillator accumulates it in, and
    /// narrowing a number this crate does not own is a rounding taken for nothing.
    /// What it would eventually cost is real but distant — an `f32` stops
    /// separating consecutive beats somewhere past eight million of them — so the
    /// reason to keep it is the first one and not the second.
    ///
    /// Which dot is lit and which bar it is are both read off this one number — see
    /// [`Transport::beat`] and [`Transport::bar`].
    pub beats: f64,
    /// How many beats there are in a bar, which is how many dots the grid has.
    ///
    /// A field rather than a constant, and that is a conclusion rather than
    /// caution. The mock draws four and `karakuri_signal`'s
    /// `oscillator::BEATS_PER_BAR` is 4, so the number is not in doubt today — but
    /// that constant says of itself that *"v0.2 of the IR spec has no
    /// time-signature concept anywhere … a fixed assumption of common time rather
    /// than something derived from the record stream. Treat it as provisional until
    /// a real time signature shows up in the format."* A constant here would be
    /// this crate transcribing a number the crate that owns it has written down as
    /// provisional, and the day a time signature lands the console would draw four
    /// dots against a grid of three with nothing saying so. So it is asked of
    /// whoever owns the grid, once a frame, and the grid is that many dots wide.
    ///
    /// Read through [`Transport::dots`], which is where a zero is dealt with.
    pub beats_per_bar: u32,
    /// Frames a second, or `None` where nobody can say yet.
    ///
    /// A rate is measured over a stretch and is a claim about now, so a window that
    /// has not drawn a stretch yet, or has nothing live on it, has no rate — and
    /// `None` draws no `fps` at all rather than a `0` or a stale number. Not
    /// derived from [`Transport::frame_ms`]: `1000 / frame_ms` is the rate the
    /// loop *could* manage, and what it *does* draw is decided by
    /// the display and by whether anything asked for a frame. On a `Fifo` surface
    /// those two differ by the whole vsync wait, which is most of the frame.
    pub fps: Option<f32>,
    /// What one frame cost on the CPU, in milliseconds — the `12.4` in the mock's
    /// `12.4/16.6 ms`.
    ///
    /// It is a fact about a frame that has already been drawn, so it does not go
    /// stale on a still window the way a rate does: the last frame did cost this.
    pub frame_ms: f32,
    /// What a frame has to fit in, in milliseconds — the `16.6`, and `None` where
    /// nothing can say what it is.
    ///
    /// Then the row draws the frame time with no budget beside it, which is the
    /// rule the whole of this value follows: a reading nobody has is not drawn as a
    /// plausible one.
    pub budget_ms: Option<f32>,
    /// What the master chain costs this frame, in milliseconds — the `+ 0.98
    /// chain` after the budget, and `None` for a chain with no slot in it and for
    /// a console with no engine behind it.
    ///
    /// A term beside the decks and charged to none of them
    /// ([ADR-0349](../../../../docs/adr/0349-the-chain-is-the-frames-so-it-reads-the-session-clock-and-is-charged-against-the-frames-budget.md)).
    ///
    /// It is an estimate and not a measurement — one rate against the chain's
    /// static op count and the frame's area — where [`Transport::frame_ms`]
    /// beside it is a measurement. Zero draws nothing rather than a `0.00`,
    /// which is the rule the two readings above follow.
    pub chain_ms: Option<f32>,
    /// What the last write did — the mock's `landed` capsule at the end of this
    /// row, and `None` until a write has done anything.
    ///
    /// # It is [`Stage`] and not a fourth spelling of the same three words
    ///
    /// `docs/manual/console.html` gives the pill and the Staging lane's rows one
    /// sentence apiece and they are the same answers — *"landed, overloaded, failed
    /// to build, or did not compile"* under *Health, in the transport*, and
    /// *"whether it is on screen: landed, overloaded for costing more than one
    /// frame may, refused, or did not compile"* on a candidate row. A second enum
    /// here would be `docs/contributing.md` §4's *a name meaning two things* built
    /// on purpose, so [`Stage`] has two readers and one set of words.
    ///
    /// The two readings are not the same reading, which is why both are drawn. A
    /// candidate row is one deck slot with a verdict *outstanding* and leaves the
    /// lane the moment the watchdog says the version held the budget — *"empty is
    /// this lane's ordinary state"*. This is the last verdict there was, on
    /// whichever slot, and it stands after the lane has emptied. So the lane
    /// answers *what is unsettled* and this answers *what did the last write do*,
    /// which is the row the operations page gives this pill and gives the lane's
    /// controls a different one.
    ///
    /// # `None` is *nothing has been written*, and it draws no capsule
    ///
    /// Not a word for it, not a dash, and not an `armed` pill reading `landed`
    /// about a build nobody made — the rule the two fields above follow, and the
    /// Staging lane's own *"no row, no placeholder, and no standing sentence"*. A
    /// run in which nobody rewrites a procedure never draws this, which is most
    /// runs.
    ///
    /// Two `swap::Event`s leave it alone rather than clearing it. The watchdog's
    /// verdict in favour settles a candidate and does not take the last write off
    /// the screen, and a lost build worker says nothing about a write that already
    /// happened — `crates/karakuri/src/main.rs`'s `staging`, which is where the one
    /// drain feeds both readings.
    pub health: Option<Stage>,
    /// Whether a session is being recorded — the mock's `● rec` pill at the very
    /// end of this row, after [`Transport::health`] — and `None` for a console
    /// nobody has told anything about recording, which draws no pill at all.
    ///
    /// # `None` is *nobody said*, and it is not *not recording*
    ///
    /// [`View::audio`]'s distinction one row along, and for its reason: a recording
    /// is a file being written by a program that has a store, and `src/` has
    /// neither (ADR-0156). `Some(Rec::Idle)` is a program that holds a store and is
    /// not recording into it, and it draws the pill in the plain treatment because
    /// a press on it would start one; `None` is a console that was never told, and
    /// a pill drawn for it would be this crate answering a question about a disk on
    /// its own authority. Every test in this crate that does not say otherwise
    /// leaves it `None`.
    ///
    /// # It is on [`Transport`] rather than beside it, which [`View::look`] is not
    ///
    /// [`Transport::health`]'s reason, and this is the second field here that is
    /// not a clock: what this type is is *what the transport row reads this frame*,
    /// and the two capsules at the end of the row are read by it. The difference
    /// from the look — which is two fields away on [`View`] because the row draws
    /// it and the master chain owns it — is that neither of these two capsules is
    /// drawn anywhere else and neither belongs to another bay.
    pub rec: Option<Rec>,
}

impl Transport {
    /// How many dots the beat grid has: [`Transport::beats_per_bar`], and at least
    /// one.
    ///
    /// The clamp is here and in one place because a bar of no beats is two failures
    /// at once — a grid with nothing in it, and a division by zero in
    /// [`Transport::beat`] that comes out `NaN` and lights no dot in a row that has
    /// none. One beat to the bar is the nearest thing to a grid that can be drawn,
    /// and every reader of the field goes through here.
    pub fn dots(&self) -> u32 {
        self.beats_per_bar.max(1)
    }

    /// Where the beat is inside the current bar, from zero, and continuous: `0.0`
    /// is on the first dot, `1.5` is halfway between the second and the third, and
    /// `3.75` is three quarters of the way from the last dot back round to the
    /// first.
    ///
    /// A position and not an index, which is the whole of what the grid draws now
    /// ([ADR-0212](../../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)):
    /// the light is somewhere on the grid at every instant rather than on one dot
    /// at a time, so what the row reads is the fractional beat the oscillator
    /// already accumulates rather than a truncation of it. The fraction was always
    /// in [`Transport::beats`] and was thrown away here.
    ///
    /// It is the whole of the beat's motion, and it is not a clock. No second value
    /// is written for the grid and none is derived from [`Phase`]: the beat moves
    /// because the session moves, which is what a beat grid is for, and a
    /// wall-clock sweep would go on sweeping at its own rate over a session running
    /// at another one — see [`beat_at`].
    ///
    /// `rem_euclid` rather than `%` because [`Transport::beats`] can be negative —
    /// `Oscillator::behind` reads the same grid at an earlier time, and a slot
    /// warming behind the session is exactly what that is for — and `%` on a
    /// negative is negative, which is a position no grid has.
    ///
    /// The clamp this used to need went with the index. `(-1e-18_f64)
    /// .rem_euclid(4.0)` is `4.0` exactly: the true remainder is a hair under the
    /// divisor and rounds up to it. As a dot *index* that was one past the end of
    /// the grid — a rectangle drawn beside it, or a panic — and it was clamped. As
    /// a *position* it is the downbeat: `4.0` and `0.0` are the same point on a
    /// cycle of four, [`beat_at`] measures round the cycle, and the value arrives
    /// where it belongs with nothing written for it.
    pub fn position(&self) -> f32 {
        self.beats.rem_euclid(self.dots() as f64) as f32
    }

    /// Which bar it is, counting from one — the `37` in the mock's `bar 37`.
    ///
    /// One-based because that is how a bar is counted out loud, and there is no bar
    /// 0 in anything an operator says. Signed for the reason [`Transport::beat`]
    /// takes `rem_euclid`: a position before the session's zero is a bar before the
    /// first one, and `as u32` on it would saturate to zero and draw `bar 1` for
    /// every one of them.
    pub fn bar(&self) -> i64 {
        self.beats.div_euclid(self.dots() as f64) as i64 + 1
    }
}

/// The transport row, laid out: where each of the five readouts goes, and
/// which beat is lit.
///
/// # One derivation, for the reason [`Outputs`] is one
///
/// [`View::draw`] paints exactly these rectangles. Nothing hit-tests *these*
/// four, because none of them is a control — see below — but this now has
/// three call sites rather than the one it was written with: the frame paints
/// it, and [`arrangement`] asks it where the bar ended and where the frame
/// readout begins, from both the frame and [`crate::input::claim`]. A value
/// rather than a paint-as-you-go pass is what made that possible, and it was
/// already the right shape for the reason it was written: it makes the
/// arithmetic something `tests/transport.rs` can ask about without a device,
/// which is the whole of how this crate is checked.
///
/// # Four of the six are readouts, and two are controls
///
/// A point in this row that is not inside a boundary's [`GRAB`] and not on one
/// of this row's controls is `egui`'s. A beat, a bar, a frame time and what
/// the last write did are four readouts, and a readout is not something a
/// press acts on. The last of them is drawn as a capsule and is still one:
/// the shape is the mock's, and what makes a thing a control here is that a
/// press on it asks for something. `tests/transport.rs` asserts it over the
/// row rather than leaving it to be inferred from the absence of a hit test,
/// and it asks with the pill drawn so that the assertion cannot pass on the
/// control having gone.
///
/// The tempo is not one of the four, and it left them when the figure
/// became the track: the number is still a reading, and a press on it names a
/// value outright — [`TransportRow::tempo`], and ADR-0291. A readout that a
/// press acts on is a control, which is the same test the capsule fails.
///
/// The other control is [`TransportRow::rec`], and it is drawn *inside* this
/// derivation where [`arrangement`] is drawn beside it. The difference is
/// where each one sits: the arrangement pill is laid out from
/// [`TransportRow::bar`] and held clear of [`TransportRow::frame`], so a row
/// that never heard of it is laid out exactly as it is now; the `rec` pill
/// takes the row's right padding, and the health capsule and the frame readout
/// are laid out backwards from it. A pill drawn beside the row could not move
/// them, and two derivations of one right-hand end are two answers.
///
/// This paragraph said *nothing here is a control*, then *none of these five
/// is*, then *the sixth is this row's one control*, and every one of those
/// changes is deliberate. What a press on the capsule asks for is
/// [`TransportRow::record`], and what a press on the number asks for is
/// [`TransportRow::tempo`].
///
/// # What is in the mock's row and is deliberately not here
///
/// The list is empty as of 2026-09-10, and that is a state rather than a
/// deletion. It held `learn` and `map · nanoKONTROL2 ▾` — *"each a control
/// over machinery that is in neither this crate nor the program"* — and both
/// are drawn now:
///
/// - `learn` is [`learn_pill`], and the machinery arrived with
///   `docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md`:
///   the panel opens a port at start-up, and a press on this arms the gesture
///   the mock's tooltip describes.
/// - `map · <name>` is [`map_pill`], and it is a readout. What was written
///   against it was that *"a menu naming a device nobody has plugged in is
///   worse than no menu"*, and that half is unchanged — reaching a map while
///   running is not built, so the pill has no chevron and no menu and says
///   which file is loaded. A capsule that looked like a menu and opened none
///   would still be the scaffolding this module refuses; one that states a
///   true thing is not.
///
/// The rule that emptied it is the one that kept them out: a control is
/// drawn when there is something behind it. [`arrangement`] was the first item
/// in this row to pass that test and [`audio_in`] the second; these are the
/// third and fourth.
///
/// `● rec` left this list on 2026-09-08, and what was written against it
/// was that *"there is no session recorder behind this panel and no record
/// stream is written from it"*. There is one now: `crates/karakuri` opens a
/// `karakuri_environment::session::Recorder` on a press and closes it on the
/// next, and this pill is the toggle — [`TransportRow::record`], drawn in the
/// mock's `.pill.on` while a recording runs. The mock's tip named one gesture
/// on it, *click to stop*, and the control is both: what a press means is what
/// [`Transport::rec`] says it will be, before it is made.
///
/// This paragraph carried a total of the row and it no longer does, which
/// is a deletion rather than an oversight. It said *"the mock draws ten things
/// and five of them exist"*, and the ten was a number nobody could check
/// against the markup: `.transport` has fourteen children and one of them —
/// `.tracker` — holds four more. A list of what is missing is checkable one
/// item at a time; a total of what a row holds is a second count of the mock,
/// and it had already gone stale once, when `audio-in` left this list.
///
/// `landed` left it on 2026-09-08 and it is the one item here that never
/// was a control. What was written against it was that *"nothing writes a
/// procedure while this panel runs, so the pill would be reporting on a write
/// that never happens"*, and that was already false when it was read again:
/// `crates/karakuri` watches every slot's sources, so a save from any editor
/// builds, swaps and is judged, and every verdict of that is drawn in the
/// Staging lane. This row is where the same stream says *what the last write
/// did* — [`Transport::health`], which is a value the harness hands in and not
/// a control this crate offers.
///
/// Three items left it on 2026-09-08 — `tap`, `offset` and the octave's
/// `½ ×2` — which is M5.4's own work: [`tracker_group`] draws them, and what
/// was written here about the first of them was that *"nothing times a tap
/// here, and the pill would correct nothing"*. Something does now, and it
/// always did on the keyboard: `karakuri_environment::audio::Audio::tap` is
/// what `b` has been reaching, and the pill reaches it by emitting the same
/// operation. `audio-in` was the first to leave, on the same terms.
///
/// The `.sep` between the bar and the frame readout is drawn, in the only
/// way a `flex: 1` spacer can be: it is space, so what it does is push the
/// frame readout to the right edge, and that is where [`transport_row`] puts
/// it.
///
/// # No tooltips, for the reason the Outputs row has none
///
/// Most of the mock's items carry a `data-tip` and so do most of the drawn
/// ones — the beat grid's is what the travelling light means, and it landed
/// with the light
/// ([ADR-0212](../../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)). A tooltip needs `egui` to own a widget, this console paints, and
/// giving one readout a widget is a decision about who owns the pointer — see
/// [`outputs`], where the same sentence is written about a control.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. `ctx` is asked for the type, because where each readout ends is where
/// the next one starts.
pub fn transport(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
) -> Option<TransportRow> {
    // **No engine behind the console, so there is no row.** Every test in this
    // crate is here, and so is the whole of `cargo test -p karakuri-console`.
    // Drawing a row of zeroes would be inventing a tempo nothing is running
    // at; this is `View::picture`'s rule, one row along.
    let values = values?;
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // `outputs`. Nothing routes a pointer here, so this is only the frame
    // before the first one — and there is nothing to draw on it either.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let row = to_egui(layout.rect(layout.find("transport")?));
    let width = |job: LayoutJob| ctx.fonts_mut(|f| f.layout_job(job).size().x);
    let bpm = width(bpm_job(&values, Color32::PLACEHOLDER, Color32::PLACEHOLDER));
    let label = width(span(BPM_LABEL, Color32::PLACEHOLDER));
    let bar = width(span(&bar_text(&values), Color32::PLACEHOLDER));
    let frame = width(frame_job(
        &values,
        Color32::PLACEHOLDER,
        Color32::PLACEHOLDER,
    ));
    // **The health capsule, measured only where there is a verdict to draw.**
    // `.pill`'s `padding: 0 8px` around one word — there is no chevron here
    // and no second span, because nothing about this capsule opens and it
    // says one thing.
    let health = values
        .health
        .map(|stage| size::PILL_PAD_X * 2.0 + width(span(stage.word(), Color32::PLACEHOLDER)));
    // **The `rec` pill, measured only where somebody has said whether a
    // recording is running**, and the same width either way it is: the mark
    // and the word do not change between the two states — only the treatment
    // does — so this is one number rather than the wider of two, and the pill
    // does not move under a press that lands on it.
    let rec = values.rec.map(|_| {
        size::PILL_PAD_X * 2.0
            + size::SINK_DOT
            + size::SINK_GAP
            + width(span(REC_LABEL, Color32::PLACEHOLDER))
    });
    transport_row(row, &values, bpm, label, bar, frame, health, rec)
}

/// The word in the `rec` pill, and the mock's `&#9679; rec` without its mark:
/// the mark is drawn rather than typed, which is [`Mask`]'s rule for the same
/// reason — *"whether `◯` and `◑` are in `egui`'s default face is a question
/// with no good answer, and a circle is the same mark either way"*.
const REC_LABEL: &str = "rec";

/// The faint word beside the number, and the mock's own capitalisation this
/// time: `.transport`'s `BPM` is upper-case in the markup rather than in CSS,
/// so it is upper-case here.
const BPM_LABEL: &str = "BPM";

/// The transport row's furniture: a rectangle for each readout, and which beat
/// is lit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransportRow {
    /// The tempo, in `.bpm`'s 20px box.
    pub bpm: Rect,
    /// The faint `BPM` beside it.
    pub label: Rect,
    /// The whole `.beat-grid`: [`TransportRow::dots`] dots and the gaps between
    /// them. One dot is [`TransportRow::dot`].
    pub grid: Rect,
    /// How many dots are in that grid — [`Transport::dots`], carried so the painter
    /// and a test read the same number the width was built from.
    pub dots: u32,
    /// Where the light is, in beats from the first dot's centre —
    /// [`Transport::position`], carried for the reason [`TransportRow::dots`] is
    /// carried: the painter and a test read the one number the row was measured
    /// from. How much of it lands on any given dot is [`TransportRow::lit`].
    pub at: f32,
    /// `bar 37`.
    pub bar: Rect,
    /// The frame readout, pushed to the right by the `.sep` — against the row's
    /// right padding where there is no verdict to draw, and one
    /// [`size::TRANSPORT_GAP`] before [`TransportRow::health`] where there is.
    pub frame: Rect,
    /// The health capsule, and `None` where [`Transport::health`] is — which is
    /// every run until somebody rewrites a procedure.
    ///
    /// The `rec` pill is what follows it, so the right padding is
    /// [`TransportRow::rec`]'s where there is one and this capsule's where there is
    /// not — and everything before it is laid out backwards from whichever ends the
    /// row. No gap is left for a pill that is not drawn.
    pub health: Option<Rect>,
    /// The `● rec` pill, and `None` where [`Transport::rec`] is — which is a
    /// console nobody has told anything about recording.
    ///
    /// It is the last thing in the row and takes the right padding, which is where
    /// the mock puts it: after `landed`, hard against the end of `.transport`. It
    /// is the one thing in this row that is a control — a press on it starts a
    /// recording or stops the one running — and it is carried here rather than
    /// derived beside the row for [`TransportRow::health`]'s reason one item along:
    /// where each readout ends is where the next one starts, so the pill's place
    /// and the readouts' places are one derivation and not two that agree until
    /// they do not.
    ///
    /// It said it was the one control in this row and it is one of two: the tempo
    /// figure is the other ([`TransportRow::tempo`]), landed in the same commit and
    /// drawn at the row's other end.
    pub rec: Option<Rect>,
    /// The values these rectangles were measured from.
    ///
    /// Carried rather than passed to the painter beside this, for [`Picture`]'s own
    /// reason: whoever measured the type and whoever paints it are then one
    /// statement, so a row laid out for one tempo and painted with another cannot
    /// be written by accident. It is also what leaves one place where *no engine
    /// means nothing at all* is decided — this answering `None` — rather than that
    /// rule being asked once here and again in [`View::draw`], which is two answers
    /// that agree until they do not.
    pub values: Transport,
}

/// How far a press may move the tempo: ±15% of what the grid is running at when
/// it lands.
///
/// # It is a guard against a mis-click, and not a bound on what a tempo may be
///
/// Nothing in this workspace says a tempo may not be 240 —
/// `karakuri_audio::tempo::BPM_RANGE` says outright that it is *not* the range
/// of answers, and *"a grid at 240 bpm is a perfectly good grid"* — and this
/// band does not say it either: it bounds one press, against the tempo of the
/// moment, so a hand walks the grid anywhere it likes — 240 is five presses up
/// from the tempo the mock draws, and the figure names it outright on the last
/// of them.
///
/// The `½ ×2` beside the figure is not the shortcut it looks like, and at this
/// row's own tempo it is not available at all: [`Tracker::double`] is
/// `karakuri_audio`'s `BPM_RANGE` against twice the grid, 256 is outside it,
/// and the mock draws that half inert. So the figure is the whole of the way up
/// from 128, and `docs/manual/console.html`'s *"get there in two presses"* is
/// not arithmetic that works from there — reported rather than quietly
/// satisfied by a wider band than the one that was decided.
///
/// # A press outside it is ignored, and *ignored* is the decision
///
/// Not clamped, which is the one thing a track this crate has drawn before does
/// do — [`unit_of_offset`] draws a value past either end *at* that end, because
/// the offset's ends are the range of the value. These are not ends of
/// anything: they are how far a hand is trusted to have meant it. Clamping
/// would turn a press the operator did not mean into a 15% move of the grid,
/// which is the loudest thing this row can do; ignoring it leaves the tempo
/// where it was, and a tempo that did not move is a press the operator can see
/// did not land
/// ([ADR-0291](../../../../docs/adr/0291-the-tempo-figure-is-the-track-and-the-band-is-a-guard-on-the-hand.md)).
///
/// # It is here and nowhere else
///
/// It is not in `karakuri-audio`, and must not be. The tracker folds every
/// candidate into a window centred on the grid and the beat lock re-acquires on
/// evidence; a ±15% bound anywhere in that path is a grid that cannot follow a
/// song, which is a far worse failure than a hand that can ask for anything. So
/// the band lives at the one point a *press* becomes an operation, which is
/// this type — the same seam [`TrackerGroup::half`] draws an inert octave chip
/// at, one group along.
pub const TEMPO_BAND: f32 = 0.15;

/// What the whole figure spans, as the same fraction of the same tempo: two
/// bands either way, so [`TEMPO_BAND`] is the middle half of the number and the
/// outer half is the guard.
///
/// The figure has to reach past the band or the band could not be missed, and a
/// guard nothing can land in is not a guard — it is an assertion that the
/// surface already satisfies, and `tests/tempo_figure.rs` could only pass it by
/// construction. So the question is not whether the number reaches further than
/// a press may go, but by how much, and the answer is *one more band*: a press
/// that misses by up to as much again as it is allowed to move is refused, and
/// a press that misses by more than that is off the figure and was never this
/// control's.
///
/// Linear, and that is the offset track's arithmetic rather than the
/// exposure's. [`offset_at`] is linear because a latency offset is a
/// *difference*, and [`exposure_at`] is logarithmic because an exposure is a
/// *ratio* — a tempo is the second kind of quantity, which is why the control
/// beside this one is an octave and why this band is a percentage. But the band
/// is stated as ±15% of the tempo at the press, and at the instant of a press
/// that is a fixed number of beats a minute: the whole geometry is settled by
/// the one number the row was measured with, and equal distances along the
/// figure are equal numbers of beats a minute. Over a span this narrow the two
/// curves are within a percent of each other in any case, and the linear one is
/// the one whose middle is exactly the tempo drawn there.
pub const TEMPO_SPAN: f32 = TEMPO_BAND * 2.0;

impl TransportRow {
    /// One dot of the beat grid, from the left. The gaps are between the dots and
    /// nowhere else, which is what a flex row with a `gap` is — the same reading
    /// [`preview_cells`] takes of a grid.
    ///
    /// Panics on a dot this grid has not got, which is a caller having invented a
    /// beat: [`TransportRow::on`] is [`Transport::beat`] and that is inside the
    /// grid by construction.
    pub fn dot(&self, index: u32) -> Rect {
        assert!(index < self.dots, "dot {index} of a grid of {}", self.dots);
        Rect::from_min_size(
            Pos2::new(self.grid.min.x + BEAT_PITCH * index as f32, self.grid.min.y),
            egui::vec2(size::BEAT_W, size::BEAT_H),
        )
    }

    /// How much of the light is on the dot at `index` this frame, from `0.0` to
    /// `1.0` — [`beat_at`] at this row's position, so the painter and a test ask
    /// one question and get one answer.
    ///
    /// Panics on a dot this grid has not got, for [`TransportRow::dot`]'s reason.
    pub fn lit(&self, index: u32) -> f32 {
        assert!(index < self.dots, "dot {index} of a grid of {}", self.dots);
        beat_at(self.at, index, self.dots)
    }

    /// How far along the figure `p` is, on `[0, 1]` across [`TransportRow::bpm`]
    /// and outside it either side.
    ///
    /// Not clamped, unlike [`unit_of_offset`]'s: what is off the number is not at
    /// its end, it is not this control's, and every caller here has asked
    /// [`Rect::contains`] first.
    fn along(&self, p: karakuri_layout::Point) -> f32 {
        (p.x - self.bpm.min.x) / self.bpm.width()
    }

    /// What a press `unit` of the way along the figure names, in beats a minute —
    /// [`offset_at`]'s job on the one control whose track is the reading itself.
    ///
    /// The middle of the number is the number: at `0.5` this is [`Transport::bpm`]
    /// exactly, which is what `docs/manual/console.html` means by *"the number
    /// under your finger is the one you get"* — the figure is drawn at the tempo it
    /// names, so pressing where it is drawn asks for what is already there. The
    /// ends are [`TEMPO_SPAN`] either way, and the half of the figure between the
    /// quarters is [`TEMPO_BAND`].
    ///
    /// It is a function of the tempo the row was measured at, so the whole control
    /// moves with the grid: at 128 a press at the right-hand end asks for 166.4,
    /// and at 256 the same pixel asks for 332.8. There is no track with ends in it
    /// anywhere here, which is why there is no constant naming them — the offset's
    /// [`LATENCY_OFFSET_MIN_MS`] is a restatement of a range something else holds,
    /// and nothing holds one for this.
    pub fn tempo_at(&self, unit: f32) -> f32 {
        self.values.bpm * (1.0 + (unit - 0.5) * 2.0 * TEMPO_SPAN)
    }

    /// Whether a tempo is one a press may ask for: within [`TEMPO_BAND`] of what
    /// the grid is running at, measured against the tempo this row was derived
    /// with.
    ///
    /// Which is the tempo at the press, and not one from an earlier frame:
    /// `karakuri/src/main.rs` derives this row again on the pointer event, off the
    /// same [`View::transport`] the frame before it was painted from — the honest
    /// cost [`tracker_group`] states for the group beside this one, paid here for
    /// the same reason.
    pub fn in_band(&self, bpm: f32) -> bool {
        // **A number that is not a tempo is not one a press may ask for**, and
        // the case it guards is a band of nothing: a row drawn at `0.0` has a
        // band `0.0` wide, and every point on the figure would name exactly
        // zero and pass a comparison written with `<=`. Nothing in this
        // workspace runs a grid at zero — `karakuri_signal`'s own clamp floors
        // it at 1.0 — but this row is drawn from whatever the harness hands
        // in, and a control that emits an operation naming a tempo no
        // oscillator will take is a press that does nothing (P-0094).
        bpm.is_finite()
            && bpm > 0.0
            && (bpm - self.values.bpm).abs() <= self.values.bpm.abs() * TEMPO_BAND
    }

    /// Whether `p` is on the tempo figure asking for something it may have — on the
    /// number, and inside [`TransportRow::in_band`].
    ///
    /// The band is part of the hit test and not only of the answer, which is
    /// [`TrackerGroup::half`]'s rule word for word: *inert is not claimed*, and
    /// this crate's own *a control claims what it acts on and no more*. So a press
    /// on the guard is `egui`'s — it falls through exactly as a press on the
    /// refused half of the octave does, rather than being swallowed by a control
    /// that then does nothing, which is the one outcome an operator cannot tell
    /// from a panel that has stopped
    /// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    ///
    /// The number's own box is the target and nothing is grown, unlike
    /// [`OffsetTrack::grip`]: `.bpm` is 20px type at `line-height: 1.5`, so this is
    /// 30 tall against that track's 5, and it leaves 9 of the row's 48 above and
    /// below — clear of the [`GRAB`] of the boundary under the row, which is 6.
    pub fn on_tempo(&self, p: karakuri_layout::Point) -> bool {
        self.bpm.contains(Pos2::new(p.x, p.y)) && self.in_band(self.tempo_at(self.along(p)))
    }

    /// What a press at `p` asks the grid to run at, or `None` off the figure and on
    /// the guard around it.
    ///
    /// # The press names the tempo outright, and that is the decision
    ///
    /// [`Operation::SetFreeRunTempo`] carries the number, and the number is where
    /// along the figure the press landed. `docs/manual/console.html`: *"A press
    /// names a value outright rather than stepping, so the figure is the track and
    /// the number under your finger is the one you get."* It is [`OffsetTrack`]'s
    /// sentence one control to the left
    /// ([ADR-0277](../../../../docs/adr/0277-the-latency-offset-is-a-track-because-a-capsule-cannot-name-a-value.md)),
    /// and it arrives here without that record's derivation: this row has no key,
    /// so there is no press to make a pixel out of, and what decides the figure's
    /// scale is the figure's own width and [`TEMPO_BAND`].
    ///
    /// # It is emitted with a room being tracked, and that is not this crate's call
    ///
    /// The operation is *what the grid runs at with nothing driving it*, and the
    /// state it is for is a program with no audio device — which is the state
    /// `crates/karakuri` runs in, where `tap` and the octave both refuse out loud
    /// because there is no room. With a room open it is accepted rather than
    /// refused: the tracker searches a window centred on the grid, so moving the
    /// grid moves the window and the estimate is made again around the new target —
    /// `karakuri_audio`'s `BeatLock::retarget`, which is where that is written down
    /// and is the reason this is not a control that undoes itself two seconds
    /// later. Nothing here can see whether a room is being tracked in any case:
    /// this crate takes no device (ADR-0156), and [`Tracker`] carries what the
    /// panel *draws* rather than what a press is allowed to be.
    pub fn tempo(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.on_tempo(p).then(|| Operation::SetFreeRunTempo {
            bpm: self.tempo_at(self.along(p)),
        })
    }

    /// Whether `p` is on the `rec` pill, which is one of the two controls this
    /// derivation owns: the tempo figure is the other ([`TransportRow::on_tempo`]),
    /// and the four things beside them — the beat grid, the bar, the frame readout
    /// and the health capsule — are readouts, which are not something a press acts
    /// on.
    ///
    /// `false` where the pill is not drawn — a console nobody has told anything
    /// about recording claims nothing.
    pub fn on_rec(&self, p: karakuri_layout::Point) -> bool {
        self.rec
            .is_some_and(|pill| pill.contains(Pos2::new(p.x, p.y)))
    }

    /// What a press at `p` on the `rec` pill asks for, or `None` off it.
    ///
    /// It is a toggle and the two ends are two payloads, which is what
    /// [`karakuri_operation::Recording`] is: a press with nothing running asks to
    /// begin one, a press with something running asks to end it, and the pill's own
    /// treatment is what says which the next press will be before it is made. So
    /// one capsule is one control and not two, and the state it reads is the state
    /// the press acts on — one reading, so the pill that is drawn and the press
    /// that is performed cannot come apart.
    ///
    /// `id` is `None`, and the payload saying so is what makes each start a fresh
    /// recording. A capsule types no name — this console's one letter-taking flow
    /// is bounded to naming an arrangement — so whoever performs it files under a
    /// stamp, exactly as the `keep` capsule's save does. That is not only a
    /// convention here: a second head under one id would land in the middle of an
    /// existing stream and be read back as edits, so a start that could name the id
    /// a stop just closed would be a press that corrupts a session (ADR-0289).
    ///
    /// It refuses nothing. Whether there is a deck to write a head from and whether
    /// the store will take the file are the instrument's answers rather than this
    /// surface's — this crate reaches no disk at all (ADR-0156) — so the press
    /// names the operation and whoever performs it says what happened.
    pub fn record(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let rec = self.values.rec?;
        self.on_rec(p).then_some(Operation::RecordSession {
            recording: match rec {
                Rec::Idle => Recording::Start { id: None },
                Rec::Running => Recording::Stop,
            },
        })
    }
}

/// From one dot to the next: [`size::BEAT_W`] and the [`size::BEAT_GAP`] after
/// it, which is what one beat of travel measures on this grid.
///
/// Derived rather than transcribed: the mock declares an item width and a flex
/// `gap`, and the pitch of a flex row is the one plus the other. It is the unit
/// [`beat_at`] measures in and the unit [`TransportRow::dot`] steps by, which
/// is why it is a constant rather than the same sum written twice.
pub const BEAT_PITCH: f32 = size::BEAT_W + size::BEAT_GAP;

/// How many steps one beat of travel is drawn in: the pixels in one
/// [`BEAT_PITCH`], so the light moves by at most one of them between updates.
///
/// [`mixer::ROLL_STEPS`]'s shape and a different answer, because the two
/// motions are different sizes: the roll travels a few pixels of a word's pitch
/// and twelve steps is smooth over it, where the light crosses a whole dot and
/// a gap every beat.
const BEAT_STEPS: u64 = BEAT_PITCH as u64;

/// How stale the beat grid may get, which is what the transport row declares
/// under
/// [P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)
/// and what the harness turns into a deadline.
///
/// [`BEAT_MICROS`] in [`BEAT_STEPS`] steps — 24.67 ms, about forty a second.
/// One beat of travel is one [`BEAT_PITCH`], so this is the light moving by one
/// pixel and no more, which is the coarsest step that reads as a movement
/// rather than as a sequence of positions
/// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
///
/// Stated at the mock's tempo, and it is the one number here that the music
/// moves. A beat is 468.75 ms at 128.0 BPM and 375 ms at 160, so the same
/// declaration is a pixel and a quarter a step up there. The alternative —
/// derive it per frame from [`Transport::bpm`], which the row is handed — is
/// [ADR-0212](../../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md),
/// and it lost on what it does to the arithmetic rather than on the drawing: a
/// staleness that falls with the tempo makes `Σ (cost / staleness)` a function
/// of how fast the music is, so the two schedulability conditions could only be
/// asserted against a fastest tempo nobody has written down — and inventing one
/// inside the test that noticed it was missing is exactly what
/// [ADR-0210](../../../../docs/adr/0210-a-declared-cost-is-one-panel-pass-written-down-and-held-against-the-run.md)
/// refused for the panel's share of the budget.
///
/// Finer than [`ROLL_STALENESS`]'s 33.33 ms and coarser than a frame, and both
/// are the presentation rather than a preference for frames: the roll travels a
/// few pixels and this crosses the grid, so it wants more steps; and a 60 Hz
/// frame is 16.6 ms, so a beat grid that asked for every frame would be asking
/// for more than its own drawing can use.
pub const BEAT_STALENESS: Duration = Duration::from_micros(BEAT_MICROS / BEAT_STEPS);

/// How much of the light is on the dot at `index`, with the light `at` beats
/// into a bar of `dots`: `1.0` under its centre, `0.0` a whole [`BEAT_PITCH`]
/// away, and a raised cosine between the two.
///
/// # The same curve as [`roll_at`], and two of the reasons are the same
///
/// It leaves and arrives at zero with zero velocity, so a dot does not snap
/// into being dark as the light leaves it. And it is a pure function of a value
/// the harness handed in, so a test asserts it at a position it chose and
/// nothing samples a clock to find out what the panel is doing.
///
/// # Two dots at once, and the row's total light is constant
///
/// The falloff is exactly one pitch wide, so at most two dots are lit and `f(d)
/// + f(1 - d) = 1` for every `d` — the raised cosine's own identity, and
/// therefore the grid's dots always sum to exactly one dot's worth of light
/// (every grid but the degenerate one below, which is one dot and holds all of
/// it). The light moves along the grid rather than the grid brightening and
/// dimming as it goes, which is what makes a stop visible: a still grid at half
/// brightness would be indistinguishable from a light sat between two dots.
///
/// # Measured round the cycle, not along the row
///
/// The bar wraps, so the distance from the last dot to the first is one pitch
/// and not three: the light leaves the right-hand end of the grid and arrives
/// at the left-hand end in the same instant, each dot half lit, and there is no
/// frame on which it jumps. That is also why [`Transport::position`] needs no
/// clamp — a position of exactly `dots` is a distance of zero from the first
/// dot.
///
/// # What it draws at the instant of a beat is the mock
///
/// At a whole `at` the dot under the light is `1.0` and every other is exactly
/// `0.0`: one dot in `--c-pink` with its halo and the rest in `--c-line`, which
/// is `.beat-grid i.on` and the four dots the mock's markup draws. The mock is
/// a frame of this rather than a picture this contradicts, and
/// `docs/manual/style.css` carries the travel between those frames.
pub fn beat_at(at: f32, index: u32, dots: u32) -> f32 {
    let dots = dots.max(1) as f32;
    // **A grid of one dot has nowhere for the light to go**, so it is on that
    // dot at every position. Measuring round a cycle one pitch long would say
    // *half lit* halfway through the beat and leave the row dimming with
    // nothing to dim toward — the identity above wants two dots to share the
    // light between. This is [`Transport::dots`]' own clamp seen from here: a
    // bar of no beats is drawn as the nearest thing to a grid there is.
    if dots < 2.0 {
        return 1.0;
    }
    let round = (index as f32 - at).rem_euclid(dots);
    let away = round.min(dots - round);
    match away < 1.0 {
        true => 0.5 * (1.0 + (away * std::f32::consts::PI).cos()),
        false => 0.0,
    }
}

/// The arithmetic of the row, away from the type it measures and the layout it
/// reads.
///
/// Term for term from `.transport` and what is in it, in `style.css`:
///
/// - `.transport { display: flex; align-items: center; gap: 14px;
///   padding: 9px 12px }` — the readouts laid left to right from
///   [`size::TRANSPORT_PAD_X`], one [`size::TRANSPORT_GAP`] between each pair,
///   and every one of them centred in the row rather than sat on its
///   padding. The four boxes are four different heights — 30 for the number,
///   16.5 for a line of type, 6 for the beat grid — and `align-items: center`
///   is what puts them on one line through the middle.
/// - `.sep { flex: 1 }` — a spacer that takes everything left over, so what
///   comes after it is against the row's right padding. The frame readout is
///   laid out from the right edge backwards for that reason, and it is the one
///   thing in the row whose position is not the position of the thing before
///   it.
///
/// # The 48 comes back here
///
/// The row is 48 and the number in it is 30 (`.bpm`'s 20px at
/// `line-height: 1.5`), so there is (48 - 30) / 2 = 9 of row above and
/// below it — which is `.transport`'s own `padding: 9px`, arrived at from the
/// other end. The arrangement's `9 + 30 + 9` and the centring agree exactly
/// here, where the Outputs row's 8 + 18.5 + 8 had to give up a quarter pixel.
///
/// `None` where the row cannot hold what goes in it: folded away, soloed away,
/// or too narrow to keep the frame readout clear of the bar. The mock wraps
/// and this does not, which is the argument [`outputs_row`] makes about the
/// second sink, in the one case where the mock's `flex-wrap: wrap` has
/// something to wrap: a row too narrow draws nothing rather than a second line
/// of transport in a bay 48 tall that has no room for one.
// Eight, where clippy's line is seven. Six of them are one measured width
// apiece — the tempo figure, its label, the bar, the frame readout, the health
// capsule and the `rec` pill — and measuring is the one thing this function
// exists not to do: [`transport`] holds the `egui::Context`, asks the fonts for
// each width, and hands them here so that the arithmetic can be read, and
// tested, without one. The other two are the rectangle it lays out inside and
// the values it lays out for. A struct to carry the six would be `TransportRow`
// again with a width where each `Rect` is, built one field at a time by the
// caller and consumed once here — which is these arguments with a name on them
// and a second type to keep in step with the first.
#[allow(clippy::too_many_arguments)]
fn transport_row(
    row: Rect,
    t: &Transport,
    bpm_w: f32,
    label_w: f32,
    bar_w: f32,
    frame_w: f32,
    health_w: Option<f32>,
    rec_w: Option<f32>,
) -> Option<TransportRow> {
    let mid = row.center().y;
    // An inline span at the console's own type: `font-size: 11px` at
    // `line-height: 1.5`, which is the box every line of ordinary text in the
    // mock sits in.
    let span_h = size::BASE * size::LINE;
    let bpm = Rect::from_min_size(
        Pos2::new(row.min.x + size::TRANSPORT_PAD_X, mid - size::BPM_H * 0.5),
        egui::vec2(bpm_w, size::BPM_H),
    );
    let label = Rect::from_min_size(
        Pos2::new(bpm.max.x + size::TRANSPORT_GAP, mid - span_h * 0.5),
        egui::vec2(label_w, span_h),
    );
    let dots = t.dots();
    let grid = Rect::from_min_size(
        Pos2::new(label.max.x + size::TRANSPORT_GAP, mid - size::BEAT_H * 0.5),
        egui::vec2(
            size::BEAT_W * dots as f32 + size::BEAT_GAP * (dots - 1) as f32,
            size::BEAT_H,
        ),
    );
    let bar = Rect::from_min_size(
        Pos2::new(grid.max.x + size::TRANSPORT_GAP, mid - span_h * 0.5),
        egui::vec2(bar_w, span_h),
    );
    // The `.sep`: everything after it is against the right padding. **The
    // last of those is the `rec` pill where there is one and the health
    // capsule where there is not**, so the right padding is claimed by
    // whichever of the three ends the row, and everything before it is laid
    // out backwards from there — which is the same "from the right edge
    // backwards" the frame readout has always been laid out by, asked of the
    // thing beside it rather than of the row.
    //
    // **The pill is last because the mock puts it last**, after `landed`; and
    // nothing is reserved for one that is not drawn, which is the rule this
    // end of the row has always followed.
    let right = row.max.x - size::TRANSPORT_PAD_X;
    let rec = rec_w.map(|w| {
        Rect::from_min_size(
            Pos2::new(right - w, mid - size::PILL_H * 0.5),
            egui::vec2(w, size::PILL_H),
        )
    });
    let health_end = match rec {
        Some(pill) => pill.min.x - size::TRANSPORT_GAP,
        None => right,
    };
    let health = health_w.map(|w| {
        Rect::from_min_size(
            Pos2::new(health_end - w, mid - size::PILL_H * 0.5),
            egui::vec2(w, size::PILL_H),
        )
    });
    let frame_end = match health {
        Some(pill) => pill.min.x - size::TRANSPORT_GAP,
        None => health_end,
    };
    let frame = Rect::from_min_size(
        Pos2::new(frame_end - frame_w, mid - span_h * 0.5),
        egui::vec2(frame_w, span_h),
    );
    // The same rule `picture_rect` states, on the two ends of the row: the
    // number is the tallest thing in it and the frame readout is the furthest
    // right, so a row that holds both holds everything between them — and the
    // last clause is the wrap the mock does and this does not.
    //
    // **The capsule is inside the same `and`, and the whole row goes when it
    // does not fit.** It is one of the row's readouts rather than a control
    // drawn over it, so a window too narrow to hold it draws nothing — which
    // is the answer this row already gives, one item along.
    match row.contains_rect(bpm)
        && row.contains_rect(frame)
        && health.is_none_or(|pill| row.contains_rect(pill))
        && rec.is_none_or(|pill| row.contains_rect(pill))
        && frame.min.x >= bar.max.x + size::TRANSPORT_GAP
    {
        true => Some(TransportRow {
            bpm,
            label,
            grid,
            dots,
            at: t.position(),
            bar,
            frame,
            health,
            rec,
            values: *t,
        }),
        false => None,
    }
}

/// The tempo as one laid-out run, so that measuring it and painting it cannot
/// be two different runs of type.
///
/// `.bpm`'s `128.0` is one decimal place, which is also as fine as a tempo is
/// ever named — and the mock's own number, so a transcription that started
/// printing `128` would be visible against it.
///
/// # The gradient is drawn per glyph, which is what the CSS comes to here
///
/// `.bpm` is `background: linear-gradient(94deg, var(--c-mint), var(--c-lav))`
/// with `background-clip: text`: the two colours run left to right across the
/// number, near enough — 94deg is four degrees off horizontal. `epaint` fills a
/// galley with one colour, so the run is split into one format run per glyph
/// and each takes its own point along the ramp. Five glyphs is a coarse ramp
/// and it is the mock's two colours rather than one of them; the alternative is
/// picking an end and losing the other, which is a transcription that drops
/// half of what it read.
fn bpm_job(t: &Transport, from: Color32, to: Color32) -> LayoutJob {
    let text = bpm_text(t);
    let mut job = LayoutJob::default();
    let last = text.chars().count().saturating_sub(1).max(1) as f32;
    for (n, (at, ch)) in text.char_indices().enumerate() {
        job.append(
            &text[at..at + ch.len_utf8()],
            0.0,
            TextFormat {
                font_id: FontId::new(size::BPM_SIZE, FontFamily::Proportional),
                extra_letter_spacing: size::BPM_TRACKING,
                color: mix(from, to, n as f32 / last),
                ..Default::default()
            },
        );
    }
    job
}

/// The tempo, as the mock writes it.
fn bpm_text(t: &Transport) -> String {
    format!("{:.1}", t.bpm)
}

/// The bar, as the mock writes it: `bar 37`.
fn bar_text(t: &Transport) -> String {
    format!("bar {}", t.bar())
}

/// The frame readout as one laid-out run: `58 fps · 12.4/16.6 ms`, with the
/// numbers in `.val` and everything else faint.
///
/// One [`LayoutJob`] rather than four galleys laid end to end, because the mock
/// is one run of text with two colours in it — `.val { color: var(--c-text);
/// font-weight: 500 }` inside a span that is `--c-faint` — and laying it out as
/// one is what keeps the spaces between the parts the type's own rather than a
/// gap this file invented. The weight is not honoured: `egui`'s default
/// proportional face has no bold, which `room` says once for the whole crate.
///
/// Both absences drop their own words and nothing else. No rate drops `58 fps ·
/// ` and leaves `12.4 ms`; no budget drops `/16.6` and leaves `12.4 ms`.
/// Neither draws a `0`, a `—` or a plausible 16.6 that nothing measured.
fn frame_job(t: &Transport, val: Color32, faint: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut push = |text: String, colour: Color32| {
        job.append(
            &text,
            0.0,
            TextFormat {
                font_id: FontId::new(size::BASE, FontFamily::Proportional),
                color: colour,
                ..Default::default()
            },
        );
    };
    if let Some(fps) = t.fps {
        push(format!("{fps:.0}"), val);
        push(" fps · ".to_owned(), faint);
    }
    push(format!("{:.1}", t.frame_ms), val);
    match t.budget_ms {
        Some(budget) => push(format!("/{budget:.1} ms"), faint),
        None => push(" ms".to_owned(), faint),
    }
    // The chain's own term, after the frame's: it is charged against the frame
    // and to no deck (ADR-0349). It drops its own words when there is nothing to
    // say, which is the rule the two readings above follow.
    if let Some(chain) = t.chain_ms.filter(|ms| *ms > 0.0) {
        push(" + ".to_owned(), faint);
        push(format!("{chain:.2}"), val);
        push(" chain".to_owned(), faint);
    }
    job
}

/// A plain span of the console's own type: `font-size: 11px`, one colour.
fn span(text: &str, colour: Color32) -> LayoutJob {
    LayoutJob::simple_singleline(
        text.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        colour,
    )
}

/// Two colours mixed, `t` of the way from the first to the second.
///
/// In gamma space, component by component, because that is where a CSS
/// `linear-gradient` in `srgb` interpolates and this is transcribing one.
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let at = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color32::from_rgb(at(a.r(), b.r()), at(a.g(), b.g()), at(a.b(), b.b()))
}

/// The transport row's contents: the tempo, the beat grid, the bar and the
/// frame readout.
///
/// Where everything goes is [`transport_row`]'s, so this paints and derives
/// nothing. Term for term from `style.css`:
///
/// - `.bpm` — the gradient, which is [`bpm_job`]'s.
/// - the label beside it — `style="color:var(--c-faint)"` in the markup, which
///   is `pal.faint`.
/// - `.beat-grid i` — `background: var(--c-line)`, and `.on` is
///   `var(--c-pink)` with `box-shadow: 0 0 9px var(--c-glowp)`. The halo is an
///   [`egui::epaint::Shadow`] at [`size::BEAT_GLOW`] with a corner radius of
///   half the dot's height, which is the same mechanism the Outputs row's dot
///   uses for its own glow. Those two are the ends of a ramp rather than two
///   states: [`beat_at`] says how much of the light is on each dot, the fill
///   is mixed between the two colours by it and the halo is scaled by it, and
///   the mock's `.on` is what a dot with all of the light on it looks like.
///   The mock's own travel between those frames is `@keyframes beat-sweep`.
/// - `bar 37` — `style="color:var(--c-dim)"`, `pal.dim`, *a label beside a
///   value*.
/// - the frame readout — `.val` over `--c-faint`, which is [`frame_job`]'s.
///
/// Every galley is centred in the box [`transport_row`] gave it, which is
/// `align-items: center` done where the type's real height is known: a
/// 20px face is not 30 tall and an 11px one is not 16.5, so the CSS box and
/// the galley are different heights and the box is what is centred on.
pub(crate) fn transport_into(ui: &Ui, pal: &Palette, row: &TransportRow) {
    let t = &row.values;
    let painter = ui.painter();
    let centred = |rect: Rect, galley: std::sync::Arc<egui::Galley>| {
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            // Every run in these jobs carries its own colour, so the fallback
            // `epaint` substitutes for a `Color32::PLACEHOLDER` is never
            // reached — and where it were, ink is a better answer than none.
            pal.text,
        );
    };

    centred(row.bpm, painter.layout_job(bpm_job(t, pal.mint, pal.lav)));
    centred(row.label, painter.layout_job(span(BPM_LABEL, pal.faint)));

    let radius = CornerRadius::same((size::BEAT_H * 0.5) as u8);
    for index in 0..row.dots {
        let dot = row.dot(index);
        let lit = row.lit(index);
        // **The halo belongs to the light rather than to the dot**, so it
        // fades in and out with it instead of switching. Nothing at all is
        // added for a dot the light has left: `beat_at` is exactly zero a
        // pitch away, so a grid of four is one halo at the instant of a beat
        // and two between two of them, and never four.
        if lit > 0.0 {
            painter.add(
                egui::epaint::Shadow {
                    offset: [0, 0],
                    blur: size::BEAT_GLOW,
                    spread: 0,
                    color: pal.glow_pink.gamma_multiply(lit),
                }
                .as_shape(dot, radius),
            );
        }
        painter.rect_filled(dot, radius, mix(pal.line, pal.pink, lit));
    }

    centred(row.bar, painter.layout_job(span(&bar_text(t), pal.dim)));
    centred(
        row.frame,
        painter.layout_job(frame_job(t, pal.text, pal.faint)),
    );

    // **The health capsule**, in the mock's own `.pill.armed` and only for the
    // verdict the mock draws it on. `armed` is this console's *live in the
    // good sense* — the same treatment the audio-in pill wears with an input
    // open and the wipe shape wears with a shape chosen — so it is the word
    // for a build that is on screen and running, and the wrong one for a
    // build that is not: a stopped slot drawn in the mint that says *this is
    // working* would read as the opposite of what it means. The other two take the plain
    // `.pill`, which is the mock's other treatment and not a third one
    // invented here; a colour of alarm is a decision `console.html` has not
    // taken for this capsule and is not taken for it here.
    if let (Some(rect), Some(stage)) = (row.health, t.health) {
        pill_into(ui, pal, rect, stage.word(), stage == Stage::Landed);
    }

    // **The `rec` pill**, in the mock's own two treatments and in no third
    // one: `.pill` while nothing is being recorded, and `.pill.on` while
    // something is. The pink is the mock's choice and it is the right one —
    // `.pill.on` is *this is the press that does it* and the pink is the pink
    // a tally on air is, which is what a recording running is. `armed` would
    // have said *this setting is chosen*, which is not what a running writer
    // is.
    if let (Some(rect), Some(rec)) = (row.rec, t.rec) {
        rec_into(ui, pal, rect, rec);
    }
}

/// The `● rec` pill: a capsule, a round mark, and the word after it.
///
/// It is not [`pill_into`] because it is not a capsule with a word in it: the
/// mock writes `&#9679; rec`, and the mark is drawn rather than set as a glyph
/// for [`Mask`]'s reason — a font this crate does not choose is not something a
/// control's width should depend on.
///
/// The mark's diameter and the gap after it are the Outputs row's sink's
/// ([`size::SINK_DOT`] and [`size::SINK_GAP`]), because that is this console's
/// other round mark before a word inside a capsule. Two numbers invented here
/// would be a second answer to a question `style.css` has already been read for
/// once.
///
/// The mark takes the pill's own colour, both ways: `.pill.on` is a pink word
/// over a pink wash and `.pill` is a dim word inside a hairline, and a mark in
/// some third ink would be a state this capsule does not have.
fn rec_into(ui: &Ui, pal: &Palette, rect: Rect, rec: Rec) {
    let painter = ui.painter();
    let radius = CornerRadius::same((rect.height() * 0.5) as u8);
    let ink = match rec {
        // `.pill.on`: no border, a `--c-pink` word over a wash of the same,
        // and the `box-shadow: 0 0 10px var(--c-glowp)` that goes with it —
        // `on_pill_at`'s three lines, drawn here because the word inside is
        // not the whole of what this capsule holds.
        Rec::Running => {
            painter.add(
                egui::epaint::Shadow {
                    offset: [0, 0],
                    blur: ON_GLOW,
                    spread: 0,
                    color: pal.glow_pink,
                }
                .as_shape(rect, radius),
            );
            painter.rect_filled(rect, radius, tint(pal.pink, ON_WASH));
            pal.pink
        }
        // The plain `.pill`: a hairline round nothing, and the dim.
        Rec::Idle => {
            painter.rect_stroke(rect, radius, Stroke::new(1.0, pal.line), StrokeKind::Inside);
            pal.dim
        }
    };
    let dot = Rect::from_min_size(
        Pos2::new(
            rect.min.x + size::PILL_PAD_X,
            rect.center().y - size::SINK_DOT * 0.5,
        ),
        egui::vec2(size::SINK_DOT, size::SINK_DOT),
    );
    painter.circle_filled(dot.center(), size::SINK_DOT * 0.5, ink);
    let galley = painter.layout_no_wrap(
        REC_LABEL.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        ink,
    );
    painter.galley(
        Pos2::new(
            dot.max.x + size::SINK_GAP,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );
}
