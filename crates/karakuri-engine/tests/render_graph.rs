//! Integration tests for Transient Render Graph (DAG & Memory Aliasing).

use karakuri_engine::graph::{BufferDesc, GraphError, RenderGraph, TextureDesc};

#[test]
fn topological_sorting_orders_passes_by_read_after_write_dependencies() {
    let mut graph = RenderGraph::new();

    let t_a = graph.create_transient_texture(TextureDesc::d2(
        1920,
        1080,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    ));
    let t_b = graph.create_transient_texture(TextureDesc::d2(
        1920,
        1080,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    ));
    let t_c = graph.create_transient_texture(TextureDesc::d2(
        1920,
        1080,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    ));

    // Intentionally register passes out of dependency order:
    // Pass 3 (reads C)
    // Pass 1 (reads A, writes B)
    // Pass 2 (reads B, writes C)
    // Pass 0 (writes A)
    let p3 = graph.add_pass("OutputConsumer").read(t_c).id();
    let p1 = graph.add_pass("FilterB").read(t_a).write(t_b).id();
    let p2 = graph.add_pass("FilterC").read(t_b).write(t_c).id();
    let p0 = graph.add_pass("BaseGeometry").write(t_a).id();

    let compiled = graph.compile().expect("graph should compile cleanly");

    // Dependency chain is: p0 -> p1 -> p2 -> p3
    assert_eq!(
        compiled.pass_order,
        vec![p0, p1, p2, p3],
        "Topological sort must order passes respecting read-after-write dependencies"
    );
    assert_eq!(compiled.metrics.executed_passes, 4);
    assert_eq!(compiled.metrics.culled_passes, 0);
}

#[test]
fn cycle_detection_returns_error_when_passes_form_dependency_loop() {
    let mut graph = RenderGraph::new();

    let res_x = graph.create_transient_texture(TextureDesc::d2(
        256,
        256,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    ));
    let res_y = graph.create_transient_texture(TextureDesc::d2(
        256,
        256,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    ));

    // Pass Alpha writes X, reads Y
    graph.add_pass("PassAlpha").write(res_x).read(res_y);
    // Pass Beta writes Y, reads X
    graph.add_pass("PassBeta").write(res_y).read(res_x);

    let result = graph.compile();
    match result {
        Err(GraphError::CycleDetected(msg)) => {
            assert!(
                msg.contains("PassAlpha") || msg.contains("PassBeta"),
                "Error message should mention passes in cycle: {msg}"
            );
        }
        other => panic!("Expected CycleDetected error, got: {:?}", other),
    }
}

#[test]
fn dead_pass_culling_removes_dangling_and_disabled_passes() {
    let mut graph = RenderGraph::new();

    let t_geom = graph.create_transient_texture(TextureDesc::d2(
        512,
        512,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    ));
    let t_dangling = graph.create_transient_texture(TextureDesc::d2(
        512,
        512,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    ));
    let t_output = graph.create_transient_texture(TextureDesc::d2(
        512,
        512,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    ));

    // Contributing chain
    let p_geom = graph.add_pass("Geometry").write(t_geom).id();
    let p_tone = graph.add_pass("ToneMap").read(t_geom).write(t_output).id();

    // Dangling passes never read by output
    let p_dead1 = graph.add_pass("Dangling1").write(t_dangling).id();
    let p_dead2 = graph
        .add_pass("Dangling2")
        .read(t_dangling)
        .write(t_dangling)
        .id();

    // Muted / disabled pass
    let p_disabled = graph
        .add_pass("MutedPass")
        .with_enabled(false)
        .write(t_output)
        .id();

    graph.mark_output(t_output);

    let compiled = graph.compile().expect("graph should compile");

    assert_eq!(
        compiled.pass_order,
        vec![p_geom, p_tone],
        "Only passes contributing to output must be scheduled"
    );
    assert!(compiled.culled_passes.contains(&p_dead1));
    assert!(compiled.culled_passes.contains(&p_dead2));
    assert!(compiled.culled_passes.contains(&p_disabled));
    assert_eq!(compiled.metrics.culled_passes, 3);
    assert_eq!(compiled.metrics.executed_passes, 2);
    assert_eq!(compiled.metrics.total_passes, 5);
}

#[test]
fn side_effect_passes_are_retained_even_without_marked_output() {
    let mut graph = RenderGraph::new();

    let t_buf = graph.create_transient_buffer(BufferDesc::new(
        1024,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    ));

    let p_probe = graph
        .add_pass("GpuCostProbe")
        .write(t_buf)
        .with_side_effects(true)
        .id();

    let p_dangling = graph.add_pass("DeadPass").id();

    // Mark an output on a dummy buffer to trigger dead pass culling
    let t_out = graph.create_transient_buffer(BufferDesc::new(256, wgpu::BufferUsages::COPY_DST));
    let p_out = graph.add_pass("OutputProducer").write(t_out).id();
    graph.mark_output(t_out);

    let compiled = graph.compile().expect("graph should compile");

    assert!(
        compiled.pass_order.contains(&p_probe),
        "Side effect pass must be retained"
    );
    assert!(
        compiled.pass_order.contains(&p_out),
        "Output producer must be retained"
    );
    assert!(
        compiled.culled_passes.contains(&p_dangling),
        "Dangling pass without side-effects must be culled"
    );
}

#[test]
fn non_overlapping_transient_textures_share_physical_pool_slot() {
    let mut graph = RenderGraph::new();

    let desc = TextureDesc::d2(
        1920,
        1080,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    );

    // Texture A: used in passes 0 and 1 (lifetime [0, 1])
    let tex_a = graph.create_transient_texture(desc.clone());
    // Texture B: used in passes 2 and 3 (lifetime [2, 3])
    let tex_b = graph.create_transient_texture(desc.clone());
    // Texture C: used in passes 4 and 5 (lifetime [4, 5])
    let tex_c = graph.create_transient_texture(desc.clone());

    let out = graph.create_transient_texture(TextureDesc::d2(
        1920,
        1080,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    ));

    let bridge1 = graph.create_transient_buffer(BufferDesc::new(64, wgpu::BufferUsages::STORAGE));
    let bridge2 = graph.create_transient_buffer(BufferDesc::new(64, wgpu::BufferUsages::STORAGE));

    graph.add_pass("Pass0").write(tex_a);
    graph.add_pass("Pass1").read(tex_a).write(bridge1);
    graph.add_pass("Pass2").read(bridge1).write(tex_b);
    graph.add_pass("Pass3").read(tex_b).write(bridge2);
    graph.add_pass("Pass4").read(bridge2).write(tex_c);
    graph.add_pass("Pass5").read(tex_c).write(out);

    graph.mark_output(out);

    let compiled = graph.compile().expect("graph should compile");

    // All 3 non-overlapping transient textures must map to the same physical slot 0!
    let slot_a = compiled.physical_slots.get(&tex_a).copied();
    let slot_b = compiled.physical_slots.get(&tex_b).copied();
    let slot_c = compiled.physical_slots.get(&tex_c).copied();

    assert_eq!(slot_a, Some(0));
    assert_eq!(slot_b, Some(0));
    assert_eq!(slot_c, Some(0));

    assert_eq!(
        compiled.metrics.allocated_physical_textures,
        2, // 1 for Rgba16Float pool + 1 for Rgba8Unorm out
        "Tex A, B, and C must share 1 physical slot"
    );
    assert_eq!(compiled.metrics.virtual_transient_textures, 4);

    // VRAM savings: 3 virtual Rgba16Float textures (1920*1080*8 = 16,588,800 bytes each)
    // aliased into 1 physical texture saves 2 * 16,588,800 = 33,177,600 bytes!
    let single_tex_bytes = desc.estimated_byte_size();
    assert_eq!(single_tex_bytes, 1920 * 1080 * 8);
    assert_eq!(compiled.metrics.vram_saved_bytes, 2 * single_tex_bytes);
}

#[test]
fn overlapping_transient_textures_allocate_distinct_physical_slots() {
    let mut graph = RenderGraph::new();

    let desc = TextureDesc::d2(
        1920,
        1080,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    );

    // Both Tex A and Tex B are accessed together in Pass 0 and Pass 1:
    // Lifetimes overlap completely!
    let tex_a = graph.create_transient_texture(desc.clone());
    let tex_b = graph.create_transient_texture(desc.clone());

    let out = graph.create_transient_texture(TextureDesc::d2(
        1920,
        1080,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    ));

    graph.add_pass("Pass0_Produce").write(tex_a).write(tex_b);
    graph
        .add_pass("Pass1_Consume")
        .read(tex_a)
        .read(tex_b)
        .write(out);

    graph.mark_output(out);

    let compiled = graph.compile().expect("graph should compile");

    let slot_a = compiled.physical_slots.get(&tex_a).copied().unwrap();
    let slot_b = compiled.physical_slots.get(&tex_b).copied().unwrap();

    assert_ne!(
        slot_a, slot_b,
        "Overlapping transient textures must NOT share the same physical slot"
    );
    assert_eq!(
        compiled.metrics.allocated_physical_textures,
        3, // slot for A, slot for B, slot for out
        "No aliasing possible between overlapping textures"
    );
    assert_eq!(compiled.metrics.vram_saved_bytes, 0);
}

#[test]
fn alternating_transient_texture_lifetimes_alias_optimally() {
    let mut graph = RenderGraph::new();

    let desc = TextureDesc::d2(
        1024,
        1024,
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    );

    // Pass 0: writes A
    // Pass 1: reads A, writes B (A and B overlap during pass 1)
    // Pass 2: reads B, writes C (B and C overlap during pass 2; A [0,1] does NOT overlap C [2,3])
    // Pass 3: reads C, writes Out
    let tex_a = graph.create_transient_texture(desc.clone());
    let tex_b = graph.create_transient_texture(desc.clone());
    let tex_c = graph.create_transient_texture(desc.clone());
    let out = graph.create_transient_texture(desc.clone());

    graph.add_pass("Pass0").write(tex_a);
    graph.add_pass("Pass1").read(tex_a).write(tex_b);
    graph.add_pass("Pass2").read(tex_b).write(tex_c);
    graph.add_pass("Pass3").read(tex_c).write(out);

    graph.mark_output(out);

    let compiled = graph.compile().expect("graph should compile");

    let slot_a = compiled.physical_slots.get(&tex_a).copied().unwrap();
    let slot_b = compiled.physical_slots.get(&tex_b).copied().unwrap();
    let slot_c = compiled.physical_slots.get(&tex_c).copied().unwrap();
    let slot_out = compiled.physical_slots.get(&out).copied().unwrap();

    // A and B overlap at pass index 1
    assert_ne!(slot_a, slot_b);
    // B and C overlap at pass index 2
    assert_ne!(slot_b, slot_c);
    // A [0, 1] and C [2, 3] do not overlap! They should alias to the same slot
    assert_eq!(slot_a, slot_c, "Tex A and Tex C must share physical slot");
    // Out [3, 3] does not overlap with B [1, 2], so Out can reuse B's slot
    assert_eq!(slot_b, slot_out, "Out and Tex B must share physical slot");

    // 4 virtual textures packed into 2 physical slots!
    assert_eq!(compiled.metrics.allocated_physical_textures, 2);
    assert_eq!(compiled.metrics.virtual_transient_textures, 4);
    assert_eq!(
        compiled.metrics.vram_saved_bytes,
        2 * desc.estimated_byte_size()
    );
}

#[test]
fn execute_mock_runs_pass_callbacks_in_scheduled_order() {
    use std::sync::{Arc, Mutex};

    let mut graph = RenderGraph::new();

    let t_a = graph.create_transient_texture(TextureDesc::d2(
        128,
        128,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    ));
    let t_b = graph.create_transient_texture(TextureDesc::d2(
        128,
        128,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    ));

    let execution_log = Arc::new(Mutex::new(Vec::new()));

    let log1 = Arc::clone(&execution_log);
    graph
        .add_pass("PassSecond")
        .read(t_a)
        .write(t_b)
        .execution_mock(move |resolver| {
            assert!(resolver.is_available(t_a));
            assert!(resolver.is_available(t_b));
            log1.lock().unwrap().push("PassSecond");
        });

    let log0 = Arc::clone(&execution_log);
    graph
        .add_pass("PassFirst")
        .write(t_a)
        .execution_mock(move |resolver| {
            assert!(resolver.is_available(t_a));
            log0.lock().unwrap().push("PassFirst");
        });

    graph.mark_output(t_b);

    let metrics = graph.execute_mock().expect("execute_mock should succeed");
    assert_eq!(metrics.executed_passes, 2);

    let log = execution_log.lock().unwrap().clone();
    assert_eq!(
        log,
        vec!["PassFirst", "PassSecond"],
        "Pass callbacks must run in topological order"
    );
}

mod gpu {
    use super::*;
    use karakuri_engine::Gpu;

    #[test]
    fn execute_on_gpu_records_commands_and_allocates_transient_pool() {
        let gpu = Gpu::headless().expect("no GPU available");

        let mut graph = RenderGraph::new();

        let desc = TextureDesc::d2(
            256,
            256,
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        );

        let t_a = graph.create_transient_texture(desc.clone());
        let t_b = graph.create_transient_texture(desc.clone());

        // Pass 0: clears t_a
        graph
            .add_pass("ClearPassA")
            .write(t_a)
            .execution(move |encoder, resolver| {
                let view_a = resolver
                    .texture_view(t_a)
                    .expect("texture view A must exist");
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("clear_a"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: view_a,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
            });

        // Pass 1: clears t_b
        graph
            .add_pass("ClearPassB")
            .read(t_a)
            .write(t_b)
            .execution(move |encoder, resolver| {
                let view_b = resolver
                    .texture_view(t_b)
                    .expect("texture view B must exist");
                let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("clear_b"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: view_b,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
            });

        graph.mark_output(t_b);

        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        let metrics = graph
            .execute(&gpu.device, &mut encoder)
            .expect("graph execution on GPU should succeed");

        assert_eq!(metrics.executed_passes, 2);
        // Submit recorded commands to verify no validation errors in wgpu
        gpu.queue.submit([encoder.finish()]);

        // Second execution should reuse pool without new allocations
        let mut encoder2 = gpu.device.create_command_encoder(&Default::default());
        let metrics2 = graph
            .execute(&gpu.device, &mut encoder2)
            .expect("second execution should reuse pool");
        assert_eq!(metrics2.executed_passes, 2);
        gpu.queue.submit([encoder2.finish()]);
    }
}
