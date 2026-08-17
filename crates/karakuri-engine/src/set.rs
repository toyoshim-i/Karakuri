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
//! What is genuinely the grouping's is what is left here: the parameter values
//! and their bindings, the viewport, the clock, and the order the nodes run in.
//! **One clock serves every node in a Set**, so a node holding its own copy
//! would be a second place for it to be — which is why `t` did not leave with
//! the simulation that advances by it. A node is handed the instants its work
//! lands on ([`crate::node::Tick`], [`crate::node::View`]) and derives none of
//! its own.
//!
//! **The camera was on that list and no longer is.** A Set still owns the
//! `Orbit` that produces it, because a `camera` record and a Set file both set
//! one from outside — but what a renderer reads is a GPU buffer owned by
//! [`crate::node::Camera`], derived in a pass. `L4 : (Geometry, Camera) ->
//! Texture` makes it an input edge, and it stopped being handed down the moment
//! it became one.
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

use crate::binding::{Binding, ParamWrite, Signals, CONTROL_PREFIX};
use crate::mix::Input;
use crate::camera::Orbit;
use crate::node::{Deform, Renderer, Simulation};
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

/// **Whether the renderers overdraw or composite**, which is the one thing the
/// presence of an L5 node decides — `docs/ir-spec.md`, "Overdraw and
/// compositing are different operations, and the graph says which".
///
/// Not a dial on one operation. Overdraw runs the renderers over one
/// attachment, the first clearing and the rest loading, so each meets what is
/// there through its own blend state; compositing gives each a cleared target
/// and folds them through a gain, an opacity, a blend mode and a mask per
/// input. They agree for additive renderers and do not for weighted ones, and
/// the second costs a frame-sized target per renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layering {
    /// One target, however many renderers. The default, and what every Set was
    /// before an L5 could be nested.
    #[default]
    Overdraw,
    /// A target per renderer, folded by an [`crate::node::Merge`].
    Composite,
}

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
    /// An amplified chain that asks for a buffer bigger than the device binds.
    ///
    /// **Checked here rather than left to fail**, because failing is not what
    /// it does: wgpu's uncaptured error handler panics the thread that built
    /// it, which at startup takes the process down. `MAX_AMPLIFY` in the
    /// checker is a bound on one declaration and cannot see either the Set's
    /// `capacity` or the other factors in the chain — the product is only in
    /// hand here, and so is the device.
    ///
    /// The limit is the device's, so this is a Set that runs on one machine and
    /// is refused on another. That is the honest report: what is too large is a
    /// property of where it is being asked to run, and the alternative to
    /// naming it is a validation panic with the same cause and no sentence.
    #[error(
        "`{l2}` amplifies to {elements} elements ({bytes} bytes), and this device binds \
         at most {limit}\n\
         hint: lower `--capacity`, lower `amplify` in `{l2}`, or narrow the chain's `emit` \
         — the buffer is the Set's capacity times every factor above this node, times the \
         element stride"
    )]
    TooManyElements {
        l2: String,
        elements: u64,
        bytes: u64,
        limit: u64,
    },
    /// More renderers than an L5 can fold.
    ///
    /// **Only under [`Layering::Composite`].** Overdrawing has no limit — the
    /// renderers share one attachment and run in order, so a hundred of them
    /// cost a hundred passes and one target. Compositing binds one texture per
    /// input and `shaders/composite.wgsl` declares four, which is the same
    /// number a deck holds and for the same reason: raising it is an edit
    /// there.
    ///
    /// Refused rather than truncated. A Set that quietly dropped its fifth
    /// renderer would draw a picture nobody asked for, with no error and no log
    /// — the exact shape this pass exists to refuse.
    #[error(
        "`{l1}` composites {count} renderers and an L5 folds at most {max}\n\
         hint: drop `--merge` for this slot and they overdraw instead, which has no limit — \
         or split them across Sets, which is what a deck is"
    )]
    TooManyInputs { l1: String, count: usize, max: usize },
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
    /// **The L2 nodes, in chain order.** Each reads what the one before it
    /// wrote and writes its own buffer, so the geometry the renderers see is
    /// the last one's — or the simulation's, when there are none.
    ///
    /// They run once per frame, after every substep, rather than once per
    /// substep: a deformation is a function of the instant the simulation
    /// reached, and running it between substeps would deform states nothing
    /// ever draws.
    deforms: Vec<Deform>,
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
    /// The declared `[min, max]` of every param, in the same node order as
    /// [`Set::params`]. **Kept because an interface needs it**: a published
    /// range is checked as a subset of the declared one, and a Set with no
    /// interface publishes every control over the range its procedure declared.
    /// Nothing else in the engine reads it — the ranges are the console's and
    /// the agent's, and no uniform write is clamped by them.
    ranges: Vec<HashMap<String, [f32; 2]>>,
    /// **The producer of the camera state**, and the only one there is until an
    /// L3 can be a procedure. Public because a `camera` record and a Set file
    /// both set it from outside; the six numbers it produces reach a renderer
    /// through [`Set::camera_node`] and never directly.
    pub camera: Orbit,
    /// **The camera edge.** Written from `camera` above every frame, derived on
    /// the GPU, and read by every renderer that projects or marches — see
    /// [`crate::node::Camera`] for why the derivation is a pass rather than host
    /// arithmetic.
    camera_node: crate::node::Camera,
    /// **The L5 node, when the Set has one.** `None` is overdraw: the renderers
    /// run in order over the one attachment. `Some` is compositing: each gets a
    /// cleared target of its own and this folds them — see
    /// [`crate::node::Merge`] for why the two are different operations rather
    /// than one with a dial.
    merge: Option<crate::node::Merge>,
    /// One per renderer, in draw order. Unread under [`Layering::Overdraw`] —
    /// an edge into an L5 is meaningless without an L5 — and the whole of what
    /// a merge knows about its inputs otherwise.
    edges: Vec<Input>,
    /// At most one per (layer, param). Resolved once per frame in
    /// [`Set::prepare`] and read back out wherever a param value is written.
    bindings: Vec<Binding>,
    /// **The Set's interface**: which of its internal controls appear on a
    /// console, under what name, and over what part of their declared range.
    ///
    /// **Empty publishes everything**, which is what [`Set::published`] does
    /// with it — so the feature is additive, every Set that predates it keeps
    /// working, and an author opts in by naming what they want rather than by
    /// hiding twenty-four things. See `docs/ir-spec.md`, "What a Set publishes".
    interface: Vec<Published>,
}

/// One control on the console, and where it lands inside the Set.
///
/// **Publishing decides what is *shown*, never what is *reachable*.** A `param`
/// record still addresses any control in any node, published or not — that is
/// how a Set file records the values its author froze, how `--param` works, and
/// how an agent tunes something the console does not show. If publishing gated
/// access a Set's author could lock an operator out of their own machine, and
/// this project's standing position is the opposite one everywhere it has come
/// up: **a surface is a choice about attention, not about authority.**
#[derive(Debug, Clone, PartialEq)]
pub struct Published {
    /// What the console shows. The Set's choice, so two nodes' `exposure` can be
    /// published as two controls under two names.
    pub name: String,
    /// **Which node, or every node that declares the key.** `None` is the
    /// wildcard, on the same terms [`ParamWrite::at`] and [`Binding::index`] are
    /// — the address is present or absent as a unit, and absent means the same
    /// thing everywhere: every declaration.
    ///
    /// It is what the *default* interface is made of. `docs/ir-spec.md` settles
    /// that "a bare name means every node that declares it" — a `--param
    /// exposure=2.0` moves both renderers, "which is exactly the one control
    /// driving both case" — so a Set with no interface publishes one control per
    /// key, not one per declaration. Publishing one per declaration would give
    /// two of them the same name, which `publish` itself refuses.
    pub at: Option<(Kind, u32)>,
    /// The param's own name inside the node.
    pub key: String,
    /// **Narrows, never redefines** — a subset of the declared range, refused
    /// rather than clamped if it is not. The declared range is the procedure's
    /// statement about where it still looks like itself.
    pub range: [f32; 2],
}

/// Why a control could not be published.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PublishError {
    #[error("nothing in this Set declares `{key}`{at}")]
    NoSuchControl { key: String, at: String },
    #[error(
        "`{name}` publishes `{key}` over [{low}, {high}], which is outside the [{min}, {max}] \
         the procedure declares\n\
         hint: a published range narrows and never redefines — the declared range is the \
         procedure's statement about where it still looks like itself"
    )]
    RangeNotASubset {
        name: String,
        key: String,
        low: f32,
        high: f32,
        min: f32,
        max: f32,
    },
    #[error("`{0}` is published twice; a console shows one control per name")]
    DuplicateName(String),
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
        Set::build_many(device, queue, l1, &[], None, &[l4], Layering::Overdraw, capacity, seed_salt)
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
    // Eight, where clippy's line is seven. Four of them are the chain — an L1, a
    // list of L2s, an optional L3, a list of L4s — and grouping them into a
    // struct would be a second spelling of "the nodes of a Set", which is what
    // the Set being returned already is.
    #[allow(clippy::too_many_arguments)]
    pub fn build_many(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        l1: &Checked,
        l2s: &[&Checked],
        l3: Option<&Checked>,
        l4s: &[&Checked],
        layering: Layering,
        capacity: u32,
        seed_salt: u32,
    ) -> Result<Set, SetError> {
        if l4s.is_empty() {
            return Err(SetError::NoRenderer { l1: l1.name.clone() });
        }
        if layering == Layering::Composite && l4s.len() > crate::deck::MAX_SLOTS {
            return Err(SetError::TooManyInputs {
                l1: l1.name.clone(),
                count: l4s.len(),
                max: crate::deck::MAX_SLOTS,
            });
        }
        if l1.kind != Kind::L1 {
            return Err(SetError::WrongKind {
                slot: "L1",
                expected: Kind::L1,
                actual: l1.kind,
            });
        }
        // **`consumes ⊆ available at this position`, walked down the chain.**
        //
        // A node reads the element struct the node above it wrote, so a consumed
        // attribute nothing upstream produced has no field to read. Left
        // unchecked it surfaces as a WGSL parse failure inside
        // `create_shader_module` — an internal error where the contract calls
        // for a diagnostic.
        //
        // **What is available grows as the chain runs**, which is why this is a
        // walk rather than a comparison against `l1.emit`: an L2 may `emit` an
        // attribute no L1 in the library produces, and everything below it can
        // then consume that. So the L2 that adds `tint` and the L4 that draws it
        // compose, while the same L4 over the bare L1 does not — and the error
        // has to name the position rather than the pair.
        //
        // Every missing attribute of one node is reported at once, for the same
        // reason the IR checker reports every error at once: one regeneration
        // should fix all of them. The *first node* that fails stops the build,
        // because everything after it would be reported against a chain that
        // will not exist.
        let mut available: Vec<karakuri_ir::Attr> = l1.emit.clone();
        let check_against = |node: &Checked, available: &[karakuri_ir::Attr]| {
            let missing: Vec<String> = node
                .consumes
                .iter()
                .filter(|a| !available.contains(a))
                .map(|a| format!("`{}`", a.name()))
                .collect();
            if missing.is_empty() {
                None
            } else {
                Some(SetError::Composition {
                    l1: l1.name.clone(),
                    l4: node.name.clone(),
                    missing: missing.join(", "),
                })
            }
        };
        for l2 in l2s {
            if l2.kind != Kind::L2 {
                return Err(SetError::WrongKind {
                    slot: "L2",
                    expected: Kind::L2,
                    actual: l2.kind,
                });
            }
            if let Some(e) = check_against(l2, &available) {
                return Err(e);
            }
            for &attr in &l2.emit {
                if !available.contains(&attr) {
                    available.push(attr);
                }
            }
        }
        for l4 in l4s {
            if l4.kind != Kind::L4 {
                return Err(SetError::WrongKind {
                    slot: "L4",
                    expected: Kind::L4,
                    actual: l4.kind,
                });
            }
            if let Some(e) = check_against(l4, &available) {
                return Err(e);
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

        // **The L2 nodes**, each built against what reaches it. The chain is
        // walked here rather than inside a node because the *grouping* decides
        // the order — a node knows how it deforms and not what is above it.
        let mut deforms: Vec<Deform> = Vec::new();
        let mut upstream: Vec<karakuri_ir::Attr> = l1.emit.clone();
        // The engine-written slots and the element count at the current
        // position, both of which an amplifier changes for everything below it.
        let mut synthetic = karakuri_codegen::layout::Synthetic::NONE;
        let mut chain_capacity = capacity;
        // **Index of the last node that amplified**, which is where the chain's
        // liveness and counts live from that point on. Tracked rather than
        // recomputed from `deforms.last()`, because a node that does *not*
        // amplify hands on whatever reached it — so the answer after
        // `[amplify, plain]` is the first node's buffers, and asking the last
        // node alone would give the simulation's.
        let mut live: Option<usize> = None;
        for l2 in l2s {
            let node = {
                let from = sim.geometry();
                let (alive, counts) = match live {
                    None => (from.alive, from.counts),
                    Some(k) => {
                        let g = deforms[k].geometry(from.alive, from.counts);
                        (g.alive, g.counts)
                    }
                };
                let input = match deforms.last() {
                    None => sim.geometry(),
                    Some(prev) => prev.geometry(alive, counts),
                };
                Deform::build(device, l2, &upstream, synthetic, &input, chain_capacity)?
            };
            upstream = node.emits().to_vec();
            synthetic = node.synthetic();
            chain_capacity = chain_capacity.saturating_mul(node.amplify());
            if node.amplifies() {
                live = Some(deforms.len());
            }
            deforms.push(node);
        }

        // **The L4 nodes**, generated, compiled and bound against the edge the
        // last node in the chain offers: the element layout, and the two
        // buffers indexed by parity. Everything about how one draws is its own
        // — see [`crate::node::Renderer`] — including the blend-mode rule that
        // needs both halves in hand. They all read the same edge, which is the
        // whole point: one simulation, several ways of looking at it.
        let renderer_count = l4s.len();
        // **One camera node, however many renderers.** Sharing is edge fan-out
        // and needs no rule: two L4s reading one camera are one viewpoint drawn
        // two ways. Two reading *different* cameras is a graph, which is what an
        // L5 is for and not what a Set is.
        if let Some(l3) = l3 {
            if l3.kind != Kind::L3 {
                return Err(SetError::WrongKind { slot: "L3", expected: Kind::L3, actual: l3.kind });
            }
        }
        let camera_node = crate::node::Camera::build(device, l3);
        let renderers: Vec<Renderer> = {
            let from = sim.geometry();
            let (alive, counts) = match live {
                None => (from.alive, from.counts),
                Some(k) => {
                    let g = deforms[k].geometry(from.alive, from.counts);
                    (g.alive, g.counts)
                }
            };
            let geometry = match deforms.last() {
                None => sim.geometry(),
                Some(last) => last.geometry(alive, counts),
            };
            l4s.iter()
                .map(|l4| Renderer::build(device, l4, &geometry, &camera_node))
                .collect()
        };

        // One map per node, in the order [`Set::slot_of`] addresses them: the
        // L1's, then each renderer's. Two nodes declaring one name now hold two
        // values, which is what a name meaning "this node's" buys.
        let declared = |p: &&karakuri_ir::Param| default_scalar(p).map(|v| (p.name.clone(), v));
        let map = |node: &Checked| -> HashMap<String, f32> {
            node.params.iter().filter_map(|p| declared(&p)).collect()
        };
        let params = std::iter::once(map(l1))
            .chain(l2s.iter().map(|n| map(n)))
            .chain(l3.map(map))
            .chain(l4s.iter().map(|n| map(n)))
            .collect();
        // The same walk, so a node's values and its ranges cannot end up at
        // different indices — the defect this file has already paid for twice.
        let declared = |node: &Checked| -> HashMap<String, [f32; 2]> {
            node.params.iter().map(|p| (p.name.clone(), [p.min, p.max])).collect()
        };
        let ranges = std::iter::once(declared(l1))
            .chain(l2s.iter().map(|n| declared(n)))
            .chain(l3.map(declared))
            .chain(l4s.iter().map(|n| declared(n)))
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
            // Every node, because a Set is only seekable if everything in it
            // is — and in practice this is still the L1's, since an L2 and an L4
            // are both vacuously closed form. The conjunction is written out
            // anyway: it is the sentence that is true, and one that named a
            // layer would have to be revisited by every layer added after it.
            closed_form: l1.closed_form
                && l2s.iter().all(|n| n.closed_form)
                && l3.is_none_or(|n| n.closed_form)
                && l4s.iter().all(|n| n.closed_form),
            reads_beats: l1.reads_beats
                || l2s.iter().any(|n| n.reads_beats)
                || l3.is_some_and(|n| n.reads_beats)
                || l4s.iter().any(|n| n.reads_beats),
            sim,
            deforms,
            renderers,
            params,
            ranges,
            camera: Orbit::default(),
            camera_node,
            // **Built at one texel and resized before anything draws.** A Set
            // is built before it is sized — `Set::resize` is a separate call
            // and `viewport` starts at `[1, 1]` — so allocating at the frame
            // size here would mean allocating at the wrong one. Every caller
            // resizes; the one that did not would draw a one-texel mix and say
            // so loudly.
            merge: (layering == Layering::Composite)
                .then(|| crate::node::Merge::build(device, renderer_count, 1, 1)),
            edges: vec![Input::default(); renderer_count],
            bindings: Vec::new(),
            interface: Vec::new(),
        };
        set.sim.initialize(queue);
        // **A camera before the first `prepare`.** The state buffer starts
        // zeroed, and a camera whose eye and target coincide has no forward
        // direction — `normalize` of it is NaN, and a NaN view matrix is a blank
        // frame with no diagnostic. Every path that draws writes this first, so
        // nothing depends on it; it costs 64 bytes once and removes a shape of
        // failure that would only ever appear in a caller's test.
        set.camera_node.write_state(queue, &set.camera.state(0.0));
        set.camera_node.write_canvas(queue, 1.0);
        // **After the simulation's own initialisation**, because what it primes
        // is a function of that. See [`Set::prime`].
        set.prime(device, queue);
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
        if let Some(merge) = &mut self.merge {
            merge.resize(device, width, height);
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

    /// How many elements the L1 node's current buffer holds, and the range the
    /// next step will scan. Not quite the alive count: an element killed during
    /// the step that just ran still occupies its slot until the next step's
    /// scan reclaims it.
    ///
    /// **Not the draw's instance count where the chain amplifies**, which it
    /// used to be and is the sentence this doc carried until an L2 could change
    /// a count. A renderer draws from [`Set::output_counts`]; below an amplifier
    /// that is this number times every factor above it. This one is the
    /// simulation's population, which is the figure a status line wants — how
    /// much material a Set is holding, not how many primitives came of it.
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
        // **A control source is checked against the interface**, not against the
        // params. A misspelt one used to be accepted, hold its param wherever it
        // found it, and be reported by the terminal as deciding that param
        // outright — the same failure the confidence display had, one step
        // further along. The order this puts on a caller is the order a macro
        // needs anyway: publish, then bind.
        if let Some(name) = binding.signal.strip_prefix(CONTROL_PREFIX) {
            if !self.published().iter().any(|p| p.name == name) {
                return false;
            }
        }
        let range = self.nodes_of(binding.layer);
        let names: Vec<&[String]> = match binding.layer {
            Kind::L1 => vec![self.sim.param_names()],
            Kind::L2 => self.deforms.iter().map(|d| d.param_names()).collect(),
            Kind::L3 => match self.camera_node.node_count() {
                0 => Vec::new(),
                _ => vec![self.camera_node.param_names()],
            },
            Kind::L4 => self.renderers.iter().map(|r| r.param_names()).collect(),
        };
        let found = names.iter().enumerate().any(|(at, n)| {
            binding.covers(at)
                && self
                    .params
                    .get(range.start + at)
                    .is_some_and(|map| declares(n, map))
        });
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

    /// **Where a layer's nodes start in [`Set::params`].**
    ///
    /// The maps are in node order — the L1, then each deformation, then each
    /// renderer — so this depends on how long the chain is and cannot be a
    /// constant. That is the price of one flat list, and it is the right price:
    /// a `Vec` per layer would make "which node is this" three questions
    /// instead of one arithmetic.
    fn slot_of(&self, layer: Kind) -> usize {
        match layer {
            Kind::L1 => 0,
            Kind::L2 => 1,
            // **An L3's place is between the deformations and the renderers**,
            // and there is at most one — a Set is a grouping around one
            // viewpoint. A Set whose camera is the built-in has none at all, so
            // this and `L4` name the same slot and `nodes_of` hands back an
            // empty range: a `--param L3:…` then reaches no node and is reported
            // as reaching none, which is the answer a name no procedure declares
            // already gets.
            Kind::L3 => 1 + self.deforms.len(),
            Kind::L4 => 1 + self.deforms.len() + self.camera_node.node_count(),
        }
    }

    /// Every map in [`Set::params`] belonging to `layer`, in node order.
    fn nodes_of(&self, layer: Kind) -> std::ops::Range<usize> {
        let start = self.slot_of(layer);
        match layer {
            Kind::L1 => start..start + 1,
            Kind::L2 => start..start + self.deforms.len(),
            // Zero or one: the built-in camera is a field on this struct rather
            // than a node, and has no parameter map to address.
            Kind::L3 => start..start + self.camera_node.node_count(),
            Kind::L4 => start..self.params.len(),
        }
    }

    /// **Set every declaration of `name`, and say how many there were.**
    ///
    /// Zero means nothing in this Set declares it, which is the caller's cue to
    /// say so — a `--param` for a name a regenerated artifact no longer has
    /// should not take the show down.
    ///
    /// A name rather than an address, because "the Set's `exposure`" is the
    /// useful default when two nodes both have one: one knob moves both, which
    /// is what a bare `--param` asks for and what a console would publish as one
    /// control. [`Set::set_param_at`] is the addressed form, for setting them
    /// apart; [`Set::write_param`] is the one entry point both come through.
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
    ///
    /// **Bounded by the layer, not by the list.** `slot_of(layer) + index` is a
    /// position in `params` and says nothing about whether that position still
    /// belongs to `layer` — the layers are laid end to end, so an index past a
    /// layer's last node lands on the *next* layer's first. A Set with no L3
    /// makes that concrete: `slot_of(L3)` and `slot_of(L4)` are the same number,
    /// and `--param L3:0:exposure=0.0` was reaching renderer 0 and blacking out
    /// the frame. `nodes_of` is the range, and stepping into it is the only
    /// spelling that cannot walk out the other end.
    pub fn set_param_at(&mut self, layer: Kind, index: u32, name: &str, value: f32) -> bool {
        let Some(slot) = self.nodes_of(layer).nth(index as usize) else {
            return false;
        };
        match self.params.get_mut(slot).and_then(|n| n.get_mut(name)) {
            Some(held) => {
                *held = value;
                true
            }
            None => false,
        }
    }

    /// Apply one [`ParamWrite`], addressed or not. Returns how many nodes it
    /// reached; zero is the caller's cue to say so.
    ///
    /// The one entry point a `param` record and a `--param` both come through,
    /// so the wildcard and the address cannot come to mean different things on
    /// the two paths.
    pub fn write_param(&mut self, write: &ParamWrite) -> usize {
        match write.at {
            None => self.set_param(&write.key, write.value),
            Some((layer, index)) => {
                usize::from(self.set_param_at(layer, index, &write.key, write.value))
            }
        }
    }

    /// **Add one control to this Set's interface.**
    ///
    /// The first call makes the list *be* the interface — before it, a Set
    /// publishes everything. That is one sentence of rule and it means an author
    /// opts in by naming what they want rather than by hiding twenty-four
    /// things.
    pub fn publish(&mut self, control: Published) -> Result<(), PublishError> {
        let Some([min, max]) = self.declared_range(control.at, &control.key) else {
            return Err(PublishError::NoSuchControl {
                key: control.key,
                at: match control.at {
                    Some((layer, index)) => format!(" at {layer:?}:{index}"),
                    None => String::new(),
                },
            });
        };
        // **A subset, and refused rather than clamped.** Clamping would let a
        // Set file say one thing and the console show another; the declared
        // range is the procedure's statement about where it still looks like
        // itself, so publishing outside it is a claim the procedure did not
        // make.
        let [low, high] = control.range;
        if low < min || high > max || low > high {
            return Err(PublishError::RangeNotASubset {
                name: control.name,
                key: control.key,
                low,
                high,
                min,
                max,
            });
        }
        if self.interface.iter().any(|p| p.name == control.name) {
            return Err(PublishError::DuplicateName(control.name));
        }
        self.interface.push(control);
        Ok(())
    }

    /// The declared range of a control, **narrowed to what every addressed node
    /// allows** when the address is a wildcard.
    ///
    /// The intersection rather than the union: a wildcard control moves every
    /// declaration at once, so a position outside any one of their ranges is a
    /// position that procedure did not say it still looks like itself at.
    fn declared_range(&self, at: Option<(Kind, u32)>, key: &str) -> Option<[f32; 2]> {
        let mut found: Option<[f32; 2]> = None;
        for layer in [Kind::L1, Kind::L2, Kind::L3, Kind::L4] {
            for (index, slot) in self.nodes_of(layer).enumerate() {
                if at.is_some_and(|(l, i)| l != layer || i != index as u32) {
                    continue;
                }
                let Some([min, max]) = self.ranges.get(slot).and_then(|n| n.get(key)).copied()
                else {
                    continue;
                };
                found = Some(match found {
                    None => [min, max],
                    Some([lo, hi]) => [lo.max(min), hi.min(max)],
                });
            }
        }
        found
    }

    /// **What a console shows**, which for a Set with no interface is
    /// everything it declares, each over its own declared range.
    ///
    /// Allocates, so not the frame path. A console reads this when a Set lands,
    /// not per frame.
    pub fn published(&self) -> Vec<Published> {
        if !self.interface.is_empty() {
            return self.interface.clone();
        }
        // **The default interface, computed rather than stored.** Storing it
        // would make "publishes everything" a list that a rebuild has to
        // regenerate and a Set file has to carry — and the first `publish` call
        // would then have to *remove* twenty-four entries to mean what it means.
        //
        // **One control per key, not per declaration**, which is the rule
        // `docs/ir-spec.md` already states for a bare name: two renderers'
        // `exposure` is one knob moving both. Per declaration would put two
        // controls called `exposure` on the console, which `publish` refuses
        // when it is asked for explicitly and which nothing could address.
        let mut keys: Vec<&String> = self.ranges.iter().flat_map(|node| node.keys()).collect();
        // Declaration order is not kept in a map, and a console showing its
        // controls in a different order each run is not a console.
        keys.sort();
        keys.dedup();
        keys.into_iter()
            .filter_map(|key| {
                Some(Published {
                    name: key.clone(),
                    at: None,
                    key: key.clone(),
                    range: self.declared_range(None, key)?,
                })
            })
            .collect()
    }

    /// **Set a published control, in the units the console shows it in.**
    /// `false` if nothing publishes that name.
    ///
    /// The value is clamped to the *published* range, which is the one place
    /// narrowing bites: a console cannot ask for more than a Set offered. An
    /// agent that wants the whole declared range writes the param by address
    /// instead — see [`Published`] on why that is deliberate.
    pub fn set_published(&mut self, name: &str, value: f32) -> bool {
        let Some(control) = self.published().into_iter().find(|p| p.name == name) else {
            return false;
        };
        let clamped = value.clamp(control.range[0], control.range[1]);
        // **Through `write_param`**, which is the one entry point a `--param`
        // and a `param` record both come through — so a wildcard control means
        // exactly what a bare name means everywhere else, and an addressed one
        // means exactly what an addressed `--param` does.
        self.write_param(&ParamWrite { at: control.at, key: control.key, value: clamped }) > 0
    }

    /// A published control's position in `[0, 1]`, which is what a binding's
    /// curve and range expect. `0.0` for a name nothing publishes — a binding on
    /// a control that is not there resolves to the bottom of its own range,
    /// which is quieter than a panic on the render thread and is what every
    /// other miss in this file does.
    fn control_position(&self, name: &str) -> Option<f32> {
        // **Without allocating**, which `published()` cannot promise: this is
        // called from `resolve_bindings`, which `Set::prepare` calls, and the
        // first invariant in `README.md` is the one about not allocating on the
        // render thread. So the interface is searched in place and the default
        // one — where a control's name *is* a param's key — is answered without
        // building the list it would appear in.
        // **The key, not the name.** A control is published under a name the Set
        // chose and lands on a param with its own — `blend` on `radius` — so
        // reading the value back by the console's name finds nothing. It is only
        // in the default interface that the two coincide, which is why every
        // test of a *renamed* control is the one that catches this.
        let (at, key, [low, high]) = match self.interface.iter().find(|p| p.name == name) {
            Some(control) => (control.at, control.key.as_str(), control.range),
            // Only when nothing is published: with an interface, a name that is
            // not in it is not a control, and a param that happens to share the
            // name is not one either.
            None if self.interface.is_empty() => (None, name, self.declared_range(None, name)?),
            None => return None,
        };
        let value = self.value_at(at, key)?;
        // A published range of zero width is one position, and it is the top of
        // it: an author who froze a control at a value did not ask for the
        // bottom of an empty interval.
        if high <= low {
            return Some(1.0);
        }
        Some(((value - low) / (high - low)).clamp(0.0, 1.0))
    }

    /// What a published control currently holds, in its own units.
    pub fn published_value(&self, name: &str) -> Option<f32> {
        let control = self.published().into_iter().find(|p| p.name == name)?;
        self.value_at(control.at, &control.key)
    }

    /// What a control holds: the addressed node's value, or the first
    /// declaration's for a wildcard.
    ///
    /// **The first is the only one there is**, for a wildcard: every write
    /// through one moves every declaration together, so they cannot disagree
    /// unless something addressed one of them behind the control's back — which
    /// is exactly what an unpublished control still being reachable means, and
    /// is the operator's business rather than a case to reconcile here.
    fn value_at(&self, at: Option<(Kind, u32)>, key: &str) -> Option<f32> {
        for layer in [Kind::L1, Kind::L2, Kind::L3, Kind::L4] {
            for (index, slot) in self.nodes_of(layer).enumerate() {
                if at.is_some_and(|(l, i)| l != layer || i != index as u32) {
                    continue;
                }
                if let Some(v) = self.params.get(slot).and_then(|n| n.get(key)) {
                    return Some(*v);
                }
            }
        }
        None
    }

    /// **The edges into this Set's L5**, in draw order. Empty of meaning under
    /// [`Layering::Overdraw`] — there is no L5 for an edge to go into — and
    /// present either way, because whether a Set composites is a build decision
    /// and a caller reading its controls should not have to branch on it.
    pub fn inputs(&self) -> &[Input] {
        &self.edges
    }

    /// Set one renderer's edge into the L5. `false` if there is no such
    /// renderer.
    ///
    /// **Silently ineffective under `Overdraw`**, which is stated rather than
    /// refused: a Set built to overdraw has the controls and nothing reads them,
    /// exactly as a `param` a procedure declares and never uses is written and
    /// never read. Refusing would make every caller ask a question it has no
    /// reason to have an answer to.
    pub fn set_input(&mut self, at: usize, input: Input) -> bool {
        match self.edges.get_mut(at) {
            Some(edge) => {
                *edge = input;
                true
            }
            None => false,
        }
    }

    /// Whether this Set composites its renderers or overdraws them.
    pub fn layering(&self) -> Layering {
        if self.merge.is_some() {
            Layering::Composite
        } else {
            Layering::Overdraw
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
        // **Addressed through [`Set::nodes_of`]**, for the reason the same
        // arithmetic spelled out by hand went wrong twice: it is one fact —
        // where a layer's nodes are — and a copy of it is a copy that can be
        // right about `L2` and wrong about `L3`. This one was, and reported
        // every camera parameter as a renderer's.
        let addressed: Vec<(Kind, u32, usize)> = [Kind::L1, Kind::L2, Kind::L3, Kind::L4]
            .into_iter()
            .flat_map(|layer| {
                self.nodes_of(layer)
                    .enumerate()
                    .map(move |(index, slot)| (layer, index as u32, slot))
            })
            .collect();
        addressed.into_iter().flat_map(move |(layer, index, slot)| {
            self.params[slot].iter().map(move |(k, v)| (layer, index, k.as_str(), *v))
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
            let (bindings, params) = (&self.bindings, &self.params[0]);
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
    /// different shapes — and which `t` they are packed from is the grouping's,
    /// since one clock serves every node in it.
    ///
    /// **One view, every renderer**, and each reads its own parameter map. The
    /// clock and the viewport are the grouping's and are therefore the same
    /// number for all of them; `exposure` is the node's and is not. The camera
    /// is neither: it is written once here, into its own edge, and read by every
    /// renderer off the GPU.
    fn write_l4_uniforms(&mut self, queue: &wgpu::Queue) {
        self.write_l2_uniforms(queue);
        // Read before the borrow: `time` takes `&self` and each node's packer
        // takes `&mut` its own scratch, but `param` below borrows this Set.
        let t = self.time();
        // **The camera's edge, not a renderer's field.** This goes in here
        // rather than into each uniform because there is one camera and several
        // readers; the aspect ratio goes with it because a renderer no longer
        // knows what projection it is drawing under. Both are writes rather than
        // passes — the derivation is recorded in [`Set::draw`].
        //
        // Which producer gets written is the node's decision and not this
        // one's: a Set hands down the frame and the built-in's six numbers, and
        // an L3 uses the first while the orbit uses the second.
        self.camera_node.write_canvas(queue, self.viewport[0] / self.viewport[1]);
        if let Some(merge) = &self.merge {
            merge.write_uniform(queue, &self.edges);
        }
        {
            // **`nodes_of` rather than `slot_of`**, because there may be no
            // camera node at all: with no L3 the two layers share a slot number,
            // so `slot_of(L3)` names the first *renderer's* parameter map. It is
            // unread today — the built-in producer declares no params and never
            // calls this closure — and it would become a renderer's `exposure`
            // arriving as the orbit's `radius` the moment the built-in took one.
            let at = self.nodes_of(Kind::L3).next();
            let (bindings, params, dt) =
                (&self.bindings, at.and_then(|at| self.params.get(at)), self.dt);
            let view = crate::node::View {
                t,
                beats: self.last_beats,
                seed_salt: self.seed_salt,
                viewport: self.viewport,
                param: &|name: &str| {
                    params.and_then(|p| effective(bindings, p, Kind::L3, 0, name))
                },
            };
            let fallback = self.camera.state(t);
            self.camera_node.prepare(queue, &view, dt, &fallback);
        }
        let (bindings, beats, salt, viewport) =
            (&self.bindings, self.last_beats, self.seed_salt, self.viewport);
        // **Asked rather than re-derived.** This line spelled out `1 +
        // deforms.len()` and was right until an L3 landed between the
        // deformations and the renderers — after which every renderer read the
        // node before it, and `soft_points` drew a black frame because its
        // `exposure` resolved against the camera's parameter map. `slot_of` is
        // the one answer to "where does this layer start"; a second copy of it
        // is a second thing to remember to change.
        let first = self.slot_of(Kind::L4);
        for (at, (renderer, params)) in
            self.renderers.iter_mut().zip(&self.params[first..]).enumerate()
        {
            let view = crate::node::View {
                t,
                beats,
                seed_salt: salt,
                viewport,
                param: &|name: &str| effective(bindings, params, Kind::L4, at, name),
            };
            renderer.write_uniforms(queue, &view);
        }
    }

    /// The deformations' uniform blocks, from the same view the renderers get.
    ///
    /// **One instant for the whole chain.** An L2 runs after every substep, at
    /// the point the simulation reached, which is the same instant a renderer
    /// draws at — so a node in the middle of a chain and the node that draws its
    /// output cannot disagree about when this frame is.
    fn write_l2_uniforms(&mut self, queue: &wgpu::Queue) {
        let t = self.time();
        let (bindings, beats, salt, viewport, dt) =
            (&self.bindings, self.last_beats, self.seed_salt, self.viewport, self.dt);
        let capacity = self.sim.capacity();
        // The range is read before the loop: `self.deforms` is borrowed mutably
        // by the iterator and `self.params` immutably by the closure, which are
        // disjoint fields — but a call on `self` inside the same expression is
        // not.
        //
        // **Asked, not spelled out.** This was `1..1 + self.deforms.len()`, which
        // is the identical arithmetic the L4 pass had and that an L3 broke. It
        // happens to be right for L2 because that layer starts at a constant —
        // which is exactly the kind of accident that stops being one.
        let range = self.nodes_of(Kind::L2);
        for (at, (node, params)) in self.deforms.iter_mut().zip(&self.params[range]).enumerate() {
            let view = crate::node::View {
                t,
                beats,
                seed_salt: salt,
                viewport,
                param: &|name: &str| effective(bindings, params, Kind::L2, at, name),
            };
            node.write_uniforms(queue, &view, dt, capacity);
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
        // Read before the loop: `slot_of` and `nodes_of` take `&self`, and the
        // loop holds `self.bindings` mutably. Three small numbers rather than a
        // borrow that cannot be had.
        let ranges: Vec<(usize, std::ops::Range<usize>)> = [Kind::L1, Kind::L2, Kind::L3, Kind::L4]
            .into_iter()
            .map(|k| (self.slot_of(k), self.nodes_of(k)))
            .collect();
        // **The bindings move out and back rather than being borrowed**, because
        // resolving a control-driven one needs `&self` — a published control's
        // position is a value this Set holds — while the loop needs them
        // mutably. A `mem::take` is a pointer swap and this is the render
        // thread; collecting the positions into a map first was the obvious
        // shape and allocated one `String` per control per frame.
        let mut bindings = std::mem::take(&mut self.bindings);
        let params = &self.params;
        for binding in &mut bindings {
            // `L3`'s range is empty until a Set holds one — see `Set::slot_of`.
            // A binding cannot be attached to it either, so this arm resolves
            // nothing rather than being unreachable.
            let (base, range) = match binding.layer {
                Kind::L1 => ranges[0].clone(),
                Kind::L2 => ranges[1].clone(),
                Kind::L3 => ranges[2].clone(),
                Kind::L4 => ranges[3].clone(),
            };
            let manual = range
                .clone()
                .filter(|slot| binding.covers(slot - base))
                .filter_map(|slot| params.get(slot))
                .find_map(|node| node.get(&binding.key).copied())
                // Cannot miss — `Set::bind` refuses a name no node of that
                // layer declares — and a panic on the render thread is not the
                // way to find out if it ever does.
                .unwrap_or(0.0);
            // **A macro is a binding whose source is a published control**, and
            // it needed no new record and no new semantics — `docs/ir-spec.md`,
            // "What a Set publishes". Resolved here rather than on the bus
            // because a published control is the *Set's*: four Sets publishing
            // `twist` are four controls, where a signal name is one thing across
            // the session. The confidence is 1 because this is the operator's
            // hand rather than a guess at something unobserved.
            match binding.signal.strip_prefix(CONTROL_PREFIX) {
                // **A name nothing publishes leaves the param alone**, rather
                // than driving it to the bottom of the binding's range. A
                // misspelt control is a mistake, and the honest reading of a
                // source that is not there is that nothing is driving this —
                // which is what a manual value is for.
                Some(name) => {
                    match self.control_position(name) {
                        Some(at) => binding.drive(at),
                        None => binding.hold(manual),
                    };
                }
                None => {
                    binding.resolve(signals, manual);
                }
            }
        }
        self.bindings = bindings;
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
        // **Every amplifier's counts, before any node dispatches from one.**
        // They derive from the simulation's, which the scan has just written,
        // and in chain order because a second amplifier derives from the first.
        self.record_counts(encoder);
        // **After every substep, once.** A deformation is a function of the
        // instant the simulation reached; running it between substeps would
        // deform states nothing ever draws, and cost one pass per substep to do
        // it. It runs even at `steps == 0` — a paused frame still has to leave
        // the chain's output holding what the renderers are about to read, and
        // the parity has not moved, so it recomputes the same thing.
        let parity = self.sim.parity();
        // **Each node dispatches over the range at *its* position**, which the
        // node above it decides. Walking it here rather than asking
        // `self.sim` once is the whole of what an amplifier costs the chain: it
        // multiplies the range for everything below it, and a stage handed the
        // simulation's counts instead would deform the first `range` of
        // `range * factor` elements and leave the rest holding the previous
        // frame.
        let mut counts = self.sim.counts();
        for node in &self.deforms {
            node.record(encoder, parity, counts);
            if let Some(own) = node.counts() {
                counts = own;
            }
        }
    }

    /// Every amplifier's derived counts, in chain order.
    ///
    /// Chain order because a second amplifier derives from the first, and one
    /// invocation each because a count does not scale with anything.
    fn record_counts(&self, encoder: &mut wgpu::CommandEncoder) {
        for node in &self.deforms {
            node.record_counts(encoder);
        }
    }

    /// **Run the deformation chain once, at build, over the state the
    /// simulation was initialised with.**
    ///
    /// A Set draws without stepping — that is what an audition of an
    /// `Allocated` slot is, and both [`Set::draw`] and the deck say it shows
    /// the still the Set stopped at. For a chain of plain L2s that is free:
    /// they hand on the L1's own liveness and counts, which
    /// `Simulation::initialize` writes at build, so a Set nothing has stepped
    /// draws its initial state. **An amplifier has buffers of its own and they
    /// are not free**: freshly allocated, therefore zeroed, therefore no
    /// instances and every copy dead. A working Set auditioned black.
    ///
    /// So the derived buffers are primed here, on the same principle and in the
    /// same place the simulation's are. What it costs is one pass per node at
    /// build; what it buys is that *the chain's output always reflects the
    /// simulation's current state*, from birth rather than from the first step
    /// — which is the sentence every reader of that output already assumed.
    ///
    /// This is legal precisely because an L2 is stateless: its output is a pure
    /// function of its input, so recomputing it advances nothing. A stateful
    /// layer could not be primed without deciding what priming *means*.
    fn prime(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.deforms.is_empty() {
            return;
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("prime the deformation chain"),
        });
        self.record_counts(&mut encoder);
        let parity = self.sim.parity();
        let mut counts = self.sim.counts();
        for node in &self.deforms {
            node.record(&mut encoder, parity, counts);
            if let Some(own) = node.counts() {
                counts = own;
            }
        }
        queue.submit([encoder.finish()]);
    }

    /// The counts the chain ends on: the last amplifier's, or the simulation's
    /// where there is none.
    ///
    /// **Asked of the list rather than remembered**, so that it cannot disagree
    /// with the walk in [`Set::step`] about which node that is — the two are the
    /// same question at two positions, and a stored answer is the shape this
    /// file has already paid for three times.
    fn output_counts(&self) -> &wgpu::Buffer {
        self.deforms
            .iter()
            .rev()
            .find_map(|node| node.counts())
            .unwrap_or_else(|| self.sim.counts())
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
        // **Ahead of every renderer, and here rather than in [`Set::step`].**
        // The camera is an input edge of an L4, so it has to be current wherever
        // an L4 runs — and a preview draws a slot that nothing stepped. One pass
        // for the whole Set, because one camera serves every node in it.
        self.camera_node.record(encoder);
        // The chain's output, not the simulation's: a renderer draws
        // `instance_count` instances of whatever reached it, and below an
        // amplifier that is `factor` times what the simulation holds.
        let (parity, counts) = (self.sim.parity(), self.output_counts());
        // **The presence of an L5 is what decides overdraw from compositing**,
        // and it decides it here, in the one place the renderers are given
        // somewhere to draw. Under overdraw they share `target` and the first
        // one clears it; under compositing each has a cleared target of its own
        // — `first` is true for every one of them, because "first onto this
        // attachment" is what it means and each of them is.
        let Some(merge) = &self.merge else {
            for (i, renderer) in self.renderers.iter().enumerate() {
                renderer.draw(encoder, target, parity, counts, i == 0);
            }
            return;
        };
        for (i, renderer) in self.renderers.iter().enumerate() {
            renderer.draw(encoder, merge.target(i), parity, counts, true);
        }
        merge.record(encoder, target);
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

    /// **Every field of a derived `Counts` means what its name says**, including
    /// the ones nothing below an amplifier reads.
    ///
    /// `survivors` is the scan's output and no pass below a deformation looks at
    /// it, so a wrong value there is invisible in every picture — a fact
    /// confirmed the hard way: dropping its multiplication left the whole
    /// GPU test file green. It is still wrong. A `Counts` is handed on as a
    /// whole, and one whose fields are true only where they happen to be read is
    /// a buffer whose meaning depends on where it came from.
    ///
    /// Here rather than in `tests/amplify.rs` because the buffers are the node's
    /// own and reaching them from outside the crate would mean widening the API
    /// to say something only a test wants to know.
    #[test]
    fn an_amplifiers_derived_counts_multiply_every_element_count_and_no_other_field() {
        let Some(gpu) = Gpu::headless().ok() else { return };
        const FACTOR: u32 = 4;
        const CAPACITY: u32 = 64;

        let compile = |src: &str| -> Checked {
            let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
            let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
            karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("cost: {e:?}"));
            checked
        };
        let l1 = compile(
            r#"
proc still {
  kind     L1
  topology points
  capacity [64, 64] = 64

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#,
        );
        let l2 = compile(
            r#"
proc mirror {
  kind    L2
  amplify 4

  consumes position

  deform {
    position = position + vec3(0.0, float(copy), 0.0);
  }
}
"#,
        );
        let l4 = compile(
            r#"
proc dots {
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
"#,
        );
        let set = Set::build_many(
            &gpu.device,
            &gpu.queue,
            &l1,
            &[&l2],
            None,
            &[&l4],
            Layering::Overdraw,
            CAPACITY,
            1,
        )
        .expect("a chain of one L1, one amplifying L2 and one L4");

        // `build_many` primes the chain, so the derived counts are current
        // without a step — which is the other thing this asserts.
        let read = |buf: &wgpu::Buffer| -> [u32; 12] {
            let size = karakuri_codegen::layout::counts::SIZE;
            let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("counts readback"),
                size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            let mut encoder = gpu.device.create_command_encoder(&Default::default());
            encoder.copy_buffer_to_buffer(buf, 0, &readback, 0, size);
            gpu.queue.submit([encoder.finish()]);
            let slice = readback.slice(..);
            slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
            gpu.device.poll(wgpu::PollType::Wait).expect("poll");
            let data = slice.get_mapped_range();
            let mut out = [0u32; 12];
            for (i, w) in data.chunks_exact(4).take(12).enumerate() {
                out[i] = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
            }
            drop(data);
            readback.unmap();
            out
        };

        let from = read(set.sim.counts());
        let derived = read(set.deforms[0].counts().expect("the node amplifies"));

        // Field order is `counts::WGSL`'s: elem_xyz, range, vertex_count,
        // instance_count, first_vertex, first_instance, survivors.
        assert_eq!(derived[3], from[3] * FACTOR, "range");
        assert_eq!(derived[5], from[5] * FACTOR, "instance_count");
        assert_eq!(derived[8], from[8] * FACTOR, "survivors");
        assert_eq!(
            derived[0],
            from[3] * FACTOR / karakuri_codegen::layout::WORKGROUP_SIZE,
            "workgroups, over the amplified range"
        );
        assert_eq!((derived[1], derived[2]), (1, 1), "the other two dimensions");

        // **And the two that are not counts of elements.** `vertex_count` is the
        // corners of one primitive, which is a property of how a renderer
        // expands an element; the two `first_*` are where a draw starts.
        assert_eq!(derived[4], from[4], "vertex_count");
        assert_eq!(derived[6], from[6], "first_vertex");
        assert_eq!(derived[7], from[7], "first_instance");
    }
}
