use super::*;

impl View {
    /// What the Library bay's load control is aimed at, as the one value
    /// [`LibraryBay::load`] lays itself out from — see [`Target`], and
    /// [`View::target_deck`] for the argument.
    ///
    /// Read once for the frame and handed to the paint and to the press, exactly as
    /// [`View::filters`] is: the capsule that is drawn and the capsule a press
    /// lands on are one derivation of one reading.
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

    /// Aim the load at `deck`, put the list away, and answer whether anything
    /// moved.
    ///
    /// A deck the mixer has no strip for is refused, which is [`View::select`]'s
    /// rule read a second time and not a second rule: the letter says where a press
    /// lands, so a target past the deck's slots would be a letter naming a deck the
    /// press would be turned down on. `console.html`: *"A deck the mixer is drawing
    /// no strip for is not in the list, which is the count `0`–`3` are refused
    /// on"*.
    ///
    /// It refuses rather than clamping, for `select`'s reason: a pick of deck D at
    /// a two-slot deck means *deck D*, and clamping would aim the load at deck B,
    /// which is a different deck than the one asked for.
    ///
    /// The list goes away here, because a pick is one gesture and this is the whole
    /// of it: nothing is emitted, so there is no host arm to end it in, and a list
    /// left down after a pick would be a card still claiming every press on the
    /// console. It is put away even where the deck did not move — picking the deck
    /// already aimed at is still a hand finishing what it started.
    ///
    /// The deck selection does not move, and nothing here touches it: that is the
    /// whole of what this mark is for.
    ///
    /// The `bool` is [`View::select`]'s: a caller repaints on a move and not on a
    /// press.
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

    /// Put the list down, and answer whether it went down.
    ///
    /// Refused where the mixer is drawing no strip, which is the field's own rule:
    /// a card with no rows in it offers nothing to pick, and
    /// [`crate::input::claim`]'s rule 2 would give it every press on the console
    /// until a second press shut it again. A console with no deck behind it draws
    /// no mixer either, so there is nothing this refusal hides.
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

    /// What a row's menu is open on, and how many decks it offers — the one value
    /// [`LibraryBay::menu`] lays itself out from.
    ///
    /// [`View::target`]'s shape one control along, and for that method's reason:
    /// read once for the frame and handed to the paint and to the press, so the
    /// card that is drawn and the card a press lands on are one derivation of one
    /// reading.
    pub fn menued(&self) -> Menued {
        Menued {
            row: self.menu_row,
            decks: self.mixer.len().min(DECKS),
            // **Whether the row it is on can be sent**, which is whether it is
            // a Set: a procedure row's menu is the loads and no separator
            // (ADR-0338). Read here beside the row it is about, so the card
            // that is drawn and the card a press lands on are one reading.
            sends: self
                .menu_row
                .is_some_and(|row| self.rows().set(row).is_some()),
        }
    }

    /// Whether a row's menu is down — see [`View::menu_row`] the field.
    pub fn menu_open(&self) -> bool {
        self.menu_row.is_some()
    }

    /// Put the menu down on `row`, and answer whether it went down.
    ///
    /// A row the bay is not drawing is refused, which is [`View::aim_at`]'s rule
    /// read on a different list: a card hanging off a row nobody can see would be a
    /// gesture with nothing under it, and the row is where the card is measured
    /// from. The count is the *listing*'s rather than the bay's, because this crate
    /// is not told how many rows the bay had room for until it is laid out —
    /// [`LibraryBay::menu_ask`] is where a press is turned down against the drawn
    /// rows, and this is the wall behind it.
    ///
    /// It does not refuse a console with no strip, unlike [`View::open_target`]:
    /// the send under the separator names no deck, so a menu with no loads in it is
    /// still a card with something to pick.
    ///
    /// The `bool` is [`View::select`]'s: a caller repaints on a move and not on a
    /// press.
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

    /// Which Set in the Library bay a load would take — the mock's
    /// `.lib-row.cursor`, and that bay's remembered address read as a row.
    ///
    /// It has no operation at all, where the deck selection has a row of its own,
    /// and `console.html`'s *How a Set reaches a deck* is where that asymmetry is
    /// argued: the selection is what every deck-addressed operation's keyboard
    /// translator fills its `deck` in from, and this is read by exactly one
    /// operation — which carries the Set id in its own payload. A map cannot name a
    /// Set, a model names one outright, and the panel's route is the drag, so three
    /// of the four surfaces would have nothing to reach.
    ///
    /// An index into [`View::library`] and not a name, because a name this console
    /// kept would be a second copy of a listing it is handed per frame — and a copy
    /// that goes on naming a Set the store no longer holds. [`View::walk`] and
    /// [`View::point_at`] are what keep it inside the listing — a key steps and a
    /// press names, which is this console's division everywhere a pointer meets a
    /// key — and both are asked at the move rather than at the draw: a cursor
    /// clamped while painting would move on a frame nobody pressed anything on.
    ///
    /// Answered against the listing rather than read back bare: a store that shrank
    /// between two frames leaves an index past its end, and the row a load would
    /// take is then the last one there is. Zero on an empty listing, which is a bay
    /// with no row to draw at all.
    pub fn cursor_row(&self) -> usize {
        self.stored_row().min(self.library.len().saturating_sub(1))
    }

    /// The row as it is stored — the number [`View::cursor_row`] clamps and never
    /// the one it clamped, which is [`View::library_scroll`]'s rule one pointer
    /// along.
    ///
    /// The first row for a bay nobody has addressed. The digit that names it is
    /// `1`, so this is where the step down to a position is written — the Library's
    /// half of the arithmetic [`View::selection`] does for the Mixer.
    fn stored_row(&self) -> usize {
        self.focus
            .address(focus::LIBRARY)
            .and_then(|address| address.remembered(&[]))
            .map_or(0, |nth| nth.saturating_sub(1))
    }

    /// Put the library cursor on `row`, with nothing refused and nothing clamped —
    /// the one write [`View::walk`], [`View::point_at`] and the two scope methods
    /// all end at.
    ///
    /// It is private because every rule about *which rows there are* belongs to its
    /// caller: `walk` holds the cursor inside the rows the bay drew, `point_at`
    /// refuses a row past the listing, and a scope press puts it back at the top.
    /// This is the storage.
    fn put_row(&mut self, row: usize) {
        self.focus
            .address_mut(focus::LIBRARY)
            .remember(&[], row + 1);
    }

    /// Move the library cursor by `step` rows, and answer whether it moved.
    ///
    /// `drawn` is which rows the bay is drawing — [`LibraryBay::drawn`], off the
    /// same [`library`] call the paint and [`crate::input::claim`] make, so there
    /// is no second derivation of *which rows are on screen*.
    ///
    /// The cursor is held inside the rows that are drawn, not inside the store, and
    /// that sentence is older than the scroll: a cursor allowed past them would sit
    /// on a row nobody can see, under a pill that says a press will load it. What
    /// has changed is that *drawn* is a range rather than a prefix, because the bay
    /// scrolls now
    /// ([ADR-0312](../../../../docs/adr/0312-the-params-pill-is-a-toggle-and-the-library-bay-scrolls.md)).
    /// It used to take a count and clamp to `0..listed`.
    ///
    /// The wheel is what moves the window and the arrows are what move the cursor
    /// inside it, which is the division
    /// [ADR-0307](../../../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)
    /// made one bay over: *"the keyboard's route is not bound here"*. Nothing the
    /// keyboard could reach before is out of reach now — the rows past the end of
    /// the list were unreachable by any means, and they are one notch away — and a
    /// key that scrolls is owed to M5.13 with the arrows' walk of a bay's items,
    /// not invented here.
    ///
    /// Clamped at both ends rather than wrapping. A listing is a walk and not a
    /// cycle: wrapping from the last row to the first would jump the length of the
    /// list on one press, which is the one move a key held down must not make.
    ///
    /// Relative because that is what a key can say. Nothing here is an operation
    /// ([`View::cursor_row`] the field), so there is no absolute spelling owed to a
    /// map or a model.
    ///
    /// The `bool` is [`View::select`]'s, for the same reason: a press that changed
    /// nothing costs no frame.
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

    /// Put the library cursor on `row`, and answer whether it moved.
    ///
    /// [`View::walk`]'s absolute door, and the pointer is what needs it: a key can
    /// only say *one further on*, and a press lands on exactly one row — the same
    /// division `e` and a scope chip make one bay up, and the same one
    /// [`LibraryBay::filter`] makes against them.
    ///
    /// What it is for is the carry. A press on a row takes that Set in hand
    /// ([`LibraryBay::take`]), and the mark on the row is the whole of what this
    /// console can show for it: the mock draws `.lib-row.cursor` and draws no ghost
    /// under a pointer and no lit strip, so the honest affordance is the one the
    /// page already has, moved to the row the hand is on. It outlives the gesture
    /// on purpose — a carry that was let go over nothing leaves the cursor where
    /// the hand went, which is where `l` would load from next.
    ///
    /// A row past the listing is refused rather than clamped, which is
    /// [`View::select`]'s rule rather than [`View::walk`]'s, and for `select`'s
    /// reason: a walk is *from where the cursor is*, so the nearest row is what a
    /// key meant, and a press names a row outright — a press answered with a
    /// different row than the one under it would move the load somewhere nobody
    /// pointed.
    ///
    /// The `bool` is [`View::walk`]'s, for the same reason.
    pub fn point_at(&mut self, row: usize) -> bool {
        if row >= self.library.len() {
            return false;
        }
        let moved = row != self.stored_row();
        self.put_row(row);
        moved
    }

    /// What the Library bay's cursor row has open, or `None` where nothing is.
    ///
    /// Answered against the listing rather than read back bare, which is
    /// [`View::cursor_row`]'s rule with a name in it: a reading is of one Set, the
    /// listing under it can be rewritten by any press on a scope chip or a filter
    /// field, and a reading left drawn under whatever has taken that position would
    /// be this bay describing one Set under the name of another. So it is drawn
    /// where the row under the cursor is still the Set it was read of, and nowhere
    /// else.
    ///
    /// It is not put away when that happens. Nothing here is `&mut`, and the
    /// reading a press asked for is still what the host answered — what has changed
    /// is that there is nowhere to draw it. A narrowing that takes the row away and
    /// a second that brings it back are one gesture to the hand that made them.
    pub fn opened(&self) -> Option<Opened<'_>> {
        let reading = self.reading.as_ref()?;
        let at = self.cursor_row();
        (self.library.get(at).map(String::as_str) == Some(reading.id.as_str()))
            .then_some(Opened { at, reading })
    }

    /// Open a reading under the cursor, which is what a host answers
    /// [`Operation::ReadSet`] with.
    ///
    /// The value is the host's whole answer — see [`Reading`], and
    /// [`View::reading`] the field for why the reading is read out there and the
    /// opening is kept in here.
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

    /// The listing, where its rows are Sets — [`View::library`] under the four
    /// library scopes, and empty under [`Scope::History`], whose rows are versions.
    ///
    /// One place the question is asked, rather than four. The star, the `params`
    /// chip, the `load` button and the carry all read a row as a Set id, and each
    /// of them already refuses a listing shorter than the rows drawn rather than
    /// clamping — so handing them nothing is the refusal they already have, said
    /// once. See [`Scope::lists_sets`].
    ///
    /// A console with no scope row at all still lists Sets. [`View::scope`] answers
    /// `None` there, and a bay nobody has told what libraries there are is every
    /// test in this crate that does not say otherwise — so the absence of a scope
    /// is not the absence of a listing.
    pub fn sets(&self) -> &[String] {
        match self.scope() {
            Some(scope) if !scope.lists_sets() => &[],
            _ => &self.library,
        }
    }

    /// The listing as this bay's controls read it — the names of [`View::sets`] and
    /// what each of those rows is ([`View::kinds`]), answered together.
    ///
    /// One reading and not two, which is [`Rows`]' own argument: the star, the
    /// `params` chip, the `load` button, the row menu and the carry each need to
    /// know whether the row under them is a Set or a procedure, and two slices
    /// fetched separately are two slices that can disagree about it.
    ///
    /// Empty under [`Scope::History`], exactly as [`View::sets`] is and for its
    /// reason: those rows are versions, and every control that takes this refuses a
    /// row it cannot name rather than being told which scope is marked.
    pub fn rows(&self) -> Rows<'_> {
        match self.scope() {
            Some(scope) if !scope.lists_sets() => Rows::NONE,
            _ => Rows {
                names: &self.library,
                kinds: &self.kinds,
            },
        }
    }

    /// The listing, where its rows are versions of one Set — [`View::library`]
    /// under [`Scope::History`] and empty under every other scope, which is
    /// [`View::sets`]' answer the other way round.
    ///
    /// The one reader is [`LibraryBay::land`], and the pair is what lets one press
    /// on one rectangle mean a carry or a landing without either method being told
    /// which scope is marked.
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

    /// Which scope the bay is listing, as it is stored — the Library bay's
    /// remembered address one level in, at the control the head names.
    ///
    /// A pointer and not a reading, which is [`View::selection`]'s argument
    /// arriving at a second control: [`Operation::SelectScope`] is
    /// `Silent(Surface)` — *"it changes which library the bay is reading and
    /// nothing about what any deck is playing"* — so nothing downstream can be the
    /// model of record for it, and a host that kept a copy would be keeping the
    /// console's state on its behalf. The host reads it, through [`View::scope`],
    /// to know which listing to answer with.
    ///
    /// A position and not a [`Scope`], for [`View::cursor_row`]'s reason: what is
    /// drawn is the row of chips this console was handed, so what a pointer into it
    /// can be is a place in that row — and a scope held here that the host stopped
    /// offering would be a mark drawn on no chip at all.
    ///
    /// Under [`focus::HEAD`] rather than beside the cursor, which is the whole of
    /// why one bay has two of these and they do not collide: the scope chips are
    /// the bay's *head* — the controls that are about the bay rather than about
    /// anything in it — and the cursor is which *item* the bay is on. ADR-0259
    /// walks the Library exactly that way: *"`0` is the head and its controls are
    /// the scope chips … Items are the listed Sets."*
    ///
    /// The first chip for a bay nobody has addressed, which is `all` in the row
    /// this console draws and is what a run opens on — a bay that opened on the
    /// starred subset would be empty on a store nobody has starred in. See
    /// [`View::select_scope`], which is how a host that wants another chip says so.
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

    /// Mark `scope`, and answer whether that moved anything.
    ///
    /// A scope this console was not handed is refused, which is [`View::select`]'s
    /// rule and the same reasoning: the mark is drawn on a chip, so a scope with no
    /// chip is a mark drawn nowhere and a listing nobody can see the question for.
    /// It refuses rather than clamping, for that method's reason too — a scope that
    /// is not on the row is not the scope next to it.
    ///
    /// The one caller is a host that opens on a scope other than the first chip,
    /// and the program in this workspace no longer is one: `all` is the first chip
    /// and is where a run opens (ADR-0299). What is left for this is a press on a
    /// chip, which names one outright.
    ///
    /// The `bool` is [`View::select`]'s: a caller repaints on a move and not on a
    /// press.
    pub fn select_scope(&mut self, scope: Scope) -> bool {
        let Some(at) = self.scopes.iter().position(|drawn| *drawn == scope) else {
            return false;
        };
        let moved = self.marked() != at;
        self.put_scope(at);
        if moved {
            // **The cursor goes back to the top of a listing it has never
            // seen.** It is a position in [`View::library`] and that field is
            // about to be rewritten by whoever answers the new scope, so a
            // cursor left where it was would point at the fifteenth row of a
            // list of three — which [`View::cursor_row`] would then clamp to
            // *the last row*, a Set nobody chose sitting under a pill that
            // says a press will load it.
            self.put_row(0);
            // **And so does the scroll, for the same reason and not for a
            // second one.** A position is a distance into [`View::library`],
            // that field is about to be rewritten, and a listing of three read
            // at four hundred pixels down draws its last row or nothing at
            // all. **It is a press moving the console's own state and not a
            // resize rewriting it**, which is what
            // [P-0082](../../../../docs/principles/0082-looking-never-writes-back.md)
            // is about: the clamp at the draw is still the only clamp, and a
            // scope pressed twice does not move it a second time
            // (ADR-0312).
            //
            // **A filter narrowing is not this**, and it is not an omission:
            // it asks the same scope a narrower question, so the rows are the
            // same rows fewer of them, and what a position past the end draws
            // is the clamp's answer — the same division [`View::cursor_row`]
            // makes between a reset here and a clamp there.
            self.library_scroll = 0.0;
        }
        moved
    }

    /// Mark the next scope along, wrapping, and answer whether that moved anything.
    ///
    /// The stepping is here and not in the vocabulary, which is
    /// [`Operation::SelectScope`]'s own instruction: *"The key steps and this does
    /// not … that is the translator's arithmetic rather than this operation's
    /// payload"*
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    /// A bare press cannot type a name and here it does not have to: the scopes are
    /// a short row of chips in front of you, so stepping says *which* by showing
    /// you.
    ///
    /// And it wraps where [`View::walk`] clamps, which is not an inconsistency: a
    /// listing is a walk and wrapping it would jump the length of a list on one
    /// press, where the scopes are a *cycle* of four chips a step apart —
    /// `docs/manual/operations.html` says so at the row (*"The key steps to the
    /// next scope and wraps"*), and a step that stopped at the last chip would need
    /// a second key to come back.
    ///
    /// Nothing to step where there are no chips or one, and that is `false` rather
    /// than a wrap onto itself: a press that changed nothing costs no frame.
    pub fn step_scope(&mut self) -> bool {
        if self.scopes.len() < 2 {
            return false;
        }
        let to = (self.marked() + 1) % self.scopes.len();
        self.put_scope(to);
        // The listing is about to be a different listing — see
        // [`View::select_scope`], where this is argued.
        self.put_row(0);
        true
    }

    /// What the two filter fields are narrowing the listing to — see [`Filters`],
    /// and [`LibraryBay::filter`] for what a press on one of them asks.
    ///
    /// Answered against [`View::holds`] rather than read back bare, which is
    /// [`View::scope`]'s rule with the opposite answer at the end of it: a host
    /// that handed candidates and then none leaves a position past the end, and
    /// what that resolves to is unset. So a scope whose listing has no summaries
    /// behind it — `presets`, whose rows are files rather than Sets this store
    /// holds — draws `holds…` and narrows nothing, and the filter is in force again
    /// when a scope that has candidates comes back.
    pub fn filters(&self) -> Filters<'_> {
        Filters {
            holds: self
                .holds_at
                .and_then(|at| self.holds.get(at))
                .map(String::as_str),
            kinds: self.showing,
        }
    }

    /// What the `.path` row reads, or `None` for a bay that is pointed nowhere and
    /// has a folder over nothing — which is where a run starts and is every test in
    /// this crate that does not say otherwise.
    ///
    /// A folder over the window wins over the folder that was chosen, which is the
    /// whole of [`Pointed::incoming`]: the mock draws one `.path` row, so a hover
    /// *replaces* the line rather than adding one, and what is on screen while a
    /// drag is over this window is what a release would set. Take the folder back
    /// out of the window and the row goes back with it, because the platform says
    /// when a drag leaves as well as when it arrives (ADR-0275).
    ///
    /// It is the whole of what the bay's arithmetic needs, and it is asked once per
    /// pass beside [`View::filters`] for that method's reason: `draw` takes `&mut
    /// self`, and this borrows two of its fields.
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

    /// Narrow the listing to `holds` and to these kinds, and answer whether that
    /// moved anything.
    ///
    /// A `holds` this console cannot draw is refused, which is
    /// [`View::select_scope`]'s rule and the same reasoning: the field reads the
    /// value, so a filter that is not among [`View::holds`] is a word the bay would
    /// draw with no way to step off it. The kinds are not refused, because every
    /// one of the sixty-four states six toggles can be in is a row this bay can
    /// draw.
    ///
    /// It refuses the pair or neither, so a call that would have set the kinds and
    /// dropped the `holds` on the floor sets nothing. The two callers hand back an
    /// [`Operation::ListSets`] or an [`Operation::FilterLibrary`] this bay built,
    /// so the refusal is a caller's error rather than a state — as
    /// [`View::select_scope`]'s is.
    ///
    /// The `bool` is [`View::select`]'s: a caller repaints on a move and not on a
    /// press.
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
            // **The cursor goes back to the top of a listing it has never
            // seen**, which is [`View::select_scope`]'s reason word for word:
            // [`View::library`] is about to be rewritten by whoever answers the
            // narrowing, and a cursor left where it was would point at the
            // fifteenth row of a list of three.
            self.put_row(0);
        }
        moved
    }
}
