//! Compiles L3 `camera` procedures into a compute entry point writing `CameraState`,
//! plus a `reduce` entry point when the camera reads a reduction of its geometry.

use karakuri_ir::layout::ElementLayout;
use karakuri_ir::typed::{Checked, Reduction, TStmt, Target};
use karakuri_ir::{Ambient, Attr, BlockKind, Kind, Output};

use crate::layout::{self, binding, group, UniformLayout, UniformLayoutBuilder};
use crate::lower::{lower_expr, mangle_local, Resolver};
use crate::prelude::{self, Requirements};
use crate::ty::wgsl_ty;

#[derive(Debug, Clone)]
pub struct L3Shader {
    pub source: String,
    pub uniform_layout: UniformLayout,
    /// Trailing `f32` pad slots after [`L3Shader::uniform_layout`]'s named
    /// fields — wire padding only, never a value the engine sets.
    pub uniform_pad_f32: u32,
    /// True when the module has a [`REDUCE`] entry point, which the engine
    /// dispatches before [`ENTRY`].
    pub reduces: bool,
}

/// The entry point an engine dispatches. Not `camera`, which the generated
/// module also uses as a binding name.
pub const ENTRY: &str = "produce";

/// The reduction entry point, one workgroup of [`REDUCE_WORKGROUP`] threads.
pub const REDUCE: &str = "reduce";

/// Threads in the single `reduce` workgroup.
pub const REDUCE_WORKGROUP: u32 = 256;

/// Generate the compute shader for one L3.
///
/// `subject` is the element layout of the source bound to the L3's geometry
/// slot, and is `Some` exactly when the L3 declares one.
pub fn generate_l3(
    checked: &Checked,
    fields: crate::Bound<'_>,
    subject: Option<&ElementLayout>,
) -> L3Shader {
    assert_eq!(
        checked.kind,
        Kind::L3,
        "generate_l3 called on a non-L3 procedure"
    );

    let mut b = UniformLayoutBuilder::new();
    b.field("t", "f32");
    b.field("beats", "f32");
    b.field("dt", "f32");
    b.field("seed_salt", "u32");
    for p in &checked.params {
        b.param_field(p.name.clone(), wgsl_ty(p.ty));
    }
    // Spliced field parameters, prefixed with the slot name to avoid naming collisions.
    let splices = crate::splices(checked, fields);
    for f in &splices {
        for (name, ty) in &f.params {
            b.field_param_field(&f.slot, name, ty);
        }
    }
    let (uniform_layout, uniform_pad_f32) = b.finish();

    debug_assert_eq!(
        checked.geometry_slot().is_some(),
        subject.is_some(),
        "`{}` declares a geometry slot and was handed no geometry for it, or the reverse",
        checked.name
    );
    let resolver = L3Resolver { subject };
    let mut req = Requirements::default();
    let block = checked
        .block(BlockKind::Camera)
        .expect("an L3 procedure must have a camera block");
    let body = {
        let mut out = String::new();
        emit_stmts(&block.stmts, &resolver, &mut req, 1, &mut out);
        out
    };
    let reduces = checked.reads_reduction();

    let mut src = String::new();
    layout::write_uniform_struct(&mut src, &uniform_layout, uniform_pad_f32);
    src.push_str(&format!(
        "\n@group({}) @binding({}) var<uniform> u: Uniforms;\n\n",
        group::UNIFORMS,
        binding::UNIFORM,
    ));
    src.push_str(layout::camera::STATE_WGSL);
    src.push_str(&format!(
        "\n@group({}) @binding({}) var<storage, read_write> cam: CameraState;\n\n",
        group::STATE,
        binding::UNIFORM,
    ));
    if let Some(layout) = subject {
        write_subject_bindings(&mut src, layout);
    }
    // Absorb prelude requirements and functions needed by spliced fields.
    for f in &splices {
        req.absorb(&f.requirements);
    }
    src.push_str(&prelude::render(&req));
    for f in &splices {
        src.push('\n');
        src.push_str(&f.source);
    }
    src.push('\n');
    if reduces {
        src.push_str(REDUCE_WGSL);
        src.push('\n');
    }
    src.push_str(&entry(&body, subject.is_some()));

    L3Shader {
        source: src,
        uniform_layout,
        uniform_pad_f32,
        reduces,
    }
}

/// Declares the subject's element, alive, counts and reduction buffers in
/// [`group::SUBJECT`]. The identifiers are fixed and prefixed so no user
/// symbol, which is mangled with `usr_`, can collide with them.
fn write_subject_bindings(src: &mut String, layout: &ElementLayout) {
    layout::write_element_struct_named(src, "SubjectElement", layout);
    src.push_str(layout::counts::WGSL);
    src.push_str(layout::reduced::WGSL);
    src.push_str(&format!(
        "\n@group({g}) @binding({}) var<storage, read> subject: array<SubjectElement>;\n\
         @group({g}) @binding({}) var<storage, read> subject_alive: array<u32>;\n\
         @group({g}) @binding({}) var<storage, read> subject_counts: Counts;\n\
         @group({g}) @binding({}) var<storage, read_write> subject_reduced: Reduced;\n\n",
        binding::ELEMENT,
        binding::ALIVE,
        binding::SUBJECT_COUNTS,
        binding::REDUCED,
        g = group::SUBJECT,
    ));
}

/// One workgroup strides over the live range, skipping elements killed this
/// step, and writes the mean, min and max `position` to `subject_reduced`.
/// With nothing alive it writes nothing, so the previous result stands.
const REDUCE_WGSL: &str = "\
var<workgroup> red_sum: array<vec3<f32>, 256>;
var<workgroup> red_min: array<vec3<f32>, 256>;
var<workgroup> red_max: array<vec3<f32>, 256>;
var<workgroup> red_n: array<u32, 256>;

@compute @workgroup_size(256)
fn reduce(@builtin(local_invocation_index) li: u32) {
    let range = subject_counts.range;
    var sum = vec3<f32>(0.0);
    var lo = vec3<f32>(3.0e38);
    var hi = vec3<f32>(-3.0e38);
    var n = 0u;
    for (var i = li; i < range; i = i + 256u) {
        if subject_alive[i] != 0u {
            let p = subject[i].position;
            sum = sum + p;
            lo = min(lo, p);
            hi = max(hi, p);
            n = n + 1u;
        }
    }
    red_sum[li] = sum;
    red_min[li] = lo;
    red_max[li] = hi;
    red_n[li] = n;
    workgroupBarrier();
    for (var s = 128u; s > 0u; s = s >> 1u) {
        if li < s {
            red_sum[li] = red_sum[li] + red_sum[li + s];
            red_min[li] = min(red_min[li], red_min[li + s]);
            red_max[li] = max(red_max[li], red_max[li + s]);
            red_n[li] = red_n[li] + red_n[li + s];
        }
        workgroupBarrier();
    }
    if li == 0u && red_n[0] > 0u {
        subject_reduced.centroid = red_sum[0] / f32(red_n[0]);
        subject_reduced.bounds_min = red_min[0];
        subject_reduced.bounds_max = red_max[0];
    }
}
";

/// Returns the local variable name used to accumulate each camera output before writing to `CameraState`.
fn output_local(o: Output) -> &'static str {
    match o {
        Output::Eye => "_eye",
        Output::Target => "_target",
        Output::Up => "_up",
        Output::FovY => "_fov_y",
        Output::Near => "_near",
        Output::Far => "_far",
        other => unreachable!(
            "{other:?} is not a camera output and cannot appear in a checked camera block"
        ),
    }
}

struct L3Resolver<'a> {
    subject: Option<&'a ElementLayout>,
}

impl Resolver for L3Resolver<'_> {
    fn read_element(&self, attr: Attr, index: &str) -> String {
        let layout = self
            .subject
            .expect("an element read is refused where no geometry slot is declared");
        // `_subject_last` is bound in the entry prologue; an index past the
        // live range reads the youngest living element.
        let at = format!("subject[min({index}, _subject_last)]");
        if layout.derived.contains(&attr) {
            match attr.derivation() {
                Some(karakuri_ir::Derivation::SinceBirth) => format!("(u.t - {at}.birth_t)"),
                other => unreachable!("{other:?} is not synthesised at the read site"),
            }
        } else {
            format!("{at}.{}", attr.name())
        }
    }

    fn read_reduction(&self, r: Reduction) -> String {
        format!("subject_reduced.{}", r.name())
    }

    fn read_attr(&self, attr: karakuri_ir::Attr) -> String {
        unreachable!(
            "`{}` cannot appear in a checked camera block: an L3 has no element",
            attr.name()
        )
    }

    fn read_seed(&self) -> String {
        unreachable!("`seed` is per element and an L3 has none")
    }

    fn read_ambient(&self, amb: Ambient) -> String {
        match amb {
            Ambient::T => "u.t".to_string(),
            Ambient::Beats => "u.beats".to_string(),
            // The fixed step, unscaled. There is no birth fraction to correct
            // for here — that is an L1's first-update adjustment, and a camera
            // is never new.
            Ambient::Dt => "u.dt".to_string(),
            Ambient::Seed => unreachable!("read_seed handles this"),
            // An L3 runs once a frame over no geometry, so there is no chain
            // instance for `source` to name — refused in the checker, beside
            // `seed` and for the same reason.
            Ambient::Source
            | Ambient::Point
            | Ambient::Copy
            | Ambient::Capacity
            | Ambient::Camera
            | Ambient::PointCoord
            | Ambient::Eye
            | Ambient::Ray => {
                unreachable!("{amb:?} is not available in a checked camera block")
            }
        }
    }
}

fn emit_stmts(
    stmts: &[TStmt],
    resolver: &L3Resolver<'_>,
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
                    Target::Output(o) => {
                        out.push_str(&format!("{pad}{} = {v};\n", output_local(*o)));
                    }
                    Target::Attr(a) => {
                        unreachable!("a camera block cannot assign `{}`", a.name())
                    }
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
                unreachable!("`kill()` in a camera block is refused by the checker")
            }
        }
    }
}

/// `Orbit::default`'s field of view and planes, so that replacing the built-in
/// camera with the simplest L3 anyone would write does not change the picture
/// underneath it. A third of pi, and 0.1 to 100.
const DEFAULTS: &str = "\
    var _eye    = vec3<f32>(0.0, 0.0, 0.0);
    var _target = vec3<f32>(0.0, 0.0, 0.0);
    var _up     = vec3<f32>(0.0, 1.0, 0.0);
    var _fov_y  = 1.0471976;
    var _near   = 0.1;
    var _far    = 100.0;
";

/// With a geometry slot, the camera block does not run while its source has
/// no live range, and the state keeps the previous frame's camera.
const SUBJECT_PROLOGUE: &str = "\
    if subject_counts.range == 0u {
        return;
    }
    let _subject_last = subject_counts.range - 1u;
";

fn entry(body: &str, has_subject: bool) -> String {
    let prologue = if has_subject { SUBJECT_PROLOGUE } else { "" };
    format!(
        "@compute @workgroup_size(1)\n\
         fn {ENTRY}() {{\n\
         {prologue}\
         {DEFAULTS}\n\
         {body}\n\
         \x20   cam.eye     = _eye;\n\
         \x20   // `target` is a reserved word in WGSL and nowhere else — see\n\
         \x20   // `layout::camera::STATE_WGSL`.\n\
         \x20   cam.look_at = _target;\n\
         \x20   cam.up      = _up;\n\
         \x20   cam.fov_y   = _fov_y;\n\
         \x20   cam.near    = _near;\n\
         \x20   cam.far     = _far;\n\
         }}\n"
    )
}
