use karakuri_ir::layout::ElementLayout;
use karakuri_ir::Kind;

use crate::node::{Deform, Simulation};
use crate::set::types::{ElementStorage, Set, Source};
use crate::video_source::VideoSource;

pub(crate) fn allocate_hdr_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: &str,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: crate::present::Present::HDR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

impl Set {
    /// Resizes renderer viewports and accumulation targets to match new dimensions.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        self.viewport = [width as f32, height as f32];
        if self.depth_texture.is_some() {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Set depth target"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.depth_view = Some(texture.create_view(&wgpu::TextureViewDescriptor::default()));
            self.depth_texture = Some(texture);
        }
        if self.l5_target.is_some() {
            let (t, v) = allocate_hdr_target(device, width, height, "Set L5 target");
            self.l5_target = Some(t);
            self.l5_target_view = Some(v);
        }
        if self.l5_ping_target.is_some() {
            let (t, v) = allocate_hdr_target(device, width, height, "Set L5 ping");
            self.l5_ping_target = Some(t);
            self.l5_ping_view = Some(v);
        }
        if self.l5_held_target.is_some() {
            let (t, v) = allocate_hdr_target(device, width, height, "Set L5 held");
            self.l5_held_target = Some(t);
            self.l5_held_view = Some(v);
        }
        if let (Some(layout), Some(sampler)) = (&self.l5_layout, &self.l5_sampler) {
            let total = self.l5s.len();
            for at in 0..total {
                let src_view = if at == 0 {
                    self.l5_target_view.as_ref().unwrap()
                } else if at % 2 == 1 {
                    self.l5_ping_view.as_ref().unwrap()
                } else {
                    self.l5_target_view.as_ref().unwrap()
                };
                let held_view = if self.l5s[at].retains {
                    self.l5_held_view.as_ref().unwrap_or(src_view)
                } else {
                    src_view
                };
                self.l5s[at].bind_group = self.l5s[at].pass.bind(
                    device,
                    layout,
                    src_view,
                    held_view,
                    sampler,
                    Some(&format!("Set L5[{at}] bind")),
                );
            }
        }
        for renderer in self.sources.iter_mut().flat_map(|s| &mut s.renderers) {
            renderer.resize(device, width, height);
        }
        if let Some(merge) = &mut self.merge {
            merge.resize(device, width, height);
        }
    }

    /// Returns the currently clamped viewport dimensions `(width, height)`.
    pub fn viewport(&self) -> (u32, u32) {
        (self.viewport[0] as u32, self.viewport[1] as u32)
    }

    /// Returns the committed simulation steps elapsed.
    pub fn steps_taken(&self) -> u64 {
        self.steps_taken
    }

    /// Returns the uncommitted steps staged for the current frame.
    pub fn staged_delta(&self) -> u64 {
        self.staged_delta
    }

    /// Returns the active ping-pong buffer index for the primary geometry.
    pub fn parity(&self) -> usize {
        self.sources.first().map_or(0, |s| s.sim.parity())
    }

    /// Returns the committed ping-pong buffer index for the primary geometry.
    pub fn committed_parity(&self) -> usize {
        self.sources.first().map_or(0, |s| s.sim.committed_parity())
    }

    /// Commits staged clock steps, buffer parities, and composite input edges upon submission.
    pub fn commit(&mut self) {
        self.steps_taken += self.staged_delta;
        self.staged_delta = 0;
        if let Some(edges) = self.staged_edges.take() {
            self.edges = edges;
        }
        for source in &mut self.sources {
            source.sim.commit();
            if let Some(other) = &mut source.paired {
                other.commit();
            }
        }
    }

    /// Discards staged simulation clock advancement and ping-pong parities.
    pub fn discard(&mut self) {
        self.staged_delta = 0;
        self.staged_edges = None;
        for source in &mut self.sources {
            source.sim.discard();
            if let Some(other) = &mut source.paired {
                other.discard();
            }
        }
    }

    /// Returns elapsed simulation time in seconds.
    pub fn time(&self) -> f32 {
        self.t_at(self.steps_taken)
    }

    /// Reads back the total active element count across all sources. Blocks GPU queue.
    pub fn live_count(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> u32 {
        self.sources
            .iter()
            .map(|s| s.sim.live_count(device, queue))
            .sum()
    }

    /// Reads back raw element buffer bytes decoded against the layout. Blocks GPU queue.
    pub fn read_elements(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        self.sources[0].sim.read_elements(device, queue)
    }

    /// Returns the primary geometry's element buffer layout.
    pub fn element_layout(&self) -> &ElementLayout {
        self.sources[0].sim.element_layout()
    }

    /// Returns the total element capacity across all geometry sources.
    pub fn capacity(&self) -> u32 {
        self.sources.iter().map(|s| s.sim.capacity()).sum()
    }

    /// Returns true if all Set procedures are closed-form functions of seed, time, and params.
    pub fn is_closed_form(&self) -> bool {
        self.closed_form
    }

    /// Returns true if any procedure in the Set references the ambient beat count.
    pub fn reads_beats(&self) -> bool {
        self.reads_beats
    }

    /// Seeks the simulation clock directly to `steps_taken` without running intermediate steps.
    pub fn seek(&mut self, steps_taken: u64) {
        self.discard();
        self.steps_taken = steps_taken;
    }

    /// Resets simulation buffers and clock state back to initial post-build conditions.
    pub fn rewind(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue) {
        self.discard();
        self.steps_taken = 0;
        for source in &mut self.sources {
            source.sim.rewind(queue);
            if let Some(other) = &mut source.paired {
                other.rewind(queue);
            }
        }
    }

    /// Returns the element storage allocation for each node instance in the Set.
    pub fn element_storage(&self) -> Vec<ElementStorage> {
        self.sources
            .iter()
            .flat_map(|source| {
                std::iter::once(source.sim.element_storage())
                    .chain(source.paired.iter().map(Simulation::element_storage))
                    .chain(source.deforms.iter().map(Deform::element_storage))
            })
            .collect()
    }

    /// Returns the total bytes allocated across all element buffers in the Set.
    pub fn element_storage_bytes(&self) -> u64 {
        self.element_storage().iter().map(|e| e.bytes).sum()
    }

    /// Returns canonical names for each node in execution order.
    pub fn node_names(&self) -> &[String] {
        &self.names
    }

    /// Returns hash salts assigned to each geometry source.
    pub fn source_salts(&self) -> &[u32] {
        &self.source_salts
    }

    /// Returns allocated element capacities for each geometry source.
    pub fn source_capacities(&self) -> Vec<u32> {
        let mut out = vec![0; self.l1_count];
        for source in &self.sources {
            for (k, sim) in std::iter::once(&source.sim)
                .chain(source.paired.iter())
                .enumerate()
            {
                out[source.procedures[k]] = sim.capacity();
            }
        }
        out
    }

    /// Returns declared `[min, max, default]` capacity specifications for each geometry.
    pub fn declared_capacities(&self) -> &[[u32; 3]] {
        &self.declared_capacities
    }

    /// Looks up a node by name, returning its `(layer, index)` address if found.
    pub fn node_named(&self, name: &str) -> Option<(Kind, u32)> {
        let at = self.names.iter().position(|n| n == name)?;
        Kind::ALL.into_iter().find_map(|kind| {
            let range = self.nodes_of(kind);
            range
                .contains(&at)
                .then(|| (kind, (at - range.start) as u32))
        })
    }

    /// Advances simulation and deformation passes for the current frame without rasterizing.
    pub fn step(&mut self, encoder: &mut wgpu::CommandEncoder, steps: u8) {
        let steps = if self
            .sources
            .iter()
            .flat_map(|s| &s.renderers)
            .all(|r| r.is_fullscreen())
        {
            0
        } else {
            steps
        };
        for source in &mut self.sources {
            source.sim.record(encoder, steps);
            if let Some(other) = &mut source.paired {
                other.record(encoder, steps);
            }
        }
        self.record_counts(encoder);
        for source in &self.sources {
            let parity = source.sim.parity();
            let mut counts = source.sim.counts();
            for node in &source.deforms {
                node.record(encoder, parity, counts);
                if let Some(own) = node.counts() {
                    counts = own;
                }
            }
        }
    }

    /// Records amplifier compute passes to derive instance counts.
    pub(crate) fn record_counts(&self, encoder: &mut wgpu::CommandEncoder) {
        for node in self.sources.iter().flat_map(|s| &s.deforms) {
            node.record_counts(encoder);
        }
    }

    /// Returns the output count buffer from the last amplifier in the deformation chain.
    pub(crate) fn output_counts<'a>(&self, source: &'a Source) -> &'a wgpu::Buffer {
        source
            .deforms
            .iter()
            .rev()
            .find_map(|node| node.counts())
            .unwrap_or_else(|| source.sim.counts())
    }

    /// Records rasterization passes for all renderers into `target`.
    pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        for camera in &self.cameras {
            let parity = camera
                .subject_head()
                .map_or(0, |head| self.sources[head].sim.parity());
            camera.record(encoder, parity);
        }
        let merge = self.merge.as_ref();
        let depth_view = self.depth_view.as_ref();

        let render_dest = if self.l5s.is_empty() {
            target
        } else {
            self.l5_target_view.as_ref().unwrap()
        };

        for (source_at, source) in self.sources.iter().enumerate() {
            let (parity, counts) = (source.sim.parity(), self.output_counts(source));
            for (i, renderer) in source.renderers.iter().enumerate() {
                match merge {
                    None => renderer.draw(
                        encoder,
                        render_dest,
                        depth_view,
                        parity,
                        counts,
                        source_at == 0 && i == 0,
                    ),
                    Some(merge) => renderer.draw(
                        encoder,
                        merge.target(i),
                        depth_view,
                        parity,
                        counts,
                        source_at == 0,
                    ),
                }
            }
        }
        if let Some(merge) = merge {
            merge.record(encoder, render_dest);
        }

        if !self.l5s.is_empty() {
            let total = self.l5s.len();
            for at in 0..total {
                let is_last = at == total - 1;
                let dst_view = if is_last {
                    target
                } else if at % 2 == 0 {
                    self.l5_ping_view.as_ref().unwrap()
                } else {
                    self.l5_target_view.as_ref().unwrap()
                };
                self.l5s[at]
                    .pass
                    .record(encoder, dst_view, &self.l5s[at].bind_group);
            }
            if let (Some(target), Some(held)) = (&self.l5_target, &self.l5_held_target) {
                let (w, h) = self.viewport();
                encoder.copy_texture_to_texture(
                    target.as_image_copy(),
                    held.as_image_copy(),
                    wgpu::Extent3d {
                        width: w.max(1),
                        height: h.max(1),
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
    }

    /// Records rasterization pass for a single stereo eye into a specified viewport.
    pub fn draw_stereo_eye(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        matrices: &crate::camera::StereoMatrices,
        first: bool,
        viewport: (f32, f32, f32, f32),
    ) {
        for camera in &self.cameras {
            camera.write_derived_matrices(queue, matrices);
        }
        let merge = self.merge.as_ref();
        let depth_view = self.depth_view.as_ref();

        for (source_at, source) in self.sources.iter().enumerate() {
            let (parity, counts) = (source.sim.parity(), self.output_counts(source));
            for (i, renderer) in source.renderers.iter().enumerate() {
                match merge {
                    None => renderer.draw_viewport(
                        encoder,
                        target,
                        depth_view,
                        parity,
                        counts,
                        first && source_at == 0 && i == 0,
                        Some(viewport),
                    ),
                    Some(merge) => renderer.draw(
                        encoder,
                        merge.target(i),
                        depth_view,
                        parity,
                        counts,
                        source_at == 0,
                    ),
                }
            }
        }
        if let Some(merge) = merge {
            merge.record_viewport(encoder, target, first, Some(viewport));
        }
    }

    /// Records Side-by-Side (SBS) stereo rasterization passes for WebXR into `target`.
    /// Left eye draws to left viewport (0..w/2), Right eye draws to right viewport (w/2..w).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_stereo(
        &mut self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        left: &crate::camera::StereoMatrices,
        right: &crate::camera::StereoMatrices,
        width: u32,
        height: u32,
    ) {
        let half_w = (width / 2) as f32;
        let h = height as f32;
        let left_vp = (0.0, 0.0, half_w, h);
        let right_vp = (half_w, 0.0, half_w, h);

        self.draw_stereo_eye(queue, encoder, target, left, true, left_vp);
        self.draw_stereo_eye(queue, encoder, target, right, false, right_vp);
    }
}

impl VideoSource for Set {
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        steps: u8,
    ) {
        self.step(encoder, steps);
        self.draw(encoder, target);
    }

    fn commit(&mut self) {
        self.commit();
    }

    fn discard(&mut self) {
        self.discard();
    }
}
