use super::super::*;
use super::*;

// ---------------------------------------------------------------------------
// Scopes and path
// ---------------------------------------------------------------------------

/// One chip in the Library bay's scope row, and it names *which library is
/// being read* rather than a place a Set can be.
///
/// `docs/manual/operations.html`'s *Choose which scope the library shows* is
/// the whole of the argument, and `console.html` argues the sharpest part of
/// it, which is what makes this one type rather than four: the four chips
/// are four questions rather than four acts — [`Scope::MySets`] is the
/// starred subset of [`Scope::AllSets`], so choosing one and choosing the
/// other differ in the question asked and not in what is asked.
///
/// # Four here, and [`Operation::SelectScope`] still carries `Undecided`
///
/// The two are not in disagreement. That payload is open because
/// `docs/manual/operations.html`'s row calls the scopes *"the one thing about
/// the library that is not closed"*, and a vocabulary that named a member
/// of a growable list would go short the moment an operator points the bay at
/// a directory.
/// This type is not that: it is the row of chips this panel draws, which is
/// the mock's four and no more, and it is handed to [`library`] as a slice for
/// exactly that reason — the bay draws the scopes it is given, so a fifth is a
/// value crossing the seam and not a signature.
///
/// # All four answer now, and the last of them answered on 2026-09-08
///
/// - [`Scope::Folder`] waited on a directory rather than on an operation,
///   and ADR-0275 is the mechanism: a folder dragged off the desktop onto the
///   window, window-global rather than aimed at this bay.
///   `Operation::ListSets { holds, layer }` is still not missing a field —
///   both are filters over what a store already holds, and a directory is
///   *which store is asked at all*, which is the host's outside the operation
///   entirely.
/// - [`Scope::MySets`] waited on somewhere for a star to be, and
///   [ADR-0299](../../../../docs/adr/0299-my-sets-is-the-starred-subset-and-the-star-is-kept-beside-the-sets.md)
///   is that: `<store>/favourites.json`, beside the Sets. The chip that lists
///   everything the store holds is [`Scope::AllSets`], which is what that
///   record's *"what lists everything the store holds still needs a chip"*
///   asked for.
///
/// A folder nobody has pointed anywhere is still a scope with nothing in
/// it, and so is a store nobody has starred in. That is a question that has
/// been asked and answered rather than a chip that is quiet, and the host is
/// what says which — in the words at its own key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// `all`: everything this store holds, which is
    /// `karakuri_store::Store::list_sets` and is the listing every other row of
    /// this bay is a question about.
    ///
    /// It is the chip [`Scope::MySets`] stopped being. ADR-0299 made *my sets* the
    /// starred subset, and *"what lists everything the store holds still needs a
    /// chip"* — this is it. The word is the maintainer's pending one: one string,
    /// here and in the page.
    AllSets,
    /// `my sets`: the starred subset of [`Scope::AllSets`], and never the listing
    /// of what `<store>/sets/` holds (ADR-0299).
    ///
    /// A preset packaged on load and a recording's head both land in the library
    /// and neither appears here until somebody presses the star on its row — which
    /// is what the roadmap's *"a `my sets` filling up with things the operator did
    /// not make"* was the symptom of.
    MySets,
    /// What ships with the program: the `.kset` files in the presets root, which is
    /// a directory the program is told (ADR-0230) rather than one it works out.
    /// Read-only — a row here is taken into the store and then loaded, and the row
    /// that leaves is under [`Scope::AllSets`]: a Set the store holds is what a
    /// take-in makes, and [`Scope::MySets`] is the starred subset of that, so it
    /// appears there only if somebody presses its star
    /// ([ADR-0299](../../../../docs/adr/0299-my-sets-is-the-starred-subset-and-the-star-is-kept-beside-the-sets.md)).
    /// This sentence said `my sets` until that record was written.
    Presets,
    /// A directory somebody names during the run.
    Folder,
    /// `history`: the versions of one Set, and the one chip here whose rows are not
    /// Sets.
    ///
    /// Every write that compiled is kept under `<store>/history/` — gated on
    /// compiling rather than on landing — and this is the reading of them: most
    /// recent first, each row naming when it was written, which node it was a
    /// version of and what that procedure called itself. The Set it is narrowed to
    /// is the one the load pulldown's deck is running ([`View::target_deck`]),
    /// which is the host's answer for [`View::library`]'s reason: a version's Set
    /// id rides the aim a load sends and this crate reaches no store.
    ///
    /// A deck running no Set lists nothing, and that is an answer rather than an
    /// absence: those versions are filed under *no Set*, and a narrowing to a Set
    /// matches none of them rather than all of them.
    ///
    /// Landing on a row is `Operation::RestoreProcedure` and never
    /// `Operation::LoadSet` — see [`LibraryBay::land`], and [`Scope::lists_sets`]
    /// for the four controls in this bay that have nothing to name while it is
    /// marked.
    History,
}

impl Scope {
    /// The four the mock draws, and the order is the bay's rather than the mock's:
    /// `all` comes first because it is the listing the other three are questions
    /// about, where the mock drew `favourites` there and `my sets` second. It is
    /// the order a step through them goes in.
    ///
    /// A `+` is drawn after them there and is not here: it is the arena's own gap
    /// drawn a fifth time, which [`outputs`] already names, and adding a scope is
    /// what [`Scope::Folder`] is waiting on anyway.
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

    /// Whether a row of this scope is a Set, which is true of four of the five and
    /// false of [`Scope::History`], whose rows are versions.
    ///
    /// Every control in this bay whose operand is a Set id asks this, and it is one
    /// question rather than four: the star, the `params` chip, the `load` button
    /// and the carry all read the row under a cursor as an id, and a version handed
    /// to any of them would name a Set no store holds. See [`View::sets`] and
    /// [`View::versions`], which is where the answer is applied — the controls take
    /// the listing they can act on, so a scope whose rows they cannot name hands
    /// them nothing rather than being special-cased at each of them.
    pub fn lists_sets(self) -> bool {
        !matches!(self, Scope::History)
    }
}

/// What a press on a scope chip asks for: the chip it landed on, and the
/// operation of the vocabulary that names the asking.
///
/// # Two fields because the payload cannot carry the first one
///
/// [`Operation::SelectScope`] is `SelectScope { scope: Undecided }`, and that
/// is deliberate at the operation: *"an enum of the four here would assert that
/// the list can be finished, which is the claim that row exists to refuse"*. So
/// the operation says that a library was chosen and cannot say which, and a
/// control that could say which has to say it beside the operation rather than
/// inside it. That is what this type is: one press, one answer, and the two
/// halves cannot be got out of step because they are derived together from the
/// chip the pointer was on.
///
/// This is the first thing in the workspace that knows which chip. A key press
/// cannot type a name, so `e` steps and the arithmetic is the translator's
/// ([P-0090]); a map line names a word from a closed list and this list is not
/// closed; a model has no chip in front of it. A *pointer* press is none of
/// those — it lands on one capsule and on no other, which is a way of naming a
/// member of a growable list that did not exist here before. Whether that
/// settles the payload is a decision about the vocabulary and it is not taken
/// here: settling it means saying what a scope is named *by* — a folder scope
/// has a path, `presets` has a root the program was told, and `all` and `my
/// sets` have neither — and that sentence belongs on
/// `docs/manual/operations.html` and in `karakuri-operation`, not in the first
/// control that happened to want it. So the press works with the payload as it
/// stands, and the proposal is written down where a maintainer reads it rather
/// than performed here.
///
/// [P-0090]: ../../../docs/principles/0090-a-surface-offers-it-never-decides.md
///
/// # The fifth chip asks a different row, and the row is why
///
/// [`Scope::History`] is a scope chip and [`Operation::WalkHistory`] is what a
/// press on it names, where the other four name [`Operation::SelectScope`]. One
/// press is one operation, and the one to name is the row that describes what
/// was asked for: the four are libraries of Sets and *"the one thing about the
/// library that is not closed"*, where the fifth is the edit history and has a
/// row of its own on the page.
///
/// The payload is what settles it. `SelectScope` cannot say *which*, so a
/// `SelectScope` emitted for this chip would be indistinguishable from one
/// emitted for `all` — a record saying a library was chosen for a press that
/// asked for a history. `console.html`'s *Walking a Set's edit history* is
/// where that is argued on the page, which is where it is decided.
#[derive(Debug, Clone, PartialEq)]
pub struct Chosen {
    /// The chip the pointer was on, which is a value of the row this console was
    /// handed rather than a position in it: the caller marks it through
    /// [`View::select_scope`], which refuses a scope with no chip.
    pub scope: Scope,
}

impl Chosen {
    /// What the press names: [`Operation::SelectScope`] for the four library chips,
    /// whose payload is `Undecided`, and [`Operation::WalkHistory`] for
    /// [`Scope::History`] — see this type's own documentation for both.
    ///
    /// `aimed` is the Set the walk is of, which is [`View::aimed`] and is the one
    /// thing about this press the console cannot answer for itself: the bay holds a
    /// deck letter and a Set id rides the aim (ADR-0308). It is a method rather
    /// than a field for that reason — the press names the row and the value the row
    /// needs is read where it lives, on the way out, so there is no moment at which
    /// a `Chosen` is carrying a Set nobody has checked against the deck the
    /// pulldown is on.
    ///
    /// `None` is not *no answer*, it is *no Set*. A deck playing the pair the run
    /// launched with has its versions filed under no Set, and a narrowing to a Set
    /// matches none of them — so the walk lists nothing and the bay says why, which
    /// is the same value `history::Version::set` carries for those rows (ADR-0276).
    ///
    /// The four beside it ignore it, because `SelectScope` says nothing about which
    /// library was chosen — the press *knows* and the payload cannot carry it,
    /// which is what put the fifth chip on a row of its own.
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

/// One scope chip's width: the word at [`size::BASE`] inside
/// [`size::SCOPE_PAD_X`] either side, which is the whole of what `.scope` is as
/// wide as — it draws no border, so there is nothing else to count.
///
/// Asked of `egui` rather than derived, for [`LibraryBay::pill`]'s reason one
/// row up: a capsule is as wide as the words in it, and the only thing that
/// knows how wide a word is is the thing that will paint it.
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
    /// The scope chips and where each one goes, left to right in the order
    /// `scopes` hands them.
    ///
    /// The widths are asked of `egui` rather than derived, for [`chip_width`]'s
    /// reason: a chip is as wide as the word in it.
    ///
    /// Empty for a bay with no scope row at all, which is a console nobody has told
    /// what libraries there are.
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
                // **One padding down from the top of the row**, which is where
                // `.scopes` puts it — and not the row's middle, which is half
                // a pixel lower because the rule at the bottom is inside the
                // row.
                Pos2::new(x, top),
                egui::vec2(chip_width(ctx, scope.name()), size::SCOPE_H),
            );
            x += chip.width() + size::SCOPES_GAP;
            (*scope, chip)
        })
    }

    /// What a press at `p` on the scope row asks for, or `None` where there is no
    /// chip under it.
    ///
    /// # The chip is the control, and it names the library rather than a place
    ///
    /// `console.html` puts the affordance on the chip itself — *"Click to show it;
    /// click another scope to leave it"* — and `docs/manual/operations.html` names
    /// the whole row as this operation's home. What comes out is [`Chosen`]: the
    /// chip the pointer was on, and [`Operation::SelectScope`] beside it, because
    /// that operation's payload is `Undecided` and cannot carry the chip. See
    /// [`Chosen`], which is where the argument is and where the proposal that would
    /// change it is written down.
    ///
    /// It does not cycle. The chip that was pressed is the chip that is asked for,
    /// where `e` steps to the next one and wraps — and that is not two answers to
    /// one question, it is P-0090's own division: a bare press cannot say *which*
    /// and this one can, so the key does the arithmetic and the pointer does not.
    ///
    /// # A chip is pressed only where it is drawn
    ///
    /// The row clips, so at the mock's width `folder` runs out past the bay's own
    /// edge and into the pane divider's grab. The part of it that is outside the
    /// row is not drawn, and a press there is a press on whatever is drawn under
    /// the pointer — so the point is held to the row before any chip is asked
    /// about. The part inside the divider's grab is the boundary's, which
    /// [`crate::input::claim`]'s rule 3 decides and this never sees.
    ///
    /// `None` before the first pass, which is [`mixer`]'s guard and [`outputs`]':
    /// there are no fonts until `egui` has run one, so there is no chip width to
    /// measure and nothing has been drawn to press.
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
            // **The row the press names follows the chip**, which is
            // [`Chosen::asked`] and not a special case here: four chips choose
            // among libraries of Sets and the fifth asks for an edit history,
            // and those are two rows of `docs/manual/operations.html`.
            .map(|(scope, _)| Chosen { scope })
    }
}

/// The scope row, painted: the chips left to right, the marked one washed,
/// and the rule under the row.
///
/// Where the row goes is [`library`]'s and where each chip in it goes is
/// [`LibraryBay::chips`]'; this is [`rend_row_into`]'s shape one bay along,
/// and deliberately so — the two are the same drawing. A chip is as wide as
/// the word in it, so the widths are asked of `egui` rather than derived —
/// and they are asked once, by the derivation this paint and
/// [`crate::input::claim`] both walk, because a chip is a control now and a
/// second measurement here would be a capsule a press could miss.
/// `tests/library.rs` is where that is held.
///
/// Term for term from `style.css`:
///
/// - `.scopes { gap: 4px; padding: 7px 9px; border-bottom: 1px solid
///   var(--c-hair) }` — the chips from the left of the row, one
///   [`size::SCOPES_GAP`] apart, over a rule the row's bottom pixel.
/// - `.scope { padding: 0 8px; border-radius: 999px; color: var(--c-faint) }`
///   — a word at [`size::BASE`] in a capsule with no border at all.
/// - `.scope.sel { color: var(--c-lav); background: color-mix(in srgb,
///   var(--c-lav) 15%, transparent) }` — the same wash and the same colour
///   the `load` button is drawn in, and that is the mock's own doing rather
///   than a shortcut here: both say *this is where a press lands*, one about a
///   deck and one about a library.
///
/// One row and not a wrap, and at the mock's own width that costs the fourth
/// chip its right-hand half. `.scopes` carries a wrapping flex, and this
/// console draws one row of it and clips — which is [`rend_row_into`]'s answer
/// to the same declaration and a Set name's answer to a row too narrow for it.
/// The four words laid end to end are 246 wide at [`size::BASE`] and the
/// mock's left pane is 218, so `folder` starts inside the bay and finishes
/// outside it: it is drawn, it is marked when it is marked, and what brings
/// the rest of it in is widening the pane, which that boundary allows and no
/// maximum stops.
///
/// The alternative is a row whose height is a measurement, and it is a
/// real one rather than a thing not got to: the mock's own bay wraps to two
/// lines at 218, so a browser draws this row 52 tall where [`size::SCOPES_H`]
/// is 31.5. What it would cost is a bay whose furniture moves when a word
/// changes length — the list one row shorter at one width and not at another —
/// and a height the arrangement's own minimum could not be written from. So
/// the clip is chosen, and it is chosen the same way the same question was
/// answered one bay along.
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

    // `border-bottom: 1px solid var(--c-hair)` — the row's own bottom pixel,
    // and the same hairline the bay head above it and the foot below it are
    // both drawn with. It is inside the row rather than under it, which is
    // what keeps the list's top where [`library_box`] put it.
    let rule = row.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [Pos2::new(row.min.x, rule), Pos2::new(row.max.x, rule)],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
}

/// The path row, painted: the directory this library is pointed at, and
/// the rule under it.
///
/// Where the row goes is [`library`]'s, so this paints and derives nothing —
/// [`scopes_into`]'s rule one row up.
///
/// Term for term from `style.css`:
///
/// - `.path { padding: 4px 10px; color: var(--c-faint); border-bottom: 1px
///   solid var(--c-hair); font-size: 10px }` — the path at
///   [`size::PATH_SIZE`], one [`size::PATH_PAD_X`] in from the left and
///   centred across the row's own height, over a rule the row's bottom pixel.
/// - `.path.incoming { color: var(--c-text) }` — the same row in the panel's
///   text ink while a folder is over the window, which is the whole of the
///   mark that gesture gets: a folder dragged in from outside tells this
///   window a path and never a position, so nothing can be ringed the way
///   `.strip.drop` rings the rectangle a carried Set would land on (ADR-0275).
///   It says nothing about whether the release will be allowed — the text ink
///   is what a word is drawn in when nothing is being said about it.
///
/// A path too long for the row is clipped rather than elided, which is
/// `.lib-row`'s answer one box down and is a departure from this row's own
/// declaration: `.path` sets `text-overflow: ellipsis` where `.lib-row` sets
/// none, and `egui` has no ellipsis to draw here — [`room`](crate::room)'s own
/// sentence about a mock declaration this console cannot honour. What is cut
/// is the end of the path, which is the half that says where you have got
/// to; the alternative is a second layout pass measuring the string against
/// the row, and a readout is not worth a measurement the rest of this bay does
/// not make.
pub(crate) fn path_into(ui: &Ui, pal: &Palette, bay: &LibraryBay, at: Pointed<'_>) {
    let Some(row) = bay.path else {
        return;
    };
    let painter = ui.painter().with_clip_rect(row);
    // `.path` is `--c-faint`; `.path.incoming` is `--c-text`. One line and one
    // colour: the row comes up out of its own faint rather than being drawn a
    // second way, so the line that changes is the line that will hold the
    // answer.
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

    // `border-bottom: 1px solid var(--c-hair)` — the row's own bottom pixel,
    // and the same hairline the scope row above it draws. It is inside the row
    // rather than under it, which is what keeps the list's top where
    // [`library_box`] put it.
    let rule = row.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [Pos2::new(row.min.x, rule), Pos2::new(row.max.x, rule)],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
}
