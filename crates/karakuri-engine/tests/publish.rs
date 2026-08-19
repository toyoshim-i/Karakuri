//! A Set's interface: which of its controls a console shows.
//!
//! **Publishing decides what is shown, never what is reachable** —
//! `docs/ir-spec.md`, "What a Set publishes". Every claim here is one of the
//! four that section makes:
//!
//! - **An empty interface publishes everything**, so the feature is additive and
//!   every Set that predates it keeps working.
//! - **A published range narrows, never redefines.** A subset is checkable and a
//!   range outside the declared one is refused rather than clamped.
//! - **A `param` still reaches an unpublished control**, because a surface is a
//!   choice about attention and not about authority.
//! - **A macro is a binding whose source is a published control**, which is the
//!   case the whole thing is for: two scenes, one knob on the desk, and the
//!   twenty other numbers left where the author put them.

use karakuri_engine::binding::{Binding, Curve};
use karakuri_engine::set::{Bound, Layering, PublishError, Published};
use karakuri_engine::{Gpu, Set, Signals};
use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;

const GRID: &str = r#"
proc grid {
  kind     L1
  topology points
  capacity [4, 4] = 4

  param radius : float [0.5, 8.0] = 2.0

  emit position

  element {
    position = sphere_point(hash1(seed), hash1(seed + 7u)) * radius;
  }
}
"#;

/// Two of them, so the Set has two `exposure`s to tell apart — which is the case
/// an interface exists for.
///
/// **They declare different ranges**, which is what makes the wildcard control's
/// range a decision rather than a copy: one knob moving both cannot offer a
/// position that only one of them said it still looks like itself at.
fn dots(name: &str, top: f32) -> String {
    format!(
        r#"
proc {name} {{
  kind  L4
  blend additive

  param exposure : float [0.0, {top:?}] = 1.0

  consumes position

  vertex {{
    clip       = camera * vec4(position, 1.0);
    point_size = 4.0;
  }}

  fragment {{
    color = vec4(exposure, exposure, exposure, 1.0);
  }}
}}
"#
    )
}

fn compile(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)))
}

fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n")
}

fn build(gpu: &Gpu) -> Set {
    let a = compile(&dots("near", 8.0));
    let b = compile(&dots("far", 4.0));
    Set::build_many(
        &gpu.device,
        &gpu.queue,
        &[(&compile(GRID), 4)],
        &[],
        None,
        None,
        &[&a, &b],
        Layering::Overdraw,
        1,
    )
    .expect("one L1 and two L4s")
}

fn control(name: &str, layer: Kind, index: u32, key: &str, range: [f32; 2]) -> Published {
    Published {
        name: name.to_string(),
        at: Some((layer, index)),
        key: key.to_string(),
        range,
    }
}

/// The wildcard form: every node that declares the key, which is what the
/// default interface is made of.
fn every(name: &str, key: &str, range: [f32; 2]) -> Published {
    Published { name: name.to_string(), at: None, key: key.to_string(), range }
}

// ---------------------------------------------------------------------------

/// **An empty interface publishes everything**, each control over the range its
/// own procedure declared. That is what happens today, so the feature is
/// additive and every Set file that predates it keeps working.
#[test]
fn a_set_with_no_interface_publishes_every_control_it_declares() {
    let gpu = Gpu::headless().expect("no GPU available");
    let set = build(&gpu);
    let all = set.published();

    // **One control per key, not per declaration.** Two renderers declare
    // `exposure` and it is one knob moving both — which is what a bare name
    // means everywhere else in this system, and what publishing it per
    // declaration could not be: two controls of one name is a console that
    // cannot address either.
    assert_eq!(all.len(), 2, "{all:#?}");
    // **The intersection, not the union.** `near` declares `[0, 8]` and `far`
    // declares `[0, 4]`; one knob moving both must not offer a position only one
    // of them said it still looks like itself at.
    assert_eq!(
        all,
        vec![every("exposure", "exposure", [0.0, 4.0]), every("radius", "radius", [0.5, 8.0])]
    );

    // And it moves both, exactly as `--param exposure=` does.
    let mut set = set;
    assert!(set.set_published("exposure", 4.0));
    let held: Vec<f32> = set
        .params()
        .filter(|(_, _, key, _)| *key == "exposure")
        .map(|(_, _, _, v)| v)
        .collect();
    assert_eq!(held, vec![4.0, 4.0], "one knob did not move both renderers");
}

/// **The first declaration makes the list the interface.** One sentence of rule,
/// and it means an author opts in by naming what they want rather than by hiding
/// the other twenty-four things.
#[test]
fn the_first_published_control_becomes_the_whole_interface() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = build(&gpu);
    set.publish(control("size", Kind::L1, 0, "radius", [1.0, 4.0]))
        .expect("the L1 declares `radius`");

    let all = set.published();
    assert_eq!(all.len(), 1, "{all:#?}");
    assert_eq!(all[0].name, "size");
    assert_eq!(all[0].range, [1.0, 4.0]);
}

/// **A published range narrows and never redefines.** A subset is what the
/// procedure's declaration allows; anything outside it is refused rather than
/// clamped, because the declared range is the procedure's statement about where
/// it still looks like itself and a Set cannot make a claim on its behalf.
#[test]
fn a_published_range_must_be_inside_the_declared_one() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = build(&gpu);

    let err = set
        .publish(control("size", Kind::L1, 0, "radius", [0.1, 4.0]))
        .expect_err("0.1 is below the declared 0.5");
    assert!(matches!(err, PublishError::RangeNotASubset { .. }), "{err:?}");
    assert!(err.to_string().contains("narrows"), "{err}");

    // And nothing was published, so a refusal leaves the Set as it was rather
    // than half-configured: the default interface, one control per key.
    assert_eq!(set.published().len(), 2);

    set.publish(control("size", Kind::L1, 0, "radius", [0.5, 8.0]))
        .expect("the declared range itself is a subset of itself");
}

/// A control nothing declares, and a name published twice: a console shows one
/// control per name, so the second is a mistake rather than a replacement.
#[test]
fn publishing_refuses_a_control_that_is_not_there_and_a_name_that_is() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = build(&gpu);

    let err = set
        .publish(control("nope", Kind::L4, 0, "twist", [0.0, 1.0]))
        .expect_err("no renderer declares `twist`");
    assert!(matches!(err, PublishError::NoSuchControl { .. }), "{err:?}");

    // A Set with no L2 has no L2 node to address, on the same terms an
    // out-of-range renderer index has none.
    let err = set
        .publish(control("nope", Kind::L2, 0, "exposure", [0.0, 1.0]))
        .expect_err("this Set holds no deformations");
    assert!(matches!(err, PublishError::NoSuchControl { .. }), "{err:?}");

    set.publish(control("level", Kind::L4, 0, "exposure", [0.0, 2.0])).expect("first");
    // And an addressed control is checked against *that* node's declaration:
    // `far` declares `[0, 4]`, so publishing it over `[0, 8]` is refused even
    // though its sibling would allow it.
    let err = set
        .publish(control("hot", Kind::L4, 1, "exposure", [0.0, 8.0]))
        .expect_err("renderer 1 declares [0, 4]");
    assert!(matches!(err, PublishError::RangeNotASubset { .. }), "{err:?}");
    let err = set
        .publish(control("level", Kind::L4, 1, "exposure", [0.0, 2.0]))
        .expect_err("`level` is taken");
    assert!(matches!(err, PublishError::DuplicateName(_)), "{err:?}");
}

/// **A published control is set in the units the console shows it in**, and
/// clamped to the published range — which is the one place narrowing bites: a
/// console cannot ask for more than the Set offered.
#[test]
fn a_published_control_writes_the_param_it_names_and_is_clamped_to_its_range() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = build(&gpu);
    set.publish(control("size", Kind::L1, 0, "radius", [1.0, 4.0])).expect("published");

    assert!(set.set_published("size", 3.0));
    assert_eq!(set.published_value("size"), Some(3.0));
    assert_eq!(set.param("radius"), Some(3.0), "the param itself moved");

    assert!(set.set_published("size", 7.5), "a write past the published top is still a write");
    assert_eq!(set.published_value("size"), Some(4.0), "clamped to what the Set offered");

    assert!(!set.set_published("radius", 2.0), "the internal name is not on the console");
}

/// **Publishing decides what is shown, never what is reachable.** A `param`
/// record — and `--param`, and an agent over MCP — still addresses any control
/// in any node, published or not. If publishing gated access, a Set's author
/// could lock an operator out of their own machine.
#[test]
fn an_unpublished_control_is_still_reachable_by_address() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = build(&gpu);
    // An interface of exactly one control, so the two `exposure`s are both off
    // the console.
    set.publish(control("size", Kind::L1, 0, "radius", [1.0, 4.0])).expect("published");
    assert_eq!(set.published().len(), 1);

    assert!(set.set_param_at(Kind::L4, 1, "exposure", 0.25));
    assert!(
        set.params()
            .any(|(layer, index, key, value)| layer == Kind::L4
                && index == 1
                && key == "exposure"
                && value == 0.25),
        "an unpublished control was not writable"
    );
    // And the published range does not clamp an addressed write, because it is
    // a statement about the console rather than about the value.
    assert!(set.set_param_at(Kind::L1, 0, "radius", 7.0));
    assert_eq!(set.param("radius"), Some(7.0));
}

/// **A macro is a binding whose source is a published control**, which needed no
/// new record and no new semantics — `bind`'s source became "a signal, or a
/// published control", and each bound control follows through its own curve and
/// range.
///
/// This is the case the whole feature is for: one knob on the desk moving two
/// renderers' exposure in opposite directions, and neither of them on the
/// console.
#[test]
fn one_published_control_drives_several_internal_ones_through_their_own_ranges() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = build(&gpu);
    set.publish(control("blend", Kind::L1, 0, "radius", [1.0, 3.0])).expect("published");

    let bind = |index: u32, range: [f32; 2]| {
        Binding::new(Kind::L4, "exposure", "control:blend", Curve::Lin, range).at(index)
    };
    // One renderer rises with the knob and the other falls: a crossfade, spelled
    // as two bindings that share a source.
    assert!(set.bind(bind(0, [0.0, 2.0])).attached());
    assert!(set.bind(bind(1, [2.0, 0.0])).attached());

    let exposure = |set: &Set, index: u32| {
        set.bindings()
            .iter()
            .find(|b| b.layer == Kind::L4 && b.index == Some(index) && b.key == "exposure")
            .expect("bound")
            .value()
    };

    // Bottom of the published range: the first renderer at its own bottom, the
    // second at its own top.
    set.set_published("blend", 1.0);
    set.prepare(&gpu.queue, 1, &Signals::default());
    assert!((exposure(&set, 0) - 0.0).abs() < 1e-4, "{}", exposure(&set, 0));
    assert!((exposure(&set, 1) - 2.0).abs() < 1e-4, "{}", exposure(&set, 1));

    // Top of it, and the two have swapped.
    set.set_published("blend", 3.0);
    set.prepare(&gpu.queue, 1, &Signals::default());
    assert!((exposure(&set, 0) - 2.0).abs() < 1e-4, "{}", exposure(&set, 0));
    assert!((exposure(&set, 1) - 0.0).abs() < 1e-4, "{}", exposure(&set, 1));

    // And the middle is the middle, which is what says the position is mapped
    // rather than thresholded.
    set.set_published("blend", 2.0);
    set.prepare(&gpu.queue, 1, &Signals::default());
    assert!((exposure(&set, 0) - 1.0).abs() < 1e-3, "{}", exposure(&set, 0));
    assert!((exposure(&set, 1) - 1.0).abs() < 1e-3, "{}", exposure(&set, 1));
}

/// **A binding on a control nothing publishes is refused.**
///
/// It used to be accepted, hold its param wherever it found it, and be reported
/// by the terminal as deciding that param outright — the same failure the
/// confidence display had, one step further along. A misspelt `control:` name is
/// a mistake, and the only moment it can be caught is when the binding is
/// attached.
///
/// The order it puts on a caller is the order a macro needs anyway: publish,
/// then bind. Both the command line and the swap worker already do that, for
/// this reason.
#[test]
fn a_binding_on_a_control_nothing_publishes_is_refused() {
    let gpu = Gpu::headless().expect("no GPU available");
    let mut set = build(&gpu);
    set.publish(control("twist", Kind::L1, 0, "radius", [1.0, 4.0])).expect("published");

    let bind = |name: &str| {
        Binding::new(Kind::L4, "exposure", format!("control:{name}"), Curve::Lin, [0.0, 2.0])
    };
    // **And it says which of the two it missed.** The param is fine — it is
    // the control that is not there — and a refusal that says "no L4 parameter
    // named `exposure`" sends whoever reads it to the wrong half of their
    // command line.
    assert_eq!(
        set.bind(bind("twst")),
        Bound::NoSuchControl,
        "a misspelt control was accepted, or reported as a missing param"
    );
    assert_eq!(
        set.bind(Binding::new(Kind::L4, "no_such_param", "control:twist", Curve::Lin, [0.0, 1.0])),
        Bound::NoSuchParam,
        "a published control does not make an undeclared param bindable"
    );
    assert!(set.bindings().is_empty(), "the refusal still attached it");
    assert!(set.bind(bind("twist")).attached(), "the control that is there");
}
