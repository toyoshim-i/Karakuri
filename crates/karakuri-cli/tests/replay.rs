//! What a replay actually reproduces, driven through the binary.
//!
//! Everything else in this workspace tests a decoder, a deck or a shader. The
//! claim a replay makes is larger than any of them and lives in none of them:
//! *what drove the engine live and what drives it on replay are the same
//! function, so they cannot come apart.* The only place that is observable is
//! the far end — a session in, a PNG out — so this is the one test that runs
//! the program.
//!
//! **Every test here is a pair with a control.** Two sessions that differ in
//! exactly one record, replayed, and the question is whether the pixels differ.
//! On its own that proves nothing: this material moves on `t`, so "the frames
//! are different" is the answer whatever the code does. The control is the same
//! session replayed twice — which must come back byte for byte, because an
//! offscreen run is documented to be a function of its inputs. Without it, a
//! renderer with any run-to-run noise at all would pass every assertion below.

// Every test here drives `karakuri-cli` as a subprocess, and that binary takes
// a device of its own — so these are GPU tests that no in-process rule can see.
// The whole file is one `mod gpu`; `tests/gpu_tests_are_under_mod_gpu.rs` in
// karakuri-engine is what keeps that honest, by counting a spawn of a binary
// that reaches `Gpu::headless` as a reach of its own.
mod gpu {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// The workspace root. Integration tests run with the *package* as the working
    /// directory, and the default `.kir` pair is named relative to the workspace —
    /// so without this every run here fails on a missing example rather than on
    /// anything it means to test.
    fn workspace() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root")
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("karakuri-replay-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        dir
    }

    fn run(args: &[&str]) -> String {
        let out = Command::new(env!("CARGO_BIN_EXE_karakuri-cli"))
            .current_dir(workspace())
            .args(args)
            .output()
            .expect("run karakuri-cli");
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        assert!(out.status.success(), "{args:?} failed:\n{stderr}");
        stderr
    }

    /// A store with one saved Set, and that Set's records as a session head.
    fn store_with_a_set(dir: &Path) -> (String, String) {
        let store = dir.join("store");
        let store_arg = store.to_str().expect("utf-8 path").to_string();
        run(&["--store", &store_arg, "--save-set", "base"]);
        let head = std::fs::read_to_string(store.join("sets/base.kbset")).expect("the saved Set");
        (store_arg, head)
    }

    fn write_session(store: &str, id: &str, head: &str, body: &[&str]) {
        let path = Path::new(store)
            .join("sessions")
            .join(format!("{id}.ndjson"));
        std::fs::create_dir_all(path.parent().expect("sessions dir")).expect("sessions dir");
        let mut text = head.to_string();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        for line in body {
            text.push_str(line);
            text.push('\n');
        }
        std::fs::write(path, text).expect("write session");
    }

    /// Replay `id` to a single PNG and return its bytes.
    fn replay(store: &str, id: &str, out: &Path) -> Vec<u8> {
        let out_arg = out.to_str().expect("utf-8 path");
        run(&["--store", store, "--replay", id, "--render", out_arg]);
        std::fs::read(out).expect("the rendered PNG")
    }

    const CANVAS: &str = r#"{"t":"canvas","width":256,"height":256}"#;
    const TICK: &str = r#"{"t":"tick","steps":1}"#;

    /// **The control.** An offscreen run is a function of its inputs, so the same
    /// session replayed twice is the same bytes. Every other test in this file
    /// reads a *difference* as evidence, and a difference means nothing unless
    /// sameness is possible.
    #[test]
    fn the_same_session_replays_to_the_same_bytes() {
        let dir = scratch("control");
        let (store, head) = store_with_a_set(&dir);
        write_session(&store, "plain", &head, &[CANVAS, TICK, TICK, TICK]);

        let first = replay(&store, "plain", &dir.join("a.png"));
        let second = replay(&store, "plain", &dir.join("b.png"));
        assert_eq!(first, second, "a replay is not reproducible");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A `look` record part-way through a session changes what the rest of it
    /// renders as.
    ///
    /// It did not. The offscreen renderer took a `Look` argument and applied it
    /// once before the loop, while the replay driver wrote every decoded `look`
    /// into a variable nothing read again — so `t`, `-`, `=` and `` ` `` moved the
    /// output live and moved nothing on replay, and a session replayed under the
    /// look it *started* with however many times the operator changed it. The
    /// exposure keys are the ones that matter here: this project deliberately has
    /// no automatic gain, so the exposure is a control an operator is expected to
    /// ride during a set.
    #[test]
    fn a_look_record_reaches_the_frames_after_it() {
        let dir = scratch("look");
        let (store, head) = store_with_a_set(&dir);

        // Identical but for one record, and it sits after the first frame — so the
        // two runs share their material, their tick count and their step counts,
        // and the only thing that can separate their last frames is the look.
        write_session(&store, "plain", &head, &[CANVAS, TICK, TICK, TICK]);
        write_session(
            &store,
            "dimmed",
            &head,
            &[
                CANVAS,
                TICK,
                r#"{"t":"look","op":"clamp","exposure":0.02,"white_point":1.0}"#,
                TICK,
                TICK,
            ],
        );

        let plain = replay(&store, "plain", &dir.join("plain.png"));
        let dimmed = replay(&store, "dimmed", &dir.join("dimmed.png"));
        assert_ne!(
            plain, dimmed,
            "the `look` record changed nothing — it is being read once before the run \
         rather than per frame"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The canvas comes out of the stream, and the flag cannot overrule it.
    ///
    /// Two sessions that differ only in their `canvas` render at different sizes,
    /// and `--canvas` on top of a stream that carries one is refused rather than
    /// obeyed — because a replay at a size the performance never ran at is not a
    /// replay of it.
    #[test]
    fn the_stream_decides_what_a_replay_renders_at() {
        let dir = scratch("canvas");
        let (store, head) = store_with_a_set(&dir);
        write_session(&store, "small", &head, &[CANVAS, TICK, TICK]);
        write_session(
            &store,
            "odd",
            &head,
            // Not a multiple of 64, which used to be an assertion inside the PNG
            // path — so this session was recordable and unreplayable at once.
            &[r#"{"t":"canvas","width":300,"height":200}"#, TICK, TICK],
        );

        assert_eq!(
            png_size(&replay(&store, "small", &dir.join("s.png"))),
            (256, 256)
        );
        assert_eq!(
            png_size(&replay(&store, "odd", &dir.join("o.png"))),
            (300, 200)
        );

        let refused = Command::new(env!("CARGO_BIN_EXE_karakuri-cli"))
            .current_dir(workspace())
            .args([
                "--store",
                &store,
                "--replay",
                "small",
                "--render",
                dir.join("no.png").to_str().expect("utf-8 path"),
                "--canvas",
                "640x480",
            ])
            .output()
            .expect("run karakuri-cli");
        assert!(
            !refused.status.success(),
            "`--canvas` was obeyed on a replay"
        );
        assert!(
            String::from_utf8_lossy(&refused.stderr).contains("--canvas"),
            "the refusal does not name the flag"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **A procedure rewritten mid-session replays as the rewrite.**
    ///
    /// The thing that was false until there was a record for it: the material was
    /// written once, before the first frame, so a set in which a procedure changed
    /// at minute ten replayed as though it never had. With `--mcp` at the other end
    /// that is the ordinary case rather than a corner, which is why it stopped
    /// being acceptable.
    ///
    /// Paired with a control, as everything here is: the same stream without the
    /// one record, so "the pixels differ" is about the record and not about
    /// material that moves on `t` anyway.
    #[test]
    fn a_procedure_record_changes_what_the_rest_of_the_session_renders() {
        let dir = scratch("procedure");
        let (store, head) = store_with_a_set(&dir);

        // **A fixture of this test's own, not one of `examples/`.** This used to
        // derive a different L4 by substituting a phrase out of the example, and it
        // broke the first time somebody rewrote that example through MCP — which is
        // a thing this program exists to let them do. A fixture the product can
        // rewrite is not a fixture. `flat.kir` is grey dots and consumes only
        // `position`, so it composes with any L1 and cannot be confused with what
        // the head was playing.
        let root = workspace();
        let drained =
            std::fs::read_to_string(root.join("crates/karakuri-cli/tests/fixtures/flat.kir"))
                .expect("the flat fixture");

        // Both procedures into the store, which is where a `procedure` record
        // points. The L1 is unchanged and named anyway: a Set is built from all of them.
        let l1 = std::fs::read_to_string(root.join("examples/drift_shell.kir")).expect("L1");
        // Through the store's own API rather than by writing a file with a name
        // this test guessed: the record spells a hash one way and the filename
        // another, and a test that reproduces the layout by hand is testing its own
        // reproduction.
        let opened = karakuri_store::store::Store::open(&store).expect("store");
        let put = |source: &str| -> String {
            opened
                .put_artifact(source.as_bytes())
                .expect("artifact")
                .to_string()
        };
        let l1_hash = put(&l1);
        let l4_hash = put(&drained);

        let ticks = [TICK; 8];
        let plain: Vec<&str> = std::iter::once(CANVAS)
            .chain(ticks.iter().copied())
            .collect();
        write_session(&store, "plain", &head, &plain);

        let procedure_l1 =
            format!(r#"{{"t":"procedure","slot":0,"layer":"L1","proc":"{l1_hash}"}}"#);
        let procedure_l4 =
            format!(r#"{{"t":"procedure","slot":0,"layer":"L4","proc":"{l4_hash}"}}"#);
        let changed: Vec<&str> = vec![
            CANVAS,
            TICK,
            TICK,
            procedure_l1.as_str(),
            procedure_l4.as_str(),
            TICK,
            TICK,
            TICK,
            TICK,
            TICK,
            TICK,
        ];
        write_session(&store, "changed", &head, &changed);

        let before = replay(&store, "plain", &dir.join("plain.png"));
        let after = replay(&store, "changed", &dir.join("changed.png"));
        assert_ne!(
            before, after,
            "the `procedure` record changed nothing — the material is still whatever the \
         head said"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Width and height out of a PNG header, which is fixed-layout: an 8-byte
    /// signature, then a length and `IHDR`, then the two dimensions big-endian.
    fn png_size(bytes: &[u8]) -> (u32, u32) {
        let at = |i: usize| u32::from_be_bytes(bytes[i..i + 4].try_into().expect("4 bytes"));
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "not a PNG");
        (at(16), at(20))
    }

    /// **A replay meeting a `save` renders its frames and writes no Set file.**
    ///
    /// The one record in the vocabulary whose subject is outside the stream. Every
    /// other record here describes the deck, and obeying it is what replaying
    /// means; obeying this one would mean creating a file in a store nobody asked
    /// this run to touch — under an id that may already exist, holding somebody
    /// else's material. So `--replay` stops being a function from a stream to some
    /// frames.
    ///
    /// Three claims, and the third is the one this file's method is for:
    ///
    /// - **no file appears** under the id the record names;
    /// - **the skip is said out loud**, naming that id, because a replay that
    ///   passed over an outside effect in silence would be the one place in this
    ///   program where something did not happen and nothing mentioned it;
    /// - **the frames are unaffected** — byte for byte the same as the same
    ///   session without the record. Without that last one, "nothing was written"
    ///   would be consistent with a replay that had refused the whole stream.
    #[test]
    fn a_save_record_is_skipped_out_loud_and_writes_nothing() {
        let dir = scratch("save");
        let (store, head) = store_with_a_set(&dir);
        let id = "20260816-143052-271";
        let save = format!(r#"{{"t":"save","slot":0,"id":"{id}"}}"#);

        write_session(&store, "plain", &head, &[CANVAS, TICK, TICK, TICK]);
        write_session(
            &store,
            "saved",
            &head,
            &[CANVAS, TICK, save.as_str(), TICK, TICK],
        );

        let plain = replay(&store, "plain", &dir.join("plain.png"));

        let out = dir.join("saved.png");
        let said = run(&[
            "--store",
            &store,
            "--replay",
            "saved",
            "--render",
            out.to_str().expect("utf-8 path"),
        ]);
        let saved = std::fs::read(&out).expect("the rendered PNG");

        assert!(
            !Path::new(&store)
                .join("sets")
                .join(format!("{id}.kbset"))
                .exists(),
            "a replay wrote a Set file"
        );
        // **The whole sentence, not two substrings of it.** Asserting only that
        // "save" and the id appear is an assertion the in-frame and after-the-last-
        // tick spellings could drift apart under: both would keep passing while
        // saying different things about the same event. The words below are one
        // function's — `skipped_save` — and this is the reader that holds it still.
        assert!(
            said.contains(&format!(
                "a `save` of slot 0 was skipped: a replay writes no Set files. \
             The material it named is set `{id}`"
            )),
            "the skip was silent, or no longer says what a replay does with a \
         `save` and which id it passed over:\n{said}"
        );
        assert_eq!(
            plain, saved,
            "the `save` record changed the picture — it is being applied to the deck \
         rather than passed over"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
