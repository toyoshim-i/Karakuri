// Order-preserving stream compaction: a multi-pass exclusive prefix sum over
// the alive flags, turning them into destination indices plus a new live
// count. See `compaction.rs` for why this exists and how these passes chain
// together across a capacity that can be much larger than one workgroup.
//
// `{{WG}}` is substituted at shader-module build time with
// `karakuri_codegen::layout::WORKGROUP_SIZE`, so the number 64 is spelled in
// exactly one place (the Rust constant) and not restated here.
//
// The scan is hierarchical, the standard reduce / scan-block-sums / add-back
// shape for a size that exceeds one workgroup, applied recursively so it
// covers any capacity:
//
//   - `scan_from_alive` (level 0, the "reduce" half): one workgroup per
//     64-element block of the alive-flags buffer. Computes each element's
//     exclusive prefix *within its own block* and reduces the block to a
//     single total, written to `block_sums_out`. Elements at or past
//     `indirect_in.live_count` — the previous frame's live range — are
//     treated as dead no matter what is stored there: that memory is stale
//     once compaction has shrunk the live range, which is why the scan
//     restricts itself to the previous live range and not the whole
//     capacity.
//   - `scan_inplace` (levels 1..N-1, same "reduce" shape applied to the
//     block sums): identical block scan, but over a plain `u32` array of
//     block sums from the level below, overwritten in place — once a raw
//     block sum has been folded into the local scan, nothing needs the raw
//     value again.
//   - This repeats until a level's block-sum output has exactly one entry:
//     at that point the level's local scan already covers its entire input
//     in one workgroup, so it needs no correction and no further recursion.
//     That single entry is the grand total — the new live count.
//   - `add_offsets` (the "add the offsets back" half): walked top-down,
//     after every level has been locally scanned bottom-up. Adds each
//     block's fully-resolved prefix (already correct, computed by the level
//     above) into that block's locally-scanned values.
//   - `finalize`: turns the top-level total into
//     `dispatch_workgroups_indirect` arguments plus the raw count, so
//     `element` can dispatch over the new live range without a readback.

const WG: u32 = {{WG}}u;

// Four plain `u32` fields rather than `len: u32, _pad: vec3<u32>`: a
// `vec3<u32>` field inside a uniform-buffer struct is rounded up to 16-byte
// alignment by WGSL's host-shareable layout rules, which would silently
// stop matching the Rust `repr(C)` struct that fills this buffer. Four
// scalars have no such rule — each is 4-byte aligned, so the struct is
// exactly 16 bytes with no implicit padding to get wrong.
struct LevelLen {
    len: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

struct IndirectArgs {
    workgroups_x: u32,
    workgroups_y: u32,
    workgroups_z: u32,
    live_count: u32,
}

var<workgroup> temp: array<u32, WG>;

// Exclusive scan of `v` (this invocation's value) across the workgroup,
// Hillis-Steele with an explicit read / barrier / write / barrier per step
// so every thread's read of a neighbor happens before any thread's write
// can clobber it. `temp[WG - 1u]` holds the block's total (the inclusive
// sum of everything in the block) when this returns, which callers use as
// the block sum.
fn block_exclusive_scan(v: u32, li: u32) -> u32 {
    temp[li] = v;
    workgroupBarrier();
    for (var offset: u32 = 1u; offset < WG; offset = offset * 2u) {
        var t: u32 = 0u;
        if li >= offset {
            t = temp[li - offset];
        }
        workgroupBarrier();
        temp[li] = temp[li] + t;
        workgroupBarrier();
    }
    let inclusive = temp[li];
    // temp[WG - 1u] is read by the caller after this returns; nothing
    // further writes to `temp` this dispatch, so no barrier is needed here.
    return inclusive - v;
}

// --- level 0: reads the alive-flags attribute buffer directly ---

@group(0) @binding(0) var<storage, read> alive_in: array<vec4<u32>>;
@group(0) @binding(1) var<storage, read_write> dest_out: array<u32>;
@group(0) @binding(2) var<storage, read_write> block_sums_l0: array<u32>;
@group(0) @binding(3) var<uniform> level_len_l0: LevelLen;
@group(0) @binding(4) var<storage, read> indirect_in: IndirectArgs;

@compute @workgroup_size(WG)
fn scan_from_alive(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wgid: vec3<u32>,
) {
    let i = gid.x;
    let li = lid.x;
    var v: u32 = 0u;
    // In range of this level, and in range of the previous live range —
    // not the whole capacity. Anything at or past `live_count` is stale
    // once compaction has shrunk the range, regardless of what bit pattern
    // sits there.
    if i < level_len_l0.len && i < indirect_in.live_count {
        v = alive_in[i].x;
    }
    let ex = block_exclusive_scan(v, li);
    if i < level_len_l0.len {
        dest_out[i] = ex;
    }
    if li == WG - 1u {
        block_sums_l0[wgid.x] = temp[li];
    }
}

// --- levels 1..N-1: the same block scan, over block sums, in place ---

@group(0) @binding(5) var<storage, read_write> data_inplace: array<u32>;
@group(0) @binding(6) var<storage, read_write> block_sums_inplace: array<u32>;
@group(0) @binding(7) var<uniform> level_len_inplace: LevelLen;

@compute @workgroup_size(WG)
fn scan_inplace(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wgid: vec3<u32>,
) {
    let i = gid.x;
    let li = lid.x;
    var v: u32 = 0u;
    if i < level_len_inplace.len {
        v = data_inplace[i];
    }
    let ex = block_exclusive_scan(v, li);
    if i < level_len_inplace.len {
        data_inplace[i] = ex;
    }
    if li == WG - 1u {
        block_sums_inplace[wgid.x] = temp[li];
    }
}

// --- add the offsets back, one level at a time, top-down ---

@group(0) @binding(8) var<storage, read_write> add_local: array<u32>;
@group(0) @binding(9) var<storage, read> add_offsets_in: array<u32>;
@group(0) @binding(10) var<uniform> level_len_add: LevelLen;

@compute @workgroup_size(WG)
fn add_offsets(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(workgroup_id) wgid: vec3<u32>,
) {
    let i = gid.x;
    if i < level_len_add.len {
        add_local[i] = add_local[i] + add_offsets_in[wgid.x];
    }
}

// --- finalize: grand total -> indirect dispatch args + raw live count ---

@group(0) @binding(11) var<storage, read> total_in: array<u32>;
@group(0) @binding(12) var<storage, read_write> indirect_out: IndirectArgs;

@compute @workgroup_size(WG)
fn finalize(@builtin(global_invocation_id) gid: vec3<u32>) {
    if gid.x == 0u {
        let total = total_in[0];
        indirect_out.workgroups_x = (total + (WG - 1u)) / WG;
        indirect_out.workgroups_y = 1u;
        indirect_out.workgroups_z = 1u;
        indirect_out.live_count = total;
    }
}
