use super::*;

// ---------------------------------------------------------------------------
// The Library bay
// ---------------------------------------------------------------------------

/// The word at the head of the Library bay, in the source's own capitalisation
/// for [`Kind::Bay`]'s reason: the mock upper-cases in CSS, and that is done at
/// paint time so the word a reader searches for is the word in the source.
pub(super) const LIBRARY_TITLE: &str = "Library";

/// The word in the foot's button, which is the whole of the mock's own `load` —
/// the first of the three the readout `load &rarr; A` became (ADR-0305).
///
/// The arrow is not here, and that is the older change: it was typed, as
/// `\u{2192}`, and `egui`'s default face has no rightwards arrow — so the panel
/// drew `load □ A` and the pill's one job, saying where a press would land, was
/// done through a tofu. Whether a glyph is in the default face is a question
/// with no good answer ([`CHEVRON_W`], three bays along, and [`arrow_mark`] one
/// bay along), so the mark is drawn and an arrow is the same arrow either way —
/// see [`LOAD_ARROW`].
///
/// The letter is not here either: it is [`View::selection`]'s, read at the one
/// place both marks are, and [`LibraryBay::load`] is where the word, the mark
/// and the letter are measured together — so the pill that is painted and the
/// pill a test asks about are one statement.
const LOAD_PILL: &str = "load";

/// The `→` between the word and the letter, drawn rather than typed —
/// [`CHEVRON_W`]'s reason and [`arrow_mark`]'s shape, which is what the two
/// scrub arrows in the Inspector already are.
///
/// Half the type it sits beside, which is [`CHEVRON_W`]'s own rule and is about
/// what the glyph's ink measures at [`size::BASE`]. It is square rather than
/// [`CHEVRON_W`]'s 2:1, for the reason [`arrow_mark`] gives: the ink of a
/// right-pointing small triangle is about as wide as it is tall.
const LOAD_ARROW: f32 = size::BASE * 0.5;

/// What a row menu's four load items are called, without the letter — `Load to
/// Slot A`, and it is the maintainer's own spelling
/// ([ADR-0311](../../../../docs/adr/0311-a-row-menu-loads-a-set-onto-a-named-deck-and-saves-it-through-the-systems-own-dialog.md)).
///
/// It says *slot* where the rest of this panel says *deck*, and that is kept.
/// The words the operator asked for are the words the menu carries; what the
/// item asks for is `Operation::LoadSet { deck, .. }` either way, and the two
/// are the same thing under two names on this panel already — a deck holds one
/// to four slots and the mixer draws a strip per slot, so the letter names
/// both.
const MENU_LOAD: &str = "Load to Slot";

/// The one item of a row menu that is not a load, under the separator.
///
/// It names the *file* it writes rather than the act, which is the mock's own
/// wording and is what tells it apart from the four above it: the loads move a
/// deck and this one leaves a file behind.
const MENU_SAVE: &str = "Save as a kbset";

// -- the star at the left of every row --------------------------------------

/// The box the star stands in, which is the box the glyph it stands in for
/// would have had: `.lib-row`'s type is the console's [`size::BASE`] and the
/// mark is square at that size, which is [`arrow_mark`]'s rule for a mark drawn
/// instead of typed.
///
/// Derived rather than transcribed for that reason — the mock sets no width on
/// `.star` at all, because a glyph has one.
const STAR_SIZE: f32 = size::BASE;

/// The gap between the star and the name, which is `.lib-row`'s `gap: 7px` —
/// the row is a flex of the star, the name and the time, and this is the one
/// spacing in it that is not padding.
///
/// It happens to be [`size::LIB_ROW_PAD_X`]'s number and is not read off it:
/// one is the row's padding and the other is the gap between two of its
/// children, and they move for different reasons.
const STAR_GAP: f32 = 7.0;

/// How far in from a star's own box its points reach. A five-pointed star is
/// ten rim points at two radii, and this is the inner one as a fraction of the
/// outer.
///
/// The console's own number: a pentagram's exact inner radius is about `0.382`
/// of its outer and reads as a spike at eleven pixels, so this is the fatter
/// star a small mark wants. Nothing in the mock says it, because the mock has a
/// glyph.
const STAR_WAIST: f32 = 0.46;

// -- what one Set holds and declares, opened under its row ------------------

/// The word the foot's first capsule reads, which is the mock's own `params` —
/// the chip between the count and the `load` button.
///
/// # It was `read` until 2026-09-09
///
/// The maintainer's own reading of the word: *"readだと意味わかりづらいな"* — `read` is
/// unclear. It names what the press does to the file, and an operator who has
/// not pressed it has no way to know what appears; `params` names the block,
/// which is what nine of the ten rows under it are. The other candidate was
/// `expand`, and it lost on the same test: it names the *motion*, and nothing
/// else on this panel is named for one — `load`, `go`, `rec`, `keep`, `solo`
/// are all nouns or verbs about the instrument
/// ([ADR-0312](../../../../docs/adr/0312-the-params-pill-is-a-toggle-and-the-library-bay-scrolls.md)).
///
/// # It carries a lit state now, and it did not
///
/// This said *"it carries no lit state"*, and the argument it carried is kept
/// where it is still true: a run of lines appears under the cursor, *"which is
/// not a thing anybody misses"*, so the block is the loudest part of the state.
/// That was an argument for the block being enough and not for the chip being
/// wrong, and rule 03 of [the manual](../../../../docs/manual/index.html) asks
/// a symbol what state it is in. So it is [`armed_pill_at`]'s mint while a
/// reading is open and [`pill_at`]'s hairline round `--c-dim` while none is.
///
/// Mint and not lavender, which is the half of the old argument that survives
/// intact: lavender is *the deck the keys are addressed to*, this bay spends it
/// on [`LOAD_PILL`], and a second lavender capsule beside it would make the
/// colour mean two things at a width of eight characters. Mint is what this
/// console draws *on* in. Pink was the other pressed treatment and is wrong
/// here: pink is *live* on this panel — a deck on air, a recording running —
/// and a Set being looked at is not live.
///
/// The note says nine lines and the mock draws ten, and neither number is
/// transcribed here for that reason: how many rows a reading is, is
/// [`Reading::rows`]'s answer over the Set it is of — a head, one per key, the
/// capacity and the emitted attributes where there are any, and a foot. Over
/// the mock's own Set that answer is ten, so the derivation agrees with the
/// markup rather than with the note.
///
/// Both numbers are in `docs/manual/console.html` and the disagreement is only
/// here. The page says nine twice — this chip's tip, *"nine lines that were not
/// there a moment ago"*, and the comment over the block, *"one box rather than
/// nine loose rows"* — and draws ten `.lib-row`s between them, without anywhere
/// saying the two are different numbers. So somebody editing the mock meets one
/// of them and not the pair, and the page is where that would be repaired; it
/// is written down here because this is the file that had to choose.
const PARAMS_PILL: &str = "params";

/// The two words at the head of a reading: what the row under the cursor
/// declares, and how many controls that comes to.
///
/// `.addr` in the mock, which is the one place in this bay the lav is spent on
/// something that is not the load — it is the same mark the Inspector writes a
/// node's address in, and it says *this box is a reading of the row above it*
/// rather than a sixth Set.
const READING_HEAD: &str = "declares";

/// The word the capacity row is drawn under, which the mock draws in the same
/// range-and-default shape as a knob *"because that is how a procedure declares
/// it"*.
const READING_CAPACITY: &str = "capacity";

/// The word the emitted attributes are drawn under — *"what a renderer drawn
/// over it can consume"*.
const READING_EMITS: &str = "emits";

/// One published control of a Set, as a reading of it.
///
/// Named for what `karakuri_engine::set::Set::published` calls the same thing,
/// and read off the cards instead of off a built Set: this crate takes no
/// engine (ADR-0156), and a reading that had to build one would be the load
/// this chip exists to be pressed before.
///
/// One row per key and never one per node, which is that function's own rule: a
/// name two nodes declare is a single control over the part of the range both
/// of them accept. The mock's note says why that is the list the Inspector will
/// draw — *"`exposure` is one row here for the same reason it is one knob
/// there"*.
///
/// Both halves are the host's, which is [`Node::addr`]'s seam one bay along:
/// this crate reads no store (ADR-0156), so what crosses is the key and the
/// words drawn beside it. The position is deliberately not here — the note:
/// *"That number is the deck's published interface in order, and printing it
/// here would be a promise about a map made before the load that put the
/// interface there."*
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
    /// The key the control is published under — `radius`, `exposure`.
    pub key: String,
    /// The declared range and the default, as the host spelled them: the mock's `0
    /// – 8 · 2`. A range and never a value: a Set on disk has nothing turned to
    /// anything, and the number under a hand is the Inspector's to show after the
    /// load.
    pub range: String,
}

/// What the Set under the cursor holds and declares, opened under its row.
///
/// # It is not a second Inspector, and the difference is not one of degree
///
/// `console.html`'s note is the whole of the argument: the Inspector groups by
/// node because the two things it draws beside a value — who is holding it and
/// what authority it is under — are one node's, and a node is a thing a deck is
/// running. Nothing here is running, so there is no address to group under, no
/// value to show, nobody holding anything and no `man / sug / auto` to set.
/// What is left is what a procedure declares.
///
/// # Every line is a card's, and one figure is deliberately absent
///
/// The knobs, the capacity and the emitted attributes are each a record on the
/// metadata card the store keeps beside an artifact, so the row's promise on
/// `docs/manual/operations.html` — *"each read off the artifact's own card, so
/// those three fetch no source and compile nothing"* — holds for all three.
/// What a node's element storage comes to is not here, and it is the one thing
/// the MCP `read_set` tool answers that this does not: sizing it needs every
/// source in the Set fetched and checked
/// (`karakuri_environment::setfile::load`), which is the cost that sentence
/// says these three do not pay. The tip is the half that gave way, and
/// `console.html`'s note says where the figure goes instead — *"A model asking
/// over MCP gets it and pays for it; a press in a library list is not the place
/// to spend that"*.
///
/// # A node with no card is counted rather than drawn
///
/// [`Reading::described`] is how many of [`Reading::nodes`] had one, and the
/// foot says so. A card is derived rather than kept, so an artifact stored as
/// bytes has none — *"an ordinary state of a working store rather than a
/// damaged one"* — and the one thing that would make this reading lie is a knob
/// missing from the list without a word.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Reading {
    /// Which Set it is a reading of, which is what keeps a reading and the row it
    /// is drawn under from coming apart: [`View::opened`] draws it only under the
    /// row of that name, so a listing rewritten under a reading closes it rather
    /// than filing it under whatever is now in that position.
    pub id: String,
    /// One per published key, in the order the Set publishes them.
    pub knobs: Vec<Published>,
    /// How many elements the geometry declares it can carry and how many it takes
    /// by default, in [`Published::range`]'s own shape — or `None` where nothing in
    /// the Set declares a capacity, which is a Set with no geometry in it. `None`
    /// draws no row: a row with nothing after it is the blank the mock's tip
    /// refuses.
    pub capacity: Option<String>,
    /// What the geometry emits, as the host joined them — or `None` where nothing
    /// in the Set declares any, on [`Reading::capacity`]'s rule.
    pub emits: Option<String>,
    /// How many nodes the Set file names.
    pub nodes: usize,
    /// How many of those the store holds a card for, which is how many of them this
    /// reading could read at all.
    pub described: usize,
}

impl Reading {
    /// How many rows it is drawn in: the head, one per knob, the capacity and the
    /// emitted attributes where there are any, and the foot.
    ///
    /// The head and the foot are always drawn, which is what makes a reading with
    /// nothing in it an *answer* rather than an empty box: `0 knobs` under a Set
    /// that declares none is the same sentence `karakuri-environment`'s card
    /// renderer says in words.
    pub fn rows(&self) -> usize {
        2 + self.knobs.len()
            + usize::from(self.capacity.is_some())
            + usize::from(self.emits.is_some())
    }

    /// The head's right-hand word: `6 knobs`.
    pub fn knobs_word(&self) -> String {
        format!(
            "{} knob{}",
            self.knobs.len(),
            match self.knobs.len() {
                1 => "",
                _ => "s",
            }
        )
    }

    /// The foot's left-hand word: `5 nodes`.
    pub fn nodes_word(&self) -> String {
        format!(
            "{} node{}",
            self.nodes,
            match self.nodes {
                1 => "",
                _ => "s",
            }
        )
    }

    /// The foot's right-hand word: the mock's `all described`, or how many nodes
    /// declared nothing this could read.
    ///
    /// The mock draws the first of the two and this has to be able to say the
    /// second, which is the tip's own instruction: a node with no card *"is counted
    /// here rather than drawn as a blank row"*. How that count is spelled is not on
    /// the page, so it is spelled here — in the words the card reader already uses
    /// for the same absence.
    pub fn cards_word(&self) -> String {
        match self.nodes.saturating_sub(self.described) {
            0 => "all described".to_owned(),
            missing => format!("{missing} without a card"),
        }
    }
}

/// A reading and the row it is open under, handed to [`library`] together.
///
/// Two halves rather than one because neither is the other's: what the reading
/// says is the host's answer, and which row it opens under is the console's own
/// cursor. They are handed in as a pair so that the bay cannot lay a block out
/// under one row and paint it under another — [`View::opened`] is the one place
/// they are put together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opened<'a> {
    /// Which row of the listing the reading belongs to — [`View::cursor_row`].
    pub at: usize,
    /// What that Set declares.
    pub reading: &'a Reading,
}

/// The box a reading is drawn in, inside [`LibraryBay::list`] and under the row
/// the cursor is on.
///
/// One box rather than a run of loose rows, which is the mock's own reading of
/// it: the block belongs to the row above it, and a reading drawn flat into
/// this list would read as Sets nested under a Set. So it stands on the well
/// the staging lane's candidates stand on, inside [`size::READING_MARGIN_X`] of
/// the list either side and at [`size::READING_RADIUS`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Block {
    /// The well itself: the rows' own box, without the margin around it.
    pub well: Rect,
    /// How many rows are in it — [`Reading::rows`], whether or not the list had
    /// room for them. The list clips, exactly as it clips a name too long for the
    /// track.
    pub rows: usize,
    /// The first row of the *listing* drawn under the block, which is the cursor's
    /// own row plus one. [`LibraryBay::row`] pushes every row from here down by
    /// what the block takes.
    pub under: usize,
}

impl Block {
    /// The `index`th row of the reading, counting from the top of the well.
    ///
    /// [`LibraryBay::row`]'s derivation one box in: the rows are a stride and a
    /// count, at [`size::LIB_ROW_H`] because a reading's row is a `.lib-row` with
    /// its left padding overridden and nothing else changed.
    pub fn row(&self, index: usize) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                self.well.min.x,
                self.well.min.y + size::LIB_ROW_H * index as f32,
            ),
            egui::vec2(self.well.width(), size::LIB_ROW_H),
        )
    }
}

/// What a press on the `params` chip asks for.
///
/// Two answers because the press means two different things and only one of
/// them is an operation. Opening asks the host to read a Set — the operation
/// carries the id, the host answers it off the cards, and the record it writes
/// is `Silent(Question)`, *it asks rather than changes*. Closing asks nothing
/// at all: what it changes is which rows this bay is drawing, which is the
/// console's own state and is no more an operation than a fold is. A press that
/// emitted `ReadSet` to put a reading away would say a question was asked at
/// the moment one stopped being.
///
/// [`TransitionRow::go`]'s shape three bays along, where one control's press
/// also leaves by two doors.
#[derive(Debug, Clone, PartialEq)]
pub enum Read {
    /// Nothing is open under the cursor: read the Set that is there.
    Open(Operation),
    /// The reading under the cursor is open, and the press puts it away.
    Shut,
}

/// A library row taken in hand: which row, and the Set on it.
///
/// [`Chosen`]'s shape two rows up — a value travelling beside what the press
/// meant, because the press means more than one operation can say. There the
/// extra value is the chip `SelectScope` cannot name; here there is no
/// operation at all yet, and both halves are wanted by different callers:
/// [`crate::panel::Panel::carry`] takes the Set, and the row is what marks the
/// list while a hand is on it.
///
/// The row is a position and not a second copy of the name, which is
/// [`View::cursor_row`]'s own rule: what the mark can be drawn on is a place in
/// the listing this console was handed, and a name kept beside it would go on
/// naming a Set after the listing had been rewritten under it.
#[derive(Debug, Clone, PartialEq)]
pub struct Taken {
    /// Which row of the listing, counting from the top of the drawn list.
    pub row: usize,
    /// The Set on it, by the name the listing carried.
    pub set: String,
    /// Whether that name is a procedure's — which decides whether a drop over a
    /// strip names `Operation::LoadProcedure` or `Operation::LoadSet` (ADR-0338).
    /// It travels with the name because the operation is built at the release,
    /// where the listing this row came from is no longer in hand.
    pub procedure: bool,
}

// -- what one row of the listing is -------------------------------------

/// What one row of the Library bay's listing is: a Set or a procedure, and the
/// layers its badge names.
///
/// # Both facts, because a row draws two things off them
///
/// The badge is what the row implements — a Set's is the layers its own files
/// fill, a procedure's is the one `kind` it declares — and it is a readout with
/// nothing to press. Whether the row is a procedure decides three other things:
/// no star (a star is a control over a Set this store holds and is refused on
/// anything else, ADR-0299), no `params` reading yet, and a load that names
/// [`Operation::LoadProcedure`] rather than [`Operation::LoadSet`] — one layer
/// of what the deck is playing replaced, where a Set load replaces every layer
/// (`docs/adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md`).
///
/// # The default is a Set with no badges, and that is what makes the seam cheap
///
/// [`View::kinds`] is a parallel row to [`View::library`] and a host that has
/// not written it leaves every row *a Set the console knows nothing else about*
/// — which is what this bay drew before ADR-0338 and is every test in this
/// crate that does not say otherwise. So a console handed a listing and no
/// kinds behaves exactly as it did.
///
/// The badges are [`Layer`] values and not words, so the chip that filters by a
/// kind and the badge that reports one are spelled once, in [`LAYERS`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowKind {
    /// The layers this row implements, in the order they are drawn — a Set's slots,
    /// or a procedure's one `kind`. Empty is a row with no badge, which is a Set
    /// the host could not read and a procedure that declares no kind at all.
    pub badges: Vec<Layer>,
    /// A procedure and not a Set.
    pub procedure: bool,
}

/// The listing, as the controls of this bay read it: the names and what each
/// row is.
///
/// # One argument because they are one reading
///
/// [`Listed`]'s rule at the seam rather than at the paint: the host writes
/// [`View::library`] and [`View::kinds`] on one press, and a control handed the
/// two halves separately could star a row whose kind it read out of a listing
/// that had been rewritten under it. Borrowed for [`Filters`]' reason — nothing
/// is cloned to answer a pointer event.
///
/// A row past the end of either half is not a row. [`Rows::name`] answers
/// `None` there and every control in this bay refuses rather than clamps, which
/// is `LibraryBay::take`'s own rule: an index answered bare would name a row
/// nobody can see.
#[derive(Debug, Clone, Copy)]
pub struct Rows<'a> {
    /// The names the bay lists, in the order the host listed them —
    /// [`View::library`].
    pub names: &'a [String],
    /// What each of those rows is — [`View::kinds`], in the same order. Shorter
    /// than `names` is not an error: a row past the end of it is a Set with no
    /// badges, which is this seam's own default.
    pub kinds: &'a [RowKind],
}

impl<'a> Rows<'a> {
    /// Nothing listed at all, which is a scope whose rows this control cannot name
    /// — `history` under [`View::rows`], whose rows are versions.
    pub const NONE: Rows<'static> = Rows {
        names: &[],
        kinds: &[],
    };

    /// How many rows there are, which is the names': the kinds are a decoration of
    /// them and a shorter row of kinds lists nothing away.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether the listing holds nothing.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// The name on this row, or `None` past the end.
    pub fn name(&self, row: usize) -> Option<&'a str> {
        self.names.get(row).map(String::as_str)
    }

    /// Whether this row is a procedure — `false` for a row past the end of either
    /// half, which is [`RowKind`]'s default and is what a control that has already
    /// refused the row never asks.
    pub fn procedure(&self, row: usize) -> bool {
        self.kinds.get(row).is_some_and(|kind| kind.procedure)
    }

    /// The name on this row where it is a Set, and `None` where it is a procedure
    /// or past the end.
    ///
    /// It is what every control whose operand is a Set id takes — the star, the
    /// `params` chip and the row menu's send — so a procedure row hands them
    /// nothing rather than being special-cased at each of them, which is
    /// [`Scope::lists_sets`]'s arrangement one question finer.
    pub fn set(&self, row: usize) -> Option<&'a str> {
        (!self.procedure(row)).then(|| self.name(row)).flatten()
    }

    /// The words on this row's badges, in the order they are drawn — [`LAYERS`]'
    /// spelling, so a badge and the chip that filters by it read the same word.
    /// Empty for a row with no badge and for a row past the end.
    pub fn badges(&self, row: usize) -> Vec<&'static str> {
        self.kinds
            .get(row)
            .map(|kind| {
                kind.badges
                    .iter()
                    .filter_map(|layer| {
                        LAYERS
                            .iter()
                            .find(|(kind, _)| kind == layer)
                            .map(|(_, word)| *word)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

// -- the library's filter field and its kind chips -----------------------

/// What the `holds` field reads with no filter set, which is the mock's own
/// `holds&hellip;` — the ellipsis is the placeholder saying the field is empty,
/// and a field that *is* set reads the value instead. That is the whole of the
/// difference between the two states, because `.field` has one rule in
/// `style.css` and no set variant: inventing a second colour for it would be a
/// reading the mock never took.
pub const HOLDS_UNSET: &str = "holds…";

/// The layers, in the order the kind chips draw them and with the word each
/// chip carries.
///
/// Curated here because the vocabulary has no list to cycle, which is
/// [`WIPE_SHAPES`]' whole argument three bays along:
/// `karakuri_operation::Layer` is an enum with no `ALL` and no word for a
/// member, so a control that draws it has to say which order and which
/// spelling. The spellings are the five the MCP `list_sets` tool takes and the
/// five the Inspector's `.addr` writes — `L1:0`, `L4` — so a kind a hand
/// presses and a layer a model names are the same five words.
///
/// It was the `layer` field's cycle until 2026-09-10, and what reads it now is
/// [`KindChip`]: ADR-0338 replaced that field with six toggles, and the list of
/// five outlived the control because the list was never what was wrong with it
/// — *"ADR-0262 was right about the list and wrong about the control, and what
/// changed is that the list grew a sixth member which is not a layer at all"*.
///
/// Five and not every [`Layer`] there is, since `kind L5` joined the vocabulary
/// (ADR-0340): the word for it, the seventh chip and the badge are that
/// record's own pass, which draws them here and in the mock together. An array
/// is not a match, so a variant added to the vocabulary does not fail to
/// compile here — this is the line to read the day one lands, and
/// `tests/library.rs` counts them.
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

/// One chip of the kind filter row: a layer, or the Sets.
///
/// # Six and not five, because a Set is not a kind
///
/// A procedure declares one of [`Layer`]'s five kinds; a Set fills several and
/// declares none. So *is this its kind* is a question a procedure answers and a
/// Set does not, and the sixth chip asks the other question — *show me the
/// Sets* — which is why the row is a partition of the rows rather than a filter
/// over one kind of them
/// (`docs/adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md`).
///
/// # A toggle, and the arithmetic is this console's
///
/// [`karakuri_operation::LibraryKinds`] is six named booleans and says nothing
/// about a press ([P-0090]). A press here flips this chip's field and sends all
/// six — [`LibraryBay::kind`] — because what leaves has to be a destination
/// rather than a step: six statements each saying *this one changed* are six
/// things a second surface can arrive in the middle of.
///
/// [P-0090]: ../../../docs/principles/0090-a-surface-offers-it-never-decides.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KindChip {
    /// One of the five kinds a procedure declares.
    Layer(Layer),
    /// The Sets, which is the row kind that declares no `kind` at all.
    Sets,
}

impl KindChip {
    /// The six, left to right in the order `.lib-kinds` draws them — the five of
    /// [`LAYERS`] and the Sets last, which is the mock's own `L1 L2 L3 L4 FIELD
    /// SET`.
    ///
    /// A seventh for `kind L5` is ADR-0340's, whose own pass draws it here and in
    /// the mock together: a control the panel draws that the mock does not is a
    /// defect (`docs/contributing.md` §5), and the payload already carries the
    /// field it would set — [`KindChip::on`] reads it.
    pub const ALL: [KindChip; 6] = [
        KindChip::Layer(Layer::L1),
        KindChip::Layer(Layer::L2),
        KindChip::Layer(Layer::L3),
        KindChip::Layer(Layer::L4),
        KindChip::Layer(Layer::Field),
        KindChip::Sets,
    ];

    /// The word in the chip — [`LAYERS`]' spelling, or [`SETS_CHIP`].
    ///
    /// A layer with no entry in [`LAYERS`] reads [`SETS_CHIP`], which is
    /// unreachable while that list is exhaustive and is the fallback rather than a
    /// panic for [`Filters::holds_word`]'s reason: a bay draws what it was handed,
    /// and a missing word is a chip an operator cannot read rather than a run that
    /// stops.
    pub fn word(self) -> &'static str {
        match self {
            KindChip::Sets => SETS_CHIP,
            KindChip::Layer(layer) => LAYERS
                .iter()
                .find(|(kind, _)| *kind == layer)
                .map_or(SETS_CHIP, |(_, word)| *word),
        }
    }

    /// Whether this chip is on, read off the payload the host was last handed — the
    /// mint `.kind.on` against the plain `.kind`.
    ///
    /// It is the chip's own field and never [`LibraryKinds::narrowing`]: a row with
    /// nothing on shows everything, and drawing all six lit for it would say six
    /// presses had been made.
    pub fn on(self, kinds: LibraryKinds) -> bool {
        match self {
            KindChip::Sets => kinds.sets,
            KindChip::Layer(Layer::L1) => kinds.l1,
            KindChip::Layer(Layer::L2) => kinds.l2,
            KindChip::Layer(Layer::L3) => kinds.l3,
            KindChip::Layer(Layer::L4) => kinds.l4,
            KindChip::Layer(Layer::Field) => kinds.field,
            // **Answered, and not yet drawn.** `KindChip` is parameterised by
            // [`Layer`], so the sixth kind gives it a variant by existing;
            // whether the filter row shows a seventh chip is
            // [`KindChip::ALL`]'s question and is the pass that gives the
            // chain its slots.
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

/// Which filter field a press landed on, and there is one.
///
/// # It has one variant, and it is an enum rather than nothing
///
/// `Field::Layer` was the second until 2026-09-10, when ADR-0338 replaced the
/// `layer…` field with [`KindChip`]'s six toggles — *"a filter over a closed
/// list of six is a set of toggles, because every subset of six is askable and
/// a position in a cycle can only ever name one"*. What is left is `holds…`,
/// which is untouched and is still a first cut (ADR-0292 took its premise away
/// and ADR-0338 does not answer it).
///
/// Kept as a name rather than collapsed into the method, for [`Knob`]'s reason:
/// [`LibraryBay::field`] and [`LibraryBay::filter`] are asked *which field*,
/// the row is drawn from a list, and a caller passing nothing would have to be
/// edited again the day the row grows the filter this one is a first cut of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    /// `holds…` — what a node in the Set is called.
    Holds,
}

impl Field {
    /// The row's fields, left to right, in the order `.lib-filters` draws them.
    pub const ALL: [Field; 1] = [Field::Holds];
}

/// What the two filter fields are narrowing the listing to, read off the
/// console for the frame that draws them and the press that changes them.
///
/// It is [`Operation::ListSets`]'s two payload fields, borrowed: `holds` points
/// into [`View::holds`], which is the row of candidates the host handed in, so
/// nothing is cloned to draw a frame and the `String` is built once, at the
/// press, where the operation is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Filters<'a> {
    /// Part of a node's name, or `None` for a field nobody has set.
    pub holds: Option<&'a str>,
    /// Which kinds of row the listing shows, which is the six chips under the field
    /// — [`karakuri_operation::LibraryKinds`], and [`LibraryKinds::EVERYTHING`]
    /// where none of them is on.
    ///
    /// A payload and not a position, where `layer` beside it was an `Option<Layer>`
    /// until 2026-09-10: a cycle can name six of the sixty-four states this row
    /// has, and a filter over a closed list of six is every subset of it
    /// (ADR-0338).
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

/// Where the `holds` filter goes when the field is pressed: the next candidate
/// along, and off the end back to nothing.
///
/// `choices` is [`View::holds`] — the node names the host read off the store —
/// and a filter that is not among them steps to the first, which is the state a
/// host that rewrote the candidates leaves behind.
///
/// With no candidates at all it stays `None`, and the press is still answered:
/// what it asks for is the listing again, unnarrowed, which is the same
/// question `LibraryBay::chip` answers when the chip already marked is pressed.
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

/// Where this library is pointed: the directory in the `.path` row, and whether
/// what it reads is a folder that has *arrived* or one that is on its way in.
///
/// # One string for two states, because the row is one row
///
/// `console.html`'s *How a folder is chosen, and why the drop is the window's*
/// gives that row two readings and never both at once — the directory the bay
/// is pointed at, and *"the path a release would set"* while a folder is over
/// the window. So this is the row's own value: what is drawn, and which of the
/// two it is. [`View::pointed`] is where the second wins over the first, and it
/// is the whole of the difference between them here — a hover replaces the line
/// rather than adding one, because the mock has one `.path` and a second would
/// be a place, which is the one thing this gesture cannot say.
///
/// # A path this crate is handed and never reads
///
/// It is a `&str` the host formatted, for [`View::library`]'s reason two rows
/// down: a directory is a thing on a disk, and this crate reaches no disk at
/// all (ADR-0156). Whether it is a directory is not this type's question either
/// — that is asked of the file system once, on the drop, by whoever owns it
/// (ADR-0275), and a bay that asked it would be asking it on a frame (P-0091).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointed<'a> {
    /// The directory, as the host spells it.
    pub path: &'a str,
    /// A folder is over the window and this is what a release would set.
    ///
    /// `.path.incoming` in `style.css`, which is the row in `--c-text` rather than
    /// its own `--c-faint` — and it is the whole of the mark, because a drop
    /// carries no pointer position and there is no rectangle to ring (ADR-0275). It
    /// says nothing about whether the release will be allowed: one path is refused
    /// for being a file and two are refused for being two, and neither is known
    /// before the release.
    pub incoming: bool,
}

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

/// The Library bay, laid out: where the rows go, how many of them there is
/// room for, and where the count under them goes.
///
/// # What the bay is standing on: a scope, and the listing that scope answers
///
/// The manual: *"A scope and a walk, not one flat list: favourites, my sets,
/// app presets, a folder."* The scopes are drawn, and all four are answered
/// now — `all`, which is
/// [`karakuri_store::Store::list_sets`](../../../../crates/karakuri-store/src/store.rs)
/// and is a directory of Set files; `my sets`, which is the starred subset of
/// it and is `<store>/favourites.json` intersected with that listing
/// (ADR-0299); `presets`, which is the `.kset` files in the root the program
/// was told about (ADR-0230); and `folder`, which is the Set files in a
/// directory somebody dropped on this window during the run (ADR-0275).
///
/// The chips answer a press, which is `console.html`'s own affordance on
/// each of them — *"Click to show it; click another scope to leave it"* — and
/// it is the row `docs/manual/operations.html` names as this operation's home.
/// What a press asks for is [`LibraryBay::chip`], and it is asked of the same
/// derivation that paints the capsule. A scope can still answer with
/// nothing, and that is a question asked and answered rather than a chip
/// gone quiet: a store nobody has starred in, and a folder nobody has pointed
/// anywhere. The host says which of them it is, in the words at its own key.
///
/// Both halves are handed in. The scopes are a slice and the rows are a
/// slice, and which rows go with which scope is the host's answer rather than
/// this bay's: a listing is a directory read, a frame path does not do those
/// (P-0091), and this crate could not do it anyway (ADR-0156). So the bay
/// draws the row of questions it was given and the answer to the one that is
/// marked.
///
/// # The `.path` row, and it is where this library is pointed
///
/// `~/sets/tour-2026/night-b › opening` in the mock, between the scope row and
/// the filters, and it is drawn whichever scope is marked — because it is
/// two things at once (`console.html`): the walk inside a folder scope, and
/// the place a send's save dialog opens on, so a Set handed to somebody starts
/// where the listing is
/// ([ADR-0311](../../../../docs/adr/0311-a-row-menu-loads-a-set-onto-a-named-deck-and-saves-it-through-the-systems-own-dialog.md)).
/// It said *the place a send lands* until 2026-09-09, which was ADR-0267
/// and is superseded: the file is named in the system's own dialog now, and
/// where it lands is the operator's answer rather than this row's.
///
/// Until a folder has been dropped the row is not drawn at all, so the bay
/// is one line shorter and the scopes sit straight on the filters. That is the
/// staging lane's own answer to an empty lane read one bay up: a row saying
/// there is no folder would be a sentence about an absence. It is why
/// [`LibraryBay::path`] is an `Option` and why every rectangle under it moves
/// with it.
///
/// While a folder is over the window it reads the path a release would
/// set, in `--c-text` where the row is otherwise `--c-faint` — `.path`
/// against `.path.incoming`, and the whole of the mark this gesture gets. The
/// drop-mark idiom one bay over does not transfer and the page says so
/// plainly: that mark says *where*, and a folder coming in from the desktop
/// carries no pointer position at all (ADR-0275). See [`Pointed`], which is
/// the row's value, and `path_into`, which paints it.
///
/// It is a readout and takes no press. Nothing in [`crate::input::claim`]
/// hit-tests it, and the reason is its own rather than the foot's: re-pointing
/// the bay is another drop, and the gesture that points it is not one this
/// panel can offer as a capsule. The foot's own readout stopped being one on
/// 2026-09-08 — `load &rarr; A` is a button and a pulldown now (ADR-0305) —
/// which is why this row no longer cites it.
///
/// [P-0090]: ../../../docs/principles/0090-a-surface-offers-it-never-decides.md
///
/// # What is in the mock's bay and is deliberately not here
///
/// - The `+` at the end of the scope row. Adding a scope is the arena's
///   own gap drawn a fifth time, which [`outputs`] already names, and what it
///   would add is a folder — which is the chip already drawn, and what that
///   chip waits on is a directory rather than a fifth chip ([`Scope`]).
/// - `.lib-row .dim`, the time beside each name. This one is different
///   from the others and is worth the sentence: the *value* exists —
///   `SetEntry::written` is the Set file's own mtime — and what does not exist
///   is a spelling for it. The one answer in this workspace is
///   `karakuri_environment::setfile::written_at`, local time to the second
///   where the mock's column is `16:09`. It is reachable now — ADR-0214
///   moved it out of a package with no library target, which is the reason
///   this comment used to give — but not from here: `karakuri-console` takes
///   no engine, no store and no environment by design, so the host formats it
///   and hands it in, the way every other derived value in this module
///   arrives. Writing a second spelling here would be the kind of second
///   answer this repository deletes rather than adds.
/// # `.lib-row`'s `.star` is here, and it is the row's first control
///
/// A hollow star at the left of every row, filled on the rows the store has
/// starred — the mock's `.star` and `.star.off`, `--c-sun` against
/// `--c-faint`. It was the last of the six things this bay drew nothing for,
/// and what it was short of was somewhere for the value to be: ADR-0299 put it
/// in `<store>/favourites.json`, beside the Sets, so a star does not travel
/// with a Set file and the file stays byte for byte what it was.
///
/// The mark is drawn rather than typed, for [`LOAD_ARROW`]'s reason one
/// row down: whether `☆` is in `egui`'s default face is a question with no
/// good answer, and a control whose one job is saying *starred or not* must
/// not do it through a tofu. See [`star_mark`], and [`STAR_SIZE`] for the box
/// it stands in — which is the box the glyph would have had, so the name
/// beside it starts where `.lib-row`'s `gap: 7px` puts it either way.
///
/// A press names the state and does not flip one
/// ([`LibraryBay::starred`]): `Operation::SetFavourite { id, favourite }` is
/// what leaves, carrying the state the row is being put *in*, because a map
/// with a button per direction and a model that says which one it wants both
/// have to be able to say *star this* and mean it.
///
/// Which rows are starred is handed in, like the listing above it and for
/// the same reason: the marks are a file beside the Sets and this crate reads
/// no store (ADR-0156). See [`View::starred`].
///
/// # The `.lib-filters` fields are here, and the passage that said they could
/// not be was wrong rather than stale
///
/// It read: *"nothing in the store answers it: `list_sets` reads names off a
/// directory and no index anywhere says what a Set holds"*. The first half is
/// true of `karakuri_store::Store::list_sets` and the second was already false
/// when it was written — `karakuri_environment::setfile::summarise` reads what
/// each Set holds, node by node, off the Set file and the cards behind it, and
/// `karakuri-environment`'s MCP `list_sets` has been applying both filters
/// against it. What was missing was that this bay's host asked the *store* for
/// a list of names instead of asking that. It does not any more, and the two
/// fields are [`LibraryBay::field`] and [`LibraryBay::filter`] — drawn, hit
/// tested, and stepping rather than taking letters, which is that method's
/// argument and the one thing about them a maintainer may want back.
///
/// # What is in the mock's bay and is here, which is the load route
///
/// `.lib-row.cursor`, and the three things the foot's readout became.
/// `console.html`'s *How a Set reaches a deck* is what settles their shape:
/// *"what was missing was never the operation but the route"*, and there are
/// three routes now. The key is the cursor and the deck *selection* — *"a
/// cursor and a key with no pointer anywhere in it"* — the drag names both
/// operands in one gesture, and the button in the foot takes the Set from
/// the cursor and the deck from the pulldown beside it.
///
/// The cursor is moved by a pointer as well as by the keys, and that is
/// the drag below arriving rather than a second control: a press on a row
/// takes that Set in hand ([`LibraryBay::take`]) and the mark follows the
/// hand, because the mark on a row is what this bay already draws for *which
/// row* and the mock draws nothing else a carry could use.
///
/// # The foot was a readout until 2026-09-08, and what it is now
///
/// `load → A` said where a *key* press would land before it was made and
/// answered no pointer at all: nothing in [`crate::input::claim`] hit-tested
/// it, and this section used to end *what this bay owes is the drag; what it
/// must not grow is a button*. It grew one, and the argument that was
/// against it is in
/// [ADR-0305](../../../../docs/adr/0305-the-library-bays-load-is-a-button-and-a-pulldown-and-the-deck-it-names-is-not-the-selection.md)
/// rather than deleted, because it is a good argument that lost to one fact:
/// a readout reading the selection cannot aim a load at a deck without taking
/// the keys off the deck being played.
///
/// So the foot is three things and two of them are controls.
/// [`LibraryBay::load`] lays out all three; [`LibraryBay::aim`] is what a
/// press on either capsule asks for, and the `→` between them is a label on
/// the foot's own ground that answers no pointer — which is the sentence this
/// section used to make about the whole pill, kept where it is still true.
///
/// The deck is [`View::target`] and never [`View::selection`], and that is
/// the whole of the record: the ring on a strip and the letter in this foot
/// are free to name two different decks, `l` goes on loading onto the ring's,
/// and a press on the pulldown emits nothing at all.
///
/// This row's badge names three panel routes now. A chip is the control
/// the page names for its row; when the drag landed, a press on a row and a
/// release over a strip are what made this badge true, and `operations.html`
/// read `library → deck`. It gained the button with ADR-0305 and a row menu's
/// four load items with ADR-0311, reading
/// `load button, row menu → load to slot, or library → deck`, and all three
/// arrive at one `Operation::LoadSet`.
///
/// # A Set is written out from a row's own menu, and the file is named outside
/// # this program
///
/// `docs/manual/operations.html`'s *Send a Set to somebody, and take one in*
/// names two directions and this bay is the home of both. Both are reached
/// from here now. The taking-in half is *"Taking one in is not a second row —
/// opening a preset is this row"*, and a row of a `folder` the bay has been
/// pointed at is that same row again: a press packages the file into the store
/// and then loads it, which the host performs and this bay's list is the
/// picker for. The sending half is [`RowMenu`]'s `Save as a kbset`, under the
/// separator, and what this bay emits for it is
/// `Operation::TransferSet { transfer: SetTransfer::Send { id } }` and nothing
/// else.
///
/// The asymmetry this section was written about is what decided the
/// control. Taking in names a file that exists and sending names one that
/// does not yet — and no listing can point at a file nobody has written. That
/// is why the destination is the platform's ask rather than a row of anything
/// drawn here.
///
/// The operation carries no destination and never will. ADR-0260 refused
/// the premise that it owes one — sending is a *read*, and a read's answer goes
/// where the surface that asked puts answers, so `SetTransfer::Send { id }`
/// gains no field. What *this* surface asks with is the system's own save
/// dialog, opened by the host on the file the operator names
/// ([ADR-0311](../../../../docs/adr/0311-a-row-menu-loads-a-set-onto-a-named-deck-and-saves-it-through-the-systems-own-dialog.md)),
/// and no path crosses this crate at all: nothing here reads a disk
/// (ADR-0156) and nothing here spells a place.
///
/// It was the `.path` row's folder until 2026-09-09, which is
/// [ADR-0267](../../../../docs/adr/0267-the-panel-sends-into-the-folder-the-library-bay-is-pointed-at-and-the-destination-is-drawn-before-the-press.md),
/// superseded by ADR-0311. That row is still drawn and is still this library's
/// readout — [`Pointed`], and ADR-0275's — and what it is for a send now is
/// where the dialog opens rather than where the file lands.
///
/// And the one control here that asks for letters could not spell a path
/// anyway. [`Menu::Naming`] is it, and what it takes is a name —
/// ADR-0221's *one path component of letters, digits, `-` and `_`*. ADR-0229
/// says in as many words why that rule does not stretch: *"an include is a
/// relative path and has separators in it by construction, so the rule cannot
/// be copied."* Nothing needs it to: the file is named in a window this program
/// does not own, which takes no rule from ADR-0221 because it is the operator's
/// own file system asked by the operator's own tool.
///
/// The pulldown's letter is [`View::target`], which this console keeps —
/// ADR-0219 recorded a deck mark as living *"in the specification and not in
/// `karakuri-console`'s code"*, and that is the sentence this bay's letter
/// waited on. It read [`View::selection`] until ADR-0305 split the readout,
/// and the paragraph above is where the two marks are held apart. Either way it
/// is refused past the strips the mixer is drawing, so the letter never names a
/// deck the press would be turned down on — and a row menu's items are cut to
/// the same count for the same reason.
///
/// The key is `l`, chosen by `docs/manual/operations.html` because which
/// keys exist is that page's to say (ADR-0198, ADR-0220) — this module reads
/// the choice and does not make it, and nothing here presses anything: the
/// press is the host's, and what it re-points is the slot's *source*, so the
/// worker builds the Set and the watchdog judges it exactly as it does an
/// edit.
///
/// # The foot's number is the mock's own, read the mock's way
///
/// `5 of 27` is how many rows are drawn whole against how many the scope
/// holds, and both halves are here: the total is what the harness handed over,
/// and the count is [`LibraryBay::rows`]. So a library taller than its list
/// says so in the one place the mock puts it.
///
/// And what is out of sight is reachable, which it was not. This paragraph
/// said this bay does not scroll — *"which is honest, because it has no
/// scroll position and inventing one here would be a control"* — and then, once
/// an Inspector pane had one
/// ([ADR-0307](../../../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)),
/// that the reason this list had none was unchanged. Both are gone: the bay
/// scrolls, the position is [`View::library_scroll`], and the wheel over the
/// bay is the way in
/// ([ADR-0312](../../../../docs/adr/0312-the-params-pill-is-a-toggle-and-the-library-bay-scrolls.md)).
///
/// What the old argument got right is what the count still does. A row out
/// of reach *was* a row a press would name and a hand could not see, and that
/// is exactly the failure the pair [`rows`](Self::rows) and
/// [`drawn`](Self::drawn) is arranged against: the cursor is held inside the
/// rows the bay is drawing ([`View::walk`]), a press outside the list reaches
/// no row at all, and the foot counts the whole ones.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LibraryBay {
    /// `.scopes`: the row of chips between the bay head and the list, one
    /// [`size::SCOPES_H`] tall and the full width of the bay, with its own rule
    /// along the bottom of it.
    ///
    /// `None` where the console was handed no scopes at all, which is a console
    /// nobody has told what libraries there are — every test in this crate that
    /// does not say otherwise. The list then starts directly under the head, which
    /// is where it started before this row was drawn: a band of empty card with a
    /// rule under it would be a scope row with no questions in it.
    pub scopes: Option<Rect>,
    /// `.path`: the directory this library is pointed at, between the scope row and
    /// the filters, one [`size::PATH_H`] tall and the full width of the bay, with
    /// its own rule along the bottom of it.
    ///
    /// `None` until a folder has been chosen, which is every run before a drop and
    /// every test in this crate that does not say otherwise — see this module's
    /// [`library`] and the section on this row at [`LibraryBay`] for why an absent
    /// row rather than an empty one.
    ///
    /// It is not [`scopes`](Self::scopes)' condition and not the filters'. A
    /// console handed no scopes at all can still be pointed somewhere — the row
    /// says where a send lands, and that is true of a bay with no chips drawn over
    /// it — so the two are independent and the arithmetic below takes them in the
    /// order the mock stacks them.
    pub path: Option<Rect>,
    /// `.lib-filters`: the row of two fields between the scope row and the list,
    /// one [`size::LIB_FILTERS_H`] tall and the full width of the bay, with its own
    /// rule along the bottom of it.
    ///
    /// `None` wherever [`scopes`](Self::scopes) is, and that is one condition
    /// rather than two: a filter is a question about a listing, and a console
    /// nobody has told what libraries there are has no listing to ask it of. It is
    /// also `None` in a bay too narrow to hold two fields, which is `list`'s own
    /// rule stated on a row that has a `gap` in the middle of it — see
    /// `library_box`.
    pub filters: Option<Rect>,
    /// `.lib-kinds`: the row of six kind toggles under the filter field, one
    /// [`size::LIB_KINDS_H`] tall and the full width of the bay, with its own rule
    /// along the bottom of it.
    ///
    /// `None` wherever [`filters`](Self::filters) is, and that is one condition
    /// rather than two: both are the bay's head rather than its body — one narrows
    /// the listing by what a node is called and the other by what a row *is* — so a
    /// console that has been told about no library draws neither, and a bay too
    /// narrow to hold the field is too narrow to hold six chips.
    ///
    /// Under the fields and not beside them, which is `style.css`'s own note:
    /// `.field` carries `flex: 1` and six chips sharing one row with it in a bay
    /// this narrow would leave the chips a few pixels each, and a chip nobody can
    /// hit is not a control (ADR-0338).
    pub kinds: Option<Rect>,
    /// `.lib-list`'s content box: the region under the bay head and the scope row
    /// and above the foot, inside [`size::LIB_LIST_PAD`], where the rows are laid
    /// from the top with no gap between them.
    pub list: Rect,
    /// How many rows this bay is drawing whole, which is the `n` of the `n of m` in
    /// its foot — rule 04 of [the manual](../../../../docs/manual/index.html): *"A
    /// list that showed you part of itself says so and says how much."*
    ///
    /// # It is the readout's number and not the walk's
    ///
    /// [`LibraryBay::drawn`] is what is painted and what a press is hit-tested
    /// against, and it is the wider of the two: a row cut by the top edge or the
    /// bottom one is drawn as far as the list goes and can be pressed where it is
    /// drawn. This counts the ones that are whole, so that `m of m` means *nothing
    /// is out of sight* and can never be read off a bay with a row hanging over an
    /// edge. [`InspectorPane::shown`] is the same pair one bay over, and for the
    /// same reason
    /// ([ADR-0312](../../../../docs/adr/0312-the-params-pill-is-a-toggle-and-the-library-bay-scrolls.md),
    /// [ADR-0307](../../../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)).
    ///
    /// It used to be how many were drawn, and the two were one number, because
    /// there was nowhere to scroll to: a listing longer than the list simply
    /// stopped, and the rows past the end were unreachable by any means. At rest
    /// the two are still one number, so every reading of the foot that was true
    /// before is true now.
    ///
    /// Zero is a state now, and it is the one the scope row bought. A scope that
    /// holds nothing is a question that has been asked and answered — *my sets*
    /// with nothing starred, a presets root nobody filled — so the chips are drawn,
    /// the list is empty and the foot reads `0 of 0`. That is not the row of zeroes
    /// ADR-0177 is about: that one is a reading nobody took, and this is the answer
    /// to a question the chip above it is asking. A bay with no *room* for a row is
    /// still no bay at all — see `library_box`.
    pub rows: usize,
    /// How far down the listing this bay has come, as it is drawn — the clamped
    /// position, and never the stored one.
    ///
    /// # Clamped here and stored nowhere
    ///
    /// The clamp is `0 ..= (content - list height)`, and both ends of that range
    /// move when a divider moves — so a clamp written back into [`View`] would be a
    /// resize rewriting what an operator scrolled to. That is
    /// [`InspectorPane::scroll`]'s rule one bay over and
    /// [P-0082](../../../../docs/principles/0082-looking-never-writes-back.md): a
    /// shorter bay draws less of the same position and stores nothing, so dragging
    /// it back reproduces what was on screen exactly rather than nearly.
    ///
    /// What [`View::scroll_library_by`] clamps is the *content*, which is a reading
    /// of the listing rather than of a viewport.
    pub scroll: f32,
    /// How tall the whole listing comes to, the reading block included, whether or
    /// not any of it is on screen — [`library_content_h`].
    ///
    /// It is the number [`scroll`](Self::scroll) is clamped against and the number
    /// [`View::scroll_library_by`] is clamped against, derived in one place so the
    /// two cannot disagree.
    pub content: f32,
    /// The whole bay, which is what a wheel over it is aimed at —
    /// [`crate::input::wheeled`], the region rather than the list.
    ///
    /// The head rows are part of the thing being scrolled, which is
    /// [`InspectorPane`]'s own answer: a hand resting over the scope chips or the
    /// filter fields turns the list under them, exactly as a hand over a deck head
    /// turns that pane. The foot is in it for the same reason and for one more — it
    /// is the readout the scroll is about.
    pub bay: Rect,
    /// How many Sets the selected scope holds, which is what the harness handed
    /// over. The second half of the foot's `n of m`.
    pub total: usize,
    /// `.lib-foot`, along the bottom edge of the bay, with its rule on top.
    pub foot: Rect,
    /// The reading open under the cursor, or `None` where nothing is open — which
    /// is every console until a press on the `params` chip, and every one of this
    /// crate's tests that does not say otherwise.
    ///
    /// `None` also where the row it would open under is not drawn, which is the one
    /// state the two halves can be in and disagree about: a cursor clamps against
    /// the listing ([`View::cursor_row`]) and the rows clamp against the room there
    /// is, so a bay with room for two rows and a reading of the fifth has nowhere
    /// to put it. It is answered here rather than at the paint, so what is laid out
    /// and what is painted are one statement.
    pub reading: Option<Block>,
}

/// What the foot's load control is aimed at, and whether its list is down.
///
/// [`Filters`]' shape one row up: the three values [`LibraryBay::load`] needs
/// to lay itself out, read off the [`View`] once for the frame and handed in,
/// so the capsule that is painted and the capsule a press lands on are one
/// derivation of one reading. See [`View::target`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    /// Which deck a press on `load` lands on, as a slot number — the letter the
    /// pulldown shows. See [`View::target_deck`], which is where the argument for a
    /// second mark is.
    ///
    /// A deck [`DECK_LETTERS`] has no letter for is a caller's error, not a state:
    /// [`View::aim_at`] refuses one, exactly as [`View::select`] refuses a
    /// selection past the strips.
    pub deck: u8,
    /// How many decks the pulldown offers, which is how many strips the mixer is
    /// drawing — [`View::mixer`]'s length, arriving the way every other reading of
    /// the deck does.
    ///
    /// `console.html`: *"A deck the mixer is drawing no strip for is not in the
    /// list, which is the count `0`–`3` are refused on"*. So this is
    /// [`View::select`]'s own count read a second time and not a second rule: the
    /// strips are what both of them count.
    pub decks: usize,
    /// Whether the list is down. The console's own state, like
    /// [`Arrangement::menu`] — see [`View::target_open`].
    pub open: bool,
}

impl Target {
    /// The letter the pulldown shows, which is the letter of the deck a press on
    /// `load` lands on.
    ///
    /// Panics on a deck there is no letter for, which is [`LibraryBay::row`]'s
    /// rule: the caller has invented a deck, and [`View::aim_at`] is what stops one
    /// being invented here.
    pub fn letter(&self) -> &'static str {
        DECK_LETTERS[usize::from(self.deck)]
    }
}

/// What a press on the foot's load control asks for, and it is a control in
/// three parts: a button, a label and a pulldown.
///
/// [`Ask`]'s shape three bays along, and the two enums are the same division:
/// every arm is either a move of this control's own state or one named
/// operation, and never a load performed here ([P-0090]). The pulldown asks for
/// nothing at all — picking a deck is this bay's own mark moving, the way the
/// cursor in the list above it is — which is why it has an arm of its own
/// rather than an `Operation`.
///
/// [P-0090]: ../../../docs/principles/0090-a-surface-offers-it-never-decides.md
#[derive(Debug, Clone, PartialEq)]
pub enum Aim {
    /// Put the list down — a press on the pulldown with it shut.
    Open,
    /// Take it away — a press on the pulldown again, or anywhere else while it is
    /// down. The press is spent on the dismissal, which is
    /// [`crate::input::claim`]'s rule 2 said in the control: a list that is down is
    /// a hand mid-choice, and the next press is part of that gesture.
    Shut,
    /// A deck named, and nothing asked for: the target moves to this slot and the
    /// list goes away. [`View::aim_at`] is the one door into it.
    Deck(u8),
    /// The load, named — `Operation::LoadSet { deck, set }`, with the deck off the
    /// pulldown and the Set off the list's cursor.
    Load(Operation),
    /// A press on `load` with no row under the cursor, which is a library listing
    /// nothing. [`crate::panel::Released::Nowhere`]'s answer one bay along: the
    /// control says what it did with the press rather than going quiet, and nothing
    /// is emitted for a load with one operand missing.
    NoSet,
}

/// What a press on a node group's `uses` line asks for, and it is a control in
/// two parts: a capsule and the card it puts down.
///
/// [`Aim`]'s shape one bay along and the same division: every arm is either a
/// move of this console's own state or one named operation, and never a
/// rewiring performed here
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
/// The capsule asks for nothing at all — opening a list is this console's own
/// pointer moving — which is why it has an arm of its own rather than an
/// `Operation`, and why the row it belongs to owes the operations page nothing
/// extra (ADR-0305).
#[derive(Debug, Clone, PartialEq)]
pub enum Wiring {
    /// A capsule pressed, named by the pane, the node and which of that node's
    /// inputs it is. The host opens that card; a press on the capsule of a card
    /// that is already down never reaches this arm, because a press outside the
    /// card is the dismissal and the capsule is outside it.
    Chip {
        pane: usize,
        node: usize,
        input: usize,
    },
    /// Take the card away — a press anywhere but inside it while it is down. The
    /// press is spent on the dismissal, which is [`crate::input::claim`]'s rule 2
    /// said in the control.
    Shut,
    /// The rewiring, named — `Operation::WireInput { deck, node, slot, to }`, with
    /// the node and the input off the line the card came out of and `to` off the
    /// row that was picked.
    Pick(Operation),
}

/// What a press on a pane head's `▾` asks for, and it is a control in two
/// parts: a mark and the card it puts down.
///
/// [`Wiring`]'s shape one row up and the same division: every arm is either a
/// move of this console's own state or one named operation, and never a pane
/// pointed here
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
/// The mark asks for nothing at all — opening a list is this console's own
/// pointer moving — which is why it has an arm of its own rather than an
/// `Operation`, and why the operations page carries one row for this control
/// and not two (ADR-0305).
#[derive(Debug, Clone, PartialEq)]
pub enum Pointing {
    /// The mark pressed, named by the pane whose head it is in. The host puts that
    /// card down; a press on the mark of a card that is already down never reaches
    /// this arm, because a press outside the card is the dismissal and the mark is
    /// outside it.
    Mark(usize),
    /// Take the card away — a press anywhere but inside it while it is down. The
    /// press is spent on the dismissal, which is [`crate::input::claim`]'s rule 2
    /// said in the control.
    Shut,
    /// The pointing, named — `Operation::PointPane { pane, deck }`, with the pane
    /// off the head the card came out of and the deck off the row that was picked.
    Pick(Operation),
}

/// The foot's load control, laid out: the `load` button, the `→` label, the
/// pulldown and the list under it.
///
/// [`AudioInPill`]'s shape two bays along and for the same reason — one
/// derivation, so that what [`library_into`] paints and what a test asks about
/// are the same rectangles.
///
/// # It was one capsule and a readout until 2026-09-08
///
/// `load → A` was a readout: the cursor said which Set, the deck selection said
/// which deck, and the pill said where a press would land before it was made —
/// so a load was *"a cursor and a key with no pointer anywhere in it"* and
/// nothing in [`crate::input::claim`] hit-tested any of it. What that could not
/// do is aim a load at a deck without taking the keys off the deck being
/// played, and that is what the split bought
/// ([ADR-0305](../../../../docs/adr/0305-the-library-bays-load-is-a-button-and-a-pulldown-and-the-deck-it-names-is-not-the-selection.md)).
///
/// Two of the three parts are controls and the middle one is not. The button
/// asks for the load, the pulldown names the deck, and the `→` between them is
/// punctuation on the foot's own ground — untipped in the mock, like the `5 of
/// 27` at the other end of the same row, because this page tips controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Load {
    /// The `load` button, [`size::PILL_H`] tall, one [`size::LIB_FOOT_GAP`] before
    /// the label.
    pub button: Rect,
    /// Where [`LOAD_PILL`]'s word is painted, inside the button's padding.
    pub text: Rect,
    /// The `→` label's box, [`LOAD_ARROW`] square between the two capsules and on
    /// neither of them. Drawn rather than typed — see [`LOAD_ARROW`], and
    /// [`arrow_mark`], which draws it.
    ///
    /// It takes no press, which is the one thing in this foot that is neither a
    /// control nor a reading of a value: it says how to read the two capsules
    /// either side of it, and `tests/library.rs` sweeps it with the foot's own
    /// ground.
    pub arrow: Rect,
    /// The pulldown's capsule, at the far end of the foot.
    pub deck: Rect,
    /// Where the deck's letter is painted, inside the capsule's padding.
    pub letter: Rect,
    /// The `▾` after it. Drawn rather than typed — see [`CHEVRON_W`], which is the
    /// same mark the two menu pills in the transport row already carry.
    pub chevron: Rect,
    /// How many decks the list offers, which is [`Target::decks`] while it is down
    /// and zero while it is shut — [`ArrangementPill::rows`]'s shape, and what
    /// stops [`Load::row`] handing out a rectangle for a list that is not there.
    pub rows: usize,
}

impl Load {
    /// Whether `p` is on the `load` button.
    pub fn hit_button(&self, p: karakuri_layout::Point) -> bool {
        self.button.contains(Pos2::new(p.x, p.y))
    }

    /// Whether `p` is on the pulldown's capsule.
    pub fn hit_deck(&self, p: karakuri_layout::Point) -> bool {
        self.deck.contains(Pos2::new(p.x, p.y))
    }

    /// The list under the pulldown, or `None` while it is shut — and `None` for a
    /// console with no strip to offer, which is every test in this crate that does
    /// not hand a mixer in.
    ///
    /// # It hangs up, where the arrangement pill's menu hangs down
    ///
    /// Both hang into the room there is. That pill is in the transport row at the
    /// top of the console, so its card has the whole window under it; this one is
    /// in the foot of a bay, which is the bottom of it, and what is under it is
    /// whatever the operator has dragged the boundary to. So the card stands on the
    /// pulldown's top edge, one [`size::PILL_GAP`] clear of it, over this bay's own
    /// list.
    ///
    /// The rows are not counted against the room, which is the other half of the
    /// same difference: the arrangement's card lists a *store* and says `n of m`
    /// where the window cannot hold it, and this one lists at most [`DECKS`] rows —
    /// a window too short for four rows of type is a window with no transport row
    /// in it either. It is held inside the viewport so a very short console draws
    /// it over the bays above rather than off the top.
    pub fn list(&self, viewport: Rect) -> Option<Rect> {
        if self.rows == 0 {
            return None;
        }
        let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * self.rows as f32;
        let width = self
            .deck
            .width()
            .max(size::LIB_ROW_H + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0);
        Some(held_inside(
            &viewport,
            self.deck.min.x,
            self.deck.min.y - size::PILL_GAP - height,
            width,
            height,
        ))
    }

    /// Where one row of the list is, from the top of `card` — the decks in
    /// [`DECK_LETTERS`] order, stacked with no gap between them, which is
    /// `.lib-list`'s own reading and [`ArrangementPill::row`]'s.
    ///
    /// Panics on a row this list has not got, which is that method's rule: a caller
    /// has invented a deck.
    pub fn row(&self, card: Rect, index: usize) -> Rect {
        assert!(index < self.rows, "deck {index} of a list of {}", self.rows);
        Rect::from_min_size(
            Pos2::new(
                card.min.x + size::LIB_LIST_PAD,
                card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32,
            ),
            egui::vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
        )
    }

    /// Which deck `p` is on, or `None` for a point on no row — the card's padding,
    /// or anywhere off it.
    pub fn picked(&self, viewport: Rect, p: karakuri_layout::Point) -> Option<u8> {
        let card = self.list(viewport)?;
        let at = Pos2::new(p.x, p.y);
        (0..self.rows)
            .find(|index| self.row(card, *index).contains(at))
            .map(|index| index as u8)
    }
}

/// What a row's own menu is open on, and how many decks it offers.
///
/// [`Target`]'s shape one control along, and the difference is where the *open*
/// went: a pulldown is a capsule that is there whether or not its list is down,
/// so [`Target`] carries a `bool` beside the deck; a row menu is the card and
/// nothing else, so *which row* and *whether it is down* are one field. An open
/// menu with no row under it cannot be spelled.
///
/// See [`View::menued`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Menued {
    /// Which row of the listing the menu is down on, or `None` for no menu at all —
    /// which is every console until a secondary press on a row.
    ///
    /// It is a row of the *drawn* list, the way [`LibraryBay::take`]'s answer is: a
    /// row the bay had no room for is a row nobody pressed.
    pub row: Option<usize>,
    /// How many decks the loads offer, which is how many strips the mixer is
    /// drawing — [`Target::decks`]'s own reading, arriving the same way.
    ///
    /// `console.html`: *"A deck the mixer is drawing no strip for is not offered,
    /// which is the count the pulldown's list is cut to rather than a second
    /// rule"*.
    pub decks: usize,
    /// Whether this row can be sent, which is whether it is a Set.
    ///
    /// `Save as a kbset` writes that Set out with every source it names inlined
    /// after it, each checked against the address its `slot` record carries — and a
    /// bare `.kir` names nothing, which is ADR-0338's own reason a `folder` scope
    /// lists no procedure. So a procedure row's menu is the loads and no separator,
    /// and this is the condition the card is laid out from rather than an item
    /// drawn and then refused.
    pub sends: bool,
}

/// One item of a row's menu, as what a press on it is about.
///
/// Deliberately not an `Operation`: which Set the menu is on is the *bay's*
/// reading and this is the geometry's answer, so the two are put together in
/// [`LibraryBay::menu_ask`] where both are in hand — the arrangement
/// [`ArrangementPill::ask`] is in one bay along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowItem {
    /// `Load to Slot A` … `Load to Slot D`, and the deck is the item.
    Load(u8),
    /// `Save as a kbset`, under the separator.
    Save,
}

/// What a press on a row's menu asks for.
///
/// [`Aim`]'s shape one control along, and the same division: every arm is
/// either this console's own state moving or one named operation, and never an
/// act performed here ([P-0090]).
///
/// [P-0090]: ../../../docs/principles/0090-a-surface-offers-it-never-decides.md
#[derive(Debug, Clone, PartialEq)]
pub enum Picked {
    /// Put the menu down on this row — a secondary press on a row of the listing,
    /// with no menu already down.
    Open(usize),
    /// Take it away — a press on the card's own ground, on the separator, or
    /// anywhere else on the console while it is down. The press is spent on the
    /// dismissal, which is [`crate::input::claim`]'s rule 2 said in the control.
    Shut,
    /// A load, named — `Operation::LoadSet { deck, set }`, with the deck off the
    /// item and the Set off the row the menu was opened on. It is the `load`
    /// button's own payload reached without either mark.
    Load(Operation),
    /// The send, named — `Operation::TransferSet` carrying `SetTransfer::Send`,
    /// with the id off the row the menu was opened on. No destination, which is
    /// that operation's own shape: sending is a read, and where the answer goes is
    /// the surface's
    /// ([ADR-0260](../../../../docs/adr/0260-sending-a-set-is-a-read-and-a-reads-answer-goes-where-the-surface-that-asked-puts-answers.md)).
    Send(Operation),
}

/// A row's menu, laid out: the card, its load items, the separator and the send
/// under it.
///
/// [`Load`]'s shape one control along and for the same reason — one derivation,
/// so that what [`row_menu_into`] paints and what a press lands on are the same
/// rectangles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowMenu {
    /// The card itself, held inside the viewport.
    pub card: Rect,
    /// How many load items there are, which is [`Menued::decks`] — never more than
    /// [`DECKS`], because that is as many letters as there are.
    ///
    /// Zero is a state: a console the mixer is drawing no strip for still has a
    /// menu, because the send under the separator does not name a deck. That is
    /// where this parts company with the deck pulldown, whose card would be empty
    /// and which refuses to open at all.
    pub loads: usize,
    /// The separator's band — [`size::ROW_MENU_RULE_H`] of air, hairline and air,
    /// between the loads and the send. It takes no press: a press inside it is the
    /// dismissal, which is what *the separator is nothing* means as a rectangle.
    ///
    /// `None` where there is no send under it, which is a procedure row: a rule
    /// with nothing on the far side of it separates something from nothing
    /// (ADR-0338, [`Menued::sends`]).
    pub rule: Option<Rect>,
    /// The `Save as a kbset` item, under the band — `None` on a row that cannot be
    /// sent, which is [`Menued::sends`].
    pub save: Option<Rect>,
}

impl RowMenu {
    /// Where one load item is, from the top of the card — the decks in
    /// [`DECK_LETTERS`] order, stacked with no gap between them, which is
    /// [`Load::row`]'s own reading.
    ///
    /// Panics on an item this menu has not got, which is that method's rule: a
    /// caller has invented a deck.
    pub fn load(&self, index: usize) -> Rect {
        assert!(
            index < self.loads,
            "load {index} of a menu of {}",
            self.loads
        );
        Rect::from_min_size(
            Pos2::new(
                self.card.min.x + size::LIB_LIST_PAD,
                self.card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32,
            ),
            egui::vec2(
                self.card.width() - size::LIB_LIST_PAD * 2.0,
                size::LIB_ROW_H,
            ),
        )
    }

    /// Which item `p` is on, or `None` for a point on the card's padding, on the
    /// separator, or off the card altogether.
    pub fn picked(&self, p: karakuri_layout::Point) -> Option<RowItem> {
        let at = Pos2::new(p.x, p.y);
        if self.save.is_some_and(|save| save.contains(at)) {
            return Some(RowItem::Save);
        }
        (0..self.loads)
            .find(|index| self.load(*index).contains(at))
            .map(|index| RowItem::Load(index as u8))
    }
}

/// The word one load item carries — `Load to Slot A`, and the maintainer's own
/// spelling.
///
/// A function rather than four constants because the letter is
/// [`DECK_LETTERS`]', which is the one derivation of a deck's name this crate
/// has: four written-out strings would be a fifth deck's item nobody added.
///
/// Panics on a deck there is no letter for, which is [`Target::letter`]'s rule.
pub fn load_item(deck: u8) -> String {
    format!("{MENU_LOAD} {}", DECK_LETTERS[usize::from(deck)])
}

impl LibraryBay {
    /// The `index`th row's rectangle, with [`scroll`](Self::scroll) already taken
    /// off — so a row above the list has a negative-going top and one below it a
    /// top past `list.max.y`.
    ///
    /// Derived rather than stored for [`TransportRow::dot`]'s reason: the rows are
    /// a stride and a count, and a `Vec` of them would be an allocation a frame
    /// does not need.
    ///
    /// Every index in the listing is an answer, where this used to be a caller's
    /// error past [`rows`](Self::rows): a scrolled bay has rows off both edges and
    /// [`drawn`](Self::drawn) is what says which of them reach the picture, so the
    /// rectangle has to exist before that question can be asked.
    /// [`InspectorPane::group`] is the same change one bay over (ADR-0312,
    /// ADR-0307).
    pub fn row(&self, index: usize) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                self.list.min.x,
                self.list.min.y - self.scroll + size::LIB_ROW_H * index as f32 + self.pushed(index),
            ),
            egui::vec2(self.list.width(), size::LIB_ROW_H),
        )
    }

    /// Which rows reach the picture, as a range into the listing — the ones a
    /// scrolled list has any of on screen, cut edges included.
    ///
    /// It is what [`library_into`] paints and what [`LibraryBay::take`],
    /// [`LibraryBay::land`], [`LibraryBay::starred`] and [`LibraryBay::menu_ask`]
    /// walk, so a control is hit-tested over exactly the rows that were drawn. It
    /// is not [`rows`](Self::rows), which counts the whole ones and is the foot's
    /// number: a star in a row cut by the bottom edge is drawn and is pressable,
    /// and the row it is in is not counted.
    ///
    /// Arithmetic rather than a walk, which is where this parts company with
    /// [`InspectorPane::drawn`]: the rows are a stride, and the one thing that is
    /// not is the reading block, which [`pushed`](Self::pushed) already holds. So
    /// the range is found by asking the two ends rather than by walking every row
    /// of a listing a store answered.
    pub fn drawn(&self) -> std::ops::Range<usize> {
        let touching = |index: usize| {
            let row = self.row(index);
            row.max.y > self.list.min.y && row.min.y < self.list.max.y
        };
        let first = (0..self.total).find(|index| touching(*index));
        match first {
            None => 0..0,
            // **From the first one that touches, and it is a run**: every row
            // after it either touches or is below the list, so the end is the
            // first that does not.
            Some(first) => {
                let end = (first..self.total)
                    .find(|index| !touching(*index))
                    .unwrap_or(self.total);
                first..end
            }
        }
    }

    /// How far a reading pushes the `index`th row down, which is nothing at all for
    /// every row above it and the whole block for every row below.
    ///
    /// The block is between two rows and not over them, which is what makes this a
    /// mode of the list rather than a card drawn on top of one: the rows under the
    /// cursor keep their order and their stride and start lower down. So the block
    /// scrolls with them — it is part of [`library_content_h`]'s content, and the
    /// rows it pushes past the bottom are one notch of the wheel away rather than
    /// out of reach (ADR-0312). The foot's `n of m` says how many are whole, in the
    /// words it says it in for a library taller than its list.
    fn pushed(&self, index: usize) -> f32 {
        match self.reading {
            Some(block) if index >= block.under => {
                size::READING_MARGIN_TOP + block.well.height() + size::READING_MARGIN_BOTTOM
            }
            _ => 0.0,
        }
    }

    /// The `index`th row's star, at the left of the row inside `.lib-row`'s own
    /// padding — [`STAR_SIZE`] square, centred across the row's height.
    ///
    /// Derived rather than stored for [`LibraryBay::row`]'s reason, and off that
    /// method rather than off the list, so a star follows the row a reading pushed
    /// down exactly as the name beside it does.
    ///
    /// One derivation for the paint and the press, which is [`LibraryBay::load`]'s
    /// rule two capsules along: `library_into` paints from this and
    /// [`LibraryBay::starred`] hit-tests it, so the mark a press lands on is the
    /// mark that is drawn.
    pub fn star(&self, index: usize) -> Rect {
        let row = self.row(index);
        Rect::from_min_size(
            Pos2::new(
                row.min.x + size::LIB_ROW_PAD_X,
                row.center().y - STAR_SIZE * 0.5,
            ),
            egui::vec2(STAR_SIZE, STAR_SIZE),
        )
    }

    /// Where the name in the `index`th row starts, which is one star and one
    /// [`STAR_GAP`] in from where it used to.
    ///
    /// It is derived here rather than at the paint so that the mark and the word
    /// are placed by one arithmetic — `.lib-row` is a flex row, and a name laid out
    /// from the row's padding while the star was laid out from the same padding
    /// would draw the two on top of each other.
    fn named(&self, index: usize) -> f32 {
        self.star(index).max.x + STAR_GAP
    }

    /// What a press at `p` on a row's star asks for, or `None` where there is no
    /// star under it.
    ///
    /// # It names the state, and the state is the one the row is not in
    ///
    /// `Operation::SetFavourite { id, favourite }` is not a toggle (ADR-0299) — *"a
    /// map with a button per direction, a model that says which one it wants and a
    /// key all have to be able to say star this and mean it"* — so the control is
    /// what reads the row's present state and asks for the other one. That is
    /// [`TransitionRow::shape`]'s division three bays along: the press names where
    /// it arrived, and the arithmetic that got it there is the surface's
    /// ([P-0090]).
    ///
    /// [P-0090]: ../../../docs/principles/0090-a-surface-offers-it-never-decides.md
    ///
    /// # The listing and the marks both go in with the point
    ///
    /// [`LibraryBay::take`]'s arrangement one control along and for its reason: the
    /// operand is a name this crate reads no store for (ADR-0156), so the rows the
    /// host handed in are what a row index means, and which of them are starred is
    /// the host's answer too — see [`View::starred`].
    ///
    /// A listing shorter than the rows drawn asks nothing, which is `take`'s
    /// refusal rather than a clamp: a star answered bare would name a Set nobody
    /// can see. A procedure row has no star and answers nothing, which is
    /// [`Rows::set`]'s whole job: a star is refused on an id `<store>/sets/` does
    /// not hold — `StoreError::NoSet`, ADR-0299 — so a procedure row draws the
    /// column with nothing in it and a press there is a press on the row.
    pub fn starred(
        &self,
        rows: Rows<'_>,
        marks: &std::collections::BTreeSet<String>,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let p = Pos2::new(p.x, p.y);
        // **The star's own box, inside the list** — `at_row`'s bound stated on
        // the smaller rectangle: a star belongs to a row, and a row off the
        // top of the list has one.
        let row = self
            .drawn()
            .find(|index| self.list.contains(p) && self.star(*index).contains(p))?;
        let id = rows.set(row)?;
        Some(Operation::SetFavourite {
            id: id.to_owned(),
            favourite: !marks.contains(id),
        })
    }

    /// Which row `p` is on, or `None` for a point on the list's own ground, on the
    /// reading between two rows, or outside the list altogether.
    ///
    /// # It is bounded by the list and not only by the row
    ///
    /// A scrolled bay has rows off both edges and [`LibraryBay::row`] answers for
    /// every one of them, so a rectangle under the filter fields or over the foot
    /// is a rectangle a press could land in while the row it belongs to is not on
    /// screen. That is [`InspectorPane::grip`]'s *refuses a press outside the body*
    /// one bay over, and it is the one thing here that would fail silently: the bay
    /// would claim a press on a row nobody can see, under controls that are drawn
    /// there (ADR-0312, ADR-0307).
    ///
    /// [`drawn`](Self::drawn) and not [`rows`](Self::rows), so a press on the
    /// visible half of a cut row reaches it — the row is drawn, and a control
    /// claims what it acts on.
    fn at_row(&self, p: Pos2) -> Option<usize> {
        if !self.list.contains(p) {
            return None;
        }
        self.drawn().find(|index| self.row(*index).contains(p))
    }

    /// What the foot reads: `n of m`, the mock's own `5 of 27`.
    pub fn count(&self) -> String {
        format!("{} of {}", self.rows, self.total)
    }

    /// A capsule `width` wide at the far end of the foot, which is where the
    /// pulldown goes and what everything else in the row is measured back from.
    ///
    /// `.lib-foot` is a flex row of the count, a `.sep { flex: 1 }` and the three
    /// capsules, so the count is one [`size::LIB_FOOT_PAD_X`] in from the left and
    /// the last item is one in from the right with the whole of the leftover
    /// between them. Nothing else in the row has a width, so the spacer's share is
    /// the only arithmetic and it is a subtraction.
    ///
    /// Taken as an argument rather than derived, because a capsule is as wide as
    /// the words in it and this derivation asks `egui` for nothing — [`library`]'s
    /// own rule. The caller measures the galley it is about to paint and hands the
    /// number in, so the box the capsule is drawn in and the box a test asks about
    /// are one statement.
    pub fn pill(&self, width: f32) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                self.foot.max.x - size::LIB_FOOT_PAD_X - width,
                self.foot.center().y - size::PILL_H * 0.5,
            ),
            egui::vec2(width, size::PILL_H),
        )
    }

    /// The foot's load control, measured and laid out: the `load` button, the `→`
    /// label between them, the pulldown and how many decks its list holds.
    ///
    /// One derivation for the paint and for the press, which is
    /// [`ArrangementPill`]'s arrangement one bay along: each capsule is as wide as
    /// what is in it, and two measurements would be a control drawn in one box and
    /// pressed in another. [`library_into`] paints from this,
    /// [`crate::input::claim`] hit-tests it and `tests/library.rs` asks it where
    /// the capsules are.
    ///
    /// Laid out from the right, because the pulldown is the far end of the row.
    /// `.lib-foot`'s `gap: 8px` ([`size::LIB_FOOT_GAP`]) is between every pair of
    /// children, so the label and the button are stepped back from the pulldown by
    /// it and [`LibraryBay::params_chip`] is stepped back from the button by it
    /// again. The pulldown is as wide as a one-letter deck name and the button is
    /// as wide as `load`, so a row measured forwards from the count would move both
    /// capsules whenever the letter did.
    ///
    /// Why it takes the context: a word's width is `egui`'s to answer and nobody
    /// else's, which is [`pill_width`]'s reason and [`mixer`]'s. Before the first
    /// pass there are no fonts, and a zero-width word makes a capsule of the
    /// padding and the mark — which is what a console that has drawn nothing has.
    pub fn load(&self, ctx: &egui::Context, at: Target) -> Load {
        let run = |text: &str| {
            ctx.fonts_mut(|f| {
                f.layout_no_wrap(
                    text.to_owned(),
                    FontId::new(size::BASE, FontFamily::Proportional),
                    Color32::PLACEHOLDER,
                )
                .size()
            })
        };
        let (word, mark) = (run(LOAD_PILL), run(at.letter()));
        let mid = self.foot.center().y;
        // **The pulldown**, and the gap inside it between the letter and the
        // chevron is the one gap the mock states inside a capsule — the same
        // `.sink` gap `arr · night ▾` puts between its words and its mark.
        let deck = self.pill(size::PILL_PAD_X * 2.0 + mark.x + size::SINK_GAP + CHEVRON_W);
        let letter = Rect::from_min_size(
            Pos2::new(deck.min.x + size::PILL_PAD_X, mid - mark.y * 0.5),
            mark,
        );
        let chevron = Rect::from_center_size(
            Pos2::new(deck.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5, mid),
            egui::vec2(CHEVRON_W, CHEVRON_H),
        );
        // **The label, on the foot's own ground and on neither capsule.**
        let arrow = Rect::from_center_size(
            Pos2::new(deck.min.x - size::LIB_FOOT_GAP - LOAD_ARROW * 0.5, mid),
            egui::vec2(LOAD_ARROW, LOAD_ARROW),
        );
        let button = Rect::from_min_size(
            Pos2::new(
                arrow.min.x - size::LIB_FOOT_GAP - (word.x + size::PILL_PAD_X * 2.0),
                mid - size::PILL_H * 0.5,
            ),
            egui::vec2(word.x + size::PILL_PAD_X * 2.0, size::PILL_H),
        );
        Load {
            button,
            text: Rect::from_min_size(
                Pos2::new(button.min.x + size::PILL_PAD_X, mid - word.y * 0.5),
                word,
            ),
            arrow,
            deck,
            letter,
            chevron,
            // **Zero while it is shut**, which is what stops [`Load::row`]
            // handing out a rectangle for a list nobody opened.
            rows: match at.open {
                true => at.decks,
                false => 0,
            },
        }
    }

    /// The foot's `params` chip, laid out: the capsule between the count and the
    /// `load` button.
    ///
    /// `.lib-foot` is a flex row of the count, a `.sep { flex: 1 }` and the
    /// capsules one [`size::LIB_FOOT_GAP`] apart, so this is measured back from
    /// where [`LibraryBay::load`] put the button rather than forward from the
    /// count: the capsules to its right are as wide as the words and the letter in
    /// them, and a chip placed from the left would move whenever any of them did.
    ///
    /// One derivation for the paint and the press, which is [`LibraryBay::load`]'s
    /// own rule one capsule along — [`library_into`] paints this and
    /// [`LibraryBay::read`] hit-tests it, so the capsule a press lands on is the
    /// capsule the word is in.
    pub fn params_chip(&self, ctx: &egui::Context, at: Target) -> Rect {
        let load = self.load(ctx, at).button;
        let width = pill_width(ctx, PARAMS_PILL);
        Rect::from_min_size(
            Pos2::new(load.min.x - size::LIB_FOOT_GAP - width, load.min.y),
            egui::vec2(width, size::PILL_H),
        )
    }

    /// What a press at `p` on the `params` chip asks for, or `None` where there is
    /// no chip under it.
    ///
    /// # The operand is the cursor, which is the load button's operand
    ///
    /// `console.html`'s note: *"Its operand is the cursor, which is the same
    /// operand the pill beside it already uses — so the route costs one chip in the
    /// foot and nothing else"*. So `set` is the Set under the cursor, handed in the
    /// way every other reading of the listing is (ADR-0156), and what comes back
    /// names it.
    ///
    /// # The two answers, and why closing is not an operation
    ///
    /// See [`Read`]. Whether the press opens or closes is read off
    /// [`LibraryBay::reading`] — the block this bay is *drawing* — and not off
    /// anything this method is told, so the chip cannot answer *shut* for a reading
    /// nobody can see.
    ///
    /// A press with no row under the cursor asks nothing, which is a library that
    /// lists nothing: there is no Set to read and the chip is still drawn, because
    /// the foot is what the count is in. It is [`Mixer::grab`]'s answer for a press
    /// on a fader's track — a control claims what it acts on, and claiming a press
    /// to throw it away would put the rule and the act out of step.
    pub fn read(
        &self,
        ctx: &egui::Context,
        at: Target,
        set: Option<&str>,
        p: karakuri_layout::Point,
    ) -> Option<Read> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        if !self.params_chip(ctx, at).contains(Pos2::new(p.x, p.y)) {
            return None;
        }
        match self.reading {
            Some(_) => Some(Read::Shut),
            None => Some(Read::Open(Operation::ReadSet {
                id: set?.to_owned(),
            })),
        }
    }

    /// What a press at `p` on the foot's load control asks for, or `None`
    /// where the press was on nothing this control owns.
    ///
    /// The same derivation [`crate::input::claim`] hit-tests, asked a second
    /// time rather than copied — [`ArrangementPill::ask`]'s arrangement, and
    /// the reason is the same: the control that claims a press and the control
    /// that acts on it cannot come apart.
    ///
    /// # The three answers a press can give, and which mark each one moves
    ///
    /// - The pulldown puts its list down, or takes it away again. Neither
    ///   is an operation and neither moves a deck.
    /// - A row of that list is [`Aim::Deck`]: the target moves to the deck
    ///   that was picked and nothing is asked for, exactly as a press on a
    ///   row of the listing above asks for nothing (`LibraryBay::take`). The
    ///   deck selection does not move — that is the whole of what the second
    ///   mark is for, and `View::select` is not called from here.
    /// - The button is `Operation::LoadSet { deck, set }`, with the deck
    ///   off [`Target::deck`] and the Set off the cursor — or
    ///   `Operation::LoadProcedure` where the row under the cursor is a
    ///   procedure, which is one file written over what the deck is playing
    ///   rather than every layer replaced (ADR-0338). With no row under the
    ///   cursor it is [`Aim::NoSet`] and nothing is emitted: a load with one
    ///   operand missing is not a load, and answering `None` would leave the
    ///   press claimed and unaccounted for.
    ///
    /// While the list is down, every press is the dismissal, which is
    /// [`ArrangementPill::ask`]'s rule and `input::claim`'s rule 2: the card
    /// is drawn over this bay's own list, so a press on the rows underneath it
    /// belongs to the card and not to what it is covering. So the button
    /// answers [`Aim::Shut`] while the list is down rather than loading
    /// through it.
    ///
    /// `None` before the first pass, which is [`LibraryBay::read`]'s guard
    /// and [`mixer`]'s: there are no fonts until `egui` has run one, so there
    /// is no capsule width to measure and nothing has been drawn to press.
    pub fn aim(
        &self,
        ctx: &egui::Context,
        viewport: Rect,
        at: Target,
        rows: Rows<'_>,
        cursor: usize,
        p: karakuri_layout::Point,
    ) -> Option<Aim> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let load = self.load(ctx, at);
        if load.hit_deck(p) {
            return Some(match at.open {
                true => Aim::Shut,
                false => Aim::Open,
            });
        }
        if at.open {
            return Some(match load.picked(viewport, p) {
                Some(deck) => Aim::Deck(deck),
                None => Aim::Shut,
            });
        }
        if !load.hit_button(p) {
            return None;
        }
        // **Which of the two loads it is is the row's, and the operand is the
        // same word either way.** A Set row names every layer of what the deck
        // will play; a procedure row names one file written over what it is
        // playing already (ADR-0338). The button, the row menu and the drag all
        // arrive at this pair, which is why the division is here rather than
        // three times over.
        Some(match (rows.name(cursor), rows.procedure(cursor)) {
            (Some(name), true) => Aim::Load(Operation::LoadProcedure {
                deck: at.deck,
                procedure: name.to_owned(),
            }),
            (Some(id), false) => Aim::Load(Operation::LoadSet {
                deck: at.deck,
                set: id.to_owned(),
            }),
            (None, _) => Aim::NoSet,
        })
    }

    /// A row's menu, laid out, or `None` where none is down.
    ///
    /// # It hangs down off the row, where the deck pulldown's card hangs up
    ///
    /// Both hang into the room there is, which is [`Load::list`]'s own rule read
    /// from the other end: that card stands on a capsule in the bay's foot and so
    /// has only this bay's list above it, and this one stands on a row of that list
    /// and has the rest of the list below it. It is inset from the row's left edge
    /// by [`size::ROW_MENU_INSET`] so that it hangs under the name that was pressed
    /// rather than under the star beside it, and it is held inside the viewport, so
    /// a press on the last row of a bay at the bottom of the window draws the card
    /// over the bay rather than off the screen.
    ///
    /// The rows are not counted against the room, which is [`Load::list`]'s clause
    /// and the same argument: this card lists at most [`DECKS`] loads and one send,
    /// and a window too short for five rows of type has no transport row in it
    /// either.
    ///
    /// Why it takes the context: the items are as wide as the words in them, which
    /// is `egui`'s to answer and nobody else's — [`LibraryBay::load`]'s reason one
    /// control along. `None` before the first pass for that reason too.
    pub fn menu(&self, ctx: &egui::Context, viewport: Rect, at: Menued) -> Option<RowMenu> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let row = self.row(at.row?);
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
        let loads = at.decks.min(DECKS);
        // **The send is measured only where it is drawn**, which is
        // [`Menued::sends`]: a card as wide as `Save as a kbset` with no such
        // item in it would be a menu whose width said what it holds and was
        // wrong.
        let widest = (0..loads)
            .map(|deck| width(&load_item(deck as u8)))
            .chain(at.sends.then(|| width(MENU_SAVE)))
            .fold(size::ROW_MENU_MIN_W, f32::max);
        let sent = match at.sends {
            true => size::LIB_ROW_H + size::ROW_MENU_RULE_H,
            false => 0.0,
        };
        let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * loads as f32 + sent;
        let card = held_inside(
            &viewport,
            row.min.x + size::ROW_MENU_INSET,
            row.max.y,
            widest + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0,
            height,
        );
        let band = card.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * loads as f32;
        Some(RowMenu {
            card,
            loads,
            rule: at.sends.then(|| {
                Rect::from_min_size(
                    Pos2::new(card.min.x + size::LIB_LIST_PAD, band),
                    egui::vec2(
                        card.width() - size::LIB_LIST_PAD * 2.0,
                        size::ROW_MENU_RULE_H,
                    ),
                )
            }),
            save: at.sends.then(|| {
                Rect::from_min_size(
                    Pos2::new(
                        card.min.x + size::LIB_LIST_PAD,
                        band + size::ROW_MENU_RULE_H,
                    ),
                    egui::vec2(card.width() - size::LIB_LIST_PAD * 2.0, size::LIB_ROW_H),
                )
            }),
        })
    }

    /// What a press at `p` asks of a row's menu, or `None` where the press
    /// was on nothing this control owns.
    ///
    /// # Two questions, and which one it is depends on whether a menu is down
    ///
    /// - With none down this is *the secondary press*: which row it named,
    ///   and nothing else. A press on the list's own ground below the last row
    ///   answers `None`, exactly as [`LibraryBay::take`] does, and so does a
    ///   press on a row with no Set behind it — a `history` row is a version,
    ///   and every item this menu carries names a Set.
    /// - With one down every press is the card's, which is
    ///   [`crate::input::claim`]'s rule 2 and [`ArrangementPill::ask`]'s rule:
    ///   on an item it picks, on the separator or anywhere else it dismisses.
    ///   So this never answers `None` while a menu is down.
    ///
    /// Both operands are in hand here, which is why the arms carry whole
    /// operations: the deck is the item and the Set is `sets[at.row]`, the row
    /// the menu was opened on rather than the cursor's — a menu opened on the
    /// fourth row and picked at `Load to Slot C` loads the fourth Set onto
    /// deck C whatever the cursor and the pulldown are doing.
    ///
    /// The row is re-checked against the listing rather than trusted, for
    /// [`LibraryBay::take`]'s reason: a listing that shrank between the press
    /// that opened the menu and the press that picked from it would otherwise
    /// name a Set nobody can see. A menu over a row that has gone dismisses.
    pub fn menu_ask(
        &self,
        ctx: &egui::Context,
        viewport: Rect,
        at: Menued,
        rows: Rows<'_>,
        p: karakuri_layout::Point,
    ) -> Option<Picked> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let Some(row) = at.row else {
            let row = self.at_row(Pos2::new(p.x, p.y))?;
            rows.name(row)?;
            return Some(Picked::Open(row));
        };
        let Some(menu) = self.menu(ctx, viewport, at) else {
            return Some(Picked::Shut);
        };
        let Some(name) = rows.name(row) else {
            return Some(Picked::Shut);
        };
        Some(match menu.picked(p) {
            // **The item names the deck and the row names which load it
            // is**, which is [`LibraryBay::aim`]'s division arriving by the
            // third route: four items on a procedure row load that one file
            // over that deck's layer, and four on a Set row load every layer
            // (ADR-0338).
            Some(RowItem::Load(deck)) => Picked::Load(match rows.procedure(row) {
                true => Operation::LoadProcedure {
                    deck,
                    procedure: name.to_owned(),
                },
                false => Operation::LoadSet {
                    deck,
                    set: name.to_owned(),
                },
            }),
            // **The send is a Set's and a procedure row has none.** What is
            // written out is that Set with every source it names inlined after
            // it, checked against the address each `slot` record carries — and
            // a loose `.kir` names nothing, which is ADR-0338's own reason a
            // `folder` lists no procedure. So the item is not drawn on such a
            // row (see [`Menued::sends`]) and a press where it would have been
            // dismisses.
            Some(RowItem::Save) => match rows.set(row) {
                Some(id) => Picked::Send(Operation::TransferSet {
                    transfer: karakuri_operation::SetTransfer::Send { id: id.to_owned() },
                }),
                None => Picked::Shut,
            },
            None => Picked::Shut,
        })
    }

    /// What a press at `p` on the list takes in hand, or `None` where there is no
    /// drawn row under it.
    ///
    /// # It answers a payload where every other control here answers an operation
    ///
    /// A press on a row asks for nothing yet. `console.html`'s *How a Set reaches a
    /// deck* has the gesture as a drag — *"Dragging a row onto a strip is a second
    /// route to the same command, and never the first … it names both operands in
    /// the one gesture"* — and half a gesture names one operand. So what comes back
    /// is the Set, on its way to [`crate::panel::Panel::carry`], and the operation
    /// is built at the drop where the second operand is
    /// ([`crate::panel::Released::Dropped`]).
    ///
    /// A press that is never dragged anywhere asks for nothing either, and that is
    /// the same sentence rather than a second rule: the row is picked up, carried
    /// nowhere, and let go over nothing.
    ///
    /// # The listing goes in with the point
    ///
    /// [`LibraryBay::read`]'s arrangement one row up, and for the reason that
    /// method states: the operand is a name this crate reads no store for
    /// (ADR-0156), so the rows the host handed in are what a row index means.
    /// Handing them in is also what makes this refuse rather than clamp — a listing
    /// shorter than the rows drawn takes nothing in hand, where an index answered
    /// bare would name a Set nobody can see.
    ///
    /// The rows are walked rather than divided. A row's stride is
    /// [`size::LIB_ROW_H`] and a reading pushes the rows under it down by a whole
    /// block, so *which row is at `y`* is not one division — [`LibraryBay::row`]
    /// already holds that arithmetic, and asking it per row is what keeps the row a
    /// press lands on the row the paint drew. Never more than [`LibraryBay::rows`]
    /// of them, so a press below the last row is on the list's own ground and
    /// belongs to nobody.
    pub fn take(&self, rows: Rows<'_>, p: karakuri_layout::Point) -> Option<Taken> {
        self.at_row(Pos2::new(p.x, p.y)).and_then(|row| {
            Some(Taken {
                row,
                set: rows.name(row)?.to_owned(),
                // **What is in hand says which load a drop names**, which is
                // the drag's half of [`LibraryBay::aim`]'s division: the
                // gesture names both operands and the second arrives at the
                // release, so the first has to carry what kind of row it was
                // (ADR-0338).
                procedure: rows.procedure(row),
            })
        })
    }

    /// What a press at `p` on a row of the `history` listing asks for, or `None`
    /// where there is no drawn row under it.
    ///
    /// # It is a load, and it is not [`Operation::LoadSet`]
    ///
    /// A row here is a version of one node rather than a Set, so what a press on it
    /// asks for is `Operation::RestoreProcedure` carrying that version — *put a
    /// node's previous version back*, which is the row
    /// `docs/manual/operations.html` names for landing and says so at the walk
    /// beside it: *"landing is not this row"*. Nothing else in this bay changes:
    /// the version is written over that node's working copy and the watcher builds
    /// it, so the load is the path an edit already takes (ADR-0228).
    ///
    /// # Both operands are marks this console keeps
    ///
    /// The deck is [`Target::deck`] — the pulldown in the foot, which is what
    /// narrowed the listing to a Set in the first place, so the rows a hand is
    /// looking at and the deck a press lands on cannot come apart. The version is
    /// the row, by the word the host handed in: this crate reads no store
    /// (ADR-0156), and the name is matched back against the listing that produced
    /// it, which is `SetTransfer::Take`'s arrangement and the rule that keeps a
    /// path out of a payload.
    ///
    /// # It takes the versions and [`LibraryBay::take`] takes the Sets
    ///
    /// The two are one press on one rectangle and they are told apart by which
    /// listing is handed in: [`View::sets`] is empty under `history` and
    /// [`View::versions`] is empty everywhere else, so exactly one of them can
    /// answer and neither has to be told what the scope is. A carry of a version
    /// would be a Set named by a word no store holds, and a landing on a Set would
    /// be a node named by a word that addresses none.
    ///
    /// The rows are walked rather than divided, which is `take`'s rule and for its
    /// reason: a reading pushes the rows under it down by a whole block, so
    /// [`LibraryBay::row`] is the arithmetic and asking it per row is what keeps
    /// the row a press lands on the row the paint drew.
    pub fn land(
        &self,
        versions: &[String],
        at: Target,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let row = self.at_row(Pos2::new(p.x, p.y))?;
        Some(Operation::RestoreProcedure {
            deck: at.deck,
            revision: karakuri_operation::Revision::Picked(versions.get(row)?.clone()),
        })
    }

    /// Every scope chip and its box, left to right in the order the row was handed
    /// them — the same walk [`scopes_into`] paints and [`LibraryBay::chip`]
    /// hit-tests, so the capsule a press lands on is the capsule the wash is drawn
    /// in.
    ///
    /// A chip is as wide as the word in it, so this is the one thing about this bay
    /// that has to ask `egui` — [`library`] asks it for nothing, and that sentence
    /// is still true of every *rectangle* the bay derives. The widths are measured
    /// here rather than stored for [`LibraryBay::row`]'s reason: a `Vec` of four
    /// rectangles would be an allocation on a path that is asked once per pointer
    /// event.
    ///
    /// The boxes are not clipped and the paint is. `.scopes` is one row and this
    /// console draws one row of it, so at the mock's own width the fourth chip
    /// starts inside the bay and finishes outside it. What is yielded here is the
    /// capsule's whole rectangle, because that is what the paint wants;
    /// [`LibraryBay::chip`] is where a press is held to the part of it that is
    /// drawn.
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

    /// One filter field's box, or `None` for a bay with no filter row.
    ///
    /// `.field` carries `flex: 1; min-width: 0` and nothing else is in the row, so
    /// it takes the whole of `.lib-filters`'s content box — which is a division
    /// rather than a measurement, and is why this bay still asks `egui` for
    /// nothing. The chips above and below it are the exception and stay the
    /// exception: a chip is as wide as the word in it and a field is not.
    ///
    /// It shared the row with `layer…` until 2026-09-10, each taking half of what
    /// was left after one [`size::LIB_FILTERS_GAP`]. ADR-0338 retired that field
    /// for the six chips in [`LibraryBay::kinds`], which are their own row under
    /// this one — six toggles and a `flex: 1` field on one row in a bay this narrow
    /// would leave the chips a few pixels each, and a chip nobody can hit is not a
    /// control (`style.css`, `.lib-kinds`).
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

    /// One kind chip and its box, left to right in [`KindChip::ALL`]'s order — the
    /// same walk `library_into` paints and [`LibraryBay::kind`] hit-tests, so the
    /// capsule a press lands on is the capsule the mint is drawn in.
    ///
    /// A chip is as wide as the word in it, which is [`LibraryBay::chips`]'
    /// sentence one row up and the second place this bay has to ask `egui`
    /// anything. `.kind` is `font-size: 9px; padding: 0 6px` with a hairline
    /// border, so the box is the word at [`size::KIND_SIZE`] inside
    /// [`size::KIND_PAD_X`] either side.
    ///
    /// Empty for a bay with no kind row, which is a bay with no filter row — see
    /// [`LibraryBay::kinds`], where the one condition is written.
    pub fn kind_chips<'a>(
        &self,
        ctx: &'a egui::Context,
    ) -> impl Iterator<Item = (KindChip, Rect)> + 'a {
        let row = self.kinds;
        let mut x = row.map_or(0.0, |row| row.min.x + size::LIB_KINDS_PAD_X);
        let top = row.map_or(0.0, |row| row.min.y + size::LIB_KINDS_PAD_Y);
        let drawn = row.map_or(0, |_| KindChip::ALL.len());
        KindChip::ALL.into_iter().take(drawn).map(move |chip| {
            let width = ctx.fonts_mut(|f| {
                f.layout_no_wrap(
                    chip.word().to_owned(),
                    FontId::new(size::KIND_SIZE, FontFamily::Proportional),
                    Color32::PLACEHOLDER,
                )
                .size()
                .x
            }) + size::KIND_PAD_X * 2.0;
            let box_ = Rect::from_min_size(Pos2::new(x, top), egui::vec2(width, size::KIND_H));
            x += width + size::LIB_KINDS_GAP;
            (chip, box_)
        })
    }

    /// What a press at `p` on the kind row asks for, or `None` where there is no
    /// chip under it.
    ///
    /// # The chip flips and the operation names all six
    ///
    /// `Operation::FilterLibrary { kinds }` carries the whole row, never one chip:
    /// *"six presses that each say this one changed are six statements two hands
    /// can disagree about, and one that says these are the kinds showing is a
    /// destination"* (ADR-0338, which is `Operation::Publish`'s rule on a different
    /// list). So the flip is this console's arithmetic ([P-0090]) and what leaves
    /// is where it arrived — [`KindChip::flipped`].
    ///
    /// A press on a chip that is on turns it off, and with the last one off the row
    /// is [`LibraryKinds::EVERYTHING`] again: every state this row can be in is one
    /// a press can leave, which is what a control with more than two positions
    /// owes.
    ///
    /// [P-0090]: ../../../docs/principles/0090-a-surface-offers-it-never-decides.md
    ///
    /// # A chip is pressed only where it is drawn
    ///
    /// [`LibraryBay::chip`]'s rule one row up and for its reason: the row clips, so
    /// a chip that runs past the bay's own edge is pressable only where it is
    /// painted, and the point is held to the row before any chip is asked about.
    /// `None` before the first pass for that method's reason too — there are no
    /// fonts to measure a word with.
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

    /// The `index`th row's badges, right to left from the row's own padding — one
    /// box per word, in the order [`Rows::badges`] hands them and laid out so the
    /// last word ends where the row's padding starts.
    ///
    /// A badge is as wide as the word in it, which is [`kind_chips`]' sentence one
    /// row up: `.badge` is `font-size: 8px; padding: 0 4px`, so the box is the word
    /// at [`size::BADGE_SIZE`] inside [`size::BADGE_PAD_X`] either side, and
    /// `.badges`' `gap: 3px` is [`size::BADGE_GAP`].
    ///
    /// One derivation for the paint and the hover, which is [`LibraryBay::load`]'s
    /// rule: `library_into` paints from this and `hover`'s probe asks it whether
    /// the pointer is on one, so the readout a tip explains is the readout that is
    /// drawn. Nothing presses it — what narrows the list by kind is the row of
    /// chips above.
    ///
    /// Laid out from the right, because the mock puts the badges at the end of the
    /// row after the name: a row measured forwards from the name would move every
    /// badge whenever a name got longer.
    pub fn badges<'a>(
        &self,
        ctx: &'a egui::Context,
        index: usize,
        words: &'a [&'static str],
    ) -> impl Iterator<Item = (&'static str, Rect)> + 'a {
        let row = self.row(index);
        let widths: Vec<f32> = words
            .iter()
            .map(|word| {
                ctx.fonts_mut(|f| {
                    f.layout_no_wrap(
                        (*word).to_owned(),
                        FontId::new(size::BADGE_SIZE, FontFamily::Proportional),
                        Color32::PLACEHOLDER,
                    )
                    .size()
                    .x
                }) + size::BADGE_PAD_X * 2.0
            })
            .collect();
        let whole: f32 =
            widths.iter().sum::<f32>() + size::BADGE_GAP * widths.len().saturating_sub(1) as f32;
        let mut x = row.max.x - size::LIB_ROW_PAD_X - whole;
        let top = row.center().y - size::BADGE_H * 0.5;
        words
            .iter()
            .zip(widths)
            .map(move |(word, width)| {
                let box_ = Rect::from_min_size(Pos2::new(x, top), egui::vec2(width, size::BADGE_H));
                x += width + size::BADGE_GAP;
                (*word, box_)
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    /// What a press at `p` on the filter row asks the store for, or `None` where
    /// there is no field under it.
    ///
    /// # The field steps, and the operation names where it arrived
    ///
    /// [`TransitionRow::shape`]'s affordance, in a bay where it costs an
    /// explanation rather than a sentence. What [`Operation::ListSets`] carries is
    /// free text and a layer — *"narrowed by what a node is called or by which
    /// layer a Set uses"* — and this console has one letter-taking flow,
    /// [`Menu::Naming`], which ADR-0221 bounds to one path component of a name. A
    /// filter is not a name, so typing into these two would be a second
    /// letter-taking flow, and what that flow *is* is a decision about the console
    /// rather than about this bay ([`Chosen`]'s rule, one control up: the first
    /// control that wants a thing is not where it is decided).
    ///
    /// So the fields step, over two closed lists, and the operation names the
    /// destination — never a step, because there is no step in the vocabulary to
    /// name (P-0090). `layer` steps [`LAYERS`], which is the console's own curation
    /// of an enum with no list in it. `holds` steps [`View::holds`], which is the
    /// host's answer to *what are this store's Sets made of* and arrives across the
    /// same seam as the listing itself — this crate reads no store (ADR-0156). Both
    /// wrap through unset, so every state either field can be in is one the press
    /// can leave.
    ///
    /// A press with nothing to step to is answered all the same, and that is the
    /// state a `holds` field has on a store whose Sets name no node: it asks for
    /// the listing again, which is a real question and the one [`LibraryBay::chip`]
    /// already answers for the chip that is marked.
    ///
    /// # One derivation, asked twice
    ///
    /// [`crate::input::claim`]'s rule 4 asks this and so does the caller that acts
    /// on the press, exactly as they both ask [`LibraryBay::chip`]. The whole field
    /// is the target, its border included, which is [`Outputs::sink`]'s rule about
    /// a padding being what makes a word a hand can find.
    ///
    /// The `9` pixels of padding either side of the row and the
    /// [`size::LIB_FILTERS_GAP`] between the two fields are not targets, and this
    /// answers `None` for them: they are bare card, the way the gaps between the
    /// scope chips are.
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
            // **`layer` goes out unset and the field it came from is gone.**
            // `Operation::ListSets` keeps that field — `list_sets` over MCP and
            // `--list-sets` are where *which Sets hold a node on this layer*
            // lives — and this console stopped asking it on 2026-09-10, because
            // a kind chip asks a different question of a different thing
            // (ADR-0338). The panel's way to *which Sets use this* is `holds`,
            // which asks by node name.
            Field::Holds => Operation::ListSets {
                holds: stepped_holds(holds, at.holds),
                layer: None,
            },
        })
    }
}

/// The Library bay's rows, derived — see [`LibraryBay`] for what is drawn here
/// and for the six things in the mock's bay that are not.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. Unlike [`outputs`], [`transport`] and [`mixer`] this asks `egui` for
/// nothing: every box in the bay is the full width of the list, so no rectangle
/// here is the width of the type in it.
///
/// `None` where there is nothing to draw at all, and `None` where there is no
/// room to draw it: a console with no scopes and no rows has had nothing said
/// to it about any library, which is every test in this crate that does not say
/// otherwise and the whole of `cargo test -p karakuri-console`, and what the
/// bay draws then is the card and its head and nothing else — this is
/// [`mixer`]'s rule, one column along, and [`View::picture`]'s before that.
///
/// A scope with nothing in it is not that, and the difference is the whole of
/// what the chips bought: handed scopes and no rows, the bay draws the row of
/// questions and answers the marked one with `0 of 0`, because a question that
/// has been asked is owed an answer even where the answer is *nothing*.
/// `console.html`: *"An empty tier is a library nobody has filled rather than
/// something gone wrong."*
///
/// `scopes` is the row of chips, in the order they are drawn; `sets` is the
/// listing of whichever of them is marked. Which one that is does not reach
/// here, because no rectangle in this bay depends on it: it is a pointer, and a
/// pointer goes to the paint beside the library cursor — see `library_into` and
/// [`View::scope`].
///
/// `open` is the exception, and it is the one pointer a rectangle in this bay
/// does depend on. A reading is drawn *under a row*, so where every row below
/// it goes and how many of them there is room for both follow the cursor — see
/// [`Opened`] and [`Block`]. `None` is a bay with nothing open, which is every
/// console until a press on the `params` chip.
///
/// `pointed` is a row of furniture rather than a pointer, and only whether
/// there is one reaches the arithmetic: the `.path` row is drawn once a folder
/// has been chosen and not at all before, so everything under it — the fields,
/// the list, and how many rows there is room for — is measured from where it
/// ends. What it *reads* goes to `path_into` and nowhere else, exactly as the
/// marked chip does. `None` is a bay pointed nowhere with nothing over the
/// window, which is every console until a folder is dropped on one (ADR-0275) —
/// see [`Pointed`] and [`View::pointed`].
pub fn library(
    layout: &karakuri_layout::Layout,
    scopes: &[Scope],
    sets: &[String],
    open: Option<Opened<'_>>,
    pointed: Option<Pointed<'_>>,
    scroll: f32,
) -> Option<LibraryBay> {
    // **Nothing said about any library, so there is nothing to draw.** Not the
    // same as a scope that holds nothing — see this function's own doc, and
    // ADR-0177 for the row of zeroes this is still refusing.
    if scopes.is_empty() && sets.is_empty() {
        return None;
    }
    library_box(
        to_egui(layout.rect(layout.find("library")?)),
        !scopes.is_empty(),
        pointed.is_some(),
        sets.len(),
        open.map(|open| (open.at, open.reading.rows())),
        scroll,
    )
}

/// The arithmetic of the bay, away from the layout it reads.
///
/// Term for term from `style.css`:
///
/// - `.lib-foot { padding: 5px 10px; font-size: 10px; border-top: 1px solid
///   var(--c-hair) }` — a [`size::LIB_FOOT_H`] row along the bottom of the
///   bay, its rule the top pixel of it.
/// - `.scopes { display: flex; gap: 4px; padding: 7px 9px }` — a
///   [`size::SCOPES_H`] row under the bay head, its own rule the bottom pixel
///   of it, and drawn only where there are scopes to put in it.
/// - `.path { padding: 4px 10px; font-size: 10px; border-bottom: 1px solid
///   var(--c-hair) }` — a [`size::PATH_H`] row under the scopes, its own rule
///   the bottom pixel of it, and drawn only once a folder has been dropped on
///   the window.
/// - `.lib-list { display: flex; flex-direction: column; padding: 3px }` —
///   what is left between the scope row and the foot, inset by
///   [`size::LIB_LIST_PAD`] on all four sides.
/// - `.lib-row { padding: 3px 7px }` — [`size::LIB_ROW_H`] each, stacked from
///   the top of the list with no gap, because `.lib-list` states none.
///
/// # The foot is at the bottom and the leftover is the list's
///
/// The mock's bay is a flow: its children stack from the top and whatever is
/// left over is under the last of them. Here the leftover goes to the list
/// instead, and the mock says which: the Library is the bay that carries
/// `style="flex:1"` in its column and a `.grip` in its head, which is *"the
/// bay that absorbs its column's height"* — and what absorbs it is the list,
/// since a foot of one line at a fixed type size has nothing in it that gets
/// bigger. A foot left floating under the last row would also put its
/// `border-top` between the list and bare card, which is a rule separating
/// something from nothing.
///
/// `None` where the region cannot hold the foot and one row, which is
/// [`picture_rect`]'s rule stated on a listing. It is room and not content:
/// a scope that lists nothing still wants room for a row, because the bay it is
/// drawn in is the bay the next scope's rows land in and a question drawn over
/// somewhere there is no room to answer it is worse than no question.
fn library_box(
    region: Rect,
    chips: bool,
    pointed: bool,
    total: usize,
    open: Option<(usize, usize)>,
    scroll: f32,
) -> Option<LibraryBay> {
    let foot = Rect::from_min_max(
        Pos2::new(region.min.x, region.max.y - size::LIB_FOOT_H),
        region.max,
    );
    let under_head = region.min.y + size::HEAD_H;
    // **The scope row is the head's business and not the list's**, which is
    // why it is taken off the top before the list is measured: the mock draws
    // it between the bay head's rule and `.lib-list`, and `.lib-list`'s own
    // padding is inside whatever is left.
    let scopes = chips.then(|| {
        Rect::from_min_max(
            Pos2::new(region.min.x, under_head),
            Pos2::new(region.max.x, under_head + size::SCOPES_H),
        )
    });
    // **The path row is under the chips and above the fields**, which is where
    // the mock puts it, and it is nobody's condition: a bay is pointed at a
    // directory or it is not, and that is true whether or not it was handed
    // scopes to draw over it. **Its own condition is that there is a path at
    // all** — until a folder has been dropped the row is not drawn and this
    // bay is a line shorter (ADR-0275), which is why every rectangle under it
    // is measured from where it ends rather than from the scope row.
    let under_scopes = scopes.map_or(under_head, |scopes| scopes.max.y);
    let path = pointed.then(|| {
        Rect::from_min_max(
            Pos2::new(region.min.x, under_scopes),
            Pos2::new(region.max.x, under_scopes + size::PATH_H),
        )
    });
    let under_path = path.map_or(under_scopes, |path| path.max.y);
    // **The filter row is the scope row's condition and not a second one.**
    // Both are the bay's head rather than its body — one says which library is
    // being read and the other narrows what that library answers — so a
    // console that has been told about no library draws neither. The width
    // check is `list`'s below, stated on a row that has a box and a padding in
    // it: a row too narrow for the field would draw a sliver and a rule.
    let filters = (chips && region.width() - size::LIB_FILTERS_PAD_X * 2.0 > 0.0).then(|| {
        Rect::from_min_max(
            Pos2::new(region.min.x, under_path),
            Pos2::new(region.max.x, under_path + size::LIB_FILTERS_H),
        )
    });
    // **The kind row is the filter row's condition and not a third one**, which
    // is the sentence above read once more: both are the bay's head, one asks
    // what a Set is made of and the other what a row *is*, and a bay with room
    // for neither draws neither. **The chips are not measured against the
    // width**, which is the scope row's own answer one band up: a chip is as
    // wide as the word in it, the row clips, and a press is held to the part of
    // a chip that is drawn (ADR-0338, `LibraryBay::kind`).
    let kinds = filters.map(|filters| {
        Rect::from_min_max(
            Pos2::new(region.min.x, filters.max.y),
            Pos2::new(region.max.x, filters.max.y + size::LIB_KINDS_H),
        )
    });
    let top = match (kinds, filters) {
        (Some(kinds), _) => kinds.max.y,
        (None, Some(filters)) => filters.max.y,
        (None, None) => under_path,
    };
    let list = Rect::from_min_max(
        Pos2::new(region.min.x + size::LIB_LIST_PAD, top + size::LIB_LIST_PAD),
        Pos2::new(
            region.max.x - size::LIB_LIST_PAD,
            foot.min.y - size::LIB_LIST_PAD,
        ),
    );
    // **Narrower than its own padding is no list**, which is
    // [`picture_rect`]'s rule stated across the axis. There is no matching
    // check down it: a bay too short for the foot is already a bay too short
    // for a row, and `fits` below is what answers that.
    if list.width() <= 0.0 {
        return None;
    }
    let fits = (list.height() / size::LIB_ROW_H).floor().max(0.0) as usize;
    // **A reading opens under the row it is of, whether or not that row is on
    // screen.** It used to open only under a row this bay was *drawing*,
    // because a reading scrolled past the bottom had nowhere to be; a bay with
    // a position has somewhere, and the block scrolls with the rows because it
    // is between two of them rather than over them. What still holds the two
    // together is [`View::opened`], which answers `None` the moment the row
    // under the cursor stops being the Set the reading is of.
    let block = open
        .filter(|(at, _)| *at < total)
        // **The block is between the cursor's row and the next**, so what is
        // above it is the cursor's row and everything before it.
        .map(|(at, rows)| (at + 1, rows));
    let content = library_content_h(total, block.map(|(_, rows)| rows));
    // **The clamp that is drawn and never stored** — see [`LibraryBay::scroll`]
    // and [P-0082]. Zero-width where the listing is shorter than the list,
    // which is a bay that cannot be scrolled at all.
    //
    // [P-0082]: ../../../docs/principles/0082-looking-never-writes-back.md
    let scroll = scroll.clamp(0.0, (content - list.height()).max(0.0));
    let reading = block.map(|(under, rows)| {
        let top = list.min.y - scroll + size::LIB_ROW_H * under as f32 + size::READING_MARGIN_TOP;
        Block {
            well: Rect::from_min_max(
                Pos2::new(list.min.x + size::READING_MARGIN_X, top),
                Pos2::new(
                    list.max.x - size::READING_MARGIN_X,
                    top + size::LIB_ROW_H * rows as f32,
                ),
            ),
            rows,
            under,
        }
    });
    // **Built once with the count unanswered and then answered off itself**,
    // because how many rows are whole is a question about the rectangles this
    // bay hands out — [`LibraryBay::row`] and [`LibraryBay::drawn`] — and a
    // second arithmetic here would be a second answer to where a row is.
    let bay = LibraryBay {
        scopes,
        path,
        filters,
        kinds,
        list,
        rows: 0,
        total,
        foot,
        reading,
        scroll,
        content,
        bay: region,
    };
    // **Down the column and not across it**: a row is exactly as wide as the
    // list and starts where it starts, so the only edge a row can be cut by is
    // the top one or the bottom one.
    let whole = bay
        .drawn()
        .filter(|index| {
            let row = bay.row(*index);
            row.min.y >= list.min.y && row.max.y <= list.max.y
        })
        .count();
    (fits > 0).then_some(LibraryBay { rows: whole, ..bay })
}

/// How tall a library listing comes to, the reading block included —
/// [`LibraryBay::content`], and what both clamps are taken against.
///
/// One function because the two clamps are one number read twice: `library_box`
/// clamps the position it *draws* against it and [`View::scroll_library_by`]
/// clamps the position it *stores* against it, and a second arithmetic in
/// either would be a bay that could be scrolled to a place it will not draw.
/// [`content_h`] is the same shape one bay over.
///
/// `block` is how many rows the reading under the cursor is, or `None` where
/// none is open — the same run [`LibraryBay::pushed`] adds to every row below
/// it, so the two cannot disagree about what a reading costs.
fn library_content_h(total: usize, block: Option<usize>) -> f32 {
    size::LIB_ROW_H * total as f32
        + block.map_or(0.0, |rows| {
            size::READING_MARGIN_TOP + size::LIB_ROW_H * rows as f32 + size::READING_MARGIN_BOTTOM
        })
}

/// The Library bay's rows, painted.
///
/// Where everything goes is [`library`]'s, so this paints and derives nothing.
///
/// Term for term from `style.css`:
///
/// - `.lib-row` — `color: var(--c-dim)`, a star and then a name at
///   [`size::BASE`], one [`size::LIB_ROW_PAD_X`] in from the left of the list
///   and centred across the row's own height, with `.lib-row`'s own
///   [`STAR_GAP`] between the two.
/// - `.lib-row .star` — `color: var(--c-sun)` where the store has starred that
///   Set, and `.star.off`'s `var(--c-faint)` where it has not. Drawn rather
///   than typed ([`star_mark`]), so the two states are a filled mark and an
///   outline of the same mark rather than two characters that may or may not
///   be in the face.
/// - `.lib-foot` — `color: var(--c-faint)` at [`size::LIB_FOOT_SIZE`], one
///   [`size::LIB_FOOT_PAD_X`] in and centred, over a
///   `border-top: 1px solid var(--c-hair)`.
///
/// A name too long for the track is clipped rather than elided, which is
/// the mock's own answer: `.lib-row` sets no `text-overflow` where `.path` and
/// `.strip-name` both do, so there is no ellipsis to draw. The clip is
/// `.lib-list`'s box, which is the same `with_clip_rect` the picture, a
/// preview cell and a tally are each drawn inside.
pub(super) fn library_into(
    ui: &Ui,
    pal: &Palette,
    bay: &LibraryBay,
    listed: Listed<'_>,
    cursor: usize,
    at: Target,
    open: Option<Opened<'_>>,
) {
    let Listed { rows, starred } = listed;
    let painter = ui.painter().with_clip_rect(bay.list);
    // **The rows that reach the picture and not the ones that are whole**,
    // which is `LibraryBay::drawn` against `LibraryBay::rows`: a row cut by an
    // edge is drawn as far as the list goes, and the clip above is what cuts
    // it. The foot's count is the other number and says how many are whole.
    for index in bay.drawn() {
        let Some(name) = rows.name(index) else {
            continue;
        };
        let row = bay.row(index);
        // `.lib-row.cursor` — `background: color-mix(in srgb, var(--c-lav)
        // 13%, transparent)` and `color: var(--c-text)`, where every other row
        // is `var(--c-dim)` over the bare card. **The wash is the whole of the
        // mark**: the mock puts no rule, no caret and no chevron on the row,
        // so a row that is not under the cursor is drawn exactly as it was
        // before this line existed.
        let ink = match index == cursor {
            true => {
                painter.rect_filled(
                    row,
                    CornerRadius::same(size::LIB_ROW_RADIUS as u8),
                    tint(pal.lav, 13),
                );
                pal.text
            }
            false => pal.dim,
        };
        // **The star before the name**, and its ink is the store's answer
        // rather than the cursor's: `.lib-row .star` is `--c-sun` whatever
        // else the row is wearing, and `.star.off` is `--c-faint` — the one
        // mark in this bay that a row's own state colours and the wash above
        // does not.
        //
        // **A procedure row draws none at all**, which is `.star.none`'s
        // `visibility: hidden` in the mock: the column is kept so every name
        // starts in the same place, and there is nothing in it because a star
        // is a control over a Set this store holds (ADR-0299, ADR-0338).
        if !rows.procedure(index) {
            let on = starred.contains(name);
            star_mark(
                &painter,
                bay.star(index).center(),
                STAR_SIZE,
                match on {
                    true => pal.sun,
                    false => pal.faint,
                },
                on,
            );
        }
        let galley = painter.layout_job(span_at(name, size::BASE, ink));
        painter.galley(
            Pos2::new(bay.named(index), row.center().y - galley.size().y * 0.5),
            galley,
            ink,
        );
        // **The badges, at the right of the row.** `.badge` is a hairline round
        // `--c-faint` and `.badge.kind` — the one badge a procedure row wears —
        // is `--c-line` round `--c-dim`, which is `style.css`'s own pair and
        // carries a real distinction: on a procedure row the single badge is
        // what the row *is*, where a Set's badges are a list of what it holds.
        let (ring, word) = match rows.procedure(index) {
            true => (pal.line, pal.dim),
            false => (pal.hair, pal.faint),
        };
        for (badge, box_) in bay.badges(ui.ctx(), index, &rows.badges(index)) {
            painter.rect_stroke(
                box_,
                CornerRadius::same(size::BADGE_RADIUS as u8),
                Stroke::new(size::HAIRLINE, ring),
                StrokeKind::Inside,
            );
            let galley = painter.layout_no_wrap(
                badge.to_owned(),
                FontId::new(size::BADGE_SIZE, FontFamily::Proportional),
                word,
            );
            painter.galley(
                Pos2::new(
                    box_.center().x - galley.size().x * 0.5,
                    box_.center().y - galley.size().y * 0.5,
                ),
                galley,
                word,
            );
        }
    }

    // **The reading, under the row it is a reading of.** Drawn inside the same
    // clip as the rows, which is what makes a reading taller than the bay a
    // clipped box rather than a box drawn over the foot — `.lib-list`'s own
    // answer to a name too long for the track, one axis round.
    if let (Some(block), Some(open)) = (bay.reading, open) {
        reading_into(&painter, pal, &block, open.reading);
    }

    let painter = ui.painter().with_clip_rect(bay.foot);
    let rule = bay.foot.min.y + size::HAIRLINE * 0.5;
    painter.line_segment(
        [
            Pos2::new(bay.foot.min.x, rule),
            Pos2::new(bay.foot.max.x, rule),
        ],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
    let galley = painter.layout_job(span_at(&bay.count(), size::LIB_FOOT_SIZE, pal.faint));
    painter.galley(
        Pos2::new(
            bay.foot.min.x + size::LIB_FOOT_PAD_X,
            bay.foot.center().y - galley.size().y * 0.5,
        ),
        galley,
        pal.faint,
    );

    // **The `params` chip**, between the count and the `load` button, and it
    // is a toggle drawn as one: `.pill.armed`'s mint while a reading is open
    // and `.pill`'s hairline round `--c-dim` while none is.
    //
    // **Which of the two is read off the block this bay is *drawing***, and
    // off nothing else — the same `LibraryBay::reading` that
    // [`LibraryBay::read`] matches on to decide what a press asks for. So the
    // capsule an operator is looking at and the answer the press gives cannot
    // come apart, which is what a toggle owes and is why this is not a second
    // reading of `View::reading` (ADR-0312).
    pill_into(
        ui,
        pal,
        bay.params_chip(ui.ctx(), at),
        PARAMS_PILL,
        bay.reading.is_some(),
    );

    let load = bay.load(ui.ctx(), at);
    // **`.pill.lav`, and it is the one pill on this panel with no border**:
    // `border-color: transparent; color: var(--c-lav); background:
    // color-mix(in srgb, var(--c-lav) 15%, transparent)`. Every other capsule
    // here is [`pill_at`]'s hairline round `--c-dim`, and the difference is
    // the point — `console.html`: *"`load` is lav because it is the press this
    // bay exists for, and a second lav chip beside it would make the colour
    // mean two things at a width of eight characters"*.
    painter.rect_filled(
        load.button,
        // `border-radius: 999px` on a box this short is a capsule.
        CornerRadius::same((size::PILL_H * 0.5) as u8),
        tint(pal.lav, 15),
    );
    let galley = painter.layout_no_wrap(
        LOAD_PILL.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        pal.lav,
    );
    painter.galley(load.text.min, galley, pal.lav);

    // **The `→` between the two capsules is drawn and is on neither of them**,
    // which is the whole of what [`LOAD_ARROW`] is: the mock's `&rarr;` was
    // typed here and `egui`'s default face has no U+2192, so the row read
    // `load □ A` — a label saying how to read two controls, with a tofu where
    // the reading was. It is `--c-faint`, which is `.lib-foot`'s own colour
    // and the colour the count at the other end of the row is in: it is the
    // foot's furniture rather than either control's, and a mark in the lav
    // would put this bay's accent on a thing nobody can press.
    arrow_mark(&painter, load.arrow.center(), LOAD_ARROW, pal.faint, false);

    // **The pulldown, drawn as the `params` chip is and not as the button is.**
    // `console.html`: *"It is deliberately not lavender. Lavender here is the
    // deck the keys are addressed to, and this is the one letter on the
    // console that is allowed to name a different one."* So it is
    // [`pill_at`]'s hairline round `--c-dim` with the chevron the two menu
    // pills in the transport row already carry.
    pill_at(ui, pal, load.deck, at.letter());
    painter.add(egui::Shape::convex_polygon(
        vec![
            load.chevron.left_top(),
            load.chevron.right_top(),
            Pos2::new(load.chevron.center().x, load.chevron.max.y),
        ],
        pal.dim,
        Stroke::NONE,
    ));
}

/// The pulldown's list, painted — the card and a row per deck the mixer is
/// drawing a strip for.
///
/// Where everything goes is [`Load`]'s, so this paints and derives nothing.
/// Drawn from [`View::draw`] after the bays for the arrangement menu's reason:
/// the card hangs out of the foot it belongs to and over this bay's own list,
/// so a card painted from inside the Library arm would go on before the rows
/// and end up under them.
///
/// The card is the arrangement menu's card term for term — the panel's own
/// fill, a hairline and the shadow — because it is the same object one bay
/// along and a second treatment would be a second answer to *what does a list
/// hanging off a capsule look like* (`console.html` draws neither, which is
/// what makes this the console's own).
///
/// The row the target is on is drawn in `--c-text` and the rest in `--c-dim`,
/// which is the arrangement menu's own reading of *which of these is in use*.
pub(super) fn deck_list_into(ui: &Ui, pal: &Palette, load: &Load, at: Target, card: Rect) {
    let painter = ui.painter();
    painter.add(pal.shadow.as_shape(card, CornerRadius::same(8)));
    painter.rect_filled(card, CornerRadius::same(8), pal.panel);
    painter.rect_stroke(
        card,
        CornerRadius::same(8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    // `take` rather than a range, because the rows are the letters: a list
    // longer than [`DECK_LETTERS`] is a deck this crate has no letter for, and
    // `View::aim_at` is what stops one being asked for.
    for (index, letter) in DECK_LETTERS.iter().enumerate().take(load.rows) {
        let row = load.row(card, index);
        let ink = match index == usize::from(at.deck) {
            true => pal.text,
            false => pal.dim,
        };
        let galley = painter.layout_no_wrap(
            (*letter).to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            ink,
        );
        painter.galley(
            Pos2::new(
                row.min.x + size::LIB_ROW_PAD_X,
                row.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
    }
}

/// A row's menu, painted — the card, a row per deck the mixer is drawing a
/// strip for, the separator, and the send under it.
///
/// Where everything goes is [`RowMenu`]'s, so this paints and derives nothing,
/// which is [`deck_list_into`]'s own sentence one control along. Drawn from
/// [`View::draw`] after the bays for that card's reason: it hangs out of the
/// row it belongs to and over the rows under it, so a card painted from inside
/// the Library arm would go on before them and end up underneath.
///
/// The card is the deck pulldown's card term for term — the panel's own fill, a
/// hairline and the shadow — because it is the same object one control along
/// and a second treatment would be a second answer to *what does a card hanging
/// off something look like*. `.rowmenu` in `docs/manual/style.css` says the
/// same thing from the mock's side.
///
/// Every item is drawn in `--c-dim`, and none of them is in `--c-text`. The
/// pulldown's card marks the row the target is on, because that list is a mark
/// being moved; this one is six acts and none of them is a state, so there is
/// nothing here for an ink to say. `.rowmenu .item` carries `color:
/// var(--c-dim)` and no second rule.
///
/// The separator is drawn and is not an item: one hairline in `--c-hair` across
/// the band, inset from the card's edge, which is the mock's `.rowmenu .rule`
/// and the same pixel every other rule on this panel is drawn at.
pub(super) fn row_menu_into(ui: &Ui, pal: &Palette, menu: &RowMenu) {
    let painter = ui.painter();
    painter.add(pal.shadow.as_shape(menu.card, CornerRadius::same(8)));
    painter.rect_filled(menu.card, CornerRadius::same(8), pal.panel);
    painter.rect_stroke(
        menu.card,
        CornerRadius::same(8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let word = |at: Rect, text: String| {
        let galley = painter.layout_no_wrap(
            text,
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        );
        painter.galley(
            Pos2::new(
                at.min.x + size::LIB_ROW_PAD_X,
                at.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.dim,
        );
    };
    // `take` rather than a range, for the deck list's reason: the items are
    // the letters, and `Menued::decks` past `DECKS` is a deck this crate has
    // no letter for.
    for index in 0..menu.loads.min(DECKS) {
        word(menu.load(index), load_item(index as u8));
    }
    // **The separator and the send are drawn where there is one**, which is a
    // Set row: a procedure cannot be written out as a `.kbset` — nothing takes
    // a bare `.kir` in — so its menu is the loads and stops (ADR-0338).
    if let Some(band) = menu.rule {
        let rule = band.center().y;
        painter.line_segment(
            [
                Pos2::new(band.min.x + size::LIB_ROW_PAD_X, rule),
                Pos2::new(band.max.x - size::LIB_ROW_PAD_X, rule),
            ],
            Stroke::new(size::HAIRLINE, pal.hair),
        );
    }
    if let Some(save) = menu.save {
        word(save, MENU_SAVE.to_owned());
    }
}

/// What the list is drawing this frame: the rows, and which of them the store
/// has starred.
///
/// One argument because they are one reading — the host's `listing` writes the
/// two halves together, and a row and its mark drawn from two answers could
/// disagree about a Set that arrived between them. It is [`Filters`]' shape one
/// row down, borrowed for the same reason: nothing is cloned to draw a frame.
#[derive(Debug, Clone, Copy)]
pub(super) struct Listed<'a> {
    /// The names the bay lists and what each of them is — [`View::rows`].
    pub(super) rows: Rows<'a>,
    /// The ids the store has starred — [`View::starred`].
    pub(super) starred: &'a std::collections::BTreeSet<String>,
}

/// A star's mark, drawn rather than typed — [`arrow_mark`]'s reason one bay
/// along, and the case is sharper here: this mark's whole job is saying
/// *starred* or *not starred*, and a face with no `★` would answer it with a
/// tofu in both states.
///
/// `across` wide and the same tall, which is [`arrow_mark`]'s rule for a mark
/// that stands in for a glyph: the box is the size the glyph would have been,
/// so the name beside it starts in the same place whichever way this is drawn.
///
/// Ten rim points at two radii, the outer at the top and the rest every 36°
/// round — [`STAR_WAIST`] is the inner one. `filled` is `.star` and the outline
/// is `.star.off`, which is the mock's own pair.
///
/// A fan from the centre and not a polygon, because a five-pointed star is not
/// convex: `egui::Shape::convex_polygon` fans from the first vertex, which for
/// this outline puts triangles outside the ink. A star *is* star-shaped about
/// its own centre, so a fan anchored there is exact — which is
/// [`mixer::mask_mark`]'s answer to `epaint` having no arc, one shape along.
fn star_mark(painter: &egui::Painter, centre: Pos2, across: f32, colour: Color32, filled: bool) {
    let outer = across * 0.5;
    let rim: Vec<Pos2> = (0..10)
        .map(|step| {
            // **From straight up, and clockwise**, which is where a star's
            // point is drawn: `egui`'s y runs down the screen, so the turn is
            // taken as a positive angle off `-y`.
            let angle = std::f32::consts::PI * 0.2 * step as f32;
            let r = match step % 2 {
                0 => outer,
                _ => outer * STAR_WAIST,
            };
            Pos2::new(centre.x + r * angle.sin(), centre.y - r * angle.cos())
        })
        .collect();
    if !filled {
        painter.add(egui::Shape::closed_line(
            rim,
            Stroke::new(size::HAIRLINE, colour),
        ));
        return;
    }
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(centre, colour);
    for point in &rim {
        mesh.colored_vertex(*point, colour);
    }
    for step in 0..10u32 {
        mesh.add_triangle(0, step + 1, (step + 1) % 10 + 1);
    }
    painter.add(egui::Shape::mesh(mesh));
}

/// A reading, painted: the well, and a row of it per line.
///
/// Where the box goes is [`library`]'s and where each row in it goes is
/// [`Block::row`]'s, so this paints and derives nothing — [`library_into`]'s
/// own rule one box out.
///
/// Term for term from `style.css` and from the markup the mock sets inline:
///
/// - the box — `background: var(--c-well)` at [`size::READING_RADIUS`], which
///   is the well a candidate row stands on in the staging lane.
/// - `.lib-row` with its left padding overridden — a name at [`size::BASE`],
///   one [`size::READING_PAD_X`] in from the left of the well, centred across
///   the row's own height.
/// - `.lib-row .dim` — `color: var(--c-faint)` at [`size::LIB_FOOT_SIZE`],
///   `margin-left: auto`, so the value ends one [`size::LIB_ROW_PAD_X`] in
///   from the right of the well. The same 10px the foot's count is drawn
///   at, which is the mock's own reading: a declaration is what the row is
///   *about* and the range beside it is the small type this bay uses for
///   everything a row is not named by.
/// - `.addr` — `color: var(--c-lav)`, on the head's word alone. The weight is
///   not honoured and cannot be, which is [`room`](crate::room)'s own sentence:
///   `egui`'s default proportional face has no bold, so a `font-weight: 700`
///   is a colour and a size here.
fn reading_into(painter: &egui::Painter, pal: &Palette, block: &Block, reading: &Reading) {
    painter.rect_filled(
        block.well,
        CornerRadius::same(size::READING_RADIUS as u8),
        pal.well,
    );
    let mut at = 0usize;
    let mut line = |left: &str, right: &str, ink: Color32| {
        let row = block.row(at);
        at += 1;
        let galley = painter.layout_job(span_at(left, size::BASE, ink));
        painter.galley(
            Pos2::new(
                row.min.x + size::READING_PAD_X,
                row.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
        let value = painter.layout_job(span_at(right, size::LIB_FOOT_SIZE, pal.faint));
        painter.galley(
            Pos2::new(
                row.max.x - size::LIB_ROW_PAD_X - value.size().x,
                row.center().y - value.size().y * 0.5,
            ),
            value,
            pal.faint,
        );
    };
    // **The head is the one lav in the box**, which is `.addr`'s own colour
    // and the mark that says this is a reading of the row above rather than
    // a sixth Set.
    line(READING_HEAD, &reading.knobs_word(), pal.lav);
    for knob in &reading.knobs {
        line(&knob.key, &knob.range, pal.dim);
    }
    // **The capacity is drawn in a knob's shape**, because that is how a
    // procedure declares it — and it is the declaration rather than this
    // Set's own number, which nothing on this panel says before a load. The
    // mock's note carries that as owed rather than answered here.
    if let Some(capacity) = &reading.capacity {
        line(READING_CAPACITY, capacity, pal.dim);
    }
    if let Some(emits) = &reading.emits {
        line(READING_EMITS, emits, pal.dim);
    }
    // **The foot counts the nodes and says how many of them could be read**,
    // which is the one thing that keeps a knob missing for want of a card
    // from being a knob missing in silence.
    line(&reading.nodes_word(), &reading.cards_word(), pal.dim);
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
pub(super) fn scopes_into(
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
        let ink = match marked {
            true => pal.lav,
            false => pal.faint,
        };
        // **The wash is the whole of the mark**, exactly as it is on the row
        // under the library cursor: `.scope` sets no border, no rule and no
        // dot, so a chip that is not marked draws nothing but its word.
        if marked {
            painter.rect_filled(
                chip,
                // `border-radius: 999px` on a box this short is a capsule.
                CornerRadius::same((size::SCOPE_H * 0.5) as u8),
                tint(pal.lav, 15),
            );
        }
        let galley = painter.layout_no_wrap(
            kind.name().to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            ink,
        );
        painter.galley(
            Pos2::new(
                chip.min.x + size::SCOPE_PAD_X,
                chip.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
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
pub(super) fn path_into(ui: &Ui, pal: &Palette, bay: &LibraryBay, at: Pointed<'_>) {
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

/// The filter row, painted: two fields and the rule under them.
///
/// Where the row goes and where each field in it goes are [`library`]'s and
/// [`LibraryBay::field`]'s, so this paints and derives nothing —
/// [`scopes_into`]'s rule one row up, and it is stricter here because the
/// fields are hit-tested and a second division would put a capsule a press
/// lands on somewhere the border is not.
///
/// Term for term from `style.css`:
///
/// - `.lib-filters { display: flex; gap: 5px; padding: 6px 9px; border-bottom:
///   1px solid var(--c-hair) }` — the field from the left of the row, over a
///   rule the row's bottom pixel.
/// - `.field { border: 1px solid var(--c-line); border-radius: 999px; padding:
///   0 9px; color: var(--c-faint); flex: 1 }` — a word at [`size::BASE`] in a
///   bordered capsule, taking the whole of what is left.
///
/// A set field and an unset one differ in the word alone, which is
/// [`HOLDS_UNSET`]'s sentence: `style.css` gives `.field` one rule and no set
/// variant, so `L4` where `layer…` was is the whole of the mark. `.scope.sel`'s
/// wash is not borrowed for it — that mark says *this is where a press lands*
/// about a chip a press moves between, and every press here lands on the field
/// it is already on.
///
/// A word too long for its field is clipped rather than elided, which is
/// `.lib-row`'s answer one box down and for the same reason: `.field` sets
/// `min-width: 0` and no `text-overflow`, so there is no ellipsis to draw. A
/// node name is what can be long enough for it.
pub(super) fn filters_into(ui: &Ui, pal: &Palette, bay: &LibraryBay, at: Filters<'_>) {
    let Some(row) = bay.filters else {
        return;
    };
    let painter = ui.painter().with_clip_rect(row);
    for field in Field::ALL {
        let Some(box_) = bay.field(field) else {
            continue;
        };
        painter.rect_stroke(
            box_,
            // `border-radius: 999px` on a box this short is a capsule, drawn
            // as half its own height — [`pill_at`]'s reason.
            CornerRadius::same((box_.height() * 0.5) as u8),
            Stroke::new(size::HAIRLINE, pal.line),
            StrokeKind::Inside,
        );
        let painter = painter.with_clip_rect(box_);
        let galley = painter.layout_job(span_at(at.word(field), size::BASE, pal.faint));
        painter.galley(
            Pos2::new(
                box_.min.x + size::FIELD_PAD_X,
                box_.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.faint,
        );
    }

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

/// The kind row, painted: the six toggles and the rule under them.
///
/// Where each chip goes is [`LibraryBay::kind_chips`], so this paints and
/// derives nothing — [`scopes_into`]'s rule two rows up, and it is that method's
/// shape term for term because it is the same object: a row of capsules, as
/// wide as the words in them, over a hairline that is the row's own bottom
/// pixel.
///
/// Term for term from `style.css`:
///
/// - `.lib-kinds { display: flex; gap: 4px; padding: 5px 9px; border-bottom:
///   1px solid var(--c-hair) }` — six chips from the left of the row, one
///   [`size::LIB_KINDS_GAP`] apart.
/// - `.kind { font-size: 9px; padding: 0 6px; border-radius: 999px; border: 1px
///   solid var(--c-line); color: var(--c-faint) }` — a word at
///   [`size::KIND_SIZE`] in a bordered capsule.
/// - `.kind.on { border-color: transparent; color: var(--c-mint); background:
///   color-mix(in srgb, var(--c-mint) 15%, transparent) }` — mint and not
///   lavender, which is what this console draws a control that is *on*: the
///   `params` pill in this bay's own foot is lit the same way, and lavender
///   here is the deck the keys are addressed to.
///
/// All six plain is the row a run opens on, and it says *everything shows*
/// rather than *nothing does* — [`karakuri_operation::LibraryKinds::narrowing`]
/// settles that reading once, and nothing here draws a seventh state for it.
pub(super) fn kinds_into(ui: &Ui, pal: &Palette, bay: &LibraryBay, at: Filters<'_>) {
    let Some(row) = bay.kinds else {
        return;
    };
    let painter = ui.painter().with_clip_rect(row);
    for (chip, box_) in bay.kind_chips(ui.ctx()) {
        let on = chip.on(at.kinds);
        let ink = match on {
            true => pal.mint,
            false => pal.faint,
        };
        match on {
            true => painter.rect_filled(
                box_,
                CornerRadius::same((size::KIND_H * 0.5) as u8),
                tint(pal.mint, 15),
            ),
            false => painter.rect_stroke(
                box_,
                CornerRadius::same((size::KIND_H * 0.5) as u8),
                Stroke::new(size::HAIRLINE, pal.line),
                StrokeKind::Inside,
            ),
        };
        let galley = painter.layout_no_wrap(
            chip.word().to_owned(),
            FontId::new(size::KIND_SIZE, FontFamily::Proportional),
            ink,
        );
        painter.galley(
            Pos2::new(
                box_.center().x - galley.size().x * 0.5,
                box_.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
    }

    let rule = row.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [Pos2::new(row.min.x, rule), Pos2::new(row.max.x, rule)],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
}

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

    /// Turn the Library bay's wheel by `by` pixels, positive down the listing, and
    /// answer whether the stored position moved.
    ///
    /// # Two clamps, and only one of them is here
    ///
    /// This one is against the content — how tall the listing and the reading under
    /// it come to ([`library_content_h`]) — and it is a reading of what the store
    /// answered rather than of a viewport, so a stored position bounded by it is
    /// not a position any resize can rewrite. Without it a wheel spun over a
    /// listing of three would put the number in the thousands and an operator would
    /// have to spin it all the way back before anything moved.
    ///
    /// The other clamp is against the list's own height and belongs where the bay
    /// is laid out — `library_box`, which is
    /// [P-0082](../../../../docs/principles/0082-looking-never-writes-back.md): a
    /// shorter bay draws less of the same position and stores nothing, so dragging
    /// it back reproduces the picture exactly rather than nearly.
    ///
    /// [`View::scroll_by`]'s shape one bay over, and the difference is what the two
    /// are told: a pane is named by index and this bay is the only one of itself.
    ///
    /// A console with nothing listed refuses the wheel rather than storing a
    /// position for it, which is [`View::point_at`]'s rule: what a pointer can be
    /// at is something drawn.
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

    /// Whether a reading is open at all, whatever row it was read of.
    ///
    /// [`View::opened`] is what the bay is drawn from and answers `None` where the
    /// row it belongs to is gone; this is the flatter question a host asks after
    /// moving the cursor, because the reading follows the cursor: a move with one
    /// open is a read of the row it arrived at, and a move with nothing open is a
    /// pointer moving.
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

    /// Which library the bay is listing, or `None` for a console nobody has told
    /// what libraries there are.
    ///
    /// Answered against [`View::scopes`] rather than read back bare, which is
    /// [`View::cursor_row`]'s rule: a host that offered four chips and then three
    /// leaves a position past the end, and the scope that is marked is then the
    /// last one there is.
    ///
    /// This is what the host answers with. It says which listing belongs in
    /// [`View::library`] and what a load off a row means — a row of
    /// [`Scope::MySets`] is a Set the store already holds and a row of
    /// [`Scope::Presets`] is a file that has to be taken in first (`console.html`'s
    /// *A Set has two forms, and loading one is packaging it*).
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
    pub(super) fn marked(&self) -> usize {
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
