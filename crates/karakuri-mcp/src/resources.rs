use agent_man::DocEngine;

use crate::operations;

pub const SPEC: &str = "karakuri://doc/ir-spec";
pub const VOCABULARY: &str = "karakuri://doc/ir-vocabulary";
pub const OPERATIONS: &str = "karakuri://doc/operations";
pub const VR_PROJECTION: &str = "karakuri://doc/vr-projection";

/// Builds the documentation engine loaded with all static and generated Karakuri documentation.
pub fn doc_engine() -> DocEngine {
    let mut engine = DocEngine::new("karakuri")
        .with_scheme("karakuri")
        .with_version(env!("CARGO_PKG_VERSION"))
        .with_subcommands("--guide", "--explain");
    engine.add_pages(include!(concat!(env!("OUT_DIR"), "/agent_man_pages.rs")));
    engine.add_page("docs/agent/ir-vocabulary.md", &vocabulary());
    engine.add_page("docs/agent/operations.md", &operations());
    engine
}

/// Builds the documentation engine loaded with documentation and registered tools for the given state.
pub(crate) fn doc_engine_with_state(
    state: std::sync::Arc<std::sync::Mutex<crate::State>>,
) -> DocEngine {
    let mut engine = doc_engine();
    for tool in crate::tools::build_tools(state) {
        let _ = engine.register_tool(tool);
    }
    engine
}

/// Renders built-in language functions, topologies, and outputs as markdown reference.
pub(crate) fn vocabulary() -> String {
    use karakuri_ir::builtin::Builtin;
    use karakuri_ir::{Blend, Output, Topology};
    let mut out = String::from(
        "---\n\
         id: ir-vocabulary\n\
         title: Built-in functions the checker accepts\n\
         summary: Every built-in with its signature, generated from the checker's own table rather than written down beside it.\n\
         ---\n\n\
         # Built-in functions\n\n\
         Generated from the checker's own table, so this is exactly what will be \
         accepted.\n\n\
         `Same` means the argument takes the shape of the others; `Scalar` is a single \
         float; `Exact(T)` is that type and no other. `domain` says whether a function \
         is defined on floats and vectors alike, on vectors only, or on one concrete \
         shape.\n\n\
         | name | arguments | returns | domain | must be constant |\n\
         |---|---|---|---|---|\n",
    );
    for builtin in Builtin::ALL {
        let signature = builtin.signature();
        let args = signature
            .args
            .iter()
            .map(|a| format!("{a:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        let constants = if signature.const_args.is_empty() {
            String::new()
        } else {
            format!("{:?}", signature.const_args)
        };
        out.push_str(&format!(
            "| `{}` | {args} | {:?} | {:?} | {constants} |\n",
            builtin.name(),
            signature.ret,
            signature.domain
        ));
    }

    out.push_str(
        "\n# Topologies\n\nDeclared by an L1's `topology`. What a *renderer* draws \
                  is not declared: an L4 draws segments when its `vertex` block assigns \
                  `clip_b` and sprites when it does not.\n\n",
    );
    for topology in [
        Topology::Points,
        Topology::Lines,
        Topology::Triangles,
        Topology::Grid,
        Topology::Ribbon,
        Topology::Fullscreen,
    ] {
        let note = match topology {
            Topology::Points => "one sprite per element",
            Topology::Lines => "one segment per element, `clip` to `clip_b`",
            Topology::Triangles => "vertex-shader-art triangle mesh, 3 elements per triangle (`Checked::is_static` sources only)",
            Topology::Grid => "2D tessellated quad grid mesh with automated index buffer (`Checked::is_static` sources only)",
            Topology::Ribbon => "connected quad strip ribbon for trails (`Checked::is_static` sources only)",
            Topology::Fullscreen => {
                "the whole frame, from an L4 with **no `vertex` block**. It must \
                 `consumes` nothing — there is no element to read from — and it gets `eye` \
                 and `ray` in `fragment`, which nothing else does"
            }
        };
        out.push_str(&format!("- `{}` — {note}\n", topology.name()));
    }

    // Between the two, because a blend is declared where a topology is not and
    // the contrast is the point: a model that has just read \"the renderer's
    // topology is inferred\" will assume the same of `blend` unless told.
    out.push_str(
        "\n# Blend modes\n\nDeclared by an L4's `blend`, and **declared rather than \
         inferred** — unlike the topology above. Nothing an L4 writes could imply one over \
         the other, because the two differ in how the results of identical assignments are \
         combined.\n\nThey read `color`'s alpha differently, which is the part that \
         changes how a procedure is written.\n\n",
    );
    for blend in [Blend::Additive, Blend::Weighted, Blend::Opaque] {
        let note = match blend {
            Blend::Additive => {
                "colour sums and nothing occludes. Alpha is **emission strength** and may \
                 exceed 1.0, scaling what the fragment adds"
            }
            // Named with the refusal it can meet, so that a model writing a
            // marcher does not reach for it and get a Set-build error it had no
            // way to predict.
            Blend::Weighted => {
                "order-independent transparency, so material **occludes** what is behind \
                 it. Alpha is **opacity** and is clamped to `[0, 1]`. Not available on a \
                 fullscreen L4: one fragment per texel makes it identical to `additive`, \
                 and building such a pair is refused"
            }
            Blend::Opaque => {
                "opaque rasterization with depth testing and depth writing against a per-Set \
                 Depth32 target. Alpha writes full coverage (1.0). Non-opaque renderers test depth \
                 and do not write, occluding behind opaque surfaces via early-Z"
            }
        };
        out.push_str(&format!("- `{}` — {note}\n", blend.name()));
    }

    out.push_str(
        "\n# Stage outputs\n\nAssigned like attributes; reading one is an error. \
                  `clip` and `point_rate` are required in a `vertex` block, and `color` in a \
                  `fragment` block, on every path through it — but **a `vertex` block is \
                  itself optional**, which is how an L4 says it draws the whole frame. \
                  `clip_b` is the one optional output, and assigning it on only some paths \
                  is rejected.\n\n**`point_rate` is a fraction of the render target's \
                  height, not a count of pixels.** A sprite at 0.005 is a two-hundredth of \
                  the frame's height however large the frame is, and it is square in \
                  pixels; under `lines` the same number is the stroke's width. Typical \
                  values are thousandths, and 1.0 fills the frame. A rate that works out \
                  below one pixel does not disappear: the primitive is drawn at one pixel \
                  and its colour multiplied by the coverage it lost, so a small target \
                  gets the same picture dimmer rather than a sparser one. Zero or less \
                  draws nothing.\n\n",
    );
    out.push_str("| name | type | block |\n|---|---|---|\n");
    for output in Output::ALL {
        out.push_str(&format!(
            "| `{}` | {} | `{}` |\n",
            output.name(),
            output.ty().name(),
            output.block().name()
        ));
    }
    out
}
