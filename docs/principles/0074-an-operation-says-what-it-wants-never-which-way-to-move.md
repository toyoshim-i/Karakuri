# An operation says what it wants, never which way to move

Every operation in the vocabulary names a **destination**: `SetOnAir { deck, on_air: bool }`,
`SetBlendMode { deck, blend }`, `Fold { region }` and `Unfold { region }`. None of them says
*toggle*, *cycle* or *step*.

**A toggle is an affordance, built over operations by whoever draws the control**, and it belongs
there. A pad that flips on-air is one control emitting two operations; a mini that cycles the blend
is one control emitting three. The operator sees a toggle; the vocabulary never does.

**What it buys is the first rule of the manual** — one vocabulary, four ways in. A MIDI map with a
button per direction, a model over MCP that says which state it wants, and a keyboard that has room
for only one key all have to be able to say *put deck B on air* and mean it. A vocabulary of
`Toggle` cannot: a surface that can only step has no way to arrive, and two surfaces stepping the
same control disagree about where they are.

**The rule has a cost and it is the crate's shape.** A vocabulary that names a destination must
name the *value* — so `karakuri-operation` owns the enumerations a destination is drawn from
(blend modes, sync modes, tone map operators, curves, wipe shapes) rather than passing strings.
Refusing to name them is what forces a vocabulary back into toggles, and `karakuri-midi`'s `Action`
is the worked example: it declared itself engine-neutral, could therefore not say *set blend to
over*, and was left with `CycleBlend`. **The two rules — be engine-neutral, and have no toggles —
are not jointly satisfiable unless the vocabulary owns the lists.**

See [ADR-0180](../adr/0180-the-operation-vocabulary-is-a-crate-with-no-dependencies.md).
