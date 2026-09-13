# Karakuri — Vision and Roadmap

**This file is the ledger of open work.** It is not a manual and not a design document.
Before implementing anything here, read [principles/](principles/) for the rules in force and
[contributing.md](contributing.md) §4 for the ADR lifecycle.
The code is [architecture.md](architecture.md); the language is [ir-spec.md](ir-spec.md); how to play
it is [manual.md](manual.md) and [manual/](manual/); why a thing was decided is [adr/](adr/);
milestones that closed are kept whole in [history/](history/).

**No count, figure or percentage is written into this file.** *The instrumentation*, below, is the
commands that derive them.

---

## The end state

A performer opens an empty session and types a prompt. Over the course of a set the system generates
visual material, warms it up out of sight, and offers it at musically sensible moments. The performer
takes control of anything they want and leaves the rest to the system. Everything generated is saved,
searchable and reusable in later sessions.

Four properties define whether this succeeded:

1. **Procedural, not generated frames.** The AI writes procedures that run on the GPU every frame,
   not images or video. What is saved is an instrument with parameters, not a recording.
2. **Manual and autonomous on the same mechanism.** A human and an agent affect the system through
   the identical interface. Authority is set per node of a Set
   ([ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md)). The senses each
   word carries are in [architecture.md](architecture.md#words-that-carry-more-than-one-sense).
3. **Reproducible.** The same record stream and the same seeds produce the same show.
4. **Unbreakable on stage.** Nothing stops working when the network drops or the DJ gear changes.
   Defended continuously rather than built once — see *Continuous concerns*.

### What this is not

- Not a general-purpose compositor. Video file playback and editing are out of scope except as an
  external source node.
- Not a 3D content creation tool. There is no modelling, no asset import pipeline, no animation
  timeline.
- Not primarily a live-coding environment for humans. The IR is written by LLMs and read by humans,
  not the other way round.
- Not a cloud service. The engine runs locally and keeps running when generation is unavailable.

### The reference point

TouchDesigner is the closest existing thing. It is largely one operator per GPU pass, connected by
texture ping-pong, driven from the CPU. Karakuri compiles a subgraph into a minimal set of passes and
fuses procedural chains into single shaders. The cost is a compilation step and less immediate
feedback; the gain is that primitive-centric generation at high element counts becomes affordable.

---

## Milestones

### M1 — Closing the loop — closed, [history/m1.md](history/m1.md)

### M2 — Playable — closed, [history/m2.md](history/m2.md)

### M3 — Expressive depth — closed, [history/m3.md](history/m3.md)

### M4 — Library at scale — closed, [history/m4.md](history/m4.md)

### M5 — Interface — closed 2026-09-14, [history/m5.md](history/m5.md)

### M6 — Autonomy

The system prepares material without being asked and is safe to let run. It follows M5 because an
autonomous system that cannot be observed and overridden is not usable on stage.

What it owes, all of it scheduled in M6:

- **Agents.** Each owns a prompt, watches designated inputs, selects among its prepared variants and
  queues generation for material it expects to need. Selection chooses among what already exists and
  never generates on a path the frame waits for. What an agent is attached to — a deck slot or a node
  — is undecided and is under *The decisions nobody has taken*.
- **Director.** Coherence across the kinds: whether the camera behaviour suits the geometry, whether
  L4 is killing the shape L1 produced.
- **Mix agent.** Set-level arc, energy trajectory, monitoring for staleness.
- **Authority model.** Budget constraints such as *camera changes at most once per four bars*, and
  manual intervention demoting to `Suggest` whichever agent was moving the control. The demotion
  writes `Operation::SetAuthority` on the node, under
  [P-0094](principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md).
- **Generation queue** with priority and cost awareness.
- **`parent`, recorded from the first generated artifact.** M6 is where something first generates a
  procedure, and an empty `parent` written before then fills the genealogy's root hole rather than
  leaving it visible. From M4.
- **A deck-slot variant pool**, where alternatives differ at L1 or L2 and so carry state of their
  own, and the unselected ones are primed off air. The half that lives inside one Set is built.
  From M4.

Demands on earlier work: the record stream is the sole mutation path, so an agent is structurally
incapable of doing anything a human could not do through the same interface; every procedure declares
parameter ranges, from M1, because an agent with no declared ranges has nothing to search over.

Exit condition: none recorded. ADR-0226 gave M5 one; M6's is the maintainer's.

### M7 — Musical foresight

Transitions land on phrase boundaries because the system knew they were coming. Audio analysis
reports what already happened; track analysis is anticipatory, which is what makes generation-time
latency compatible with live performance.

What it owes, all of it scheduled in M7:

- **rekordbox integration.** Beatgrid and transport position, current track identification, and
  phrase structure (`PSSI`) from the local collection database.
- **A lookahead timeline as a queryable structure**, not a signal — *the first Chorus within 16
  bars*.
- **Backward scheduling.** The transition point fixes priming start, which fixes generation start,
  and *will not make it* becomes a decision to postpone rather than a dropped frame.
- **Track metadata into visual direction.** Key to palette, colour tags, and a convention for writing
  VJ hints in the rekordbox comment field.
- **PRO DJ LINK via beat-link and crate-digger**, optionally, for club CDJs.

Demands on earlier work: the Set lifecycle must be schedulable by absolute time. A transition
schedules on beat count and nothing else, which is what makes a fade reproduce at a different frame
rate and follow a tempo change mid-fade, so absolute time is a second axis rather than a correction
and arrives with whatever needs it. Signal confidence must drive transition policy, these sources
being unofficial reverse-engineered integrations that will break.

Exit condition: none recorded.

### Mx — TODO

Work that is owed, has no milestone whose subject it is, and is not scheduled. It carries no number
because a holding list with a milestone number would claim a goal it does not have. Each entry names
what it is, what it needs, and the milestone it came from.

- **The mixer's mask has no front position.** *Set a deck's mask position* carries no badge in the
  panel or key column, so no exit condition reads it. It needs a surface that gives the mask's front
  a position; [ADR-0206](adr/0206-a-fader-marks-where-it-is-going-and-keeps-reaching-for-it.md) is
  the record. From M5.2.

- **A session stream cannot say what a deck held.** A Set file describes one Set, so only slot 0's
  material reaches the head of a recording and a multi-slot session replays the rest against a deck
  of one, reporting what it skipped. It needs a format change. From M2.

- **How a mask names a source.** The L2 `mask` block would compare a `Source` slot against a `source`
  uniform, specified in [ir-spec.md](ir-spec.md) under *A mask that names the source it applies to*.
  Not the mixer's mask, which is a different sense of the word and already reaches the panel.
  From M3.

- **`perf` on a card.** It is a measurement and needs the stage-7 probe; what a compile pass can
  answer is `ops_per_element`, a different quantity in different units. From M4.

- **Dual embeddings.** The text side needs `origin` and `tag`, which have no producer until something
  generates procedures; the visual side needs the thumbnail below. From M4.

- **Search over more than a node's name, and grouping.** Both need the same absent card fields.
  From M4.

- **An artifact card states no default for a vector param.** `meta.rs` writes
  `default: p.default_scalar()`, which is `None` for a `vec3` since
  [ADR-0268](adr/0268-a-vector-parameter-is-driven-one-component-at-a-time.md) gave the engine three
  keys and three numbers, so the card is silent where the run is not and the Library's reading draws
  that field. It needs a decision the metadata format owns: whether `Record::ParamDecl`'s `default`
  grows an array, or a card carries one record per component. M6's agents are what make it matter.
  From M4.

- **Thumbnails and live previews in the Set browser, and what a thumbnail is *of*.** The drawing was
  the Library bay's and the judgement was M4's, handed to that bay; neither has a row on
  [every operation](manual/operations.html), so no exit condition read either. From M5.3, and from M4
  before it.

- **MCP offers the effective limit beside the declared one.** A model writing a procedure is told the
  ceiling `cost::estimate` refuses at and never what this machine has afforded. The second number is
  inferable from the preparation slot's measurements and the governor's history; the two do not
  compare directly, a ceiling being in ops and a measurement in milliseconds. It needs the estimate
  wired and a run long enough to have a history. From M5.

- **A Set installed part-way through a session starts at `t = 0` against a clock that has run**, so
  material reading `beats` jumps when that slot goes on air. Recorded in
  [ADR-0269](adr/0269-a-slot-that-is-drawn-is-stepped-and-a-preview-runs-at-the-rooms-tempo.md).
  From M5.

- **Whether `l` loads onto the deck selection or onto the load pulldown's deck.**
  [ADR-0305](adr/0305-the-library-bays-load-is-a-button-and-a-pulldown-and-the-deck-it-names-is-not-the-selection.md)
  left the key on the selection and names this as its one assumption, the maintainer's to confirm.
  Changing it is a line in `crates/karakuri/src/main.rs` and a paragraph on two manual pages.
  From M5.3.

- **What the Library bay's filter fields should be.**
  [ADR-0262](adr/0262-a-library-filter-field-steps-through-what-the-store-already-holds-rather-than-taking-letters.md)
  deferred it to a console-wide decision about focus and
  [ADR-0292](adr/0292-the-pane-heads-name-takes-letters-and-the-keep-capsule-stays-a-stamp.md) took
  that decision, so the deferral is spent. Both built letter-taking flows are bounded to one path
  component and a filter is an unbounded fragment matched against what a node is called; ADR-0262's
  stepping stands as the first cut. From M5.3.

- **`written(TapBeat)` and `written(ScaleGrid)` answer `Owed(NotSettled)`.** A gap in
  `karakuri-operation-record` rather than in the panel — both controls are built
  ([ADR-0278](adr/0278-an-operation-no-record-can-be-written-for-leaves-the-window-before-it-is-written.md)).
  It needs somebody to say what a `Current` carries about a beat lock. From M5.4.

- **Which number the transport's frame readout carries.** The numerator is `Cost::whole`, the CPU's
  three stretches, while `Cost::period` is the frame
  ([ADR-0303](adr/0303-a-frames-cost-is-the-period-and-a-measurement-names-which-resolution-it-is-about.md)),
  so a frame the GPU is holding up draws as headroom. The period in place of the CPU figure, both
  side by side, or the tooltip's promise rewritten: all three are page changes before they are code.
  From M5.4.

- **The transport's own notes, read against the tips its controls now carry.** Every control in that
  row carries a `data-tip`; what is left is the reading rather than a drawing. From M5.4.

- **What `over_budget` says once `committed_ms` is the deck's total, and the budget is still measured
  over one slot rather than over the frame.** `committed_ms` and `headroom_ms` sum only Live slots,
  so since ADR-0269 they understate the deck by three slots of step and draw. Making `committed_ms`
  the total would leave `over_budget` permanently on for any four-slot deck of heavy material.
  [ADR-0313](adr/0313-a-candidate-is-judged-on-its-own-cost-and-the-decks-period-is-a-deck-level-alarm.md)
  put `Report::deck_over_period` beside it as a second flag rather than as an answer, and a frame
  interval cannot be divided among slots, which is what this needs. From M5.14.

- **Caching a bay to a texture.**
  [ADR-0283](adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)
  is its precondition and is built; ADR-0303's frame instrument is what would measure it.
  From M5.14.

- **Whether the risk badge marks an estimated number differently from a measured one.** The dot is
  the band of the number the governor spent, and which it was crosses the seam on `Decision::basis`
  and is drawn nowhere
  ([ADR-0298](adr/0298-the-badge-is-the-band-of-the-number-the-governor-spent-and-how-it-was-taken-crosses-the-seam-undrawn.md)).
  [P-0095](principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md)
  rules out handing an inferred number out as a measured one, so what is open is the mark the page
  gains, and that is the maintainer's. From M5.14.

- **The extrapolation is wrong on a concentrated Set.** `estimate` measures how much coverage a Set
  has and not how concentrated it is, so it is right at the sizes it measures and wrong extrapolating
  a concentrated Set to full size. The worked case is a Set whose elements land on one destination
  texel; no limit is added for it. It needs either a refusal on
  [ADR-0266](adr/0266-two-rungs-and-a-fit-because-a-frame-is-an-invariant-part-plus-a-fragment-part.md)'s
  terms or a fit that reads concentration as well as coverage. From M5.14.

- **`estimate` is built, exported and wired to nothing.**
  `crates/karakuri-engine/src/estimate.rs` draws a Set at two rungs, solves `a + b·area` through
  them, and returns either the fit or a named refusal (ADR-0266). Wiring it to `Deck::govern` is
  undecided, and it is what would produce the risk badge's estimate. From M5.14.

- **`estimate` refuses every `Points` and `Lines` Set with `Unfit::FloorUnknown`**, because
  `point_rate` is a per-element vertex expression the CPU side never reads, so the floor below is not
  a number it can compute; `estimate_above_floor` takes the floor from a caller that knows it.
  Closing it needs a static analysis of the `point_rate` expression against its params' declared
  ranges, and that is not written. From M5.14.

- **The four cost ceilings have never been measured against a frame time.**
  `MAX_OPS_PER_ELEMENT`, `MAX_OPS_PER_SPAWN`, `MAX_OPS_PER_FRAGMENT` and the fullscreen one arrived
  in one commit; the doc on the first says it will need retuning once stage-7 probe data exists, and
  that data exists. They are a backstop rather than a budget, the governor being what enforces.
  From M5.14.

- **What a pattern or a master chain saved under a name looks like.**
  [ADR-0227](adr/0227-a-pattern-and-a-master-chain-setting-are-library-data-in-two-tiers.md) puts the
  two in the library as two tiers and both were built without that half: no store directory holds a
  chain and `crates/karakuri-pattern` ships with no serialiser.
  [ADR-0340](adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md) says
  what the chain's file would hold — `Record::MasterChain`'s slot list — and leaves the directory's
  name, so what is owed is the tier rather than the format. From M5.8, M5.9 and M5.16.

- **Reordering the master chain has no operation, no control and no row.** The vocabulary's three
  chain operations name a position, add at the end and remove, and the Master bay draws no handle.
  ADR-0340 states it as a limit. From M5.16.

- **A Library drop on the chain appends rather than inserting at a position.**
  `Operation::AddChainEffect` carries no position, which is why the chain's list is one landing
  rectangle rather than one per slot
  ([ADR-0352](adr/0352-the-chains-list-is-the-master-bays-items-and-a-slot-is-taken-out-by-a-glyph-on-its-row.md)).
  An insert is a payload change to that operation. From M5.16.

- **A per-deck chain does not exist.** [architecture.md](architecture.md)'s *a per-deck effect and a
  master effect become one node at different points* is what `kind L5` makes possible, and there is
  one chain, at the master. No row and no control. From M5.16.

- **The tap weights are chosen against a raymarcher and not measured against a blur.**
  `crates/karakuri-ir/src/cost.rs` names `W_TEXEL`, `W_TAP` and `W_FRAME_STEP` as the three numbers a
  measurement should replace.
  `crates/karakuri-engine/tests/master.rs::the_three_shipped_procedures_price_the_chains_rate` fails
  if a re-weighting moves the chain's price silently, which is a guard rather than the measurement.
  ADR-0340 owes that pass and
  [ADR-0349](adr/0349-the-chain-is-the-frames-so-it-reads-the-session-clock-and-is-charged-against-the-frames-budget.md)
  names it again. From M5.16.

- **`mix::apply_chain` compiles on the caller's thread.** Where the chain's shape changed — an add, a
  remove, a drop — it compiles the procedures and allocates the targets inline, in
  `crates/karakuri/src/app/handler.rs`, `crates/karakuri-cli/src/live.rs` and
  `crates/karakuri-environment/src/render.rs`. A swapped-in Set gets a worker and the chain does not;
  [P-0091](principles/0091-cost-is-known-before-it-is-paid.md) is what the cheap path is written for.
  From M5.16.

- **Fullscreen on a chosen display.** The projector window is a second `winit` window with a surface
  and a `WindowSink`, made neither fullscreen nor put on a named monitor. It needs one line of
  `winit` and the monitor list this program does not read. From M5.6.

- **[manual.md](manual.md)'s *The canvas is fixed for a run* is false for the panel.** The render
  size belongs to the output and the frame follows the largest enabled one
  ([ADR-0325](adr/0325-the-frame-follows-the-largest-enabled-output-and-a-resize-costs-0-145-ms.md)),
  so the sentence holds only for `karakuri-cli`, whose `--canvas` is untouched. It is a page change
  on the CLI's own page. From M5.6.

- **A slot's estimate is dropped on every frame of a drag**, because `HotSwap::resize` drops it and
  the frame is re-derived per changed output size. `set_estimated_cost` has no caller today, so it
  costs nothing now and whoever wires the estimate meets it. From M5.6.

- **The watcher's other two dead ends go unreported.** `Source::poll` answers *I refused* for the
  checker's refusal only; the two other paths in `Watch::rebuild` — a file that will not read, and a
  stack that will not sort into a Set — still print and return `None`.
  [ADR-0310](adr/0310-a-source-can-say-it-refused-and-the-lane-draws-it.md) names them and declines
  to give either a word, so naming them is a further decision. From M5.7.

- **The Staging lane omits `origin`, the timestamp and the head's count.** Each is named at the code
  and none is a row on the page. The first has no producer anywhere; the second waits on the spelling
  the Library bay's own time column waits on. From M5.7.

- **ADR-0323's refusal is built and uncalled, so a live lane still kills a fade onto its deck.** A
  lane reaches `Deck::set_opacity`, which cancels, where
  [ADR-0323](adr/0323-a-scheduled-move-is-refused-on-a-control-a-lane-holds.md) decides the schedule
  is refused and the refusal names the lane. The lookup exists and the caller does not. From M5.9.

- **A channel fader does not say who is holding it.** The mock draws `seq 1` as a source on a
  parameter row and nothing of the kind on a strip, which rule 02 of
  [the seven](manual/index.html) asks for. [The console page](manual/console.html) carries a
  purposeful note until the readout is drawn. From M5.9.

- **Removing a lane has no control, no row and no operation.** `+ lane` adds one and nothing takes
  one away. From M5.9.

- **The `Param` lane arm reaches only the decks the Inspector holds a pane for.** The published rows
  come from `View::inspector`, which is pointed at two decks, so a target deck no pane is showing
  offers its fader and no parameters. From M5.9.

- **The window's minimum is a measurement, and a bay that grows a control invalidates it.**
  `MINIMUM_VIEWPORT` is the declared minima summed along each axis
  ([ADR-0272](adr/0272-the-window-has-a-minimum-and-only-one-of-adr-0250s-three-cases-is-real.md)),
  a test recomputes it from the tree, and what the test cannot check is whether a region's *declared*
  minimum is still a reading of its content. `centre`'s first number was a CSS track rather than a
  reading, and at it the parameter faders were not drawn
  ([ADR-0279](adr/0279-the-centre-is-two-parameter-rows-wide-because-a-pane-that-cannot-draw-a-fader-is-not-a-minimum.md)).
  Every bay still growing controls owes that reading again. From *The console's own shape* and M5.5.

- **A narrowing survives no replay and no keep.** `Operation::Publish` writes `Silent(NoRecord)` and
  nothing in the vocabulary says what a Set publishes, so a narrowing lives in the run that made it.
  [ADR-0329](adr/0329-an-input-is-wired-on-the-node-that-declares-it-and-the-number-is-the-publish-mark.md)
  refuses to invent the spelling. A gap in the record format, carried on both manual pages.
  From M5.5.

- **A renderer's declared input is unreachable from the panel.** The renderers of a deck are drawn as
  one group under a bare `L4` head, and a renderer may declare an input —
  `examples/second_eye.kir` declares `uses view : Camera`. A `uses` line on that head would be one
  node's declaration drawn on a head standing for several, so the folded head takes no line and the
  input is reached from the file and from a model (ADR-0329, alternative **e**). From M5.5.

- **The `uses` capsule's candidate list is inferred from the layer the input already reaches**, so a
  kind the deck reaches by no edge offers nothing, which is the side of
  [P-0090](principles/0090-a-surface-offers-it-never-decides.md) to be wrong on. ADR-0329 refuses an
  engine reader for a node's declared inputs as a second answer to a question `Set::validate` already
  settles, so this needs a way to widen the offer without keeping a second answer in step. From M5.5.

- **The aim carries one capacity per slot, so a per-geometry capacity is unaskable from any
  surface.** `Property::Capacity` lost its node rather than `Aim::capacity` gaining an entry per
  geometry, because no route filled the address
  ([ADR-0328](adr/0328-the-inspectors-deck-head-steps-a-slots-capacity-and-re-salts-it.md)). The
  revival condition is in the record: a pairing Set whose two geometries want two capacities.
  From M5.5.

- **Reaching a MIDI map while running.** A map is a file saved and recalled per controller; the file
  exists ([ADR-0196](adr/0196-a-map-line-names-a-state-and-an-old-line-is-refused.md)) and no row
  says it can be loaded while the instrument runs. The `map` pill is a readout and carries no chevron
  for that reason. It needs the menu — *save · load · new* — and a `Surface::reload` beside
  `Surface::first`; the arrangement pill is that menu over a different file and is built
  ([ADR-0221](adr/0221-an-arrangement-is-named-by-the-operator-and-kept-in-a-fourth-place.md)).
  From M5.12.

- **14-bit learn is owed.** Inside one drain two faders 32 apart are indistinguishable from one pair,
  so `learn` writes a `cc` line and an operator changes the one word. From M5.12.

- **A `params` press on a procedure row asks nothing.** The chip is on every row of the Library bay
  and the reading a `ReadSet` opens is a Set's, so a procedure row has the chip and no answer behind
  it. What a procedure row's card would be is specified nowhere;
  [the console page](manual/console.html) specifies the reading's shape for a Set alone. From M5.15.

- **A preset Set row carries no badge, and the mock draws one.** `Presets::list_sets` opens no file,
  so the shipped tier's Set rows have no layers to say, while its procedure rows are badged.
  [P-0091](principles/0091-cost-is-known-before-it-is-paid.md) prices the reading rather than
  refusing it — bounded by the directory and off the frame path — so what is owed is either the badge
  or a sentence on the page. From M5.15.

- **Listing procedures over MCP is nobody's.** *Filter the library by kind* is `gap` because a bay's
  narrowing is a window and a model has no window
  ([ADR-0315](adr/0315-a-model-has-no-window-so-the-twelve-surface-rows-mcp-badges-are-gap.md)), so
  *what procedures does this store hold* has no route. It is *List what the store holds*' to grow,
  and what a listing hands back for a procedure row is undecided. From M5.15.

- **A procedure load lands on node 0 of its kind.** A procedure declares one `kind` and nothing about
  where it goes, and a library row cannot say an index, so a `kind L4` always replaces `L4:0`. The
  revival condition is in
  [ADR-0338](adr/0338-a-procedure-is-a-row-of-the-library-and-one-loaded-over-a-layer-makes-a-set-with-no-name.md):
  the day the Inspector's node head grows a *replace this node* control is the day the payload gains
  a `NodeAt`. From M5.15.

- **A keep of a derived deck names a Set and re-points nothing.** Only `played` and `overlaid` write
  the strip's readout, so it goes on reading `<base> + <kir>` after the keep and `Aim::set` goes on
  being the base id. Whether a keep adopts the name it just wrote is undecided (ADR-0338).
  From M5.15.

- **The walk's row does not carry the page's `read` mark.** `Operation::WalkHistory` is
  `Silent(Question)`, and
  [ADR-0281](adr/0281-every-route-reaches-every-write-and-a-read-is-the-routes-own-interface-design.md)
  marks a read row so an empty column reads as design rather than debt;
  [ADR-0342](adr/0342-a-walk-names-the-set-it-is-of-and-the-two-rows-beside-it-are-gap.md)'s
  alternative (e) left the mark open. Whether the mark tracks the arm is a decision about the manual,
  and nothing on the page is false today. From M5.10.

- **A press on a kind chip narrows what the console draws and does not re-read the listing the
  narrowing is applied to.** The window's re-listing branch matches the scope press, the walk, a
  listing and a favourite, and not `Operation::FilterLibrary`, so ADR-0338's six kind chips mark a
  narrowing over rows that were read before it. ADR-0342 fixed the same defect on the `history` chip
  and recorded this one. From M5.10.

- **The hover layer does not know that a card is down.** `claim`'s rule 2 gives every press to
  whichever card is open, and the hover layer is handed `claim`'s answer rather than re-deriving its
  condition, so a pointer over a control an open card covers still draws that control's tip. What
  closes it is a seam in `input` answering *is a card down* for the press path and the hover path at
  once
  ([ADR-0330](adr/0330-the-console-paints-its-own-hover-layer-and-the-tips-are-the-manuals-own-words.md)).
  From M5.11.

- **Eight letters are free and nothing binds them.** `y`, `x`, `c`, `t`, `-`, `=`, the backtick and
  `a` are bound to nothing in `crates/karakuri` and no badge on
  [every operation](manual/operations.html) reserves one. The list is what a global key could still
  bind. From M5.13.

- **`z` is a bulk act whose recorded reason is gone.** *Bring back what is folded* unfolds everything
  rather than one region because a region a pointer cannot reach is one a key cannot name either, and
  focus can name a folded bay since ADR-0332. Whether the key still earns its place as a bulk act is
  unanswered. From M5.13.

- **`map.rs` has not moved to `karakuri-map`.** A global key is a complete line exactly as a `cc` is,
  so it is a third `Key` beside `Cc` and `Note`, and the crate holding the layer between a surface
  and the vocabulary
  ([ADR-0236](adr/0236-a-map-is-the-layer-between-a-surface-and-the-vocabulary-and-the-audit-is-one-of-the-things-it-does.md))
  then stops being named for a wire. `ls crates/` lists no `karakuri-map` and the file is
  `crates/karakuri-midi/src/map.rs`. From M5.13.

- **The page is not held to which pair of arrows a badge spells.** `SPELLED` in
  `crates/karakuri/src/keymap.rs` resolves `&uarr;&darr;` and `&larr;&rarr;` both to *the arrows*, so
  a badge naming the wrong pair passes. The axis is `karakuri_console::focus::Built::across`, and
  `karakuri-console`'s `grammar.rs` holds the console against it where nothing holds the page.
  From M5.13.

- **`focus::Addressed` carries five variants that differ only in the path they hang from.** `OfHead`,
  `UnderHead`, `InCard`, `Of` and `Under` each name a control at a different depth, and `walk_card`
  and `walk_chooser` in `crates/karakuri-console/src/focus.rs` are one walk written twice over two of
  them.
  [ADR-0350](adr/0350-the-transports-two-cards-are-walked-and-the-tempo-figure-steps-by-a-beat-a-minute.md)
  recorded the fold as the reviewer's on 2026-09-13 and it has not been taken. From M5.13.

- **The *console's own shape* stub in [history/m5.md](history/m5.md) says `esc` still quits.** It
  stopped quitting under ADR-0332 and *Quit*'s key badge reads `&mdash;`. It is a documentation edit.
  From M5.13.

- **The two Mixer rows' MIDI badges stay `gap`.** *Fade a deck out or in* and *Crossfade to the next
  deck* read nothing in the MIDI column: a map line names a slot number, a range, a word from a
  closed list or a position in a published interface, and a scheduled move names an instant, a length
  and a curve. They move the day a map line names a helper that carries time, which is ADR-0236's
  third job and does not exist. From M5.13.

- **Adding or removing a node has no operation and no row.** `karakuri-layout`'s arena has no insert
  and no remove, and `NodeId` is a bare index, so a removal shifts every id anything is holding. The
  node editor needs it. From M5.

- **The mock's six body controls are not operations and have no rows.** `2 up` names a region of the
  arrangement; the other five add something a bay draws inside its own body. From M5.

- **Nothing in the test suite reaches the startup path.** On 2026-09-08 `cargo test --workspace` was
  green while `cargo run -p karakuri` aborted on its first frame, because nothing in the suite
  reaches `resumed`. What closes it is a test that does. From M5.

- **A `#[cfg(test)]` above a source-scanning test silences it.** Each such test bounds its scan at
  the first `#[cfg(test)]` it finds, so one placed above the test modules in
  `crates/karakuri/src/main.rs` silenced five of them. Nothing is written down that closes this.
  From M5.

- **P-0094 owes a delete-and-re-record pass and it is the maintainer's.** Its *Undo* example is
  `swap.rs`, which no longer rolls anything back
  ([ADR-0316](adr/0316-an-over-budget-candidate-stays-in-the-slot-and-the-slot-stops-updating.md));
  [ADR-0313](adr/0313-a-candidate-is-judged-on-its-own-cost-and-the-decks-period-is-a-deck-level-alarm.md)
  left a pass owed on it too. From M5.

---

## The decisions nobody has taken

Each is open. Nothing below is settled by a record; everything that was has been deleted from this
list and lives in [adr/](adr/).

- **Whether loading a shipped preset should write into the operator's own library.**
  [ADR-0229](adr/0229-a-set-file-is-authored-beside-its-parts-and-travels-as-a-bundle.md) makes a
  load a packaging step that stores the bundle, so opening a shipped Set adds an entry to
  `<store>/sets/` — a consequence of two decisions rather than a decision anybody took. It is
  copy-on-load exactly as `scratch.rs` performs it for material, and what keeps `examples/` unwritten
  is [P-0096](principles/0096-the-operators-library-is-written-by-an-operators-own-act.md). The first
  symptom if it is wrong is a `my sets` filling with things the operator did not make.

- **What `expand` under a solo should do.** `Layout::soloed()` can lie: the solve never reads it and
  `check_structure` only checks that it addresses a node, so a solo's exclusivity lives in collapsed
  flags any later `expand` may contradict, and the next `unsolo` restores its snapshot and throws the
  expand away with nothing said.
  [ADR-0161](adr/0161-solo-remembers-which-region-because-it-cannot-be-derived.md) stores the soloed
  node and settles nothing about `expand`. Nothing reaches that state today.

- **What an agent is one of.** M6's first item asks for one per deck slot; the older statement of the
  control plane called them node agents. The authority is no longer part of the question: it is set
  per node ([ADR-0211](adr/0211-authority-is-set-per-node-and-the-record-is-the-sessions.md)), and an
  agent scoped to a deck slot can respect a per-node authority. M6's.

- **Whether an addressed write by an agent onto a node the operator kept is refused.** It cannot be
  decided until something writes a parameter on an agent's behalf.
  [P-0094](principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
  is the rule it is a case of. M6's.

- **The fate of `karakuri-cli`.** The recommendation on record is to shrink it as features move to
  the panel rather than delete it;
  [ADR-0242](adr/0242-the-command-line-is-test-tooling-and-the-instruments-principles-do-not-bind-it.md)
  scopes deleting it out of that record.

---

## The instrumentation

These commands are the only source of the numbers in this project. They are not to be summarised or
transcribed into prose.

The meter is the panel column of [every operation](manual/operations.html), where a `has` badge means
an operator running the instrument reaches that operation
([ADR-0213](adr/0213-the-interface-milestones-meter-is-the-panel-column-and-has-means-an-operator-reaches-it.md)),
and the key column is the instrument's keyboard rather than the command line's
([ADR-0220](adr/0220-the-key-column-is-the-instruments-keyboard-and-the-clis-keys-are-its-own.md)).

```sh
# the meter — how far each surface has got
for col in panel key MIDI MCP CLI; do
  echo -n "$col: "
  grep -o "class=\"rt [a-z]*\">$col " docs/manual/operations.html |
    sed 's/.*rt //;s/">.*//' | sort | uniq -c | tr '\n' ' '; echo
done

# the board — what is left on the panel, grouped by where the control lives
grep -o 'rt plan">panel <b>[^<]*' docs/manual/operations.html | sed 's/.*<b>//' |
  sort | uniq -c | sort -rn

# the vocabulary — how many operations there are
grep -c '<h3' docs/manual/operations.html

# drawing or machine — the operations nothing in this workspace constructs
sed -n '/^operations! {/,/^}$/p' crates/karakuri-operation/src/lib.rs |
  grep -oE '^    [A-Z][A-Za-z]+( \{[^}]*\})? => "[^"]+"' |
  sed -E 's/^ *//; s/ *(\{[^}]*\})? *=> "/|/; s/"$//' |
  while IFS='|' read -r v t; do
    n=$(grep -rn --include='*.rs' "Operation::$v" crates/ |
        grep -vE 'crates/karakuri-operation(-record)?/' |
        sed -E 's#^[^:]*:[0-9]+:##; s#//.*##' |
        grep -c "Operation::$v")
    [ "$n" -eq 0 ] && echo "$t"
  done
```

The fourth answers a question the badges cannot: a `plan` badge says a surface is meant to reach an
operation and does not yet, and an operation nothing constructs is one nobody can route to at all.
Two cuts make it honest, both of them `crates/karakuri-console/tests/panel_column.rs`' — the record
crate is excluded because it would answer *constructed* for every operation, and each line is
truncated at its first `//`, because a doc comment naming an operation is not a call site. Its
pattern reads only the macro arms whose payload fits one line.

The panel measures itself rather than being measured here. `crates/karakuri` installs a counting
`#[global_allocator]` and holds each run against its own `WRITTEN_ALLOCS`, saying so when the two
part company
([ADR-0217](adr/0217-the-counting-allocator-ships-because-a-written-number-nothing-checks-goes-stale.md));
`karakuri-console`'s `tests/schedulable.rs` holds both of ADR-0164's schedulability conditions and
`tests/parked.rs` holds the still panel's zero.

---

## Continuous concerns

Not milestones. These degrade silently if not defended at every step, and the rules in force are in
[principles/](principles/) rather than here.

### Live safety

[P-0094](principles/0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md)
is the whole of it: what a mechanism does at its worst, the three admissible answers, and what each
rules out. The show continuing when the generation API is unavailable is meaningful only once there
is a pool to fall back on, so it is a promise from M4 onward rather than from M1.

### Determinism

[P-0092](principles/0092-the-same-inputs-produce-the-same-frame.md) is the whole of it. Undo, replay,
A/B comparison and session recording all follow from it, and anything introducing run-to-run variance
breaks all four at once.

### Performance discipline

Per-element cost is the artifact's intrinsic property; total cost is a function of capacity, and
metadata records the former. Any change touching the frame path comes with a measurement;
[contributing.md](contributing.md) §1 holds the benchmarking standards — the reference workload, the
timestamp caveat and the conditions a figure must carry —
[P-0091](principles/0091-cost-is-known-before-it-is-paid.md) holds when a cost must be known, and
[P-0095](principles/0095-an-instrument-that-cannot-measure-says-so-rather-than-reporting-a-number.md)
holds what an instrument may report.

The machine this is developed on is not the baseline, and 4 GiB of dedicated VRAM is the reference
for *does this fit* rather than a cap:
[ADR-0110](adr/0110-this-machine-is-not-the-reference.md) holds both. A rich machine is allowed to
spend what it has — more capacity, more slots resident, longer chains — and what is not allowed is
waste a bigger machine merely hides; the test between them is whether spending it buys anything.
Development is a desktop and a performance is probably a laptop, and the second is the machine that
matters.

Round the estimate toward refusing. A show is cheaper to protect before it starts than to rescue
during it.

Two slots live and two preparing is the standard way of working, and it is the premise the risk
badge's bands are read against.

---

## Deferred by decision

Designed, understood, deliberately not scheduled.

**Fusion — the graph compiler's other half.** Folding a chain of stateless stages into one shader at
codegen. An L2's statelessness is structural, since every generated `deform` overwrites its output
from its input before the block runs, so nothing in the language has to change. It is deferred
because it is a trade rather than a win — it pays the stage's cost once per reader, so a heavy
deformation read by three renderers costs three times, where materialising pays once — and because
nothing is near a budget: a stage is one compute pass and one read-write of the element buffer, and
the longest chain anything ships is two stages. What revives it is a probe measurement putting a Set
over budget where the cost is in a chain of stateless stages with one reader; `probe.rs` measures at
real capacity and the governor acts on the number, so the instrument to notice it exists.

**External shader source compatibility.** Shadertoy-style material is a single fullscreen fragment
shader with no geometry stage and no parameters. Two import modes are designed: wholesale, as an
opaque L4 fullscreen node, and extraction, where an LLM pulls out the SDF or noise function and
discards the raymarch loop so it becomes a composable `Field`. The value in both is parameter
lifting — hardcoded constants become declared `param`s. It is deferred rather than dropped because
`VideoSource` makes it cheap later: a foreign source only needs to produce a colour texture.
Imported material is often extremely expensive, which makes the budget governor mandatory rather than
advisory.

**A keyboard is mapped and learned the same way a control surface is.** The vocabulary is what made a
MIDI map possible and a key is another way to name the same operation; the grammar is written —
`examples/surface.map` says `<target> <deck> <value>` and a key is a third `Key` beside `Cc` and
`Note` — so the map file's parser is nearly all of the work, and moving it out of `karakuri-midi` is
the *Mx — TODO* entry above. Learning belongs on the control: a GUI component is a translator between
one operation and N ways in, so *learn this control* is one gesture from one place. The MIDI half
landed in the hover layer
([ADR-0330](adr/0330-the-console-paints-its-own-hover-layer-and-the-tips-are-the-manuals-own-words.md),
[ADR-0336](adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md)); the key's own
teaching path does not exist, which is rule 01's keyboard surface with no teaching path
([ADR-0205](adr/0205-a-question-whose-reply-the-vocabulary-cannot-say-gets-no-row.md)). A keyboard
map would be the first map file naming operations the record layer never sees — twelve of the CLI's
keys write no record at all, being the surface's own state
([ADR-0198](adr/0198-a-gesture-converts-in-the-parts-that-are-decided.md)) — so what a map file may
say is a larger question than *which keys*.

**Also deferred:** projection mapping and multi-output warping, DMX and Art-Net lighting sync,
collaborative multi-operator sessions, video file playback, and any browser or wasm target. The
engine is native for Syphon, NDI, low-latency audio, MIDI, and GPU timestamp queries.

---

## Settled decisions

[docs/adr/INDEX.md](adr/INDEX.md) is the index of every decision, with the alternative that lost in
each record. Every standing rule that was once listed here is one file in
[principles/](principles/), where `ls` is the index because each filename is the rule it states.

---

## Document map

| | |
|---|---|
| `README.md` | The front door: what this is, a quickstart, and where every other document is |
| [`docs/architecture.md`](architecture.md) | The code: crates, pipeline, threading, the layer model, the three clocks, and the vocabulary |
| [`docs/principles/`](principles/) | The rules in force, one file each — `ls` is the index |
| [`docs/adr/`](adr/) | Every decision, with the alternatives that lost — append-only |
| [`docs/history/`](history/) | Milestones that closed, kept whole. History, never the present tense |
| [`docs/ir-spec.md`](ir-spec.md) | The IR: grammar, semantics, lowering, record formats |
| [`docs/manual.md`](manual.md) | How to play it: every flag, every key, and what each does |
| [`docs/manual/`](manual/) | The console's manual, published |
| [`docs/plugins.md`](plugins.md) | The out-of-process boundary, and why those two things are outside it |
| This file | The open milestones, what each owes, and the holding list under *Mx — TODO* |

None of the others restates this one, and this one does not restate them. What is built, part by
part, is not here: the code's own documentation and the two manuals are the primary sources.

---

## Reading order for implementation

1. [`docs/principles/`](principles/) — the rules in force
2. [`docs/architecture.md`](architecture.md) — the crates, the pipeline, and what the words mean
3. [`docs/ir-spec.md`](ir-spec.md) — the whole thing before writing any parser code
4. This file — *Mx — TODO* first if you are picking work up
5. [`docs/manual/`](manual/) — what the console is checked against
