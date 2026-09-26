use super::*;

#[test]
fn a_bare_positional_pair_still_works() {
    let args = parse(&["a.kir", "b.kir"]).expect("should parse");
    assert_eq!(
        args.sets,
        vec![(Named::bare("a.kir"), vec![Named::bare("b.kir")])]
    );
}

#[test]
fn one_positional_file_is_rejected() {
    let err = parse(&["a.kir"]).unwrap_err();
    assert!(err.contains("1 file argument"), "message: {err}");
}

#[test]
fn three_positional_files_are_rejected() {
    let err = parse(&["a.kir", "b.kir", "c.kir"]).unwrap_err();
    assert!(err.contains("3 file argument"), "message: {err}");
}

#[test]
fn a_positional_pair_becomes_the_last_slot_after_set() {
    let args = parse(&["--set", "a.kir,b.kir", "c.kir", "d.kir"]).expect("should parse");
    assert_eq!(
        args.sets,
        vec![
            (Named::bare("a.kir"), vec![Named::bare("b.kir")]),
            (Named::bare("c.kir"), vec![Named::bare("d.kir")]),
        ]
    );
}

// -- --set, adversarially ---------------------------------------------

#[test]
fn set_with_no_value_fails() {
    let err = parse(&["--set"]).unwrap_err();
    assert!(err.contains("--set"), "message: {err}");
}

#[test]
fn set_with_one_path_and_no_comma_fails() {
    let err = parse(&["--set", "a.kir"]).unwrap_err();
    assert!(err.contains("--set a.kir"), "message: {err}");
}

/// A bare `--param` is a wildcard and stays one. Every node declaring the name
/// — "the Set's `exposure`", one knob moving both renderers — which is what
/// this flag has always meant, so the address arriving must not quietly turn it
/// into "node 0".
#[test]
fn a_param_with_no_address_reaches_every_node_declaring_it() {
    let args = parse(&["--param", "exposure=2.5"]).expect("should parse");
    assert_eq!(
        args.overrides,
        vec![ParamWrite::everywhere("exposure", 2.5)],
        "a bare name must stay unaddressed"
    );
}

/// And the prefix is what sets two renderers apart, which a bare name cannot do
/// by construction.
#[test]
fn a_param_can_address_one_renderer() {
    let args = parse(&["--param", "L4:1:exposure=2.5", "--param", "L1:0:radius=3.0"])
        .expect("should parse");
    assert_eq!(
        args.overrides,
        vec![
            ParamWrite::at(karakuri_ir::Kind::L4, 1, "exposure", 2.5),
            ParamWrite::at(karakuri_ir::Kind::L1, 0, "radius", 3.0),
        ]
    );
}

/// A half-address is refused rather than read as a name with a colon in it. The
/// address is `layer:index:` present or absent as a unit, and a param name
/// cannot contain a colon, so there is nothing else `L4:exposure` can be trying
/// to say.
#[test]
fn a_half_written_param_address_fails() {
    for bad in [
        "L4:exposure=2.5",
        "L4:x:exposure=2.5",
        "L9:0:exposure=2.5",
        ":0:e=1",
        "=2.5",
    ] {
        let err = parse(&["--param", bad]).unwrap_err();
        assert!(
            err.contains("--param"),
            "`{bad}` was accepted or misreported: {err}"
        );
    }
}

/// `--bind` needed no new grammar at all: its fields are the record's, so the
/// address is one more field. Absent is a wildcard there too.
#[test]
fn a_bind_can_address_one_renderer_and_defaults_to_all_of_them() {
    let all =
        parse(&["--bind", "layer=L4,key=exposure,signal=beat,range=0..1"]).expect("should parse");
    assert_eq!(
        all.bindings[0].index, None,
        "a bind with no index is the layer's"
    );

    let one = parse(&[
        "--bind",
        "layer=L4,index=2,key=exposure,signal=beat,range=0..1",
    ])
    .expect("should parse");
    assert_eq!(one.bindings[0].index, Some(2));

    let err = parse(&["--bind", "layer=L4,index=x,key=e,signal=beat,range=0..1"]).unwrap_err();
    assert!(err.contains("index"), "a bad index is not named: {err}");
}

/// Verifies that multiple comma-separated paths on `--set` map to one geometry and subsequent renderers.
#[test]
fn set_with_three_paths_is_one_geometry_and_two_renderers() {
    let args = parse(&["--set", "a.kir,b.kir,c.kir"]).expect("should parse");
    assert_eq!(
        args.sets,
        vec![(
            Named::bare("a.kir"),
            vec![Named::bare("b.kir"), Named::bare("c.kir")],
        )],
        "the first path is the geometry and the rest are renderers, in draw order"
    );
}

/// The stray comma the rule above used to catch is still caught, because an
/// empty part is not a filename: `a.kir,b.kir,` names a renderer with no name.
#[test]
fn set_with_a_trailing_comma_fails() {
    let err = parse(&["--set", "a.kir,b.kir,"]).unwrap_err();
    assert!(err.contains("--set"), "message: {err}");
}

#[test]
fn set_with_an_empty_value_fails() {
    let err = parse(&["--set", ""]).unwrap_err();
    assert!(err.contains("--set"), "message: {err}");
}

#[test]
fn set_with_an_empty_side_fails() {
    assert!(parse(&["--set", ",b.kir"]).is_err());
    assert!(parse(&["--set", "a.kir,"]).is_err());
}

#[test]
fn set_given_five_times_exceeds_max_slots() {
    let err = parse(&[
        "--set", "a,b", "--set", "c,d", "--set", "e,f", "--set", "g,h", "--set", "i,j",
    ])
    .unwrap_err();
    assert!(err.contains("5 Sets"), "message: {err}");
}

#[test]
fn a_bare_pair_alongside_four_sets_exceeds_max_slots() {
    let err = parse(&[
        "--set", "a,b", "--set", "c,d", "--set", "e,f", "--set", "g,h", "i.kir", "j.kir",
    ])
    .unwrap_err();
    assert!(err.contains("5 Sets"), "message: {err}");
}

#[test]
fn set_given_exactly_four_times_is_allowed() {
    let args = parse(&[
        "--set", "a,b", "--set", "c,d", "--set", "e,f", "--set", "g,h",
    ])
    .expect("4 sets should fit MAX_SLOTS");
    assert_eq!(args.sets.len(), 4);
}

// -- --exposure, adversarially -----------------------------------------

#[test]
fn exposure_missing_value_fails() {
    assert!(parse(&["--exposure"]).is_err());
}

#[test]
fn exposure_non_number_fails() {
    let err = parse(&["--exposure", "bright"]).unwrap_err();
    assert!(err.contains("--exposure bright"), "message: {err}");
}

#[test]
fn exposure_negative_fails() {
    assert!(parse(&["--exposure", "-2"]).is_err());
}

#[test]
fn exposure_zero_fails() {
    assert!(parse(&["--exposure", "0"]).is_err());
}

#[test]
fn exposure_positive_is_accepted_unclamped() {
    // Parsing does not clamp — only the interactive control does, on
    // purpose, so a batch render can ask for something extreme. `100`
    // is well outside the interactive `-`/`=` bound.
    let args = parse(&["--exposure", "100"]).expect("should parse");
    assert_eq!(args.look.exposure, 100.0);
}

// -- --tonemap, adversarially -------------------------------------------

#[test]
fn tonemap_bad_name_fails() {
    let err = parse(&["--tonemap", "bloom"]).unwrap_err();
    assert!(err.contains("--tonemap bloom"), "message: {err}");
}

#[test]
fn tonemap_every_documented_name_is_accepted() {
    for (name, op) in [
        ("clamp", TonemapOp::Clamp),
        ("reinhard", TonemapOp::Reinhard),
        ("aces", TonemapOp::Aces),
        ("agx", TonemapOp::AgX),
    ] {
        let args = parse(&["--tonemap", name]).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(args.look.op, op, "{name}");
    }
}
