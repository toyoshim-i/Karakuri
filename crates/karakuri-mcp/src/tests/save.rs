use super::*;

/// A save the render loop does not answer ends, and says something true.
///
/// Three ways a model can be left waiting, and none of them may end in a call
/// that never returns or in a claim nobody can support. Over the channel rather
/// than over a socket for the reason `drained_saves` is tested that way: the
/// bound is the whole point and a render loop is not needed to see it.
#[test]
fn a_save_the_loop_does_not_answer_ends_and_says_something_true() {
    // Never taken. Nothing was saved and saying so is safe.
    let (kept, news) = mpsc::channel::<News>();
    let started = std::time::Instant::now();
    let said = awaited(&news, std::time::Duration::from_millis(60))
        .expect_err("a save nobody took came back as a success");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "the wait ran past its bound"
    );
    assert!(said.contains("Nothing was saved"), "{said}");
    drop(kept);

    // Taken and named, then silence. **The id is in the answer** — that is
    // what the first message is for — and the answer claims neither success
    // nor failure.
    let (tx, news) = mpsc::channel();
    tx.send(News::Accepted(
        "slot 0: saving 2 nodes as set `keeper` in <store>".to_string(),
    ))
    .expect("accepted");
    let said = awaited(&news, std::time::Duration::from_millis(60))
        .expect_err("a save with no outcome came back as a success");
    assert!(
        said.contains("`keeper`"),
        "a timed-out save did not name the id it was accepted under: {said}"
    );
    assert!(
        said.contains("neither a success nor a failure"),
        "a timed-out save was reported as one or the other: {said}"
    );

    // The loop ended without answering: told at once rather than at the
    // deadline, which the long wait here is what proves.
    let (tx, news) = mpsc::channel();
    tx.send(News::Accepted(
        "slot 0: saving 2 nodes as set `keeper` in <store>".to_string(),
    ))
    .expect("accepted");
    drop(tx);
    let started = std::time::Instant::now();
    let said = awaited(&news, std::time::Duration::from_secs(60))
        .expect_err("a loop that ended came back as a success");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "a client waited out the deadline on a loop that was already gone"
    );
    assert!(said.contains("`keeper`"), "{said}");
}

/// Verifies that when a client times out after the render loop accepts a save request,
/// the timeout response includes the pending Set ID so the client does not assume failure.
#[test]
fn a_save_the_loop_has_taken_names_its_id_to_a_client_that_times_out() {
    let (tx, news) = mpsc::channel();
    let reply = Reply(tx);
    // One node, because the sentence counts them and a fixture that agreed
    // with a hardcoded plural would be checking the fixture.
    let sources =
        karakuri_environment::setfile::Sources(vec![karakuri_environment::setfile::SavedNode {
            layer: "L1",
            index: 0,
            hash: karakuri_store::hash::Hash::of(b"kind L1"),
            name: None,
            source: None,
            meta: None,
        }]);
    let id = karakuri_environment::accepted_save(
        1,
        // **A `Reply` exists only because a model asked**, so this is the
        // arm this test has always been about — see
        // `docs/principles/0096-the-operators-library-is-written-by-an-operators-own-act.md`.
        karakuri_environment::Asked::Model,
        Some("keeper".to_string()),
        &sources,
        std::path::Path::new("/nowhere/store"),
        Some(&reply),
    );
    assert!(
        id.ends_with("_keeper") && id != "keeper",
        "a client's own name rides behind the stamp so a snapshot cannot be \
         written over: {id}"
    );

    let said = awaited(&news, std::time::Duration::from_millis(60))
        .expect_err("a save with no outcome yet came back as a success");
    assert!(
        said.starts_with(&format!(
            "slot 1: saving 1 node as set `{id}` in /nowhere/store/sandbox"
        )),
        "the loop's acceptance did not reach the client, so a timeout has no id \
         to offer, and no directory to look in: {said}"
    );
    assert!(
        said.contains("neither a success nor a failure"),
        "a save with no outcome was reported as one or the other: {said}"
    );
    assert!(
        !said.contains("Nothing was saved"),
        "a save that had been taken and is being written was reported to a model \
         as one that never happened: {said}"
    );
    drop(reply);
}

/// An id from a client is one path component, which is what a Set id is
/// everywhere else in this program.
#[test]
fn a_set_id_from_a_client_is_one_path_component() {
    assert_eq!(checked_id("keeper-01"), Ok("keeper-01".to_string()));
    assert_eq!(checked_id("a_B_9"), Ok("a_B_9".to_string()));
    // What a save with no id is called, so a client can name one the same
    // way the run would have.
    let stamp = karakuri_environment::history::stamped_id();
    assert_eq!(checked_id(&stamp), Ok(stamp.clone()), "{stamp}");

    for bad in [
        "../../../etc/passwd",
        "sets/../../elsewhere",
        "a/b",
        "",
        "a b",
        "night.01",
        "~/mine",
    ] {
        assert!(
            checked_id(bad).is_err(),
            "`{bad}` was accepted as the name of a file in the store"
        );
    }
    assert!(checked_id(&"x".repeat(MAX_ID + 1)).is_err());

    // **The over-length refusal counts what it measures.** `str::len` is
    // bytes and the message said "characters", which agree for everything
    // that would get past the charset check and disagree for exactly the
    // caller this message exists for. Thirty-three two-byte characters is
    // sixty-six bytes, so the two readings cannot both be right here.
    let multibyte = "é".repeat(33);
    let refusal = checked_id(&multibyte).expect_err("66 bytes is past the cap");
    assert!(
        refusal.contains("is 66 bytes"),
        "an id was refused for a length its caller cannot count to: {refusal}"
    );
}
