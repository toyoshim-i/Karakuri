use karakuri_console::budget::PANEL_PASS;
use karakuri_console::view;
use karakuri_engine::probe::MeasurementMethod;

use super::entry::{
    drifted, drifted_ms, Still, DRIFT, STILL, WRITTEN_ALLOCS, WRITTEN_KB, WRITTEN_ON,
};
use super::{ms, Costs};
use crate::{PROFILE, WINDOW};

impl Costs {
    /// The reading. Median and worst rather than a mean for the per-frame figures:
    /// a frame path is judged by its tail. `capacity` and `material` are the run's,
    /// handed in rather than read off a constant: this program takes its `.kir`
    /// pair from the command line and a load can move a slot off it, so what the
    /// engine half of this reading was taken over is only known at run time — and
    /// is every slot's name in slot order rather than one. See
    /// [`Engine::capacity`], [`Sources::material`] and [`Gfx::material`]. `at` is
    /// what the frame was actually composited at when the reading was taken —
    /// `Present::size()`, which is the largest enabled output's size and no longer
    /// a constant ([`render_size`]). A measurement names which resolution it is
    /// about
    /// ([ADR-0303](../../../docs/adr/0303-a-frames-cost-is-the-period-and-a-measurement-names-which-resolution-it-is-about.md),
    /// [P-0095](../../../docs/principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md)),
    /// and [`CANVAS`] would now be the wrong one on any run whose Program bay is
    /// not exactly that size — which is every run.
    pub(crate) fn say(
        &mut self,
        capacity: u32,
        material: &str,
        refresh_ms: Option<f32>,
        at: (u32, u32),
    ) {
        if self.said {
            return;
        }
        self.said = true;

        println!();
        println!(
            "{}",
            match self.live {
                // There is no still panel to cost while anything is live, and
                // calling it one would be the reading describing a program
                // that is not running — which is the failure the sample this
                // replaces made, one revision ago. **"Something live" rather
                // than "a live picture"**: the picture can be folded away with
                // deck A still auditioning under it, and the window is no more
                // still then than it was before.
                true => "what an untouched window costs with something live in it:",
                false => "what a still panel costs, measured on this window:",
            }
        );
        println!(
            "  over {:.1} s with nothing touching it: {} frames drawn, {} allocations, \
             {} bytes",
            STILL.as_secs_f64(),
            self.still.frames,
            self.still.allocs,
            self.still.bytes
        );
        let rate = self.rate_over(STILL.as_secs_f64());
        match (self.live, self.still == Still::default()) {
            // The reading this was written for. It is reachable with the
            // picture, the preview row, the mixer bay **and the transport
            // row** folded away — the first two stop the texels and the last
            // two stop the two declarations (ADR-0193) — and with any one of
            // the four on screen it is not. The fourth is P-0094 arriving:
            // the beat is a light travelling the grid, it declares for as
            // long as it is drawn, and a console claiming to show a live
            // instrument has something moving on it (ADR-0212).
            (false, true) => println!(
                "  so ADR-0164's still-panel clause holds here: no per-frame work is \
                 done to redraw what nobody has touched and nothing has moved."
            ),
            (false, false) => match self.declared {
                // **The panel said it needed them**, and that is ADR-0164's
                // second clause working rather than its still-panel one failing: the
                // parked deck's tally declares a staleness and the window
                // serves it. Measured on 2026-08-26 with the picture and the
                // preview row folded away and the mixer bay on screen: 28.0
                // and 28.3 frames a second over two runs, at 427 and 425
                // allocations a frame.
                //
                // **Folding the mixer bay away takes this arm out of reach**,
                // and that is ADR-0193: the slot stays parked, the chip is not
                // drawn, and a region that is not laid out declares nothing —
                // the same two runs read 0 frames with the bay folded, which
                // is the arm above. It read 28.7 to 29.0 a second at 260
                // allocations before that change, which is the defect that
                // record closes.
                //
                // **What is printed is a deadline and no longer a rate**, and
                // ADR-0283 is why: a region declares its staleness for the
                // motion it has, so the mixer's answer is 33.3 ms through the
                // 400 ms its roll travels and the remainder of the rest
                // through the 600 ms it does not. The reciprocal of one of
                // those is not the rate anything runs at, and printing it as
                // one is how a reading names the wrong cause. The rate the
                // window actually drew at is the line above this one, which is
                // the number that was measured rather than derived.
                Some(deadline) => println!(
                    "  so ADR-0164's still-panel clause does NOT hold here, and the \
                     reason is a declaration rather than a fault: {}, and the soonest \
                     a declaring region will next move is {:.1} ms away. Folding the \
                     region that draws it ends its term: a region that is not laid out \
                     declares nothing (ADR-0193).",
                    match deadline == view::BEAT_STALENESS {
                        // The one that runs whether or not anything is
                        // happening, which is the whole of why it is here —
                        // and the one region whose deadline is its declared
                        // staleness on every frame, because the light never
                        // rests (ADR-0283).
                        true =>
                            "the beat grid is a light travelling the transport row, \
                                 and it moves for as long as the console is live rather \
                                 than while something is pending (P-0094, ADR-0212)",
                        false =>
                            "something on this panel is parked and the mixer's tally \
                                  is rolling toward a residency nobody granted (ADR-0190)",
                    },
                    deadline.as_secs_f64() * 1000.0,
                ),
                // Nothing live, nothing declared, and frames drawn anyway.
                None => println!(
                    "  so ADR-0164's still-panel clause does NOT hold here — something \
                     is asking for frames on an untouched window, nothing on the panel is \
                     making texels and nothing has declared a staleness, so the likeliest \
                     something is an `egui` repaint delay answered immediately instead of \
                     waited out."
                ),
            },
            // **The expected reading now**, and the whole of what this run is
            // for. It is stated as a price rather than as a failure, because
            // that is what it is: the clause is about a panel with nothing
            // changing on it, and a live engine frame is something changing on
            // it.
            (true, _) => {
                println!(
                    "  so ADR-0164's still-panel clause has stopped holding, and the \
                     reason is the engine: there is a live frame in the Program bay — the \
                     picture, deck A auditioning in the preview row under it, or both — so \
                     every one of those frames was asked for by what is live rather than \
                     by anybody touching the window."
                );
                println!(
                    "  that is {rate:.1} frames a second, against 0 with the engine out — \
                     which is the whole of the difference, since what one frame costs is \
                     below and did not change."
                );
                println!(
                    "  it is P-0091 from here — anything that must be live declares its \
                     price — and this is the price, measured. Two regions declare it: the \
                     transport row for as long as the beat grid is drawn (P-0094, \
                     ADR-0212) and the mixer bay while something in it is pending. \
                     Nothing in this run schedules, caches the panel to a texture or \
                     arbitrates between the two; the number is what the next decision gets \
                     made on."
                );
            }
        }

        if !self.frames.is_empty() {
            let mut engine: Vec<f64> = self.frames.iter().map(|c| ms(c.engine)).collect();
            let mut ui: Vec<f64> = self.frames.iter().map(|c| ms(c.ui)).collect();
            let mut paint: Vec<f64> = self.frames.iter().map(|c| ms(c.paint)).collect();
            let mut textures: Vec<f64> = self.frames.iter().map(|c| ms(c.textures)).collect();
            let mut buffers: Vec<f64> = self.frames.iter().map(|c| ms(c.buffers)).collect();
            let mut record: Vec<f64> = self.frames.iter().map(|c| ms(c.record)).collect();
            let mut submit: Vec<f64> = self.frames.iter().map(|c| ms(c.submit)).collect();
            let mut wait: Vec<f64> = self.frames.iter().map(|c| ms(c.wait)).collect();
            // **What drawing the panel costs**, which is the figure
            // `karakuri_console::budget::PANEL_PASS` declares and the reason
            // this vector exists: the immediate-mode pass, plus the panel's
            // own texture and geometry uploads and the recording of its render
            // pass. Not `engine`, which is the governor's and would be counted
            // twice (ADR-0210); not `wait`, which is doing nothing on purpose;
            // and **not `submit`**, which is larger than all of this together
            // and carries the engine's half of the frame as well, so charging
            // it to a region would charge a region for a frame it did not ask
            // for.
            let mut draw: Vec<f64> = self
                .frames
                .iter()
                .map(|c| ms(c.ui + c.textures + c.buffers + c.record))
                .collect();
            // **Median, where the figure this replaces was a mean.** The mean
            // was over 180 frames and the first one was lost in it; the sample
            // here is however many frames somebody asked for, which on a run
            // nobody touches is three — and the first of those builds the font
            // atlas and allocates ten times what a frame does. A mean of three
            // is that one frame with two others attached.
            let mut allocs: Vec<u64> = self.frames.iter().map(|c| c.allocs).collect();
            let mut bytes: Vec<u64> = self.frames.iter().map(|c| c.bytes).collect();
            engine.sort_by(f64::total_cmp);
            ui.sort_by(f64::total_cmp);
            paint.sort_by(f64::total_cmp);
            textures.sort_by(f64::total_cmp);
            buffers.sort_by(f64::total_cmp);
            record.sort_by(f64::total_cmp);
            submit.sort_by(f64::total_cmp);
            wait.sort_by(f64::total_cmp);
            draw.sort_by(f64::total_cmp);
            allocs.sort_unstable();
            bytes.sort_unstable();
            let n = self.frames.len();

            println!();
            println!(
                "what a frame costs when something asks for one, over the {} drawn so far \
                 ({} sampled):",
                self.drawn, n
            );
            println!(
                "  engine pass  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms",
                engine[n / 2],
                engine[n * 95 / 100],
                engine[n - 1]
            );
            println!(
                "  egui pass    median {:.3} ms   p95 {:.3} ms   worst {:.3} ms",
                ui[n / 2],
                ui[n * 95 / 100],
                ui[n - 1]
            );
            println!(
                "  upload+pass  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms",
                paint[n / 2],
                paint[n * 95 / 100],
                paint[n - 1]
            );
            // **The four parts of `upload+pass`, and the reason they are
            // printed rather than derived.** `submit` alone told the caching
            // decision what it was *not* — the submission is not what caching
            // a bay into a texture would make cheaper — without telling it
            // what it was. The upload is the line that decision turns on, and
            // it is worth what a bus costs on the machine reading it, so it
            // has to be a number this program prints on every machine rather
            // than one somebody instruments for once.
            println!(
                "    of which texture uploads  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms — \
                 the font atlas, built once, so zero on a steady frame",
                textures[n / 2],
                textures[n * 95 / 100],
                textures[n - 1]
            );
            println!(
                "    of which buffer uploads   median {:.3} ms   p95 {:.3} ms   worst {:.3} ms — \
                 the tessellated geometry. Not the price of crossing a bus: it is seven times \
                 larger here than on a discrete GPU that crosses one (ADR-0167)",
                buffers[n / 2],
                buffers[n * 95 / 100],
                buffers[n - 1]
            );
            println!(
                "    of which record the pass  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms",
                record[n / 2],
                record[n * 95 / 100],
                record[n - 1]
            );
            println!(
                "    of which submit  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms — \
                 `wgpu`'s per-submission work, not the GPU's and not a wait",
                submit[n / 2],
                submit[n * 95 / 100],
                submit[n - 1]
            );
            println!(
                "  waiting for vsync  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms — \
                 blocked in `get_current_texture`, and in none of the three above",
                wait[n / 2],
                wait[n * 95 / 100],
                wait[n - 1]
            );
            println!(
                "  the egui pass allocates a median {} times a frame, {:.1} kB a frame \
                 (worst {} and {:.1} kB, which is the first frame building the font atlas)",
                allocs[n / 2],
                bytes[n / 2] as f64 / 1024.0,
                allocs[n - 1],
                bytes[n - 1] as f64 / 1024.0
            );
            println!(
                "  the CPU's three stretches are a median {:.3} ms between them, so at \
                 {:.1} frames a second the loop is spending {:.1}% of a second inside \
                 them. THAT IS NOT WHAT THE FRAME COST — see the block below, which \
                 measures the frame itself.",
                engine[n / 2] + ui[n / 2] + paint[n / 2],
                rate,
                (engine[n / 2] + ui[n / 2] + paint[n / 2]) * rate / 10.0
            );
            println!(
                "  that per-frame price is what immediate mode pays by construction, and it \
                 is no longer ADR-0164's: that record measured 184 allocations and 226.2 kB \
                 a frame here with every bay empty, which this panel has not been since the \
                 mixer bay landed — 456 allocations there, and 525 once deck B was parked \
                 (ADR-0191). Taken again on {WRITTEN_ON} over nine runs of this program \
                 with nothing touching the window: {WRITTEN_ALLOCS} allocations and \
                 {WRITTEN_KB:.1} kB a frame, which is what every one of the nine read — to \
                 the allocation, and to the tenth of a kilobyte. What the panel had in it \
                 while they were taken is the last paragraph below. What ADR-0164 is still right about is that \
                 the price is paid on every frame drawn; what changed is how many frames pay \
                 it — 0 with a still panel and nothing in the Program bay, and the rate above \
                 with anything live in it."
            );
            // **The sentence above is checked against the run that has just
            // been taken**, which is the only place either can be: the number
            // needs a window, three seconds of nobody touching it and a
            // device, and none of those is reachable from `cargo test`. So the
            // claim and its check are printed together, and the figure in the
            // prose is the figure being checked rather than a second copy of
            // it.
            match drifted(allocs[n / 2], WRITTEN_ALLOCS) {
                Some(factor) => println!(
                    "  and THIS run read {}, which is {factor:.1}x that — past the {DRIFT:.0}x \
                     this file will quote a figure across. **The sentence above is stale.** \
                     Re-take it over several runs of this program, write what the panel had in \
                     it, and re-date `WRITTEN_ALLOCS`, `WRITTEN_KB` and `WRITTEN_ON` in \
                     `crates/karakuri/src/main.rs` — which is what nobody did for the two commits before \
                     this line existed.",
                    allocs[n / 2]
                ),
                None => println!(
                    "  and THIS run read {}, within {DRIFT:.0}x of that, so the sentence above \
                     is still one this window produces.",
                    allocs[n / 2]
                ),
            }
            // **What P-0091 calls a cost, measured and held against what
            // declares it.** `budget::PANEL_PASS` is a constant somebody wrote
            // down — ADR-0164 refuses a schedule made of measurements, because
            // one reorders itself with the machine's noise — and a constant
            // that nothing checks is the failure `WRITTEN_ALLOCS` above exists
            // for, one number along. So the declaration and the reading are
            // printed together, and this is the only place either can be: the
            // figure needs a window, three seconds of nobody touching it and a
            // device, and none of those is reachable from `cargo test`.
            //
            // **It reports and does not fail**, which is deliberate. This
            // machine reads about a sixth of these numbers with its other
            // cores loaded, and a gate on a millisecond here would be one
            // nobody could keep passing — flaky is worse than broken
            // (`docs/contributing.md` §1). What *is* asserted, without a
            // clock, is that every region declares this one constant:
            // `tests/schedulable.rs`.
            println!(
                "  drawing the panel is a median {:.3} ms of that — the egui pass, its \
                 texture and geometry uploads and the recording of its render pass, which \
                 is what one update of a live region costs under P-0091. The submission is \
                 not in it: it carries the engine's half of the frame as well. \
                 `karakuri_console::budget::PANEL_PASS` declares {:.3} ms,",
                draw[n / 2],
                ms(PANEL_PASS),
            );
            match drifted_ms(draw[n / 2], ms(PANEL_PASS)) {
                Some(factor) => println!(
                    "  and THIS run read {:.3}, which is {factor:.1}x that — past the \
                     {DRIFT:.0}x this file will quote a figure across. **The declared cost \
                     is stale.** Re-take it over several runs of this program and rewrite \
                     `PANEL_PASS` in `crates/karakuri-console/src/budget.rs`, with the \
                     machine and the date beside it, because both schedulability \
                     conditions are asserted against that number and nothing else measures \
                     it.",
                    draw[n / 2]
                ),
                None => println!(
                    "  and THIS run read {:.3}, within {DRIFT:.0}x of that, so the declared \
                     cost is still one this window produces.",
                    draw[n / 2]
                ),
            }
            println!("  taken on {}", self.taken_on);
            println!(
                "  the panel half is taken on this window at {:.0}x{:.0} logical, drawing a \
                 live picture, four preview cells with deck A auditioning in one and three \
                 off, the mixer bay with a strip in every one of its four tracks, the \
                 transport row with its `audio-in` and arrangement pills, the outputs row \
                 and deck B's parked \
                 tally rolling once a second — over the Library bay's scope row and however \
                 many rows the scope marked in it lists, over the Master bay's out row, over \
                 the Inspector's two panes read off the running Set, and over Staging and \
                 Sequencer, which are a head and nothing else. \
                 That is NOT the workspace's \
                 reference workload. The \
                 engine half is four slots of `{}` — {} elements each at {}x{}, and each \
                 advances by the frame's own measured step count, which on a 60 Hz \
                 display is one step a frame apiece and on a faster one is one step \
                 every second or third frame (ADR-0297). The other three are allocated, \
                 one of them parked, and \
                 every one of them steps and draws into its own cell on every frame \
                 (ADR-0269), so all four simulations and all four draws are in these \
                 numbers — and five presents at the canvas's own shape: each slot's \
                 canvas into its own preview cell, and the mix into the picture's \
                 rectangle. The workspace's reference workload is \
                 `examples/drift_cloud.kset` at 1280x720 (docs/contributing.md §1, \
                 ADR-0270) — a named Set rather than whatever this program opens on — \
                 and the size above is this window's rather than that one: the mix is \
                 composited at the largest enabled output and the only output here is the \
                 Program bay's picture, so the number moves with the window and with every \
                 divider (ADR-0247). This reading is comparable with the rest of this \
                 repository's figures exactly as far as it is that material at that size, and \
                 never with a headless one: this is a deck of four stepped and drawn \
                 slots with a panel over it, and a headless figure is one Set. Host \
                 clock, {}.",
                WINDOW.0, WINDOW.1, material, capacity, at.0, at.1, PROFILE
            );
            println!(
                "  and every figure above is taken on a core that spends the vsync wait \
                 asleep. On THIS machine that matters a great deal — the identical run with \
                 the other cores loaded reports about a sixth of these numbers, proportions \
                 unchanged — and it is this machine's power management rather than a rule: \
                 two Windows machines were asked the same way and got 1.3x and 1.6x WORSE \
                 under load, which is ordinary contention. Compare ratios, not magnitudes."
            );

            // -- what a whole frame cost -----------------------------
            // **The block the three medians above cannot be.** Everything
            // printed so far is CPU time that stops at a submission, and the
            // one field that is not — `wait` — is reported beside the frame
            // rather than in it. That reading holds exactly while the GPU is
            // not the bottleneck, and says nothing at all when it is: a panel
            // at four frames a second read off those three as a loop idle
            // 97.6% of the time, and the loop was not idle. See
            // `Cost::period`, `Cost::drained` and ADR-0303.
            //
            // **Filtered series, so they get their own lengths.** A frame has
            // no period until it has a predecessor and no drain unless it was
            // audited, so `n` above is not theirs and neither is its median.
            pub(crate) fn middle(xs: &[f64]) -> Option<f64> {
                let mut xs = xs.to_vec();
                xs.sort_by(f64::total_cmp);
                xs.get(xs.len() / 2).copied()
            }
            let mut periods: Vec<f64> = self
                .frames
                .iter()
                .filter_map(|c| c.period)
                .map(ms)
                .collect();
            let elsewhere: Vec<f64> = self
                .frames
                .iter()
                .filter_map(|c| c.elsewhere())
                .map(ms)
                .collect();
            let drained: Vec<f64> = self
                .frames
                .iter()
                .filter_map(|c| c.drained)
                .map(ms)
                .collect();
            // **What an audited frame's period was, against the rest.** This
            // is the instrument reporting its own price: a blocking poll is
            // the one thing here that could become the cost it is measuring,
            // and the two medians beside each other are the only honest way to
            // say it did not.
            let audited: Vec<f64> = self
                .frames
                .iter()
                .filter(|c| c.drained.is_some())
                .filter_map(|c| c.period)
                .map(ms)
                .collect();
            let rest: Vec<f64> = self
                .frames
                .iter()
                .filter(|c| c.drained.is_none())
                .filter_map(|c| c.period)
                .map(ms)
                .collect();
            periods.sort_by(f64::total_cmp);

            println!();
            println!(
                "what a WHOLE frame cost, with the wait inside it rather than beside it \
                 (ADR-0303):"
            );
            match periods.is_empty() {
                // One frame drawn and no second one, so there is no interval.
                // Said rather than divided: a period over no frames is not a
                // small number, it is not a number (P-0095).
                true => println!(
                    "  no frame had a predecessor to be an interval from, so this run \
                     measured no frame period at all."
                ),
                false => {
                    let m = periods.len();
                    println!(
                        "  frame period  median {:.3} ms   p95 {:.3} ms   worst {:.3} ms — top of \
                         one redraw to the top of the next, over {} of the {} sampled",
                        periods[m / 2],
                        periods[m * 95 / 100],
                        periods[m - 1],
                        m,
                        n
                    );
                    println!(
                        "    of which the three stretches above are {:.3} ms and the wait is \
                         {:.3} ms, leaving a median {:.3} ms this file times nowhere — the sinks \
                         aimed, the bay rearranged, the panel solved, `Queue::present`, and \
                         whatever `winit` does between two redraws. `Cost::whole` said the three \
                         *tile the frame exactly*; this is the measurement that says otherwise.",
                        engine[n / 2] + ui[n / 2] + paint[n / 2],
                        wait[n / 2],
                        middle(&elsewhere).unwrap_or(0.0)
                    );
                    println!(
                        "    and 1000/period is {:.1} frames a second against the {:.1} counted \
                         over the stretch above — two routes to one rate, taken by two clocks, \
                         which is the only check either of them gets.",
                        1000.0 / periods[m / 2],
                        rate
                    );
                    match refresh_ms {
                        // **What tells a vsync wait from a wait on the GPU**,
                        // and the only thing that can on a host clock: the
                        // wait itself is one field whichever it was.
                        Some(refresh) => println!(
                            "    against this display's {refresh:.1} ms refresh interval that is \
                             {:.2}x. At about 1x the wait is the display's pace and the loop has \
                             headroom; well past it the wait is the GPU and the three medians \
                             above are measuring a loop that is not idle at all.",
                            periods[m / 2] / f64::from(refresh)
                        ),
                        None => println!(
                            "    and with no refresh interval from `winit` there is nothing to \
                             hold it against, so this run cannot say whether the wait was the \
                             display's pace or the GPU."
                        ),
                    }
                }
            }
            match middle(&drained) {
                Some(owed) => println!(
                    "  the GPU still owed a median {owed:.3} ms when the CPU had finished the \
                     frame — `Device::poll` to a drained queue, over {} audited frames at one \
                     every {:.0} ms. Host clock and biased high: the poll's own round trip is in \
                     it, and so is anything of the previous frame still in flight. It is the \
                     whole submission — four slots stepped and drawn, the composite, five \
                     presents and the panel's pass — and not one of them.",
                    drained.len(),
                    ms(Costs::AUDIT)
                ),
                // Not a zero. A drain nobody measured has no duration, and
                // 0.0 ms here would read as a GPU with nothing to do.
                None => println!(
                    "  and what the GPU owed was not measured on this run: no frame was audited, \
                     so this reading has no GPU number in it and does not have one to give."
                ),
            }
            match (middle(&audited), middle(&rest)) {
                (Some(a), Some(b)) => println!(
                    "  an audited frame ran a median {a:.3} ms against {b:.3} ms for the rest, \
                     which is what this instrument costs on this machine — measured, because a \
                     blocking poll is the one thing here that could become the cost it is \
                     measuring."
                ),
                _ => println!(
                    "  and what the audit costs is not in this run: it takes both audited and \
                     unaudited frames in the sample to say."
                ),
            }
            println!(
                "  taken on a host clock, and this is why: {}",
                match self.clock {
                    Some(MeasurementMethod::GpuTimestamp) =>
                        "this adapter's timestamp queries survived the deck probe's calibration, \
                         so a *pass* can be timed on the GPU here — and a *frame* still cannot. \
                         Most of one is outside every command buffer: the block in \
                         `get_current_texture`, the `egui` pass, `Queue::present`. The period is \
                         a host figure on any adapter",
                    Some(MeasurementMethod::HostWallClock) =>
                        "this adapter's timestamp queries did not survive the deck probe's \
                         calibration — the feature is advertised, and a load that cannot take \
                         zero time resolved to zero anyway (P-0095, ADR-0169) — so there is no \
                         GPU clock here to have used. The period would be a host figure \
                         regardless: most of a frame is outside every command buffer",
                    // Not read off `Features`. A flag is what the platform
                    // says rather than what it does, which is the whole of
                    // P-0095.
                    None =>
                        "nothing probed this adapter on this run, so this program has no verdict \
                         on its timestamps and will not read one off `Features`",
                }
            );
            println!(
                "  what it cannot see: which pass inside the submission the drain belongs to; a \
                 wait on the display told apart from a wait on the GPU except by the ratio \
                 above; and any frame that was not audited, whose GPU cost is in no field here \
                 and arrives one frame later folded into the wait."
            );
            // **Which size each number is about**, which this application can
            // answer in exactly two ways and no third one: ADR-0247 composites
            // the mix once at the output's size and resizes that one render
            // into every rectangle, and a slot is auditioned in a deck cell
            // sized from the cell. A measurement taken at neither — the
            // 1280x720 constant ADR-0303 removed — is a number about a frame
            // nobody draws.
            println!(
                "  and the sizes these are about, of which this program has two: the frame is \
                 this whole window at {:.0}x{:.0} logical, with the mix composited once at \
                 {}x{} and that one render resized into every rectangle it is drawn in \
                 (ADR-0247); a slot's own measurement is taken at its preview cell, which is \
                 sized from the cell (ADR-0303). No figure here is taken at a third size. \
                 The mix's size is the largest enabled output's and is not a constant \
                 (ADR-0247): it is the Program bay's picture here, and it follows a divider \
                 drag and a projector window.",
                WINDOW.0, WINDOW.1, at.0, at.1
            );
        }
        println!();
        println!(
            "{}",
            match self.live {
                // **Two sinks, so two folds.** This said "fold the picture
                // away (f over it) and it does" while deck A's cell was
                // drawing in the row underneath, which is a sentence that
                // sends an operator to watch a window that is still drawing at
                // full rate — the same false claim, in the same place, that
                // cost this file 270 frames once already.
                true =>
                    "the loop asks for the next frame from inside the last one for as long \
                     as anything is making texels, so `ControlFlow::Wait` never gets to \
                     block. Two things are: the picture, and the four cells in the preview \
                     row under it — all four of them, because every slot is drawn whatever \
                     its residency. Fold the picture away (f over it) and the cells keep \
                     the loop awake on their own; fold the preview row away as well \
                     and nothing is making texels — and the window still draws, because \
                     the beat grid declares a deadline of its own for as long as it is on \
                     screen (P-0094, ADR-0212) and the mixer bay declares another while \
                     deck B is parked. `ControlFlow::Wait` blocks when all three are gone, \
                     which is a fourth fold, and it is what ADR-0164's remaining clauses \
                     for.",
                false =>
                    "the loop is on `ControlFlow::Wait` from here: it does nothing at all \
                     until the window is touched or `egui` names a deadline of its own.",
            }
        );
        println!();
    }
}
