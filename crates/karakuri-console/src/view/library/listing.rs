use super::super::*;
use super::*;

// ---------------------------------------------------------------------------
// The Library listing
// ---------------------------------------------------------------------------

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
            if ctx.cumulative_pass_nr() == 0 {
                return egui::Vec2::ZERO;
            }
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
        let widths: Vec<f32> = if ctx.cumulative_pass_nr() == 0 {
            words.iter().map(|_| size::BADGE_PAD_X * 2.0).collect()
        } else {
            words
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
                .collect()
        };
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
}

/// What the list is drawing this frame: the rows, and which of them the store
/// has starred.
///
/// One argument because they are one reading — the host's `listing` writes the
/// two halves together, and a row and its mark drawn from two answers could
/// disagree about a Set that arrived between them. It is [`Filters`]' shape one
/// row down, borrowed for the same reason: nothing is cloned to draw a frame.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Listed<'a> {
    /// The names the bay lists and what each of them is — [`View::rows`].
    pub(crate) rows: Rows<'a>,
    /// The ids the store has starred — [`View::starred`].
    pub(crate) starred: &'a std::collections::BTreeSet<String>,
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
pub(crate) fn reading_into(
    painter: &egui::Painter,
    pal: &Palette,
    block: &Block,
    reading: &Reading,
) {
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

/// One row of the Library bay, painted: the wash if under the cursor, the star,
/// the name and the badges.
pub(crate) fn row_into(
    ui: &Ui,
    pal: &Palette,
    bay: &LibraryBay,
    rows: Rows<'_>,
    starred: &std::collections::BTreeSet<String>,
    cursor: usize,
    index: usize,
) {
    let Some(name) = rows.name(index) else {
        return;
    };
    let painter = ui.painter().with_clip_rect(bay.list);
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
    for (badge_text, box_) in bay.badges(ui.ctx(), index, &rows.badges(index)) {
        badge(&painter, box_, badge_text, ring, word);
    }
}

/// The Library bay's rows, painted into the list area.
pub(crate) fn rows_into(
    ui: &Ui,
    pal: &Palette,
    bay: &LibraryBay,
    listed: Listed<'_>,
    cursor: usize,
) {
    let Listed { rows, starred } = listed;
    for index in bay.drawn() {
        row_into(ui, pal, bay, rows, starred, cursor, index);
    }
}

/// The foot's load control, painted: the button, the arrow label and the deck pulldown.
pub(crate) fn load_into(ui: &Ui, pal: &Palette, load: &Load, at: Target) {
    let painter = ui.painter();
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
    chevron_down(&painter, load.chevron, pal.dim);
}

/// The foot of the Library bay, painted: the count, the params chip, and the load controls.
pub(crate) fn foot_into(ui: &Ui, pal: &Palette, bay: &LibraryBay, at: Target) {
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
    load_into(ui, pal, &load, at);
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
pub(crate) fn deck_list_into(ui: &Ui, pal: &Palette, load: &Load, at: Target, card: Rect) {
    let painter = ui.painter();
    popup_card(painter, pal, card);
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
pub(crate) fn row_menu_into(ui: &Ui, pal: &Palette, menu: &RowMenu) {
    let painter = ui.painter();
    popup_card(painter, pal, menu.card);
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
