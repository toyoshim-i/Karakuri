use super::*;

pub mod pill;
pub use pill::*;

// ---------------------------------------------------------------------------
// The arrangement pill
// ---------------------------------------------------------------------------

/// The pill's first word, the mock's own abbreviation one pill along from `map
/// · nanoKONTROL2 ▾`. The map pill names a file with *save*, *load* and *start
/// a new one* under it, and this is the same three over a different file, so it
/// is the same two-part label.
const ARRANGEMENT_LABEL: &str = "arr";

/// Fallback label when using the unnamed default layout (ADR-0221).
const NO_ARRANGEMENT: &str = "the default";

/// What *save* is called in the menu. The ellipsis is the one thing on this
/// panel that says *this item asks for something before it does anything*, and
/// it is only there while it is true: with a name in use, saving again means
/// that name and asks for nothing, so the word loses it.
const SAVE_ITEM: &str = "save";
const SAVE_ITEM_ASKING: &str = "save as…";

/// Label for resetting to a default arrangement in the arrangement menu.
const NEW_ITEM: &str = "start a new one";

/// Current arrangement state and dropdown menu presentation.
#[derive(Debug, Clone, PartialEq)]
pub struct Arrangement {
    /// Active arrangement name, or `None` for the unnamed default layout.
    pub name: Option<String>,
    /// Saved arrangement names retrieved from storage, sorted alphabetically (P-0091).
    pub filed: Vec<String>,
    /// What the control is doing. See [`Menu`].
    pub menu: Menu,
}

/// Modal state of the arrangement dropdown menu (closed, open list, or naming text field).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Menu {
    /// The pill alone.
    #[default]
    Shut,
    /// The menu is down: *save*, *start a new one*, and the names filed.
    Open,
    /// Text input buffer for naming an arrangement being saved (ADR-0156, ADR-0221, P-0090).
    Naming(String),
}

impl Arrangement {
    /// Default arrangement configuration with no active saved name and menu closed.
    pub const NONE: Arrangement = Arrangement {
        name: None,
        filed: Vec::new(),
        menu: Menu::Shut,
    };

    /// What the pill says after `arr ·`: the name in use, or the word for the
    /// arrangement that has none. See [`NO_ARRANGEMENT`].
    pub fn word(&self) -> &str {
        self.name.as_deref().unwrap_or(NO_ARRANGEMENT)
    }

    /// The menu is down, whether or not a name is being typed into it.
    pub fn open(&self) -> bool {
        !matches!(self.menu, Menu::Shut)
    }

    /// What has been typed so far, or `None` when nothing is asking for a name. The
    /// caret is drawn after it and there is no selection: this is a name, not a
    /// document.
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

    /// Take it away, typed name and all. A name abandoned half-typed is not kept
    /// for the next time the menu opens: the buffer is the gesture, and the gesture
    /// ended.
    pub fn shut(&mut self) {
        self.menu = Menu::Shut;
    }

    /// Ask for a name, starting from empty. Reached only where there is no name in
    /// use — with one in use, *save* means that name and asks nothing
    /// ([`ArrangementPill::ask`]).
    pub fn asks_a_name(&mut self) {
        self.menu = Menu::Naming(String::new());
    }

    /// Appends a printable character to the active naming buffer, returning true if accepted.
    pub fn typed(&mut self, c: char) -> bool {
        match (&mut self.menu, c.is_control()) {
            (Menu::Naming(name), false) => {
                name.push(c);
                true
            }
            _ => false,
        }
    }

    /// The last character back out again, and `false` where there was nothing to
    /// take — no name being typed, or an empty one.
    pub fn rubbed_out(&mut self) -> bool {
        match &mut self.menu {
            Menu::Naming(name) => name.pop().is_some(),
            _ => false,
        }
    }

    /// Returns the number of menu rows available (save, reset, and filed items) (ADR-0350).
    pub fn rows(&self) -> usize {
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

/// The two items above the list: *save* and *start a new one*. The names filed
/// follow them, and *load* is that list rather than an item of its own — which
/// is the manual's own sentence: *"load is a list — the names already filed,
/// which a hand can pick without typing anything."*
const VERBS: usize = 2;

/// Categorizes which item or action in the arrangement menu was selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// *save*. With no name in use it asks for one; with a name in use it means
    /// that name.
    Save,
    /// *start a new one*, which is the reset.
    New,
    /// The name at this index of [`Arrangement::filed`] — put that arrangement
    /// back.
    Filed(usize),
}

/// Outcome or operation requested by interacting with the arrangement pill/menu (P-0090).
#[derive(Debug, Clone, PartialEq)]
pub enum Ask {
    /// Put the menu down — a press on the pill with the menu shut.
    Open,
    /// Take it away — a press on the pill again, or anywhere off the menu while it
    /// is down.
    Shut,
    /// Ask for a name: *save* with no arrangement in use. The one item on this
    /// panel that asks for letters.
    Name,
    /// The reset, which reaches code and not a file, and is the same [`Op`] the `r`
    /// key performs — one operation, two surfaces
    /// ([ADR-0208](../../../../docs/adr/0208-resetting-is-the-default-case-of-restoring-an-arrangement.md)).
    Panel(Op),
    /// One operation of the vocabulary, named: a save under a name, or a restore of
    /// one. Both reach a file and only whoever holds the store can perform either.
    Operation(Operation),
}

/// Layout metrics and menu bounds for the arrangement selector pill.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrangementPill {
    /// The capsule, which is what a press has to land in to open the menu. The mock
    /// gives the whole pill the click and so does this.
    pub pill: Rect,
    /// Where `arr · night` is painted, inside the capsule's padding.
    pub text: Rect,
    /// The `▾` after it. Drawn rather than typed — see [`CHEVRON_W`].
    pub chevron: Rect,
    /// The menu, or `None` while it is shut — the whole card, which is what a press
    /// has to land in to be a pick rather than a dismissal.
    pub menu: Option<Rect>,
    /// Number of menu rows that fit vertically in the viewport.
    pub rows: usize,
    /// What the menu would list if the window were tall enough, carried so that the
    /// foot and the rows are one number rather than two.
    pub of: usize,
}

impl ArrangementPill {
    /// Whether `p` is on the pill itself.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is anywhere this control owns — the pill, or the menu while it
    /// is down. This is what [`crate::input::claim`] asks, and it is wider than
    /// [`ArrangementPill::hit`] by exactly the open menu.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        let at = Pos2::new(p.x, p.y);
        self.pill.contains(at) || self.menu.is_some_and(|menu| menu.contains(at))
    }

    /// Returns bounding box for the menu row at `index`.
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

    /// Which item `p` is on, or `None` for a point on no row — the menu's padding,
    /// its foot, or anywhere off it.
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

    /// Hit-tests a click at `p` against the pill or open menu items (ADR-0208, ADR-0221).
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

/// Computes layout rectangles for the arrangement pill and open dropdown menu.
pub fn arrangement(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    // Map configuration used to anchor layout offset following the previous group.
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

    let text_w = width(&pill_text(arr));
    let pill_w = size::PILL_PAD_X * 2.0 + text_w + size::SINK_GAP + CHEVRON_W;
    let mid = strip.center().y;
    // Anchors after map pill, learn pill, tracker group, audio-in, or bar.
    let after = match (
        map_pill(ctx, layout, values, audio, tracker, map),
        learn_pill(ctx, layout, values, audio, tracker, map, false),
        tracker_group(ctx, layout, values, audio, tracker),
        audio_in(ctx, layout, values, audio),
    ) {
        (Some(pill), _, _, _) => pill.pill.max.x,
        (None, Some(learn), _, _) => learn.pill.max.x,
        (None, None, Some(group), _) => group.double.max.x,
        (None, None, None, Some(before)) => before.pill.max.x,
        (None, None, None, None) => row.bar.max.x,
    };
    let pill = Rect::from_min_size(
        Pos2::new(after + size::TRANSPORT_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    // The rule `outputs_row` states: a capsule that does not fit in the row it
    // is drawn in is no control at all, rather than half of one over the frame
    // readout.
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

/// Formats the arrangement pill label as `arr · <name>`.
fn pill_text(arr: &Arrangement) -> String {
    format!("{ARRANGEMENT_LABEL} · {}", arr.word())
}

/// Computes position, size, and capacity for the arrangement dropdown menu card.
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
    let furniture = size::LIB_LIST_PAD * 2.0;

    // A name being typed: one row with the buffer in it, and nothing else.
    if let Some(typed) = arr.naming() {
        let card = held_inside(
            &viewport,
            pill.min.x,
            top,
            width(&naming_text(typed)).max(pill.width())
                + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
            furniture + size::LIB_ROW_H,
        );
        return (Some(card), 0, 0);
    }

    let of = arr.rows();
    // Widest of the two verbs, every name filed, and the pill itself.
    let widest = [width(save_word(arr)), width(NEW_ITEM)]
        .into_iter()
        .chain(arr.filed.iter().map(|name| width(name)))
        .fold(pill.width(), f32::max);
    // How many rows there is room for between the menu's top and the bottom of
    // the console. Asked twice: the foot is only owed where something is left
    // out, so the first ask sees whether everything fits without one.
    let room = |foot: f32| {
        (((viewport.max.y - top - furniture - size::HAIRLINE - foot) / size::LIB_ROW_H).floor())
            .max(0.0) as usize
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

/// What *save* is called with this arrangement in use. The ellipsis is there
/// exactly while the item asks for something — see [`SAVE_ITEM`].
fn save_word(arr: &Arrangement) -> &'static str {
    match arr.name {
        Some(_) => SAVE_ITEM,
        None => SAVE_ITEM_ASKING,
    }
}

/// Formats the naming buffer with trailing cursor caret (`night▏`).
fn naming_text(typed: &str) -> String {
    format!("{typed}{CARET}")
}

/// Paints the arrangement pill.
pub(crate) fn arrangement_into(ui: &Ui, pal: &Palette, pill: &ArrangementPill, arr: &Arrangement) {
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
    chevron_down(painter, pill.chevron, pal.dim);

    let Some(card) = pill.menu else {
        return;
    };
    popup_card(painter, pal, card);

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
        card_row_text(painter, field, &naming_text(typed), pal.text);
        return;
    }

    for index in 0..pill.rows {
        let row = pill.row(index);
        let (name, colour) = match index {
            0 => (save_word(arr), pal.text),
            1 => (NEW_ITEM, pal.text),
            other => (arr.filed[other - VERBS].as_str(), pal.dim),
        };
        card_row_text(painter, row, name, colour);
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
