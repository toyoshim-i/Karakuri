//! Order-preserving stream compaction for element buffers.
//!
//! Compacts active elements into a contiguous range using hierarchical exclusive prefix sums.
//! Preserves the relative order of survivors to ensure deterministic blend order.
//!
//! The compaction executes as a multi-pass hierarchical scan:
//! 1. Reduce: Compute block prefix sums across the active range.
//! 2. Scan: Recursively compute prefix sums of the block sums.
//! 3. Add offsets: Propagate block offsets down the hierarchy.
//! 4. Finalize and advance: Update survivor counts, ranges, and indirect dispatch arguments.

use karakuri_codegen::layout::{counts, step_args, WORKGROUP_SIZE};
use wgpu::util::DeviceExt;

/// Matches `LevelLen` in `shaders/scan.wgsl` for uniform buffer binding.
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

/// GPU stream compaction pipeline and memory resources.
pub struct Compaction {
    capacity: u32,
    /// Number of workgroups dispatched per pyramid level.
    workgroups: Vec<u32>,

    dest: wgpu::Buffer,
    #[allow(dead_code)]
    sums: Vec<wgpu::Buffer>,

    pipeline_from_alive: wgpu::ComputePipeline,
    pipeline_inplace: wgpu::ComputePipeline,
    pipeline_add_offsets: wgpu::ComputePipeline,
    pipeline_finalize: wgpu::ComputePipeline,
    pipeline_advance: wgpu::ComputePipeline,

    /// Bind groups for the base scan level, indexed by buffer ping-pong parity.
    bg_level0: [wgpu::BindGroup; 2],
    /// Bind groups for intermediate reduction levels.
    bg_levels: Vec<wgpu::BindGroup>,
    /// Bind groups for offset distribution passes.
    bg_add_offsets: Vec<wgpu::BindGroup>,
    bg_finalize: wgpu::BindGroup,
    /// Per-substep bind groups for advancing counts and indirect arguments.
    bg_advance: Vec<wgpu::BindGroup>,
}

impl Compaction {
    /// Creates a compaction pipeline and intermediate buffers for the given capacity.
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

        // Calculate workgroup counts across hierarchy levels until reduced to 1.
        let mut level_sizes = vec![capacity];
        while *level_sizes.last().unwrap() > 1 {
            let prev = *level_sizes.last().unwrap();
            level_sizes.push(prev.div_ceil(wg));
        }
        if level_sizes.len() == 1 {
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

        let dest = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("compaction dest"),
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

    /// Returns the buffer containing compacted destination indices for active elements.
    pub fn dest_buffer(&self) -> &wgpu::Buffer {
        &self.dest
    }

    /// Records the hierarchical scan compute passes into `encoder`.
    pub fn record(&self, encoder: &mut wgpu::CommandEncoder, parity: bool) {
        let num_levels = self.workgroups.len();
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("compaction scan"),
            timestamp_writes: None,
        });

        // Reduce bottom-up.
        pass.set_pipeline(&self.pipeline_from_alive);
        pass.set_bind_group(0, &self.bg_level0[usize::from(parity)], &[]);
        pass.dispatch_workgroups(self.workgroups[0], 1, 1);
        for p in 1..num_levels {
            pass.set_pipeline(&self.pipeline_inplace);
            pass.set_bind_group(0, &self.bg_levels[p - 1], &[]);
            pass.dispatch_workgroups(self.workgroups[p], 1, 1);
        }

        // Propagate block offsets top-down.
        for p in (0..num_levels.saturating_sub(1)).rev() {
            pass.set_pipeline(&self.pipeline_add_offsets);
            pass.set_bind_group(0, &self.bg_add_offsets[p], &[]);
            pass.dispatch_workgroups(self.workgroups[p], 1, 1);
        }

        // Finalize survivor count.
        pass.set_pipeline(&self.pipeline_finalize);
        pass.set_bind_group(0, &self.bg_finalize, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }

    /// Records the advance compute pass to update element counts and dispatch arguments.
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
