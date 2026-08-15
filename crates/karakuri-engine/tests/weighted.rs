//! `blend weighted`, while the language has it and the engine does not.
//!
//! One claim, and it is about a refusal rather than about a picture: a Set whose
//! L4 declares a blend mode with no lowering behind it must fail to build. The
//! alternative is not a missing feature but a wrong one — `blend` reaches no
//! part of lowering, so the pair would compile, build, and draw additively.

use karakuri_engine::{Gpu, Set, SetError};
use karakuri_ir::typed::Checked;

const L1: &str = r#"
proc cloud {
  kind     L1
  topology points
  capacity [8, 64] = 16

  emit position

  element {
    position = vec3(0.0, 0.0, 0.0);
  }
}
"#;

fn l4(blend: &str) -> String {
    format!(
        r#"
proc glassy {{
  kind  L4
  blend {blend}

  consumes position

  vertex {{
    clip       = vec4(position, 1.0);
    point_size = 4.0;
  }}

  fragment {{
    color = vec4(1.0, 1.0, 1.0, 0.5);
  }}
}}
"#
    )
}

fn compile(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    let checked = karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    checked
}

fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter().map(|e| e.render(src)).collect::<Vec<_>>().join("\n")
}

fn build(gpu: &Gpu, l4_src: &str) -> Result<Set, SetError> {
    Set::build(&gpu.device, &gpu.queue, &compile(L1), &compile(l4_src), 16, 3)
}

/// **The refusal, against a control that differs in one word.** The same pair
/// under `additive` has to build, or the test would pass for a procedure that
/// was simply broken.
#[test]
fn a_weighted_l4_is_refused_by_name_and_an_additive_one_builds() {
    let gpu = Gpu::headless().expect("no GPU available");

    build(&gpu, &l4("additive")).expect("the control must build");

    match build(&gpu, &l4("weighted")) {
        Err(SetError::UnbuiltBlend { l4, mode }) => {
            assert_eq!(l4, "glassy");
            assert_eq!(mode, "weighted");
        }
        Err(other) => panic!("refused for the wrong reason: {other}"),
        Ok(_) => panic!("`blend weighted` built, and would have drawn additively"),
    }
}
