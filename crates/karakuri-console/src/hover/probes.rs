//! Control probes, hit-test derivations, and registration table for the console hover layer.

use egui::Pos2;
use karakuri_layout::Point;
use karakuri_operation::gate::Class;
use karakuri_operation::{Layer, Operation, Output};

use super::citation::Cite;
use crate::control::{descriptor_for, ControlDescriptor, ControlId};
use crate::input::PROBES;
use crate::panel::Panel;
use crate::view::{
    arrangement, audio_in, deck_head, inspector, keep_pill, library, look, master, mcp_pill, mixer,
    outputs, program_bay, program_head, sequencer, staging, tracker_group, transition, transport,
    Field, KindChip, Scope, View,
};

/// One control that explains itself on hover: its identity, where its words are
/// in the mock, and the derivation that says the pointer is on it.
pub struct Tipped {
    /// The control's name, in the words [`crate::input::PROBES`] uses for the row
    /// it belongs to. It is what a test and a reader address this row by, and it is
    /// never drawn.
    pub control: &'static str,
    /// Where this control's words are in the mock.
    pub cites: Cite,
    /// The derivation that draws the control, asked whether the point is on it, and
    /// nothing is stored — [`crate::input::Probe::ask`]'s signature and its rule,
    /// one question finer.
    pub at: fn(&Panel, &egui::Context, &View, Point) -> bool,
}

/// Every tip this console can draw, one entry per row of
/// [`crate::input::PROBES`] and in that table's order.
///
/// The array is `PROBES.len()` long, so a control registered there and never
/// given a tip here is a compile error rather than a control that quietly
/// explains nothing. `tests/hover.rs` holds each entry against the row it
/// answers for, by name and by position, so an entry cannot answer for its
/// neighbour.
///
/// An empty slice is the reading rule and not an omission. The maintainer,
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
    ("the Inspector pane heads' slot mcp policy", &[]),
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
    // **Six kinds of control and nine entries**, in `Sequencer::press`'s own
    // order — the bank pills, a cell, a label, a lane's minus, the mode pill
    // — and then `+ lane`, whose press is not an operation and comes back
    // through `Sequencer::chose`.
    //
    // **The four banks are four entries and the cells, the labels and the
    // minus glyphs are one each.** A bank is one of `karakuri_pattern::BANKS` fixed pills that
    // the page tips one at a time — `seq 3` is where the plus went and its tip
    // says so, which is not what `seq 1`'s says — and `SelectPattern` carries
    // which one. A cell and a label are per drawn step and per lane of a
    // pattern the host handed in: the page tips lane A's row and this console
    // draws whatever lanes there are, so a second entry there would be a cite
    // for a lane the mock does not have. A lane's minus is the same reading:
    // the page draws one per lane and this cites lane A's.
    (
        "the Sequencer bay's cells, labels, minus glyphs, mode pill, bank pills and + lane",
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
            // **The third `.minus` on the page**, the two before it being the
            // chain's slots: the Master bay's glyph drawn on a lane's row, and
            // the page tips lane A's the way it tips lane A's cells.
            Tipped {
                control: "a lane's minus",
                cites: Cite {
                    class: "minus",
                    text: "&minus;",
                    nth: 2,
                },
                at: on_lane_remove,
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

/// Which control the pointer is on, as an index into [`flat`], or `None` where
/// it is on none of them.
///
/// The first row that answers wins, which is why the order inside a slice is
/// the caller's: the controls inside a container come before the container.
pub fn resolve(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> Option<usize> {
    if ctx.cumulative_pass_nr() == 0 || view.has_modal_overlay() {
        return None;
    }
    flat().position(|tip| (tip.at)(panel, ctx, view, p))
}

/// Returns the [`ControlId`] corresponding to a flat tip index.
pub fn control_id_at(index: usize) -> Option<ControlId> {
    let mut count = 0;
    for (probe_idx, (_, tips)) in TIPS.iter().enumerate() {
        if index < count + tips.len() {
            return ControlId::from_probe_index(probe_idx);
        }
        count += tips.len();
    }
    None
}

/// Returns the [`ControlDescriptor`] corresponding to a flat tip index.
pub fn descriptor_at(index: usize) -> Option<&'static ControlDescriptor> {
    control_id_at(index).map(descriptor_for)
}

/// Returns the keyboard shortcut assigned to a specific tipped control, if any.
pub fn hotkey_for_tip(index: usize) -> Option<&'static str> {
    let id = control_id_at(index)?;
    let tip = flat().nth(index)?;
    match id {
        ControlId::Transition => {
            if tip.control == "the go capsule" {
                Some("space")
            } else {
                None
            }
        }
        ControlId::Tracker => {
            if tip.control == "the tap" {
                Some("b")
            } else {
                None
            }
        }
        _ => descriptor_at(index).and_then(|d| d.hotkey),
    }
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

/// The `go` capsule, as what is left of the row. `TransitionRow::go` takes the
/// selection and the deck count with the point — it answers *what a press asks
/// for*, and a press on it is refused where there is no deck to run it on —
/// where this question is only *is the pointer on the capsule*. The row's own
/// `owns` is that question over all four, so the three above it having been
/// asked first is what leaves this one the capsule.
fn on_go(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    transition(ctx, panel.layout(), view.transition()).is_some_and(|row| row.owns(p))
}

fn on_master_out(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    master(
        ctx,
        panel.layout(),
        view.master_out,
        view.master_chain.as_ref(),
        &view.chain_choices(),
    )
    .is_some_and(|row| row.grab(p).is_some())
}

fn on_master_chip(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    master(
        ctx,
        panel.layout(),
        view.master_out,
        view.master_chain.as_ref(),
        &view.chain_choices(),
    )
    .is_some_and(|row| row.chip(p).is_some() || row.chose(p, &view.chain_choices()).is_some())
}

fn on_keep(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    view.inspector.iter().enumerate().any(|(index, pane)| {
        inspector(panel.layout(), index, pane, view.scroll_in(index))
            .and_then(|at| keep_pill(ctx, &at, pane))
            .is_some_and(|pill| pill.hit(p))
    })
}

/// The mark and not the card, where `input`'s row answers for both: a tip
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

/// The capacity chip, and it is `resized` rather than `hit_size`. The two are
/// one question — `DeckHead::hit_size` is the chip *and* somewhere to step to —
/// and this is the one the press handler asks, so a chip that is drawn and
/// claims nothing (a deck whose geometries share no range) explains itself
/// exactly where a press on it does something.
fn on_size(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_head(panel, ctx, view, |head| head.resized(p).is_some())
}

/// The `re-salt` capsule. There is no state in which it is drawn and inert, so
/// this is `hit_salt` and `re_salted` at once; it is the latter for the chip
/// above's reason and for the press handler's.
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

/// Which of a sensitivity row's two controls the pointer is on, told apart by
/// what a press on it would ask for — `InspectorPane::sensitivity` walks the
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

/// One preview cell, told from the three beside it by the deck
/// `ProgramBay::cell` says the pointer is over — the row's own answer, which is
/// what `ProgramBay::owns` is the union of and what a release on the row is
/// resolved against. The cell of a deck with no slot is a cell like any other
/// here: it is drawn, so it is pointed at, and `dropped`'s refusal is about the
/// carry rather than about the rectangle.
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

/// One scope chip, told from the four beside it by the chip the bay says the
/// pointer is on — `LibraryBay::chip`'s own answer, which carries the scope
/// beside the operation, rather than a second walk of the row.
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

/// One kind chip, told from the five beside it by the chip the bay says is at
/// that point — `LibraryBay::kind_chips` is the same walk the paint makes and
/// the same one a press is resolved against, so a tip and a press cannot land
/// on two different chips.
fn on_kind(panel: &Panel, ctx: &egui::Context, view: &View, p: Point, want: KindChip) -> bool {
    let at = Pos2::new(p.x, p.y);
    library_bay(panel, view).is_some_and(|bay| {
        bay.kinds.is_some_and(|row| row.contains(at))
            && bay
                .kind_chips(ctx)
                .any(|(chip, box_)| chip == want && box_.contains(at))
    })
}

/// A row's badges, which is the one readout in this bay's list: nothing is
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

/// One filter field. `LibraryBay::filter` answers with the `ListSets` a press
/// asks for and not with which box it landed in, so the box is asked for —
/// `LibraryBay::field` is the same rectangle that method tests, asked by name.
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

/// One class pill. The four are in four different bay heads and cannot be one
/// laid-out box, so the class is the argument that lays one out — which is the
/// press handler's own arrangement, one derivation asked four times.
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

/// One bank pill, told from the three beside it by the pattern the press would
/// name: `Sequencer::press` answers `SelectPattern` carrying the bank, which is
/// this bay's own rule that *every arm names the bank*.
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

fn on_lane_remove(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_seq(panel, ctx, view, p, |op| {
        matches!(op, Operation::RemoveLane { .. })
    })
}

fn on_step_mode(panel: &Panel, ctx: &egui::Context, view: &View, p: Point) -> bool {
    on_seq(panel, ctx, view, p, |op| {
        matches!(op, Operation::SetPatternGrid { .. })
    })
}

/// Which kind of control the pointer is on in the Sequencer bay, told apart by
/// the operation a press there would ask for.
///
/// `Sequencer::press` is *four controls and one answer*, and which of them it
/// was is inside the operation it hands back — the press handler's own sentence
/// about this bay. So the sub-question is that operation read, and there is no
/// second walk of the cells here to disagree with the one the paint made.
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

/// The `+ lane` pill, which is the one control in this bay whose press is not
/// an operation: it puts a card down, so `Sequencer::chose` is what answers for
/// it and `Chose::Open` is the pill itself.
///
/// `Shut` is every other point on the console while the card is down, and that
/// is what keeps this from claiming the whole window: with the card up this
/// answers `Open` on the pill and `None` everywhere else, and with one down it
/// answers `Shut` — which is not this control — everywhere including on the
/// pill. A tip under an open card is ADR-0330's own open seam.
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
