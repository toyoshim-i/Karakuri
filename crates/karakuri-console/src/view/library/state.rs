use super::*;

impl View {
    /// Returns the target deck configuration for the Library bay's load control.
    pub fn target(&self) -> Target {
        Target {
            deck: self.target,
            decks: self.mixer.len(),
            open: self.target_open,
        }
    }

    /// Which deck a press on the Library bay's `load` button lands on — see
    /// [`View::target`] the field, which is where the argument is.
    pub fn target_deck(&self) -> u8 {
        self.target
    }

    /// Aims the load at `deck`, closes the target dropdown, and returns true if changed.
    ///
    /// Refuses decks outside the mixer bounds without clamping.
    pub fn aim_at(&mut self, deck: u8) -> bool {
        if usize::from(deck) >= self.mixer.len() {
            return false;
        }
        let moved = self.target != deck || self.target_open;
        self.target = deck;
        self.target_open = false;
        moved
    }

    /// Whether the pulldown's list is down — see [`View::target_open`] the field.
    pub fn target_open(&self) -> bool {
        self.target_open
    }

    /// Opens the target deck selection dropdown if mixer decks are available.
    pub fn open_target(&mut self) -> bool {
        if self.mixer.is_empty() || self.target_open {
            return false;
        }
        self.target_open = true;
        true
    }

    /// Take the list away, and answer whether there was one down.
    ///
    /// [`View::shut_reading`]'s shape: a caller repaints on a move, so a dismissal
    /// of nothing costs no frame.
    pub fn shut_target(&mut self) -> bool {
        let was = self.target_open;
        self.target_open = false;
        was
    }

    /// Returns layout parameters for the open row context menu, if any.
    pub fn menued(&self) -> Menued {
        Menued {
            row: self.menu_row,
            decks: self.mixer.len().min(DECKS),
            sends: self
                .menu_row
                .is_some_and(|row| self.rows().set(row).is_some()),
        }
    }

    /// Whether a row's menu is down — see [`View::menu_row`] the field.
    pub fn menu_open(&self) -> bool {
        self.menu_row.is_some()
    }

    /// Opens the context menu on `row`, returning true if newly opened or changed.
    pub fn open_menu(&mut self, row: usize) -> bool {
        if row >= self.library.len() {
            return false;
        }
        let moved = self.menu_row != Some(row);
        self.menu_row = Some(row);
        moved
    }

    /// Take the menu away, and answer whether there was one down.
    ///
    /// [`View::shut_target`]'s shape: a caller repaints on a move, so a dismissal
    /// of nothing costs no frame.
    pub fn shut_menu(&mut self) -> bool {
        self.menu_row.take().is_some()
    }

    /// How far the Library bay is scrolled, as it is stored — the number
    /// [`library`] clamps and never the one it clamped.
    ///
    /// [`View::scroll_in`]'s shape one bay over.
    pub fn library_scroll(&self) -> f32 {
        self.library_scroll
    }

    /// Turn the Library bay's wheel by `by` pixels (positive down the listing).
    ///
    /// Clamps scroll offset between zero and total content height ([`library_content_h`]).
    /// Returns true if scroll position changed (P-0082, ADR-0312).
    pub fn scroll_library_by(&mut self, by: f32) -> bool {
        if self.library.is_empty() {
            return false;
        }
        let content = library_content_h(
            self.library.len(),
            self.opened().map(|open| open.reading.rows()),
        );
        let next = (self.library_scroll + by).clamp(0.0, content);
        let moved = next != self.library_scroll;
        self.library_scroll = next;
        moved
    }

    /// Returns the active library cursor row index, clamped to available items.
    pub fn cursor_row(&self) -> usize {
        self.stored_row().min(self.library.len().saturating_sub(1))
    }

    /// Returns the raw stored cursor row from navigation focus address.
    fn stored_row(&self) -> usize {
        self.focus
            .address(focus::LIBRARY)
            .and_then(|address| address.remembered(&[]))
            .map_or(0, |nth| nth.saturating_sub(1))
    }

    /// Sets the stored library row in focus navigation address.
    fn put_row(&mut self, row: usize) {
        self.focus
            .address_mut(focus::LIBRARY)
            .remember(&[], row + 1);
    }

    /// Steps the library cursor by `step` rows, clamped within `drawn` range (ADR-0307, ADR-0312).
    ///
    /// Returns true if the cursor moved.
    pub fn walk(&mut self, step: i32, drawn: std::ops::Range<usize>) -> bool {
        let last = drawn.end.min(self.library.len());
        if drawn.start >= last {
            return false;
        }
        let to = (self.stored_row() as i64 + step as i64)
            .clamp(drawn.start as i64, (last - 1) as i64) as usize;
        let moved = to != self.stored_row();
        self.put_row(to);
        moved
    }

    /// Places the cursor on `row` (used for drag/carry and direct pointer clicks).
    ///
    /// Refuses indices beyond the listing length. Returns true if the cursor moved.
    pub fn point_at(&mut self, row: usize) -> bool {
        if row >= self.library.len() {
            return false;
        }
        let moved = row != self.stored_row();
        self.put_row(row);
        moved
    }

    /// Returns the reading active under the current cursor row, if still matching the Set id.
    pub fn opened(&self) -> Option<Opened<'_>> {
        let reading = self.reading.as_ref()?;
        let at = self.cursor_row();
        (self.library.get(at).map(String::as_str) == Some(reading.id.as_str()))
            .then_some(Opened { at, reading })
    }

    /// Attaches an inspected Set [`Reading`] under the cursor.
    pub fn read(&mut self, reading: Reading) {
        self.reading = Some(reading);
    }

    /// Whether a reading is open at all, regardless of the row it was read from.
    pub fn reading_open(&self) -> bool {
        self.reading.is_some()
    }

    /// Put the reading away, and answer whether there was one.
    ///
    /// The `bool` is [`View::select`]'s: a caller repaints on a change and not on a
    /// press.
    pub fn shut_reading(&mut self) -> bool {
        self.reading.take().is_some()
    }

    /// The active library scope, or `None` if no scopes are configured.
    pub fn scope(&self) -> Option<Scope> {
        self.scopes.get(self.marked()).copied()
    }

    /// Returns the slice of Sets for the current scope, or empty if viewing [`Scope::History`].
    pub fn sets(&self) -> &[String] {
        match self.scope() {
            Some(scope) if !scope.lists_sets() => &[],
            _ => &self.library,
        }
    }

    /// Returns combined names and kinds for the active listing (or empty for history).
    pub fn rows(&self) -> Rows<'_> {
        match self.scope() {
            Some(scope) if !scope.lists_sets() => Rows::NONE,
            _ => Rows {
                names: &self.library,
                kinds: &self.kinds,
            },
        }
    }

    /// Returns version rows when [`Scope::History`] is active, or empty otherwise.
    pub fn versions(&self) -> &[String] {
        match self.scope() {
            Some(Scope::History) => &self.library,
            _ => &[],
        }
    }

    /// Which chip is marked, as a position — clamped to the row that is drawn, and
    /// zero for a row with nothing in it. The paint's half of [`View::scope`].
    pub(in crate::view) fn marked(&self) -> usize {
        self.stored_scope().min(self.scopes.len().saturating_sub(1))
    }

    /// Returns the stored scope chip index from focus address state (ADR-0259).
    fn stored_scope(&self) -> usize {
        self.focus
            .address(focus::LIBRARY)
            .and_then(|address| address.remembered(&[focus::HEAD]))
            .map_or(0, |nth| nth.saturating_sub(1))
    }

    /// Mark the chip at `at`, with nothing refused and nothing clamped —
    /// [`View::put_row`]'s half one level in, and private for its reason: which
    /// chips there are belongs to [`View::select_scope`] and [`View::step_scope`].
    fn put_scope(&mut self, at: usize) {
        self.focus
            .address_mut(focus::LIBRARY)
            .remember(&[focus::HEAD], at + 1);
    }

    /// Selects `scope`, resetting cursor and scroll if changed (ADR-0299, ADR-0312, P-0082).
    pub fn select_scope(&mut self, scope: Scope) -> bool {
        let Some(at) = self.scopes.iter().position(|drawn| *drawn == scope) else {
            return false;
        };
        let moved = self.marked() != at;
        self.put_scope(at);
        if moved {
            // Reset cursor and scroll to top for the new scope listing (P-0082, ADR-0312).
            self.put_row(0);
            self.library_scroll = 0.0;
        }
        moved
    }

    /// Steps to the next scope chip, wrapping around (P-0090). Returns true if changed.
    pub fn step_scope(&mut self) -> bool {
        if self.scopes.len() < 2 {
            return false;
        }
        let to = (self.marked() + 1) % self.scopes.len();
        self.put_scope(to);
        self.put_row(0);
        true
    }

    /// Returns current filter criteria (held node name and kind bitflags).
    pub fn filters(&self) -> Filters<'_> {
        Filters {
            holds: self
                .holds_at
                .and_then(|at| self.holds.get(at))
                .map(String::as_str),
            kinds: self.showing,
        }
    }

    /// Returns current folder path, prioritizing incoming drag hover over selected folder (ADR-0275).
    pub fn pointed(&self) -> Option<Pointed<'_>> {
        match (self.incoming.as_deref(), self.folder.as_deref()) {
            (Some(path), _) => Some(Pointed {
                path,
                incoming: true,
            }),
            (None, Some(path)) => Some(Pointed {
                path,
                incoming: false,
            }),
            (None, None) => None,
        }
    }

    /// Sets filter criteria (`holds` and `kinds`), returning true if changed.
    pub fn narrow(&mut self, holds: Option<&str>, kinds: LibraryKinds) -> bool {
        let holds_at = match holds {
            None => None,
            Some(want) => match self.holds.iter().position(|held| held == want) {
                Some(at) => Some(at),
                None => return false,
            },
        };
        let moved = self.filters() != (Filters { holds, kinds });
        self.holds_at = holds_at;
        self.showing = kinds;
        if moved {
            // Reset cursor to top for the narrowed listing.
            self.put_row(0);
        }
        moved
    }
}
