//! Shared fixtures and helpers for GPU tests.

#![allow(unused_imports)]

pub(super) use super::super::tests::scratch_dir;
pub(super) use super::super::*;
pub(super) use karakuri_engine::letterbox;
pub(super) use std::time::{Duration, Instant};

pub(super) use karakuri_console::focus::Step;
pub(super) use karakuri_console::input::Claim;
pub(super) use karakuri_console::panel::Panel;
pub(super) use karakuri_console::view::{
    look as look_row, mixer as mixer_bay, picture_rect, preview_rects, tracker_group, Picture,
    Reading, RowKind, Scope, Tracker, TransitionSettings, View, DECKS, SCRUB_BEATS, SYNCS,
};
pub(super) use karakuri_console::{egui, egui_wgpu};
pub(super) use karakuri_engine::set::Layering;
pub(super) use karakuri_engine::transport::Sync as EngineSync;
pub(super) use karakuri_engine::{
    compose, Blend, Committed, Cut, Deck, DeckSlot as EngineSlot, Event, Gpu, Look, Mask, MaskKind,
    Residency, Sink, Skip, TonemapOp,
};
pub(super) use karakuri_environment::{audio, mix, setfile, watch, Asked, Opening};
pub(super) use karakuri_layout::{Layout, Point};
pub(super) use karakuri_operation::{
    BeatSource, BlendMode, GridScale, Operation, SetTransfer, Undecided,
};
pub(super) use karakuri_operation_record::{
    not_performed, written, Current, Owed, Silent, Written,
};
pub(super) use karakuri_store::record::{DeckSlot, Record};
pub(super) use karakuri_store::store::Store;
pub(super) use winit::event::WindowEvent;

/// What a frame these tests compose advances by.
///
/// A stated count rather than a measured one, and it is honest here for the
/// reason it was not in the live path: these frames are composed to assert what
/// was *drawn* — a cell that took a pass, the rectangle it was aimed at — and
/// they time nothing at all, so this is a fixture rather than a measurement
/// withheld. The live count is `App::clock`, which measures the interval and
/// writes it into the `tick` (ADR-0297).
pub(super) const STEPS_A_FRAME: u8 = 1;

/// The picture format a headless harness hands [`Engine::new`].
///
/// A stand-in, and it is named here once rather than at nineteen call sites: a
/// run reads its picture format off the console's own surface (`Gfx::
/// picture_format`) and there is no surface here, so these tests name an sRGB
/// 8-bit format every backend can make a texture in. What is asserted is never
/// the value — `a_picture_format_is_a_value_read_off_a_surface` in `mod tests`
/// is what holds the source to one derivation.
pub(super) const HEADLESS_PICTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// A [`Keeping`] with nothing served and nothing yet built, which is what a run
/// holds on its first frame.
pub(super) fn keeping() -> Keeping {
    let (_, built) = std::sync::mpsc::channel();
    let (save_tx, saves) = std::sync::mpsc::channel();
    let (send_tx, sends) = std::sync::mpsc::channel();
    let (keep_tx, keeps) = std::sync::mpsc::channel();
    Keeping {
        mcp: None,
        playing: Playing {
            playing: Vec::new(),
        },
        built,
        pending: Vec::new(),
        saves,
        save_tx,
        sends,
        send_tx,
        keeps,
        keep_tx,
        in_flight: 0,
    }
}

/// One request, one reply, over TCP exactly as a client would — the shape
/// `karakuri-environment/src/mcp.rs`'s own `wire_tests` use, restated here
/// because that module is `#[cfg(test)]` and nothing outside it can call in.
///
/// Over a socket, because that is the only way to read a [`mcp::Reporter`]
/// back. A report handed to the server goes into a queue only the protocol can
/// drain, which is exactly the property under test: a swap the lane drew is a
/// swap a model can ask about.
pub(super) fn call(port: u16, name: &str, args: serde_json::Value) -> (bool, String) {
    use std::io::{BufRead, Write};
    let body = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
                                  "params":{"name":name,"arguments":args}})
    .to_string();
    let request = format!(
        "POST / HTTP/1.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("a read timeout, so a wedged server fails as a timeout");
    stream.write_all(request.as_bytes()).expect("write");
    let mut reader = std::io::BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).expect("no status line");
    let mut length = 0usize;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).expect("header");
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
    }
    let mut body = vec![0u8; length];
    std::io::Read::read_exact(&mut reader, &mut body).expect("body");
    let reply: serde_json::Value = serde_json::from_slice(&body).expect("the answer is not JSON");
    let result = &reply["result"];
    (
        result["isError"].as_bool().unwrap_or(true),
        result["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    )
}

/// Clear a cell's texture, so that what is in it afterwards can only have come
/// from a pass recorded after this one.
pub(super) fn clear(gpu: &Gpu, view: &wgpu::TextureView) {
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear a cell"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    }));
    gpu.queue.submit([encoder.finish()]);
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
}

/// How many texels of a cell's texture are not transparent black. The row pitch
/// is padded to 256 because a cell is 112 wide and `copy_texture_to_buffer`
/// will not take 448.
pub(super) fn texels(gpu: &Gpu, texture: &wgpu::Texture) -> usize {
    let (width, height) = (texture.width(), texture.height());
    let pitch = (width * 4).div_ceil(256) * 256;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("cell readback"),
        size: u64::from(pitch * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(pitch),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    gpu.queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("poll");
    let data = slice.get_mapped_range().expect("map");
    let lit = (0..height as usize)
        .flat_map(|y| {
            data[y * pitch as usize..y * pitch as usize + width as usize * 4].chunks_exact(4)
        })
        .filter(|p| p[..3] != [0, 0, 0])
        .count();
    drop(data);
    buffer.unmap();
    lit
}
