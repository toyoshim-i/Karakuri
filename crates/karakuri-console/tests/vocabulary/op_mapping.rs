use super::vocabulary_common::*;

/// A floor rather than a count, and it is here because the list above is the
/// one thing the compiler cannot check: `rows_of` and `name_of` refuse a new
/// variant, `ops` merely omits it. Raise this when one is added.
#[test]
fn every_variant_is_in_the_list() {
    let p = panel();
    let root = p.layout().root();
    assert!(
        ops(root).len() >= 8,
        "only {} operations listed — `Op` has more than this file knows about",
        ops(root).len()
    );
}

/// A row an operation names and the page does not have is a row that was
/// renamed, moved or deleted under the console — and the console would go on
/// claiming to reach it.
#[test]
fn every_row_an_operation_names_is_on_the_page() {
    let p = panel();
    let root = p.layout().root();
    let page: BTreeSet<String> = rows().into_iter().collect();
    assert!(
        !page.is_empty(),
        "no rows found under `{SECTION}` in {PAGE} — the scan matched nothing, which is not the \
         same as the section being empty"
    );
    for op in ops(root) {
        for row in rows_of(op) {
            assert!(
                page.contains(*row),
                "`Op::{}` names the row `{row}`, and {PAGE} has no such row under `{SECTION}` — \
                 the page is the specification, so this is the console reaching for an operation \
                 nobody named",
                name_of(op)
            );
        }
    }
}

/// The other direction: a row the console cannot perform. Every one of them is
/// in [`NO_OP`] with a reason, so a new row arrives as a failure rather than as
/// an operation the panel silently does not have.
#[test]
fn every_row_on_the_page_has_an_operation() {
    let p = panel();
    let root = p.layout().root();
    let reached: BTreeSet<&str> = ops(root)
        .into_iter()
        .flat_map(|op| rows_of(op).iter().copied())
        .collect();
    for row in rows() {
        assert!(
            reached.contains(row.as_str()) || NO_OP.contains(&row.as_str()),
            "{PAGE} specifies `{row}` under `{SECTION}` and no `Op` reaches it — give it one, or \
             add it to `NO_OP` here with the reason it is not an operation"
        );
    }
}

/// And the variants with no row are exactly [`NO_ROW`], both ways round: one
/// that gains a row should stop being an exception, and a new exception has to
/// be written down rather than discovered.
#[test]
fn the_variants_with_no_row_are_the_ones_written_down() {
    let p = panel();
    let root = p.layout().root();
    let unspecified: Vec<&str> = ops(root)
        .into_iter()
        .filter(|op| rows_of(*op).is_empty())
        .map(name_of)
        .collect();
    assert_eq!(
        unspecified, NO_ROW,
        "the operations {PAGE} does not name are not the ones this file says they are — an \
         operation the page never specified is one no surface but this crate can reach"
    );
}

/// Two regions the vocabulary cannot say, and the pointer can.
///
/// `Operation` names a region by `String`, and `Layout::name` answers `None`
/// for a split the arrangement left unnamed. There are exactly two of those —
/// the root column and the body row — and both are handed to a caller as
/// `Hit::Divider { split, .. }`, which `crates/karakuri/src/main.rs` turns into
/// `Op::Fold` of the split. So this is the cost of `Op` becoming `Operation`,
/// counted: it is two, and they are these.
///
/// The decision was taken rather than left pending (ADR-0204): the two stay
/// unnamed, so `Op` stays `Op`, and this assertion is the standing price rather
/// than a note that somebody still has to choose. Naming either of them is what
/// fails here, and it should: it would be asserting that folding the whole
/// panel away, or folding the row of three panes, is an operation an operator
/// asks for — and the page says the opposite twice, once by having no row for
/// either and once by reaching the outcome an operator does want through *Solo
/// a region*.
///
/// The count is asserted rather than the list alone, because a third unnamed
/// split would be a third region only a mouse could fold, arriving without
/// anybody deciding it should.
#[test]
fn exactly_two_splits_have_no_name_for_an_operation_to_use() {
    let p = panel();
    let l = p.layout();
    let unnamed: Vec<NodeId> = p
        .nodes()
        .iter()
        .map(|n| n.id)
        .filter(|id| l.name(*id).is_none())
        .collect();
    let root = l.root();
    // The body row is the root's second child: the transport, the row of three
    // columns, and the outputs row.
    let body = l.children(root)[1];
    assert_eq!(
        unnamed,
        vec![root, body],
        "the arrangement's unnamed splits are not the two this file knows about — every other \
         node is reachable by name from a MIDI map or an MCP call, and one that is not is a \
         region only the pointer can fold"
    );
    assert!(l.name(root).is_none() && l.name(body).is_none());
}
