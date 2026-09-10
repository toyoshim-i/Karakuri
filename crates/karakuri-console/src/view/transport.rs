use super::*;

// ---------------------------------------------------------------------------
// The Transport bay
// ---------------------------------------------------------------------------

/// **What the transport row reads this frame**: the session's tempo and
/// position, and what the frame before this one cost.
///
/// # The same seam as [`View::picture`], and it is not crossed
///
/// Every field is a number and none of them is a `Deck`, an `Instant` or a
/// `Duration` that means *now*. This crate takes no device, no window and no
/// clock (ADR-0156), and this row is made of a clock and an engine — so
/// whoever owns those reads them and writes this per frame, exactly as
/// whoever owns the device registers a texture and writes [`View::picture`].
/// A `Deck` here would put the engine in this crate's dependencies; an
/// `Instant` here would put a clock in it, and then the row would be reading
/// wall time in a repository whose first principle is that nothing does
/// ([P-0092](../../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
///
/// **It carries what cannot be derived and nothing that can.** The beat within
/// the bar and the bar number are arithmetic on [`Transport::beats`] and are
/// [`Transport::beat`] and [`Transport::bar`] rather than two more fields:
/// three numbers for one position is three chances for the lit dot and the bar
/// beside it to come from different arithmetic and disagree.
///
/// **No repaint arm.** [`crate::repaint::Change`] is one list of everything
/// that can change what the console shows, and this is not on it: the values
/// move when the engine draws a frame, and a caller that is drawing engine
/// frames is already asking for frames for the picture beside this row. A
/// panel with nothing live on it is a panel where the oscillator is not
/// advancing either, so the row is right to be still — see
/// [`Transport::fps`], which is the one field that would go stale there and
/// is the one the program leaves `None`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transport {
    /// The tempo, drawn in `.bpm`'s treatment. `karakuri_signal`'s
    /// `Oscillator::bpm` is an `f32` and so is this.
    ///
    /// **Not derivable from [`Transport::beats`]**, which is why it is its
    /// own field: the oscillator's position is an accumulator precisely so
    /// that a tempo correction changes the rate from now on without moving a
    /// beat that has already happened, so the tempo is not the slope of the
    /// position and cannot be read off it.
    pub bpm: f32,
    /// **Musical position, unbounded and monotone** — `Oscillator::beats`,
    /// which counts from the start of the session and never wraps.
    ///
    /// `f64` because that is the type the oscillator accumulates it in, and
    /// narrowing a number this crate does not own is a rounding taken for
    /// nothing. What it would eventually cost is real but distant — an `f32`
    /// stops separating consecutive beats somewhere past eight million of
    /// them — so the reason to keep it is the first one and not the second.
    ///
    /// Which dot is lit and which bar it is are both read off this one number
    /// — see [`Transport::beat`] and [`Transport::bar`].
    pub beats: f64,
    /// **How many beats there are in a bar**, which is how many dots the grid
    /// has.
    ///
    /// **A field rather than a constant, and that is a conclusion rather than
    /// caution.** The mock draws four and `karakuri_signal`'s
    /// `oscillator::BEATS_PER_BAR` is 4, so the number is not in doubt
    /// today — but that constant says of
    /// itself that *"v0.2 of the IR spec has no time-signature concept
    /// anywhere … a fixed assumption of common time rather than something
    /// derived from the record stream. Treat it as provisional until a real
    /// time signature shows up in the format."* A constant here would be this
    /// crate transcribing a number the crate that owns it has written down as
    /// provisional, and the day a time signature lands the console would draw
    /// four dots against a grid of three with nothing saying so. So it is
    /// asked of whoever owns the grid, once a frame, and the grid is that many
    /// dots wide.
    ///
    /// Read through [`Transport::dots`], which is where a zero is dealt with.
    pub beats_per_bar: u32,
    /// **Frames a second**, or `None` where nobody can say yet.
    ///
    /// A rate is measured over a stretch, so a window that has not been left
    /// alone for one has no rate — and `None` draws no `fps` at all rather
    /// than a `0` or a stale number. **Not derived from
    /// [`Transport::frame_ms`]**: `1000 / frame_ms` is the rate the loop
    /// *could* manage, and what it *does* draw is decided by the display and
    /// by whether anything asked for a frame. On a `Fifo` surface those two
    /// differ by the whole vsync wait, which is most of the frame.
    pub fps: Option<f32>,
    /// **What one frame cost on the CPU**, in milliseconds — the `12.4` in the
    /// mock's `12.4/16.6 ms`.
    ///
    /// It is a fact about a frame that has already been drawn, so it does not
    /// go stale on a still window the way a rate does: the last frame did cost
    /// this.
    pub frame_ms: f32,
    /// **What a frame has to fit in**, in milliseconds — the `16.6`, and
    /// `None` where nothing can say what it is.
    ///
    /// Then the row draws the frame time with no budget beside it, which is
    /// the rule the whole of this value follows: a reading nobody has is not
    /// drawn as a plausible one.
    pub budget_ms: Option<f32>,
    /// **What the last write did** — the mock's `landed` capsule at the end of
    /// this row, and `None` until a write has done anything.
    ///
    /// # It is [`Stage`] and not a fourth spelling of the same three words
    ///
    /// `docs/manual/console.html` gives the pill and the Staging lane's rows
    /// one sentence apiece and they are the same answers — *"landed,
    /// overloaded, failed to build, or did not compile"* under *Health, in the
    /// transport*, and *"whether it is on screen: landed, overloaded for
    /// costing more than one frame may, refused, or did not compile"* on a
    /// candidate row. A second enum here
    /// would be `docs/contributing.md` §4's *a name meaning two things* built
    /// on purpose, so [`Stage`] has two readers and one set of words.
    ///
    /// **The two readings are not the same reading, which is why both are
    /// drawn.** A candidate row is one deck slot with a verdict *outstanding*
    /// and leaves the lane the moment the watchdog says the version held the
    /// budget — *"empty is this lane's ordinary state"*. This is the last
    /// verdict there was, on whichever slot, and it stands after the lane has
    /// emptied. So the lane answers *what is unsettled* and this answers *what
    /// did the last write do*, which is the row the operations page gives this
    /// pill and gives the lane's controls a different one.
    ///
    /// # `None` is *nothing has been written*, and it draws no capsule
    ///
    /// Not a word for it, not a dash, and not an `armed` pill reading
    /// `landed` about a build nobody made — the rule the two fields above
    /// follow, and the Staging lane's own *"no row, no placeholder, and no
    /// standing sentence"*. A run in which nobody rewrites a procedure never
    /// draws this, which is most runs.
    ///
    /// **Two `swap::Event`s leave it alone rather than clearing it.** The
    /// watchdog's verdict in favour settles a candidate and does not take the
    /// last write off the screen, and a lost build worker says nothing about a
    /// write that already happened — `crates/karakuri/src/main.rs`'s
    /// `staging`, which is where the one drain feeds both readings.
    pub health: Option<Stage>,
    /// **Whether a session is being recorded** — the mock's `● rec` pill at
    /// the very end of this row, after [`Transport::health`] — and `None` for
    /// a console nobody has told anything about recording, which draws no
    /// pill at all.
    ///
    /// # `None` is *nobody said*, and it is not *not recording*
    ///
    /// [`View::audio`]'s distinction one row along, and for its reason: a
    /// recording is a file being written by a program that has a store, and
    /// `src/` has neither (ADR-0156). `Some(Rec::Idle)` is a program that
    /// holds a store and is not recording into it, and it draws the pill in
    /// the plain treatment because a press on it would start one; `None` is a
    /// console that was never told, and a pill drawn for it would be this
    /// crate answering a question about a disk on its own authority. Every
    /// test in this crate that does not say otherwise leaves it `None`.
    ///
    /// # It is on [`Transport`] rather than beside it, which [`View::look`] is
    /// not
    ///
    /// [`Transport::health`]'s reason, and this is the second field here that
    /// is not a clock: what this type is is *what the transport row reads this
    /// frame*, and the two capsules at the end of the row are read by it. The
    /// difference from the look — which is two fields away on [`View`] because
    /// the row draws it and the master chain owns it — is that neither of
    /// these two capsules is drawn anywhere else and neither belongs to
    /// another bay.
    pub rec: Option<Rec>,
}

/// **What the `● rec` pill says this frame**, and the whole of its state.
///
/// Two values because the control is a toggle and a toggle has two ends: a
/// press on it starts a recording or stops the one running, and which of those
/// a press means is exactly this. There is no third value for *starting* —
/// opening a recorder writes a Set file and creates another, so it happens off
/// the frame path (`crates/karakuri/src/main.rs`), and until the recorder is
/// open nothing is being recorded and the pill says so.
///
/// **A two-valued enum rather than a `bool`**, which is
/// [P-0087](../../../../docs/principles/0087-name-the-property-never-the-shape.md):
/// `Some(true)` at a call site says nothing, and this is read at four of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rec {
    /// Nothing is being recorded. The mock's plain `.pill`, and a press starts
    /// one.
    Idle,
    /// A session is being written to the store as it happens. The mock's
    /// `.pill.on`, and a press stops it.
    Running,
}

impl Transport {
    /// **How many dots the beat grid has**: [`Transport::beats_per_bar`], and
    /// at least one.
    ///
    /// The clamp is here and in one place because a bar of no beats is two
    /// failures at once — a grid with nothing in it, and a division by zero in
    /// [`Transport::beat`] that comes out `NaN` and lights no dot in a row
    /// that has none. One beat to the bar is the nearest thing to a grid that
    /// can be drawn, and every reader of the field goes through here.
    pub fn dots(&self) -> u32 {
        self.beats_per_bar.max(1)
    }

    /// **Where the beat is inside the current bar**, from zero, and
    /// continuous: `0.0` is on the first dot, `1.5` is halfway between the
    /// second and the third, and `3.75` is three quarters of the way from the
    /// last dot back round to the first.
    ///
    /// **A position and not an index**, which is the whole of what the grid
    /// draws now
    /// ([ADR-0212](../../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md)):
    /// the light is somewhere on the grid at every instant rather than on one
    /// dot at a time, so what the row reads is the fractional beat the
    /// oscillator already accumulates rather than a truncation of it. The
    /// fraction was always in [`Transport::beats`] and was thrown away here.
    ///
    /// **It is the whole of the beat's motion, and it is not a clock.** No
    /// second value is written for the grid and none is derived from
    /// [`Phase`]: the beat moves because the session moves, which is what a
    /// beat grid is for, and a wall-clock sweep would go on sweeping at its
    /// own rate over a session running at another one — see [`beat_at`].
    ///
    /// `rem_euclid` rather than `%` because [`Transport::beats`] can be
    /// negative — `Oscillator::behind` reads the same grid at an earlier time,
    /// and a slot warming behind the session is exactly what that is for — and
    /// `%` on a negative is negative, which is a position no grid has.
    ///
    /// **The clamp this used to need went with the index.** `(-1e-18_f64)
    /// .rem_euclid(4.0)` is `4.0` exactly: the true remainder is a hair under
    /// the divisor and rounds up to it. As a dot *index* that was one past the
    /// end of the grid — a rectangle drawn beside it, or a panic — and it was
    /// clamped. As a *position* it is the downbeat: `4.0` and `0.0` are the
    /// same point on a cycle of four, [`beat_at`] measures round the cycle,
    /// and the value arrives where it belongs with nothing written for it.
    pub fn position(&self) -> f32 {
        self.beats.rem_euclid(self.dots() as f64) as f32
    }

    /// **Which bar it is, counting from one** — the `37` in the mock's
    /// `bar 37`.
    ///
    /// One-based because that is how a bar is counted out loud, and there is
    /// no bar 0 in anything an operator says. Signed for the reason
    /// [`Transport::beat`] takes `rem_euclid`: a position before the session's
    /// zero is a bar before the first one, and `as u32` on it would saturate
    /// to zero and draw `bar 1` for every one of them.
    pub fn bar(&self) -> i64 {
        self.beats.div_euclid(self.dots() as f64) as i64 + 1
    }
}

/// **The transport row, laid out**: where each of the five readouts goes, and
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
/// press acts on. **The last of them is drawn as a capsule and is still one**:
/// the shape is the mock's, and what makes a thing a control here is that a
/// press on it asks for something. `tests/transport.rs` asserts it over the
/// row rather than leaving it to be inferred from the absence of a hit test,
/// and it asks with the pill drawn so that the assertion cannot pass on the
/// control having gone.
///
/// **The tempo is not one of the four**, and it left them when the figure
/// became the track: the number is still a reading, and a press on it names a
/// value outright — [`TransportRow::tempo`], and ADR-0291. A readout that a
/// press acts on is a control, which is the same test the capsule fails.
///
/// **The other control is [`TransportRow::rec`], and it is drawn *inside* this
/// derivation where [`arrangement`] is drawn beside it.** The difference is
/// where each one sits: the arrangement pill is laid out from
/// [`TransportRow::bar`] and held clear of [`TransportRow::frame`], so a row
/// that never heard of it is laid out exactly as it is now; the `rec` pill
/// takes the row's right padding, and the health capsule and the frame readout
/// are laid out backwards from it. A pill drawn beside the row could not move
/// them, and two derivations of one right-hand end are two answers.
///
/// **This paragraph said *nothing here is a control*, then *none of these five
/// is*, then *the sixth is this row's one control*, and every one of those
/// changes is deliberate.** What a press on the capsule asks for is
/// [`TransportRow::record`], and what a press on the number asks for is
/// [`TransportRow::tempo`].
///
/// # What is in the mock's row and is deliberately not here
///
/// **The list is empty as of 2026-09-10, and that is a state rather than a
/// deletion.** It held `learn` and `map · nanoKONTROL2 ▾` — *"each a control
/// over machinery that is in neither this crate nor the program"* — and both
/// are drawn now:
///
/// - `learn` is [`learn_pill`], and the machinery arrived with
///   `docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md`:
///   the panel opens a port at start-up, and a press on this arms the gesture
///   the mock's tooltip describes.
/// - `map · <name>` is [`map_pill`], **and it is a readout**. What was written
///   against it was that *"a menu naming a device nobody has plugged in is
///   worse than no menu"*, and that half is unchanged — reaching a map while
///   running is not built, so the pill has no chevron and no menu and says
///   which file is loaded. A capsule that looked like a menu and opened none
///   would still be the scaffolding this module refuses; one that states a
///   true thing is not.
///
/// **The rule that emptied it is the one that kept them out**: a control is
/// drawn when there is something behind it. [`arrangement`] was the first item
/// in this row to pass that test and [`audio_in`] the second; these are the
/// third and fourth.
///
/// **`● rec` left this list on 2026-09-08**, and what was written against it
/// was that *"there is no session recorder behind this panel and no record
/// stream is written from it"*. There is one now: `crates/karakuri` opens a
/// `karakuri_environment::session::Recorder` on a press and closes it on the
/// next, and this pill is the toggle — [`TransportRow::record`], drawn in the
/// mock's `.pill.on` while a recording runs. The mock's tip named one gesture
/// on it, *click to stop*, and the control is both: what a press means is what
/// [`Transport::rec`] says it will be, before it is made.
///
/// **This paragraph carried a total of the row and it no longer does**, which
/// is a deletion rather than an oversight. It said *"the mock draws ten things
/// and five of them exist"*, and the ten was a number nobody could check
/// against the markup: `.transport` has fourteen children and one of them —
/// `.tracker` — holds four more. A list of what is missing is checkable one
/// item at a time; a total of what a row holds is a second count of the mock,
/// and it had already gone stale once, when `audio-in` left this list.
///
/// **`landed` left it on 2026-09-08 and it is the one item here that never
/// was a control.** What was written against it was that *"nothing writes a
/// procedure while this panel runs, so the pill would be reporting on a write
/// that never happens"*, and that was already false when it was read again:
/// `crates/karakuri` watches every slot's sources, so a save from any editor
/// builds, swaps and is judged, and every verdict of that is drawn in the
/// Staging lane. This row is where the same stream says *what the last write
/// did* — [`Transport::health`], which is a value the harness hands in and not
/// a control this crate offers.
///
/// **Three items left it on 2026-09-08** — `tap`, `offset` and the octave's
/// `½ ×2` — which is M5.4's own work: [`tracker_group`] draws them, and what
/// was written here about the first of them was that *"nothing times a tap
/// here, and the pill would correct nothing"*. Something does now, and it
/// always did on the keyboard: `karakuri_environment::audio::Audio::tap` is
/// what `b` has been reaching, and the pill reaches it by emitting the same
/// operation. `audio-in` was the first to leave, on the same terms.
///
/// The `.sep` between the bar and the frame readout **is** drawn, in the only
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

/// **The word in the `rec` pill**, and the mock's `&#9679; rec` without its
/// mark: the mark is drawn rather than typed, which is [`Mask`]'s rule for the
/// same reason — *"whether `◯` and `◑` are in `egui`'s default face is a
/// question with no good answer, and a circle is the same mark either way"*.
const REC_LABEL: &str = "rec";

/// **The faint word beside the number**, and the mock's own capitalisation
/// this time: `.transport`'s `BPM` is upper-case in the markup rather than in
/// CSS, so it is upper-case here.
const BPM_LABEL: &str = "BPM";

/// The transport row's furniture: a rectangle for each readout, and which beat
/// is lit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransportRow {
    /// The tempo, in `.bpm`'s 20px box.
    pub bpm: Rect,
    /// The faint `BPM` beside it.
    pub label: Rect,
    /// The whole `.beat-grid`: [`TransportRow::dots`] dots and the gaps
    /// between them. One dot is [`TransportRow::dot`].
    pub grid: Rect,
    /// How many dots are in that grid — [`Transport::dots`], carried so the
    /// painter and a test read the same number the width was built from.
    pub dots: u32,
    /// **Where the light is**, in beats from the first dot's centre —
    /// [`Transport::position`], carried for the reason [`TransportRow::dots`]
    /// is carried: the painter and a test read the one number the row was
    /// measured from. How much of it lands on any given dot is
    /// [`TransportRow::lit`].
    pub at: f32,
    /// `bar 37`.
    pub bar: Rect,
    /// The frame readout, pushed to the right by the `.sep` — against the
    /// row's right padding where there is no verdict to draw, and one
    /// [`size::TRANSPORT_GAP`] before [`TransportRow::health`] where there is.
    pub frame: Rect,
    /// **The health capsule**, and `None` where [`Transport::health`] is —
    /// which is every run until somebody rewrites a procedure.
    ///
    /// **The `rec` pill is what follows it**, so the right padding is
    /// [`TransportRow::rec`]'s where there is one and this capsule's where
    /// there is not — and everything before it is laid out backwards from
    /// whichever ends the row. No gap is left for a pill that is not drawn.
    pub health: Option<Rect>,
    /// **The `● rec` pill**, and `None` where [`Transport::rec`] is — which is
    /// a console nobody has told anything about recording.
    ///
    /// **It is the last thing in the row and takes the right padding**, which
    /// is where the mock puts it: after `landed`, hard against the end of
    /// `.transport`. It is the one thing in this row that is a control — a
    /// press on it starts a recording or stops the one running — and it is
    /// carried here rather than derived beside the row for
    /// [`TransportRow::health`]'s reason one item along: where each readout
    /// ends is where the next one starts, so the pill's place and the
    /// readouts' places are one derivation and not two that agree until they
    /// do not.
    ///
    /// **It said it was the one control in this row and it is one of two**:
    /// the tempo figure is the other ([`TransportRow::tempo`]), landed in the
    /// same commit and drawn at the row's other end.
    pub rec: Option<Rect>,
    /// **The values these rectangles were measured from.**
    ///
    /// Carried rather than passed to the painter beside this, for
    /// [`Picture`]'s own reason: whoever measured the type and whoever paints
    /// it are then one statement, so a row laid out for one tempo and painted
    /// with another cannot be written by accident. It is also what leaves
    /// **one** place where *no engine means nothing at all* is decided — this
    /// answering `None` — rather than that rule being asked once here and
    /// again in [`View::draw`], which is two answers that agree until they do
    /// not.
    pub values: Transport,
}

/// **How far a press may move the tempo**: **±15%** of what the grid is
/// running at when it lands.
///
/// # It is a guard against a mis-click, and not a bound on what a tempo may be
///
/// Nothing in this workspace says a tempo may not be 240 —
/// `karakuri_audio::tempo::BPM_RANGE` says outright that it is *not* the range
/// of answers, and *"a grid at 240 bpm is a perfectly good grid"* — and this
/// band does not say it either: it bounds **one press**, against the tempo of
/// the moment, so a hand walks the grid anywhere it likes — **240 is five
/// presses up from the tempo the mock draws**, and the figure names it
/// outright on the last of them.
///
/// **The `½ ×2` beside the figure is not the shortcut it looks like**, and at
/// this row's own tempo it is not available at all: [`Tracker::double`] is
/// `karakuri_audio`'s `BPM_RANGE` against twice the grid, 256 is outside it,
/// and the mock draws that half inert. So the figure is the whole of the way
/// up from 128, and `docs/manual/console.html`'s *"get there in two presses"*
/// is not arithmetic that works from there — reported rather than quietly
/// satisfied by a wider band than the one that was decided.
///
/// # A press outside it is ignored, and *ignored* is the decision
///
/// Not clamped, which is the one thing a track this crate has drawn before
/// does do — [`unit_of_offset`] draws a value past either end *at* that end,
/// because the offset's ends are the range of the value. These are not ends of
/// anything: they are how far a hand is trusted to have meant it. Clamping
/// would turn a press the operator did not mean into a 15% move of the grid,
/// which is the loudest thing this row can do; ignoring it leaves the tempo
/// where it was, and a tempo that did not move is a press the operator can see
/// did not land ([ADR-0291](../../../../docs/adr/0291-the-tempo-figure-is-the-track-and-the-band-is-a-guard-on-the-hand.md)).
///
/// # It is here and nowhere else
///
/// **It is not in `karakuri-audio`, and must not be.** The tracker folds every
/// candidate into a window centred on the grid and the beat lock re-acquires
/// on evidence; a ±15% bound anywhere in that path is a grid that cannot
/// follow a song, which is a far worse failure than a hand that can ask for
/// anything. So the band lives at the one point a *press* becomes an
/// operation, which is this type — the same seam
/// [`TrackerGroup::half`] draws an inert octave chip at, one group along.
pub const TEMPO_BAND: f32 = 0.15;

/// **What the whole figure spans**, as the same fraction of the same tempo:
/// **two bands either way**, so [`TEMPO_BAND`] is the middle half of the
/// number and the outer half is the guard.
///
/// **The figure has to reach past the band or the band could not be missed**,
/// and a guard nothing can land in is not a guard — it is an assertion that
/// the surface already satisfies, and `tests/tempo_figure.rs` could only pass
/// it by construction. So the question is not whether the number reaches further
/// than a press may go, but by how much, and the answer is *one more band*:
/// a press that misses by up to as much again as it is allowed to move is
/// refused, and a press that misses by more than that is off the figure and
/// was never this control's.
///
/// **Linear, and that is the offset track's arithmetic rather than the
/// exposure's.** [`offset_at`] is linear because a latency offset is a
/// *difference*, and [`exposure_at`] is logarithmic because an exposure is a
/// *ratio* — a tempo is the second kind of quantity, which is why the control
/// beside this one is an octave and why this band is a percentage. But the
/// band is stated as ±15% **of the tempo at the press**, and at the instant of
/// a press that is a fixed number of beats a minute: the whole geometry is
/// settled by the one number the row was measured with, and equal distances
/// along the figure are equal numbers of beats a minute. Over a span this
/// narrow the two curves are within a percent of each other in any case, and
/// the linear one is the one whose middle is exactly the tempo drawn there.
pub const TEMPO_SPAN: f32 = TEMPO_BAND * 2.0;

impl TransportRow {
    /// One dot of the beat grid, from the left. **The gaps are between the
    /// dots and nowhere else**, which is what a flex row with a `gap` is —
    /// the same reading [`preview_cells`] takes of a grid.
    ///
    /// Panics on a dot this grid has not got, which is a caller having
    /// invented a beat: [`TransportRow::on`] is [`Transport::beat`] and that
    /// is inside the grid by construction.
    pub fn dot(&self, index: u32) -> Rect {
        assert!(index < self.dots, "dot {index} of a grid of {}", self.dots);
        Rect::from_min_size(
            Pos2::new(self.grid.min.x + BEAT_PITCH * index as f32, self.grid.min.y),
            egui::vec2(size::BEAT_W, size::BEAT_H),
        )
    }

    /// **How much of the light is on the dot at `index` this frame**, from
    /// `0.0` to `1.0` — [`beat_at`] at this row's position, so the painter and
    /// a test ask one question and get one answer.
    ///
    /// Panics on a dot this grid has not got, for [`TransportRow::dot`]'s
    /// reason.
    pub fn lit(&self, index: u32) -> f32 {
        assert!(index < self.dots, "dot {index} of a grid of {}", self.dots);
        beat_at(self.at, index, self.dots)
    }

    /// **How far along the figure `p` is**, on `[0, 1]` across
    /// [`TransportRow::bpm`] and outside it either side.
    ///
    /// Not clamped, unlike [`unit_of_offset`]'s: what is off the number is not
    /// at its end, it is not this control's, and every caller here has asked
    /// [`Rect::contains`] first.
    fn along(&self, p: karakuri_layout::Point) -> f32 {
        (p.x - self.bpm.min.x) / self.bpm.width()
    }

    /// **What a press `unit` of the way along the figure names**, in beats a
    /// minute — [`offset_at`]'s job on the one control whose track is the
    /// reading itself.
    ///
    /// **The middle of the number is the number**: at `0.5` this is
    /// [`Transport::bpm`] exactly, which is what `docs/manual/console.html`
    /// means by *"the number under your finger is the one you get"* — the
    /// figure is drawn at the tempo it names, so pressing where it is drawn
    /// asks for what is already there. The ends are [`TEMPO_SPAN`] either way,
    /// and the half of the figure between the quarters is [`TEMPO_BAND`].
    ///
    /// **It is a function of the tempo the row was measured at**, so the whole
    /// control moves with the grid: at 128 a press at the right-hand end asks
    /// for 166.4, and at 256 the same pixel asks for 332.8. There is no track
    /// with ends in it anywhere here, which is why there is no constant naming
    /// them — the offset's [`LATENCY_OFFSET_MIN_MS`] is a restatement of a
    /// range something else holds, and nothing holds one for this.
    pub fn tempo_at(&self, unit: f32) -> f32 {
        self.values.bpm * (1.0 + (unit - 0.5) * 2.0 * TEMPO_SPAN)
    }

    /// **Whether a tempo is one a press may ask for**: within [`TEMPO_BAND`]
    /// of what the grid is running at, measured against the tempo this row was
    /// derived with.
    ///
    /// **Which is the tempo at the press**, and not one from an earlier frame:
    /// `karakuri/src/main.rs` derives this row again on the pointer event, off
    /// the same [`View::transport`] the frame before it was painted from — the
    /// honest cost [`tracker_group`] states for the group beside this one, paid
    /// here for the same reason.
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

    /// **Whether `p` is on the tempo figure asking for something it may
    /// have** — on the number, and inside [`TransportRow::in_band`].
    ///
    /// **The band is part of the hit test and not only of the answer**, which
    /// is [`TrackerGroup::half`]'s rule word for word: *inert is not claimed*,
    /// and this crate's own *a control claims what it acts on and no more*. So
    /// a press on the guard is `egui`'s — it falls through exactly as a press
    /// on the refused half of the octave does, rather than being swallowed by
    /// a control that then does nothing, which is the one outcome an operator
    /// cannot tell from a panel that has stopped
    /// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    ///
    /// **The number's own box is the target and nothing is grown**, unlike
    /// [`OffsetTrack::grip`]: `.bpm` is 20px type at `line-height: 1.5`, so
    /// this is 30 tall against that track's 5, and it leaves 9 of the row's 48
    /// above and below — clear of the [`GRAB`] of the boundary under the row,
    /// which is 6.
    pub fn on_tempo(&self, p: karakuri_layout::Point) -> bool {
        self.bpm.contains(Pos2::new(p.x, p.y)) && self.in_band(self.tempo_at(self.along(p)))
    }

    /// **What a press at `p` asks the grid to run at**, or `None` off the
    /// figure and on the guard around it.
    ///
    /// # The press names the tempo outright, and that is the decision
    ///
    /// [`Operation::SetFreeRunTempo`] carries the number, and the number is
    /// where along the figure the press landed. `docs/manual/console.html`:
    /// *"A press names a value outright rather than stepping, so the figure is
    /// the track and the number under your finger is the one you get."* It is
    /// [`OffsetTrack`]'s sentence one control to the left
    /// ([ADR-0277](../../../../docs/adr/0277-the-latency-offset-is-a-track-because-a-capsule-cannot-name-a-value.md)),
    /// and it arrives here without that record's derivation: **this row has no
    /// key**, so there is no press to make a pixel out of, and what decides the
    /// figure's scale is the figure's own width and [`TEMPO_BAND`].
    ///
    /// # It is emitted with a room being tracked, and that is not this crate's
    /// call
    ///
    /// The operation is *what the grid runs at with nothing driving it*, and
    /// the state it is for is a program with no audio device — which is the
    /// state `crates/karakuri` runs in, where `tap` and the octave both refuse
    /// out loud because there is no room. With a room open it is **accepted**
    /// rather than refused: the tracker searches a window centred on the grid,
    /// so moving the grid moves the window and the estimate is made again
    /// around the new target — `karakuri_audio`'s `BeatLock::retarget`, which
    /// is where that is written down and is the reason this is not a control
    /// that undoes itself two seconds later. Nothing here can see whether a
    /// room is being tracked in any case: this crate takes no device
    /// (ADR-0156), and [`Tracker`] carries what the panel *draws* rather than
    /// what a press is allowed to be.
    pub fn tempo(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.on_tempo(p).then(|| Operation::SetFreeRunTempo {
            bpm: self.tempo_at(self.along(p)),
        })
    }

    /// **Whether `p` is on the `rec` pill**, which is one of the two controls
    /// this derivation owns: the tempo figure is the other
    /// ([`TransportRow::on_tempo`]), and the four things beside them — the
    /// beat grid, the bar, the frame readout and the health capsule — are
    /// readouts, which are not something a press acts on.
    ///
    /// `false` where the pill is not drawn — a console nobody has told
    /// anything about recording claims nothing.
    pub fn on_rec(&self, p: karakuri_layout::Point) -> bool {
        self.rec
            .is_some_and(|pill| pill.contains(Pos2::new(p.x, p.y)))
    }

    /// **What a press at `p` on the `rec` pill asks for**, or `None` off it.
    ///
    /// **It is a toggle and the two ends are two payloads**, which is what
    /// [`karakuri_operation::Recording`] is: a press with nothing running asks
    /// to begin one, a press with something running asks to end it, and the
    /// pill's own treatment is what says which the next press will be before
    /// it is made. So one capsule is one control and not two, and the state it
    /// reads is the state the press acts on — one reading, so the pill that is
    /// drawn and the press that is performed cannot come apart.
    ///
    /// **`id` is `None`, and the payload saying so is what makes each start a
    /// fresh recording.** A capsule types no name — this console's one
    /// letter-taking flow is bounded to naming an arrangement — so whoever
    /// performs it files under a stamp, exactly as the `keep` capsule's save
    /// does. That is not only a convention here: a second head under one id
    /// would land in the middle of an existing stream and be read back as
    /// edits, so a start that could name the id a stop just closed would be a
    /// press that corrupts a session (ADR-0289).
    ///
    /// **It refuses nothing.** Whether there is a deck to write a head from
    /// and whether the store will take the file are the instrument's answers
    /// rather than this surface's — this crate reaches no disk at all
    /// (ADR-0156) — so the press names the operation and whoever performs it
    /// says what happened.
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

/// **From one dot to the next**: [`size::BEAT_W`] and the [`size::BEAT_GAP`]
/// after it, which is what one beat of travel measures on this grid.
///
/// Derived rather than transcribed: the mock declares an item width and a flex
/// `gap`, and the pitch of a flex row is the one plus the other. It is the
/// unit [`beat_at`] measures in and the unit [`TransportRow::dot`] steps by,
/// which is why it is a constant rather than the same sum written twice.
pub const BEAT_PITCH: f32 = size::BEAT_W + size::BEAT_GAP;

/// **How many steps one beat of travel is drawn in**: the pixels in one
/// [`BEAT_PITCH`], so the light moves by at most one of them between updates.
///
/// [`mixer::ROLL_STEPS`]'s shape and a different answer, because the two motions are
/// different sizes: the roll travels a few pixels of a word's pitch and twelve
/// steps is smooth over it, where the light crosses a whole dot and a gap
/// every beat.
const BEAT_STEPS: u64 = BEAT_PITCH as u64;

/// **How stale the beat grid may get**, which is what the transport row
/// declares under
/// [P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)
/// and what the harness turns into a deadline.
///
/// [`BEAT_MICROS`] in [`BEAT_STEPS`] steps — **24.67 ms, about forty a
/// second**. One beat of travel is one [`BEAT_PITCH`], so this is the light
/// moving by one pixel and no more, which is the coarsest step that reads as a
/// movement rather than as a sequence of positions
/// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
///
/// **Stated at the mock's tempo, and it is the one number here that the music
/// moves.** A beat is 468.75 ms at 128.0 BPM and 375 ms at 160, so the same
/// declaration is a pixel and a quarter a step up there. The alternative —
/// derive it per frame from [`Transport::bpm`], which the row is handed — is
/// [ADR-0212](../../../../docs/adr/0212-the-beat-is-a-light-that-travels-and-it-declares-for-itself.md),
/// and it lost on what it does to the arithmetic rather than on the drawing: a
/// staleness that falls with the tempo makes `Σ (cost / staleness)` a function
/// of how fast the music is, so the two schedulability conditions could only
/// be asserted against a fastest tempo nobody has written down — and inventing
/// one inside the test that noticed it was missing is exactly what
/// [ADR-0210](../../../../docs/adr/0210-a-declared-cost-is-one-panel-pass-written-down-and-held-against-the-run.md)
/// refused for the panel's share of the budget.
///
/// **Finer than [`ROLL_STALENESS`]'s 33.33 ms and coarser than a frame**, and
/// both are the presentation rather than a preference for frames: the roll
/// travels a few pixels and this crosses the grid, so it wants more steps; and
/// a 60 Hz frame is 16.6 ms, so a beat grid that asked for every frame would
/// be asking for more than its own drawing can use.
pub const BEAT_STALENESS: Duration = Duration::from_micros(BEAT_MICROS / BEAT_STEPS);

/// **How much of the light is on the dot at `index`**, with the light `at`
/// beats into a bar of `dots`: `1.0` under its centre, `0.0` a whole
/// [`BEAT_PITCH`] away, and a raised cosine between the two.
///
/// # The same curve as [`roll_at`], and two of the reasons are the same
///
/// It leaves and arrives at zero **with zero velocity**, so a dot does not
/// snap into being dark as the light leaves it. And it is a pure function of a
/// value the harness handed in, so a test asserts it at a position it chose
/// and nothing samples a clock to find out what the panel is doing.
///
/// # Two dots at once, and the row's total light is constant
///
/// The falloff is exactly one pitch wide, so at most two dots are lit and
/// `f(d) + f(1 - d) = 1` for every `d` — the raised cosine's own identity, and
/// therefore the grid's dots always sum to exactly one dot's worth of light
/// (every grid but the degenerate one below, which is one dot and holds all of
/// it). The
/// light **moves along the grid** rather than the grid brightening and dimming
/// as it goes, which is what makes a stop visible: a still grid at half
/// brightness would be indistinguishable from a light sat between two dots.
///
/// # Measured round the cycle, not along the row
///
/// The bar wraps, so the distance from the last dot to the first is one pitch
/// and not three: the light leaves the right-hand end of the grid and arrives
/// at the left-hand end in the same instant, each dot half lit, and there is
/// no frame on which it jumps. That is also why [`Transport::position`] needs
/// no clamp — a position of exactly `dots` is a distance of zero from the
/// first dot.
///
/// # What it draws at the instant of a beat is the mock
///
/// At a whole `at` the dot under the light is `1.0` and every other is exactly
/// `0.0`: one dot in `--c-pink` with its halo and the rest in `--c-line`,
/// which is `.beat-grid i.on` and the four dots the mock's markup draws. **The
/// mock is a frame of this** rather than a picture this contradicts, and
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
///   and every one of them **centred in the row** rather than sat on its
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
/// `line-height: 1.5`), so there is (48 - 30) / 2 = **9** of row above and
/// below it — which is `.transport`'s own `padding: 9px`, arrived at from the
/// other end. The arrangement's `9 + 30 + 9` and the centring agree exactly
/// here, where the Outputs row's 8 + 18.5 + 8 had to give up a quarter pixel.
///
/// `None` where the row cannot hold what goes in it: folded away, soloed away,
/// or too narrow to keep the frame readout clear of the bar. **The mock wraps
/// and this does not**, which is the argument [`outputs_row`] makes about the
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

/// **The tempo as one laid-out run**, so that measuring it and painting it
/// cannot be two different runs of type.
///
/// `.bpm`'s `128.0` is one decimal place, which is also as fine as a tempo is
/// ever named — and the mock's own number, so a transcription that started
/// printing `128` would be visible against it.
///
/// # The gradient is drawn per glyph, which is what the CSS comes to here
///
/// `.bpm` is `background: linear-gradient(94deg, var(--c-mint), var(--c-lav))`
/// with `background-clip: text`: the two colours run left to right across the
/// number, near enough — 94deg is four degrees off horizontal. `epaint` fills
/// a galley with one colour, so the run is split into one format run per glyph
/// and each takes its own point along the ramp. Five glyphs is a coarse ramp
/// and it is the mock's two colours rather than one of them; the alternative
/// is picking an end and losing the other, which is a transcription that drops
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

/// **The frame readout as one laid-out run**: `58 fps · 12.4/16.6 ms`, with
/// the numbers in `.val` and everything else faint.
///
/// One [`LayoutJob`] rather than four galleys laid end to end, because the
/// mock is one run of text with two colours in it — `.val { color:
/// var(--c-text); font-weight: 500 }` inside a span that is `--c-faint` — and
/// laying it out as one is what keeps the spaces between the parts the type's
/// own rather than a gap this file invented. The weight is not honoured:
/// `egui`'s default proportional face has no bold, which `room` says once for
/// the whole crate.
///
/// **Both absences drop their own words and nothing else.** No rate drops
/// `58 fps · ` and leaves `12.4 ms`; no budget drops `/16.6` and leaves
/// `12.4 ms`. Neither draws a `0`, a `—` or a plausible 16.6 that nothing
/// measured.
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

/// **The transport row's contents**: the tempo, the beat grid, the bar and the
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
///   uses for its own glow. **Those two are the ends of a ramp rather than two
///   states**: [`beat_at`] says how much of the light is on each dot, the fill
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
pub(super) fn transport_into(ui: &Ui, pal: &Palette, row: &TransportRow) {
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

/// **The `● rec` pill**: a capsule, a round mark, and the word after it.
///
/// It is not [`pill_into`] because it is not a capsule with a word in it: the
/// mock writes `&#9679; rec`, and the mark is **drawn** rather than set as a
/// glyph for [`Mask`]'s reason — a font this crate does not choose is not
/// something a control's width should depend on.
///
/// **The mark's diameter and the gap after it are the Outputs row's sink's**
/// ([`size::SINK_DOT`] and [`size::SINK_GAP`]), because that is this console's
/// other round mark before a word inside a capsule. Two numbers invented here
/// would be a second answer to a question `style.css` has already been read
/// for once.
///
/// **The mark takes the pill's own colour**, both ways: `.pill.on` is a pink
/// word over a pink wash and `.pill` is a dim word inside a hairline, and a
/// mark in some third ink would be a state this capsule does not have.
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

// ---------------------------------------------------------------------------
// The audio-in pill
// ---------------------------------------------------------------------------

/// **The pill's first word**, `docs/manual/console.html`'s own, hyphen and
/// all: it is the head of the group of four that need an input, and the page
/// names that group by this pill.
const AUDIO_LABEL: &str = "audio-in";

/// **What the pill says with no input open**, and it is a word for a state
/// rather than a name — [`NO_ARRANGEMENT`] one pill to the left, and the
/// preview cell's `C · no slot` one bay down.
///
/// **It is not the same statement as silence**, which is the whole of
/// [P-0084](../../../../docs/principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md):
/// a quiet room measures `0.0` at full confidence and this pill would name the
/// input it measured it through. `none` is *no provider* — every name answers
/// what it answered before audio existed — and the two must not read alike on
/// a panel, because one of them is a room and the other is a cable.
const NO_INPUT: &str = "none";

/// **What the card says where the machine has no inputs at all.**
///
/// A menu with nothing in it would be a card an operator presses and cannot
/// tell from one that failed to open, so the empty case says which it is. It
/// is drawn the way [`Menu::Naming`]'s field is — a card with one line in it
/// and no rows, so [`AudioInPill::row`] hands out no rectangle for something
/// that is not a list — and it is a sentence rather than a row because there
/// is nothing to pick: a press on it shuts the menu like a press on any other
/// part of the card.
const NO_INPUTS: &str = "no inputs on this machine";

/// **What the audio-in pill reads this frame, and whether its menu is down.**
///
/// # The input and the list are handed in, for [`Arrangement`]'s reason
///
/// Which input is open is a device and what inputs there are is an
/// enumeration of the host. `src/` takes no device (ADR-0156) and this crate's
/// manifest holds nothing that could reach one, so whoever opened the input
/// reads both and writes them here — the arrangement pill's seam with a
/// microphone in place of a directory.
///
/// # The menu is not handed in, and that is the other half of the same seam
///
/// Whether the card is down is what the *control* is doing rather than what
/// the instrument is doing, so it lives here and moves only through
/// [`AudioIn::opened`] and [`AudioIn::shut`] — ADR-0225 §d, one pill along. A
/// menu open in the program's memory would be this console drawing a state it
/// could not answer questions about.
///
/// **Two states and not [`Menu`]'s three.** That enum's third state is a name
/// being typed, and nothing here asks for letters: an input is picked from the
/// list of the ones that exist, and a device is not named into being. The
/// *mechanism* is ADR-0225's and is shared — the card hangs from the pill, it
/// is `.lib-row` inside `.lib-list` padding, it counts itself in the Library
/// bay's foot, and `input.rs`'s rule 2 is modal over it — and it is the
/// mechanism that record is about, not the shape of the flag.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioIn {
    /// **The input in use**, by the description the device answers to, or
    /// `None` for an instrument with nothing open. See [`NO_INPUT`].
    pub device: Option<String>,
    /// **Every input the machine has**, in the order the host listed them.
    ///
    /// **Read when the menu opens rather than per frame**, which is
    /// [`Arrangement::filed`]'s rule for its reason: enumerating a host is a
    /// device read and a frame path does not do one (P-0091). It changes when
    /// somebody plugs something in, and the press that opens the menu is the
    /// moment that matters — so the list a hand is about to read is the list
    /// as of the press.
    ///
    /// Empty is a machine with no inputs, and the card says so in as many
    /// words ([`NO_INPUTS`]).
    pub inputs: Vec<String>,
    /// Whether the card is down. Private for [`Arrangement::menu`]'s reason.
    down: bool,
}

impl AudioIn {
    /// **An instrument with nothing open and nothing listed**, and the menu
    /// shut. A `const` for [`Arrangement::NONE`]'s reason: a test can name the
    /// state without building one.
    ///
    /// It is not what [`View::audio`] holds by default — that is `None`, which
    /// is a console nobody has told anything about audio and draws no pill at
    /// all. This is the console that has been told, and told there is nothing.
    pub const NONE: AudioIn = AudioIn {
        device: None,
        inputs: Vec::new(),
        down: false,
    };

    /// **What the pill says after `audio-in ·`**: the input in use, or
    /// [`NO_INPUT`].
    pub fn word(&self) -> &str {
        self.device.as_deref().unwrap_or(NO_INPUT)
    }

    /// The card is down.
    pub fn open(&self) -> bool {
        self.down
    }

    /// Put it down.
    pub fn opened(&mut self) {
        self.down = true;
    }

    /// Take it away.
    pub fn shut(&mut self) {
        self.down = false;
    }

    /// **How many rows the open card has**: one per input. Zero while it is
    /// shut, and zero on a machine with no inputs — that card is a sentence,
    /// not a list.
    fn rows(&self) -> usize {
        match self.down {
            true => self.inputs.len(),
            false => 0,
        }
    }
}

/// **What a press on the audio-in pill or on one of its rows asks for.**
///
/// Three arms rather than [`Ask`]'s five, and it is a second enum rather than
/// three of that one: two of those arms are the arrangement family's — a name
/// being asked for and a `panel::Op` — and neither is a thing this control can
/// ever want. A shared enum with two arms that cannot happen is a `match` every
/// caller has to answer for twice.
#[derive(Debug, Clone, PartialEq)]
pub enum AudioAsk {
    /// Put the card down — a press on the pill with it shut. **The caller
    /// reads the host on this**, and writes what it found into
    /// [`AudioIn::inputs`] before the next frame draws the card.
    Open,
    /// Take it away — a press on the pill again, or anywhere on the card that
    /// is not a row.
    Shut,
    /// **Listen to this input**, named. [`Operation::AttachBeatSource`] with a
    /// [`BeatSource::AudioInput`] carrying the name the row was drawn with —
    /// which is the name the device answered to when the list was read, and
    /// the name `AudioInput::open` matches a selector against.
    ///
    /// **Nothing is refused here.** A device that has gone away since the list
    /// was read is refused where it is opened, out loud, with the list as it
    /// is then (P-0094, P-0090): this control cannot see a device and must not
    /// pretend to.
    Operation(Operation),
}

/// **The audio-in pill, laid out**: the capsule, what is written in it, and
/// the card under it while it is down.
///
/// One derivation, for [`ArrangementPill`]'s reason — [`View::draw`] paints
/// exactly these rectangles and [`crate::input::claim`] hit-tests exactly
/// these rectangles.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioInPill {
    /// **The capsule**, which is what a press has to land in to open the card.
    pub pill: Rect,
    /// Where `audio-in · Scarlett 2i2` is painted, inside the capsule's
    /// padding.
    pub text: Rect,
    /// The `▾` after it. Drawn rather than typed — see [`CHEVRON_W`].
    pub chevron: Rect,
    /// **The card, or `None` while it is shut.**
    pub menu: Option<Rect>,
    /// **How many of [`AudioIn::rows`] the card has room for**, between the
    /// pill and the bottom of the console. Fewer than there are is a machine
    /// with more inputs than the window is tall, and the foot says `n of m`
    /// in the Library bay's own words rather than the list quietly ending.
    /// Zero while it is shut, and zero on a machine with no inputs.
    pub rows: usize,
    /// **What the card would list if the window were tall enough**, carried so
    /// that the foot and the rows are one number rather than two.
    pub of: usize,
}

impl AudioInPill {
    /// Whether `p` is on the pill itself.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// **Whether `p` is anywhere this control owns** — the pill, or the card
    /// while it is down.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        let at = Pos2::new(p.x, p.y);
        self.pill.contains(at) || self.menu.is_some_and(|menu| menu.contains(at))
    }

    /// **Where one row of the card is**, from the top — [`ArrangementPill::row`]
    /// without the rule, because this card has no verbs above its list.
    ///
    /// Panics on a row this card has not got, which is that function's rule: a
    /// caller has invented an item.
    pub fn row(&self, index: usize) -> Rect {
        assert!(index < self.rows, "row {index} of a card of {}", self.rows);
        let menu = self.menu.expect("a card with rows in it");
        Rect::from_min_size(
            Pos2::new(
                menu.min.x + size::LIB_LIST_PAD,
                menu.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32,
            ),
            egui::vec2(menu.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        )
    }

    /// **Which input `p` is on**, as an index into [`AudioIn::inputs`], or
    /// `None` for a point on no row — the card's padding, its foot, or
    /// anywhere off it.
    pub fn item(&self, p: karakuri_layout::Point) -> Option<usize> {
        let at = Pos2::new(p.x, p.y);
        (0..self.rows).find(|index| self.row(*index).contains(at))
    }

    /// **What a press at `p` asks for**, or `None` where the press was on
    /// nothing this control owns. [`ArrangementPill::ask`]'s shape over a list
    /// with no verbs in it.
    ///
    /// A press on the pill toggles the card; a press anywhere else on the card
    /// shuts it, because a press that did nothing at all is the one thing
    /// worse than a press that declines.
    pub fn ask(&self, audio: &AudioIn, p: karakuri_layout::Point) -> Option<AudioAsk> {
        if self.hit(p) {
            return Some(match audio.open() {
                true => AudioAsk::Shut,
                false => AudioAsk::Open,
            });
        }
        let menu = self.menu?;
        if !menu.contains(Pos2::new(p.x, p.y)) {
            return None;
        }
        Some(
            match self.item(p).and_then(|index| audio.inputs.get(index)) {
                Some(name) => AudioAsk::Operation(Operation::AttachBeatSource {
                    source: BeatSource::AudioInput(name.clone()),
                }),
                None => AudioAsk::Shut,
            },
        )
    }
}

/// **The audio-in pill's furniture, derived**: the capsule, the words in it,
/// and the card under it.
///
/// # Where it sits, and why it is first of the drawn controls in this row
///
/// `docs/manual/console.html`'s `.transport` puts `.tracker` — the four things
/// that need an input — immediately after `bar 37`, and this pill is the head
/// of that group. Everything the mock draws between the bar and here is
/// nothing at all, so this lands one [`size::TRANSPORT_GAP`] after the bar.
///
/// **The other three of its group are drawn now**, and they are
/// [`tracker_group`]'s: the offset, the tap and the octave, laid out from this
/// pill's right edge at [`size::PILL_GAP`] — the group's own tighter gap,
/// which is what `.tracker` sets and what this paragraph said would happen the
/// day they landed. [`arrangement`] is laid out from the *group's* right edge
/// now rather than from this pill's, at the row's gap, because what ends there
/// is a whole group.
///
/// # No pill at all where the console has not been told
///
/// `None` wherever [`transport`] answers `None` — the row folded away, soloed
/// away, too narrow, or a console with no engine behind it — and `None` again
/// where `audio` is `None`, which is a console nobody has said anything to
/// about audio and is every test in this crate that does not say otherwise. A
/// pill reading `audio-in · none` on a console that was never told is a
/// reading invented here, which is [`View::transport`]'s own rule: **empty is
/// a state and unasked is not.**
///
/// # What it costs to ask
///
/// One galley lookup for the pill's own words, always, and **while the card is
/// down one more per input**, since the card is as wide as the widest name in
/// it. Paid on a pointer event and on a frame, and only while an operator is
/// looking at the card.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
pub fn audio_in(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
) -> Option<AudioInPill> {
    let audio = audio?;
    // The row, asked once and for everything, exactly as `arrangement` asks
    // it: whether there is one at all, and where the bar ended.
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let width = |text: &str| {
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

    let text_w = width(&audio_text(audio));
    let pill_w = size::PILL_PAD_X * 2.0 + text_w + size::SINK_GAP + CHEVRON_W;
    let mid = strip.center().y;
    let pill = Rect::from_min_size(
        Pos2::new(
            row.bar.max.x + size::TRANSPORT_GAP,
            mid - size::PILL_H * 0.5,
        ),
        egui::vec2(pill_w, size::PILL_H),
    );
    // The same rule `arrangement` and `outputs_row` state: a capsule that does
    // not fit in the row it is drawn in is no control at all, rather than half
    // of one over the frame readout.
    if !strip.contains_rect(pill) || pill.max.x + size::TRANSPORT_GAP > row.frame.min.x {
        return None;
    }
    let text = Rect::from_min_size(
        Pos2::new(pill.min.x + size::PILL_PAD_X, mid - size::PILL_H * 0.5),
        egui::vec2(text_w, size::PILL_H),
    );
    let chevron = Rect::from_center_size(
        Pos2::new(pill.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5, mid),
        egui::vec2(CHEVRON_W, CHEVRON_H),
    );

    let (menu, rows, of) = input_card(&pill, layout, audio, &width);
    Some(AudioInPill {
        pill,
        text,
        chevron,
        menu,
        rows,
        of,
    })
}

/// **The pill's words**: `audio-in · Scarlett 2i2`, or `audio-in · none`. One
/// run of text, for [`pill_text`]'s reason.
fn audio_text(audio: &AudioIn) -> String {
    format!("{AUDIO_LABEL} · {}", audio.word())
}

/// **The card under the audio-in pill**: where it is, how many rows fit in it,
/// and how many there are.
///
/// [`menu_card`]'s arithmetic without the hairline, because this list has no
/// verbs over it — so the furniture is the padding alone. A machine with no
/// inputs gets a card one line tall with [`NO_INPUTS`] in it and no rows at
/// all, which is the shape `menu_card` gives a name being typed and for the
/// same reason: that card is not a list, and nothing may hand out a row
/// rectangle for it.
fn input_card(
    pill: &Rect,
    layout: &karakuri_layout::Layout,
    audio: &AudioIn,
    width: &dyn Fn(&str) -> f32,
) -> (Option<Rect>, usize, usize) {
    if !audio.open() {
        return (None, 0, 0);
    }
    let viewport = to_egui(layout.viewport());
    let top = pill.max.y + size::PILL_GAP;
    let furniture = size::LIB_LIST_PAD * 2.0;

    if audio.inputs.is_empty() {
        let card = held_inside(
            &viewport,
            pill.min.x,
            top,
            width(NO_INPUTS).max(pill.width()) + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
            furniture + size::LIB_ROW_H,
        );
        return (Some(card), 0, 0);
    }

    let of = audio.rows();
    let widest = audio
        .inputs
        .iter()
        .map(|name| width(name))
        .fold(pill.width(), f32::max);
    // How many rows there is room for between the card's top and the bottom of
    // the console. Asked twice for `menu_card`'s reason: the foot is only owed
    // where something is left out.
    let room = |foot: f32| {
        (((viewport.max.y - top - furniture - foot) / size::LIB_ROW_H).floor()).max(0.0) as usize
    };
    let rows = match room(0.0) >= of {
        true => of,
        false => room(size::LIB_FOOT_H).min(of),
    };
    let foot = match rows < of {
        true => size::LIB_FOOT_H,
        false => 0.0,
    };
    let card = held_inside(
        &viewport,
        pill.min.x,
        top,
        widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
        furniture + size::LIB_ROW_H * rows as f32 + foot,
    );
    (Some(card), rows, of)
}

/// **The audio-in pill, painted**, and the card under it.
///
/// Where everything goes is [`audio_in`]'s, so this paints and derives
/// nothing. Term for term from `.pill` and `.pill.armed` in `style.css`:
///
/// - shut, with nothing open: the ordinary capsule — `border: 1px solid
///   var(--c-line); color: var(--c-dim)` — which is [`arrangement_into`]'s
///   treatment and this is the same pill two places along the same row.
/// - **with an input open, `.armed`**: `border-color: transparent; color:
///   var(--c-mint); background: color-mix(in srgb, var(--c-mint) 14%,
///   transparent)`, which is the mock's own class on this pill and the one
///   place in this row a colour means *live*. The `box-shadow: 0 0 9px
///   var(--c-glow)` goes with it, exactly as the beat grid's lit dot carries
///   its halo.
///
/// **The colour and the word say the same thing on purpose.** A pill that was
/// only lit would leave *which* room is being heard unanswered, and a pill
/// that only carried a name would make a dead input and a live one look alike
/// at the distance a panel is read from.
pub(super) fn audio_in_into(ui: &Ui, pal: &Palette, pill: &AudioInPill, audio: &AudioIn) {
    let painter = ui.painter();
    let radius = CornerRadius::same((size::PILL_H * 0.5) as u8);
    let armed = audio.device.is_some();
    let ink = match armed {
        true => pal.mint,
        false => pal.dim,
    };
    if armed {
        painter.add(
            egui::epaint::Shadow {
                offset: [0, 0],
                blur: ARMED_GLOW,
                spread: 0,
                color: pal.glow,
            }
            .as_shape(pill.pill, radius),
        );
        painter.rect_filled(pill.pill, radius, tint(pal.mint, ARMED_WASH));
    } else {
        painter.rect_stroke(
            pill.pill,
            radius,
            Stroke::new(size::HAIRLINE, pal.line),
            StrokeKind::Inside,
        );
    }
    let galley = painter.layout_no_wrap(
        audio_text(audio),
        FontId::new(size::BASE, FontFamily::Proportional),
        ink,
    );
    painter.galley(
        Pos2::new(
            pill.text.min.x,
            pill.text.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            pill.chevron.left_top(),
            pill.chevron.right_top(),
            Pos2::new(pill.chevron.center().x, pill.chevron.max.y),
        ],
        ink,
        Stroke::NONE,
    ));

    let Some(card) = pill.menu else {
        return;
    };
    painter.add(pal.shadow.as_shape(card, CornerRadius::same(8)));
    painter.rect_filled(card, CornerRadius::same(8), pal.panel);
    painter.rect_stroke(
        card,
        CornerRadius::same(8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );

    let line = |rect: Rect, text: String, colour: Color32| {
        let galley = painter.layout_no_wrap(
            text,
            FontId::new(size::BASE, FontFamily::Proportional),
            colour,
        );
        painter.galley(
            Pos2::new(
                rect.min.x + size::LIB_ROW_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            colour,
        );
    };

    // **A machine with no inputs**, which is a sentence and not a list —
    // `pill.rows` is zero, so `row` hands out nothing.
    if audio.inputs.is_empty() {
        let only = Rect::from_min_size(
            Pos2::new(
                card.min.x + size::LIB_LIST_PAD,
                card.min.y + size::LIB_LIST_PAD,
            ),
            egui::vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        );
        line(only, NO_INPUTS.to_owned(), pal.faint);
        return;
    }

    for index in 0..pill.rows {
        let name = &audio.inputs[index];
        // **The one that is open is the one colour the list has**, for the
        // reason the pill has one: a card of names with nothing marked leaves
        // an operator to remember which they picked.
        let colour = match audio.device.as_deref() == Some(name.as_str()) {
            true => pal.mint,
            false => pal.text,
        };
        line(pill.row(index), name.clone(), colour);
    }
    // **`n of m`, in the Library bay's own words**, and only where the list
    // could not be shown whole.
    if pill.rows < pill.of {
        let foot = Rect::from_min_max(
            Pos2::new(card.min.x, card.max.y - size::LIB_FOOT_H),
            card.max,
        );
        let galley = painter.layout_no_wrap(
            format!("{} of {}", pill.rows, pill.of),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.faint,
        );
        painter.galley(
            Pos2::new(
                foot.min.x + size::LIB_ROW_PAD_X,
                foot.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.faint,
        );
    }
}

// ---------------------------------------------------------------------------
// The tracker group: the latency offset, the tap and the octave
// ---------------------------------------------------------------------------

/// **The word before the offset track** — [`EXPOSURE_LABEL`]'s arrangement at
/// the other end of this row, and the mock's own word: a faint label, one
/// [`size::TRIM_GAP`], a horizontal [`fader`], another gap and the figure.
const OFFSET_LABEL: &str = "offset";

/// **The unit, written into the figure and never left off.**
///
/// `docs/manual/console.html` is plain about why: *"Two things on this panel
/// are called an offset, and they are not the same thing … The unit is what
/// tells them apart, so neither is ever drawn without it."* The other is the
/// deck head's anchor, which is in **beats** and is one deck's.
const OFFSET_UNIT: &str = " ms";

/// **What is written in the tap capsule**, and the whole of it — the mock
/// writes one word with no value beside it, because a tap has no state to
/// read back. It is [`TONEMAP_LABEL`]'s kind of capsule and not
/// [`AUDIO_LABEL`]'s: no `▾`, because nothing opens.
const TAP_LABEL: &str = "tap";

/// **The two halves of the octave**, in the mock's own marks — `&frac12;` and
/// `&times;2`, which are one character and two.
const HALVE_MARK: &str = "½";
const DOUBLE_MARK: &str = "×2";

/// **The two ends of the latency offset, in milliseconds.**
///
/// `karakuri_environment::audio::LATENCY_OFFSET_RANGE` is `-200.0..=200.0` and
/// this is that range restated, for [`EXPOSURE_STOPS`]' reason one control
/// along: this crate depends on nothing that could reach it (ADR-0156), and a
/// control has to know its own ends to lay a track out.
///
/// **The copy is held to the original where both are visible**, which is
/// `crates/karakuri` — the binary that has this crate and that one as
/// dependencies — rather than asserted here. That is the difference between
/// this restatement and [`EXPOSURE_STOPS`]': the exposure's ends are private
/// to `karakuri-cli` and nothing can compare them, and these are public.
pub const LATENCY_OFFSET_MIN_MS: f32 = -200.0;
pub const LATENCY_OFFSET_MAX_MS: f32 = 200.0;

/// **What one press of an offset key moves it by**, in milliseconds —
/// `karakuri_environment::audio::LATENCY_OFFSET_STEP_MS`, restated here for
/// the reason the two ends above are, and held to the original in the same
/// place.
pub const LATENCY_OFFSET_STEP_MS: f32 = 5.0;

/// **How many presses of an offset key cross the whole span**: four hundred
/// milliseconds at five a press is **eighty**.
const OFFSET_PRESSES: f32 =
    (LATENCY_OFFSET_MAX_MS - LATENCY_OFFSET_MIN_MS) / LATENCY_OFFSET_STEP_MS;

/// **The track, in pixels: one pixel a press** — [`EXPOSURE_TRACK_W`]'s whole
/// argument, one control along and on a value that is a difference rather than
/// a ratio
/// ([ADR-0277](../../../../docs/adr/0277-the-latency-offset-is-a-track-because-a-capsule-cannot-name-a-value.md)).
///
/// A pointer on this track can ask for any of the eighty positions along it
/// and `o` and `p` can ask for any of the eighty values between the ends, so
/// **neither surface can reach a value the other cannot** — which is what
/// stops a track and a pair of keys nearly agreeing. `docs/manual/console.html`
/// carries the same sentence for the exposure and now for this.
pub const OFFSET_TRACK_W: f32 = OFFSET_PRESSES;

/// **What a point `unit` of the way along the track asks for**, in
/// milliseconds.
///
/// **Linear, and that is not the exposure's arithmetic.** An exposure is a
/// *ratio* and its track is logarithmic for a stated reason — *"an additive
/// step would be enormous at 0.1 and invisible at 8.0"*. A latency offset is a
/// **difference** between two arrival times: five milliseconds is five
/// milliseconds at either end of the range, which is why the key steps by an
/// addition and why equal distances along this track are equal numbers of
/// milliseconds. The middle of the track is exactly zero, because the range is
/// symmetric and the subtraction is exact at a half.
pub fn offset_at(at: f32) -> f32 {
    LATENCY_OFFSET_MIN_MS + unit(at) * (LATENCY_OFFSET_MAX_MS - LATENCY_OFFSET_MIN_MS)
}

/// **Where an offset sits on the track**, on `[0, 1]` — [`offset_at`]
/// inverted, and clamped to the ends for [`unit_of`]'s reason: a value past
/// either end is drawn at that end and the figure beside the track is what
/// says the number. Nothing on the way in is held to these ends — the clamp is
/// `karakuri_environment::audio`'s, at the one place an offset is applied — so
/// this is drawing a reading rather than enforcing a bound.
pub fn unit_of_offset(ms: f32) -> f32 {
    unit((ms - LATENCY_OFFSET_MIN_MS) / (LATENCY_OFFSET_MAX_MS - LATENCY_OFFSET_MIN_MS))
}

/// **What the three controls beside the audio-in pill read this frame**: the
/// offset the session is holding, and which way the grid can still be moved an
/// octave.
///
/// # The same seam as [`View::transport`], and it is the beat lock's rather
/// than the engine's
///
/// Every field is a number or a `bool` and none of them is a device. Whether
/// an octave is available is `karakuri_audio`'s `BPM_RANGE` against the tempo
/// the session is running at, and the offset is a value
/// `karakuri_environment::audio::Audio` holds — `src/` has neither (ADR-0156),
/// so whoever owns them reads them and writes this per frame, exactly as
/// whoever owns the engine writes [`View::look`].
///
/// **It carries what cannot be derived and nothing that can.** The console
/// holds the tempo already — [`Transport::bpm`] — but not the range the
/// tracker searches, so *which half is live* is one of the two things it
/// cannot work out for itself. `docs/manual/console.html` says the panel can
/// work it out *before the press*, and this is what makes that true.
///
/// **A second value rather than three more fields on [`Transport`]**, for
/// [`Look`]'s reason: a tempo and a frame cost are what the session is doing,
/// and these are what the *tracker* is doing. A console driving one and not the
/// other is a state the type should be able to say.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tracker {
    /// **The latency offset the open session is holding**, in milliseconds, or
    /// `None` where nothing is open.
    ///
    /// `None` draws no offset control at all rather than a track at a
    /// plausible figure, which is [`View::audio`]'s own rule one control to the
    /// left: an offset belongs to a session, and until one is open there is no
    /// value being held anywhere for a track to point at. The page says so.
    pub offset_ms: Option<f32>,
    /// **Whether halving the grid is available at this tempo** — the tracker's
    /// range against half of what the session is running at.
    pub halve: bool,
    /// **And whether doubling is.** Never both: the range is under two octaves
    /// wide, which is the mock's own note and is why this is two fields rather
    /// than a direction.
    pub double: bool,
}

/// **The offset control, laid out**: the word, the track, what the value fills
/// of it, the band a press has to land in, and the figure.
///
/// [`LookRow`]'s exposure half, on a linear value — and a type of its own
/// because it is the one part of [`TrackerGroup`] that is not always there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetTrack {
    /// The faint `offset` before the track.
    pub label: Rect,
    /// **The track**, `.fader`'s 5px well lying down at [`OFFSET_TRACK_W`]
    /// long.
    pub track: Rect,
    /// What the offset fills of it, from the left — [`unit_of_offset`].
    pub fill: Rect,
    /// **What a press has to land in to set the offset**: the track grown to a
    /// line's height and no wider, which is [`LookRow::grip`]'s rule and its
    /// argument — a 5px-tall target is not something a hand finds, and a
    /// target reaching past either end would have two pixels of itself asking
    /// for the same value.
    pub grip: Rect,
    /// `−15 ms`, after the track. See [`tracker_group`] for why it is after it.
    pub value: Rect,
}

/// **The three controls that finish the tracker group, laid out**: the offset,
/// the tap and the two halves of the octave.
///
/// # One derivation, for [`LookRow`]'s reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. They are laid end to end from the
/// audio-in pill's right edge, so where the octave is depends on how wide the
/// offset's figure is — three questions about one laid-out group, and a second
/// walk would put the chip a press lands on somewhere the mark is not.
///
/// # Why they are one group and not three controls in a row
///
/// `docs/manual/style.css` says it at `.tracker`, which is the class the mock
/// puts round exactly these four: *"the pill that says whether there is an
/// audio input, the latency offset … the tap, and the octave. None of them
/// means anything without an input, so they are one item at the pills' own 5px
/// rather than four at the row's 14."* So the gap **inside** this group is
/// [`size::PILL_GAP`] and the gap between the group and what follows it is
/// [`size::TRANSPORT_GAP`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackerGroup {
    /// **The offset**, or `None` where no session is open to be holding one.
    pub offset: Option<OffsetTrack>,
    /// **The tap capsule**, which is what a press has to land in to tap the
    /// beat. Drawn whatever the audio-in pill says, because what it asks for
    /// does not depend on a reading — a tap with no room to tap against is
    /// refused **out loud**, which is `Operation::TapBeat`'s own row on
    /// `docs/manual/operations.html`.
    pub tap: Rect,
    /// Where `tap` is painted, inside the capsule's padding.
    pub tap_text: Rect,
    /// **The `½` half**, drawn whether or not it is live.
    pub halve: Rect,
    /// **The `×2` half.** At most one of the two is ever live, which is the
    /// range being under two octaves wide, and the inert one keeps its shape
    /// rather than going away — `.octave i.idle`, and the deck head's scrub
    /// arrows one bay over.
    pub double: Rect,
    /// **The values these rectangles were measured from**, carried for
    /// [`TransportRow::values`]' reason: whoever measured the type and whoever
    /// paints it are one statement.
    pub values: Tracker,
}

impl TrackerGroup {
    /// Whether `p` is on the tap capsule.
    pub fn hit_tap(&self, p: karakuri_layout::Point) -> bool {
        self.tap.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is on the offset track's band. See [`OffsetTrack::grip`],
    /// and `false` where there is no offset drawn at all.
    pub fn hit_offset(&self, p: karakuri_layout::Point) -> bool {
        self.offset
            .is_some_and(|offset| offset.grip.contains(Pos2::new(p.x, p.y)))
    }

    /// **Which half of the octave `p` is on, as the factor it asks for** — or
    /// `None` off both, and `None` on either while that direction is refused.
    ///
    /// **Inert is not claimed**, which is [`DeckHead::arrow`]'s rule word for
    /// word and [`crate::input`]'s *a control claims what it acts on and no
    /// more*. Both halves are drawn on every tempo, because the range is under
    /// two octaves wide and a pair that vanished would move the rest of the
    /// row under the hand every time the grid crossed 100 or 120 BPM.
    fn half(&self, p: karakuri_layout::Point) -> Option<GridScale> {
        let p = Pos2::new(p.x, p.y);
        match (
            self.values.halve && self.halve.contains(p),
            self.values.double && self.double.contains(p),
        ) {
            (true, _) => Some(GridScale::Halve),
            (_, true) => Some(GridScale::Double),
            _ => None,
        }
    }

    /// **Whether `p` is on any of the three**, which is what
    /// [`crate::input::claim`] asks.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.hit_tap(p) || self.hit_offset(p) || self.half(p).is_some()
    }

    /// **What a press at `p` asks for when it lands on the tap capsule**:
    /// [`Operation::TapBeat`], which carries nothing because a tap is an
    /// instant and the instant is when the operation arrives.
    ///
    /// **It is emitted with no input open too**, and that is the decision
    /// rather than a gap: the operations page says the refusal *"says where to
    /// open one rather than doing nothing, because a key that declines and a
    /// key that is not bound are the same experience"*, and the surface that
    /// could see there was no room is not this one — this crate has no device
    /// (ADR-0156). So the press names the operation and whoever performs it
    /// says what happened, exactly as `b` already does.
    pub fn tapped(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_tap(p).then_some(Operation::TapBeat)
    }

    /// **What a press at `p` asks the grid to do**:
    /// [`Operation::ScaleGrid`] naming the direction, or `None` off both
    /// halves and on a refused one.
    pub fn octave(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.half(p).map(|by| Operation::ScaleGrid { by })
    }

    /// **What a press at `p` asks the offset to become**, or `None` where
    /// there is no track under it.
    ///
    /// # The press sets it outright, and that is the decision
    ///
    /// Where along the track the press landed *is* the value, through
    /// [`offset_at`], and what comes out is [`Operation::SetLatencyOffset`]
    /// naming it. The manual is where the words come from — *"The value is
    /// absolute and the keys are the nudge … because a control that could only
    /// be nudged is a control no fader can reach"* — and it is
    /// [`LookRow::exposure`]'s argument arriving one control earlier, because
    /// that control was built from this sentence.
    ///
    /// **It is not [`Mixer::grab`]**, for [`LookRow::exposure`]'s reason:
    /// there is no knob here and no gesture to be mid-way through, so a press
    /// that names a value cannot move the mix under a hand.
    pub fn nudge(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let offset = self.offset?;
        self.hit_offset(p).then(|| Operation::SetLatencyOffset {
            ms: offset_at((p.x - offset.track.min.x) / offset.track.width()),
        })
    }
}

/// **The tracker's other three controls, derived**: the offset, the tap and
/// the octave.
///
/// # Where they sit, and why it is after the audio-in pill
///
/// `docs/manual/console.html`'s `.tracker` is the four things that need an
/// input, in this order: the pill that says whether there is one, the offset,
/// the tap, the octave. [`audio_in`] is the head of it and this is the rest,
/// laid out from that pill's right edge at [`size::PILL_GAP`] — the group's
/// own tighter gap and not the row's — which is exactly what [`audio_in`]'s
/// own documentation said would happen the day these were drawn.
///
/// **With no pill it lands after the bar**, at [`size::TRANSPORT_GAP`], which
/// is where [`arrangement`] used to land and is the same fallback: a console
/// told about the tracker and not about audio is a state nothing in this
/// workspace produces, and it is laid out rather than refused because a
/// derivation that panicked on it would be answering a question about a device
/// on its own authority.
///
/// # The order inside the group, and where the figure goes
///
/// The mock's, unchanged. **The figure is after the track** for
/// [`look`]'s reason, which is the one piece of this layout decided by what a
/// hand does rather than by what the mock draws: it is the only part whose
/// width moves with its value, so putting it last leaves the track and
/// everything after it where they were, and a target that walked away from the
/// pointer as it was set would be worst at the moment it was being used most
/// precisely.
///
/// # No row and no values means none of this
///
/// `None` wherever [`transport`] answers `None` — the row folded away, soloed
/// away, too narrow, or a console with no engine behind it — and `None` again
/// where `tracker` is `None`, which is a console nobody has told anything
/// about the beat tracker and is every test in this crate that does not say
/// otherwise. Drawing a tap on a console with no tracker behind it would be
/// the scaffolding this module refuses.
///
/// # What it costs to ask
///
/// **Five galley lookups of its own**, and two of them go with the offset: the
/// `tap` word, the `offset` word, the figure — which is the one whose width
/// moves with its value — and one for each of `½` and `×2`, since a capsule
/// here is as wide as the mark in it. A console with nothing open pays three
/// of the five.
///
/// And **it re-derives the row and the pill**, which is [`look`]'s honest cost
/// one group to the left and for the same reason: the derivation that draws a
/// control is the one that hit-tests it.
///
/// Paid on a pointer event and on a frame, and a console with no tracker
/// behind it pays none of it: the `tracker?` is the first line.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
pub fn tracker_group(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
) -> Option<TrackerGroup> {
    let tracker = tracker?;
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let width = |text: &str| {
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
    let mark = |text: &str| {
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                text.to_owned(),
                FontId::new(size::OCTAVE_SIZE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    };

    let mid = strip.center().y;
    let span_h = size::BASE * size::LINE;
    // **Where the group's head ended.** The pill where the console has been
    // told about audio, and the bar where it has not — [`arrangement`]'s own
    // one-answer arrangement, read one item earlier in the same row.
    let (after, gap) = match audio_in(ctx, layout, values, audio) {
        Some(pill) => (pill.pill.max.x, size::PILL_GAP),
        None => (row.bar.max.x, size::TRANSPORT_GAP),
    };

    // The offset, where a session is open to be holding one.
    let mut at = after + gap;
    let offset = tracker.offset_ms.map(|ms| {
        let label = Rect::from_min_size(
            Pos2::new(at, mid - span_h * 0.5),
            egui::vec2(width(OFFSET_LABEL), span_h),
        );
        let track = Rect::from_min_size(
            Pos2::new(label.max.x + size::TRIM_GAP, mid - size::FADER_H * 0.5),
            egui::vec2(OFFSET_TRACK_W, size::FADER_H),
        );
        let grip = Rect::from_min_max(
            Pos2::new(track.min.x, mid - size::PILL_H * 0.5),
            Pos2::new(track.max.x, mid + size::PILL_H * 0.5),
        );
        let value = Rect::from_min_size(
            Pos2::new(track.max.x + size::TRIM_GAP, mid - span_h * 0.5),
            egui::vec2(width(&offset_text(ms)), span_h),
        );
        at = value.max.x + size::PILL_GAP;
        OffsetTrack {
            label,
            track,
            fill: filled(track, Axis::Row, unit_of_offset(ms)),
            grip,
            value,
        }
    });

    let tap_w = size::PILL_PAD_X * 2.0 + width(TAP_LABEL);
    let tap = Rect::from_min_size(
        Pos2::new(at, mid - size::PILL_H * 0.5),
        egui::vec2(tap_w, size::PILL_H),
    );
    let tap_text = Rect::from_min_size(
        Pos2::new(tap.min.x + size::PILL_PAD_X, mid - size::PILL_H * 0.5),
        egui::vec2(width(TAP_LABEL), size::PILL_H),
    );

    let chip = |x: f32, text: &str| {
        Rect::from_min_size(
            Pos2::new(x, mid - size::OCTAVE_H * 0.5),
            egui::vec2(
                mark(text) + size::OCTAVE_PAD_X * 2.0 + size::HAIRLINE * 2.0,
                size::OCTAVE_H,
            ),
        )
    };
    let halve = chip(tap.max.x + size::PILL_GAP, HALVE_MARK);
    let double = chip(halve.max.x + size::OCTAVE_GAP, DOUBLE_MARK);

    // The rule [`arrangement`], [`look`] and `outputs_row` all state: a control
    // that does not fit in the row it is drawn in is no control at all, rather
    // than half of one over the frame readout.
    if !strip.contains_rect(tap)
        || !strip.contains_rect(double)
        || double.max.x + size::TRANSPORT_GAP > row.frame.min.x
    {
        return None;
    }

    Some(TrackerGroup {
        offset,
        tap,
        tap_text,
        halve,
        double,
        values: tracker,
    })
}

/// **The figure after the track**: the offset, signed both ways and never
/// without its unit — `−15 ms`, and `+0 ms` at the middle.
///
/// **The sign is always drawn**, which is the page's own instruction: *"the
/// sign is the half that gets read wrong at two in the morning, so the panel
/// says it in words rather than leaving −15 ms to be interpreted."* The words
/// are `karakuri`'s, in the line it prints; the **mark** is this control's, and
/// a `+` that only appeared above zero would be a control that says less
/// exactly where it matters.
///
/// Whole milliseconds, because the step is five of them and the ends are two
/// hundred: a decimal place here would be a precision no surface can ask for.
///
/// **The sign is written in ASCII**, as the deck head's anchor writes its own
/// signed offset — the `−` in the mock and in this comment is the page's
/// typography, and the panel paints what a `{:+}` writes.
///
/// **And the rounding is taken before the sign, so there is no `-0 ms`.** A
/// press a fraction of a pixel left of the middle asks for a value that rounds
/// to zero, and `{:+.0}` writes `-0` for it: two spellings of the one value an
/// operator is most likely to be aiming at.
fn offset_text(ms: f32) -> String {
    // `-0.0 == 0.0` in IEEE, so this is the whole of the normalisation.
    let whole = match ms.round() == 0.0 {
        true => 0.0,
        false => ms.round(),
    };
    format!("{whole:+.0}{OFFSET_UNIT}")
}

/// **The tracker's three controls, painted.**
///
/// Where everything goes is [`tracker_group`]'s, so this paints and derives nothing.
/// Term for term from `style.css`:
///
/// - the `offset` before the track — `style="color:var(--c-faint)"` in the
///   markup, which is `pal.faint`, and is [`look_into`]'s treatment of `exp`.
/// - `.fader` and `.fader b` — the well and the ramp [`look_into`] paints, and
///   `.fader s` is deliberately not drawn for its reason: the mock's knob is a
///   handle and this control has none.
/// - `.pill` — the tap capsule, which is [`arrangement_into`]'s treatment with
///   no `▾`.
/// - `.octave i` — `border: 1px solid var(--c-line); color: var(--c-dim)` at
///   `border-radius: 999px`, and `.octave i.idle` is `color: var(--c-faint);
///   border-color: var(--c-hair)`. **Never grey without a reason** is the
///   stylesheet's own comment, and the reason is on the mock's tooltip.
pub(super) fn tracker_into(ui: &Ui, pal: &Palette, group: &TrackerGroup) {
    let painter = ui.painter();
    let centred = |rect: Rect, galley: std::sync::Arc<egui::Galley>, colour: Color32| {
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };

    if let Some(offset) = group.offset {
        centred(
            offset.label,
            painter.layout_no_wrap(
                OFFSET_LABEL.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                pal.faint,
            ),
            pal.faint,
        );
        let radius = CornerRadius::same((size::FADER_H * 0.5) as u8);
        painter.rect_filled(offset.track, radius, pal.well);
        painter.rect_stroke(
            offset.track,
            radius,
            Stroke::new(size::HAIRLINE, pal.hair),
            StrokeKind::Inside,
        );
        gradient(painter, offset.fill, Axis::Row, pal.mint, pal.lav, true);
        centred(
            offset.value,
            painter.layout_no_wrap(
                offset_text(group.values.offset_ms.unwrap_or_default()),
                FontId::new(size::BASE, FontFamily::Proportional),
                pal.text,
            ),
            pal.text,
        );
    }

    painter.rect_stroke(
        group.tap,
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    centred(
        group.tap_text,
        painter.layout_no_wrap(
            TAP_LABEL.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        ),
        pal.dim,
    );

    let half = |rect: Rect, text: &str, live: bool| {
        let (ink, edge) = match live {
            true => (pal.dim, pal.line),
            false => (pal.faint, pal.hair),
        };
        painter.rect_stroke(
            rect,
            CornerRadius::same((size::OCTAVE_H * 0.5) as u8),
            Stroke::new(size::HAIRLINE, edge),
            StrokeKind::Inside,
        );
        let galley = painter.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::OCTAVE_SIZE, FontFamily::Proportional),
            ink,
        );
        painter.galley(
            Pos2::new(
                rect.center().x - galley.size().x * 0.5,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
    };
    half(group.halve, HALVE_MARK, group.values.halve);
    half(group.double, DOUBLE_MARK, group.values.double);
}

// ---------------------------------------------------------------------------
// learn, and the map it writes into
// ---------------------------------------------------------------------------

/// **What the `map` pill reads** — the name of the map file in use, or `None`
/// for a surface running without one.
///
/// [`AudioIn`]'s shape one pill along, and it is the same seam: a map is a
/// file, `src/` reads none (ADR-0156), so whoever loaded one writes its name
/// here and this crate draws it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MapPill {
    /// What the map is called — the file's stem, which is the name an operator
    /// gave it and not its path (P-0087).
    pub name: Option<String>,
}

impl MapPill {
    /// **A surface with no map**, which draws `map · none`. It is a state and
    /// not an absence: a run that opened a port and found no file to load is
    /// one an operator can still learn into.
    pub const NONE: MapPill = MapPill { name: None };

    /// What the pill says after `map ·`.
    pub fn word(&self) -> &str {
        self.name.as_deref().unwrap_or(NO_MAP)
    }
}

/// **What the pill says where no map is loaded**, and it is deliberately not a
/// name: `none` is a word for a state, where a name would be a reading this
/// console invented. [`NO_ARRANGEMENT`]'s argument one pill along, and the
/// same shape [`audio_text`] uses for an input nobody opened.
const NO_MAP: &str = "none";

/// The `learn` pill's word, which is the whole of its label.
const LEARN_LABEL: &str = "learn";

/// The `map` pill's first word, the mock's own abbreviation — `map · <name>`.
const MAP_LABEL: &str = "map";

/// **The `learn` pill**: the capsule, and whether it is lit.
///
/// **A pill and not a capsule with two ends**, which the `rec` control is: a
/// press means *the other state* and the word never changes, because *learn*
/// is what the control is rather than what a press will do. It is `.pill.lav`
/// in the mock, lit while armed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LearnPill {
    /// **The control**: the capsule a press has to land in.
    pub pill: Rect,
    /// Whether learn is armed, read off [`View::learn`] and kept nowhere here.
    pub armed: bool,
}

impl LearnPill {
    /// Whether `p` is on the control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// **What a press asks for**, which is a state and never a direction
    /// (P-0090): the arming, the other way round.
    ///
    /// It is a `bool` and not an [`Operation`] because **a learn is not an
    /// operation** — it is a setting of the map layer every surface reaches
    /// the vocabulary through, which is `Vocabulary::Setting`'s own shape
    /// ([ADR-0236](../../../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md),
    /// and [ADR-0336](../../../../docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md)
    /// for the argument against the other reading). The four `mcp` pills are
    /// the same kind of control and carry no operation either.
    pub fn next(&self) -> bool {
        !self.armed
    }
}

/// **The `map` pill**: where the readout goes.
///
/// **No press and no menu**, which is what makes this a readout rather than
/// the control the mock draws. The mock's tip says *"Click to save, load, or
/// start a new one"*, and reaching a map while running is not built — the pill
/// says which file is loaded and nothing else. A capsule that looked like a
/// menu and opened none is the scaffolding [`transport`] refuses; a capsule
/// that says a true thing is not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapRow {
    /// The capsule, painted.
    pub pill: Rect,
}

/// **Where the `learn` pill goes**, or `None` where there is no room or
/// nothing to say.
///
/// `None` where the console has not been told about a map — `View::map` — for
/// [`audio_in`]'s reason exactly: a program with no surface has nothing to
/// learn onto, and a lit-able pill drawn for it would be this crate answering
/// a question about a device on its own authority.
///
/// `layout` must be solved.
pub fn learn_pill(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    map: Option<&MapPill>,
    armed: bool,
) -> Option<LearnPill> {
    map?;
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let text_w = width_of(ctx, LEARN_LABEL);
    let pill_w = size::PILL_PAD_X * 2.0 + text_w;
    let mid = strip.center().y;
    // **Where the group before this one ended** — the tracker group's last
    // chip, the audio-in pill, or the bar. The same one answer [`arrangement`]
    // takes, asked two items further back; the gap is the row's own.
    let after = match (
        tracker_group(ctx, layout, values, audio, tracker),
        audio_in(ctx, layout, values, audio),
    ) {
        (Some(group), _) => group.double.max.x,
        (None, Some(before)) => before.pill.max.x,
        (None, None) => row.bar.max.x,
    };
    let pill = Rect::from_min_size(
        Pos2::new(after + size::TRANSPORT_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    // [`outputs_row`]'s rule: a capsule that does not fit in its row is no
    // control at all rather than half of one over the frame readout.
    if !strip.contains_rect(pill) || pill.max.x + size::TRANSPORT_GAP > row.frame.min.x {
        return None;
    }
    Some(LearnPill { pill, armed })
}

/// **Where the `map` pill goes**, or `None` where there is no room or nothing
/// to say.
///
/// It follows [`learn_pill`], which is the mock's order — `learn`, then
/// `map · nanoKONTROL2` — and falls back through the same chain where the
/// `learn` pill did not fit, so one control dropping for want of room does not
/// take the next with it.
pub fn map_pill(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    map: Option<&MapPill>,
) -> Option<MapRow> {
    let map = map?;
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let words = format!("{MAP_LABEL} · {}", map.word());
    let pill_w = size::PILL_PAD_X * 2.0 + width_of(ctx, &words);
    let mid = strip.center().y;
    let after = match (
        learn_pill(ctx, layout, values, audio, tracker, Some(map), false),
        tracker_group(ctx, layout, values, audio, tracker),
        audio_in(ctx, layout, values, audio),
    ) {
        (Some(learn), _, _) => learn.pill.max.x,
        (None, Some(group), _) => group.double.max.x,
        (None, None, Some(before)) => before.pill.max.x,
        (None, None, None) => row.bar.max.x,
    };
    let pill = Rect::from_min_size(
        Pos2::new(after + size::TRANSPORT_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    if !strip.contains_rect(pill) || pill.max.x + size::TRANSPORT_GAP > row.frame.min.x {
        return None;
    }
    Some(MapRow { pill })
}

/// One text measurement, the way every pill in this row takes one.
fn width_of(ctx: &egui::Context, text: &str) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    })
}

// ---------------------------------------------------------------------------
// The arrangement pill
// ---------------------------------------------------------------------------

/// **The pill's first word**, the mock's own abbreviation one pill along from
/// `map · nanoKONTROL2 ▾`. The map pill names a file with *save*, *load* and
/// *start a new one* under it, and this is the same three over a different
/// file, so it is the same two-part label.
const ARRANGEMENT_LABEL: &str = "arr";

/// **What the pill says where no arrangement has been named, and it is
/// deliberately not a name.**
///
/// The default arrangement *has* no name: it reaches
/// [`crate::layout`] rather than a file, so it is the one arrangement nobody
/// could have saved and nothing filed can shadow it
/// ([ADR-0221](../../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)
/// §2). Writing `default` here would be this console inventing one — and a
/// worse invention than most, because an operator *may* save an arrangement
/// called `default` and it shadows nothing, so the pill would read the same
/// for two different states.
///
/// A space is what makes it safe as well as honest: a name is one path
/// component of letters, digits, `-` and `_`, so `the default` is not a name
/// anything can be filed under and no save can make this line ambiguous. It is
/// the preview cell's `C · no slot` one row up — a word for a state, where a name
/// would be a reading invented for a console that has none.
const NO_ARRANGEMENT: &str = "the default";

/// **What *save* is called in the menu.** The ellipsis is the one thing on
/// this panel that says *this item asks for something before it does
/// anything*, and it is only there while it is true: with a name in use,
/// saving again means that name and asks for nothing, so the word loses it.
const SAVE_ITEM: &str = "save";
const SAVE_ITEM_ASKING: &str = "save as…";

/// **What the reset is called in the menu**, in the manual's own words rather
/// than in the vocabulary's. *Reset the arrangement* is the row and
/// [`Op::Reset`] is the operation; *start a new one* is what the family reads
/// as from inside this control, where the default is *"one arrangement among
/// the ones you could name"* and not a fourth thing beside the three.
const NEW_ITEM: &str = "start a new one";

/// **What the arrangement pill reads this frame, and what its menu is doing.**
///
/// # The name and the list are handed in, for [`View::picture`]'s reason
///
/// Which arrangement is in use is *which file was last written or read*, and
/// which names exist is a directory. `src/` takes no device, no window and no
/// clock (ADR-0156) and it takes no disk either — `karakuri-console`'s
/// manifest has no entry that could reach one — so whoever owns the store
/// reads both and writes them here, exactly as whoever owns the engine writes
/// [`View::transport`]. What crosses the seam is a name and a list of names.
///
/// # The menu is not handed in, and that is the other half of the same seam
///
/// [`Menu`] is this crate's: it is what the *control* is doing, not what the
/// instrument is doing, and it moves only through the methods below. A menu
/// open in the program's memory would be the arrangement living in the
/// toolkit's memory one level along (ADR-0156's own argument), and the console
/// would then be drawing a state it could not answer questions about.
///
/// It is held here rather than in [`Panel`] because it is not part of the
/// arrangement: nothing about an open menu is saved, restored or reset, and a
/// [`Panel::restore`] that put somebody else's open menu back would be
/// restoring a gesture.
#[derive(Debug, Clone, PartialEq)]
pub struct Arrangement {
    /// **The arrangement in use**, or `None` for the default — which is not a
    /// name and is drawn as [`NO_ARRANGEMENT`].
    ///
    /// A save and a restore both put a name here, because both leave that
    /// arrangement the one in use; a reset takes it away, because the default
    /// is what is now on screen and it has no name.
    pub name: Option<String>,
    /// **Every name already filed**, in the order whoever read the store
    /// listed them — `Store::list_arrangements` sorts by name, so this is
    /// alphabetical and the menu does not sort it again.
    ///
    /// **Read when it changes rather than per frame**, which is
    /// [`View::library`]'s rule for its reason: a listing is a directory read
    /// and that is not a thing to do on a frame path (P-0091). It changes
    /// exactly when a save lands, and whoever performed the save is who
    /// re-reads it.
    ///
    /// Empty is a console with no store behind it — every test in this crate —
    /// and the menu then offers *save* and *start a new one* and lists
    /// nothing, which is honest: there is nothing to put back.
    pub filed: Vec<String>,
    /// What the control is doing. See [`Menu`].
    pub menu: Menu,
}

/// **What the pill's menu is doing**, and the console's only state that is
/// neither the arrangement nor a value handed in.
///
/// Three states rather than a `bool` and a buffer beside it: *shut*, *open*,
/// and *open with a name being typed into it*. The third is a state of the
/// menu and not a fourth thing, which is what stops a buffer being read while
/// nothing is asking for one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Menu {
    /// The pill alone.
    #[default]
    Shut,
    /// The menu is down: *save*, *start a new one*, and the names filed.
    Open,
    /// **The one flow on this panel that asks for letters**, with what has
    /// been typed so far.
    ///
    /// The buffer is a `String` this crate owns and whoever holds the keyboard
    /// fills, one character at a time, through [`Arrangement::typed`] and
    /// [`Arrangement::rubbed_out`] — the same split as everything else here,
    /// since `src/` has no key events to read (ADR-0156).
    ///
    /// **Nothing in it is checked.** A name that is not one path component is
    /// refused where the record is applied, in one sentence, by whoever writes
    /// the file — the surface owns the affordance and never the authority
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md),
    /// [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    /// A pill that quietly dropped the characters it did not like would be a
    /// rule an operator could only find by experiment.
    Naming(String),
}

impl Arrangement {
    /// **A console with no store behind it**: the default arrangement, nothing
    /// filed, and the menu shut.
    ///
    /// A `const` rather than a `Default` impl alone so that a test — and
    /// [`crate::input::claim`]'s own documentation — can name the state
    /// without building one. Every test in this crate is this.
    pub const NONE: Arrangement = Arrangement {
        name: None,
        filed: Vec::new(),
        menu: Menu::Shut,
    };

    /// **What the pill says after `arr ·`**: the name in use, or the word for
    /// the arrangement that has none. See [`NO_ARRANGEMENT`].
    pub fn word(&self) -> &str {
        self.name.as_deref().unwrap_or(NO_ARRANGEMENT)
    }

    /// The menu is down, whether or not a name is being typed into it.
    pub fn open(&self) -> bool {
        !matches!(self.menu, Menu::Shut)
    }

    /// **What has been typed so far**, or `None` when nothing is asking for a
    /// name. The caret is drawn after it and there is no selection: this is a
    /// name, not a document.
    pub fn naming(&self) -> Option<&str> {
        match &self.menu {
            Menu::Naming(typed) => Some(typed),
            _ => None,
        }
    }

    /// Put the menu down.
    pub fn opened(&mut self) {
        self.menu = Menu::Open;
    }

    /// Take it away, typed name and all. A name abandoned half-typed is not
    /// kept for the next time the menu opens: the buffer is the gesture, and
    /// the gesture ended.
    pub fn shut(&mut self) {
        self.menu = Menu::Shut;
    }

    /// **Ask for a name**, starting from empty. Reached only where there is no
    /// name in use — with one in use, *save* means that name and asks nothing
    /// ([`ArrangementPill::ask`]).
    pub fn asks_a_name(&mut self) {
        self.menu = Menu::Naming(String::new());
    }

    /// **One character into the name being typed**, and `false` where nothing
    /// was asking for one.
    ///
    /// Control characters are not a name and never reach the buffer — a
    /// newline is Return arriving as text, which is the commit and not a
    /// letter. Everything else does, unchecked, for the reason
    /// [`Menu::Naming`] gives.
    pub fn typed(&mut self, c: char) -> bool {
        match (&mut self.menu, c.is_control()) {
            (Menu::Naming(name), false) => {
                name.push(c);
                true
            }
            _ => false,
        }
    }

    /// **The last character back out again**, and `false` where there was
    /// nothing to take — no name being typed, or an empty one.
    pub fn rubbed_out(&mut self) -> bool {
        match &mut self.menu {
            Menu::Naming(name) => name.pop().is_some(),
            _ => false,
        }
    }

    /// **How many rows the open menu has**: *save*, *start a new one*, and one
    /// per name filed. Zero while the menu is shut or asking for a name, which
    /// is a menu with a field in it rather than a list.
    fn rows(&self) -> usize {
        match self.menu {
            Menu::Open => VERBS + self.filed.len(),
            _ => 0,
        }
    }
}

impl Default for Arrangement {
    fn default() -> Arrangement {
        Arrangement::NONE
    }
}

/// **The two items above the list**: *save* and *start a new one*. The names
/// filed follow them, and *load* is that list rather than an item of its own —
/// which is the manual's own sentence: *"load is a list — the names already
/// filed, which a hand can pick without typing anything."*
const VERBS: usize = 2;

/// **What a press on one of the menu's rows lands on.**
///
/// [`ArrangementPill::ask`] turns one of these into what the press *asks for*;
/// this is only which row it was. Split in two so that the hit test and the
/// operation are one derivation asked twice rather than one function that does
/// both — [`Outputs::op`]'s arrangement, over a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// *save*. With no name in use it asks for one; with a name in use it
    /// means that name.
    Save,
    /// *start a new one*, which is the reset.
    New,
    /// The name at this index of [`Arrangement::filed`] — put that
    /// arrangement back.
    Filed(usize),
}

/// **What a press on the pill or on one of its rows asks for.**
///
/// Every arm is either a move of this control's own state or one named
/// operation, and never a change to the arrangement made here: the pill asks,
/// and whoever applies the record decides
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
/// **Nothing is refused in this list.** A name nothing is filed under and a
/// file that disagrees with itself are both refused where the bytes are, in
/// one sentence each, and this control cannot see either.
#[derive(Debug, Clone, PartialEq)]
pub enum Ask {
    /// Put the menu down — a press on the pill with the menu shut.
    Open,
    /// Take it away — a press on the pill again, or anywhere off the menu
    /// while it is down.
    Shut,
    /// **Ask for a name**: *save* with no arrangement in use. The one item on
    /// this panel that asks for letters.
    Name,
    /// **The reset**, which reaches code and not a file, and is the same
    /// [`Op`] the `r` key performs — one operation, two surfaces
    /// ([ADR-0208](../../../../docs/adr/0208-resetting-is-the-default-case-of-restoring-an-arrangement.md)).
    Panel(Op),
    /// **One operation of the vocabulary, named**: a save under a name, or a
    /// restore of one. Both reach a file and only whoever holds the store can
    /// perform either.
    Operation(Operation),
}

/// **The arrangement pill, laid out**: the capsule, what is written in it, and
/// the menu under it while it is down.
///
/// # One derivation, for [`Outputs`]' reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. Two copies of the arithmetic is a menu
/// row that lights under a pointer that cannot pick it, with nothing on screen
/// saying so.
///
/// # It carries no name and no list
///
/// The rectangles are here and the words are [`Arrangement`]'s, which is why
/// [`ArrangementPill::ask`] takes one: a laid-out pill that had *copied* the
/// name it was measured from is a second copy to drift, and the caller has the
/// first one in its hand already. It is [`Mixer`]'s split with the borrow
/// turned round — the mixer keeps the strips because a knob's position *is* a
/// value, and nothing here moves with the name except the width.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrangementPill {
    /// **The capsule**, which is what a press has to land in to open the menu.
    /// The mock gives the whole pill the click and so does this.
    pub pill: Rect,
    /// Where `arr · night` is painted, inside the capsule's padding.
    pub text: Rect,
    /// The `▾` after it. Drawn rather than typed — see [`CHEVRON_W`].
    pub chevron: Rect,
    /// **The menu, or `None` while it is shut** — the whole card, which is
    /// what a press has to land in to be a pick rather than a dismissal.
    pub menu: Option<Rect>,
    /// **How many of [`Arrangement::rows`] the menu has room for**, between
    /// the pill and the bottom of the console.
    ///
    /// Fewer than there are is a store with more arrangements than the window
    /// is tall, and the foot says so in the Library bay's own words — `n of m`
    /// — rather than the list quietly ending. Zero while the menu is shut, and
    /// while it is asking for a name: that menu is a field, not a list.
    pub rows: usize,
    /// **What the menu would list if the window were tall enough**, carried so
    /// that the foot and the rows are one number rather than two.
    pub of: usize,
}

impl ArrangementPill {
    /// Whether `p` is on the pill itself.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// **Whether `p` is anywhere this control owns** — the pill, or the menu
    /// while it is down. This is what [`crate::input::claim`] asks, and it is
    /// wider than [`ArrangementPill::hit`] by exactly the open menu.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        let at = Pos2::new(p.x, p.y);
        self.pill.contains(at) || self.menu.is_some_and(|menu| menu.contains(at))
    }

    /// **Where one row of the menu is**, from the top. The rows stack with no
    /// gap between them, which is `.lib-list`'s own reading — the one list in
    /// the mock that has none.
    ///
    /// Panics on a row this menu has not got, which is [`TransportRow::dot`]'s
    /// rule: a caller has invented an item.
    pub fn row(&self, index: usize) -> Rect {
        assert!(index < self.rows, "row {index} of a menu of {}", self.rows);
        let menu = self.menu.expect("a menu with rows in it");
        let top = menu.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32
            // The rule under the two verbs, which the names sit below.
            + match index >= VERBS {
                true => size::HAIRLINE,
                false => 0.0,
            };
        Rect::from_min_size(
            Pos2::new(menu.min.x + size::LIB_LIST_PAD, top),
            egui::vec2(menu.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        )
    }

    /// **Which item `p` is on**, or `None` for a point on no row — the menu's
    /// padding, its foot, or anywhere off it.
    pub fn item(&self, p: karakuri_layout::Point) -> Option<Item> {
        let at = Pos2::new(p.x, p.y);
        (0..self.rows)
            .find(|index| self.row(*index).contains(at))
            .map(|index| match index {
                0 => Item::Save,
                1 => Item::New,
                other => Item::Filed(other - VERBS),
            })
    }

    /// **What a press at `p` asks for**, or `None` where the press was on
    /// nothing this control owns.
    ///
    /// The same derivation [`crate::input::claim`] hit-tests, asked a second
    /// time rather than copied — [`Outputs::op`]'s arrangement, and the reason
    /// is the same: the pill that claims a press and the pill that acts on it
    /// cannot come apart.
    ///
    /// # The three answers a press on a row can give
    ///
    /// - **Save** with a name in use is [`Operation::SaveArrangement`] naming
    ///   it. *"Once a name is in use, saving again means that name: saving
    ///   over it is what saving it again is."* With no name in use there is
    ///   nothing to save over, so it asks for one instead.
    /// - **Start a new one** is [`Op::Reset`] — the same operation `r`
    ///   performs, reached from the other end of the panel exactly as the
    ///   Outputs row's dot reaches `f`'s fold.
    /// - **A name** is [`Operation::RestoreArrangement`] naming it, which is
    ///   the reset's own sentence with a name in it.
    ///
    /// A press on the pill toggles the menu; a press anywhere else on the menu
    /// — its padding, its foot — shuts it, because a press that did nothing at
    /// all is the one thing worse than a press that declines.
    pub fn ask(&self, arr: &Arrangement, p: karakuri_layout::Point) -> Option<Ask> {
        if self.hit(p) {
            return Some(match arr.open() {
                true => Ask::Shut,
                false => Ask::Open,
            });
        }
        let menu = self.menu?;
        if !menu.contains(Pos2::new(p.x, p.y)) {
            return None;
        }
        Some(match self.item(p) {
            Some(Item::Save) => match &arr.name {
                Some(name) => Ask::Operation(Operation::SaveArrangement { name: name.clone() }),
                None => Ask::Name,
            },
            Some(Item::New) => Ask::Panel(Op::Reset),
            Some(Item::Filed(index)) => match arr.filed.get(index) {
                Some(name) => Ask::Operation(Operation::RestoreArrangement { name: name.clone() }),
                // A row past the end of the list, which `row` has already
                // refused to produce a rectangle for. Unreachable rather than
                // guessed at.
                None => Ask::Shut,
            },
            None => Ask::Shut,
        })
    }
}

/// **The arrangement pill's furniture, derived**: the capsule, the words in
/// it, and the menu under it.
///
/// # Where it sits, and why it is after the bar
///
/// `docs/manual/console.html`'s `.transport` is a flex row and this pill is
/// the last item in it before the `.sep`, immediately after
/// `map · nanoKONTROL2 ▾`. Everything the mock draws between the tracker group
/// and this pill — `learn`, and then `map` — is one of the controls
/// [`transport`] names and does not draw, so a flex row closes up and this
/// lands one [`size::TRANSPORT_GAP`] after the octave's second half — or after
/// the audio-in pill on a console with no tracker behind it, or after
/// `bar 37` on one with neither. That is the mock's own layout with the
/// undrawn items taken out, and not a position chosen here.
///
/// **It moves with the tempo, by a glyph or two.** `92.5` is narrower than
/// `128.0` and everything after it slides, which is what a flex row is and
/// what the beat grid and the bar already do. The alternative — pinning it to
/// the right edge, where the mock's `landed` and `rec` sit — buys a control
/// that never moves and puts it in the group the manual does not put it in.
///
/// # No row means no pill, and that is the row's answer rather than a second
/// one
///
/// `None` wherever [`transport`] answers `None`: the row folded away, soloed
/// away, too narrow, or a console with no engine behind it. The last is the
/// one worth stating, because the arrangement exists whether or not a tempo
/// does — but the *row* does not, and a pill floating in a bay that is drawing
/// nothing at all would be a control in a row that is not there. Where the row
/// is, this asks it for the bar's right edge and lays out from there, so the
/// pill's place and the readouts' places are one derivation.
///
/// # What it costs to ask
///
/// One galley lookup for the pill's own words, always. **While the menu is
/// down it is one more per row** — the two verbs and every name filed — since
/// the card is as wide as the widest thing in it, and a name this crate never
/// measured would be a name drawn outside its own card. Paid on a pointer
/// event and on a frame, and only while the menu is open, which is a menu an
/// operator is looking at.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
pub fn arrangement(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    // **The map, only so this pill knows where the group before it ended.**
    // `learn` and `map` sit between the tracker group and this one in the
    // mock, and a console that has not been told about a surface draws
    // neither — so this is `None` on every run without one and the pill lands
    // exactly where it did before they existed.
    map: Option<&MapPill>,
    arr: &Arrangement,
) -> Option<ArrangementPill> {
    // The row, asked once and for everything: whether there is one at all,
    // and where the bar ended. `transport` answers the first for the whole
    // module, which is what keeps *no engine means nothing at all* in one
    // place.
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let width = |text: &str| {
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

    let words = pill_text(arr);
    let text_w = width(&words);
    // `.pill`'s `padding: 0 8px` around the words, one `.sink` gap, and the
    // chevron — the one gap the mock states inside a capsule.
    let pill_w = size::PILL_PAD_X * 2.0 + text_w + size::SINK_GAP + CHEVRON_W;
    let mid = strip.center().y;
    // **Where the group before this one ended**, which is the tracker group's
    // last chip where the console has been told about the tracker, the
    // audio-in pill where it has been told only about audio, and the bar where
    // it has been told neither — the same one-answer arrangement [`look`]
    // takes of *this* pill, asked one item further back. The gap is the row's
    // own either way: what ends before this pill is a whole group, and
    // `.tracker`'s tighter 5 is the gap *inside* that group.
    let after = match (
        map_pill(ctx, layout, values, audio, tracker, map),
        learn_pill(ctx, layout, values, audio, tracker, map, false),
        tracker_group(ctx, layout, values, audio, tracker),
        audio_in(ctx, layout, values, audio),
    ) {
        (Some(map), _, _, _) => map.pill.max.x,
        (None, Some(learn), _, _) => learn.pill.max.x,
        (None, None, Some(group), _) => group.double.max.x,
        (None, None, None, Some(before)) => before.pill.max.x,
        (None, None, None, None) => row.bar.max.x,
    };
    let pill = Rect::from_min_size(
        Pos2::new(after + size::TRANSPORT_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    // The same rule [`outputs_row`] states: a capsule that does not fit in the
    // row it is drawn in is no control at all, rather than half of one over
    // the frame readout.
    if !strip.contains_rect(pill) || pill.max.x + size::TRANSPORT_GAP > row.frame.min.x {
        return None;
    }
    let text = Rect::from_min_size(
        Pos2::new(pill.min.x + size::PILL_PAD_X, mid - size::PILL_H * 0.5),
        egui::vec2(text_w, size::PILL_H),
    );
    let chevron = Rect::from_center_size(
        Pos2::new(pill.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5, mid),
        egui::vec2(CHEVRON_W, CHEVRON_H),
    );

    let (menu, rows, of) = menu_card(&pill, layout, arr, &width);
    Some(ArrangementPill {
        pill,
        text,
        chevron,
        menu,
        rows,
        of,
    })
}

/// **The pill's words**: `arr · night`, or `arr · the default`.
///
/// One string rather than three galleys laid end to end, for [`frame_job`]'s
/// reason: the mock writes one run of text and laying it out as one keeps the
/// spaces round the `·` the type's own rather than a gap this file invented.
fn pill_text(arr: &Arrangement) -> String {
    format!("{ARRANGEMENT_LABEL} · {}", arr.word())
}

/// **The menu card under the pill**: where it is, how many rows fit in it, and
/// how many there are.
///
/// `(None, 0, 0)` while the menu is shut, which is the ordinary state and
/// costs one branch.
///
/// # It hangs from the pill and is held inside the console
///
/// Down from the pill's bottom edge by one [`size::PILL_GAP`], left-aligned
/// with it, and pushed back inside the viewport's right edge where a long name
/// would take it past — a card half outside the window is a list with items
/// nobody can read.
///
/// **The bottom is a count and not a clip.** The transport row is at the top
/// of the console, so a menu hanging down has the whole window; where a store
/// holds more arrangements than that window is tall, the card lists as many as
/// fit and says `n of m` in the Library bay's own foot. Truncating in silence
/// is the failure P-0094 is about, and this is that bay's answer to the same
/// question rather than a second one.
fn menu_card(
    pill: &Rect,
    layout: &karakuri_layout::Layout,
    arr: &Arrangement,
    width: &dyn Fn(&str) -> f32,
) -> (Option<Rect>, usize, usize) {
    if !arr.open() {
        return (None, 0, 0);
    }
    let viewport = to_egui(layout.viewport());
    let top = pill.max.y + size::PILL_GAP;

    // **Asking for a name is a field and not a list**, so the card is one row
    // wide enough to type into and there is nothing to pick.
    if let Some(typed) = arr.naming() {
        let field = width(&naming_text(typed)).max(pill.width());
        let card = held_inside(
            &viewport,
            pill.min.x,
            top,
            field + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
            size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H,
        );
        return (Some(card), 0, 0);
    }

    let of = arr.rows();
    let widest = std::iter::once(save_word(arr))
        .chain(std::iter::once(NEW_ITEM))
        .chain(arr.filed.iter().map(String::as_str))
        .map(width)
        .fold(pill.width(), f32::max);
    // How many rows there is room for between the card's top and the bottom of
    // the console, once the padding and the rule under the verbs are paid for.
    // The foot is only owed where something is left out, so it is asked for
    // twice: once assuming it is not there and once assuming it is.
    let furniture = size::LIB_LIST_PAD * 2.0 + size::HAIRLINE;
    let room = |foot: f32| {
        (((viewport.max.y - top - furniture - foot) / size::LIB_ROW_H).floor()).max(0.0) as usize
    };
    let rows = match room(0.0) >= of {
        true => of,
        false => room(size::LIB_FOOT_H).min(of),
    };
    let foot = match rows < of {
        true => size::LIB_FOOT_H,
        false => 0.0,
    };
    let card = held_inside(
        &viewport,
        pill.min.x,
        top,
        widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
        furniture + size::LIB_ROW_H * rows as f32 + foot,
    );
    (Some(card), rows, of)
}

/// **What *save* is called with this arrangement in use.** The ellipsis is
/// there exactly while the item asks for something — see [`SAVE_ITEM`].
fn save_word(arr: &Arrangement) -> &'static str {
    match arr.name {
        Some(_) => SAVE_ITEM,
        None => SAVE_ITEM_ASKING,
    }
}

/// **The name being typed, with the caret after it** — `night▏`.
///
/// One run of text rather than a galley and a drawn bar, so that measuring the
/// field and painting it cannot be two different runs: the caret is the width
/// of the caret whatever the face is, and a field measured without it would
/// put the caret outside its own card at the moment the name filled it.
fn naming_text(typed: &str) -> String {
    format!("{typed}{CARET}")
}

/// **The arrangement pill, painted**, and the menu under it.
///
/// Where everything goes is [`arrangement`]'s, so this paints and derives
/// nothing. Term for term from `.pill` in `style.css`:
///
/// - `border: 1px solid var(--c-line); border-radius: 999px; padding: 0 8px;
///   color: var(--c-dim)` — a capsule with a hairline round it, which is
///   [`pill_at`]'s treatment and this is the same pill in another row.
/// - the `▾` — `--c-dim` with the words, since it is part of the same run in
///   the mock's markup.
///
/// The menu has no term in the stylesheet, because the mock draws no menu: it
/// is the Library bay's list, which is the one list this console already
/// draws, at the same `.lib-row` box and inside the same `.lib-list` padding
/// (P-0085). It sits on a card with the panel's own shadow under it, which is
/// what says it is above the bays rather than inside one.
/// **The `learn` pill, painted** — lavender while armed, the row's own outline
/// while it is not.
///
/// `.pill.lav` is the mock's class on this control and the palette's `lav` is
/// that colour, so *armed* is drawn in the ink the page already gave it rather
/// than in one this file chose. The word never changes: *learn* is what the
/// control **is**, and what a press will do is said by the lamp — which is the
/// distinction the `rec` capsule draws the other way, being one control with
/// two ends.
pub(super) fn learn_into(ui: &Ui, pal: &Palette, pill: &LearnPill) {
    let painter = ui.painter();
    let radius = CornerRadius::same((size::PILL_H * 0.5) as u8);
    let ink = match pill.armed {
        true => pal.lav,
        false => pal.dim,
    };
    if pill.armed {
        painter.rect_filled(pill.pill, radius, pal.tint);
    }
    painter.rect_stroke(
        pill.pill,
        radius,
        Stroke::new(size::HAIRLINE, ink),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        LEARN_LABEL.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        ink,
    );
    painter.galley(
        Pos2::new(
            pill.pill.min.x + size::PILL_PAD_X,
            pill.pill.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );
}

/// **The `map` pill, painted** — `map · default`, and no chevron.
///
/// **The chevron is the one thing left off the mock's own drawing**, and it is
/// left off on purpose: `▾` on this console means *there is a menu under
/// this*, which the arrangement pill and the `audio-in` pill both keep, and
/// there is no menu here. Drawing one over a readout would be the scaffolding
/// [`transport`] refuses — a control that looks like a control and does
/// nothing.
pub(super) fn map_into(ui: &Ui, pal: &Palette, pill: &MapRow, map: &MapPill) {
    let painter = ui.painter();
    painter.rect_stroke(
        pill.pill,
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        format!("{MAP_LABEL} · {}", map.word()),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.dim,
    );
    painter.galley(
        Pos2::new(
            pill.pill.min.x + size::PILL_PAD_X,
            pill.pill.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.dim,
    );
}

pub(super) fn arrangement_into(ui: &Ui, pal: &Palette, pill: &ArrangementPill, arr: &Arrangement) {
    let painter = ui.painter();
    painter.rect_stroke(
        pill.pill,
        // `border-radius: 999px` on a box this short is a capsule.
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        pill_text(arr),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.dim,
    );
    painter.galley(
        Pos2::new(
            pill.text.min.x,
            pill.text.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.dim,
    );
    // The `▾`: a triangle with its point down, in the box `arrangement`
    // measured for it.
    painter.add(egui::Shape::convex_polygon(
        vec![
            pill.chevron.left_top(),
            pill.chevron.right_top(),
            Pos2::new(pill.chevron.center().x, pill.chevron.max.y),
        ],
        pal.dim,
        Stroke::NONE,
    ));

    let Some(card) = pill.menu else {
        return;
    };
    painter.add(pal.shadow.as_shape(card, CornerRadius::same(8)));
    painter.rect_filled(card, CornerRadius::same(8), pal.panel);
    painter.rect_stroke(
        card,
        CornerRadius::same(8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );

    let row_text = |painter: &egui::Painter, rect: Rect, text: String, colour: Color32| {
        let galley = painter.layout_no_wrap(
            text,
            FontId::new(size::BASE, FontFamily::Proportional),
            colour,
        );
        painter.galley(
            Pos2::new(
                rect.min.x + size::LIB_ROW_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            colour,
        );
    };

    // **The field, where a name is being asked for.** The card has one row in
    // it and `pill.rows` is zero, which is what stops `row` handing out a
    // rectangle for something that is not a list.
    if let Some(typed) = arr.naming() {
        let field = Rect::from_min_size(
            Pos2::new(
                card.min.x + size::LIB_LIST_PAD,
                card.min.y + size::LIB_LIST_PAD,
            ),
            egui::vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        );
        row_text(painter, field, naming_text(typed), pal.text);
        return;
    }

    for index in 0..pill.rows {
        let rect = pill.row(index);
        let (text, colour) = match index {
            0 => (save_word(arr).to_owned(), pal.text),
            1 => (NEW_ITEM.to_owned(), pal.text),
            other => (arr.filed[other - VERBS].clone(), pal.dim),
        };
        row_text(painter, rect, text, colour);
    }
    // The rule under the two verbs, which is what makes the names below it a
    // list rather than two more items.
    if pill.rows > VERBS {
        let y = pill.row(VERBS).min.y - size::HAIRLINE * 0.5;
        painter.line_segment(
            [
                Pos2::new(card.min.x + size::LIB_LIST_PAD, y),
                Pos2::new(card.max.x - size::LIB_LIST_PAD, y),
            ],
            Stroke::new(size::HAIRLINE, pal.hair),
        );
    }
    // **`n of m`, in the Library bay's own words**, and only where the list
    // could not be shown whole.
    if pill.rows < pill.of {
        let foot = Rect::from_min_max(
            Pos2::new(card.min.x, card.max.y - size::LIB_FOOT_H),
            card.max,
        );
        let galley = painter.layout_no_wrap(
            format!("{} of {}", pill.rows.saturating_sub(VERBS), pill.of - VERBS),
            FontId::new(size::LIB_FOOT_SIZE, FontFamily::Proportional),
            pal.faint,
        );
        painter.line_segment(
            [
                Pos2::new(foot.min.x, foot.min.y),
                Pos2::new(foot.max.x, foot.min.y),
            ],
            Stroke::new(size::HAIRLINE, pal.hair),
        );
        painter.galley(
            Pos2::new(
                foot.min.x + size::LIB_FOOT_PAD_X,
                foot.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.faint,
        );
    }
}

// ---------------------------------------------------------------------------
// The look: the tone map, and the level going into it
// ---------------------------------------------------------------------------

/// **The tone map pill's first word**, the mock's own two-part label one pill
/// along from `arr · night ▾` — and **no chevron**, which is what says this one
/// acts on the press instead of putting a card down. The two pills that open a
/// menu carry the mark and the three that do not (`learn`, `tap` — drawn since
/// 2026-09-08, and [`TAP_LABEL`] — and this) do not.
const TONEMAP_LABEL: &str = "tone";

/// **The word before the exposure track** — `.trim`'s `g` one row up, and the
/// same arrangement: a faint label, [`size::TRIM_GAP`], a horizontal
/// [`fader`].
const EXPOSURE_LABEL: &str = "exp";

/// **Every tone map operator, in the order this control walks them.**
///
/// `karakuri_operation::Tonemap` has no `ALL` — nothing had read one until now
/// — so unlike the blend chip's cycle this is **not** a second copy of a list
/// the vocabulary keeps ([`after`] says what that costs). It is the only
/// statement of the order in this workspace apart from `karakuri-cli`'s own
/// `next_tonemap`, which walks the engine's `TonemapOp` in exactly this order,
/// and it is what [`next_tonemap`] is checked against.
///
/// The order is the engine's own declaration order, which is the order
/// [ADR-0037](../../../../docs/adr/0037-tone-mapping-is-a-uniform-and-the-default-is-chosen-by-looking.md)
/// compared them in: `Clamp` first, because it is *"not a tone mapper — the
/// control that shows what the other three are fixing"* and is where a
/// `Present` that nobody has written to starts. A cycle should start where the
/// thing it cycles starts, which is [`next_shape`]'s own reason.
const TONEMAPS: [Tonemap; 4] = [
    Tonemap::Clamp,
    Tonemap::Reinhard,
    Tonemap::Aces,
    Tonemap::AgX,
];

/// **How many stops of exposure the track spans**, and the whole of what
/// decides its two ends: 1.0 sits at the middle, so the track runs from
/// `2^-6` to `2^6` — a sixty-fourth to sixty-four.
///
/// It is `karakuri-cli`'s own interactive range, restated rather than shared
/// because `EXPOSURE_MIN` and `EXPOSURE_MAX` are private to that binary and
/// this crate depends on nothing that could reach them. What is stated there
/// is *"the interactive bounds, and only the interactive ones"* — the bounds a
/// key nudges inside, where `--exposure` is deliberately not held to them
/// because *"a batch render asks for something extreme on purpose"*. A control
/// under a hand is exactly the interactive case, so this is the same span for
/// the same reason.
const EXPOSURE_STOPS: f32 = 12.0;

/// The two ends of that span, which is [`exposure_at`] at each end of the
/// track. Written out so a reader meets the numbers rather than an exponent,
/// and asserted against the derivation in `tests/look.rs`.
pub const EXPOSURE_MIN: f32 = 1.0 / 64.0;
pub const EXPOSURE_MAX: f32 = 64.0;

/// **How many presses of an exposure key cross the whole span.**
///
/// `karakuri-cli`'s `EXPOSURE_STEP` is `1.189_207`, which is `2^(1/4)`, and it
/// says why it is a ratio rather than an addition: *"a stop is a ratio, and an
/// additive step would be enormous at 0.1 and invisible at 8.0. This is a
/// quarter of a stop, near enough."* Four presses to a stop over
/// [`EXPOSURE_STOPS`] stops is forty-eight.
const EXPOSURE_PRESSES: f32 = EXPOSURE_STOPS * 4.0;

/// **The track, in pixels: one pixel a press.**
///
/// This is the number that makes the two surfaces agree instead of nearly
/// agreeing. A pointer on this track can ask for any of the 48 positions along
/// it and a keyboard stepping a quarter stop at a time can ask for any of the
/// 48 values between the ends, so **neither surface can reach a value the
/// other cannot** — and one pixel is the coarsest step that still reads as a
/// movement rather than as a sequence of positions, which is
/// [`BEAT_STALENESS`]' own rule about the beat's travel
/// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
///
/// A width picked for looks would have been a number with nothing behind it,
/// which is what the mock's `width: 48px` says here as well.
pub const EXPOSURE_TRACK_W: f32 = EXPOSURE_PRESSES;

/// **What the look controls read this frame**: the operator that is running,
/// and the level going into it.
///
/// # The same seam as [`View::transport`], and it is not the Master bay's `out`
///
/// The look is the engine's — `karakuri_engine::frame::Look`, which is exactly
/// what `Present::set_tonemap` is told — and `src/` has no engine (ADR-0156),
/// so whoever owns one reads it and writes this per frame.
///
/// **`white_point` is not here**, and it is not an oversight: it is in the
/// record, it is Reinhard's parameter alone, and no surface has a control for
/// it, so a field here would be a value this crate carries and no control
/// names. That is the vocabulary's own line — `Operation::SetExposure` asks
/// for the level and nothing else, and whoever writes the record fills the
/// other two thirds in from the look that is running
/// ([ADR-0192](../../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)).
///
/// **And `exposure` here is the tone mapper's, never `Deck::set_out`.** The
/// two are levels that multiply in different places — the master out where the
/// mix writes the composited frame, this where the present pass reads it — and
/// with nothing in the master chain they are indistinguishable in every frame
/// this program can draw
/// ([ADR-0224](../../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md)).
/// The Master bay's row is the other one and nothing can move it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    /// The transfer that is running. One of [`TONEMAPS`].
    pub tonemap: Tonemap,
    /// The level going into it. Unbounded on the way **in** — a `--exposure`
    /// flag is not held to the track's ends — and drawn wherever
    /// [`unit_of`] puts it, which is at an end for anything past one.
    pub exposure: f32,
}

/// **The two look controls, laid out**: the capsule that names the operator,
/// and the track that sets the level.
///
/// # One derivation, for [`Outputs`]' reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. Two copies of the arithmetic is a track
/// that lights under a pointer that cannot set it.
///
/// # Two controls and one type, because they are one group in the row
///
/// They are two operations and two presses
/// ([ADR-0192](../../../../docs/adr/0192-an-operation-asks-for-what-a-surface-can-say-and-the-record-stays-whole.md)),
/// and they are laid out together because the second's place is measured from
/// the first's — which is what a flex row is. Splitting them into two
/// functions would mean measuring the capsule twice, once to draw it and once
/// to find out where the track starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LookRow {
    /// **The capsule**, which is what a press has to land in to move the tone
    /// map on.
    ///
    /// **As wide as the widest of the four names whatever word is in it**,
    /// which is [`StripBox::tally`]'s rule and not the blend chip's: a target
    /// sized to the word it is showing moves under the hand that is pressing
    /// it, and here it would take the exposure track with it.
    pub tone: Rect,
    /// Where `tone · aces` is painted, inside the capsule's padding. Narrower
    /// than the capsule for every operator but the widest, so the words are
    /// centred in it.
    pub tone_text: Rect,
    /// The faint `exp` before the track.
    pub label: Rect,
    /// **The track**, `.fader`'s 5px well lying down at
    /// [`EXPOSURE_TRACK_W`] long.
    pub track: Rect,
    /// What the level fills of it, from the left — [`unit_of`] of the
    /// exposure.
    pub fill: Rect,
    /// **What a press has to land in to set the exposure**: the track grown to
    /// a line's height and no wider.
    ///
    /// Grown, because a 5px-tall target is not something a hand finds — the
    /// `.mini`'s own argument (*"a 15px word is not something a hand finds,
    /// and `.mini`'s padding is what makes it one"*), met here by a band
    /// rather than by drawing a fatter track. **No wider**, because the value
    /// a press asks for is where along the track it landed, and a target that
    /// reached past either end would have two pixels of itself asking for the
    /// same value.
    pub grip: Rect,
    /// `1.00`, after the track. See [`look`] for why it is after it.
    pub value: Rect,
    /// **The values these rectangles were measured from**, carried for
    /// [`TransportRow::values`]' reason: whoever measured the type and whoever
    /// paints it are one statement.
    pub values: Look,
}

impl LookRow {
    /// Whether `p` is on the tone map's capsule.
    pub fn hit_tone(&self, p: karakuri_layout::Point) -> bool {
        self.tone.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is on the exposure track's band. See [`LookRow::grip`].
    pub fn hit_exposure(&self, p: karakuri_layout::Point) -> bool {
        self.grip.contains(Pos2::new(p.x, p.y))
    }

    /// **Whether `p` is on either of the two**, which is what
    /// [`crate::input::claim`] asks.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.hit_tone(p) || self.hit_exposure(p)
    }

    /// **What a press at `p` asks the tone map to become**, or `None` where
    /// there is no capsule under it.
    ///
    /// # The capsule cycles, and the operation names where it arrived
    ///
    /// Click it and the look moves to the next of [`TONEMAPS`] — `clamp`,
    /// `reinhard`, `aces`, `agx`, wrapping — and what comes out is
    /// [`Operation::SetTonemap`] naming the **destination**, never a step,
    /// because there is no step in the vocabulary to name. The affordance is
    /// [`Mixer::blend`]'s exactly
    /// ([ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md))
    /// and so is the division it rests on: the cycle is [`next_tonemap`] here
    /// and nothing at all in `karakuri-operation`, which is P-0090's
    /// division: a toggle is an affordance, built over operations by whoever
    /// draws the control.
    ///
    /// **What a map is offered is the four values, not the cycle** — the same
    /// sentence ADR-0187 wrote about three, and the reason the manual's row
    /// says a map or a model *"would name the one it wants"*.
    ///
    /// # Why not a menu, now that there is one
    ///
    /// ADR-0187 could not have a popup: the blend chip is in a 53-wide strip,
    /// and a card had nowhere to be and no rule to be modal under. Both of
    /// those changed
    /// ([ADR-0225](../../../../docs/adr/0225-a-menu-is-a-gesture-in-hand-rather-than-a-rectangle-on-the-panel.md)),
    /// so the alternative is genuinely available here and it is still refused.
    /// A menu is **a gesture in hand**: it takes every pointer event on the
    /// console until it is shut, and a boundary cannot be dragged while it is
    /// down. That is a price worth paying for a list of files an operator
    /// named and did not pay for four words that never change — and it puts a
    /// second press in front of a control that is pressed on the beat.
    pub fn tonemap(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_tone(p).then(|| Operation::SetTonemap {
            tonemap: next_tonemap(self.values.tonemap),
        })
    }

    /// **What a press at `p` asks the exposure to become**, or `None` where
    /// there is no track under it.
    ///
    /// # The press sets it outright, and that is the decision
    ///
    /// Where along the track the press landed *is* the value, through
    /// [`exposure_at`], and what comes out is [`Operation::SetExposure`]
    /// naming it. The manual is where the words for that come from — *"the
    /// value is absolute and the keys are the nudge … because a control that
    /// could only be nudged is a control no fader can reach"* — and the whole
    /// span is reachable in one press.
    ///
    /// **It is not [`Mixer::grab`], and the difference is not an
    /// inconsistency.** A fader answers `None` for a press on its *track*, on
    /// purpose: it has a knob, the knob keeps the offset it was taken hold of
    /// at, and a press that jumped the value would move the mix under a hand
    /// mid-gesture. There is no knob here and no gesture to be mid-way
    /// through — the press is the whole of it — so the two rules are answers
    /// to two different questions rather than one rule applied twice. Nothing
    /// is drawn that looks like a handle, for the same reason.
    ///
    /// **[`crate::input`]'s *a control claims what it acts on and no more* is
    /// kept exactly.** Every point of [`LookRow::grip`] asks for a value, so
    /// nothing is claimed in order to be thrown away.
    ///
    /// # It says one thing per press, so it needs no coalescing
    ///
    /// [ADR-0207](../../../../docs/adr/0207-a-continuous-control-says-one-thing-per-frame.md)
    /// coalesces a swept MIDI fader to one operation a frame, and names the
    /// console's own fader as the other precedent — which emits only when the
    /// value changed, because it holds the value it last drew. Neither applies
    /// to a press: one press is one operation, and a second press in the same
    /// frame is a second thing an operator asked for.
    ///
    /// # And it can never be pending, which is why nothing marks a destination
    ///
    /// [ADR-0206](../../../../docs/adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md)
    /// gives a fader a mark for where a scheduled move is going.
    /// `karakuri_engine::deck::Control` is **per slot** — a gain, an opacity
    /// and a mask front — so there is no transition that can be scheduled on
    /// the look at all, and a mark here would be a presentation of a state
    /// that cannot occur.
    pub fn exposure(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_exposure(p).then(|| Operation::SetExposure {
            exposure: exposure_at((p.x - self.track.min.x) / self.track.width()),
        })
    }
}

/// **The next tone map round the cycle**, wrapping from the last back to the
/// first — the whole of the affordance the capsule is.
///
/// [`after`]'s division, one row up: the cycle is four lines here and nothing
/// in `karakuri-operation`, which owns the four operators and not the order a
/// pointer walks them in (P-0090).
///
/// **A match rather than an index into [`TONEMAPS`]**, for [`after`]'s reason:
/// a fifth operator does not compile until somebody says what follows it. The
/// price is that the order is written twice — here and in [`TONEMAPS`] — so
/// `tests/look.rs` walks the array through this and asserts they are the same
/// cycle, which is `tests/blend.rs`'s own measurement.
///
/// **`karakuri-cli` has this function over the engine's `TonemapOp`** and in
/// this order. Two crates naming the same order is what P-0090 costs a
/// vocabulary that depends on nothing, and the test above is what stops the
/// two drifting on this side.
pub(crate) fn next_tonemap(tonemap: Tonemap) -> Tonemap {
    match tonemap {
        Tonemap::Clamp => Tonemap::Reinhard,
        Tonemap::Reinhard => Tonemap::Aces,
        Tonemap::Aces => Tonemap::AgX,
        Tonemap::AgX => Tonemap::Clamp,
    }
}

/// **What a point `unit` of the way along the track asks for**, in exposure.
///
/// `2^((unit − ½) · stops)`: the middle of the track is `2^0`, which is
/// **exactly 1.0**, and the two ends are exactly [`EXPOSURE_MIN`] and
/// [`EXPOSURE_MAX`] — the subtraction is exact at a half and `exp2` of a whole
/// number is a power of two with no rounding in it.
///
/// # Logarithmic, and that is not a preference
///
/// Exposure is a ratio: `karakuri-cli`'s step is a *factor* and says why —
/// *"an additive step would be enormous at 0.1 and invisible at 8.0"*. A
/// linear track is that same mistake drawn in space rather than in time: over
/// [`EXPOSURE_MIN`] to [`EXPOSURE_MAX`] unity would sit **0.75 of a pixel**
/// from the left-hand end of [`EXPOSURE_TRACK_W`], so the half of the range
/// that darkens the picture would be one pixel wide and the other half would
/// be the whole control. Equal distances along this track are equal ratios,
/// which is the only reading under which both halves are usable.
pub fn exposure_at(unit: f32) -> f32 {
    ((crate::panel::unit(unit) - 0.5) * EXPOSURE_STOPS).exp2()
}

/// **Where an exposure sits on the track**, on `[0, 1]` — [`exposure_at`]
/// inverted, and clamped to the ends.
///
/// **A value past either end is drawn at that end**, which is deliberate and
/// is stated rather than hidden: `--exposure 200` is accepted unclamped by the
/// flag on purpose, and a fill that ran off the track would be a picture of a
/// value nobody could point at. The figure beside the track is what says the
/// number in that case, which is [`LookRow::value`]'s whole job. Zero and
/// negative — which the flag refuses everywhere — take `log2` to negative
/// infinity and land at the floor rather than anywhere surprising, and a NaN
/// is zero for [`crate::panel::unit`]'s reason.
pub fn unit_of(exposure: f32) -> f32 {
    crate::panel::unit(exposure.log2() / EXPOSURE_STOPS + 0.5)
}

/// **The look controls, derived**: the capsule that names the operator, the
/// track that sets the level, and the figure beside it.
///
/// # Where they sit, and why it is after the arrangement pill
///
/// `docs/manual/console.html`'s `.transport` is a flex row and these are the
/// last two items in it before the `.sep`, immediately after `arr · night ▾`.
/// The page's own argument for the place is that this row is already four
/// groups with a reason each — the clock, the four things that need an audio
/// input, the three that say what this console is *set up* as, and health past
/// the spacer — and a look is none of them: it needs no input, it is no file,
/// and it is played rather than arranged. So it is a fourth group at the end
/// of the left half, and the row then reads in the order a frame does, since
/// the tone map is the one transfer the pipeline ends in, applied immediately
/// before the single encode at the output
/// ([P-0064](../../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md),
/// [ADR-0037](../../../../docs/adr/0037-tone-mapping-is-a-uniform-and-the-default-is-chosen-by-looking.md)).
///
/// **Nothing already drawn moves**, which was a consequence rather than the
/// reason and is worth reading with its date on it: this said [`arrangement`]
/// was *"still one [`size::TRANSPORT_GAP`] after the bar"*, and it is not, as
/// of 2026-09-08 — [`tracker_group`] draws three of the items the mock puts
/// between the two, and the pill is laid out from the last of them. What has
/// not changed is that this group is measured from the pill's right edge, so
/// where it sits is still one answer read one item further along.
///
/// # The order inside the group, and where the figure goes
///
/// The operator first and its level second, which is `Record::Look`'s own
/// order and the manual's row order — the exposure is *the level going into
/// that transfer*, so it reads as belonging to the thing on its left.
///
/// **The figure is after the track**, and that is the one piece of this layout
/// decided by what a hand does rather than by what the mock draws. It is the
/// only part of the group whose width moves with its value, so putting it last
/// leaves the capsule and the track exactly where they were: a target that
/// walked away from the pointer as it was set would be worst at the moment it
/// was being used most precisely. The capsule ahead of it is held to the
/// widest of the four names for the same reason
/// ([`LookRow::tone`]).
///
/// # No row and no pill means none of this
///
/// `None` wherever [`arrangement`] answers `None` — which is wherever
/// [`transport`] does — and `None` for a console with no engine behind it,
/// which is every test in this crate that does not hand a look in. The place
/// is measured from the pill's right edge, so a console that cannot draw the
/// pill cannot draw what comes after it either; that is one answer to *does
/// this row fit* rather than a second one.
///
/// # What it costs to ask
///
/// **Six galley lookups of its own**: the four operator names, because the
/// capsule is as wide as the widest of them; the `exp` label; and the figure.
///
/// **And it re-derives the row and the pill**, which is the honest cost of
/// being laid out from something else's right edge rather than from the row's:
/// [`transport`] is asked twice more per frame and [`arrangement`] once more,
/// where a paint-as-you-go pass that carried a cursor along the row would ask
/// each once. That is deliberate and it is [`Outputs`]' rule — the derivation
/// that draws a control is the one that hit-tests it, so a control cannot be
/// painted anywhere a press cannot reach — and it is what makes every
/// rectangle here something `tests/look.rs` can ask about without a device.
/// The measured price of the whole panel pass is a median 1518 allocations a
/// frame (`crates/karakuri`'s `WRITTEN_ALLOCS`, taken 2026-08-31); this adds
/// on the order of forty, which is inside the factor of two that file will
/// quote a figure across and is why the figure has not been re-taken here —
/// re-taking one needs a window and three seconds of nobody touching it.
///
/// Paid on a pointer event and on a frame, and a console with no look behind
/// it pays none of it: the `look?` is the first line.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
// **Eight, where clippy's line is seven**, and it went past it when the row
// gained `learn` and `map`. Every one is a thing the *row before this one* is
// laid out from — the tempo, the input, the tracker, the map, the arrangement
// — and this group is the last item in a flex row, so it is laid out from all
// of them. A struct carrying them would be `View` itself with the fields no
// pill in this row reads left out, which is `App::new`'s sentence one crate
// along.
#[allow(clippy::too_many_arguments)]
pub fn look(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    // Handed straight to [`arrangement`], which is the item before this one
    // and the only reason this needs it — see that function's own note.
    map: Option<&MapPill>,
    arr: &Arrangement,
    look: Option<Look>,
) -> Option<LookRow> {
    let look = look?;
    // The pill, asked once and for both things it answers: whether the row
    // exists at all, and where the group before this one ended. `transport`
    // answers the first for the whole module and `arrangement` is laid out
    // from it, so this is that one answer read one item further along.
    let pill = arrangement(ctx, layout, values, audio, tracker, map, arr)?;
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let width = |text: &str| {
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

    let mid = strip.center().y;
    let span_h = size::BASE * size::LINE;
    // The capsule is the widest of the four whatever it says — see
    // `LookRow::tone`.
    let widest = TONEMAPS
        .iter()
        .map(|op| width(&tone_text(*op)))
        .fold(0.0, f32::max);
    let tone = Rect::from_min_size(
        Pos2::new(
            pill.pill.max.x + size::TRANSPORT_GAP,
            mid - size::PILL_H * 0.5,
        ),
        egui::vec2(size::PILL_PAD_X * 2.0 + widest, size::PILL_H),
    );
    // Centred in a capsule it does not fill, which is what *as wide as the
    // widest* comes to for the other three.
    let words = tone_text(look.tonemap);
    let tone_text = Rect::from_center_size(
        Pos2::new(tone.center().x, mid),
        egui::vec2(width(&words), size::PILL_H),
    );

    let label = Rect::from_min_size(
        Pos2::new(tone.max.x + size::TRANSPORT_GAP, mid - span_h * 0.5),
        egui::vec2(width(EXPOSURE_LABEL), span_h),
    );
    let track = Rect::from_min_size(
        Pos2::new(label.max.x + size::TRIM_GAP, mid - size::FADER_H * 0.5),
        egui::vec2(EXPOSURE_TRACK_W, size::FADER_H),
    );
    let grip = Rect::from_min_max(
        Pos2::new(track.min.x, mid - size::PILL_H * 0.5),
        Pos2::new(track.max.x, mid + size::PILL_H * 0.5),
    );
    let value = Rect::from_min_size(
        Pos2::new(track.max.x + size::TRIM_GAP, mid - span_h * 0.5),
        egui::vec2(width(&exposure_text(look.exposure)), span_h),
    );

    // The rule `arrangement` and `outputs_row` both state: a control that does
    // not fit in the row it is drawn in is no control at all, rather than half
    // of one over the frame readout.
    if !strip.contains_rect(tone)
        || !strip.contains_rect(grip)
        || value.max.x + size::TRANSPORT_GAP > row.frame.min.x
    {
        return None;
    }

    Some(LookRow {
        tone,
        tone_text,
        label,
        track,
        fill: filled(track, Axis::Row, unit_of(look.exposure)),
        grip,
        value,
        values: look,
    })
}

/// **The capsule's words**: `tone · aces`.
///
/// One string rather than two galleys laid end to end, for [`pill_text`]'s
/// reason: the mock writes one run of text and laying it out as one keeps the
/// spaces round the `·` the type's own.
fn tone_text(tonemap: Tonemap) -> String {
    format!("{TONEMAP_LABEL} · {}", tonemap.name())
}

/// **The level, as the Master bay's own row writes it** — `out 1.00`, two
/// places. The two are the same kind of reading and are deliberately written
/// the same way; where they stop being the same *number* is
/// [ADR-0224](../../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md).
fn exposure_text(exposure: f32) -> String {
    format!("{exposure:.2}")
}

/// **The look controls, painted.**
///
/// Where everything goes is [`look`]'s, so this paints and derives nothing.
/// Term for term from `style.css`:
///
/// - `.pill` — the capsule, which is [`arrangement_into`]'s treatment and this
///   is the same pill three items along. No `▾`, because nothing opens.
/// - the `exp` before the track — `style="color:var(--c-faint)"` in the
///   markup, which is `pal.faint`, and it is `.trim .lbl`'s job one bay up.
/// - `.fader` — `background: var(--c-well)` with
///   `box-shadow: inset 0 0 0 1px var(--c-hair)` and `border-radius: 999px`,
///   which on a 5px box is a capsule.
/// - `.fader b` — `linear-gradient(90deg, var(--c-mint), var(--c-lav))`,
///   through [`gradient`], which is the one place that ramp is drawn and is
///   what the mixer's own faders are painted with.
/// - the figure — `.val`, `pal.text`, *a value*.
///
/// **`.fader s` is deliberately not drawn.** The mock's knob is a handle and
/// this control has none — see [`LookRow::exposure`].
pub(super) fn look_into(ui: &Ui, pal: &Palette, row: &LookRow) {
    let painter = ui.painter();
    painter.rect_stroke(
        row.tone,
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let centred = |rect: Rect, galley: std::sync::Arc<egui::Galley>, colour: Color32| {
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            colour,
        );
    };
    let words = tone_text(row.values.tonemap);
    centred(
        row.tone_text,
        painter.layout_no_wrap(
            words,
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        ),
        pal.dim,
    );
    centred(
        row.label,
        painter.layout_no_wrap(
            EXPOSURE_LABEL.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.faint,
        ),
        pal.faint,
    );

    let radius = CornerRadius::same((size::FADER_H * 0.5) as u8);
    painter.rect_filled(row.track, radius, pal.well);
    painter.rect_stroke(
        row.track,
        radius,
        Stroke::new(size::HAIRLINE, pal.hair),
        StrokeKind::Inside,
    );
    gradient(painter, row.fill, Axis::Row, pal.mint, pal.lav, true);

    centred(
        row.value,
        painter.layout_no_wrap(
            exposure_text(row.values.exposure),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.text,
        ),
        pal.text,
    );
}

impl View {
    /// **What the transport row declares**: the beat's staleness for as long
    /// as the grid is being drawn, and nothing when it is not.
    ///
    /// # Two conditions, and neither of them is *pending*
    ///
    /// **The row is laid out**, which is ADR-0193 asked of a row rather than
    /// of a bay — [`Layout::visible`](karakuri_layout::Layout::visible)
    /// answers for `transport` exactly as it answers for `mixer`, and a folded
    /// row is not showing a beat, so it cannot be showing one out of date.
    /// The transport is a direct child of the unnamed root, so the only thing
    /// that encloses it is the root itself (ADR-0204).
    ///
    /// **There are values behind it.** [`View::transport`] is `None` for a
    /// console with no engine, and [`transport`] draws no row at all then —
    /// there is no light on a grid that is not there. That is the same seam as
    /// [`View::picture`] and not a second rule.
    ///
    /// **Nothing else is asked, and that is the declaration's whole content.**
    /// P-0094's forced clause is that *something is moving continuously while
    /// the console is live*, so a beat that declared only while something was
    /// pending would be the signal going quiet at the moment it is worth
    /// having. It is also why this is not folded into
    /// [`View::mixer_declares`]: two rates, two deadlines, two regions.
    ///
    /// # The health capsule declares nothing, and that is the right answer
    /// rather than an omission
    ///
    /// [`Transport::health`] changes when a build lands, is thrown out or
    /// fails to assemble, which is a person saving a file — so its own
    /// `moves_in` is *not until something happens*, and a region cannot
    /// declare that. It does not have to: **the unit of declaration is the
    /// region and not the presentation** ([`Declared`] — *"a second rate would
    /// be a second declaration; a second user of one rate is not"*), and the
    /// region it is in is already drawn at [`BEAT_STALENESS`] for as long as
    /// there is a row at all. A staleness written for the capsule would be a
    /// rate for something that does not move (ADR-0283), and it could only
    /// ever be slower than the beat's, so it would change nothing a window
    /// does — [`View::animating`] takes the soonest.
    pub(super) fn transport_declares(&self, layout: &karakuri_layout::Layout) -> Option<Declared> {
        let row = layout
            .find("transport")
            .is_some_and(|id| layout.visible(id));
        (row && self.transport.is_some()).then_some(Declared {
            region: "transport",
            cost: PANEL_PASS,
            staleness: BEAT_STALENESS,
            // **The one region whose two numbers are the same number**, and
            // that is what an honest *I move at this rate* looks like: the
            // light is on the grid at [`Transport::beats`], the harness
            // advances the session by one step on every frame it composes, so
            // every frame this declaration buys draws the light somewhere it
            // was not (ADR-0283). There is no rest to find and nothing to
            // gate — P-0094 is the rule that says there had better not be.
            moves_in: BEAT_STALENESS,
        })
    }
}

#[cfg(test)]
mod tests {
    //! **The one figure this row writes that is worth asserting from `src/`
    //! rather than from `tests/transport.rs`.**
    //!
    //! [`offset_text`] is private, and moving the assertion out would mean
    //! widening a surface to test it — a wider surface for a narrower reason.
    //! `view::mod`'s own `tests` module states the same rule for the
    //! Inspector's arithmetic, which is the precedent this one follows.

    use super::*;

    /// **The offset figure carries its sign and its unit, and it has no
    /// `-0 ms`.**
    ///
    /// `docs/manual/console.html` is why the sign is always drawn — *"the sign
    /// is the half that gets read wrong at two in the morning"* — and why the
    /// unit is never left off: *"Two things on this panel are called an
    /// offset … The unit is what tells them apart."*
    ///
    /// **The zero is the case worth an assertion.** The track is 80 pixels for
    /// a range of 400 ms, so a press a fraction of a pixel either side of the
    /// middle asks for a value that rounds to zero — and a `{:+.0}` written
    /// straight off it answers `-0 ms` on one side and `+0 ms` on the other,
    /// which is two spellings of the one value an operator is most likely to
    /// be aiming at.
    #[test]
    fn the_offset_figure_is_signed_carries_its_unit_and_has_one_zero() {
        assert_eq!(offset_text(-15.0), "-15 ms");
        assert_eq!(offset_text(15.0), "+15 ms");
        assert_eq!(offset_text(LATENCY_OFFSET_MIN_MS), "-200 ms");
        assert_eq!(offset_text(LATENCY_OFFSET_MAX_MS), "+200 ms");
        // Either side of the middle, and exactly on it.
        assert_eq!(offset_text(0.0), "+0 ms");
        assert_eq!(offset_text(-0.4), "+0 ms");
        assert_eq!(offset_text(0.4), "+0 ms");
        // And the first value on each side that is not zero still says which
        // side it is on, so the normalisation above has not swallowed a sign.
        assert_eq!(offset_text(-0.6), "-1 ms");
        assert_eq!(offset_text(0.6), "+1 ms");
    }
}
