//! L5 lowering: `frame` as one fullscreen fragment pass over `src`.
//!
//! # It is the fullscreen L4 with the camera taken out
//!
//! Almost every sentence here is that one read out loud. The vertex stage is
//! generated rather than lowered, because the procedure has no `vertex` block
//! and no per-element form for one to be a declaration against; `point_coord`
//! comes out 0..1 across the frame, which is what it already means everywhere
//! else; and `color` is the one output, `vec4`, linear and unclamped, because
//! it is the same value at the next node down.
//!
//! What it does *not* share is the ray. A fullscreen L4 opens with a ray
//! prologue and binds the camera unconditionally; an L5 binds no camera at all,
//! because the frame in front of it may hold several decks' material seen from
//! several cameras and there is no one viewpoint for it to name. `eye`, `ray`
//! and `camera` are refused in the check pass, so nothing here has to answer
//! for them.
//!
//! # The bind group is `master.wgsl`'s, entry for entry
//!
//! ```text
//! @group(0) @binding(0)  var<uniform> u: Uniforms
//! @group(0) @binding(1)  var src:  texture_2d<f32>
//! @group(0) @binding(2)  var held: texture_2d<f32>   // only under `retains`
//! @group(0) @binding(3)  var samp: sampler
//! @group(0) @binding(4…) var tex_<slot>: texture_2d<f32>
//! ```
//!
//! **The first four are the hand-written chain's own layout**, in its own order
//! — `chain`, `src`, `aux`, `samp` — so that the engine's chain binds a written
//! pass with the layout it already has rather than with a second one invented
//! here. `held` is what `master.rs` routes into `aux`, which is the same
//! binding under the name the language gives it.
//!
//! **Binding 2 is left empty rather than renumbered** where the procedure
//! declares no `retains`. A hole is legal — wgpu accepts a pipeline layout
//! naming a group entry the module does not use — and it is what keeps every
//! slot's layout the same shape whatever is in it: renumbering would make the
//! sampler's binding depend on whether a pass reads its own history, which is a
//! fact about the picture leaking into the plumbing.
//!
//! **The Texture slots start after the sampler**, in header order, so a nested
//! merge's inputs are numbered by what the file declared and not by what an
//! edge happened to bind first.
//!
//! # What is not here, and is pass 2's
//!
//! The chain itself: which cut `held` holds, how a slot's output becomes the
//! next slot's `src`, and where the retention is copied from. A procedure
//! declares `retains` and says nothing about which cut — that is the slot's
//! answer, `mix` or `exit` — and **the retained frame is sanitised where it is
//! written rather than where it is read**, so nothing in this module guards
//! against a non-finite texel. That is `master.wgsl`'s `keepable` moved one
//! step earlier, and it is what makes it something a `.kir` neither needs nor
//! can leave out.

use karakuri_ir::typed::{Checked, TStmt, Target, TexRef};
use karakuri_ir::{Ambient, Attr, BlockKind, Kind, Output};

use crate::layout::{self, UniformLayout, UniformLayoutBuilder};
use crate::lower::{lower_expr, mangle_local, Resolver};
use crate::prelude::{self, Requirements};
use crate::ty::wgsl_ty;

/// The uniform, at [`crate::layout::group::UNIFORMS`]'s binding 0 like every
/// other module's.
const BINDING_UNIFORM: u32 = 0;
/// The incoming picture, implicit and always bound.
const BINDING_SRC: u32 = 1;
/// The retained cut, bound only where the procedure declares `retains` — and
/// the number is held open either way. See the module doc.
const BINDING_HELD: u32 = 2;
/// The filtering sampler `tap` reads through.
const BINDING_SAMPLER: u32 = 3;
/// The first `uses … : Texture` slot; the rest follow in header order.
const BINDING_SLOT_BASE: u32 = 4;

/// The WGSL name the incoming picture is declared under.
///
/// The same word the language uses, which is safe for the reason every fixed
/// name this crate emits is: a procedure's own locals all carry
/// [`mangle_local`]'s `usr_` prefix, so no `.kir` can spell this identifier.
const SRC: &str = "src";
const HELD: &str = "held";
const SAMPLER: &str = "samp";

/// This fragment's own texel as an integer coordinate — bound once at the top
/// of the entry point, so a body with several `texel` calls computes it once.
const TEXEL_AT: &str = "_at";

/// The WGSL binding name for one declared Texture slot.
///
/// Prefixed for [`mangle_local`]'s reason and because the slot's name is
/// `.kir` text: `uses src : Texture` is refused by the check pass, but `uses
/// samp : Texture` is not, and a slot binding that could collide with the
/// sampler would be a shader that compiles and fetches from the wrong thing.
pub fn slot_binding(slot: &str) -> String {
    format!("tex_{slot}")
}

/// Generated L5 WGSL plus what the engine needs to bind it.
#[derive(Debug, Clone)]
pub struct L5Shader {
    pub source: String,
    pub uniform_layout: UniformLayout,
    pub uniform_pad_f32: u32,
    /// **Whether this pass reads a retained frame**, and so whether
    /// [`BINDING_HELD`] is declared at all.
    ///
    /// Echoed off the procedure rather than recomputed, because it is what
    /// decides a cost nobody would pay otherwise: the engine allocates a
    /// retention only where a slot's answer names one, and this is the half of
    /// that answer the file gives.
    pub retains: bool,
    /// The Texture slots this pass folds in, in header order — the same order
    /// their bindings are numbered in, so an `edge` resolves to a binding by
    /// position rather than by a second table.
    pub texture_slots: Vec<String>,
}

impl L5Shader {
    /// Which binding one declared slot is read at. Header order, after the
    /// sampler.
    pub fn slot_binding_number(&self, slot: &str) -> Option<u32> {
        self.texture_slots
            .iter()
            .position(|s| s == slot)
            .map(|at| BINDING_SLOT_BASE + at as u32)
    }
}

/// `present.wgsl`'s and `master.wgsl`'s triangle, term for term: one oversized
/// triangle rather than two, no seam and no vertex buffer.
///
/// **`clip` is carried as `@builtin(position)`**, which is what makes `texel`
/// an unfiltered read: `vec2<i32>(in.clip.xy)` is this fragment's own texel and
/// nothing else's, so a centre tap cannot drift by a filter.
const VS: &str = "struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) point_coord: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    // (0,0), (2,0), (0,2) in `point_coord`, so the frame is the 0..1 corner.
    let uv = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var out: VsOut;
    out.clip = vec4<f32>(uv * 2.0 - 1.0, 0.0, 1.0);
    // Flipped in y: NDC runs up and a framebuffer runs down, and the whole
    // point of this value is that a fragment can say where on screen it is.
    out.point_coord = vec2<f32>(uv.x, 1.0 - uv.y);
    return out;
}

";

/// Lower one checked `kind L5` procedure to a fullscreen fragment pass.
pub fn generate_l5(checked: &Checked) -> L5Shader {
    assert_eq!(
        checked.kind,
        Kind::L5,
        "generate_l5 called on a non-L5 procedure"
    );

    let frame = checked
        .block(BlockKind::Frame)
        .expect("an L5 procedure must have a frame block");

    let slots: Vec<String> = checked
        .texture_slots()
        .into_iter()
        .map(str::to_string)
        .collect();

    let mut b = UniformLayoutBuilder::new();
    b.field("t", "f32");
    b.field("beats", "f32");
    // **`dt` is here and is not on an L4**, which is the asymmetry the ambient
    // table already draws: a frame effect that moves at a rate rather than to a
    // position is written against a step, and the step comes from a record
    // rather than from a clock.
    b.field("dt", "f32");
    // **Carried though an L5 reads no `source`.** Every seeded builtin bottoms
    // out in `hash1`, which reads `u.seed_salt` straight from the module-scope
    // uniform — so a `frame` block that calls `hash1(uint(...))` for dither or
    // grain needs the field in scope, exactly as every other module does. What
    // is written into it is the chain's answer and not this crate's.
    b.field("seed_salt", "u32");
    // **The frame's own size, and the only thing that reads it is
    // `frame_step`.** No ambient carries it and none will: the render size is
    // not part of the picture, and one frame is rendered at the largest enabled
    // output's size and scaled into the rest — so a `.kir` that wanted a radius
    // in texels would be writing a number that means a different distance on a
    // second display. The conversion is performed without the number being
    // handed over.
    b.field("viewport", "vec2<f32>");
    for p in &checked.params {
        b.param_field(p.name.clone(), wgsl_ty(p.ty));
    }
    let (uniform_layout, uniform_pad_f32) = b.finish();

    let mut req = Requirements::default();
    let resolver = L5Resolver;
    let body = {
        let mut out = String::new();
        emit_stmts(&frame.stmts, &resolver, &mut req, 1, &mut out);
        out
    };

    let mut src = String::new();
    layout::write_uniform_struct(&mut src, &uniform_layout, uniform_pad_f32);
    src.push_str(&format!(
        "\n@group(0) @binding({BINDING_UNIFORM}) var<uniform> u: Uniforms;\n"
    ));
    src.push_str(&format!(
        "@group(0) @binding({BINDING_SRC}) var {SRC}: texture_2d<f32>;\n"
    ));
    if checked.retains {
        src.push_str(&format!(
            "@group(0) @binding({BINDING_HELD}) var {HELD}: texture_2d<f32>;\n"
        ));
    }
    src.push_str(&format!(
        "@group(0) @binding({BINDING_SAMPLER}) var {SAMPLER}: sampler;\n"
    ));
    for (at, slot) in slots.iter().enumerate() {
        src.push_str(&format!(
            "@group(0) @binding({}) var {}: texture_2d<f32>;\n",
            BINDING_SLOT_BASE + at as u32,
            slot_binding(slot)
        ));
    }
    src.push('\n');
    src.push_str(&prelude::render(&req));
    src.push('\n');
    src.push_str(VS);
    src.push_str("@fragment\nfn fs(in: VsOut) -> @location(0) vec4<f32> {\n");
    src.push_str("    let point_coord = in.point_coord;\n");
    src.push_str(&format!("    let {TEXEL_AT} = vec2<i32>(in.clip.xy);\n"));
    // `var` rather than `let`, because the block may assign `color` on several
    // paths — the coverage check requires every path, not one.
    src.push_str("    var _color: vec4<f32>;\n");
    src.push_str(&body);
    src.push_str("    return _color;\n}\n");

    L5Shader {
        source: src,
        uniform_layout,
        uniform_pad_f32,
        retains: checked.retains,
        texture_slots: slots,
    }
}

/// Names resolve to the uniform, to the entry point's two locals, and to the
/// texture bindings. There is nothing else in scope: no element buffer, no
/// varying but the screen position, and no camera.
struct L5Resolver;

impl Resolver for L5Resolver {
    fn read_attr(&self, attr: Attr) -> String {
        unreachable!(
            "`{}` is refused in a frame block: an L5 is handed a picture, not elements",
            attr.name()
        )
    }

    fn read_seed(&self) -> String {
        unreachable!("`seed` is per element and a frame pass has none")
    }

    fn read_ambient(&self, amb: Ambient) -> String {
        match amb {
            Ambient::T => "u.t".to_string(),
            Ambient::Beats => "u.beats".to_string(),
            Ambient::Dt => "u.dt".to_string(),
            Ambient::PointCoord => "point_coord".to_string(),
            other => unreachable!("{other:?} is not available in a checked frame block"),
        }
    }

    fn read_texture(&self, tex: &TexRef) -> String {
        match tex {
            TexRef::Src => SRC.to_string(),
            TexRef::Held => HELD.to_string(),
            TexRef::Slot(name) => slot_binding(name.as_str()),
        }
    }

    fn texel_at(&self) -> String {
        TEXEL_AT.to_string()
    }

    fn sampler(&self) -> String {
        SAMPLER.to_string()
    }
}

fn emit_stmts(
    stmts: &[TStmt],
    resolver: &L5Resolver,
    req: &mut Requirements,
    indent: usize,
    out: &mut String,
) {
    let pad = "    ".repeat(indent);
    for stmt in stmts {
        match stmt {
            TStmt::Let { name, value, .. } => {
                let v = lower_expr(value, resolver, req);
                out.push_str(&format!("{pad}let {} = {v};\n", mangle_local(name)));
            }
            TStmt::Var { name, value, .. } => {
                let v = lower_expr(value, resolver, req);
                out.push_str(&format!("{pad}var {} = {v};\n", mangle_local(name)));
            }
            TStmt::Assign { target, value, .. } => {
                let v = lower_expr(value, resolver, req);
                match target {
                    Target::Local(name) => {
                        out.push_str(&format!("{pad}{} = {v};\n", mangle_local(name)))
                    }
                    Target::Output(Output::Color) => {
                        out.push_str(&format!("{pad}_color = {v};\n"));
                    }
                    other => unreachable!("an L5 never assigns {other:?}"),
                }
            }
            TStmt::If {
                cond, then, els, ..
            } => {
                let c = lower_expr(cond, resolver, req);
                out.push_str(&format!("{pad}if {c} {{\n"));
                emit_stmts(then, resolver, req, indent + 1, out);
                if els.is_empty() {
                    out.push_str(&format!("{pad}}}\n"));
                } else {
                    out.push_str(&format!("{pad}}} else {{\n"));
                    emit_stmts(els, resolver, req, indent + 1, out);
                    out.push_str(&format!("{pad}}}\n"));
                }
            }
            TStmt::For {
                var,
                start,
                end,
                body,
                ..
            } => {
                let v = mangle_local(var);
                out.push_str(&format!(
                    "{pad}for (var {v}: i32 = {start}; {v} < {end}; {v} = {v} + 1) {{\n"
                ));
                emit_stmts(body, resolver, req, indent + 1, out);
                out.push_str(&format!("{pad}}}\n"));
            }
            TStmt::Kill { .. } => {
                unreachable!("`kill()` in a frame block is refused by the checker")
            }
        }
    }
}
