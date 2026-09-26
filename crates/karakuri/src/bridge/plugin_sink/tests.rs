use super::*;

mod gpu {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn macos_surface_creates_and_allocates_iosurface_id() {
        let gpu = Gpu::headless().expect("no GPU");
        let surface = macos::MacOsSurface::new(
            &gpu,
            640,
            480,
            wgpu::TextureFormat::Bgra8Unorm.add_srgb_suffix(),
        )
        .expect("MacOsSurface");
        assert!(surface.surface_id > 0, "IOSurfaceID must be non-zero");
        assert_eq!(surface.width, 640);
        assert_eq!(surface.height, 480);

        // Verify lookup by ID succeeds (cross-process lookup simulation)
        unsafe {
            let looked_up = macos::IOSurfaceLookup(surface.surface_id);
            assert!(
                !looked_up.is_null(),
                "IOSurfaceLookup must succeed for globally shared IOSurface"
            );
            macos::CFRelease(looked_up as *const std::ffi::c_void);
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn windows_surface_creates_and_allocates_dxgi_shared_handle() {
        let Ok(gpu) = Gpu::headless() else {
            eprintln!("headless GPU initialization failed; skipping");
            return;
        };
        println!(
            "Running windows_surface test on backend: {:?}",
            gpu.adapter.get_info().backend
        );
        println!("Adapter name: {}", gpu.adapter.get_info().name);
        println!(
            "Adapter has VULKAN_EXTERNAL_MEMORY_WIN32: {}",
            gpu.adapter
                .features()
                .contains(wgpu::Features::VULKAN_EXTERNAL_MEMORY_WIN32)
        );
        println!(
            "Device has VULKAN_EXTERNAL_MEMORY_WIN32: {}",
            gpu.device
                .features()
                .contains(wgpu::Features::VULKAN_EXTERNAL_MEMORY_WIN32)
        );
        let surface = windows::WindowsSurface::new(
            &gpu,
            640,
            480,
            wgpu::TextureFormat::Bgra8Unorm.add_srgb_suffix(),
        )
        .expect("WindowsSurface");
        assert!(
            surface.surface_id > 0,
            "DXGI shared handle must be non-zero"
        );
        assert_eq!(surface.width, 640);
        assert_eq!(surface.height, 480);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn plugin_sink_integration_with_karakuri_syphon_binary() {
        let candidates = [
            "../Karakuri-syphon/target/debug/karakuri-syphon",
            "../../Karakuri-syphon/target/debug/karakuri-syphon",
            "../../../Karakuri-syphon/target/debug/karakuri-syphon",
            "/Users/toyoshim/Work/GitHub/Karakuri/Karakuri-syphon/target/debug/karakuri-syphon",
        ];
        let mut command = None;
        for c in candidates {
            let p = std::path::Path::new(c);
            if p.exists() {
                command = Some(
                    p.canonicalize()
                        .unwrap_or_else(|_| p.to_path_buf())
                        .to_string_lossy()
                        .into_owned(),
                );
                break;
            }
        }
        let Some(command) = command else {
            return;
        };

        let gpu = Gpu::headless().expect("no GPU");
        let mut sink = PluginSink::open(
            &gpu,
            &command,
            1280,
            720,
            wgpu::TextureFormat::Bgra8Unorm.add_srgb_suffix(),
        )
        .expect("PluginSink::open");
        assert!(sink.is_alive());
        assert_eq!(sink.server_name(), "Karakuri");
        assert_eq!(sink.size(), (1280, 720));

        let present = karakuri_engine::Present::new(
            &gpu.device,
            wgpu::TextureFormat::Bgra8Unorm.add_srgb_suffix(),
            1280,
            720,
        );
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        present.draw(&mut encoder, sink.view(), sink.size());
        sink.after_draw(&mut encoder);
        gpu.queue.submit([encoder.finish()]);

        sink.acquire(&gpu).expect("acquire");
        sink.present(&gpu).expect("present frame 0");

        let t = sink.telemetry();
        assert_eq!(t.host_dropped, 0);

        drop(sink);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn plugin_sink_integration_with_karakuri_spout_binary() {
        let candidates = [
            "../Karakuri-spout/target/debug/karakuri-spout.exe",
            "../../Karakuri-spout/target/debug/karakuri-spout.exe",
            "../../../Karakuri-spout/target/debug/karakuri-spout.exe",
        ];
        let mut command = None;
        for c in candidates {
            let p = std::path::Path::new(c);
            if p.exists() {
                command = Some(
                    p.canonicalize()
                        .unwrap_or_else(|_| p.to_path_buf())
                        .to_string_lossy()
                        .into_owned(),
                );
                break;
            }
        }
        let Some(command) = command else {
            return;
        };

        std::env::remove_var("WGPU_BACKEND");
        let Ok(gpu) = Gpu::headless() else {
            return;
        };
        println!(
            "Running Spout integration test with backend: {:?}",
            gpu.adapter.get_info().backend
        );

        let mut sink = PluginSink::open(
            &gpu,
            &command,
            1280,
            720,
            wgpu::TextureFormat::Bgra8Unorm.add_srgb_suffix(),
        )
        .expect("PluginSink::open");
        assert!(sink.is_alive());
        assert_eq!(sink.server_name(), "Karakuri");
        assert_eq!(sink.size(), (1280, 720));

        let _present = karakuri_engine::Present::new(
            &gpu.device,
            wgpu::TextureFormat::Bgra8Unorm.add_srgb_suffix(),
            1280,
            720,
        );
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("test red clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: sink.view(),
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }
        sink.after_draw(&mut encoder);
        gpu.queue.submit([encoder.finish()]);

        sink.acquire(&gpu).expect("acquire");
        sink.present(&gpu).expect("present frame 0");

        let t = sink.telemetry();
        assert_eq!(t.host_dropped, 0);

        drop(sink);
    }
}
