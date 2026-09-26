use super::*;

pub mod strip;
pub mod transition;

pub use strip::*;
pub(crate) use strip::{next, next_shape, residency, wipe_kind};
pub use transition::*;

/// Where a slot sits between compiled and composited (ADR-0178).
pub use super::widgets::chip::{centre_galley, tally_job, Tally};

// ---------------------------------------------------------------------------
// The Mixer bay
// ---------------------------------------------------------------------------

/// The word at the head of the Mixer bay, in the source's own capitalisation
/// for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and that is done at
/// paint time so the word a reader searches for is the word in the source.
pub(super) const MIXER_TITLE: &str = "Mixer";

/// Layout geometry and strip references for the Mixer bay.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mixer<'a> {
    /// The values these rectangles were measured from, in slot order.
    pub strips: &'a [Strip],
    /// Where each of them is. `None` past the last strip: a track on the page no
    /// deck fills, which is drawn as nothing at all.
    boxes: [Option<StripBox>; DECKS],
}

impl<'a> Mixer<'a> {
    /// How many strips there are — the deck's slot count, as far as one page of
    /// this bay reaches.
    pub fn count(&self) -> usize {
        self.boxes.iter().filter(|at| at.is_some()).count()
    }

    /// One strip's furniture. Panics on a strip this bay has not got, which is a
    /// caller having invented a slot — the rule [`TransportRow::dot`] states about
    /// a dot of a grid.
    pub fn strip(&self, index: usize) -> StripBox {
        let count = self.count();
        self.boxes
            .get(index)
            .copied()
            .flatten()
            .unwrap_or_else(|| panic!("strip {index} of a mixer of {count}"))
    }

    /// Returns the fader or trim [`Grab`] under point `p`, if any.
    pub fn grab(&self, p: karakuri_layout::Point) -> Option<Grab> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                let at = at?;
                // The manual's *deck* is the code's *slot*, and a deck holds
                // `MAX_SLOTS` of them — `DECKS`, which is 4 — so the index is
                // a `u8` with room to spare. See `Knob::operation`.
                let deck = index as u8;
                grabbed(at.trim_at(strip.gain), Knob::Trim { deck }, p)
                    .or_else(|| grabbed(at.fader_at(strip.opacity), Knob::Fader { deck }, p))
            })
    }

    /// Returns the [`Operation::SetBlendMode`] to cycle the blend mode under point `p`, or `None`.
    ///
    /// See ADR-0187 for blend cycling details.
    pub fn blend(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.blend.contains(p))
                    .map(|_| Operation::SetBlendMode {
                        deck: index as u8,
                        blend: after(strip.blend),
                    })
            })
    }

    /// What a press at `p` asks the deck's solo state to become, or `None` where
    /// there is no solo button under it.
    pub fn solo(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.solo.contains(p))
                    .map(|_| Operation::SetSolo {
                        deck: index as u8,
                        solo: !strip.is_soloed,
                    })
            })
    }

    /// What a press at `p` asks the deck's mute state to become, or `None` where
    /// there is no mute button under it.
    pub fn mute(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.mute.contains(p))
                    .map(|_| Operation::SetMute {
                        deck: index as u8,
                        mute: !strip.is_muted,
                    })
            })
    }

    /// Returns the [`Operation::SetResidency`] to cycle the tally state under `p` (ADR-0187, ADR-0195, P-0090).
    pub fn tally(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.tally.contains(p))
                    .map(|_| Operation::SetResidency {
                        deck: index as u8,
                        residency: residency(next(strip.requested)),
                    })
            })
    }

    /// Returns the [`Operation::SetMaskShape`] to cycle the mask shape under `p` (ADR-0156, ADR-0203, P-0090).
    pub fn mask(&self, p: karakuri_layout::Point) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        self.strips
            .iter()
            .zip(self.boxes)
            .enumerate()
            .find_map(|(index, (strip, at))| {
                at.filter(|at| at.mask.contains(p))
                    .map(|_| Operation::SetMaskShape {
                        deck: index as u8,
                        kind: wipe_kind(next_shape(strip.mask)),
                        angle: strip.mask_angle,
                    })
            })
    }

    /// Returns the [`Operation::SelectDeck`] to select the deck strip under `p`.
    pub fn select(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.deck_at(p).map(|deck| Operation::SelectDeck { deck })
    }

    /// Returns the deck index if a drag-and-drop was released over a strip at `p` (ADR-0273, P-0090).
    pub fn dropped(&self, p: karakuri_layout::Point) -> Option<u8> {
        self.deck_at(p)
    }

    /// Returns the deck index whose strip contains `p`, if any.
    fn deck_at(&self, p: karakuri_layout::Point) -> Option<u8> {
        let p = Pos2::new(p.x, p.y);
        self.boxes
            .iter()
            .enumerate()
            .find_map(|(index, at)| at.filter(|at| at.rect.contains(p)).map(|_| index as u8))
    }

    /// Returns the full strip bounding box for `deck` for drawing the selection ring.
    pub fn selected(&self, deck: u8) -> Option<Rect> {
        self.boxes
            .get(usize::from(deck))
            .copied()
            .flatten()
            .map(|at| at.rect)
    }

    /// Every strip and its box, in slot order.
    pub fn placed(&self) -> impl Iterator<Item = (&'a Strip, StripBox)> {
        let boxes = self.boxes;
        self.strips
            .iter()
            .zip(boxes)
            .filter_map(|(strip, at)| at.map(|at| (strip, at)))
    }
}

/// Derives layout boxes for all channel strips in the Mixer bay (ADR-0177, ADR-0178, ADR-0219).
pub fn mixer<'a>(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    strips: &'a [Strip],
) -> Option<Mixer<'a>> {
    // **No deck behind the console, so there are no strips.** Every test in
    // this crate is here, and so is the whole of `cargo test -p
    // karakuri-console`. Drawing four empty strips would be inventing six
    // readings a slot; this is `View::picture`'s rule, one bay along.
    if strips.is_empty() {
        return None;
    }
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // `outputs` and `transport`.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let region = to_egui(layout.rect(layout.find("mixer")?));
    let row = strips_row(region)?;
    let width = |job: LayoutJob| ctx.fonts_mut(|f| f.layout_job(job).size().x);
    let label = width(span_at(
        TRIM_LABEL,
        size::TRIM_LABEL_SIZE,
        Color32::PLACEHOLDER,
    ));
    // Sized to the widest residency word to prevent resizing during animation.
    let tally = Tally::ALL
        .into_iter()
        .map(|tally| width(tally_job(tally, Color32::PLACEHOLDER)))
        .fold(0.0f32, f32::max);
    let mut boxes = [None; DECKS];
    for (index, strip) in strips.iter().take(DECKS).enumerate() {
        let blend = width(span_at(
            strip.blend.name(),
            size::MINI_SIZE,
            Color32::PLACEHOLDER,
        ));
        boxes[index] = strip_box(
            track(row, DECKS, index, size::STRIP_GAP, Axis::Row),
            label,
            tally,
            blend,
        );
    }
    Some(Mixer { strips, boxes })
}

/// Paints the mixer channel strips, drop highlight ring, and selection focus ring.
pub(super) fn mixer_into(
    ui: &Ui,
    pal: &Palette,
    mixer: &Mixer,
    phase: Phase,
    selection: u8,
    marked: Option<u8>,
) {
    for (strip, at) in mixer.placed() {
        strip_into(ui, pal, strip, at, phase);
    }

    // Drop target highlight ring (outer outline) when a drag hovers over this strip.
    if let Some(rect) = marked.and_then(|deck| mixer.selected(deck)) {
        drop_ring(ui, pal, rect, size::STRIP_RADIUS);
    }
    // Deck selection focus ring (inner lavender outline) drawn over strip contents.
    if let Some(rect) = mixer.selected(selection) {
        ui.painter().with_clip_rect(rect).rect_stroke(
            rect,
            CornerRadius::same(size::STRIP_RADIUS as u8),
            Stroke::new(size::STRIP_FOCUS_RING, pal.lav),
            StrokeKind::Inside,
        );
    }
}

impl View {
    /// Returns the current transition settings for wipes and crossfades.
    pub fn transition(&self) -> TransitionSettings {
        self.transition
    }

    /// Applies a transition setting if curated, returning whether it changed (P-0090, P-0091).
    pub fn set_transition(&mut self, setting: TransitionSetting) -> bool {
        self.transition.take(setting)
    }

    /// Declares frame timing requirements for active mixer animations (ADR-0283, ADR-0290, P-0094).
    pub(super) fn mixer_declares(&self, layout: &karakuri_layout::Layout) -> Option<Declared> {
        let bay = layout.find("mixer").is_some_and(|id| layout.visible(id));
        if !bay {
            return None;
        }
        if self.mixer_dirty {
            return Some(Declared {
                region: "mixer",
                cost: PANEL_PASS,
                staleness: ROLL_STALENESS,
                moves_in: Duration::ZERO,
            });
        }
        let moving = self.mixer.iter().any(|strip| {
            strip.pending().is_some()
                || strip.gain_pending().is_some()
                || strip.opacity_pending().is_some()
        });
        moving.then_some(Declared {
            region: "mixer",
            cost: PANEL_PASS,
            staleness: ROLL_STALENESS,
            moves_in: roll_moves_in(self.phase),
        })
    }
}
