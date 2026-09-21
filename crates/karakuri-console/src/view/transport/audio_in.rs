use super::super::*;
use super::*;

/// What the `● rec` pill says this frame, and the whole of its state.
///
/// Two values because the control is a toggle and a toggle has two ends: a
/// press on it starts a recording or stops the one running, and which of those
/// a press means is exactly this. There is no third value for *starting* —
/// opening a recorder writes a Set file and creates another, so it happens off
/// the frame path (`crates/karakuri/src/main.rs`), and until the recorder is
/// open nothing is being recorded and the pill says so.
///
/// A two-valued enum rather than a `bool`, which is
/// [P-0087](../../../../docs/principles/0087-name-the-property-never-the-shape.md):
/// `Some(true)` at a call site says nothing, and this is read at four of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rec {
    /// Nothing is being recorded. The mock's plain `.pill`, and a press starts one.
    Idle,
    /// A session is being written to the store as it happens. The mock's
    /// `.pill.on`, and a press stops it.
    Running,
}

// ---------------------------------------------------------------------------
// The audio-in pill
// ---------------------------------------------------------------------------

/// The pill's first word, `docs/manual/console.html`'s own, hyphen and all: it
/// is the head of the group of four that need an input, and the page names that
/// group by this pill.
const AUDIO_LABEL: &str = "audio-in";

/// What the pill says with no input open, and it is a word for a state rather
/// than a name — [`NO_ARRANGEMENT`] one pill to the left, and the preview
/// cell's `C · no slot` one bay down.
///
/// It is not the same statement as silence, which is the whole of
/// [P-0084](../../../../docs/principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md):
/// a quiet room measures `0.0` at full confidence and this pill would name the
/// input it measured it through. `none` is *no provider* — every name answers
/// what it answered before audio existed — and the two must not read alike on a
/// panel, because one of them is a room and the other is a cable.
const NO_INPUT: &str = "none";

/// What the card says where the machine has no inputs at all.
///
/// A menu with nothing in it would be a card an operator presses and cannot
/// tell from one that failed to open, so the empty case says which it is. It is
/// drawn the way [`Menu::Naming`]'s field is — a card with one line in it and
/// no rows, so [`AudioInPill::row`] hands out no rectangle for something that
/// is not a list — and it is a sentence rather than a row because there is
/// nothing to pick: a press on it shuts the menu like a press on any other part
/// of the card.
const NO_INPUTS: &str = "no inputs on this machine";

/// Audio input state for the transport pill, including device selection and menu state.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioIn {
    /// The input in use, by the description the device answers to, or `None` for an
    /// instrument with nothing open. See [`NO_INPUT`].
    pub device: Option<String>,
    /// Host audio inputs enumerated when opening the menu.
    pub inputs: Vec<String>,
    /// Whether the card is down. Private for [`Arrangement::menu`]'s reason.
    down: bool,
}

impl AudioIn {
    /// An instrument with nothing open and nothing listed, and the menu shut. A
    /// `const` for [`Arrangement::NONE`]'s reason: a test can name the state
    /// without building one.
    ///
    /// It is not what [`View::audio`] holds by default — that is `None`, which is a
    /// console nobody has told anything about audio and draws no pill at all. This
    /// is the console that has been told, and told there is nothing.
    pub const NONE: AudioIn = AudioIn {
        device: None,
        inputs: Vec::new(),
        down: false,
    };

    /// What the pill says after `audio-in ·`: the input in use, or [`NO_INPUT`].
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

    /// How many rows the open card has: one per input. Zero while it is shut, and
    /// zero on a machine with no inputs — that card is a sentence, not a list.
    ///
    /// The address descends into this card and a digit names the nth row of it,
    /// which is the same number [`AudioInPill`] lays out
    /// ([ADR-0350](../../../../docs/adr/0350-the-transports-two-cards-are-walked-and-the-tempo-figure-steps-by-a-beat-a-minute.md)).
    pub fn rows(&self) -> usize {
        match self.down {
            true => self.inputs.len(),
            false => 0,
        }
    }
}

/// What a press on the audio-in pill or on one of its rows asks for.
///
/// Three arms rather than [`Ask`]'s five, and it is a second enum rather than
/// three of that one: two of those arms are the arrangement family's — a name
/// being asked for and a `panel::Op` — and neither is a thing this control can
/// ever want. A shared enum with two arms that cannot happen is a `match` every
/// caller has to answer for twice.
#[derive(Debug, Clone, PartialEq)]
pub enum AudioAsk {
    /// Put the card down — a press on the pill with it shut. The caller reads the
    /// host on this, and writes what it found into [`AudioIn::inputs`] before the
    /// next frame draws the card.
    Open,
    /// Take it away — a press on the pill again, or anywhere on the card that is
    /// not a row.
    Shut,
    /// Listen to this input, issuing an [`Operation::AttachBeatSource`].
    Operation(Operation),
}

/// The audio-in pill, laid out: the capsule, what is written in it, and the
/// card under it while it is down.
///
/// One derivation, for [`ArrangementPill`]'s reason — [`View::draw`] paints
/// exactly these rectangles and [`crate::input::claim`] hit-tests exactly these
/// rectangles.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioInPill {
    /// The capsule, which is what a press has to land in to open the card.
    pub pill: Rect,
    /// Where `audio-in · Scarlett 2i2` is painted, inside the capsule's padding.
    pub text: Rect,
    /// The `▾` after it. Drawn rather than typed — see [`CHEVRON_W`].
    pub chevron: Rect,
    /// The card, or `None` while it is shut.
    pub menu: Option<Rect>,
    /// How many of [`AudioIn::rows`] the card has room for, between the pill and
    /// the bottom of the console. Fewer than there are is a machine with more
    /// inputs than the window is tall, and the foot says `n of m` in the Library
    /// bay's own words rather than the list quietly ending. Zero while it is shut,
    /// and zero on a machine with no inputs.
    pub rows: usize,
    /// What the card would list if the window were tall enough, carried so that the
    /// foot and the rows are one number rather than two.
    pub of: usize,
}

impl AudioInPill {
    /// Whether `p` is on the pill itself.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is anywhere this control owns — the pill, or the card while it
    /// is down.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        let at = Pos2::new(p.x, p.y);
        self.pill.contains(at) || self.menu.is_some_and(|menu| menu.contains(at))
    }

    /// Where one row of the card is, from the top — [`ArrangementPill::row`]
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

    /// Which input `p` is on, as an index into [`AudioIn::inputs`], or `None` for a
    /// point on no row — the card's padding, its foot, or anywhere off it.
    pub fn item(&self, p: karakuri_layout::Point) -> Option<usize> {
        let at = Pos2::new(p.x, p.y);
        (0..self.rows).find(|index| self.row(*index).contains(at))
    }

    /// What a press at `p` asks for, or `None` where the press was on nothing this
    /// control owns. [`ArrangementPill::ask`]'s shape over a list with no verbs in
    /// it.
    ///
    /// A press on the pill toggles the card; a press anywhere else on the card
    /// shuts it, because a press that did nothing at all is the one thing worse
    /// than a press that declines.
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

/// The audio-in pill's furniture, derived: the capsule, the words in it, and
/// the card under it.
///
/// # Where it sits, and why it is first of the drawn controls in this row
///
/// `docs/manual/console.html`'s `.transport` puts `.tracker` — the four things
/// that need an input — immediately after `bar 37`, and this pill is the head
/// of that group. Everything the mock draws between the bar and here is nothing
/// at all, so this lands one [`size::TRANSPORT_GAP`] after the bar.
///
/// The other three of its group are drawn now, and they are
/// [`tracker_group`]'s: the offset, the tap and the octave, laid out from this
/// pill's right edge at [`size::PILL_GAP`] — the group's own tighter gap, which
/// is what `.tracker` sets and what this paragraph said would happen the day
/// they landed. [`arrangement`] is laid out from the *group's* right edge now
/// rather than from this pill's, at the row's gap, because what ends there is a
/// whole group.
///
/// # No pill at all where the console has not been told
///
/// `None` wherever [`transport`] answers `None` — the row folded away, soloed
/// away, too narrow, or a console with no engine behind it — and `None` again
/// where `audio` is `None`, which is a console nobody has said anything to
/// about audio and is every test in this crate that does not say otherwise. A
/// pill reading `audio-in · none` on a console that was never told is a reading
/// invented here, which is [`View::transport`]'s own rule: empty is a state and
/// unasked is not.
///
/// # What it costs to ask
///
/// One galley lookup for the pill's own words, always, and while the card is
/// down one more per input, since the card is as wide as the widest name in it.
/// Paid on a pointer event and on a frame, and only while an operator is
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

/// The pill's words: `audio-in · Scarlett 2i2`, or `audio-in · none`. One run
/// of text, for [`pill_text`]'s reason.
fn audio_text(audio: &AudioIn) -> String {
    format!("{AUDIO_LABEL} · {}", audio.word())
}

/// The card under the audio-in pill: where it is, how many rows fit in it, and
/// how many there are.
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

/// The audio-in pill, painted, and the card under it.
///
/// Where everything goes is [`audio_in`]'s, so this paints and derives
/// nothing. Term for term from `.pill` and `.pill.armed` in `style.css`:
///
/// - shut, with nothing open: the ordinary capsule — `border: 1px solid
///   var(--c-line); color: var(--c-dim)` — which is [`arrangement_into`]'s
///   treatment and this is the same pill two places along the same row.
/// - with an input open, `.armed`: `border-color: transparent; color:
///   var(--c-mint); background: color-mix(in srgb, var(--c-mint) 14%,
///   transparent)`, which is the mock's own class on this pill and the one
///   place in this row a colour means *live*. The `box-shadow: 0 0 9px
///   var(--c-glow)` goes with it, exactly as the beat grid's lit dot carries
///   its halo.
///
/// The colour and the word say the same thing on purpose. A pill that was
/// only lit would leave *which* room is being heard unanswered, and a pill
/// that only carried a name would make a dead input and a live one look alike
/// at the distance a panel is read from.
pub(crate) fn audio_in_into(ui: &Ui, pal: &Palette, pill: &AudioInPill, audio: &AudioIn) {
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
    chevron_down(painter, pill.chevron, ink);

    let Some(card) = pill.menu else {
        return;
    };
    popup_card(painter, pal, card);

    // **A machine with no inputs**, which is a sentence and not a list —
    // `pill.rows` is zero, so `row` hands out nothing.
    if audio.inputs.is_empty() {
        let foot = Rect::from_min_size(
            Pos2::new(
                card.min.x + size::LIB_LIST_PAD,
                card.min.y + size::LIB_LIST_PAD,
            ),
            egui::vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        );
        card_row_text(painter, foot, NO_INPUTS, pal.faint);
        return;
    }

    for index in 0..pill.rows {
        let row = pill.row(index);
        let input = &audio.inputs[index];
        // **The one that is open is the one colour the list has**, for the
        // reason the pill has one: a card of names with nothing marked leaves
        // an operator to remember which they picked.
        let colour = match audio.device.as_deref() == Some(input.as_str()) {
            true => pal.mint,
            false => pal.text,
        };
        card_row_text(painter, row, input, colour);
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
