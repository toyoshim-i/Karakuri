//! The Set: a grouping of nodes forming one video source, and the unit of both
//! compilation and lifecycle.
//!
//! **A Set does not own everything, and no longer pretends to.** `Ln` is a node
//! and the unit that owns GPU state is the node rather than the grouping. Both of the nodes there are
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
//! `Orbit` that produces the built-in one, because a `camera` record and a Set
//! file both set it from outside — but what a renderer reads is a GPU buffer
//! owned by [`crate::node::Camera`], derived in a pass. `L4 : (Geometry,
//! Camera) -> Texture` makes it an input edge, and it stopped being handed down
//! the moment it became one. **There are as many as the Set's files declare**,
//! and which renderer reads which is an `edge` — including the built-in, which
//! is a node with a name for exactly that reason.
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

use karakuri_codegen::{generate_l1, generate_l2};
use karakuri_ir::layout::{ElementLayout, Synthetic};
use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;

use crate::binding::{Binding, ParamWrite, Signals, CONTROL_PREFIX};
use crate::camera::Orbit;
use crate::mix::Input;
use crate::node::{Deform, Renderer, Simulation};
use crate::storage::{DeformStorage, SimulationStorage};
use crate::video_source::VideoSource;

/// Past this the simulation falls behind rather than catching up — the
/// ir-spec's cap, restated here because it is now load-bearing rather than
/// advisory: each substep needs its own spawn-count entry, and that array is
/// sized once, at build time.
pub const MAX_STEPS: u8 = 4;

/// **What the built-in camera is called when nobody named it.**
///
/// Every node has a name so that an `edge` can point at it, and a name that is
/// derived is derived from the procedure — which the built-in has not got. So
/// it is written down once, here, rather than in the caller that needs to spell
/// it: `--edge lens.view=orbit` is the whole of how a renderer says it draws
/// from the camera a Set has when its files declare none.
///
/// Disambiguated like any other derived name, so a Set holding a `proc orbit`
/// beside it has an `orbit` and an `orbit-2` rather than a collision.
pub const BUILTIN_CAMERA: &str = "orbit";

/// **What one node allocated to hold elements, and how many elements those
/// bytes cover.**
///
/// **One entry per element** is the rule that decides what is counted. The
/// element buffer, the alive array and the compaction scan's destination
/// indices are all indexed by element, which is what makes them a per-element
/// figure at all; the counts block, the uniform block and the scan's block-sum
/// pyramid are not — there is one counts block per node whatever the capacity,
/// and the pyramid is indexed by workgroup. Leaving those out is what keeps
/// [`ElementStorage::per_element`] an exact division rather than a rounded one,
/// and what keeps the number answering "what does one more element cost".
///
/// **Every byte here is read off a buffer rather than recomputed.** The fields
/// are sums of `wgpu::Buffer::size()` over the buffers the node created, so
/// there is no second expression beside the `create_buffer` call for anybody to
/// keep in step. That is the whole reason the figure lives at this end: stage 4
/// published one derived beside the allocation instead of from it, and it was a
/// third of the real number by the time anybody measured — see the module doc
/// on [`karakuri_ir::cost`].
///
/// **It is not what the node occupies in VRAM.** Everything not indexed by
/// element is outside it: the render targets a renderer or a merge owns, every
/// uniform block, the `counts` block, `step_args`, and the compaction scan's
/// per-level length uniforms and block-sum pyramid — see `crate::compaction`,
/// where the two the scan owns are described from the other end. Those are per
/// node, per level or per pass rather than per element, so counting them would
/// both answer a different question and stop the division being exact. A caller
/// sizing a real allocation against a real device needs this and more; a caller
/// asking what one more element costs wants exactly this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementStorage {
    /// The sum of the real sizes of this node's per-element buffers.
    pub bytes: u64,
    /// **The element count those bytes cover, which is the node's own and not
    /// the Set's.** An amplifier multiplies the capacity for everything below
    /// it, so a node under one is sized — and reports — at the multiplied
    /// count. This is exactly the input a per-procedure figure cannot have.
    pub capacity: u32,
}

impl ElementStorage {
    /// Bytes per element, exactly: every buffer counted is a whole multiple of
    /// [`ElementStorage::capacity`].
    ///
    /// Zero capacity is unreachable through a built node, and the guard is in
    /// the checker rather than in the range test: stage 3 refuses an L1 whose
    /// declared `capacity` minimum is below 1 — `karakuri_ir::check`,
    /// "`capacity` minimum must be at least 1; a Set of no elements has nothing
    /// to run" — so no range a build can be asked for contains zero.
    /// [`capacity_in_range`] refusing a capacity outside the declared range is
    /// *not* what rules it out, because a range is only as strong as its own
    /// minimum and one written `[0, …]` would admit it.
    ///
    /// The division is guarded anyway, because a panic on the *reporting* path
    /// is the worst place for a Set to discover a capacity it should never have
    /// accepted.
    pub fn per_element(self) -> u64 {
        match self.capacity {
            0 => 0,
            n => self.bytes / u64::from(n),
        }
    }
}

/// The fixed simulation step. **Not** the real frame delta — see
/// `docs/principles/0092-the-same-inputs-produce-the-same-frame.md`.
/// Public because the session clock a
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

/// **Who may move one node of a Set.** The manual's sixth rule as a list of
/// three: *"Each node of a Set is manual, suggesting, or automatic, and you set
/// that node by node"* —
/// `docs/adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md`.
///
/// **A permission granted forward, not a record of who moved something last.**
/// The second is read off the binding that is driving a param
/// ([`Set::bindings`]); this is the other question, asked before anything
/// moves.
///
/// **The engine's own copy of the list, which is [`Layering`]'s and
/// [`crate::deck::Residency`]'s and [`crate::transport::Sync`]'s position and
/// not a new one.** `karakuri-operation` names three destinations for a
/// vocabulary every surface can depend on without pulling in a device, and it
/// says at [`Residency`](crate::deck::Residency)'s counterpart that its copies
/// of `Blend`, `Sync` and `Residency` are "this crate's copies of lists
/// `karakuri-engine` and `karakuri-store` already hold". This is the list they
/// are a copy *of*: what a node's authority is allowed to be is the engine's to
/// say, the same way what a residency or a sync mode is allowed to be is. The
/// two are checked against each other where every other pair already is —
/// `karakuri-cli`'s `mix.rs`, the one crate that sees both spellings at once
/// (`docs/adr/0194-…`).
///
/// **[`Authority::Manual`] is the default and nothing else could be.** Rule 06
/// is about what an operator *grants* — *"There is no switch that hands the
/// whole instrument to an agent"* — so a node nobody has spoken for is not
/// granted, and a build that came up any other way would hand every node of
/// every Set to an agent nobody asked for. It is also the only default that
/// leaves
/// `docs/principles/0078-the-operator-wins-and-an-automatic-writer-yields-to-a-hand.md`
/// true of a Set that has just been built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Authority {
    /// Yours alone. Nothing else writes this node's params.
    #[default]
    Manual,
    /// An agent proposes and you accept.
    Suggesting,
    /// An agent acts.
    Automatic,
}

impl Authority {
    /// Every level there is, in the order a control conventionally shows them —
    /// the order this enum declares them, most restrictive first.
    ///
    /// **A list is not a cycle**, on [`crate::deck::Blend::ALL`]'s terms: the
    /// console's `man / sug / auto` chip is an affordance built over the three
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`), and the cycle belongs to whoever draws it.
    pub const ALL: [Authority; 3] = [
        Authority::Manual,
        Authority::Suggesting,
        Authority::Automatic,
    ];

    /// **The lower-case word for this level**, which is what
    /// `karakuri_store::record::Record::Authority` carries and what
    /// `karakuri_operation::Authority::name` writes. The console's `man / sug /
    /// auto` is a node head's abbreviation for a reader and deliberately not
    /// this.
    ///
    /// A match rather than a table, for [`crate::transport::Sync::name`]'s
    /// reason: a level added to the enum does not compile until it has a name.
    pub fn name(self) -> &'static str {
        match self {
            Authority::Manual => "manual",
            Authority::Suggesting => "suggesting",
            Authority::Automatic => "automatic",
        }
    }

    /// The level a record's word names, or `None` for a word this build does
    /// not have — a stream from a newer build reaches a diagnostic rather than
    /// a parser that refuses the line, which is why the record carries a
    /// `String`.
    pub fn from_name(name: &str) -> Option<Authority> {
        Authority::ALL.into_iter().find(|a| a.name() == name)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SetError {
    /// **Two nodes answering to one name.** Refused rather than disambiguated:
    /// a name is an address, so choosing one of the two for the author would
    /// leave whatever was written against it pointing at the winner of a
    /// tie-break. A name derived from a procedure is disambiguated where it is
    /// derived; a collision reaching here is two names somebody wrote.
    #[error("two nodes are both called `{name}` — a name addresses one node in a Set")]
    DuplicateNodeName { name: String },
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
         hint: {hint}"
    )]
    Composition {
        l1: String,
        l4: String,
        missing: String,
        /// **Written where the refusal is decided, not assembled from a
        /// template.** A blanket hint saying `velocity` is synthesised, printed
        /// on a refusal *of* `velocity`, tells a regenerating model the spec is
        /// wrong — and that is what one said, because the walk that knows the
        /// real reason (the rule's source attribute is missing) dropped it and
        /// let the generic message speak.
        hint: String,
    },
    /// **wgpu refused something this engine asked it to build**, and the ask
    /// was one no check above caught.
    ///
    /// Every diagnostic beside this one is a refusal *this* compiler decided,
    /// with a sentence about the `.kir` that caused it. This one is the
    /// residue: a validation error from the driver's own checks, carried out
    /// rather than allowed to reach wgpu's uncaptured handler — which panics
    /// the thread that made the call, and at startup takes the process down.
    ///
    /// **It is a bug report, not a diagnostic.** Every instance is something
    /// the check pass should have refused with a sentence about the file, so
    /// the message says so: an author reading this has found a hole rather than
    /// made a mistake. Five distinct ones were found in a single milestone —
    /// `seed` in a fullscreen L4, a derived attribute in `spawn`, an L3
    /// evaluating a field the Set has none of, a field evaluating itself, and
    /// an amplified chain past the device's binding limit — and each was a
    /// process death before it was a refusal.
    #[error(
        "`{proc}` produced something this device refused, which is a compiler bug rather \
         than a mistake in the file: {detail}\n\
         hint: it should have been refused with a sentence about the `.kir`. Please report \
         the procedure and this message"
    )]
    Invalid { proc: String, detail: String },
    /// A caller and the field it evaluates are together over a cost ceiling.
    ///
    /// **Neither file is over on its own**, which is why this is here: a
    /// `field(p)` weighs nothing where a single procedure is estimated, so the
    /// ceiling each of them passed was applied to a figure that was missing the
    /// other. A field is inlined at every call site, so a marcher evaluating one
    /// forty-eight times pays for it forty-eight times.
    #[error(
        "`{caller}` with `{field}` inlined through `{slot}` is over budget: {detail}\n\
         hint: a field costs its caller once per evaluation — cut the field, the \
         evaluations, or the loop around them"
    )]
    FieldTooExpensive {
        caller: String,
        /// The slot the expensive field was reached through. **Named as well as
        /// the field**, because a procedure may declare several and the
        /// question the author has is which of its call sites to cut — which is
        /// a question about the name in its own file, not about the node the
        /// Set bound to it.
        slot: String,
        field: String,
        detail: String,
    },
    /// A pairing L2 that is not the first node in the chain.
    ///
    /// Its second input is a **simulation**, not whatever reached its position:
    /// there is one paired geometry and it is a source. A pairing node further
    /// down would be reading a raw source beside a deformed one, which is two
    /// different instants of the same material.
    #[error(
        "`{l2}` pairs two geometries and sits at position {at} in the chain\n\
         hint: a pairing L2 reads a *source*, so it has to be the first one. Put the \
         deformations after it"
    )]
    PairingNotFirst { l2: String, at: usize },
    /// An L2 with a geometry slot, in a Set that does not hold exactly two
    /// sources.
    ///
    /// **Two: the one the chain runs over, and the one the slot names.** Which
    /// is which is the edge's answer now rather than `--set` order, but how
    /// many there are is still this: a node declares one slot, and a Set with a
    /// third geometry has one nothing reads and nothing draws.
    #[error(
        "`{l2}` takes a second geometry and this Set has {sources}\n\
         hint: name exactly two L1s — the one the chain runs over and the one the slot is \
         bound to. A third would be a source no node reads"
    )]
    PairingArity { l2: String, sources: usize },
    /// A pairing L2 over a source that compacts.
    ///
    /// **The correspondence is the slot index**, and it is the same element in
    /// both sources only while nothing moves one. A `spawn` block allocates and
    /// a `kill()` makes the next step's scan compact the survivors down; either
    /// one and the pairing quietly matches each element with a stranger.
    #[error(
        "`{l2}` pairs by slot index and `{l1}` does not keep its elements at fixed slots\n\
         hint: a paired source must have no `spawn` block and no `kill()` — either one \
         compacts, and after a compaction element 5 of one source is not element 5 of the \
         other"
    )]
    PairingNotStatic { l2: String, l1: String },
    /// A pairing L2 whose far source cannot support a derivation the chain
    /// needs.
    ///
    /// Both sides are addressed with the same element struct — one chain runs
    /// over the pair — so a rule that applies to one applies to both, and one
    /// that the far side has nothing to derive from is a slot nothing fills.
    #[error(
        "`{l2}` pairs with `{l1}`, and `{attr}` is derived from something `{l1}` does not \
         emit\n\
         hint: both sides of a pairing carry the same attributes — emit the source attribute \
         in both, or stop consuming `{attr}`"
    )]
    PairingDerivation {
        l2: String,
        l1: String,
        attr: String,
    },
    /// A pairing L2 over two sources of different sizes.
    #[error(
        "`{l2}` pairs two geometries of {a} and {b} elements\n\
         hint: pairing is by slot index, so both sources have to be the same size — set one \
         `--capacity`, or declare the same default in both"
    )]
    PairingCapacity { l2: String, a: u32, b: u32 },
    /// **A declared slot that nothing in this Set binds**, of either type.
    ///
    /// Refused, and this is the refusal the whole notation is for. The rule it
    /// replaced was "there is exactly one, so it needs no name", which is
    /// exactly what capped fan-in at one — so filling an unbound slot from
    /// whatever happened to be lying around would put that rule back under a
    /// new spelling. A procedure says what it needs; the Set says what fills
    /// it; neither guesses.
    ///
    /// **One variant for both types**, unlike the two `EdgeTo…` refusals below:
    /// what is missing is the same fact whatever the slot takes, and the
    /// sentence that helps is the same sentence with the declared type read
    /// back out of it.
    #[error(
        "`{node}` declares `{slot} : {takes}` and nothing in this Set says what fills it\n\
         hint: bind it — `--edge {node}.{slot}=<node>`. This Set holds: {holds}"
    )]
    SlotUnbound {
        node: String,
        slot: String,
        /// The type the header wrote, so the hint names the sort of node that
        /// would fit rather than a generic one.
        takes: &'static str,
        holds: String,
    },
    /// An edge naming a slot the node it addresses does not declare.
    ///
    /// **The node is in this Set, so the statement is about it and is wrong.**
    /// An edge whose *node* names nothing here is a different matter — it is a
    /// statement about another Set, and is passed over rather than refused, on
    /// the same terms a `--param` naming a node this Set has not got is.
    #[error(
        "`{node}` declares no slot called `{slot}`\n\
         hint: an edge names a slot the procedure declared with `uses {slot} : \
         <Geometry|Field|Camera|Source>`{declares}"
    )]
    NoSuchSlot {
        node: String,
        slot: String,
        /// What it does declare, ready to be appended — empty where it declares
        /// nothing, since "and it declares none" reads better as silence than
        /// as an empty list.
        declares: String,
    },
    /// An edge whose far end names no node of this Set.
    #[error(
        "`{node}.{slot}` is bound to `{to}`, which is not a node of this Set\n\
         hint: this Set holds: {holds}"
    )]
    EdgeToUnknown {
        node: String,
        slot: String,
        to: String,
        holds: String,
    },
    /// An edge whose far end names a node that is not geometry.
    ///
    /// A slot declared `: Geometry` takes an L1, and nothing else in a Set has
    /// elements to read. A deformer has a buffer, but reading it would be
    /// reading whatever instant the chain had reached — which is the same
    /// reason a slot has to be bound at the head of the chain.
    #[error(
        "`{node}.{slot}` is bound to `{to}`, which is {layer}\n\
         hint: a slot declared `: Geometry` takes an L1 — the sources in this Set are: {sources}"
    )]
    EdgeToNotGeometry {
        node: String,
        slot: String,
        to: String,
        /// What the bound node is, article and all — "an L2", say. One field
        /// rather than two so that this variant stays under the size at which
        /// every `Result<_, SetError>` in the crate starts being reported as
        /// carrying a large error.
        layer: &'static str,
        sources: String,
    },
    /// An edge whose far end names a node that is not a field.
    ///
    /// **A sibling of [`SetError::EdgeToNotGeometry`] rather than one variant
    /// with a flag**, and the two sentences are why: what would help is a
    /// different list — the Set's sources against the Set's field — and a
    /// different statement about what the slot takes. Folding them together
    /// would be one struct carrying both lists and a discriminator to choose
    /// which half is a lie.
    #[error(
        "`{node}.{slot}` is bound to `{to}`, which is {layer}\n\
         hint: a slot declared `: Field` takes a `kind Field` procedure — this Set's is: {fields}"
    )]
    EdgeToNotField {
        node: String,
        slot: String,
        to: String,
        /// What the bound node is, article and all — "an L1", say.
        layer: &'static str,
        /// The field this Set holds, or a sentence saying it holds none: an
        /// edge pointing at the wrong node and a Set with nothing to point at
        /// are different mistakes, and this is where they read differently.
        fields: String,
    },
    /// An edge whose far end names a node that is not a camera.
    ///
    /// **A third sibling**, on the terms the second one set out: what helps is
    /// this Set's *cameras*, which is a different list again, and a statement
    /// about what a `: Camera` slot takes. The list is never empty — every Set
    /// holds at least the built-in — so this one has no "holds none" half to
    /// say.
    #[error(
        "`{node}.{slot}` is bound to `{to}`, which is {layer}\n\
         hint: a slot declared `: Camera` takes an L3 or the built-in camera — \
         this Set's are: {cameras}"
    )]
    EdgeToNotCamera {
        node: String,
        slot: String,
        to: String,
        /// What the bound node is, article and all — "an L4", say.
        layer: &'static str,
        /// The cameras this Set holds, by name.
        cameras: String,
    },
    /// An edge whose far end names a node that is not geometry, where the slot
    /// asked for a source's *identity* rather than its elements.
    ///
    /// **A fourth sibling, and not a second use of
    /// [`SetError::EdgeToNotGeometry`].** The two take the same kind of node
    /// and say different things about why: that one binds an element buffer to
    /// read beside the ones a node runs over, and this one binds the `u32` that
    /// says which geometry a chain instance is. An author who bound a mask's
    /// comparand to an L2 is not being told about buffers.
    #[error(
        "`{node}.{slot}` is bound to `{to}`, which is {layer}\n\
         hint: a slot declared `: Source` takes an L1 — it is the identity `source` is \
         compared against, and only a geometry has one. The sources in this Set are: {sources}"
    )]
    EdgeToNotSource {
        node: String,
        slot: String,
        to: String,
        /// What the bound node is, article and all — "an L2", say.
        layer: &'static str,
        sources: String,
    },
    /// Two edges binding one slot.
    ///
    /// Refused rather than last-one-wins, on [`SetError::DuplicateNodeName`]'s
    /// terms: a slot is one input and two answers to which geometry fills it is
    /// two different pictures, one of which is being discarded in silence.
    #[error(
        "`{node}.{slot}` is bound twice, to `{first}` and to `{second}`\n\
         hint: a slot is one input — remove one of the edges"
    )]
    SlotBoundTwice {
        node: String,
        slot: String,
        first: String,
        second: String,
    },
    /// A Set with no geometry at all.
    ///
    /// **Refused for the same reason an empty renderer list is**: a Set is a
    /// video source, and one with nothing to simulate has nothing for its
    /// renderers to draw.
    #[error("a Set needs at least one L1 — there is nothing to draw")]
    NoGeometry,
    // **There is no `NoField` here any more**, and nothing lost a refusal. It
    // said "this procedure evaluates `field(p)` and the Set holds no field",
    // which was the only way to ask for a field that was not there: the call
    // named no slot, so there was nothing earlier to check. A call names a slot
    // now, and a slot has to be bound — so the same file is turned away by
    // `SlotUnbound` before a shader is generated, pointing at the declaration
    // rather than at the call.
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
    TooManyInputs {
        l1: String,
        count: usize,
        max: usize,
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

/// **One geometry source, and the chain over it.**
///
/// A Set may hold several — `docs/ir-spec.md`, "Multiple L1 sources" — and what
/// makes that work is that the *chain* is per source rather than the geometry
/// being concatenated into one buffer. Two things force it and one falls out.
///
/// **Two sources kill independently**, so compaction is each source's own and
/// there is no shared live range to concatenate into. And with a chain instance
/// per source, **two sources need not agree on what they `emit`**: each
/// instance is compiled against the layout of the source it runs over, which is
/// a question that has no answer at all if one buffer has to hold both.
///
/// What falls out is that `source` need not be an element slot. A chain
/// instance knows statically which source it belongs to, so what varies with
/// the source is a uniform — and a `u32` on every element of every merged Set,
/// plus whatever alignment it drags behind it, is what carrying it would have
/// cost.
///
/// **The procedures are shared and the instances are not.** `--param L2:0:x`
/// addresses the first L2 *procedure*, and the Set writes that value into every
/// source's instance of it — the same relationship a spliced field's params
/// already have with their callers.
pub(crate) struct Source {
    /// **This source's hash salt**, which is *not* the Set's.
    ///
    /// `docs/ir-spec.md` moves the salt from per layer to per source, and the
    /// picture it buys is the point: two identical grids differ in colour by
    /// default rather than by being arranged to, because `hash1(seed)` differs
    /// between sources while `seed % 512u` does not.
    ///
    /// **Assigned where the caller had a value, derived where it did not.** The
    /// spec calls for a value chosen once when a source is added, recorded in
    /// the stream, and read back from there forever after — which a Set file now
    /// does, one `seed` record per geometry. So a Set that was ever saved comes
    /// back with its salts in hand and reordering the list no longer moves the
    /// colours. A bare `--set` has recorded nothing and gets [`derived_salt`],
    /// which the same paragraph licenses: *where it came from stops mattering
    /// once it is recorded*.
    salt: u32,
    /// **Which L1 procedure each of this source's simulations is**, as an index
    /// into the list the Set was built from, near one first.
    ///
    /// Recorded rather than assumed, because the assumption stopped holding: a
    /// Set used to draw `l1s[0]` and read `l1s[1]`, so walking the simulations
    /// and walking the procedures were the same walk. An `edge` names which
    /// geometry fills the slot, so the far one may be the first in the list —
    /// and everything addressed at `L1:<n>` means the *procedure's* ordinal.
    procedures: Vec<usize>,
    /// **The L1 node.** Every buffer, pipeline and bind group the simulation
    /// needs, and the spawn accumulator that decides what it creates.
    sim: Simulation,
    /// **The far geometry**, for a Set whose chain begins with an L2 that
    /// declares a `uses` slot — and *which* geometry it is came from the
    /// [`Edge`] that bound the slot, not from a position in the list.
    ///
    /// It belongs to this source rather than being one of its own, and that is
    /// what answers "is the second geometry drawn?" by construction: such a Set
    /// has one `Source`, one chain and one set of renderers, and the far
    /// simulation feeds that node and nothing else.
    ///
    /// `Option` rather than a list, because the language says one slot per
    /// node: a second `uses` is refused, and a node that took several would
    /// need a bound buffer apiece.
    paired: Option<Simulation>,
    /// **The L2 nodes, in chain order**, instantiated for this source. Each
    /// reads what the one before it wrote and writes its own buffer, so the
    /// geometry the renderers see is the last one's — or the simulation's, when
    /// there are none.
    deforms: Vec<Deform>,
    /// **One per L4 procedure**, compiled against *this* source's element
    /// layout and drawing this source's elements.
    renderers: Vec<Renderer>,
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

    /// **The geometry sources, and the chain over each** — see [`Source`].
    ///
    /// **At least one, and more than one is reachable**: `--set a.kir,b.kir,
    /// renderer.kir` builds two, and a pairing chain builds one source holding
    /// two simulations. Writing every path below against the list rather than
    /// against its first entry is what made accepting a second one a change
    /// where the sources are *made* and nowhere else.
    ///
    /// **What a second source is called is settled.** Every node has a name —
    /// the caller's where it wrote one, the procedure's own otherwise — and a
    /// hot-swap rebuild, a Set file, an MCP edit and the edit history all
    /// address a node by `(layer, index)` and carry that name beside it, so
    /// "the second geometry" is something every one of them can say. A
    /// procedure names the *slot* it takes rather than the node that fills it,
    /// and an `edge` binds the two — see
    /// `docs/adr/0152-a-kir-names-a-slot-and-the-set-names-the-nodes.md`.
    sources: Vec<Source>,
    /// **What each geometry is salted with**, in L1-procedure order — so a
    /// pairing Set has two entries and one [`Source`].
    ///
    /// Kept rather than asked of the sources, because the far side of a pairing
    /// is a `Simulation` inside a `Source` and a Set file records a salt per
    /// *geometry*. Held so that a writer can record what the Set is running at
    /// instead of deriving it a second time — see [`Set::source_salts`].
    source_salts: Vec<u32>,
    /// **What each node is called**, in node order — the same order [`Set::params`]
    /// and `ranges` are in, and for the same reason: one walk decides it.
    ///
    /// Every node has one. A name the caller wrote where it wrote one, and the
    /// procedure's own declared name everywhere else — which is a *type* name
    /// and collides when one procedure is used twice, so a caller that cares
    /// disambiguates before handing them over. What arrives here is refused if
    /// two are the same.
    names: Vec<String>,
    /// **Who may move each node**, in the same node order [`Set::names`],
    /// `params` and `ranges` are in — one walk decides all four.
    ///
    /// **Every node has one, and a Set that was built and never spoken for is
    /// every node at [`Authority::default`].** There is no `Option` here for
    /// [`Request::camera`](crate::swap::Request::camera)'s reason: a `None`
    /// would mean nothing, because "nobody has said" and "manual" are the same
    /// arrangement — the node is the operator's.
    ///
    /// **Not carried across a hot swap by being read out of the outgoing Set.**
    /// A rebuild states it, on
    /// [`Request::authorities`](crate::swap::Request::authorities); see there
    /// for what reading it off whatever happened to be live would cost.
    authorities: Vec<Authority>,
    /// **How many L1 *procedures* the Set was built from**, which is not
    /// `sources.len()` when the chain pairs: two procedures become one source
    /// with two simulations in it. `params` and `ranges` are per procedure and
    /// the addressing is per procedure, so this is the number both use — asking
    /// the source list gave an answer one too small, and every layer after L1
    /// shifted with it.
    l1_count: usize,

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
    /// keying by the node that declares the name was named as the fix when the
    /// refusal landed
    /// (`docs/adr/0052-a-parameter-is-keyed-by-its-layer-and-a-collision-is-refused.md`),
    /// and the error is gone with it.
    params: Vec<HashMap<String, f32>>,
    /// The declared `[min, max]` of every param, in the same node order as
    /// [`Set::params`]. **Kept because an interface needs it**: a published
    /// range is checked as a subset of the declared one, and a Set with no
    /// interface publishes every control over the range its procedure declared.
    /// Nothing else in the engine reads it — the ranges are the console's and
    /// the agent's, and no uniform write is clamped by them.
    ///
    /// **The keys and not their order.** A map keeps none, so the default
    /// interface's order comes from [`Set::declared_names`] and this is asked
    /// only for the range behind a key it already has — see [`Set::published`],
    /// which used to sort these keys and now looks each one up instead.
    ranges: Vec<HashMap<String, [f32; 2]>>,
    /// **The producer of the built-in camera's state.** Public because a
    /// `camera` record and a Set file both set it from outside; the six numbers
    /// it produces reach a renderer through the camera node below and never
    /// directly.
    ///
    /// **One, because a Set holds one built-in camera**: the last camera node
    /// is the orbit, whatever else the Set's files declared, and it is the only
    /// one whose six numbers come from outside. See [`Set::cameras`].
    pub camera: Orbit,
    /// **The camera edges, one per camera node**, in the order they are
    /// addressed as `L3:n`. Written every frame, derived on the GPU, and each
    /// read by the renderers bound to it — see [`crate::node::Camera`] for why
    /// the derivation is a pass rather than host arithmetic.
    ///
    /// **Never empty, and the last one is always the built-in orbit above.**
    /// The alternative — a built-in that exists only where no procedure does —
    /// is a camera that is a node in some Sets and a field on this struct in
    /// others, which is one fact with two shapes and was exactly what stopped
    /// an `edge` naming it.
    cameras: Vec<crate::node::Camera>,
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
    /// **The spliced fields' params, under their semantic names — one set per
    /// slot any node reaches one through.** Held here rather than on each node
    /// because a field has no node of its own to hold them — see
    /// [`crate::node::View::field_params`].
    ///
    /// A key names a *slot*, so this is the union over every binding rather
    /// than any one field's param list. It is a superset of what any one module
    /// holds and that is what it is for: each node writes the keys its own
    /// layout has, which is how one list serves five kinds of uniform struct
    /// without any of them knowing about the others.
    field_params: Vec<String>,
    /// The same params under the names they were **declared** with, one list
    /// per field, which is what an address names: `--param Field:1:ball`,
    /// `--bind layer=Field`, and a published control all use these, and only
    /// the uniform uses the others.
    field_declared: Vec<Vec<String>>,
    /// **Which field fills each Field slot, by node.** `(node, slot, field
    /// ordinal)`, where the node and the ordinal are both indices into
    /// [`Set::params`]'s node order.
    ///
    /// **Kept rather than re-derived**, because a slot's spelling is the
    /// *caller's* and two callers may spell one name for two different fields:
    /// `field_params` says which keys exist and only this says which map each
    /// one reads. Resolving it a second time from the edges would be the second
    /// home for a fact — the shape this file has already been wrong about
    /// twice — and the edges are not kept anyway.
    field_bound: Vec<(usize, String, usize)>,
    /// **Which geometry fills each Source slot, by node.** `(node, slot,
    /// geometry ordinal)`, where the node is an index into [`Set::params`]'s
    /// node order and the ordinal indexes [`Set::source_salts`].
    ///
    /// **The ordinal and not the salt**, which is the one thing to keep right
    /// here: a salt is assigned once and the list is in hand, so storing the
    /// index costs nothing and leaves one home for the value. Storing the salt
    /// would be a second copy of it, and the two would agree until something
    /// re-salted a source.
    ///
    /// Kept rather than re-derived on the frame path for [`Set::field_bound`]'s
    /// reason: a slot's spelling is the declaring node's, so two nodes may each
    /// call one `only` and mean different geometries.
    source_bound: Vec<(usize, String, usize)>,
    /// How many `kind Field` procedures this Set holds. Not
    /// `!field_params.is_empty()`: a field may declare no `param`, and the two
    /// questions are different ones.
    field_count: usize,
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

/// **Why a bare-name parameter write was refused.**
///
/// A [`ParamWrite`] with no address moves every node that declares the key —
/// `docs/ir-spec.md`'s *one control per key, not one per declaration*, which is
/// what the default published interface is made of and therefore what nearly
/// every control a surface draws is. An authority is per **node**
/// (`docs/adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md`).
/// So where one key is declared by nodes that are not under one authority, a
/// bare name is one control over two arrangements: granting an agent one
/// renderer would grant it, through that control, a renderer the operator kept.
///
/// **Refused whole rather than landed on the nodes that permit it.** A control
/// that moved three renderers of four and looked like it moved all of them is
/// exactly
/// `docs/principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md`, and
/// the addressed write is never refused — so what is taken away is one spelling
/// and not the reach. Argued in
/// `docs/adr/0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md`.
///
/// **The sentence is here and nowhere else**, which is
/// `docs/principles/0090-a-surface-offers-it-never-decides.md`:
/// a `--param`, a published control and a `param` record are the same wildcard
/// and reach it through the one entry point, [`Set::write_param`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error(
    "`{key}` is declared by nodes that are not under one authority — {landing} — and a bare \
     name writes every node that declares it\n\
     hint: an authority is per node, so this write would cross a grant. Address the node \
     it is meant for, or put the nodes it lands on under one authority"
)]
pub struct CrossesAuthority {
    /// The parameter's name — what the caller wrote, so the sentence names the
    /// control the operator or the model actually asked for.
    pub key: String,
    /// **Every node the write lands on and the authority it is under**, in node
    /// order: `L1:0 manual, L4:0 automatic`.
    ///
    /// The whole landing rather than the minority. Three nodes under three
    /// authorities have no majority for the odd one out to disagree with, and
    /// the question a reader has is *which nodes is this control over*, which
    /// only the whole list answers.
    pub landing: String,
}

impl CrossesAuthority {
    /// The refusal a bare-name write of `key` earns, or `None` where every node
    /// it lands on is under one authority.
    ///
    /// **A Set that declares `key` nowhere lands on nothing and is not a
    /// refusal**: an empty landing is uniform, and the caller's answer is the
    /// `0` it already prints *no parameter named* for. A name a regenerated
    /// artifact no longer has should not take the show down, and it does not
    /// start doing so by way of an authority it never had.
    ///
    /// **Taken apart from the walk that finds the nodes**, which needs a built
    /// `Set` and therefore a device. This is the decision and it is checked
    /// without one —
    /// `docs/adr/0130-a-wrapper-that-needs-a-gpu-does-not-excuse-the-decision-inside-it.md`.
    fn over(key: &str, landing: &[(Kind, u32, Authority)]) -> Option<CrossesAuthority> {
        let first = landing.first()?.2;
        if landing.iter().all(|(_, _, held)| *held == first) {
            return None;
        }
        let named: Vec<String> = landing
            .iter()
            .map(|(layer, index, held)| format!("{layer:?}:{index} {}", held.name()))
            .collect();
        Some(CrossesAuthority {
            key: key.to_string(),
            landing: named.join(", "),
        })
    }
}

/// **One binding of a procedure's declared input slot to a node of this Set.**
///
/// `uses far : Geometry` says what a procedure takes and refuses to say where
/// it comes from — a `.kir` that named a node would be coupled to one Set and
/// would stop being a library part. This is the other half, and it belongs
/// where the *use* is recorded: an `edge` record in a Set file, written from
/// the command line as `--edge morph.far=sphere_shell`.
///
/// **Both ends are names**, because that is what a Set has to point with: an
/// address moves when the list is reordered, which is the property that made
/// `--set` order an unwritable answer in the first place. Every node has a name
/// whether or not one was written — see [`Set::node_names`] — so both ends
/// always resolve to something.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    /// The node that declares the slot.
    pub node: String,
    /// What that node's procedure calls the slot, from its `uses` declaration.
    pub slot: String,
    /// The node bound to it.
    pub to: String,
}

/// What a caller decided about the nodes of a Set it is building: what each one
/// is called, and which node fills each declared input slot.
///
/// **Names and edges travel together because neither is answerable alone.** An
/// edge is written in terms of names, and a name nobody wrote is derived here
/// rather than in a caller — so a caller that supplied the two apart would be
/// resolving one against a spelling it does not have. They are also the same
/// *kind* of fact: the Set's answer rather than the file's, restated on every
/// rebuild for the reason `Request::bindings` gives.
///
/// **Per layer, in the same shape the procedures themselves are passed in.** The
/// node *order* belongs to [`Set::build_many`] — `slot_of` and `nodes_of` decide
/// it — so a caller that laid names out in that order would be a second place
/// for a fact this file has already had wrong twice.
///
/// **Only what was written.** A `None` — or an entry past the end — is a node
/// nobody named, and [`Set::build_many`] derives one for it from the procedure,
/// disambiguating against every name already taken. Deriving in a caller as well
/// would be two places for one fact; ask [`Set::node_names`] for what a node
/// ended up called.
#[derive(Debug, Clone, Copy, Default)]
pub struct Wiring<'a> {
    pub l1s: &'a [Option<String>],
    pub l2s: &'a [Option<String>],
    /// **A list, on the same terms as the renderers**, and one entry longer
    /// than the procedures where a Set has none: the built-in camera is a node
    /// and a caller may name it like any other. What it is called when nobody
    /// does is [`BUILTIN_CAMERA`].
    pub l3s: &'a [Option<String>],
    pub l4s: &'a [Option<String>],
    /// **A list, on the same terms as the renderers.** A Set holds as many
    /// fields as the files it was given declare, and each of them is a node
    /// with a name for an edge to point at — so the `Option<&str>` this
    /// replaced could only ever name the one there was.
    pub fields: &'a [Option<String>],
    /// **Every edge the caller was given, including ones about other Sets.**
    ///
    /// A deck is several Sets and a flag is one command line, so an edge naming
    /// a node this Set has not got is a statement about a different one and is
    /// passed over — the same rule a `--param` addressed at a node this Set has
    /// not got follows. What is *not* passed over is an edge whose node is
    /// here: then the statement is about this Set and every part of it has to
    /// resolve.
    pub edges: &'a [Edge],
}

/// The first value that appears twice, if any.
fn first_duplicate(names: &[String]) -> Option<String> {
    names
        .iter()
        .enumerate()
        .find(|(at, name)| names[..*at].contains(name))
        .map(|(_, name)| name.clone())
}

/// What became of a [`Set::bind`].
///
/// **Two ways to fail, and they are different mistakes.** A binding names a
/// param and, when its source is a published control, a control — so it can
/// miss on either, and the sentence that helps points at the one it missed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bound {
    /// Attached, and riding from the next frame.
    Yes,
    /// No node this binding covers declares a scalar param by that name.
    NoSuchParam,
    /// A `control:` source naming something this Set's interface does not
    /// publish. The param exists; the thing that was to drive it does not.
    NoSuchControl,
}

impl Bound {
    /// Whether the binding is riding. For callers that only need the yes/no —
    /// a test asserting the param was there, mostly. A caller that *reports*
    /// wants the variant, because "which one" is the whole content of the
    /// message.
    pub fn attached(self) -> bool {
        self == Bound::Yes
    }
}

/// **What [`Set::validate`] worked out, on the way to deciding the Set is
/// buildable.**
///
/// Opaque on purpose. It is not a result a caller asked for — the answer to
/// "is this Set legal?" is the `Result`, and everything in here is the working
/// [`Set::build_many`] would otherwise have to do a second time: the node
/// names, which node fills each declared slot, which geometry is the far side
/// of a pairing, what each source is salted with, and which attributes are
/// synthesised for each chain.
///
/// **It exists so the checks have one home.** A `validate` that returned
/// nothing would leave `build_inner` re-deriving every one of these to build
/// against, and a re-derivation is one edit away from being a re-check — which
/// is the defect this split was made to remove rather than to spread.
pub struct Plan<'a> {
    /// Every node's name, in node order: the geometries, the deformations, the
    /// cameras, the renderers, the fields.
    names: Vec<String>,
    /// One per camera node — a procedure, or `None` for the built-in orbit,
    /// which is last and always present.
    cameras: Vec<Option<&'a Checked>>,
    /// Where the cameras sit in `names`.
    camera_range: std::ops::Range<usize>,
    /// Node index, slot name, and the field ordinal bound to it.
    field_bound: Vec<(usize, String, usize)>,
    /// Node index and the camera ordinal bound to its `Camera` slot.
    camera_bound: Vec<(usize, usize)>,
    /// Node index, slot name, and the `l1s` index bound to it.
    source_bound: Vec<(usize, String, usize)>,
    /// The geometry a pairing L2's slot names, as an index into `l1s`.
    far_at: Option<usize>,
    /// The geometries a chain is instantiated over — every one the pairing
    /// edge did not name.
    heads: Vec<usize>,
    /// What each geometry is salted with, in `l1s` order.
    source_salts: Vec<u32>,
    /// The attributes synthesised for each head's chain, in `heads` order.
    derived: Vec<Vec<karakuri_ir::Attr>>,
    /// **The material this plan is about**, kept so that a question about it
    /// can be answered from the plan alone — [`Plan::element_storage`] is the
    /// one that asks. Handing those slices in a second time instead would let a
    /// caller cost one Set against another Set's plan, and the answer would look
    /// exactly as plausible as a right one. Only the layers that hold elements
    /// are kept: a camera, a renderer and a merge allocate none.
    l1s: Vec<(&'a Checked, u32)>,
    l2s: Vec<&'a Checked>,
    /// The `kind Field` procedures `field_bound` indexes into. Kept because a
    /// bound field is spliced into a node's shader, and a shader is where an
    /// element layout comes from.
    fields: Vec<&'a Checked>,
}

/// **One node instance's element storage, and which node of the Set it is an
/// instance of.**
///
/// A *node* is a procedure in the Set; an *instance* is that procedure running
/// over one geometry. A Set over two sources instantiates its chain of
/// deformations twice, so two entries here can name the same node — and they
/// are not the same figure, since each is sized against the source it runs
/// over. See [`Plan::element_storage`], which returns these, and
/// [`Set::element_storage`], which is the same list read off a built Set with
/// the node dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedStorage {
    /// Index into [`Plan::node_names`] — the node this instance is an instance
    /// of. **Not an address a caller can build a name from arithmetic**: node
    /// order is [`Set::validate`]'s own and the name is what a reader should
    /// print, which is why this is an index into a list rather than a layer and
    /// an ordinal.
    pub node: usize,
    /// What that instance will allocate.
    pub storage: ElementStorage,
}

impl<'a> Plan<'a> {
    /// **What every node of the Set will allocate to hold elements** — before
    /// anything is compiled, with no adapter and no device.
    ///
    /// **In the order [`Set::element_storage`] reports the same Set**: per
    /// source, the simulation, the far simulation it pairs with where there is
    /// one, then the chain of deformations — the order the elements themselves
    /// travel in. The two lists agree entry for entry, which is what a `mod gpu`
    /// test in `tests/storage.rs` asserts and what makes this figure worth
    /// publishing at all.
    ///
    /// **Nothing here re-derives a size.** The per-node arithmetic is
    /// `crate::storage`, called by this and by the constructors that allocate;
    /// what this walk contributes is the four inputs no single procedure has —
    /// which chains exist, what stride each node writes at (the chain's, not the
    /// procedure's own `emit` list), what an amplifier did to the count below
    /// it, and whether an L1 pays for a compaction scan. A figure taken from one
    /// `.kir` was missing three of those, was 85% low, and was withdrawn —
    /// `docs/adr/0116-stage-four-stops-claiming-the-byte-figure.md`.
    ///
    /// **It generates the shaders to ask them.** An element layout is decided
    /// by the generator and by nothing else, so the alternative is a second
    /// implementation of the layout rules — the defect this is written to avoid.
    /// Generation is string building and touches no hardware; the cost that
    /// makes a build slow is the pipeline compilation this does not do.
    ///
    /// **Element storage and not device memory**, on the terms
    /// [`ElementStorage`] sets out: render targets, uniform blocks, the counts
    /// block and the scan's block-sum pyramid are all outside it. Anything
    /// reporting this to a person owes them that sentence too.
    pub fn element_storage(&self) -> Vec<PlannedStorage> {
        // **The same lookup `build_inner` makes**, and for the same reason: a
        // bound field is spliced into the node's shader, so a node built with
        // one and a node costed without it are two different shaders and can be
        // two different strides.
        let bound_at = |at: usize| -> Vec<(&str, &Checked)> {
            self.field_bound
                .iter()
                .filter(|(node, _, _)| *node == at)
                .map(|(_, slot, ordinal)| (slot.as_str(), self.fields[*ordinal]))
                .collect()
        };
        let simulation = |at: usize, derived: &[karakuri_ir::Attr]| {
            let (l1, capacity) = self.l1s[at];
            let shader = generate_l1(l1, derived, &bound_at(at));
            PlannedStorage {
                node: at,
                storage: SimulationStorage::of(
                    capacity,
                    shader.element_layout.stride,
                    shader.compacted,
                )
                .total(),
            }
        };

        let mut out = Vec::new();
        for (head, &at) in self.heads.iter().enumerate() {
            let (l1, capacity) = self.l1s[at];
            // **This chain's synthesised attributes**, which widen every
            // element under this source. Per head, because two sources emitting
            // different things need different slots derived.
            let derived = &self.derived[head];
            out.push(simulation(at, derived));
            // **Charged once per head and not once per Set**, because that is
            // what `build_inner` builds: a chain instantiated over a second
            // geometry reads a far side generated against *its* derived list,
            // so there are two simulations rather than one shared.
            //
            // Every pairing Set today has exactly one head — `SetError::
            // PairingArity` refuses one that is not two geometries, and one of
            // the two is the far side — so this is a distinction nothing can
            // currently see. It is written the way the build walks anyway,
            // because the arity rule is somewhere else and a figure that agreed
            // with the build only by borrowing that rule is the shape this
            // whole file keeps paying for.
            if let Some(far_at) = self.far_at {
                out.push(simulation(far_at, derived));
            }

            // The chain, walked exactly as `build_inner` walks it: what reaches
            // a node decides the struct it writes, and an amplifier changes the
            // count for everything below.
            let mut upstream: Vec<karakuri_ir::Attr> = l1.emit.clone();
            let mut synthetic = Synthetic::NONE;
            let mut chain_capacity = capacity;
            for (k, l2) in self.l2s.iter().enumerate() {
                let far = self
                    .far_at
                    .filter(|_| l2.geometry_slot().is_some())
                    .map(|far_at| self.l1s[far_at].0.emit.as_slice());
                let shader = generate_l2(
                    l2,
                    &upstream,
                    synthetic,
                    derived,
                    far,
                    &bound_at(self.l1s.len() + k),
                );
                // Saturating for the reason `Deform::build` saturates: a chain
                // of amplifiers is a product a `u32` can be walked off the end
                // of, and a wrapped count here would report a Set as cheaper
                // than the one the device refuses to build.
                chain_capacity = chain_capacity.saturating_mul(shader.amplify.unwrap_or(1));
                out.push(PlannedStorage {
                    node: self.l1s.len() + k,
                    storage: DeformStorage::of(
                        chain_capacity,
                        shader.element_layout.stride,
                        shader.amplify.is_some(),
                    )
                    .total(),
                });
                upstream = shader.emits;
                synthetic = shader.synthetic;
            }
        }
        out
    }

    /// **What each node is called**, in node order: the geometries, the
    /// deformations, the cameras, the renderers, the fields.
    ///
    /// The names a built Set answers [`Set::node_names`] with — derived here,
    /// which is why a caller can print them before there is a Set to ask.
    pub fn node_names(&self) -> &[String] {
        &self.names
    }
}

/// **`capacity` against the range the L1 artifact declares.**
///
/// Here rather than in `Simulation::build`, where it used to be. The range is
/// that node's own, but the comparison is between two integers and a
/// constructor that takes a `&wgpu::Device` was the only way to reach it — so
/// being told 999999 is above a declared 262144 cost an adapter and a compiled
/// pipeline. `Simulation::build` does not look any more, and is infallible
/// because of it: nothing it can be handed is refusable, which is the type
/// system saying this rule has one home.
fn capacity_in_range(l1: &Checked, capacity: u32) -> Result<(), SetError> {
    let range = l1
        .capacity
        .ok_or_else(|| SetError::NoCapacity(l1.name.clone()))?;
    if !range.contains(capacity) {
        return Err(SetError::Capacity {
            proc: l1.name.clone(),
            requested: capacity,
            min: range.min,
            max: range.max,
        });
    }
    Ok(())
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
        Set::build_many(
            device,
            queue,
            &[(l1, capacity)],
            &[],
            &[],
            &[],
            &[l4],
            Layering::Overdraw,
            seed_salt,
            // A pair assigns nothing, so the one source is salted from the
            // Set's seed and its ordinal — which for source 0 is that seed
            // unchanged.
            &[],
            // A pair names nothing, so both nodes are called what their
            // procedures are.
            Wiring::default(),
        )
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
    // Eleven, where clippy's line is seven. Four of them are the chain — an L1,
    // a list of L2s, an optional L3, a list of L4s — and grouping them into a
    // struct would be a second spelling of "the nodes of a Set", which is what
    // the Set being returned already is. Three more are what a caller knows
    // about the nodes it is handing over rather than about the nodes
    // themselves: how they layer, what they are salted with, and how they are
    // wired — what each is called and which of them fills each declared input
    // slot, which travel together as one [`Wiring`] because an edge is written
    // in terms of the names beside it.
    /// **`salts` is one hash salt per geometry, and only what was assigned.**
    /// A `None` — or an entry past the end — is a source nobody salted, and
    /// [`derived_salt`] gives it one from `seed_salt` and its ordinal. Same
    /// rule as [`Wiring`] and for the same reason: a caller supplies what it
    /// knows, and what it did not supply is filled in here rather than in two
    /// places at once. `seed_salt` stays because it is still the Set's own —
    /// what an L3 reads, and what an unsalted source is derived from.
    ///
    /// **Everything this function builds happens inside a validation error
    /// scope**, which is the difference between a diagnostic and a dead
    /// process.
    ///
    /// wgpu's default answer to a validation error is to report it to an
    /// uncaptured handler that *panics the thread that made the call*. On the
    /// swap worker that is a `SetError::Panicked` — recoverable, and the
    /// running Set survives; at startup there is no `catch_unwind` above it and
    /// the process exits. A scope changes where the error goes: per the WebGPU
    /// rules an error a scope captures is **not** reported to the uncaptured
    /// handler, so it arrives here as a value.
    ///
    /// **This is a net, not a plan.** Everything it catches is something the
    /// check pass should have refused with a sentence about the `.kir`, and
    /// `SetError::Invalid` says so. What the net buys is that finding the next
    /// hole costs a diagnostic rather than a crash — five were found in one
    /// milestone, each of them a process death first and a refusal afterwards.
    #[allow(clippy::too_many_arguments)]
    pub fn build_many(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        l1s: &[(&Checked, u32)],
        l2s: &[&Checked],
        l3s: &[&Checked],
        fields: &[&Checked],
        l4s: &[&Checked],
        layering: Layering,
        seed_salt: u32,
        salts: &[Option<u32>],
        wiring: Wiring<'_>,
    ) -> Result<Set, SetError> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let built = Set::build_inner(
            device, queue, l1s, l2s, l3s, fields, l4s, layering, seed_salt, salts, wiring,
        );
        // **Popped on every path**, which is why the body is a second function
        // rather than this one: it returns early in a dozen places, and a scope
        // left on the stack would catch the *next* build's errors and report
        // them against this one. Since wgpu 30 the guard's `Drop` pops it too,
        // so the split no longer *prevents* that — what it still buys is the
        // captured error as a value, which only an explicit `pop()` yields.
        let captured = pollster::block_on(scope.pop());

        match (built, captured) {
            // **Our own refusal wins.** Where both fired, ours is the one with
            // a sentence about the file in it, and the driver's is the same
            // fact stated in the driver's terms.
            (Err(e), _) => Err(e),
            // **The Set is dropped rather than returned.** A build that
            // produced a validation error produced a resource that does not
            // exist, and every handle naming it is one wgpu will refuse again
            // at the first draw — silently, since by then nothing is watching.
            (Ok(_), Some(e)) => Err(SetError::Invalid {
                proc: l1s
                    .first()
                    .map_or_else(String::new, |(p, _)| p.name.clone()),
                detail: e.to_string(),
            }),
            (Ok(set), None) => Ok(set),
        }
    }

    /// **Every refusal a Set can decide without a device**, and the working
    /// that produced them.
    ///
    /// This is the whole of [`Set::build_many`]'s check pass. Of the refusals
    /// a build can return, three need hardware — [`SetError::Invalid`] is a
    /// wgpu validation error captured from a scope, [`SetError::TooManyElements`]
    /// is a comparison against `device.limits()`, and [`SetError::Panicked`] is
    /// a build that died on the swap worker — and every other variant is a
    /// statement about the `.kir` files, the capacities and the wiring, all of
    /// which are in hand here.
    ///
    /// **`build` reaches these rules by calling this**, and re-checks nothing:
    /// what it gets back is a [`Plan`], and it builds against that. A copy of
    /// any check below living on the build path would be a second home for a
    /// rule, which is the shape this file has already paid for twice.
    ///
    /// The payoff is that asking "is this Set legal?" costs no adapter: a
    /// caller with two `.kir` files and a wiring can be told before anything is
    /// compiled, and a test that asserts a refusal stops paying for a pipeline
    /// it never reaches.
    #[allow(clippy::too_many_arguments)]
    pub fn validate<'a>(
        l1s: &[(&'a Checked, u32)],
        l2s: &[&'a Checked],
        l3s: &[&'a Checked],
        fields: &[&'a Checked],
        l4s: &[&'a Checked],
        layering: Layering,
        seed_salt: u32,
        salts: &[Option<u32>],
        wiring: Wiring<'_>,
    ) -> Result<Plan<'a>, SetError> {
        let Some(&(first_l1, _)) = l1s.first() else {
            return Err(SetError::NoGeometry);
        };
        if l4s.is_empty() {
            return Err(SetError::NoRenderer {
                l1: first_l1.name.clone(),
            });
        }
        if layering == Layering::Composite && l4s.len() > crate::deck::MAX_SLOTS {
            return Err(SetError::TooManyInputs {
                l1: first_l1.name.clone(),
                count: l4s.len(),
                max: crate::deck::MAX_SLOTS,
            });
        }
        // **Names first, because the edges are written against them.** A name
        // arrives per layer,
        // because the node *order* is this function's own — `slot_of` and
        // `nodes_of` decide it — and a caller that laid the names out in node
        // order would be the second place that fact lives. This file has paid
        // for that twice.
        //
        // A procedure's own declared name where nothing named the node, so
        // every node has one: a node nothing can address is a node nothing can
        // point a mask, a `--param` or a rebuild at.
        let given = |at: usize, from: &[Option<String>]| from.get(at).cloned().flatten();
        // **The camera nodes**: one per L3 procedure, and then the built-in
        // orbit, which every Set has.
        //
        // `None` is that orbit, and it is a *node* here rather than a field on
        // the Set for one reason: an edge points at names, so a camera nothing
        // can name is a camera no renderer can be bound to. **Unconditional,
        // and last.** Conditional on there being no procedure is the shape this
        // whole commit removes — "if there is exactly one, use it" wearing a
        // different hat — and it would make a renderer's right to draw from the
        // orbit depend on which *other* files the Set was given. Last, so that
        // `L3:0` is the first procedure where there is one and every address a
        // Set file ever recorded still names what it named.
        //
        // A Set with no procedure at all therefore addresses one camera at
        // `L3:0` where it addressed none, and [`Set::params`] grows an entry
        // beside it in the same motion — see where the maps are built, and
        // `docs/ir-spec.md`, "Several cameras".
        let cameras: Vec<Option<&Checked>> = l3s
            .iter()
            .copied()
            .map(Some)
            .chain(std::iter::once(None))
            .collect();
        let wanted: Vec<(Option<String>, Option<&Checked>)> = l1s
            .iter()
            .enumerate()
            .map(|(at, (l1, _))| (given(at, wiring.l1s), Some(*l1)))
            .chain(
                l2s.iter()
                    .enumerate()
                    .map(|(at, n)| (given(at, wiring.l2s), Some(*n))),
            )
            .chain(
                cameras
                    .iter()
                    .enumerate()
                    .map(|(at, n)| (given(at, wiring.l3s), *n)),
            )
            .chain(
                l4s.iter()
                    .enumerate()
                    .map(|(at, n)| (given(at, wiring.l4s), Some(*n))),
            )
            .chain(
                fields
                    .iter()
                    .enumerate()
                    .map(|(at, n)| (given(at, wiring.fields), Some(*n))),
            )
            .collect();
        // **The procedures, in node order**, so that an edge resolved against a
        // name can ask the node it found what it declares. `wanted` is consumed
        // deriving the names below, and this is the half of it the edges need.
        //
        // **`None` is the built-in camera**, which is the one node in a Set
        // that is not a procedure: it declares no slots, no params and no
        // attributes, so every walk below reads it as a node with nothing to
        // say rather than as a special case.
        let nodes: Vec<Option<&Checked>> = wanted.iter().map(|(_, n)| *n).collect();
        // **Every written name is taken first, and the rest are derived
        // against what is already taken.** Doing it in one pass would let a
        // derived `lens` claim the name a written one further down the list
        // asked for, and the written one is the address somebody chose.
        let mut taken: Vec<String> = wanted.iter().filter_map(|(n, _)| n.clone()).collect();
        if let Some(dup) = first_duplicate(&taken) {
            return Err(SetError::DuplicateNodeName { name: dup });
        }
        let names: Vec<String> = wanted
            .into_iter()
            .map(|(name, node)| match name {
                Some(written) => written,
                // **Derived here and nowhere else.** A procedure's name is a
                // *type* name — two renderers over one field are two nodes and
                // one `proc lens` — so the second use is told apart the way
                // `scratch` tells two files with one basename apart. Deriving
                // it in a caller as well would be the second place a fact
                // lives, which is the shape this file has been wrong about
                // twice; a caller that wants to know what a node ended up
                // called asks [`Set::node_names`].
                None => {
                    // **The built-in camera's own**, since it has no procedure
                    // to take one from — and it is disambiguated against
                    // everything else exactly as a procedure's is, so a Set
                    // holding a `proc orbit` beside it still has two names.
                    let mut candidate =
                        node.map_or(BUILTIN_CAMERA, |n| n.name.as_str()).to_string();
                    let mut at = 1;
                    while taken.contains(&candidate) {
                        at += 1;
                        candidate =
                            format!("{}-{at}", node.map_or(BUILTIN_CAMERA, |n| n.name.as_str()));
                    }
                    taken.push(candidate.clone());
                    candidate
                }
            })
            .collect();

        // **What a name points at**, over the list just derived: node order is
        // the geometries, the deformers, the camera, the renderers, the field,
        // so a name found below `l1s.len()` is a geometry and one at
        // `l1s.len() + k` is the k-th deformer. Asked here rather than through
        // [`Set::node_named`], which wants a built Set and there is not one yet.
        let node_at = |name: &str| names.iter().position(|n| n == name);
        let geometry_at = |name: &str| node_at(name).filter(|at| *at < l1s.len());
        let holds = || names.join(", ");
        let sources = || names[..l1s.len()].join(", ");
        // **Last in the order**, after the renderers — see the chain `wanted`
        // was built from. A run of positions rather than one, and derived from
        // the end rather than counted from the start: everything before it is
        // already what the arithmetic above is written in terms of.
        let field_range = names.len() - fields.len()..names.len();
        // **Between the deformations and the renderers**, where a camera's
        // place in the node order has always been. A run rather than a
        // position, and never empty.
        let camera_range = l1s.len() + l2s.len()..l1s.len() + l2s.len() + cameras.len();
        // Which camera a node index is, counting from zero — the ordinal
        // `L3:1` names, and `None` for a node that is not one.
        let camera_ordinal =
            |at: usize| camera_range.contains(&at).then(|| at - camera_range.start);
        let holds_cameras = || names[camera_range.clone()].join(", ");
        // Which field a node index is, counting from zero — the ordinal
        // `--param Field:1:x` names, and `None` for a node that is not one.
        let field_ordinal = |at: usize| field_range.contains(&at).then(|| at - field_range.start);
        let holds_fields = || match field_range.is_empty() {
            true => "none — this Set holds no `kind Field` procedure".to_string(),
            false => names[field_range.clone()].join(", "),
        };
        // What one node is, for a refusal that has to say what was bound where
        // a slot wanted something else.
        let layer_of = |at: usize| -> &'static str {
            if at < l1s.len() {
                "an L1"
            } else if at < l1s.len() + l2s.len() {
                "an L2"
            } else if camera_range.contains(&at) {
                "a camera"
            } else if field_range.contains(&at) {
                "a field"
            } else {
                "an L4"
            }
        };

        // **Every edge whose node is in this Set has to resolve.** One whose
        // node is not is a statement about another Set of the deck — a flag is
        // one command line and a deck is several Sets — and is passed over on
        // the terms a `--param` addressed at an absent node already follows.
        for edge in wiring.edges {
            let Some(at) = node_at(&edge.node) else {
                continue;
            };
            // **Asked of the node, whatever kind it is.** This used to look
            // only at the L2s, because a slot was a geometry slot and a
            // geometry slot is L2's alone; a Field slot is legal on four kinds,
            // so an edge naming a renderer's is an ordinary edge and refusing
            // it as "declares no slot" would be a refusal about the wrong
            // thing.
            // **The built-in camera declares nothing**, which is the honest
            // answer rather than a special case: it is a node with no
            // procedure, so an edge naming a slot on it is an edge naming a
            // slot nothing declares.
            let declared: &[karakuri_ir::typed::Slot] =
                nodes[at].map_or(&[], |n| n.uses.as_slice());
            if !declared.iter().any(|slot| slot.name == edge.slot) {
                return Err(SetError::NoSuchSlot {
                    node: edge.node.clone(),
                    slot: edge.slot.clone(),
                    declares: match declared.is_empty() {
                        true => String::new(),
                        false => format!(
                            "; `{}` declares {}",
                            edge.node,
                            declared
                                .iter()
                                .map(|s| format!("`{}`", s.name))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    },
                });
            }
        }

        // **A chain with a geometry slot is one source made of two
        // simulations**, not two sources: the far one feeds the slot and
        // nothing else, which is what makes "is it drawn?" a question with no
        // place to be asked.
        let pairing = l2s.iter().position(|n| n.geometry_slot().is_some());
        // **Which geometry the slot is bound to**, as an index into `l1s`.
        // `None` where no node declares a slot. This used to be `l1s[1]` and
        // was written nowhere at all — reordering the command line silently
        // changed the picture, which is the whole reason an edge exists.
        let far_at: Option<usize> = match pairing {
            None => None,
            Some(at) => {
                let l2 = l2s[at];
                let node = names[l1s.len() + at].clone();
                let slot = l2
                    .geometry_slot()
                    .expect("`pairing` is the position of a node that declares a slot")
                    .to_string();
                if at != 0 {
                    return Err(SetError::PairingNotFirst {
                        l2: l2.name.clone(),
                        at,
                    });
                }
                if l1s.len() != 2 {
                    return Err(SetError::PairingArity {
                        l2: l2.name.clone(),
                        sources: l1s.len(),
                    });
                }
                // **The slot as well as the node.** One node may declare a
                // geometry slot and a field slot at once, and a filter on the
                // node alone read the field's edge as a second binding of this
                // one — a Set refused for being wired twice when it was wired
                // once each.
                let mut bound = wiring
                    .edges
                    .iter()
                    .filter(|e| e.node == node && e.slot == slot);
                let Some(edge) = bound.next() else {
                    return Err(SetError::SlotUnbound {
                        node,
                        slot,
                        takes: karakuri_ir::SlotTy::Geometry.name(),
                        holds: holds(),
                    });
                };
                if let Some(second) = bound.next() {
                    return Err(SetError::SlotBoundTwice {
                        node,
                        slot,
                        first: edge.to.clone(),
                        second: second.to.clone(),
                    });
                }
                let Some(far_at) = geometry_at(&edge.to) else {
                    return Err(match node_at(&edge.to) {
                        // In the Set and not a geometry: the layer it *is* is
                        // the useful half of the sentence.
                        Some(other) => SetError::EdgeToNotGeometry {
                            node,
                            slot,
                            to: edge.to.clone(),
                            layer: match other < l1s.len() + l2s.len() {
                                true => "an L2",
                                false => "a node that holds no elements",
                            },
                            sources: sources(),
                        },
                        None => SetError::EdgeToUnknown {
                            node,
                            slot,
                            to: edge.to.clone(),
                            holds: holds(),
                        },
                    });
                };
                for (l1, _) in l1s {
                    if !l1.is_static() {
                        return Err(SetError::PairingNotStatic {
                            l2: l2.name.clone(),
                            l1: l1.name.clone(),
                        });
                    }
                }
                // **The near side is whichever geometry the edge did not
                // name**, which with two sources is exactly one. Not `l1s[0]`:
                // the point of writing the edge down is that the list's order
                // stops deciding anything, and a near side still read off
                // position 0 would leave half the old rule in place.
                let near_at = (0..l1s.len())
                    .find(|at| *at != far_at)
                    .expect("two sources, one of them bound");
                if l1s[near_at].1 != l1s[far_at].1 {
                    return Err(SetError::PairingCapacity {
                        l2: l2.name.clone(),
                        a: l1s[near_at].1,
                        b: l1s[far_at].1,
                    });
                }
                Some(far_at)
            }
        };
        // **Every Field slot on every node, bound by an edge or refused.**
        //
        // Every node, because a Field slot is legal on four of the five kinds —
        // the four the loop below already walks when it decides who evaluates a
        // field. The geometry slot above is one node's question and this is the
        // whole Set's, which is why it is a walk rather than a `position`.
        //
        // **Unbound is refused rather than filled in**, exactly as a geometry
        // slot's is. A Set holding one field could resolve every slot to it and
        // be right every time today, and that is precisely the rule this
        // notation exists to remove — "if there is exactly one, use it" is what
        // capped a procedure at one input, and reinstating it here would cap
        // the next Set at one field with nothing in the language to say so.
        //
        // **What it leaves behind is the binding**, node and slot to field
        // ordinal. Everything downstream — which body to splice, which params
        // to write, which map a `--param Field:1:x` lands in — is a question
        // about one slot on one node, and answering it a second time by walking
        // the edges again is the shape this file has already been wrong about
        // twice.
        let mut field_bound: Vec<(usize, String, usize)> = Vec::new();
        for (at, node) in nodes.iter().enumerate() {
            let Some(node) = node else { continue };
            for slot in node
                .uses
                .iter()
                .filter(|s| s.ty == karakuri_ir::SlotTy::Field)
            {
                let node_name = names[at].clone();
                let mut bound = wiring
                    .edges
                    .iter()
                    .filter(|e| e.node == node_name && e.slot == slot.name);
                let Some(edge) = bound.next() else {
                    return Err(SetError::SlotUnbound {
                        node: node_name,
                        slot: slot.name.clone(),
                        takes: slot.ty.name(),
                        holds: holds(),
                    });
                };
                if let Some(second) = bound.next() {
                    return Err(SetError::SlotBoundTwice {
                        node: node_name,
                        slot: slot.name.clone(),
                        first: edge.to.clone(),
                        second: second.to.clone(),
                    });
                }
                match node_at(&edge.to).map(|to| (to, field_ordinal(to))) {
                    Some((_, Some(ordinal))) => {
                        field_bound.push((at, slot.name.clone(), ordinal));
                    }
                    // In the Set and not a field: the layer it *is* is the
                    // useful half of the sentence.
                    Some((other, None)) => {
                        return Err(SetError::EdgeToNotField {
                            node: node_name,
                            slot: slot.name.clone(),
                            to: edge.to.clone(),
                            layer: layer_of(other),
                            fields: holds_fields(),
                        })
                    }
                    None => {
                        return Err(SetError::EdgeToUnknown {
                            node: node_name,
                            slot: slot.name.clone(),
                            to: edge.to.clone(),
                            holds: holds(),
                        })
                    }
                }
            }
        }
        // **Every Camera slot on every node, bound by an edge or refused** —
        // the same walk as the one above and for the same reasons, over a slot
        // type that is legal on one kind rather than four.
        //
        // **Unbound is refused rather than filled in from the Set's first
        // camera**, which is the rule this whole notation exists to remove: "if
        // there is exactly one, use it" is what a Set of one camera could get
        // away with, and a renderer that meant the Set's camera says so by
        // declaring no slot at all. The two spellings are the difference
        // between a picture that is right by luck and one that is right by
        // being written down.
        //
        // **What it leaves behind is which camera each renderer reads.** A
        // renderer with no slot reads camera 0 — the Set's — and that is where
        // the `camera`, `eye` and `ray` ambients have always pointed.
        let mut camera_bound: Vec<(usize, usize)> = Vec::new();
        for (at, node) in nodes.iter().enumerate() {
            let Some(node) = node else { continue };
            for slot in node
                .uses
                .iter()
                .filter(|s| s.ty == karakuri_ir::SlotTy::Camera)
            {
                let node_name = names[at].clone();
                let mut bound = wiring
                    .edges
                    .iter()
                    .filter(|e| e.node == node_name && e.slot == slot.name);
                let Some(edge) = bound.next() else {
                    return Err(SetError::SlotUnbound {
                        node: node_name,
                        slot: slot.name.clone(),
                        takes: slot.ty.name(),
                        holds: holds(),
                    });
                };
                // **Two renderers naming one camera is the ordinary case and
                // needs no rule** — that is fan-out, and it is what one
                // viewpoint drawn two ways is. What is refused here is two
                // edges into *one slot*, which is one renderer with two
                // answers to where it is looking from.
                if let Some(second) = bound.next() {
                    return Err(SetError::SlotBoundTwice {
                        node: node_name,
                        slot: slot.name.clone(),
                        first: edge.to.clone(),
                        second: second.to.clone(),
                    });
                }
                match node_at(&edge.to).map(|to| (to, camera_ordinal(to))) {
                    Some((_, Some(ordinal))) => camera_bound.push((at, ordinal)),
                    Some((other, None)) => {
                        return Err(SetError::EdgeToNotCamera {
                            node: node_name,
                            slot: slot.name.clone(),
                            to: edge.to.clone(),
                            layer: layer_of(other),
                            cameras: holds_cameras(),
                        })
                    }
                    None => {
                        return Err(SetError::EdgeToUnknown {
                            node: node_name,
                            slot: slot.name.clone(),
                            to: edge.to.clone(),
                            holds: holds(),
                        })
                    }
                }
            }
        }
        // **Every Source slot on every node, bound by an edge or refused** —
        // the third walk of this shape, over a slot type legal on the three
        // kinds a Set instantiates per source.
        //
        // **A list per node rather than one entry**, unlike the camera walk
        // above: several are legal, because what a Source slot costs is a `u32`
        // in a uniform block the module already has, and `source == a || source
        // == b` is an ordinary thing for a mask to want. There is nothing here
        // for an arity rule to protect.
        //
        // **What it leaves behind is which geometry each slot names**, as an
        // index into `l1s` — resolved once here, the way the Field slots'
        // ordinals are, rather than walked out of the edges again on the frame
        // path. The salt itself is not taken yet: `salts` is applied below this
        // point, and reading it here would capture the value a source was
        // *given* rather than the one it ended up with.
        let mut source_bound: Vec<(usize, String, usize)> = Vec::new();
        for (at, node) in nodes.iter().enumerate() {
            let Some(node) = node else { continue };
            for slot in node
                .uses
                .iter()
                .filter(|s| s.ty == karakuri_ir::SlotTy::Source)
            {
                let node_name = names[at].clone();
                let mut bound = wiring
                    .edges
                    .iter()
                    .filter(|e| e.node == node_name && e.slot == slot.name);
                let Some(edge) = bound.next() else {
                    return Err(SetError::SlotUnbound {
                        node: node_name,
                        slot: slot.name.clone(),
                        takes: slot.ty.name(),
                        holds: holds(),
                    });
                };
                if let Some(second) = bound.next() {
                    return Err(SetError::SlotBoundTwice {
                        node: node_name,
                        slot: slot.name.clone(),
                        first: edge.to.clone(),
                        second: second.to.clone(),
                    });
                }
                match node_at(&edge.to) {
                    Some(to) => match geometry_at(&edge.to) {
                        Some(l1_at) => source_bound.push((at, slot.name.clone(), l1_at)),
                        None => {
                            return Err(SetError::EdgeToNotSource {
                                node: node_name,
                                slot: slot.name.clone(),
                                to: edge.to.clone(),
                                layer: layer_of(to),
                                sources: sources(),
                            })
                        }
                    },
                    None => {
                        return Err(SetError::EdgeToUnknown {
                            node: node_name,
                            slot: slot.name.clone(),
                            to: edge.to.clone(),
                            holds: holds(),
                        })
                    }
                }
            }
        }
        for (l1, _) in l1s {
            if l1.kind != Kind::L1 {
                return Err(SetError::WrongKind {
                    slot: "L1",
                    expected: Kind::L1,
                    actual: l1.kind,
                });
            }
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
        // **Compiled once per slot, spliced into everything that evaluates
        // it.** A field has no node — it lowers into its callers — so this is
        // the whole of what a Set does with one, and the list is the whole of
        // "a Set may hold as many as its edges name".
        for f in fields {
            if f.kind != Kind::Field {
                return Err(SetError::WrongKind {
                    slot: "Field",
                    expected: Kind::Field,
                    actual: f.kind,
                });
            }
        }
        // **The ceiling every caller passed was applied to an incomplete
        // figure**, because a `field(p)` weighs nothing where a single file is
        // estimated. This is where it is completed.
        //
        // **Skipped for a Set with no field**, because there is nothing to
        // complete: `check_with_field` reports a caller that is over on its own
        // terms as well, and that refusal belongs to the cost pass that runs
        // over one file, under a sentence that is about one file.
        if !fields.is_empty() {
            // **Estimated here rather than read off `Checked`.** That field is
            // never filled by anything — `cost::estimate` returns its answer and
            // the callers discard it — so reading it made both this check and
            // the one below silently dead. Asking is cheap: a tree walk over
            // material that has already been through the same walk once.
            let per_evaluation: Vec<u64> = fields
                .iter()
                .map(|f| {
                    karakuri_ir::cost::estimate(f)
                        .map(|c| c.ops_per_evaluation)
                        .unwrap_or(0)
                })
                .collect();
            // The callers, at the node indices the bindings are keyed by —
            // every node that is not itself a field, since a field cannot take
            // one.
            for (at, caller) in nodes
                .iter()
                .enumerate()
                .filter(|(at, _)| !field_range.contains(at))
                .filter_map(|(at, n)| n.map(|n| (at, n)))
            {
                // **Asked per slot, and answered by whichever field that slot
                // is bound to.** A shape and a cutter are two procedures with
                // two prices, and the sum is what `check_with_field` charges —
                // so an answer that ignored the slot would charge one of them
                // twice and the other never.
                let per_slot = |slot: &str| {
                    field_bound
                        .iter()
                        .find(|(node, name, _)| *node == at && name == slot)
                        .map(|(_, _, ordinal)| per_evaluation[*ordinal])
                        .unwrap_or(0)
                };
                if let Err(over) = karakuri_ir::cost::check_with_field(caller, &per_slot) {
                    // **The field the named slot reaches**, not the Set's
                    // first: the sentence says which procedure is the
                    // expensive one, and with several it would otherwise name
                    // whichever happened to be given first.
                    let blamed = field_bound
                        .iter()
                        .find(|(node, name, _)| *node == at && *name == over.slot)
                        .map(|(_, _, ordinal)| fields[*ordinal].name.clone())
                        .unwrap_or_default();
                    return Err(SetError::FieldTooExpensive {
                        caller: caller.name.clone(),
                        slot: over.slot,
                        field: blamed,
                        detail: over
                            .errors
                            .first()
                            .map(|e| e.message.clone())
                            .unwrap_or_default(),
                    });
                }
            }
        }

        // **A procedure that evaluates a field needs one to be there**, and it
        // is the slot walk above that says so now rather than a search here. A
        // call names a slot, a slot is declared in the header, and a declared
        // slot is bound or refused — so a Set with no field turns the same file
        // away at the declaration, which is where the author can do something
        // about it. The search this replaced could only report the *caller*,
        // because a call carried no name to report.

        // **As many cameras as the Set's files declare**, on the terms its
        // fields and its renderers already had: several is a Set watched from
        // several places at once, and which renderer reads which is the edge's
        // answer rather than the list's.
        for l3 in l3s {
            if l3.kind != Kind::L3 {
                return Err(SetError::WrongKind {
                    slot: "L3",
                    expected: Kind::L3,
                    actual: l3.kind,
                });
            }
        }
        // **A bound geometry collapses the list.** Two sources become one
        // `Source` with two simulations in it, so the loop below — and the one
        // `build_inner` runs over the same heads — runs once, and everything
        // under it is what a Set of one source has.
        //
        // **The drawn one is whichever the edge did not name.** Not position 0:
        // an edge exists so that the order of the list decides nothing, and a
        // head taken from position 0 would keep half of the rule this replaced.
        let heads: Vec<usize> = (0..l1s.len()).filter(|at| Some(*at) != far_at).collect();
        // **Assigned where the caller had one, derived where it had none** —
        // `salts` is indexed by geometry, and every geometry gets one whether
        // it is drawn or read.
        let salt_of = |at: usize| -> u32 {
            salts
                .get(at)
                .copied()
                .flatten()
                .unwrap_or_else(|| derived_salt(seed_salt, at))
        };
        // **What each geometry is actually salted with**, in `l1s` order, kept
        // so that whatever writes a Set file can record the value rather than
        // work it out a second time — see [`Set::source_salts`].
        //
        // In `l1s` order and not in the order `build_inner` builds them,
        // because the order it builds them in is no longer the list's: with a
        // slot bound to the first geometry the far side is built first.
        let source_salts: Vec<u32> = (0..l1s.len()).map(salt_of).collect();
        let mut derived_per_head: Vec<Vec<karakuri_ir::Attr>> = Vec::with_capacity(heads.len());
        for &at in &heads {
            let (l1, capacity) = l1s[at];
            // **The plan, before anything is built.**
            //
            // A consumed attribute nothing emits used to be an unconditional error.
            // Two of them have a derivation rule, and this is where the rule is
            // applied: the Set is the first point that holds every procedure at
            // once, so it is the only place that can tell "nobody emits this" from
            // "nobody emits this *yet*".
            //
            // **Nothing any node emits is ever derived**, whatever the positions
            // involved. An attribute that is both would have a slot and a
            // substitution, and every reader would have to know which one applied
            // where — so a chain that emits `age` somewhere keeps the old answer for
            // a node above the emitter, which is a composition error naming a
            // position, and that is the honest report.
            let emitted: Vec<karakuri_ir::Attr> = l1
                .emit
                .iter()
                .chain(l2s.iter().flat_map(|n| n.emit.iter()))
                .copied()
                .collect();
            let mut derived: Vec<karakuri_ir::Attr> = Vec::new();
            // Rules that would have applied and could not, with what they wanted.
            let mut blocked: Vec<(karakuri_ir::Attr, karakuri_ir::Attr)> = Vec::new();
            {
                let mut seen: Vec<karakuri_ir::Attr> = l1.emit.clone();
                // **The L1 is in this walk too**, and leaving it out is a shader
                // that names a field nothing allocated. A procedure may consume
                // what it does not emit — the checker allows exactly the two rules
                // — and the slots those rules read are written by this same node,
                // so what it reads back is the previous frame's, which is what
                // `prev` means everywhere else in its own block.
                for node in std::iter::once(&l1).chain(l2s.iter()).chain(l4s.iter()) {
                    for &attr in &node.consumes {
                        if seen.contains(&attr)
                            || derived.contains(&attr)
                            || emitted.contains(&attr)
                        {
                            continue;
                        }
                        let Some(rule) = attr.derivation() else {
                            continue;
                        };
                        // **The source has to be on the element the L1 writes.** A
                        // rule reading `position` cannot run over geometry that has
                        // no position, and deriving from something an L2 adds later
                        // would mean the L1 writing a slot from a value it does not
                        // have.
                        //
                        // The reason is kept rather than dropped: this is the one
                        // case where the eventual refusal is *about the rule*, and
                        // a message that does not say so reads as the spec
                        // contradicting itself.
                        if let Some(from) = rule.source().filter(|from| !l1.emit.contains(from)) {
                            blocked.push((attr, from));
                            continue;
                        }
                        derived.push(attr);
                    }
                    for &attr in &node.emit {
                        if !seen.contains(&attr) {
                            seen.push(attr);
                        }
                    }
                }
            }

            let mut available: Vec<karakuri_ir::Attr> = l1.emit.clone();
            available.extend(derived.iter().copied());
            let check_against = |node: &Checked, available: &[karakuri_ir::Attr]| {
                let missing: Vec<String> = node
                    .consumes
                    .iter()
                    .filter(|a| !available.contains(a))
                    .map(|a| format!("`{}`", a.name()))
                    .collect();
                if missing.is_empty() {
                    return None;
                }
                // If a rule was blocked for one of these, say which value it wanted
                // rather than repeating the generic advice — that is the whole of
                // what makes the refusal actionable.
                let hint = node
                    .consumes
                    .iter()
                    .find_map(|a| blocked.iter().find(|(attr, _)| attr == a))
                    .map(|(attr, from)| {
                        format!(
                            "`{}` is synthesised from `{}`, and `{}` emits neither. Add `{}` to \
                             `{}`'s `emit` and `{}` follows",
                            attr.name(),
                            from.name(),
                            l1.name,
                            from.name(),
                            l1.name,
                            attr.name()
                        )
                    })
                    .unwrap_or_else(|| {
                        format!(
                            "add {} to `{}`'s `emit`, or pair `{}` with an L1 that emits it. \
                             `age` and `velocity` are synthesised where nothing emits them; nothing \
                             else is",
                            missing.join(", "),
                            l1.name,
                            node.name
                        )
                    });
                Some(SetError::Composition {
                    l1: l1.name.clone(),
                    l4: node.name.clone(),
                    missing: missing.join(", "),
                    hint,
                })
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
            // would refuse the pairing several renderers over one simulation
            // exist to enable: the same cloud drawn as sprites by one L4 and as
            // streaks by another. A segment under
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
                    return Err(SetError::WeightedFullscreen {
                        l4: only.name.clone(),
                    });
                }
            }

            // **The near geometry's capacity against its own declared range.**
            // Where `Simulation::build` used to check it, in the order it used
            // to: this is the point the near source was built at.
            capacity_in_range(l1, capacity)?;
            if let Some(far_at) = far_at {
                let (far, far_capacity) = l1s[far_at];
                // A rule the far side cannot support is refused here rather
                // than producing a slot nothing fills: `velocity` is derived
                // from `position`, and a geometry emitting neither has
                // nothing to derive it from.
                for attr in &derived {
                    if attr
                        .derivation()
                        .and_then(|d| d.source())
                        .is_some_and(|from| !far.emit.contains(&from))
                    {
                        return Err(SetError::PairingDerivation {
                            l2: l2s
                                [pairing.expect("a bound geometry is a node that declared a slot")]
                            .name
                            .clone(),
                            l1: far.name.clone(),
                            attr: attr.name().to_string(),
                        });
                    }
                }
                // **The far geometry's own**, on the same terms as the near
                // one, and after the pairing rule exactly as it was when the
                // far side was built here.
                capacity_in_range(far, far_capacity)?;
            }
            derived_per_head.push(derived);
        }

        Ok(Plan {
            names,
            cameras,
            camera_range,
            field_bound,
            camera_bound,
            source_bound,
            far_at,
            heads,
            source_salts,
            derived: derived_per_head,
            l1s: l1s.to_vec(),
            l2s: l2s.to_vec(),
            fields: fields.to_vec(),
        })
    }

    /// **The device half**, and nothing else: every refusal that can be decided
    /// without hardware was decided by [`Set::validate`] one line down, and
    /// what is left here builds against the [`Plan`] it returned. Nothing below
    /// re-checks any of it — a rule with two homes is what this split removed.
    #[allow(clippy::too_many_arguments)]
    fn build_inner(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        l1s: &[(&Checked, u32)],
        l2s: &[&Checked],
        l3s: &[&Checked],
        fields: &[&Checked],
        l4s: &[&Checked],
        layering: Layering,
        seed_salt: u32,
        salts: &[Option<u32>],
        wiring: Wiring<'_>,
    ) -> Result<Set, SetError> {
        let Plan {
            names,
            cameras,
            camera_range,
            field_bound,
            camera_bound,
            source_bound,
            far_at,
            heads,
            source_salts,
            derived: derived_per_head,
            // The material, which this function was handed itself and uses its
            // own copy of. The plan keeps it for the callers that have a plan
            // and nothing else — see [`Plan::element_storage`].
            l1s: _,
            l2s: _,
            fields: _,
        } = Set::validate(
            l1s, l2s, l3s, fields, l4s, layering, seed_salt, salts, wiring,
        )?;
        // **What each node's Field slots are bound to**, ready to hand to a
        // generator — read off the plan's bindings rather than walked out of
        // the edges again. The near geometry, the far geometry, every deform
        // and every renderer all want the same answer and three of them are
        // inside loops.
        let bound_at = |at: usize| -> Vec<(&str, &Checked)> {
            field_bound
                .iter()
                .filter(|(node, _, _)| *node == at)
                .map(|(_, slot, ordinal)| (slot.as_str(), fields[*ordinal]))
                .collect()
        };

        // **Each camera's own field bindings, asked at its own node index.**
        // `bound_at` is keyed by node, and the built-in's index holds a node
        // that declares nothing — so this is the same call for both kinds of
        // camera and comes back empty for the one with no procedure.
        let camera_nodes: Vec<crate::node::Camera> = cameras
            .iter()
            .enumerate()
            .map(|(k, l3)| {
                crate::node::Camera::build(device, *l3, &bound_at(camera_range.start + k))
            })
            .collect();

        // **Everything below is per source**, because everything below depends
        // on what that source emits: which attributes are derived, which the
        // chain may consume, what the element layout is, and therefore what
        // every node over it compiles against. Two sources that emit different
        // things are two different chains — which is the whole reason the chain
        // is instantiated per source rather than the geometry concatenated into
        // one buffer.
        // **One target per renderer *procedure*** under compositing, not per
        // instance: every source draws into the target its renderer owns, and
        // the first source is the one that clears it.
        let renderer_count = l4s.len();
        let mut sources: Vec<Source> = Vec::with_capacity(heads.len());
        for (head, &at) in heads.iter().enumerate() {
            let (l1, capacity) = l1s[at];
            let salt = source_salts[at];
            // **The attributes this chain synthesises**, worked out by
            // `Set::validate` — the same list every node over this
            // source is generated against.
            let derived = &derived_per_head[head];

            // **The L1 node**, generated, compiled and allocated at `capacity`
            // — which `Set::validate` has already checked against the range
            // the artifact declares, so this constructor cannot refuse.
            let sim = Simulation::build(device, l1, capacity, salt, derived, &bound_at(at));

            // **The far geometry is built with the same `derived` list**, and
            // that is not a convenience: the node addresses its buffer with a
            // struct generated from this list, so a far side built with a
            // different one is a struct that disagrees about every offset past
            // the first derived slot. It read the wrong bytes and the picture
            // went black, which is the quietest way that can go wrong.
            let paired: Option<(Vec<karakuri_ir::Attr>, Simulation)> = match far_at {
                None => None,
                Some(far_at) => {
                    let (far, far_capacity) = l1s[far_at];
                    // **The far geometry's own salt**, on the same terms as the
                    // near one: a Set with a slot bound is two geometries and
                    // one `Source`, and a Set file records a salt per geometry.
                    let far_salt = source_salts[far_at];
                    Some((
                        far.emit.clone(),
                        Simulation::build(
                            device,
                            far,
                            far_capacity,
                            far_salt,
                            derived,
                            &bound_at(far_at),
                        ),
                    ))
                }
            };

            // **The L2 nodes**, each built against what reaches it. The chain is
            // walked here rather than inside a node because the *grouping* decides
            // the order — a node knows how it deforms and not what is above it.
            let mut deforms: Vec<Deform> = Vec::new();
            // **Not `available`.** `upstream` is what has a *slot* at this position,
            // which the layout function widens with the derivation slots itself —
            // putting a derived attribute in this list would give it a second one.
            let mut upstream: Vec<karakuri_ir::Attr> = l1.emit.clone();
            // The engine-written slots and the element count at the current
            // position, both of which an amplifier changes for everything below it.
            let mut synthetic = karakuri_ir::layout::Synthetic::NONE;
            let mut chain_capacity = capacity;
            // **Index of the last node that amplified**, which is where the chain's
            // liveness and counts live from that point on. Tracked rather than
            // recomputed from `deforms.last()`, because a node that does *not*
            // amplify hands on whatever reached it — so the answer after
            // `[amplify, plain]` is the first node's buffers, and asking the last
            // node alone would give the simulation's.
            let mut live: Option<usize> = None;
            for (k, l2) in l2s.iter().enumerate() {
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
                    // **The far geometry, for the node that declared a slot.**
                    // It reads a *simulation* rather than whatever reached this
                    // position, which is why such a node has to be first in the
                    // chain — refused above if it is not.
                    let paired = paired.as_ref().filter(|_| l2.geometry_slot().is_some());
                    let far = paired.map(|(emits, sim): &(Vec<karakuri_ir::Attr>, Simulation)| {
                        (emits.as_slice(), sim.geometry())
                    });
                    Deform::build(
                        device,
                        l2,
                        &upstream,
                        synthetic,
                        derived,
                        far.as_ref().map(|(a, g)| (*a, g)),
                        &bound_at(l1s.len() + k),
                        &input,
                        chain_capacity,
                    )?
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
            // **Which camera each one reads is the edge's answer.** Sharing
            // needs no rule — two L4s bound to one camera are one viewpoint
            // drawn two ways, which is ordinary fan-out — and a renderer that
            // declares no slot reads camera 0, the Set's, which is what
            // `camera`, `eye` and `ray` have always meant.
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
                let first_l4 = camera_range.end;
                l4s.iter()
                    .enumerate()
                    .map(|(k, l4)| {
                        let at = first_l4 + k;
                        let camera = camera_bound
                            .iter()
                            .find(|(node, _)| *node == at)
                            .map_or(0, |(_, ordinal)| *ordinal);
                        Renderer::build(device, l4, &geometry, &camera_nodes[camera], &bound_at(at))
                    })
                    .collect()
            };
            sources.push(Source {
                salt,
                // **In the order `Set::prepare` walks the simulations**: the
                // near one, then the far one where there is one. The two lists
                // used to agree by construction, because the near side was
                // always `l1s[0]`; an edge is what stopped them agreeing, and
                // `--param L1:1:radius` still has to reach the procedure that
                // declared `radius`.
                procedures: std::iter::once(at).chain(far_at).collect(),
                sim,
                paired: paired.map(|(_, s)| s),
                deforms,
                renderers,
            });
        }

        // One map per node, in the order [`Set::slot_of`] addresses them: the
        // L1's, then each renderer's. Two nodes declaring one name now hold two
        // values, which is what a name meaning "this node's" buys. The fold
        // behind it is [`declared_defaults`], which is `karakuri-ir`'s.
        //
        // **One map per L1 procedure**, which is one per source: each source
        // *is* an L1, and `--param L1:1:spawn_rate` names the second one.
        let params = l1s
            .iter()
            .map(|(l1, _)| declared_defaults(l1))
            .chain(l2s.iter().map(|n| declared_defaults(n)))
            // **One map per camera node, including the built-in's**, which is
            // empty: it is not a procedure and declares no params. Empty rather
            // than absent is the whole of what makes this list addressable —
            // `slot_of` sums the layers before it, so a Set whose camera
            // contributed no entry would put the first renderer's map at the
            // camera's index and hand every `L4:n` the node before it.
            .chain(
                cameras
                    .iter()
                    .map(|n| n.map(declared_defaults).unwrap_or_default()),
            )
            .chain(l4s.iter().map(|n| declared_defaults(n)))
            // **Last, and by declared name.** The prefix belongs to the WGSL
            // spelling and to nothing else: an operator writes
            // `--param Field:0:ball`, which is the name the file declares.
            .chain(fields.iter().map(|n| declared_defaults(n)))
            .collect();
        // The same walk, so a node's values and its ranges cannot end up at
        // different indices — the defect this file has already paid for twice.
        let declared = |node: &Checked| -> HashMap<String, [f32; 2]> {
            node.params
                .iter()
                .map(|p| (p.name.clone(), [p.min, p.max]))
                .collect()
        };
        let ranges = l1s
            .iter()
            .map(|(l1, _)| declared(l1))
            .chain(l2s.iter().map(|n| declared(n)))
            .chain(cameras.iter().map(|n| n.map(declared).unwrap_or_default()))
            .chain(l4s.iter().map(|n| declared(n)))
            .chain(fields.iter().map(|n| declared(n)))
            .collect();
        // **Every node the operator's, on a Set nobody has spoken for yet.**
        // Derived from the same `names` walk the params and the ranges are, so
        // a node's authority cannot end up at a different index from its name.
        // A request states the rest — see `swap::Request::authorities`.
        let authorities = vec![Authority::default(); names.len()];
        let set = Set {
            names,
            authorities,
            seed_salt,
            source_salts,
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
            // **Every source, because a Set is seekable only if all of it is.**
            // One accumulating geometry beside four closed-form ones is a Set
            // that cannot be scrubbed to, and the conjunction is what says so.
            closed_form: l1s.iter().all(|(n, _)| n.closed_form)
                && l2s.iter().all(|n| n.closed_form)
                && l3s.iter().all(|n| n.closed_form)
                && l4s.iter().all(|n| n.closed_form),
            reads_beats: l1s.iter().any(|(n, _)| n.reads_beats)
                || l2s.iter().any(|n| n.reads_beats)
                || l3s.iter().any(|n| n.reads_beats)
                || l4s.iter().any(|n| n.reads_beats),
            sources,
            params,
            ranges,
            camera: Orbit::default(),
            cameras: camera_nodes,
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
            l1_count: l1s.len(),
            field_count: fields.len(),
            field_declared: fields
                .iter()
                .map(|f| f.params.iter().map(|p| p.name.clone()).collect())
                .collect(),
            // **One key per binding**, not per slot spelling: two nodes may
            // each declare a `shape` and have them bound to different fields,
            // so the union is taken over what was *bound* and the value behind
            // each key is a question about the node reading it — see
            // [`Set::field_bound`].
            field_params: {
                let mut keys: Vec<String> = field_bound
                    .iter()
                    .flat_map(|(_, slot, ordinal)| {
                        fields[*ordinal]
                            .params
                            .iter()
                            .map(move |p| karakuri_codegen::layout::field_param_key(slot, &p.name))
                    })
                    .collect();
                keys.sort_unstable();
                keys.dedup();
                keys
            },
            field_bound,
            source_bound,
        };
        for source in &set.sources {
            source.sim.initialize(queue);
            if let Some(other) = &source.paired {
                other.initialize(queue);
            }
        }
        // **A camera before the first `prepare`.** The state buffer starts
        // zeroed, and a camera whose eye and target coincide has no forward
        // direction — `normalize` of it is NaN, and a NaN view matrix is a blank
        // frame with no diagnostic. Every path that draws writes this first, so
        // nothing depends on it; it costs 64 bytes once and removes a shape of
        // failure that would only ever appear in a caller's test.
        for camera in &set.cameras {
            camera.write_state(queue, &set.camera.state(0.0));
            camera.write_canvas(queue, 1.0);
        }
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
        for renderer in self.sources.iter_mut().flat_map(|s| &mut s.renderers) {
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
        // **Summed**, because a Set's population is all of it. The paired
        // geometry is *not* counted: it feeds the pairing node and is never
        // drawn, so counting it would report twice the material anyone can see.
        self.sources
            .iter()
            .map(|s| s.sim.live_count(device, queue))
            .sum()
    }

    /// The raw bytes of the element buffer the renderer is currently reading,
    /// decoded against [`Set::element_layout`]. **A stall, on the same terms as
    /// [`Set::live_count`]** — this exists so a test can check that survivors
    /// kept their order, which is a claim about `seed` values in slots and
    /// cannot be made from a rendered image.
    pub fn read_elements(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        // **The first source's.** With several there are several buffers and
        // no one of them is "the elements"; a caller that wants another asks
        // for it, and none does yet.
        self.sources[0].sim.read_elements(device, queue)
    }

    pub fn element_layout(&self) -> &ElementLayout {
        self.sources[0].sim.element_layout()
    }

    pub fn capacity(&self) -> u32 {
        // **Summed, to match `live_count`.** A Set of two sources allocates
        // both, and reporting one of them beside a population that is all of
        // them said "8192 live of 4096". The paired geometry is left out for
        // the same reason it is left out of the count: nothing draws it.
        self.sources.iter().map(|s| s.sim.capacity()).sum()
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
        for source in &mut self.sources {
            source.sim.rewind(queue);
            if let Some(other) = &mut source.paired {
                other.rewind(queue);
            }
        }
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
    ///
    /// **The two ways to fail are different mistakes**, which is why this
    /// answers with a reason rather than a `bool`. A caller that collapses
    /// them reports a misspelt *control* as a missing *parameter*, and sends
    /// whoever reads it to look at the wrong half of their command line.
    pub fn bind(&mut self, binding: Binding) -> Bound {
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
                return Bound::NoSuchControl;
            }
        }
        let range = self.nodes_of(binding.layer);
        let names = self.declared_names(binding.layer);
        let found = names.iter().enumerate().any(|(at, n)| {
            binding.covers(at)
                && self
                    .params
                    .get(range.start + at)
                    .is_some_and(|map| declares(n, map))
        });
        if !found {
            return Bound::NoSuchParam;
        }
        // At most one per (layer, index, param). A wildcard and an addressed
        // binding on one name are two bindings and the addressed one wins for
        // the node it names, because `effective` takes the first match and an
        // addressed binding is pushed later — which is the same "the last one
        // attached wins" rule a repeated binding already follows, applied to a
        // narrower target.
        self.bindings.retain(|b| {
            b.layer != binding.layer || b.key != binding.key || b.index != binding.index
        });
        self.bindings.push(binding);
        Bound::Yes
    }

    /// **What every node of this Set allocated to hold elements**, one entry
    /// per node — see [`ElementStorage`].
    ///
    /// In the order the sources are walked, and within a source: the
    /// simulation, the far simulation it pairs with where there is one, then
    /// the chain of deforms — the order the elements themselves travel in.
    ///
    /// **Not the order [`Set::node_names`] is in**, and it cannot be made to
    /// be: a name is per *procedure* and an entry here is per *instance*, so a
    /// Set over two sources instantiates one chain of deforms twice and has
    /// more entries than there are names. Labelling these belongs to whatever
    /// gives a node *instance* an address, which nothing does yet: naming
    /// settled on the procedure and deliberately left the instance alone, and
    /// nothing outside this method has wanted one since — see
    /// `docs/adr/0152-a-kir-names-a-slot-and-the-set-names-the-nodes.md`.
    ///
    /// **A renderer, a camera and a merge are absent rather than zero.** An L4
    /// draws from the buffer the node above it allocated, so charging it would
    /// count the same memory twice; an L3 has no elements at all; and an L5
    /// folds finished targets, which are not element storage under any reading
    /// — see [`ElementStorage`] for what the figure excludes. A row for any of
    /// them would be a zero the reader has to work out the meaning of.
    ///
    /// Those three and the two kinds walked below — the simulations, including
    /// the far one a pairing Set holds, and the deforms — are every node kind a
    /// [`Set`] has a field for, so a reader auditing this against the struct
    /// finds nothing unaccounted for.
    pub fn element_storage(&self) -> Vec<ElementStorage> {
        self.sources
            .iter()
            .flat_map(|source| {
                std::iter::once(source.sim.element_storage())
                    .chain(source.paired.iter().map(Simulation::element_storage))
                    .chain(source.deforms.iter().map(Deform::element_storage))
            })
            .collect()
    }

    /// **What this Set holds in element storage, in bytes.**
    ///
    /// The question the per-element figures exist to answer, and the first
    /// place it can be asked: `capacity` differs per node and an amplifier
    /// multiplies it for everything below, so no node — and no procedure —
    /// knows how much element storage the Set as a whole allocated. A deck that
    /// wants its own total sums this over its slots, which is a sum over Sets
    /// rather than a second walk of the nodes.
    ///
    /// **Element storage and not device memory**, on the terms
    /// [`ElementStorage`] sets out: render targets, uniform blocks and
    /// everything else not indexed by element are outside this sum, so it is a
    /// floor on what the Set costs a device and never the figure to allocate
    /// against. The word *residency* is deliberately not used for it — in this
    /// project that names a slot's Live/Priming/Parked level, which is a
    /// different question about a different thing.
    pub fn element_storage_bytes(&self) -> u64 {
        self.element_storage().iter().map(|e| e.bytes).sum()
    }

    /// **What each node is called**, in node order.
    pub fn node_names(&self) -> &[String] {
        &self.names
    }

    /// **What each geometry is salted with**, in the order its L1 procedures
    /// were given — one entry per geometry, so a pairing Set has two.
    ///
    /// For whatever records a Set: the spec's salt is a value *assigned* and
    /// written into the record stream, and what has to be written is the value
    /// the Set is actually running at. A caller that assigned one is being told
    /// its own number back; a caller that assigned none finds out what it got.
    /// Either way it is one fact read from where it lives rather than a second
    /// derivation somewhere else.
    pub fn source_salts(&self) -> &[u32] {
        &self.source_salts
    }

    /// **What each geometry was allocated at**, in the order its L1 procedures
    /// were given — one entry per geometry, exactly the shape
    /// [`Set::source_salts`] has and for the same reason.
    ///
    /// **Not [`Set::capacity`], which is the sum.** That one answers "how many
    /// elements does this Set hold", which is the question a population figure
    /// is asked beside; this one answers "what is each geometry running at",
    /// which is what a `capacity` record says — one per L1 node, since each
    /// source declares its own range and one number cannot size two of them. A
    /// writer handed the sum would record a two-geometry Set as a single number
    /// that is neither geometry's.
    ///
    /// **Placed by the procedure's ordinal rather than appended**, on the same
    /// terms as the `Kind::L1` arm of [`Set::bind`]: an `edge` decides which of
    /// a pairing Set's two geometries is the far one, so the order the
    /// simulations are walked in is not the order the procedures were given in.
    pub fn source_capacities(&self) -> Vec<u32> {
        let mut out = vec![0; self.l1_count];
        for source in &self.sources {
            for (k, sim) in std::iter::once(&source.sim)
                .chain(source.paired.iter())
                .enumerate()
            {
                out[source.procedures[k]] = sim.capacity();
            }
        }
        out
    }

    /// The node a name addresses, as the `(layer, index)` every other surface
    /// in this system uses.
    ///
    /// **A name is an alias and the position is the address** — the same shape
    /// [`Published`] already gives a parameter, and for the same reason: an
    /// alias can be chosen, changed and recorded without anything underneath it
    /// moving. So this resolves and hands back the pair rather than becoming a
    /// second way to reach a node.
    pub fn node_named(&self, name: &str) -> Option<(Kind, u32)> {
        let at = self.names.iter().position(|n| n == name)?;
        Kind::ALL.into_iter().find_map(|kind| {
            let range = self.nodes_of(kind);
            range
                .contains(&at)
                .then(|| (kind, (at - range.start) as u32))
        })
    }

    /// **Where a layer's nodes start in [`Set::params`].**
    ///
    /// The maps are in node order — the L1, then each deformation, then each
    /// renderer — so this depends on how long the chain is and cannot be a
    /// constant. That is the price of one flat list, and it is the right price:
    /// a `Vec` per layer would make "which node is this" three questions
    /// instead of one arithmetic.
    /// How many **procedures** of `layer` this Set holds, which is not how many
    /// instances of them run.
    ///
    /// A chain is instantiated once per source, so a Set of two sources and one
    /// L2 runs two deformations and addresses one. Read off the first source
    /// because every source runs the same procedures, in the same order —
    /// which is what makes an address mean one thing.
    fn procedures(&self, layer: Kind) -> usize {
        let first = &self.sources[0];
        match layer {
            Kind::L1 => self.l1_count,
            Kind::L2 => first.deforms.len(),
            Kind::L3 => self.cameras.len(),
            Kind::L4 => first.renderers.len(),
            Kind::Field => self.field_count,
        }
    }

    fn slot_of(&self, layer: Kind) -> usize {
        match layer {
            Kind::L1 => 0,
            // **After every L1 procedure**, which is more than one now. This
            // said `1` and was right while a Set held one geometry — the same
            // constant the L4 arm below spelled out before an L3 landed between
            // them, and the same defect: an origin that happens to be a
            // constant is an origin nobody notices stopping being one.
            Kind::L2 => self.l1_count,
            // **An L3's place is between the deformations and the
            // renderers**, and there is at least one: a Set whose camera is the
            // built-in holds it as a node like any other, so `L3:0` addresses
            // something in every Set. It used to address nothing there, and
            // `L4` began at the same slot — which made `slot_of(L3)` a name for
            // the first renderer's map and was carefully worked around at every
            // reader rather than fixed here.
            Kind::L3 => self.l1_count + self.procedures(Kind::L2),
            Kind::L4 => self.l1_count + self.procedures(Kind::L2) + self.procedures(Kind::L3),
            // **Last, and it addresses nodes that do not exist.** A field has
            // no pass and no buffers — it lowers into whoever evaluates it — so
            // what the slot points at is a parameter map and nothing else. That
            // is enough for every surface an operator has: an override, a
            // signal binding, a published control, a saved Set file. Every
            // procedure that reaches a field through a bound slot writes that
            // field's values into its own uniform, so one address reaches every
            // caller of the field it names — and only of that one.
            Kind::Field => {
                self.l1_count
                    + self.procedures(Kind::L2)
                    + self.procedures(Kind::L3)
                    + self.procedures(Kind::L4)
            }
        }
    }

    /// Every map in [`Set::params`] belonging to `layer`, in node order.
    fn nodes_of(&self, layer: Kind) -> std::ops::Range<usize> {
        let start = self.slot_of(layer);
        match layer {
            Kind::L1 => start..start + self.l1_count,
            Kind::L2 => start..start + self.procedures(Kind::L2),
            // At least one, because the built-in camera is a node too — it
            // declares no params, so the map at its index is empty and an
            // address into it reaches a node that holds nothing.
            Kind::L3 => start..start + self.procedures(Kind::L3),
            Kind::L4 => start..self.params.len() - self.field_count,
            // One per field, and empty for a Set with none — on the same terms
            // `L3` is empty for a Set with no camera procedure: the kind is
            // addressable and a Set that holds nothing there reports
            // `--param Field:…` as reaching nothing.
            Kind::Field => start..start + self.field_count,
        }
    }

    /// **What each node of `layer` declares, in node order and in the order
    /// its procedure declared them.**
    ///
    /// One entry per node of that layer, aligned with [`Set::nodes_of`]: entry
    /// `i` belongs to the slot at `nodes_of(layer).start + i` in
    /// [`Set::params`] and `ranges`. A node that declares nothing — the
    /// built-in camera — is an empty entry rather than a missing one, for the
    /// reason its empty param map exists: a list that skipped it would shift
    /// every node after it.
    ///
    /// **The only place declaration order survives.** `params` and `ranges`
    /// are maps and a map keeps no order; these lists come straight off each
    /// procedure's `param` list, so they are what [`Set::published`] walks to
    /// put the default interface in the order the author wrote — and what
    /// [`Set::bind`] checks a binding's key against. Two readers of one walk,
    /// which is why this is a method rather than the block it used to be
    /// inside `bind`.
    fn declared_names(&self, layer: Kind) -> Vec<&[String]> {
        match layer {
            // **One entry per L1 *procedure***, which includes a far
            // geometry: it is an L1 with params of its own, and an operator
            // riding them is riding the far end of a morph.
            //
            // **Placed by the procedure's ordinal rather than appended**, for
            // the reason `Source::procedures` exists: the order the simulations
            // are walked in is not the order the procedures were given in once
            // an edge decides which of them is the far one.
            Kind::L1 => {
                let mut out: Vec<&[String]> = vec![&[]; self.l1_count];
                for source in &self.sources {
                    for (k, sim) in std::iter::once(&source.sim)
                        .chain(source.paired.iter())
                        .enumerate()
                    {
                        out[source.procedures[k]] = sim.param_names();
                    }
                }
                out
            }
            // **One entry per L2 procedure, not per instance.** A chain is
            // instantiated once per source and the procedures are shared, so an
            // address names the procedure and the Set writes it to every
            // instance — the first source's list is every procedure's list.
            Kind::L2 => self.sources[0]
                .deforms
                .iter()
                .map(|d| d.param_names())
                .collect(),
            // **One entry per camera node**, so that `L3:1:dist` is checked
            // against the second camera's declarations. The built-in's is
            // empty — it is a node and not a procedure.
            Kind::L3 => self.cameras.iter().map(|c| c.param_names()).collect(),
            // **One "node" that is no node at all.** A field has no pass and no
            // buffers, and its params are still declared, addressable, and an
            // operator's to ride — so what is returned here is the list the
            // field declared, and every caller writes the same answer into its
            // own uniform.
            // **One entry per field**, so that `Field:1:radius` is checked
            // against the second field's declarations and not the first's.
            Kind::Field => self.field_declared.iter().map(Vec::as_slice).collect(),
            Kind::L4 => self.sources[0]
                .renderers
                .iter()
                .map(|r| r.param_names())
                .collect(),
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

    /// **Who may move one node.** `None` if that node does not exist.
    ///
    /// The read the console's `man / sug / auto` chip has been missing:
    /// `Operation::SetAuthority` and `Record::Authority` landed with
    /// `docs/adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md`
    /// and had a vocabulary to speak and nothing to answer them. This is the
    /// value a surface draws.
    ///
    /// Addressed the way every other per-node read is — `(layer, index)`
    /// stepped through `nodes_of` rather than added to `slot_of`. See
    /// [`Set::set_param_at`] for the walk-out-the-other-end that refuses.
    pub fn authority(&self, layer: Kind, index: u32) -> Option<Authority> {
        let slot = self.nodes_of(layer).nth(index as usize)?;
        self.authorities.get(slot).copied()
    }

    /// **Grant or take back one node.** `false` if that node does not exist,
    /// which is the caller's cue to say so — a rebuild may name fewer nodes
    /// than the Set an authority was recorded against.
    ///
    /// A destination and never a step, which is
    /// `docs/principles/0090-a-surface-offers-it-never-decides.md`:
    /// a control that cycles the three is an affordance built over this.
    pub fn set_authority(&mut self, layer: Kind, index: u32, authority: Authority) -> bool {
        let Some(slot) = self.nodes_of(layer).nth(index as usize) else {
            return false;
        };
        match self.authorities.get_mut(slot) {
            Some(held) => {
                *held = authority;
                true
            }
            None => false,
        }
    }

    /// **Every node that declares `key`**, addressed and with the authority it
    /// is under — the walk [`CrossesAuthority::over`] decides on.
    ///
    /// Addressed through [`Set::nodes_of`] for [`Set::params`]'s reason: where a
    /// layer's nodes are is one fact, and a second copy of the arithmetic is a
    /// copy that can be right about `L2` and wrong about `L3`. A node whose
    /// authority is missing reads as [`Authority::default`], which is what a
    /// node nobody has spoken for is.
    fn landing(&self, key: &str) -> Vec<(Kind, u32, Authority)> {
        Kind::ALL
            .into_iter()
            .flat_map(|layer| {
                self.nodes_of(layer)
                    .enumerate()
                    .map(move |(index, slot)| (layer, index as u32, slot))
            })
            .filter(|(_, _, slot)| self.params[*slot].contains_key(key))
            .map(|(layer, index, slot)| {
                (
                    layer,
                    index,
                    self.authorities.get(slot).copied().unwrap_or_default(),
                )
            })
            .collect()
    }

    /// Apply one [`ParamWrite`], addressed or not. Returns how many nodes it
    /// reached; zero is the caller's cue to say so.
    ///
    /// The one entry point a `param` record and a `--param` both come through,
    /// so the wildcard and the address cannot come to mean different things on
    /// the two paths — and, since it is the one entry point, the place the
    /// refusal below belongs
    /// (`docs/principles/0090-a-surface-offers-it-never-decides.md`).
    ///
    /// **A bare name is refused where the nodes it lands on are not under one
    /// authority**, and the refusal names them: see [`CrossesAuthority`]. It is
    /// checked before anything is written, so a refused write moves nothing —
    /// half of a wildcard landing is the plausible wrong picture
    /// `docs/principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md`
    /// rules out.
    ///
    /// **The addressed form is not checked, and that is what this decision
    /// leaves for the asker.** Authority says *who* may move a node, so
    /// refusing one write of a node and not another needs the write to say
    /// whether an operator or an agent is asking — and nothing in this
    /// workspace does: no surface writes a param on an agent's behalf at all.
    /// What is refused here needs no asker, because a control spanning two
    /// arrangements is one whoever is holding it. See
    /// `docs/adr/0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md`.
    pub fn write_param(&mut self, write: &ParamWrite) -> Result<usize, CrossesAuthority> {
        match write.at {
            None => {
                if let Some(refused) = CrossesAuthority::over(&write.key, &self.landing(&write.key))
                {
                    return Err(refused);
                }
                Ok(self.set_param(&write.key, write.value))
            }
            Some((layer, index)) => Ok(usize::from(self.set_param_at(
                layer,
                index,
                &write.key,
                write.value,
            ))),
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
        for layer in Kind::ALL {
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
    /// **In declaration order, because the position in this list is an
    /// address.** A MIDI control is learned against *the deck and the position
    /// in its published interface* — `docs/manual/console.html`, "A knob is
    /// bound to a deck, not to a Set" — so this order is what the Inspector
    /// numbers its rows with. An authored interface is the author's own, in the
    /// order they published it: [`Set::publish`] appends and this hands the list
    /// back as it stands. The default interface is the Set's own: node by node
    /// in the order the nodes run, and inside a node the order its procedure
    /// declared them, each key taken where it first appears.
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
        //
        // **The requirement is that the order holds still, and alphabetical met
        // it badly.** This walked `ranges` and sorted the keys, and the sort was
        // there for a real reason: the keys came out of a `HashMap`, so left
        // alone they came out in a different order on every run, and a console
        // showing its controls in a different order each run is not a console —
        // a MIDI map is worthless against an order that moves. Spelling holds
        // still and says nothing: the number read down the pane skipped about,
        // and the order was an accident the Set's author never chose and could
        // not change without renaming a parameter.
        //
        // **What keeps it still now is that nothing here reads a map's order.**
        // [`Set::declared_names`] is a `Vec` per layer, each entry a node's own
        // `param_names` — cloned from the procedure's `param` list at build time
        // and never re-derived — and [`Kind::ALL`] is a constant array. So the
        // walk is the same walk every run, on a Set built the same way, and the
        // order it produces is one the author wrote rather than one the hasher
        // happened to. `ranges` is still asked for each key's declared range,
        // which is a lookup and not an iteration.
        //
        // Linear membership rather than a `HashSet`, and that is the point: a
        // set would decide *whether* a key is new, which is all that is wanted,
        // but reaching for one here is how the iteration this walk must not do
        // gets back in. An interface is tens of controls and this is off the
        // frame path.
        let mut keys: Vec<&String> = Vec::new();
        for layer in Kind::ALL {
            for names in self.declared_names(layer) {
                for key in names {
                    if !keys.contains(&key) {
                        keys.push(key);
                    }
                }
            }
        }
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
    /// `Ok(false)` if nothing publishes that name.
    ///
    /// The value is clamped to the *published* range, which is the one place
    /// narrowing bites: a console cannot ask for more than a Set offered. An
    /// agent that wants the whole declared range writes the param by address
    /// instead — see [`Published`] on why that is deliberate.
    ///
    /// **The refusal is carried rather than flattened, and this is the route it
    /// was decided for.** A published control is a wildcard unless the author
    /// named a node — `Published::at` is an `Option` and the default interface
    /// is entirely bare names — so this is the path
    /// [`CrossesAuthority`] exists to stop, and answering it with the same
    /// `false` that means *nothing publishes that name* would lose the sentence
    /// on the one route the decision was about
    /// (`docs/adr/0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md`,
    /// `docs/principles/0090-a-surface-offers-it-never-decides.md`).
    /// The two answers are two types now: *there is no such control* and *this
    /// control spans two authorities* are not the same news.
    pub fn set_published(&mut self, name: &str, value: f32) -> Result<bool, CrossesAuthority> {
        let Some(control) = self.published().into_iter().find(|p| p.name == name) else {
            return Ok(false);
        };
        let clamped = value.clamp(control.range[0], control.range[1]);
        // **Through `write_param`**, which is the one entry point a `--param`
        // and a `param` record both come through — so a wildcard control means
        // exactly what a bare name means everywhere else, and an addressed one
        // means exactly what an addressed `--param` does.
        Ok(self.write_param(&ParamWrite {
            at: control.at,
            key: control.key,
            value: clamped,
        })? > 0)
    }

    /// A published control's position in `[0, 1]`, which is what a binding's
    /// curve and range expect. `0.0` for a name nothing publishes — a binding on
    /// a control that is not there resolves to the bottom of its own range,
    /// which is quieter than a panic on the render thread and is what every
    /// other miss in this file does.
    fn control_position(&self, name: &str) -> Option<f32> {
        // **Without allocating**, which `published()` cannot promise: this is
        // called from `resolve_bindings`, which `Set::prepare` calls, and
        // nothing on the render thread allocates. So the interface is searched
        // in place and the default
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
        for layer in Kind::ALL {
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

    /// **Make one renderer live and the rest not** — selecting among the
    /// alternatives a composited Set folds. `false` if there is no such
    /// renderer, and nothing is written in that case.
    ///
    /// Three things this is not, each stated here because a reader meets the
    /// control before they meet what it costs:
    ///
    /// - **It saves the fold, not the frame.** Every renderer still draws,
    ///   into a cleared target of its own, exactly as it did before one of
    ///   them was selected — [`Set::draw`] gives each its target whatever its
    ///   edge says, and only the fold skips the ones that are not live. So an
    ///   alternative nobody is watching costs a render pass and a frame-sized
    ///   target: 7.03 MB at 1280x720. That is *cheap* rather than free, and it
    ///   is the reason this half of a variant pool is buildable at all — the
    ///   alternatives are already resident and already drawing, so choosing
    ///   between them is a uniform write.
    /// - **It covers L4 and only L4.** These are renderers over one
    ///   simulation, so alternatives that differ in how the material is *drawn*
    ///   are what a Set can hold. An alternative that differs at L1 or L2
    ///   carries state of its own, and selecting between those means a second
    ///   Set, priming it off air, and sharing the geometry across the two —
    ///   none of which exists. See
    ///   `docs/adr/0148-a-variant-pool-is-a-set-and-the-deck-stays-a-mixer.md`.
    /// - **It is saved with the Set and not with the performance.**
    ///   [`Layering`] *is* a Set file record — `merge` — and the choice this
    ///   makes rides on it as that record's `live`, so a composited Set written
    ///   out and loaded back composites and comes up folded to the renderer it
    ///   was folded to. What is still not a Set file's is a `select` record: it
    ///   names a deck slot at an instant, and no session record says which
    ///   slot's Set composites, so a stream folded down cannot tell whether a
    ///   selection it meets is about the Set being written. See
    ///   `karakuri_store::record::Record::Merge`.
    ///
    /// **Silently ineffective under [`Layering::Overdraw`]**, on
    /// [`Set::set_input`]'s terms and for its reason: the edges exist either
    /// way and nothing reads them without an L5.
    pub fn select_renderer(&mut self, at: usize) -> bool {
        crate::mix::select(&mut self.edges, at)
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
        let addressed: Vec<(Kind, u32, usize)> = Kind::ALL
            .into_iter()
            .flat_map(|layer| {
                self.nodes_of(layer)
                    .enumerate()
                    .map(move |(index, slot)| (layer, index as u32, slot))
            })
            .collect();
        addressed.into_iter().flat_map(move |(layer, index, slot)| {
            self.params[slot]
                .iter()
                .map(move |(k, v)| (layer, index, k.as_str(), *v))
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
    /// nothing on it allocates.
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
    /// **A monitor cell is the case where the frame before *is* drawn**, and
    /// it is the one place that discontinuity is visible: a warming slot is
    /// shown at its own grid position, so material that reads `beats` or
    /// carries a binding moves the moment it goes on air, by however far behind
    /// the governor's rate had it. Every slot is drawn on every frame now
    /// (see "Every slot is drawn; only a Live slot is mixed" in
    /// [`crate::deck`]), so this is visible on the console rather than
    /// hypothetical. Named rather than closed, because the close is to read the
    /// session's grid instead — which is the defect this split cost a repair to
    /// fix.
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
            let bindings = &self.bindings;
            let field_range = self.nodes_of(Kind::Field);
            let field_maps = &self.params[field_range.clone()];
            let field_bound = &self.field_bound;
            let field_params = &self.field_params;
            let params = &self.params;
            let source_bound = &self.source_bound;
            let source_salts = &self.source_salts;

            // **Every L1 procedure resolves against its own map**, and a paired
            // geometry is one of them. Handing it the near side's was a silent
            // miss for every name the two do not share: `sphere_shell`'s
            // `radius` was looked up in `lattice_shell`'s map, came back
            // `None`, and the sphere collapsed to the origin — a picture with a
            // shape in it, drawn from a value nobody set.
            //
            // The index is the *procedure's*, read off the source rather than
            // counted along with the walk: a Set whose slot is bound to the
            // first geometry builds the far side first, so counting would hand
            // each simulation the other one's map.
            for source in &mut self.sources {
                let procedures = source.procedures.clone();
                for (k, sim) in std::iter::once(&mut source.sim)
                    .chain(source.paired.as_mut())
                    .enumerate()
                {
                    let at = procedures[k];
                    let own = &params[at];
                    let param = |name: &str| effective(bindings, own, Kind::L1, at, name);
                    let tick = crate::node::Tick {
                        steps,
                        dt: self.dt,
                        instants,
                        param: &param,
                        field_params,
                        // **The procedure's node index**, which is `at` and not
                        // `k`: a bound geometry builds the far side first, so
                        // counting along the walk would read the other
                        // simulation's slots.
                        field_value: &|name: &str| field_value(field_bound, field_maps, at, name),
                        // **The procedure's node index too**, and for the
                        // reason above it: a bound geometry builds the far
                        // side first, so counting along the walk would hand
                        // one simulation the other's slots.
                        source_value: &|key: &str| {
                            source_value(source_bound, source_salts, at, key)
                        },
                    };
                    sim.prepare(queue, &tick);
                }
            }
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
    /// Split out of [`Set::prepare_on`] because a monitor draw needs it without
    /// the rest: an off-air slot is drawn and nothing prepared it, and every field there
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
        // **The cameras' edges, not a renderer's field.** These go in here
        // rather than into each uniform because a camera has several readers;
        // the aspect ratio goes with them because a renderer no longer knows
        // what projection it is drawing under. Both are writes rather than
        // passes — the derivation is recorded in [`Set::draw`].
        //
        // Which producer gets written is the node's decision and not this
        // one's: a Set hands down the frame and the built-in's six numbers, and
        // an L3 uses the first while the orbit uses the second.
        if let Some(merge) = &self.merge {
            merge.write_uniform(queue, &self.edges);
        }
        {
            // **Every camera, each against its own parameter map.** The index
            // is the node's, so a Set of two writes `L3:0`'s params into the
            // first and `L3:1`'s into the second — one address, one producer,
            // however many renderers read it.
            //
            // **`slot_of` rather than an arithmetic of its own**, and it is
            // sound now in a way it was not: the camera layer is never empty,
            // so `slot_of(L3)` names a camera's map rather than the first
            // renderer's. That was the hazard this block used to work around.
            let first = self.slot_of(Kind::L3);
            let aspect = self.viewport[0] / self.viewport[1];
            let field_range = self.nodes_of(Kind::Field);
            let (bindings, dt, field_params, field_bound, field_maps, viewport, beats, salt) = (
                &self.bindings,
                self.dt,
                &self.field_params,
                &self.field_bound,
                &self.params[field_range.clone()],
                self.viewport,
                self.last_beats,
                self.seed_salt,
            );
            // **Always `None` here, and it is a statement rather than a
            // stub.** An L3 may not declare a Source slot — it runs once a
            // frame over no geometry — so a camera's uniform has no such field
            // for the walk to find, and the answer is never asked for.
            let no_sources = |_: &str| None;
            let fallback = self.camera.state(t);
            let params = &self.params[first..];
            for (at, (camera, params)) in self.cameras.iter_mut().zip(params).enumerate() {
                camera.write_canvas(queue, aspect);
                let view = crate::node::View {
                    t,
                    beats,
                    seed_salt: salt,
                    viewport,
                    field_params,
                    field_value: &|name: &str| {
                        field_value(field_bound, field_maps, first + at, name)
                    },
                    source_value: &no_sources,
                    param: &|name: &str| effective(bindings, params, Kind::L3, at, name),
                };
                camera.prepare(queue, &view, dt, &fallback);
            }
        }
        let field_range = self.nodes_of(Kind::Field);
        let (bindings, beats, viewport, field_params, field_bound, field_maps) = (
            &self.bindings,
            self.last_beats,
            self.viewport,
            &self.field_params,
            &self.field_bound,
            &self.params[field_range.clone()],
        );
        let (source_bound, source_salts) = (&self.source_bound, &self.source_salts);
        // **Asked rather than re-derived.** This line spelled out `1 +
        // deforms.len()` and was right until an L3 landed between the
        // deformations and the renderers — after which every renderer read the
        // node before it, and `soft_points` drew a black frame because its
        // `exposure` resolved against the camera's parameter map. `slot_of` is
        // the one answer to "where does this layer start"; a second copy of it
        // is a second thing to remember to change.
        let first = self.slot_of(Kind::L4);
        // **The procedure's index, not the instance's.** Every source runs the
        // same renderers in the same order, so the value addressed at
        // `L4:2:exposure` is written into every source's third one — which is
        // what makes one address mean one thing however many sources there are.
        let params = &self.params[first..];
        for source in &mut self.sources {
            // **The source's salt, not the Set's.** `docs/ir-spec.md` moves it
            // from per layer to per source so that two identical geometries
            // differ in colour by default rather than by being arranged to.
            let salt = source.salt;
            for (at, (renderer, params)) in source.renderers.iter_mut().zip(params).enumerate() {
                let view = crate::node::View {
                    t,
                    beats,
                    seed_salt: salt,
                    viewport,
                    field_params,
                    field_value: &|name: &str| {
                        field_value(field_bound, field_maps, first + at, name)
                    },
                    // **The procedure's index, like the params beside it.**
                    // Every source runs the same renderers, so a slot bound on
                    // `L4:2` names one geometry in every instance — which is
                    // exactly what makes `source == only` select one of them.
                    source_value: &|key: &str| {
                        source_value(source_bound, source_salts, first + at, key)
                    },
                    param: &|name: &str| effective(bindings, params, Kind::L4, at, name),
                };
                renderer.write_uniforms(queue, &view);
            }
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
        let field_range = self.nodes_of(Kind::Field);
        let (bindings, beats, viewport, dt, field_params, field_bound, field_maps) = (
            &self.bindings,
            self.last_beats,
            self.viewport,
            self.dt,
            &self.field_params,
            &self.field_bound,
            &self.params[field_range.clone()],
        );
        let (source_bound, source_salts) = (&self.source_bound, &self.source_salts);
        let capacity = self.sources[0].sim.capacity();
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
        // **Where this layer starts in node order**, which is what a field
        // binding is keyed by: `at` below is the deformer's ordinal and the
        // binding names the node.
        let first = range.start;
        let params = &self.params[range];
        for source in &mut self.sources {
            let salt = source.salt;
            for (at, (node, params)) in source.deforms.iter_mut().zip(params).enumerate() {
                let view = crate::node::View {
                    t,
                    beats,
                    seed_salt: salt,
                    viewport,
                    field_params,
                    field_value: &|name: &str| {
                        field_value(field_bound, field_maps, first + at, name)
                    },
                    source_value: &|key: &str| {
                        source_value(source_bound, source_salts, first + at, key)
                    },
                    param: &|name: &str| effective(bindings, params, Kind::L2, at, name),
                };
                node.write_uniforms(queue, &view, dt, capacity);
            }
        }
    }

    /// **Rewrite the L4 uniforms against the current viewport**, without
    /// advancing anything.
    ///
    /// For an off-air slot that nothing is preparing. `viewport` and
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
        let ranges: Vec<(usize, std::ops::Range<usize>)> = Kind::ALL
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
                // Nothing to resolve against, and `Set::bind` refuses the key
                // before this runs — see `Set::nodes_of`.
                Kind::Field => (0, 0..0),
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
    /// See "Priming" in [`crate::deck`] for why that is the right shape, and
    /// `docs/adr/0053-priming-runs-the-simulation-and-skips-rendering.md` for
    /// the "reduced resolution" it supersedes.
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
        let steps = if self
            .sources
            .iter()
            .flat_map(|s| &s.renderers)
            .all(|r| r.is_fullscreen())
        {
            0
        } else {
            steps
        };
        for source in &mut self.sources {
            source.sim.record(encoder, steps);
            // **The paired geometry steps too.** It is a simulation, not a
            // buffer: it has its own `element` block and its own clock, and a
            // Set that stepped only the near side would pair a moving geometry
            // with a frozen one.
            if let Some(other) = &mut source.paired {
                other.record(encoder, steps);
            }
        }
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
        // **Each node dispatches over the range at *its* position**, which the
        // node above it decides. Walking it here rather than asking the
        // simulation once is the whole of what an amplifier costs the chain: it
        // multiplies the range for everything below it, and a stage handed the
        // simulation's counts instead would deform the first `range` of
        // `range * factor` elements and leave the rest holding the previous
        // frame.
        //
        // **Per source**, because the walk is over that source's own chain and
        // its own parity — two sources compact independently, so neither number
        // is shared.
        for source in &self.sources {
            let parity = source.sim.parity();
            let mut counts = source.sim.counts();
            for node in &source.deforms {
                node.record(encoder, parity, counts);
                if let Some(own) = node.counts() {
                    counts = own;
                }
            }
        }
    }

    /// Every amplifier's derived counts, in chain order.
    ///
    /// Chain order because a second amplifier derives from the first, and one
    /// invocation each because a count does not scale with anything.
    fn record_counts(&self, encoder: &mut wgpu::CommandEncoder) {
        for node in self.sources.iter().flat_map(|s| &s.deforms) {
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
        if self.sources.iter().all(|s| s.deforms.is_empty()) {
            return;
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("prime the deformation chain"),
        });
        self.record_counts(&mut encoder);
        for source in &self.sources {
            let parity = source.sim.parity();
            let mut counts = source.sim.counts();
            for node in &source.deforms {
                node.record(&mut encoder, parity, counts);
                if let Some(own) = node.counts() {
                    counts = own;
                }
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
    fn output_counts<'a>(&self, source: &'a Source) -> &'a wgpu::Buffer {
        source
            .deforms
            .iter()
            .rev()
            .find_map(|node| node.counts())
            .unwrap_or_else(|| source.sim.counts())
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
    /// looking moving it — see "Every slot is drawn; only a Live slot is mixed"
    /// in [`crate::deck`].
    ///
    /// **What a Set decides is the order and which one clears**, not how any of
    /// them draws. The renderers run in list order over the one attachment, the
    /// first clearing it and the rest loading what is there — see
    /// [`Set::build_many`] for why that is overdraw and not compositing.
    pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        // **Ahead of every renderer, and here rather than in [`Set::step`].**
        // The camera is an input edge of an L4, so it has to be current wherever
        // an L4 runs — and a preview draws a slot that nothing stepped. One
        // pass per camera the Set holds: a renderer bound to the second one
        // needs it derived exactly as much as the first.
        for camera in &self.cameras {
            camera.record(encoder);
        }
        // **The presence of an L5 is what decides overdraw from compositing**,
        // and it decides it here, in the one place the renderers are given
        // somewhere to draw. Under overdraw they share `target` and the first
        // one clears it; under compositing each renderer *procedure* has a
        // cleared target of its own.
        //
        // **"First" is about the attachment, not about the list**, which is
        // what makes several sources fit without a second rule: whoever writes
        // an attachment first clears it and everyone after loads. Under
        // overdraw that is the very first draw of the frame; under compositing
        // it is the first source, since every source draws into the target its
        // renderer procedure owns.
        //
        // The counts are the chain's output rather than the simulation's — a
        // renderer draws `instance_count` instances of whatever reached it, and
        // below an amplifier that is `factor` times what the simulation holds —
        // and they are each source's own, because two sources compact
        // independently.
        let merge = self.merge.as_ref();
        for (source_at, source) in self.sources.iter().enumerate() {
            let (parity, counts) = (source.sim.parity(), self.output_counts(source));
            for (i, renderer) in source.renderers.iter().enumerate() {
                match merge {
                    None => {
                        renderer.draw(encoder, target, parity, counts, source_at == 0 && i == 0)
                    }
                    Some(merge) => {
                        renderer.draw(encoder, merge.target(i), parity, counts, source_at == 0)
                    }
                }
            }
        }
        if let Some(merge) = merge {
            merge.record(encoder, target);
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

/// **The salt a source takes when nothing assigned it one**, from the Set's
/// seed and the source's ordinal.
///
/// `docs/ir-spec.md` asks for a value *assigned* when a source is added and
/// recorded in the stream, and a Set file now carries one `seed` record per
/// geometry — so a Set that was saved arrives with its salts in hand and this
/// is never consulted for it. What is left for this is the case that has
/// recorded nothing: a bare `--set`, where the ordinal is the only thing there
/// is. The spec licenses exactly that — *where it came from stops mattering
/// once it is recorded* — so a value derived here and then written into a file
/// is an assigned value from the moment the file exists, and reordering the
/// list stops moving the colours at that same moment.
///
/// **Public because the writer needs the same answer.** `--save-set` records
/// what the run it describes is salted with, and it has no built Set to ask —
/// it writes the material and stops before a GPU is opened. A second formula
/// there would be two places for one fact, which is the shape this file has
/// already paid for twice.
///
/// An odd multiplier, so that adjacent ordinals do not give adjacent salts —
/// `hash1` mixes, but a salt that walks by one is a salt whose first mixing
/// round is nearly the same.
pub fn derived_salt(seed_salt: u32, source: usize) -> u32 {
    seed_salt.wrapping_add((source as u32).wrapping_mul(0x9E37_79B9))
}

/// A spliced field's parameter value, by the **semantic** name the layout holds
/// and the node whose uniform is being written.
///
/// The map is keyed by the declared name, so the prefix comes off here — one
/// place, rather than at each of the four nodes that write it. The separator is
/// a character no `.kir` identifier can contain, which is what makes this
/// strip unambiguous — see `layout::field_param_key`.
fn field_value(
    bound: &[(usize, String, usize)],
    maps: &[HashMap<String, f32>],
    node: usize,
    key: &str,
) -> Option<f32> {
    // **Both separators come off and the slot decides which map is read.** A
    // key names the slot a param was reached through, and the slot is what
    // says which field that is: `Field:0` and `Field:1` are two procedures
    // with two `radius`es, and a caller's `shape(p)` reads exactly one of
    // them. This line used to discard the slot and hand every caller the
    // Set's only field, which was right for as long as there was only one.
    //
    // **The node is asked as well**, because a slot's spelling is the caller's:
    // two renderers may each declare `shape` and be bound to different fields,
    // and the key alone cannot tell those apart.
    let (slot, declared) = key.strip_prefix("field\u{1}")?.split_once('\u{1}')?;
    let (_, _, ordinal) = bound
        .iter()
        .find(|(at, name, _)| *at == node && name == slot)?;
    maps.get(*ordinal)?.get(declared).copied()
}

/// **The declared defaults one node enters a Set with**, keyed by the name the
/// `.kir` declares.
///
/// **The fold is `karakuri-ir`'s, not this file's.** It was a private function
/// here with a note saying a second evaluator elsewhere would agree with the
/// shader only by coincidence; `karakuri-environment`'s metadata writer is that
/// elsewhere, and it records the same number in a `param_decl`. Two folds could
/// disagree, and the disagreement would be a metadata file describing a run
/// that never happened.
///
/// **A function rather than the closure inside [`Set::build`] it used to be**,
/// and that is the whole of what it buys: `Set::build` needs a device, so the
/// engine's *use* of the one fold could only be reached through a GPU. The
/// closure was named and lifted out so that
/// `tests::the_engines_param_map_reads_a_negative_default_through_the_ir_fold`
/// can ask this function the same question
/// `a_negative_param_default_is_read_as_its_declared_value` asks
/// `Param::default_scalar`, on a machine with no adapter. A param this cannot state a number for is left out of the
/// map entirely — the node then has no value for it and its uniform never packs
/// one, which is how a `vec3` param stays undriven rather than being packed as
/// a scalar.
fn declared_defaults(node: &Checked) -> HashMap<String, f32> {
    node.params
        .iter()
        .filter_map(|p| p.default_scalar().map(|v| (p.name.clone(), v)))
        .collect()
}

/// **Which geometry a declared Source slot names, as its assigned identity** —
/// the answer behind one uniform key, for the node that declared the slot.
///
/// [`field_value`]'s shape, one separator shorter: a key carries the slot and
/// the *node* is asked as well, because a slot's spelling is the declaring
/// node's and two nodes may each call one `only` while meaning different
/// geometries.
///
/// **The salt is looked up rather than stored**, so that a source re-salted by
/// `--load-set` is answered for by the value it ended up with rather than the
/// one the binding was resolved against.
fn source_value(
    bound: &[(usize, String, usize)],
    salts: &[u32],
    node: usize,
    key: &str,
) -> Option<u32> {
    let slot = key.strip_prefix("source\u{1}")?;
    let (_, _, at) = bound
        .iter()
        .find(|(node_at, name, _)| *node_at == node && name == slot)?;
    salts.get(*at).copied()
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
    /// `= -0.35` parses as a negation of a literal rather than as one, and the
    /// fold matched `Expr::Lit` alone — so a legal `.kir` had its declared
    /// default silently discarded, could not be bound, and reached the shader as
    /// whatever the miss produced. No example declares one, which is the only
    /// reason it was never seen; nothing in the language forbids it.
    ///
    /// **Asked of `karakuri_ir::Param::default_scalar`, which is where the fold
    /// now lives** — it was private here until the metadata writer needed the
    /// same number. This stays because it is the *engine's* reading that broke,
    /// and the value it reads is what a uniform is packed from.
    ///
    /// **It does not, on its own, pin the engine to that fold.** This doc used
    /// to say the map a uniform is packed from "is built from exactly this
    /// call", and after the fold moved out that stopped being true of anything
    /// here: replacing `Set::build`'s call with a divergent local fold left
    /// every lib test in this crate green. The engine's *use* is pinned by
    /// `the_engines_param_map_reads_a_negative_default_through_the_ir_fold`
    /// below, which is why [`declared_defaults`] is a function.
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
    point_rate = 0.004;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, drift + plain);
  }
}
"#;
        let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
        let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
        let of = |name: &str| {
            checked
                .params
                .iter()
                .find(|p| p.name == name)
                .expect("the param is declared")
                .default_scalar()
        };
        assert_eq!(
            of("drift"),
            Some(-0.35),
            "a negative default was read as an absence"
        );
        assert_eq!(
            of("plain"),
            Some(0.25),
            "a positive default stopped being read"
        );
    }

    /// **The engine reads its declared defaults through `karakuri-ir`'s fold
    /// and through no reader of its own.**
    ///
    /// The number a node enters a Set with and the number
    /// `karakuri-environment`'s metadata writer puts in a `param_decl` are one
    /// declaration read twice, and the rule that keeps them equal is that both
    /// call `karakuri_ir::Param::default_scalar`. Nothing in this crate held
    /// the engine to that: the fold moved to `karakuri-ir`, the test above
    /// followed it there, and a divergent local fold in
    /// [`declared_defaults`] passed every lib test here — the disagreement
    /// showed up only in `karakuri-cli`'s
    /// `the_engine_and_the_metadata_writer_cannot_disagree_about_a_default`,
    /// which builds a `Set` and so needs an adapter. On a machine with no GPU
    /// the engine half of the one-fold rule was unguarded.
    ///
    /// [`declared_defaults`] is that map's builder, lifted out of `Set::build`
    /// so this question can be asked without a device. **A negative default is
    /// what asks it**, because that is the one declaration the two folds have
    /// actually disagreed about: a reader matching `Expr::Lit` alone reads
    /// `-0.35` as an absence, and an absent entry is a param the uniform never
    /// packs.
    ///
    /// What is still only in `karakuri-cli` is the *packing* and the card
    /// beside it — this pins the map, not the bytes in the uniform buffer.
    #[test]
    fn the_engines_param_map_reads_a_negative_default_through_the_ir_fold() {
        let src = r#"
proc signed_defaults {
  kind  L4
  blend additive

  param drift  : float [-1.0, 1.0] = -0.35
  param plain  : float [ 0.0, 1.0] =  0.25

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = 0.004;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, drift + plain);
  }
}
"#;
        let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
        let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
        let map = declared_defaults(&checked);
        assert_eq!(
            map.get("drift").copied(),
            Some(-0.35),
            "the map a uniform is packed from has stopped agreeing with \
             `karakuri_ir::Param::default_scalar`: a fold of this file's own read `-0.35` \
             as an absence, so the run loads a default the `param_decl` beside it does not \
             state"
        );
        assert_eq!(
            map.get("plain").copied(),
            Some(0.25),
            "the map a uniform is packed from has stopped agreeing with \
             `karakuri_ir::Param::default_scalar` about an ordinary positive default"
        );
    }

    /// **A node nobody has spoken for is the operator's.**
    ///
    /// `docs/adr/0211-…` never states a default in so many words, so this is
    /// where the one the engine took is written down and held. Rule 06 is about
    /// what an operator *grants* — *"There is no switch that hands the whole
    /// instrument to an agent"* — and any other default would be exactly that
    /// switch, thrown for every node of every Set at build time and by nobody.
    ///
    /// Asserted through `Default` rather than by naming the variant twice: the
    /// build path fills the list with `Authority::default()`, so this is the
    /// value it actually puts there.
    ///
    /// No GPU: this is the value list and its default, and pinning it here
    /// rather than through a built `Set` is what keeps the failure legible.
    #[test]
    fn a_node_nobody_has_spoken_for_is_manual() {
        assert_eq!(Authority::default(), Authority::Manual);
        assert_eq!(
            Authority::ALL[0],
            Authority::default(),
            "the list should start where a node starts"
        );
    }

    /// **The three words a record is read back with**, and the round trip that
    /// keeps `Record::Authority`'s `String` readable by this build.
    ///
    /// They are rule 06's own — *manual*, *suggesting*, *automatic* — and not
    /// the console's `man / sug / auto`, which is a node head's abbreviation
    /// for a reader. `karakuri_operation::Authority::name` writes the same
    /// three, and `karakuri-operation-record`'s
    /// `the_three_authority_levels_are_named_as_the_record_spells_them` is the
    /// other half of the pair.
    #[test]
    fn every_authority_has_a_name_and_answers_to_it() {
        assert_eq!(Authority::Manual.name(), "manual");
        assert_eq!(Authority::Suggesting.name(), "suggesting");
        assert_eq!(Authority::Automatic.name(), "automatic");
        for level in Authority::ALL {
            assert_eq!(
                Authority::from_name(level.name()),
                Some(level),
                "`{}` does not read back as the level that wrote it",
                level.name()
            );
        }
        assert_eq!(
            Authority::from_name("man"),
            None,
            "the console's abbreviation is a surface's vocabulary, not a record's"
        );
    }

    /// **A bare name over nodes that disagree is refused, and the refusal names
    /// every node it would have landed on.**
    ///
    /// The decision half of the wildcard rule
    /// (`docs/adr/0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md`),
    /// taken apart from the walk that finds the nodes so it can be checked with
    /// no device — `docs/adr/0130-…`. `Set::landing` supplies the pairs in a
    /// run and the `mod gpu` test below drives the whole path through a real
    /// Set; what is pinned here is what counts as disagreement and what the
    /// sentence says.
    ///
    /// **The three uniform cases are the negative control**
    /// (`docs/principles/0089-a-check-you-have-not-watched-fail-is-guessing.md`): a check that refused every landing would
    /// pass the mixed case on its own, and today every node of every Set is
    /// `Manual`, so an over-eager rule would refuse every `--param` in the
    /// program and this is the assertion that would not let it.
    #[test]
    fn a_bare_name_is_refused_only_where_the_nodes_it_lands_on_disagree() {
        let uniform = [
            (Kind::L1, 0, Authority::Manual),
            (Kind::L4, 0, Authority::Manual),
            (Kind::L4, 1, Authority::Manual),
        ];
        assert_eq!(
            CrossesAuthority::over("radius", &uniform),
            None,
            "three nodes the operator kept are one arrangement, not a disagreement"
        );
        assert_eq!(
            CrossesAuthority::over("radius", &[]),
            None,
            "a key this Set declares nowhere lands on nothing, which is `no parameter \
             named` and not a refusal"
        );
        assert_eq!(
            CrossesAuthority::over("radius", &[(Kind::L4, 0, Authority::Automatic)]),
            None,
            "one node cannot disagree with itself, whatever it was granted to"
        );

        let refused = CrossesAuthority::over(
            "radius",
            &[
                (Kind::L1, 0, Authority::Manual),
                (Kind::L4, 0, Authority::Automatic),
            ],
        )
        .expect("a node the operator kept and a node an agent acts on are two arrangements");
        assert_eq!(refused.key, "radius");
        assert_eq!(
            refused.landing, "L1:0 manual, L4:0 automatic",
            "the refusal names every node the write lands on, with what each is under"
        );
        let sentence = refused.to_string();
        assert!(
            sentence.contains("L1:0 manual, L4:0 automatic") && sentence.contains("`radius`"),
            "the one sentence has to carry the control and the nodes that disagreed: \
             {sentence}"
        );
    }

    // The three that build a Set for real. The ones above check the layout arithmetic
    // `Set::build` would go on to use, and take no device — which is most of what
    // `cargo test -p karakuri-engine --lib -- --skip gpu::` is for.
    // See `../tests/gpu_tests_are_under_mod_gpu.rs`.
    mod gpu {
        use super::*;

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
    point_rate = 0.004;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
            let compile = |src: &str| -> Checked {
                let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
                let checked =
                    karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
                karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("cost: {e:?}"));
                checked
            };
            let l1 = compile(l1_src);
            let l4 = compile(l4_src);
            let set = Set::build(&gpu.device, &gpu.queue, &l1, &l4, 8, 1).expect("compatible pair");

            assert_eq!(
                set.live_count(&gpu.device, &gpu.queue),
                0,
                "a spawn-block procedure starts empty"
            );
        }
        /// **A wildcard write is refused where the nodes it lands on are not
        /// under one authority, and it moves nothing when it is.**
        ///
        /// The whole path through a real Set:
        /// [`Set::landing`] finds the nodes a bare name reaches,
        /// [`CrossesAuthority::over`] decides, and [`Set::write_param`] is the
        /// one entry point that asks. The CPU test above pins the decision; this
        /// pins the two things only a built Set can answer — that the walk
        /// pairs each node with *its own* authority, and that a refusal leaves
        /// every value where it was.
        ///
        /// **What it is about**
        /// (`docs/adr/0223-a-wildcard-write-is-refused-where-the-nodes-it-lands-on-disagree.md`):
        /// granting an agent one node of a Set would otherwise grant it, through
        /// any bare-name control declaring that key, the node the operator kept
        /// — which is rule 06's *"there is no switch that hands the whole
        /// instrument to an agent"* at the width of a key. Landing on the
        /// permitted nodes instead is the alternative that lost, to
        /// `docs/principles/0027-a-silently-wrong-image-loses-to-a-loud-failure.md`,
        /// and the assertion that the values did not move is what holds that
        /// half.
        ///
        /// **The addressed write at the end is the second negative control.**
        /// What this rule takes away is one spelling and never the reach: an
        /// operator who granted a node can still write it, and a rule that had
        /// refused this too would have made a granted node unreachable.
        #[test]
        fn a_wildcard_write_is_refused_where_the_nodes_it_lands_on_disagree() {
            let gpu = Gpu::headless().expect("no GPU available");
            let l1_src = r#"
proc probe_shared_l1 {
  kind     L1
  topology points
  capacity [4, 64] = 8

  param radius : float [0.0, 8.0] = 1.0

  emit position

  element {
    position = vec3(radius, 0.0, 0.0);
  }
}
"#;
            let l4_src = r#"
proc probe_shared_l4 {
  kind  L4
  blend additive

  param radius : float [0.0, 8.0] = 1.0

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_rate = radius;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0);
  }
}
"#;
            let compile = |src: &str| -> Checked {
                let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
                let checked =
                    karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
                karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("cost: {e:?}"));
                checked
            };
            let l1 = compile(l1_src);
            let l4 = compile(l4_src);
            let mut set =
                Set::build(&gpu.device, &gpu.queue, &l1, &l4, 8, 1).expect("compatible pair");
            let radius = |set: &Set| -> Vec<(Kind, u32, f32)> {
                let mut found: Vec<(Kind, u32, f32)> = set
                    .params()
                    .filter(|(_, _, key, _)| *key == "radius")
                    .map(|(layer, index, _, value)| (layer, index, value))
                    .collect();
                found.sort_by_key(|(layer, index, _)| (format!("{layer:?}"), *index));
                found
            };

            // Nobody has spoken for either node, so the bare name is one
            // control over one arrangement and reaches both.
            assert_eq!(
                set.write_param(&ParamWrite::everywhere("radius", 2.0)),
                Ok(2),
                "a Set nobody has spoken for is uniform, and a bare name reaches every \
                 declaration of the key"
            );
            assert_eq!(
                radius(&set),
                vec![(Kind::L1, 0, 2.0), (Kind::L4, 0, 2.0)],
                "both declarations should hold what the wildcard wrote"
            );

            // One node handed to an agent, and the same control now spans two
            // arrangements.
            assert!(
                set.set_authority(Kind::L4, 0, Authority::Automatic),
                "the renderer is a node of this Set"
            );
            let refused = set
                .write_param(&ParamWrite::everywhere("radius", 7.0))
                .expect_err("a bare name over a kept node and a granted one is refused");
            assert_eq!(refused.key, "radius");
            assert_eq!(
                refused.landing, "L1:0 manual, L4:0 automatic",
                "the refusal names the nodes it would have landed on and what each is under"
            );
            assert_eq!(
                radius(&set),
                vec![(Kind::L1, 0, 2.0), (Kind::L4, 0, 2.0)],
                "a refused write moves nothing — landing on the permitted node is the \
                 silently partial control P-0027 rules out"
            );

            // And the reach is not what was taken away.
            assert_eq!(
                set.write_param(&ParamWrite::at(Kind::L4, 0, "radius", 7.0)),
                Ok(1),
                "an addressed write says which node it means, so it crosses nothing"
            );
            assert_eq!(
                radius(&set),
                vec![(Kind::L1, 0, 2.0), (Kind::L4, 0, 7.0)],
                "the addressed write lands on the node it names and on no other"
            );
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
            // Was a silent `return` — the one test in the workspace that
            // reported success for having done nothing at all. Now the same
            // panic as the rest; a machine without an adapter uses
            // `--skip gpu::` rather than a test that lies to it.
            let gpu = Gpu::headless().expect("no GPU available");
            const FACTOR: u32 = 4;
            const CAPACITY: u32 = 64;

            let compile = |src: &str| -> Checked {
                let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("parse: {e:?}"));
                let checked =
                    karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("check: {e:?}"));
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
    point_rate = 0.004;
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
                &[(&l1, CAPACITY)],
                &[&l2],
                &[],
                &[],
                &[&l4],
                Layering::Overdraw,
                1,
                &[],
                Wiring::default(),
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
                gpu.device
                    .poll(wgpu::PollType::wait_indefinitely())
                    .expect("poll");
                let data = slice.get_mapped_range().expect("map");
                let mut out = [0u32; 12];
                for (i, w) in data.chunks_exact(4).take(12).enumerate() {
                    out[i] = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
                }
                drop(data);
                readback.unmap();
                out
            };

            let source = &set.sources[0];
            let from = read(source.sim.counts());
            let derived = read(source.deforms[0].counts().expect("the node amplifies"));

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
}
