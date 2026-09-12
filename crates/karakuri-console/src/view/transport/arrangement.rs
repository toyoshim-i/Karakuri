use super::super::*;
use super::*;

// ---------------------------------------------------------------------------
// learn, and the map it writes into
// ---------------------------------------------------------------------------

/// What the `map` pill reads — the name of the map file in use, or `None` for a
/// surface running without one.
///
/// [`AudioIn`]'s shape one pill along, and it is the same seam: a map is a
/// file, `src/` reads none (ADR-0156), so whoever loaded one writes its name
/// here and this crate draws it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MapPill {
    /// What the map is called — the file's stem, which is the name an operator gave
    /// it and not its path (P-0087).
    pub name: Option<String>,
}

impl MapPill {
    /// A surface with no map, which draws `map · none`. It is a state and not an
    /// absence: a run that opened a port and found no file to load is one an
    /// operator can still learn into.
    pub const NONE: MapPill = MapPill { name: None };

    /// What the pill says after `map ·`.
    pub fn word(&self) -> &str {
        self.name.as_deref().unwrap_or(NO_MAP)
    }
}

/// What the pill says where no map is loaded, and it is deliberately not a
/// name: `none` is a word for a state, where a name would be a reading this
/// console invented. [`NO_ARRANGEMENT`]'s argument one pill along, and the same
/// shape [`audio_text`] uses for an input nobody opened.
const NO_MAP: &str = "none";

/// The `learn` pill's word, which is the whole of its label.
const LEARN_LABEL: &str = "learn";

/// The `map` pill's first word, the mock's own abbreviation — `map · <name>`.
const MAP_LABEL: &str = "map";

/// The `learn` pill: the capsule, and whether it is lit.
///
/// A pill and not a capsule with two ends, which the `rec` control is: a press
/// means *the other state* and the word never changes, because *learn* is what
/// the control is rather than what a press will do. It is `.pill.lav` in the
/// mock, lit while armed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LearnPill {
    /// The control: the capsule a press has to land in.
    pub pill: Rect,
    /// Whether learn is armed, read off [`View::learn`] and kept nowhere here.
    pub armed: bool,
}

impl LearnPill {
    /// Whether `p` is on the control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// What a press asks for, which is a state and never a direction (P-0090): the
    /// arming, the other way round.
    ///
    /// It is a `bool` and not an [`Operation`] because a learn is not an operation
    /// — it is a setting of the map layer every surface reaches the vocabulary
    /// through, which is `Vocabulary::Setting`'s own shape
    /// ([ADR-0236](../../../../docs/adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md),
    /// and
    /// [ADR-0336](../../../../docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md)
    /// for the argument against the other reading). The four `mcp` pills are the
    /// same kind of control and carry no operation either.
    pub fn next(&self) -> bool {
        !self.armed
    }
}

/// The `map` pill: where the readout goes.
///
/// No press and no menu, which is what makes this a readout rather than the
/// control the mock draws. The mock's tip says *"Click to save, load, or start
/// a new one"*, and reaching a map while running is not built — the pill says
/// which file is loaded and nothing else. A capsule that looked like a menu and
/// opened none is the scaffolding [`transport`] refuses; a capsule that says a
/// true thing is not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapRow {
    /// The capsule, painted.
    pub pill: Rect,
}

/// Where the `learn` pill goes, or `None` where there is no room or nothing to
/// say.
///
/// `None` where the console has not been told about a map — `View::map` — for
/// [`audio_in`]'s reason exactly: a program with no surface has nothing to
/// learn onto, and a lit-able pill drawn for it would be this crate answering a
/// question about a device on its own authority.
///
/// `layout` must be solved.
pub fn learn_pill(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    map: Option<&MapPill>,
    armed: bool,
) -> Option<LearnPill> {
    map?;
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let text_w = width_of(ctx, LEARN_LABEL);
    let pill_w = size::PILL_PAD_X * 2.0 + text_w;
    let mid = strip.center().y;
    // **Where the group before this one ended** — the tracker group's last
    // chip, the audio-in pill, or the bar. The same one answer [`arrangement`]
    // takes, asked two items further back; the gap is the row's own.
    let after = match (
        tracker_group(ctx, layout, values, audio, tracker),
        audio_in(ctx, layout, values, audio),
    ) {
        (Some(group), _) => group.double.max.x,
        (None, Some(before)) => before.pill.max.x,
        (None, None) => row.bar.max.x,
    };
    let pill = Rect::from_min_size(
        Pos2::new(after + size::TRANSPORT_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    // [`outputs_row`]'s rule: a capsule that does not fit in its row is no
    // control at all rather than half of one over the frame readout.
    if !strip.contains_rect(pill) || pill.max.x + size::TRANSPORT_GAP > row.frame.min.x {
        return None;
    }
    Some(LearnPill { pill, armed })
}

/// Where the `map` pill goes, or `None` where there is no room or nothing to
/// say.
///
/// It follows [`learn_pill`], which is the mock's order — `learn`, then `map ·
/// nanoKONTROL2` — and falls back through the same chain where the `learn` pill
/// did not fit, so one control dropping for want of room does not take the next
/// with it.
pub fn map_pill(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    map: Option<&MapPill>,
) -> Option<MapRow> {
    let map = map?;
    let row = transport(ctx, layout, values)?;
    let strip = to_egui(layout.rect(layout.find("transport")?));
    let words = format!("{MAP_LABEL} · {}", map.word());
    let pill_w = size::PILL_PAD_X * 2.0 + width_of(ctx, &words);
    let mid = strip.center().y;
    let after = match (
        learn_pill(ctx, layout, values, audio, tracker, Some(map), false),
        tracker_group(ctx, layout, values, audio, tracker),
        audio_in(ctx, layout, values, audio),
    ) {
        (Some(learn), _, _) => learn.pill.max.x,
        (None, Some(group), _) => group.double.max.x,
        (None, None, Some(before)) => before.pill.max.x,
        (None, None, None) => row.bar.max.x,
    };
    let pill = Rect::from_min_size(
        Pos2::new(after + size::TRANSPORT_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    if !strip.contains_rect(pill) || pill.max.x + size::TRANSPORT_GAP > row.frame.min.x {
        return None;
    }
    Some(MapRow { pill })
}

/// One text measurement, the way every pill in this row takes one.
fn width_of(ctx: &egui::Context, text: &str) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    })
}

// ---------------------------------------------------------------------------
// The arrangement pill
// ---------------------------------------------------------------------------

/// The pill's first word, the mock's own abbreviation one pill along from `map
/// · nanoKONTROL2 ▾`. The map pill names a file with *save*, *load* and *start
/// a new one* under it, and this is the same three over a different file, so it
/// is the same two-part label.
const ARRANGEMENT_LABEL: &str = "arr";

/// What the pill says where no arrangement has been named, and it is
/// deliberately not a name.
///
/// The default arrangement *has* no name: it reaches [`crate::layout`] rather
/// than a file, so it is the one arrangement nobody could have saved and
/// nothing filed can shadow it
/// ([ADR-0221](../../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)
/// §2). Writing `default` here would be this console inventing one — and a
/// worse invention than most, because an operator *may* save an arrangement
/// called `default` and it shadows nothing, so the pill would read the same for
/// two different states.
///
/// A space is what makes it safe as well as honest: a name is one path
/// component of letters, digits, `-` and `_`, so `the default` is not a name
/// anything can be filed under and no save can make this line ambiguous. It is
/// the preview cell's `C · no slot` one row up — a word for a state, where a
/// name would be a reading invented for a console that has none.
const NO_ARRANGEMENT: &str = "the default";

/// What *save* is called in the menu. The ellipsis is the one thing on this
/// panel that says *this item asks for something before it does anything*, and
/// it is only there while it is true: with a name in use, saving again means
/// that name and asks for nothing, so the word loses it.
const SAVE_ITEM: &str = "save";
const SAVE_ITEM_ASKING: &str = "save as…";

/// What the reset is called in the menu, in the manual's own words rather than
/// in the vocabulary's. *Reset the arrangement* is the row and [`Op::Reset`] is
/// the operation; *start a new one* is what the family reads as from inside
/// this control, where the default is *"one arrangement among the ones you
/// could name"* and not a fourth thing beside the three.
const NEW_ITEM: &str = "start a new one";

/// What the arrangement pill reads this frame, and what its menu is doing.
///
/// # The name and the list are handed in, for [`View::picture`]'s reason
///
/// Which arrangement is in use is *which file was last written or read*, and
/// which names exist is a directory. `src/` takes no device, no window and no
/// clock (ADR-0156) and it takes no disk either — `karakuri-console`'s manifest
/// has no entry that could reach one — so whoever owns the store reads both and
/// writes them here, exactly as whoever owns the engine writes
/// [`View::transport`]. What crosses the seam is a name and a list of names.
///
/// # The menu is not handed in, and that is the other half of the same seam
///
/// [`Menu`] is this crate's: it is what the *control* is doing, not what the
/// instrument is doing, and it moves only through the methods below. A menu
/// open in the program's memory would be the arrangement living in the
/// toolkit's memory one level along (ADR-0156's own argument), and the console
/// would then be drawing a state it could not answer questions about.
///
/// It is held here rather than in [`Panel`] because it is not part of the
/// arrangement: nothing about an open menu is saved, restored or reset, and a
/// [`Panel::restore`] that put somebody else's open menu back would be
/// restoring a gesture.
#[derive(Debug, Clone, PartialEq)]
pub struct Arrangement {
    /// The arrangement in use, or `None` for the default — which is not a name and
    /// is drawn as [`NO_ARRANGEMENT`].
    ///
    /// A save and a restore both put a name here, because both leave that
    /// arrangement the one in use; a reset takes it away, because the default is
    /// what is now on screen and it has no name.
    pub name: Option<String>,
    /// Every name already filed, in the order whoever read the store listed them —
    /// `Store::list_arrangements` sorts by name, so this is alphabetical and the
    /// menu does not sort it again.
    ///
    /// Read when it changes rather than per frame, which is [`View::library`]'s
    /// rule for its reason: a listing is a directory read and that is not a thing
    /// to do on a frame path (P-0091). It changes exactly when a save lands, and
    /// whoever performed the save is who re-reads it.
    ///
    /// Empty is a console with no store behind it — every test in this crate — and
    /// the menu then offers *save* and *start a new one* and lists nothing, which
    /// is honest: there is nothing to put back.
    pub filed: Vec<String>,
    /// What the control is doing. See [`Menu`].
    pub menu: Menu,
}

/// What the pill's menu is doing, and the console's only state that is neither
/// the arrangement nor a value handed in.
///
/// Three states rather than a `bool` and a buffer beside it: *shut*, *open*,
/// and *open with a name being typed into it*. The third is a state of the menu
/// and not a fourth thing, which is what stops a buffer being read while
/// nothing is asking for one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Menu {
    /// The pill alone.
    #[default]
    Shut,
    /// The menu is down: *save*, *start a new one*, and the names filed.
    Open,
    /// The one flow on this panel that asks for letters, with what has been typed
    /// so far.
    ///
    /// The buffer is a `String` this crate owns and whoever holds the keyboard
    /// fills, one character at a time, through [`Arrangement::typed`] and
    /// [`Arrangement::rubbed_out`] — the same split as everything else here, since
    /// `src/` has no key events to read (ADR-0156).
    ///
    /// Nothing in it is checked. A name that is not one path component is refused
    /// where the record is applied, in one sentence, by whoever writes the file —
    /// the surface owns the affordance and never the authority
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md),
    /// [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    /// A pill that quietly dropped the characters it did not like would be a rule
    /// an operator could only find by experiment.
    Naming(String),
}

impl Arrangement {
    /// A console with no store behind it: the default arrangement, nothing filed,
    /// and the menu shut.
    ///
    /// A `const` rather than a `Default` impl alone so that a test — and
    /// [`crate::input::claim`]'s own documentation — can name the state without
    /// building one. Every test in this crate is this.
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

    /// One character into the name being typed, and `false` where nothing was
    /// asking for one.
    ///
    /// Control characters are not a name and never reach the buffer — a newline is
    /// Return arriving as text, which is the commit and not a letter. Everything
    /// else does, unchecked, for the reason [`Menu::Naming`] gives.
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

    /// How many rows the open menu has: *save*, *start a new one*, and one per name
    /// filed. Zero while the menu is shut or asking for a name, which is a menu
    /// with a field in it rather than a list.
    fn rows(&self) -> usize {
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

/// What a press on one of the menu's rows lands on.
///
/// [`ArrangementPill::ask`] turns one of these into what the press *asks for*;
/// this is only which row it was. Split in two so that the hit test and the
/// operation are one derivation asked twice rather than one function that does
/// both — [`Outputs::op`]'s arrangement, over a list.
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

/// What a press on the pill or on one of its rows asks for.
///
/// Every arm is either a move of this control's own state or one named
/// operation, and never a change to the arrangement made here: the pill asks,
/// and whoever applies the record decides
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
/// Nothing is refused in this list. A name nothing is filed under and a file
/// that disagrees with itself are both refused where the bytes are, in one
/// sentence each, and this control cannot see either.
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

/// The arrangement pill, laid out: the capsule, what is written in it, and the
/// menu under it while it is down.
///
/// # One derivation, for [`Outputs`]' reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. Two copies of the arithmetic is a menu
/// row that lights under a pointer that cannot pick it, with nothing on screen
/// saying so.
///
/// # It carries no name and no list
///
/// The rectangles are here and the words are [`Arrangement`]'s, which is why
/// [`ArrangementPill::ask`] takes one: a laid-out pill that had *copied* the
/// name it was measured from is a second copy to drift, and the caller has the
/// first one in its hand already. It is [`Mixer`]'s split with the borrow
/// turned round — the mixer keeps the strips because a knob's position *is* a
/// value, and nothing here moves with the name except the width.
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
    /// How many of [`Arrangement::rows`] the menu has room for, between the pill
    /// and the bottom of the console.
    ///
    /// Fewer than there are is a store with more arrangements than the window is
    /// tall, and the foot says so in the Library bay's own words — `n of m` —
    /// rather than the list quietly ending. Zero while the menu is shut, and while
    /// it is asking for a name: that menu is a field, not a list.
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

    /// Where one row of the menu is, from the top. The rows stack with no gap
    /// between them, which is `.lib-list`'s own reading — the one list in the mock
    /// that has none.
    ///
    /// Panics on a row this menu has not got, which is [`TransportRow::dot`]'s
    /// rule: a caller has invented an item.
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

    /// What a press at `p` asks for, or `None` where the press was on
    /// nothing this control owns.
    ///
    /// The same derivation [`crate::input::claim`] hit-tests, asked a second
    /// time rather than copied — [`Outputs::op`]'s arrangement, and the reason
    /// is the same: the pill that claims a press and the pill that acts on it
    /// cannot come apart.
    ///
    /// # The three answers a press on a row can give
    ///
    /// - Save with a name in use is [`Operation::SaveArrangement`] naming
    ///   it. *"Once a name is in use, saving again means that name: saving
    ///   over it is what saving it again is."* With no name in use there is
    ///   nothing to save over, so it asks for one instead.
    /// - Start a new one is [`Op::Reset`] — the same operation `r`
    ///   performs, reached from the other end of the panel exactly as the
    ///   Outputs row's dot reaches `f`'s fold.
    /// - A name is [`Operation::RestoreArrangement`] naming it, which is
    ///   the reset's own sentence with a name in it.
    ///
    /// A press on the pill toggles the menu; a press anywhere else on the menu
    /// — its padding, its foot — shuts it, because a press that did nothing at
    /// all is the one thing worse than a press that declines.
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

/// The arrangement pill's furniture, derived: the capsule, the words in it, and
/// the menu under it.
///
/// # Where it sits, and why it is after the bar
///
/// `docs/manual/console.html`'s `.transport` is a flex row and this pill is the
/// last item in it before the `.sep`, immediately after `map · nanoKONTROL2 ▾`.
/// Everything the mock draws between the tracker group and this pill — `learn`,
/// and then `map` — is one of the controls [`transport`] names and does not
/// draw, so a flex row closes up and this lands one [`size::TRANSPORT_GAP`]
/// after the octave's second half — or after the audio-in pill on a console
/// with no tracker behind it, or after `bar 37` on one with neither. That is
/// the mock's own layout with the undrawn items taken out, and not a position
/// chosen here.
///
/// It moves with the tempo, by a glyph or two. `92.5` is narrower than `128.0`
/// and everything after it slides, which is what a flex row is and what the
/// beat grid and the bar already do. The alternative — pinning it to the right
/// edge, where the mock's `landed` and `rec` sit — buys a control that never
/// moves and puts it in the group the manual does not put it in.
///
/// # No row means no pill, and that is the row's answer rather than a second
/// one
///
/// `None` wherever [`transport`] answers `None`: the row folded away, soloed
/// away, too narrow, or a console with no engine behind it. The last is the one
/// worth stating, because the arrangement exists whether or not a tempo does —
/// but the *row* does not, and a pill floating in a bay that is drawing nothing
/// at all would be a control in a row that is not there. Where the row is, this
/// asks it for the bar's right edge and lays out from there, so the pill's
/// place and the readouts' places are one derivation.
///
/// # What it costs to ask
///
/// One galley lookup for the pill's own words, always. While the menu is down
/// it is one more per row — the two verbs and every name filed — since the card
/// is as wide as the widest thing in it, and a name this crate never measured
/// would be a name drawn outside its own card. Paid on a pointer event and on a
/// frame, and only while the menu is open, which is a menu an operator is
/// looking at.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
pub fn arrangement(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    values: Option<Transport>,
    audio: Option<&AudioIn>,
    tracker: Option<Tracker>,
    // **The map, only so this pill knows where the group before it ended.**
    // `learn` and `map` sit between the tracker group and this one in the
    // mock, and a console that has not been told about a surface draws
    // neither — so this is `None` on every run without one and the pill lands
    // exactly where it did before they existed.
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

    let words = pill_text(arr);
    let text_w = width(&words);
    // `.pill`'s `padding: 0 8px` around the words, one `.sink` gap, and the
    // chevron — the one gap the mock states inside a capsule.
    let pill_w = size::PILL_PAD_X * 2.0 + text_w + size::SINK_GAP + CHEVRON_W;
    let mid = strip.center().y;
    // **Where the group before this one ended**, which is the tracker group's
    // last chip where the console has been told about the tracker, the
    // audio-in pill where it has been told only about audio, and the bar where
    // it has been told neither — the same one-answer arrangement [`look`]
    // takes of *this* pill, asked one item further back. The gap is the row's
    // own either way: what ends before this pill is a whole group, and
    // `.tracker`'s tighter 5 is the gap *inside* that group.
    let after = match (
        map_pill(ctx, layout, values, audio, tracker, map),
        learn_pill(ctx, layout, values, audio, tracker, map, false),
        tracker_group(ctx, layout, values, audio, tracker),
        audio_in(ctx, layout, values, audio),
    ) {
        (Some(map), _, _, _) => map.pill.max.x,
        (None, Some(learn), _, _) => learn.pill.max.x,
        (None, None, Some(group), _) => group.double.max.x,
        (None, None, None, Some(before)) => before.pill.max.x,
        (None, None, None, None) => row.bar.max.x,
    };
    let pill = Rect::from_min_size(
        Pos2::new(after + size::TRANSPORT_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    // The same rule [`outputs_row`] states: a capsule that does not fit in the
    // row it is drawn in is no control at all, rather than half of one over
    // the frame readout.
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

/// The pill's words: `arr · night`, or `arr · the default`.
///
/// One string rather than three galleys laid end to end, for [`frame_job`]'s
/// reason: the mock writes one run of text and laying it out as one keeps the
/// spaces round the `·` the type's own rather than a gap this file invented.
fn pill_text(arr: &Arrangement) -> String {
    format!("{ARRANGEMENT_LABEL} · {}", arr.word())
}

/// The menu card under the pill: where it is, how many rows fit in it, and how
/// many there are.
///
/// `(None, 0, 0)` while the menu is shut, which is the ordinary state and costs
/// one branch.
///
/// # It hangs from the pill and is held inside the console
///
/// Down from the pill's bottom edge by one [`size::PILL_GAP`], left-aligned
/// with it, and pushed back inside the viewport's right edge where a long name
/// would take it past — a card half outside the window is a list with items
/// nobody can read.
///
/// The bottom is a count and not a clip. The transport row is at the top of the
/// console, so a menu hanging down has the whole window; where a store holds
/// more arrangements than that window is tall, the card lists as many as fit
/// and says `n of m` in the Library bay's own foot. Truncating in silence is
/// the failure P-0094 is about, and this is that bay's answer to the same
/// question rather than a second one.
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

    // **Asking for a name is a field and not a list**, so the card is one row
    // wide enough to type into and there is nothing to pick.
    if let Some(typed) = arr.naming() {
        let field = width(&naming_text(typed)).max(pill.width());
        let card = held_inside(
            &viewport,
            pill.min.x,
            top,
            field + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
            size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H,
        );
        return (Some(card), 0, 0);
    }

    let of = arr.rows();
    let widest = std::iter::once(save_word(arr))
        .chain(std::iter::once(NEW_ITEM))
        .chain(arr.filed.iter().map(String::as_str))
        .map(width)
        .fold(pill.width(), f32::max);
    // How many rows there is room for between the card's top and the bottom of
    // the console, once the padding and the rule under the verbs are paid for.
    // The foot is only owed where something is left out, so it is asked for
    // twice: once assuming it is not there and once assuming it is.
    let furniture = size::LIB_LIST_PAD * 2.0 + size::HAIRLINE;
    let room = |foot: f32| {
        (((viewport.max.y - top - furniture - foot) / size::LIB_ROW_H).floor()).max(0.0) as usize
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

/// The name being typed, with the caret after it — `night▏`.
///
/// One run of text rather than a galley and a drawn bar, so that measuring the
/// field and painting it cannot be two different runs: the caret is the width
/// of the caret whatever the face is, and a field measured without it would put
/// the caret outside its own card at the moment the name filled it.
fn naming_text(typed: &str) -> String {
    format!("{typed}{CARET}")
}

/// The arrangement pill, painted, and the menu under it.
///
/// Where everything goes is [`arrangement`]'s, so this paints and derives
/// nothing. Term for term from `.pill` in `style.css`:
///
/// - `border: 1px solid var(--c-line); border-radius: 999px; padding: 0 8px;
///   color: var(--c-dim)` — a capsule with a hairline round it, which is
///   [`pill_at`]'s treatment and this is the same pill in another row.
/// - the `▾` — `--c-dim` with the words, since it is part of the same run in
///   the mock's markup.
///
/// The menu has no term in the stylesheet, because the mock draws no menu: it
/// is the Library bay's list, which is the one list this console already
/// draws, at the same `.lib-row` box and inside the same `.lib-list` padding
/// (P-0085). It sits on a card with the panel's own shadow under it, which is
/// what says it is above the bays rather than inside one.
/// The `learn` pill, painted — lavender while armed, the row's own outline
/// while it is not.
///
/// `.pill.lav` is the mock's class on this control and the palette's `lav` is
/// that colour, so *armed* is drawn in the ink the page already gave it rather
/// than in one this file chose. The word never changes: *learn* is what the
/// control is, and what a press will do is said by the lamp — which is the
/// distinction the `rec` capsule draws the other way, being one control with
/// two ends.
pub(crate) fn learn_into(ui: &Ui, pal: &Palette, pill: &LearnPill) {
    let painter = ui.painter();
    let radius = CornerRadius::same((size::PILL_H * 0.5) as u8);
    let ink = match pill.armed {
        true => pal.lav,
        false => pal.dim,
    };
    if pill.armed {
        painter.rect_filled(pill.pill, radius, pal.tint);
    }
    painter.rect_stroke(
        pill.pill,
        radius,
        Stroke::new(size::HAIRLINE, ink),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        LEARN_LABEL.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        ink,
    );
    painter.galley(
        Pos2::new(
            pill.pill.min.x + size::PILL_PAD_X,
            pill.pill.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );
}

/// The `map` pill, painted — `map · default`, and no chevron.
///
/// The chevron is the one thing left off the mock's own drawing, and it is left
/// off on purpose: `▾` on this console means *there is a menu under this*,
/// which the arrangement pill and the `audio-in` pill both keep, and there is
/// no menu here. Drawing one over a readout would be the scaffolding
/// [`transport`] refuses — a control that looks like a control and does
/// nothing.
pub(crate) fn map_into(ui: &Ui, pal: &Palette, pill: &MapRow, map: &MapPill) {
    let painter = ui.painter();
    painter.rect_stroke(
        pill.pill,
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let galley = painter.layout_no_wrap(
        format!("{MAP_LABEL} · {}", map.word()),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.dim,
    );
    painter.galley(
        Pos2::new(
            pill.pill.min.x + size::PILL_PAD_X,
            pill.pill.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.dim,
    );
}

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
    painter.add(egui::Shape::convex_polygon(
        vec![
            pill.chevron.left_top(),
            pill.chevron.right_top(),
            Pos2::new(pill.chevron.center().x, pill.chevron.max.y),
        ],
        pal.dim,
        Stroke::NONE,
    ));

    let Some(card) = pill.menu else {
        return;
    };
    popup_card(painter, pal, card);

    let row_text = |painter: &egui::Painter, rect: Rect, text: String, colour: Color32| {
        let galley = painter.layout_no_wrap(
            text,
            FontId::new(size::BASE, FontFamily::Proportional),
            colour,
        );
        painter.galley(
            Pos2::new(
                rect.min.x + size::LIB_ROW_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            colour,
        );
    };

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
        row_text(painter, field, naming_text(typed), pal.text);
        return;
    }

    for index in 0..pill.rows {
        let rect = pill.row(index);
        let (text, colour) = match index {
            0 => (save_word(arr).to_owned(), pal.text),
            1 => (NEW_ITEM.to_owned(), pal.text),
            other => (arr.filed[other - VERBS].clone(), pal.dim),
        };
        row_text(painter, rect, text, colour);
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
