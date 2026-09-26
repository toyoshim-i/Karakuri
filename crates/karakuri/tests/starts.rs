//! Verification of application startup as a subprocess (ADR-0141).
//!
//! Spawns `karakuri` in a subprocess and polls child stdout until the startup legend
//! is emitted, capturing stderr upon failure.

// GPU test suite exercising `karakuri` subprocess startup (ADR-0141).
mod gpu {
    use std::io::{BufRead, BufReader, Read};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, ExitStatus, Stdio};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    /// Target prefix in the startup legend confirming the window/device initialized
    /// (lines before it occur prior to GPU adapter acquisition).
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

    /// Drains a child process stream onto a dedicated thread into shared memory to avoid
    /// blocking both the child process write buffer and the test poll loop.
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
        /// Spawns the binary with isolated store and explicit `--presets examples/`
        /// path so preset resolution does not depend on binary location.
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

        /// Asserts the child remains running and stderr contains no panics.
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

        /// Polls across a window to ensure startup and event loop entry remain stable.
        fn stays_up_for(&mut self, duration: Duration) {
            let deadline = Instant::now() + duration;
            while Instant::now() < deadline {
                self.is_still_running();
                std::thread::sleep(POLL);
            }
            self.is_still_running();
        }

        fn no_panic(&self) {
            let said = self.err();
            assert!(
                !said.contains("panicked at")
                    && !said.contains("panic caught in window_event")
                    && !said.contains("panic in a function that cannot unwind"),
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

    /// Asserts termination via external signal (`SIGKILL`) rather than internal exit or panic.
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

    /// Asserts that a standard startup sequence reaches window initialization and prints the startup legend.
    #[test]
    fn the_panel_reaches_a_window_and_prints_its_legend() {
        let mut panel = Panel::launch("bare", &[]);
        panel.wait_for(LEGEND);
        panel.stays_up_for(Duration::from_millis(500));

        let transcript = panel.transcript();
        let status = panel.stop();
        ended_on_the_signal(status, &transcript);
        // Read again after the wait: anything the child wrote as it died is in
        // the stream by now, and a panic on the way down is still a panic.
        panel.no_panic();
    }

    /// Verifies `--mcp 0` binds an ephemeral port before window initialization
    /// and prints the resolved non-zero port before continuing to the legend.
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
        panel.stays_up_for(Duration::from_millis(500));

        let transcript = panel.transcript();
        let status = panel.stop();
        ended_on_the_signal(status, &transcript);
        panel.no_panic();
    }
}
