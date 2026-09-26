use super::hit_test::*;
use super::Tipped;
use crate::hover::citation::Cite;
use crate::input::PROBES;

/// Tooltip entries for each control registered in [`crate::input::PROBES`].
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
    // Six entries for deck head controls in press-handler order; capacity and re-salt
    // cite the primary pane's definitions (ADR-0328).
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
    // Cites the comprehensive node head keep capsule description from `drift_shell`.
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
    // Four preview cells in `DECK_LETTERS` order, citing mock deck descriptions (ADR-0330).
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
    // One entry per class in `Class::ALL` order, using `Cite::nth` ordinals to differentiate
    // identical `.pill` markups and their class-specific refused operations.
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
    // Sequencer controls in `Sequencer::press` order (bank pills, cells, labels, minus glyphs,
    // mode pill) followed by `+ lane` chooser action.
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
