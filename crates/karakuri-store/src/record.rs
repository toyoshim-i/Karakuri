//! ndjson records.
//!
//! One record per line, and concatenation is composition. The same record types
//! serve two files:
//!
//! - a **Set file** (`.set.ndjson`) is a state projection — what is loaded and
//!   what every value currently is. It carries no time, so it never contains
//!   [`Record::Tick`].
//! - a **session stream** is a timeline — a Set file followed by ticks and the
//!   edits between them. Every edit lands at an exact frame position because it
//!   sits between two known ticks.
//!
//! A Set file is the session stream with the ticks dropped and the state folded
//! down. See the Set file and session stream sections of `docs/ir-spec.md`.

use serde::{Deserialize, Serialize};

use crate::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Layer {
    L1,
    L2,
    L3,
    L4,
}

/// A parameter value. Ranges are declared in the `.kir`; this is just the value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Scalar(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
}

/// The noise generator a `bind` declares for itself.
///
/// Every field has a default, and they are the defaults of the generator in
/// `karakuri-signal`: a `bind` that names `noise` and says nothing else gets
/// one cycle per beat of perlin on stream 0. A generator omitted field by
/// field is a generator that was under-specified, not one that was refused —
/// this is the one record whose parameters an LLM has to invent numbers for,
/// and every number here has a defensible one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BindNoise {
    /// `white`, `value`, `perlin`, or `fbm`. A `String` for the same reason
    /// `curve` is one: an unrecognised value is the engine's to diagnose
    /// against what it actually supports, not the decoder's to reject before
    /// anything can say what the alternatives were.
    #[serde(default = "default_noise_kind")]
    pub kind: String,
    /// Cycles per beat, so period is tempo-relative.
    #[serde(default = "default_noise_rate")]
    pub rate: f32,
    /// Decorrelates one binding from another. Two bindings sharing a stream
    /// move together.
    #[serde(default)]
    pub stream: u64,
    /// `fbm` only, ignored by the other three kinds. Not in v0.2 of the spec,
    /// which listed `fbm` as a `kind` while giving it nowhere to say how many
    /// octaves — the spec is corrected rather than the field being dropped,
    /// because an octave count baked into the engine is exactly the "fixed
    /// property nobody can reach" that Spawn timing rejects Poisson for.
    #[serde(default = "default_noise_octaves")]
    pub octaves: u32,
}

fn default_noise_kind() -> String {
    "perlin".to_string()
}

fn default_noise_rate() -> f32 {
    1.0
}

fn default_noise_octaves() -> u32 {
    4
}

impl Default for BindNoise {
    fn default() -> BindNoise {
        BindNoise {
            kind: default_noise_kind(),
            rate: default_noise_rate(),
            stream: 0,
            octaves: default_noise_octaves(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Record {
    Set {
        id: String,
        v: u32,
    },
    Slot {
        layer: Layer,
        #[serde(rename = "proc")]
        proc_hash: Hash,
    },
    /// Overrides the `.kir` default. Outside the range the artifact declares,
    /// the Set is rejected at build time.
    Capacity {
        layer: Layer,
        value: u32,
    },
    Param {
        layer: Layer,
        key: String,
        value: Value,
    },
    /// Attaches a signal to one `param` of one layer. The value written every
    /// frame is the signal put through `curve`, mapped onto `range`, and then
    /// blended against the param's own value by the sample's confidence — see
    /// "Set file format" in `docs/ir-spec.md`.
    Bind {
        layer: Layer,
        key: String,
        signal: String,
        curve: String,
        range: [f32; 2],
        /// Only a `signal` of `"noise"` reads this, because a noise generator
        /// is the one signal with parameters of its own. Absent means the
        /// default generator rather than no generator: there is nothing else
        /// for the name to mean.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        noise: Option<BindNoise>,
    },
    Camera {
        kind: String,
        radius: f32,
        speed: f32,
    },
    /// Salts the hash builtins for a layer, so re-seeding changes randomness
    /// without touching anything structural.
    Seed {
        stream: Layer,
        value: u64,
    },
    /// Inlined `.kir` source, for bundling an artifact with the Set that uses it.
    Src {
        hash: Hash,
        line: u32,
        s: String,
    },
    // -- The mix: durable state that belongs to the *session* -------------
    //
    // Everything above describes one Set and goes into a Set file. These seven
    // describe the deck the Sets are playing on, and a Set file must not
    // contain them — a Set does not know what fader it is under or whether it
    // is on air, and one that carried its gain would restore that gain
    // wherever it was next loaded. They are state all the same, which is what
    // separates them from the three below: `is_set_state` says no to all
    // twelve and means two different things by it.
    /// A deck slot's linear gain into the mix.
    ///
    /// The slot is an index into the deck rather than anything about the Set
    /// in it. Moving a Set to another slot moves it under another fader,
    /// which is what a fader is.
    Gain {
        slot: u8,
        value: f32,
    },
    /// A deck slot's fader: how much of its blend lands in the mix, `[0, 1]`.
    ///
    /// **Separate from [`Record::Gain`] because they are separate controls**,
    /// which only became visible once there was a blend mode that was not
    /// `add`. Gain is the level the material arrives at; opacity is how much of
    /// the blend takes effect, including how much the layer covers. Under `add`
    /// the two multiply together and a stream could have carried either — under
    /// `over` a session that replayed one as the other would replay a layer
    /// that hides as a layer that dims.
    Opacity {
        slot: u8,
        value: f32,
    },
    /// How a deck slot's layer meets the ones under it: `add`, `over` or `max`.
    ///
    /// Slot order is stacking order, so this is the one mix control whose
    /// meaning depends on where the slot sits — which is why it is recorded per
    /// slot rather than as a property of the Set in it. Moving a Set to another
    /// slot moves it to another place in the stack.
    ///
    /// A `String` on the same terms as `residency` and `sync`: what a mode is
    /// allowed to be is the engine's to say, and a stream from a newer build
    /// reaches the engine's diagnostic rather than the parser.
    Blend {
        slot: u8,
        mode: String,
    },
    /// What a deck slot is asked to do: `live`, `priming`, or `allocated`.
    ///
    /// **The request, never the effective level.** The governor recomputes the
    /// second every pass from the budget of the machine that is running, so a
    /// session recorded on a fast machine and replayed on a slow one must
    /// re-derive it rather than replay it — recording what was decided would
    /// replay one machine's budget onto another's. This is the same choice
    /// [`Record::Tempo`] makes in the other direction and for the opposite
    /// reason: there, what was decided is the reproducible thing.
    ///
    /// A `String` rather than an enum, on the same terms as `curve` and
    /// `noise.kind`: an unrecognised level is the engine's to diagnose against
    /// what it actually supports. This one has earned it — `docs/roadmap.md`
    /// planned five residency levels and three were built.
    Residency {
        slot: u8,
        level: String,
    },
    /// The output look: tone map operator, exposure, and the operator's white
    /// point. One record rather than three because it is one value in the
    /// engine, written to one uniform, and a stream that could set the
    /// exposure without saying which operator it applies to would be
    /// describing a look nobody can reconstruct.
    ///
    /// Session-wide and deliberately not per slot: tone mapping happens once,
    /// after the mix, which is the whole argument in `karakuri-engine`'s
    /// `present` module.
    Look {
        op: String,
        exposure: f32,
        white_point: f32,
    },
    /// **What size the session renders at**, in texels — the canvas every
    /// `VideoSource` draws into and every deck slot is sized to match.
    ///
    /// Not the size of any window. A window is a preview and is fitted to this
    /// rather than the other way round, so dragging one changes what an
    /// operator can see and nothing about what is drawn. Without this record
    /// the two were the same number: a session played in a small window and
    /// replayed with a large `--size` rendered different pixels, and nothing in
    /// the stream said which of them was the performance.
    ///
    /// **Written once, at the head, and a stream carries no second one.**
    /// Changing it reallocates every slot's target and the HDR target, which is
    /// an allocation on the render thread — the one thing this engine's frame
    /// path forbids. So the canvas is a property of a run rather than a control
    /// an operator moves during one, and this is the only record here that is
    /// session state without being something a hand can reach mid-set.
    Canvas {
        width: u32,
        height: u32,
    },
    /// **What shape of the frame a deck slot's layer reaches.**
    ///
    /// A mask multiplies the layer's opacity per texel, which is what makes it
    /// a mask rather than a second fader — everything opacity does, done to
    /// part of the frame. `position` is how far the front has travelled and is
    /// the number a `transition` moves, so **a wipe is this record plus a
    /// `transition` on `mask`** and needs nothing of its own.
    ///
    /// `kind` is a `String` on the same terms as `blend` and `residency`: what
    /// a shape is allowed to be is the engine's to say. `angle` is in radians
    /// and is the linear front's alone.
    Mask {
        slot: u8,
        /// `none`, `linear` or `radial`.
        kind: String,
        /// Which way a linear front runs, in radians.
        angle: f32,
        /// How far it has travelled, `[0, 1]`. 0 reveals nothing, 1 reveals
        /// everything — both exactly.
        position: f32,
        /// How wide the soft edge is, `[0, 1]`. 0 is a hard edge.
        softness: f32,
    },
    /// **A mix control moving over musical time**: a fade, a cut, or half of a
    /// crossfade.
    ///
    /// One record for the whole move, and **the values it produces are not
    /// recorded at all**. A value per frame would be 216,000 lines an hour
    /// describing something the grid already determines — the same argument
    /// `tick` makes, where the engine advances by a step count and everything
    /// downstream is a function of it.
    ///
    /// `start` is an absolute position on the session's beat count rather than
    /// "in two bars", because a relative instant is a different instant
    /// depending on when it is read and a beat count is the same one on every
    /// run. Quantising to the next bar happens where the operator asked, once.
    ///
    /// **`from` is deliberately absent**, and is read where the move is
    /// *scheduled* rather than where it starts. Capturing it at the start would
    /// mean capturing it on the first frame at or after a musical instant, and
    /// a machine running at a different rate would capture it at a different
    /// beat — which is the one property this record exists to have. Nothing can
    /// move the control in between: a hand cancels the move, another move
    /// replaces it.
    ///
    /// `control` and `curve` are strings on the same terms as `curve` on a
    /// `bind`: what a name is allowed to be is the engine's to say.
    Transition {
        slot: u8,
        /// `gain` or `opacity`.
        control: String,
        /// Where the control ends up.
        to: f32,
        /// The musical instant it begins, in beats.
        start: f64,
        /// How long it lasts, in beats. Zero is a cut.
        beats: f64,
        /// `lin`, `pow2`, `sqrt` or `smooth`.
        curve: String,
    },
    /// **Which slot is being auditioned**, or none of them for the mix.
    ///
    /// Not a mix control — it changes nothing about how the Sets are combined,
    /// only which of them the output is showing — but it is session state and it
    /// is in the stream for one reason: **today the preview is the output**.
    /// A replay that ignored it would show the mix where the operator was
    /// looking at one slot, which is replaying a different picture than the one
    /// that happened.
    ///
    /// That reason has an expiry date. When output routing gives the deck a
    /// second output, this becomes the monitor's choice and stops being the
    /// programme's, and the record stops belonging in a session stream — where
    /// `gain` and `blend` will still belong. Recorded here as a fact about what
    /// was shown, not as a claim that auditioning is part of a performance.
    ///
    /// `slot` is `None` for the mix rather than a sentinel index, so a deck of
    /// a different size cannot read one as the other.
    Preview {
        slot: Option<u8>,
    },
    /// **What a deck slot's clock does with the session's** — `free`, `tempo`
    /// or `beat`, with the two numbers that make the mode mean something.
    ///
    /// `anchor_bpm` is the tempo at which this material runs at 1x, and it has
    /// to be recorded because **material has no intrinsic tempo**: a `.kir`
    /// declares parameters and a capacity, not a bar length, so "one beat of
    /// music is how many seconds of material" is an operator's answer rather
    /// than the artifact's. `offset_beats` is the scrub — signed, unbounded,
    /// and the one value in this format that is meant to go backwards.
    ///
    /// Both are carried even under `free`, where neither does anything, so that
    /// a slot moved back onto the grid returns to where the operator left it
    /// rather than to a default.
    Transport {
        slot: u8,
        sync: String,
        anchor_bpm: f32,
        offset_beats: f64,
    },
    // -- What a frame saw or decided --------------------------------------
    /// How far this frame advances. Emitted from real time when live, read back
    /// verbatim on replay — which is what keeps substepping deterministic.
    /// Always 1 in v0.2, capped at [`MAX_STEPS`].
    Tick {
        steps: u8,
    },
    /// One frame's worth of **measured** signals, on the same terms as
    /// [`Record::Tick`]: derived from a device when live, read back verbatim on
    /// replay, and the engine cannot tell which happened.
    ///
    /// Live audio is not reproducible and the record stream is required to be,
    /// so the two can only both be true if the measurement joins the stream.
    /// This is that record, and its shape follows from what a frame is:
    ///
    /// - **One line per frame**, so it interleaves with `tick` and every edit
    ///   lands at an exact frame position, the way the session stream format
    ///   already promises.
    /// - **Named fields for named signals** and a positional array for the
    ///   bands, because `band0`…`bandN` *are* positions — an array is the
    ///   naming scheme rather than a second one. A map of name to value would
    ///   also encode which names it carries, at the cost of repeating those
    ///   names 200,000 times an hour and of letting a stream invent a name the
    ///   bus has rules about (`noise` is not a bus name, and a generic map is
    ///   where that rule would be broken).
    /// - **One confidence for the frame**, not one per signal: these values all
    ///   came out of the same block of samples at the same instant, so their
    ///   staleness is one number. A per-signal confidence would be four copies
    ///   of it.
    ///
    /// The array's length is the band count, so a stream carrying more bands
    /// than a reader knows about still decodes — a fixed-length array would
    /// make a band count a breaking format change, which is the opposite of
    /// what the unknown-`t` rule is for.
    Audio {
        /// Broadband level, `[0, 1]`.
        energy: f32,
        /// Transient envelope, `[0, 1]`: 1.0 at a detected onset, decaying from
        /// there. Not an impulse — an impulse one analysis block wide would be
        /// missed by some frames and seen twice by others.
        onset: f32,
        /// Per-band level, `[0, 1]`, low band first. Position is the name.
        bands: Vec<f32>,
        /// How much of this frame to believe, `[0, 1]`. Full while a device is
        /// open and delivering — **a silent room is 0.0 energy at confidence
        /// 1.0** — falling as the last block goes stale, and 0.0 when nothing
        /// has arrived, which leaves every bound parameter at its own value.
        confidence: f32,
    },
    /// What the local oscillator's tempo and phase are corrected to, this
    /// frame. The same terms as [`Record::Tick`] and [`Record::Audio`]: derived
    /// live, read back verbatim on replay.
    ///
    /// **v0.2 had no tempo record at all**, which `docs/roadmap.md` notes: the
    /// session tempo arrived by CLI flag and nothing in the stream could say
    /// what it was. This closes that, and it closes it with the *correction*
    /// rather than with the estimate, for a reason worth stating: an analyser
    /// is allowed to improve, and a session recorded today has to replay the
    /// same way after it does. Recording what was decided rather than what was
    /// heard is what makes that true. It is also why replay does not need the
    /// audio.
    ///
    /// The first one in a stream is what sets the session tempo, so this record
    /// is both "the tempo is now this" and "the tempo has moved a little"; a
    /// correction with `shift` 0.0 and `confidence` 0.0 is a free-running
    /// tempo being stated.
    Tempo {
        /// The tempo from now on. A tempo change does not move a beat that has
        /// already happened — see `Oscillator::correct`.
        bpm: f32,
        /// Phase shift in beats, positive meaning the next beat arrives sooner.
        /// Almost always tiny: the grid is predicted and trimmed, not chased.
        shift: f32,
        /// How much the estimate behind this correction was believed. Carried
        /// so a replay can show an operator what the live run showed, and
        /// because a correction that was applied at low confidence is a
        /// different event from the same numbers applied at high confidence.
        confidence: f32,
    },
    /// Forward compatibility: an unrecognised `t` is ignored, not an error.
    #[serde(other)]
    Unknown,
}

/// Beyond this the simulation is allowed to fall behind rather than catch up.
/// Unbounded catch-up turns a load spike into a death spiral.
pub const MAX_STEPS: u8 = 4;

impl Record {
    /// Whether this record belongs in a Set file. **Thirteen say no, for two
    /// different reasons, and keeping them apart is the point of the name** —
    /// it is `is_set_state` rather than `is_state` because most of what it
    /// refuses is state.
    ///
    /// - [`Record::Tick`], [`Record::Audio`] and [`Record::Tempo`] are not
    ///   state at all: they are what a *frame* saw or decided. A Set file
    ///   carries no time, and one holding an audio frame would be claiming a
    ///   particular moment's microphone reading is part of what a Set is.
    /// - [`Record::Gain`], [`Record::Opacity`], [`Record::Blend`],
    ///   [`Record::Residency`], [`Record::Look`], [`Record::Canvas`],
    ///   [`Record::Transport`],
    ///   [`Record::Preview`], [`Record::Mask`] and [`Record::Transition`] are
    ///   the **session's**
    ///   rather than any Set's. The last is the one that is not durable state
    ///   at all but an *event* — a move scheduled at an instant — and it is
    ///   here rather than beside `tick` because what it moves is the deck.
    ///   Folding a session down to a Set file drops it for both reasons at
    ///   once. A Set does
    ///   not know what fader it is under; one that carried its gain would
    ///   restore that gain wherever it was next loaded, which is a Set file
    ///   reaching outside the Set.
    ///
    ///   [`Record::Canvas`] is in this group for a reason worth stating apart:
    ///   a Set renders at whatever size it is given, and one that carried a
    ///   canvas would make loading it resize every *other* Set in the deck.
    ///
    /// A session stream carries all thirteen. That is the difference between
    /// the two files, stated from this side.
    pub fn is_set_state(&self) -> bool {
        !matches!(
            self,
            Record::Tick { .. }
                | Record::Audio { .. }
                | Record::Tempo { .. }
                | Record::Gain { .. }
                | Record::Opacity { .. }
                | Record::Blend { .. }
                | Record::Residency { .. }
                | Record::Look { .. }
                | Record::Canvas { .. }
                | Record::Transport { .. }
                | Record::Preview { .. }
                | Record::Transition { .. }
                | Record::Mask { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The wire line the spec documents, parsed and written back.** Every
    /// other record has one of these; without it a rename or a reordered field
    /// breaks every recorded session and nothing says so.
    #[test]
    fn a_transition_round_trips_through_the_line_the_spec_prints() {
        let line = r#"{"t":"transition","slot":0,"control":"opacity","to":0.0,"start":64.0,"beats":8.0,"curve":"smooth"}"#;
        let rec = round_trip(line);
        assert_eq!(
            rec,
            Record::Transition {
                slot: 0,
                control: "opacity".to_string(),
                to: 0.0,
                start: 64.0,
                beats: 8.0,
                curve: "smooth".to_string(),
            }
        );
        // And it is the session's rather than a Set's, for both reasons at
        // once: it is the deck's, and it is an event rather than state.
        assert!(!rec.is_set_state());
    }

    /// The wire line the spec prints, parsed and written back.
    #[test]
    fn a_mask_round_trips_through_the_line_the_spec_prints() {
        let line = r#"{"t":"mask","slot":1,"kind":"linear","angle":0.0,"position":0.5,"softness":0.1}"#;
        let rec = round_trip(line);
        assert_eq!(
            rec,
            Record::Mask {
                slot: 1,
                kind: "linear".to_string(),
                angle: 0.0,
                position: 0.5,
                softness: 0.1,
            }
        );
        assert!(!rec.is_set_state());
    }

    /// The wire line the spec prints, parsed and written back.
    #[test]
    fn a_canvas_round_trips_through_the_line_the_spec_prints() {
        let line = r#"{"t":"canvas","width":1920,"height":1080}"#;
        let rec = round_trip(line);
        assert_eq!(
            rec,
            Record::Canvas {
                width: 1920,
                height: 1080,
            }
        );
        // **The one that would be most tempting to put in a Set file**, and the
        // one it would do the most damage in: a Set renders at whatever size it
        // is handed, so a Set file carrying a canvas would resize every *other*
        // Set in the deck by being loaded.
        assert!(!rec.is_set_state());
    }

    fn round_trip(line: &str) -> Record {
        let rec: Record = serde_json::from_str(line).expect("parse");
        let back = serde_json::to_string(&rec).expect("serialise");
        let again: Record = serde_json::from_str(&back).expect("reparse");
        assert_eq!(rec, again);
        rec
    }

    #[test]
    fn tick_round_trips() {
        assert_eq!(round_trip(r#"{"t":"tick","steps":1}"#), Record::Tick { steps: 1 });
    }

    #[test]
    fn capacity_override_round_trips() {
        assert_eq!(
            round_trip(r#"{"t":"capacity","layer":"L1","value":524288}"#),
            Record::Capacity {
                layer: Layer::L1,
                value: 524288
            }
        );
    }

    #[test]
    fn unknown_records_are_ignored_not_rejected() {
        // Forward compatibility: a newer engine's record must not break an
        // older reader.
        assert_eq!(
            serde_json::from_str::<Record>(r#"{"t":"phrase","at":4.0}"#).unwrap(),
            Record::Unknown
        );
    }

    #[test]
    fn a_bind_without_a_noise_object_round_trips_and_stays_without_one() {
        // The field is absent rather than `null` on the way out: a Set file is
        // read by humans and generated by LLMs, and a `"noise":null` on every
        // ordinary binding teaches both that it is a thing to fill in.
        let rec = round_trip(
            r#"{"t":"bind","layer":"L1","key":"turbulence","signal":"energy","curve":"pow2","range":[0.1,2.4]}"#,
        );
        assert_eq!(
            rec,
            Record::Bind {
                layer: Layer::L1,
                key: "turbulence".into(),
                signal: "energy".into(),
                curve: "pow2".into(),
                range: [0.1, 2.4],
                noise: None,
            }
        );
        assert!(!serde_json::to_string(&rec).unwrap().contains("noise"));
    }

    #[test]
    fn a_noise_bind_round_trips_with_its_generator() {
        // The example out of `docs/ir-spec.md`'s "Binding noise", verbatim.
        let rec = round_trip(
            r#"{"t":"bind","layer":"L1","key":"spawn_rate","signal":"noise",
                "noise":{"kind":"perlin","rate":0.5,"stream":3},"curve":"lin","range":[4000,16000]}"#,
        );
        let Record::Bind { noise: Some(n), .. } = rec else {
            panic!("expected a bind carrying a noise generator");
        };
        assert_eq!(n.kind, "perlin");
        assert_eq!(n.rate, 0.5);
        assert_eq!(n.stream, 3);
        // Absent in the spec's own example, so it has to have a default or the
        // example does not decode.
        assert_eq!(n.octaves, 4);
    }

    #[test]
    fn an_empty_noise_object_is_the_default_generator() {
        // Field by field: a generator an LLM under-specified is one that runs,
        // not one that is refused. `{}` is the extreme case of that.
        let rec: Record = serde_json::from_str(
            r#"{"t":"bind","layer":"L1","key":"k","signal":"noise","curve":"lin","range":[0,1],"noise":{}}"#,
        )
        .expect("parse");
        let Record::Bind { noise: Some(n), .. } = rec else {
            panic!("expected a bind carrying a noise generator");
        };
        assert_eq!(n, BindNoise::default());
    }

    /// A decoded frame reproduces the values it was emitted with. That is the
    /// whole promise of putting the measurement in the stream: replay writes
    /// the same uniforms live did, so it has to be the same numbers.
    #[test]
    fn an_audio_frame_round_trips_with_every_value_it_carried() {
        let rec = round_trip(
            r#"{"t":"audio","energy":0.42,"onset":0.75,
                "bands":[0.9,0.4,0.2,0.11,0.05,0.02,0.01,0.0],"confidence":1.0}"#,
        );
        assert_eq!(
            rec,
            Record::Audio {
                energy: 0.42,
                onset: 0.75,
                bands: vec![0.9, 0.4, 0.2, 0.11, 0.05, 0.02, 0.01, 0.0],
                confidence: 1.0,
            }
        );
    }

    /// A silent room is a measurement and reads as one: zeroes at full
    /// confidence. A dead input is the same zeroes at no confidence. Two
    /// different lines, and a decoder that lost the difference would make an
    /// unplugged interface look like a quiet one.
    #[test]
    fn silence_and_absence_are_different_lines() {
        let silent = round_trip(
            r#"{"t":"audio","energy":0.0,"onset":0.0,"bands":[0.0,0.0],"confidence":1.0}"#,
        );
        let absent = round_trip(
            r#"{"t":"audio","energy":0.0,"onset":0.0,"bands":[0.0,0.0],"confidence":0.0}"#,
        );
        assert_ne!(silent, absent);
        let Record::Audio { confidence, .. } = silent else {
            panic!("expected an audio record");
        };
        assert_eq!(confidence, 1.0);
    }

    /// The band count is the array's length, so a stream from something that
    /// measures more bands than this reader knows about still decodes rather
    /// than failing — the same forward compatibility the unknown-`t` rule is
    /// for, one level down.
    #[test]
    fn a_band_count_this_reader_does_not_expect_still_decodes() {
        let rec: Record = serde_json::from_str(
            r#"{"t":"audio","energy":0.5,"onset":0.0,"bands":[0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,0.9,1.0,0.1,0.2],"confidence":1.0}"#,
        )
        .expect("parse");
        let Record::Audio { bands, .. } = rec else {
            panic!("expected an audio record");
        };
        assert_eq!(bands.len(), 12);
    }

    /// A correction round-trips exactly, because replay applies it verbatim: a
    /// shift that decoded to a different number would put the beat somewhere
    /// else than the live run did.
    #[test]
    fn a_tempo_correction_round_trips() {
        assert_eq!(
            round_trip(r#"{"t":"tempo","bpm":128.25,"shift":-0.0125,"confidence":0.82}"#),
            Record::Tempo {
                bpm: 128.25,
                shift: -0.0125,
                confidence: 0.82,
            }
        );
        // A free-running tempo being stated: no shift, no claim.
        assert_eq!(
            round_trip(r#"{"t":"tempo","bpm":120.0,"shift":0.0,"confidence":0.0}"#),
            Record::Tempo {
                bpm: 120.0,
                shift: 0.0,
                confidence: 0.0,
            }
        );
    }

    #[test]
    fn ticks_are_not_state() {
        assert!(!Record::Tick { steps: 1 }.is_set_state());
        // Nor is anything else a frame measured or decided.
        assert!(!Record::Audio {
            energy: 0.5,
            onset: 0.0,
            bands: vec![0.1],
            confidence: 1.0
        }
        .is_set_state());
        assert!(!Record::Tempo {
            bpm: 128.0,
            shift: 0.0,
            confidence: 0.9
        }
        .is_set_state());
        assert!(Record::Set {
            id: "drift_01".into(),
            v: 1
        }
        .is_set_state());
    }
}
