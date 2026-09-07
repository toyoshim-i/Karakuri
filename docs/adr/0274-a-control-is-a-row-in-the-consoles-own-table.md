---
id: 0274
title: A control is a row in the console's own table
status: accepted
date: 2026-09-07
supersedes: []
superseded_by: []
principles: []
tags: [console, input, tests, registry]
---

# A control is a row in the console's own table

## Context

Adding one control to the panel touched five files, and two of them are the files every queued piece
of work collides on. The core is one derivation in `karakuri-console/src/view.rs`; the *registration*
was spread over `input.rs`'s module documentation, `input.rs`'s `CLAIMS`, `input.rs`'s `claim`,
`karakuri/src/main.rs`'s press handler, and `karakuri/src/main.rs`'s `press_handler::TABLE`.

**Two of those were a hand-summed parallel array.** `CLAIMS: [usize; 15]` counted what each of rule
4's derivations answers for, and `claim` built `probes: [&dyn Fn() -> bool; CLAIMS.len()]` out of
fifteen closures. The array length tied the two together, so a derivation added without a count did
not compile — and nothing tied either of them to the control. `CLAIMS`' own documentation admitted
the hole: *"What it cannot see is a control added inside a derivation already here: a sixth chip on a
strip is one more thing a press reaches, and `on_strip` would go on answering for five."*

**That hole is not hypothetical.** `ba61353`, on 2026-09-07, restored `Mixer::select` — a press
anywhere on a strip that no knob claimed, which is how a deck is addressed with the pointer. It had
been unreachable for eight days: the control was drawn, the press handler had an arm for it, both
manual pages described it, and `on_strip` went on asking four questions, so `claim` handed every
press on a strip's ground to `egui`, which owns no widget anywhere on this console. Five places said
it worked.

**`karakuri/src/main.rs` had the same list a third way, and derived it from text.**
`press_handler`'s job is the seam — *is every control the console draws asked by the window that
draws it* — and it is a real one: between `74719c3` and `094804c` the transition row was drawn,
claimed, and had no branch in the handler at all, while `docs/manual/operations.html` marked the
operation `has`. Its header said why it read text rather than reading a list: *"There is no list in
`karakuri-console` of what its controls offer, and a list written here would be the second copy of
one."* So it ran a non-recursive `read_dir` over `crates/karakuri-console/src`, took every indented
`pub fn` inside an `impl` whose parameters named a `Point` and whose return type was not `bool`, and
required a four-column `TABLE` entry — type, method, derivation, local — for each, four columns
because method names are not unique and every receiver in the handler is a local called `bay`, `row`,
`pill` or `head`.

**The directory listing carried a hazard of its own.** It asserted `files.len() >= 7` against exactly
seven `.rs` files, and `karakuri-console/tests/panel_column.rs` ran the identical listing with the
identical floor. Splitting `view.rs` — 14,565 lines — into a directory would leave seven files there
and take every control and every emission in it out of both scans, silently, with both floors still
true.

## Decision

**`karakuri-console` owns a table of its controls, and it is the value `claim` walks.**

`input::PROBES` is `[Probe; 15]`, and a `Probe` is three fields: the row's `name`, the `claims` it
contributes, and `ask` — the probe itself, as a `fn(&Panel, &egui::Context, &View, Point) -> bool`.
The fifteen closures in rule 4 had wildly different derivations and answered nine different types to
the caller, but every one of them closed over exactly the four values `claim` itself takes, so the
probe half is one signature and needed no new plumbing. `CLAIMS` is gone and `CONTROLS` is summed
over `PROBES`.

**The row order is preserved and its reason is now on the table.** It is *one probe per derivation,
cheapest answer first*, with the class pills last because each of them lays out a whole bay head to
find one capsule. `claim` answers a `bool` and the walk short-circuits, so **no order over these rows
can change what it says**: it is a cost ordering, and the table says so, so the next reader may
reorder it freely.

**Three rows stopped carrying a number.** The class pills count `Class::ALL`, the preview cells
count `DECKS` and the scope chips already counted `Scope::ALL` — so a fifth of any of the three
raises `CONTROLS` on its own.

**`karakuri/src/main.rs` reads that table instead of that crate's text.** `press_handler::ASKED` is
`[(&str, &str, &[&str]); PROBES.len()]` — the row's name, the derivation the handler calls, and the
calls it makes on the receiver — one entry per row and in the console's own order. The array length
is the backward direction: **a control added to the console is a compile error here**, where it used
to be a scan finding a `pub fn`. The scan, `TABLE`'s four columns, `CONSOLE`, `console_files`,
`impl_type`, `offers`, `controls`, `console_text`, `NOT_A_CONTROL`, `EXEMPT` and the `>= 7` floor all
go with it.

**The forward direction stays, and it got wider.** *Does the press handler ask this row* is the check
that would have caught `74719c3`..`094804c`, and it is now keyed on rows: the derivation must appear,
and each call the entry names must appear after it, counted so that one `pill.ask(` cannot stand in
for both cards'. Because a row is a control and not a `pub fn` signature, **three controls the old
criterion could not see are now checked** — the Outputs sink, the Program bay head's `solo` and the
four class pills, all of which answer `hit` as a `bool` and were read as `input::claim`'s business
rather than the handler's.

**`panel_column.rs`'s half of the directory hazard is closed by making its scan recursive**, since
deleting `console_files` only closed the other half.

## Alternatives rejected

**Keep the scan and add the table.** Belt and braces: the table for the count, the scan for the
`pub fn`s. It loses because the scan is then a *second derivation of the same fact* — the thing §4's
*A statement is held true by the thing it describes* exists to prevent — and because the scan is the
weaker of the two by construction: it answers from text, cannot see a control whose method is already
tabled, needs `impl_type` to attribute a `pub fn` to a type, needs `EXEMPT` and `NOT_A_CONTROL` to
wave through what it wrongly catches, and stands on a directory listing whose floor cannot fail the
one way the directory can change. Keeping it would also have kept `press_handler` reading a second
crate's source, which is what made its `the_two_cuts` refusal have to cover eight files.

**One row per control — thirty-six rows rather than fifteen.** It is the reading that would turn
*a control added inside a derivation already here* into a compile error, which is the residual hole,
and it loses to the reason the fifteen exist: **the mixer bay is derived once and asked five times**,
because *five derivations of one laid-out strip would be five answers*, and a strip's layout is two
galley lookups. Thirty-six rows means thirty-six derivations per press unless a row carries a probe
that cannot be called on its own — at which point the row is not a value. The hole is real and is
recorded rather than closed: it is caught by the clearance test every control already owes, by
`tests/mixer.rs` asking `claim` at the points a strip has no chip on, and now by the count and the
probe being one line apart instead of two arrays apart.

**Unify the fifteen answers into an `Ask` enum in the same step.** The controls hand back nine
different types today, `Readout::pointer` and `apply` are shaped around that, and the row would then
carry order data as well as a probe. It is the next decision and wants its own record; this one
commits to none of it, which is why `Probe::ask` answers `bool` and nothing else.

**Move `input.rs`'s module documentation onto the rows.** ~530 lines of it, and it is the per-control
argument for every clearance — that the `read` chip clears its foot by 4.75 against a `GRAB` of 6, and
that this is rule 3's ordinary price. Rows are a registry; that documentation is an argument, and it
is what would be moved by building the registry rather than by taking this step. Where the
documentation stated a count the table now derives, it points at the table instead.

## Consequences

- **`CONTROLS` is 36 and is summed over `PROBES`.** Nothing else changed about what it means or where
  it is printed.
- **A probe function no row names is `dead_code`**, so the console crate itself refuses it under
  `-D warnings`. That is the console-side half of what the `[…; PROBES.len()]` length is on the
  binary's side.
- **`press_handler` no longer reads `karakuri-console` at all.** Its `the_two_cuts_are_the_whole_of_
  the_comment_syntax_they_meet` refusal now covers `crates/karakuri/src/main.rs` alone, which is also
  what `key_column` and `event_response` read, so all three still stand under it.
- **`EXEMPT` is gone and nothing replaces it.** Its three entries — `ProgramBay::cell`,
  `AudioInPill::item`, `ArrangementPill::item` — existed because the scan enumerated `pub fn`s and
  caught methods that are a control's internals rather than its offer. Nothing enumerates those now,
  so there is nothing to exempt: every row of `PROBES` is asked by at least one call. `ProgramBay::
  cell`'s own documentation still carries why a press on a cell asks for nothing, and
  [ADR-0240](0240-the-output-shows-the-mix-and-residency-keys-belong-to-the-mixer.md) and
  [ADR-0273](0273-the-carry-lands-on-two-sets-of-rectangles-and-wears-a-face.md) are where that
  decision is.
- **What the scan could see and the table cannot: a new offer on a control already rowed.** A sixth
  method on `Mixer` taking a `Point` used to fail `every_offer_a_console_control_makes_is_in_the_
  table` and now fails nothing until somebody raises the row's `claims` and this entry's list. What
  the table can see and the scan could not is a control that answers `bool` — three of which are now
  checked for the first time. Neither reading is the whole of it, and this one costs no directory
  listing.
- **Adding a control is now three places rather than five**: the derivation in `view.rs`, a row in
  `input::PROBES`, and an entry in `press_handler::ASKED` that the compiler demands. The module
  documentation's paragraph and the manual page are the argument and the specification, and they were
  never the registration.
