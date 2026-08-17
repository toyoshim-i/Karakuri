// Derive an amplifying L2's `Counts` from the one its input came with.
//
// **Everything downstream of an amplifier reads a longer buffer, and this is
// where the length is stated.** A `Counts` carries three things at once — the
// workgroup count a compute pass dispatches over, the live range a pass bounds
// itself by, and the instance count a renderer draws — and all three multiply
// by the same factor, because every entry of the input becomes exactly `factor`
// entries of the output.
//
// One invocation, once a frame, whatever the capacity: there is nothing here
// that scales with anything. The same shape as `camera.wgsl`, and for the same
// reason — a value the GPU has to compute because the number it is computed
// from lives in a buffer the host never reads back.
//
// The three placeholders below are filled by `node::Deform::build` — the counts
// struct verbatim from `karakuri_codegen::layout::counts::WGSL`, the factor from
// the declaration, the workgroup size from the same constant the generated
// shaders use — so this file cannot disagree with any of them.
//
// **Their names are not spelled anywhere in this comment**, and that is not
// fussiness: substitution is textual and does not know what a comment is, so a
// placeholder named in prose is replaced by a multi-line struct of which only
// the first line stays commented out.

{{COUNTS_STRUCT}}

@group(0) @binding(0) var<storage, read> src: Counts;
@group(0) @binding(1) var<storage, read_write> dst: Counts;

@compute @workgroup_size(1)
fn derive() {
    let factor = {{FACTOR}}u;
    let range = src.range * factor;

    dst.range = range;
    dst.elem_x = (range + ({{WG}}u - 1u)) / {{WG}}u;
    dst.elem_y = 1u;
    dst.elem_z = 1u;

    // **`vertex_count` is not multiplied and the other three are.** It is the
    // corners of one primitive, which is a property of how a renderer expands
    // an element and has nothing to do with how many elements there are.
    dst.vertex_count = src.vertex_count;
    dst.instance_count = src.instance_count * factor;
    dst.first_vertex = src.first_vertex;
    dst.first_instance = src.first_instance;

    // Carried rather than dropped: nothing downstream of an amplifier scans or
    // spawns, so no pass reads this — but a `Counts` that lied about one of its
    // fields would be a buffer whose meaning depends on where it came from.
    dst.survivors = src.survivors * factor;
}
