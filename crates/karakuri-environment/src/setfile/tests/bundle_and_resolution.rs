#![allow(unused_imports)]

use super::common::*;

// -- Bundling --------------------------------------------------------

/// The text a bundle is written out as, back through the reader — so a test
/// round-trips through the *file*, which is what `--package >` writes and what
/// `--take-in` reads, rather than through records held in memory that could not
/// have survived a serialisation.
fn as_a_file(lines: &[Line]) -> Vec<Line> {
    parsed(
        &lines
            .iter()
            .map(|line| format!("{}\n", line.as_str()))
            .collect::<String>(),
    )
}

/// The round trip both flags exist for: a bundle written out of one store loads
/// in a store that has never held its artifacts.
///
/// `inlined_source_loads_without_a_store_that_knows_the_artifact` above proves
/// the *reader* does that, from `src` records a test hand-built. This is the
/// writing half beside it: nothing here spells a record out — [`bundle`]
/// produces the file and [`unbundle`] takes it in, and the material arrives on
/// the far side as procedures with their own names.
///
/// And the cards come with it. An artifact whose card is missing is an ordinary
/// store rather than a damaged one, so this is not the difference between a
/// bundle that works and one that does not — but a bundle that dropped them
/// would leave every library taken in thinner than the one it came from,
/// silently.
#[test]
fn a_bundle_loads_in_a_store_that_has_never_seen_the_artifacts() {
    let (_dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let sent = as_a_file(&bundle(&store, "s1").expect("bundle"));

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let bare = Store::open(elsewhere.path()).expect("store");
    let said = unbundle(&bare, CameFrom::Somebody, &sent).expect("the file carries its own source");
    assert!(said.contains("`s1`"), "{said}");

    let loaded = load(&bare, "s1").expect("the set is filed and its artifacts are here");
    assert_eq!(loaded.l1s[0].name, "ring");
    assert_eq!(loaded.l4s[0].name, "points");
    for hash in [
        Hash::of(&std::fs::read(&l1).expect("read")),
        Hash::of(&std::fs::read(&l4).expect("read")),
    ] {
        bare.read_meta(&hash)
            .unwrap_or_else(|e| panic!("{}: {e}", hash.short(12)));
    }
}

/// A bundle missing one procedure is refused whole, naming it.
///
/// The alternative is a file that looks self-contained and is not, whose
/// failure surfaces on somebody else's machine — where the artifact it wants is
/// not, and never was.
#[test]
fn a_bundle_is_refused_when_the_store_lacks_a_source() {
    let (_dir, store, l1, _l4) = fixture();
    let here = stored(&store, &l1);
    let missing = Hash::of(b"a renderer that was never put in this store");
    let text = format!(
        r#"{{"t":"set","id":"gone","v":1}}
{{"t":"slot","layer":"L1","proc":"{here}"}}
{{"t":"slot","layer":"L4","name":"veil","proc":"{missing}"}}
"#
    );
    store.write_set("gone", &parsed(&text)).expect("write");

    let e = bundle(&store, "gone").expect_err("a bundle cannot carry what is not there");
    assert!(e.contains("veil"), "the node is not named: {e}");
    assert!(e.contains(&missing.short(12)), "{e}");
}

/// Two nodes over one artifact inline it once. The reader keys `src` by hash,
/// so a second run would be a second copy of the same bytes that nothing ever
/// reads — and this Set is one geometry drawn twice by the same renderer, which
/// is the ordinary way that happens.
#[test]
fn one_artifact_referenced_twice_is_inlined_once() {
    let (_dir, store, l1, l4) = fixture();
    let twice = vec![l4.clone(), l4.clone()];
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, &twice), &[]),
    )
    .expect("save");
    let bundled = bundle(&store, "s1").expect("bundle");

    let renderer = Hash::of(&std::fs::read(&l4).expect("read"));
    let slots = bundled
            .iter()
            .filter(|line| matches!(line.record(), Record::Slot { proc_hash, .. } if *proc_hash == renderer))
            .count();
    assert_eq!(slots, 2, "the fixture is meant to name the renderer twice");
    // One run, so line 0 appears once.
    let heads = bundled
        .iter()
        .filter(
            |line| matches!(line.record(), Record::Src { hash, line: 0, .. } if *hash == renderer),
        )
        .count();
    assert_eq!(heads, 1, "the renderer's source was inlined {heads} times");
    // And the run is whole: as many `src` records as the source has lines.
    let run = bundled
        .iter()
        .filter(|line| matches!(line.record(), Record::Src { hash, .. } if *hash == renderer))
        .count();
    assert_eq!(run, L4.split('\n').count());
}

/// A source that does not hash to the address its `slot` names is refused, and
/// nothing is stored.
///
/// This is the check that makes a bundle worth trusting at all: without it a
/// `src` run is a way to file arbitrary text under an address the operator on
/// the far side recognises, and every guarantee content addressing makes is
/// gone. Refusing *after* storing some of it would be nearly as bad — the store
/// would hold half a stranger's file.
#[test]
fn an_unbundle_refuses_a_source_that_does_not_hash_to_its_address() {
    let (_dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let renderer = Hash::of(&std::fs::read(&l4).expect("read"));
    // One line of the renderer's inlined source rewritten, everything else
    // — the `slot` record's hash included — left exactly as written.
    let tampered: Vec<Line> = bundle(&store, "s1")
        .expect("bundle")
        .into_iter()
        .map(|line| match line.record() {
            Record::Src { hash, line: at, .. } if *hash == renderer && *at == 1 => {
                Line::new(Record::Src {
                    hash: renderer,
                    line: 1,
                    s: "proc points_but_not_really {".to_string(),
                })
            }
            _ => line,
        })
        .collect();

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let bare = Store::open(elsewhere.path()).expect("store");
    let e = unbundle(&bare, CameFrom::Somebody, &as_a_file(&tampered))
        .expect_err("text that is not the bytes its address names");
    // The node, by the address the file gives it and by what it is called
    // — which in a store with no card for it is its short hash.
    assert!(e.contains("L4:0"), "the node is not named: {e}");
    assert!(e.contains(&renderer.short(12)), "{e}");
    assert!(
        bare.list_artifacts().expect("list").is_empty(),
        "a refused bundle left an artifact behind"
    );
    assert!(
        bare.list_sets().expect("list").is_empty(),
        "a refused bundle left a Set file behind"
    );
}

/// An id already taken is refused, and the Set that was there is left exactly
/// as it was.
///
/// Deliberately not `--save-set`'s rule, which overwrites: an id you type is an
/// instruction, and an id that arrived inside somebody else's file is not. The
/// bytes are compared before and after, because "it refused" and "it refused
/// without having written" are two different claims.
#[test]
fn an_unbundle_refuses_an_id_already_taken_and_leaves_the_set_alone() {
    let (dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let sent = as_a_file(&bundle(&store, "s1").expect("bundle"));

    // Somebody else's store, with a Set of their own under that word: one
    // geometry drawn by two renderers, where the bundle names one.
    let elsewhere = tempfile::tempdir().expect("tempdir");
    let theirs = Store::open(elsewhere.path()).expect("store");
    let l2 = beside(&dir, "theirs.kir", L2);
    let mine = vec![
        Node {
            hash: stored(&theirs, &l1),
            layer: Kind::L1,
            index: 0,
            name: None,
        },
        Node {
            hash: stored(&theirs, &l2),
            layer: Kind::L2,
            index: 0,
            name: Some("preset".to_string()),
        },
        Node {
            hash: stored(&theirs, &l4),
            layer: Kind::L4,
            index: 0,
            name: None,
        },
    ];
    save(&theirs, Asked::Operator, "s1", plain(&mine, &[])).expect("save");
    let before = written(&theirs, "s1");

    let e = unbundle(&theirs, CameFrom::Somebody, &sent)
        .expect_err("an id that arrived in a file is not typed");
    assert!(e.contains("`s1`"), "the id is not named: {e}");
    assert_eq!(before, written(&theirs, "s1"), "the preset was overwritten");
}

/// The shipped library replaces its own row, and the Set that was there is kept
/// under a stamped id (ADR-0347).
///
/// The pair of the test above: same taken id, same store, one argument
/// different. "It wrote" would pass on a plain overwrite, so what is asserted
/// is that both Sets are there afterwards, that the plain id holds the new
/// material, and that the retired copy names itself by the id it is filed
/// under.
#[test]
fn a_shipped_row_replaces_a_taken_id_and_what_was_there_is_kept() {
    let (dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let sent = as_a_file(&bundle(&store, "s1").expect("bundle"));

    // An `s1` of its own: the same geometry under a second renderer, so the id
    // is taken and the material differs.
    let elsewhere = tempfile::tempdir().expect("tempdir");
    let theirs = Store::open(elsewhere.path()).expect("store");
    let other = beside(&dir, "other.kir", L4);
    save(
        &theirs,
        Asked::Operator,
        "s1",
        plain(&ordinary(&theirs, &l1, &[l4.clone(), other]), &[]),
    )
    .expect("save");
    let before = written(&theirs, "s1");

    let said = unbundle(&theirs, CameFrom::TheShippedLibrary, &sent)
        .expect("the shipped library may replace its own row");

    let held: Vec<String> = theirs
        .list_sets()
        .expect("list")
        .into_iter()
        .map(|entry| entry.id)
        .collect();
    let retired: Vec<&String> = held.iter().filter(|id| *id != "s1").collect();
    assert_eq!(
        retired.len(),
        1,
        "the copy that was replaced was not kept: {held:?}"
    );
    let retired = retired[0];
    assert!(
        retired.starts_with("s1-"),
        "the retired copy is filed as `{retired}`"
    );
    assert!(
        said.contains(retired) && said.contains("kept as"),
        "the operator is not told where it went: `{said}`"
    );

    assert_eq!(
        written(&theirs, retired),
        before.replace("\"id\":\"s1\"", &format!("\"id\":\"{retired}\"")),
        "the retired copy is not the Set that was there, under its new id"
    );
    assert_eq!(
        written(&theirs, "s1"),
        written(&store, "s1"),
        "the plain id does not hold what the library ships"
    );
}

/// A shipped row this store already holds unchanged writes nothing, says so,
/// and is not an error.
///
/// Two claims. Not an error, because a press is a take-in and then a load, and
/// the refusal this replaced meant nothing loaded. Nothing written, because the
/// alternative files a dated copy of an unchanged Set on every press.
#[test]
fn a_shipped_row_this_store_already_holds_writes_nothing_and_says_so() {
    let (_dir, store, l1, l4) = fixture();
    save(
        &store,
        Asked::Operator,
        "s1",
        plain(&ordinary(&store, &l1, std::slice::from_ref(&l4)), &[]),
    )
    .expect("save");
    let sent = as_a_file(&bundle(&store, "s1").expect("bundle"));

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let theirs = Store::open(elsewhere.path()).expect("store");
    unbundle(&theirs, CameFrom::TheShippedLibrary, &sent).expect("the first take-in");
    let before = written(&theirs, "s1");

    let said = unbundle(&theirs, CameFrom::TheShippedLibrary, &sent)
        .expect("a preset already current is not a refusal");
    assert!(
        said.contains("already what the preset library ships"),
        "what the operator reads is `{said}`"
    );
    assert_eq!(
        before,
        written(&theirs, "s1"),
        "an unchanged preset rewrote the Set file"
    );
    assert_eq!(
        theirs.list_sets().expect("list").len(),
        1,
        "an unchanged preset filed a retired copy of itself"
    );
}

/// A source this build cannot compile is stored, keeps its slot, and is
/// reported.
///
/// Refusing the whole file would tell an operator that *something* is wrong.
/// Storing it means `--load-set` fails against the source itself, with the
/// checker's span and hint on the line that is wrong — which is a thing they
/// can fix. So the note says which node and what the checker said, and the
/// artifact is on disk to be read and edited.
#[test]
fn an_unbundle_stores_a_source_that_does_not_compile_and_says_so() {
    let broken = "proc veil {\n  kind L4\n  this is not a renderer\n}\n";
    let renderer = Hash::of(broken.as_bytes());
    let geometry = Hash::of(L1.as_bytes());
    let mut lines = vec![
        Line::new(Record::Set {
            id: "sent".to_string(),
            v: VERSION,
        }),
        Line::new(Record::Slot {
            at: NodeAddress {
                layer: Layer::L1,
                index: 0,
            },
            name: None,
            proc_hash: geometry,
        }),
        Line::new(Record::Slot {
            at: NodeAddress {
                layer: Layer::L4,
                index: 0,
            },
            name: Some("veil".to_string()),
            proc_hash: renderer,
        }),
    ];
    for (hash, src) in [(geometry, L1), (renderer, broken)] {
        for (n, text) in src.split('\n').enumerate() {
            lines.push(Line::new(Record::Src {
                hash,
                line: n as u32,
                s: text.to_string(),
            }));
        }
    }

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let bare = Store::open(elsewhere.path()).expect("store");
    let said = unbundle(&bare, CameFrom::Somebody, &as_a_file(&lines))
        .expect("one bad source is not a refusal");

    let report = crate::compile::check(broken).expect_err("the fixture must not compile");
    let first = report.lines().next().expect("a diagnostic").trim();
    assert!(said.contains("veil"), "the node is not named: {said}");
    assert!(
        said.contains(first),
        "the checker's own words are not in the note: {said}"
    );
    bare.get_artifact(&renderer)
        .expect("a source that will not compile is still stored");
    assert!(
        bare.read_meta(&renderer).is_err(),
        "a card was written for a source that never compiled"
    );
    bare.read_meta(&geometry).expect("the good one is carded");
    let filed = bare.read_set("sent").expect("the set is filed");
    assert_eq!(
        filed
            .iter()
            .filter(|line| matches!(line.record(), Record::Slot { .. }))
            .count(),
        2,
        "the node that will not compile lost its slot"
    );
}

// -- The authoring form: resolution, and the wall around it -----------

/// An authoring Set file beside the two `.kir` the fixture wrote, naming them
/// by the relative paths they actually have.
///
/// Written by hand rather than by a writer, because there is no writer: a
/// `.kset` is a file a person authors, and what these tests are about is
/// reading one somebody else wrote.
fn authored(dir: &tempfile::TempDir, name: &str, parts: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(
        &path,
        format!("{{\"t\":\"set\",\"id\":\"authored\",\"v\":1}}\n{parts}"),
    )
    .expect("write");
    path
}

/// A `.kset` resolves to the `.kbset` it names, with its parts in the store as
/// artifacts.
///
/// The whole of what resolution is, checked as three separate facts because two
/// of them can hold while the third does not: every `part` has become a `slot`,
/// each `slot` names the content address of the bytes on disk, and the store
/// can hand those bytes back. A resolver that emitted the right records and
/// stored nothing would pass the first two and produce a file nobody can load.
#[test]
fn a_kset_resolves_to_the_kbset_it_names_with_its_parts_in_the_store() {
    let (dir, store, l1, l4) = fixture();
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n\
             {\"t\":\"part\",\"layer\":\"L4\",\"name\":\"veil\",\"path\":\"l4.kir\"}\n\
             {\"t\":\"capacity\",\"layer\":\"L1\",\"value\":8192}\n",
    );

    let resolved = resolve(&store, &kset).expect("every part is beside the file");

    let l1_hash = Hash::of(&std::fs::read(&l1).expect("read"));
    let l4_hash = Hash::of(&std::fs::read(&l4).expect("read"));
    let records: Vec<&Record> = resolved.iter().map(Line::record).collect();
    assert!(
        matches!(records[1], Record::Slot { at: NodeAddress { layer: Layer::L1, index: 0 }, name: None, proc_hash } if *proc_hash == l1_hash),
        "the L1 part became a slot naming its source's address: {:?}",
        records[1]
    );
    assert!(
        matches!(records[2], Record::Slot { at: NodeAddress { layer: Layer::L4, .. }, name: Some(name), proc_hash, .. } if name == "veil" && *proc_hash == l4_hash),
        "the L4 part kept the name this Set gave it: {:?}",
        records[2]
    );
    // **And everything else is passed through unchanged**, which is half of
    // what makes the two forms one format.
    assert!(
        matches!(records[0], Record::Set { id, v: 1 } if id == "authored"),
        "{:?}",
        records[0]
    );
    assert!(
        matches!(
            records[3],
            Record::Capacity {
                at: NodeAddress {
                    layer: Layer::L1,
                    index: 0
                },
                value: 8192
            }
        ),
        "{:?}",
        records[3]
    );
    assert_eq!(records.len(), 4, "no record was added or dropped");

    for hash in [l1_hash, l4_hash] {
        assert!(
            store.get_artifact(&hash).is_ok(),
            "{}: resolution puts the bytes in the store, not only their address",
            hash.short(12)
        );
    }
}

/// A `.kbset` made from a `.kset` loads with the authoring file deleted, and
/// with the parts it named deleted too — which is the whole point of the form.
///
/// An authoring file is only readable beside its neighbours; the resolved one
/// is readable anywhere its material is, and a bundle carries the material with
/// it. So this deletes the entire directory the `.kset` and its `.kir` files
/// lived in, takes it into a store that has never held any of it, and loads.
/// Nothing that resolves a path could survive that, which is what makes it the
/// test of the difference rather than of the pipeline.
#[test]
fn a_kbset_made_from_a_kset_loads_with_the_authoring_file_deleted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let beside_it = dir.path().join("parts");
    std::fs::create_dir(&beside_it).expect("mkdir");
    std::fs::write(beside_it.join("l1.kir"), L1).expect("write l1");
    std::fs::write(beside_it.join("l4.kir"), L4).expect("write l4");
    std::fs::write(
        beside_it.join("night.kset"),
        "{\"t\":\"set\",\"id\":\"night\",\"v\":1}\n\
             {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n\
             {\"t\":\"part\",\"layer\":\"L4\",\"path\":\"l4.kir\"}\n",
    )
    .expect("write kset");

    let author = Store::open(dir.path().join("store")).expect("store");
    let sent = as_a_file(
        &bundle_authored(&author, &beside_it.join("night.kset"))
            .expect("bundle the authoring file"),
    );

    // The authoring file, the parts, and the store that resolved them: all
    // gone. What is left is the text in `sent`.
    std::fs::remove_dir_all(dir.path()).expect("remove the whole directory");

    let elsewhere = tempfile::tempdir().expect("tempdir");
    let bare = Store::open(elsewhere.path()).expect("store");
    unbundle(&bare, CameFrom::Somebody, &sent).expect("the bundle carries its own sources");
    let loaded = load(&bare, "night").expect("the set is filed and its artifacts are here");
    assert_eq!(loaded.l1s[0].name, "ring");
    assert_eq!(loaded.l4s[0].name, "points");
}

/// A `part` in a `.kbset` refuses the load, because it is a file disagreeing
/// with its own extension.
///
/// Not skipped with a note, which is what this reader does with every other
/// line it cannot honour: a `part` is a *node*, and skipping one hands back a
/// Set that is a geometry short. The refusal names the node and the path it
/// wanted.
#[test]
fn a_part_in_a_resolved_set_file_refuses_the_load() {
    let (_dir, store, l1, _l4) = fixture();
    let here = stored(&store, &l1);
    let text = format!(
        "{{\"t\":\"set\",\"id\":\"mixed\",\"v\":1}}\n\
             {{\"t\":\"slot\",\"layer\":\"L1\",\"proc\":\"{here}\"}}\n\
             {{\"t\":\"part\",\"layer\":\"L4\",\"path\":\"l4.kir\"}}\n"
    );
    let refused = from_lines(&store, "mixed", &parsed(&text)).expect_err("a part is refused");
    assert!(refused.contains("l4.kir"), "{refused}");
    assert!(refused.contains("content address"), "{refused}");
}

/// A part naming an absolute path is refused, and the refusal names the path.
///
/// The first of the three spellings of one escape. It is refused without the
/// filesystem being asked anything, which is why the path here need not exist —
/// and why a machine where it *does* exist gets the same answer.
#[test]
fn a_part_naming_an_absolute_path_is_refused() {
    let (dir, store, _l1, _l4) = fixture();
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"/etc/passwd\"}\n",
    );
    let refused = resolve(&store, &kset).expect_err("an absolute path is not relative");
    assert!(refused.contains("/etc/passwd"), "{refused}");
    assert!(refused.contains("absolute path"), "{refused}");
    assert!(
        refused.contains("Refused rather than repaired"),
        "{refused}"
    );
}

/// A part that climbs out of the Set file's own directory is refused, naming
/// what it climbed out of.
///
/// The second spelling. The `.kset` is one level down so that `..` has
/// somewhere to go, and the file it reaches for genuinely exists — a wall that
/// only refuses paths that were not there anyway is not a wall.
#[test]
fn a_part_that_climbs_out_of_the_set_files_directory_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("l1.kir"), L1).expect("write the neighbour above");
    let inside = dir.path().join("inside");
    std::fs::create_dir(&inside).expect("mkdir");
    let kset = inside.join("night.kset");
    std::fs::write(
        &kset,
        "{\"t\":\"set\",\"id\":\"authored\",\"v\":1}\n\
             {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"../l1.kir\"}\n",
    )
    .expect("write");
    let store = Store::open(dir.path().join("store")).expect("store");

    let refused = resolve(&store, &kset).expect_err("`..` climbs out");
    assert!(refused.contains("../l1.kir"), "{refused}");
    assert!(refused.contains("climbs out of"), "{refused}");
    assert!(
        refused.contains(&inside.display().to_string())
            || refused.contains(
                &std::fs::canonicalize(&inside)
                    .expect("canonicalize")
                    .display()
                    .to_string()
            ),
        "the refusal names the directory that was escaped: {refused}"
    );
}

/// A `..` that lands back inside is an ordinary path and is allowed.
///
/// What the wall refuses is *leaving*, not the spelling — a rule that refused
/// every `..` would refuse `parts/../l1.kir`, which names a file in the
/// directory the Set file is in, and an operator would learn that by
/// experiment. This is the test that keeps the check on containment rather than
/// on characters.
#[test]
fn a_dotdot_that_lands_back_inside_is_a_path_and_is_allowed() {
    let (dir, store, l1, _l4) = fixture();
    std::fs::create_dir(dir.path().join("parts")).expect("mkdir");
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"parts/../l1.kir\"}\n",
    );
    let resolved = resolve(&store, &kset).expect("this path never leaves the directory");
    let l1_hash = Hash::of(&std::fs::read(&l1).expect("read"));
    assert!(
        matches!(resolved[1].record(), Record::Slot { proc_hash, .. } if *proc_hash == l1_hash),
        "{:?}",
        resolved[1].record()
    );
}

/// A part that is a symlink out of the directory is refused, which is the
/// spelling that gets missed.
///
/// Lexically this include is one plain component with no `..` and no leading
/// `/`; every character in it is one the other two rules allow. It is only an
/// escape once the link is followed, which is why the comparison is between
/// canonical paths — and why the fixture's own directory is canonicalised too,
/// since on macOS a temporary directory is itself reached through a symlink and
/// a naive comparison would refuse everything.
#[test]
#[cfg(unix)]
fn a_part_that_is_a_symlink_out_of_the_directory_is_refused() {
    let elsewhere = tempfile::tempdir().expect("tempdir");
    let secret = elsewhere.path().join("secret.kir");
    std::fs::write(&secret, L1).expect("write the file outside");

    let (dir, store, _l1, _l4) = fixture();
    std::os::unix::fs::symlink(&secret, dir.path().join("innocent.kir")).expect("symlink");
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"innocent.kir\"}\n",
    );

    let refused = resolve(&store, &kset).expect_err("the link points out of the directory");
    assert!(refused.contains("innocent.kir"), "{refused}");
    assert!(refused.contains("symlink out"), "{refused}");
    assert!(
        refused.contains(
            &std::fs::canonicalize(&secret)
                .expect("canonicalize")
                .display()
                .to_string()
        ),
        "the refusal names where the link actually went: {refused}"
    );
    assert!(
        store
            .get_artifact(&Hash::of(&std::fs::read(&secret).expect("read")))
            .is_err(),
        "a refused part is not in the store: the wall runs before anything is read"
    );
}

/// A directory reached through a symlink still contains its own parts.
///
/// The other half of the sentence above, and the failure the first
/// implementation of a containment check makes: canonicalise the target and not
/// the root, and every part of every Set authored under `/var/folders` on macOS
/// — or under any linked path anywhere — is refused as an escape. A wall that
/// refuses everything is a wall somebody switches off.
#[test]
#[cfg(unix)]
fn a_directory_reached_through_a_symlink_still_contains_its_own_parts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let real = dir.path().join("real");
    std::fs::create_dir(&real).expect("mkdir");
    std::fs::write(real.join("l1.kir"), L1).expect("write");
    std::fs::write(
        real.join("night.kset"),
        "{\"t\":\"set\",\"id\":\"authored\",\"v\":1}\n\
             {\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n",
    )
    .expect("write");
    let linked = dir.path().join("linked");
    std::os::unix::fs::symlink(&real, &linked).expect("symlink");
    let store = Store::open(dir.path().join("store")).expect("store");

    resolve(&store, &linked.join("night.kset"))
        .expect("the file's own directory contains the file's own parts, link or no link");
}

/// A file that is not a `.kset` is not resolved, because the extension is the
/// whole of what says which of a Set's two forms a file is.
#[test]
fn a_file_that_is_not_a_kset_is_not_resolved() {
    let (dir, store, _l1, _l4) = fixture();
    let kbset = authored(
        &dir,
        "night.kbset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l1.kir\"}\n",
    );
    let refused = resolve(&store, &kbset).expect_err("a `.kbset` is read, not resolved");
    assert!(refused.contains(".kset"), "{refused}");
}

/// A part naming a file that is not there says so, rather than saying it
/// escaped.
///
/// The two are different mistakes and an operator fixes them differently: one
/// is a typo or a part left behind, the other is a file that was trying to
/// leave. A wall that answered "refused" to both would send whoever mistyped
/// `l1.kir` looking for a security problem.
#[test]
fn a_part_naming_a_file_that_is_not_there_says_so() {
    let (dir, store, _l1, _l4) = fixture();
    let kset = authored(
        &dir,
        "night.kset",
        "{\"t\":\"part\",\"layer\":\"L1\",\"path\":\"l9.kir\"}\n",
    );
    let refused = resolve(&store, &kset).expect_err("there is no `l9.kir`");
    assert!(refused.contains("l9.kir"), "{refused}");
    assert!(
        refused.contains("no such file beside the Set file"),
        "{refused}"
    );
}
