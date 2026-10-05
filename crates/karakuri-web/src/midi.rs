//! Web MIDI API integration feeding hardware MIDI controller inputs and
//! driving LED/fader outputs via abstracted Karakuri Port and Out.

use std::sync::Arc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{MidiAccess, MidiInput, MidiOutput};
use winit::event_loop::EventLoopProxy;

use karakuri_environment::midi::Surface;
use karakuri_midi::{Out, Port};

const DEFAULT_SURFACE_MAP: &str = include_str!("../../../examples/surface.map");

pub struct WebMidiSession {
    _access: MidiAccess,
    _input_closures: Vec<Closure<dyn FnMut(web_sys::Event)>>,
}

/// Attempts to request Web MIDI access and connect the first available MIDI input (and optional output).
pub async fn start_web_midi(
    waker: EventLoopProxy<()>,
) -> Result<(Surface, WebMidiSession), String> {
    let window = web_sys::window().ok_or("no window available")?;
    let navigator = window.navigator();

    let options = web_sys::MidiOptions::new();
    options.set_sysex(false);

    let promise = navigator
        .request_midi_access_with_options(&options)
        .map_err(|e| format!("request_midi_access_with_options error: {e:?}"))?;

    let access_val = JsFuture::from(promise)
        .await
        .map_err(|e| format!("request_midi_access rejected: {e:?}"))?;

    let access: MidiAccess = access_val
        .dyn_into()
        .map_err(|e| format!("failed to cast to MidiAccess: {e:?}"))?;

    let inputs = access.inputs();
    let input_values = inputs.values();

    let mut first_input: Option<MidiInput> = None;
    while let Ok(next) = input_values.next() {
        if next.done() {
            break;
        }
        if let Ok(input) = next.value().dyn_into::<MidiInput>() {
            first_input = Some(input);
            break;
        }
    }

    let input = first_input.ok_or_else(|| "no MIDI input devices found".to_string())?;
    let port_name = input.name().unwrap_or_else(|| "Web MIDI In".to_string());

    let (port, sender) = Port::custom(
        &port_name,
        Some(Arc::new(move || {
            let _ = waker.send_event(());
        })),
    );

    let onmessage = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
        if let Ok(midi_event) = event.dyn_into::<web_sys::MidiMessageEvent>() {
            if let Ok(data) = midi_event.data() {
                sender.send_bytes(&data);
            }
        }
    });

    input.set_onmidimessage(Some(onmessage.as_ref().unchecked_ref()));

    // Try finding matching or first MIDI output for feedback (motorized faders, LEDs)
    let outputs = access.outputs();
    let output_values = outputs.values();
    let mut first_output: Option<MidiOutput> = None;
    while let Ok(next) = output_values.next() {
        if next.done() {
            break;
        }
        if let Ok(output) = next.value().dyn_into::<MidiOutput>() {
            first_output = Some(output);
            break;
        }
    }

    let out = first_output.map(|output| {
        let out_name = output.name().unwrap_or_else(|| "Web MIDI Out".to_string());
        Out::custom(&out_name, move |bytes: [u8; 3]| {
            let data = js_sys::Uint8Array::from(&bytes[..]);
            output.send(&data.into()).is_ok()
        })
    });

    let (surface, notes) = Surface::custom(
        port,
        out,
        Some(DEFAULT_SURFACE_MAP),
        Some("surface".to_string()),
    );

    for note in notes {
        log::info!("Web MIDI map note: {note}");
    }

    let session = WebMidiSession {
        _access: access,
        _input_closures: vec![onmessage],
    };

    Ok((surface, session))
}
