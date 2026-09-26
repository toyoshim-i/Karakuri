use super::hit_test::*;
use super::Tipped;
use crate::hover::citation::Cite;
use crate::input::PROBES;

/// Tooltip entries for each control registered in [`crate::input::PROBES`].
pub const TIPS: [(&str, &[Tipped]); PROBES.len()] = [
    // Outputs row sink chips: plugin chip switches nothing.
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
    // Tracker group: tempo tap, octave display, and tempo readout.
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
    // Map pill: cites mock nanoKONTROL2 device element.
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
    // Mixer strip controls in probe resolution order.
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
    // Pane head name has no tooltip in mock.
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
    // Inspector pane deck pulldown glyph (`▾`).
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
    // Parameter row publish indicator (published vs unpublished, ADR-0329).
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
                    text: "&middot;",
                    nth: 0,
                },
                at: on_publish,
            },
        ],
    ),
    // Node input rewiring pill.
    (
        "a node group's `uses` capsule and its card",
        &[Tipped {
            control: "a node's declared input",
            cites: Cite {
                // Targets the inner pill element.
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
    // Sensor row active interactive controls.
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
    // Bay head fold grip has no tooltip in mock.
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
    // Library listing row has no tooltip in mock.
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
            // Lane remove button (third `.minus` occurrence on page).
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
    // Staging lane back capsule has no standalone tooltip in mock.
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
