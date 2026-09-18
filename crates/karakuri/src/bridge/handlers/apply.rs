use super::*;

/// What this window says when a control's operation wrote no record, and
/// the two ways that happens are not the same thing — so they are not the
/// same sentence.
///
/// [`written`] has four answers and only one of them is a record.
/// A harness that printed a line for that one and nothing at all for the
/// other three would tell an operator that a press did nothing, which is true
/// of none of them:
///
/// - [`Written::Silent`] is settled. Selecting a deck or folding a bay is
///   a surface's own state and there is nothing to write; the sentence says
///   which of the four kinds of nothing it is, and that is the end of it.
/// - [`Written::Owed`] is a gap nobody has closed yet. A tap owes a
///   record and no build can make it, so a press that reads as *nothing
///   happened* is exactly the wrong reading — the sentence names the question
///   instead, which is `Owed::why`'s whole job and the reason `Owed` is not
///   an error.
/// - [`Written::Refused`] is a decision taken, and it is the one answer here
///   that is neither settled silence nor a gap: a scheduled move on a fader an
///   unmuted lane of the armed pattern holds writes no record, and the sentence
///   names the lane to mute (ADR-0323). It is `Refusal::why`'s words and not
///   this file's, because the keys, the pointer, a mapped control and a model
///   meet the same sentence.
///
/// `None` for [`Written::Records`], because that line is [`apply`]'s: it says
/// the record *and* what the deck holds afterwards, and printing both would
/// say one press twice.
///
/// Nothing on this panel reaches the `Owed` arm on purpose any more, and
/// the paragraph that used to stand here is worth keeping as history because
/// it was twice wrong in the same place. It first said no control could reach
/// either arm and was written for the day one did; the deck head was that day,
/// and it said the sync chip and the anchor beside it were reachable
/// affordances over an unwritable record — the press claimed, the operation
/// emitted, this sentence printed with the question in it, and the deck not
/// moving.
///
/// What made the record unwritable was a question that had already been
/// answered. `Transport::engaged` decides what engaging a mode means, with
/// the reason at its own definition: the anchor is the session tempo and the
/// scrub is cleared. The clamp that looked like a decision about the bytes on
/// disk is the identity on every tempo an oscillator can report, so there were
/// never two answers to choose between — only a reading nobody was handing in.
/// [`reading`] hands it in now, `written` writes `Record::Transport`, and
/// [`apply`] moves the deck, which is the ninth and tenth of this panel's ten
/// emitting controls arriving where the other eight already were.
///
/// The refusal to route around it is what made that cheap. A surface owns
/// the affordance and never the authority
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
/// so this file never wrote a `Record::Transport` of its own for a `SetSync` —
/// computing the anchor here would have been a window binary taking a decision
/// about a file format, and the printed line was the right answer until the
/// conversion existed
/// ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
/// What changed is the conversion, not this file's authority: the anchor is
/// still the engine's policy and this window still only reads a tempo.
///
/// So all ten controls write records. Five are the mixer's — `SetGain`,
/// `SetOpacity`, `SetBlendMode`, `SetResidency` and `SetMaskShape` — two are
/// the look's, and the last three are the deck head's: the scrub, the chip
/// that cycles and the anchor that re-asks for the mode the deck is in
/// (ADR-0218). The Outputs dot never arrives here at all, because it asks the
/// panel for an arrangement [`Op`] and the panel performs it
/// ([`Acted::Operated`]).
///
/// The mask is also the one that can reach [`Written::Owed`] by accident,
/// and that is worth having rather than designing away: a reading that did not
/// arrive answers `Owed(NotRead(Reading::Mask))`, so a harness that stopped
/// handing one in would say so out loud instead of moving nothing.
pub(crate) fn unwritten(operation: &Operation, written: &Written) -> Option<String> {
    match written {
        Written::Records(_) => None,
        Written::Silent(silent) => Some(format!(
            "  emitted: {operation:?} -> no record, and that is settled: {}",
            silent.why()
        )),
        Written::Owed(owed) => Some(format!(
            "  emitted: {operation:?} -> no record, and that is a gap rather than a \
             decision: {}. nothing moved, and nothing here decides it",
            owed.why()
        )),
        // **A decision rather than a gap, so it is not the line above**
        // (ADR-0323). Nothing was scheduled and no record was written, which is
        // what a replay of this session will also see.
        Written::Refused(refusal) => Some(format!(
            "  emitted: {operation:?} -> refused, and nothing was scheduled: {}",
            refusal.why()
        )),
    }
}

/// What a model is told about an operation this window drained off `--mcp`,
/// and `None` where the answer is *it was performed*.
///
/// [`unwritten`]'s neighbour and its opposite audience: that one writes the
/// terminal's line, which reaches an operator who can see the deck and read
/// every other line this frame printed, and this one writes the reply that
/// leaves the process. A model has neither the terminal nor the window
/// ([ADR-0315](../../../docs/adr/0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md)),
/// so an answer of *performed* over a refusal is the whole of what it is
/// given: it reads the fade as done, sees no frame, and asks for the next
/// thing.
///
/// The two that are answered, and they are the two [`written`] does not perform:
///
/// - [`Written::Refused`] is a decision taken. The move was not scheduled, the
///   lane still holds the fader, and what the next attempt needs is to mute
///   that lane
///   ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md),
///   ADR-0323).
/// - [`Written::Owed`] is a gap nobody has closed. Nothing moved and nothing
///   here decides it, which is the terminal's own words for it one function up.
///
/// `None` for [`Written::Records`], which is a performance, **and `None` for
/// [`Written::Silent`], which is one too**. That is where this differs from
/// `karakuri-cli`'s `answered`, and the difference is the two programs rather
/// than the two sentences: that program performs an operation *by* writing the
/// records it converts to, so a `Silent` there is an operation it has no
/// control for. This window has the control. A `LoadSet`, a `SelectDeck`, a
/// `RouteFrame`, a `SetTransition` and a save all answer `Silent`, and every one
/// of them is performed a few lines above the conversion in `App::performed`
/// or in `App::operated` itself — telling a model that its `load_set` changed
/// nothing would be this fix committing the defect it repairs, in the other
/// direction.
///
/// The sentence is `karakuri_operation_record::not_performed`'s and not this
/// file's, for the reason `Refusal::why` is the refusal crate's: `--mcp` on
/// this program and `--mcp` on `karakuri-cli` are two front doors onto one
/// vocabulary, and one mistake gets one explanation whichever a model came
/// through (ADR-0131).
pub(crate) fn unperformed(title: &str, written: &Written) -> Option<String> {
    match written {
        Written::Records(_) | Written::Silent(_) => None,
        Written::Owed(owed) => Some(not_performed(title, owed.why())),
        Written::Refused(refusal) => Some(not_performed(title, &refusal.why())),
    }
}

/// A press on a strip or a deck key, applied to the console's own pointer, and
/// what to say about it. `None` for every operation that is not it.
///
/// `Operation::SelectDeck` *"writes no record, and is the reason every other
/// variant names its deck instead of meaning the selected one"*, so there is
/// nothing on the deck for [`apply`] to move and the surface that emits it is
/// what performs it (ADR-0198). This is that performance, and it is one line
/// beside [`arrangement`]'s for the same reason: the alternative is a second
/// route into the view.
///
/// A deck the mixer has no strip for is refused, and `View::select` is where
/// that rule lives — the ring would be drawn nowhere and the library's pill
/// would name a deck a load could not reach. It is said here rather than
/// swallowed, because a key that does nothing and a key that is not bound are
/// the same experience.
pub(crate) fn pointed(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::SelectDeck { deck } = *operation else {
        return None;
    };
    let letter = deck_letter(deck);
    if usize::from(deck) >= view.mixer.len() {
        return Some(format!(
            "  select: deck {letter} refused — this deck has {} slot{}, and a selection with no \
             strip under it is a ring drawn nowhere and a `load` pill naming a deck the press \
             could not reach",
            view.mixer.len(),
            match view.mixer.len() {
                1 => "",
                _ => "s",
            }
        ));
    }
    view.select(deck);
    Some(format!(
        "  select: deck {letter} -> SelectDeck {{ deck: {deck} }} -> no record, and that is \
         settled: it is a surface's own pointer. The keys are addressed here, and the library's \
         foot reads `load -> {letter}`"
    ))
}

/// A pick on a pane head's pulldown, applied to the console's own pointer, and
/// what to say about it. `None` for every operation that is not it.
///
/// [`pointed`]'s shape one mark along and for its sentence:
/// `Operation::PointPane` is `Silent(Surface)`, so there is nothing on the deck
/// for [`apply`] to move and the surface that emits it performs it.
///
/// It is not the deck selection, and nothing here touches it — that is the
/// whole of what this mark is for (ADR-0338, decision 5): a pane can show a
/// deck the keys are not on, which is the Library bay's load pulldown's
/// argument one bay along.
///
/// The pane is named rather than numbered, because `Operation::PointPane { pane
/// }` is a `String` — `karakuri-operation` has no dependencies and cannot hold
/// the arrangement's handle type — so this is where the name is resolved back
/// to a position in `View::inspector`. A name no pane has is refused with the
/// two that exist, which is what the next attempt needs (P-0083).
///
/// A deck the mixer has no strip for is refused, and `View::point_pane` is
/// where that rule lives — [`pointed`]'s own refusal, read on a pane instead of
/// on the ring.
pub(crate) fn pointed_pane(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::PointPane { pane, deck } = operation else {
        return None;
    };
    let Some(at) = view::PANE_NAMES.iter().position(|name| name == pane) else {
        return Some(format!(
            "  pane: `{pane}` refused — this console's panes are {}",
            view::PANE_NAMES.join(" and ")
        ));
    };
    let letter = deck_letter(*deck);
    if !view.point_pane(at, *deck) && usize::from(*deck) >= view.mixer.len() {
        return Some(format!(
            "  pane: `{pane}` -> deck {letter} refused — this deck has {} slot{}, and a pane \
             pointed at one it has not got is a head naming a deck with nothing under it",
            view.mixer.len(),
            match view.mixer.len() {
                1 => "",
                _ => "s",
            }
        ));
    }
    Some(format!(
        "  pane: `{pane}` -> deck {letter} -> no record, and that is settled: a pane's target is \
         a surface's own pointer. The keys stay where they are and the pane next door does not \
         move"
    ))
}

/// What a refused `go` says, and the whole of what this window puts on this
/// side of that seam.
///
/// `karakuri_console::view::Go` answers *which* refusal, because the console is
/// what can see a shape is unset and how many strips it drew; the sentence is
/// here because this package is the one that has anywhere to print. What each
/// of them owes is
/// [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):
/// a rejection is a message to whoever makes the next attempt, and it carries
/// the constraint and where to go rather than *refused*.
///
/// `karakuri-cli`'s `c` prints the same two, and its wording is where these
/// come from — *"a wipe needs somewhere to come from — this deck holds one
/// slot"* and *"no mask shape — `z` chooses one, and a wipe is a shape
/// moving"*. What changes on this surface is where the next attempt is made: a
/// pill two capsules to the left rather than a key.
pub(crate) fn refusal(refused: &Go, decks: usize) -> String {
    match refused {
        // **Unreachable while this window builds a full deck** — `SLOTS` is
        // `MAX_SLOTS` and the mixer draws one strip per slot — so what this
        // answers for is a console drawn before the deck reached it, and it
        // is written rather than left to be an unexplained silence. The count
        // is said because it is the thing that would have to change.
        Go::NoOtherDeck => format!(
            "  wipe: refused — this mixer draws {decks} strip{}, and a wipe needs a deck to \
             come from as well as one to arrive. Two decks side by side in the bay is what \
             makes the press mean something",
            match decks {
                1 => "",
                _ => "s",
            }
        ),
        Go::NoShape => String::from(
            "  wipe: refused — no shape chosen, and a wipe is a shape moving. The first pill \
             on this row is where one is picked: it reads `no shape` now, and a press on it \
             takes it to `left`. The vocabulary refuses this one too — \
             `Operation::Wipe` is *refused with no shape chosen* at its own definition — and \
             the refusal is here because the shape is this console's own setting",
        ),
        // A wipe is not a refusal, and this arm exists so that the day a
        // fourth answer lands somebody has to say what it reads rather than
        // a wildcard printing one of these two over it.
        Go::Wipe(operation) => format!(
            "  wipe: {} is not a refusal and this line should not have been reached",
            operation.title()
        ),
    }
}

/// A press on one of the transition row's three pills, applied to the console's
/// own setting, and what to say about it. `None` for every operation that is
/// not it.
///
/// [`pointed`]'s shape one row down, and for the same reason:
/// `Operation::SetTransition` *"changes nothing you can see and writes nothing
/// to the stream"* — `written` answers `Silent(Surface)` for it and no record
/// in `karakuri-store` carries a quantum, a length or a wipe shape — so there
/// is nothing on the deck for [`apply`] to move and the surface that emits it
/// is what performs it. `View::set_transition` is the only door into that
/// setting, which is where the refusal below lives.
///
/// What it changes is what the next wipe means, and that is why the line says
/// the whole row rather than the field that moved: an operator reading *the
/// shape is an iris* still has to know what grid it starts on.
///
/// A setting no pill can draw is refused, and it is said rather than swallowed
/// for [`pointed`]'s reason — a press that does nothing and a press that is not
/// bound are the same experience. Nothing this window emits can reach it: the
/// pills name a destination out of the console's own cycles. It is a mapped
/// controller or an MCP call that could, the day either reaches this row.
pub(crate) fn scheduled(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::SetTransition { setting } = operation else {
        return None;
    };
    if !view.set_transition(*setting) {
        let at = view.transition();
        return Some(format!(
            "  transition: {setting:?} refused or already there — the row is on `{}`, `{}`, \
             `{}`, and a pill draws only what its own cycle names",
            at.shape_word(),
            at.quantum_word(),
            at.length_word()
        ));
    }
    let at = view.transition();
    Some(format!(
        "  transition: {setting:?} -> no record, and that is settled: it is a surface's own \
         setting. The next fade, crossfade or wipe is `{}` on the `{}`, over {} beat{}",
        at.shape_word(),
        at.quantum_word(),
        at.length,
        match at.length == 1.0 {
            true => "",
            false => "s",
        }
    ))
}

/// A candidate kept, applied to the lane, and what to say about it. `None` for
/// every operation that is not it.
///
/// [`scheduled`]'s shape one bay over, and for its reason: `written` answers
/// `Silent(Silent::Surface)` for `Operation::KeepCandidate`, so there is no
/// record for [`apply`] to move a deck with and the surface that draws the row
/// is what performs the press. What it changes is one line in one list.
///
/// Nothing else moves, and that is the operation rather than a shortfall. The
/// version is where it was, the store holds every version it held, and the
/// picture is the picture. What a keep says is that a person has looked at this
/// node and is done with it — `console.html`'s *Accepting settles the node and
/// changes nothing on screen*.
///
/// A row that is not there is said rather than swallowed, which is
/// [`pointed`]'s rule: nothing this window emits can reach it — the control is
/// the row and a row that is not drawn takes no press — so a line here is a
/// mapped controller or an MCP call arriving at a node with no candidate on it,
/// the day either reaches this row.
pub(crate) fn kept(view: &mut View, operation: &Operation) -> Option<String> {
    let Operation::KeepCandidate { deck, node } = operation else {
        return None;
    };
    let slot = usize::from(*deck);
    let addr = node_addr(ir_layer(node.layer), node.index);
    let before = view.staging.len();
    view.staging
        .retain(|row| row.deck != slot || row.at != Some(*node));
    let letter = deck_letter(*deck);
    if view.staging.len() == before {
        return Some(format!(
            "  keep: deck {letter} {addr} has no candidate row — nothing was outstanding on \
             that node, and the lane is as it was"
        ));
    }
    Some(format!(
        "  keep: deck {letter} {addr} -> KeepCandidate -> no record, and that is settled: the \
         material already changed and its `procedure` record was written at the swap. The row \
         leaves the lane and nothing else moves; {} still waiting",
        match view.staging.len() {
            0 => "nothing".to_owned(),
            n => format!(
                "{n} row{}",
                match n {
                    1 => "",
                    _ => "s",
                }
            ),
        }
    ))
}

/// Which salt a slot's material is seeded from — the one it was built at, and
/// the one a load restates when the Set file recorded none.
///
/// Deck A's is [`SEED_SALT`] and every slot after it is one further along, so
/// this deck's four slots are four different simulations of the one procedure
/// [`Sources`] names. That generalises the reason the second salt was written
/// for and then retires the constant. `WARM_SEED_SALT` existed so that *the
/// slot the budget parks is a different simulation rather than a second copy of
/// the same one* — an argument about the parked slot, made when the parked slot
/// was the only other slot there was. What it was really saying is that a deck
/// of one picture repeated is not a mixer, and that is true of every slot
/// rather than of deck B, so it is said once here and no constant states a
/// reason that has gone ([`docs/contributing.md`
/// §4](../../../docs/contributing.md)).
///
/// `+ slot` rather than a table, because a table of four numbers is four values
/// with nothing to say about each other, and what is wanted is exactly
/// *distinct, and deck A's is the one the CLI's tests use*. Distinctness is
/// then arithmetic rather than four typed numbers nobody re-reads — which
/// `every_slot_is_its_own_simulation` asserts salt by salt, off the Sets the
/// deck actually built rather than off this function.
///
/// The salts the run was built with, and no others: a Set loaded into a slot is
/// new *material* and not a new simulation, so a rebuild that derived its own
/// seed would repaint every element in the slot for a reason nobody asked for —
/// which is `Watch::salts`' own argument, met from the loading side.
pub(crate) fn slot_salt(slot: usize) -> u32 {
    SEED_SALT + slot as u32
}

/// What the derived material a procedure load leaves is a derivation *of*: the
/// Set the slot is filed under, or the pair the run was launched with where it
/// is filed under none.
///
/// `watch::Aim::set` first, because that is the one field a load moves and a
/// procedure load does not (ADR-0304, ADR-0338): a slot that has been loaded is
/// running that Set with one layer over it, and the strip has to say so. A slot
/// nobody has loaded is running the launch pair, which no id names — that is
/// the state `Aim::set` is `None` in, and the launch pair is what the strip has
/// been reading since the first frame.
pub(crate) fn base_material(set: Option<&str>, launch: &str) -> String {
    set.map(str::to_owned).unwrap_or_else(|| launch.to_owned())
}

/// What the strip reads once a layer has been written over what a deck is
/// playing: `<base> + <kir>`, which is the maintainer's own `drift_night +
/// orbit_wide`.
///
/// So what is on air says what it is made of and never claims to be a Set the
/// library holds — `keep` is what gives it a name (ADR-0338).
pub(crate) fn derived_material(base: &str, procedure: &str) -> String {
    format!("{base} + {procedure}")
}

/// Write one procedure over the layer it declares and re-aim the slot, or say
/// why it did not.
///
/// # Where the file comes from, and it is the two tiers and nothing else
///
/// `<store>/procedures/<name>.kir` first and the presets root's `<name>.kir`
/// after it, which is the order the Library bay lists them in and the only two
/// places a procedure row can have come from (ADR-0227's two tiers, ADR-0338's
/// decision 1). The content-addressed artifacts at the store root are not
/// searched: that population is the edit history's, addressed by hash, and a
/// name is not one.
///
/// # Which position it lands on, and the limit is recorded rather than designed
/// around
///
/// The first node of that kind. A procedure declares one `kind` and nothing
/// about where it goes, and a library row cannot say an index — so the payload
/// carries none, and `L4:0` is the renderer a `kind L4` replaces. The second
/// renderer of a three-renderer Set is unreachable from this row, and the day
/// the Inspector's node head grows a *replace this node* control is the day the
/// payload gains a `NodeAddress` (ADR-0338, stated at the point it bites).
///
/// Where the slot has no node of that kind the procedure is added as node 0 of
/// it, which is the case the request is about: a Set of a geometry and a
/// renderer declares no camera, so it holds the built-in orbit at `L3:0` and a
/// `kind L3` row takes that position — the picture changes camera with nothing
/// else moving.
///
/// # What each file already on the slot is
///
/// Read off the files themselves with `history::declared_kind`, which is the
/// one scanner for a `kind` line, and with `compile`'s own fallback where a
/// file declares none — the first node is an L1 and the rest are L4s, which is
/// what a bare pair is. That is one small read per node, on the press, and it
/// compiles nothing (P-0091).
///
/// # The node name is kept, and that is what keeps the edges
///
/// A replaced position keeps the name the Set gave that node, because an `edge`
/// and a `bind` in the aim resolve against it: a rebuild that renamed the node
/// would break the wiring the slot is running. A node that is *added* is named
/// after the row, and a name the slot already holds is refused rather than
/// shadowed.
pub(crate) fn overlaying(
    root: &std::path::Path,
    presets: Option<&std::path::Path>,
    slot: usize,
    aim: &mut Aiming,
    name: &str,
) -> Result<String, String> {
    let (source, tier) = kept_source(root, presets, name)?;
    let kind = karakuri_environment::history::declared_kind(&source).ok_or_else(|| {
        format!(
            "`{name}` declares no `kind`, so there is no layer to write it over — a procedure \
             says which layer it implements in a `kind` line, and this one says nothing"
        )
    })?;
    // **Head and rest are one list here**, because *the first node of that kind*
    // is a question about the slot's files in order and the split is only how a
    // `watch::Aim` carries them.
    let mut files: Vec<karakuri_environment::compile::Named> = std::iter::once(aim.at.head.clone())
        .chain(aim.at.rest.iter().cloned())
        .collect();
    let at = files.iter().enumerate().find_map(|(index, file)| {
        let source = std::fs::read(&file.path).ok()?;
        let declared = karakuri_environment::history::declared_kind(&source)
            .unwrap_or(if index == 0 { "L1" } else { "L4" });
        (declared == kind).then_some(index)
    });
    let (at, added) = match at {
        Some(at) => (at, false),
        None => (files.len(), true),
    };
    if added && files.iter().any(|file| file.name.as_deref() == Some(name)) {
        return Err(format!(
            "deck {} already holds a node called `{name}`, and this row would add a second — \
             rename one of them and load again",
            deck_letter(slot as u8)
        ));
    }
    let path = karakuri_environment::scratch::place(
        root,
        &karakuri_environment::scratch::node_name(slot, at, name),
        &String::from_utf8_lossy(&source),
    )?;
    match added {
        // **Named after the row**, because nothing in the slot named it: a node
        // added here is addressed by the name an operator can read off the row
        // they pressed.
        true => files.push(karakuri_environment::compile::Named {
            name: Some(name.to_string()),
            path,
        }),
        // **The node keeps the name the Set gave it**, which is what an `edge`
        // resolves against — see this function's own head.
        false => files[at].path = path,
    }
    let mut files = files.into_iter();
    let head = files
        .next()
        .ok_or_else(|| String::from("this deck is running no files at all"))?;
    let rest: Vec<karakuri_environment::compile::Named> = files.collect();
    // **Every other field restated**, which is `Aiming::changed`'s single
    // derivation: the layering, the fold, the capacity, the seed and the salts,
    // the camera, the overrides, the wiring, the grants and the Set this slot is
    // filed under all come back as the slot's own rather than as a default
    // (ADR-0314). `Aim::set` is among them, which is why the history goes on
    // filing under the base.
    aim.changed(|at| {
        at.head = head;
        at.rest = rest;
    })
    .map_err(|()| {
        format!(
            "deck {}'s build worker is gone, so `{name}` cannot be built; what is on that deck \
             keeps running",
            deck_letter(slot as u8)
        )
    })?;
    let where_ = match added {
        true => format!("added as {kind}:0, which this deck had no node for"),
        false => format!("written over {kind}:0"),
    };
    Ok(format!(
        "  load: `{name}` ({tier}) {where_} on deck {} — the slot is recompiling on the worker \
         with every other layer where it was, and the staging lane says whether the build \
         landed, was overloaded or did not compile",
        deck_letter(slot as u8)
    ))
}

/// A procedure's bytes, out of whichever tier holds it, with the word for the
/// tier so the sentence a press prints says where the file came from.
///
/// The operator's own first and what ships after it, which is the order the
/// listing draws them under `all` and `presets`: a name kept in this store is
/// this store's answer, and nothing this program does writes where the presets
/// are (P-0096).
pub(crate) fn kept_source(
    root: &std::path::Path,
    presets: Option<&std::path::Path>,
    name: &str,
) -> Result<(Vec<u8>, &'static str), String> {
    if let Ok(store) = Store::open(root) {
        if let Ok(source) = store.read_procedure(name) {
            return Ok((source, "kept"));
        }
    }
    let shipped = presets.map(|dir| dir.join(format!("{name}.kir")));
    if let Some(path) = shipped {
        if let Ok(source) = std::fs::read(&path) {
            return Ok((source, "shipped"));
        }
    }
    Err(format!(
        "no procedure named `{name}` — this store's `{}/` does not hold one and the presets root \
         does not ship one, so the row it was listed under has gone since the listing was built",
        Store::PROCEDURES
    ))
}

/// What a [`karakuri_operation::Revision`] asked for, as a refusal names it — a
/// version by the name it was filed under, or a node by its address.
///
/// One function because the two refusals about the *deck* are the same sentence
/// whichever arm arrived: a slot the deck has not got and a deck playing no Set
/// are answered before anything is read, and what they have to name is only
/// what was asked for.
pub(crate) fn asked_for(revision: &karakuri_operation::Revision) -> String {
    match revision {
        karakuri_operation::Revision::Picked(name) => format!("`{name}`"),
        karakuri_operation::Revision::Previous(node) => format!(
            "the version before {}'s",
            node_addr(ir_layer(node.layer), node.index)
        ),
    }
}

/// The half of [`restored`] that reaches a disk, split out for the reason
/// [`seeded`] is a free function: `main` cannot be entered from a test, a
/// `Gfx` cannot be built without a device, and what this does is worth
/// asserting — it writes over the file a deck is playing from.
///
/// Everything it needs is an argument: the store to walk, the slot and its
/// letter, the Set the slot is running, the revision that was asked for, and
/// where that slot's nodes are ([`karakuri_mcp::Slots`], the
/// run's one published layout, which the caller reads off [`Engine::pointing`]).
/// The refusals here are the ones that are about files — a version that is
/// not in the listing, a node with nothing behind the one it is playing, a
/// node this slot does not hold, and a file that will not be read or written —
/// where the two about the *deck* are the caller's and are answered before
/// this is reached.
///
/// # One function and two ways of naming the file
///
/// `karakuri_operation::Revision` has two arms because two surfaces can ask
/// and each says the half it holds (ADR-0308), and what differs between them
/// is which row of this listing — nothing after that. So the walk, the
/// read, the address and the write are one path, and the arms are one `match`
/// over the same `Vec<Version>`:
///
/// - `Picked` is a name the Library bay's `history` scope handed over, matched
///   back against the listing that produced it by rebuilding each row's
///   spelling — `SetTransfer::Take`'s own arrangement, and the reason no
///   surface here spells a path.
/// - `Previous` is a node the Staging lane's row handed over, and the version
///   is the one before the one running: the listing is most recent first,
///   the newest entry for that node is what the slot is playing — the history
///   is gated on compiling and not on landing, so a version that was stopped
///   for cost is filed too — and the entry after it is the step back
///   (`docs/adr/0326-a-staging-row-is-a-changed-node-and-the-row-is-the-keep.md`).
///   A node with exactly one version in the listing is refused naming what is
///   in the way, which is `P-0083`: it says the node has nothing behind what
///   it is playing rather than that the press failed.
///
/// The narrowing to the Set is both arms', and it is the same narrowing
/// for the same reason — a version filed under no Set is a version of nothing.
pub(crate) fn put_back(
    store: &std::path::Path,
    slot: usize,
    letter: &str,
    id: &str,
    revision: &karakuri_operation::Revision,
    slots: &karakuri_mcp::Slots,
) -> String {
    let asked = asked_for(revision);
    let found = match karakuri_environment::history::list(store, HISTORY_MOST) {
        Ok(found) => found,
        Err(why) => {
            return format!(
                "  put back: the edit history could not be read: {why} — nothing moved, and \
                 what is on deck {letter} is still running"
            );
        }
    };
    // **`Some(id)` and never `None`**, which is [`walked`]'s filter said again
    // where it decides what is written rather than what is drawn: a version
    // filed under no Set is a version of nothing, and landing one because a
    // `None` read as a wildcard would put another run's edit on a deck.
    let of_the_set = || {
        found
            .versions
            .iter()
            .filter(|version| version.set.as_deref() == Some(id))
    };
    let version = match revision {
        karakuri_operation::Revision::Picked(picked) => {
            let Some(version) = of_the_set().find(|version| version_row(version) == *picked) else {
                return format!(
                    "  put back: `{picked}` is not one of `{id}`'s versions in the last \
                     {HISTORY_MOST} this store wrote — a day directory an operator deleted by \
                     hand is the ordinary way that happens, since `rm -rf history/YYYY/MM` is \
                     the whole of the retention policy; nothing moved, and deck {letter} is \
                     still playing `{id}`"
                );
            };
            version
        }
        karakuri_operation::Revision::Previous(node) => {
            let layer = setfile::kind_name(ir_layer(node.layer));
            let addr = node_addr(ir_layer(node.layer), node.index);
            // **Most recent first is `history::list`'s own order**, kept
            // rather than re-sorted here: the newest is what the slot is
            // running and the one after it is the step back.
            let mut chain = of_the_set()
                .filter(|version| version.layer == layer && version.index == node.index as usize);
            let running = chain.next();
            let Some(version) = running.and(chain.next()) else {
                return format!(
                    "  put back: deck {letter} {addr} has {} in `{id}`'s history, so there is \
                     nothing before what it is playing to step back to — the history keeps \
                     every version that compiled, and this node has only ever had the one. \
                     Nothing moved.",
                    match running {
                        Some(_) => "one version",
                        None => "no version",
                    }
                );
            };
            version
        }
    };
    let source = match std::fs::read(&version.file) {
        Ok(source) => source,
        Err(e) => {
            return format!(
                "  put back: {}: {e} — the row is a name and the file behind it is gone, so \
                 nothing moved and deck {letter} is still playing `{id}`",
                version.file.display()
            );
        }
    };
    let target = match slots.file(slot, version.layer, version.index) {
        Ok(path) => path.clone(),
        Err(why) => {
            return format!(
                "  put back: deck {letter} does not hold the node {asked} was a version \
                 of — {why}; `{id}` has been edited since, or this version came off another \
                 slot. Nothing moved."
            );
        }
    };
    match std::fs::write(&target, &source) {
        Ok(()) => format!(
            "  put back: deck {letter} {}:{} <- `{}` -> written into {}; the worker \
             builds it and the budget judges it, and the version it replaces is kept because \
             the rebuild compiles it",
            version.layer,
            version.index,
            version_row(version),
            target.display()
        ),
        Err(e) => format!(
            "  put back: {}: {e} — nothing moved, and what is on deck {letter} is still \
             running",
            target.display()
        ),
    }
}

/// The record, applied to the deck, and what to say about it.
///
/// This is not the half ADR-0185 promised to delete, and it did not go with it.
/// Turning an `Operation` into a `Record` was the shortcut — that function is
/// gone and [`written`] answers instead
/// ([ADR-0194](../../../docs/adr/0194-where-an-operation-becomes-a-record-is-a-crate-that-depends-on-both.md)).
/// Turning a record into a *deck movement* is a different job and is the
/// harness's by design: `karakuri-operation-record` has no engine and never
/// will, so somebody who owns a deck has to decode.
///
/// `karakuri-cli`'s `mix::change` is the real decoder and it does two things
/// this does not: it refuses a slot the deck has not got, with the same
/// sentence every other surface refuses one with, and it turns a record into a
/// `Change` that a caller applies. This is the shortest path from the records
/// [`written`] answers with to the setters they name.
///
/// The line it returns is the loop closing, printed so that it can be read
/// rather than inferred: the operation, the record, and what the deck says
/// afterwards — which is where the next frame's strip comes from.
///
/// # It takes the look as well as the deck, and that is not a second target
///
/// `Record::Look` is the one record here that does not name a slot: the look is
/// what *every* sink is drawn under, so it is `Engine::look` rather than
/// anything on the deck ([`Engine::look`], and `karakuri_engine::frame::Look`
/// for why the master out is deliberately not in it). Handing both in is what
/// keeps this one function the only place a record becomes a movement — a
/// second `apply_look` beside it would be the second route into the engine that
/// P-0090 exists to refuse.
pub(crate) fn apply(
    record: &Record,
    deck: &mut Deck,
    look: &mut Look,
    chain: &mut Vec<karakuri_engine::SlotSpec>,
) -> Option<String> {
    // **A slot the deck has not got is refused rather than indexed**, and the
    // guard is [`held`] rather than a closure here, because the key arms in
    // `window_event` need the same answer one step earlier: a press reads the
    // trim it is stepping from before it can name where it is going, so a
    // guard on the record alone would be a read that panicked on its way to a
    // refusal. The argument for refusing at all is at that function.
    match *record {
        Record::Gain { slot, value } => {
            let slot = held(deck, slot.0)?;
            deck.set_gain(slot, value);
            Some(format!(
                "  fader: deck {} trim -> SetGain {{ deck: {slot}, gain: {value:.3} }} \
                 -> Record::Gain -> deck.gain({slot}) = {:.3}",
                deck_letter(slot.0),
                deck.gain(slot)
            ))
        }
        Record::Opacity { slot, value } => {
            let slot = held(deck, slot.0)?;
            deck.set_opacity(slot, value);
            Some(format!(
                "  fader: deck {} fader -> SetOpacity {{ deck: {slot}, opacity: {value:.3} }} \
                 -> Record::Opacity -> deck.opacity({slot}) = {:.3}",
                deck_letter(slot.0),
                deck.opacity(slot)
            ))
        }
        Record::Mute { slot, muted } => {
            let slot = held(deck, slot.0)?;
            deck.set_mute(slot, muted);
            Some(format!(
                "  mute: deck {} -> SetMute {{ deck: {slot}, mute: {muted} }} \
                 -> Record::Mute -> deck.is_muted({slot}) = {}",
                deck_letter(slot.0),
                deck.is_muted(slot)
            ))
        }
        Record::Solo { slot, soloed } => {
            let slot = held(deck, slot.0)?;
            deck.set_solo(slot, soloed);
            Some(format!(
                "  solo: deck {} -> SetSolo {{ deck: {slot}, solo: {soloed} }} \
                 -> Record::Solo -> deck.is_soloed({slot}) = {}",
                deck_letter(slot.0),
                deck.is_soloed(slot)
            ))
        }
        Record::Online { slot, online } => {
            let slot = held(deck, slot.0)?;
            deck.set_online(slot, online);
            Some(format!(
                "  online: deck {} -> SetOnline {{ deck: {slot}, online: {online} }} \
                 -> Record::Online -> deck.is_online({slot}) = {}",
                deck_letter(slot.0),
                deck.is_online(slot)
            ))
        }
        // **The mode comes back off the wire name, and an unknown one is
        // refused rather than defaulted.** `Record::Blend` carries a `String`
        // because what a mode is allowed to be is the engine's to say, so this
        // is the engine saying it — `mix::change` refuses the same way, with
        // the sentence `karakuri-cli`'s `no_such_blend` writes. Nothing in
        // this file can produce a name the engine has not got, since the chip
        // only ever emits one of `BlendMode::ALL`, so this is the guard rather
        // than the message.
        Record::Blend { slot, ref mode } => {
            let slot = held(deck, slot.0)?;
            let blend = Blend::from_name(mode)?;
            deck.set_blend(slot, blend);
            Some(format!(
                "  blend: deck {} -> SetBlendMode {{ deck: {slot}, blend: {mode} }} \
                 -> Record::Blend -> deck.blend({slot}) = {}",
                deck_letter(slot.0),
                deck.blend(slot).name()
            ))
        }
        // **The one record here that is followed by a governor pass**, and it
        // is not a flourish: `Deck::set_residency` writes the request *and
        // grants it*, so a harness that stopped there would put a slot the
        // budget has no room for into `Priming` and draw a primed deck the
        // governor never admitted. That is
        // [ADR-0191](../../../docs/adr/0191-the-panels-parked-deck-is-parked-by-the-governor-or-it-is-a-drawing-of-one.md)
        // exactly — the panel's parked deck is the governor's verdict or it is
        // a drawing of one — and it is `karakuri-cli`'s own order, where
        // `mix::Change::Residency` sets the level and calls `govern` beside
        // it. The report is dropped here rather than printed: the line below
        // says what the deck ended up at, which is the half this window shows.
        Record::Residency { slot, ref level } => {
            let slot = held(deck, slot.0)?;
            let residency = mix::parse_residency(level)?;
            deck.set_residency(slot, residency);
            deck.govern();
            Some(format!(
                "  tally: deck {} -> SetResidency {{ deck: {slot}, residency: {level} }} \
                 -> Record::Residency -> deck.requested_residency({slot}) = {:?}, \
                 deck.residency({slot}) = {:?}",
                deck_letter(slot.0),
                deck.requested_residency(slot),
                deck.residency(slot)
            ))
        }
        // **The whole mask, because the record is a state and not an ask.**
        // `Record::Mask` carries a shape, an angle, a position and a softness,
        // and `Deck::set_mask` is what it decodes to — the engine says so at
        // that setter. Reaching for `Deck::set_mask_shape` instead, to keep a
        // running wipe alive, would be this file decoding a record by picking
        // two fields out of it and dropping the softness on the floor: a
        // second route to the deck, where P-0090 is that every control ends in
        // the same record. **So a shape press stops a wipe on that deck**, and
        // that is not a fault here — it is the honest limit
        // `docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md`
        // states about the record stream, met by the first surface to make the
        // press
        // ([ADR-0203](../../../docs/adr/0203-the-mask-chip-carries-the-angle-it-does-not-control.md)).
        //
        // **The shape comes back off the wire name**, refused rather than
        // defaulted, exactly as the blend's and the residency's do. Nothing in
        // this file can produce a name the engine has not got, since the chip
        // only ever emits one of `WipeKind`'s three and the record is written
        // from `WipeKind::name`.
        Record::Mask {
            slot,
            ref kind,
            angle,
            position,
            softness,
        } => {
            let slot = held(deck, slot.0)?;
            let shape = MaskKind::from_name(kind)?;
            deck.set_mask(slot, Mask::new(shape, angle, position, softness));
            Some(format!(
                "  mask: deck {} -> SetMaskShape {{ deck: {slot}, kind: {kind}, \
                 angle: {angle:.3} }} -> Record::Mask -> deck.mask({slot}) = {} \
                 at {:.3} rad, front at {:.3}",
                deck_letter(slot.0),
                deck.mask(slot).kind().name(),
                deck.mask(slot).angle(),
                deck.mask(slot).position()
            ))
        }
        // **The whole look, because the record is a state and not an ask.**
        // `Record::Look` carries the operator, the exposure and the white
        // point together for its own stated reason — a stream that set a level
        // without naming the operator would describe a look nobody can
        // reconstruct — and each of the two controls asks for one of the three
        // (ADR-0192). The other two arrive here already filled in from the
        // reading [`reading`] took, so this writes what it is given and picks
        // nothing out of it, exactly as the mask arm does.
        //
        // **The operator comes back off the wire name, refused rather than
        // defaulted**, as the blend's, the residency's and the shape's do.
        // Nothing in this file can produce a name the engine has not got: the
        // capsule only ever emits one of `Tonemap`'s four and the record is
        // written from `Tonemap::name`, which is the same lower-case spelling
        // `mix::op_wire_name` parses.
        //
        // **No `set_tonemap` call.** `compose` uploads the tone-map uniform
        // every frame from the `Committed` the closure hands back, so writing
        // the field *is* the write — and a `Present::set_tonemap` here would be
        // a second writer, with the last one each frame winning.
        Record::Look {
            ref op,
            exposure,
            white_point,
        } => {
            let op = mix::parse_op(op)?;
            *look = Look {
                op,
                exposure,
                white_point,
            };
            Some(format!(
                "  look: -> Record::Look {{ op: {op_name}, exposure: {exposure:.3}, \
                 white_point: {white_point:.3} }} -> every sink is drawn under {op_name} at \
                 exposure {exposure:.3}",
                op_name = mix::op_wire_name(op)
            ))
        }
        // **A scheduled move, and the first record this window applies that
        // does not land now.** `Record::Transition` carries the slot, which
        // control is moving, where it ends up, the beat it starts on, how long
        // it lasts and the shape it eases with — and the one thing it does not
        // carry is where the move starts *from*, because that is where the
        // control already is at the moment the record is applied. Reading it
        // here rather than off the record is what makes a replay fade from
        // where the run did, and it is `karakuri-cli`'s `schedule_from`, in
        // the one place this window needs it.
        //
        // **It landed with the `go` capsule, and until then the window drew a
        // wipe's other records and dropped this one on the floor**: the
        // arriving deck was masked to nothing and put on air, and the front
        // never travelled. A record with no arm here is silent — the `_` at
        // the foot of this match — which is why the gap was invisible.
        //
        // **The two names come back off the wire, refused rather than
        // defaulted**, as the blend's, the residency's and the sync mode's do:
        // a control this engine has not got and a curve it cannot ease with
        // are both a record from a stream this build does not understand, and
        // guessing at either would schedule a move nobody wrote.
        Record::Transition {
            slot,
            ref control,
            to,
            start,
            beats,
            ref curve,
        } => {
            let slot = held(deck, slot.0)?;
            let control = Control::from_name(control)?;
            let curve = karakuri_engine::binding::Curve::parse(curve)?;
            let from = match control {
                Control::Gain => deck.gain(slot),
                Control::Opacity => deck.opacity(slot),
                Control::MaskPosition => deck.mask(slot).position(),
            };
            deck.schedule(karakuri_engine::Transition::new(
                slot.index(),
                control,
                from,
                to,
                start,
                beats,
                curve,
            ));
            Some(format!(
                "  transition: deck {} {} {from:.3} -> {to:.3} -> Record::Transition {{ \
                 start: {start:.3}, beats: {beats:.3}, curve: {} }} -> \
                 deck.transitions_on({slot}) = {}",
                deck_letter(slot.0),
                control.name(),
                curve.name(),
                deck.transitions_on(slot).count()
            ))
        }
        // **One level on the whole fold, and the one arm here that names no
        // slot at all.** `Record::MasterOut` carries a number and nothing
        // else, so unlike the look and the mask there is no other half of it
        // to fill in from what is running — which is why the reading below
        // has no arm for this control (ADR-0224).
        //
        // **The engine clamps and this does not.** `Deck::set_out` floors at
        // zero and is deliberately open above 1.0, through the same
        // `clamp_gain` the per-slot gain goes through, because the mix is HDR
        // and this level is applied to values a tone mapper has not seen —
        // [P-0064](../../../docs/principles/0064-the-pipeline-is-linear-hdr-and-srgb-is-encoded-once-at-final-output.md).
        // A clamp here would be a second opinion about a range the setter
        // already holds, which is the rule the whole conversion is written
        // under.
        //
        // **No `cancel` to worry about**, where the gain and the fader each
        // stop whatever was moving them: a `Control` is per slot and this is
        // not, so nothing in the engine can be moving it and there is nothing
        // for a hand to win against.
        // **The one record that reaches inside a Set**, and the row the
        // Inspector bay was blocked on. `Deck::write_param` is the public road
        // and it compiles nothing — the map is packed into the uniform by the
        // next `Set::prepare`, so the value is on screen on the next frame.
        //
        // **Decoded by `mix::change` rather than here**, which is the one arm
        // in this function that does not take the short path, and the reason is
        // the expansion: a `vec3` value is three writes under the component
        // keys ADR-0268 made, and spelling that a second time in this file is
        // the drift `karakuri-operation-record` exists to end. It is also what
        // refuses a slot this deck has not got, in the sentence every other
        // surface refuses one with — so `held` is not asked first here.
        Record::Ride { .. } => {
            let writes = match karakuri_environment::mix::change(record, deck.slot_count()) {
                Ok(Some(karakuri_environment::mix::Change::Ride { slot, writes })) => {
                    let mut reached = 0;
                    for write in &writes {
                        match deck.write_param(EngineSlot(slot as u8), write) {
                            Ok(n) => reached += n,
                            Err(refused) => return Some(format!("  {refused}")),
                        }
                    }
                    (slot, writes, reached)
                }
                Ok(_) => return None,
                Err(refused) => return Some(format!("  {refused}")),
            };
            let (slot, writes, reached) = writes;
            match reached {
                0 => Some(format!(
                    "  {}",
                    karakuri_environment::no_such_param(slot, &writes[0].key)
                )),
                _ => Some(format!(
                    "  knob: deck {} -> WriteParam -> Record::Ride -> {} on {reached} node(s)",
                    deck_letter(slot as u8),
                    writes
                        .iter()
                        .map(|w| format!("{} = {:.3}", w.key, w.value))
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            }
        }
        // **What is driving a knob, and the taking of it back** — the other
        // record that reaches inside a Set, and the one the Inspector's
        // sensitivity row was blocked on. `Deck::bind` and `Deck::unbind` are
        // the public roads and neither compiles anything: a binding is
        // resolved by the next `Set::prepare`, so an attachment is riding on
        // the next frame.
        //
        // **Decoded by `mix::change` rather than here**, which is the ride's
        // reason one arm up: the three diagnostics a binding owes — a `bpm`
        // source, an `octaves` without `fbm`, a generator on a binding that is
        // not to `noise` — belong to `setfile::binding_from_record` and are
        // said in its words on every route, so spelling them a second time
        // here is the drift `karakuri-operation-record` exists to end.
        Record::Source { .. } => {
            let (slot, key, bound) = match karakuri_environment::mix::change(
                record,
                deck.slot_count(),
            ) {
                Ok(Some(karakuri_environment::mix::Change::Source {
                    slot,
                    layer,
                    index,
                    key,
                    binding,
                })) => match binding {
                    Some(binding) => {
                        let signal = binding.signal.clone();
                        let curve = binding.curve.name();
                        let range = binding.range;
                        match deck.bind(EngineSlot(slot as u8), binding) {
                                karakuri_engine::set::Bound::Yes => (
                                    slot,
                                    key,
                                    format!(
                                        "{signal} through {curve} onto [{:.2}, {:.2}]",
                                        range[0], range[1]
                                    ),
                                ),
                                karakuri_engine::set::Bound::NoSuchParam => {
                                    return Some(format!(
                                        "  {}",
                                        karakuri_environment::no_such_param(slot, &key)
                                    ))
                                }
                                karakuri_engine::set::Bound::NoSuchControl => {
                                    return Some(format!(
                                        "  slot {slot}: `{signal}` is not a control this Set                                          publishes"
                                    ))
                                }
                            }
                    }
                    None => match deck.unbind(EngineSlot(slot as u8), layer, index, &key) {
                        true => (slot, key, "nothing — taken back".to_string()),
                        false => {
                            return Some(format!("  slot {slot}: nothing was driving `{key}`"))
                        }
                    },
                },
                Ok(_) => return None,
                Err(refused) => return Some(format!("  {refused}")),
            };
            Some(format!(
                "  source: deck {} -> Record::Source -> `{key}` is driven by {bound}",
                deck_letter(slot as u8)
            ))
        }
        // **Who may move one node**, and the writer ADR-0211 said the engine
        // owed. It is not enforcement: nothing writes a parameter on an
        // agent's behalf here, so what a level reaches today is
        // `Set::write_param`'s wildcard refusal — a bare-name control over
        // nodes that no longer agree is refused whole from the next press.
        Record::Authority { .. } => {
            let (slot, level) = match karakuri_environment::mix::change(record, deck.slot_count()) {
                Ok(Some(karakuri_environment::mix::Change::Authority {
                    slot,
                    layer,
                    index,
                    authority,
                })) => match deck.set_authority(EngineSlot(slot as u8), layer, index, authority) {
                    true => (slot, authority),
                    false => {
                        return Some(format!(
                            "  slot {slot}: no node {}:{index} to speak for",
                            karakuri_environment::meta::layer_name(
                                karakuri_environment::meta::layer_of(layer),
                            ),
                        ))
                    }
                },
                Ok(_) => return None,
                Err(refused) => return Some(format!("  {refused}")),
            };
            Some(format!(
                "  authority: deck {} -> SetAuthority -> Record::Authority -> {}",
                deck_letter(slot as u8),
                level.name()
            ))
        }
        Record::MasterOut { value } => {
            deck.set_out(value);
            Some(format!(
                "  master: out -> SetMasterOut {{ out: {value:.3} }} -> Record::MasterOut -> \
                 deck.out() = {:.3}, at the entry to the master chain",
                deck.out()
            ))
        }
        // **The chain itself, one record for all three passes.** Written whole
        // for `Record::Look`'s reason and applied whole: the value lands on
        // [`Engine::chain`] and the frame loop hands it to
        // `Present::set_chain`, so nothing here touches a uniform. That is the
        // look arm's arrangement one pass upstream, and it is why there is no
        // `set_chain` call in this function.
        //
        // **The cut comes back off the wire word, refused rather than
        // defaulted**, as the tone map operator and the blend mode do: a cut
        // this engine has not got is a stream saying something this build
        // cannot draw, and a default would silently play the other picture.
        //
        // **No clamp here either.** `Chain::clamped` is the wall and it is
        // inside the setter, so a stream carrying 4.0 meets the same ceiling
        // a fader does.
        Record::MasterChain(ref want) => {
            let mut slots = Vec::with_capacity(want.slots.len());
            for slot in &want.slots {
                slots.push(karakuri_engine::SlotSpec {
                    procedure: slot.procedure.clone(),
                    cut: match &slot.cut {
                        None => None,
                        Some(word) => Some(Cut::parse(word)?),
                    },
                    params: slot.params.clone(),
                });
            }
            let said = format!(
                "  master: chain -> Record::MasterChain -> {} slot{} — {}",
                slots.len(),
                if slots.len() == 1 { "" } else { "s" },
                if slots.is_empty() {
                    "the frame is the mix".to_string()
                } else {
                    slots
                        .iter()
                        .map(|s| {
                            let cut = s
                                .cut
                                .map(|c| format!(" ({})", c.name()))
                                .unwrap_or_default();
                            format!("{}{cut}", &s.procedure[..s.procedure.len().min(14)])
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            );
            *chain = slots;
            Some(said)
        }
        // **The whole of one slot's clock, because the record is a state and
        // not an ask.** `Record::Transport` carries the sync mode, the anchor
        // and the scrub together for a stated reason — a scrub position
        // without the mode and the anchor beside it *"would replay a slot onto
        // a grid it was never on"* — and `Deck::set_transport` is what it
        // decodes to. The mode and the anchor arrive here unchanged, from the
        // reading [`reading`] took a moment earlier; only the scrub has moved.
        //
        // **The mode comes back off the wire name, refused rather than
        // defaulted**, as the blend's, the residency's, the shape's and the
        // operator's do. Nothing in this file can produce a name the engine
        // has not got: the scrub only ever writes back the mode it just read
        // off the same deck, and the record is written from `Sync::name`.
        //
        // **`set_transport` refuses, and the refusal is dropped here on
        // purpose.** It refuses a mode this slot's material cannot honour, and
        // a scrub cannot present one — it names the mode the slot is already
        // in, which the slot is in because the engine allowed it. What it
        // guards against is the case the engine names at `sync_allowed`: a
        // swap that puts accumulating material into a slot that is beat-synced
        // makes the mode it is *already in* unavailable, *"and whatever wires
        // swapping to this owes it"*. **Both slots are watched now, so that
        // case is reachable**: save an accumulating procedure into a
        // beat-synced slot and the next scrub is refused. What should be
        // printed then is the deck's own refusal rather than this arm's line,
        // and it is what this owes.
        Record::Transport {
            slot,
            ref sync,
            anchor_bpm,
            scrub_beats,
        } => {
            let slot = held(deck, slot.0)?;
            let mode = EngineSync::from_name(sync)?;
            deck.set_transport(slot, mode, anchor_bpm, scrub_beats)
                .ok()?;
            Some(format!(
                "  scrub: deck {} -> ScrubDeck {{ deck: {slot} }} -> Record::Transport {{ \
                 sync: {sync}, anchor_bpm: {anchor_bpm:.1}, scrub_beats: {scrub_beats:+.2} }} \
                 -> deck.transport({slot}).scrub_beats() = {:+.2} beats",
                deck_letter(slot.0),
                deck.transport(slot).scrub_beats()
            ))
        }
        // **A choice and not a position, so nothing interpolates and nothing
        // is left running.** `Record::Select` carries the slot, the renderer
        // and the instant alone — *"half way to renderer 2" does not name a
        // picture* — and `Deck::schedule_selection` is what it decodes to: a
        // later selection on a slot replaces the earlier one, which is
        // `Deck::schedule`'s own rule for a fade.
        //
        // **The renderer is not checked here and is checked at the beat.** The
        // Set in a slot can change under a hot swap between the schedule and
        // the instant, so a selection that no longer names a renderer is
        // dropped where it is applied. The *slot* is checked, by `held`,
        // because a deck does not change size.
        //
        // **It says nothing about a slot that overdraws**, and that is carried
        // rather than refused: `Record::Select` names an edge into the Set's
        // L5 and a slot built without a composite fold has none, so refusing
        // it would make a replay fail on a line describing a performance that
        // happened.
        Record::Select {
            slot,
            renderer,
            start,
        } => {
            let slot = held(deck, slot.0)?;
            deck.schedule_selection(Selection::new(slot.index(), renderer as usize, start));
            Some(format!(
                "  renderer: deck {} -> SelectRenderer {{ renderer: {renderer} }} -> \
                 Record::Select {{ start: {start:.3} }} -> deck.selections_on({slot}) = {} \
                 armed, landing on the beat grid",
                deck_letter(slot.0),
                deck.selections_on(slot).count()
            ))
        }
        // **The session's grid, and the one record here that names no slot and
        // touches no deck control at all.** `Record::Tempo` is a correction —
        // a tempo, a phase shift and how much the estimate behind it was
        // believed — and `karakuri_environment::audio::apply_tempo` is *"the
        // only way a correction reaches the oscillator, live or on replay"*.
        // So this arm is the live half of that sentence, and it is one call
        // rather than a `signals.correct` beside it for exactly the reason
        // that function says so of itself.
        //
        // **The signals are copied out of the deck and back in**, which is
        // what `Deck::signals` and `set_signals` are for and is
        // [`measure_audio`]'s own line: the bus is a `Copy` value and the deck
        // is the model of record for it.
        //
        // **What reaches here today is `Operation::SetFreeRunTempo` and
        // nothing else**, which is *what the grid runs at with nothing driving
        // it* — the one control on this panel that works in the state this
        // program actually runs in, where there is no device and `tapped` and
        // `scaled` both refuse because there is no room. A tap and an octave
        // end in this same record and do **not** come through here: theirs is
        // the beat lock's answer and `written` cannot build it (ADR-0278), so
        // they are applied where they are computed.
        //
        // **The confidence is not printed and the shift is.** A hand-named
        // tempo carries `shift: 0.0` and `confidence: 0.0` — `Record::Tempo`'s
        // own words for a free-running tempo being stated — and a `0.0`
        // confidence beside a tempo an operator just chose would read as *this
        // is not believed*, which is the opposite of what it means. The shift
        // is printed because a beat that did not move is the claim this arm
        // makes.
        Record::Tempo { bpm, shift, .. } => {
            let mut signals = *deck.signals();
            karakuri_environment::audio::apply_tempo(&mut signals, record);
            deck.set_signals(signals);
            Some(format!(
                "  tempo: -> Record::Tempo {{ bpm: {bpm:.1}, shift: {shift:+.3} }} -> \
                 deck.signals().oscillator().bpm() = {:.1}, from now on and without moving a \
                 beat that has already happened",
                deck.signals().oscillator().bpm()
            ))
        }
        _ => None,
    }
}

/// The reading an operation's record needs, taken off the deck it names.
///
/// [`written`] builds `Record::Mask` whole — a shape, an angle, a position and
/// a softness — out of an operation that names two of the four, and the other
/// two come from a reading of the mask that is running (ADR-0201). This is that
/// reading, and it is the harness's because the deck is (ADR-0156, ADR-0194).
///
/// `Current::default()` is *I read nothing*, and it is still the answer for
/// four of this panel's emitting controls: a gain, an opacity, a blend mode and
/// a residency each carry everything their record carries, so handing a reading
/// in would be this file inventing a value. The mask mini is one of those that
/// need one, and it needs it for the deck the operation *names* rather than for
/// the deck the pointer is over — which is `Reading::Mask`'s own wording and
/// the reason this takes the operation and not a slot.
///
/// The two look controls are two of the other three, and they read one thing
/// between them: the look that is running. Each names a third of `Record::Look`
/// and the other two thirds come from here — which is [`Reading::Look`]'s own
/// wording and the reason the reading is taken for the operation rather than
/// per control.
///
/// The scrub's two arrows are the fourth, and the reading they take is the one
/// thing on this list that is not a completion: see the arm.
///
/// The sync chip and the anchor are the fifth and sixth, and they read the one
/// thing here that belongs to no deck: the session tempo. That arm used to be
/// absent and the two controls used to print a question instead of moving
/// anything — see [`unwritten`] for what the question turned out to be.
///
/// The softness is read back, where `karakuri-cli`'s `mix::current_mask`
/// substitutes its own `MASK_SOFTNESS`: that program writes wipes and has a
/// softness of its own to write, and this window has never written one. What is
/// read back here is therefore what is actually on the slot, and reading it
/// back is what stops a press rewriting it — the same argument the angle's is,
/// one field along.
///
/// The `go` capsule is the last of them and it is the one that reads three: a
/// wipe is written against the transition settings, the mask of the deck
/// arriving and where that deck already sits in the mix. The first is the
/// console's own — `settings` is what [`View::transition`] holds and what the
/// row's three pills move — and the other two are the deck's, taken for the
/// `to` slot and never for the `from`, which is `Current::mask`'s own wording:
/// everything a wipe writes is about the deck arriving.
///
/// A slot the deck has not got answers `None`, and [`written`] then says the
/// reading was owed rather than indexing something that is not there — the
/// guard [`apply`] has, at the other end of the same press.
///
/// The last reading is the session's four banks, and it is the one that is not
/// about a deck at all: which lanes of the armed pattern hold which controls,
/// so that a scheduled move on a fader a lane holds is refused before any
/// record is written (ADR-0323).
pub(crate) fn reading(
    operation: &Operation,
    deck: &Deck,
    look: &Look,
    chain: &[karakuri_engine::SlotSpec],
    settings: TransitionSettings,
    banks: &karakuri_pattern::Banks,
) -> Current {
    // The whole chain, for whichever slot was asked for: `Record::MasterChain`
    // carries the whole list and a press names one slot of it, so the chain
    // that is running is handed in and `written` takes the slots the press did
    // not name — and answers the position that is not in it.
    let master_chain = match *operation {
        Operation::SetChainParam { .. }
        | Operation::AddChainEffect { .. }
        | Operation::RemoveChainEffect { .. } => Some(mix::current_chain(chain)),
        _ => None,
    };
    let look = match *operation {
        // **Two thirds of the record, for whichever third was asked for.**
        // `SetTonemap` carries an operator and `SetExposure` a level, and
        // `Record::Look` needs all three — so the running look is handed in
        // and `written` takes the two the press did not name. That is
        // ADR-0192's argument executable in this file: the operation carries
        // what a surface can say and the translator completes the record.
        // `white_point` is on no surface at all, so it survives every press by
        // arriving here and going straight back out.
        Operation::SetTonemap { .. } | Operation::SetExposure { .. } => {
            Some(karakuri_operation_record::Look {
                tonemap: mix::tonemap(look.op),
                exposure: look.exposure,
                white_point: look.white_point,
            })
        }
        _ => None,
    };
    // **The scrub is the third reading, and it is the only one that is
    // relative.** `Record::Transport` is absolute — a sync mode, an anchor and
    // a scrub position — and `ScrubDeck` names an amount, so the record is
    // where the slot already is plus what was asked for. The reading is
    // therefore not a completion of a record the way the look's and the mask's
    // are: it is the left-hand side of an addition, and without it the
    // conversion answers `Owed(NotRead(Transport))` rather than starting a
    // deck's scrub from zero.
    //
    // **All three fields, because the record is written whole.** A scrub that
    // wrote a position without the mode and the anchor beside it *"would
    // replay a slot onto a grid it was never on"* —
    // `karakuri_operation_record::Transport` says so at its own definition —
    // and the two it does not touch survive the press by arriving here and
    // going straight back out, which is the white point's arrangement one
    // reading up.
    let transport = match *operation {
        Operation::ScrubDeck { deck: slot, .. } => {
            EngineSlot::new(slot, deck.slot_count()).map(|slot| {
                let transport = deck.transport(slot);
                karakuri_operation_record::Transport {
                    sync: mix::sync(transport.sync()),
                    anchor_bpm: transport.anchor_bpm(),
                    scrub_beats: transport.scrub_beats(),
                }
            })
        }
        _ => None,
    };
    // **The tempo the room is going at, and nothing about a slot.** Engaging
    // a sync mode anchors the slot at the session tempo so that the picture
    // does not move at the instant it goes on the grid, which is
    // `karakuri_engine::transport::Transport::engaged`'s policy and the whole
    // of what this reading is for. `mix::current_tempo` takes the oscillator
    // rather than an `f32`, so this window cannot hand in a tempo the session
    // never ran at — and it reads the grid rather than a clock, which is what
    // lets the record be replayed
    // ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
    //
    // **No slot check, unlike the three below.** The tempo is the session's,
    // so a `SetSync` naming a slot the deck has not got is a record with a slot
    // out of range rather than a reading that could not be taken, and
    // `apply`'s own guard is what says so at the other end of the press.
    let tempo = match *operation {
        Operation::SetSync { .. } => Some(mix::current_tempo(deck.signals().oscillator())),
        _ => None,
    };
    // **Three operations read this and one of them names two decks.** A wipe
    // writes `Record::Mask` for the deck *arriving* — twice, at the front's
    // present position and then at 0 — so the mask handed over is `to`'s and
    // `from` is read for nothing at all. Handing in the covered deck's would
    // put somebody else's soft edge on the front that is about to cross the
    // frame, which is `Current::mask`'s own sentence and `karakuri-cli`'s
    // `Live::operate` arm exactly.
    //
    // **`SetMaskPosition` was missing from this list until 2026-09-10**, and
    // it is the one arm ADR-0334 recorded as a defect rather than a scope:
    // both halves of a mask write `Record::Mask` whole, so a position needs
    // the shape, the angle and the soft edge it does not name, and without
    // this `written` answered `Owed(NotRead(Mask))` and nothing moved. It was
    // reachable from a mapped controller before it was reachable from a model,
    // so the hole was a MIDI knob that did nothing as well as a call that
    // could not be accepted. `karakuri-cli`'s arm has always named all three,
    // which is what makes this a slip in one file rather than a decision.
    let mask = match *operation {
        Operation::SetMaskShape { deck: slot, .. }
        | Operation::SetMaskPosition { deck: slot, .. }
        | Operation::Wipe { to: slot, .. } => {
            EngineSlot::new(slot, deck.slot_count()).map(|slot| {
                let mask = deck.mask(slot);
                karakuri_operation_record::Mask {
                    kind: wipe_kind(mask.kind()),
                    angle: mask.angle(),
                    position: mask.position(),
                    softness: mask.softness(),
                }
            })
        }
        _ => None,
    };
    // **Every field named, and no `..Current::default()` behind them.** Every
    // reading the type carries is answered here, so a fill would be dead — and
    // the day it grows one more, this stops compiling and somebody has to say
    // whether this window can take it, rather than a `None` arriving silently.
    //
    // **The count that used to be in this sentence is gone rather than
    // corrected.** It said four where there are three and named a fifth that
    // would be a fourth, which is a figure nothing checks going stale in the
    // one comment whose whole argument is that the compiler does the checking.
    //
    // **The transition settings used to be answered `None` here, on the
    // grounds that this panel drew no control that set any of them.** That
    // sentence was true of a window with no transition row in it and is not
    // true of this one: the row is drawn, its three pills emit
    // `Operation::SetTransition`, and `View::transition` is the model of
    // record for what they arrive at. So the reading is taken, and it is the
    // one here that is read off the *console* rather than off the deck —
    // a quantum, a length and a wipe shape are a surface's setting deciding
    // what the next move means, and this surface now holds one.
    //
    // **Four operations, which is every one that schedules a move**, and it
    // is `karakuri-cli`'s `Live::operate` arm exactly: only the wipe has a
    // control on this panel today, and the other three are answered because
    // what the reading *is* does not depend on which surface asked. A
    // conversion that came back `Owed(NotRead(Transition))` for a fade the
    // day a fader learned to schedule one would be this arm having to be
    // found again.
    //
    // `mix::current_transition` takes the oscillator and the quantum rather
    // than an instant, so the start is
    // `karakuri_engine::transition::quantise`'s answer and this file cannot
    // hand in a beat the grid was never on — the arrangement `mix::current_tempo`
    // is in one reading up
    // ([P-0092](../../../docs/principles/0092-the-same-inputs-produce-the-same-frame.md)).
    // `mix::FADE_CURVE` is the shape every scheduled fade takes, and it is
    // asked for by name rather than spelled here because two surfaces easing
    // one fade differently is a value a replay carries.
    let transition = match *operation {
        Operation::FadeDeck { .. }
        | Operation::Crossfade { .. }
        | Operation::SelectRenderer { .. }
        | Operation::Wipe { .. } => Some(mix::current_transition(
            deck.signals().oscillator(),
            settings.quantum,
            settings.length,
            mix::FADE_CURVE,
            mask_kind(settings.kind),
            settings.angle,
        )),
        _ => None,
    };
    // **Where the deck a wipe is arriving on already sits in the mix**, and
    // the one reading here taken so that a record can be left *out* rather
    // than written. A wipe puts that deck under `over` and on air, and both
    // are a state it may be in already: under `add` the same gesture is a wipe
    // *on* rather than a wipe *over*, a different picture and a legitimate
    // one, so the mode is left where the operator put it. The condition is the
    // conversion's and what it needs to hold it is this
    // ([P-0094](../../../../docs/principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)).
    //
    // **What the deck reports rather than what it was asked for**, which is
    // `Deck::residency`'s answer: the governor may hold a slot below the
    // request, and what a wipe needs to know is whether the put-on-air it is
    // about to write would change anything.
    let mix = match *operation {
        Operation::Wipe { to: slot, .. } => EngineSlot::new(slot, deck.slot_count())
            .map(|slot| mix::current_mix(deck.blend(slot), deck.residency(slot))),
        _ => None,
    };
    // **The armed pattern's lanes, handed in for every operation** — unlike the
    // readings above, which are taken for the operations that need them. A
    // `transition` this file forgot to hand over is said out loud
    // (`Owed::NotRead`); a `lanes` this file forgot to hand over refuses
    // nothing and says nothing, so it is taken once here rather than off a
    // second list of which operations can be refused (ADR-0323).
    //
    // It is the armed bank alone, because a lane in a bank that is not armed
    // drives nothing: `Banks::pattern` is what the sequencer polls.
    let lanes = Some(karakuri_operation_record::Lanes {
        held: banks
            .pattern()
            .held()
            .map(|(at, target)| (at, target.clone()))
            .collect(),
    });
    Current {
        look,
        master_chain,
        mask,
        transport,
        tempo,
        transition,
        mix,
        lanes,
    }
}
