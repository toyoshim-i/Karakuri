use super::*;

// ---------------------------------------------------------------------------
// Scopes and path
// ---------------------------------------------------------------------------

/// One chip in the Library bay's scope row, selecting which library or version history to display.
///
/// Ref: ADR-0275 (folder scope via drag-and-drop), ADR-0299 (starred sets in `favourites.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// `all`: everything this store holds (`karakuri_store::Store::list_sets`).
    AllSets,
    /// `my sets`: the starred subset of [`Scope::AllSets`] (ADR-0299).
    MySets,
    /// Built-in `.kset` files from the presets root directory (ADR-0230, ADR-0299).
    Presets,
    /// A directory somebody names during the run.
    Folder,
    /// `history`: Set version history under `<store>/history/` for the aimed Set (ADR-0276, ADR-0308).
    History,
}

impl Scope {
    /// All available scopes in display order.
    pub const ALL: [Scope; 5] = [
        Scope::AllSets,
        Scope::MySets,
        Scope::Presets,
        Scope::Folder,
        Scope::History,
    ];

    /// The chip's word, `style.css`'s own — lower case, because `.scope` sets no
    /// `text-transform` where a bay head does.
    pub fn name(self) -> &'static str {
        match self {
            Scope::AllSets => "all",
            Scope::MySets => "my sets",
            Scope::Presets => "presets",
            Scope::Folder => "folder",
            Scope::History => "history",
        }
    }

    /// Returns true if this scope lists Sets rather than versions ([`Scope::History`]).
    pub fn lists_sets(self) -> bool {
        !matches!(self, Scope::History)
    }
}

/// Selection result when clicking a scope chip, pairing the scope with its corresponding operation.
///
/// Emits [`Operation::WalkHistory`] for [`Scope::History`], or [`Operation::SelectScope`] otherwise (P-0090).
#[derive(Debug, Clone, PartialEq)]
pub struct Chosen {
    /// The chip the pointer was on, which is a value of the row this console was
    /// handed rather than a position in it: the caller marks it through
    /// [`View::select_scope`], which refuses a scope with no chip.
    pub scope: Scope,
}

impl Chosen {
    /// Returns the operation for this scope selection (ADR-0276, ADR-0308).
    #[must_use]
    pub fn asked(&self, aimed: Option<&str>) -> Operation {
        match self.scope {
            Scope::History => Operation::WalkHistory {
                set: aimed.map(str::to_owned),
            },
            _ => Operation::SelectScope { scope: Undecided },
        }
    }
}

/// Computes width of a scope chip using font measurements from `egui`.
fn chip_width(ctx: &egui::Context, name: &str) -> f32 {
    if ctx.cumulative_pass_nr() == 0 {
        return size::SCOPE_PAD_X * 2.0;
    }
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            name.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    }) + size::SCOPE_PAD_X * 2.0
}

impl LibraryBay {
    /// Returns an iterator over scope chips and their layout bounding boxes.
    pub fn chips<'a>(
        &self,
        ctx: &'a egui::Context,
        scopes: &'a [Scope],
    ) -> impl Iterator<Item = (Scope, Rect)> + 'a {
        let row = self.scopes;
        let mut x = row.map_or(0.0, |row| row.min.x + size::SCOPES_PAD_X);
        let top = row.map_or(0.0, |row| row.min.y + size::SCOPES_PAD_Y);
        let drawn = row.map_or(0, |_| scopes.len());
        scopes.iter().take(drawn).map(move |scope| {
            let chip = Rect::from_min_size(
                Pos2::new(x, top),
                egui::vec2(chip_width(ctx, scope.name()), size::SCOPE_H),
            );
            x += chip.width() + size::SCOPES_GAP;
            (*scope, chip)
        })
    }

    /// Returns the scope chip pressed at point `p`, or `None` if outside chips (P-0090).
    pub fn chip(
        &self,
        ctx: &egui::Context,
        scopes: &[Scope],
        p: karakuri_layout::Point,
    ) -> Option<Chosen> {
        let row = self.scopes?;
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let p = Pos2::new(p.x, p.y);
        if !row.contains(p) {
            return None;
        }
        self.chips(ctx, scopes)
            .find(|(_, chip)| chip.contains(p))
            .map(|(scope, _)| Chosen { scope })
    }
}

/// Paints the scope row chips, highlighting the active chip and drawing the bottom hairline rule.
pub(crate) fn scopes_into(
    ui: &Ui,
    pal: &Palette,
    bay: &LibraryBay,
    scopes: &[Scope],
    scope: usize,
) {
    let Some(row) = bay.scopes else {
        return;
    };
    let painter = ui.painter().with_clip_rect(row);
    for (at, (kind, chip)) in bay.chips(ui.ctx(), scopes).enumerate() {
        let marked = at == scope;
        scope_tab(&painter, chip, marked, kind.name(), pal);
    }

    let rule = row.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [Pos2::new(row.min.x, rule), Pos2::new(row.max.x, rule)],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
}

/// Renders the path row showing directory breadcrumbs, highlighted when incoming (ADR-0275).
pub(crate) fn path_into(ui: &Ui, pal: &Palette, bay: &LibraryBay, at: Pointed<'_>) {
    let Some(row) = bay.path else {
        return;
    };
    let painter = ui.painter().with_clip_rect(row);
    let ink = match at.incoming {
        true => pal.text,
        false => pal.faint,
    };
    let galley = painter.layout_job(span_at(at.path, size::PATH_SIZE, ink));
    painter.galley(
        Pos2::new(
            row.min.x + size::PATH_PAD_X,
            row.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );

    let rule = row.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [Pos2::new(row.min.x, rule), Pos2::new(row.max.x, rule)],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
}
