// -- resources -------------------------------------------------------------

use super::*;
use serde_json::{json, Value};

/// The one path this server serves. Everything else is a 404, which is what
/// tells a client probing for authorization metadata that there is none.
pub(crate) const ENDPOINT: &str = "/";

pub(crate) const SPEC: &str = "karakuri://ir-spec";
pub(crate) const VOCABULARY: &str = "karakuri://ir-vocabulary";
pub(crate) const OPERATIONS: &str = "karakuri://operations";

pub(crate) fn resources() -> Value {
    json!([
        {
            "uri": SPEC,
            "name": "The IR specification",
            "description":
                "The language a procedure is written in, in full, with the reasoning. \
                 Read this before writing a procedure for the first time.",
            "mimeType": "text/markdown",
        },
        {
            "uri": VOCABULARY,
            "name": "Built-in functions the checker accepts",
            "description":
                "Every built-in with its signature, generated from the checker's own \
                 table rather than written down beside it. Prose drifts from code; this \
                 cannot, because the same list is what rejects a procedure.",
            "mimeType": "text/markdown",
        },
        {
            "uri": OPERATIONS,
            "name": "Operations `operate` takes",
            "description":
                "Every operation this instrument can be asked for by name, with the shape \
                 of each payload and one call that names it — generated from the same \
                 table the tool accepts against. Read this before calling `operate`.",
            "mimeType": "text/markdown",
        },
    ])
}

pub(crate) fn read_resource(request: &Value) -> Result<Value, String> {
    let uri = request
        .get("params")
        .and_then(|p| p.get("uri"))
        .and_then(Value::as_str)
        .ok_or("no uri")?;
    let text = match uri {
        SPEC => include_str!("../../../docs/ir-spec.md").to_string(),
        VOCABULARY => vocabulary(),
        OPERATIONS => operations(),
        other => return Err(format!("no resource `{other}`")),
    };
    Ok(json!({
        "contents": [{ "uri": uri, "mimeType": "text/markdown", "text": text }],
    }))
}

/// The built-ins, rendered from [`karakuri_ir::builtin::Builtin::ALL`].
///
/// **Generated, and that is the whole point.** `docs/ir-spec.md` describes this
/// language in prose and prose goes stale; this list is the one the checker
/// matches against, so it cannot say a function exists that does not, or miss
/// one that does.
pub(crate) fn vocabulary() -> String {
    use karakuri_ir::builtin::Builtin;
    use karakuri_ir::{Blend, Output, Topology};
    let mut out = String::from(
        "# Built-in functions\n\n\
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

    // Generated for the same reason the table above is: these are closed
    // vocabularies in the checker, so a hand-written list here could name a
    // topology or an output that does not exist. What cannot be generated is
    // which outputs are *required* — that is a rule in the check pass rather
    // than a property of the enum — so the prose says it and the spec resource
    // carries the detail.
    out.push_str(
        "\n# Topologies\n\nDeclared by an L1's `topology`. What a *renderer* draws \
                  is not declared: an L4 draws segments when its `vertex` block assigns \
                  `clip_b` and sprites when it does not.\n\n",
    );
    for topology in [Topology::Points, Topology::Lines, Topology::Fullscreen] {
        let note = match topology {
            Topology::Points => "one sprite per element",
            Topology::Lines => "one segment per element, `clip` to `clip_b`",
            // Listed with the rule it brings rather than only with what it
            // draws, because `consumes` is checked rather than merely expected
            // and a model that did not know would meet the refusal after
            // writing the file. An earlier version of this arm said the engine
            // could not run one at all, and stayed there after it could.
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
    for blend in [Blend::Additive, Blend::Weighted] {
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
