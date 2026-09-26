use super::*;

/// Recording state for the audio-in pill (`● rec`) (P-0087).
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

/// Label shown when no audio input device is selected (P-0084).
const NO_INPUT: &str = "none";

/// Placeholder text shown when no audio inputs are available on the machine.
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
    /// Initial empty audio-in configuration with no open device or dropdown.
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

    /// Returns the number of visible rows in the open card dropdown (ADR-0350).
    pub fn rows(&self) -> usize {
        match self.down {
            true => self.inputs.len(),
            false => 0,
        }
    }
}

/// User action requested from clicking the audio input pill or dropdown menu.
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

/// Layout metrics and dropdown bounds for the audio input selector pill.
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
    /// Number of menu rows that fit within the viewport height.
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

    /// Returns the bounding box for the row at `index` in the input dropdown card.
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

    /// Hit-tests a click at `p` against the pill or dropdown items.
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

/// Computes layout rectangles for the audio input selector pill and dropdown card.
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

/// Computes position and capacity for the dropdown card below the audio input pill.
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

/// Paints the audio-in pill and dropdown menu card with selection highlights.
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

    // Fallback row displayed when host provides no input devices.
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
        // Active device row is highlighted in mint.
        let colour = match audio.device.as_deref() == Some(input.as_str()) {
            true => pal.mint,
            false => pal.text,
        };
        card_row_text(painter, row, input, colour);
    }
    // Foot summary readout (`n of m`) when list is truncated.
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
