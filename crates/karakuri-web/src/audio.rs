//! Web Audio API microphone capture and streaming to unified PCM audio core.
//! Captures microphone audio using getUserMedia and ScriptProcessorNode,
//! feeding raw PCM samples directly to karakuri-audio's AudioCore for spectral
//! analysis and tempo tracking (ADR-0384).

use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    AudioContext, AudioProcessingEvent, GainNode, MediaStream, MediaStreamAudioSourceNode,
    MediaStreamConstraints, MediaStreamTrack, ScriptProcessorNode,
};

use karakuri_audio::AudioInput;
use karakuri_environment::audio::{Audio, DEFAULT_LATENCY_OFFSET_MS};

/// Number of frames per Web Audio script processor callback buffer.
const BUFFER_SIZE: u32 = 1024;

/// Active Web Audio microphone session and node graph.
/// Drops and disconnects nodes and closes the audio context when dropped.
pub struct WebAudioSession {
    ctx: AudioContext,
    stream: MediaStream,
    source: MediaStreamAudioSourceNode,
    processor: ScriptProcessorNode,
    mute_gain: GainNode,
    _closure: Closure<dyn FnMut(AudioProcessingEvent)>,
}

impl Drop for WebAudioSession {
    fn drop(&mut self) {
        let _ = self.processor.disconnect();
        let _ = self.source.disconnect();
        let _ = self.mute_gain.disconnect();
        let _ = self.ctx.close();

        let tracks = self.stream.get_tracks();
        for i in 0..tracks.length() {
            if let Ok(track) = tracks.get(i).dyn_into::<MediaStreamTrack>() {
                track.stop();
            }
        }
        log::info!("Karakuri Web: Web Audio session closed");
    }
}

/// Requests microphone permission via `getUserMedia`, initializes the Web Audio API processing
/// graph, and connects PCM sample feeding to `karakuri-audio::AudioCore`.
pub async fn start_web_audio(session_bpm: f32) -> Result<(Audio, WebAudioSession), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("global window unavailable"))?;
    let navigator = window.navigator();
    let media_devices = navigator
        .media_devices()
        .map_err(|e| JsValue::from_str(&format!("mediaDevices unavailable: {e:?}")))?;

    let constraints = MediaStreamConstraints::new();
    constraints.set_audio(&JsValue::TRUE);
    constraints.set_video(&JsValue::FALSE);

    let promise = media_devices.get_user_media_with_constraints(&constraints)?;
    let js_stream = wasm_bindgen_futures::JsFuture::from(promise).await?;
    let stream: MediaStream = js_stream.dyn_into()?;

    let ctx = AudioContext::new()?;
    let sample_rate = ctx.sample_rate() as u32;

    let (input, core) = AudioInput::custom("Default Microphone", sample_rate, session_bpm);

    let source = ctx.create_media_stream_source(&stream)?;
    let processor = ctx.create_script_processor_with_buffer_size_and_number_of_input_channels_and_number_of_output_channels(
        BUFFER_SIZE,
        1,
        1,
    )?;

    // To ensure onaudioprocess fires reliably across all browsers, connect processor to a muted gain
    // node before destination. Muting to 0.0 prevents microphone feedback through speakers.
    let mute_gain = ctx.create_gain()?;
    mute_gain.gain().set_value(0.0);

    source.connect_with_audio_node(&processor)?;
    processor.connect_with_audio_node(&mute_gain)?;
    mute_gain.connect_with_audio_node(&ctx.destination())?;

    let core_cb = Arc::clone(&core);
    let closure = Closure::wrap(Box::new(move |e: AudioProcessingEvent| {
        if let Ok(buf) = e.input_buffer() {
            let frames = buf.length() as usize;
            if let Ok(data) = buf.get_channel_data(0) {
                if let Ok(mut c) = core_cb.lock() {
                    let mut iter = data.into_iter();
                    c.feed(&mut iter, frames, sample_rate as f32);
                }
            }
        }
    }) as Box<dyn FnMut(AudioProcessingEvent)>);

    processor.set_onaudioprocess(Some(closure.as_ref().unchecked_ref()));

    let audio = Audio::from_input(input, DEFAULT_LATENCY_OFFSET_MS, 1.0 / 60.0);

    log::info!(
        "Karakuri Web: Microphone audio input active at {} Hz (buffer: {})",
        sample_rate,
        BUFFER_SIZE
    );

    Ok((
        audio,
        WebAudioSession {
            ctx,
            stream,
            source,
            processor,
            mute_gain,
            _closure: closure,
        },
    ))
}
