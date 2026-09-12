//! Per-slot luminance metering and level measurement.
//!
//! Measures linear Rec.709 luminance (mean and peak) across each slot's rendered
//! target before gain and opacity are applied. Non-finite texels (NaN / Inf) are
//! excluded from luminance calculations and recorded separately in `bad_texels`.
//!
//! Reduction executes on the GPU in two compute dispatches and copies results into
//! a ring buffer mapped asynchronously with non-blocking status polling.

use std::sync::mpsc::{self, Receiver, Sender};

/// Threads per workgroup for the first-pass tile reduction.
const WORKGROUP: u32 = 64;

/// Byte size of the level reduction result buffer (aligned to 16 bytes).
const LEVEL_SIZE: u64 = 16;

/// Number of staging buffers in the readback ring per slot.
pub const RING: usize = 4;

/// Measured luminance levels for one deck slot.
///
/// Computed using linear Rec.709 weights (`0.2126 R + 0.7152 G + 0.0722 B`)
/// over the slot's pre-fader render target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Level {
    /// Mean luminance across all valid texels in the frame.
    pub mean: f32,
    /// Peak luminance across all valid texels in the frame.
    pub peak: f32,
    /// Number of frames elapsed since this measurement was recorded.
    pub frames_behind: u32,
    /// Count of texels excluded from luminance calculation due to non-finite values.
    pub bad_texels: u32,
}

/// What a completed measurement carried, kept until a newer one arrives.
#[derive(Debug, Clone, Copy)]
struct Reading {
    mean: f32,
    peak: f32,
    bad: u32,
    /// The slot's frame ordinal at the time it was recorded.
    frame: u64,
}

/// A staging buffer's outstanding measurement.
#[derive(Debug, Clone, Copy)]
struct Claim {
    /// Staging buffer claim era generation.
    generation: u64,
    frame: u64,
    /// Whether `map_async` has been invoked for this buffer.
    armed: bool,
}

struct Staging {
    buffer: wgpu::Buffer,
    claim: Option<Claim>,
}

/// GPU resources and state tracking for a single slot's meter.
struct SlotMeter {
    partials: wgpu::Buffer,
    result: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    ring: [Staging; RING],
    tx: Sender<(usize, bool)>,
    rx: Receiver<(usize, bool)>,
    frame: u64,
    generation: u64,
    last: Option<Reading>,
    skipped: u64,
}

/// Per-slot luminance meters over individual slot HDR render targets.
pub struct Meters {
    layout: wgpu::BindGroupLayout,
    tiles: wgpu::ComputePipeline,
    total: wgpu::ComputePipeline,
    slots: Vec<SlotMeter>,
}

impl Meters {
    /// Creates a meter set for the provided texture views.
    pub fn new(device: &wgpu::Device, views: &[&wgpu::TextureView]) -> Meters {
        assert!(
            !views.is_empty(),
            "a meter over no targets measures nothing"
        );

        let source = include_str!("shaders/meter.wgsl").replace("{{WG}}", &WORKGROUP.to_string());
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("meter"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("meter"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                storage_entry(1),
                storage_entry(2),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("meter"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |entry_point: &str, label: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let tiles = pipeline("reduce_tiles", "meter reduce_tiles");
        let total = pipeline("reduce_total", "meter reduce_total");

        let slots = views
            .iter()
            .enumerate()
            .map(|(i, view)| SlotMeter::new(device, &layout, i, view))
            .collect();

        Meters {
            layout,
            tiles,
            total,
            slots,
        }
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Records tile and total reduction passes for `slot` into `encoder`.
    ///
    /// Copies the reduced level into an available staging buffer in the ring.
    /// If all staging buffers are currently busy, increments `skipped`.
    pub fn record(&mut self, slot: usize, encoder: &mut wgpu::CommandEncoder) {
        let (tiles, total) = (&self.tiles, &self.total);
        let meter = &mut self.slots[slot];
        let frame = meter.frame;
        meter.frame += 1;

        let Some(index) = meter.ring.iter().position(|s| s.claim.is_none()) else {
            meter.skipped += 1;
            return;
        };

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("meter"),
                timestamp_writes: None,
            });
            pass.set_bind_group(0, &meter.bind_group, &[]);
            pass.set_pipeline(tiles);
            pass.dispatch_workgroups(WORKGROUP, 1, 1);
            pass.set_pipeline(total);
            pass.dispatch_workgroups(1, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&meter.result, 0, &meter.ring[index].buffer, 0, LEVEL_SIZE);
        meter.ring[index].claim = Some(Claim {
            generation: meter.generation,
            frame,
            armed: false,
        });
    }

    /// Maps claimed staging buffers asynchronously after command submission.
    pub fn arm(&mut self) {
        for meter in &mut self.slots {
            let tx = &meter.tx;
            for (index, staging) in meter.ring.iter_mut().enumerate() {
                if let Some(claim) = &mut staging.claim {
                    if !claim.armed {
                        claim.armed = true;
                        let tx = tx.clone();
                        staging
                            .buffer
                            .slice(..)
                            .map_async(wgpu::MapMode::Read, move |r| {
                                let _ = tx.send((index, r.is_ok()));
                            });
                    }
                }
            }
        }
    }

    /// Polls the GPU device and ingests completed readback buffers.
    pub fn collect(&mut self, device: &wgpu::Device) {
        let _ = device.poll(wgpu::PollType::Poll);
        for meter in &mut self.slots {
            while let Ok((index, mapped)) = meter.rx.try_recv() {
                let claim = meter.ring[index]
                    .claim
                    .take()
                    .expect("a map callback for a staging buffer nothing claimed");
                if !mapped {
                    continue;
                }
                let staging = &meter.ring[index].buffer;
                let data = staging
                    .slice(..)
                    .get_mapped_range()
                    .expect("the map callback reported success");
                let mean = f32::from_le_bytes(data[0..4].try_into().expect("4-byte chunk"));
                let peak = f32::from_le_bytes(data[4..8].try_into().expect("4-byte chunk"));
                let bad = f32::from_le_bytes(data[8..12].try_into().expect("4-byte chunk"));
                drop(data);
                staging.unmap();
                let bad = bad as u32;

                let current = claim.generation == meter.generation;
                let newer = meter.last.is_none_or(|last| claim.frame >= last.frame);
                if current && newer {
                    meter.last = Some(Reading {
                        mean,
                        peak,
                        bad,
                        frame: claim.frame,
                    });
                }
            }
        }
    }

    /// Returns the most recent completed level reading for `slot`, if available.
    pub fn level(&self, slot: usize) -> Option<Level> {
        let meter = &self.slots[slot];
        meter.last.map(|reading| Level {
            mean: reading.mean,
            peak: reading.peak,
            bad_texels: reading.bad,
            frames_behind: (meter.frame.saturating_sub(1) - reading.frame) as u32,
        })
    }

    /// Retires a slot's current reading and invalidates in-flight measurements.
    pub fn retire(&mut self, slot: usize) {
        let meter = &mut self.slots[slot];
        meter.generation += 1;
        meter.last = None;
    }

    /// Returns the number of frames skipped due to all ring buffers being busy.
    pub fn skipped(&self, slot: usize) -> u64 {
        self.slots[slot].skipped
    }

    /// Rebinds the meters to new texture views following a resize.
    pub fn rebind(&mut self, device: &wgpu::Device, views: &[&wgpu::TextureView]) {
        assert_eq!(
            views.len(),
            self.slots.len(),
            "a rebind cannot change how many slots are metered"
        );
        for (slot, view) in views.iter().enumerate() {
            self.slots[slot].bind_group = SlotMeter::bind(
                device,
                &self.layout,
                slot,
                view,
                &self.slots[slot].partials,
                &self.slots[slot].result,
            );
            self.retire(slot);
        }
    }
}

impl SlotMeter {
    fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        slot: usize,
        view: &wgpu::TextureView,
    ) -> SlotMeter {
        let partials = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("meter {slot} partials")),
            // `vec3<f32>` per workgroup, which WGSL lays out at a stride of
            // 16 rather than 12: an array element is aligned to the type's
            // alignment and `vec3` is aligned as a `vec4`.
            size: u64::from(WORKGROUP) * 16,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let result = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("meter {slot} level")),
            size: LEVEL_SIZE,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let bind_group = SlotMeter::bind(device, layout, slot, view, &partials, &result);
        let ring = std::array::from_fn(|i| Staging {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("meter {slot} staging {i}")),
                size: LEVEL_SIZE,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            claim: None,
        });
        let (tx, rx) = mpsc::channel();

        SlotMeter {
            partials,
            result,
            bind_group,
            ring,
            tx,
            rx,
            frame: 0,
            generation: 0,
            last: None,
            skipped: 0,
        }
    }

    fn bind(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        slot: usize,
        view: &wgpu::TextureView,
        partials: &wgpu::Buffer,
        result: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("meter {slot}")),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: partials.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: result.as_entire_binding(),
                },
            ],
        })
    }
}

fn storage_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
