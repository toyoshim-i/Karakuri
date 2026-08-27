# The operator wins, and an automatic writer yields to a hand

Moving a control by hand stops whatever automatic thing was moving it. A transition, a follower, a
sequencer — whatever writes a control on its own gives it up the instant an operator touches it, and
never the other way round.

## What it rules out

**A control that fights back.** The one place an operator reaches when something is wrong is the
same place the automatic thing is writing: a fader is what silences material that has gone NaN, and
a wipe's front is what stops a transition that is taking the wrong picture across the screen. A
control that had to be held against a writer that reasserts itself sixty times a second is not a
control at all — it is a display that moves when you push it. Every proposal to make an automatic
writer *win*, or to resolve the two by mixing them, or to make the hand's write take effect only
after the move finishes, is this.

**And it rules out the surface deciding.** A hand is a hand on all four routes: a pointer on a
fader, a key, a mapped control change and an MCP call are the same write and cancel the same move.
The rule lives where the write lands, not where the gesture started, which is
[P-0076](0076-a-surface-owns-the-affordance-never-the-authority.md) applied to this — a panel that
cancelled and a pad that did not would be one instrument behaving two ways.

## What it does not rule out

**A second automatic writer replacing the first.** Two moves scheduled on one control is an operator
changing their mind, and the later one wins outright; running both would put the control wherever
the two arithmetics happened to land in whichever order they ran. Replacement is not a fight.

**A write that touches no control the move is carrying.** This is the exemption that is really a
boundary, and the mask is where it shows: a transition on a mask carries its **position**, so
choosing a different *shape* mid-wipe changes what is being wiped and is not a hand on the control
that is moving. `karakuri_engine::deck::Deck::set_mask_shape` therefore does not cancel and
`set_mask_position` does — and the two are separate setters **because** they answer this
differently. Before the split, one setter took a whole `Mask` and did not cancel, with the reason
written at it; that was safe only while the sole caller wrote position 0 before scheduling a move,
and it stopped being safe the moment the vocabulary grew a row that writes a position by hand
([ADR-0201](../adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md)). **The
exemption is now a split**, which is the difference between a rule with a hole in it and a rule that
says what it is about.

## Where it holds

- **`Deck::set_gain`, `Deck::set_opacity` and `Deck::set_mask_position` cancel**; `Deck::schedule`
  cancels the move it replaces. That is every writer of every control
  `karakuri_engine::transition::Control` names, and it is enforced by a call rather than by a type —
  `Deck::cancel` is called by hand on each of them.
- **Not through the record stream, and this is the honest limit.** `Record::Mask` is a *state* — a
  shape, an angle, a position and a softness — because a replay reconstructs a picture from it and
  not an intention. Both mask operations write it whole, so nothing downstream can tell a shape
  change that restated the front from a hand that moved it, and the decode applies the whole state
  and cancels. Of the two available readings only one leaves the operator winning, so that is the
  one taken; the cost is that a shape change **routed through the stream** while a wipe is running
  stops the wipe. Nothing routes one today — the shape's only home is the console's mask mini and it
  is still a readout — so the first surface to make that press is where this is decided again, with
  the shape's own setter already in the engine waiting for it.
- **The transition is the only automatic writer that exists.** A follower, a sequencer lane and a
  signal binding are all named in the plan and none of them writes a mix control yet. This is
  written down now rather than when the second one arrives, because the second one is the change
  that would otherwise settle it by accident.

Decided at the deck, argued in
[ADR-0201](../adr/0201-the-mask-is-two-rows-because-a-control-change-can-only-set.md), and stated in
prose at `crates/karakuri-engine/src/transition.rs` under *The operator wins* since transitions
landed — which is why this file says the rule and the module says the mechanism. What is asserted is
`crates/karakuri-engine/tests/deck.rs`:
`moving_a_control_by_hand_cancels_the_transition_moving_it` for the faders, and
`a_hand_on_the_front_stops_the_wipe_and_a_hand_on_the_shape_does_not` for the two halves of a mask,
which fails in both directions on purpose.
