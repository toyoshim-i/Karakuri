# An operator can see a slot's own material without putting it on air

Choosing between candidates is the basic workflow of this instrument, and it cannot be done blind.
So the instrument owes the operator a look at what a slot is making **before** that slot reaches the
audience. This is a prerequisite rather than a convenience: without it the only way to find out what
a slot holds is to show it to the room.

**This rule binds the instrument.** That is `crates/karakuri`, the program a player runs. It does
not bind `karakuri-cli`, which is test tooling rather than something anybody plays, and a surface of
the command line that cannot audition a slot is not a gap in this rule —
[ADR-0242](../adr/0242-the-command-line-is-test-tooling-and-the-instruments-principles-do-not-bind-it.md).

## The order the instrument is played in

The operator looks at the slot's cell, which is running its material on its own. They decide from
that. Then they raise the fader, and the material goes to the main output. Those are three steps and
they happen in that order, which is what the cells are for: the look is what the decision is made
on, and the fader is what carries the decision out.

Nothing about the main output is part of the look. **Once material is on main it is the broadcast of
the final result**, not a preview of it — the picture in the Program bay is an output and the four
cells are monitors
([ADR-0243](../adr/0243-the-program-picture-is-an-output-and-the-four-cells-are-monitors.md)).

## The clauses

Each is what the rule is *for* rather than a limit on how it is met.

**Its own material, with no fader in it.** What the operator sees is that slot's texels, moving,
through the transfer curve the audience will see them through. Not the mix it would join, and not a
still, a thumbnail or a scaled-down copy — what is being judged is the colour and the motion the
material arrives at, and none of those three carries it. The channel fader is not in it either: the
fader is the step *after* the decision, and a monitor that had it applied would be showing the
decision back rather than the material it was made on — a slot at opacity zero would be a black cell
for the whole of the moment it is being judged in. Nor is the gain trim, the blend or the mask. **A
slot's own target is fader-free by construction**: `gain`, `opacity`, `blend` and `mask` are applied
in the composite and nowhere else (`crates/karakuri-engine/src/deck.rs`), so what a slot renders into
its target is the input to setting a fader rather than the output of having set one. A surface that
samples `Deck::slot_view` gets that; a surface that samples the mix does not.

**Whatever its residency.** An off-air slot is exactly the one worth looking at: it is the candidate,
and residency is the thing the operator has not decided yet. An Allocated slot shows the still it
stopped at, a Priming one shows what it is warming into. Offering only the slots that are easy to
offer — the ones already drawing, because they are already drawing — removes the rule from the one
moment it is needed.

**Without changing it.** Looking adds a draw and never a step. A slot that is watched and then put
on air resumes where it stopped, or the audition would have moved the thing it was opened to judge —
[P-0082](0082-looking-never-writes-back.md).

**Never nothing.** A slot with something loaded in it is running, and it is on its cell from the
moment it is loaded. If the build was rejected — over budget, or it failed to compile — the cell
shows black, or shows what went wrong; a dark cell that says why is a reading. What a cell must
never do is show nothing *because the slot is not on air*, which is an absence the operator cannot
tell apart from a slot with nothing in it. A slot nobody has loaded anything into is the one case
where a cell has nothing to draw, and it says so.

## This is the property, and the control it replaced was never an instance of it

The rule this replaces named a mechanism: *an operator can put any one slot on the output in place of
the mix*, with `Deck::set_preview` and the CLI's `v` key as where it held. **That operation was not a
way of meeting this requirement and should never have existed**, because the main output is the
broadcast rather than a look at a candidate. How it got there is legible: a design made for the
console — a picture and four cells — was bent to fit the command line, which has one window, and with
one window the only place to put a look is the output, so the look took the output.

[P-0087](0087-name-the-property-never-the-shape.md) is why the rule is written as a property now: the
requirement is that the operator can see a slot on its own, and *where that look is drawn* was never
the thing being required. The retirement is
[ADR-0240](../adr/0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md) and the
re-recording is
[ADR-0241](../adr/0241-auditioning-survives-the-control-that-was-retired-and-is-re-recorded-as-a-property.md).
Both are annotated: 0240 retired the control for the wrong reason — redundancy with the cells rather
than the operation being wrong from the start — and 0241 carried that framing forward.

## What it costs, and that the bill is paid rather than argued

An audition adds a draw that is not in the budget and falls inside the window `swap.rs`'s watchdog
judges candidates on, so watching a heavy slot can roll back an unrelated slot's build
([ADR-0072](../adr/0072-auditioning-adds-a-draw-and-never-a-step.md)). Every loaded slot is watched
here, so that is four draws rather than one, and it is unbudgeted risk on the live path added
knowingly. It is the case
[P-0094](0094-the-show-does-not-stop-it-does-not-go-quiet-and-it-does-not-leave-the-operators-hands.md) loses, and
it loses for the reason written there: a rule protecting a performance may not be used to remove what
the performance is played with. The answer to the cost is to pay it and write it down
([P-0085](0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)), not to remove the look or hide
it behind a flag. An operator may still turn a cell off by hand; nothing else may.

## Where it holds

The console's Program bay draws four deck preview cells beside or below the picture, one per deck
slot, each presented from `Deck::slot_view` for the slot it is lettered for — so no cell can show
another deck's material under the wrong letter, and no operator has to give up the mix to use one
([ADR-0239](../adr/0239-the-program-bay-preserves-preview-size-when-arranging-beside.md),
`crates/karakuri/src/main.rs`). The engine draws a slot into its own target and never resamples the
composite for it, which is what makes the first clause true rather than approximately true.

## Where it is not met

**Stated because a rule in force says where it is not yet true**
([P-0093](0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md)).

**The console's cells were gated on `Residency::Live`, and that was the failure.** The gate sat in
`Engine::aim` (`crates/karakuri/src/main.rs`) and the engine drew an Allocated or Priming slot into
its target only under an audition nothing could switch on, so *whatever its residency* — the clause
this rule turns on — was met by no surface at all. The reason given for the gate was that an off-air
slot has nothing new in its view, which was true only because the engine refused to draw one.

**The residency gate is gone.** `Deck` draws every slot into its own target on every frame and the
field that chose one is deleted (`crates/karakuri-engine/src/deck.rs`); `Engine::aim` asks whether
there is a slot behind the cell rather than whether it is Live. The console's cell no longer says
`off` either: `state_word` answers `material` or `no slot`, which is what the code distinguishes.
The prose in `crates/karakuri/src/main.rs` that described the old behaviour was corrected on
2026-09-02. **A surface that draws the right thing and says it draws the old thing has not finished
meeting this rule**
([P-0093](0093-a-statement-is-held-true-by-the-thing-it-describes-or-it-is-deleted.md)), and the rejected
build's cell — black, or what went wrong — is owed on top of it.
