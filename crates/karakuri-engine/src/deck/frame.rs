use crate::binding::Signals;
use crate::mix::Input;
use crate::present::Present;
use crate::set::{DT, MAX_STEPS};
use crate::transition::{Selection, Transition};
use crate::transport::Advance;
use crate::video_source::VideoSource;

use super::types::{Mask, Residency};
use super::Deck;

/// Staged slot control state evaluated during frame rendering.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StagedSlotControl {
    pub(crate) slot: usize,
    pub(crate) gain: f32,
    pub(crate) opacity: f32,
    pub(crate) mask: Mask,
}

/// Active frame recording guard holding the command encoder and exclusive deck access.
pub struct Frame<'a> {
    pub(crate) deck: &'a mut Deck,
    pub(crate) queue: &'a wgpu::Queue,
    pub(crate) encoder: Option<wgpu::CommandEncoder>,
    pub(crate) rendered: bool,
    pub(crate) staged_signals: Option<Signals>,
    pub(crate) staged_transitions: Option<Vec<Transition>>,
    pub(crate) staged_selections: Option<Vec<Selection>>,
    pub(crate) staged_slot_controls: Option<Vec<StagedSlotControl>>,
}

impl Frame<'_> {
    /// Advances live slots by `steps`, renders each into its target, and composites into `target`.
    pub fn render(&mut self, target: &wgpu::TextureView, target_size: (u32, u32), steps: u8) {
        assert!(
            !self.rendered,
            "one `render` per frame: a second one would advance every Live slot by \
             another `steps` off the same tick"
        );
        self.rendered = true;

        let mut signals = self.deck.signals;
        signals.advance(steps.min(MAX_STEPS), DT);
        self.staged_signals = Some(signals);

        assert_eq!(
            target_size,
            (self.deck.width, self.deck.height),
            "the mix target is {target_size:?} and the deck's slots are {:?} — \
             resize both or the composite silently reads out of range and mixes black",
            (self.deck.width, self.deck.height)
        );

        let encoder = self
            .encoder
            .as_mut()
            .expect("the encoder is open until `finish` or drop");

        let beats = signals.oscillator().beats();
        let (staged_transitions, staged_controls) = self.deck.stage_transitions(beats);
        self.staged_transitions = Some(staged_transitions);
        self.staged_slot_controls = Some(staged_controls);
        let staged_selections = self.deck.stage_selections(beats);
        self.staged_selections = Some(staged_selections);

        for (i, slot) in self.deck.slots.iter_mut().enumerate() {
            let stopped = slot.swap.overloaded();
            let on_air = slot.effective == Residency::Live || slot.online;
            match on_air {
                true if stopped => {
                    if let Some(meters) = &mut self.deck.meters {
                        meters.record(i, encoder);
                    }
                }
                true => {
                    let view = &slot.view;
                    let set = slot.swap.live_mut();
                    let steps = match slot.transport.advance(steps, signals.oscillator(), DT) {
                        Advance::Steps(n) => n,
                        Advance::SeekTo(target) => {
                            set.seek(target.saturating_sub(1));
                            1
                        }
                    };
                    set.prepare(self.queue, steps, &signals);
                    set.render(encoder, view, steps);
                    if let Some(meters) = &mut self.deck.meters {
                        meters.record(i, encoder);
                    }
                }
                false if stopped => {}
                false => {
                    let view = &slot.view;
                    let set = slot.swap.live_mut();
                    set.prepare_warming(self.queue, steps, &signals);
                    set.render(encoder, view, steps);
                }
            }
        }

        let mut edges: Vec<Input> = Vec::with_capacity(self.deck.slots.len());
        for (i, slot) in self.deck.slots.iter().enumerate() {
            let mut edge = slot.edge();
            if let Some(controls) = &self.staged_slot_controls {
                if let Some(c) = controls.iter().find(|c| c.slot == i) {
                    edge.gain = c.gain;
                    edge.opacity = c.opacity;
                    edge.mask = c.mask;
                }
            }
            let in_mix = slot.online && edge.opacity > 0.0;
            edges.push(Input {
                live: in_mix,
                ..edge
            });
        }
        self.deck
            .composite
            .write_uniform(self.queue, &edges, self.deck.out);
        self.deck.composite.record(encoder, target);
    }

    /// Returns a mutable reference to the open frame command encoder.
    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        self.encoder
            .as_mut()
            .expect("the encoder is open until `finish` or drop")
    }

    /// Submits the command buffer and commits staged state transitions to the deck.
    pub fn finish(mut self) {
        self.submit();
    }

    /// Discards the frame without submitting commands or committing staged state.
    pub fn discard(mut self) {
        let _ = self.encoder.take();
        self.staged_signals = None;
        self.staged_transitions = None;
        self.staged_selections = None;
        self.staged_slot_controls = None;
        for slot in &mut self.deck.slots {
            slot.swap.live_mut().discard();
        }
    }

    fn submit(&mut self) {
        if let Some(encoder) = self.encoder.take() {
            self.queue.submit([encoder.finish()]);
            if let Some(signals) = self.staged_signals.take() {
                self.deck.signals = signals;
            }
            if let Some(transitions) = self.staged_transitions.take() {
                self.deck.transitions = transitions;
            }
            if let Some(selections) = self.staged_selections.take() {
                self.deck.selections = selections;
            }
            if let Some(controls) = self.staged_slot_controls.take() {
                for c in controls {
                    self.deck.slots[c.slot].gain = c.gain;
                    self.deck.slots[c.slot].opacity = c.opacity;
                    self.deck.slots[c.slot].mask = c.mask;
                }
            }
            for slot in &mut self.deck.slots {
                slot.swap.live_mut().commit();
            }
            if let Some(meters) = &mut self.deck.meters {
                meters.arm();
            }
        }
    }
}

impl Drop for Frame<'_> {
    fn drop(&mut self) {
        self.submit();
    }
}

/// Creates an HDR texture target and corresponding view for a deck slot.
pub(crate) fn make_slot_target(
    device: &wgpu::Device,
    slot: usize,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&format!("deck slot {slot}")),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: Present::HDR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    (texture, view)
}
