use super::*;

/// How [`BINDINGS`] spells the arms whose pattern is a name rather than a
/// character. Nothing in `Key::Named(NamedKey::Escape)` says `esc`, so this one
/// mapping cannot be derived and is stated — but it is stated as a *table*, and
/// a `NamedKey` arm missing from it fails
/// [`every_key_the_live_path_acts_on_is_documented`] by name rather than being
/// passed over. That is the whole difference from the list of characters that
/// used to be here: a named key added tomorrow is a test failure that says
/// which key, not a silence.
const NAMED_KEY_SPELLINGS: &[(&str, &str)] = &[("Escape", "esc"), ("Space", "space")];

/// The end of the character literal starting at `at`, or `None` if what is
/// there is not one.
///
/// A lifetime has to be told from a literal — `'a` is one, `'a'` and `'\n'` are
/// the other — because getting it wrong lets a `'"'` open a string that
/// swallows the rest of the file. `'\''` is why an escape cannot simply look
/// for the next quote: the escaped quote *is* the next quote.
fn char_literal_end(src: &str, at: usize) -> Option<usize> {
    let b = src.as_bytes();
    if b.get(at) != Some(&b'\'') {
        return None;
    }
    if b.get(at + 1) == Some(&b'\\') {
        // A two-byte escape — `\\`, `\'`, `\n`, `\0` — closes at `at + 3`.
        if b.get(at + 3) == Some(&b'\'') {
            return Some(at + 4);
        }
        // `\x41`, `\u{2026}`: the first quote after the escape. Bounds
        // first, so a file ending mid-literal is `None` and not a panic.
        if at + 3 > src.len() {
            return None;
        }
        return src[at + 3..].find('\'').map(|n| at + 3 + n + 1);
    }
    let c = src[at + 1..].chars().next()?;
    let close = at + 1 + c.len_utf8();
    (b.get(close) == Some(&b'\'')).then_some(close + 1)
}

/// The character a literal's inside spells, the way rustc reads it.
///
/// Panics rather than returning nothing on a spelling it does not know: a key
/// quietly dropped here is a key quietly undocumented, which is the exact
/// failure this file is closing.
fn unescape(lit: &str) -> char {
    let mut cs = lit.chars();
    let first = cs.next().expect("a character literal is not empty");
    if first != '\\' {
        return first;
    }
    match cs.next().expect("an escape has a second character") {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '0' => '\0',
        c @ ('\\' | '\'' | '"') => c,
        _ => panic!("`'{lit}'` is a key spelling this scanner cannot read"),
    }
}

/// Comments and string literals blanked to spaces, keeping length and line
/// structure. Character literals are left exactly as written — they are the
/// payload.
///
/// Load-bearing in both directions. Comments are prose and prose is full of
/// apostrophes; string literals hold `"{BINDINGS}"` and every message the arms
/// print. Cut down from the same function in
/// `karakuri-engine/tests/gpu_tests_are_under_mod_gpu.rs`; this file has no
/// block comments and no raw strings, and if one arrives the floors below are
/// what refuses the mis-scan.
fn blank_comments_and_strings(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(b"//") {
            let end = src[i..].find('\n').map_or(b.len(), |n| i + n);
            for c in out[i..end].iter_mut() {
                *c = b' ';
            }
            i = end;
            continue;
        }
        if b[i] == b'"' {
            let start = i;
            i += 1;
            while i < b.len() {
                match b[i] {
                    b'\\' => i += 2,
                    b'"' => {
                        i += 1;
                        break;
                    }
                    _ => i += 1,
                }
            }
            let end = i.min(b.len());
            for c in out[start..end].iter_mut() {
                if *c != b'\n' {
                    *c = b' ';
                }
            }
            continue;
        }
        if let Some(end) = char_literal_end(src, i) {
            i = end;
            continue;
        }
        i += 1;
    }
    String::from_utf8(out).expect("blanking only ever writes spaces")
}

/// Extracts the blanked function body of [`Live::key`] identified by signature.
fn live_key_body() -> String {
    let blanked = blank_comments_and_strings(SOURCE);
    let needle = concat!("fn ", "key(&mut self, key: &Key) -> bool {");
    assert_eq!(
        blanked.matches(needle).count(),
        1,
        "`{needle}` is not in this file exactly once — the scanner is \
         reading nothing, or reading the wrong function"
    );
    let open = blanked.find(needle).expect("just counted one") + needle.len();
    let b = blanked.as_bytes();
    let (mut i, mut depth) = (open, 1usize);
    let end = loop {
        if i >= b.len() {
            panic!("`{needle}` never closes");
        }
        // A braced character literal is a key like any other: stepped over
        // whole, so binding `{` could not open a block here.
        if let Some(skip) = char_literal_end(&blanked, i) {
            i = skip;
            continue;
        }
        match b[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    break i;
                }
            }
            _ => {}
        }
        i += 1;
    };
    blanked[open..end].to_string()
}

/// One arm of [`Live::key`]'s match, as its pattern reads.
enum Arm {
    /// A character it acts on: `'x' => …`.
    Char(char),
    /// A span of them: `'0'..='3' => …`.
    Range(char, char),
    /// A `NamedKey`, whose pattern is a name and not a character at all.
    Named(String),
}

/// Every arm of [`Live::key`], read off the pattern side of each `=>` in its
/// body.
///
/// The pattern side and not the line, because everything after the first `=>`
/// is the arm's *body* — where `unwrap_or('\0')` lives, and `\0` is not a
/// binding. Line by line, because a pattern and its `=>` share a line; an arm
/// body that grew an `=>` of its own would be read as a pattern, which can only
/// ever demand documentation for a key nobody binds, and that fails loudly
/// rather than passing quietly.
fn live_key_arms(body: &str) -> Vec<Arm> {
    let mut arms = Vec::new();
    for line in body.lines() {
        let Some(cut) = line.find("=>") else {
            continue;
        };
        let pattern = &line[..cut];
        for (at, marker) in pattern.match_indices("NamedKey::") {
            let name: String = pattern[at + marker.len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                arms.push(Arm::Named(name));
            }
        }
        let mut chars = Vec::new();
        let mut i = 0;
        while i < pattern.len() {
            match char_literal_end(pattern, i) {
                Some(end) => {
                    chars.push(unescape(&pattern[i + 1..end - 1]));
                    i = end;
                }
                None => i += 1,
            }
        }
        // `'0'..='3'` is one binding spanning four keys, not two bindings.
        if pattern.contains("..=") {
            assert_eq!(
                chars.len(),
                2,
                "`{pattern}` is a range with {} endpoints",
                chars.len()
            );
            arms.push(Arm::Range(chars[0], chars[1]));
        } else {
            arms.extend(chars.into_iter().map(Arm::Char));
        }
    }
    arms
}

/// The key column of [`BINDINGS`]: two spaces, the keys, then the gap before
/// the description. Keys are documented in pairs where they come in pairs — `[
/// ]`, `u i`, `h ?` — so it is the column that is read and not the first
/// character of a line.
///
/// A column and never a substring of the whole constant, which is the trap this
/// text is laid out to defuse: `?` occurs in the prose of the `, .` line, so
/// `BINDINGS.contains("?")` is true whether or not `?` is bound to anything.
/// See [`a_key_named_only_in_prose_is_not_documented`].
fn documented_keys(bindings: &str) -> Vec<&str> {
    bindings
        .lines()
        .filter_map(|line| line.strip_prefix("  "))
        .filter(|line| !line.starts_with(' '))
        .flat_map(|line| line.split("  ").next().unwrap_or_default().split(' '))
        .filter(|key| !key.is_empty())
        .collect()
}

/// Every key `Live::key` acts on is in [`BINDINGS`], which is the only thing
/// standing between a control and being undiscoverable: there is no on-screen
/// UI, and this text is both what `--help` prints and what `h` does.
///
/// The keys are *read out of the match arms* — see [`live_key_body`] — and not
/// restated here. The version of this test that restated them iterated a
/// hard-coded array of 32 characters, so what it enforced was "these 32 keys
/// are documented"; `'h' | '?'` had been a live arm with no entry in the column
/// the whole time and this test passed on every run. A check weaker than its
/// own name is worse than no check, because the name is what stops anyone
/// looking again — nobody re-reads a green
/// `every_key_the_live_path_acts_on_is_documented`.
///
/// Which is also why the counts are asserted. A scanner whose pattern stops
/// matching finds nothing and then passes everything, and that is the same
/// failure a second time.
#[test]
fn every_key_the_live_path_acts_on_is_documented() {
    let documented = documented_keys(BINDINGS);
    let (mut chars, mut ranges, mut named) = (0, 0, 0);

    for arm in live_key_arms(&live_key_body()) {
        match arm {
            Arm::Char(c) => {
                chars += 1;
                let key = c.to_string();
                assert!(
                    documented.contains(&key.as_str()),
                    "`{c}` is a key `Live::key` acts on and has no entry in the \
                     bindings — an operator has no way to find it"
                );
            }
            // Documented as the span it is, `0-3`, and the span is built
            // from the arm's own endpoints rather than being spelled here.
            Arm::Range(from, to) => {
                ranges += 1;
                let key = format!("{from}-{to}");
                assert!(
                    documented.contains(&key.as_str()),
                    "`{key}` is a range `Live::key` acts on and has no entry in \
                     the bindings"
                );
            }
            // A named arm carries no character to look for, so its spelling
            // comes from the one table there is — and an unknown name stops
            // the run rather than being skipped.
            Arm::Named(name) => {
                named += 1;
                let (_, spelling) = NAMED_KEY_SPELLINGS
                    .iter()
                    .find(|(known, _)| *known == name)
                    .unwrap_or_else(|| {
                        panic!(
                            "`NamedKey::{name}` is a key `Live::key` acts on and \
                             NAMED_KEY_SPELLINGS does not say how the bindings \
                             spell it"
                        )
                    });
                assert!(
                    documented.contains(spelling),
                    "`{spelling}` (`NamedKey::{name}`) has no entry in the bindings"
                );
            }
        }
    }

    // Floors, not counts. They are what `Live::key` actually holds today —
    // 32 characters, one range, two named keys — rather than a round number
    // under them, because a control surface is small enough that losing one
    // key is news and the scan going quiet is the thing being guarded
    // against. Three of them because they fail apart: a signature change
    // gives no arms at all, a broken literal reader gives named arms and no
    // characters, and a `NamedKey` renamed away gives characters and no
    // named ones. Raise them when a key is added; lowering one is a claim
    // that a control was deliberately removed.
    //
    // **It was 33 and is 32**, and that is the claim being made: ADR-0240
    // retired *Choose what the output shows*, so `v` is not a key any
    // more. The floor came down with the control rather than the control
    // being kept alive to hold a number up.
    assert!(
        chars >= 32,
        "only {chars} character keys read out of `Live::key` — the scan is not \
         seeing the match arms"
    );
    assert!(
        ranges >= 1,
        "no `'a'..='b'` arm read out of `Live::key` — slot focus is one"
    );
    assert!(
        named >= 2,
        "only {named} `NamedKey` arms read out of `Live::key` — escape and space \
         are two"
    );
}

/// A character that occurs only in a binding's prose is not documented.
///
/// The reason [`documented_keys`] parses a column instead of asking
/// `BINDINGS.contains(key)`, and it is not hypothetical: `?` appears inside the
/// `, .` entry's description, so the substring form of this check would have
/// called `?` documented while it was bound to nothing. That version would have
/// looked stronger than the hard-coded array it replaced and enforced less.
#[test]
fn a_key_named_only_in_prose_is_not_documented() {
    // `?` twice in prose and never in the column: once in the description
    // beside a key, once on the continuation line under it. Both are places
    // the real text puts it, and each one is a different way the parse
    // could go wrong.
    let prose = concat!(
        "keys:\n",
        "  s          print the status line — the tracker writes ? there when\n",
        "             it is unsure, and ? again if it stays unsure\n",
    );
    // What a substring check sees, and it is a lie.
    assert!(prose.contains('?'));
    assert!(
        !documented_keys(prose).contains(&"?"),
        "a character in a description was counted as a documented key"
    );
    // The column, where a real binding lives, still reads.
    assert!(documented_keys(prose).contains(&"s"));
    // And in the real text `?` is now in the column, not only the prose.
    assert!(documented_keys(BINDINGS).contains(&"?"));
}

/// Every method of [`Live`] that writes a record without going through
/// [`Live::operate`], with the reason its operation cannot convert.
///
/// This is the key handler's form of the single arm [`Live::run_surface`] keeps
/// for `TapBeat`: a table rather than a comment, so a key that grows a second
/// derivation of a record is a failing test naming the function instead of a
/// line nobody reads. Every operation named here answers `Owed::NotSettled`,
/// which is asserted separately — see
/// [`the_keys_that_keep_their_own_path_are_the_ones_whose_record_is_not_settled`]
/// — so an entry that stops being owed fails there rather than lingering here
/// as a stale excuse.
const OWED_RECORD_PATHS: &[(&str, &str)] = &[(
    "performed",
    "the route itself: this is where `written`'s records are written, and \
     `Live::operate` is the wrapper over it for a caller with nobody waiting on an \
     answer",
)];

/// The method a byte offset falls inside, read off the nearest `fn` above it at
/// `impl` indentation.
///
/// Four spaces and not any `fn `, because a closure or a nested helper would
/// otherwise answer for the method it sits in. The needle is spelled with a
/// leading newline so a `fn` inside an expression cannot match.
fn enclosing_method(blanked: &str, at: usize) -> &str {
    let (start, prefix_len) = [
        "\n    fn ",
        "\n    pub(super) fn ",
        "\n    pub(crate) fn ",
        "\n    pub fn ",
    ]
    .into_iter()
    .filter_map(|prefix| blanked[..at].rfind(prefix).map(|pos| (pos, prefix.len())))
    .max_by_key(|(pos, _)| *pos)
    .expect("every record written in this file is inside a method");
    let start = start + prefix_len;
    let name_end = blanked[start..]
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .map_or(blanked.len(), |n| start + n);
    &blanked[start..name_end]
}

/// Asserts that any method in `Live` calling `self.record(` directly is listed in
/// `OWED_RECORD_PATHS` with a documented reason why its operation cannot convert.
#[test]
fn every_record_written_outside_operate_is_a_path_whose_conversion_is_owed() {
    let blanked = blank_comments_and_strings(SOURCE);
    let needle = concat!("self.", "record(");
    let mut reached: Vec<&str> = Vec::new();
    for (at, _) in blanked.match_indices(needle) {
        let name = enclosing_method(&blanked, at);
        assert!(
            OWED_RECORD_PATHS.iter().any(|(known, _)| *known == name),
            "`Live::{name}` writes a record without going through `Live::operate`, \
             and OWED_RECORD_PATHS does not say why its operation cannot convert — \
             a surface that derives a record beside the conversion is the drift \
             `karakuri-operation-record` exists to end"
        );
        if !reached.contains(&name) {
            reached.push(name);
        }
    }
    // Verify that at least one method is matched so that dead scans fail.
    assert!(
        !reached.is_empty(),
        "no method reads as a record writer — the scan is not seeing `Live`'s \
         bodies, and `Live::operate` itself is one: {reached:?}"
    );
    // And nothing in the table is a leftover. A path whose last direct
    // write moved to `operate` is a row to delete, not a permission to
    // keep.
    for (name, why) in OWED_RECORD_PATHS {
        assert!(
            reached.contains(name),
            "OWED_RECORD_PATHS excuses `Live::{name}` — {why} — and it writes no \
             record of its own any more, so the row outlived what it was for"
        );
    }
}

/// Asserts that direct key paths only remain for operations whose conversion
/// is not yet settled (`Written::Owed(Owed::NotSettled)`).
#[test]
fn the_keys_that_keep_their_own_path_are_the_ones_whose_record_is_not_settled() {
    use karakuri_operation_record::Owed;
    let owed = [
        ("b", Operation::TapBeat),
        (
            ", .",
            Operation::ScaleGrid {
                by: karakuri_operation::GridScale::Double,
            },
        ),
    ];
    for (keys, operation) in owed {
        assert_eq!(
            karakuri_operation_record::written(&operation, &Current::default()),
            Written::Owed(Owed::NotSettled),
            "`{}` no longer owes its record, so `{keys}` reaches the deck by a \
             derivation of its own where `Live::operate` would now do — see \
             `Live::key`'s survey",
            operation.title()
        );
    }
}

/// Asserts that in-flight background saves are awaited up to `SAVE_WAIT` when
/// a run ends (ADR-0120).
#[test]
fn a_save_in_flight_at_the_end_of_a_run_is_waited_for() {
    let (tx, rx) = std::sync::mpsc::channel();
    let late = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(120));
        tx.send(Saved {
            slot: 0,
            asked: Asked::Operator,
            id: "late".to_string(),
            outcome: Ok(()),
            // A hand pressed the key; nobody is waiting on a socket.
            reply: None,
        })
    });
    let landed = drained_saves(&rx, 1, Instant::now() + SAVE_WAIT);
    late.join().expect("the save thread").expect("it sent");
    assert_eq!(
        landed.len(),
        1,
        "the run quit before the save it was told to wait for reported back"
    );

    // **The bound, held against a save that never arrives.** The sender is
    // still alive, so there is nothing but the deadline to end this.
    let (_alive, rx) = std::sync::mpsc::channel::<Saved>();
    let started = Instant::now();
    let landed = drained_saves(&rx, 1, started + Duration::from_millis(80));
    assert!(landed.is_empty());
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "a save that never reports back held the quit past its deadline"
    );
}

/// Asserts that save request handling and save outcome draining in `Live::frame`
/// precede any early returns or GPU composition calls (ADR-0078).
#[test]
fn a_frame_attends_to_its_saves_before_anything_can_stop_them() {
    let source = include_str!("../../live/mod.rs");
    let body = source
        .split_once("\n    pub(crate) fn frame(&mut self) {")
        .or_else(|| source.split_once("\n    fn frame(&mut self) {"))
        .expect("`Live::frame` is no longer spelled that way")
        .1;
    let body = body
        .split("\n    fn ")
        .next()
        .expect("the end of `Live::frame`");
    let code: Vec<&str> = body
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect();
    let code = code.join("\n");

    // The end of the function if it never returns early, which is what it
    // does today — every position below is genuinely above it either way.
    let first_return = code.find("return").unwrap_or(code.len());
    let composed = code
        .find("frame::compose(")
        .expect("`Live::frame` no longer composes a frame, and this test is about where");
    for call in ["self.run_requests();", "self.finished_saves();"] {
        let at = code
            .find(call)
            .unwrap_or_else(|| panic!("`Live::frame` no longer calls `{call}`"));
        assert!(
            at < composed,
            "`{call}` is below `frame::compose` in `Live::frame`: a save's outcome has \
             nothing to do with whether there is a surface to draw on"
        );
        assert!(
            at < first_return,
            "`{call}` is below an early return in `Live::frame`: a frame that publishes \
             nowhere takes saves and never answers for them"
        );
    }
}
