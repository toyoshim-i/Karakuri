//! Order-preserving stream compaction.
//!
//! `element` dispatches indirectly over the live count, which requires the live
//! set to be contiguous. That contiguity, not determinism, is the primary
//! reason this exists: a free list leaves live elements scattered, and
//! dispatching over the live count then needs a separately maintained compact
//! list of live slots — which costs a scan anyway.
//!
//! The compaction is **order preserving**. Survivors keep their relative order,
//! so the live set is always a stable subsequence. That costs a scan per frame
//! and buys a stable blend order: floating-point addition is not associative,
//! so bit-exact reproduction depends on elements being combined in the same
//! order on every run, under additive blending as much as under anything else.
//! Atomic allocation would be cheaper and forfeits exactly that.
//!
//! # Algorithm
//!
//! A single workgroup cannot scan `capacity` elements — v0.2 allows up to
//! 1,048,576 — so this is the standard hierarchical scan, applied
//! recursively so its depth is `ceil(log_WORKGROUP_SIZE(capacity))` rather
//! than a fixed number of levels:
//!
//! 1. **Reduce.** Each 64-wide block computes its own exclusive prefix sum
//!    (`scan_from_alive` for the alive-flags buffer itself, `scan_inplace`
//!    for every level above it) and reduces to a single block sum.
//! 2. **Scan the block sums.** The block sums from step 1 are themselves
//!    scanned the same way, recursively, until a level's block-sum output
//!    has exactly one entry — at that point one workgroup covers the whole
//!    level, so its local scan is already globally correct and the
//!    recursion bottoms out. That one entry is the new live count.
//! 3. **Add the offsets back.** Walking back down, each level's local scan
//!    gets the level above's now-fully-resolved block offset added in
//!    (`add_offsets`), one pass per level.
//!
//! `shaders/scan.wgsl` has the pass-by-pass detail. See [`Compaction::new`]
//! for how the level pyramid is sized and [`Compaction::record`] for how the
//! passes chain within one compute pass.
//!
//! # Previous live range, not whole capacity
//!
//! The scan restricts itself to the *previous* live range: entries at or
//! past `counts.range` are ignored regardless of what bit pattern is stored
//! there, because that memory is stale once compaction has shrunk the range
//! — `element` only writes destinations below the survivor count, so
//! anything beyond `range` is left over from an earlier step and may read
//! back as "alive" by accident. The counts buffer is exactly the persistent
//! state this needs, and the ordering works out because within one step the
//! scan reads `range` before [`Compaction::record_advance`] rewrites it, and
//! the two are separate commands in the same encoder.
//!
//! # Where the buffers come from
//!
//! Every buffer this touches belongs to the `Set`: the two ping-ponging
//! alive buffers and the shared counts buffer are passed to
//! [`Compaction::new`], which builds one bind group per parity so that
//! [`Compaction::record`] can pick between them without creating anything.
//! Only `dest` and the block-sum pyramid are this module's own. `record` and
//! `record_advance` take no buffer arguments and allocate nothing; they
//! encode commands and nothing else.
//!
//! **Those two are counted differently by another crate, so their sizes are
//! not free to change here.** [`crate::set::ElementStorage`] reports what a
//! node allocated per element, and it counts `dest` and leaves the pyramid
//! out. `dest` is one `u32` per element, so it is a whole multiple of capacity
//! and divides exactly; the pyramid is indexed by *workgroup*, so `sums[0]`
//! holds `ceil(capacity / WORKGROUP_SIZE)` entries and counting it would turn
//! [`crate::set::ElementStorage::per_element`] into a rounding-down division.
//!
//! **The reason is the exactness and not the size.** The pyramid is
//! `Θ(capacity / 16)` bytes — linear in capacity with a small constant, since
//! each level is a 64th of the one below it and the sum of the series is about
//! a 16th of `capacity` — which is roughly 16 KiB against the 27 MiB counted
//! for a compacted L1 at capacity 262144 with a 48-byte stride, 0.06%. It is
//! negligible; it is not sublinear, and an argument for excluding it that
//! rested on its growth would be wrong.
//!
//! So two edits here break a claim made in `set.rs` rather than merely
//! changing a number: sizing `dest` at anything other than a whole multiple of
//! capacity, and giving the pyramid an input indexed by element. Either one
//! wants that claim revisited in the same change.
//!
//! # What this does not do
//!
//! It does not decide *whether* to run. A procedure with no `spawn` block
//! and no `kill()` never changes its live set, so its scan would compute the
//! identity permutation every frame at full capacity; `Set` skips
//! constructing a `Compaction` at all for one of those. See
//! `karakuri_codegen::l1`'s `compacted` flag.

use karakuri_codegen::layout::{counts, step_args, WORKGROUP_SIZE};
use wgpu::util::DeviceExt;

/// Matches `LevelLen` in `shaders/scan.wgsl`: four plain `u32`s so nothing
/// in WGSL's uniform-address-space alignment rules can make this struct's
/// host-shareable layout drift from this one's `repr(C)` layout.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct LevelLen {
    len: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn buffer_entry(binding: u32, resource: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: resource.as_entire_binding(),
    }
}

fn make_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::BindGroupLayout,
    entry_point: &str,
    label: &str,
) -> wgpu::ComputePipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: Some(&pipeline_layout),
        module: shader,
        entry_point: Some(entry_point),
        compilation_options: Default::default(),
        cache: None,
    })
}

/// Records an exclusive prefix sum over the alive flags and turns it into
/// destination indices plus a survivor count.
pub struct Compaction {
    capacity: u32,
    /// `workgroups[p]` is both the number of workgroups pass `p` dispatches
    /// with and the element count of level `p + 1` (the block sums pass `p`
    /// produces) — the same number for both reasons, since a level's block
    /// count *is* how many workgroups cover the level below it.
    workgroups: Vec<u32>,

    dest: wgpu::Buffer,
    /// `sums[p]` is level `p + 1`: the block-sum output of pass `p`, and the
    /// (in-place scanned) input of pass `p + 1`. Never read directly after
    /// construction — every pass reaches these buffers through the bind
    /// groups built alongside them — but held here so they live as long as
    /// `Compaction` does rather than only as long as those bind groups
    /// happen to keep wgpu's internal buffer alive.
    #[allow(dead_code)]
    sums: Vec<wgpu::Buffer>,

    pipeline_from_alive: wgpu::ComputePipeline,
    pipeline_inplace: wgpu::ComputePipeline,
    pipeline_add_offsets: wgpu::ComputePipeline,
    pipeline_finalize: wgpu::ComputePipeline,
    pipeline_advance: wgpu::ComputePipeline,

    /// Indexed by parity, like every other ping-ponging bind group in the
    /// engine: the alive buffer the scan reads is whichever one is "prev"
    /// for the step being recorded.
    bg_level0: [wgpu::BindGroup; 2],
    /// One per level `p = 1..num_levels`, i.e. `bg_levels[p - 1]`.
    bg_levels: Vec<wgpu::BindGroup>,
    /// One per level `p = 0..num_levels - 1` (the top level needs none).
    bg_add_offsets: Vec<wgpu::BindGroup>,
    bg_finalize: wgpu::BindGroup,
    /// One per substep, each binding the spawn-args buffer at that substep's
    /// offset — see `karakuri_codegen::layout::step_args` for why the
    /// parameters differ between the substeps of one frame.
    bg_advance: Vec<wgpu::BindGroup>,
}

impl Compaction {
    /// `alive` is the Set's ping-ponging pair of alive buffers, indexed by
    /// parity; `counts` and `step_args_buf` are the Set's shared engine
    /// state. All three are borrowed only for the length of this call —
    /// wgpu bind groups keep their own references — which is what lets the
    /// Set own them and this own the passes over them.
    pub fn new(
        device: &wgpu::Device,
        capacity: u32,
        alive: [&wgpu::Buffer; 2],
        counts_buf: &wgpu::Buffer,
        step_args_buf: &wgpu::Buffer,
        substeps: u32,
    ) -> Compaction {
        assert!(capacity >= 1, "compaction requires a non-zero capacity");
        assert!(
            substeps >= 1,
            "at least one substep's advance bind group is needed"
        );
        let wg = WORKGROUP_SIZE;

        // The level pyramid: level_sizes[0] is `capacity`, and each further
        // entry is how many workgroups of WORKGROUP_SIZE cover the level
        // before it. It ends the moment a level's count is 1 — one
        // workgroup then covers the whole level, so recursion can stop.
        // This is what lets one shader pair handle any capacity: depth is
        // computed here, not fixed at some maximum.
        let mut level_sizes = vec![capacity];
        while *level_sizes.last().unwrap() > 1 {
            let prev = *level_sizes.last().unwrap();
            level_sizes.push(prev.div_ceil(wg));
        }
        if level_sizes.len() == 1 {
            // capacity == 1 still needs one pass to learn whether that one
            // element is alive; the loop above stops immediately since 1
            // is already <= 1.
            level_sizes.push(1);
        }
        let workgroups: Vec<u32> = level_sizes[1..].to_vec();
        let num_levels = workgroups.len();

        let shader_source = include_str!("shaders/scan.wgsl")
            .replace("{{WG}}", &wg.to_string())
            .replace("{{COUNTS}}", counts::WGSL)
            .replace("{{STEP_ARGS}}", step_args::WGSL);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("compaction scan"),
            source: wgpu::ShaderSource::Wgsl(shader_source.into()),
        });

        // --- buffers, all created once here; `record` never allocates ---

        let dest = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("compaction dest"),
            // **Not `capacity * 4` written out here.** The same buffer has to
            // be sized by whatever *reports* what a node allocated, with no
            // device and no `Compaction` in hand — `crate::storage` is where
            // that is decided, and this is the allocating half of the one
            // expression. The module doc's paragraph on this size not being
            // free to change here is the reason it has a home rather than a
            // spelling.
            size: crate::storage::dest_bytes(capacity),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let sums: Vec<wgpu::Buffer> = workgroups
            .iter()
            .map(|&count| {
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("compaction block sums"),
                    size: u64::from(count) * 4,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                })
            })
            .collect();

        let len_uniforms: Vec<wgpu::Buffer> = level_sizes[..num_levels]
            .iter()
            .map(|&len| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("compaction level len"),
                    contents: bytemuck::bytes_of(&LevelLen {
                        len,
                        _pad0: 0,
                        _pad1: 0,
                        _pad2: 0,
                    }),
                    usage: wgpu::BufferUsages::UNIFORM,
                })
            })
            .collect();

        // --- bind group layouts: one per entry point, binding numbers
        // matching the WGSL exactly (see shaders/scan.wgsl) ---

        let bgl_from_alive = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compaction scan_from_alive"),
            entries: &[
                storage_entry(0, true),
                storage_entry(1, false),
                storage_entry(2, false),
                uniform_entry(3),
                storage_entry(4, true),
            ],
        });
        let bgl_inplace = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compaction scan_inplace"),
            entries: &[
                storage_entry(5, false),
                storage_entry(6, false),
                uniform_entry(7),
            ],
        });
        let bgl_add_offsets = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compaction add_offsets"),
            entries: &[
                storage_entry(8, false),
                storage_entry(9, true),
                uniform_entry(10),
            ],
        });
        let bgl_finalize = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compaction finalize"),
            entries: &[storage_entry(11, true), storage_entry(12, false)],
        });
        let bgl_advance = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compaction advance"),
            entries: &[storage_entry(13, false), uniform_entry(14)],
        });

        let pipeline_from_alive = make_pipeline(
            device,
            &shader,
            &bgl_from_alive,
            "scan_from_alive",
            "compaction scan_from_alive",
        );
        let pipeline_inplace = make_pipeline(
            device,
            &shader,
            &bgl_inplace,
            "scan_inplace",
            "compaction scan_inplace",
        );
        let pipeline_add_offsets = make_pipeline(
            device,
            &shader,
            &bgl_add_offsets,
            "add_offsets",
            "compaction add_offsets",
        );
        let pipeline_finalize = make_pipeline(
            device,
            &shader,
            &bgl_finalize,
            "finalize",
            "compaction finalize",
        );
        let pipeline_advance = make_pipeline(
            device,
            &shader,
            &bgl_advance,
            "advance",
            "compaction advance",
        );

        let bg_level0 = std::array::from_fn(|parity| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("compaction level0"),
                layout: &bgl_from_alive,
                entries: &[
                    buffer_entry(0, alive[parity]),
                    buffer_entry(1, &dest),
                    buffer_entry(2, &sums[0]),
                    buffer_entry(3, &len_uniforms[0]),
                    buffer_entry(4, counts_buf),
                ],
            })
        });

        let bg_levels: Vec<wgpu::BindGroup> = (1..num_levels)
            .map(|p| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("compaction level"),
                    layout: &bgl_inplace,
                    entries: &[
                        buffer_entry(5, &sums[p - 1]),
                        buffer_entry(6, &sums[p]),
                        buffer_entry(7, &len_uniforms[p]),
                    ],
                })
            })
            .collect();

        // The top level (p == num_levels - 1) needs no add-back: its
        // block-sum output has exactly one entry, so its local scan is
        // already the whole level's correct global scan. See the module
        // doc's step 2.
        let bg_add_offsets: Vec<wgpu::BindGroup> = (0..num_levels.saturating_sub(1))
            .map(|p| {
                let local = if p == 0 { &dest } else { &sums[p - 1] };
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("compaction add_offsets"),
                    layout: &bgl_add_offsets,
                    entries: &[
                        buffer_entry(8, local),
                        buffer_entry(9, &sums[p]),
                        buffer_entry(10, &len_uniforms[p]),
                    ],
                })
            })
            .collect();

        let bg_finalize = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compaction finalize"),
            layout: &bgl_finalize,
            entries: &[
                buffer_entry(11, &sums[num_levels - 1]),
                buffer_entry(12, counts_buf),
            ],
        });

        let bg_advance: Vec<wgpu::BindGroup> = (0..substeps)
            .map(|step| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("compaction advance"),
                    layout: &bgl_advance,
                    entries: &[
                        buffer_entry(13, counts_buf),
                        wgpu::BindGroupEntry {
                            binding: 14,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer: step_args_buf,
                                offset: u64::from(step) * step_args::STRIDE,
                                size: wgpu::BufferSize::new(step_args::SIZE),
                            }),
                        },
                    ],
                })
            })
            .collect();

        Compaction {
            capacity,
            workgroups,
            dest,
            sums,
            pipeline_from_alive,
            pipeline_inplace,
            pipeline_add_offsets,
            pipeline_finalize,
            pipeline_advance,
            bg_level0,
            bg_levels,
            bg_add_offsets,
            bg_finalize,
            bg_advance,
        }
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    /// `array<u32>`, length `capacity`. `dest[i]` is element `i`'s
    /// destination index once every survivor before it (within the
    /// previous live range) has been counted — undefined for `i` at or
    /// past the range, since those entries were not scanned.
    pub fn dest_buffer(&self) -> &wgpu::Buffer {
        &self.dest
    }

    /// Encodes every scan pass, over the alive buffer `parity` selects as
    /// "prev". Allocates nothing: every buffer and bind group already exists
    /// from [`Compaction::new`].
    pub fn record(&self, encoder: &mut wgpu::CommandEncoder, parity: bool) {
        let num_levels = self.workgroups.len();
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("compaction scan"),
            timestamp_writes: None,
        });

        // Step 1 + the start of step 2: reduce, bottom-up. Level 0 reads the
        // alive-flags buffer; every level above reads the level below's
        // block sums, in place.
        pass.set_pipeline(&self.pipeline_from_alive);
        pass.set_bind_group(0, &self.bg_level0[usize::from(parity)], &[]);
        pass.dispatch_workgroups(self.workgroups[0], 1, 1);
        for p in 1..num_levels {
            pass.set_pipeline(&self.pipeline_inplace);
            pass.set_bind_group(0, &self.bg_levels[p - 1], &[]);
            pass.dispatch_workgroups(self.workgroups[p], 1, 1);
        }

        // Step 3: add the offsets back, top-down. The top level needs no
        // correction — see the module doc.
        for p in (0..num_levels.saturating_sub(1)).rev() {
            pass.set_pipeline(&self.pipeline_add_offsets);
            pass.set_bind_group(0, &self.bg_add_offsets[p], &[]);
            pass.dispatch_workgroups(self.workgroups[p], 1, 1);
        }

        // Grand total -> `counts.survivors`, and nothing else: `element`
        // still has to run against the pre-scan range. See the shader.
        pass.set_pipeline(&self.pipeline_finalize);
        pass.set_bind_group(0, &self.bg_finalize, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }

    /// Rolls `range` forward and recomputes the dispatch and draw arguments
    /// from it, using substep `step`'s spawn count. Runs last in a step,
    /// after both `element` and `spawn` — until it does, `counts.range` is
    /// still the range those two were dispatched against.
    pub fn record_advance(&self, encoder: &mut wgpu::CommandEncoder, step: usize) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("compaction advance"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline_advance);
        pass.set_bind_group(0, &self.bg_advance[step], &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
}
