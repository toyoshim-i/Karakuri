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

/// All sync modes in order: Free, Tempo, Beat.
pub const SYNCS: [Sync; 3] = [Sync::Free, Sync::Tempo, Sync::Beat];

/// Returns the next supported sync mode after `now`, skipping modes unsupported by the material.
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

/// Formats anchor text for tempo/beat sync modes, or `None` under Free sync.
pub(crate) fn anchor_text(pane: &Pane) -> Option<String> {
    let letter = anchor_letter(pane.sync)?;
    Some(match pane.sync {
        Sync::Beat => format!("{letter}{:.0} {:+.2}", pane.anchor_bpm, pane.scrub_beats),
        _ => format!("{letter}{:.0}", pane.anchor_bpm),
    })
}

/// Step distance per scrub press in beats (0.25 beats).
pub const SCRUB_BEATS: f64 = 0.25;

/// What the `re-salt` capsule reads.
pub const RE_SALT_LABEL: &str = "re-salt";

/// Returns the next capacity value from `list` cycling after `now`.
fn stepped_capacity(candidates: &[u32], at: u32) -> Option<u32> {
    candidates
        .iter()
        .find(|candidate| **candidate > at)
        .or_else(|| candidates.first())
        .copied()
}

/// Laid-out controls in a deck head (sync chip, anchor, scrub arrows, fold chip, capacity chips).
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
    /// Bounding rectangle for the compositing fold chip, or `None`.
    pub composite: Rect,
    /// Capacity and re-salt chip rectangles, or `None` if unconfigured.
    pub aim: Option<AimChips>,
    /// Which deck this head belongs to, as [`Operation::SetSync`],
    /// [`Operation::ScrubDeck`] and [`Operation::SetCompositing`] each name one —
    /// [`Pane::deck`], carried so that a press answers with the deck it was
    /// measured for.
    pub deck: usize,
    /// Whether the fold chip is currently enabled.
    pub composited: bool,
    /// What the mode chip is showing, and what re-anchoring re-asks for. Carried
    /// for [`LookRow::values`]' reason: whoever measured this row and whoever acts
    /// on a press in it are one statement.
    pub locked: Sync,
    /// What this deck's material can honour, [`Pane::allows`] as it was read — the
    /// whole of what the cycle skips on.
    pub allows: [bool; SYNCS.len()],
}

/// Layout and target values for capacity stepping and re-salting chips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimChips {
    /// The capacity chip, reading [`Aimed::capacity`]. `.mini`'s box round the
    /// number, one gap left of [`AimChips::salt`].
    pub size: Rect,
    /// Target capacity value after stepping, or `None`.
    pub resize: Option<u32>,
    /// The `re-salt` capsule, one gap left of [`DeckHead::composite`].
    pub salt: Rect,
    /// Next salt value requested by re-salt chip.
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

    /// Hit-tests scrub arrows at `p`, returning signed beat delta or `None`.
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

    /// Returns whether `p` lands on any deck head control.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.hit_mode(p)
            || self.hit_anchor(p)
            || self.arrow(p).is_some()
            || self.hit_size(p)
            || self.hit_salt(p)
            || self.hit_composite(p)
    }

    /// Returns [`Operation::SetSync`] cycling sync mode at `p` per [P-0090] and [ADR-0187].
    pub fn sync(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_mode(p).then(|| Operation::SetSync {
            deck: self.deck as u8,
            sync: next_sync(self.locked, self.allows),
        })
    }

    /// Returns [`Operation::SetSync`] to re-anchor current sync mode at `p` per [ADR-0218].
    pub fn reanchor(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_anchor(p).then_some(Operation::SetSync {
            deck: self.deck as u8,
            sync: self.locked,
        })
    }

    /// Returns [`Operation::ScrubDeck`] to nudge playback by [`SCRUB_BEATS`] per [ADR-0207].
    pub fn scrub(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.arrow(p).map(|beats| Operation::ScrubDeck {
            deck: self.deck as u8,
            beats,
        })
    }

    /// Returns [`Operation::SetCompositing`] toggling layer compositing per [ADR-0314] and ADR-0156.
    pub fn compositing(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_composite(p).then_some(Operation::SetCompositing {
            deck: self.deck as u8,
            compositing: !self.composited,
        })
    }

    /// Returns [`Operation::SetProperty`] stepping slot capacity per [P-0091] and [ADR-0262].
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

    /// Returns [`Operation::SetProperty`] with the next deterministic salt per [P-0092].
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

/// Derives deck head control layout within pane bounds per [P-0094].
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
    // Drop build chips first if pane is too narrow to preserve core controls per P-0094.
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

/// Paints deck head controls: sync chip, anchor, scrub arrows, and compositing fold.
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
