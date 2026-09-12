//! The panel starts. Not that a deck builds, not that a widget draws — that
//! `cargo run -p karakuri` reaches a window and says so, in the process an
//! operator would have launched.
//!
//! This is the one claim nothing else in the workspace makes. `main.rs`'s own
//! `mod gpu` tests build an `Engine` on a headless device and assert about what
//! it produces; not one of them runs [`main`], parses a command line, resolves
//! a preset library, writes working copies, opens a store, seeds the snapshots,
//! binds MCP, creates a window, makes a surface, builds the panel and prints
//! the legend — which is the *order* those happen in and therefore the only
//! place a startup panic can live. The panel has stopped starting more than
//! once with the whole suite green, because the suite has never started it.
//!
//! Why the legend and not the exit code. There is no flag that makes this
//! program exit on its own: `--help` prints [`USAGE`] and returns without
//! touching a device, and every other run ends when the operator closes the
//! window or presses `esc`. So "it started" cannot be a status; it has to be
//! something the program says *after* the part that can fail. The legend is
//! that line — `Readout::print_legend` is called from `resumed`, after the
//! surface, the adapter, `Engine::new` and the first solve, so a process that
//! has printed it is past every step this test exists to watch. A panic inside
//! `resumed` on macOS aborts rather than unwinds (it cannot cross the
//! Objective-C frame), so the legend is also the only evidence that would
//! survive one.
//!
//! Bounded, and not a fixed wait. The window is polled for the legend every few
//! milliseconds for up to ten seconds and the test finishes the moment the line
//! appears — usually well under a second. Ten seconds is the ceiling on a
//! failure, not the cost of a pass. Sleeping ten seconds instead would have
//! made the pass cost the failure's price and told the reader nothing more.
//!
//! What a red run looks like. If the process exits before the legend, this test
//! fails *with the child's stderr in the message* — which is the panic, the
//! refusal or the `no adapter:` line, printed where whoever ran the test will
//! read it. That is the whole point: the failure this catches is a sentence the
//! program already writes and nobody was listening for.

// Every test here drives `karakuri` as a subprocess, and that binary opens a
// window and takes a device of its own — so these are GPU tests that no
// in-process rule can see, exactly as `karakuri-cli/tests/replay.rs` is. The
// whole file is one `mod gpu`, and karakuri-engine's
// `tests/gpu_tests_are_under_mod_gpu.rs` is what keeps that honest: `karakuri`
// is named in its `GPU_BINARIES`, so a spawn of it reads as the reach it is.
//
// On a machine with no adapter these fail rather than skip, and the message is
// the program's own `no adapter: …` — which is ADR-0141 working as written.
mod gpu {
    use std::io::{BufRead, BufReader, Read};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, ExitStatus, Stdio};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    /// The first line of the legend, and the shortest prefix of it that is not a
    /// number: `print_legend` opens with `"the console, in a {w} x {h} viewport."`
    /// and the two figures depend on the window this machine opened. Everything
    /// before them is fixed.
    ///
    /// It is deliberately a line from *after* the device rather than one of the
    /// three `main` prints before it — `scratch:`, `presets:` and `mcp:` are all
    /// reached without an adapter, so waiting on one of those would pass on a run
    /// that then died making a surface.
    const LEGEND: &str = "the console, in a ";

    /// The ceiling on a failure. A pass leaves the moment the line lands.
    const WAIT: Duration = Duration::from_secs(10);

    /// Short enough that the pass is not paced by it, long enough that the poll is
    /// not a spin.
    const POLL: Duration = Duration::from_millis(20);

    /// The workspace root. Integration tests run with the *package* as the working
    /// directory, and this program's preset library and its examples are named
    /// relative to the workspace — the same move `replay.rs` makes and for the same
    /// reason.
    fn workspace() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root")
    }

    /// A store of this run's own. Never the default `.karakuri`: this test writes
    /// four decks' working copies and a snapshot history, and doing that in
    /// whatever store the machine's operator keeps their Sets in would make the
    /// test a thing you have to think about before running.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("karakuri-starts-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch");
        dir
    }

    /// Read a child's stream to its end on a thread of its own, into a string the
    /// test can look at whenever it likes.
    ///
    /// A thread rather than a read in the poll loop, for two reasons. A `read` on a
    /// pipe blocks, so reading inline would stop the poll from ever noticing that
    /// the process had died; and the legend is several kilobytes, so a child whose
    /// pipe nobody drains can block *writing* it and hang exactly where this test
    /// is watching for it.
    fn drained<R: Read + Send + 'static>(stream: R) -> Arc<Mutex<String>> {
        let held = Arc::new(Mutex::new(String::new()));
        let into = Arc::clone(&held);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            // `read_line` rather than `lines()`: the last thing a killed child
            // wrote may have no newline on it, and that is exactly the half
            // line a panic message ends on.
            while reader.read_line(&mut line).unwrap_or(0) > 0 {
                into.lock().expect("the stream buffer").push_str(&line);
                line.clear();
            }
        });
        held
    }

    /// A running panel, with both of its streams being drained.
    struct Panel {
        child: Child,
        out: Arc<Mutex<String>>,
        err: Arc<Mutex<String>>,
        dir: PathBuf,
        said: String,
    }

    impl Panel {
        /// Launch the program an operator launches, with a store and a preset library
        /// that belong to this test.
        ///
        /// `--presets` is given rather than left to resolve. Without it the library is
        /// looked for beside the binary and then in the workspace the binary was
        /// compiled in, and which of those answers depends on where the run happened —
        /// a test that passes in a checkout and fails from an installed prefix is
        /// testing the machine. Named, it is `examples/`, on every machine.
        fn launch(name: &str, extra: &[&str]) -> Panel {
            let dir = scratch(name);
            let store = dir.join("store");
            let root = workspace();
            let mut child = Command::new(env!("CARGO_BIN_EXE_karakuri"))
                .current_dir(&root)
                .args([
                    "--presets",
                    root.join("examples").to_str().expect("utf-8 path"),
                ])
                .args(["--store", store.to_str().expect("utf-8 path")])
                .args(extra)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("run karakuri");
            let out = drained(child.stdout.take().expect("stdout is a pipe"));
            let err = drained(child.stderr.take().expect("stderr is a pipe"));
            Panel {
                child,
                out,
                err,
                dir,
                said: extra.join(" "),
            }
        }

        fn out(&self) -> String {
            self.out.lock().expect("the stdout buffer").clone()
        }

        fn err(&self) -> String {
            self.err.lock().expect("the stderr buffer").clone()
        }

        /// Everything both streams hold, for a failure message. A startup failure is a
        /// sentence this program already writes, and the whole value of this test is
        /// putting that sentence in front of whoever ran it.
        fn transcript(&self) -> String {
            format!(
                "--- stdout ---\n{}\n--- stderr ---\n{}",
                self.out().trim_end(),
                self.err().trim_end()
            )
        }

        /// Poll for a line, and stop on the first of three things: the line arrives,
        /// the process exits, or the ceiling is reached. Returns as soon as the line is
        /// there.
        fn wait_for(&mut self, needle: &str) {
            let deadline = Instant::now() + WAIT;
            loop {
                if self.out().contains(needle) {
                    return;
                }
                let exited = self.child.try_wait().expect("try_wait on the panel");
                if let Some(status) = exited {
                    // The drain threads may be a beat behind the exit, so the
                    // line is looked for once more before this is called a
                    // failure. Give them the end of the stream first.
                    std::thread::sleep(POLL);
                    if self.out().contains(needle) {
                        return;
                    }
                    panic!(
                        "the panel exited ({status}) before printing `{needle}`. \
                         `karakuri {}` is what an operator types, and it did not reach a \
                         window — the reason is below, and it is the panic this test \
                         exists to show.\n{}",
                        self.said,
                        self.transcript()
                    );
                }
                assert!(
                    Instant::now() < deadline,
                    "{:?} and the panel has not printed `{needle}` — it is still running, so \
                     it is hung somewhere before the legend rather than dead.\n{}",
                    WAIT,
                    self.transcript()
                );
                std::thread::sleep(POLL);
            }
        }

        /// Still up, and nothing on stderr that reads as a panic.
        ///
        /// Both, because either alone is weak. A process can print the legend and abort
        /// on the very next frame; and a `winit` callback that aborts on macOS may
        /// leave nothing but the abort itself, so a clean stderr on a dead process
        /// proves nothing either.
        fn is_still_running(&mut self) {
            let exited = self.child.try_wait().expect("try_wait on the panel");
            assert!(
                exited.is_none(),
                "the panel printed its legend and then exited ({}) — it starts and does not \
                 stay up.\n{}",
                exited.expect("just checked"),
                self.transcript()
            );
            self.no_panic();
        }

        fn no_panic(&self) {
            let said = self.err();
            assert!(
                !said.contains("panicked at"),
                "the panel panicked on the way up.\n{}",
                self.transcript()
            );
        }

        /// End it, the only way this program ends without a keyboard: a signal.
        fn stop(&mut self) -> ExitStatus {
            self.child.kill().expect("kill the panel");
            self.child.wait().expect("wait for the panel")
        }
    }

    /// Kill it whatever happened, including on a failed assertion above — otherwise
    /// a red test leaves a window open and a process running with nobody left to
    /// close it. The scratch goes the same way and for the same reason.
    impl Drop for Panel {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    /// The exit of a killed process is the signal's, and not a panic's.
    ///
    /// A Rust panic that unwinds out of `main` exits 101; an abort is a signal of
    /// its own. Neither is what `kill` produces, and the assertion is that what
    /// came back is the killing rather than something the program decided on its
    /// own in the moment before it.
    fn ended_on_the_signal(status: ExitStatus, transcript: &str) {
        assert_ne!(
            status.code(),
            Some(101),
            "the panel exited with a panic's code rather than on the signal.\n{transcript}"
        );
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            assert_eq!(
                status.signal(),
                Some(9),
                "the panel did not end on the signal it was sent — it had already died of \
                 something else.\n{transcript}"
            );
        }
    }

    /// A bare run reaches the window. The command line, the preset library, the
    /// working copies, the store, the snapshot seed, the event loop, the surface,
    /// the adapter, the deck and the panel — in that order, in one process, and the
    /// legend is the far side of all of it.
    #[test]
    fn the_panel_reaches_a_window_and_prints_its_legend() {
        let mut panel = Panel::launch("bare", &[]);
        panel.wait_for(LEGEND);
        panel.is_still_running();

        let transcript = panel.transcript();
        let status = panel.stop();
        ended_on_the_signal(status, &transcript);
        // Read again after the wait: anything the child wrote as it died is in
        // the stream by now, and a panic on the way down is still a panic.
        panel.no_panic();
    }

    /// `--mcp 0` binds before the window and says which port it got, and the run
    /// goes on to the legend anyway.
    ///
    /// The flag is here because it is the one startup step that reaches outside
    /// this process before the device does, and `0` is the argument with no way to
    /// collide: an ephemeral port is one the machine says is free, where a fixed
    /// number in a test is a number some other program on the machine may be
    /// holding. The printed port is asked of the server rather than echoed from the
    /// flag, so `0` in and `0` out would be the bug — hence the parse rather than a
    /// `contains`.
    #[test]
    fn the_mcp_port_is_bound_and_printed_and_the_panel_still_starts() {
        let mut panel = Panel::launch("mcp", &["--mcp", "0"]);
        panel.wait_for("mcp: 127.0.0.1:");
        let said = panel.out();
        let port: u16 = said
            .split("mcp: 127.0.0.1:")
            .nth(1)
            .expect("the line just waited for")
            .split(|c: char| !c.is_ascii_digit())
            .next()
            .expect("digits after the colon")
            .parse()
            .expect("a port number");
        assert_ne!(
            port,
            0,
            "`--mcp 0` printed the argument rather than the port it bound.\n{}",
            panel.transcript()
        );

        panel.wait_for(LEGEND);
        panel.is_still_running();

        let transcript = panel.transcript();
        let status = panel.stop();
        ended_on_the_signal(status, &transcript);
        panel.no_panic();
    }
}
