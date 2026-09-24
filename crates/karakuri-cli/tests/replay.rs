//! Subprocess integration tests verifying replay reproducibility, state restoration, and frame equivalence.

// Subprocess GPU integration tests for `karakuri-cli`.
// Placed in `mod gpu` to align with workspace GPU test suite conventions.
mod gpu {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// Returns the workspace root path for locating fixtures and examples.
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

    /// Verifies deterministic reproduction of PNG renders across repeated replay runs.
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

    /// Extracts procedure addresses from a saved Set head definition.
    fn addresses(head: &str) -> (String, String) {
        let of = |layer: &str| {
            head.lines()
                .find(|line| {
                    line.contains(&format!(r#""layer":"{layer}""#))
                        && line.contains(r#""t":"slot""#)
                })
                .and_then(|line| line.split(r#""proc":""#).nth(1))
                .and_then(|rest| rest.split('"').next())
                .unwrap_or_else(|| panic!("no {layer} slot in the saved Set"))
                .to_string()
        };
        (of("L1"), of("L4"))
    }

    /// Verifies that session replay properly reconstructs and renders multiple active deck slots.
    #[test]
    fn a_head_naming_two_slots_replays_both_of_them() {
        let dir = scratch("deck");
        let (store, head) = store_with_a_set(&dir);
        let (l1, l4) = addresses(&head);
        let slot_1 = [
            format!(r#"{{"t":"procedure","slot":1,"layer":"L1","proc":"{l1}"}}"#),
            format!(r#"{{"t":"procedure","slot":1,"layer":"L4","proc":"{l4}"}}"#),
        ];

        write_session(&store, "one", &head, &[CANVAS, TICK, TICK]);
        write_session(
            &store,
            "two",
            &head,
            &[&slot_1[0], &slot_1[1], CANVAS, TICK, TICK],
        );
        write_session(
            &store,
            "muted",
            &head,
            &[
                &slot_1[0],
                &slot_1[1],
                r#"{"t":"gain","slot":1,"value":0.0}"#,
                CANVAS,
                TICK,
                TICK,
            ],
        );

        let one = replay(&store, "one", &dir.join("one.png"));
        let two = replay(&store, "two", &dir.join("two.png"));
        let muted = replay(&store, "muted", &dir.join("muted.png"));

        assert_ne!(
            one, two,
            "the head's second slot is not in the picture — a replay is still building \
             a deck of one"
        );
        assert_ne!(
            two, muted,
            "a `gain` naming slot 1 changed nothing — records naming a slot beyond the \
             head's are still being skipped"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Verifies that a master_chain defined in the session head applies to frame 0 (ADR-0340).
    #[test]
    fn a_master_chain_in_the_head_reaches_the_first_frame() {
        let dir = scratch("chain");
        let (store, head) = store_with_a_set(&dir);
        // One of the three shipped effects, at the address a record names it by —
        // `mix::resolve_procedure` finds these without a store at all.
        let bloom =
            karakuri_environment::mix::shipped::address(karakuri_environment::mix::shipped::BLOOM);
        let chain = format!(
            r#"{{"t":"master_chain","slots":[{{"proc":"{bloom}","params":{{"amount":3.0}}}}]}}"#
        );

        write_session(&store, "bare", &head, &[CANVAS, TICK]);
        write_session(&store, "bloomed", &head, &[CANVAS, &chain, TICK]);

        let bare = replay(&store, "bare", &dir.join("bare.png"));
        let bloomed = replay(&store, "bloomed", &dir.join("bloomed.png"));
        assert_ne!(
            bare, bloomed,
            "the head's `master_chain` did not reach the first frame — the chain is \
             being seeded from nothing and moved only by a later record"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Verifies that a mid-session `look` record updates tone mapping across subsequent frames.
    #[test]
    fn a_look_record_reaches_the_frames_after_it() {
        let dir = scratch("look");
        let (store, head) = store_with_a_set(&dir);

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

    /// Verifies that mid-session source bindings affect playback and taking them back restores parameter values (ADR-0319).
    #[test]
    fn an_attachment_reaches_the_frames_after_it_and_a_take_back_gives_the_param_back() {
        let dir = scratch("source");
        let (store, head) = store_with_a_set(&dir);

        // A constant range well away from what the file declares, so the
        // difference is the attachment rather than the phase.
        // `scale` is a declared L1 param of the pair a bare run opens on,
        // over `[0.3, 4.0]` at 2.8 — pinned here at the far end of that range.
        const ATTACH: &str = r#"{"t":"source","slot":0,"layer":"L1","key":"scale","source":{"signal":"beat","curve":"lin","range":[0.4,0.4]}}"#;
        const TAKE_BACK: &str = r#"{"t":"source","slot":0,"layer":"L1","key":"scale"}"#;

        write_session(&store, "plain", &head, &[CANVAS, TICK, TICK, TICK]);
        write_session(&store, "bound", &head, &[CANVAS, TICK, ATTACH, TICK, TICK]);
        write_session(
            &store,
            "freed",
            &head,
            &[CANVAS, TICK, ATTACH, TAKE_BACK, TICK, TICK],
        );

        let plain = replay(&store, "plain", &dir.join("plain.png"));
        let bound = replay(&store, "bound", &dir.join("bound.png"));
        let freed = replay(&store, "freed", &dir.join("freed.png"));

        assert_ne!(
            plain, bound,
            "the `source` record changed nothing — an attachment made live is not \
             reaching the Set the slot is playing"
        );
        assert_eq!(
            plain, freed,
            "a `source` with no attachment did not give the parameter back to its own \
             value: the binding is still driving it, or the last value it wrote was left \
             behind"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Verifies that canvas size defined in the session stream governs replay rendering and cannot be overridden by CLI flags.
    #[test]
    fn the_stream_decides_what_a_replay_renders_at() {
        let dir = scratch("canvas");
        let (store, head) = store_with_a_set(&dir);
        write_session(&store, "small", &head, &[CANVAS, TICK, TICK]);
        write_session(
            &store,
            "odd",
            &head,
            // Non-64 multiple canvas dimensions.
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

    /// Verifies that hot-swapped procedures recorded mid-session are replayed correctly.
    #[test]
    fn a_procedure_record_changes_what_the_rest_of_the_session_renders() {
        let dir = scratch("procedure");
        let (store, head) = store_with_a_set(&dir);

        let root = workspace();
        let drained =
            std::fs::read_to_string(root.join("crates/karakuri-cli/tests/fixtures/flat.kir"))
                .expect("the flat fixture");

        let l1 = std::fs::read_to_string(root.join("examples/drift_shell.kir")).expect("L1");
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

    /// Extracts width and height from standard PNG headers (bytes 16..24).
    fn png_size(bytes: &[u8]) -> (u32, u32) {
        let at = |i: usize| u32::from_be_bytes(bytes[i..i + 4].try_into().expect("4 bytes"));
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "not a PNG");
        (at(16), at(20))
    }

    /// Verifies that encountering a `save` record during replay does not write to the store,
    /// logs a skip message, and leaves rendered frames identical.
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
