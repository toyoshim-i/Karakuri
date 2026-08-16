//! The Set: a grouping of nodes forming one video source, and the unit of both
//! compilation and lifecycle.
//!
//! **A Set does not own everything, and no longer pretends to.** `Ln` is a node
//! — `docs/roadmap.md`, "a Set stops owning everything" — and the unit that owns
//! GPU state is the node rather than the grouping. Both of the nodes there are
//! today live in [`crate::node`]: the **L1 node** owns the element and alive
//! buffers, the counts, the compaction scan, the spawn accumulator, its
//! pipelines and its bind groups; the **L4 node** owns its pipeline, its
//! uniform, its accumulation targets and the bind groups naming the buffers it
//! reads across.
//!
//! What is genuinely the grouping's is what is left here: the camera, the
//! parameter values and their bindings, the viewport, the clock, and the order
//! the nodes run in. **One clock and one camera serve every node in a Set**, so
//! a node holding its own copy would be a second place for them to be — which is
//! why `t` did not leave with the simulation that advances by it. A node is
//! handed the instants its work lands on ([`crate::node::Tick`],
//! [`crate::node::View`]) and derives none of its own.
//!
//! One node of each kind today, and a list is what several renderers over one
//! geometry will be. **Three places still reach into a node**, and each is a
//! decision about a *pair* rather than about either: [`Set::draw`] reads the
//! simulation's parity and counts (the per-frame half of the edge — see
//! [`crate::node`]), [`Set::step`] asks the renderer whether it is fullscreen
//! before running a simulation nothing would read, and [`Set::bind`] asks both
//! which params they declare. The first two are what a list changes: the
//! fullscreen skip becomes a question about *every* renderer, and the per-frame
//! edge stops wanting to be fetched once per reader.
//!
//! Nothing here mutates a live Set in place; parameter values are the one
//! exception, and they are uniform writes.
//!
//! **Nothing here is generated and nothing here creates a pipeline.** Both moved
//! out with the nodes; this module no longer calls `karakuri-codegen` at all
//! beyond naming an [`ElementLayout`] in a signature. What it still owns is the
//! two refusals that need both procedures in hand — a consumed attribute the L1
//! never emitted, and a param name declared on both sides.

use std::collections::HashMap;

use karakuri_codegen::layout::ElementLayout;
use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;

use crate::binding::{Binding, Signals};
use crate::camera::Orbit;
use crate::node::{Renderer, Simulation};
use crate::video_source::VideoSource;

/// Past this the simulation falls behind rather than catching up — the
/// ir-spec's cap, restated here because it is now load-bearing rather than
/// advisory: each substep needs its own spawn-count entry, and that array is
/// sized once, at build time.
pub const MAX_STEPS: u8 = 4;

/// The fixed simulation step. **Not** the real frame delta — see the
/// determinism invariant in `README.md`. Public because the session clock a
/// binding reads has to advance by exactly this: an oscillator on a different
/// step would drift away from the `t` the Sets are running at, and the drift
/// would be invisible until a beat landed in the wrong place.
pub const DT: f32 = 1.0 / 60.0;

#[derive(Debug, thiserror::Error)]
pub enum SetError {
    #[error("slot {slot} needs a {expected:?} procedure, got {actual:?}")]
    WrongKind {
        slot: &'static str,
        expected: Kind,
        actual: Kind,
    },
    #[error("capacity {requested} is outside the range [{min}, {max}] that `{proc}` declares")]
    Capacity {
        proc: String,
        requested: u32,
        min: u32,
        max: u32,
    },
    #[error("`{0}` declares no capacity range")]
    NoCapacity(String),
    /// `consumes ⊆ emit`, checked here because it is the first point where
    /// both procedures are in hand — a `.kir` declaring `consumes` alone is
    /// the normal shape of an L4 file, not an error, so no single-procedure
    /// pass can decide this. See the IR spec's validation pipeline, stage 6.
    #[error(
        "`{l4}` consumes {missing} which `{l1}` does not emit\n\
         hint: add {missing} to `{l1}`'s `emit`, or pair `{l4}` with an L1 that emits it \
         — there is no derivation step"
    )]
    Composition {
        l1: String,
        l4: String,
        missing: String,
    },
    /// A Set with no renderer.
    ///
    /// A `Set` is a [`VideoSource`], and a video source with nothing to draw has
    /// no frame to give. Reachable only through [`Set::build_many`] with an
    /// empty slice, which is a caller bug rather than an authoring mistake —
    /// but it is the sort of caller bug that arrives as an empty `--set` list,
    /// so it gets a sentence rather than a panic.
    #[error("a Set needs at least one L4 to draw `{l1}` with")]
    NoRenderer { l1: String },
    /// `blend weighted` on a procedure that draws the whole frame.
    ///
    /// **Refused because it is the identity, not because it is unbuilt.** A
    /// fullscreen L4 puts exactly one fragment on each texel, and for one layer
    /// the resolve gives back what the accumulation was made of:
    /// `(c * a * w) / (a * w) * (1 - (1 - a))` is `c * a`, whatever the weight
    /// was, which is precisely what additive blending into a cleared target
    /// leaves. So the two extra targets and the resolve pass buy an identical
    /// picture.
    ///
    /// **Identical for an alpha in `[0, 1]`**, which is the caveat and is the
    /// clamp. A fullscreen fragment writing `color = vec4(rgb, 1.5)` is legal
    /// under `additive` and comes out half again as bright as the weighted
    /// version would have been — so the hint below, which says to declare
    /// `additive`, is not always a picture-preserving swap. It is the right
    /// advice anyway: an alpha above 1 means nothing under a mode that reads it
    /// as opacity, so a procedure writing one is asking for `additive`.
    ///
    /// **It is a rule about the Set and not about the procedure**, which is why
    /// it lives here rather than in the checker: what makes it true is that a
    /// Set holds one L4. When several L4s can draw into one slot the sentence
    /// stops being true and this goes with it — and `generate_l4` lowers the
    /// combination perfectly well already, so there will be nothing else to
    /// change.
    #[error(
        "`{l4}` draws the whole frame, where `blend weighted` resolves to exactly what \
         `additive` accumulates\n\
         hint: one fragment per texel makes the weighted resolve the identity — it would \
         cost a revealage target and a resolve pass to reproduce the picture `blend \
         additive` gives for nothing. Declare `additive`"
    )]
    WeightedFullscreen { l4: String },
    /// A build panicked rather than returning. Not reachable through any
    /// `.kir` a checker accepts, which is exactly why it needs a variant:
    /// wgpu's default handler for an uncaptured validation error is a panic,
    /// so generated WGSL that naga refuses kills whatever thread built it.
    /// On the swap worker that is silent — the render thread keeps running
    /// and simply never receives anything again. A rejection says so.
    #[error("building `{label}` panicked, which is a bug in this compiler rather than in the `.kir`: {detail}")]
    Panicked { label: String, detail: String },
}

/// Which clock a binding's oscillator signals are read on. Private: the choice
/// belongs to [`Set::prepare`] and [`Set::prepare_warming`], which name the two
/// situations it distinguishes, and a caller picking a clock directly would be
/// picking one without the situation that justifies it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Clock {
    /// The session's, as handed in. **What a slot on air reads**: it is in the
    /// room, and the room's beat is the session's however far behind the
    /// slot's own clock has fallen.
    Session,
    /// The same grid, read at this Set's own `t`. **What a slot warming off
    /// air reads** — see [`Set::prepare_warming`] for the whole argument.
    Local,
}

pub struct Set {
    seed_salt: u32,
    /// Simulation steps elapsed. Time is `steps_taken * dt`, computed on
    /// demand rather than accumulated: a running `t += dt * steps` sum drifts
    /// by an ULP or two depending on how the steps were grouped, so twenty
    /// steps taken one at a time would land at a different `t` from ten taken
    /// in pairs. Two tick histories reaching the same elapsed time have to be
    /// the same point in the session, and a float sum is not that function.
    steps_taken: u64,
    dt: f32,
    /// The `beats` the last [`Set::prepare`] wrote into the L4 uniform block.
    ///
    /// Kept so that [`Set::refresh_view`] can rewrite that block for a slot
    /// nothing is preparing without moving the grid position it was drawn at.
    /// Derived, never authoritative: the oscillator is the grid, and this is
    /// the answer it gave at this Set's `t`.
    last_beats: f32,
    viewport: [f32; 2],
    /// Both procedures are a pure function of `seed`, `t`, and their params —
    /// see [`Set::is_closed_form`]. Decided by the check pass and carried here
    /// rather than re-derived; the engine never looks at IR.
    closed_form: bool,
    /// Whether either procedure reads the `beats` ambient — see
    /// [`Set::reads_beats`].
    reads_beats: bool,

    /// **The L1 node.** Every buffer, pipeline and bind group the simulation
    /// needs, and the spawn accumulator that decides what it creates. What
    /// crosses from it to the renderer below is a [`Geometry`](crate::node::Geometry)
    /// resolved once at build time, plus a parity and a counts buffer this
    /// module fetches every frame — the half of that edge that has no type yet.
    sim: Simulation,
    /// **The L4 nodes, in draw order.** Each owns its pipeline, its uniform, its
    /// accumulation targets under `blend weighted`, and the bind groups naming
    /// the element buffers the node above holds — the edge, resolved.
    ///
    /// **Several of them is overdraw, not compositing.** They run in order over
    /// the one attachment, the first clearing it and the rest loading what is
    /// there, so a stack of five costs one target rather than five. A target
    /// apiece is what an L5 is for. Never empty: a Set with nothing to draw is
    /// refused at build.
    renderers: Vec<Renderer>,

    /// **Manual** parameter values: the `.kir` defaults, as moved by a `param`
    /// record or a `--param` override. A binding never writes here — it blends
    /// *from* here — so a param that is both bound and set by hand has one
    /// answer rather than a race between two writers. See [`Set::bind`].
    ///
    /// **One map per node**, `[0]` the L1's and the rest the renderers' in
    /// order — see [`Set::slot_of`]. It was one flat map keyed by name across
    /// the whole Set, which made two procedures declaring `exposure` into one
    /// value and was refused at build time by a `ParamCollision` error rather
    /// than resolved. Every L4 in `examples/` declares `exposure`, so that
    /// refusal is exactly what forbade several renderers over one geometry;
    /// keying by the node that declares the name is what the roadmap named as
    /// the fix, and the error is gone with it.
    params: Vec<HashMap<String, f32>>,
    pub camera: Orbit,
    /// At most one per (layer, param). Resolved once per frame in
    /// [`Set::prepare`] and read back out wherever a param value is written.
    bindings: Vec<Binding>,
}

impl Set {
    /// Compile two checked procedures into a runnable Set.
    ///
    /// `capacity` is a Set-level dial, not part of either procedure's identity,
    /// so it is passed in here and validated against the range the L1 artifact
    /// declares rather than read out of it.
    ///
    /// There is no target-format parameter: every `VideoSource` renders
    /// `Rgba16Float`, and the conversion to whatever the display wants happens
    /// once, in the present pass.
    pub fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        l1: &Checked,
        l4: &Checked,
        capacity: u32,
        seed_salt: u32,
    ) -> Result<Set, SetError> {
        Set::build_many(device, queue, l1, &[l4], capacity, seed_salt)
    }

    /// **One geometry, several renderers over it, drawn in list order.**
    ///
    /// The payoff the primitive-centric bet was made for: `drift_shell` drawn as
    /// sprites *and* as streaks *and* as a solid is one simulation and three
    /// draw passes, where it used to be three simulations. Nothing about a
    /// renderer changes to be in a list — it was already a node owning
    /// everything it needs and reading the geometry across a typed edge.
    ///
    /// **List order is draw order, and it is overdraw.** The passes run over one
    /// attachment: the first clears it, the rest load what is there, and each
    /// blend mode already knows how to meet what is under it. So a stack costs
    /// one render target however long it is. Compositing — a target apiece, with
    /// gain and opacity and a blend per layer — is what an L5 is for, and the
    /// two are different operations rather than one with a dial.
    ///
    /// `l4s` must not be empty. A Set is a video source and a video source with
    /// nothing to draw has no frame to give.
    pub fn build_many(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        l1: &Checked,
        l4s: &[&Checked],
        capacity: u32,
        seed_salt: u32,
    ) -> Result<Set, SetError> {
        if l4s.is_empty() {
            return Err(SetError::NoRenderer { l1: l1.name.clone() });
        }
        if l1.kind != Kind::L1 {
            return Err(SetError::WrongKind {
                slot: "L1",
                expected: Kind::L1,
                actual: l1.kind,
            });
        }
        for l4 in l4s {
            if l4.kind != Kind::L4 {
                return Err(SetError::WrongKind {
                    slot: "L4",
                    expected: Kind::L4,
                    actual: l4.kind,
                });
            }
            // Before anything is generated: an L4 reads the element struct an L1
            // wrote, so a consumed attribute the L1 never emitted has no field to
            // read. Left unchecked it surfaces as a WGSL parse failure inside
            // `create_shader_module` — an internal error where the contract calls
            // for a diagnostic. Every missing attribute is reported at once, for
            // the same reason the IR checker reports every error at once: one
            // regeneration should be able to fix all of them.
            //
            // Per renderer, and the *first* one that fails stops the build: a
            // stack whose third node consumes `velocity` is as unbuildable as a
            // lone node that does, and reporting the rest would be reporting
            // them against a Set that will not exist either way.
            let missing: Vec<&str> = l4
                .consumes
                .iter()
                .filter(|a| !l1.emit.contains(a))
                .map(|a| a.name())
                .collect();
            if !missing.is_empty() {
                return Err(SetError::Composition {
                    l1: l1.name.clone(),
                    l4: l4.name.clone(),
                    missing: missing
                        .iter()
                        .map(|a| format!("`{a}`"))
                        .collect::<Vec<_>>()
                        .join(", "),
                });
            }
        }

        // **There is deliberately no third check, comparing the two
        // topologies.** An L1 declares one and an L4 now carries an inferred
        // one, so the comparison is available and looks principled — and it
        // would refuse the pairing M3 exists to enable: the same cloud drawn
        // as sprites by one L4 and as streaks by another. A segment under
        // `Topology::Lines` gets both of its ends from attributes the L4
        // consumes, so a renderer needs nothing from the geometry beyond what
        // the composition check above already verifies. The declaration on
        // the L1 side says what the geometry is *meant to read as*; it
        // constrains no renderer, and requiring the two to agree would invent
        // a dependency the lowering does not have.

        // **The rule that needs to know how many renderers there are.** A
        // fullscreen L4 puts one fragment on each texel, so a weighted resolve
        // of it alone reproduces exactly what `additive` accumulates into a
        // cleared target — the two extra attachments and the resolve pass buy an
        // identical picture. That stops being true the moment something else is
        // drawing into the same target, because then the resolve composites
        // `over` what is under it rather than replacing a clear. So it is
        // refused for a lone renderer and allowed in a stack, and the rule is
        // about the *count* rather than about the position: making it depend on
        // which slot the node sits in would be a refusal an author trips over by
        // reordering.
        if let [only] = l4s {
            if only.blend == Some(karakuri_ir::Blend::Weighted)
                && only.topology == Some(karakuri_ir::Topology::Fullscreen)
            {
                return Err(SetError::WeightedFullscreen { l4: only.name.clone() });
            }
        }

        // **The L1 node**, generated, compiled and allocated at `capacity` —
        // which is where the range check lives, because the range is that
        // node's own. Everything the simulation needs is inside it.
        let sim = Simulation::build(device, l1, capacity, seed_salt)?;

        // **The L4 nodes**, generated, compiled and bound against the edge the
        // node above offers: the element layout, and the two buffers indexed by
        // parity. Everything about how one draws is its own — see
        // [`crate::node::Renderer`] — including the blend-mode rule that needs
        // both halves in hand. They all read the same edge, which is the whole
        // point: one simulation, several ways of looking at it.
        let renderers: Vec<Renderer> = l4s
            .iter()
            .map(|l4| Renderer::build(device, l4, &sim.geometry()))
            .collect();

        // One map per node, in the order [`Set::slot_of`] addresses them: the
        // L1's, then each renderer's. Two nodes declaring one name now hold two
        // values, which is what a name meaning "this node's" buys.
        let declared = |p: &&karakuri_ir::Param| default_scalar(p).map(|v| (p.name.clone(), v));
        let params = std::iter::once(l1.params.iter().filter_map(|p| declared(&p)).collect())
            .chain(
                l4s.iter()
                    .map(|l4| l4.params.iter().filter_map(|p| declared(&p)).collect()),
            )
            .collect();

        let set = Set {
            seed_salt,
            steps_taken: 0,
            dt: DT,
            last_beats: 0.0,
            viewport: [1.0, 1.0],
            // Both, because a Set is only seekable if everything in it is. L4
            // is stateless and its flag is vacuously true, so in practice this
            // is the L1's — and it will still be when L2 arrives, since an L2 is
            // stateless by rule (`docs/ir-spec.md`, "L2 and L3"). The
            // conjunction is written out anyway: it is the sentence that is
            // true, and a Set whose seekability came from one named layer would
            // have to be revisited by every layer added after it.
            closed_form: l1.closed_form && l4s.iter().all(|l4| l4.closed_form),
            reads_beats: l1.reads_beats || l4s.iter().any(|l4| l4.reads_beats),
            sim,
            renderers,
            params,
            camera: Orbit::default(),
            bindings: Vec::new(),
        };
        set.sim.initialize(queue);
        Ok(set)
    }

    /// **Takes a device because a Set can own render targets.** Under `blend
    /// weighted` it holds two of them and they are the size of the frame, so a
    /// resize is a reallocation — the same shape as [`Present::resize`] and
    /// [`Deck::resize`](crate::deck::Deck::resize), which is what every other
    /// owner of a target in this engine already does. Under `additive` the
    /// device is unused and this is the one-line assignment it always was.
    ///
    /// Never from the render thread mid-frame, on those same terms.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.viewport = [width.max(1) as f32, height.max(1) as f32];
        for renderer in &mut self.renderers {
            renderer.resize(device, width, height);
        }
    }

    /// What [`Set::resize`] last set, as it was clamped. The camera's aspect
    /// ratio comes off this, so a caller that resizes a Set temporarily — the
    /// probe does, to a fixed reference size — has somewhere to read the old
    /// value back from rather than having to remember it.
    pub fn viewport(&self) -> (u32, u32) {
        (self.viewport[0] as u32, self.viewport[1] as u32)
    }

    pub fn time(&self) -> f32 {
        self.t_at(self.steps_taken)
    }

    /// Simulation time after `n` steps. The one place `t` is derived, so
    /// there is exactly one function from a step count to an instant.
    fn t_at(&self, n: u64) -> f32 {
        n as f32 * self.dt
    }

    /// How many elements the L1 node's current buffer holds — the draw's
    /// instance count, and the range the next step will scan. Not quite the
    /// alive count: an element killed during the step that just ran still
    /// occupies its slot until the next step's scan reclaims it.
    ///
    /// **This is a stall.** It copies four bytes off the GPU and blocks until
    /// the queue drains to read them, which is exactly what indirect dispatch
    /// exists to avoid. It is here for tests and for a status line printed once
    /// at the end of a run; **never call it on the frame path.** The alternative
    /// — tracking an estimate host-side — would be worse: a number that is
    /// usually right is harder to distrust than one that is honestly expensive.
    pub fn live_count(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> u32 {
        self.sim.live_count(device, queue)
    }

    /// The raw bytes of the element buffer the renderer is currently reading,
    /// decoded against [`Set::element_layout`]. **A stall, on the same terms as
    /// [`Set::live_count`]** — this exists so a test can check that survivors
    /// kept their order, which is a claim about `seed` values in slots and
    /// cannot be made from a rendered image.
    pub fn read_elements(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        self.sim.read_elements(device, queue)
    }

    pub fn element_layout(&self) -> &ElementLayout {
        self.sim.element_layout()
    }

    pub fn capacity(&self) -> u32 {
        self.sim.capacity()
    }

    /// **Whether this Set's state at any `t` is reachable by evaluating it
    /// rather than by running forward to it.**
    ///
    /// True when both procedures are a pure function of `seed`, `t`, and their
    /// parameters. Two consequences: it can be taken from Cold to Live with no
    /// warm-up, and it can be scrubbed — forwards at any rate, held, or
    /// backwards. Decided by the check pass (see `Checked::closed_form`) and
    /// deliberately conservative: a `true` here is a promise that skipping the
    /// warm-up shows the same image warming would have, and a `false` may be
    /// pessimistic.
    ///
    /// Two consumers: [`crate::governor`], where a closed-form Set has nothing
    /// to prime and is never worth compute budget; and [`crate::transport`],
    /// where it is what makes beat sync possible at all — a position lock has
    /// to be able to land on a position.
    pub fn is_closed_form(&self) -> bool {
        self.closed_form
    }

    /// **Whether this Set's material is written against the tempo grid** —
    /// either procedure reads the `beats` ambient.
    ///
    /// Such material already follows the room, so a transport that also scaled
    /// its clock by the tempo would make it follow twice and run at roughly the
    /// square of the tempo ratio. [`crate::transport`] refuses that combination
    /// rather than offering it; see `Checked::reads_beats`.
    pub fn reads_beats(&self) -> bool {
        self.reads_beats
    }

    /// **Put the simulation clock at `steps_taken` without running anything.**
    ///
    /// The seek half of a transport. What follows must be exactly one
    /// [`Set::prepare`] of one step and one [`Set::render`] of one step: this
    /// leaves the counter one short of the target, `prepare` bumps it onto the
    /// target and writes the uniforms for that instant, and the single element
    /// pass evaluates the procedure there. One pass is enough because the
    /// caller has promised the procedure is closed form, which is exactly the
    /// promise that its state at `t` does not depend on how it got there.
    ///
    /// **Only for a closed-form Set**, and this does not check, because it
    /// cannot usefully: the Set knows ([`Set::is_closed_form`]) but the
    /// alternative to a caller that checks is a caller that gets a silent
    /// wrong answer either way — an accumulating Set seeked to `t` evaluates
    /// once from wherever it happened to be, which is garbage rather than an
    /// error. [`crate::transport`] is the one caller and refuses beat sync on
    /// accumulating material at the point the operator asks for it, where there
    /// is something to say.
    ///
    /// The element buffers are left alone. They hold the previous instant's
    /// values, which a closed-form `element` block does not read.
    pub fn seek(&mut self, steps_taken: u64) {
        self.steps_taken = steps_taken;
    }

    /// Put this Set back to exactly what [`Set::build`] left: element and alive
    /// buffers at their initial contents, `t` at zero, parity, the spawn
    /// accumulator and the seed counter all reset.
    ///
    /// **Not for the frame path and not a lifecycle operation.** It exists for
    /// one caller: `swap.rs` measures a freshly built Set with the probe before
    /// handing it to the render thread, and measuring means stepping it. A
    /// swapped-in Set is documented as arriving cold, so the measurement has to
    /// leave no trace — this is what makes that true rather than nearly true.
    ///
    /// It re-uploads the whole element and alive buffers, so it is as expensive
    /// as `build`'s own upload and belongs on the worker thread beside it.
    ///
    /// **Takes a device it does not use**, like [`Set::resize`] and for the
    /// opposite reason: nothing a Set owns needs reallocating to be put back,
    /// and the parameter is kept because a caller holding one anyway is a
    /// cheaper contract than one that has to find out whether this is the
    /// version that needs it.
    pub fn rewind(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue) {
        self.steps_taken = 0;
        self.sim.rewind(queue);
    }

    /// Attach a signal to a `param`. Returns `false` if `binding.layer`
    /// declares no scalar `param` of that name, which is the same non-fatal
    /// shape a `--param` for an unknown name has: a Set file naming a param a
    /// regenerated artifact no longer has should not take the show down.
    ///
    /// **At most one binding per (layer, param)**, so a second one replaces
    /// the first rather than stacking behind it. Two bindings on one param
    /// would be resolved in vector order and the winner would be whichever was
    /// attached last — "the last writer wins", which is exactly the answer
    /// this design refuses everywhere else.
    ///
    /// Allocates, so not on the render thread. A binding arrives with a Set
    /// (from a Set file, from `--bind`, or from a rebuild's `Request`), and
    /// all three are off the frame path.
    pub fn bind(&mut self, binding: Binding) -> bool {
        // Both checks: a node's map holds only its scalar params — a vector one
        // is declared but has no value here — and a binding produces one float.
        //
        // **Any node of that layer will do.** A binding names a layer, so it
        // means the same as a bare `--param` does: every node of that layer
        // declaring the name. One of them declaring it is enough for the
        // binding to have somewhere to land.
        let declares = |names: &[String], map: &HashMap<String, f32>| {
            names.contains(&binding.key) && map.contains_key(&binding.key)
        };
        // **Only the nodes this binding covers.** A wildcard needs one of them
        // to declare the name; an addressed one needs *that* node to, so
        // `bind(L4, index 2, "exposure")` on a Set of two renderers is refused
        // rather than attached to nothing.
        let found = match binding.layer {
            Kind::L1 => binding.covers(0) && declares(self.sim.param_names(), &self.params[0]),
            Kind::L4 => self
                .renderers
                .iter()
                .zip(&self.params[1..])
                .enumerate()
                .any(|(at, (r, map))| binding.covers(at) && declares(r.param_names(), map)),
        };
        if !found {
            return false;
        }
        // At most one per (layer, index, param). A wildcard and an addressed
        // binding on one name are two bindings and the addressed one wins for
        // the node it names, because `effective` takes the first match and an
        // addressed binding is pushed later — which is the same "the last one
        // attached wins" rule a repeated binding already follows, applied to a
        // narrower target.
        self.bindings
            .retain(|b| b.layer != binding.layer || b.key != binding.key || b.index != binding.index);
        self.bindings.push(binding);
        true
    }

    /// **The node a bare layer address resolves to**: the L1, or the *first*
    /// renderer.
    ///
    /// A binding names a layer and not a node, so this is where its blend base
    /// comes from — and the value it produces is then written to every renderer
    /// declaring the name, exactly as [`Set::set_param`] writes every
    /// declaration. Which renderer's manual value is the base only matters when
    /// two of them declare one name at different values, and setting them apart
    /// needs the address the record vocabulary still owes: `layer` plus an
    /// `index` defaulting to 0. See `docs/roadmap.md`, "How a param is
    /// addressed".
    fn slot_of(layer: Kind) -> usize {
        match layer {
            Kind::L1 => 0,
            Kind::L4 => 1,
        }
    }

    /// Every map in [`Set::params`] belonging to `layer`, in node order. One for
    /// the L1; one per renderer for L4.
    fn nodes_of(layer: Kind) -> std::ops::Range<usize> {
        match layer {
            Kind::L1 => 0..1,
            Kind::L4 => 1..usize::MAX,
        }
    }

    /// **Set every declaration of `name`, and say how many there were.**
    ///
    /// Zero means nothing in this Set declares it, which is the caller's cue to
    /// say so — a `--param` for a name a regenerated artifact no longer has
    /// should not take the show down.
    ///
    /// A name rather than an address, because that is what a `--param` and a
    /// `param` record carry, and because "the Set's `exposure`" is the useful
    /// default when two nodes both have one: one knob moves both. Addressing a
    /// single node is what the record vocabulary will need when someone wants
    /// them apart — `docs/roadmap.md`, "How a param is addressed" — and this is
    /// deliberately not that.
    pub fn set_param(&mut self, name: &str, value: f32) -> usize {
        let mut written = 0;
        for node in &mut self.params {
            if let Some(slot) = node.get_mut(name) {
                *slot = value;
                written += 1;
            }
        }
        written
    }

    /// **Set one node's declaration of `name`.** `false` if that node does not
    /// exist or does not declare it.
    ///
    /// The addressed form of [`Set::set_param`], and the one that can set two
    /// renderers' `exposure` apart — a bare name reaches every declaration and
    /// therefore cannot. `index` is which node of `layer`; the L1 is one node,
    /// so only 0 addresses it.
    pub fn set_param_at(&mut self, layer: Kind, index: u32, name: &str, value: f32) -> bool {
        let slot = Self::slot_of(layer) + index as usize;
        match self.params.get_mut(slot).and_then(|n| n.get_mut(name)) {
            Some(held) => {
                *held = value;
                true
            }
            None => false,
        }
    }

    /// What `name` currently holds, from the first node that declares it.
    ///
    /// Enough while [`Set::set_param`] writes every declaration together, so
    /// the first is the only value there is. It stops being enough the moment
    /// an addressed write lands, which is why nothing in the engine builds on
    /// it — it exists for tests and for a status line.
    pub fn param(&self, name: &str) -> Option<f32> {
        self.params.iter().find_map(|node| node.get(name).copied())
    }

    /// Every parameter value, addressed by the node that declares it: the
    /// layer, which node of that layer, the name, and the value.
    pub fn params(&self) -> impl Iterator<Item = (Kind, u32, &str, f32)> + '_ {
        self.params.iter().enumerate().flat_map(|(slot, node)| {
            let (layer, index) = match slot {
                0 => (Kind::L1, 0),
                n => (Kind::L4, n as u32 - 1),
            };
            node.iter().map(move |(k, v)| (layer, index, k.as_str(), *v))
        })
    }

    /// Every bound param and what it was last written with. For a status line:
    /// a binding that is doing nothing and a binding that is not there look
    /// identical from outside otherwise.
    pub fn bound(&self) -> impl Iterator<Item = (&str, f32)> {
        self.bindings.iter().map(|b| (b.key.as_str(), b.value()))
    }

    /// The bindings themselves, for a caller that has to carry them across a
    /// rebuild.
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// Uploads uniforms and advances simulation time. A parameter change is a
    /// uniform write, which is why it does not need a fork.
    ///
    /// Also quantizes this frame's spawning. `steps` is clamped to
    /// [`MAX_STEPS`] here and in [`VideoSource::render`] alike: past that the
    /// simulation is allowed to fall behind rather than catch up, and the
    /// two have to agree or `t` would advance further than the element
    /// passes did.
    ///
    /// `signals` is the **session's** oscillator and seed, one per deck rather
    /// than one per Set, and it has already been advanced by this frame's
    /// `steps` when this is called — so a binding reads the phase at the
    /// instant of the frame's last substep. Every binding is resolved once,
    /// here, and the value is reused wherever that param is written; resolving
    /// twice in one frame would put two different values into one frame.
    ///
    /// **This is the on-air form**, and it reads the session's position on the
    /// grid. A Set warming off air is behind that position and wants
    /// [`Set::prepare_warming`], which is this function with one difference.
    ///
    /// **Nothing in here allocates.** Both uniform writes go through storage
    /// sized at build time (`crate::uniforms::UniformScratch`) and the step
    /// arguments through a stack array, because this is the render thread and
    /// the first invariant in `README.md` is the one about allocating on it.
    /// Binding resolution is the same: a fixed `Vec` written in place, a
    /// stack-sized bus over a borrowed oscillator, and a linear scan to read
    /// values back out.
    pub fn prepare(&mut self, queue: &wgpu::Queue, steps: u8, signals: &Signals) {
        self.prepare_on(queue, steps, signals, Clock::Session);
    }

    /// [`Set::prepare`] for a Set that is **warming off air**: identical in
    /// every respect but one — oscillator signals are read on this Set's own
    /// clock instead of the session's.
    ///
    /// The difference only exists because a warming slot's clock is behind the
    /// room's. It steps on some frames and not others, so `steps_taken * dt`
    /// falls further behind the session's `t` the slower it is warmed, and
    /// handing it the session's phase would make a binding advance by a whole
    /// frame's worth of beats for every step the slot actually takes. **The
    /// governor picks that rate.** A Set warmed at one step in four would then
    /// warm into different material than the same Set warmed at full rate — a
    /// performance knob, invisible to the operator, silently changing the
    /// picture. Reading the grid at the slot's own `t` removes the rate from
    /// the arithmetic entirely.
    ///
    /// **The lag is a step count, so a slot that is not behind reads exactly
    /// what it would have read on air.** A slot warming at full rate takes a
    /// step whenever the session does, its lag is zero, and
    /// [`Signals::behind`] hands back the session's own oscillator bit for bit
    /// — which is what makes "primed then Live" *identical* to "always Live"
    /// rather than close to it, for bound material as well as unbound. Deriving
    /// a position from this Set's `t` instead would cost an f32 rounding the
    /// session's accumulated `t` never took, and the identity would hold to
    /// about seven digits: it diverges within a second at 120 bpm, and every
    /// step after that reads a different value.
    ///
    /// Two residues, both real and neither fixable here:
    ///
    /// - **Tempo corrections.** Warming slower spans more wall time and
    ///   therefore more corrections, and [`Oscillator::behind`] gives the grid
    ///   as it stands rather than as it was. Invariance holds against a steady
    ///   tempo, not across a change of one.
    /// - **Measured audio.** `energy` and the bands are this frame's
    ///   measurement at every rate, because there is no other measurement to
    ///   give. A Set bound to audio warms into whatever the room was doing while
    ///   it warmed. *Synthesized* `energy` — what the bus invents when nothing
    ///   is measuring — does move with the clock, because it is a function of
    ///   it; [`Signals::behind`] has the split.
    ///
    /// Going on air moves the slot back to the session's grid, and normally
    /// nothing sees the discontinuity that causes: the frame before was not
    /// drawn. That is the whole reason the split is safe — **off air is not in
    /// the room**, and a slot on air must be on the room's beat however far
    /// behind its own clock is.
    ///
    /// **An audition is the case where the frame before *is* drawn**, and it is
    /// the one place that discontinuity is visible: a warming slot is shown at
    /// its own grid position, so material that reads `beats` or carries a
    /// binding moves the moment it goes on air, by however far behind the
    /// governor's rate had it. Named rather than closed, because the close is
    /// to read the session's grid instead — which is the defect this split cost
    /// a repair to fix. See
    /// [`Deck::set_preview`](crate::deck::Deck::set_preview).
    pub fn prepare_warming(&mut self, queue: &wgpu::Queue, steps: u8, signals: &Signals) {
        self.prepare_on(queue, steps, signals, Clock::Local);
    }

    /// The one body. Both entry points come through here so that "identical in
    /// every respect but one" is structural rather than a claim two functions
    /// have to keep making about each other.
    fn prepare_on(&mut self, queue: &wgpu::Queue, steps: u8, signals: &Signals, clock: Clock) {
        let steps = steps.min(MAX_STEPS);
        self.steps_taken += u64::from(steps);
        // After the bump, so `Clock::Local` measures the lag as of *this*
        // frame's last substep — the instant `Clock::Session` reads, because
        // the deck advances the session's oscillator before it prepares
        // anything. Reading before it would put every warming binding a frame
        // early.
        //
        // **One grid position for this Set, this frame**, and everything that
        // reads the grid reads it: the bindings below and the `beats` every
        // substep is given. Two lookups could not disagree even in principle,
        // but computing it once is what makes that true by construction rather
        // than by two call sites happening to pass the same argument.
        let view = match clock {
            Clock::Session => *signals,
            // **Subtract step counts, not times.** Both clocks are integer
            // counters of the same `dt`, so their difference is exact and is
            // zero whenever they agree; two `t`s derived from them are not
            // exact and their difference is not zero. `saturating_sub` because
            // a Set may have taken more steps than the session's oscillator —
            // a Set built and stepped before it was ever put on a deck — and
            // that is a slot ahead of the room, which reads the room's phase
            // rather than an extrapolated future one.
            Clock::Local => {
                let lag = signals
                    .oscillator()
                    .steps_taken()
                    .saturating_sub(self.steps_taken);
                signals.behind(lag as f64 * f64::from(self.dt))
            }
        };
        self.resolve_bindings(&view);

        // **The instants this frame's substeps land on, derived here because
        // the clock is the grouping's.** Substep `k` is step number `first + k`
        // of the session, and its `t` is that number's instant; `steps_taken`
        // has already been advanced past this frame, so count back from it —
        // deriving both ends from the same counter is what makes a frame of two
        // steps land on the same two instants two frames of one step do.
        //
        // `beats` is defined as the grid at the instant `t` names, so it is
        // derived from that `t` rather than from a position counted back from
        // the session's — which would be the same number and a different claim.
        // See `Oscillator::at_time` for why "the same number" is a measured fact
        // here rather than a hopeful one.
        let first = self.steps_taken - u64::from(steps) + 1;
        let mut instants = [(0.0f32, 0.0f32); MAX_STEPS as usize];
        for (k, slot) in instants.iter_mut().enumerate().take(usize::from(steps)) {
            let t = self.t_at(first + k as u64);
            *slot = (t, view.oscillator().at_time(f64::from(t)).beats() as f32);
        }

        {
            let (bindings, params) = (&self.bindings, &self.params[Self::slot_of(Kind::L1)]);
            let param = |name: &str| effective(bindings, params, Kind::L1, 0, name);
            let tick = crate::node::Tick { steps, dt: self.dt, instants, param: &param };
            self.sim.prepare(queue, &tick);
        }

        // The grid at exactly this frame's `t`, on the same terms as the
        // per-substep `beats` above: one instant, named twice, derived once.
        // Kept, because [`Set::refresh_view`] has to be able to rewrite this
        // block without moving it.
        self.last_beats = view.oscillator().at_time(f64::from(self.time())).beats() as f32;
        self.write_l4_uniforms(queue);
    }

    /// The L4 node's uniform block, from state this does not change.
    ///
    /// Split out of [`Set::prepare_on`] because a preview needs it without the
    /// rest: an audition draws a slot nothing prepared, and every field there
    /// but the viewport is already what it should be.
    ///
    /// **The Set supplies the view and the node packs it.** Which fields exist
    /// is the node's business — a marcher's uniform and a sprite renderer's are
    /// different shapes — and which `t` and which camera they are packed from is
    /// the grouping's, since one clock and one camera serve every node in it.
    ///
    /// **One view, every renderer**, and each reads its own parameter map. The
    /// clock, the camera and the viewport are the grouping's and are therefore
    /// the same number for all of them; `exposure` is the node's and is not.
    fn write_l4_uniforms(&mut self, queue: &wgpu::Queue) {
        // Read before the borrow: `time` takes `&self` and each node's packer
        // takes `&mut` its own scratch, but `param` below borrows this Set.
        let t = self.time();
        let (bindings, camera, beats, salt, viewport) = (
            &self.bindings,
            &self.camera,
            self.last_beats,
            self.seed_salt,
            self.viewport,
        );
        for (at, (renderer, params)) in self.renderers.iter_mut().zip(&self.params[1..]).enumerate() {
            let view = crate::node::View {
                t,
                beats,
                seed_salt: salt,
                viewport,
                camera,
                param: &|name: &str| effective(bindings, params, Kind::L4, at, name),
            };
            renderer.write_uniforms(queue, &view);
        }
    }

    /// **Rewrite the L4 uniforms against the current viewport**, without
    /// advancing anything.
    ///
    /// For a slot being auditioned that nothing is preparing. `viewport` and
    /// the camera's aspect ratio are written by [`Set::prepare`] and by nothing
    /// else, while [`Set::resize`] moves only the host-side value — so an
    /// `Allocated` slot drawn after a resize would draw at the aspect ratio it
    /// had before it, for as long as it stayed off air. Which is to say
    /// permanently, since going off air is what stops it being prepared.
    ///
    /// **Every other field comes out unchanged**, and that is the whole
    /// contract: `t` is `steps_taken * dt` and nothing here steps, `beats` is
    /// the value the last `prepare` derived, and a bound parameter is whatever
    /// it last resolved to — bindings are not re-resolved, because resolving
    /// them against a moving grid would make a parked slot's parameters drift
    /// while its geometry stood still.
    pub fn refresh_view(&mut self, queue: &wgpu::Queue) {
        self.write_l4_uniforms(queue);
    }

    /// Every binding, once, against the signals it was handed — the session's
    /// on air, the same ones read at this Set's `t` while warming. Which is
    /// [`Set::prepare_on`]'s to decide and not this function's: it resolves
    /// against what it is given.
    ///
    /// Allocates nothing: the `Vec` is written in place, and each binding's
    /// manual value is read out of `params` — which is never written here, so
    /// a `--param` on a bound param survives the frame.
    ///
    /// **The blend base comes from the first node of that layer that declares
    /// the name**, which is not the same as the first node of that layer. A
    /// binding names a layer and [`Set::bind`] accepts it if *any* renderer
    /// declares it, so reading `params[1]` unconditionally read a map that may
    /// not have the key — and `unwrap_or(0.0)` then turned a renderer's
    /// declared default into zero, on the render path, with nothing said. A
    /// signal of confidence 0 writes the param's own value unchanged, so a
    /// binding to a name only the *second* renderer declares collapsed it to
    /// nothing.
    ///
    /// Which declaration wins when two of them have one name is
    /// [`Set::slot_of`]'s open question and is not this: the point here is only
    /// that it must be a declaration.
    fn resolve_bindings(&mut self, signals: &Signals) {
        for binding in &mut self.bindings {
            let base = Self::slot_of(binding.layer);
            let manual = Self::nodes_of(binding.layer)
                .filter(|slot| binding.covers(slot - base))
                .filter_map(|slot| self.params.get(slot))
                .find_map(|node| node.get(&binding.key).copied())
                // Cannot miss — `Set::bind` refuses a name no node of that
                // layer declares — and a panic on the render thread is not the
                // way to find out if it ever does.
                .unwrap_or(0.0);
            binding.resolve(signals, manual);
        }
    }
}

/// What a param is actually written with: its binding's value if it has one,
/// its manual value otherwise, or `None` for a name with no scalar value at all.
///
/// A linear scan, deliberately. This is the render thread: a `HashMap` keyed
/// by `String` would hash a name per param per frame to search a list that is
/// never longer than the params a procedure declares, and the scan touches one
/// cache line for a Set with no bindings at all — which is every Set today.
fn effective(
    bindings: &[Binding],
    params: &HashMap<String, f32>,
    layer: Kind,
    index: usize,
    name: &str,
) -> Option<f32> {
    match bindings
        .iter()
        .find(|b| b.layer == layer && b.key == name && b.covers(index))
    {
        Some(binding) => Some(binding.value()),
        // `params` holds only the scalar params, so a declared vector one
        // misses. `None` rather than an index: a node writing 0.0 into a
        // uniform field beats a panic on the render thread — the same
        // reasoning `resolve_bindings` gives about the same map.
        None => params.get(name).copied(),
    }
}

impl Set {
    /// Run this frame's simulation and **nothing else** — no render pass, no
    /// target, no draw.
    ///
    /// This is the whole of what a Priming slot runs. All of a Set's
    /// per-element state is the L1 node's: L4 is stateless and reads whatever
    /// L1 last wrote, so warming a Set means running this and skipping the draw.
    /// See "Priming" in [`crate::deck`] for why that is the right shape and why
    /// `docs/roadmap.md`'s "reduced resolution" is superseded by it.
    ///
    /// [`VideoSource::render`] is this followed by the draw, so the two cannot
    /// disagree about what a step is: there is one copy of the pass sequence
    /// and the parity flip that goes with it.
    ///
    /// Must be paired with a [`Set::prepare`] in the same frame, exactly as
    /// `render` must: the uniforms and each substep's `t` come from there.
    ///
    /// **The one decision made here is whether to run it at all**, and it is a
    /// decision about a pair of nodes rather than about either: a fullscreen L4
    /// consumes no attribute — the check pass refuses one that claims to — so
    /// the whole simulation would be work for a reader that does not exist. `t`
    /// still advances, because a marcher reads it; it advances in
    /// [`Set::prepare`], which is not this.
    pub fn step(&mut self, encoder: &mut wgpu::CommandEncoder, steps: u8) {
        // **Every** renderer, not any: one node that reads no attribute does
        // not excuse the simulation if another reads them all. `all` on an empty
        // list would be vacuously true, which is why an empty list is refused at
        // build rather than handled here.
        let steps = if self.renderers.iter().all(|r| r.is_fullscreen()) {
            0
        } else {
            steps
        };
        self.sim.record(encoder, steps);
    }

    /// **The draw, without advancing anything.**
    ///
    /// The L4 pass over whatever L1 last wrote, which for a Set nothing has
    /// stepped this frame is the state it stopped at. Split out of
    /// [`VideoSource::render`] for the same reason [`Set::step`] was split out
    /// of it: a preview draws without stepping and a Priming slot steps without
    /// drawing, and two copies of a render pass is how the two come to disagree
    /// about which parity L4 reads.
    ///
    /// Nothing here touches `t`, `steps_taken` or the simulation's parity. That
    /// is what lets an operator look at an `Allocated` slot without the act of
    /// looking moving it — see
    /// [`Deck::set_preview`](crate::deck::Deck::set_preview).
    ///
    /// **What a Set decides is the order and which one clears**, not how any of
    /// them draws. The renderers run in list order over the one attachment, the
    /// first clearing it and the rest loading what is there — see
    /// [`Set::build_many`] for why that is overdraw and not compositing.
    pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        let (parity, counts) = (self.sim.parity(), self.sim.counts());
        for (i, renderer) in self.renderers.iter().enumerate() {
            renderer.draw(encoder, target, parity, counts, i == 0);
        }
    }
}

impl VideoSource for Set {
    /// This frame's L1 passes, then the draw.
    ///
    /// The compute half is [`Set::step`] verbatim and the raster half is
    /// [`Set::draw`] verbatim, because a Priming slot runs the first and a
    /// preview runs the second; keeping one copy of each is what stops "primed
    /// for thirty frames then put on air" from being a different simulation
    /// than "on air for thirty frames", and an auditioned slot from being a
    /// different picture than the same slot on air.
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        steps: u8,
    ) {
        self.step(encoder, steps);
        self.draw(encoder, target);
    }
}

/// The scalar default of a param, for the uniform. Vector params are not yet
/// driven from here — every param the examples declare is a float.
///
/// **A negation is folded, because the parser does not fold it.** `= -0.35` is
/// `Unary { Neg, Lit }` and not a literal, so matching `Expr::Lit` alone silently
/// dropped every negative default: the param never entered [`Set::params`], so
/// its declared value was discarded, [`Set::bind`] refused it, and the uniform
/// got whatever the miss produced — a panic on the render thread before
/// `effective` returned an `Option`, and a quiet `0.0` after. A `.kir` declaring
/// `param drift : float [-1.0, 1.0] = -0.35` is legal and none of that is the
/// engine's to decide.
///
/// **Not general constant folding**, deliberately. A default is checked in an
/// empty scope, so it is *some* constant, but the useful set is one literal with
/// an optional sign in front of it; anything past that wants folding in
/// `karakuri-ir` where the checker could also use it, rather than a second
/// evaluator here that agrees with the shader by coincidence.
fn default_scalar(p: &karakuri_ir::Param) -> Option<f32> {
    use karakuri_ir::{Expr, Lit, UnOp};
    match &p.default {
        Expr::Lit { value: Lit::Float(v), .. } => Some(*v),
        Expr::Unary { op: UnOp::Neg, value, .. } => match value.as_ref() {
            Expr::Lit { value: Lit::Float(v), .. } => Some(-v),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    //! The two things `Set::build` does that no node does: put the two halves
    //! together, and read the `.kir`'s declared defaults. The byte-level checks
    //! for the L1 node's initial upload moved with it — see
    //! `crate::node::simulation`'s tests.
    use super::*;
    use crate::gpu::Gpu;

    /// **A negative default is a value, not an absence.**
    ///
    /// `= -0.35` parses as a negation of a literal rather than as one, and
    /// [`default_scalar`] matched `Expr::Lit` alone — so a legal `.kir` had its
    /// declared default silently discarded, could not be bound, and reached the
    /// shader as whatever the miss produced. No example declares one, which is
    /// the only reason it was never seen; nothing in the language forbids it.
    ///
    /// No GPU: this is about reading a declaration, and pinning it here rather
    /// than through a built `Set` is what keeps the failure legible.
    #[test]
    fn a_negative_param_default_is_read_as_its_declared_value() {
        let src = r#"
proc signed_defaults {
  kind  L4
  blend additive

  param drift  : float [-1.0, 1.0] = -0.35
  param plain  : float [ 0.0, 1.0] =  0.25

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 1.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, drift + plain);
  }
}
"#;
        let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
        let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
        let of = |name: &str| {
            default_scalar(
                checked
                    .params
                    .iter()
                    .find(|p| p.name == name)
                    .expect("the param is declared"),
            )
        };
        assert_eq!(of("drift"), Some(-0.35), "a negative default was read as an absence");
        assert_eq!(of("plain"), Some(0.25), "a positive default stopped being read");
    }

    /// End-to-end smoke test that a procedure *with* a `spawn` block builds
    /// a `Set` successfully and starts with a zero live count —
    /// exercising the real `generate_l1`/`generate_l4` path (not the
    /// hand-built `ElementLayout` the two tests above use) for the one
    /// shape `crates/karakuri-engine/tests/generated.rs` never covers.
    #[test]
    fn a_procedure_with_a_spawn_block_builds_and_starts_empty() {
        let gpu = Gpu::headless().expect("no GPU available");
        let l1_src = r#"
proc probe_spawn_l1 {
  kind     L1
  topology points
  capacity [4, 64] = 8

  param spawn_rate : float [0.0, 40000.0] = 1000.0

  emit position, age

  spawn {
    position = vec3(0.0, 0.0, 0.0);
    age      = 0.0;
  }

  element {
    position = position;
    age      = age;
  }
}
"#;
        let l4_src = r#"
proc probe_l4 {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 1.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
        let compile = |src: &str| -> Checked {
            let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
            let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
            karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("cost: {e:?}"));
            checked
        };
        let l1 = compile(l1_src);
        let l4 = compile(l4_src);
        let set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, 8, 1).expect("compatible pair");

        assert_eq!(set.live_count(&gpu.device, &gpu.queue), 0, "a spawn-block procedure starts empty");
    }
}
