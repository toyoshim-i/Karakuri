use super::*;

/// menu that is empty because the directory would not open looks exactly like
/// one that is empty because nobody has saved.
///
/// Read when it changes rather than per frame. Once at startup, and again after
/// a save lands — which is the only thing in this program that adds a name.
/// `Store::list_arrangements` sorts by name, so the menu draws what it is
/// handed and sorts nothing.
pub(crate) fn arrangements(root: &std::path::Path) -> Vec<String> {
    if !root.is_dir() {
        return Vec::new();
    }
    match Store::open(root).and_then(|store| store.list_arrangements()) {
        Ok(filed) => filed.into_iter().map(|entry| entry.name).collect(),
        Err(e) => {
            println!("arrangements: {} could not be listed: {e}", root.display());
            Vec::new()
        }
    }
}

/// A star put on a Set or taken off it, and the second route in this program
/// that both reads an operation and reaches a disk.
///
/// # It is [`arrangement`]'s shape and sits beside it for its reason
///
/// The panel cannot reach the store (ADR-0156) and the store cannot reach the
/// panel, so the two halves meet in a third party and this file is it. It
/// answers `None` for every other operation, which is what lets it sit on the
/// one path an emitted operation already takes rather than being a second route
/// into the store.
///
/// # What it writes, and what it deliberately does not
///
/// `Store::set_favourite` — one stat, one atomic write of
/// `<store>/favourites.json`, and the whole of the layout question is
/// ADR-0299's rather than this file's. Nothing here re-lists: the marks the bay
/// draws and the rows `my sets` holds are both [`listing`]'s answer, and a
/// second derivation here would be a second answer to *what is starred* with a
/// file write between them. The caller re-lists on the same branch it re-lists
/// a scope press on.
///
/// # Who asked decides where it lands, and for a star there is nowhere else
///
///
/// [P-0096](../../../../docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md)
/// is the actor and not the flag, and `my sets` is by construction the list of
/// Sets the operator chose — so a model's star must not reach
/// `<store>/favourites.json`. That much is
/// [ADR-0261](../../../docs/adr/0261-a-model-asked-save-lands-in-a-sandbox-because-the-operators-library-is-the-operators-own-act.md)'s
/// rule applied one control along, and it is why this branches on [`Asked`]
/// exactly as `karakuri_environment::filed_as` does for a save.
///
/// Where the two part company is the second directory. A model's save lands in
/// `<store>/sandbox/` because what lands there is *material* — an edit-history
/// snapshot an operator goes looking for after a show — so refusing it would
/// lose an evening of work. A star is one bit whose whole meaning is *this row
/// appears under `my sets`*, so a sandbox favourites file would be a list no
/// scope lists, no tool reads and the operator never sees, while the model was
/// told it had succeeded. So a model's star is refused out loud (ADR-0301), and
/// the refusal names the id and says where the Set is — which is the same shape
/// the class pills' refusals take, so that a model can tell the person beside
/// it which mark to press.
///
/// `Standing::Open` stays and `gate.rs` is untouched. The refusal is the
/// performer's and not the gate's, exactly as a model's save is not refused at
/// the gate but filed somewhere else by whoever performs it.
///
/// The model arm is written before the route is, which is [`arrangement`]'s own
/// position: no tool publishes `SetFavourite` today, the page's MCP badge is
/// `plan`, and a control that arrives at this function finds the rule already
/// here rather than adding it.
///
/// # The three things it can say, and each is said out loud
///
/// The state was already the one asked for, which is `Ok(false)` and is an
/// ordinary answer rather than a refusal: the operation names a state and not a
/// toggle, so a second press of *star this* says the same thing again and the
/// file's own time is not touched. The Set is not one this store holds, which
/// is `StoreError::NoSet` carrying the id back
/// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md))
/// — a `presets` or a `folder` row is a file rather than a Set of this
/// library's, and starring one is refused with the sentence saying so. Taking a
/// star off is never refused, which is the asymmetry that makes a mark left
/// behind by a file somebody deleted clearable from the row it no longer draws.
///
/// Never panics, for [`arrangement`]'s reason: a panic reachable from an event
/// handler aborts this process rather than unwinding.
pub(crate) fn favourite(
    root: &std::path::Path,
    asked: Asked,
    operation: &Operation,
) -> Option<String> {
    let Operation::SetFavourite { id, favourite } = operation else {
        return None;
    };
    if asked == Asked::Model {
        return Some(format!(
            "star: `{id}` was not {} — `my sets` is the list of Sets the operator chose, and a \
             star is theirs to put on: it is one press on the mark at the left of that row in \
             the Library bay, under `all`. The Set itself is untouched and is listed there",
            match favourite {
                true => "starred",
                false => "unstarred",
            }
        ));
    }
    let wrote = Store::open(root).and_then(|store| store.set_favourite(id, *favourite));
    Some(match wrote {
        Ok(true) => format!(
            "star: `{id}` {} — `my sets` is the starred subset of `all`, and this row is {} \
             it",
            match favourite {
                true => "is starred",
                false => "has its star off",
            },
            match favourite {
                true => "in",
                false => "out of",
            }
        ),
        Ok(false) => format!(
            "star: `{id}` was already {}, so nothing was written",
            match favourite {
                true => "starred",
                false => "unstarred",
            }
        ),
        Err(e) => format!(
            "star: `{id}` was not {}: {e}",
            match favourite {
                true => "starred",
                false => "unstarred",
            }
        ),
    })
}

/// Where a named arrangement is kept and put back, and the one route in
/// this program that both reads an operation and reaches a disk.
///
/// # Why it is here, in a package neither side depends on
///
/// The panel cannot reach the store. `karakuri-console` dropped
/// `karakuri-store` when this program moved out of it, and the drop was the
/// point — a crate that takes no device and no disk is what ADR-0156 bought,
/// and its manifest now has no entry that could be reached for at all. The
/// store cannot reach the panel either: `karakuri-store`'s `src/` must not
/// name `karakuri-layout`, so it keeps an arrangement as bytes it does not
/// understand, exactly as it keeps `.kir` source
/// ([ADR-0221](../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)
/// §4). So the two halves meet in a third party, and this file is the third
/// party — the same position it holds for a record, where the vocabulary
/// says what to write and only somebody holding a `Deck` can apply it
/// ([`apply`]).
///
/// # The route, and where each half of it is decided
///
/// - Saving is `serde_json::to_vec` of [`Panel::layout`] into
///   `Store::write_arrangement`, which is ADR-0221's own sentence. The format
///   is `karakuri-layout`'s hand-written `Serialize`, so an unbounded maximum
///   goes out as an explicit absence rather than as an infinity JSON cannot
///   spell, and a `NodeId` goes out as the bare number it is.
/// - Putting one back is `Store::read_arrangement`, `serde_json` into a
///   [`Layout`], and [`Panel::restore`]. Neither this file nor the panel
///   checks the arrangement: `Layout`'s `TryFrom<Wire>` is the one place a
///   file that disagrees with itself is refused rather than repaired
///   (ADR-0158), and a check here would be a second answer to a question that
///   already has one.
///
/// # What it does with each of the three ways it can fail
///
/// Says it and moves nothing, and never panics: a panic reachable from an
/// event handler aborts this process rather than unwinding (see the module
/// documentation). The three are a store it could not open or write, a name
/// nothing is filed under, and a file that will not read back — and the third
/// is the one that has to be told apart from the second, because *there is no
/// such arrangement* and *the arrangement you saved is broken* send an
/// operator to two different places.
///
/// A name nothing is filed under never falls back to the default.
/// `Store::read_arrangement` answers `StoreError::NoArrangement(name)` and
/// that sentence carries the name, which is the whole reason the store has a
/// fourth error variant rather than reusing `NotFound`: an operator who
/// mistyped a name needs to be told the name, not to watch their console reset
/// (ADR-0221 §2).
///
/// # The store is created by a save and not by a restore
///
/// [`library`] refuses to create one, because *"a program that listed a
/// library by first making one would change the directory it was run in"*, and
/// a restore is a read on exactly those terms. A save is the case
/// `Store::open` establishing the layout is right for — it is a program that
/// is about to write — so the two halves below differ, deliberately, and the
/// restore's guard is what keeps `cargo run -p karakuri` in somebody's home
/// directory from leaving a `.karakuri` behind for having asked a question.
///
/// # It answers `None` for every other operation
///
/// Which is what lets it sit on the one path every emitted operation already
/// takes ([`App::performed`]) rather than being a second route into the
/// panel. The transport row's arrangement pill emits both, and the
/// manual's two rows say it is the only one of the four surfaces that can: a
/// `panel` badge each and three empty ones, because a name is what a key
/// press, a map line and an unpublished tool each have no way to say. This
/// wiring was written before that control existed — exactly as [`unwritten`]
/// is written for controls that do not exist yet — and the control is what
/// arrived at it.
pub(crate) fn arrangement(
    root: &std::path::Path,
    panel: &mut Panel,
    arr: &mut view::Arrangement,
    operation: &Operation,
) -> Option<String> {
    match operation {
        Operation::SaveArrangement { name } => {
            let (line, kept) = keep_arrangement(root, panel, name);
            // **The name in use moves only when the file did.** A save that
            // was refused leaves the pill saying what it said, because
            // nothing under that name is on the disk — and the listing is
            // re-read only then, since a refusal added no name to it.
            if kept {
                arr.name = Some(name.clone());
                arr.filed = arrangements(root);
            }
            Some(line)
        }
        Operation::RestoreArrangement { name } => {
            let (line, back) = put_arrangement_back(root, panel, name);
            if back {
                arr.name = Some(name.clone());
            }
            Some(line)
        }
        _ => None,
    }
}

/// The running arrangement, filed under `name` — and whether it landed. See
/// [`arrangement`].
pub(crate) fn keep_arrangement(
    root: &std::path::Path,
    panel: &Panel,
    name: &str,
) -> (String, bool) {
    if let Err(refusal) = checked_name(name) {
        return (refusal, false);
    }
    let bytes = match serde_json::to_vec(panel.layout()) {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                format!("arrangement: `{name}` was not kept — it did not serialise: {e}"),
                false,
            )
        }
    };
    let wrote = Store::open(root).and_then(|store| store.write_arrangement(name, &bytes));
    match wrote {
        Ok(()) => (
            format!(
                "arrangement: kept as `{name}` — {} bytes at {}",
                bytes.len(),
                root.join("arrangements")
                    .join(format!("{name}.arrangement.json"))
                    .display()
            ),
            true,
        ),
        Err(e) => (format!("arrangement: `{name}` was not kept: {e}"), false),
    }
}

/// The one place a typed arrangement name is refused, and the reason it is here
/// rather than in the pill that took the letters.
///
/// `<name>` becomes one path component under `<store>/arrangements/`, and
/// `karakuri-store` says outright that nothing there checks it: *"`<name>`
/// becomes one path component and that is the caller's rule to keep"*
/// ([ADR-0221](../../../docs/adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)
/// §1, which names letters, digits, `-` and `_`). So `../../elsewhere` is a
/// path, and a path never reaches that call from here.
///
/// The surface owns the affordance and never the authority
/// ([P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)):
/// the pill takes whatever is typed and this is where it meets the wall, so a
/// name refused by a hand and a name refused by anything else that ever reaches
/// this operation meet the same one. A pill that silently dropped the
/// characters it did not like would be a rule an operator could only find by
/// experiment — which is the failure the console page names about a control
/// that quietly declines.
///
/// It says the same three things `mcp::checked_id` says about a Set id, which
/// is
/// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
/// as far as it can be kept today and no further: that function is private to
/// `karakuri-environment`'s `mcp` module and its sentences say `id` and
/// `<store>/sets/`, so it cannot be called from here and could not be quoted if
/// it were. When an arrangement name gets a second surface — a map line, an MCP
/// tool, a `--restore-arrangement` flag — the two collapse into one shared
/// `checked_name`, and this comment is where whoever does it should start.
pub(crate) fn checked_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err(
            "arrangement: nothing was typed, and an arrangement is filed under a name — \
             the default arrangement is the one that has none"
                .to_owned(),
        );
    }
    match name
        .chars()
        .find(|c| !c.is_ascii_alphanumeric() && *c != '-' && *c != '_')
    {
        Some(bad) => Err(format!(
            "arrangement: `{name}` holds `{bad}`, and an arrangement name is letters, digits, \
             `-` and `_`: it is one path component and it names a file under \
             `<store>/arrangements/`"
        )),
        None => Ok(()),
    }
}

/// The arrangement filed under `name`, into the window the panel already has.
/// See [`arrangement`].
pub(crate) fn put_arrangement_back(
    root: &std::path::Path,
    panel: &mut Panel,
    name: &str,
) -> (String, bool) {
    if !root.is_dir() {
        return (
            format!(
                "arrangement: no store at {}, so nothing is filed under `{name}` — and the \
                 console has not moved",
                root.display()
            ),
            false,
        );
    }
    let bytes = match Store::open(root).and_then(|store| store.read_arrangement(name)) {
        Ok(bytes) => bytes,
        Err(e) => return (format!("arrangement: `{name}` is not back — {e}"), false),
    };
    let layout: Layout = match serde_json::from_slice(&bytes) {
        Ok(layout) => layout,
        // **Refused whole rather than repaired**, which is the loader's own
        // sentence and not this file's judgement (ADR-0158).
        Err(e) => {
            return (
                format!(
                    "arrangement: `{name}` is not back — the file disagrees with itself and is \
                     refused rather than repaired: {e}"
                ),
                false,
            )
        }
    };
    let viewport = panel.layout().viewport();
    panel.restore(layout);
    (
        format!(
            "arrangement: `{name}` is back, at the {} x {} this window already had",
            viewport.w, viewport.h
        ),
        true,
    )
}
