use super::*;

// ---------------------------------------------------------------------------
// The Mixer bay's transition row
// ---------------------------------------------------------------------------

/// Scheduled transition parameters: wipe mask shape, angle, musical quantum, and length in beats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionSettings {
    /// The shape the next wipe's front takes. [`WipeKind::None`] is *no shape*,
    /// under which `karakuri-cli`'s `c` is refused — which is the vocabulary's own
    /// sentence at that variant, not a rule this row holds.
    pub kind: WipeKind,
    /// Orientation of a linear transition front in radians (ADR-0203).
    pub angle: f32,
    /// The grid the next scheduled move starts on, in beats: 4 for the next bar, 1
    /// for the next beat, 0 for now.
    pub quantum: f64,
    /// How long the next scheduled move lasts, in beats. Zero is a cut.
    pub length: f64,
}

/// Curated wipe mask shape options in cycle order: `(WipeKind, angle_radians, label)`.
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

/// Musical quantize alignments in cycle order: `(beats, label)`.
const QUANTA: [(f64, &str); 3] = [(4.0, "next bar"), (1.0, "next beat"), (0.0, "now")];

/// Fade transition lengths in cycle order: `(beats, label)`.
const FADE_BEATS: [(f64, &str); 4] = [
    (4.0, "4 beats"),
    (2.0, "2 beats"),
    (8.0, "8 beats"),
    (0.0, "cut"),
];

impl TransitionSettings {
    /// Default transition settings: no shape, next bar (4 beats), 4 beats length.
    pub const START: TransitionSettings = TransitionSettings {
        kind: WIPE_SHAPES[0].0,
        angle: WIPE_SHAPES[0].1,
        quantum: QUANTA[0].0,
        length: FADE_BEATS[0].0,
    };

    /// Returns the index of this transition shape in [`WIPE_SHAPES`].
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

    /// Returns true if a non-empty wipe shape is selected (not [`WipeKind::None`]) (P-0090).
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

    /// Applies a transition setting if valid and returns whether any value changed.
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

/// Layout geometry for the mixer transition row (`.xfade`), pills, and go capsule.
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

/// Result of activating the transition go capsule: an operation or refusal reason (P-0083).
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
    /// Cycles to the next wipe shape if `p` is within the shape pill (P-0090).
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

    /// Evaluates activation of the transition go capsule at `p` across active decks.
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

    /// Returns true if the transition row is armed and there are at least two decks to crossfade.
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

    /// Returns true if point `p` falls on any pill or the go capsule in this row.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.shape(p).is_some()
            || self.quantum(p).is_some()
            || self.length(p).is_some()
            || self.go.contains(Pos2::new(p.x, p.y))
    }
}

/// Computes the bounding rectangle for `.xfade` within the mixer bay region.
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

/// Derives the layout geometry of the transition row, pills, and go capsule.
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

/// Label text for the transition trigger capsule.
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
