use super::super::*;
use super::*;

// ---------------------------------------------------------------------------
// The Inspector's parameters and sensor sources
// ---------------------------------------------------------------------------

/// The word in a sensitivity row's left-hand track, `.sens`'s first column.
pub const SENS_LABEL: &str = "sensitivity";

/// The word on a sensitivity row's last chip, which is the second rule's other
/// half.
pub const TAKE_BACK: &str = "take back";

/// What a parameter row's leftmost cell reads where the control is not on the
/// interface — the mock's `&middot;`, in `.param.unpub .ord`'s hairline colour.
///
/// A dot where a number would be, because a control off the interface has no
/// position and a position is exactly what a MIDI knob counts. It is the same
/// cell either way: the number and the mark are one control's two states rather
/// than a mark drawn beside a number (`docs/adr/0329-…`).
const UNPUBLISHED: &str = "·";

/// One parameter row: what the mock's `.param` reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// The position in the deck's published interface, counting from one and
    /// spanning nodes — the mock's `.ord`, and the number a MIDI control is learned
    /// against: *"knob 3 is knob 3 whatever Set is loaded"*.
    ///
    /// `None` is a control the interface does not carry, and it is the one state of
    /// this field rather than a missing value: a control off the interface has no
    /// position, and a position is exactly what a knob counts. Such a row keeps its
    /// place in its group and loses its number, its fader and its figure — drawn so
    /// the mark can be pressed again, because this bay is where publishing is
    /// chosen and a choice nobody can see is one nobody can unmake
    /// ([ADR-0100](../../../../docs/adr/0100-a-published-interface-is-a-choice-of-attention.md),
    /// `docs/adr/0329-…`).
    ///
    /// It is not *hidden* and it is not *locked*: `--param`, a `param` record and a
    /// model naming the address all still reach the value, which is ADR-0100's
    /// whole sentence — publishing is a choice of attention and never one of
    /// authority.
    pub ord: Option<usize>,
    /// What the Set published it as, which may be an alias for the key inside the
    /// node.
    pub name: String,
    /// What it holds.
    pub value: f32,
    /// What the Set published it over, low then high — `Published::range`, which
    /// *"narrows, never redefines"* the range the procedure declared.
    ///
    /// # It used to be the position and is now the range, and that is a decision
    /// rather than a widening
    ///
    /// This field read `at: f32`, *"where it sits in its published range, `[0,
    /// 1]`"*, and its own argument was that the fader is the only reader, so a
    /// range plus a value would be a second derivation of *where along the track*.
    /// A fader a hand can move has a second reader — the grab, which turns a
    /// pointer back into a value — and that one needs the range whichever way this
    /// field is spelled. So the position is [`Param::at`], derived here, and the
    /// two directions are one statement in one place: [`ParamGrip`] and
    /// [ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md).
    pub range: [f32; 2],
    /// Which control this row is, as the vocabulary addresses one — `Published::at`
    /// and `Published::key`, carried over unchanged.
    ///
    /// Not the group the row was drawn in, which is the other reading and is the
    /// one ADR-0286 refuses: a wildcard covering exactly one node is *drawn* in
    /// that node's group, and it goes on meaning every node that declares the key.
    /// See [`ParamGrip`].
    ///
    /// The vocabulary's own type rather than a pair of this crate's, because the
    /// operation carries exactly this and a second spelling of an address is what
    /// `karakuri-operation` exists to stop
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    pub param: karakuri_operation::ParamAt,
    /// What is holding this control, or `None` for a row nothing is driving — the
    /// mock's `.param.bound` and the `.sens` row under it.
    ///
    /// Written by whoever read the Set, off its bindings, and it is the seventh
    /// reading the harness takes: `Set::bindings` was the one of them this pane did
    /// not ask for, on ADR-0191's terms, because nothing bound anything and a bound
    /// row was a state the program could not enter. What changed is that a press
    /// can now attach one.
    pub bound: Option<Source>,
}

/// What is driving one parameter row — the mock's `.pval.src` and the three
/// readouts on the `.sens` row under it.
///
/// # It is the attachment's own answer and not a second derivation
///
/// The signal, the shape and the range are read off the binding the Set is
/// holding and carried over unchanged, which is [`Param::param`]'s rule one
/// field along: what a control *is* comes from whoever published it, and a
/// surface that rebuilt any of it would be a second statement about one
/// attachment.
///
/// [`Source::range`] is not [`Param::range`], and the two are two facts. The
/// row's range is what the control was *published* over — the span the fader
/// rides, which is the procedure's declaration narrowed — and this one is what
/// the signal is *mapped onto*, which a `bind` may narrow again within it.
/// Drawing one and writing the other is what a chip on this row would do if
/// there were one range here, and the operator would see a mapping change under
/// a press that only asked for a different curve.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    /// The signal's own name, as the row's `.pval.src` reads it: `energy`, `beat`,
    /// `band3`, `noise`, or `control:<name>` for a macro.
    pub signal: String,
    /// The shape the signal is put through, and the one chip on this row that is a
    /// control.
    pub curve: karakuri_operation::Curve,
    /// What the signal is mapped onto, low then high — the attachment's, not the
    /// row's. See the type's own note.
    pub range: [f32; 2],
    /// Which attachment this is, as the vocabulary addresses one.
    ///
    /// `karakuri_operation::BindAt` and not [`Param::param`]'s `ParamAt`, carried
    /// over from the binding rather than derived from the row: an attachment is one
    /// layer's, where a value's wildcard names no layer at all, and the two are two
    /// facts rather than two spellings (see `BindAt`). It is also what makes a
    /// row's *take back* remove the attachment it is drawn from rather than one
    /// that happens to match by name.
    pub at: karakuri_operation::BindAt,
}

impl Param {
    /// Where the value sits in the published range, `[0, 1]` — the fader's fill,
    /// and what a knob's centre is put on.
    ///
    /// A range of no width is a control with one position, and the fader sits at
    /// its start rather than at a division by zero. That is the guard the harness
    /// used to carry when this was a field; it is here now, so there is one place a
    /// degenerate range is answered for.
    pub fn at(&self) -> f32 {
        let [low, high] = self.range;
        match high > low {
            false => 0.0,
            true => unit((self.value - low) / (high - low)),
        }
    }

    /// What this control holds with its fader at `at` — [`Param::at`] inverted, and
    /// the whole of what a hand on this row asks for.
    ///
    /// `at` is a position on `[0, 1]`, which is what [`Grab::value`] answers, and
    /// it is clamped here for [`unit`]'s reason rather than trusted: a published
    /// range narrows and never redefines, so a value outside it is one the
    /// procedure did not say it still looks like itself over.
    pub fn valued(&self, at: f32) -> f32 {
        let [low, high] = self.range;
        low + (high - low) * unit(at)
    }

    /// Whether a hand can move this row at all.
    ///
    /// `Grab::new`'s refusal read on the value axis instead of on the track: a
    /// published range of no width is a control with one position, so a knob on it
    /// is a handle with nowhere to go and every drag of it would ask for the value
    /// it already holds. The row is still drawn — the fill and the figure say what
    /// it is — and it is not taken hold of.
    ///
    /// # A bound row is drawn and is not taken hold of either
    ///
    /// The number a hand would write is not the number the row is showing. A knob a
    /// hand moves writes the value a binding blends *from*, and at a measurement's
    /// full confidence that value carries no weight at all — so the handle would
    /// move under the hand and the picture would not, which is the one thing
    /// [ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md)
    /// refuses a track press for: *"a handle that jumped to the pointer would be a
    /// lie about what a handle is"*, read on the value axis.
    ///
    /// It is not a refusal of the write, and that distinction is
    /// [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)'s:
    /// `Operation::WriteParam` on a bound parameter is legal, lands, and leaves the
    /// attachment where it is — a `--param` does exactly that today. What this says
    /// is that *this row's fader* is not the affordance for it while something else
    /// is holding the control, and the affordance that is there is `take back`, one
    /// row down. After it the row is a handle again.
    ///
    /// That is the question ADR-0286 left open — *"whether a hand may move a knob a
    /// signal is holding is Take a parameter back's question"* — and
    /// `docs/manual/console.html`'s *Who is holding a control* is where the page
    /// says it. This row as an entry of a published interface — the shape
    /// [`Operation::Publish`](karakuri_operation::Operation::Publish) carries,
    /// which is `karakuri_engine::set::Published`'s.
    ///
    /// The address and the range are the ones the row was drawn from, not ones
    /// re-derived here: a wildcard stays a wildcard and a narrowed range stays
    /// narrowed, which is [`ParamGrip`]'s own rule about writing the control the
    /// row draws rather than the group it was placed in (ADR-0286).
    pub(crate) fn control(&self) -> karakuri_operation::Control {
        karakuri_operation::Control {
            name: self.name.clone(),
            node: self.param.node,
            key: self.param.key.clone(),
            range: self.range,
        }
    }

    pub(crate) fn movable(&self) -> bool {
        // **A control off the interface draws no fader**, which is what
        // publishing decides: the row is a name and a mark, and there is
        // nothing on it to take hold of.
        self.ord.is_some() && self.bound.is_none() && self.range[1] > self.range[0]
    }
}

/// A parameter fader taken hold of: which deck, which control, and the
/// track the value rides.
///
/// # What it answers, and what it deliberately does not
///
/// The brief on this control is *a pointer landing on a parameter row's fader
/// answers which deck, which parameter and what value*, and those are the
/// three things here: [`deck`](Self::deck), [`param`](Self::param)'s
/// [`Param::param`], and [`Param::valued`] at wherever the drag gets to.
///
/// It is not a `Grab`, and that is the seam rather than a gap. A
/// [`crate::panel::Knob`] is what turns a track position into an
/// [`Operation`], and the arm for this control is
/// [`crate::panel::Knob`]'s to grow — see the module the operation is
/// constructed in. What is here is everything the view can answer without it:
/// where the knob is, which is geometry and the value it was drawn from, and
/// which control it belongs to, which is what the harness read off
/// `Set::published`. The one line that closes it reads
///
/// ```ignore
/// grabbed(
///     grip.fader,
///     Knob::Param {
///         deck: grip.deck,
///         param: grip.param.param.clone(),
///         range: grip.param.range,
///     },
///     Pos2::new(p.x, p.y),
/// )
/// ```
///
/// and it is [`grabbed`] — the same inverse of [`fader`] the mixer's two
/// knobs and the master out are taken hold of through, so a parameter fader
/// keeps whatever it grabbed at and the value does not jump.
///
/// # The control it names is the published one, not the group it was drawn in
///
/// [`Param::param`] is `Published::at` and `Published::key` carried over, so a
/// wildcard row stays a wildcard: `None` means every node that declares
/// the key, and the engine refuses one that spans nodes under disagreeing
/// authorities (ADR-0223). The row was *placed* in a group by resolving that
/// wildcard where it covered exactly one node, and writing what the placement
/// resolved to would narrow the control to the node it happens to reach today
/// — [ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md).
#[derive(Debug, Clone, PartialEq)]
pub struct ParamGrip<'a> {
    /// Which deck's, which is the manual's word for what the code calls a slot —
    /// [`Pane::deck`], the deck this pane is pointed at, and not
    /// [`View::selection`].
    pub deck: u8,
    /// The row a hand landed on, borrowed from the pane it was drawn from rather
    /// than copied: the address, the range and the value it holds are all on it
    /// already, and a copy of any of them here would be a second statement about
    /// one control.
    pub param: &'a Param,
    /// The track it took hold of, at the value the row was drawn at — what
    /// [`grabbed`] measures the grip's offset and travel from.
    pub fader: Fader,
}

/// How tall one parameter row and whatever is under it comes to: a
/// [`size::PARAM_H`] row, and a [`size::SENS_H`] sensitivity row where
/// something is holding the control.
///
/// One function because three callers must agree. [`group_h`] sums it,
/// [`param_rect`] walks it as an offset, and [`sens_rect`] steps off the end of
/// one row — and a group as tall as *n* rows with a press resolved against a
/// stride of *n* is a chip drawn where a hand cannot reach it. That is
/// [`param_rect`]'s own argument about a running sum, one level down.
pub(crate) fn rows_h(param: &Param) -> f32 {
    size::PARAM_H
        + match param.bound {
            None => 0.0,
            Some(_) => size::SENS_H,
        }
}

/// One chip on a sensitivity row, and which of the four a press landed on.
///
/// # Two are controls and two are readouts, and that is the decision
///
/// The mock draws four pills and this crate claims two of them, which is
/// [`InspectorPane::select_renderer`]'s arrangement on an inert renderer row:
/// *a control claims what it acts on and no more*.
///
/// - [`SensChip::Signal`] is a readout. The signal bus is open by
///   design — `SignalBus::sample` cannot fail and a name nobody provides
///   comes back at confidence 0.0 — so there is no list of sources anywhere
///   for a chooser to be built over, and a console inventing one would be the
///   surface deciding what may be asked for, which is
///   [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
///   exactly inverted. A source is named where a source can be named: a Set
///   file's `bind` line, or `--bind`.
/// - [`SensChip::Curve`] is the control, and it is the blend chip's shape
///   one bay along: four destinations, a cycle drawn over them here, and
///   `Operation::AttachSignal` naming the one it arrives at. It re-attaches
///   the same signal over the same range through the next shape.
/// - [`SensChip::Range`] is a readout, because a range is the procedure's
///   declaration and not an operator's to write —
///   `karakuri_operation::ParamValue`'s own sentence — and there is no second
///   number on this row for a confidence either: a value arrives with how well
///   it is known.
/// - [`SensChip::TakeBack`] is the other control, and it is the row this
///   whole arrangement exists for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensChip {
    Signal,
    Curve,
    Range,
    TakeBack,
}

impl SensChip {
    /// The four, left to right, in the order the mock draws them.
    pub const ALL: [SensChip; 4] = [
        SensChip::Signal,
        SensChip::Curve,
        SensChip::Range,
        SensChip::TakeBack,
    ];

    /// What this chip reads, off the attachment the row was drawn from.
    ///
    /// The range is written to two places, which is `.pval`'s figure and the mock's
    /// own `0.10 – 2.40`; the en dash is the mock's `&ndash;`.
    pub fn text(self, source: &Source) -> String {
        match self {
            SensChip::Signal => source.signal.clone(),
            SensChip::Curve => source.curve.name().to_string(),
            SensChip::Range => {
                format!("{:.2} \u{2013} {:.2}", source.range[0], source.range[1])
            }
            SensChip::TakeBack => TAKE_BACK.to_string(),
        }
    }

    /// What a press on this chip asks for, or `None` for the two that are readouts
    /// — see the type's own note for why those two are not controls.
    ///
    /// The curve chip restates the attachment. An `AttachSignal` carries the whole
    /// of what an attachment is, so changing one field means sending the other
    /// three back unchanged — the source and the range come off [`Source`] rather
    /// than being rebuilt, which is what stops a press for a different shape from
    /// silently re-mapping the signal.
    pub fn operation(self, deck: u8, source: &Source) -> Option<Operation> {
        match self {
            SensChip::Signal | SensChip::Range => None,
            SensChip::Curve => Some(Operation::AttachSignal {
                deck,
                param: source.at.clone(),
                signal: source.signal.clone(),
                curve: next_curve(source.curve),
                range: source.range,
            }),
            SensChip::TakeBack => Some(Operation::TakeParamBack {
                deck,
                param: source.at.clone(),
            }),
        }
    }
}

/// The next of the four shapes, wrapping — the cycle the curve chip is, and it
/// lives here rather than in the vocabulary for
/// [`karakuri_operation::Curve::ALL`]'s stated reason: the list is the
/// vocabulary's and the cycle is the surface's.
fn next_curve(curve: karakuri_operation::Curve) -> karakuri_operation::Curve {
    let all = karakuri_operation::Curve::ALL;
    let at = all.iter().position(|c| *c == curve).unwrap_or(0);
    all[(at + 1) % all.len()]
}

/// Where each chip of a sensitivity row goes, laid end to end from the row's
/// left-hand track.
///
/// [`rend_chips`]' shape one row down and for its reason: the row is painted
/// *and* pressed, and two copies of where a chip is would be a chip painted
/// where a hand cannot reach it. A chip is as wide as the word in it, so this
/// costs a galley lookup per chip.
pub fn sens_chips<'a>(
    ctx: &'a egui::Context,
    row: Rect,
    source: &'a Source,
) -> impl Iterator<Item = (SensChip, Rect)> + 'a {
    // `.sens`'s two tracks: the word, then the chips, with the grid's own gap
    // between them.
    let mut x = row.min.x + size::SENS_PAD_L + size::SENS_LABEL_W + size::SENS_GAP;
    // One padding down from the top of the row, which is where `.sens` puts
    // it: `padding: 2px 10px 6px 12px`, so a chip is not centred and the space
    // under it is three times the space over it.
    let top = row.min.y + size::SENS_PAD_T;
    SensChip::ALL.into_iter().map(move |chip| {
        let w = sens_width(ctx, &chip.text(source));
        let rect = Rect::from_min_size(Pos2::new(x, top), egui::vec2(w, size::SENS_CHIP_H));
        x += w + size::SENS_CHIP_GAP;
        (chip, rect)
    })
}

/// One sensitivity chip's width: the word at [`size::SENS_SIZE`] inside
/// `.pill`'s padding.
fn sens_width(ctx: &egui::Context, text: &str) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::SENS_SIZE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    }) + size::SENS_CHIP_PAD_X * 2.0
}

/// One parameter row, in the mock's own four tracks: the ordinal at
/// [`size::PARAM_ORD_W`] right-aligned, the name at [`size::PARAM_NAME_W`], the
/// fader taking what is left, and the value at [`size::PARAM_VAL_W`]
/// right-aligned.
///
/// The value is two places, which is the mock's `2.40`, `0.71`, `1.20` — every
/// number in its `.pval` column. It is not a second spelling of the transport's
/// tempo: that one is a BPM and this is a parameter, and the mock writes the
/// two differently for that reason.
pub(crate) fn param_into(painter: &egui::Painter, pal: &Palette, row: Rect, param: &Param) {
    let left = row.min.x + size::PARAM_PAD_L;
    let right = row.max.x - size::PARAM_PAD_R;
    // **The mark is the number**, and a control the interface does not carry
    // has none: `.param.unpub .ord` is the mock's dot in the hairline colour,
    // where a published row's is its position in `--c-faint`. One cell, two
    // states, and the state *is* whether it is published — a second mark beside
    // the number would be two spellings of one fact
    // (`docs/adr/0329-…`).
    let (word, ink) = match param.ord {
        Some(ord) => (ord.to_string(), pal.faint),
        None => (UNPUBLISHED.to_owned(), pal.hair),
    };
    let ord = painter.layout_job(span_at(&word, size::PARAM_ORD_SIZE, ink));
    painter.galley(
        Pos2::new(
            left + size::PARAM_ORD_W - ord.size().x,
            row.center().y - ord.size().y * 0.5,
        ),
        ord,
        ink,
    );
    let name_x = left + size::PARAM_ORD_W + size::PARAM_GAP;
    // `.param .pname`'s `overflow: hidden; text-overflow: ellipsis` — one row,
    // broken anywhere, with an ellipsis for what did not fit. The mock says so
    // for this column and not for the library's, which is why one elides and
    // the other clips.
    // `.param.unpub .pname` is a shade further back than `.param`'s, which is
    // the whole of what an unpublished row looks like beside a published one:
    // the name is still legible — the row is drawn so it can be pressed again —
    // and nothing about it invites a hand.
    let name_ink = match param.ord {
        Some(_) => pal.dim,
        None => pal.faint,
    };
    let mut job = span_at(&param.name, size::BASE, name_ink);
    job.wrap = egui::epaint::text::TextWrapping {
        max_width: size::PARAM_NAME_W,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let name = painter.layout_job(job);
    painter.galley(
        Pos2::new(name_x, row.center().y - name.size().y * 0.5),
        name,
        name_ink,
    );
    // **A control the interface does not carry stops here.** The fader and the
    // figure are what publishing decides the panel shows, so a row that is off
    // the list is a mark and a name and nothing else — and a value drawn beside
    // a control this pane says it is not showing would be the page's own
    // sentence contradicted in the same row.
    if param.ord.is_none() {
        return;
    }
    // **A bound row shows its source instead of a number** — `.param.bound`'s
    // `.pval.src`, and the readout the manual calls *where disagreeing with
    // the system begins*. The mock draws it in the same right-aligned track
    // the figure is in, so this is one galley either way, and it is the mint
    // `.src` carries rather than `.pval`'s text colour.
    let (text, colour) = match &param.bound {
        None => (format!("{:.2}", param.value), pal.text),
        Some(source) => (source.signal.clone(), pal.mint),
    };
    let value = painter.layout_job(span_at(&text, size::BASE, colour));
    painter.galley(
        Pos2::new(
            right - value.size().x,
            row.center().y - value.size().y * 0.5,
        ),
        value,
        colour,
    );
    if let Some(fader) = param_fader(row, param) {
        fader_into(painter, pal, fader, false, None);
    }
}

/// One sensitivity row: the word in `.sens`'s first track, then the chips that
/// say what is holding the control and offer the two things a hand can do about
/// it.
///
/// Where each chip goes is [`sens_chips`]', so this paints and derives nothing
/// — [`rend_row_into`]'s rule one row down, and the reason a press has
/// somewhere to ask what it landed on.
///
/// Two of the four are drawn as readouts and two as controls, and nothing in
/// the paint says which: the mock gives the source pill `.pill.armed` and the
/// other three a plain `.pill`, and *armed* here is the mint of something that
/// is holding a control rather than of something that can be pressed. Which
/// chips are claimed is [`SensChip::operation`]'s, and a panel that drew the
/// difference would be drawing a rule the mock does not.
pub(crate) fn sens_into(painter: &egui::Painter, pal: &Palette, row: Rect, source: &Source) {
    let label = painter.layout_job(span_at(SENS_LABEL, size::SENS_SIZE, pal.faint));
    painter.galley(
        Pos2::new(
            row.min.x + size::SENS_PAD_L,
            row.min.y + size::SENS_PAD_T + (size::SENS_CHIP_H - label.size().y) * 0.5,
        ),
        label,
        pal.faint,
    );
    let radius = CornerRadius::same((size::SENS_CHIP_H * 0.5) as u8);
    for (chip, rect) in sens_chips(painter.ctx(), row, source) {
        let armed = chip == SensChip::Signal;
        match armed {
            // `.pill.armed`: no border, a 14% wash of the mint and the same
            // nine-pixel halo an armed pill carries everywhere else on this
            // panel.
            true => {
                painter.add(
                    egui::epaint::Shadow {
                        offset: [0, 0],
                        blur: size::TALLY_GLOW - 1,
                        spread: 0,
                        color: pal.glow,
                    }
                    .as_shape(rect, radius),
                );
                painter.rect_filled(rect, radius, tint(pal.mint, 14));
            }
            // `.pill`'s `border: 1px solid var(--c-line)`.
            false => {
                painter.rect_stroke(
                    rect,
                    radius,
                    Stroke::new(size::HAIRLINE, pal.line),
                    StrokeKind::Inside,
                );
            }
        }
        let colour = match armed {
            true => pal.mint,
            false => pal.dim,
        };
        let galley = painter.layout_no_wrap(
            chip.text(source),
            FontId::new(size::SENS_SIZE, FontFamily::Proportional),
            colour,
        );
        painter.galley(
            Pos2::new(
                rect.min.x + size::SENS_CHIP_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            colour,
        );
    }
}

/// One parameter row's fader, laid out — the track between the name and the
/// figure, what the value fills of it, and the knob on the fill's moving edge.
/// `None` where the row is too narrow to have a track at all.
///
/// # One derivation, asked twice
///
/// [`param_into`] paints this and [`InspectorPane::grip`] hit-tests it, which
/// is [`Mixer::grab`]'s rule and [`MasterRow::grab`]'s: two copies of where a
/// knob is would be a knob painted where a hand cannot take hold of it. The
/// value is part of the geometry — the knob sits on the fill's moving edge, so
/// where it is depends on what the deck said this frame, and the row a hand
/// grabs is the row it saw.
///
/// `.param`'s middle track, which is the `1fr` of `grid-template-columns: 15px
/// 88px 1fr 58px`: the ordinal, the name and the figure are stated widths and
/// this is what is left between them. `lib.rs` measures the pane's own minimum
/// off exactly that — *"the fader is the `1fr` track and is drawn only where
/// what is left over is positive"* (ADR-0279) — and this is where that
/// `positive` is asked.
pub(crate) fn param_fader(row: Rect, param: &Param) -> Option<Fader> {
    let left = row.min.x + size::PARAM_PAD_L;
    let right = row.max.x - size::PARAM_PAD_R;
    let track = Rect::from_min_max(
        Pos2::new(
            left + size::PARAM_ORD_W + size::PARAM_GAP + size::PARAM_NAME_W + size::PARAM_GAP,
            row.center().y - size::FADER_H * 0.5,
        ),
        Pos2::new(
            right - size::PARAM_VAL_W - size::PARAM_GAP,
            row.center().y + size::FADER_H * 0.5,
        ),
    );
    match positive(track) {
        false => None,
        // `.fader b` fills its 5px track edge to edge, so the inset is zero —
        // the one argument that tells this fader from the mixer's vertical
        // one, which `fader` takes for exactly this reason.
        true => Some(fader(
            track,
            Axis::Row,
            param.at(),
            0.0,
            egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
        )),
    }
}

/// Where the `index`th parameter row of `node` goes inside the group rectangle
/// [`InspectorPane::group`] answered.
///
/// [`InspectorPane::group`]'s walk one level in, and a function rather than a
/// running sum inside [`node_into`] for the reason the deck head was lifted out
/// of `inspector_into`: a press had nowhere to ask what it had landed on. The
/// head is [`size::NODE_HEAD_H`], the renderer row is [`size::REND_ROW_H`]
/// where the group has one, and the rows are [`size::PARAM_H`] each from there
/// — which is [`group_h`] read as an offset instead of as a total, and the two
/// are checked against each other in `tests/param_fader.rs`.
pub(crate) fn param_rect(group: Rect, node: &Node, index: usize) -> Rect {
    let top = group.min.y
        + size::NODE_HEAD_H
        + uses_h(node)
        + match node.renderers.is_empty() {
            true => 0.0,
            false => size::REND_ROW_H,
        }
        // **A walk and not a stride, because a row is as tall as what is under
        // it.** A bound row carries a sensitivity row, so the rows above this
        // one are not all [`size::PARAM_H`] — which is [`InspectorPane::group`]'s
        // own reason for walking the groups instead of multiplying, one level in.
        + node.params.iter().take(index).map(rows_h).sum::<f32>();
    Rect::from_min_max(
        Pos2::new(group.min.x, top),
        Pos2::new(group.max.x, top + size::PARAM_H),
    )
}

/// The leftmost cell of a parameter row, which is the mark that publishes it:
/// [`size::PARAM_ORD_W`] wide at the row's left padding, the full height of the
/// row.
///
/// The whole cell and not the ink in it. A published row's number is one or two
/// glyphs and an unpublished row's is a dot, so a target the size of what is
/// drawn would be a control that shrank as the interface grew past nine — which
/// is the *drawn and not claimed* mistake made in the other direction. The cell
/// is a fixed track of the mock's own grid, so the target is the same size on
/// every row.
///
/// `param` is taken so that this cannot be asked of a row that has none to
/// give; there is no such row today, and the argument for the cell being one
/// control's two states is at [`UNPUBLISHED`].
pub(crate) fn ord_cell(row: Rect, _param: &Param) -> Rect {
    let left = row.min.x + size::PARAM_PAD_L;
    Rect::from_min_max(
        Pos2::new(left, row.min.y),
        Pos2::new(left + size::PARAM_ORD_W, row.max.y),
    )
}

/// Where the `index`th row's sensitivity row goes — directly under the row
/// itself, the full width of the group and [`size::SENS_H`] tall — or `None`
/// where nothing is holding that control.
///
/// [`param_rect`] stepped off the end of the row it belongs to, which is the
/// one place that relationship is written: the `.sens` row is not a row of its
/// own in the mock's list, it is what a `.param.bound` grows.
pub(crate) fn sens_rect(group: Rect, node: &Node, index: usize) -> Option<Rect> {
    let param = node.params.get(index)?;
    param.bound.as_ref()?;
    let row = param_rect(group, node, index);
    Some(Rect::from_min_max(
        Pos2::new(row.min.x, row.max.y),
        Pos2::new(row.max.x, row.max.y + size::SENS_H),
    ))
}
