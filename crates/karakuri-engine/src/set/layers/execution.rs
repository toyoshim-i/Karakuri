use karakuri_ir::layout::ElementLayout;
use karakuri_ir::Kind;

use crate::node::{Deform, Simulation};
use crate::set::types::{ElementStorage, Set, Source};
use crate::video_source::VideoSource;

impl Set {
    /// Resizes renderer viewports and accumulation targets to match new dimensions.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.viewport = [width.max(1) as f32, height.max(1) as f32];
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
            camera.record(encoder);
        }
        let merge = self.merge.as_ref();
        for (source_at, source) in self.sources.iter().enumerate() {
            let (parity, counts) = (source.sim.parity(), self.output_counts(source));
            for (i, renderer) in source.renderers.iter().enumerate() {
                match merge {
                    None => {
                        renderer.draw(encoder, target, parity, counts, source_at == 0 && i == 0)
                    }
                    Some(merge) => {
                        renderer.draw(encoder, merge.target(i), parity, counts, source_at == 0)
                    }
                }
            }
        }
        if let Some(merge) = merge {
            merge.record(encoder, target);
        }
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
