use super::*;

// ---------------------------------------------------------------------------
// The Inspector
// ---------------------------------------------------------------------------

/// **How many panes the inspector has**, which is `lib.rs`'s [`arrangement`]
/// and the mock's `.insp-split` read as one number: the CSS is
/// `grid-template-columns: 1fr 9px 1fr`, two tracks and the bar between them,
/// and the arrangement builds `inspector-1` and `inspector-2` to match.
///
/// A [`Spec`](karakuri_layout::Spec) builds a
/// [`Layout`](karakuri_layout::Layout) once and the arena has no insert, so
/// the count is settled at build time — the mock's `2 up ▾` is an operator
/// choosing it while running, and that is a control this pass does not add.
pub const PANES: usize = 2;

/// **The arrangement's name for each pane**, in the order the mock draws them.
///
/// Public for [`DECK_LETTERS`]'s reason: a harness that says *which pane* has
/// to say it in the names the arrangement addresses them by, and a second list
/// written out there would go on saying `inspector-1` the day this one does
/// not.
pub const PANE_NAMES: [&str; PANES] = ["inspector-1", "inspector-2"];

/// **Which deck each pane opens pointed at** — the first pane at deck A and
/// the second at deck B, which is the mock's own two heads.
///
/// It is what this console did before the pulldown existed, written down as a
/// *default* rather than left as the host's habit: the panes used to be filled
/// slot by slot, so a fourth slot could not be looked at at all. See
/// [`View::pane_deck`], which is the pointer this seeds.
///
/// **Not a reading of anything**, so a console with fewer strips than this
/// names opens with a pane pointed at a deck the mixer draws none for — which
/// is a pane with no nodes in it and is the honest state, exactly as
/// [`View::selection`]'s deck A is on a console with no deck behind it.
///
/// [`View::pane_deck`]: View::pane_deck
pub const PANE_DECKS: [u8; PANES] = [0, 1];

/// **The three levels a node head shows, in the order it shows them.**
///
/// `karakuri_operation::Authority` deliberately carries no `ALL` — *"no map
/// target names an authority … it arrives with the first reader"* — and this
/// is that first reader, so the list is written here rather than there. The
/// order is the vocabulary's own declaration order, most restrictive first,
/// which is also the mock's `man / sug / auto`.
///
/// **A fourth level cannot slip past it**: [`auth_word`] is a `match`, so a
/// level added to the vocabulary does not compile until it has a word here,
/// and `every_authority_the_vocabulary_names_is_on_the_node_head` holds this
/// array against that match.
///
/// **Public since the chips became controls**: `crate::input::PROBES` says how
/// many controls a node head's chips are, and a `3` written there would be a
/// second answer to a question this array already gives. The list is the
/// console's and the count is one reading of it — [`PANE_NAMES`]' reason for
/// being public, one list along.
pub const AUTHORITIES: [Authority; 3] = [
    Authority::Manual,
    Authority::Suggesting,
    Authority::Automatic,
];

/// **The node head's abbreviation for one level**, which is the console's own
/// word and deliberately not `Authority::name`: the vocabulary spells these
/// *manual*, *suggesting* and *automatic* because that is what a record
/// carries, and the manual's node head reads `man / sug / auto`.
///
/// A `match` for [`blend_mode`](crate::view)'s reason one crate along: a
/// fourth level in the vocabulary stops the build here rather than drawing a
/// blank chip.
fn auth_word(authority: Authority) -> &'static str {
    match authority {
        Authority::Manual => "man",
        Authority::Suggesting => "sug",
        Authority::Automatic => "auto",
    }
}

/// **The word on the sync chip**, which is `karakuri_operation::Sync::name`
/// and not a second spelling: *free*, *tempo*, *beat* are the manual's three
/// and the mock's three.
fn sync_word(sync: Sync) -> &'static str {
    sync.name()
}

/// **Every sync mode, in the order the deck head's chip walks them** — the
/// vocabulary's own declaration order, which is `Sync::ALL`'s in
/// `karakuri_engine::transport` and the mock's *free, tempo, beat*.
///
/// [`AUTHORITIES`]' array one head along, and it exists for the same two
/// reasons: [`DeckHead::mode`] steps *positions* in this list, which a `match`
/// cannot express once the step may be refused, and `tests/deck_head.rs` walks
/// it to assert the cycle reaches every mode. A fourth mode is still caught at
/// build time, by [`sync_word`]'s `match` on the way to a word and by
/// [`anchor_letter`]'s on the way to a letter.
///
/// **Public because [`Pane::allows`] is stated in this order**, and whoever
/// fills that field is in another crate: an order a caller has to match and
/// cannot name is an order two crates would each write down.
pub const SYNCS: [Sync; 3] = [Sync::Free, Sync::Tempo, Sync::Beat];

/// **The next mode the chip can actually get to**, walking [`SYNCS`] from the
/// one after `at` and taking the first this deck's material can honour.
///
/// # It skips rather than offering, and that is the mock's own sentence
///
/// *"Click to cycle, and the cycle skips a mode this material cannot honour
/// instead of offering it"*. Whether a mode is honourable is
/// `karakuri_engine::deck::Deck::sync_allowed`'s and arrives here as
/// [`Pane::allows`] — the surface owns the affordance and never the authority
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
/// so this chooses which destination to name and refuses nothing.
///
/// # Landing back on `at` is a state and not a failure
///
/// The walk starts at `at + 1`, so it comes back to `at` only when every other
/// mode is refused — and the operation that comes out then names the mode the
/// deck is in, which **re-anchors**. That is the one re-anchor a cycle can
/// reach and it is the one where re-anchoring does nothing: the only material
/// that refuses two modes is material that accumulates *and* reads the beat,
/// where the mode left is `Free` and `Free` reads no anchor. The control that
/// can ask for it deliberately is [`DeckHead::anchor`]
/// ([ADR-0218](../../../../docs/adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)).
///
/// `Free` is refused by nothing, so the loop always finds something and there
/// is no `None` to answer.
pub(crate) fn next_sync(at: Sync, allows: [bool; SYNCS.len()]) -> Sync {
    let from = SYNCS.iter().position(|s| *s == at).unwrap_or(0);
    (1..=SYNCS.len())
        .map(|step| (from + step) % SYNCS.len())
        .find(|index| allows[*index])
        .map(|index| SYNCS[index])
        .unwrap_or(at)
}

/// **The letter the anchor readout leads with**: the mock's `T128` under tempo
/// sync and `B128 +0.25` under beat, and nothing at all under free — *"a free
/// deck shows neither, because free is the absence of a transport rather than
/// a setting, and a column reading free on every deck would be four words of
/// nothing."*
fn anchor_letter(sync: Sync) -> Option<&'static str> {
    match sync {
        Sync::Free => None,
        Sync::Tempo => Some("T"),
        Sync::Beat => Some("B"),
    }
}

/// **What one pane of the inspector is showing**, handed in by whoever has a
/// deck — the same seam [`Strip`] crosses, one bay along.
///
/// `src/` takes no device and no engine (ADR-0156), so nothing here asks a
/// `Set` anything: every field is a value somebody who *can* ask read off one
/// and wrote down. `crates/karakuri` is where that reading is, and it is where
/// the two omissions below are decided as well.
#[derive(Debug, Clone, PartialEq)]
pub struct Pane {
    /// **Which deck this pane is pointed at**, as an index into
    /// [`DECK_LETTERS`] — the mock's `deck A`.
    ///
    /// **The pane is pointed rather than choosing**, and that is the mock's
    /// `showing … ▾` not being drawn: the chooser is a control and this pass
    /// adds none, so whoever fills this says which deck each pane shows.
    ///
    /// **And it is not [`View::selection`]**, which this console does now
    /// keep. That is *the* deck — one value, what a key press is addressed to,
    /// drawn as one ring — and there are two panes: a chooser here picks a
    /// deck to *look at* while the keys stay where they were, which is the
    /// whole of why the mock draws a caret in each pane head and a ring on one
    /// strip. So this waits on a per-pane pointer nothing keeps, and reading
    /// the deck selection into it would fold two facts into one and make the
    /// second pane a copy of the first.
    pub deck: usize,
    /// **What that deck is playing**, which is the same name the deck's mixer
    /// strip carries and comes from the same place — see [`Strip::name`], and
    /// the short of it is that a `Set` has no name of its own and only
    /// whoever built it knows what to call it.
    pub material: String,
    /// What this deck's clock is locked to: `karakuri_engine::transport::Sync`
    /// as the vocabulary's copy of the same three.
    pub sync: Sync,
    /// **Which of [`SYNCS`] this deck's material can honour**, in that order —
    /// what the sync chip's cycle skips over.
    ///
    /// **The answer and not the two facts it is computed from**, which is the
    /// seam every other field here crosses read one step further along.
    /// `karakuri_engine::deck::Deck::sync_allowed` is what decides it, off
    /// whether the Set in the slot is closed form and whether it reads
    /// `beats`, and `src/` has no engine (ADR-0156) — so whoever owns one asks
    /// it three times and writes the three answers here. Handing in the two
    /// properties instead would put a third copy of `Transport::allows`' rule
    /// in a crate that owns no material, and a control is not the authority on
    /// what it may ask for
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// **A property of the Set, so it moves when a build lands in the slot** —
    /// the engine says so at `sync_allowed`, and it is why this is read beside
    /// [`Pane::sync`] rather than once.
    ///
    /// All three `true` is the whole of *nothing is refused*, which is what a
    /// console with no engine behind it and every test in this crate that does
    /// not say otherwise hands in; `Free` is refused by no material at all, so
    /// the first entry is never `false` in a reading anything took.
    pub allows: [bool; SYNCS.len()],
    /// **The tempo the deck was engaged at**, which is what its rate is
    /// measured against. Drawn only under [`Sync::Tempo`] and [`Sync::Beat`] —
    /// see [`anchor_letter`].
    pub anchor_bpm: f32,
    /// **The scrub's own value, in beats**, signed. Drawn only under
    /// [`Sync::Beat`], *"since that is the only mode that reads the offset"*.
    ///
    /// In **beats** and one deck's, where the transport row's offset is in
    /// milliseconds and is the whole instrument's — `style.css` says the unit
    /// is what tells them apart, *"so neither is ever drawn without one"*, and
    /// the mock's own `+0.25` is what this is drawn as.
    pub scrub_beats: f64,
    /// Whether this deck's Set folds its renderers into one result or
    /// overdraws them: `karakuri_engine::set::Layering`, as a bit.
    ///
    /// **What the deck is doing, and what a press names the other of.** The
    /// chip's tooltip is *"Click to overdraw them instead"*, and this is both
    /// the word [`deck_head_into`] draws and the state
    /// [`DeckHead::composite`] reads to say which layering a press is asking
    /// for — a destination and never a flip
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// **It said the press was not a control until 2026-09-09**, on the
    /// grounds that layering is a *build* decision in the engine —
    /// `Set::layering` answers off whether the Set was built with a merge, and
    /// nothing writes it afterwards. Both halves of that are still true and
    /// the conclusion was wrong: a rebuild is what this instrument already
    /// does to change what a slot is running, and the layering is one field of
    /// the aim a watcher is pointed at, so a press re-aims the slot and the
    /// worker rebuilds it — the route a library load takes, judged against the
    /// budget like any other build
    /// ([ADR-0314](../../../../docs/adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)).
    /// It is a **readout of what landed** rather than of what was asked for,
    /// which is `Mixer::residency`'s division: the build may still be rolled
    /// back, and the Staging lane is what says so.
    pub composite: bool,
    /// **The two fields of this slot's aim the deck head can move**, or `None`
    /// on a deck with no geometry to size and no randomness to seed — see
    /// [`Aimed`], which is where the argument is.
    pub aimed: Option<Aimed>,
    /// **The node groups**, in node order, which is the order a Set addresses
    /// its own nodes in.
    pub nodes: Vec<Node>,
}

/// **What a slot is built at, in the two fields the deck head offers**: how
/// many elements each of its geometries runs at, and the salt its randomness
/// comes from.
///
/// # It is a reading somebody else took, which is why the candidates are here
///
/// Neither number can be worked out on this side. The ladder is the powers of
/// two inside the range the *material* declares — `capacity [min, max] =
/// default`, which is a `.kir`'s statement and reaches this crate through
/// whoever read the Set — and the salt is the next in the slot's own
/// deterministic sequence, which is the engine's arithmetic over the salt the
/// slot is actually running. So both cross the seam as answers rather than as
/// the facts they are computed from, exactly as [`Pane::allows`] does and for
/// the same reason: a control is not the authority on what it may ask for
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
/// and this crate holds no engine and no store (ADR-0156).
///
/// **The salt is handed over rather than invented**, which is the half worth
/// stating twice. A console that reached for a random number would produce a
/// picture no later run could produce again
/// ([P-0092](../../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md));
/// a console handed the next value of a sequence names a destination the way
/// every other control here does.
/// `docs/adr/0328-the-inspectors-deck-head-steps-a-slots-capacity-and-re-salts-it.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aimed {
    /// **What the slot is running at**, which is the number the chip reads.
    ///
    /// One number for the slot, because that is what a re-aim carries: a Set
    /// holding two geometries runs both at a stated capacity, and where none
    /// is stated this is the first geometry's own declared default — the same
    /// reading ADR-0228 records the aim taking.
    pub capacity: u32,
    /// **Whether [`Aimed::capacity`] was asked for**, rather than being what
    /// the material declares for itself. It is the chip's lit state and
    /// nothing else: the number is drawn either way, and this says who chose
    /// it.
    pub stated: bool,
    /// **The numbers a press steps through**, ascending — the powers of two
    /// inside the range every one of this deck's geometries accepts.
    ///
    /// **Empty is a chip that is drawn and claims nothing**, which is the
    /// arrangement an inert scrub is already in: a deck whose geometries
    /// declare no range in common has no capacity a re-aim could send that all
    /// of them would build at, and there is nothing here to offer.
    pub capacities: Vec<u32>,
    /// **The salt `re-salt` asks for**: the next in this slot's own sequence,
    /// derived from the salt it is running.
    pub salt: u32,
}

/// **One node group**: the head that names a node and says who may move it,
/// and whatever is under it.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The mock's `.addr` — `L1:0`, `L2:0`, `L4`. **Written by whoever read
    /// the Set**, because the layer names are `karakuri-ir`'s `Kind` and this
    /// crate depends on neither it nor the engine.
    pub addr: String,
    /// What the node is called: `Set::node_names`, or the mock's `renderers`
    /// for the group that folds several.
    pub name: String,
    /// **Who may move this node**, or `None` where the group is more than one
    /// node and so has no one value.
    ///
    /// The second case is the mock's own `L4 renderers` head: authority is set
    /// **per node** (ADR-0211, ADR-0216) and that head stands over every
    /// renderer the Set has, so a chip on it would be one of *n* answers drawn
    /// as *the* answer. A Set with one renderer has one node under that head
    /// and the chip is drawn.
    ///
    /// **It carries the node's address beside the level, and it is one field
    /// rather than two.** A press on a chip has to say which node it is
    /// about, and the two are absent together — a head with no one answer has
    /// no one node either — so a pair of `Option`s would be *present or absent
    /// as a unit* held true by prose, which is `karakuri_store::record::NodeAddress`'s
    /// argument one crate along and `docs/contributing.md` §4's structural
    /// tier.
    pub authority: Option<NodeAuthority>,
    /// **The node whose source a `keep` on this head would write**, or `None`
    /// on a head with nothing to keep.
    ///
    /// **Two heads carry no capsule and both are the rule rather than an
    /// omission** (`docs/adr/0338-…`, decision 4):
    ///
    /// - **A head standing over more than one node**, which is the mock's
    ///   folded `L4 renderers`. It is [`Node::authority`]'s own absence one
    ///   control along and for its sentence: one capsule over three renderers
    ///   would be one of three answers drawn as *the* answer. Open the fold
    ///   and each renderer has its own.
    /// - **The built-in camera**, which is a node with no procedure behind it:
    ///   a Set declaring no `kind L3` holds the built-in orbit at `L3:0`
    ///   (`docs/ir-spec.md`, *Several cameras*), and there is **no source to
    ///   write**. The mock draws that absence too, and this pass reproduces it
    ///   rather than drawing a capsule that refuses.
    ///
    /// **It is a field beside [`Node::authority`] rather than that field read
    /// again**, because the two absences are not the same set: the built-in
    /// camera *has* an authority and has nothing to keep. A `bool` beside the
    /// address would be *present or absent as a unit* held true by prose,
    /// which is [`NodeAuthority`]'s own argument, so the address and the
    /// having-one are one `Option`.
    pub keep: Option<NodeAddress>,
    /// The mock's `.rend-row`: every renderer this Set has, and which of them
    /// is live. Empty on every group that is not the renderers'.
    pub renderers: Vec<Renderer>,
    /// **The inputs this node's procedure declares**, and what fills each —
    /// empty on every node that declares none, which is most of them.
    pub uses: Vec<Uses>,
    /// The published controls that belong to this node.
    pub params: Vec<Param>,
}

/// **One input a node's procedure declares, and the node filling it** — the
/// mock's `.uses` line, under the node head and above that node's rows.
///
/// # The slot is the procedure's word and the node is the Set's
///
/// `uses far : Geometry` names `far` and stops, because a part that names the
/// parts around it is bound to one Set and stops being a library part
/// ([P-0086](../../../../docs/principles/0086-a-procedure-knows-only-what-it-declares.md),
/// [ADR-0152](../../../../docs/adr/0152-a-kir-names-a-slot-and-the-set-names-the-nodes.md)).
/// So `slot` is the declaration's own word and `to` is a node's name, which is
/// exactly `karakuri_engine::set::Edge`'s two halves and exactly what
/// [`Operation::WireInput`](karakuri_operation::Operation::WireInput) carries.
///
/// # The candidates are a reading somebody else took
///
/// [`Uses::candidates`] is the list the card offers, handed across the seam
/// like [`Pane::allows`] and [`View::holds`] before it: which nodes are of the
/// kind this input takes is a question about the Set, and this crate holds no
/// engine (ADR-0156). **What is offered is not what may be reached** — a name
/// the Set cannot use is refused where the Set is built, in the sentence a
/// model's `wire_input` meets
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Uses {
    /// **What the procedure calls this input** — `far` in `--edge morph.far=…`.
    pub slot: karakuri_operation::InputPort,
    /// **The node filling it**, by name. There is no unfilled state: a Set with
    /// an empty input does not build, so this control replaces and never
    /// clears.
    pub to: String,
    /// **The nodes a pick may name**, in node order — this deck's nodes of the
    /// kind the input takes, with the declaring node left out of its own list.
    pub candidates: Vec<String>,
}

/// **Which node a group's head is, and who may move it.**
///
/// The address is `karakuri_operation::NodeAddress`, **carried over from whoever
/// read the Set** and deliberately not parsed back out of [`Node::addr`]:
/// that field is the mock's `L1:0`, a display string in the layer word
/// `docs/ir-spec.md` owns, and reading an address back out of what is drawn is
/// the shape *a statement is held true by the thing it describes* forbids
/// ([ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeAuthority {
    pub at: NodeAddress,
    /// Who may move it — `manual` where nobody has spoken for it, which is
    /// what a node nobody has spoken for **is** rather than a placeholder.
    pub level: Authority,
}

/// **One renderer chip.**
#[derive(Debug, Clone, PartialEq)]
pub struct Renderer {
    pub name: String,
    /// **Whether this is the one that reaches the screen**, and it is only
    /// ever true under [`Pane::composite`]: *"On, a deck's renderers fold into
    /// one result and one of them is live; off, they are all overdrawn."*
    /// Under overdraw every renderer draws, so marking one would assert a
    /// choice the layering does not make.
    pub live: bool,
}

/// **One parameter row**: what the mock's `.param` reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    /// **The position in the deck's published interface**, counting from one
    /// and spanning nodes — the mock's `.ord`, and the number a MIDI control
    /// is learned against: *"knob 3 is knob 3 whatever Set is loaded"*.
    ///
    /// **`None` is a control the interface does not carry**, and it is the one
    /// state of this field rather than a missing value: a control off the
    /// interface has no position, and a position is exactly what a knob counts.
    /// Such a row keeps its place in its group and loses its number, its fader
    /// and its figure — drawn so the mark can be pressed again, because this
    /// bay is where publishing is chosen and a choice nobody can see is one
    /// nobody can unmake
    /// ([ADR-0100](../../../../docs/adr/0100-a-published-interface-is-a-choice-of-attention.md),
    /// `docs/adr/0329-…`).
    ///
    /// **It is not *hidden* and it is not *locked***: `--param`, a `param`
    /// record and a model naming the address all still reach the value, which
    /// is ADR-0100's whole sentence — publishing is a choice of attention and
    /// never one of authority.
    pub ord: Option<usize>,
    /// What the Set published it as, which may be an alias for the key inside
    /// the node.
    pub name: String,
    /// What it holds.
    pub value: f32,
    /// **What the Set published it over**, low then high — `Published::range`,
    /// which *"narrows, never redefines"* the range the procedure declared.
    ///
    /// # It used to be the position and is now the range, and that is a
    /// decision rather than a widening
    ///
    /// This field read `at: f32`, *"where it sits in its published range,
    /// `[0, 1]`"*, and its own argument was that **the fader is the only
    /// reader**, so a range plus a value would be a second derivation of
    /// *where along the track*. A fader a hand can move has a second reader —
    /// the grab, which turns a pointer back into a value — and that one needs
    /// the range whichever way this field is spelled. So the position is
    /// [`Param::at`], derived here, and the two directions are one statement
    /// in one place: [`ParamGrip`] and
    /// [ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md).
    pub range: [f32; 2],
    /// **Which control this row is**, as the vocabulary addresses one —
    /// `Published::at` and `Published::key`, carried over unchanged.
    ///
    /// **Not the group the row was drawn in**, which is the other reading and
    /// is the one ADR-0286 refuses: a wildcard covering exactly one node is
    /// *drawn* in that node's group, and it goes on meaning every node that
    /// declares the key. See [`ParamGrip`].
    ///
    /// The vocabulary's own type rather than a pair of this crate's, because
    /// the operation carries exactly this and a second spelling of an address
    /// is what `karakuri-operation` exists to stop
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    pub param: karakuri_operation::ParamAt,
    /// **What is holding this control**, or `None` for a row nothing is
    /// driving — the mock's `.param.bound` and the `.sens` row under it.
    ///
    /// **Written by whoever read the Set**, off its bindings, and it is the
    /// seventh reading the harness takes: `Set::bindings` was the one of them
    /// this pane did not ask for, on ADR-0191's terms, because nothing bound
    /// anything and a bound row was a state the program could not enter. What
    /// changed is that a press can now attach one.
    pub bound: Option<Source>,
}

/// **What is driving one parameter row** — the mock's `.pval.src` and the
/// three readouts on the `.sens` row under it.
///
/// # It is the attachment's own answer and not a second derivation
///
/// The signal, the shape and the range are read off the binding the Set is
/// holding and carried over unchanged, which is [`Param::param`]'s rule one
/// field along: what a control *is* comes from whoever published it, and a
/// surface that rebuilt any of it would be a second statement about one
/// attachment.
///
/// **[`Source::range`] is not [`Param::range`], and the two are two facts.**
/// The row's range is what the control was *published* over — the span the
/// fader rides, which is the procedure's declaration narrowed — and this one
/// is what the signal is *mapped onto*, which a `bind` may narrow again within
/// it. Drawing one and writing the other is what a chip on this row would do
/// if there were one range here, and the operator would see a mapping change
/// under a press that only asked for a different curve.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    /// The signal's own name, as the row's `.pval.src` reads it: `energy`,
    /// `beat`, `band3`, `noise`, or `control:<name>` for a macro.
    pub signal: String,
    /// The shape the signal is put through, and the one chip on this row that
    /// is a control.
    pub curve: karakuri_operation::Curve,
    /// What the signal is mapped onto, low then high — the attachment's, not
    /// the row's. See the type's own note.
    pub range: [f32; 2],
    /// **Which attachment this is**, as the vocabulary addresses one.
    ///
    /// `karakuri_operation::BindAt` and not [`Param::param`]'s `ParamAt`,
    /// carried over from the binding rather than derived from the row: an
    /// attachment is one layer's, where a value's wildcard names no layer at
    /// all, and the two are two facts rather than two spellings (see
    /// `BindAt`). It is also what makes a row's *take back* remove the
    /// attachment it is drawn from rather than one that happens to match by
    /// name.
    pub at: karakuri_operation::BindAt,
}

impl Param {
    /// **Where the value sits in the published range**, `[0, 1]` — the
    /// fader's fill, and what a knob's centre is put on.
    ///
    /// **A range of no width is a control with one position**, and the fader
    /// sits at its start rather than at a division by zero. That is the guard
    /// the harness used to carry when this was a field; it is here now, so
    /// there is one place a degenerate range is answered for.
    pub fn at(&self) -> f32 {
        let [low, high] = self.range;
        match high > low {
            false => 0.0,
            true => unit((self.value - low) / (high - low)),
        }
    }

    /// **What this control holds with its fader at `at`** — [`Param::at`]
    /// inverted, and the whole of what a hand on this row asks for.
    ///
    /// `at` is a position on `[0, 1]`, which is what [`Grab::value`] answers,
    /// and it is clamped here for [`unit`]'s reason rather than trusted: a
    /// published range narrows and never redefines, so a value outside it is
    /// one the procedure did not say it still looks like itself over.
    pub fn valued(&self, at: f32) -> f32 {
        let [low, high] = self.range;
        low + (high - low) * unit(at)
    }

    /// **Whether a hand can move this row at all.**
    ///
    /// `Grab::new`'s refusal read on the value axis instead of on the track: a
    /// published range of no width is a control with one position, so a knob
    /// on it is a handle with nowhere to go and every drag of it would ask for
    /// the value it already holds. The row is still **drawn** — the fill and
    /// the figure say what it is — and it is not taken hold of.
    ///
    /// # A bound row is drawn and is not taken hold of either
    ///
    /// **The number a hand would write is not the number the row is
    /// showing.** A knob a hand moves writes the value a binding blends
    /// *from*, and at a measurement's full confidence that value carries no
    /// weight at all — so the handle would move under the hand and the picture
    /// would not, which is the one thing
    /// [ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md)
    /// refuses a track press for: *"a handle that jumped to the pointer would
    /// be a lie about what a handle is"*, read on the value axis.
    ///
    /// **It is not a refusal of the write**, and that distinction is
    /// [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)'s:
    /// `Operation::WriteParam` on a bound parameter is legal, lands, and
    /// leaves the attachment where it is — a `--param` does exactly that
    /// today. What this says is that *this row's fader* is not the affordance
    /// for it while something else is holding the control, and the affordance
    /// that is there is `take back`, one row down. After it the row is a
    /// handle again.
    ///
    /// That is the question ADR-0286 left open — *"whether a hand may move a
    /// knob a signal is holding is Take a parameter back's question"* — and
    /// `docs/manual/console.html`'s *Who is holding a control* is where the
    /// page says it.
    /// **This row as an entry of a published interface** — the shape
    /// [`Operation::Publish`](karakuri_operation::Operation::Publish) carries,
    /// which is `karakuri_engine::set::Published`'s.
    ///
    /// **The address and the range are the ones the row was drawn from**, not
    /// ones re-derived here: a wildcard stays a wildcard and a narrowed range
    /// stays narrowed, which is [`ParamGrip`]'s own rule about writing the
    /// control the row draws rather than the group it was placed in
    /// (ADR-0286).
    fn control(&self) -> karakuri_operation::Control {
        karakuri_operation::Control {
            name: self.name.clone(),
            node: self.param.node,
            key: self.param.key.clone(),
            range: self.range,
        }
    }

    fn movable(&self) -> bool {
        // **A control off the interface draws no fader**, which is what
        // publishing decides: the row is a name and a mark, and there is
        // nothing on it to take hold of.
        self.ord.is_some() && self.bound.is_none() && self.range[1] > self.range[0]
    }
}

/// **The Inspector's pane, laid out**: the head that says which deck, the deck
/// head under it, and what is left for the node groups.
///
/// # What is in the mock's pane and is deliberately not here
///
/// This is [ADR-0200](../../../../docs/adr/0200-a-bays-first-pass-draws-the-values-that-exist-and-omits-the-rest.md)
/// applied to the bay it named as the next one and the hardest: *draw every
/// part of the mock that has a value behind it and omit the rest outright — no
/// placeholder, and no empty case the mock did not itself draw.* Nine things
/// are omitted and each one is named with what it waits on. **Three of the
/// nine have since been drawn**, and they are the deck head's — see
/// [`DeckHead`], which is where their argument now lives.
///
/// **Two are controls and are still not here.**
///
/// - **`showing … ▾`**, the chooser in the pane head. Which deck a pane shows
///   is a **per-pane** pointer this console does not keep, and it is not the
///   deck selection: that one is what a key press is addressed to and there is
///   one of it, where there is a caret in every pane head — see
///   [`Pane::deck`]. So the pane is *pointed* by whoever fills that field and
///   the caret is not drawn. The word `showing` and the deck it names are a
///   readout and are.
/// - **`keep`**, the pill beside it: *"Keep deck A as a Set, exactly as it is
///   on screen … It goes into the library under a name."* That is a write into
///   the store, which is the Library bay's `load` from the other end —
///   and it is the end that is still open. A load re-points a slot's source
///   and lets the worker build it; a keep has to read a *running* Set back out
///   and name it, which is `Set::published`'s side of the seam and a different
///   question entirely.
///
/// **A third was `composite`, called a readout here until 2026-09-09, and it
/// is a control** — [`Pane::composite`] and [`DeckHead::composite`]. The
/// sentence that stood here said layering is a build decision in the engine so
/// a press on it is a rebuild rather than a write, and *"there is no operation
/// in the vocabulary for it to name"*, which was wrong twice:
/// `Operation::SetCompositing` has been in the vocabulary the whole time, and a
/// rebuild is exactly how this instrument changes what a slot is running. A
/// press re-aims the slot — the layering is one field of `watch::Aim` — and the
/// worker builds it off the render thread, which is the route a library load
/// takes
/// ([ADR-0314](../../../../docs/adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)).
///
/// **And the fourth was the anchor, which the mock draws as a readout and
/// [ADR-0218](../../../../docs/adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)
/// made a control.** A press on it emits `SetSync` naming the mode the deck is
/// already in, which re-anchors, and its face goes on being a reading of two
/// numbers the deck has. The sync chip beside it and the scrub's two arrows
/// landed with it — the whole of the deck head is [`deck_head`] now, and this
/// type is the pane's three rows.
///
/// **Two had no value in this workspace at all until 2026-09-09**, which was
/// ADR-0191's rule — a panel drawing a state the engine never entered is a
/// drawing of one — **and both are drawn now**, because a press can attach a
/// signal (ADR-0319).
///
/// - **`.param.bound`'s `.pval.src`**, a bound parameter showing its source
///   instead of a number. [`Param::bound`] is the reading, off
///   `Set::bindings`, and what made it enterable is `Deck::bind` rather than
///   anything in this crate. **The mock's other two sources are still states
///   this program cannot enter**: `midi 21` and `seq 1` are not bindings at
///   all — no MIDI map reaches a Set's parameter, and a sequencer lane is a
///   fifth route into the vocabulary rather than a signal on the bus
///   (ADR-0222) — so a row drawn from either would be ADR-0191's drawing.
/// - **The `.sens` row under a bound parameter** — the signal, the curve, the
///   range and `take back`, which is [`SensChip`]. Two of its four are
///   controls and two are readouts, and the mock's `step` curve beside `seq 1`
///   is not one of the four this vocabulary has.
///
/// **Two are the shape of the mock disagreeing with the shape of a Set**, and
/// they are the two things this pass found:
///
/// - **A published control that names no node has no row.** The manual groups
///   parameters *"by node, the way a Set is addressed everywhere else"*, and
///   the mock draws every `.param` inside a `.node-group`. But
///   `Published::at` is an `Option` and the **default** interface — the one
///   every Set in `crates/karakuri` has, since that binary has no `--publish`
///   — is made entirely of wildcards: *"one control per key, not one per
///   declaration"*, addressed at every node that declares the key. A wildcard
///   covering exactly one node is that node's and is drawn there; one covering
///   several belongs to several groups and is **omitted**, because the mock
///   draws no row outside a group and inventing a place for one is a
///   specification written backwards. It waits on the page saying where such a
///   row goes.
/// - **The `L4` group's authority chip.** See [`Node::authority`].
///
/// **And one is the pane running out of room**, which is what
/// [`InspectorPane::scroll`] answers: the pane scrolls, and
/// [`InspectorPane::shown`] is what it says about that.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InspectorPane {
    /// `.half-head`, along the top of the pane, with its rule on the bottom.
    pub head: Rect,
    /// `.deck-head`, under it — the deck's own clock and its fold.
    pub deck_head: Rect,
    /// What is left under the two heads, where the node groups stack from the
    /// top with a hairline between them.
    pub body: Rect,
    /// **How far this pane's body is scrolled, in force this frame** — the
    /// stored position clamped against what there is to scroll through, and
    /// never the stored position itself.
    ///
    /// # Clamped here and stored nowhere
    ///
    /// The clamp is `0 ..= (content - body height)`, and both ends of that
    /// range move when the pane is dragged — so a clamp written back into
    /// [`View`] would be a resize rewriting what an operator scrolled to. That
    /// is
    /// [ADR-0250](../../../../docs/adr/0250-below-the-minima-the-arrangement-scales-rather-than-being-rewritten.md)'s
    /// rejected *clamp the stored size during the solve*, one region in, and
    /// [P-0082](../../../../docs/principles/0082-looking-never-writes-back.md) is
    /// the rule: **a shorter pane draws less of the same position and stores
    /// nothing**, so dragging it back reproduces what was on screen exactly
    /// rather than nearly.
    ///
    /// What [`View::scroll_by`] does clamp is the *content*, which is a
    /// reading of the deck rather than a viewport — see it for why the two are
    /// not the same clamp.
    pub scroll: f32,
    /// **How tall everything in this pane is**: [`group_h`] over every node
    /// with a [`size::HAIRLINE`] between two of them, whether or not any of it
    /// is on screen.
    ///
    /// It is the number [`scroll`](Self::scroll) is clamped against and the
    /// number [`View::scroll_by`] is clamped against, derived in one place so
    /// the two cannot disagree.
    pub content: f32,
    /// **How many node groups this pane is showing whole**, which is the `n`
    /// of the `n of m` its head reads — [`pane_count`], and rule 04 of
    /// [the manual](../../../../docs/manual/index.html): *"A list that showed you
    /// part of itself says so and says how much."*
    ///
    /// # It is the readout's number and not the walk's
    ///
    /// [`InspectorPane::drawn`] is what is painted and what a press is
    /// hit-tested against, and it is the wider of the two: a group cut by the
    /// top edge or the bottom one is drawn as far as the pane goes and can be
    /// pressed where it is drawn. This counts the ones that are **whole**, so
    /// that `m of m` means *nothing is out of sight* and can never be read off
    /// a pane with a group hanging over an edge.
    ///
    /// **It used to be how many were drawn, and the two were one number.**
    /// A pane drew a group whole or not at all, because there was no position
    /// to scroll to — so below roughly 534px of Inspector bay not one
    /// parameter row was drawn, in the bay whose whole content is parameter
    /// rows. The maintainer's answer to that was *the pane scrolls*, and this
    /// is the half of the old rule that survives it: a part-drawn group is no
    /// longer a lie about what a node has, because the count says how many are
    /// whole and the rest is one notch of the wheel away
    /// ([ADR-0307](../../../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)).
    ///
    /// **Zero is a state and not a `None`.** A pane too short to hold one
    /// group whole still says which deck it is showing and what that deck's
    /// clock is doing, and still draws as much of the group as it has room
    /// for; it is [`inspector`]'s `None` that means *there is no pane here to
    /// draw*.
    pub shown: usize,
}

impl InspectorPane {
    /// Where the `index`th group goes, and how tall it is — **in the pane's
    /// own coordinates, with [`scroll`](Self::scroll) already taken off**, so
    /// a group above the body has a negative-going top and one below it a top
    /// past `body.max.y`.
    ///
    /// Derived rather than stored for [`LibraryBay::row`]'s reason — the
    /// groups are a walk and a `Vec` of rectangles would be an allocation a
    /// frame does not need — but a walk rather than a stride, because a group
    /// is as tall as what is in it.
    ///
    /// **Every index in `nodes` is an answer**, where this used to refuse
    /// anything past `shown`: a scrolled pane has groups off both edges and
    /// [`drawn`](Self::drawn) is what says which of them reach the picture, so
    /// the rectangle has to exist before that question can be asked. Past the
    /// end of `nodes` is still a caller's error.
    pub fn group(&self, nodes: &[Node], index: usize) -> Rect {
        let top = self.body.min.y - self.scroll
            + nodes
                .iter()
                .take(index)
                .map(|node| group_h(node) + size::HAIRLINE)
                .sum::<f32>();
        Rect::from_min_size(
            Pos2::new(self.body.min.x, top),
            egui::vec2(self.body.width(), group_h(&nodes[index])),
        )
    }

    /// **Which groups reach the picture**, as a range into `nodes` — the ones
    /// a scrolled body has any of on screen, cut edges included.
    ///
    /// It is what [`inspector_into`] paints and what
    /// [`InspectorPane::grip`] and [`InspectorPane::select_renderer`] walk, so
    /// a control is hit-tested over exactly the groups that were drawn. **It
    /// is not [`shown`](Self::shown)**, which counts the whole ones and is the
    /// readout's number: a fader in a group cut by the bottom edge is drawn
    /// and is pressable, and the group it is in is not counted as shown.
    ///
    /// A walk rather than arithmetic, for [`group`](Self::group)'s reason: the
    /// groups are of unequal height. Empty where the pane's body has no
    /// height at all, which is a folded pane.
    pub fn drawn(&self, nodes: &[Node]) -> std::ops::Range<usize> {
        let mut first = nodes.len();
        let mut last = 0;
        let mut top = self.body.min.y - self.scroll;
        for (index, node) in nodes.iter().enumerate() {
            let bottom = top + group_h(node);
            if bottom > self.body.min.y && top < self.body.max.y {
                first = first.min(index);
                last = index + 1;
            }
            top = bottom + size::HAIRLINE;
        }
        match first < last {
            true => first..last,
            false => 0..0,
        }
    }

    /// **What a press at `p` on a renderer chip asks for**, or `None` off
    /// every chip this pane drew.
    ///
    /// # The row is a choice only where the deck composites and holds two
    ///
    /// [Every operation](../../../../docs/manual/operations.html) states the
    /// condition on the row itself — *"Only where the deck composites and
    /// holds two or more. One-way: no position in the cycle folds them all
    /// back in"* — and `docs/manual/console.html` states it from the chips'
    /// side: *"This Set composites, so one renderer is live and the rest are
    /// not."* Under overdraw every renderer draws, so a selection would name a
    /// state the picture is not in; with one renderer there is nothing to
    /// choose between. **Both are drawn and neither is claimed**, which is
    /// [`DeckHead::arrow`]'s arrangement on an inert scrub and this crate's
    /// own rule stated at [`crate::input`]: *a control claims what it acts on
    /// and no more*. The chips keep their shape either way, because a row that
    /// vanished when a deck stopped compositing would move every parameter row
    /// under it out from under the hand.
    ///
    /// **Every chip of a live row is claimed, the lit one included.** It names
    /// a destination, which is what an operation on this panel is
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// and pressing the lit one is the selection the deck already has asked
    /// for again — the anchor's shape two rows up. A chip that stopped being
    /// pressable the moment it lit would take the claim out from under a hand
    /// on the beat the swap landed.
    ///
    /// # What is asked before what
    ///
    /// The body, then the group's row, then the chips — [`LibraryBay::chip`]'s
    /// order one bay along and for its reason: the chips are laid end to end
    /// from the row's left padding and [`inspector_into`] clips the paint to
    /// the pane, so a chip that finishes outside the pane is a target only for
    /// the part of it that is drawn. The row is the pane's own width, so that
    /// clip is this row's `contains`.
    ///
    /// **It costs a galley lookup per chip and only inside a renderer row**,
    /// which is [`LibraryBay::chips`]' price: a chip is as wide as the word in
    /// it, and the walk stops at the one under the pointer.
    pub fn select_renderer(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        // Fonts are not valid until `egui` has run a pass — [`deck_head`]'s
        // guard, and before the first one there is no chip drawn to press.
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            if !a_choice(pane, node) {
                return None;
            }
            let row = rend_row_in(self.group(&pane.nodes, index), node);
            if !row.contains(at) {
                return None;
            }
            rend_chips(ctx, row, &node.renderers)
                .find(|(_, chip)| chip.contains(at))
                .map(|(renderer, _)| Operation::SelectRenderer {
                    deck: pane.deck as u8,
                    renderer: renderer as u32,
                })
        })
    }

    /// **What a press at `p` on a node head's `man / sug / auto` asks for**,
    /// or `None` off every chip this pane drew.
    ///
    /// [`InspectorPane::select_renderer`]'s shape one row up, and the same
    /// order of questions: the body, then the group's head, then the chips.
    /// The head is the pane's own width, so the clip [`inspector_into`] paints
    /// under is this row's `contains`.
    ///
    /// **A head with no chip is not a target**, which is `Node::authority`
    /// being `None` — a head that folds more than one node has no one answer
    /// to draw and so no destination to press. Everything else about the walk
    /// is `select_renderer`'s.
    pub fn set_authority(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        // Fonts are not valid until `egui` has run a pass, and before the
        // first one there is no chip drawn to press — `deck_head`'s guard.
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let at_node = node.authority?.at;
            let group = self.group(&pane.nodes, index);
            let head = Rect::from_min_max(
                group.min,
                Pos2::new(group.max.x, group.min.y + size::NODE_HEAD_H),
            );
            if !head.contains(at) {
                return None;
            }
            // **Inside what the `keep` capsule leaves**, and the trim is
            // [`auth_chips`]' own rather than applied here — a press on the
            // capsule is [`InspectorPane::keep_procedure`]'s and reaches no
            // chip, because the chips are not drawn there.
            auth_chips(ctx, head, node)
                .find(|(_, chip)| chip.contains(at))
                .map(|(authority, _)| Operation::SetAuthority {
                    deck: pane.deck as u8,
                    node: at_node,
                    authority,
                })
        })
    }

    /// **What a press at `p` on a node head's `keep` capsule asks for**, or
    /// `None` off every capsule this pane drew.
    ///
    /// [`InspectorPane::set_authority`]'s walk at the other end of the same
    /// row, and the same order of questions: the body, the group's head, then
    /// the capsule.
    ///
    /// **A head with no capsule is not a target**, which is [`Node::keep`]
    /// being `None` — a head over several nodes, and the built-in camera. Both
    /// fall out here by the derivation answering `None` rather than by a check
    /// of their own, which is the same shape `set_authority` refuses a folded
    /// head in.
    ///
    /// **`id: None`, and the store names the file.** This is the press that
    /// types nothing, so it takes the stamp —
    /// [ADR-0128](../../../../docs/adr/0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md)'s
    /// two routes drawn on one capsule, exactly as [`KeepPill`] draws them for
    /// the deck. The name a head *has* typed is
    /// [`View::named_set`]'s, and the host is what pairs the two: a keep sent
    /// while this pane's head is asking for a name files under what was typed.
    pub fn keep_procedure(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let at_node = node.keep?;
            let group = self.group(&pane.nodes, index);
            let head = Rect::from_min_max(
                group.min,
                Pos2::new(group.max.x, group.min.y + size::NODE_HEAD_H),
            );
            if !head.contains(at) {
                return None;
            }
            node_keep(ctx, head, node)?
                .contains(at)
                .then_some(Operation::KeepProcedure {
                    deck: pane.deck as u8,
                    node: at_node,
                    id: None,
                })
        })
    }

    /// **What a press at `p` on a sensitivity row's chips asks for**, or
    /// `None` off every chip this pane drew and off the two that are readouts.
    ///
    /// [`InspectorPane::set_authority`]'s walk one level in: the body, the
    /// row, then the chips. **A row nothing is holding has no sensitivity row
    /// at all** — [`sens_rect`] answers `None` — so there is no case for a
    /// press on one, which is the mock's own arrangement rather than a check.
    ///
    /// The two readouts answer `None` here rather than being left out of
    /// [`sens_chips`], because they are **drawn** and a press has to be able to
    /// land on them and do nothing: leaving them out would put the chips after
    /// them in the wrong place.
    pub fn sensitivity(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        let deck = pane.deck as u8;
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let group = self.group(&pane.nodes, index);
            node.params.iter().enumerate().find_map(|(at_row, param)| {
                let source = param.bound.as_ref()?;
                let row = sens_rect(group, node, at_row)?;
                if !row.contains(at) {
                    return None;
                }
                sens_chips(ctx, row, source)
                    .find(|(_, chip)| chip.contains(at))
                    .and_then(|(chip, _)| chip.operation(deck, source))
            })
        })
    }

    /// **What a press at `p` takes hold of in this pane**, or `None` where
    /// there is nothing under it a hand can move.
    ///
    /// # It is the knob, and the track is deliberately not a target
    ///
    /// [`Mixer::grab`]'s rule and [`MasterRow::grab`]'s, and it is this bay's
    /// for the same reason read one bay along: a parameter at 0.2 whose track
    /// was clicked would put the value at the far end of its published range,
    /// on stage, because a hand landed three pixels off a knob. The mock draws
    /// a `.fader s` on every `.param` and deliberately draws none on the
    /// transport's exposure track, which is what tells a control with a handle
    /// from one that is set outright — *"a handle that jumped to the pointer
    /// would be a lie about what a handle is"*.
    ///
    /// **The `.param` rows carry no tooltip in the mock**, which is where the
    /// mixer's version of this rule is written down (*"a press on the track
    /// off the knob does nothing, which is every fader in this bay's rule"*),
    /// so the page does not yet say it for this bay. The console's answer is
    /// the mixer's; `docs/manual/console.html` is where it has to be said.
    ///
    /// # A row with nowhere to go is not taken hold of
    ///
    /// [`Param::movable`]: a published range of no width is a control with one
    /// position. The row is drawn — a fill at the start and a figure — and it
    /// is not a handle, which is `Grab::new`'s own refusal read on the value
    /// axis instead of on the track.
    ///
    /// # Only what is drawn, and only where it is drawn
    ///
    /// Two conditions, and they are two because the pane scrolls.
    /// [`drawn`](Self::drawn) is the groups that reach the picture, which is
    /// what a press may land in; **`body.contains` is what keeps a row that
    /// has gone under a head from taking the press anyway**. A group scrolled
    /// off the top still has a rectangle — [`group`](Self::group) answers one
    /// for every index — and that rectangle overlaps the deck head and the
    /// pane head above it, where the paint is clipped away and a knob is
    /// therefore not on screen. Without this check the pane would claim a
    /// press on a knob nobody can see, under a control that is drawn there;
    /// with it, a press outside the body reaches this bay's other derivations
    /// and no other, exactly as `select_renderer` beside it already asked.
    ///
    /// **The clip is the authority and this is the same rectangle**, which is
    /// [`inspector_into`]'s `with_clip_rect(at.body)` asked as a question
    /// rather than applied as a paint.
    pub fn grip<'a>(&self, pane: &'a Pane, p: karakuri_layout::Point) -> Option<ParamGrip<'a>> {
        let p = Pos2::new(p.x, p.y);
        if !self.body.contains(p) {
            return None;
        }
        // The manual's *deck* is the code's *slot*, and a deck holds
        // `MAX_SLOTS` of them — `DECKS`, which is 4 — so the index is a `u8`
        // with room to spare. [`Mixer::grab`]'s note, one bay along.
        let deck = pane.deck as u8;
        self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let group = self.group(&pane.nodes, index);
            node.params
                .iter()
                .enumerate()
                .find_map(|(at, param)| match param.movable() {
                    false => None,
                    true => {
                        let fader = param_fader(param_rect(group, node, at), param)?;
                        fader
                            .knob
                            .contains(p)
                            .then_some(ParamGrip { deck, param, fader })
                    }
                })
        })
    }

    /// **Whether `p` is on a parameter fader's knob**, which is what
    /// [`crate::input::claim`] asks — the knob, and not the track under it.
    pub fn owns(&self, pane: &Pane, p: karakuri_layout::Point) -> bool {
        self.grip(pane, p).is_some()
    }

    /// **What a press at `p` on a parameter row's leftmost cell asks for**:
    /// [`Operation::Publish`](karakuri_operation::Operation::Publish) carrying
    /// the interface this deck would have with that one control's membership
    /// changed — or `None` off every mark.
    ///
    /// # The whole list, because that is what the operation is about
    ///
    /// The vocabulary says it at the variant: *"the whole ordered list, not one
    /// entry. A MIDI control is bound to a position in the published interface,
    /// so adding one entry at a time would renumber every binding after it; and
    /// an interface that publishes nothing publishes everything, which is a
    /// statement about the list and not about an entry."* So this builds the
    /// list the press is asking for — every published row **in interface
    /// order**, less the one pressed, or with it appended where it was not on
    /// the list — and names it. Nothing here says *drop this one*, which is
    /// what keeps two hands on one deck from disagreeing about what is
    /// published
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// **Sorted by [`Param::ord`] and not by where the rows are drawn.** A
    /// wildcard control is *placed* in whichever group it resolves to and is
    /// *numbered* by its position in the interface, and the two orders are not
    /// the same walk — so building the list off the pane's own order would
    /// renumber every knob on a deck with a wildcard in it, on a press that
    /// was about a different row entirely.
    ///
    /// **A row that goes back on lands at the end**, which is a decision and
    /// not an accident: nothing in the pane says where it *was*, the position
    /// it left is now somebody else's, and inventing a place for it would move
    /// knobs nobody pressed anything about. The page says so.
    pub fn publishing(&self, pane: &Pane, p: karakuri_layout::Point) -> Option<Operation> {
        let at = Pos2::new(p.x, p.y);
        if !self.body.contains(at) {
            return None;
        }
        let pressed = self.drawn(&pane.nodes).find_map(|index| {
            let node = pane.nodes.get(index)?;
            let group = self.group(&pane.nodes, index);
            node.params.iter().enumerate().find_map(|(row, param)| {
                ord_cell(param_rect(group, node, row), param)
                    .contains(at)
                    .then_some(param)
            })
        })?;
        let mut kept: Vec<(usize, &Param)> = pane
            .nodes
            .iter()
            .flat_map(|node| node.params.iter())
            .filter(|param| !std::ptr::eq(*param, pressed))
            .filter_map(|param| Some((param.ord?, param)))
            .collect();
        kept.sort_by_key(|(ord, _)| *ord);
        let mut controls: Vec<karakuri_operation::Control> =
            kept.into_iter().map(|(_, param)| param.control()).collect();
        if pressed.ord.is_none() {
            controls.push(pressed.control());
        }
        Some(Operation::Publish {
            deck: pane.deck as u8,
            controls,
        })
    }

    /// **Where one row's publish mark is** — the leftmost cell of the
    /// `row`th parameter row of the `node`th group, or `None` where the pane is
    /// not drawing that group or that row.
    ///
    /// The same rectangle [`InspectorPane::publishing`] resolves a press
    /// against and [`param_into`] paints into, which is this bay's rule
    /// everywhere: the derivation that draws a control is the one that
    /// hit-tests it.
    pub fn publish_mark(&self, pane: &Pane, node: usize, row: usize) -> Option<Rect> {
        if !self.drawn(&pane.nodes).any(|drawn| drawn == node) {
            return None;
        }
        let at = pane.nodes.get(node)?;
        let param = at.params.get(row)?;
        Some(ord_cell(
            param_rect(self.group(&pane.nodes, node), at, row),
            param,
        ))
    }

    /// **Where one node's `index`th `uses` line's control is**, or `None`
    /// where the pane is not drawing that group, that group has no such input,
    /// or the line falls outside the body.
    ///
    /// `open` is whether *this* line's card is down, which is the console's own
    /// state and not the pane's — [`View::wiring_open`], the arrangement
    /// [`Load`] is already in with [`View::target_open`].
    ///
    /// **The capsule is as wide as the name in it**, which is why this asks
    /// `egui` for a galley: a node's name is data and a capsule sized to a
    /// constant would clip one Set's names and not another's.
    pub fn uses_line(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        node: usize,
        index: usize,
        open: bool,
    ) -> Option<UsesLine> {
        if ctx.cumulative_pass_nr() == 0 {
            return None;
        }
        let at = pane.nodes.get(node)?;
        let uses = at.uses.get(index)?;
        if !self.drawn(&pane.nodes).any(|drawn| drawn == node) {
            return None;
        }
        let row = uses_rect(self.group(&pane.nodes, node), index);
        if !self.body.contains_rect(row) {
            return None;
        }
        Some(UsesLine {
            chip: uses_chip_in(ctx, row, uses),
            row,
            rows: match open {
                true => uses.candidates.len(),
                false => 0,
            },
        })
    }

    /// **Which `uses` capsule `p` is on**, as `(node, input)` — or `None` off
    /// every one of them.
    ///
    /// A press here opens a card and emits nothing, which is [`Load`]'s
    /// pulldown exactly: what a pick asks for is the operation, and *open the
    /// list* is not something a map or a model could ever want to say
    /// (ADR-0305).
    pub fn uses_chip(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        p: karakuri_layout::Point,
    ) -> Option<(usize, usize)> {
        self.drawn(&pane.nodes).find_map(|node| {
            let at = pane.nodes.get(node)?;
            (0..at.uses.len())
                .find(|index| {
                    self.uses_line(ctx, pane, node, *index, false)
                        .is_some_and(|line| line.hit_chip(p))
                })
                .map(|index| (node, index))
        })
    }

    /// **What a press at `p` on an open card asks for**:
    /// [`Operation::WireInput`](karakuri_operation::Operation::WireInput)
    /// naming the node the pick landed on — or `None` off every row.
    ///
    /// # It names the node, and the refusal is not here
    ///
    /// The card lists [`Uses::candidates`], which is a reading of the Set
    /// somebody else took, and what leaves this crate is the name that was
    /// picked. **Nothing is validated on this side**: a name the Set cannot use
    /// is refused where the Set is *built*, by name and with what the Set does
    /// hold — the same wall a model's `wire_input` meets, which sends its edge
    /// to the same place with no check of its own
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
    ///
    /// **A pick replaces**, and that is the language's shape rather than this
    /// control's: an input takes one node, `SetError::SlotBoundTwice` refuses
    /// two edges on one input, and a Set with an unbound input does not build
    /// at all (ADR-0152). So there is no *unwire*, and nothing here offers one.
    pub fn wired(
        &self,
        ctx: &egui::Context,
        pane: &Pane,
        viewport: Rect,
        open: (usize, usize),
        p: karakuri_layout::Point,
    ) -> Option<Operation> {
        let (node, index) = open;
        let at = pane.nodes.get(node)?;
        let uses = at.uses.get(index)?;
        let line = self.uses_line(ctx, pane, node, index, true)?;
        let to = uses.candidates.get(line.picked(viewport, p)?)?;
        Some(Operation::WireInput {
            deck: pane.deck as u8,
            node: at.name.clone(),
            slot: uses.slot.clone(),
            to: to.clone(),
        })
    }

    /// **What a press at `p` takes hold of**, as the model holds every other
    /// fader — [`grabbed`], so a parameter fader keeps whatever it grabbed at
    /// and the value does not jump under the hand.
    pub fn grab(&self, pane: &Pane, p: karakuri_layout::Point) -> Option<Grab> {
        let grip = self.grip(pane, p)?;
        grabbed(
            grip.fader,
            Knob::Param {
                deck: grip.deck,
                param: grip.param.param.clone(),
                range: grip.param.range,
            },
            Pos2::new(p.x, p.y),
        )
    }
}

/// **A parameter fader taken hold of**: which deck, which control, and the
/// track the value rides.
///
/// # What it answers, and what it deliberately does not
///
/// The brief on this control is *a pointer landing on a parameter row's fader
/// answers which deck, which parameter and what value*, and those are the
/// three things here: [`deck`](Self::deck), [`param`](Self::param)'s
/// [`Param::param`], and [`Param::valued`] at wherever the drag gets to.
///
/// **It is not a `Grab`, and that is the seam rather than a gap.** A
/// [`crate::panel::Knob`] is what turns a track position into an
/// [`Operation`], and the arm for this control is
/// [`crate::panel::Knob`]'s to grow — see the module the operation is
/// constructed in. What is here is everything the view can answer without it:
/// where the knob is, which is geometry and the value it was drawn from, and
/// which control it belongs to, which is what the harness read off
/// `Set::published`. The one line that closes it reads
///
/// ```ignore
/// grabbed(
///     grip.fader,
///     Knob::Param {
///         deck: grip.deck,
///         param: grip.param.param.clone(),
///         range: grip.param.range,
///     },
///     Pos2::new(p.x, p.y),
/// )
/// ```
///
/// and it is [`grabbed`] — the same inverse of [`fader`] the mixer's two
/// knobs and the master out are taken hold of through, so a parameter fader
/// keeps whatever it grabbed at and the value does not jump.
///
/// # The control it names is the published one, not the group it was drawn in
///
/// [`Param::param`] is `Published::at` and `Published::key` carried over, so a
/// **wildcard** row stays a wildcard: `None` means every node that declares
/// the key, and the engine refuses one that spans nodes under disagreeing
/// authorities (ADR-0223). The row was *placed* in a group by resolving that
/// wildcard where it covered exactly one node, and writing what the placement
/// resolved to would narrow the control to the node it happens to reach today
/// — [ADR-0286](../../../../docs/adr/0286-a-parameter-row-writes-the-control-it-draws-and-carries-the-range-rather-than-the-position.md).
#[derive(Debug, Clone, PartialEq)]
pub struct ParamGrip<'a> {
    /// **Which deck's**, which is the manual's word for what the code calls a
    /// slot — [`Pane::deck`], the deck this pane is pointed at, and not
    /// [`View::selection`].
    pub deck: u8,
    /// **The row a hand landed on**, borrowed from the pane it was drawn from
    /// rather than copied: the address, the range and the value it holds are
    /// all on it already, and a copy of any of them here would be a second
    /// statement about one control.
    pub param: &'a Param,
    /// **The track it took hold of**, at the value the row was drawn at — what
    /// [`grabbed`] measures the grip's offset and travel from.
    pub fader: Fader,
}

/// **How tall everything in a pane comes to**: [`group_h`] over every node,
/// with a [`size::HAIRLINE`] between two of them.
///
/// **One function because two callers must agree.** [`pane_box`] clamps the
/// position in force against it and [`View::scroll_by`] clamps the stored one,
/// and the same sum written twice is two answers to how far a pane scrolls.
fn content_h(nodes: &[Node]) -> f32 {
    let mut total = 0.0;
    for (index, node) in nodes.iter().enumerate() {
        if index > 0 {
            total += size::HAIRLINE;
        }
        total += group_h(node);
    }
    total
}

/// **How tall one node group is**: its head, the renderer row if it has one,
/// and a [`size::PARAM_H`] row per parameter.
///
/// The hairline between two groups is **not** in here and is added by whoever
/// stacks them — `.node-group`'s `border-bottom` is `0` on the last of them,
/// so *n* groups carry *n - 1* rules, which is [`size::PREVIEW_GAP`]'s reading
/// of a gap one axis along, and [`content_h`] is where that sum is taken.
fn group_h(node: &Node) -> f32 {
    size::NODE_HEAD_H
        + uses_h(node)
        + match node.renderers.is_empty() {
            true => 0.0,
            false => size::REND_ROW_H,
        }
        + node.params.iter().map(rows_h).sum::<f32>()
}

/// **How tall a node's declared inputs come to**: one [`size::USES_H`] line
/// each, and nothing at all on the nodes that declare none — which is most of
/// them, and is why this is an addend rather than a row every group carries.
///
/// One function because [`group_h`] sums it and [`param_rect`] and
/// [`uses_rect`] walk past it, which is [`rows_h`]'s own reason one row down.
fn uses_h(node: &Node) -> f32 {
    node.uses.len() as f32 * size::USES_H
}

/// **Where a node group's `index`th `uses` line goes**: under `.node-head`,
/// the full width of the group and [`size::USES_H`] tall.
fn uses_rect(group: Rect, index: usize) -> Rect {
    let top = group.min.y + size::NODE_HEAD_H + index as f32 * size::USES_H;
    Rect::from_min_max(
        Pos2::new(group.min.x, top),
        Pos2::new(group.max.x, top + size::USES_H),
    )
}

/// **Where a `uses` line's capsule is**, inside the line — the one derivation
/// [`InspectorPane::uses_line`] hit-tests and [`uses_into`] paints, which is
/// [`rend_chips`]' arrangement one row down and its reason: the same arithmetic
/// written twice is a capsule drawn where a hand cannot reach it.
///
/// **The name, the gap and the `▾`** — which is drawn rather than typed, so it
/// is a width here and a mark at the paint, exactly as the Outputs row's pill
/// and the Library bay's pulldown already spell it. `.uses`'s `.sep` puts it
/// against the right of the line, which is the node head's arrangement one row
/// up and the deck head's fold two bays over.
fn uses_chip_in(ctx: &egui::Context, row: Rect, uses: &Uses) -> Rect {
    let text_w = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            uses.to.clone(),
            FontId::new(size::USES_SIZE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    });
    let width = size::PILL_PAD_X * 2.0 + text_w + size::SINK_GAP + CHEVRON_W;
    Rect::from_min_size(
        Pos2::new(
            row.max.x - size::PARAM_PAD_R - width,
            row.center().y - size::USES_CHIP_H * 0.5,
        ),
        egui::vec2(width, size::USES_CHIP_H),
    )
}

/// **One `uses` line's control, laid out**: the capsule naming the node that
/// fills the input, and the card of candidates under it.
///
/// # One derivation, and the card hangs *down*
///
/// [`View::draw`] paints these rectangles and [`crate::input::claim`]
/// hit-tests them, which is [`DeckHead`]'s rule and [`Load`]'s. The card hangs
/// **down** from the capsule where the Library bay's hangs up, and the
/// difference is where each control sits: that one is in the *foot* of a bay
/// and this one is inside a pane's body, with the rest of the pane under it.
/// It is held inside the viewport for [`Load::list`]'s reason, so a line near
/// the bottom of a short console draws its card over what is above it rather
/// than off the edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UsesLine {
    /// **The whole line**, the full width of the group — what
    /// [`uses_into`] paints into and what a press has to be inside before any
    /// of it is asked.
    pub row: Rect,
    /// **The capsule naming the node filling this input**, at the right of the
    /// line. A press on it opens the card; a press on it while the card is
    /// down is the host's to read as *shut it*, which is [`Load`]'s
    /// arrangement.
    pub chip: Rect,
    /// **How many candidates the card offers**, which is [`Uses::candidates`]'
    /// length while the card is down and **zero** while it is shut —
    /// [`Load::rows`]' shape and its reason: [`UsesLine::row_at`] cannot hand
    /// out a rectangle for a card nobody opened.
    pub rows: usize,
}

impl UsesLine {
    /// Whether `p` is on the capsule.
    pub fn hit_chip(&self, p: karakuri_layout::Point) -> bool {
        self.chip.contains(Pos2::new(p.x, p.y))
    }

    /// **The card under the capsule, or `None` while it is shut** — and `None`
    /// for a line with no candidate to offer, which is a deck holding one node
    /// of the kind this input takes: the node already wired is left out of its
    /// own list, so there is nothing to pick and no card to open.
    pub fn list(&self, viewport: Rect) -> Option<Rect> {
        if self.rows == 0 {
            return None;
        }
        let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * self.rows as f32;
        let width = self
            .chip
            .width()
            .max(size::LIB_ROW_H + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0);
        Some(held_inside(
            &viewport,
            self.chip.min.x,
            self.chip.max.y + size::PILL_GAP,
            width,
            height,
        ))
    }

    /// **Where the `index`th candidate's row is**, or `None` off the end and
    /// `None` while the card is shut.
    pub fn row_at(&self, viewport: Rect, index: usize) -> Option<Rect> {
        if index >= self.rows {
            return None;
        }
        let list = self.list(viewport)?;
        let top = list.min.y + size::LIB_LIST_PAD + size::LIB_ROW_H * index as f32;
        Some(Rect::from_min_max(
            Pos2::new(list.min.x + size::LIB_LIST_PAD, top),
            Pos2::new(list.max.x - size::LIB_LIST_PAD, top + size::LIB_ROW_H),
        ))
    }

    /// **Which candidate `p` is on**, or `None` off every row.
    pub fn picked(&self, viewport: Rect, p: karakuri_layout::Point) -> Option<usize> {
        let at = Pos2::new(p.x, p.y);
        (0..self.rows).find(|index| {
            self.row_at(viewport, *index)
                .is_some_and(|row| row.contains(at))
        })
    }
}

/// **How tall one parameter row and whatever is under it comes to**: a
/// [`size::PARAM_H`] row, and a [`size::SENS_H`] sensitivity row where
/// something is holding the control.
///
/// **One function because three callers must agree.** [`group_h`] sums it,
/// [`param_rect`] walks it as an offset, and [`sens_rect`] steps off the end of
/// one row — and a group as tall as *n* rows with a press resolved against a
/// stride of *n* is a chip drawn where a hand cannot reach it. That is
/// [`param_rect`]'s own argument about a running sum, one level down.
fn rows_h(param: &Param) -> f32 {
    size::PARAM_H
        + match param.bound {
            None => 0.0,
            Some(_) => size::SENS_H,
        }
}

/// **Where a node group's renderer row is**: `.rend-row` under `.node-head`,
/// the full width of the group and [`size::REND_ROW_H`] tall.
///
/// **One formula, because the row is painted *and* pressed.** [`node_into`]
/// walks a group from the top and [`InspectorPane::select_renderer`] asks
/// where the chips in it are; the same arithmetic written twice is two answers
/// that can disagree, which is [`deck_head`]'s rule one row up — *the
/// derivation that draws a control is the one that hit-tests it*.
///
/// Asked only where [`Node::renderers`] is not empty. On a group that has none
/// the rectangle it answers is where the first parameter row goes, which is
/// [`group_h`]'s own arithmetic read the other way.
fn rend_row_in(group: Rect, node: &Node) -> Rect {
    let top = group.min.y + size::NODE_HEAD_H + uses_h(node);
    Rect::from_min_max(
        Pos2::new(group.min.x, top),
        Pos2::new(group.max.x, top + size::REND_ROW_H),
    )
}

/// **Whether this group's renderer chips are a choice a press can make**, and
/// it is [`Renderer::live`]'s own condition asked of the row rather than of
/// one chip: the deck composites, and it holds more than one renderer.
///
/// The manual's row carries the whole of it — *"Only where the deck composites
/// and holds two or more"* — and [`InspectorPane::select_renderer`] is where
/// the argument for drawing the other two cases and claiming neither is
/// written.
fn a_choice(pane: &Pane, node: &Node) -> bool {
    pane.composite && node.renderers.len() > 1
}

/// **One renderer chip's width**: the name at [`size::BASE`] inside
/// [`size::REND_PAD_X`] either side, which is what `.rend` is as wide as. Its
/// `border: 1px solid var(--c-line)` is counted in [`size::REND_H`] down the
/// chip and not across it, exactly as `.mini`'s is in [`deck_head`].
///
/// Asked of `egui` rather than derived, for [`library::chip_width`]'s reason one bay
/// along: a capsule is as wide as the word in it, and the only thing that
/// knows how wide a word is is the thing that will paint it.
fn rend_width(ctx: &egui::Context, name: &str) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            name.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    }) + size::REND_PAD_X * 2.0
}

/// **Every renderer chip and its box**, left to right in draw order — the same
/// walk [`rend_row_into`] paints and [`InspectorPane::select_renderer`]
/// hit-tests, so the capsule a press lands on is the capsule the wash is drawn
/// in.
///
/// [`LibraryBay::chips`]' arrangement one bay along, down to the reason it is
/// an iterator: a `Vec` of rectangles would be an allocation on a path asked
/// once per pointer event and once per frame.
///
/// **The index is the renderer's own, in draw order** — the numbering
/// `Operation::SelectRenderer`, `--param L4:1:…` and a `select` record all
/// use, so what comes out of a press is a position in the Set rather than a
/// position in whatever this row managed to draw.
///
/// **The boxes are not clipped and the paint is.** `.rend-row` wraps in the
/// mock and this console draws one row of it (see [`rend_row_into`]), so a
/// chip past the pane's right edge is yielded whole here and held to the part
/// of it that is drawn where the press is answered.
pub fn rend_chips<'a>(
    ctx: &'a egui::Context,
    row: Rect,
    renderers: &'a [Renderer],
) -> impl Iterator<Item = (usize, Rect)> + 'a {
    let mut x = row.min.x + size::REND_ROW_PAD_L;
    // One padding down from the top of the row, which is where `.rend-row`
    // puts it: its padding is `3px 10px 6px 12px`, so a chip is not centred in
    // the row and the space under it is twice the space over it.
    let top = row.min.y + size::REND_ROW_PAD_T;
    renderers.iter().enumerate().map(move |(index, rend)| {
        let chip = Rect::from_min_size(
            Pos2::new(x, top),
            egui::vec2(rend_width(ctx, &rend.name), size::REND_H),
        );
        x += chip.width() + size::REND_GAP;
        (index, chip)
    })
}

/// **The word in a sensitivity row's left-hand track**, `.sens`'s first
/// column.
pub const SENS_LABEL: &str = "sensitivity";

/// **The word on a sensitivity row's last chip**, which is the second rule's
/// other half.
pub const TAKE_BACK: &str = "take back";

/// **One chip on a sensitivity row**, and which of the four a press landed on.
///
/// # Two are controls and two are readouts, and that is the decision
///
/// The mock draws four pills and this crate claims two of them, which is
/// [`InspectorPane::select_renderer`]'s arrangement on an inert renderer row:
/// *a control claims what it acts on and no more*.
///
/// - **[`SensChip::Signal`] is a readout.** The signal bus is **open by
///   design** — `SignalBus::sample` cannot fail and a name nobody provides
///   comes back at confidence 0.0 — so there is no list of sources anywhere
///   for a chooser to be built over, and a console inventing one would be the
///   surface deciding what may be asked for, which is
///   [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
///   exactly inverted. A source is named where a source can be named: a Set
///   file's `bind` line, or `--bind`.
/// - **[`SensChip::Curve`] is the control**, and it is the blend chip's shape
///   one bay along: four destinations, a cycle drawn over them here, and
///   `Operation::AttachSignal` naming the one it arrives at. It re-attaches
///   the same signal over the same range through the next shape.
/// - **[`SensChip::Range`] is a readout**, because a range is the procedure's
///   declaration and not an operator's to write —
///   `karakuri_operation::ParamValue`'s own sentence — and there is no second
///   number on this row for a confidence either: a value arrives with how well
///   it is known.
/// - **[`SensChip::TakeBack`] is the other control**, and it is the row this
///   whole arrangement exists for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensChip {
    Signal,
    Curve,
    Range,
    TakeBack,
}

impl SensChip {
    /// The four, left to right, in the order the mock draws them.
    pub const ALL: [SensChip; 4] = [
        SensChip::Signal,
        SensChip::Curve,
        SensChip::Range,
        SensChip::TakeBack,
    ];

    /// **What this chip reads**, off the attachment the row was drawn from.
    ///
    /// The range is written to two places, which is `.pval`'s figure and the
    /// mock's own `0.10 – 2.40`; the en dash is the mock's `&ndash;`.
    pub fn text(self, source: &Source) -> String {
        match self {
            SensChip::Signal => source.signal.clone(),
            SensChip::Curve => source.curve.name().to_string(),
            SensChip::Range => {
                format!("{:.2} \u{2013} {:.2}", source.range[0], source.range[1])
            }
            SensChip::TakeBack => TAKE_BACK.to_string(),
        }
    }

    /// **What a press on this chip asks for**, or `None` for the two that are
    /// readouts — see the type's own note for why those two are not controls.
    ///
    /// **The curve chip restates the attachment.** An `AttachSignal` carries
    /// the whole of what an attachment is, so changing one field means sending
    /// the other three back unchanged — the source and the range come off
    /// [`Source`] rather than being rebuilt, which is what stops a press for a
    /// different shape from silently re-mapping the signal.
    pub fn operation(self, deck: u8, source: &Source) -> Option<Operation> {
        match self {
            SensChip::Signal | SensChip::Range => None,
            SensChip::Curve => Some(Operation::AttachSignal {
                deck,
                param: source.at.clone(),
                signal: source.signal.clone(),
                curve: next_curve(source.curve),
                range: source.range,
            }),
            SensChip::TakeBack => Some(Operation::TakeParamBack {
                deck,
                param: source.at.clone(),
            }),
        }
    }
}

/// **The next of the four shapes**, wrapping — the cycle the curve chip is,
/// and it lives here rather than in the vocabulary for
/// [`karakuri_operation::Curve::ALL`]'s stated reason: the list is the
/// vocabulary's and the cycle is the surface's.
fn next_curve(curve: karakuri_operation::Curve) -> karakuri_operation::Curve {
    let all = karakuri_operation::Curve::ALL;
    let at = all.iter().position(|c| *c == curve).unwrap_or(0);
    all[(at + 1) % all.len()]
}

/// **Where each chip of a sensitivity row goes**, laid end to end from the
/// row's left-hand track.
///
/// [`rend_chips`]' shape one row down and for its reason: the row is painted
/// *and* pressed, and two copies of where a chip is would be a chip painted
/// where a hand cannot reach it. A chip is as wide as the word in it, so this
/// costs a galley lookup per chip.
pub fn sens_chips<'a>(
    ctx: &'a egui::Context,
    row: Rect,
    source: &'a Source,
) -> impl Iterator<Item = (SensChip, Rect)> + 'a {
    // `.sens`'s two tracks: the word, then the chips, with the grid's own gap
    // between them.
    let mut x = row.min.x + size::SENS_PAD_L + size::SENS_LABEL_W + size::SENS_GAP;
    // One padding down from the top of the row, which is where `.sens` puts
    // it: `padding: 2px 10px 6px 12px`, so a chip is not centred and the space
    // under it is three times the space over it.
    let top = row.min.y + size::SENS_PAD_T;
    SensChip::ALL.into_iter().map(move |chip| {
        let w = sens_width(ctx, &chip.text(source));
        let rect = Rect::from_min_size(Pos2::new(x, top), egui::vec2(w, size::SENS_CHIP_H));
        x += w + size::SENS_CHIP_GAP;
        (chip, rect)
    })
}

/// One sensitivity chip's width: the word at [`size::SENS_SIZE`] inside
/// `.pill`'s padding.
fn sens_width(ctx: &egui::Context, text: &str) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            text.to_owned(),
            FontId::new(size::SENS_SIZE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    }) + size::SENS_CHIP_PAD_X * 2.0
}

/// **One pane of the Inspector, derived** — see [`InspectorPane`] for what is
/// drawn here and for the nine things in the mock's pane that are not.
///
/// `index` is which pane, into [`PANE_NAMES`]. `scroll` is the position that
/// pane is scrolled to — [`View::scroll_in`], the console's own state and not
/// a reading — and it is **taken here rather than applied by the painter**,
/// because a control is hit-tested off the derivation that draws it and an
/// offset added on one side of that seam and not the other is two answers
/// about where a knob is. It arrives unclamped and leaves clamped:
/// [`InspectorPane::scroll`] is what is in force, and nothing is written back
/// (P-0082). `layout` must be solved:
/// [`Layout::rect`](karakuri_layout::Layout::rect) refuses to answer from a
/// dirty one. Like [`library`] this asks `egui` for nothing: every box in the
/// pane is either the full width of the pane or a track of the mock's own
/// grid, so no rectangle here is the width of the type in it.
///
/// `None` where there is no pane by that name, and `None` where there is no
/// room for the two heads — which is [`picture_rect`]'s rule stated on a pane.
/// A console with no deck behind it hands over no panes at all, and what the
/// bay draws then is its card, its head and the bar between the two panes,
/// exactly as it did before this pass — [`mixer`]'s rule, one bay along.
///
/// **The bay head is taken off the top here and not in [`pane_box`]**, which
/// is what [`mixer::strips_row`] and [`library::library_box`] do one bay along: the head is
/// painted *over* the region rather than laid out beside it, so every body in
/// this file starts at `region.min.y + size::HEAD_H` and the arithmetic under
/// it is written as if the head were not there. A pane is the one body in the
/// arrangement whose region is not the bay's own — `inspector-1` is a child of
/// the split — and that is what hid this: the pane is the full height of the
/// bay, head included, so a `.half-head` drawn at `region.min` lands on top of
/// the word `Inspector`.
pub fn inspector(
    layout: &karakuri_layout::Layout,
    index: usize,
    pane: &Pane,
    scroll: f32,
) -> Option<InspectorPane> {
    let region = to_egui(layout.rect(layout.find(PANE_NAMES.get(index)?)?));
    let under_head = Rect::from_min_max(
        Pos2::new(region.min.x, region.min.y + size::HEAD_H),
        region.max,
    );
    pane_box(under_head, &pane.nodes, scroll)
}

/// The arithmetic of a pane, away from the layout it reads.
///
/// Term for term from `style.css`:
///
/// - `.half-head { padding: 5px 10px; border-bottom: 1px solid var(--c-hair) }`
///   — a [`size::HALF_HEAD_H`] row along the top of the pane, its rule the
///   bottom pixel of it.
/// - `.deck-head { padding: 5px 10px }` — a [`size::DECK_HEAD_H`] row under
///   it, with no rule of its own: *"Its box is `.node-head`'s without the
///   tint"*, and the tint is what separates it from the group below.
/// - what is left is the node groups', stacked from the top.
///
/// **The leftover is the last group's and not the pane's**, which is the
/// opposite of what [`library::library_box`] does with its foot, and the reason is the
/// same read the other way: the mock's pane is a flow with nothing under the
/// groups at all, so there is no row for a leftover to sit under. It shows as
/// the bay's own card below the last group, which is every other empty body in
/// this pass.
fn pane_box(region: Rect, nodes: &[Node], scroll: f32) -> Option<InspectorPane> {
    let head = Rect::from_min_max(
        region.min,
        Pos2::new(region.max.x, region.min.y + size::HALF_HEAD_H),
    );
    let deck_head = Rect::from_min_max(
        Pos2::new(region.min.x, head.max.y),
        Pos2::new(region.max.x, head.max.y + size::DECK_HEAD_H),
    );
    let body = Rect::from_min_max(Pos2::new(region.min.x, deck_head.max.y), region.max);
    // **Narrower than a parameter row's own padding is no pane**, which is
    // [`library::library_box`]'s width check with the mock's own indent in it. There is
    // no matching check down the pane: a pane too short for a group draws its
    // two heads and no group, which is what `shown` answers.
    if body.width() <= size::PARAM_PAD_L + size::PARAM_PAD_R {
        return None;
    }
    // **And a pane too short for its two heads is no pane**, which is what
    // this function's caller promises. `positive` is not enough on its own:
    // the two heads are stated heights, so they stay positive while running
    // off the bottom of a region shorter than their sum.
    if !positive(head) || !positive(deck_head) || deck_head.max.y > region.max.y {
        return None;
    }
    // How tall the whole stack is, from the one function `View::scroll_by`
    // clamps the stored position against as well.
    let content = content_h(nodes);
    // **The clamp is here and the store is not touched.** `max(0.0)` is what
    // a pane taller than its content answers — there is nothing to scroll
    // through, so the position in force is the top whatever an operator once
    // spun the wheel to, and the position they spun to is still where they
    // left it when the pane comes back (P-0082, ADR-0250).
    let scroll = scroll.clamp(0.0, (content - body.height()).max(0.0));
    // **How many are whole**, which is the readout's number and not the walk's
    // — `InspectorPane::drawn` is the walk. A group is whole when both its
    // edges are inside the body: the top one after the scroll has been taken
    // off, and the bottom one before the body's own.
    let mut shown = 0;
    let mut top = -scroll;
    for (index, node) in nodes.iter().enumerate() {
        let rule = match index {
            0 => 0.0,
            _ => size::HAIRLINE,
        };
        top += rule;
        let bottom = top + group_h(node);
        if top >= -EPSILON && bottom <= body.height() + EPSILON {
            shown += 1;
        }
        top = bottom;
    }
    Some(InspectorPane {
        head,
        deck_head,
        body,
        scroll,
        content,
        shown,
    })
}

/// **What counts as touching an edge**, for [`pane_box`]'s *is this group
/// whole* — a hair either way, because both sides of that comparison are sums
/// of `f32` constants and a group that exactly fills the body would otherwise
/// be counted or not by the last bit of a float.
const EPSILON: f32 = 0.001;

/// **What the anchor reads**, and `None` under free sync.
///
/// The mock's own two spellings: `T128` where a deck is tempo-synced, and
/// `B128 +0.25` where it is beat-synced and sitting a quarter beat ahead of
/// the room. The tempo is whole because the mock writes it whole —
/// `T<b>128</b>` — and the offset carries two places and a sign because the
/// mock's `<em>+0.25</em>` does. **The offset is in beats and the transport
/// row's is in milliseconds**, so neither is ever drawn without knowing which
/// it is; here that is the `B` in front of it.
fn anchor_text(pane: &Pane) -> Option<String> {
    let letter = anchor_letter(pane.sync)?;
    Some(match pane.sync {
        Sync::Beat => format!("{letter}{:.0} {:+.2}", pane.anchor_bpm, pane.scrub_beats),
        _ => format!("{letter}{:.0}", pane.anchor_bpm),
    })
}

/// **How far one press of the scrub moves a deck**, in beats.
///
/// A quarter beat, which is the vocabulary's own figure —
/// [`Operation::ScrubDeck`] writes it at the field (*"How far, in beats. A
/// quarter beat is what a key press asks for"*) and the row it fills is
/// titled *Scrub a deck a quarter beat*. The console page says the same of
/// this control: *"a quarter beat a press, into that same offset"*.
///
/// **Signed at the call site and not here.** The left arrow asks for minus
/// this and the right for plus it, so the amount is one number and the
/// direction is which chip was pressed — see [`DeckHead::scrub`], and the
/// manual's *"the one control here meant to go backwards"*.
pub const SCRUB_BEATS: f64 = 0.25;

/// **What the `re-salt` capsule reads.**
pub const RE_SALT_LABEL: &str = "re-salt";

/// **Where a press on the capacity chip arrives**: the next number up the
/// ladder, and off the top back to the bottom — or `None` where the ladder is
/// empty and there is nothing to ask for.
///
/// # The next one *above* what is running, rather than the next one along
///
/// `candidates` is [`Aimed::capacities`], ascending, and the running value is
/// not necessarily one of them: a procedure may declare `capacity [4096,
/// 1048576] = 81920`, and a Set loaded from a file may be running at whatever
/// that file recorded. Asking for the first candidate *greater than* where the
/// slot is answers both cases in one line — the next power of two from a value
/// that is on the ladder, and the next power of two up from one that is not —
/// where a `position` lookup would have fallen back to the bottom of the range
/// and taken a slot from 81920 to 4096 on a press that reads as *one step*.
///
/// **The wrap goes through the bottom and not through unset**, which is where
/// this differs from the Library's two filter fields (ADR-0262): a filter has a
/// state that is *not narrowed* and a capacity has no such state — every
/// geometry is running at some number — so the end of the ladder is the
/// beginning of it.
fn stepped_capacity(candidates: &[u32], at: u32) -> Option<u32> {
    candidates
        .iter()
        .find(|candidate| **candidate > at)
        .or_else(|| candidates.first())
        .copied()
}

/// **The deck head's controls, laid out**: the chip that names the mode, the
/// anchor that re-asks for it, the two arrows that scrub, the two chips that
/// ask for a different build, and the fold at the right.
///
/// # One derivation, for [`Outputs`]' reason
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles. Two copies of a flex row's running sum
/// is an arrow that lights under a pointer that cannot move the deck.
///
/// # Six rectangles and one type, because they are one row
///
/// [`LookRow`]'s reason one bay over: each one's place is measured from the
/// last, which is what a flex row is, and splitting them into six functions
/// would mean measuring the chip before each of them again to find out where
/// it starts. The fold is in here for the same reason, and it is a control
/// too — see [`DeckHead::composite`], which was the one rectangle here that
/// claimed nothing until 2026-09-09.
///
/// **The two before the fold are measured from the right**, because that is
/// what `.sep`'s `flex: 1` does to everything after it: the fold sits against
/// the row's right-hand padding, the `re-salt` capsule one gap before it and
/// the capacity chip one gap before that, and what says the row fits is that
/// the arrows end before the leftmost of the three begins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeckHead {
    /// **The sync chip**, which is what a press has to land in to move the
    /// mode on. `.mini`'s box round [`sync_word`], at the left of the row.
    pub mode: Rect,
    /// **The anchor**, or `None` under [`Sync::Free`], where there is nothing
    /// to read and nothing to re-anchor — see [`anchor_text`].
    ///
    /// **The text run grown to a chip's height**, which is [`LookRow::grip`]'s
    /// treatment of a 5px track and its argument: `.anchor` is a bare span at
    /// [`size::ANCHOR_SIZE`] with no padding of its own, and 13.5 pixels of
    /// type is not a target a hand finds. It is grown to [`size::MINI_H`], so
    /// it is the same 15.5 as the chips either side of it and sits in the same
    /// [`size::DECK_HEAD_PAD_Y`] the row gives them. **No wider than the
    /// words**, because the row is a flex row and a target that reached past
    /// its own text would take the arrows' places with it.
    pub anchor: Option<Rect>,
    /// **A quarter beat back.** One `.scrub i`.
    pub back: Rect,
    /// **A quarter beat forward.** The other.
    pub forward: Rect,
    /// **The fold at the right of the row**, and the fourth control on it —
    /// [`DeckHead::compositing`] is what a press on it asks for.
    ///
    /// **It was not a control until 2026-09-09**, on the argument
    /// [`Pane::composite`] carries and corrects: layering is a *build*
    /// decision, so a press is a rebuild rather than a write. That is true and
    /// is the reason this works rather than the reason it could not — a
    /// rebuild off the render thread, judged against the budget and rolled
    /// back on its own, is what this instrument does to change what a slot is
    /// running, and the layering is one field of the aim a watcher is pointed
    /// at (ADR-0314).
    ///
    /// It is here because the row is laid out **to** it: it is `.sep`'s
    /// `flex: 1` pushing it against the right-hand padding, and what says the
    /// controls on the left fit is that they end before it.
    pub composite: Rect,
    /// **The capacity chip and the `re-salt` capsule**, or `None` on a deck
    /// with nothing to size and nothing to seed — see [`AimChips`].
    ///
    /// One field for two chips because they are one reading: both are drawn
    /// exactly when [`Pane::aimed`] is, and a state where one of them was there
    /// and the other was not is not a state this row has.
    pub aim: Option<AimChips>,
    /// Which deck this head belongs to, as [`Operation::SetSync`],
    /// [`Operation::ScrubDeck`] and [`Operation::SetCompositing`] each name
    /// one — [`Pane::deck`], carried so that a press answers with the deck it
    /// was measured for.
    pub deck: usize,
    /// **What the fold chip is showing**, and what a press names the other of
    /// — [`Pane::composite`] as it was read, carried for [`DeckHead::locked`]'s
    /// reason one field down: whoever measured this row and whoever acts on a
    /// press in it are one statement, so a chip cannot name a destination
    /// computed from a state some later frame read.
    pub composited: bool,
    /// **What the mode chip is showing**, and what re-anchoring re-asks for.
    /// Carried for [`LookRow::values`]' reason: whoever measured this row and
    /// whoever acts on a press in it are one statement.
    pub locked: Sync,
    /// **What this deck's material can honour**, [`Pane::allows`] as it was
    /// read — the whole of what the cycle skips on.
    pub allows: [bool; SYNCS.len()],
}

/// **The deck head's two build chips, laid out and with what a press on each
/// one asks for** — the capacity the slot's geometries run at, and the salt
/// its randomness comes from.
///
/// # The destinations are carried, for [`DeckHead::composited`]'s reason
///
/// Whoever measured this row and whoever acts on a press in it are one
/// statement. The step is arithmetic over [`Aimed::capacities`] and the salt is
/// a number the host handed in, and both are worked out **once**, on the frame
/// that laid the chips out — so a chip a hand pressed and the operation that
/// leaves this crate cannot be about two different readings of the slot.
///
/// # Neither says *step* and neither says *again*
///
/// What crosses into the vocabulary is
/// [`Operation::SetProperty`](karakuri_operation::Operation::SetProperty)
/// naming a number: `Property::Capacity` carries the element count the step
/// arrived at and `Property::Seed` carries the salt. The affordance —
/// *press it and it moves on* — is the surface's, which is
/// [`DeckHead::sync`]'s division and [`Mixer::blend`]'s
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AimChips {
    /// **The capacity chip**, reading [`Aimed::capacity`]. `.mini`'s box round
    /// the number, one gap left of [`AimChips::salt`].
    pub size: Rect,
    /// **Where a press on [`AimChips::size`] arrives**, or `None` where there
    /// is nothing to step to — see [`stepped_capacity`] and
    /// [`Aimed::capacities`].
    ///
    /// **`None` is drawn and not claimed**, which is `input`'s *a control
    /// claims what it acts on and no more* and the arrangement an inert scrub
    /// is already in: the number is still worth reading on a deck whose
    /// geometries share no range, and a press on it has nothing to ask for.
    pub resize: Option<u32>,
    /// **The `re-salt` capsule**, one gap left of [`DeckHead::composite`].
    pub salt: Rect,
    /// **The salt a press on it asks for** — [`Aimed::salt`], carried.
    ///
    /// There is no state in which this chip is drawn and inert: a slot with a
    /// geometry has randomness to re-seed, and a slot without one draws neither
    /// of these two.
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

    /// **Whether `p` is on the capacity chip *and* the chip has somewhere to
    /// step**, which is [`DeckHead::arrow`]'s arrangement written for a chip:
    /// a deck whose geometries share no declared range draws the number and
    /// claims nothing.
    pub fn hit_size(&self, p: karakuri_layout::Point) -> bool {
        self.aim
            .is_some_and(|aim| aim.resize.is_some() && aim.size.contains(Pos2::new(p.x, p.y)))
    }

    /// **Whether `p` is on the `re-salt` capsule**, which is claimed wherever
    /// it is drawn.
    pub fn hit_salt(&self, p: karakuri_layout::Point) -> bool {
        self.aim
            .is_some_and(|aim| aim.salt.contains(Pos2::new(p.x, p.y)))
    }

    /// **Which arrow `p` is on, as the amount it asks for** — or `None` off
    /// both, and `None` on either while the scrub is inert.
    ///
    /// **Inert is not claimed**, which is [`crate::input`]'s *a control claims
    /// what it acts on and no more*, and it is why this answers for liveness
    /// as well as for position. The arrows are drawn on every deck, because a
    /// scrub is *"an offset added to the room's position under beat sync"* and
    /// a pair of chips that vanished on two modes out of three would move the
    /// rest of the row under the hand every time the chip beside them was
    /// pressed. Drawn and not claimed is the arrangement a fader's track is
    /// already in.
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

    /// **Whether `p` is on any of the six**, which is what
    /// [`crate::input::claim`] asks. The fold is one of them since 2026-09-09,
    /// and it is the only one of the six that is claimed on every deck: a
    /// sync chip is always live, an anchor is not drawn on a free deck, an
    /// arrow is not claimed off beat sync, and the two build chips are drawn
    /// only where the deck has a geometry — where a layering is a state every
    /// slot is in.
    pub fn owns(&self, p: karakuri_layout::Point) -> bool {
        self.hit_mode(p)
            || self.hit_anchor(p)
            || self.arrow(p).is_some()
            || self.hit_size(p)
            || self.hit_salt(p)
            || self.hit_composite(p)
    }

    /// **What a press at `p` asks this deck's clock to become**, or `None`
    /// where there is no sync chip under it.
    ///
    /// # The chip cycles, the operation names where it arrived, and the cycle
    /// skips
    ///
    /// Click it and the deck is asked for the next of [`SYNCS`] its material
    /// can honour — *free*, *tempo*, *beat*, wrapping — and what comes out is
    /// [`Operation::SetSync`] naming the **destination**, never a step,
    /// because there is no step in the vocabulary to name. The affordance is
    /// [`Mixer::blend`]'s and [`LookRow::tonemap`]'s exactly
    /// ([ADR-0187](../../../../docs/adr/0187-the-blend-mini-cycles-and-a-map-learns-the-three-it-cycles-through.md)),
    /// and so is the division it rests on: the cycle is [`next_sync`] here and
    /// nothing at all in `karakuri-operation`, which is P-0090's division: a
    /// toggle is an affordance, built over operations by whoever draws the
    /// control.
    ///
    /// **The skip is the one thing this cycle has that the other two do not**,
    /// and it is not a refusal: what may be asked for is the engine's
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// and this chip is choosing which of the destinations *it* offers to
    /// name, out of a reading somebody else took. A mode this material cannot
    /// honour is passed over rather than handed on to be refused, which is the
    /// mock's own *"skips a mode this material cannot honour instead of
    /// offering it"*.
    ///
    /// **What a map is offered is the three modes, not the cycle** — the
    /// sentence ADR-0187 wrote about three blend modes, and the reason the
    /// anchor beside this chip can be a fourth way to say one of them.
    pub fn sync(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_mode(p).then(|| Operation::SetSync {
            deck: self.deck as u8,
            sync: next_sync(self.locked, self.allows),
        })
    }

    /// **What a press on the anchor asks for**: [`Operation::SetSync`] naming
    /// the mode this deck is **already in**, which re-anchors it — or `None`
    /// off the anchor, and `None` on a free deck, which draws none.
    ///
    /// # It is one operation asked for from two ends, and that is the record
    ///
    /// `karakuri_engine::transport::Transport::engage`'s own documentation is
    /// where this comes from: *"Re-engaging the mode a slot is already in
    /// re-anchors it, which is how an operator says 'call **this** the
    /// reference tempo' without a second control."* `Transport::engaged`
    /// recomputes the anchor from the session tempo every time and clears the
    /// scrub with it, so naming the mode that is running is a real move rather
    /// than a press that does nothing.
    ///
    /// **A cycle structurally cannot ask for it**, which is why this is a
    /// second target on the row rather than a second press on the first:
    /// [`next_sync`] starts at the mode *after* the one the deck is in, so the
    /// one state it can never arrive at is the state it is in. That is a fault
    /// of the affordance and not a gap in the vocabulary, and it is why there
    /// is no `ReAnchor` variant here to name — an operation whose meaning is
    /// *again* is the shape P-0090 rules out
    /// ([ADR-0218](../../../../docs/adr/0218-re-anchoring-is-set-sync-naming-the-mode-the-deck-is-in-and-a-cycle-cannot-say-it.md)).
    ///
    /// **A free deck has no anchor and loses nothing.** `Free` is the absence
    /// of a transport rather than a setting and reads no anchor at all, so
    /// there is nothing for a press to re-ask for — [`anchor_text`] draws
    /// nothing there and this answers `None` off the same `Option`.
    pub fn reanchor(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_anchor(p).then_some(Operation::SetSync {
            deck: self.deck as u8,
            sync: self.locked,
        })
    }

    /// **What a press on an arrow asks for**: [`Operation::ScrubDeck`] moving
    /// this deck a [`SCRUB_BEATS`] back or forward — or `None` off both, and
    /// `None` wherever the scrub is inert.
    ///
    /// # The one control on this panel that moves by an amount
    ///
    /// Every other control here names a destination, which is P-0090's rule,
    /// and the vocabulary says at the variant why this one does not: *"it is
    /// relative because nothing in this instrument can set a position"*.
    /// Scrubbing moves closed-form material by an amount; accumulating
    /// material cannot be moved to a position at all, so an absolute
    /// `at_beat` would be an operation that does not exist for two thirds of
    /// the material. So this is not the exception to P-0090 it looks like —
    /// there is no destination in the language for it to name.
    ///
    /// **Signed and unbounded**, which is the deck head's own spelling: the
    /// offset this writes is drawn in the anchor beside it, in **beats** and
    /// for one deck, where the transport row's offset is in milliseconds and
    /// is the whole instrument's. Nothing clamps it here and there is nothing
    /// to clamp it to.
    ///
    /// **Inert under anything but beat sync**, because that is the only mode
    /// that reads the offset. The arrows keep their shape and the console page
    /// says why rather than greying them out: *"Nothing here re-runs a deck's
    /// history to place it."*
    ///
    /// # One press is one operation
    ///
    /// [ADR-0207](../../../../docs/adr/0207-a-continuous-control-says-one-thing-per-frame.md)
    /// coalesces a swept MIDI fader to one operation a frame; neither half of
    /// it applies to a press, and a second press in the same frame is a second
    /// quarter beat an operator asked for. That is the whole reason the amount
    /// is a constant rather than a distance along anything.
    pub fn scrub(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.arrow(p).map(|beats| Operation::ScrubDeck {
            deck: self.deck as u8,
            beats,
        })
    }

    /// **What a press on the fold asks for**: [`Operation::SetCompositing`]
    /// naming the layering this deck is **not** in — or `None` off the chip.
    ///
    /// # A destination and not a flip, on a chip that reads as a toggle
    ///
    /// [`Mixer::blend`]'s division and [`DeckHead::sync`]'s: the affordance is
    /// *press it and it changes*, and what leaves this crate is the state
    /// being asked for. Nothing in `karakuri-operation` says *toggle*, because
    /// two surfaces stepping one control disagree about where they are
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// and the destination is computed from [`DeckHead::composited`] — the
    /// state the frame that laid this row out drew — so the chip a hand
    /// pressed and the operation that leaves are one statement.
    ///
    /// # The press is a rebuild, and that is the mechanism rather than a cost
    ///
    /// `Set::merge` is written at `Set::build` and nothing moves it
    /// afterwards, which read for a year as *the engine has no setter for
    /// this, so the control is blocked*. What it actually means is that the
    /// control is not a write at all: the layering is one field of the
    /// description a slot's watcher is pointed at, so the window that acts on
    /// this operation restates the rest of that description with this field
    /// changed and sends it, and the worker rebuilds the slot off the render
    /// thread. That build lands at a frame boundary and is judged there and
    /// then, on what one frame of that Set was measured to cost, exactly as an
    /// edited file and a library load are, and rolls itself back if it cannot
    /// hold the frame — which is
    /// the point rather than the price, because compositing costs a
    /// frame-sized target per renderer
    /// ([P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md),
    /// [P-0085](../../../../docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md),
    /// [ADR-0314](../../../../docs/adr/0314-a-control-that-moves-a-field-of-the-aim-re-aims-the-slot-and-the-rebuild-is-the-write.md)).
    ///
    /// **Nothing here knows any of that**, and this crate could not: it names
    /// a destination and a deck, and where the rebuild happens is the window's
    /// (ADR-0156). What it does owe is that the chip goes on reading what
    /// *landed* rather than what was asked for, which is
    /// [`Pane::composite`]'s own note.
    pub fn compositing(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit_composite(p).then_some(Operation::SetCompositing {
            deck: self.deck as u8,
            compositing: !self.composited,
        })
    }

    /// **What a press on the capacity chip asks for**:
    /// [`Operation::SetProperty`] naming the element count the step arrived at
    /// — or `None` off the chip, and `None` on a chip with nowhere to step.
    ///
    /// # A number a hand should not drag, so the control steps
    ///
    /// A capacity is a number in a declared range, which everywhere else on
    /// this console is [`Param`]'s track — and the mock's own reading of a
    /// Set says so, *"capacity included, because a procedure declares one the
    /// same way it declares a knob"*. It is refused here by what a drag **is**:
    /// [`ParamGrip`] turns a pointer into a value every frame it moves, which
    /// for a parameter is a uniform write and for a capacity is a full rebuild
    /// of the slot with every element buffer in it reallocated — dozens of them
    /// across one gesture, which is
    /// [P-0091](../../../../docs/principles/0091-cost-is-known-before-it-is-paid.md)
    /// at its widest. So the affordance is the Library filters' one bay over
    /// ([ADR-0262](../../../../docs/adr/0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md)):
    /// the field steps a closed list and the operation names where it arrived.
    ///
    /// **What it steps is not this console's list.** The powers of two inside a
    /// declared range belong to the material in the slot, so they arrive as
    /// [`Aimed::capacities`] the way `holds` arrives as [`View::holds`], and
    /// what is offered is a reading somebody else took (P-0090).
    ///
    /// # The press is a rebuild, and it is the fold's mechanism exactly
    ///
    /// The capacity is one field of the description this slot's watcher is
    /// pointed at, so the window restates the rest and sends it and the worker
    /// recompiles the slot off the render thread, judged at a frame boundary
    /// against what one frame of that Set was measured to cost — see
    /// [`DeckHead::compositing`], where the argument is written out, and
    /// `docs/adr/0328-…`. **Nothing here knows any of that** and this crate
    /// could not: it names a number and a deck.
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

    /// **What a press on the `re-salt` capsule asks for**:
    /// [`Operation::SetProperty`] naming the salt this slot's randomness is to
    /// come from — or `None` off the capsule.
    ///
    /// # The number is handed in, and that is the whole of the decision
    ///
    /// A salt is the one payload on this row that could plausibly be *made up*,
    /// and a console that made one up would be a surface producing a picture no
    /// later run could produce again
    /// ([P-0092](../../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
    /// [`Aimed::salt`] is the next value of the slot's own deterministic
    /// sequence, derived by whoever read the Set from the salt the slot is
    /// actually running — so this names a destination like every other control
    /// here, the same press twice from the same place lands on the same two
    /// pictures, and a Set kept afterwards records the salts it was running at.
    ///
    /// **Nothing is refused here.** Asking for the salt a slot is already on is
    /// not a state this capsule can produce — the sequence goes forward — and a
    /// re-seed changes the picture, so unlike the fold beside it there is no
    /// press that buys a recompile and moves nothing.
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

/// **The deck head's controls, derived** — the chips in
/// [`InspectorPane::deck_head`], laid along it the way `.deck-head`'s flex row
/// lays them.
///
/// # Measured off a laid-out pane rather than off the layout
///
/// [`inspector`] answers where the pane is and this answers where the chips in
/// its second row are: two questions about one laid-out pane, which is exactly
/// what the mixer bay's four controls are about one laid-out strip. A second
/// derivation from the layout would be a second answer that could disagree
/// with the one the frame drew.
///
/// # What it costs to ask
///
/// **Five galley lookups per pane**: the mode's word, the anchor's two
/// numbers, the capacity's digits, the `re-salt` capsule's word and the fold's.
/// The two arrows cost none — they are marks rather than words, which is
/// [`Mixer::mask`]'s own saving one bay over — and nothing here asks after the
/// node groups below.
///
/// **It is asked twice on a frame**, once here and once for the galleys
/// [`deck_head_into`] paints, which is [`look`]'s honest cost written down one
/// bay along: the derivation that draws a control is the one that hit-tests
/// it, so a control cannot be painted anywhere a press cannot reach. Two panes
/// at three lookups is on the order of ten allocations a frame against the
/// panel pass's measured median of 1518 (`crates/karakuri`'s `WRITTEN_ALLOCS`,
/// taken 2026-08-31), which is inside the factor of two that file quotes a
/// figure across.
///
/// # `None` is a row that cannot hold its own controls
///
/// [`look`]'s rule and [`arrangement`]'s: *a control that does not fit in the
/// row it is drawn in is no control at all, rather than half of one*. The fold
/// is pushed against the right-hand padding and the three controls run from
/// the left, so what says the row fits is that the arrows end before the fold
/// begins. A pane narrow enough to fail that draws its two heads' words and no
/// chips at all, where it used to draw chips cut in half by
/// [`inspector_into`]'s clip rectangle — which is a picture of a control that
/// cannot be pressed
/// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
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
    // **Both or neither**, which is [`DeckHead::aim`]'s own sentence: they come
    // from one reading, so a pane with no [`Pane::aimed`] draws the row it drew
    // before this control existed.
    //
    // **And neither where the row cannot hold them**, which is the one place
    // this row's *a control that does not fit is no control at all* is answered
    // by dropping part of the row rather than all of it. The reason is a
    // measurement: an Inspector pane at the console's declared minimum window
    // is 237 pixels wide and the five chips that were here already come to
    // within a couple of dozen of that, so a row that took all seven or none
    // would answer *none* at the width this arrangement claims to work at —
    // trading two controls that were never there for four that were. So the two
    // build chips are dropped first and the row goes on drawing what it drew
    // before them, and the page says so rather than leaving an operator to
    // discover it by dragging
    // ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
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

/// **The `keep` pill in a pane's head, laid out** — the capsule at the right
/// of `.half-head`, and the one control in this bay that performs rather than
/// sets.
///
/// # It keeps the pane's deck, and `k` keeps the selection
///
/// The mock draws one of these per pane and the tooltip names the pane's own
/// deck: *"Keep deck A as a Set, exactly as it is on screen."* So this carries
/// [`Pane::deck`] the way [`DeckHead::deck`] does, and a press answers with
/// the deck the pill was measured for — a pill in the second pane keeps that
/// pane's deck while the selection stays where the operator put it. The key
/// `k` keeps *the selected deck*, because a bare key press cannot say which,
/// and the two are one operation asked for from two ends
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
///
/// # What it files it under
///
/// [`Operation::SaveSet`] with **no id**, which is the same call the key makes
/// and is a decision rather than an omission
/// ([ADR-0287](../../../../docs/adr/0287-the-keep-pill-files-under-a-stamp-because-the-consoles-one-letter-taking-flow-is-an-arrangements-name.md)).
/// What the store does with a `None` is `karakuri_environment::accepted_save`'s
/// convention — a stamp, because *"an operator looks for the time they saved
/// it"*.
///
/// **The reason has changed and the decision has not.** ADR-0287 argued the
/// `None` from there being one letter-taking flow on this console and it being
/// an arrangement's; there are two now, and the second is the name in the head
/// beside this capsule
/// ([ADR-0292](../../../../docs/adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md)).
/// What holds the capsule at `None` from here on is
/// [ADR-0128](../../../../docs/adr/0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md)
/// rather than the absence of a field: *"an operator's own act gets the name it
/// asked for; a key press cannot type one and takes a stamp"*. This is the
/// press that types nothing, so this is the one that takes the stamp — see
/// [`DeckName`] for the one that does not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeepPill {
    /// **The capsule**, which is what a press has to land in. The mock gives
    /// the whole pill the click and so does this — [`ArrangementPill::pill`]'s
    /// own reading.
    pub pill: Rect,
    /// Which deck this pill keeps, as [`Pane::deck`] — carried so that a press
    /// answers with the deck it was measured for, which is
    /// [`DeckHead::deck`]'s reason one row down.
    pub deck: usize,
}

impl KeepPill {
    /// Whether `p` is on the capsule, which is the whole of what this control
    /// owns: there is no menu under it and no second target beside it.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.pill.contains(Pos2::new(p.x, p.y))
    }

    /// **What a press at `p` asks for**, or `None` off the capsule.
    ///
    /// The same derivation [`crate::input::claim`] hit-tests, asked a second
    /// time rather than copied — [`DeckHead::sync`]'s arrangement, and the
    /// reason is the same one row up: the pill that claims a press and the
    /// pill that acts on it cannot come apart.
    ///
    /// **It refuses nothing.** What a keep costs and whether the store will
    /// take it are the instrument's answers rather than this surface's, and
    /// the operation is *"on a worker"* on the page it is specified on — the
    /// press leaves and the answer arrives later.
    pub fn keep(&self, p: karakuri_layout::Point) -> Option<Operation> {
        self.hit(p).then_some(Operation::SaveSet {
            deck: self.deck as u8,
            // **`None`, and it is the payload saying so rather than this
            // control inventing a stamp** — the sentence
            // `crates/karakuri/src/main.rs` already writes over the `k` arm,
            // and the same one: *"a caller that can type a name is not made to
            // take a timestamp"*, and this control is not one of them.
            id: None,
        })
    }
}

/// **The pane head's pill, derived** — [`inspector`] answers where the head is
/// and this answers where the capsule in it is, which is [`deck_head`]'s
/// division one row down.
///
/// `.sep`'s `flex: 1` puts it hard against the head's right-hand padding, and
/// **one padding down from the top rather than centred in the row**: the rule
/// at the bottom is inside `.half-head`, so the row's middle is half a pixel
/// below the middle of its content box — which is the scope row's own note one
/// bay along, on a row built the same way.
///
/// # What it costs to ask
///
/// **One galley lookup per pane**, for the word in the capsule, on a pointer
/// event and on a frame — [`deck_head`]'s three beside it, and paid the same
/// way.
///
/// # `None` is a head that cannot hold it
///
/// [`deck_head`]'s rule and [`look`]'s: *a control that does not fit in the
/// row it is drawn in is no control at all, rather than half of one*. The
/// words to its left are a readout and are clipped; the pill is a target and
/// is not drawn where it would be cut.
pub fn keep_pill(ctx: &egui::Context, at: &InspectorPane, pane: &Pane) -> Option<KeepPill> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`deck_head`] — and on the frame before the first one there is nothing
    // drawn here to press.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let head = at.head;
    let w = pill_width(ctx, KEEP_LABEL);
    let pill = Rect::from_min_size(
        Pos2::new(
            head.max.x - size::HALF_HEAD_PAD_X - w,
            head.min.y + size::HALF_HEAD_PAD_Y,
        ),
        egui::vec2(w, size::PILL_H),
    );
    // **Measured against `.half-head`'s content box and not against the row**,
    // because a flex item cannot be laid out inside its parent's padding: the
    // capsule is placed from the right-hand padding, so what it runs off is
    // the left one, and a head with less room between its two paddings than
    // the word needs draws none. [`positive`] is what says the head is a row
    // at all — a pane with no height has one that is not.
    let room = head.width() - size::HALF_HEAD_PAD_X * 2.0;
    (positive(head) && pill.width() <= room).then_some(KeepPill {
        pill,
        deck: pane.deck,
    })
}

/// **The word in the capsule**, which is the mock's own and is the row's name
/// in the panel column of [every operation](../../../../docs/manual/operations.html).
const KEEP_LABEL: &str = "keep";

/// **The letter of the deck a pane is pointed at**, or `?` for a pane pointed
/// past the end of [`DECK_LETTERS`] — which is a caller's error and not a
/// state, and is drawn rather than panicked for [`showing_text`]'s reason: a
/// head is a readout and a readout does not stop a frame.
fn deck_letter(pane: &Pane) -> &'static str {
    DECK_LETTERS.get(pane.deck).copied().unwrap_or("?")
}

/// **What the pane head reads**: the mock's `deck A · drift_night`.
fn showing_text(pane: &Pane) -> String {
    format!("deck {} · {}", deck_letter(pane), pane.material)
}

/// **What that same run reads while the head is taking letters** — `deck A ·
/// glass_sh▏`, with [`CARET`] after it as the arrangement's field has.
///
/// **The deck stays and the material goes.** What is being typed is the name
/// this deck's material will be filed under, so the run says which deck is
/// being filed for the whole of the gesture — and the half of it that is
/// replaced is exactly the half a name is. A field that had cleared the run
/// would take the one word that says *whose* name this is off the screen at
/// the moment an operator is looking hardest at it.
fn naming_text_in_head(pane: &Pane, typed: &str) -> String {
    format!("deck {} · {typed}{CARET}", deck_letter(pane))
}

/// The word the mock puts in front of it.
const SHOWING_LABEL: &str = "showing";

/// **The word in front of the run while the head is taking letters**, where
/// [`SHOWING_LABEL`] is the word in front of it the rest of the time.
///
/// The row stops being a readout the moment letters are going into it, and the
/// label is the only thing that can say what they are *for*: they name the Set
/// the capsule at the other end of the same row files. It is [`KEEP_LABEL`]'s
/// own word rather than a new one, which is [`SAVE_ITEM_ASKING`]'s arrangement
/// three bays along — the thing that asks for something says so in the verb it
/// is about to perform.
const NAMING_LABEL: &str = "keep as";

/// **The word this head has in front of its run**, which is the one thing
/// about the row that says whether it is reading or asking.
fn head_label(naming: Option<&str>) -> &'static str {
    match naming {
        Some(_) => NAMING_LABEL,
        None => SHOWING_LABEL,
    }
}

/// **The name in a pane head, laid out** — the mock's `.what`, and this
/// console's **second** letter-taking flow
/// ([ADR-0292](../../../../docs/adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md)).
///
/// # A press on it names the Set, and the capsule beside it goes on stamping
///
/// [ADR-0128](../../../../docs/adr/0128-a-set-saved-under-a-name-the-caller-chose-overwrites.md)
/// is what puts two routes on one row: *"an operator's own act gets the name it
/// asked for; a key press cannot type one and takes a stamp"*. The `keep`
/// capsule is the second of those and is unchanged — [`KeepPill::keep`] emits
/// `id: None` exactly as ADR-0287 decided — and this is the first: a press
/// here puts the head into [`Naming`], and the commit is
/// [`View::named_set`]'s `id: Some(typed)`.
///
/// # What it does **not** claim, and that is the whole of its right-hand edge
///
/// The mock's head is `showing`, the name, `▾`, `.sep`, `keep`. **The `▾` is
/// the chooser** — *point this pane at another deck* — which is
/// [`Pane::deck`]'s per-pane pointer and is still not a control this console
/// has (ADR-0200). It is not drawn, and this derivation reserves
/// [`DeckName::chevron`] for it anyway: the target is the run's own ink and
/// stops there, so the day the chooser lands it takes the rectangle beside the
/// name rather than taking it *back*. A name target that had run to the
/// capsule would have swallowed the chooser's place before anybody drew it,
/// and a press meant for the caret would be a press that re-points the pane.
///
/// # The run is one target and is deliberately not two
///
/// `deck A · drift_night` is one `.what` in the mock and one galley here.
/// Claiming the material and leaving `deck A ·` a readout would be a boundary
/// inside a run of text with nothing on screen drawing it, which is the
/// opposite of *a control claims what it acts on and no more*: what this acts
/// on is the name display, and the name display is the whole run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeckName {
    /// **The run as it is painted**, clipped to what the head has room for —
    /// see [`deck_name`]. A press has to land in this and nowhere else.
    pub name: Rect,
    /// **Where the mock's `▾` goes**, one `.half-head` gap after the run.
    /// Drawn by nobody and claimed by nobody: it is the chooser's place, held
    /// so that this control's edge is a measured thing rather than a comment.
    pub chevron: Rect,
    /// Which deck this head names, as [`Pane::deck`] — carried for
    /// [`KeepPill::deck`]'s reason one capsule along.
    pub deck: usize,
}

impl DeckName {
    /// Whether `p` is on the run, which is the whole of what this control
    /// owns: the label to its left is a readout, and the rectangle to its
    /// right is the chooser's.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.name.contains(Pos2::new(p.x, p.y))
    }
}

/// **What the count at the right of a pane head reads** — `n of m`, the node
/// groups this pane is showing whole out of the ones the deck has.
///
/// The Library foot's `5 of 27` counted on this bay's items rather than on
/// that one's rows, which is what
/// [ADR-0259](../../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md)
/// says a pane's items are: *"Items are its panes; a pane's controls are its
/// deck head and its node groups"*. A percentage would be a number about a
/// rectangle, and what an operator counts is groups.
pub fn count_text(at: &InspectorPane, pane: &Pane) -> String {
    format!("{} of {}", at.shown, pane.nodes.len())
}

/// **The count in a pane head, derived** — where the run goes, or `None` for a
/// head with no room for it between the label and the capsule.
///
/// It is a **readout**: nothing hit-tests it, it names no operation, and it
/// carries no row on [every operation](../../../../docs/manual/operations.html) —
/// the mixer head's `3 of 3 · page 1` one bay along, and the reason is the
/// same one that keeps the Library's cursor off that page. What it is *for* is
/// rule 04 — *"A list that showed you part of itself says so and says how
/// much"* — which is the whole of what a scrolled pane owes a reader, and is
/// why this is derived beside the two controls in the row rather than painted
/// wherever there happened to be space.
///
/// # Where it sits, and what gives way to what
///
/// `.half-head` is a flex row: the label and the run are at the left, `.sep`
/// takes what is over, and the capsule is hard against the right-hand padding.
/// This goes one [`size::HALF_HEAD_GAP`] to the left of the capsule — the
/// mixer head's order, where the readout is left of the pill — and the run to
/// its left is what gives way when the pane is narrowed, because the run is
/// the one thing in the row that is clipped rather than dropped.
///
/// **`None` is a head that cannot hold it**, which is [`keep_pill`]'s rule
/// read on a readout: measured against the room between the label and the
/// capsule, so a head that would have to draw this over the words draws none
/// of it. A pane at the declared minimum of 208 has room for all three
/// ([ADR-0279](../../../../docs/adr/0279-the-centre-is-two-parameter-rows-wide-because-a-pane-that-cannot-draw-a-fader-is-not-a-minimum.md)),
/// so this `None` is a pane below what the arrangement admits rather than a
/// state rule 04 is broken in.
pub fn pane_count(
    ctx: &egui::Context,
    at: &InspectorPane,
    pane: &Pane,
    naming: Option<&str>,
) -> Option<Rect> {
    // Fonts are not valid until `egui` has run a pass — [`keep_pill`]'s guard,
    // and before the first one there is no head painted to read.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let head = at.head;
    if !positive(head) {
        return None;
    }
    let run = |text: &str| {
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
    let right = match keep_pill(ctx, at, pane) {
        Some(pill) => pill.pill.min.x - size::HALF_HEAD_GAP,
        None => head.max.x - size::HALF_HEAD_PAD_X,
    };
    // Where the run beside it would start: the label inside the left padding,
    // and one gap. This is measured against that rather than against the
    // head's edge so that a head narrow enough to want the room for its words
    // keeps it — the words are what says *which deck*, and a count of groups
    // on a pane whose deck has gone unnamed is a number about nothing.
    let left = head.min.x + size::HALF_HEAD_PAD_X + run(head_label(naming)) + size::HALF_HEAD_GAP;
    let w = run(&count_text(at, pane));
    let top = head.min.y + size::HALF_HEAD_PAD_Y;
    (right - w >= left).then(|| {
        Rect::from_min_max(
            Pos2::new(right - w, top),
            Pos2::new(right, top + size::PILL_H),
        )
    })
}

/// **The pane head's name, derived** — [`inspector`] answers where the head is
/// and this answers where the run in it is, which is [`keep_pill`]'s division
/// along the same row.
///
/// `naming` is what the head is taking letters into, or `None` for a head that
/// is reading — and it is a parameter rather than a field of [`Pane`] because
/// a pane is rewritten whenever a Set lands ([`View::inspector`]) and a buffer
/// kept there would be a name that vanished mid-word. It lives in
/// [`View::naming_set`], which is [`Arrangement::menu`]'s argument on a second
/// control: what a *control* is doing is this crate's, and it is not part of
/// anything a host hands in.
///
/// # What it costs to ask
///
/// **Three galley lookups per pane** — the label, the run, and [`keep_pill`]'s
/// word, because where the run may be painted to is where the capsule starts.
/// The capsule is derived here rather than passed in for [`crate::input`]'s
/// own reason one bay along, where [`on_pill`](crate::input) derives the
/// tracker group as well: one derivation asked twice cannot come apart, and
/// two arguments that a caller could fill from two frames can.
///
/// # `None` is a head with no ink to press
///
/// [`keep_pill`]'s rule read on a readout instead of on a capsule. The run is
/// clipped where the words are clipped — one `.half-head` gap short of the
/// capsule — so a head narrow enough that the label alone fills it leaves no
/// name on screen, and a target over ink nobody can see is a press that lands
/// on nothing an operator could have aimed at.
pub fn deck_name(
    ctx: &egui::Context,
    at: &InspectorPane,
    pane: &Pane,
    naming: Option<&str>,
) -> Option<DeckName> {
    // Fonts are not valid until `egui` has run a pass, exactly as in
    // [`keep_pill`] — and on the frame before the first one there is nothing
    // drawn here to press.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let head = at.head;
    if !positive(head) {
        return None;
    }
    let run = |text: &str| {
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
    // **The same clip [`inspector_into`] paints the words inside**, written
    // once here and read there: everything up to whatever is next along the
    // row, one `.half-head` gap short of it. That is the count where the head
    // has room for one ([`pane_count`]), the capsule where it has not, and the
    // head's own edge where it has neither.
    let limit = match pane_count(ctx, at, pane, naming) {
        Some(count) => count.min.x - size::HALF_HEAD_GAP,
        None => match keep_pill(ctx, at, pane) {
            Some(pill) => pill.pill.min.x - size::HALF_HEAD_GAP,
            None => head.max.x,
        },
    };
    // **And the chooser's own room comes off it**, which is the half of
    // ADR-0292's *the chooser is boxed in* that the chooser landing makes
    // real: `.half-head` is `showing`, the run, `▾`, `.sep`, the count and the
    // capsule, so the `▾` sits **between** the run and everything else in the
    // row. The run is the one thing here that is clipped rather than dropped
    // (`pane_count`'s own note), so it is the run that gives way and never the
    // control. Before this the chevron was reserved and unpainted, and its
    // rectangle could sit on top of the count in a narrow head — which cost
    // nothing while nobody drew it and would be a target over another
    // control's ink now that somebody does.
    let limit = limit - (CHEVRON_W + size::HALF_HEAD_GAP);
    let left = head.min.x + size::HALF_HEAD_PAD_X + run(head_label(naming)) + size::HALF_HEAD_GAP;
    let text = match naming {
        Some(typed) => naming_text_in_head(pane, typed),
        None => showing_text(pane),
    };
    let right = (left + run(&text)).min(limit);
    if right <= left {
        return None;
    }
    let top = head.min.y + size::HALF_HEAD_PAD_Y;
    let name = Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, top + size::PILL_H));
    Some(DeckName {
        // **The chooser**, one gap after the run and at the glyph's own
        // measure — [`CHEVRON_W`], which is the arrangement pill's `▾` three
        // bays along. It was reserved and drawn by nobody until 2026-09-10
        // (ADR-0292's *the chooser is boxed in*), and it is a control now:
        // [`pane_target`] is what paints and hit-tests it, off this
        // rectangle. What has not changed is that [`DeckName::name`] stops
        // before it — the run is one target and the mark beside it is
        // another.
        chevron: Rect::from_min_size(
            Pos2::new(
                name.max.x + size::HALF_HEAD_GAP,
                name.center().y - CHEVRON_H * 0.5,
            ),
            egui::vec2(CHEVRON_W, CHEVRON_H),
        ),
        name,
        deck: pane.deck,
    })
}

/// **The pulldown on a pane head, and the card it brings down** — *point this
/// pane at another deck*.
///
/// # It is the pane's own pointer and it is not the deck selection
///
/// A pick moves this pane and nothing else: not the deck the keys are
/// addressed to ([`View::selection`]), not the pane next door, and not the
/// Library bay's load target ([`View::target_deck`]). That is the whole of why
/// the mark exists — a pane can show a deck the keys are **not** on — and it
/// is [`Load`]'s argument one bay along
/// (`docs/adr/0305-…`, `docs/adr/0338-…`, decision 5).
///
/// # A pulldown and not a flip
///
/// The maintainer's choice, and
/// [P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// underneath it: a flip is a *step*, so two panes stepping cannot both be
/// aimed without knowing where they started, and a key, a map line or a model
/// would have to count presses to say *deck C*. Every row of this card names a
/// destination.
///
/// # What it offers is what the mixer is drawing
///
/// [`Target::decks`]' count read a second time and not a second rule: a deck
/// the mixer draws no strip for is not in the list, which is
/// [`View::select`]'s own refusal met from one more direction.
///
/// # The card hangs down, as the `uses` line's does
///
/// It is inside a pane's body's own bay rather than in a foot, so what is
/// under the head is the pane — [`UsesLine::list`]'s division, and it is held
/// inside the viewport for that method's reason.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaneTarget {
    /// **The `▾` after the run** — [`DeckName::chevron`], made live. A press on
    /// it puts the card down; a press on it while the card is down is the
    /// host's to read as *shut it*, which is [`Load`]'s arrangement.
    pub chevron: Rect,
    /// **Which pane this head belongs to**, as an index into [`PANE_NAMES`] —
    /// what [`Operation::PointPane`]'s `pane` is spelled from, and what says
    /// which of [`View::pane_deck`]'s entries a pick moves.
    pub pane: usize,
    /// **How many decks the card offers**, which is how many strips the mixer
    /// is drawing while it is down and **zero** while it is shut —
    /// [`Load::rows`]' shape and its reason: [`PaneTarget::row`] cannot hand
    /// out a rectangle for a card nobody opened.
    pub rows: usize,
}

impl PaneTarget {
    /// Whether `p` is on the mark, which is the whole of what the shut control
    /// owns: the run to its left is [`DeckName`]'s and the count to its right
    /// is a readout.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.chevron.contains(Pos2::new(p.x, p.y))
    }

    /// **The card under the mark, or `None` while it is shut** — and `None`
    /// for a console with no strip to offer, which is every test in this crate
    /// that hands no mixer in.
    pub fn list(&self, viewport: Rect) -> Option<Rect> {
        if self.rows == 0 {
            return None;
        }
        let height = size::LIB_LIST_PAD * 2.0 + size::LIB_ROW_H * self.rows as f32;
        let width = size::LIB_ROW_H + (size::LIB_ROW_PAD_X + size::LIB_LIST_PAD) * 2.0;
        Some(held_inside(
            &viewport,
            self.chevron.min.x,
            self.chevron.max.y + size::PILL_GAP,
            width,
            height,
        ))
    }

    /// **Where the `index`th deck's row is**, from the top of `card` — the
    /// decks in [`DECK_LETTERS`] order, which is [`Load::row`]'s own reading.
    ///
    /// Panics on a row this card has not got, which is that method's rule: a
    /// caller has invented a deck.
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

    /// **What a press at `p` on the card asks for**, or `None` off every row.
    ///
    /// The pane is named by [`PANE_NAMES`], which is the arrangement's own
    /// handle for it — `karakuri-operation` has no dependencies and cannot
    /// hold one, which is [`Operation::FoldPane`]'s spelling and its reason.
    pub fn picked(&self, viewport: Rect, p: karakuri_layout::Point) -> Option<Operation> {
        let card = self.list(viewport)?;
        let at = Pos2::new(p.x, p.y);
        let deck = (0..self.rows).find(|index| self.row(card, *index).contains(at))?;
        Some(Operation::PointPane {
            pane: PANE_NAMES.get(self.pane)?.to_string(),
            deck: deck as u8,
        })
    }
}

/// **The pulldown on one pane head, derived** — [`deck_name`] answers where
/// the run is and this answers where the mark after it is, which is
/// [`keep_pill`]'s division along the same row.
///
/// `None` is a head with no run drawn in it, which is [`deck_name`]'s own
/// refusal: the mark sits one gap after the run, so a head too narrow to paint
/// any of the name has nowhere to put it. The run is clipped short of this
/// mark rather than over it — see [`deck_name`], where that is one line.
pub fn pane_target(
    ctx: &egui::Context,
    at: &InspectorPane,
    pane: &Pane,
    index: usize,
    naming: Option<&str>,
    decks: usize,
    open: bool,
) -> Option<PaneTarget> {
    let named = deck_name(ctx, at, pane, naming)?;
    Some(PaneTarget {
        chevron: named.chevron,
        pane: index,
        // **Zero while it is shut**, which is what stops [`PaneTarget::row`]
        // handing out a rectangle for a card nobody opened — [`Load`]'s own
        // field.
        rows: match open {
            true => decks.min(DECK_LETTERS.len()),
            false => 0,
        },
    })
}

/// **A pane head taking letters**, and the whole of the console's second
/// letter-taking flow's state.
///
/// # One at a time, and it carries which head it is in
///
/// [`Menu::Naming`] is the first flow and it is one because a menu is one; this
/// is one because **the keyboard is one**. Whoever holds the keys takes them
/// whole while a name is being asked for — `s` is an `s` in a name and not a
/// solo — so two open fields would be two places one keystroke could go, with
/// nothing on the panel saying which. So this is an `Option` on the console and
/// not a field per pane, and it names the pane the field is drawn in.
///
/// # The buffer is a `String` this crate owns and does not check
///
/// [`Menu::Naming`]'s rule, unchanged and for its reason: a name that is not
/// one path component is refused where the file is written, in one sentence, by
/// whoever writes it — the surface owns the affordance and never the authority
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)).
/// A head that quietly dropped the characters it did not like would be a rule
/// an operator could only find by experiment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Naming {
    /// **Which pane head the field is in**, as an index into
    /// [`View::inspector`] and so into [`PANE_NAMES`].
    pub pane: usize,
    typed: String,
}

impl Naming {
    /// **What has been typed so far.** The caret is drawn after it and there
    /// is no selection: this is a name, not a document —
    /// [`Arrangement::naming`]'s own sentence.
    pub fn typed(&self) -> &str {
        &self.typed
    }
}

impl View {
    /// **Which pane head is taking letters, and what is in it** — or `None`
    /// for a console where nothing is being named, which is every test in this
    /// crate that does not say otherwise.
    pub fn naming_set(&self) -> Option<&Naming> {
        self.naming.as_ref()
    }

    /// **What pane `index`'s head is taking letters into**, or `None` where it
    /// is reading. This is what [`deck_name`] and [`inspector_into`] each ask,
    /// so that one head is asking and the other is not.
    pub fn naming_set_in(&self, index: usize) -> Option<&str> {
        self.naming
            .as_ref()
            .filter(|naming| naming.pane == index)
            .map(Naming::typed)
    }

    /// **What is being typed into the head of whichever pane is showing
    /// `deck`**, or `None` where no head is asking for a name over that deck.
    ///
    /// **What it is for is a node's keep** (ADR-0338, decision 4): the capsule
    /// on a node group's head types nothing and takes a stamp, and a head that
    /// *is* taking letters is what a keep from that pane files under — which
    /// is ADR-0128's two routes drawn on one capsule, exactly as the deck's
    /// own `keep` draws them.
    ///
    /// **The pane is found by the deck rather than carried**, which is
    /// [`View::named_set`]'s own rule: the gesture spans frames, so what is
    /// filed is what the head says it is filing *now*. An empty buffer answers
    /// `Some("")`, and that is the field's rule and not this method's — the
    /// console emits what was typed, including nothing, and the wall is where
    /// the file is written (P-0090).
    pub fn naming_over(&self, deck: u8) -> Option<String> {
        let naming = self.naming.as_ref()?;
        let pane = self.inspector.get(naming.pane)?;
        (pane.deck == usize::from(deck)).then(|| naming.typed().to_owned())
    }

    /// **Ask for a name in pane `index`'s head**, starting from empty.
    ///
    /// **Starting from empty rather than from the material's name.** The run
    /// under the caret read `drift_night` a moment ago and the field does not
    /// keep it: a buffer seeded with what was there is a name an operator
    /// commits by pressing return once, which is the shape of an overwrite
    /// nobody typed. What ADR-0128 makes an instruction is a name that was
    /// *typed*.
    pub fn name_set(&mut self, index: usize) {
        self.naming = Some(Naming {
            pane: index,
            typed: String::new(),
        });
    }

    /// **Take the field away, typed name and all**, which is what escape
    /// asks and what a press somewhere else asks. [`Arrangement::shut`]'s
    /// sentence: a name abandoned half-typed is not kept for the next time,
    /// because the buffer is the gesture and the gesture ended.
    pub fn stop_naming_set(&mut self) {
        self.naming = None;
    }

    /// **One character into the name being typed**, and `false` where no head
    /// was asking for one. [`Arrangement::typed`]'s rule and its refusal:
    /// control characters are not a name and never reach the buffer, because a
    /// newline is Return arriving as text and that is the commit.
    pub fn type_into_name(&mut self, c: char) -> bool {
        match (&mut self.naming, c.is_control()) {
            (Some(naming), false) => {
                naming.typed.push(c);
                true
            }
            _ => false,
        }
    }

    /// **The last character back out again**, and `false` where there was
    /// nothing to take — no head asking, or an empty name.
    pub fn rub_out_of_name(&mut self) -> bool {
        match &mut self.naming {
            Some(naming) => naming.typed.pop().is_some(),
            None => false,
        }
    }

    /// **The name is finished, and this is what it asks for**: the deck that
    /// head is showing, filed under what was typed.
    ///
    /// # The deck is read at the commit and not at the press
    ///
    /// [`KeepPill`] carries the deck it was measured for because its press is
    /// one instant; this gesture spans frames, and what is filed has to be the
    /// deck the head says it is filing *now*. So the pane is looked up again
    /// and `None` is a pane that has gone — a console handed a shorter
    /// [`View::inspector`] while somebody was typing — where the field is
    /// taken away and nothing is emitted, rather than a keep landing on a deck
    /// whose head is no longer on screen.
    ///
    /// # It refuses nothing else
    ///
    /// An empty name arrives here as an empty name and leaves as one, which is
    /// [`Arrangement`]'s rule at the same seam: `id` is one path component and
    /// the wall is where the bytes are written. **A name typed twice
    /// overwrites**, which is ADR-0128 and is not this control's to soften.
    ///
    /// **The field is taken away whether or not the name is any good**, for
    /// `Readout::named`'s reason one bay along: a refusal is said out loud by
    /// whoever refuses it, and a field left standing over the refusal would be
    /// the panel asking the question again without saying the answer.
    pub fn named_set(&mut self) -> Option<Operation> {
        let naming = self.naming.take()?;
        let deck = self.inspector.get(naming.pane)?.deck as u8;
        Some(Operation::SaveSet {
            deck,
            id: Some(naming.typed),
        })
    }
}

/// **The word on the fold chip**, which is the mock's own and is drawn whether
/// or not it changes anything: *"A deck publishing a single renderer draws the
/// chip anyway and says that it changes nothing either way, because a deck
/// that grows a second one needs the control already where it was."*
const COMPOSITE_LABEL: &str = "composite";

/// **What a parameter row's leftmost cell reads where the control is not on
/// the interface** — the mock's `&middot;`, in `.param.unpub .ord`'s hairline
/// colour.
///
/// A dot where a number would be, because a control off the interface has no
/// **position** and a position is exactly what a MIDI knob counts. It is the
/// same cell either way: the number and the mark are one control's two states
/// rather than a mark drawn beside a number
/// (`docs/adr/0329-…`).
const UNPUBLISHED: &str = "·";

/// **One pane of the Inspector, painted.**
///
/// Where everything goes is [`inspector`]'s, so this paints and derives
/// nothing but the position of one chip after another along a row, which is
/// what a flex row is.
///
/// Term for term from `style.css`:
///
/// - `.half-head` — `color: var(--c-faint)` for the label, `.what`'s
///   `color: var(--c-text)` for the deck and its material, over a
///   `border-bottom: 1px solid var(--c-hair)`.
/// - `.deck-head` — a `.mini` for the sync mode, `.anchor` at
///   [`size::ANCHOR_SIZE`] beside it, and the fold's `.mini` pushed to the
///   right by `.sep`'s `flex: 1`.
/// - `.node-head` — `background: var(--c-tint)`, `.addr`'s
///   `color: var(--c-lav)`, the name in `var(--c-dim)`, and `.auth`'s three
///   words at the right.
/// - `.rend-row` — `.rend` chips, the live one in `var(--c-pink)` over a 15%
///   wash of it.
/// - `.param` — the mock's four tracks, with the fader taking what the other
///   three leave.
///
/// **Everything is clipped to the pane**, which is what makes the overflow
/// safe: a group that fits and a name that does not are the same clip, and it
/// is the same `with_clip_rect` the picture, a preview cell and the library's
/// list are each drawn inside.
/// **Whether the deck a pane is showing is on air**, off the Mixer bay's own
/// reading of it.
///
/// [`Strip::tally`] through [`residency`], which is the one place a tally
/// becomes a residency on this console — a second reading of it here would be
/// two statements about one fact, and the mock draws the pane's `keep` and the
/// strip's tally in one pink for exactly the reason that they are one fact.
///
/// **A deck with no strip is not on air**, which is a state rather than a
/// fallback: [`View::mixer`] is as long as the deck has slots, so a pane
/// pointed past the end is pointed at nothing, and nothing is not live.
pub(super) fn on_air(strips: &[Strip], deck: usize) -> bool {
    strips
        .get(deck)
        .is_some_and(|strip| residency(strip.tally) == Residency::Live)
}

pub(super) fn inspector_into(
    ui: &Ui,
    pal: &Palette,
    at: &InspectorPane,
    pane: &Pane,
    on_air: bool,
    naming: Option<&str>,
    // **The pulldown's mark**, derived by the caller off the same reading the
    // press is hit-tested against — `View::pane_pulldown`. It is handed in
    // rather than asked here because the card's rows are read off the mixer,
    // which `draw` has already borrowed. The card itself is painted after
    // every bay, for the `uses` line's card's reason.
    target: Option<PaneTarget>,
) {
    // **Derived here and hit-tested by `claim` off the same call**, and asked
    // before the words are painted rather than after: `.half-head` is a flex
    // row with `.sep` between them, so the readout is what gives way when the
    // pane is narrow and the pill keeps its place. `None` is a head with no
    // room for the capsule, which draws none — see [`keep_pill`].
    let keep = keep_pill(ui.ctx(), at, pane);
    // **And the count beside it**, which is what rule 04 asks of a pane that
    // is showing part of itself — derived here off the same head and painted
    // below, exactly as the capsule is. See [`pane_count`].
    let count = pane_count(ui.ctx(), at, pane, naming);
    // What is left of the head for the two words: everything up to whatever is
    // next along the row, one `.half-head` gap short of it. A name too long for
    // that is clipped, which is the row's own answer to a long name either way
    // — the head is a clip rectangle and there is no ellipsis in this console
    // to draw.
    let words = match count.map(|c| c.min.x).or(keep.map(|pill| pill.pill.min.x)) {
        Some(x) => Rect::from_min_max(
            at.head.min,
            Pos2::new(x - size::HALF_HEAD_GAP, at.head.max.y),
        ),
        None => at.head,
    };
    let painter = ui.painter().with_clip_rect(words);
    let label = painter.layout_job(span_at(head_label(naming), size::BASE, pal.faint));
    let y = at.head.center().y - label.size().y * 0.5;
    painter.galley(
        Pos2::new(at.head.min.x + size::HALF_HEAD_PAD_X, y),
        label,
        pal.faint,
    );
    // **The run, from the same derivation `claim` hit-tests** — the mock's
    // `.what`, and the console's second letter-taking flow while a name is
    // going into it. `None` is a head with no room to paint any of it, which
    // is [`deck_name`]'s own refusal and leaves the label alone in the row.
    if let Some(named) = deck_name(ui.ctx(), at, pane, naming) {
        // **A ground under the field while it is asking, and none while it is
        // reading.** A caret says letters are going *somewhere*; the tint says
        // where, which is the one thing a run of text in a row of readouts
        // cannot say for itself. It is `.node-head`'s own `--c-tint`, so the
        // console spends no new colour on it — and the pink a capsule is lit
        // in is deliberately not reached for here, because that pink means
        // *on air* two controls away.
        if naming.is_some() {
            painter.rect_filled(named.name, CornerRadius::same(3), pal.tint);
        }
        let what = painter.layout_job(span_at(
            &match naming {
                Some(typed) => naming_text_in_head(pane, typed),
                None => showing_text(pane),
            },
            size::BASE,
            pal.text,
        ));
        painter.galley(
            Pos2::new(
                named.name.min.x,
                named.name.center().y - what.size().y * 0.5,
            ),
            what,
            pal.text,
        );
    }
    // `.half-head`'s own `border-bottom`, the bottom pixel of the row — drawn
    // through the whole head rather than through the words' clip, which stops
    // one gap short of the pill.
    let painter = ui.painter().with_clip_rect(at.head);
    let rule = at.head.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [
            Pos2::new(at.head.min.x, rule),
            Pos2::new(at.head.max.x, rule),
        ],
        Stroke::new(size::HAIRLINE, pal.hair),
    );
    // **The pulldown's mark, after the run and inside the head's own clip** —
    // the mock's `▾` beside `deck A · drift_night`, drawn rather than typed
    // for [`CHEVRON_W`]'s reason. It is painted in the label's ink rather than
    // the run's: the mark is a control and the name beside it is a readout,
    // and the console draws every `▾` it has in `--c-faint`.
    if let Some(target) = target {
        let mark = target.chevron;
        painter.add(egui::Shape::convex_polygon(
            vec![
                mark.left_top(),
                mark.right_top(),
                Pos2::new(mark.center().x, mark.max.y),
            ],
            pal.faint,
            Stroke::NONE,
        ));
    }
    // **The mock draws the first pane's `keep` as `.pill.on` and the second
    // pane's as a plain `.pill`**, and what the lit one reads is now on the
    // page: the deck this pane is *showing* is on air. Deck A in the mock is
    // on air *and* holds the selection *and* is the first pane, and the wash
    // is the first of the three for two reasons the console already holds —
    // `.pill.on`'s pink *is* the pink a tally on air is drawn in
    // ([`on_pill_at`]), and the selection is drawn in lavender everywhere
    // else on this panel, so a pink wash meaning *selected* would be the one
    // colour on the console saying two things.
    //
    // **It is handed in rather than asked here**, which is `mixer_into`'s
    // `marked` and `selection` one bay over: residency is the *mixer's*
    // reading of a deck — [`Strip::tally`] — and a second derivation of it in
    // this bay would be two statements about one fact.
    if let Some(pill) = keep {
        match on_air {
            true => on_pill_at(ui, pal, pill.pill, KEEP_LABEL),
            false => pill_at(ui, pal, pill.pill, KEEP_LABEL),
        }
    }
    // **The count, in the label's own ink**: `.half-head`'s `color:
    // var(--c-faint)`, which is what the mock gives every readout in this row
    // and what the Library foot gives its own `5 of 27`. It is painted inside
    // the head's clip and not the words' — the words stop short of it.
    if let Some(rect) = count {
        let galley = painter.layout_job(span_at(&count_text(at, pane), size::BASE, pal.faint));
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            pal.faint,
        );
    }

    // **Derived here and hit-tested by `claim` off the same call**, which is
    // the rule every other control on this panel is drawn under. `None` is a
    // row too narrow to hold its chips, and it draws none rather than half of
    // each — see [`deck_head`].
    if let Some(head) = deck_head(ui.ctx(), at, pane) {
        deck_head_into(ui, pal, &head, pane);
    }

    // **The clip is what makes a scrolled pane safe**, and it is the same
    // rectangle [`InspectorPane::grip`] refuses a press outside: a group cut
    // by the top edge is painted with its head under the deck head and clipped
    // away there, and a press on the part that is not on screen reaches
    // nothing.
    let painter = ui.painter().with_clip_rect(at.body);
    for index in at.drawn(&pane.nodes) {
        let node = &pane.nodes[index];
        let rect = at.group(&pane.nodes, index);
        node_into(&painter, pal, rect, node);
        // `.node-group`'s `border-bottom: 1px solid var(--c-hair)`, which
        // `:last-child` does not carry — so it goes *between* two groups, and
        // the last node's is not drawn whether or not the pane is scrolled far
        // enough to have it on screen. It is `nodes.len()` and no longer the
        // count of what is drawn, because a group cut by the bottom edge has
        // a rule under it and the next group is what it separates from.
        if index + 1 < pane.nodes.len() {
            let rule = rect.max.y + size::HAIRLINE * 0.5;
            painter.line_segment(
                [Pos2::new(rect.min.x, rule), Pos2::new(rect.max.x, rule)],
                Stroke::new(size::HAIRLINE, pal.hair),
            );
        }
    }
}

/// **The deck head, painted**: the sync chip, the anchor beside it, the two
/// scrub arrows and the fold at the right.
///
/// Where everything goes is [`deck_head`]'s, so this paints and derives
/// nothing — which is the change this pass made to it: the row used to be a
/// running sum here and a press had nowhere to ask what it had landed on.
///
/// Term for term from `style.css`:
///
/// - `.mini` — the mode chip and the fold, through [`mini_into`], selected
///   because a mode is always one of three and the fold is on or off.
/// - `.anchor` — `font-size: 9px` in `--c-faint`, the run [`anchor_text`]
///   writes.
/// - `.scrub i` — `padding: 0 4px; border-radius: 999px; border: 1px solid
///   var(--c-line)` with `--c-dim` inside it, and `.scrub.idle i`'s
///   `--c-faint` over `--c-hair` where the deck is not beat-synced. **Never
///   grey without a reason**, which is the stylesheet's own note on this pair:
///   an inert scrub keeps its shape, and what says why is the page.
///
/// **The arrows are drawn rather than typed**, which is [`CHEVRON_W`]'s reason
/// three bays along: whether a black left-pointing small triangle is in
/// `egui`'s default face is a question with no good answer, and a triangle is
/// the same mark either way.
fn deck_head_into(ui: &Ui, pal: &Palette, at: &DeckHead, pane: &Pane) {
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

/// **An arrow's mark**, drawn rather than typed — a triangle with its point to
/// the left when `back`, to the right when not.
///
/// `across` wide and the same tall, which is [`Mask`]'s rule for a mark that
/// stands in for a glyph: the box is the size the glyph would have been. It is
/// not [`CHEVRON_W`]'s 2:1, because the ink of a left-pointing small triangle
/// is about as wide as it is tall where a down-pointing one is wider than it
/// is deep.
///
/// **The size is an argument and not [`size::SCRUB_SIZE`]**, because the same
/// mark is drawn at two sizes now: the Inspector's two scrub arrows, at the
/// size of the chip they sit in, and the Library foot's `→`, at [`library::LOAD_ARROW`]
/// beside the type it stands between. One triangle, so an arrow this console
/// draws is the same arrow wherever it is drawn.
pub(crate) fn arrow_mark(
    painter: &egui::Painter,
    centre: Pos2,
    across: f32,
    colour: Color32,
    back: bool,
) {
    let r = across * 0.5;
    let point = match back {
        true => centre.x - r,
        false => centre.x + r,
    };
    let base = match back {
        true => centre.x + r,
        false => centre.x - r,
    };
    painter.add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(point, centre.y),
            Pos2::new(base, centre.y - r),
            Pos2::new(base, centre.y + r),
        ],
        colour,
        Stroke::NONE,
    ));
}

/// **One node group**: the head, the renderer row where there is one, and a
/// row per parameter.
fn node_into(painter: &egui::Painter, pal: &Palette, rect: Rect, node: &Node) {
    let head = Rect::from_min_max(
        rect.min,
        Pos2::new(rect.max.x, rect.min.y + size::NODE_HEAD_H),
    );
    // `.node-head`'s `background: var(--c-tint)`, which is the wash that tells
    // a head from the rows under it — the same tint the *allocated* tally
    // carries.
    painter.rect_filled(head, CornerRadius::ZERO, pal.tint);
    let mut x = head.min.x + size::NODE_HEAD_PAD_X;
    let addr = painter.layout_job(span_at(&node.addr, size::BASE, pal.lav));
    let w = addr.size().x;
    painter.galley(
        Pos2::new(x, head.center().y - addr.size().y * 0.5),
        addr,
        pal.lav,
    );
    x += w + size::NODE_HEAD_GAP;
    let name = painter.layout_job(span_at(&node.name, size::BASE, pal.dim));
    painter.galley(
        Pos2::new(x, head.center().y - name.size().y * 0.5),
        name,
        pal.dim,
    );
    if let Some(authority) = node.authority {
        auth_into(painter, pal, head, node, authority.level);
    }
    // **The `keep` capsule at the right of the head**, and the chips above are
    // laid out inside what it leaves — [`node_keep`], which is where both
    // halves of that arithmetic are. A head that carries none draws none,
    // which is [`Node::keep`]'s two cases rather than a capsule that refuses.
    if let Some(pill) = node_keep(painter.ctx(), head, node) {
        // **A plain `.mini` and never `.sel`**, which is the mock's own and is
        // [`KeepPill`]'s note one row up read on a node: a keep is a press and
        // not a setting, so there is nothing here for a wash to be *on*. The
        // pane head's capsule carries one because it reads the deck's
        // residency, and a node has none.
        let galley = painter.layout_no_wrap(
            KEEP_LABEL.to_owned(),
            FontId::new(size::MINI_SIZE, FontFamily::Proportional),
            pal.faint,
        );
        mini_into(painter, pal, pill, false, |painter, colour| {
            painter.galley(
                Pos2::new(
                    pill.min.x + size::MINI_PAD_X,
                    pill.center().y - galley.size().y * 0.5,
                ),
                galley,
                colour,
            );
        });
    }

    // **The inputs this node declares, under its head and above its rows**,
    // for the same reason and asked the same way: [`uses_rect`] and
    // [`uses_chip_in`] are where that arithmetic is written, and this paints
    // what they answer.
    for (index, uses) in node.uses.iter().enumerate() {
        uses_into(painter, pal, uses_rect(rect, index), uses);
    }

    // **Where the renderer row is, asked rather than measured here**: a press
    // has to resolve to the same rectangle the chips were drawn in, and
    // [`rend_row_in`] is the one place that arithmetic is written.
    if !node.renderers.is_empty() {
        rend_row_into(painter, pal, rend_row_in(rect, node), &node.renderers);
    }
    // **Where a row is, asked rather than accumulated.** The running sum this
    // loop used to keep was a second answer to the same question the moment a
    // press had to be resolved to a row — see [`param_rect`].
    for (index, param) in node.params.iter().enumerate() {
        param_into(painter, pal, param_rect(rect, node, index), param);
        // **Where the sensitivity row is, asked rather than measured here**,
        // which is the renderer row's rule one level up: a press has to
        // resolve to the same rectangle the chips were drawn in.
        if let (Some(row), Some(source)) = (sens_rect(rect, node, index), param.bound.as_ref()) {
            sens_into(painter, pal, row, source);
        }
    }
}

/// **`man / sug / auto`, right-aligned on the node head**, with the one the
/// node is on filled: `.auth span.sel`'s `color: var(--c-mint)` over a 15%
/// wash of it, and the other two in `var(--c-faint)` with no box at all.
///
/// **All three and not only the one**, which is the manual's own row: *"`man /
/// sug / auto` on each node head, never a global mode."* What is drawn is
/// which of the three this node is on, and **all three are claimed**: each
/// names a destination, which is what an operation on this panel is
/// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
/// and pressing the one a node is already on asks for what it already has —
/// the renderer row's rule one row down, and the anchor's two bays over. A
/// chip that stopped being pressable the moment it lit would take the claim
/// out from under a hand.
///
/// **A head that folds more than one node draws none**, which is `Node::authority`
/// being `None`: authority is per node, so one chip over three renderers would
/// be one of three answers drawn as *the* answer and a press on it would set
/// three nodes at once.
fn auth_into(
    painter: &egui::Painter,
    pal: &Palette,
    head: Rect,
    node: &Node,
    authority: Authority,
) {
    for (level, rect) in auth_chips(painter.ctx(), head, node) {
        let galley = painter.layout_no_wrap(
            auth_word(level).to_owned(),
            FontId::new(size::AUTH_SIZE, FontFamily::Proportional),
            pal.faint,
        );
        let sel = level == authority;
        let colour = match sel {
            true => pal.mint,
            false => pal.faint,
        };
        if sel {
            painter.rect_filled(
                rect,
                // `border-radius: 999px` on a box this short is a capsule.
                CornerRadius::same((size::AUTH_H * 0.5) as u8),
                tint(pal.mint, 15),
            );
        }
        painter.galley(
            Pos2::new(
                rect.min.x + size::AUTH_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            colour,
        );
    }
}

/// **Where each of the three authority chips goes on a node head**,
/// right-aligned inside the head's own padding.
///
/// **One derivation, asked twice** — [`auth_into`] paints these and
/// [`InspectorPane::set_authority`] hit-tests them, which is [`rend_chips`]'
/// rule one row up: two copies of where a chip is would be a chip painted
/// where a hand cannot press it. The three were drawn and unclaimed from
/// 2026-08-29 until the writer existed, and the running sum inside the painter
/// was exactly the shape a press had nowhere to ask about.
///
/// Right-aligned, so the whole row has to be measured before the first chip
/// can be placed: `.node-head`'s `.sep` pushes `.auth` to the end of the flex
/// row.
///
/// # The node is taken because the `keep` capsule is at the same end
///
/// `.node-head` ends `.sep, .auth, .mini` — the capsule is hard against the
/// head's padding and the chips are stepped back from it — so where a chip
/// goes depends on whether this head carries one ([`node_keep`]). **The trim
/// is inside this function rather than at its callers**, because a caller that
/// forgot it would place three chips over the capsule, and the paint and the
/// hit-test would agree with each other and disagree with the mock. Two
/// callers each applying it correctly is a rule held by prose, which is
/// exactly what `docs/contributing.md` §4's structural tier is against.
pub fn auth_chips(
    ctx: &egui::Context,
    head: Rect,
    node: &Node,
) -> impl Iterator<Item = (Authority, Rect)> {
    let head = auth_head(ctx, head, node);
    let widths: Vec<f32> = AUTHORITIES
        .into_iter()
        .map(|level| auth_width(ctx, level))
        .collect();
    let total: f32 = widths.iter().sum::<f32>() + size::AUTH_GAP * (AUTHORITIES.len() - 1) as f32;
    let mut x = head.max.x - size::NODE_HEAD_PAD_X - total;
    let top = head.center().y - size::AUTH_H * 0.5;
    AUTHORITIES
        .into_iter()
        .zip(widths)
        .map(move |(level, w)| {
            let rect = Rect::from_min_size(Pos2::new(x, top), egui::vec2(w, size::AUTH_H));
            x += w + size::AUTH_GAP;
            (level, rect)
        })
        .collect::<Vec<_>>()
        .into_iter()
}

/// **Where a node head's `keep` capsule goes**, or `None` on a head that
/// carries none.
///
/// The mock's `.node-head` is a flex row of the address, the name, a `.sep`,
/// the `.auth` chips and then `<span class="mini">keep</span>` — so the
/// capsule is hard against the head's right-hand padding and the chips are
/// stepped back from it by [`size::NODE_HEAD_GAP`], which is `.node-head`'s
/// own `gap: 7px`. That is why this is derived before [`auth_chips`] rather
/// than beside it: the chips are laid out inside what this leaves.
///
/// **`None` on the two heads that carry no capsule** — [`Node::keep`], where
/// the rule is written — and `None` on a head with no room for it, which is
/// [`keep_pill`]'s rule one row down: *a control that does not fit in the row
/// it is drawn in is no control at all, rather than half of one*.
///
/// **One derivation, asked twice** — [`node_into`] paints it and
/// [`InspectorPane::keep_procedure`] hit-tests it, which is [`auth_chips`]'
/// own rule: two copies of where a capsule is would be a capsule painted where
/// a hand cannot press it.
pub fn node_keep(ctx: &egui::Context, head: Rect, node: &Node) -> Option<Rect> {
    node.keep?;
    // Fonts are not valid until `egui` has run a pass — [`keep_pill`]'s guard,
    // and before the first one there is nothing drawn here to press.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let w = ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            KEEP_LABEL.to_owned(),
            FontId::new(size::MINI_SIZE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    }) + size::MINI_PAD_X * 2.0;
    let pill = Rect::from_min_size(
        Pos2::new(
            head.max.x - size::NODE_HEAD_PAD_X - w,
            head.center().y - size::MINI_H * 0.5,
        ),
        egui::vec2(w, size::MINI_H),
    );
    // Measured against the head's content box, exactly as [`keep_pill`] is:
    // the capsule is placed from the right-hand padding, so what it runs off
    // is the left one.
    let room = head.width() - size::NODE_HEAD_PAD_X * 2.0;
    (positive(head) && w <= room).then_some(pill)
}

/// **What is left of a node head for the authority chips** — the head, less
/// the `keep` capsule and the gap before it where there is one.
///
/// One function because [`auth_into`] paints the chips and
/// [`InspectorPane::set_authority`] hit-tests them, and a head trimmed in one
/// of the two would be three chips drawn where a hand cannot press them. It is
/// [`node_keep`]'s other half: the two controls at the right of this row are
/// laid out from the right, the capsule first.
fn auth_head(ctx: &egui::Context, head: Rect, node: &Node) -> Rect {
    match node_keep(ctx, head, node) {
        Some(keep) => Rect::from_min_max(
            head.min,
            Pos2::new(
                keep.min.x - size::NODE_HEAD_GAP + size::NODE_HEAD_PAD_X,
                head.max.y,
            ),
        ),
        None => head,
    }
}

/// One authority chip's width: its word at [`size::AUTH_SIZE`] inside
/// `.auth span`'s `padding: 0 5px`.
fn auth_width(ctx: &egui::Context, level: Authority) -> f32 {
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            auth_word(level).to_owned(),
            FontId::new(size::AUTH_SIZE, FontFamily::Proportional),
            Color32::PLACEHOLDER,
        )
        .size()
        .x
    }) + size::AUTH_PAD_X * 2.0
}

/// **The renderer chips**, one per renderer the Set has, from the left.
///
/// The live one carries `.rend.sel`: `color: var(--c-pink)` over a 15% wash of
/// it, no border, and the pink halo `box-shadow: 0 0 9px var(--c-glowp)` —
/// which is the same 9 the lit beat and a live fader knob carry. The rest are
/// `.rend`'s `border: 1px solid var(--c-line)` around `var(--c-dim)`.
///
/// **One row and what fits of it.** `.rend-row` wraps in the mock and the
/// console does not: a wrapped row is a group taller than [`group_h`] said it
/// was, and the pane's own arithmetic is what says whether a group is drawn at
/// all. A chip past the right-hand edge is clipped, which is the same answer
/// the pane gives a group past the bottom.
///
/// **Where each chip goes is [`rend_chips`]', so this paints and derives
/// nothing** — the change this pass made to it, and [`deck_head_into`]'s rule
/// one row up: the row used to be a running sum here and a press had nowhere
/// to ask what it had landed on.
fn rend_row_into(painter: &egui::Painter, pal: &Palette, row: Rect, renderers: &[Renderer]) {
    for (index, rect) in rend_chips(painter.ctx(), row, renderers) {
        let rend = &renderers[index];
        let galley = painter.layout_no_wrap(
            rend.name.clone(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        );
        let radius = CornerRadius::same((size::REND_H * 0.5) as u8);
        match rend.live {
            true => {
                painter.add(
                    egui::epaint::Shadow {
                        offset: [0, 0],
                        blur: size::TALLY_GLOW - 1,
                        spread: 0,
                        color: pal.glow_pink,
                    }
                    .as_shape(rect, radius),
                );
                painter.rect_filled(rect, radius, tint(pal.pink, 15));
            }
            false => {
                painter.rect_stroke(
                    rect,
                    radius,
                    Stroke::new(size::HAIRLINE, pal.line),
                    StrokeKind::Inside,
                );
            }
        }
        painter.galley(
            Pos2::new(
                rect.min.x + size::REND_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            match rend.live {
                true => pal.pink,
                false => pal.dim,
            },
        );
    }
}

/// **One `uses` line**: what the procedure calls the input at the left, and the
/// capsule naming the node filling it at the right.
///
/// **The chevron is drawn rather than typed**, which is [`CHEVRON_W`]'s reason
/// wherever this console draws a pulldown — the Outputs row's pill, the
/// arrangement pill and the Library bay's deck capsule all carry the same mark.
fn uses_into(painter: &egui::Painter, pal: &Palette, row: Rect, uses: &Uses) {
    let painter = painter.with_clip_rect(row);
    let word = painter.layout_job(span_at(
        &format!("uses {}", uses.slot),
        size::USES_SIZE,
        pal.dim,
    ));
    painter.galley(
        Pos2::new(
            row.min.x + size::PARAM_PAD_L,
            row.center().y - word.size().y * 0.5,
        ),
        word,
        pal.dim,
    );
    let chip = uses_chip_in(painter.ctx(), row, uses);
    painter.rect_stroke(
        chip,
        CornerRadius::same((chip.height() * 0.5) as u8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let name = painter.layout_job(span_at(&uses.to, size::USES_SIZE, pal.dim));
    painter.galley(
        Pos2::new(
            chip.min.x + size::PILL_PAD_X,
            chip.center().y - name.size().y * 0.5,
        ),
        name,
        pal.dim,
    );
    // The `▾`, drawn as the same triangle every pulldown on this console draws:
    // `CHEVRON_W` across and `CHEVRON_H` deep, centred in the padding at the
    // capsule's right.
    let chevron = Rect::from_center_size(
        Pos2::new(
            chip.max.x - size::PILL_PAD_X - CHEVRON_W * 0.5,
            chip.center().y,
        ),
        egui::vec2(CHEVRON_W, CHEVRON_H),
    );
    painter.add(egui::Shape::convex_polygon(
        vec![
            chevron.left_top(),
            chevron.right_top(),
            Pos2::new(chevron.center().x, chevron.max.y),
        ],
        pal.dim,
        Stroke::NONE,
    ));
}

/// **One parameter row**, in the mock's own four tracks: the ordinal at
/// [`size::PARAM_ORD_W`] right-aligned, the name at [`size::PARAM_NAME_W`],
/// the fader taking what is left, and the value at [`size::PARAM_VAL_W`]
/// right-aligned.
///
/// **The value is two places**, which is the mock's `2.40`, `0.71`, `1.20` —
/// every number in its `.pval` column. It is not a second spelling of the
/// transport's tempo: that one is a BPM and this is a parameter, and the mock
/// writes the two differently for that reason.
fn param_into(painter: &egui::Painter, pal: &Palette, row: Rect, param: &Param) {
    let left = row.min.x + size::PARAM_PAD_L;
    let right = row.max.x - size::PARAM_PAD_R;
    // **The mark is the number**, and a control the interface does not carry
    // has none: `.param.unpub .ord` is the mock's dot in the hairline colour,
    // where a published row's is its position in `--c-faint`. One cell, two
    // states, and the state *is* whether it is published — a second mark beside
    // the number would be two spellings of one fact
    // (`docs/adr/0329-…`).
    let (word, ink) = match param.ord {
        Some(ord) => (ord.to_string(), pal.faint),
        None => (UNPUBLISHED.to_owned(), pal.hair),
    };
    let ord = painter.layout_job(span_at(&word, size::PARAM_ORD_SIZE, ink));
    painter.galley(
        Pos2::new(
            left + size::PARAM_ORD_W - ord.size().x,
            row.center().y - ord.size().y * 0.5,
        ),
        ord,
        ink,
    );
    let name_x = left + size::PARAM_ORD_W + size::PARAM_GAP;
    // `.param .pname`'s `overflow: hidden; text-overflow: ellipsis` — one row,
    // broken anywhere, with an ellipsis for what did not fit. The mock says so
    // for this column and not for the library's, which is why one elides and
    // the other clips.
    // `.param.unpub .pname` is a shade further back than `.param`'s, which is
    // the whole of what an unpublished row looks like beside a published one:
    // the name is still legible — the row is drawn so it can be pressed again —
    // and nothing about it invites a hand.
    let name_ink = match param.ord {
        Some(_) => pal.dim,
        None => pal.faint,
    };
    let mut job = span_at(&param.name, size::BASE, name_ink);
    job.wrap = egui::epaint::text::TextWrapping {
        max_width: size::PARAM_NAME_W,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let name = painter.layout_job(job);
    painter.galley(
        Pos2::new(name_x, row.center().y - name.size().y * 0.5),
        name,
        name_ink,
    );
    // **A control the interface does not carry stops here.** The fader and the
    // figure are what publishing decides the panel shows, so a row that is off
    // the list is a mark and a name and nothing else — and a value drawn beside
    // a control this pane says it is not showing would be the page's own
    // sentence contradicted in the same row.
    if param.ord.is_none() {
        return;
    }
    // **A bound row shows its source instead of a number** — `.param.bound`'s
    // `.pval.src`, and the readout the manual calls *where disagreeing with
    // the system begins*. The mock draws it in the same right-aligned track
    // the figure is in, so this is one galley either way, and it is the mint
    // `.src` carries rather than `.pval`'s text colour.
    let (text, colour) = match &param.bound {
        None => (format!("{:.2}", param.value), pal.text),
        Some(source) => (source.signal.clone(), pal.mint),
    };
    let value = painter.layout_job(span_at(&text, size::BASE, colour));
    painter.galley(
        Pos2::new(
            right - value.size().x,
            row.center().y - value.size().y * 0.5,
        ),
        value,
        colour,
    );
    if let Some(fader) = param_fader(row, param) {
        fader_into(painter, pal, fader, false, None);
    }
}

/// **One sensitivity row**: the word in `.sens`'s first track, then the chips
/// that say what is holding the control and offer the two things a hand can do
/// about it.
///
/// **Where each chip goes is [`sens_chips`]', so this paints and derives
/// nothing** — [`rend_row_into`]'s rule one row down, and the reason a press
/// has somewhere to ask what it landed on.
///
/// **Two of the four are drawn as readouts and two as controls**, and nothing
/// in the paint says which: the mock gives the source pill `.pill.armed` and
/// the other three a plain `.pill`, and *armed* here is the mint of something
/// that is holding a control rather than of something that can be pressed.
/// Which chips are claimed is [`SensChip::operation`]'s, and a panel that drew
/// the difference would be drawing a rule the mock does not.
fn sens_into(painter: &egui::Painter, pal: &Palette, row: Rect, source: &Source) {
    let label = painter.layout_job(span_at(SENS_LABEL, size::SENS_SIZE, pal.faint));
    painter.galley(
        Pos2::new(
            row.min.x + size::SENS_PAD_L,
            row.min.y + size::SENS_PAD_T + (size::SENS_CHIP_H - label.size().y) * 0.5,
        ),
        label,
        pal.faint,
    );
    let radius = CornerRadius::same((size::SENS_CHIP_H * 0.5) as u8);
    for (chip, rect) in sens_chips(painter.ctx(), row, source) {
        let armed = chip == SensChip::Signal;
        match armed {
            // `.pill.armed`: no border, a 14% wash of the mint and the same
            // nine-pixel halo an armed pill carries everywhere else on this
            // panel.
            true => {
                painter.add(
                    egui::epaint::Shadow {
                        offset: [0, 0],
                        blur: size::TALLY_GLOW - 1,
                        spread: 0,
                        color: pal.glow,
                    }
                    .as_shape(rect, radius),
                );
                painter.rect_filled(rect, radius, tint(pal.mint, 14));
            }
            // `.pill`'s `border: 1px solid var(--c-line)`.
            false => {
                painter.rect_stroke(
                    rect,
                    radius,
                    Stroke::new(size::HAIRLINE, pal.line),
                    StrokeKind::Inside,
                );
            }
        }
        let colour = match armed {
            true => pal.mint,
            false => pal.dim,
        };
        let galley = painter.layout_no_wrap(
            chip.text(source),
            FontId::new(size::SENS_SIZE, FontFamily::Proportional),
            colour,
        );
        painter.galley(
            Pos2::new(
                rect.min.x + size::SENS_CHIP_PAD_X,
                rect.center().y - galley.size().y * 0.5,
            ),
            galley,
            colour,
        );
    }
}

/// **One parameter row's fader, laid out** — the track between the name and
/// the figure, what the value fills of it, and the knob on the fill's moving
/// edge. `None` where the row is too narrow to have a track at all.
///
/// # One derivation, asked twice
///
/// [`param_into`] paints this and [`InspectorPane::grip`] hit-tests it, which
/// is [`Mixer::grab`]'s rule and [`MasterRow::grab`]'s: two copies of where a
/// knob is would be a knob painted where a hand cannot take hold of it. **The
/// value is part of the geometry** — the knob sits on the fill's moving edge,
/// so where it is depends on what the deck said this frame, and the row a hand
/// grabs is the row it saw.
///
/// **`.param`'s middle track**, which is the `1fr` of `grid-template-columns:
/// 15px 88px 1fr 58px`: the ordinal, the name and the figure are stated widths
/// and this is what is left between them. `lib.rs` measures the pane's own
/// minimum off exactly that — *"the fader is the `1fr` track and is drawn only
/// where what is left over is positive"* (ADR-0279) — and this is where that
/// `positive` is asked.
fn param_fader(row: Rect, param: &Param) -> Option<Fader> {
    let left = row.min.x + size::PARAM_PAD_L;
    let right = row.max.x - size::PARAM_PAD_R;
    let track = Rect::from_min_max(
        Pos2::new(
            left + size::PARAM_ORD_W + size::PARAM_GAP + size::PARAM_NAME_W + size::PARAM_GAP,
            row.center().y - size::FADER_H * 0.5,
        ),
        Pos2::new(
            right - size::PARAM_VAL_W - size::PARAM_GAP,
            row.center().y + size::FADER_H * 0.5,
        ),
    );
    match positive(track) {
        false => None,
        // `.fader b` fills its 5px track edge to edge, so the inset is zero —
        // the one argument that tells this fader from the mixer's vertical
        // one, which `fader` takes for exactly this reason.
        true => Some(fader(
            track,
            Axis::Row,
            param.at(),
            0.0,
            egui::vec2(size::FADER_KNOB_W, size::FADER_KNOB_H),
        )),
    }
}

/// **Where the `index`th parameter row of `node` goes** inside the group
/// rectangle [`InspectorPane::group`] answered.
///
/// [`InspectorPane::group`]'s walk one level in, and a function rather than a
/// running sum inside [`node_into`] for the reason the deck head was lifted out
/// of `inspector_into`: a press had nowhere to ask what it had landed on. The
/// head is [`size::NODE_HEAD_H`], the renderer row is [`size::REND_ROW_H`]
/// where the group has one, and the rows are [`size::PARAM_H`] each from
/// there — which is [`group_h`] read as an offset instead of as a total, and
/// the two are checked against each other in `tests/param_fader.rs`.
fn param_rect(group: Rect, node: &Node, index: usize) -> Rect {
    let top = group.min.y
        + size::NODE_HEAD_H
        + uses_h(node)
        + match node.renderers.is_empty() {
            true => 0.0,
            false => size::REND_ROW_H,
        }
        // **A walk and not a stride, because a row is as tall as what is under
        // it.** A bound row carries a sensitivity row, so the rows above this
        // one are not all [`size::PARAM_H`] — which is [`InspectorPane::group`]'s
        // own reason for walking the groups instead of multiplying, one level in.
        + node.params.iter().take(index).map(rows_h).sum::<f32>();
    Rect::from_min_max(
        Pos2::new(group.min.x, top),
        Pos2::new(group.max.x, top + size::PARAM_H),
    )
}

/// **The leftmost cell of a parameter row**, which is the mark that publishes
/// it: [`size::PARAM_ORD_W`] wide at the row's left padding, the full height of
/// the row.
///
/// **The whole cell and not the ink in it.** A published row's number is one or
/// two glyphs and an unpublished row's is a dot, so a target the size of what is
/// drawn would be a control that shrank as the interface grew past nine — which
/// is the *drawn and not claimed* mistake made in the other direction. The cell
/// is a fixed track of the mock's own grid, so the target is the same size on
/// every row.
///
/// `param` is taken so that this cannot be asked of a row that has none to give;
/// there is no such row today, and the argument for the cell being one control's
/// two states is at [`UNPUBLISHED`].
fn ord_cell(row: Rect, _param: &Param) -> Rect {
    let left = row.min.x + size::PARAM_PAD_L;
    Rect::from_min_max(
        Pos2::new(left, row.min.y),
        Pos2::new(left + size::PARAM_ORD_W, row.max.y),
    )
}

/// **Where the `index`th row's sensitivity row goes** — directly under the row
/// itself, the full width of the group and [`size::SENS_H`] tall — or `None`
/// where nothing is holding that control.
///
/// [`param_rect`] stepped off the end of the row it belongs to, which is the
/// one place that relationship is written: the `.sens` row is not a row of its
/// own in the mock's list, it is what a `.param.bound` grows.
fn sens_rect(group: Rect, node: &Node, index: usize) -> Option<Rect> {
    let param = node.params.get(index)?;
    param.bound.as_ref()?;
    let row = param_rect(group, node, index);
    Some(Rect::from_min_max(
        Pos2::new(row.min.x, row.max.y),
        Pos2::new(row.max.x, row.max.y + size::SENS_H),
    ))
}

/// **A pane head's deck list, painted** — [`deck_list_into`]'s card one bay
/// along, with the deck the pane is *showing* in the panel's own text colour
/// and the rest dim.
///
/// Where everything goes is [`PaneTarget`]'s, so this paints and derives
/// nothing, which is [`deck_list_into`]'s own sentence.
///
/// **The marked row is what this pane is pointed at and never the deck
/// selection**, which is the whole of what this mark is: a pane showing deck C
/// while the keys are on deck A draws `C` in the text colour here and the ring
/// stays on A's strip, one bay over.
pub(super) fn pane_list_into(
    ui: &Ui,
    pal: &Palette,
    target: &PaneTarget,
    showing: usize,
    card: Rect,
) {
    let painter = ui.painter();
    painter.add(pal.shadow.as_shape(card, CornerRadius::same(8)));
    painter.rect_filled(card, CornerRadius::same(8), pal.panel);
    painter.rect_stroke(
        card,
        CornerRadius::same(8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    // `take` rather than a range, because the rows are the letters — see
    // [`deck_list_into`], and `View::point_pane` is what stops a deck this
    // crate has no letter for being asked for.
    for (index, letter) in DECK_LETTERS.iter().enumerate().take(target.rows) {
        let row = target.row(card, index);
        let ink = match index == showing {
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

/// **A `uses` line's card, painted** — one row per node the input may be wired
/// to, with the one it is wired to now drawn in the panel's own text colour.
///
/// Where everything goes is [`UsesLine`]'s, so this paints and derives nothing,
/// which is [`deck_list_into`]'s own sentence one bay along. Drawn from
/// [`View::draw`] **after the bays** for that card's reason: it hangs out of a
/// line inside a pane and over the groups under it.
///
/// **The node already wired is not in the list**, so the *marked* row here is
/// never one of them — the ink says nothing about the current wiring and every
/// row is a change. That is why this is one colour where the deck pulldown's
/// list is two: a pulldown names where the *next* press lands and this one
/// names where the input goes.
pub(super) fn uses_card_into(
    ui: &Ui,
    pal: &Palette,
    line: &UsesLine,
    uses: &Uses,
    card: Rect,
    room: Rect,
) {
    let painter = ui.painter();
    painter.add(pal.shadow.as_shape(card, CornerRadius::same(8)));
    painter.rect_filled(card, CornerRadius::same(8), pal.panel);
    painter.rect_stroke(
        card,
        CornerRadius::same(8),
        Stroke::new(size::HAIRLINE, pal.line),
        StrokeKind::Inside,
    );
    let painter = painter.with_clip_rect(card);
    for (index, name) in uses.candidates.iter().enumerate().take(line.rows) {
        let Some(row) = line.row_at(room, index) else {
            continue;
        };
        let galley = painter.layout_no_wrap(
            name.clone(),
            FontId::new(size::BASE, FontFamily::Proportional),
            pal.dim,
        );
        painter.galley(
            Pos2::new(
                row.min.x + size::LIB_ROW_PAD_X,
                row.center().y - galley.size().y * 0.5,
            ),
            galley,
            pal.dim,
        );
    }
}

impl View {
    /// **Which `uses` line's card is down**, or `None` — see
    /// [`View::wiring_open`] the field.
    pub fn wiring_open(&self) -> Option<(usize, usize, usize)> {
        self.wiring_open
    }

    /// **Put one `uses` line's card down**, and answer whether anything moved.
    ///
    /// **Refused for a console with no pane**, which is [`View::open_target`]'s
    /// own guard and its reason: a card with no line under it offers nothing to
    /// pick and nothing to leave by, and `input::claim`'s rule 2 would hand it
    /// every press until a second one shut it.
    pub fn open_wiring(&mut self, pane: usize, node: usize, input: usize) -> bool {
        if self.inspector.get(pane).is_none() {
            return false;
        }
        let at = Some((pane, node, input));
        let moved = self.wiring_open != at;
        self.wiring_open = at;
        moved
    }

    /// **Put it away**, and answer whether one was down.
    pub fn shut_wiring(&mut self) -> bool {
        let was = self.wiring_open.is_some();
        self.wiring_open = None;
        was
    }

    /// **How far one Inspector pane is scrolled**, as it is stored — the
    /// number [`inspector`] clamps and never the one it clamped.
    ///
    /// Zero for a pane index past [`PANES`], which is a caller's error and not
    /// a state: the bay has two panes and `PANE_NAMES` is what says so.
    pub fn scroll_in(&self, pane: usize) -> f32 {
        self.scroll.get(pane).copied().unwrap_or(0.0)
    }

    /// **Turn one pane's wheel by `by` pixels**, positive down the list, and
    /// answer whether the stored position moved.
    ///
    /// # Two clamps, and only one of them is here
    ///
    /// This one is against the **content** — how tall everything the deck
    /// publishes comes to — and it is a reading of the deck rather than of a
    /// viewport, so a stored position bounded by it is not a position any
    /// resize can rewrite. Without it a wheel spun over a short Set would put
    /// the number in the thousands and an operator would have to spin it all
    /// the way back before anything moved, which is *"a control you cannot see
    /// being moved"* by another name.
    ///
    /// The other clamp is against the pane's own height and belongs where the
    /// pane is laid out — [`InspectorPane::scroll`], which is
    /// [P-0082](../../../../docs/principles/0082-looking-never-writes-back.md):
    /// a shorter pane draws less of the same position and stores nothing, so
    /// dragging it back reproduces the picture exactly rather than nearly
    /// (ADR-0250's argument one region in).
    ///
    /// **A pane this console is not showing anything in refuses the wheel**
    /// rather than storing a position for it, which is [`View::point_at`]'s
    /// rule: what a pointer can be at is something drawn.
    pub fn scroll_by(&mut self, pane: usize, by: f32) -> bool {
        let Some(showing) = self.inspector.get(pane) else {
            return false;
        };
        let Some(at) = self.scroll.get_mut(pane) else {
            return false;
        };
        let content = content_h(&showing.nodes);
        let next = (*at + by).clamp(0.0, content);
        let moved = next != *at;
        *at = next;
        moved
    }

    /// **Which deck one Inspector pane is pointed at** — see
    /// [`View::pane_deck`] the field, which is where the argument is.
    ///
    /// **This is what the host reads to fill the pane**, which is
    /// [`View::target_deck`]'s arrangement one bay along: the pointer is the
    /// console's and what is under it is the host's answer to *what is that
    /// deck playing*.
    ///
    /// Deck A for a pane index past [`PANES`], which is a caller's error and
    /// not a state — [`View::scroll_in`]'s own rule.
    ///
    /// [`View::pane_deck`]: Self::pane_deck
    pub fn pane_deck(&self, pane: usize) -> u8 {
        self.pane_deck.get(pane).copied().unwrap_or(0)
    }

    /// **Every pane's target at once**, which is what a host reads before it
    /// fills [`View::inspector`]: the panes are written through a `&mut` of
    /// that field, so asking pane by pane while it is borrowed is a second
    /// borrow of this struct. It is two bytes and `Copy`.
    pub fn pane_decks(&self) -> [u8; PANES] {
        self.pane_deck
    }

    /// **Point one pane at `deck`**, put the card away, and answer whether
    /// anything moved.
    ///
    /// **A deck the mixer has no strip for is refused**, which is
    /// [`View::select`]'s rule and [`View::aim_at`]'s read a third time rather
    /// than a third rule: the head says which deck it is showing, so a target
    /// past the deck's slots would be a letter naming a deck with nothing
    /// under it.
    ///
    /// **It refuses rather than clamping**, for `select`'s reason: a pick of
    /// deck D at a two-slot deck means *deck D*, and clamping would point the
    /// pane at deck B — a different deck than the one asked for.
    ///
    /// **Nothing else moves**, and that is the whole of what this mark is for:
    /// not the deck selection, not the pane next door, not the Library bay's
    /// load target. Each of those is a pointer of its own with a writer of its
    /// own, and this one touches none of them.
    ///
    /// **The card goes away here**, which is [`View::aim_at`]'s clause: a pick
    /// is one gesture and this is the whole of it, so a card left down would
    /// go on claiming every press on the console. It is put away even where
    /// the pane did not move — picking the deck a pane already shows is still
    /// a hand finishing what it started.
    ///
    /// The `bool` is [`View::select`]'s: a caller repaints on a move and not
    /// on a press.
    pub fn point_pane(&mut self, pane: usize, deck: u8) -> bool {
        if usize::from(deck) >= self.mixer.len() {
            return false;
        }
        let Some(at) = self.pane_deck.get_mut(pane) else {
            return false;
        };
        let moved = *at != deck || self.pane_open.is_some();
        *at = deck;
        self.pane_open = None;
        moved
    }

    /// **Which pane head's pulldown is down**, or `None` — see
    /// [`View::pane_open`] the field.
    ///
    /// [`View::pane_open`]: Self::pane_open
    pub fn pane_target_open(&self) -> Option<usize> {
        self.pane_open
    }

    /// **Put one pane head's card down**, and answer whether anything moved.
    ///
    /// **Refused for a pane this console is not showing**, which is
    /// [`View::open_wiring`]'s own guard and its reason: a card with no head
    /// under it offers nothing to pick and nothing to leave by, and
    /// `input::claim`'s rule 2 would hand it every press until a second one
    /// shut it.
    pub fn open_pane_target(&mut self, pane: usize) -> bool {
        if pane >= PANES || self.inspector.get(pane).is_none() {
            return false;
        }
        let moved = self.pane_open != Some(pane);
        self.pane_open = Some(pane);
        moved
    }

    /// **Put it away**, and answer whether one was down.
    pub fn shut_pane_target(&mut self) -> bool {
        let was = self.pane_open.is_some();
        self.pane_open = None;
        was
    }

    /// **What one pane head's pulldown is**, laid out — the mark and, while it
    /// is down, the card under it.
    ///
    /// Read once for the frame and handed to the paint and to the press,
    /// exactly as [`View::target`] is: the mark that is drawn and the mark a
    /// press lands on are one derivation of one reading.
    ///
    /// `None` is a head with no run in it, which is [`deck_name`]'s refusal —
    /// see [`pane_target`].
    pub fn pane_pulldown(
        &self,
        ctx: &egui::Context,
        at: &InspectorPane,
        pane: &Pane,
        index: usize,
    ) -> Option<PaneTarget> {
        pane_target(
            ctx,
            at,
            pane,
            index,
            self.naming_set_in(index),
            self.mixer.len(),
            self.pane_open == Some(index),
        )
    }
}

#[cfg(test)]
mod tests {
    //! **What is private and worth asserting, and it is in `src/` rather than
    //! in `tests/` for that one reason.**
    //!
    //! Every region of this console is asserted from a file in `tests/`, one
    //! per region, and everything here would have been in one of them. It is
    //! here because what it names is private, and moving it out would mean
    //! widening a surface to test it — a wider surface for a narrower reason.
    //!
    //! **Moved here from `view/mod.rs`**, on [`transport`]'s own precedent:
    //! [`pane_box`] and [`group_h`] are the pane's arithmetic, and the public
    //! [`inspector`] is that arithmetic plus a layout lookup — the reasoning
    //! that was already written here travels with the code under
    //! [ADR-0121](../../../../docs/adr/0121-moving-code-leaves-its-reasoning-behind.md),
    //! the same rule `offset_text`'s own assertion moved under, one bay over.
    //!
    //! What is *not* here and belongs in `tests/inspector.rs` when somebody
    //! writes it: the bay drawn against a real arrangement at the two
    //! viewports, on `tests/library.rs`'s pattern.

    use super::*;

    /// A pane with `params` parameters under one node and no renderers.
    fn one_node(params: usize) -> Pane {
        Pane {
            deck: 0,
            material: "drift_shell + soft_points".to_owned(),
            sync: Sync::Tempo,
            // Nothing refused, which is what a pane about the arithmetic of
            // its rows says about material it is not asking after.
            allows: [true; SYNCS.len()],
            anchor_bpm: 128.0,
            scrub_beats: 0.0,
            composite: false,
            // Not this module's row: the deck head's two build chips are drawn
            // from this, and everything here is about the rows under it.
            aimed: None,
            nodes: vec![Node {
                addr: "L1:0".to_owned(),
                name: "drift_shell".to_owned(),
                authority: Some(NodeAuthority {
                    at: NodeAddress {
                        layer: Layer::L1,
                        index: 0,
                    },
                    level: Authority::Manual,
                }),
                // **A node with a source**, which every node but the built-in
                // camera has — and the capsule this draws is what the tests
                // below measure the head's right-hand end against.
                keep: Some(NodeAddress {
                    layer: Layer::L1,
                    index: 0,
                }),
                uses: Vec::new(),
                renderers: Vec::new(),
                params: (0..params)
                    .map(|n| Param {
                        ord: Some(n + 1),
                        name: format!("p{n}"),
                        value: 0.5,
                        range: [0.0, 1.0],
                        param: karakuri_operation::ParamAt {
                            node: None,
                            key: format!("p{n}"),
                        },
                        bound: None,
                    })
                    .collect(),
            }],
        }
    }

    /// A rectangle the size of one inspector pane at the narrowest console the
    /// mock will draw: `.console`'s `min-width: 1010px` holds the centre at
    /// 484 and each pane at 237.
    fn pane_at(height: f32) -> Rect {
        Rect::from_min_size(Pos2::new(0.0, 0.0), egui::vec2(237.0, height))
    }

    /// **The two heads are the mock's boxes and the leftover is the groups'.**
    ///
    /// `.half-head` is `5 + 16.5 + 5` over its own `border-bottom`, and
    /// `.deck-head` is `5 + 15.5 + 5` with none — the stylesheet's own
    /// arithmetic for that row. Everything under them is the body.
    #[test]
    fn a_pane_is_two_heads_and_what_is_left() {
        let pane = one_node(2);
        let at = pane_box(pane_at(400.0), &pane.nodes, 0.0).expect("a pane with room in it");
        assert_eq!(at.head.height(), 27.5);
        assert_eq!(at.deck_head.height(), 25.5);
        assert_eq!(at.head.max.y, at.deck_head.min.y);
        assert_eq!(at.deck_head.max.y, at.body.min.y);
        assert_eq!(at.body.max.y, 400.0);
    }

    /// **The inspector's own minimum is the least this draws**, and the two
    /// are one derivation: a pane at the minimum less the bay head has room
    /// for exactly the one group of two parameters the minimum is written
    /// from, and a pixel less has room for none of it.
    ///
    /// This is what the 151.5 in `lib.rs` *means*: bay head 27, `.half-head`
    /// 27.5, `.deck-head` 25.5, `.node-head` 26.5 and two `.param` rows at
    /// 22.5.
    #[test]
    fn the_bays_minimum_is_the_least_a_pane_can_draw() {
        let pane = one_node(2);
        let pane_h = 151.5 - size::HEAD_H;
        let at = pane_box(pane_at(pane_h), &pane.nodes, 0.0).expect("a pane at the minimum");
        assert_eq!(
            at.shown, 1,
            "one node and two of its parameters is what the minimum is written from"
        );
        let short = pane_box(pane_at(pane_h - 0.5), &pane.nodes, 0.0).expect("still two heads");
        assert_eq!(
            short.shown, 0,
            "half a pixel under the minimum and the group no longer fits"
        );
    }

    /// **A group the pane cannot hold whole is not counted as shown**, which
    /// is what the head's `n of m` means since the pane started scrolling: it
    /// is drawn, as far as the pane goes, and it is not one of the ones the
    /// readout says are on screen.
    #[test]
    fn a_group_that_does_not_fit_whole_is_not_counted() {
        let pane = Pane {
            nodes: vec![one_node(2).nodes[0].clone(), one_node(5).nodes[0].clone()],
            ..one_node(2)
        };
        // Room for the first group, the hairline, and all but the last row of
        // the second.
        let first = size::NODE_HEAD_H + size::PARAM_H * 2.0;
        let second = size::NODE_HEAD_H + size::PARAM_H * 5.0;
        let body = first + size::HAIRLINE + second - size::PARAM_H;
        let at = pane_box(
            pane_at(size::HALF_HEAD_H + size::DECK_HEAD_H + body),
            &pane.nodes,
            0.0,
        )
        .expect("a pane with room in it");
        assert_eq!(
            at.shown, 1,
            "the second group is short by one row, so one is whole"
        );

        // One row more and both are drawn.
        let at = pane_box(
            pane_at(size::HALF_HEAD_H + size::DECK_HEAD_H + body + size::PARAM_H),
            &pane.nodes,
            0.0,
        )
        .expect("a pane with room in it");
        assert_eq!(at.shown, 2);
    }

    /// **Where a group goes is the sum of the groups above it and the rules
    /// between them**, and the rule is between rather than under: *n* groups
    /// carry *n - 1* of them, because `.node-group:last-child` has none.
    #[test]
    fn a_group_stacks_under_the_one_before_it_with_a_rule_between() {
        let pane = Pane {
            nodes: vec![
                one_node(2).nodes[0].clone(),
                one_node(1).nodes[0].clone(),
                one_node(3).nodes[0].clone(),
            ],
            ..one_node(2)
        };
        let at = pane_box(pane_at(400.0), &pane.nodes, 0.0).expect("a pane with room in it");
        assert_eq!(at.shown, 3);
        let first = at.group(&pane.nodes, 0);
        let second = at.group(&pane.nodes, 1);
        let third = at.group(&pane.nodes, 2);
        assert_eq!(first.min.y, at.body.min.y);
        assert_eq!(second.min.y, first.max.y + size::HAIRLINE);
        assert_eq!(third.min.y, second.max.y + size::HAIRLINE);
        assert_eq!(first.height(), size::NODE_HEAD_H + size::PARAM_H * 2.0);
        assert_eq!(second.height(), size::NODE_HEAD_H + size::PARAM_H);
    }

    /// **A renderer row costs a group its own height**, and a group without
    /// one costs nothing: the mock draws `.rend-row` in the `L4` group and
    /// nowhere else.
    #[test]
    fn a_renderer_row_is_a_row_of_the_group_it_is_in() {
        let mut node = one_node(1).nodes[0].clone();
        let without = group_h(&node);
        node.renderers = vec![Renderer {
            name: "soft_points".to_owned(),
            live: true,
        }];
        assert_eq!(group_h(&node), without + size::REND_ROW_H);
    }

    /// **The anchor is the mock's own two spellings, and free shows neither.**
    ///
    /// *"`T128` is the tempo a deck was engaged at … `B128 +0.25` is that with
    /// the deck sitting a quarter beat ahead of the room. A free deck shows
    /// neither."*
    #[test]
    fn the_anchor_reads_what_the_mock_reads() {
        let mut pane = one_node(1);
        pane.sync = Sync::Tempo;
        pane.scrub_beats = 0.25;
        assert_eq!(
            anchor_text(&pane).as_deref(),
            Some("T128"),
            "tempo sync does not read the offset, so it is not drawn"
        );
        pane.sync = Sync::Beat;
        assert_eq!(anchor_text(&pane).as_deref(), Some("B128 +0.25"));
        pane.sync = Sync::Free;
        assert_eq!(
            anchor_text(&pane),
            None,
            "free is the absence of a transport rather than a setting"
        );
    }

    /// **The pane head names the deck it is pointed at and what is in it**,
    /// which is the mock's `deck A · drift_night`.
    #[test]
    fn the_pane_head_says_which_deck_it_is_showing() {
        let mut pane = one_node(1);
        pane.deck = 1;
        assert_eq!(showing_text(&pane), "deck B · drift_shell + soft_points");
    }

    /// **Every level the vocabulary names is on the node head.**
    ///
    /// `karakuri_operation::Authority` carries no `ALL` — *"it arrives with
    /// the first reader"* — so [`AUTHORITIES`] is this crate's list, and the
    /// thing that can go wrong is the list falling behind the vocabulary while
    /// [`auth_word`] is updated. This holds the two together: every level
    /// [`auth_word`] can spell is in the array exactly once, and the array is
    /// in the vocabulary's own declaration order.
    #[test]
    fn every_authority_the_vocabulary_names_is_on_the_node_head() {
        // A `match` that a fourth level would not compile past, which is what
        // makes this a check on the *array* rather than on the enum.
        let expected = [
            Authority::Manual,
            Authority::Suggesting,
            Authority::Automatic,
        ];
        assert_eq!(
            AUTHORITIES.len(),
            expected.len(),
            "a level the vocabulary names is missing from the node head"
        );
        for (n, level) in expected.into_iter().enumerate() {
            assert_eq!(AUTHORITIES[n], level, "the three are in declaration order");
            assert!(
                !auth_word(level).is_empty(),
                "every level has the console's own abbreviation for it"
            );
        }
        let words: Vec<&str> = AUTHORITIES.into_iter().map(auth_word).collect();
        assert_eq!(
            words,
            vec!["man", "sug", "auto"],
            "the manual's node head reads `man / sug / auto`"
        );
    }

    /// **A pane narrower than a parameter row's own padding is no pane**,
    /// which is the picture's rule stated across the axis.
    #[test]
    fn a_pane_with_no_room_across_it_draws_nothing() {
        let pane = one_node(2);
        let narrow = Rect::from_min_size(
            Pos2::new(0.0, 0.0),
            egui::vec2(size::PARAM_PAD_L + size::PARAM_PAD_R, 400.0),
        );
        assert!(pane_box(narrow, &pane.nodes, 0.0).is_none());
    }

    /// **There are two panes and there is no third.**
    ///
    /// The mock's `2 up ▾` is an operator choosing how many while running, and
    /// a [`Spec`](karakuri_layout::Spec) builds a
    /// [`Layout`](karakuri_layout::Layout) once — so the count is the
    /// arrangement's, and a caller asking for a pane it has not got gets
    /// `None` rather than one of the two it has.
    ///
    /// And a console with no deck behind it hands over no pane at all, which
    /// is every test in this crate.
    #[test]
    fn there_are_two_panes_and_no_third() {
        let mut layout = crate::layout();
        layout.set_viewport(karakuri_layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 1920.0,
            h: 1080.0,
        });
        layout.solve();
        let pane = one_node(2);
        assert_eq!(PANES, 2, "the mock's `.insp-split` is `1fr 9px 1fr`");
        for index in 0..PANES {
            assert!(
                inspector(&layout, index, &pane, 0.0).is_some(),
                "pane {index} is in the arrangement and has room in it"
            );
        }
        assert!(
            inspector(&layout, PANES, &pane, 0.0).is_none(),
            "a third pane is a pane the arrangement has not got"
        );
        assert!(View::new(Room::Day).inspector.is_empty());
    }

    /// **A pane starts under the bay head and not on top of it.**
    ///
    /// The one thing `pane_box`'s own tests cannot see. They are handed a
    /// rectangle and the head has already been taken off it — the fixture
    /// says so in the arithmetic, `151.5 - size::HEAD_H` — so every one of
    /// them passes whether or not the caller does the subtraction. It did
    /// not: `.half-head` was drawn at the top of `inspector-1`, which is the
    /// top of the bay, which is where [`bay_head`] paints the word
    /// `Inspector`. Read against the arrangement rather than against a
    /// fixture, because the fixture is what could not tell.
    #[test]
    fn a_pane_starts_under_the_bay_head() {
        let mut layout = crate::layout();
        layout.set_viewport(karakuri_layout::Rect {
            x: 0.0,
            y: 0.0,
            w: 1920.0,
            h: 1080.0,
        });
        layout.solve();
        let pane = one_node(2);
        let bay = to_egui(layout.rect(layout.find("inspector").expect("the bay is named")));
        for index in 0..PANES {
            let at = inspector(&layout, index, &pane, 0.0).expect("a pane with room in it");
            assert_eq!(
                at.head.min.y,
                bay.min.y + size::HEAD_H,
                "pane {index} starts where the bay head ends"
            );
            assert!(
                bay.contains_rect(at.head) && bay.contains_rect(at.deck_head),
                "pane {index} draws inside the bay it is in"
            );
        }
    }
}
