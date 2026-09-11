//! Transient Render Graph (DAG) with Memory Aliasing and Dead Pass Culling.
//!
//! Provides a declarative pass graph representation:
//! - **Virtual Resource System**: [`ResourceId`], [`TextureDesc`], [`BufferDesc`], and [`GraphResource`].
//! - **Pass Scheduling**: Directed acyclic graph with dependency analysis,
//!   topological sorting, and cycle detection.
//! - **Dead Pass Culling**: Backward reachability from output targets and passes with side effects.
//! - **Transient Memory Aliasing**: Automatic interval-based physical texture reuse for
//!   non-overlapping transient resources via [`TransientMemoryPool`].
//! - **Execution**: Headless mock execution as well as full GPU recording via [`wgpu::CommandEncoder`].

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt;

/// Unique identifier for a virtual graph resource (transient or imported).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(pub usize);

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Res({})", self.0)
    }
}

/// Unique identifier for a pass in the render graph.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PassId(pub usize);

impl fmt::Display for PassId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pass({})", self.0)
    }
}

/// Texture descriptor specifying dimensions, format, and usages.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextureDesc {
    pub size: wgpu::Extent3d,
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub dimension: wgpu::TextureDimension,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsages,
    pub label: Option<String>,
}

impl TextureDesc {
    /// Helper to create a 2D single-sampled texture descriptor.
    pub fn d2(
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        Self {
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            label: None,
        }
    }

    /// Sets the label for this texture descriptor.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the mip level count.
    pub fn with_mip_levels(mut self, count: u32) -> Self {
        self.mip_level_count = count.max(1);
        self
    }

    /// Sets the sample count.
    pub fn with_samples(mut self, samples: u32) -> Self {
        self.sample_count = samples.max(1);
        self
    }

    /// Computes the estimated VRAM byte footprint for this texture.
    pub fn estimated_byte_size(&self) -> u64 {
        let bpp = match self.format {
            wgpu::TextureFormat::R8Unorm
            | wgpu::TextureFormat::R8Snorm
            | wgpu::TextureFormat::R8Uint
            | wgpu::TextureFormat::R8Sint => 1,

            wgpu::TextureFormat::R16Uint
            | wgpu::TextureFormat::R16Sint
            | wgpu::TextureFormat::R16Float
            | wgpu::TextureFormat::Rg8Unorm
            | wgpu::TextureFormat::Rg8Snorm
            | wgpu::TextureFormat::Rg8Uint
            | wgpu::TextureFormat::Rg8Sint => 2,

            wgpu::TextureFormat::Rgba8Unorm
            | wgpu::TextureFormat::Rgba8UnormSrgb
            | wgpu::TextureFormat::Rgba8Snorm
            | wgpu::TextureFormat::Rgba8Uint
            | wgpu::TextureFormat::Rgba8Sint
            | wgpu::TextureFormat::Bgra8Unorm
            | wgpu::TextureFormat::Bgra8UnormSrgb
            | wgpu::TextureFormat::R32Uint
            | wgpu::TextureFormat::R32Sint
            | wgpu::TextureFormat::R32Float
            | wgpu::TextureFormat::Rg16Uint
            | wgpu::TextureFormat::Rg16Sint
            | wgpu::TextureFormat::Rg16Float
            | wgpu::TextureFormat::Depth32Float
            | wgpu::TextureFormat::Depth24Plus
            | wgpu::TextureFormat::Depth24PlusStencil8 => 4,

            wgpu::TextureFormat::Rgba16Uint
            | wgpu::TextureFormat::Rgba16Sint
            | wgpu::TextureFormat::Rgba16Float
            | wgpu::TextureFormat::Rg32Uint
            | wgpu::TextureFormat::Rg32Sint
            | wgpu::TextureFormat::Rg32Float => 8,

            wgpu::TextureFormat::Rgba32Uint
            | wgpu::TextureFormat::Rgba32Sint
            | wgpu::TextureFormat::Rgba32Float => 16,

            _ => 4,
        };
        (self.size.width as u64)
            * (self.size.height as u64)
            * (self.size.depth_or_array_layers as u64)
            * bpp
            * (self.sample_count as u64).max(1)
    }
}

/// Buffer descriptor specifying size and usages.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BufferDesc {
    pub size: u64,
    pub usage: wgpu::BufferUsages,
    pub label: Option<String>,
}

impl BufferDesc {
    pub fn new(size: u64, usage: wgpu::BufferUsages) -> Self {
        Self {
            size,
            usage,
            label: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// A resource registered in the render graph.
pub enum GraphResource {
    TransientTexture(TextureDesc),
    ImportedTexture {
        texture: Option<wgpu::Texture>,
        view: wgpu::TextureView,
        desc: Option<TextureDesc>,
    },
    TransientBuffer(BufferDesc),
    ImportedBuffer(wgpu::Buffer),
}

/// Compilation or execution error in the render graph.
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("Render graph cycle detected: {0}")]
    CycleDetected(String),
    #[error("Resource {0:?} was not found in the graph")]
    ResourceNotFound(ResourceId),
    #[error("Pass {0:?} was not found in the graph")]
    PassNotFound(PassId),
    #[error("Output resource {0:?} has no producer pass")]
    OutputNotProduced(ResourceId),
    #[error("Execution error: {0}")]
    Execution(String),
}

/// Execution callback recorded directly to a GPU command encoder.
pub type GpuPassFn<'a> = Box<dyn FnMut(&mut wgpu::CommandEncoder, &ResourceResolver<'_>) + 'a>;

/// Mock execution callback for testing without a GPU device.
pub type MockPassFn<'a> = Box<dyn FnMut(&ResourceResolver<'_>) + 'a>;

/// Execution callback variant for a pass.
pub enum PassExecution<'a> {
    Gpu(GpuPassFn<'a>),
    Mock(MockPassFn<'a>),
    None,
}

/// A node in the render graph.
pub struct PassNode<'a> {
    pub id: PassId,
    pub label: String,
    pub reads: Vec<ResourceId>,
    pub writes: Vec<ResourceId>,
    pub enabled: bool,
    pub has_side_effects: bool,
    pub execution: PassExecution<'a>,
}

/// Resolves virtual [`ResourceId`]s to physical resources during pass execution.
pub struct ResourceResolver<'a> {
    views: HashMap<ResourceId, &'a wgpu::TextureView>,
    textures: HashMap<ResourceId, &'a wgpu::Texture>,
    buffers: HashMap<ResourceId, &'a wgpu::Buffer>,
    physical_slots: HashMap<ResourceId, usize>,
}

impl<'a> ResourceResolver<'a> {
    pub fn mock(physical_slots: &HashMap<ResourceId, usize>) -> Self {
        Self {
            views: HashMap::new(),
            textures: HashMap::new(),
            buffers: HashMap::new(),
            physical_slots: physical_slots.clone(),
        }
    }

    pub fn texture_view(&self, id: ResourceId) -> Option<&'a wgpu::TextureView> {
        self.views.get(&id).copied()
    }

    pub fn texture(&self, id: ResourceId) -> Option<&'a wgpu::Texture> {
        self.textures.get(&id).copied()
    }

    pub fn buffer(&self, id: ResourceId) -> Option<&'a wgpu::Buffer> {
        self.buffers.get(&id).copied()
    }

    pub fn physical_slot(&self, id: ResourceId) -> Option<usize> {
        self.physical_slots.get(&id).copied()
    }

    pub fn is_available(&self, id: ResourceId) -> bool {
        self.views.contains_key(&id)
            || self.buffers.contains_key(&id)
            || self.physical_slots.contains_key(&id)
    }
}

/// Metrics measuring render graph compilation and memory aliasing efficiency.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphMetrics {
    /// Total number of physical textures allocated in the pool.
    pub allocated_physical_textures: usize,
    /// Total virtual transient textures registered and active.
    pub virtual_transient_textures: usize,
    /// Bytes saved via transient memory aliasing.
    pub vram_saved_bytes: u64,
    /// Total passes registered in graph before culling.
    pub total_passes: usize,
    /// Number of passes scheduled to execute after dead pass culling.
    pub executed_passes: usize,
    /// Number of passes culled.
    pub culled_passes: usize,
}

/// The result of compiling a [`RenderGraph`].
#[derive(Clone, Debug)]
pub struct CompiledGraph {
    /// Scheduled execution order of active passes.
    pub pass_order: Vec<PassId>,
    /// Passes culled from execution.
    pub culled_passes: Vec<PassId>,
    /// Lifetime ranges `(first_pass_index, last_pass_index)` for transient resources.
    pub lifetimes: HashMap<ResourceId, (usize, usize)>,
    /// Aliasing map: virtual transient `ResourceId` -> physical pool slot index.
    pub physical_slots: HashMap<ResourceId, usize>,
    /// Descriptors of the allocated physical slots.
    pub physical_slot_descs: Vec<TextureDesc>,
    /// Memory and execution metrics.
    pub metrics: GraphMetrics,
}

/// Builder helper for constructing a [`PassNode`].
pub struct PassBuilder<'g, 'a> {
    graph: &'g mut RenderGraph<'a>,
    pass_id: PassId,
}

impl<'g, 'a> PassBuilder<'g, 'a> {
    /// Register a resource read dependency.
    pub fn read(self, id: ResourceId) -> Self {
        self.graph.passes[self.pass_id.0].reads.push(id);
        self
    }

    /// Register a resource write dependency.
    pub fn write(self, id: ResourceId) -> Self {
        self.graph.passes[self.pass_id.0].writes.push(id);
        self
    }

    /// Set whether the pass is active / enabled.
    pub fn with_enabled(self, enabled: bool) -> Self {
        self.graph.passes[self.pass_id.0].enabled = enabled;
        self
    }

    /// Mark pass as having external side effects (roots it during culling).
    pub fn with_side_effects(self, has_side_effects: bool) -> Self {
        self.graph.passes[self.pass_id.0].has_side_effects = has_side_effects;
        self
    }

    /// Provide a GPU command recording closure.
    pub fn execution<F>(self, f: F) -> Self
    where
        F: FnMut(&mut wgpu::CommandEncoder, &ResourceResolver<'_>) + 'a,
    {
        self.graph.passes[self.pass_id.0].execution = PassExecution::Gpu(Box::new(f));
        self
    }

    /// Provide a mock execution closure (useful for unit tests without GPU).
    pub fn execution_mock<F>(self, f: F) -> Self
    where
        F: FnMut(&ResourceResolver<'_>) + 'a,
    {
        self.graph.passes[self.pass_id.0].execution = PassExecution::Mock(Box::new(f));
        self
    }

    /// Returns the unique [`PassId`].
    pub fn id(&self) -> PassId {
        self.pass_id
    }
}

/// Physical memory pool for transient texture aliasing.
#[derive(Default)]
pub struct TransientMemoryPool {
    textures: Vec<(wgpu::Texture, wgpu::TextureView)>,
    descs: Vec<TextureDesc>,
}

impl TransientMemoryPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Prepare physical textures matching `required_descs`.
    /// Reuses existing textures across frames when descriptors match.
    pub fn prepare_textures(&mut self, device: &wgpu::Device, required_descs: &[TextureDesc]) {
        if self.textures.len() < required_descs.len() {
            self.textures
                .reserve(required_descs.len() - self.textures.len());
        }

        for (i, desc) in required_descs.iter().enumerate() {
            let needs_alloc = if i < self.descs.len() {
                self.descs[i] != *desc
            } else {
                true
            };

            if needs_alloc {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: desc
                        .label
                        .as_deref()
                        .or(Some("karakuri_transient_pool_texture")),
                    size: desc.size,
                    mip_level_count: desc.mip_level_count,
                    sample_count: desc.sample_count,
                    dimension: desc.dimension,
                    format: desc.format,
                    usage: desc.usage,
                    view_formats: &[],
                });
                let view = texture.create_view(&Default::default());
                if i < self.textures.len() {
                    self.textures[i] = (texture, view);
                    self.descs[i] = desc.clone();
                } else {
                    self.textures.push((texture, view));
                    self.descs.push(desc.clone());
                }
            }
        }
    }

    pub fn get_texture_and_view(
        &self,
        slot: usize,
    ) -> Option<(&wgpu::Texture, &wgpu::TextureView)> {
        self.textures.get(slot).map(|(t, v)| (t, v))
    }

    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }
}

/// Declarative Render Graph (DAG).
pub struct RenderGraph<'a> {
    resources: Vec<GraphResource>,
    passes: Vec<PassNode<'a>>,
    outputs: Vec<ResourceId>,
    pool: TransientMemoryPool,
}

impl<'a> Default for RenderGraph<'a> {
    fn default() -> Self {
        Self {
            resources: Vec::new(),
            passes: Vec::new(),
            outputs: Vec::new(),
            pool: TransientMemoryPool::new(),
        }
    }
}

impl<'a> RenderGraph<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a render graph with an existing persistent memory pool.
    pub fn with_pool(pool: TransientMemoryPool) -> Self {
        Self {
            pool,
            ..Default::default()
        }
    }

    pub fn pool(&self) -> &TransientMemoryPool {
        &self.pool
    }

    pub fn pool_mut(&mut self) -> &mut TransientMemoryPool {
        &mut self.pool
    }

    pub fn take_pool(self) -> TransientMemoryPool {
        self.pool
    }

    /// Create a transient texture resource managed by the graph.
    pub fn create_transient_texture(&mut self, desc: TextureDesc) -> ResourceId {
        let id = ResourceId(self.resources.len());
        self.resources.push(GraphResource::TransientTexture(desc));
        id
    }

    /// Create a transient buffer resource managed by the graph.
    pub fn create_transient_buffer(&mut self, desc: BufferDesc) -> ResourceId {
        let id = ResourceId(self.resources.len());
        self.resources.push(GraphResource::TransientBuffer(desc));
        id
    }

    /// Import an external texture view (e.g., presentation swapchain or persistent history).
    pub fn import_texture_view(&mut self, view: wgpu::TextureView) -> ResourceId {
        let id = ResourceId(self.resources.len());
        self.resources.push(GraphResource::ImportedTexture {
            texture: None,
            view,
            desc: None,
        });
        id
    }

    /// Import an external texture with its view.
    pub fn import_texture(
        &mut self,
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        desc: Option<TextureDesc>,
    ) -> ResourceId {
        let id = ResourceId(self.resources.len());
        self.resources.push(GraphResource::ImportedTexture {
            texture: Some(texture),
            view,
            desc,
        });
        id
    }

    /// Import an external buffer.
    pub fn import_buffer(&mut self, buffer: wgpu::Buffer) -> ResourceId {
        let id = ResourceId(self.resources.len());
        self.resources.push(GraphResource::ImportedBuffer(buffer));
        id
    }

    /// Mark a resource as a required output target (roots the graph for dead pass culling).
    pub fn mark_output(&mut self, id: ResourceId) {
        if !self.outputs.contains(&id) {
            self.outputs.push(id);
        }
    }

    /// Add a new pass to the render graph.
    pub fn add_pass<'g>(&'g mut self, label: impl Into<String>) -> PassBuilder<'g, 'a> {
        let pass_id = PassId(self.passes.len());
        let node = PassNode {
            id: pass_id,
            label: label.into(),
            reads: Vec::new(),
            writes: Vec::new(),
            enabled: true,
            has_side_effects: false,
            execution: PassExecution::None,
        };
        self.passes.push(node);
        PassBuilder {
            graph: self,
            pass_id,
        }
    }

    /// Compile the graph:
    /// 1. Validates resource references.
    /// 2. Performs dead pass culling via backward reachability.
    /// 3. Builds DAG and executes topological sort with cycle detection.
    /// 4. Analyzes transient resource lifetimes `[first_pass, last_pass]`.
    /// 5. Solves transient memory aliasing into minimal physical texture slots.
    pub fn compile(&self) -> Result<CompiledGraph, GraphError> {
        // 1. Validate resource references
        for pass in &self.passes {
            for &read_id in &pass.reads {
                if read_id.0 >= self.resources.len() {
                    return Err(GraphError::ResourceNotFound(read_id));
                }
            }
            for &write_id in &pass.writes {
                if write_id.0 >= self.resources.len() {
                    return Err(GraphError::ResourceNotFound(write_id));
                }
            }
        }
        for &out_id in &self.outputs {
            if out_id.0 >= self.resources.len() {
                return Err(GraphError::ResourceNotFound(out_id));
            }
        }

        // 2. Dead Pass Culling (backward reachability)
        let mut active_passes: HashSet<PassId> = HashSet::new();
        let mut needed_resources: HashSet<ResourceId> = HashSet::new();

        if !self.outputs.is_empty() {
            for &out_res in &self.outputs {
                needed_resources.insert(out_res);
            }
        }

        for pass in &self.passes {
            if pass.has_side_effects && pass.enabled {
                active_passes.insert(pass.id);
                for &res in &pass.reads {
                    needed_resources.insert(res);
                }
            }
        }

        if self.outputs.is_empty() && active_passes.is_empty() {
            // If no explicit outputs or side effects, all enabled passes are active candidates
            for pass in &self.passes {
                if pass.enabled {
                    active_passes.insert(pass.id);
                }
            }
        } else {
            let mut worklist: VecDeque<ResourceId> = needed_resources.into_iter().collect();
            let mut visited_resources: HashSet<ResourceId> = HashSet::new();

            while let Some(res_id) = worklist.pop_front() {
                if !visited_resources.insert(res_id) {
                    continue;
                }

                let mut found_writer = false;
                for pass in &self.passes {
                    if pass.writes.contains(&res_id) {
                        found_writer = true;
                        if pass.enabled && active_passes.insert(pass.id) {
                            for &in_res in &pass.reads {
                                worklist.push_back(in_res);
                            }
                        }
                    }
                }

                if !found_writer && self.outputs.contains(&res_id) {
                    if let Some(
                        GraphResource::TransientTexture(_) | GraphResource::TransientBuffer(_),
                    ) = self.resources.get(res_id.0)
                    {
                        return Err(GraphError::OutputNotProduced(res_id));
                    }
                }
            }
        }

        // Ensure active passes do not read transient resources that have no active producer
        for pass in &self.passes {
            if active_passes.contains(&pass.id) {
                for &read_res in &pass.reads {
                    if matches!(
                        self.resources.get(read_res.0),
                        Some(
                            GraphResource::TransientTexture(_) | GraphResource::TransientBuffer(_)
                        )
                    ) {
                        let has_active_writer = self
                            .passes
                            .iter()
                            .any(|p| active_passes.contains(&p.id) && p.writes.contains(&read_res));
                        if !has_active_writer {
                            return Err(GraphError::Execution(format!(
                                "Pass '{}' reads transient resource {:?}, but no active enabled pass writes it",
                                pass.label, read_res
                            )));
                        }
                    }
                }
            }
        }

        let mut culled_passes: Vec<PassId> = Vec::new();
        for pass in &self.passes {
            if !active_passes.contains(&pass.id) {
                culled_passes.push(pass.id);
            }
        }

        // 3. Dependency Graph Construction on Active Passes
        let mut adj: HashMap<PassId, Vec<PassId>> = HashMap::new();
        let mut in_degree: HashMap<PassId, usize> = HashMap::new();

        for &id in &active_passes {
            adj.insert(id, Vec::new());
            in_degree.insert(id, 0);
        }

        for res_idx in 0..self.resources.len() {
            let res_id = ResourceId(res_idx);

            let active_writers: Vec<PassId> = self
                .passes
                .iter()
                .filter(|p| active_passes.contains(&p.id) && p.writes.contains(&res_id))
                .map(|p| p.id)
                .collect();

            let active_readers: Vec<PassId> = self
                .passes
                .iter()
                .filter(|p| active_passes.contains(&p.id) && p.reads.contains(&res_id))
                .map(|p| p.id)
                .collect();

            if active_writers.len() == 1 {
                // Single producer case: all readers must execute after writer
                let writer = active_writers[0];
                for &reader in &active_readers {
                    if reader != writer {
                        let nexts = adj.get_mut(&writer).unwrap();
                        if !nexts.contains(&reader) {
                            nexts.push(reader);
                            *in_degree.get_mut(&reader).unwrap() += 1;
                        }
                    }
                }
            } else if active_writers.len() > 1 {
                // Multiple writers: write sequentially in registration order
                for i in 0..active_writers.len() - 1 {
                    let w1 = active_writers[i];
                    let w2 = active_writers[i + 1];
                    let nexts = adj.get_mut(&w1).unwrap();
                    if !nexts.contains(&w2) {
                        nexts.push(w2);
                        *in_degree.get_mut(&w2).unwrap() += 1;
                    }
                }

                // Associate readers with the latest writer registered before them
                for &reader in &active_readers {
                    if let Some(&prev_w) = active_writers
                        .iter()
                        .take_while(|&&w| w.0 < reader.0)
                        .last()
                    {
                        if prev_w != reader {
                            let nexts = adj.get_mut(&prev_w).unwrap();
                            if !nexts.contains(&reader) {
                                nexts.push(reader);
                                *in_degree.get_mut(&reader).unwrap() += 1;
                            }
                        }
                    } else if let Some(&first_w) = active_writers.first() {
                        if first_w != reader {
                            let nexts = adj.get_mut(&first_w).unwrap();
                            if !nexts.contains(&reader) {
                                nexts.push(reader);
                                *in_degree.get_mut(&reader).unwrap() += 1;
                            }
                        }
                    }

                    // And any writer registered after reader must execute after reader
                    if let Some(&next_w) = active_writers.iter().find(|&&w| w.0 > reader.0) {
                        if next_w != reader {
                            let nexts = adj.get_mut(&reader).unwrap();
                            if !nexts.contains(&next_w) {
                                nexts.push(next_w);
                                *in_degree.get_mut(&next_w).unwrap() += 1;
                            }
                        }
                    }
                }
            }
        }

        // Topological Sort (Kahn's algorithm with deterministic tie-breaking)
        let mut ready: BTreeSet<PassId> = in_degree
            .iter()
            .filter_map(|(&id, &deg)| if deg == 0 { Some(id) } else { None })
            .collect();

        let mut order: Vec<PassId> = Vec::with_capacity(active_passes.len());

        while let Some(&pass_id) = ready.iter().next() {
            ready.remove(&pass_id);
            order.push(pass_id);

            if let Some(neighbors) = adj.get(&pass_id) {
                for &next in neighbors {
                    let deg = in_degree.get_mut(&next).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        ready.insert(next);
                    }
                }
            }
        }

        if order.len() != active_passes.len() {
            let unvisited: Vec<String> = active_passes
                .iter()
                .filter(|id| !order.contains(id))
                .map(|id| format!("'{}' ({})", self.passes[id.0].label, id))
                .collect();
            return Err(GraphError::CycleDetected(format!(
                "Dependency cycle involving passes: {}",
                unvisited.join(", ")
            )));
        }

        // 4. Lifetime Analysis
        let mut lifetimes: HashMap<ResourceId, (usize, usize)> = HashMap::new();

        for (order_idx, &pass_id) in order.iter().enumerate() {
            let pass = &self.passes[pass_id.0];
            for &res_id in pass.reads.iter().chain(pass.writes.iter()) {
                if matches!(
                    self.resources.get(res_id.0),
                    Some(GraphResource::TransientTexture(_) | GraphResource::TransientBuffer(_))
                ) {
                    lifetimes
                        .entry(res_id)
                        .and_modify(|lt| lt.1 = order_idx)
                        .or_insert((order_idx, order_idx));
                }
            }
        }

        // 5. Transient Memory Aliasing
        let mut transient_textures: Vec<(ResourceId, TextureDesc, (usize, usize))> = Vec::new();
        for (&res_id, &lifetime) in &lifetimes {
            if let Some(GraphResource::TransientTexture(desc)) = self.resources.get(res_id.0) {
                transient_textures.push((res_id, desc.clone(), lifetime));
            }
        }

        // Sort deterministically: start lifetime, end lifetime, ResourceId
        transient_textures.sort_by_key(|(id, _, lt)| (lt.0, lt.1, id.0));

        struct PhysicalSlotDesc {
            desc: TextureDesc,
            intervals: Vec<(usize, usize)>,
        }

        let mut physical_slots: Vec<PhysicalSlotDesc> = Vec::new();
        let mut aliasing_map: HashMap<ResourceId, usize> = HashMap::new();

        for (res_id, desc, lifetime) in &transient_textures {
            let mut assigned_slot = None;

            for (slot_idx, slot) in physical_slots.iter_mut().enumerate() {
                if slot.desc == *desc {
                    let overlaps = slot
                        .intervals
                        .iter()
                        .any(|&(s, e)| lifetime.0 <= e && s <= lifetime.1);
                    if !overlaps {
                        slot.intervals.push(*lifetime);
                        assigned_slot = Some(slot_idx);
                        break;
                    }
                }
            }

            let slot_idx = match assigned_slot {
                Some(idx) => idx,
                None => {
                    let new_idx = physical_slots.len();
                    physical_slots.push(PhysicalSlotDesc {
                        desc: desc.clone(),
                        intervals: vec![*lifetime],
                    });
                    new_idx
                }
            };

            aliasing_map.insert(*res_id, slot_idx);
        }

        let total_virtual_bytes: u64 = transient_textures
            .iter()
            .map(|(_, desc, _)| desc.estimated_byte_size())
            .sum();

        let total_physical_bytes: u64 = physical_slots
            .iter()
            .map(|s| s.desc.estimated_byte_size())
            .sum();

        let vram_saved_bytes = total_virtual_bytes.saturating_sub(total_physical_bytes);

        let metrics = GraphMetrics {
            allocated_physical_textures: physical_slots.len(),
            virtual_transient_textures: transient_textures.len(),
            vram_saved_bytes,
            total_passes: self.passes.len(),
            executed_passes: order.len(),
            culled_passes: culled_passes.len(),
        };

        let physical_slot_descs = physical_slots.into_iter().map(|s| s.desc).collect();

        Ok(CompiledGraph {
            pass_order: order,
            culled_passes,
            lifetimes,
            physical_slots: aliasing_map,
            physical_slot_descs,
            metrics,
        })
    }

    /// Execute the compiled render graph onto a real GPU command encoder.
    pub fn execute(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<GraphMetrics, GraphError> {
        let compiled = self.compile()?;
        self.pool
            .prepare_textures(device, &compiled.physical_slot_descs);

        let mut views: HashMap<ResourceId, &wgpu::TextureView> = HashMap::new();
        let mut textures: HashMap<ResourceId, &wgpu::Texture> = HashMap::new();
        let mut buffers: HashMap<ResourceId, &wgpu::Buffer> = HashMap::new();

        for (&res_id, &slot_idx) in &compiled.physical_slots {
            if let Some((tex, view)) = self.pool.get_texture_and_view(slot_idx) {
                textures.insert(res_id, tex);
                views.insert(res_id, view);
            }
        }

        for (idx, res) in self.resources.iter().enumerate() {
            let res_id = ResourceId(idx);
            match res {
                GraphResource::ImportedTexture { texture, view, .. } => {
                    views.insert(res_id, view);
                    if let Some(tex) = texture {
                        textures.insert(res_id, tex);
                    }
                }
                GraphResource::ImportedBuffer(buf) => {
                    buffers.insert(res_id, buf);
                }
                _ => {}
            }
        }

        let resolver = ResourceResolver {
            views,
            textures,
            buffers,
            physical_slots: compiled.physical_slots.clone(),
        };

        for &pass_id in &compiled.pass_order {
            let pass = &mut self.passes[pass_id.0];
            match &mut pass.execution {
                PassExecution::Gpu(ref mut f) => f(encoder, &resolver),
                PassExecution::Mock(ref mut f) => f(&resolver),
                PassExecution::None => {}
            }
        }

        Ok(compiled.metrics)
    }

    /// Execute the render graph in mock mode (no GPU device required).
    pub fn execute_mock(&mut self) -> Result<GraphMetrics, GraphError> {
        let compiled = self.compile()?;
        let resolver = ResourceResolver::mock(&compiled.physical_slots);

        for &pass_id in &compiled.pass_order {
            let pass = &mut self.passes[pass_id.0];
            match &mut pass.execution {
                PassExecution::Gpu(_) => {}
                PassExecution::Mock(ref mut f) => f(&resolver),
                PassExecution::None => {}
            }
        }

        Ok(compiled.metrics)
    }
}
