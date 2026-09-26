use super::*;

// ---------------------------------------------------------------------------
// The library's filter field and its kind chips
// ---------------------------------------------------------------------------

/// Placeholder string displayed when the node name filter is unset.
pub const HOLDS_UNSET: &str = "holds…";

/// Layer names and display labels for kind filter chips (ADR-0338, ADR-0340).
pub const LAYERS: [(Layer, &str); 5] = [
    (Layer::L1, "L1"),
    (Layer::L2, "L2"),
    (Layer::L3, "L3"),
    (Layer::L4, "L4"),
    (Layer::Field, "FIELD"),
];

/// The word the `SET` chip carries, and the one place it is spelled — the sixth
/// of the six, and the only one that is not a [`Layer`].
pub const SETS_CHIP: &str = "SET";

/// Kind filter chip representing a layer or full Set row (ADR-0338, P-0090).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindChip {
    /// One of the five kinds a procedure declares.
    Layer(Layer),
    /// The Sets, which is the row kind that declares no `kind` at all.
    Sets,
}

impl KindChip {
    /// All kind chips in display order (ADR-0338, ADR-0340).
    pub const ALL: [KindChip; 6] = [
        KindChip::Layer(Layer::L1),
        KindChip::Layer(Layer::L2),
        KindChip::Layer(Layer::L3),
        KindChip::Layer(Layer::L4),
        KindChip::Layer(Layer::Field),
        KindChip::Sets,
    ];

    /// Returns the display label for the kind chip.
    pub fn word(self) -> &'static str {
        match self {
            KindChip::Sets => SETS_CHIP,
            KindChip::Layer(layer) => LAYERS
                .iter()
                .find(|(kind, _)| *kind == layer)
                .map_or(SETS_CHIP, |(_, word)| *word),
        }
    }

    /// Returns true if this kind chip toggle is active.
    pub fn on(self, kinds: LibraryKinds) -> bool {
        match self {
            KindChip::Sets => kinds.sets,
            KindChip::Layer(Layer::L1) => kinds.l1,
            KindChip::Layer(Layer::L2) => kinds.l2,
            KindChip::Layer(Layer::L3) => kinds.l3,
            KindChip::Layer(Layer::L4) => kinds.l4,
            KindChip::Layer(Layer::Field) => kinds.field,
            // Supported in vocabulary, awaiting UI chip slot (ADR-0340).
            KindChip::Layer(Layer::L5) => kinds.l5,
        }
    }

    /// All six with this one turned the other way, which is what a press on it asks
    /// for — the surface's arithmetic, and the whole of it.
    pub fn flipped(self, kinds: LibraryKinds) -> LibraryKinds {
        let mut kinds = kinds;
        let want = !self.on(kinds);
        match self {
            KindChip::Sets => kinds.sets = want,
            KindChip::Layer(Layer::L1) => kinds.l1 = want,
            KindChip::Layer(Layer::L2) => kinds.l2 = want,
            KindChip::Layer(Layer::L3) => kinds.l3 = want,
            KindChip::Layer(Layer::L4) => kinds.l4 = want,
            KindChip::Layer(Layer::Field) => kinds.field = want,
            KindChip::Layer(Layer::L5) => kinds.l5 = want,
        }
        kinds
    }
}

/// Identifies which text filter field was clicked (ADR-0338).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// `holds…` — what a node in the Set is called.
    Holds,
}

impl Field {
    /// The row's fields, left to right, in the order `.lib-filters` draws them.
    pub const ALL: [Field; 1] = [Field::Holds];
}

/// Borrowed filter state (`holds` string and `kinds` flags) for rendering and input handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Filters<'a> {
    /// Part of a node's name, or `None` for a field nobody has set.
    pub holds: Option<&'a str>,
    /// Active kind filter flags (ADR-0338).
    pub kinds: LibraryKinds,
}

impl Filters<'_> {
    /// Nothing narrowed at all, which is where a run begins and is every test in
    /// this crate that does not say otherwise.
    pub const NONE: Filters<'static> = Filters {
        holds: None,
        kinds: LibraryKinds::EVERYTHING,
    };

    /// What the `holds` field reads: the filter, or [`HOLDS_UNSET`].
    pub fn holds_word(&self) -> &str {
        self.holds.unwrap_or(HOLDS_UNSET)
    }

    /// The word this field reads.
    pub fn word(&self, field: Field) -> &str {
        match field {
            Field::Holds => self.holds_word(),
        }
    }
}

/// Cycles the `holds` filter candidate to the next available choice or wraps to none.
fn stepped_holds(choices: &[String], at: Option<&str>) -> Option<String> {
    let next = match at {
        None => 0,
        Some(at) => choices
            .iter()
            .position(|choice| choice == at)
            .map_or(0, |at| at + 1),
    };
    choices.get(next).cloned()
}

impl LibraryBay {
    /// Returns bounding box for the given filter field (ADR-0338).
    pub fn field(&self, which: Field) -> Option<Rect> {
        let row = self.filters?;
        let width = row.width() - size::LIB_FILTERS_PAD_X * 2.0;
        let at = match which {
            Field::Holds => row.min.x + size::LIB_FILTERS_PAD_X,
        };
        Some(Rect::from_min_size(
            Pos2::new(at, row.min.y + size::LIB_FILTERS_PAD_Y),
            egui::vec2(width, size::FIELD_H),
        ))
    }

    /// Returns an iterator over kind filter chips and their layout bounding boxes.
    pub fn kind_chips<'a>(
        &self,
        ctx: &'a egui::Context,
    ) -> impl Iterator<Item = (KindChip, Rect)> + 'a {
        let row = self.kinds;
        let mut x = row.map_or(0.0, |row| row.min.x + size::LIB_KINDS_PAD_X);
        let top = row.map_or(0.0, |row| row.min.y + size::LIB_KINDS_PAD_Y);
        let drawn = row.map_or(0, |_| KindChip::ALL.len());
        KindChip::ALL.into_iter().take(drawn).map(move |chip| {
            let width = if ctx.cumulative_pass_nr() == 0 {
                size::KIND_PAD_X * 2.0
            } else {
                ctx.fonts_mut(|f| {
                    f.layout_no_wrap(
                        chip.word().to_owned(),
                        FontId::new(size::KIND_SIZE, FontFamily::Proportional),
                        Color32::PLACEHOLDER,
                    )
                    .size()
                    .x
                }) + size::KIND_PAD_X * 2.0
            };
            let box_ = Rect::from_min_size(Pos2::new(x, top), egui::vec2(width, size::KIND_H));
            x += width + size::LIB_KINDS_GAP;
            (chip, box_)
        })
    }

    /// Returns the operation to toggle the kind chip at `p`, or `None` if unhit (ADR-0338, P-0090).
    pub fn kind(
        &self,
        ctx: &egui::Context,
        at: Filters<'_>,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let row = self.kinds?;
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let p = Pos2::new(p.x, p.y);
        if !row.contains(p) {
            return None;
        }
        self.kind_chips(ctx)
            .find(|(_, box_)| box_.contains(p))
            .map(|(chip, _)| Operation::FilterLibrary {
                kinds: chip.flipped(at.kinds),
            })
    }

    /// Returns the operation resulting from clicking a filter field at `p`, or `None` (ADR-0156, ADR-0221, P-0090).
    pub fn filter(
        &self,
        holds: &[String],
        at: Filters<'_>,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        let which = Field::ALL
            .into_iter()
            .find(|field| self.field(*field).is_some_and(|box_| box_.contains(p)))?;
        Some(match which {
            // Layer filter was superseded by kind chips (ADR-0338); holds filters by node name.
            Field::Holds => Operation::ListSets {
                holds: stepped_holds(holds, at.holds),
                layer: None,
            },
        })
    }
}

/// Paints the filter row fields and bottom hairline rule.
pub(crate) fn filters_into(ui: &Ui, pal: &Palette, bay: &LibraryBay, at: Filters<'_>) {
    let Some(row) = bay.filters else {
        return;
    };
    let painter = ui.painter().with_clip_rect(row);
    for field in Field::ALL {
        let Some(box_) = bay.field(field) else {
            continue;
        };
        filter_field(&painter, pal, box_, at.word(field));
    }

    let rule = row.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [Pos2::new(row.min.x, rule), Pos2::new(row.max.x, rule)],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
}

/// Paints kind filter chips, highlighting active toggles in mint, with bottom hairline rule.
pub(crate) fn kinds_into(ui: &Ui, pal: &Palette, bay: &LibraryBay, at: Filters<'_>) {
    let Some(row) = bay.kinds else {
        return;
    };
    let painter = ui.painter().with_clip_rect(row);
    for (chip, box_) in bay.kind_chips(ui.ctx()) {
        let on = chip.on(at.kinds);
        toggle_chip(
            &painter,
            box_,
            on,
            chip.word(),
            size::KIND_SIZE,
            pal,
            pal.mint,
        );
    }

    let rule = row.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [Pos2::new(row.min.x, rule), Pos2::new(row.max.x, rule)],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
}
