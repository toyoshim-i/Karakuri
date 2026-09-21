use super::*;

// ---------------------------------------------------------------------------
// The View struct
// ---------------------------------------------------------------------------

pub struct View {
    pub room: Room,
    /// Whether the external projector window is open (ADR-0156).
    pub projector: bool,
    /// What to draw in the Program bay's picture this frame, or `None` for a
    /// console with no engine behind it — which is every test in this crate and the
    /// whole of what `cargo test -p karakuri-console` sees.
    ///
    /// Set per frame by whoever owns the device, because that is who knows whether
    /// the texture it names is still the right size. A stale id here is a freed
    /// registration, so it is written beside the frame that made it rather than
    /// kept.
    pub picture: Option<Picture>,
    /// What to draw in each of the four deck preview cells this frame, in slot
    /// order, or `None` for a cell with no deck slot behind it.
    ///
    /// `None` is not "that deck is off air". A cell shows its slot's own material
    /// whatever the slot's residency — a parked deck's still and a warming deck's
    /// picture are the two an operator most needs to see, which is
    /// [ADR-0258](../../../../docs/adr/0258-the-look-comes-before-the-fader-so-a-cell-draws-every-slot-and-says-which-nothing-it-is.md).
    /// `None` is a cell with nothing to sample at all: a deck of fewer slots than
    /// there are cells, or a console with no engine behind it.
    ///
    /// The same seam as [`View::picture`], four times over: registering a texture
    /// takes a device, this crate has none, so whoever owns the device registers
    /// them and writes this per frame beside the frame that made them. A stale id
    /// here is a freed registration. Every test in this crate leaves every entry
    /// `None`, which is what `cargo test -p karakuri-console` sees and is a console
    /// with no engine behind it.
    ///
    /// All `None` is a state, not an absence, and the caption is where it is said:
    /// a cell with nothing behind it reads [`PREVIEW_NO_SLOT`] under its letter
    /// rather than going blank. It is deliberately not the manual's *empty* — a
    /// slot that exists with nothing loaded into it is a state the engine cannot be
    /// in — and deliberately not *off*, which was residency and has not gated a
    /// cell since ADR-0240. See [`state_word`], the module documentation, and
    /// [`preview_rects`] for where the rectangles come from.
    pub previews: [Option<Picture>; DECKS],
    /// Which of the four slots have stopped updating, in slot order, and `false`
    /// for every cell with nothing behind it — which is every test in this crate
    /// and is a console with no engine behind it.
    ///
    /// A slot is stopped when the version in it costs more than one frame may
    /// (`karakuri_engine::deck::Deck::overloaded`, ADR-0316). The engine then skips
    /// that slot's step and its draw, so its target holds the last image it made
    /// and this cell goes on showing it. The caption is where that is said —
    /// [`PREVIEW_OVERLOADED`] in place of [`PREVIEW_MATERIAL`] — because the
    /// picture cannot say it: a held frame of good material looks like material,
    /// and an unmarked still is a preview that lies
    /// ([ADR-0269](../../../../docs/adr/0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md)).
    ///
    /// A `bool` beside the picture rather than a third state of it. The two keep
    /// different clocks, exactly as [`View::costs`] does: a picture is a texture
    /// registration rewritten every frame by whoever owns the device, and this
    /// changes when a build lands. It is also not a residency and must not be read
    /// as one — a stopped Live slot is still in the mix.
    ///
    /// The same seam every value here crosses: `src/` takes no engine (ADR-0156),
    /// so whoever holds the deck reads it and writes this.
    pub overloaded: [bool; DECKS],
    /// What the governor budgeted each of the four slots at, in slot order, or
    /// `None` for a slot it has no number for — which is every test in this crate
    /// and is a console nobody has governed.
    ///
    /// The risk badge is read from this and from nothing else. [`band_of`] turns
    /// the number into one of five bands and [`caption_into`] draws the dot; `None`
    /// draws no dot at all, which is the manual's own state for a slot with no cost
    /// and is not a hollow one.
    ///
    /// A second field rather than a third member of [`Picture`], and the two halves
    /// of a cell keep different clocks on purpose. A picture is a texture
    /// registration and is rewritten every frame by whoever owns the device; a cost
    /// is `karakuri_engine::governor::Report`, which is taken on a governor pass
    /// and not on a frame — before the first frame, and again whenever a residency
    /// is written. Folding the cost into `Picture` would make every frame's texture
    /// aim carry a number it did not take, and the number would be dropped and
    /// re-fetched sixty times a second to no purpose.
    ///
    /// Where it comes from. One entry per `Decision` in `Report::decisions`, at
    /// `Decision::slot`: `budgeted_ms` and `Decision::basis`, with
    /// `governor::Basis::Unbudgetable` written as `None` — that is a slot nothing
    /// measured and nothing estimated, and it is *not* a zero. `src/` takes no
    /// engine (ADR-0156), so whoever holds the deck reads the report and writes
    /// this, exactly as [`View::previews`] is written by whoever holds the device.
    ///
    /// It is not gated on residency and it is not gated on the picture here. A
    /// parked deck is the one whose cost an operator most wants, because the cost
    /// is why it is parked; the gate that does exist is [`caption_into`]'s, which
    /// is the mock's *no slot is no cost*.
    pub costs: [Option<Budgeted>; DECKS],
    /// What the transport row reads this frame, or `None` for a console with no
    /// engine behind it — which is every test in this crate, and what the row draws
    /// then is nothing at all.
    ///
    /// The same seam as [`View::picture`], one row up and without a device: the
    /// tempo, the beat and the frame's cost are a clock and an engine, and `src/`
    /// has neither (ADR-0156). So whoever owns them reads them and writes this per
    /// frame, beside the frame that measured it. See [`Transport`] and
    /// [`transport`].
    pub transport: Option<Transport>,
    /// What the arrangement pill in that row reads, and what its menu is doing.
    ///
    /// Half of it is the same seam as [`View::transport`] and half of it is not,
    /// which is the one field here that is both. The name in use and the names
    /// filed are a file and a directory, so whoever owns the store reads them and
    /// writes them here — [`View::library`]'s seam, one row up. The menu is this
    /// crate's own and moves only through [`Arrangement`]'s methods: what the
    /// *control* is doing is not something the program can be the model of record
    /// for.
    ///
    /// [`Arrangement::NONE`] until somebody says otherwise, which is every test in
    /// this crate and is a console with no store behind it: the default
    /// arrangement, nothing filed, and the menu shut. See [`Arrangement`] and
    /// [`arrangement`].
    pub arrangement: Arrangement,
    /// What the audio-in pill in that row reads, and whether its card is down — or
    /// `None` for a console nobody has told anything about audio, which is every
    /// test in this crate and draws no pill at all.
    ///
    /// The same two halves [`View::arrangement`] has, over a device instead of a
    /// file: which input is open and what inputs there are are a microphone and an
    /// enumeration of a host, and `src/` has neither (ADR-0156) — so whoever opened
    /// one writes them here. Whether the card is down is this crate's and moves
    /// only through [`AudioIn`]'s methods.
    ///
    /// `Option`, where the arrangement is not, and the difference is real: every
    /// console has an arrangement — the default one, which is what is on screen —
    /// and a console has an *input* only if somebody opened one and said so.
    /// `Some(AudioIn::NONE)` is a program that looked and found nothing, and it
    /// draws `audio-in · none`; `None` is a program that never said, and a pill
    /// drawn for it would be this crate answering a question about a device on its
    /// own authority. See [`audio_in`].
    pub audio: Option<AudioIn>,
    /// Whether MIDI learn is armed — the transport row's `learn` pill, lit while
    /// this is true.
    ///
    /// This crate's own state, and the only console state on this row that is
    /// ([`AudioIn`]'s card is the other). Arming is a mode and it is the one thing
    /// rule 04 lets be a mode, because the pill *is* the readout: it is lit for
    /// exactly as long as the mode is on, and nothing else on the panel changes
    /// meaning while it is
    /// ([ADR-0336](../../../../docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md)).
    ///
    /// A `bool` and not an `Option`, where [`View::map`] beside it is an `Option`,
    /// and the difference is the same one `audio` draws: whether a mode is armed is
    /// a fact this crate owns outright, and *which map is loaded* is a fact about a
    /// file this crate cannot see.
    ///
    /// What it does not do is bind anything. The gesture is the host's — what the
    /// pointer is on, what arrived on the wire, and what goes in the file are all
    /// outside this crate (ADR-0156). This is the arming and the lamp.
    pub learn: bool,
    /// Which map is loaded, for the transport row's `map` pill, or `None` for a
    /// console nobody has told — which is every test in this crate and every run
    /// with no surface.
    ///
    /// It is [`View::audio`]'s shape one pill along and for its reason: a map is a
    /// *file*, `src/` reads none (ADR-0156), so whoever loaded one writes its name
    /// here. `Some(MapPill::NONE)` is a program with a surface and no map, which
    /// draws `map · none`; `None` draws no pill at all.
    pub map: Option<MapPill>,
    /// What the other three controls in the tracker group read this frame — the
    /// latency offset the open session is holding, and which way the grid can still
    /// be moved an octave — or `None` for a console nobody has told anything about
    /// the beat tracker, which is every test in this crate that does not say
    /// otherwise and draws none of the three.
    ///
    /// What the audio-in tracker in the Transport bay reads this frame, or `None`
    /// for a console where no input is open or with no engine behind it.
    ///
    /// [`View::look`]'s argument beside [`View::transport`]: the pill is *which
    /// room is being heard*, and this is *what the tracker is doing with it*. The
    /// two are `Some` together in every program that draws this row, and a type
    /// that could only say them together would be answering one question with two.
    /// See [`Tracker`] and [`tracker_group`].
    pub tracker: Option<Tracker>,
    /// Active look parameters (`karakuri_engine::frame::Look`) for this frame, or `None` (ADR-0156).
    pub look: Option<Look>,
    /// What the Master bay's out row reads this frame, or `None` for a console with
    /// no engine behind it — which is every test in this crate that does not hand
    /// one in, and what the bay draws then is its card and its head.
    ///
    /// The same seam as [`View::look`], one bay away and one level along the same
    /// chain: this is `karakuri_engine::deck::Deck::out`, which is applied where
    /// the mix *writes* the composited frame, and the look is applied where the
    /// present pass *reads* it. `src/` has no engine (ADR-0156), so whoever owns
    /// one reads it and writes this per frame beside the frame it was drawn under.
    ///
    /// A bare level rather than a struct, and it stays one now that the chain
    /// exists: this is the level at the chain's *entry*, and what the chain is set
    /// to is [`View::master_chain`] beside it. The two are two fields for the
    /// reason they are two records — one is a level a fader rides and one is a set
    /// of settings a press moves
    /// ([ADR-0224](../../../../docs/adr/0224-out-and-exposure-are-two-levels-that-multiply-in-different-places.md),
    /// [ADR-0317](../../../../docs/adr/0317-the-master-chain-is-three-fixed-passes-and-feedback-reads-either-cut.md)).
    pub master_out: Option<f32>,
    /// What the master chain is running this frame, or `None` for a console with
    /// no engine behind it — in which case the bay draws the out row and nothing
    /// under it.
    ///
    /// [`View::master_out`]'s seam exactly, one row down:
    /// `karakuri_engine::present::Present::chain_reading` is what a harness reads
    /// it from, and [`Chain`] is that value mirrored into a crate with no engine
    /// in it. A chain with no slot in it is `Some` with an empty list, which is
    /// the default chain: the bay then draws the out row and `+ add`.
    pub master_chain: Option<Chain>,
    /// Whether the master chain is currently building on the background worker thread.
    /// When true, the Master bay draws an in-flight building indicator badge.
    pub master_chain_building: bool,
    /// What the Master bay's `+ add` offers this frame: one entry per `kind L5`
    /// procedure the library holds, in the order the library lists them.
    ///
    /// Written by the host per press beside [`View::library`]: an entry carries
    /// the content address of a procedure's source and this crate reads no store
    /// (ADR-0156). Empty is a library listing no `kind L5` procedure, and the
    /// card does not go down on one.
    ///
    /// It is also what a carried Library row is resolved through at a release over
    /// the chain — a row this list does not name is not a `kind L5`, and the drop
    /// is refused with that reason.
    pub chain_add: Vec<AddChoice>,
    /// What each mixer strip reads this frame, one per slot the deck has, in slot
    /// order — and empty for a console with no deck behind it, which is every test
    /// in this crate and what the bay draws then is nothing at all.
    ///
    /// The same seam as [`View::transport`], and empty rather than `Option<Vec<_>>`
    /// because an empty list of strips is already the whole of *no deck*: a deck
    /// has one to four slots (`MAX_SLOTS`, asserted in `Deck::new`), so there is no
    /// deck that has none and no second way to say it.
    ///
    /// A `Vec` a caller keeps and rewrites, rather than a fixed array of `Option`s
    /// like [`View::previews`]: a preview cell is off or on and the row is always
    /// four, where the strips are *as many as the deck has* and a `None` in the
    /// middle of them would be a slot no `Deck` can have. [`View::new`] gives it
    /// room for [`DECKS`] so the frame path never grows it. See [`Strip`] and
    /// [`mixer`].
    pub mixer: Vec<Strip>,
    /// Whether the mixer bay needs a redraw due to an operation changing mixer state.
    pub mixer_dirty: bool,
    /// What the Library bay lists this frame: the name of every Set the store
    /// holds, in the order the store listed them — and empty for a console with no
    /// store behind it, which is every test in this crate and what the bay draws
    /// then is nothing at all.
    ///
    /// The same seam as [`View::mixer`], and empty rather than `Option<Vec<_>>` for
    /// the same reason: an empty listing is already the whole of *no library*, and
    /// a store that holds nothing and a store that is not there are the same bay —
    /// one with no row to draw.
    ///
    /// Read once rather than per frame, by whoever owns the store. A listing is a
    /// directory read, which is not a thing to do on a frame path (P-0091), and
    /// nothing in this crate can do it anyway: opening a store is
    /// `karakuri-store`'s and `src/` depends on neither it nor the engine
    /// (ADR-0156). What crosses the seam is a list of names.
    ///
    /// A name and nothing else, because a name is what exists: see [`library`] for
    /// the star and the time the mock draws beside it, and for why neither is here.
    ///
    /// It is the listing of whichever scope is marked, and not of the store: the
    /// bay lists `my sets` where that chip is marked and the presets root's `.kset`
    /// files where that one is, and both are a directory read the host does on the
    /// press that changed the scope. See [`View::scopes`] and [`Scope`].
    pub library: Vec<String>,
    /// What each row of [`View::library`] is, in the same order — a Set or a
    /// procedure, and the layers its badge names.
    ///
    /// Empty is every row a Set with no badge, which is what this bay drew before
    /// ADR-0338 and is every test in this crate that does not say otherwise. So a
    /// host that has not been changed to answer it draws exactly what it drew, and
    /// a host that has hands the two halves over on one press — see [`RowKind`],
    /// where that default is argued, and [`View::rows`], which is what every
    /// control here reads.
    ///
    /// The same seam as [`View::library`]: a Set's layers are its file's own `slot`
    /// records and a procedure's kind is the `kind` line of a source, both of which
    /// are a directory read on the press that builds a listing — and this crate
    /// reads no store (ADR-0156, P-0091).
    ///
    /// Shorter than the listing is not an error, for [`View::starred`]'s reason
    /// read the other way: a row past the end of it is a Set with no badges, which
    /// is a row this console can draw and act on.
    pub kinds: Vec<RowKind>,
    /// Which Sets this store has starred, as their ids — and empty for a console
    /// with no store behind it, which is every test in this crate that does not say
    /// otherwise and is a listing whose every row draws a hollow star.
    ///
    /// The same seam as [`View::library`], in the same rows: the marks are
    /// `<store>/favourites.json` beside the Sets (ADR-0299), and this crate reads
    /// no store (ADR-0156). So the host reads them on the same press the listing is
    /// built on and hands over the set of ids.
    ///
    /// A `BTreeSet` and not a `Vec<bool>` beside the listing. The question a row
    /// asks is *is this one starred*, once per row, and two vectors of the same
    /// length are two vectors that can disagree about it — where a set of ids
    /// answers the same question about a listing that was rewritten under it
    /// without being wrong, and is what `karakuri_store::Store::favourites` already
    /// hands back.
    ///
    /// Ids the listing does not hold are not an error here. `my sets` is the
    /// intersection of these with what the store holds and the host is what takes
    /// it; a mark left behind by a file somebody deleted draws no row and is not
    /// this field's to prune.
    pub starred: std::collections::BTreeSet<String>,
    /// Which Set the load pulldown's deck is running, as the host reads it off that
    /// deck's aim — and `None` for a deck playing the pair the run was launched
    /// with, which is where every run starts.
    ///
    /// It is here because a walk names a Set and this console cannot spell one.
    /// [`Operation::WalkHistory`] carries the id of the history it is a walk of;
    /// the id rides the aim a library load sends and this crate reads no engine
    /// (ADR-0156), so the host answers it here, exactly as it answers the rows of
    /// that walk into [`View::library`] — see [`Chosen::asked`], which is the one
    /// place this is read.
    ///
    /// A readout rebuilt per frame, like [`View::mixer`] beside it, rather than
    /// written on the press that changes the aim: the pulldown's deck can be
    /// re-pointed by a load, by a key, by a mapped control and by a model, and a
    /// value written at one of those four is a value stale after the other three.
    ///
    /// The deck it is read off is the load pulldown's and not the selection's
    /// ([`View::target_deck`], ADR-0305): the walk is drawn under the pulldown that
    /// says which deck the bay is preparing, and the landing it feeds lands there.
    pub aimed: Option<String>,
    /// Which libraries this console has to offer, in the order the chips are drawn
    /// — and empty for a console nobody has told, which is every test in this crate
    /// that does not say otherwise and what the bay then draws is no scope row at
    /// all.
    ///
    /// The same seam as [`View::library`], one row up in the same bay: what a scope
    /// can be asked is a store, a told directory and a directory somebody names
    /// during the run, and this crate has none of the three (ADR-0156). So the host
    /// says which there are and answers the marked one into [`View::library`].
    ///
    /// Empty rather than [`Scope::ALL`] by default, which is [`View::mixer`]'s rule
    /// and not a shortage: a console that has been told nothing has no libraries
    /// rather than four it cannot answer, and a default here would be this crate
    /// asserting that a presets root and a folder exist on a machine it cannot look
    /// at.
    pub scopes: Vec<Scope>,
    /// What the `holds` field can be stepped to, in the order it steps them — and
    /// empty for a console nobody has told, which is every test in this crate that
    /// does not say otherwise and is a field a press asks the listing again
    /// through.
    ///
    /// The same seam as [`View::scopes`], two rows up in the same bay: `holds` is
    /// matched against what a Set's nodes are called, which is
    /// `karakuri_environment::setfile::summarise`'s reading of the store, and this
    /// crate reads no store (ADR-0156). So the host says what there is to narrow by
    /// and the console steps through it.
    ///
    /// Written on the same read as [`View::library`] and off the same summary,
    /// because the two are one directory read: the rows are the Sets that matched
    /// and these are the names any of them could be matched by. They are the
    /// candidates of the *unnarrowed* listing, so the row a press steps to does not
    /// depend on what the field is already set to — candidates read off a filtered
    /// listing would shrink as the filter bit, and a step would then wander
    /// somewhere it could not come back from.
    pub holds: Vec<String>,
    /// Which directory this library is pointed at, as the host spells it — and
    /// `None` until a folder has been dropped on this window, which is where every
    /// run starts and is a bay with no `.path` row at all.
    ///
    /// The same seam as [`View::library`], one row up in the same bay: a directory
    /// is a thing on a disk, this crate reaches no disk (ADR-0156), and what
    /// crosses is the line to draw. The host writes it on the drop that chose it
    /// and never on a frame — a drop is one act, and asking the file system what a
    /// path is is not a thing to do per frame (P-0091).
    ///
    /// It is not a scope and it is not `Scope::Folder`. The row is drawn whichever
    /// chip is marked, because it is also where a send's save dialog opens
    /// (ADR-0311, superseding ADR-0267's *where a send lands*); what the folder
    /// scope's *listing* is arrives in [`View::library`] like every other scope's.
    ///
    /// It is a `String` rather than a `PathBuf` for the same reason the listing is:
    /// this crate never opens it, so what it needs is the spelling. See
    /// [`Pointed`].
    pub folder: Option<String>,
    /// The path a release would set, while a folder is over the window — and `None`
    /// whenever no drag is over it, which is nearly always.
    ///
    /// Beside [`View::folder`] rather than inside it, because the two are different
    /// facts: one is where this bay *is* pointed and the other is where it *would
    /// be*. A drag that leaves the window without being let go clears this and
    /// leaves the other standing, which is the row going back to what it said — see
    /// [`View::pointed`], where the two become the one row the mock draws.
    ///
    /// Written per frame by whoever reads the platform's hover, which is the one
    /// thing about this bay that is a frame's business: `egui` clones the hovered
    /// files onto every pass while a drag is over the window, so there is no event
    /// to hang it off. Nothing is asked of the file system for it (P-0091) —
    /// whether the path is a folder is the drop's question and not the hover's, and
    /// the row says nothing about whether the release will be allowed (ADR-0275).
    pub incoming: Option<String>,
    /// What the Staging lane lists this frame: one candidate per deck slot whose
    /// newest build has a verdict outstanding or whose file no longer agrees with
    /// its picture — and empty for a console with no engine behind it, which is
    /// every test in this crate that does not hand one in and what the bay draws
    /// then is its card and its head.
    ///
    /// The same seam as [`View::mixer`], and empty rather than `Option<Vec<_>>` for
    /// the same reason: an empty lane is already the whole of *nothing waiting*,
    /// and a run in which nobody has rewritten a procedure and a run with no
    /// producer at all are one bay — one with no row to draw. Empty is this lane's
    /// ordinary state, which is what makes it different from every other bay here:
    /// a library with nothing in it is a library nobody has filled.
    ///
    /// Written when an event arrives rather than per frame, by whoever drains
    /// `karakuri_engine::deck::Deck::events` — which is a `Vec` the engine only
    /// appends to when a build lands, is refused or is judged, and which a caller
    /// that never drains grows for the rest of the run. So a frame on which nothing
    /// was swapped touches nothing here, and the name a row carries is rewritten
    /// only when the row's own build changes. See [`Candidate`] and [`staging`],
    /// which is also where the four things the mock's row has and this does not are
    /// named.
    pub staging: Vec<Candidate>,
    /// What each Inspector pane is showing this frame, one per pane the console has
    /// room to point at something — and empty for a console with no deck behind it,
    /// which is every test in this crate and what the bay draws then is its card,
    /// its head and the bar between its panes.
    ///
    /// The same seam as [`View::mixer`], and empty rather than `[Option<Pane>;
    /// PANES]` for the same reason: an empty list is already the whole of *no
    /// deck*, and a pane pointed at nothing and a pane that is not there are one
    /// bay — one with no head to draw.
    ///
    /// A pane is *pointed*, not choosing. The mock's `showing … ▾` is a control and
    /// this pass adds none (ADR-0200), so which deck each pane shows is whoever
    /// fills this saying so — and pane *i* draws `inspector[i]`, in [`PANE_NAMES`]'
    /// order.
    ///
    /// Written when a Set lands rather than per frame, which is
    /// `karakuri_engine::set::Set::published`'s own instruction — *"Allocates, so
    /// not the frame path. A console reads this when a Set lands, not per frame."*
    /// — and is [`View::staging`]'s rule read one bay along: the two are written on
    /// the same frames and off the same drain, because a candidate row and a pane's
    /// rows are two readings of one event. Between landings what is in here does
    /// not move: nothing in this workspace writes a published value, binds a signal
    /// in the panel binary, or grants an authority (ADR-0216), so a Set that has
    /// landed reads the same on every frame after it until the next one does. The
    /// field is rewritable per frame like every other one; what the `man / sug /
    /// auto` chip still waits on is a writer for the authority, which is a
    /// different gap. See [`Pane`] and [`inspector`].
    pub inspector: Vec<Pane>,
    /// Program bay rendering dimensions `(width, height)` used for layout arrangement (ADR-0156).
    pub canvas: (u32, u32),
    /// Which classes the operator has opened to a model, written per frame by
    /// whoever holds the run's opening.
    ///
    /// Handed in like every other value here, and for the sharper version of the
    /// usual reason: the model of record is a `karakuri_environment::Opening`,
    /// which is a handle a *second* surface reads on every MCP call — so a copy
    /// kept in this crate would be the console answering on the server's behalf,
    /// and would go on saying *open* after something else shut it. This crate names
    /// [`karakuri_operation::gate::Open`] and nothing in `karakuri-environment` at
    /// all (ADR-0156); [`McpPill::next`] hands a value back and whoever owns the
    /// handle writes it.
    ///
    /// [`Open::CLOSED`] until somebody says otherwise, which is every test in this
    /// crate and is the state ADR-0235 says a run starts in: four classes shut, and
    /// no way to write down an `Open` that starts open.
    pub opening: Open,
    /// Slot-level MCP modification policy for each deck slot.
    pub slot_policies: [SlotPolicy; DECKS],
    /// How long the panel has been animating, written per frame by whoever has the
    /// clock — see [`Phase`], which carries the whole argument for why this is a
    /// value and not an `Instant`.
    ///
    /// The same seam as [`View::transport`], and the one this crate is least able
    /// to cross: a clock in `src/` is P-0092 broken in the file whose own doc says
    /// so. [`Phase::ZERO`] until somebody says otherwise, which is every test in
    /// this crate and is a console with no clock behind it — a panel drawn at the
    /// origin of every animation on it.
    pub phase: Phase,
    /// Which bay the keyboard is talking to, and what each bay remembers — the
    /// pointer this console owns, and the one three of its fields used to be
    /// ([`crate::focus`]).
    ///
    /// The deck selection, the library cursor and the marked scope are three
    /// readings of this, on two bays: the Mixer's remembered item is the deck the
    /// keys are addressed to, the Library's is the row under the cursor, and the
    /// control last named under the Library's head is the scope. Each of the three
    /// was a private field with the same paragraph written at it — *"nothing
    /// downstream can be the model of record for it, and a host that kept a copy
    /// would be keeping the console's state on its behalf"* — and that paragraph is
    /// written once now
    /// ([ADR-0259](../../../../docs/adr/0259-the-keyboard-is-addressed-to-the-bay-that-has-focus-and-a-global-letter-is-a-convenience-or-the-operators-own.md),
    /// [ADR-0332](../../../../docs/adr/0332-focus-is-a-pointer-the-console-owns-and-the-three-pointers-are-instances-of-it.md)).
    /// Nothing about what any of the three means changes, which is the record's own
    /// clause: what each of them refuses is still refused where it was refused, by
    /// [`View::select`], [`View::walk`] and [`View::select_scope`].
    ///
    /// Private, with [`View::select`], [`View::walk`], [`View::point_at`],
    /// [`View::select_scope`], [`View::step_scope`], [`View::tab`] and
    /// [`View::focus_up`] the only ways in, which is [`View::arrangement`]'s rule:
    /// what a *pointer* is at is not something the program can be told, because the
    /// console is what refuses a deck there is no strip for.
    ///
    /// Read by [`mixer::mixer_into`] for the solid ring, by [`View::focus_mark`]
    /// for the dashed one, and by nothing else on the panel. The Library bay's foot
    /// read the selection for its letter until 2026-09-08 and reads
    /// [`View::target`] now (ADR-0305), which is what lets a load be aimed at a
    /// deck the keys are not addressed to; `console.html`'s crossfader read it as
    /// well, and there is no crossfader. What still fills its `deck` in from there
    /// is every deck-addressed *key*, `l` included.
    pub(crate) focus: Focus,
    /// Deck targeted by Library bay load controls (ADR-0305).
    pub(crate) target: u8,
    /// Whether the pulldown's list is down, and it is the console's own state
    /// rather than a reading — [`Arrangement::menu`]'s argument on a third control:
    /// what a *control* is doing is not something the program can be the model of
    /// record for.
    ///
    /// Private, with [`View::open_target`] and [`View::shut_target`] the only ways
    /// in. `open_target` refuses a console the mixer is drawing no strip for: a
    /// card with no rows in it is a gesture with nothing to pick and nothing to
    /// leave by, and `input::claim`'s rule 2 would give it every press on the
    /// console until a second one shut it.
    pub(crate) target_open: bool,
    /// Which `uses` line's card is down, as `(pane, node, input)` — or `None` with
    /// none of them open, which is where every run begins.
    ///
    /// The console's own state, exactly as [`target_open`] beside it is. A card
    /// being down changes what the *next press* reaches and nothing about any deck,
    /// so no operation names it: opening a list is not something a map or a model
    /// could ever want to say, which is ADR-0305's argument for the pulldown one
    /// bay along and is this one's unchanged.
    ///
    /// One at a time, and one for the whole console. Two cards down at once would
    /// put two rectangles over the same pane with `input::claim`'s rule 2 giving
    /// each of them every press; and a card belongs to a pane, a node and an input
    /// together, which is why the three travel as one value rather than as three
    /// fields that can disagree.
    ///
    /// [`target_open`]: Self::target_open
    pub(crate) wiring_open: Option<(usize, usize, usize)>,
    /// Whether the `+ lane` chooser's card is down, and it is the field above one
    /// bay along — the console's own state rather than a reading, on
    /// [`Arrangement::menu`]'s argument: what a *control* is doing is not something
    /// the program can be the model of record for.
    ///
    /// Private, with [`View::open_lane`] and [`View::shut_lane`] the only ways in,
    /// and `open_lane` refuses a chooser with nothing in it for `open_target`'s
    /// reason: `input::claim`'s rule 2 would give an empty card every press on the
    /// console until a second one shut it.
    pub(crate) lane_open: bool,
    /// Whether the Master bay's `+ add` chooser is down — [`View::lane_open`]'s
    /// field one bay along. [`View::open_chain_add`] and
    /// [`View::shut_chain_add`] are the only ways in.
    pub(crate) chain_add_open: bool,
    /// Which row of the Library bay's list has its menu down, or `None` for none —
    /// the console's own state, exactly as [`target_open`] beside it is, and not a
    /// fourth mark: a menu is a card that is there or is not, and the row under it
    /// draws the same either way.
    ///
    /// One field where the pulldown takes two, and that is the whole of the
    /// difference between the two cards: a pulldown is a capsule that is drawn
    /// whether or not its list is down, so *which deck* outlives *is it open*; a
    /// row menu is the card and nothing else, so an open menu with no row under it
    /// cannot be spelled here.
    ///
    /// Private, with [`View::open_menu`] and [`View::shut_menu`] the only ways in.
    /// `open_menu` does not refuse a console the mixer is drawing no strip for,
    /// where `open_target` does: this card carries `Save as a kbset` under the
    /// separator whatever the mixer is doing, so there is always something to pick
    /// and something to leave by.
    ///
    /// [`target_open`]: Self::target_open
    pub(crate) menu_row: Option<usize>,
    /// How far down its listing the Library bay is scrolled, in logical pixels, as
    /// it is stored — the number [`library`] clamps and never the one it clamped.
    ///
    /// [`View::scroll`]'s shape one bay over and for its reasons, which are worth
    /// reading as a pair rather than restated here: the position is the console's
    /// own, no operation names it, nothing outside this crate could be the model of
    /// record for it, and it is not part of the arrangement — `karakuri-layout`'s
    /// tree holds sizes and folds and holds no scroll position
    /// ([ADR-0307](../../../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md),
    /// [ADR-0312](../../../../docs/adr/0312-the-params-pill-is-a-toggle-and-the-library-bay-scrolls.md)).
    ///
    /// One and not one per scope. A scope is a different listing in the same bay,
    /// the way a build landing is a different pane's contents one bay over — and
    /// unlike the Inspector's two panes, which are two places in one list at once,
    /// only one scope is ever being read. A position per scope would be five
    /// numbers of which four are always stale, and the chip press that changes the
    /// listing is exactly the moment an operator wants the top.
    ///
    /// Private, with [`View::library_scroll`] and [`View::scroll_library_by`] the
    /// only ways in.
    pub(crate) library_scroll: f32,
    /// Active Set reading under inspection in Library bay (ADR-0156, P-0091).
    pub(crate) reading: Option<Reading>,
    /// Which of [`View::holds`] the `holds` field is set to, or `None` for a field
    /// nobody has set — the fifth of this console's pointers.
    ///
    /// A position and not a `String`, for [`View::scope`]'s reason one field along:
    /// what the field can be set to is the row of candidates the host handed in, so
    /// what a pointer into it can be is a place in that row.
    ///
    /// A stale position reads as unset rather than as the last candidate, where
    /// [`View::scope`] and [`View::cursor_row`] both clamp. The two of them point
    /// at something drawn — a chip, a row — and the nearest one is the right
    /// answer; this one *narrows a listing*, and clamping it would leave the bay
    /// hiding Sets under a filter nobody chose.
    pub(crate) holds_at: Option<usize>,
    /// Which kinds of row the listing is showing, which is the six chips under the
    /// `holds` field — [`LibraryKinds::EVERYTHING`] where none of them is on, which
    /// is where every run starts.
    ///
    /// A value and not a position, where [`View::holds_at`] above it is a position,
    /// and it is [`View::transition`]'s distinction below: the kinds are the
    /// console's own closed list ([`KindChip::ALL`]) and cannot change under this
    /// pointer, where the `holds` candidates are a reading of a store that can.
    ///
    /// It was `layer: Option<Layer>` until 2026-09-10, the `layer…` field's value,
    /// and ADR-0338 replaced that field with these six toggles: a cycle names six
    /// of the sixty-four states this row has, and the two controls ask different
    /// questions — *which Sets hold a node on this layer*, which
    /// `Operation::ListSets` still carries, against *which kinds of row is this
    /// listing showing*.
    ///
    /// `showing` and not `kinds`, because [`View::kinds`] is already the row of
    /// what each *listed row* is: one field says what the bay is being asked for
    /// and the other says what came back, and two fields called `kinds` would be a
    /// name meaning two things.
    pub(crate) showing: LibraryKinds,
    /// Active text naming session for a pane head, or `None`.
    pub(crate) naming: Option<Naming>,
    /// How far each Inspector pane is scrolled, one position per pane, and the
    /// console's eighth pointer.
    ///
    /// A pointer and not a reading, which is [`View::cursor_row`]'s argument
    /// arriving at a second bay: no operation names it, nothing downstream could be
    /// the model of record for it, and a host that kept a copy would be keeping the
    /// console's state on its behalf. It is not part of the arrangement either —
    /// `karakuri-layout`'s tree holds sizes and folds, which is what a saved
    /// arrangement carries — so a save does not take it and a restore does not move
    /// it
    /// ([ADR-0307](../../../../docs/adr/0307-the-inspectors-pane-scrolls-and-the-position-is-the-panes-own.md)).
    ///
    /// It cannot live in [`View::inspector`], which is [`View::naming`]'s reason
    /// one field up: a pane is rewritten whenever a Set lands, so a position kept
    /// in one would go back to the top every time a build arrived — and a knob
    /// under an operator's hand would leave the screen on a frame nobody touched.
    ///
    /// One per pane and not one per deck. Two panes pointed at one deck are two
    /// places in one list, because what is scrolled is the pane; a position filed
    /// under the deck would move the pane an operator is not looking at.
    ///
    /// Stored unclamped against the *pane*, clamped at the draw. See
    /// [`InspectorPane::scroll`] and [`View::scroll_by`], which are the two halves
    /// of that: the pane's height is a viewport and P-0082 is why a solve may not
    /// write through it.
    ///
    /// Private, with [`View::scroll_by`] and [`View::scroll_in`] the only ways in,
    /// which is [`View::selection`]'s rule. Zero until somebody turns a wheel,
    /// which is every test in this crate and is a pane at the top of what its deck
    /// holds.
    pub(crate) scroll: [f32; PANES],
    /// Target deck slot displayed in each Inspector pane (ADR-0338).
    pub(crate) pane_deck: [u8; PANES],
    /// Which pane head's pulldown is down, or `None` — the console's own state,
    /// like [`View::target_open`] and [`Arrangement::menu`].
    ///
    /// One `Option` and not one flag per pane, for [`Naming`]'s reason read on a
    /// card: `input::claim`'s rule 2 hands every press to a card that is down, so
    /// two down at once would be two hands mid-choice with nothing on the panel
    /// saying which one the next press belongs to.
    pub(crate) pane_open: Option<usize>,
    /// What the next fade, crossfade or wipe means — the wipe's front shape and
    /// angle, the grid it starts on and how long it lasts — and the fourth of this
    /// console's pointers.
    ///
    /// A pointer and not a reading, which is [`View::selection`]'s argument
    /// arriving at a fourth control: [`Operation::SetTransition`] is
    /// `Silent(Surface)` — *"these change nothing you can see and write nothing to
    /// the stream"* — so nothing downstream can be the model of record for it, and
    /// a host that kept a copy would be keeping the console's state on its behalf.
    /// The host reads it to convert the next [`Operation::Wipe`], which is what
    /// `karakuri_operation_record::Current::transition` is asking for.
    ///
    /// Four values and not a position in the three cycles, where
    /// [`View::cursor_row`] and [`View::scope`] are both positions. Those point
    /// into a *listing this console was handed*, which can change under them; these
    /// three cycles are the console's own and cannot, and what the host has to be
    /// told is the shape and the two beat counts rather than where they sit in a
    /// table it cannot see.
    ///
    /// Private, with [`View::set_transition`] the only way in, which is
    /// [`View::selection`]'s rule: the console is what refuses a setting no pill
    /// can draw.
    ///
    /// [`TransitionSettings::START`] until somebody says otherwise, which is every
    /// test in this crate and is where a run begins.
    pub(crate) transition: TransitionSettings,
    /// What the Sequencer bay reads this frame, or `None` for a console with no
    /// sequencer behind it — which is every test in this crate that does not hand
    /// one in, and what the bay draws then is nothing at all.
    ///
    /// The same seam as [`View::mixer`]: a pattern is authored state a *session*
    /// holds and `src/` holds no session (ADR-0156), so whoever owns one writes
    /// this per frame beside the frame it is about. It is not this console's own
    /// pointer — a press here hands back an [`Operation`] and applies nothing,
    /// exactly as a fader's drag does.
    ///
    /// Why the reading is a whole pattern: see [`Sequenced`], which is where the
    /// alternative — a field per drawn thing — is refused.
    pub sequencer: Option<Sequenced>,
    pub(crate) placed: Vec<Placed>,
}
