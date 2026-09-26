use super::*;

// ---------------------------------------------------------------------------
// The Inspector's parameters and sensor sources
// ---------------------------------------------------------------------------

/// The word in a sensitivity row's left-hand track, `.sens`'s first column.
pub const SENS_LABEL: &str = "sensitivity";

/// The word on a sensitivity row's last chip, which is the second rule's other
/// half.
pub const TAKE_BACK: &str = "take back";

/// Glyph shown in the leftmost cell when a parameter is not published
/// on the interface (ADR-0329).
const UNPUBLISHED: &str = "·";

/// One parameter row: what the mock's `.param` reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// 1-based position in the deck's published interface, or `None` if
    /// unpublished ([ADR-0100](../../../../docs/adr/0100-a-published-interface-is-a-choice-of-attention.md), ADR-0329).
    pub ord: Option<usize>,
    /// What the Set published it as, which may be an alias for the key inside the
    /// node.
    pub name: String,
    /// What it holds.
    pub value: f32,
    /// Published range `[low, high]` used to map fader position and grip
    /// ([ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md)).
    pub range: [f32; 2],
    /// Target parameter address identifying the control across wildcards and nodes
    /// ([ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md), [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    pub param: karakuri_operation::ParamAt,
    /// Active signal source driving this control, or `None` if manual (ADR-0191).
    pub bound: Option<Source>,
}

/// Describes the signal source and mapping driving a parameter row.
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
    /// Binding address identifying the layer attachment for operations like take-back.
    pub at: karakuri_operation::BindAt,
}

impl Param {
    /// Normalized position in `[0, 1]` within the published range (defaulting to 0.0
    /// for zero-width ranges).
    pub fn at(&self) -> f32 {
        let [low, high] = self.range;
        match high > low {
            false => 0.0,
            true => unit((self.value - low) / (high - low)),
        }
    }

    /// Computes the parameter value corresponding to normalized position `at` in `[0, 1]`.
    pub fn valued(&self, at: f32) -> f32 {
        let [low, high] = self.range;
        low + (high - low) * unit(at)
    }

    /// Returns the published control definition for this parameter row
    /// ([ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md), [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
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

/// Active parameter fader grip state during interaction, tracking deck, control, and range.
///
/// See ADR-0286 for parameter fader range representation.
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

/// Total height of a parameter row including its sensitivity row if bound.
pub(crate) fn rows_h(param: &Param) -> f32 {
    size::PARAM_H
        + match param.bound {
            None => 0.0,
            Some(_) => size::SENS_H,
        }
}

/// Chip on a parameter's sensitivity row representing signal source, curve,
/// range, or take-back action ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
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

    /// The operation requested by clicking this chip, or `None` for readout chips.
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

/// Layout positions of each chip in a sensitivity row, measured from left to right.
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

/// Paints a single parameter row: ordinal, name, fader track/knob, and value readout.
pub(crate) fn param_into(painter: &egui::Painter, pal: &Palette, row: Rect, param: &Param) {
    let left = row.min.x + size::PARAM_PAD_L;
    let right = row.max.x - size::PARAM_PAD_R;
    // Display ordinal number if published, or hairline dot if unpublished (ADR-0329).
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
    // Ellipsize parameter name when it exceeds column width, muting ink if unpublished.
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
    // Unpublished rows omit the fader track and value readout.
    if param.ord.is_none() {
        return;
    }
    // Bound parameters display their signal source name instead of numeric value.
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

/// Paints a sensitivity row beneath a bound parameter showing its active signal chips.
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

/// Calculates fader track and knob geometry within a parameter row (ADR-0279).
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

/// Computes the bounding rectangle for a parameter row within its node group.
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

/// Returns the hit-test rectangle for the parameter publishing ordinal cell (ADR-0329).
pub(crate) fn ord_cell(row: Rect, _param: &Param) -> Rect {
    let left = row.min.x + size::PARAM_PAD_L;
    Rect::from_min_max(
        Pos2::new(left, row.min.y),
        Pos2::new(left + size::PARAM_ORD_W, row.max.y),
    )
}

/// Returns the bounding rectangle for a parameter's sensitivity row, if bound.
pub(crate) fn sens_rect(group: Rect, node: &Node, index: usize) -> Option<Rect> {
    let param = node.params.get(index)?;
    param.bound.as_ref()?;
    let row = param_rect(group, node, index);
    Some(Rect::from_min_max(
        Pos2::new(row.min.x, row.max.y),
        Pos2::new(row.max.x, row.max.y + size::SENS_H),
    ))
}
