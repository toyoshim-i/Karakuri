//! **The console paints its own hover layer, and the words in it are the
//! manual's own.**
//!
//! # Who owns the pointer
//!
//! `docs/roadmap.md`'s M5.11 waited on one decision — *whether the panel gains
//! `egui` widgets, or paints its own hover layer* — and
//! [`crate::input`] named it as its own and deliberately did not take it.
//! [ADR-0330](../../../docs/adr/0330-the-console-paints-its-own-hover-layer-and-the-tips-are-the-manuals-own-words.md)
//! takes it: **the console paints its own layer**, because the mechanism is
//! already here and the alternative buys nothing this crate does not have.
//!
//! - [`crate::input::PROBES`] already answers *what is under the pointer*, one
//!   derivation per row, for every control the panel draws. A tooltip is that
//!   question asked on a rest instead of on a press
//!   ([P-0085](../../../docs/principles/0085-take-the-mechanism-that-exists-and-pay-the-bill-now.md)).
//! - [ADR-0156](../../../docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)'s
//!   seam is that **the console paints and takes no device**, and that `egui`
//!   receives a rectangle rather than the arrangement. An `egui` widget under
//!   every control would hand hit testing and layout to a second system for
//!   this one feature, and `claim`'s rule 4 — *a press routed to `egui` there
//!   reaches nothing at all* — would have to become two rules.
//!
//! # The words are the page's, and only the key is written down here
//!
//! **`docs/manual/console.html` is the only copy of a tip.** The page is
//! [`PAGE`], embedded at compile time, and [`Tips::read`] parses the
//! `data-tip` attributes out of it **once, at start-up**, off the frame path
//! (P-0091). Nothing here restates a word of one, so there is no second copy
//! to drift ([`docs/contributing.md` §4](../../../docs/contributing.md), which
//! is *generated* rather than *tested*).
//!
//! **What cannot be generated is the key**: which drawn control a tip belongs
//! to. The mock carries no identity for a control — no `id`, no
//! `data-control`, and its classes repeat (twenty tipped elements are a bare
//! `.pill`) — so *which element is the tone map's capsule* is not derivable
//! from the page by any rule that survives an edit to it. That much is
//! transcribed, and it is transcribed as a **citation and not as prose**: a
//! [`Cite`] is the element's class and the words inside it, which is the
//! smallest thing that names one element of the mock. [`TIPS`] is that
//! transcription and `tests/hover.rs` is what holds it to the page, in
//! `tests/transcribed_constants_cite_the_mock.rs`'s pattern — a cite that
//! resolves to no element, or to more than the one it says, fails there.
//!
//! # The table is the console's own rows
//!
//! [`TIPS`] is `[(&str, &[Tipped]); PROBES.len()]`, one entry per row of
//! [`crate::input::PROBES`] and in that crate's own order, which is
//! `karakuri/src/main.rs`'s `ASKED` shape one crate over and for its reason:
//! **a control added to that table arrives here as a compile error.** A row
//! whose controls the mock draws with no tip carries an empty slice, which is
//! the roadmap's own reading rule — *the mock is not exhaustive … there will
//! be gaps in the functions too* — said as a value rather than as a silence.
//!
//! **Where a row claims several controls the slice has one entry each**, asked
//! in the caller's order: the controls inside a container first and the
//! container last, which is `claim`'s rule 4 (*a control claims what it acts
//! on and no more*) and `main.rs`'s press order. [`resolve`] takes the first
//! that answers, so the order is what makes a tip on a strip's fader the
//! fader's rather than the strip's.
//!
//! # What it costs
//!
//! - **340 KB of page in the binary**, and one parse of it at start-up. The
//!   alternative was ~60 long strings transcribed by hand and held equal by a
//!   test, which is a second copy of the manual's prose kept in a source file:
//!   cheaper to run and dearer to keep true, and the page is the half that
//!   moves.
//! - **One walk of [`TIPS`] per pointer move that [`crate::input::claim`]
//!   answered `Panel` to**, which is the walk that rule 4 already makes,
//!   asked a second time to say *which*. A move `claim` gave to `egui` is on
//!   no control at all and costs one comparison.
//! - **Nothing on the frame after the first.** The galley is laid out on the
//!   frame the tip appears and kept while the pointer stays on that control;
//!   every frame after it is one cached galley and four shapes.
//! - **No frame at rest.** [`Hover::owed`] answers a deadline while a dwell is
//!   running and nothing at all once the tip is up — the tip does not move
//!   while it is shown, so its picture is not different from the one on
//!   screen. That is
//!   [ADR-0283](../../../docs/adr/0283-a-region-declares-when-its-picture-next-changes-not-that-something-is-pending.md)'s
//!   `moves_in` on a region whose motion is a hand holding still.
//!
//! # What it does not do
//!
//! **It does not know that a card is down.** `claim`'s rule 2 gives every
//! press to whichever card is open, and this layer is told `claim`'s answer
//! rather than re-deriving its condition — so a pointer resting over a control
//! that an open card happens to cover still resolves to that control. Copying
//! the condition here would be the second copy this module exists to avoid;
//! what closes it is a seam in [`crate::input`] that answers *is a card down*
//! for both, and it is not worth one until a tip is seen under a card.

use std::sync::Arc;
use std::time::Duration;

use egui::epaint::text::{LayoutJob, TextFormat};
use egui::{Color32, CornerRadius, FontFamily, FontId, Galley, Pos2, Rect, Stroke, StrokeKind, Ui};
use karakuri_layout::Point;
use karakuri_operation::gate::Class;
use karakuri_operation::{Layer, Operation, Output};

use crate::input::{Claim, PROBES};
use crate::panel::Panel;
use crate::room::size::HAIRLINE;
use crate::view::{
    arrangement, audio_in, deck_head, inspector, keep_pill, library, look, master, mcp_pill, mixer,
    outputs, program_bay, program_head, sequencer, staging, to_egui, tracker_group, transition,
    transport, Field, KindChip, Scope, View,
};

/// **The mock, embedded**: the only copy of every tip on this console.
///
/// It is `include_str!` rather than a path read at run time for the reason
/// every other transcription in this crate is a `const`: a panel that had to
/// find `docs/manual/` on disk would draw no tips at all when it was installed
/// anywhere else, and a tip that is missing is indistinguishable from a
/// control that has none.
pub const PAGE: &str = include_str!("../../../docs/manual/console.html");

/// **Where one control's words are in the mock**: the element's class, exactly
/// as the page spells it, and the text inside it with its tags removed and its
/// runs of whitespace collapsed.
///
/// **It is a citation and not a copy.** Nothing here is a word of the tip —
/// what is written down is where to find it, which is the part the page cannot
/// answer for itself.
///
/// **The text is in the page's own spelling, entities and all** — `&#9662;`
/// stays `&#9662;` — because what is being cited is the markup rather than
/// what a browser makes of it, and a cite a reader can `grep` for is one they
/// can check.
///
/// [`Cite::nth`] is which of the elements matching that pair is meant, and it
/// is 0 for every cite that is unambiguous. A pair that matches nothing, or
/// that matches fewer elements than `nth` reaches, fails in `tests/hover.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cite {
    /// The element's `class` attribute, verbatim, or `""` for an element with
    /// none.
    pub class: &'static str,
    /// The text inside the element, tags stripped and whitespace collapsed.
    pub text: &'static str,
    /// Which of the matching elements, in document order, counting from zero.
    pub nth: usize,
}

/// **One control that explains itself on hover**: its identity, where its
/// words are in the mock, and the derivation that says the pointer is on it.
pub struct Tipped {
    /// **The control's name**, in the words [`crate::input::PROBES`] uses for
    /// the row it belongs to. It is what a test and a reader address this row
    /// by, and it is never drawn.
    pub control: &'static str,
    /// Where this control's words are in the mock.
    pub cites: Cite,
    /// **The derivation that draws the control, asked whether the point is on
    /// it**, and nothing is stored — [`crate::input::Probe::ask`]'s signature
    /// and its rule, one question finer.
    pub at: fn(&Panel, &egui::Context, &View, Point) -> bool,
}

/// **Every tip this console can draw, one entry per row of
/// [`crate::input::PROBES`] and in that table's order.**
///
/// The array is `PROBES.len()` long, so a control registered there and never
/// given a tip here is a compile error rather than a control that quietly
/// explains nothing. `tests/hover.rs` holds each entry against the row it
/// answers for, by name and by position, so an entry cannot answer for its
/// neighbour.
///
/// **An empty slice is the reading rule and not an omission.** The maintainer,
/// 2026-08-31: *"the mock is not exhaustive … there will be gaps in the
/// functions too"*, and the roadmap carries it — a control the mock draws
/// without a tip gets none. Each empty slice says which control it is silent
/// about.
pub const TIPS: [(&str, &[Tipped]); PROBES.len()] = [
    // **The Outputs row's chips.** Two of the four answer: a plugin chip
    // switches nothing and `Outputs::chip_at` says so, which is the same
    // `false` `claim` gets.
    (
        "the Outputs row's sinks",
        &[
            Tipped {
                control: "the program view",
                cites: Cite {
                    class: "sink on",
                    text: "program view",
                    nth: 0,
                },
                at: on_program_sink,
            },
            Tipped {
                control: "the projector window",
                cites: Cite {
                    class: "sink on",
                    text: "projector",
                    nth: 0,
                },
                at: on_projector_sink,
            },
        ],
    ),
    (
        "the audio-in pill",
        &[Tipped {
            control: "the audio-in pill",
            cites: Cite {
                class: "pill armed",
                text: "audio&#8209;in &middot; Scarlett 2i2 &#9662;",
                nth: 0,
            },
            at: on_audio,
        }],
    ),
    // **The tracker group's three**, in the row's own order. The octave is one
    // entry and the mock draws two faces of it: at any tempo at most one of
    // them is live, and `TrackerGroup::octave` answers for whichever it is.
    (
        "the tracker group's three",
        &[
            Tipped {
                control: "the tap",
                cites: Cite {
                    class: "pill",
                    text: "tap",
                    nth: 0,
                },
                at: on_tap,
            },
            Tipped {
                control: "the octave",
                cites: Cite {
                    class: "",
                    text: "&frac12;",
                    nth: 0,
                },
                at: on_octave,
            },
            Tipped {
                control: "the offset track",
                cites: Cite {
                    class: "",
                    text: "offset &minus;15 ms",
                    nth: 0,
                },
                at: on_offset,
            },
        ],
    ),
    (
        "the transport row's learn pill",
        &[Tipped {
            control: "the learn pill",
            cites: Cite {
                class: "pill lav",
                text: "learn",
                nth: 0,
            },
            at: on_learn_pill,
        }],
    ),
    // **The mock's name for this map is a device and this console's is a
    // file**, which is the page being a drawing and this being a run: the cite
    // is the element, and the words the pill *draws* are `map · <whatever is
    // loaded>`.
    (
        "the transport row's map pill",
        &[Tipped {
            control: "the map pill",
            cites: Cite {
                class: "pill",
                text: "map &middot; nanoKONTROL2 &#9662;",
                nth: 0,
            },
            at: on_map_pill,
        }],
    ),
    (
        "the arrangement pill",
        &[Tipped {
            control: "the arrangement pill",
            cites: Cite {
                class: "pill",
                text: "arr &middot; night &#9662;",
                nth: 0,
            },
            at: on_pill,
        }],
    ),
    (
        "the look group's two",
        &[
            Tipped {
                control: "the tone map",
                cites: Cite {
                    class: "pill",
                    text: "tone &middot; aces",
                    nth: 0,
                },
                at: on_tonemap,
            },
            Tipped {
                control: "the exposure track",
                cites: Cite {
                    class: "",
                    text: "exp 1.00",
                    nth: 0,
                },
                at: on_exposure,
            },
        ],
    ),
    (
        "the transport row's rec pill",
        &[Tipped {
            control: "the rec pill",
            cites: Cite {
                class: "pill on",
                text: "&#9679; rec",
                nth: 0,
            },
            at: on_rec,
        }],
    ),
    (
        "the transport row's tempo figure",
        &[Tipped {
            control: "the tempo figure",
            cites: Cite {
                class: "bpm",
                text: "128.0",
                nth: 0,
            },
            at: on_tempo,
        }],
    ),
    // **A strip's five, the four inside the column first and the column
    // last**, which is `claim`'s order and `main.rs`'s: a rest on a knob is
    // the knob's, and what is left over is the strip's.
    (
        "a mixer strip's five",
        &[
            Tipped {
                control: "a strip's fader",
                cites: Cite {
                    class: "vfader",
                    text: "",
                    nth: 0,
                },
                at: on_fader,
            },
            Tipped {
                control: "a strip's blend chip",
                cites: Cite {
                    class: "mini sel",
                    text: "add",
                    nth: 0,
                },
                at: on_blend,
            },
            Tipped {
                control: "a strip's tally chip",
                cites: Cite {
                    class: "tally live",
                    text: "live",
                    nth: 0,
                },
                at: on_tally,
            },
            Tipped {
                control: "a strip's mask mini",
                cites: Cite {
                    class: "mini",
                    text: "&#9711;",
                    nth: 0,
                },
                at: on_mask,
            },
            Tipped {
                control: "the strip itself",
                cites: Cite {
                    class: "strip live focus drop",
                    text: "drift_night live g 1.00 add &#9711;",
                    nth: 0,
                },
                at: on_select,
            },
        ],
    ),
    (
        "the transition row's four",
        &[
            Tipped {
                control: "the wipe's shape",
                cites: Cite {
                    class: "pill armed",
                    text: "iris",
                    nth: 0,
                },
                at: on_shape,
            },
            Tipped {
                control: "the wipe's quantum",
                cites: Cite {
                    class: "pill",
                    text: "next bar",
                    nth: 0,
                },
                at: on_quantum,
            },
            Tipped {
                control: "the wipe's length",
                cites: Cite {
                    class: "pill",
                    text: "8 beats",
                    nth: 0,
                },
                at: on_length,
            },
            Tipped {
                control: "the go capsule",
                cites: Cite {
                    class: "pill on",
                    text: "go",
                    nth: 0,
                },
                at: on_go,
            },
        ],
    ),
    (
        "the Master bay's five",
        &[
            Tipped {
                control: "the Master bay's out",
                cites: Cite {
                    class: "fader",
                    text: "",
                    nth: 0,
                },
                at: on_master_out,
            },
            Tipped {
                control: "an effect row's chip",
                cites: Cite {
                    class: "mini sel",
                    text: "mix",
                    nth: 0,
                },
                at: on_master_chip,
            },
        ],
    ),
    // **The name in a pane head has no tip in the mock**, which is the reading
    // rule: the page draws the head's count and its `keep` capsule with tips
    // and says nothing about the name beside them.
    ("the Inspector pane heads' name", &[]),
    (
        "the Inspector pane heads' keep",
        &[Tipped {
            control: "the keep capsule",
            cites: Cite {
                class: "pill on",
                text: "keep",
                nth: 0,
            },
            at: on_keep,
        }],
    ),
    // **The mark between the run and the count**, whose words the mock has
    // carried since the chooser was drawn: it cites the *first* pane's `▾`,
    // and the second pane's own tip says the same thing about the head next
    // door — one entry, because one probe answers for both heads.
    (
        "the Inspector pane heads' deck pulldown",
        &[Tipped {
            control: "the pane head's deck pulldown",
            cites: Cite {
                class: "",
                text: "&#9662;",
                nth: 0,
            },
            at: on_pane_target,
        }],
    ),
    // **Six entries for the row's seven controls**, in the order the press
    // handler asks them: the scrub is one entry and the mock draws two arrows
    // — `.scrub` carries the tip and the arrows inside it are one control to a
    // hand, which is `DeckHead::scrub`'s own answer.
    //
    // **The capacity chip and `re-salt` are the head's second row**
    // (ADR-0328), and the two cites are the *first* pane's: the mock draws
    // each of them twice and gives the first the whole of what the control is
    // — a number somebody asked for and what a step does to it, a salt and
    // what moves with it — where the second pane's pair says what those two
    // are on `lattice_shell`. This is the node head keep capsule's rule, for
    // its reason: the long one is the one an operator meeting the control
    // needs.
    (
        "a deck head's seven",
        &[
            Tipped {
                control: "the sync chip",
                cites: Cite {
                    class: "mini sel",
                    text: "tempo",
                    nth: 0,
                },
                at: on_sync,
            },
            Tipped {
                control: "the anchor",
                cites: Cite {
                    class: "anchor",
                    text: "T128",
                    nth: 0,
                },
                at: on_anchor,
            },
            Tipped {
                control: "the scrub arrows",
                cites: Cite {
                    class: "scrub idle",
                    text: "&#9666;&#9656;",
                    nth: 0,
                },
                at: on_scrub,
            },
            Tipped {
                control: "the capacity chip",
                cites: Cite {
                    class: "mini sel",
                    text: "524288",
                    nth: 0,
                },
                at: on_size,
            },
            Tipped {
                control: "the re-salt capsule",
                cites: Cite {
                    class: "mini",
                    text: "re-salt",
                    nth: 0,
                },
                at: on_salt,
            },
            Tipped {
                control: "the composite chip",
                cites: Cite {
                    class: "mini sel",
                    text: "composite",
                    nth: 0,
                },
                at: on_compositing,
            },
        ],
    ),
    (
        "the renderer chips",
        &[Tipped {
            control: "a renderer chip",
            cites: Cite {
                class: "rend-row",
                text: "soft_points strand_strokes spark_fountain",
                nth: 0,
            },
            at: on_rend,
        }],
    ),
    (
        "a parameter row's fader",
        &[Tipped {
            control: "a parameter row",
            cites: Cite {
                class: "pname",
                text: "exposure",
                nth: 0,
            },
            at: on_param,
        }],
    ),
    // **The mark that publishes a row**, which is the row's leftmost cell —
    // the mock tips it in both of its states, and the two rows below are those
    // two: a number on a control the interface carries, and the dot on one it
    // does not (`docs/adr/0329-…`).
    (
        "a parameter row's publish mark",
        &[
            Tipped {
                control: "a published control's number",
                cites: Cite {
                    class: "ord",
                    text: "3",
                    nth: 0,
                },
                at: on_publish,
            },
            Tipped {
                control: "an unpublished control's mark",
                cites: Cite {
                    class: "ord",
                    // The mock writes the dot as an entity, as it writes every
                    // other piece of punctuation on the page — so the text this
                    // cites is the entity and not the character it decodes to.
                    text: "&middot;",
                    nth: 0,
                },
                at: on_publish,
            },
        ],
    ),
    // **A node's declared input and the card that rewires it.** One tip, on
    // the line: the capsule and its card are one control to a hand and the
    // mock tips the line rather than the capsule inside it.
    (
        "a node group's `uses` capsule and its card",
        &[Tipped {
            control: "a node's declared input",
            cites: Cite {
                // **The capsule and not the line**, because the capsule is the
                // control: the words belong where a hand goes, and the line
                // around it is the slot's own name and a separator.
                class: "pill",
                text: "sphere_shell &#9662;",
                nth: 0,
            },
            at: on_uses,
        }],
    ),
    (
        "a node head's three authority chips",
        &[Tipped {
            control: "a node head's authority chips",
            cites: Cite {
                class: "auth",
                text: "mansugauto",
                nth: 0,
            },
            at: on_auth,
        }],
    ),
    // **The capsule at the right of the same head.** The mock writes the whole
    // of it once, on `drift_shell`'s — what it keeps, where it goes, what it
    // is filed as, where a model's lands, and the two heads that carry none —
    // and gives the other two capsules a sentence apiece pointing back at it.
    // This cites the long one, because it is the one an operator meeting the
    // control needs.
    (
        "a node head's keep capsule",
        &[Tipped {
            control: "a node head's keep capsule",
            cites: Cite {
                class: "mini",
                text: "keep",
                nth: 0,
            },
            at: on_node_keep,
        }],
    ),
    // **The row's two controls and not its four chips.** `SensChip::ALL` is
    // four and the mock tips all four; the signal and the range are readouts
    // that `SensChip::operation` answers `None` for, so a press reaches two of
    // them and this table is what a press reaches.
    (
        "a sensitivity row's curve and take back",
        &[
            Tipped {
                control: "the curve chip",
                cites: Cite {
                    class: "pill",
                    text: "pow2",
                    nth: 0,
                },
                at: on_curve,
            },
            Tipped {
                control: "the take back capsule",
                cites: Cite {
                    class: "pill",
                    text: "take back",
                    nth: 0,
                },
                at: on_take_back,
            },
        ],
    ),
    (
        "the Program bay head's solo",
        &[Tipped {
            control: "the solo capsule",
            cites: Cite {
                class: "pill",
                text: "solo",
                nth: 0,
            },
            at: on_solo,
        }],
    ),
    // **A bay head's grip has no tip in the mock.** The page draws it as
    // `.grip` and explains folding in its prose rather than on the control.
    ("the grip in a bay head", &[]),
    // **Four cells and four tips**, in `DECK_LETTERS` order, which is the
    // order the page draws them in: `ProgramBay::cell` answers *which* deck
    // the pointer is over, so each cell explains its own. The mock's classes
    // run out after two — A and B are `.preview.a` and `.preview.b` and the
    // last two are bare `.preview` — which is what `Cite::nth` is for.
    //
    // **The words are the mock's decks and not this run's**, which is
    // ADR-0330's third consequence: D's tip says nothing is behind that cell
    // because nothing is behind the mock's.
    (
        "the deck preview cells",
        &[
            Tipped {
                control: "deck A's preview cell",
                cites: Cite {
                    class: "preview a",
                    text: "",
                    nth: 0,
                },
                at: on_cell_a,
            },
            Tipped {
                control: "deck B's preview cell",
                cites: Cite {
                    class: "preview b",
                    text: "",
                    nth: 0,
                },
                at: on_cell_b,
            },
            Tipped {
                control: "deck C's preview cell",
                cites: Cite {
                    class: "preview",
                    text: "",
                    nth: 0,
                },
                at: on_cell_c,
            },
            Tipped {
                control: "deck D's preview cell",
                cites: Cite {
                    class: "preview",
                    text: "",
                    nth: 1,
                },
                at: on_cell_d,
            },
        ],
    ),
    (
        "the Library bay's scope chips",
        &[
            Tipped {
                control: "the all chip",
                cites: Cite {
                    class: "scope sel",
                    text: "all",
                    nth: 0,
                },
                at: on_scope_all,
            },
            Tipped {
                control: "the my sets chip",
                cites: Cite {
                    class: "scope",
                    text: "my sets",
                    nth: 0,
                },
                at: on_scope_mine,
            },
            Tipped {
                control: "the presets chip",
                cites: Cite {
                    class: "scope",
                    text: "presets",
                    nth: 0,
                },
                at: on_scope_presets,
            },
            Tipped {
                control: "the folder chip",
                cites: Cite {
                    class: "scope",
                    text: "folder",
                    nth: 0,
                },
                at: on_scope_folder,
            },
            Tipped {
                control: "the history chip",
                cites: Cite {
                    class: "scope",
                    text: "history",
                    nth: 0,
                },
                at: on_scope_history,
            },
        ],
    ),
    (
        "the Library bay's filter fields",
        &[Tipped {
            control: "the holds field",
            cites: Cite {
                class: "field",
                text: "holds&hellip;",
                nth: 0,
            },
            at: on_holds,
        }],
    ),
    (
        "the Library bay's kind chips",
        &[
            Tipped {
                control: "the L1 chip",
                cites: Cite {
                    class: "kind",
                    text: "L1",
                    nth: 0,
                },
                at: on_kind_l1,
            },
            Tipped {
                control: "the L2 chip",
                cites: Cite {
                    class: "kind",
                    text: "L2",
                    nth: 0,
                },
                at: on_kind_l2,
            },
            Tipped {
                control: "the L3 chip",
                cites: Cite {
                    class: "kind",
                    text: "L3",
                    nth: 0,
                },
                at: on_kind_l3,
            },
            Tipped {
                control: "the L4 chip",
                cites: Cite {
                    class: "kind",
                    text: "L4",
                    nth: 0,
                },
                at: on_kind_l4,
            },
            Tipped {
                control: "the FIELD chip",
                cites: Cite {
                    class: "kind",
                    text: "FIELD",
                    nth: 0,
                },
                at: on_kind_field,
            },
            Tipped {
                control: "the SET chip",
                cites: Cite {
                    class: "kind",
                    text: "SET",
                    nth: 0,
                },
                at: on_kind_sets,
            },
        ],
    ),
    (
        "the Library bay's row badges",
        &[Tipped {
            control: "a row's badges",
            cites: Cite {
                class: "badges",
                text: "L1L2L4",
                nth: 0,
            },
            at: on_badges,
        }],
    ),
    (
        "the params chip in the Library bay's foot",
        &[Tipped {
            control: "the params chip",
            cites: Cite {
                class: "pill armed",
                text: "params",
                nth: 0,
            },
            at: on_read,
        }],
    ),
    (
        "the Library bay's load button and deck pulldown",
        &[
            Tipped {
                control: "the load button",
                cites: Cite {
                    class: "pill lav",
                    text: "load",
                    nth: 0,
                },
                at: on_load_button,
            },
            Tipped {
                control: "the deck pulldown",
                cites: Cite {
                    class: "pill",
                    text: "A &#9662;",
                    nth: 0,
                },
                at: on_load_deck,
            },
        ],
    ),
    (
        "the Library bay's stars",
        &[Tipped {
            control: "a row's star",
            cites: Cite {
                class: "star",
                text: "&#9733;",
                nth: 0,
            },
            at: on_star,
        }],
    ),
    // **A row of the listing has no tip in the mock.** The page tips the rows
    // of a *reading* — `declares 6 knobs` and its kin — and the menu items a
    // secondary press puts down, and neither is the row this probe claims.
    ("the Library bay's list", &[]),
    // **One entry per class**, in `Class::ALL`'s order, which is the order the
    // four pills appear in the page: the Program bay's, the Mixer's, the
    // Master's and the Outputs row's. Every one of them is a bare
    // `.pill` reading `mcp &middot; shut`, so the four cites are one pair and
    // four ordinals — the one place on this console where `Cite::nth` is
    // carrying the whole of the distinction.
    //
    // **And the four tips are four different sentences**: each names the
    // operations its own class refuses while it reads shut, which is the thing
    // an operator hovers one of these to find out.
    (
        "the class pills",
        &[
            Tipped {
                control: "the Program bay's class pill",
                cites: Cite {
                    class: "pill",
                    text: "mcp &middot; shut",
                    nth: 0,
                },
                at: on_mcp_live_deck,
            },
            Tipped {
                control: "the Mixer bay's class pill",
                cites: Cite {
                    class: "pill",
                    text: "mcp &middot; shut",
                    nth: 1,
                },
                at: on_mcp_mix_faders,
            },
            Tipped {
                control: "the Master bay's class pill",
                cites: Cite {
                    class: "pill",
                    text: "mcp &middot; shut",
                    nth: 2,
                },
                at: on_mcp_master_effects,
            },
            Tipped {
                control: "the Outputs row's class pill",
                cites: Cite {
                    class: "pill",
                    text: "mcp &middot; shut",
                    nth: 3,
                },
                at: on_mcp_inputs_and_outputs,
            },
        ],
    ),
    // **Five kinds of control and eight entries**, in `Sequencer::press`'s own
    // order — the bank pills, a cell, a label, the mode pill — and then
    // `+ lane`, whose press is not an operation and comes back through
    // `Sequencer::chose`.
    //
    // **The four banks are four entries and the cells and the labels are
    // one each.** A bank is one of `karakuri_pattern::BANKS` fixed pills that
    // the page tips one at a time — `seq 3` is where the plus went and its tip
    // says so, which is not what `seq 1`'s says — and `SelectPattern` carries
    // which one. A cell and a label are per drawn step and per lane of a
    // pattern the host handed in: the page tips lane A's row and this console
    // draws whatever lanes there are, so a second entry there would be a cite
    // for a lane the mock does not have.
    (
        "the Sequencer bay's cells, labels, mode pill, bank pills and + lane",
        &[
            Tipped {
                control: "the seq 1 bank pill",
                cites: Cite {
                    class: "pill armed",
                    text: "seq 1",
                    nth: 0,
                },
                at: on_bank_1,
            },
            Tipped {
                control: "the seq 2 bank pill",
                cites: Cite {
                    class: "pill",
                    text: "seq 2",
                    nth: 0,
                },
                at: on_bank_2,
            },
            Tipped {
                control: "the seq 3 bank pill",
                cites: Cite {
                    class: "pill",
                    text: "seq 3",
                    nth: 0,
                },
                at: on_bank_3,
            },
            Tipped {
                control: "the seq 4 bank pill",
                cites: Cite {
                    class: "pill",
                    text: "seq 4",
                    nth: 0,
                },
                at: on_bank_4,
            },
            Tipped {
                control: "a lane's cell",
                cites: Cite {
                    class: "seq-lane",
                    text: "",
                    nth: 0,
                },
                at: on_step,
            },
            Tipped {
                control: "a lane's label",
                cites: Cite {
                    class: "seq-label",
                    text: "A&#9646;",
                    nth: 0,
                },
                at: on_lane_label,
            },
            Tipped {
                control: "the mode pill",
                cites: Cite {
                    class: "pill armed",
                    text: "1/16",
                    nth: 0,
                },
                at: on_step_mode,
            },
            Tipped {
                control: "the + lane pill",
                cites: Cite {
                    class: "pill",
                    text: "+ lane",
                    nth: 0,
                },
                at: on_add_lane,
            },
        ],
    ),
    // **The `back` capsule has no tip of its own in the mock**: the page tips
    // the candidate row it sits in, which is the entry below.
    ("the Staging lane's back capsules", &[]),
    (
        "the Staging lane's rows",
        &[Tipped {
            control: "a candidate row",
            cites: Cite {
                class: "cand",
                text: "L4:0drift_night&nbsp;bbackoverloaded",
                nth: 0,
            },
            at: on_candidate,
        }],
    ),
];

/// Every tipped control, flattened out of [`TIPS`] in the order [`resolve`]
/// asks them.
pub fn flat() -> impl Iterator<Item = &'static Tipped> {
    TIPS.iter().flat_map(|(_, tips)| tips.iter())
}

/// **Which control the pointer is on**, as an index into [`flat`], or `None`
/// where it is on none of them.
///
/// The first row that answers wins, which is why the order inside a slice is
/// the caller's: the controls inside a container come before the container.
pub fn resolve(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> Option<usize> {
    flat().position(|tip| (tip.at)(panel, ctx, view, p))
}

// -- the derivations, one per tipped control ------------------------------
//
// **Each is `crate::input`'s probe for the row it belongs to, asked one
// question finer**: the same derivation, and then the sub-question the caller
// asks to find out *which* control a press landed on. A second derivation
// would be a second answer that could disagree with the one the frame drew,
// which is that module's rule and is the reason these are written out here
// rather than composed out of the probes.

fn on_program_sink(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    outputs(ctx, panel.layout(), view.opening)
        .and_then(|row| row.chip_at(p))
        .is_some_and(|out| out == Output::Program)
}

fn on_projector_sink(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    outputs(ctx, panel.layout(), view.opening)
        .and_then(|row| row.chip_at(p))
        .is_some_and(|out| matches!(out, Output::Projector(_)))
}

fn on_audio(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    audio_in(ctx, panel.layout(), view.transport, view.audio.as_ref())
        .is_some_and(|pill| pill.hit(p))
}

fn on_tap(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    tracker_group(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
    )
    .is_some_and(|group| group.tapped(p).is_some())
}

fn on_octave(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    tracker_group(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
    )
    .is_some_and(|group| group.octave(p).is_some())
}

fn on_offset(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    tracker_group(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
    )
    .is_some_and(|group| group.nudge(p).is_some())
}

fn on_learn_pill(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    crate::view::learn_pill(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
        view.map.as_ref(),
        view.learn,
    )
    .is_some_and(|pill| pill.hit(p))
}

fn on_map_pill(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    crate::view::map_pill(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
        view.map.as_ref(),
    )
    .is_some_and(|row| row.pill.contains(Pos2::new(p.x, p.y)))
}

fn on_pill(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    arrangement(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
        None,
        &view.arrangement,
    )
    .is_some_and(|pill| pill.hit(p))
}

fn on_tonemap(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    look_row(panel, ctx, view).is_some_and(|row| row.tonemap(p).is_some())
}

fn on_exposure(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    look_row(panel, ctx, view).is_some_and(|row| row.exposure(p).is_some())
}

/// The look group, derived the one way [`crate::input::claim`] derives it.
fn look_row(panel: &Panel, ctx: &egui::Context, view: &View) -> Option<crate::view::LookRow> {
    look(
        ctx,
        panel.layout(),
        view.transport,
        view.audio.as_ref(),
        view.tracker,
        None,
        &view.arrangement,
        view.look,
    )
}

fn on_rec(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transport(ctx, panel.layout(), view.transport).is_some_and(|row| row.on_rec(p))
}

fn on_tempo(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transport(ctx, panel.layout(), view.transport).is_some_and(|row| row.on_tempo(p))
}

fn on_fader(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    mixer(ctx, panel.layout(), &view.mixer).is_some_and(|bay| bay.grab(p).is_some())
}

fn on_blend(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    mixer(ctx, panel.layout(), &view.mixer).is_some_and(|bay| bay.blend(p).is_some())
}

fn on_tally(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    mixer(ctx, panel.layout(), &view.mixer).is_some_and(|bay| bay.tally(p).is_some())
}

fn on_mask(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    mixer(ctx, panel.layout(), &view.mixer).is_some_and(|bay| bay.mask(p).is_some())
}

fn on_select(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    mixer(ctx, panel.layout(), &view.mixer).is_some_and(|bay| bay.select(p).is_some())
}

fn on_shape(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transition(ctx, panel.layout(), view.transition()).is_some_and(|row| row.shape(p).is_some())
}

fn on_quantum(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transition(ctx, panel.layout(), view.transition()).is_some_and(|row| row.quantum(p).is_some())
}

fn on_length(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transition(ctx, panel.layout(), view.transition()).is_some_and(|row| row.length(p).is_some())
}

/// **The `go` capsule, as what is left of the row.** `TransitionRow::go` takes
/// the selection and the deck count with the point — it answers *what a press
/// asks for*, and a press on it is refused where there is no deck to run it on
/// — where this question is only *is the pointer on the capsule*. The row's
/// own `owns` is that question over all four, so the three above it having
/// been asked first is what leaves this one the capsule.
fn on_go(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transition(ctx, panel.layout(), view.transition()).is_some_and(|row| row.owns(p))
}

fn on_master_out(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    master(ctx, panel.layout(), view.master_out, view.master_chain)
        .is_some_and(|row| row.grab(p).is_some())
}

fn on_master_chip(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    master(ctx, panel.layout(), view.master_out, view.master_chain)
        .is_some_and(|row| row.chip(p).is_some())
}

fn on_keep(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| keep_pill(ctx, &at, pane))
            .is_some_and(|pill| pill.hit(p))
    })
}

/// **The mark and not the card**, where `input`'s row answers for both: a tip
/// explains a control an operator is pointing at, and while the card is down
/// the hover layer is not what the next press is about — `claim`'s rule 2 has
/// already taken it.
fn on_pane_target(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| view.pane_pulldown(ctx, &at, pane, index))
            .is_some_and(|target| target.hit(p))
    })
}

fn on_node_keep(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.keep_procedure(ctx, pane, p).is_some())
    })
}

fn on_sync(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_head(panel, ctx, view, |head| head.sync(p).is_some())
}

fn on_anchor(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_head(panel, ctx, view, |head| head.reanchor(p).is_some())
}

fn on_scrub(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_head(panel, ctx, view, |head| head.scrub(p).is_some())
}

/// **The capacity chip, and it is `resized` rather than `hit_size`.** The two
/// are one question — `DeckHead::hit_size` is the chip *and* somewhere to step
/// to — and this is the one the press handler asks, so a chip that is drawn
/// and claims nothing (a deck whose geometries share no range) explains itself
/// exactly where a press on it does something.
fn on_size(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_head(panel, ctx, view, |head| head.resized(p).is_some())
}

/// **The `re-salt` capsule.** There is no state in which it is drawn and
/// inert, so this is `hit_salt` and `re_salted` at once; it is the latter for
/// the chip above's reason and for the press handler's.
fn on_salt(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_head(panel, ctx, view, |head| head.re_salted(p).is_some())
}

fn on_compositing(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_head(panel, ctx, view, |head| head.compositing(p).is_some())
}

/// Every pane's deck head, derived per pane the way `crate::input` derives it,
/// asked one of its own questions.
fn on_head(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
    ask: impl Fn(&crate::view::DeckHead) -> bool,
) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| deck_head(ctx, &at, pane))
            .is_some_and(|head| ask(&head))
    })
}

fn on_rend(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.select_renderer(ctx, pane, p).is_some())
    })
}

fn on_param(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.owns(pane, p))
    })
}

fn on_publish(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.publishing(pane, p).is_some())
    })
}

fn on_uses(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    let room = crate::view::to_egui(panel.layout().viewport());
    if let Some((pane_at, node, input)) = view.wiring_open() {
        let picked = view.inspector.get(pane_at).is_some_and(|pane| {
            inspector(panel.layout(), pane_at, pane, view.scroll_in(pane_at))
                .is_some_and(|at| at.wired(ctx, pane, room, (node, input), p).is_some())
        });
        if picked {
            return true;
        }
    }
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.uses_chip(ctx, pane, p).is_some())
    })
}

fn on_auth(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .is_some_and(|at| at.set_authority(ctx, pane, p).is_some())
    })
}

fn on_curve(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_sens(panel, ctx, view, p, |op| {
        matches!(op, Operation::AttachSignal { .. })
    })
}

fn on_take_back(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_sens(panel, ctx, view, p, |op| {
        matches!(op, Operation::TakeParamBack { .. })
    })
}

/// **Which of a sensitivity row's two controls the pointer is on**, told apart
/// by what a press on it would ask for — `InspectorPane::sensitivity` walks the
/// row's four chips and answers the operation the one under the pointer names,
/// which is `SensChip::operation` and is where the signal and the range being
/// readouts is already decided. A second walk here would be a second answer to
/// *which chip is this*.
fn on_sens(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
    p: Point,
    which: impl Fn(&Operation) -> bool,
) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| at.sensitivity(ctx, pane, p))
            .is_some_and(|op| which(&op))
    })
}

fn on_solo(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    program_head(ctx, panel.layout(), view.opening).is_some_and(|head| head.hit(p))
}

fn on_cell_a(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_cell(panel, view, p, 0)
}

fn on_cell_b(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_cell(panel, view, p, 1)
}

fn on_cell_c(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_cell(panel, view, p, 2)
}

fn on_cell_d(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_cell(panel, view, p, 3)
}

/// **One preview cell**, told from the three beside it by the deck
/// `ProgramBay::cell` says the pointer is over — the row's own answer, which
/// is what `ProgramBay::owns` is the union of and what a release on the row
/// is resolved against. The cell of a deck with no slot is a cell like any
/// other here: it is drawn, so it is pointed at, and `dropped`'s refusal is
/// about the carry rather than about the rectangle.
fn on_cell(panel: &Panel, view: &View, p: Point, deck: u8) -> bool {
    program_bay(panel.layout(), view.canvas).is_some_and(|bay| bay.cell(p) == Some(deck))
}

fn on_scope_all(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_scope(panel, ctx, view, p, Scope::AllSets)
}

fn on_scope_mine(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_scope(panel, ctx, view, p, Scope::MySets)
}

fn on_scope_presets(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_scope(panel, ctx, view, p, Scope::Presets)
}

fn on_scope_folder(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_scope(panel, ctx, view, p, Scope::Folder)
}

fn on_scope_history(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_scope(panel, ctx, view, p, Scope::History)
}

/// **One scope chip**, told from the four beside it by the chip the bay says
/// the pointer is on — `LibraryBay::chip`'s own answer, which carries the
/// scope beside the operation, rather than a second walk of the row.
fn on_scope(panel: &Panel, ctx: &egui::Context, view: &View, p: Point, which: Scope) -> bool {
    library_bay(panel, view)
        .and_then(|bay| bay.chip(ctx, &view.scopes, p))
        .is_some_and(|chosen| chosen.scope == which)
}

fn on_holds(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_field(panel, view, p, Field::Holds)
}

fn on_kind_l1(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_kind(panel, ctx, view, p, KindChip::Layer(Layer::L1))
}

fn on_kind_l2(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_kind(panel, ctx, view, p, KindChip::Layer(Layer::L2))
}

fn on_kind_l3(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_kind(panel, ctx, view, p, KindChip::Layer(Layer::L3))
}

fn on_kind_l4(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_kind(panel, ctx, view, p, KindChip::Layer(Layer::L4))
}

fn on_kind_field(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_kind(panel, ctx, view, p, KindChip::Layer(Layer::Field))
}

fn on_kind_sets(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_kind(panel, ctx, view, p, KindChip::Sets)
}

/// **One kind chip**, told from the five beside it by the chip the bay says is
/// at that point — `LibraryBay::kind_chips` is the same walk the paint makes
/// and the same one a press is resolved against, so a tip and a press cannot
/// land on two different chips.
fn on_kind(panel: &Panel, ctx: &egui::Context, view: &View, p: Point, want: KindChip) -> bool {
    let at = Pos2::new(p.x, p.y);
    library_bay(panel, view).is_some_and(|bay| {
        bay.kinds.is_some_and(|row| row.contains(at))
            && bay
                .kind_chips(ctx)
                .any(|(chip, box_)| chip == want && box_.contains(at))
    })
}

/// **A row's badges**, which is the one readout in this bay's list: nothing is
/// pressed there, and the tip is what says so and what the words mean. Asked of
/// the rows the bay is drawing, so a badge under the pointer is a badge on
/// screen.
fn on_badges(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    let at = Pos2::new(p.x, p.y);
    library_bay(panel, view).is_some_and(|bay| {
        let rows = view.rows();
        bay.list.contains(at)
            && bay.drawn().any(|index| {
                bay.badges(ctx, index, &rows.badges(index))
                    .any(|(_, box_)| box_.contains(at))
            })
    })
}

/// **One filter field.** `LibraryBay::filter` answers with the `ListSets` a
/// press asks for and not with which box it landed in, so the box is asked
/// for — `LibraryBay::field` is the same rectangle that method tests, asked
/// by name.
fn on_field(panel: &Panel, view: &View, p: Point, which: Field) -> bool {
    library_bay(panel, view)
        .and_then(|bay| bay.field(which))
        .is_some_and(|box_| box_.contains(Pos2::new(p.x, p.y)))
}

fn on_read(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    library_bay(panel, view).is_some_and(|bay| {
        bay.read(ctx, view.target(), view.rows().set(view.cursor_row()), p)
            .is_some()
    })
}

fn on_load_button(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    library_bay(panel, view).is_some_and(|bay| bay.load(ctx, view.target()).hit_button(p))
}

fn on_load_deck(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    library_bay(panel, view).is_some_and(|bay| bay.load(ctx, view.target()).hit_deck(p))
}

fn on_star(panel: &Panel, _ctx: &egui::Context, view: &View, p: Point) -> bool {
    library_bay(panel, view).is_some_and(|bay| bay.starred(view.rows(), &view.starred, p).is_some())
}

/// The Library bay, derived the one way [`crate::input::claim`] derives it.
fn library_bay(panel: &Panel, view: &View) -> Option<crate::view::LibraryBay> {
    library(
        panel.layout(),
        &view.scopes,
        &view.library,
        view.opened(),
        view.pointed(),
        view.library_scroll(),
    )
}

fn on_mcp_live_deck(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_mcp(panel, ctx, view, p, Class::LiveDeck)
}

fn on_mcp_mix_faders(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_mcp(panel, ctx, view, p, Class::MixFaders)
}

fn on_mcp_master_effects(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_mcp(panel, ctx, view, p, Class::MasterEffects)
}

fn on_mcp_inputs_and_outputs(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_mcp(panel, ctx, view, p, Class::InputsAndOutputs)
}

/// **One class pill.** The four are in four different bay heads and cannot be
/// one laid-out box, so the class is the argument that lays one out — which is
/// the press handler's own arrangement, one derivation asked four times.
fn on_mcp(panel: &Panel, ctx: &egui::Context, view: &View, p: Point, class: Class) -> bool {
    mcp_pill(ctx, panel.layout(), class, view.opening).is_some_and(|pill| pill.hit(p))
}

fn on_bank_1(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_bank(panel, ctx, view, p, 0)
}

fn on_bank_2(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_bank(panel, ctx, view, p, 1)
}

fn on_bank_3(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_bank(panel, ctx, view, p, 2)
}

fn on_bank_4(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_bank(panel, ctx, view, p, 3)
}

/// **One bank pill**, told from the three beside it by the pattern the press
/// would name: `Sequencer::press` answers `SelectPattern` carrying the bank,
/// which is this bay's own rule that *every arm names the bank*.
fn on_bank(panel: &Panel, ctx: &egui::Context, view: &View, p: Point, bank: u8) -> bool {
    on_seq(
        panel,
        ctx,
        view,
        p,
        |op| matches!(op, Operation::SelectPattern { pattern } if *pattern == bank),
    )
}

fn on_step(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_seq(panel, ctx, view, p, |op| {
        matches!(op, Operation::SetStep { .. })
    })
}

fn on_lane_label(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_seq(panel, ctx, view, p, |op| {
        matches!(op, Operation::SetLaneMute { .. })
    })
}

fn on_step_mode(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_seq(panel, ctx, view, p, |op| {
        matches!(op, Operation::SetPatternGrid { .. })
    })
}

/// **Which kind of control the pointer is on in the Sequencer bay**, told
/// apart by the operation a press there would ask for.
///
/// `Sequencer::press` is *four controls and one answer*, and which of them it
/// was is inside the operation it hands back — the press handler's own
/// sentence about this bay. So the sub-question is that operation read, and
/// there is no second walk of the cells here to disagree with the one the
/// paint made.
fn on_seq(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
    p: Point,
    which: impl Fn(&Operation) -> bool,
) -> bool {
    sequencer_bay(panel, ctx, view)
        .and_then(|bay| bay.press(p))
        .is_some_and(|op| which(&op))
}

/// **The `+ lane` pill, which is the one control in this bay whose press is
/// not an operation**: it puts a card down, so `Sequencer::chose` is what
/// answers for it and `Chose::Open` is the pill itself.
///
/// **`Shut` is every other point on the console while the card is down**, and
/// that is what keeps this from claiming the whole window: with the card up
/// this answers `Open` on the pill and `None` everywhere else, and with one
/// down it answers `Shut` — which is not this control — everywhere including
/// on the pill. A tip under an open card is ADR-0330's own open seam.
fn on_add_lane(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    // The one derivation, and the choices it was laid out from asked again:
    // the card's items are the chooser's own listing, so a bay drawn from one
    // reading and asked against another would answer for a card it did not
    // draw — `Sequencer::chose`'s own re-check, from this side.
    let choices = view.lane_choices();
    sequencer(ctx, panel.layout(), view.sequencer.as_ref(), &choices)
        .is_some_and(|bay| matches!(bay.chose(p, &choices), Some(crate::view::Chose::Open)))
}

/// The Sequencer bay, derived the one way [`crate::input::claim`] derives it.
fn sequencer_bay(
    panel: &Panel,
    ctx: &egui::Context,
    view: &View,
) -> Option<crate::view::Sequencer> {
    sequencer(
        ctx,
        panel.layout(),
        view.sequencer.as_ref(),
        &view.lane_choices(),
    )
}

fn on_candidate(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    staging(panel.layout(), &view.staging)
        .is_some_and(|bay| bay.keep(ctx, &view.staging, p).is_some())
}

// -- the box, transcribed from `[data-tip]::after` ------------------------
//
// **The mock draws the tip in CSS and this is that rule, term for term.**
// Every constant below cites `style.css`, and `tests/hover.rs` resolves each
// citation the way `tests/transcribed_constants_cite_the_mock.rs` resolves
// `room::size`'s: the stylesheet is the specification and it is the half that
// moves.

/// `[data-tip]::after`'s `min-width: 150px`: a tip is never narrower than
/// this, however few words are in it.
pub const TIP_MIN_W: f32 = 150.0;

/// `[data-tip]::after`'s `max-width: 236px`, which is what the words wrap to
/// once the padding is taken off.
pub const TIP_MAX_W: f32 = 236.0;

/// `[data-tip]::after`'s `padding: 7px 9px`, down the sides.
pub const TIP_PAD_X: f32 = 9.0;

/// `[data-tip]::after`'s `padding: 7px 9px`, top and bottom.
pub const TIP_PAD_Y: f32 = 7.0;

/// `[data-tip]::after`'s `border-radius: 9px`.
pub const TIP_RADIUS: f32 = 9.0;

/// `[data-tip]::after`'s `font-size: 10.5px`.
pub const TIP_SIZE: f32 = 10.5;

/// `[data-tip]::after`'s `line-height: 1.55`, as a multiple of the size above.
pub const TIP_LINE: f32 = 1.55;

/// `[data-tip]::after`'s `top: calc(100% + 6px)`: the gap between what is
/// being explained and the box explaining it.
pub const TIP_GAP: f32 = 6.0;

/// **What the words wrap to**: [`TIP_MAX_W`] less the padding either side,
/// which is the width a browser lays this text out in.
///
/// The console's own arithmetic and not a number in the stylesheet — `box-sizing`
/// is the browser's rule rather than a declaration, and this is it.
pub const TIP_WRAP: f32 = TIP_MAX_W - TIP_PAD_X * 2.0;

/// `[data-tip]::after`'s `box-shadow: 0 8px 26px rgba(0,0,0,0.22)`, and it is
/// the tip's own rather than `--c-shadow`: the mock gives this one box a
/// shadow of its own, so the palette's is not the one to draw it with. 0.22 of
/// 255 is 56.
pub const TIP_SHADOW: egui::epaint::Shadow = egui::epaint::Shadow {
    offset: [0, 8],
    blur: 26,
    spread: 0,
    color: Color32::from_rgba_premultiplied(0, 0, 0, 56),
};

// -- the words, parsed out of the page ------------------------------------

/// **Every tip [`TIPS`] cites, in [`flat`]'s order**, read out of [`PAGE`]
/// once.
///
/// An entry is `None` where its [`Cite`] resolved to nothing, which is a
/// transcription that has gone stale rather than a control with no tip — a
/// control with no tip has no row at all. `tests/hover.rs` is where that
/// fails; a panel that met one would draw nothing for that control rather than
/// something wrong.
pub struct Tips {
    words: Vec<Option<String>>,
}

impl Tips {
    /// **Parse the page.** At start-up and never on a frame: it walks 340 KB
    /// once and allocates one `String` per tip (P-0091).
    pub fn read() -> Tips {
        let elements = elements(PAGE);
        let words = flat()
            .map(|tip| {
                elements
                    .iter()
                    .filter(|el| el.class == tip.cites.class && el.text == tip.cites.text)
                    .nth(tip.cites.nth)
                    .map(|el| decode(el.tip))
            })
            .collect();
        Tips { words }
    }

    /// The words for one control, by its index in [`flat`].
    pub fn get(&self, index: usize) -> Option<&str> {
        self.words.get(index)?.as_deref()
    }

    /// How many tips were asked for, which is [`flat`]'s length.
    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// Whether nothing was asked for at all, which would mean [`TIPS`] is
    /// empty — `Tips` is never empty in this crate and `clippy` asks for this
    /// beside [`Tips::len`].
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// **Every control whose [`Cite`] resolved to nothing**, by index. Empty
    /// on a page and a table that agree.
    pub fn missing(&self) -> impl Iterator<Item = usize> + '_ {
        self.words
            .iter()
            .enumerate()
            .filter(|(_, words)| words.is_none())
            .map(|(index, _)| index)
    }
}

/// One element of the mock that carries a `data-tip`.
pub struct Element<'a> {
    /// Its `class` attribute, or `""`.
    pub class: &'a str,
    /// The text inside it, tags stripped and whitespace collapsed — the page's
    /// own spelling, entities and all.
    pub text: String,
    /// The `data-tip` attribute's value, still escaped.
    pub tip: &'a str,
}

/// **Every element in the page carrying a `data-tip`, in document order.**
///
/// A scan and not a parser: it finds the attribute, walks back to the `<` that
/// opened the tag, reads the class beside it, and takes the text up to the
/// matching close tag. That is enough for this page and it is deliberately not
/// enough for HTML in general — what it cannot read it drops, and
/// `tests/hover.rs` carries a floor so a scan that stopped reading fails
/// rather than passing over an empty set (`gpu_tests_are_under_mod_gpu.rs`'s
/// shape).
pub fn elements(page: &str) -> Vec<Element<'_>> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(found) = page[from..].find("data-tip=\"") {
        let at = from + found;
        let Some(open) = page[..at].rfind('<') else {
            from = at + 1;
            continue;
        };
        let name_end = page[open + 1..]
            .find(|c: char| c.is_whitespace() || c == '>')
            .map(|end| open + 1 + end)
            .unwrap_or(page.len());
        let name = &page[open + 1..name_end];
        // The opening tag's own end, found with quotes honoured: an attribute
        // value may hold a `>` and the page's `style` attributes do.
        let Some(tag_end) = unquoted(page, name_end, '>') else {
            from = at + 1;
            continue;
        };
        let attrs = &page[name_end..tag_end];
        let class = attribute(attrs, "class").unwrap_or("");
        let tip = attribute(attrs, "data-tip").unwrap_or("");
        let inner = inner_of(page, name, tag_end + 1);
        out.push(Element {
            class,
            text: text_of(inner),
            tip,
        });
        from = tag_end + 1;
    }
    out
}

/// The first `what` outside a quoted attribute value, from `at`.
fn unquoted(page: &str, at: usize, what: char) -> Option<usize> {
    let mut quote = None;
    for (index, ch) in page[at..].char_indices() {
        match (quote, ch) {
            (None, '"') | (None, '\'') => quote = Some(ch),
            (Some(open), ch) if ch == open => quote = None,
            (None, ch) if ch == what => return Some(at + index),
            _ => {}
        }
    }
    None
}

/// One attribute's value out of an opening tag's attribute text.
fn attribute<'a>(attrs: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let at = attrs.find(&key)? + key.len();
    let end = attrs[at..].find('"')? + at;
    Some(&attrs[at..end])
}

/// The text between an opening tag that ends at `from` and its matching close,
/// with tags of the same name nested inside it counted.
fn inner_of<'a>(page: &'a str, name: &str, from: usize) -> &'a str {
    let open = format!("<{name}");
    let close = format!("</{name}");
    let mut depth = 1usize;
    let mut at = from;
    while depth > 0 {
        let next_open = page[at..].find(&open).map(|found| at + found);
        let next_close = page[at..].find(&close).map(|found| at + found);
        match (next_open, next_close) {
            (Some(o), Some(c)) if o < c => {
                depth += 1;
                at = o + open.len();
            }
            (_, Some(c)) => {
                depth -= 1;
                if depth == 0 {
                    return &page[from..c];
                }
                at = c + close.len();
            }
            _ => return &page[from..],
        }
    }
    &page[from..]
}

/// An element's text: its tags removed and its runs of whitespace collapsed to
/// one space. The page's own entities are left as they are, because a [`Cite`]
/// quotes the markup.
fn text_of(inner: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    let mut space = false;
    for ch in inner.chars() {
        match ch {
            '<' => inside = true,
            '>' => inside = false,
            _ if inside => {}
            ch if ch.is_whitespace() => space = !out.is_empty(),
            ch => {
                if space {
                    out.push(' ');
                    space = false;
                }
                out.push(ch);
            }
        }
    }
    out
}

/// **The named entities the mock uses**, and nothing else: an entity that is
/// not here comes out of [`decode`] unchanged and `tests/hover.rs` fails
/// naming it, so the day the page uses a sixteenth this stops reading rather
/// than reading wrongly.
const NAMED: [(&str, &str); 15] = [
    ("&mdash;", "\u{2014}"),
    ("&ndash;", "\u{2013}"),
    ("&middot;", "\u{00b7}"),
    ("&rarr;", "\u{2192}"),
    ("&times;", "\u{00d7}"),
    ("&minus;", "\u{2212}"),
    ("&plusmn;", "\u{00b1}"),
    ("&plus;", "+"),
    ("&hellip;", "\u{2026}"),
    ("&frac12;", "\u{00bd}"),
    ("&lowast;", "\u{2217}"),
    ("&rsquo;", "\u{2019}"),
    ("&nbsp;", "\u{00a0}"),
    ("&lt;", "<"),
    ("&gt;", ">"),
];

/// **One tip's words, as a reader sees them**: the attribute's value with its
/// entities resolved.
///
/// `&amp;` is last on purpose — it is resolved after every other entity, so a
/// literal ampersand in the page cannot turn the text after it into one.
pub fn decode(escaped: &str) -> String {
    let mut out = String::with_capacity(escaped.len());
    let mut rest = escaped;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let Some(end) = tail.find(';').map(|end| end + 1) else {
            out.push('&');
            rest = &tail[1..];
            continue;
        };
        let entity = &tail[..end];
        match numeric(entity).or_else(|| named(entity)) {
            Some(ch) => out.push_str(&ch),
            None => out.push_str(entity),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out.replace("&amp;", "&")
}

/// `&#8853;` and `&#x2295;`, or `None` for anything else.
fn numeric(entity: &str) -> Option<String> {
    let digits = entity.strip_prefix("&#")?.strip_suffix(';')?;
    let code = match digits
        .strip_prefix('x')
        .or_else(|| digits.strip_prefix('X'))
    {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse().ok()?,
    };
    Some(char::from_u32(code)?.to_string())
}

/// One of [`NAMED`], or `None`.
fn named(entity: &str) -> Option<String> {
    NAMED
        .iter()
        .find(|(name, _)| *name == entity)
        .map(|(_, ch)| (*ch).to_owned())
}

// -- the layer ------------------------------------------------------------

/// **What the hover layer is owed**, and what [`crate::repaint::Change::Tip`]
/// turns into a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tip {
    /// **Nothing.** The pointer is on no tipped control, or the tip it is on
    /// is already drawn and does not move while it is up. This is the layer
    /// declaring nothing at rest.
    Still,
    /// **A dwell is running**, and this is what is left of it: the picture is
    /// different from the one on screen in exactly this long, which is
    /// [`crate::budget::Declared::moves_in`]'s question asked of a hand
    /// holding still (ADR-0283).
    Dwelling(Duration),
    /// **A tip is on screen that must not be**: the pointer has left the
    /// control it belongs to. A frame is owed now, to take it down.
    Gone,
}

/// **The console's hover layer**: which control the pointer is resting on,
/// since when, and the one galley the tip is drawn from.
///
/// **It holds no clock.** Every time in it arrives as a [`Duration`] from the
/// caller, which is this crate's rule and `crate::view::Transport`'s argument
/// for it: *"an `Instant` here would put a clock in it"*.
pub struct Hover {
    tips: Tips,
    resting: Option<Rest>,
    /// Whether the tip has been painted. Written by [`Hover::paint`], because
    /// what is on screen is a fact about the frame that drew it.
    up: bool,
    /// The words laid out, kept while the pointer stays on the control they
    /// belong to **and its assignment has not moved**. A learn changes the
    /// last line of the tip that is on screen, and a cache keyed on the
    /// control alone would go on drawing the old one.
    galley: Option<(usize, Option<String>, Arc<Galley>)>,
    /// **Which knob the control under the pointer is on**, as the host derived
    /// it from the live map — see [`Hover::assign`].
    assigned: Option<String>,
}

/// Where the pointer came to rest, when, and on what.
#[derive(Debug, Clone, Copy)]
struct Rest {
    at: Point,
    since: Duration,
    on: usize,
}

impl Default for Hover {
    fn default() -> Hover {
        Hover::new()
    }
}

impl Hover {
    /// **Read the page and take no other reading.** At start-up: the parse is
    /// [`Tips::read`]'s and is not on any frame path.
    pub fn new() -> Hover {
        Hover {
            tips: Tips::read(),
            resting: None,
            up: false,
            galley: None,
            assigned: None,
        }
    }

    /// **Where the pointer is resting and on which control**, or `None` where
    /// it is on none — the point, and an index into [`flat`].
    ///
    /// **This is the learn seam.** Learn is *point at a control and move a
    /// knob*, so what it needs is exactly what this layer already worked out
    /// for the tip: which control the pointer is on. A second derivation in
    /// the host would be a second answer that could disagree with the tip the
    /// operator is reading while they do it
    /// (`docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md`).
    ///
    /// **It answers before the dwell.** A tip waits half a second because
    /// reading one is a decision; pointing at a control and reaching for a
    /// knob is not, and a learn that only worked once the box was up would be
    /// a gesture with a hidden timer in it.
    pub fn resting(&self) -> Option<(Point, usize)> {
        self.resting.map(|rest| (rest.at, rest.on))
    }

    /// **Which knob the control under the pointer is on**, for the tip's last
    /// line — `Some("cc 5")`, or `None` for a control nothing is mapped to.
    ///
    /// # Why this crosses the seam instead of being derived here
    ///
    /// A control's assignment is a fact the **map** holds, and this crate
    /// cannot read one: `karakuri-midi` depends on `midir`, and
    /// [ADR-0156](../../../docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)
    /// is that this crate takes no device. So it arrives the way every other
    /// value does — derived by the host and handed in, beside the [`View`].
    ///
    /// # And why it is derived at all
    ///
    /// The page's own `⊕ MIDI:` line is the **mock's** assignment and no
    /// operator's. A run whose map puts `cc 5` on gain A read `cc → gain A` in
    /// the tip because the page said so, and the tip was then confidently
    /// wrong about the one thing somebody would hover to check
    /// ([P-0087](../../../docs/principles/0087-name-the-property-never-the-shape.md)).
    /// **Where nothing is mapped the page's sentence stays**, because it says
    /// *why* — *a map line names a slot, a range or a word from a closed list,
    /// and a Set id is none of the three* — and no reverse lookup can produce
    /// that.
    ///
    /// Set once per frame by the host; `None` on a run with no surface at all,
    /// which leaves every tip exactly as the page wrote it.
    pub fn assign(&mut self, on: Option<String>) {
        self.assigned = on;
    }

    /// **How long a pointer holds still before a tip appears**, and it is
    /// `egui`'s own `Interaction::tooltip_delay` rather than a number written
    /// here.
    ///
    /// **Read rather than transcribed**, which is
    /// [`docs/contributing.md` §4](../../../docs/contributing.md)'s first tier
    /// over a number this program would otherwise have invented: the toolkit
    /// this console is drawn with already carries the interval a tooltip waits
    /// for, so the console paints its own layer and still waits as long as
    /// everything else the operator's machine draws. It is 0.5 s in `egui`
    /// 0.36's default style, and a caller that changes the style moves this
    /// with it.
    pub fn dwell(ctx: &egui::Context) -> Duration {
        let style = ctx.style_of(ctx.theme());
        Duration::from_secs_f32(style.interaction.tooltip_delay.max(0.0))
    }

    /// **The pointer moved.** Returns what is owed for it — [`Tip::Gone`]
    /// where a tip is on screen and the pointer has left the control it
    /// belongs to, and [`Tip::Still`] otherwise, because a move that starts a
    /// dwell has already earned a frame from
    /// [`crate::repaint::Change::Pointer`] and [`Hover::owed`] is what that
    /// frame asks for the deadline.
    ///
    /// **The claim goes in with the point.** `crate::input::claim` has already
    /// walked the console's controls to answer it, and `Claim::Egui` is the
    /// panel saying the pointer is on none of them — so the common move, which
    /// is over nothing, costs one comparison here and the walk below happens
    /// only where a control really is under the pointer.
    ///
    /// **A drag is not a hover.** A gesture in hand is the pointer being used
    /// rather than pointed, so it takes any tip down and starts none: that is
    /// `claim`'s rule 1 read on this layer rather than a second copy of it.
    pub fn moved(
        &mut self,
        claim: Claim,
        panel: &Panel,
        ctx: &egui::Context,
        view: &View,
        p: Point,
        now: Duration,
    ) -> Tip {
        let on = match claim == Claim::Panel && !panel.dragging() {
            true => resolve(panel, ctx, view, p),
            false => None,
        };
        match (self.resting, on) {
            // The same control: a tip that is up stays exactly where it is,
            // and one that is not yet up starts its dwell again from here.
            (Some(rest), Some(on)) if rest.on == on => {
                if !self.up {
                    self.resting = Some(Rest {
                        at: p,
                        since: now,
                        on,
                    });
                }
                Tip::Still
            }
            // Another control, or none at all.
            (_, on) => {
                let owed = match self.up {
                    true => Tip::Gone,
                    false => Tip::Still,
                };
                self.up = false;
                self.resting = on.map(|on| Rest {
                    at: p,
                    since: now,
                    on,
                });
                owed
            }
        }
    }

    /// **The pointer left the window**, which is not a move to anywhere: any
    /// tip goes and no dwell is running.
    pub fn left(&mut self) -> Tip {
        let owed = match self.up {
            true => Tip::Gone,
            false => Tip::Still,
        };
        self.up = false;
        self.resting = None;
        owed
    }

    /// **When this layer's picture is next different from the one on screen**,
    /// asked on every frame the way `crate::view::View::animating` is.
    ///
    /// [`Tip::Dwelling`] while a dwell is running and [`Tip::Still`]
    /// everywhere else — including once the tip is up, because it does not
    /// move while it is shown. A panel nobody is pointing at asks for nothing.
    ///
    /// **A dwell with nothing left of it is `Still` and not `Dwelling(0)`**,
    /// which is what makes this *when is the picture next different from the
    /// one this frame is about to draw* rather than *from the one already on
    /// screen*: the caller asks this inside the pass that is about to paint,
    /// so the frame the deadline was for is the frame it is being asked on,
    /// and a zero here would buy one more frame drawing what this one drew.
    pub fn owed(&self, ctx: &egui::Context, now: Duration) -> Tip {
        match (self.resting, self.up) {
            (Some(rest), false) => {
                match Hover::dwell(ctx).saturating_sub(now.saturating_sub(rest.since)) {
                    left if left.is_zero() => Tip::Still,
                    left => Tip::Dwelling(left),
                }
            }
            _ => Tip::Still,
        }
    }

    /// **The words on screen**, or `None` where no tip is up. What a test
    /// reads, and the only way anything outside this module can tell.
    pub fn showing(&self) -> Option<&str> {
        match self.up {
            true => self.tips.get(self.resting?.on),
            false => None,
        }
    }

    /// **Which control the pointer is resting on**, whether or not its dwell
    /// has run out.
    pub fn resting_on(&self) -> Option<&'static Tipped> {
        flat().nth(self.resting?.on)
    }

    /// The tips this layer read out of the page.
    pub fn tips(&self) -> &Tips {
        &self.tips
    }

    /// **Paint the tip, last of everything on the console.**
    ///
    /// It is called from inside the same pass as `View::draw` and after it, so
    /// the box goes over every card and every bay — which is the mock's
    /// `z-index: 30` and the order the four cards are already painted in.
    ///
    /// **Nothing is allocated after the first frame it is up**: the galley is
    /// laid out once for the control the pointer is on and kept until the
    /// pointer leaves it. What every frame after that costs is a shadow, a
    /// fill, a stroke and one `galley` call.
    pub fn paint(&mut self, ui: &Ui, panel: &Panel, view: &View, now: Duration) {
        let Some(rest) = self.resting else {
            return;
        };
        if now.saturating_sub(rest.since) < Hover::dwell(ui.ctx()) {
            return;
        }
        let Some(words) = self.tips.get(rest.on) else {
            return;
        };
        let pal = view.room.palette();
        let words = assigned(words, self.assigned.as_deref());
        let galley = match &self.galley {
            Some((on, was, galley))
                if *on == rest.on && was.as_deref() == self.assigned.as_deref() =>
            {
                galley.clone()
            }
            _ => {
                let mut job = LayoutJob::single_section(
                    words.clone(),
                    TextFormat {
                        font_id: FontId::new(TIP_SIZE, FontFamily::Proportional),
                        color: pal.text,
                        line_height: Some(TIP_SIZE * TIP_LINE),
                        ..Default::default()
                    },
                );
                job.wrap.max_width = TIP_WRAP;
                let galley = ui.painter().layout_job(job);
                self.galley = Some((rest.on, self.assigned.clone(), galley.clone()));
                galley
            }
        };
        self.up = true;

        let size = egui::vec2(
            galley.size().x.max(TIP_MIN_W - TIP_PAD_X * 2.0) + TIP_PAD_X * 2.0,
            galley.size().y + TIP_PAD_Y * 2.0,
        );
        let box_ = placed(to_egui(panel.layout().viewport()), rest.at, size);
        let painter = ui.painter();
        let radius = CornerRadius::same(TIP_RADIUS as u8);
        painter.add(TIP_SHADOW.as_shape(box_, radius));
        painter.rect_filled(box_, radius, pal.panel);
        painter.rect_stroke(
            box_,
            radius,
            Stroke::new(HAIRLINE, pal.line),
            StrokeKind::Inside,
        );
        painter.galley(
            box_.min + egui::vec2(TIP_PAD_X, TIP_PAD_Y),
            galley,
            pal.text,
        );
    }
}

/// **The tip, with its MIDI line read off the live map** where there is one to
/// read.
///
/// The page's `⊕ MIDI:` clause is the last thing every tip says, so what this
/// does is cut there and write the fact instead of the picture — see
/// [`Hover::assign`] for why, and
/// `docs/adr/0336-a-learn-is-a-map-edit-and-the-tips-midi-line-is-the-live-map.md`
/// for the whole argument.
///
/// **Three cases and only one of them rewrites anything.**
///
/// - A control the map reaches: the clause becomes what the map says.
/// - A control nothing is mapped to (`on` is `None`): **the page's own
///   sentence stays**, because it carries the reason — *a map line names a
///   slot, a range or a word from a closed list* — which is worth more than
///   the word *unassigned* this could put there instead.
/// - A tip with no `⊕ MIDI:` clause at all: untouched. The mock is not
///   exhaustive and a clause invented for a control the page is silent about
///   would be this console writing the manual.
///
/// **It is a `Cow` in effect and an allocation only where it rewrites**: a
/// `String` is built on the frame a tip appears and on no other, which is the
/// same frame the galley is laid out on.
pub fn assigned(words: &str, on: Option<&str>) -> String {
    let Some(on) = on else {
        return words.to_owned();
    };
    let Some((head, _)) = words.rsplit_once(MIDI_LINE) else {
        return words.to_owned();
    };
    format!("{head}{MIDI_LINE} {on}, which is what the map in use says today.")
}

/// **The last clause of every tip on the page**, as `console.html` spells it
/// once the entities are resolved — the `⊕` is `&#8853;`.
///
/// Written here rather than derived because it is the page's own punctuation
/// and there is nothing to derive it from: what makes it safe is that
/// `tests/hover.rs` fails if the page stops ending its tips this way.
pub const MIDI_LINE: &str = "\u{2295} MIDI:";

/// **Where the box goes: under the pointer, and inside the window.**
///
/// The mock hangs a tip off the element it explains — `top: calc(100% + 6px)`,
/// `left: 0` — and flips it at two edges: `.tip-right` and the last column
/// open leftward, and the Outputs row opens upward, *so that hovering cannot
/// summon a scrollbar*. This console has no scrollbar to summon and the rule
/// is the same one: **a tip never leaves the window**, so it opens down and to
/// the right of the pointer and flips at whichever edge it would cross.
///
/// **It is anchored to the pointer rather than to the control**, which is the
/// one place this departs from the page. A [`Tipped::at`] answers *is the
/// pointer on this control* and not *where is it*, so a box hung off the
/// control's own box would need every derivation to hand back a rectangle. The
/// pointer is where the operator is looking; the day a rectangle is wanted for
/// something else, this is what would change.
///
/// A box wider or taller than the window is clamped to the near edge rather
/// than flipped, because a flip would only move which half is cut off.
pub fn placed(viewport: Rect, at: Point, size: egui::Vec2) -> Rect {
    let x = match at.x + size.x > viewport.max.x {
        true => at.x - size.x,
        false => at.x,
    };
    let y = match at.y + TIP_GAP + size.y > viewport.max.y {
        true => at.y - TIP_GAP - size.y,
        false => at.y + TIP_GAP,
    };
    let min = Pos2::new(
        x.clamp(
            viewport.min.x,
            (viewport.max.x - size.x).max(viewport.min.x),
        ),
        y.clamp(
            viewport.min.y,
            (viewport.max.y - size.y).max(viewport.min.y),
        ),
    );
    Rect::from_min_size(min, size)
}
