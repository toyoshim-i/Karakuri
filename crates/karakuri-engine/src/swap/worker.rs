use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use crate::binding::Signals;
use crate::estimate::{estimate, Estimate};
use crate::probe::{Measurement, Probe};
use crate::set::{Set, SetError};

use super::types::{unpacked, Built, Done, Polled, Sizes, Source, PROBE_STEPS};

/// Measures one frame execution cost of `set` at target resolution `at` and restores initial state.
///
/// Submits GPU commands and waits for completion. Rewinds buffer and simulation state via
/// [`Set::rewind`] prior to returning.
pub fn measure(
    probe: &mut Probe,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    set: &mut Set,
    at: (u32, u32),
) -> Measurement {
    let capacity = set.capacity();
    let viewport = set.viewport();
    probe.resize(device, at);
    set.resize(device, at.0, at.1);
    set.prepare(queue, PROBE_STEPS, &Signals::default());
    let measurement = probe.run(device, queue, set, PROBE_STEPS, capacity);
    set.rewind(device, queue);
    set.resize(device, viewport.0, viewport.1);
    measurement
}

/// Main execution loop for the background build and compilation worker.
pub(crate) fn run_worker(
    device: wgpu::Device,
    queue: wgpu::Queue,
    mut source: Box<dyn Source>,
    out: Sender<Done>,
    graveyard: Arc<Mutex<Vec<Set>>>,
    stop: Arc<AtomicBool>,
    sizes: Sizes,
) {
    let mut probe: Option<Probe> = None;

    while !stop.load(Ordering::Relaxed) {
        let condemned: Vec<Set> = match graveyard.lock() {
            Ok(mut held) => held.drain(..).collect(),
            Err(_) => Vec::new(),
        };
        drop(condemned);

        let request = match source.poll() {
            Some(Polled::Build(request)) => request,
            Some(Polled::Refused(refusal)) => {
                if out.send(Done::Refused(refusal)).is_err() {
                    break;
                }
                continue;
            }
            None => continue,
        };
        let id = request.id;
        let label: Arc<str> = request.label.into();

        let build = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Set::build_many(
                &device,
                &queue,
                &request.l1s.iter().map(|(p, c)| (p, *c)).collect::<Vec<_>>(),
                &request.l2s.iter().collect::<Vec<_>>(),
                &request.l3s.iter().collect::<Vec<_>>(),
                &request.fields.iter().collect::<Vec<_>>(),
                &request.l4s.iter().collect::<Vec<_>>(),
                request.layering,
                request.seed_salt,
                &request.salts,
                crate::set::Wiring {
                    l1s: &request.names.l1s,
                    l2s: &request.names.l2s,
                    l3s: &request.names.l3s,
                    l4s: &request.names.l4s,
                    fields: &request.names.fields,
                    edges: &request.edges,
                },
            )
            .map(|mut set| {
                set.aim_camera(request.camera);
                if let Some(at) = request.live {
                    if !set.select_renderer(at as usize) {
                        eprintln!(
                            "  this build has no renderer {at} to fold to — every renderer is live"
                        );
                    }
                }
                for write in &request.params {
                    match set.write_param(write) {
                        Ok(0) => eprintln!("  no parameter named `{}`, ignoring", write.key),
                        Ok(_) => {}
                        Err(refused) => eprintln!("  {refused}"),
                    }
                }
                for control in request.published {
                    let name = control.name.clone();
                    if let Err(e) = set.publish(control) {
                        eprintln!("  `{name}` is not published: {e}");
                    }
                }
                for binding in request.bindings {
                    let (layer, key) = (binding.layer, binding.key.clone());
                    let signal = binding.signal.clone();
                    match set.bind(binding) {
                        crate::set::Bound::Yes => {}
                        crate::set::Bound::NoSuchParam => {
                            eprintln!("  no {layer:?} parameter named `{key}` to bind, ignoring")
                        }
                        crate::set::Bound::NoSuchControl => eprintln!(
                            "  `{signal}` is not published by this Set, so `{layer:?} {key}` \
                             is not bound"
                        ),
                    }
                }
                for stated in &request.authorities {
                    let (layer, index) = stated.at;
                    if !set.set_authority(layer, index, stated.authority) {
                        eprintln!(
                            "  this build has no {layer:?} node {index} to give authority to, \
                             ignoring"
                        );
                    }
                }
                set
            })
        }));
        let result = match build {
            Ok(result) => result,
            Err(payload) => Err(SetError::Panicked {
                label: label.to_string(),
                detail: panic_detail(&payload),
            }),
        };

        let mut result = result;
        let mut cost = None;
        let mut prediction: Option<Estimate> = None;
        if let Ok(set) = &mut result {
            // Flush initial element buffer uploads so the Set is resident on the GPU.
            queue.submit([]);
            let _ = device.poll(wgpu::PollType::wait_indefinitely());

            let at = unpacked(sizes.measure_at.load(Ordering::Relaxed));
            let probe = probe.get_or_insert_with(|| {
                Probe::new(
                    &device,
                    &queue,
                    device.features().contains(wgpu::Features::TIMESTAMP_QUERY),
                    at,
                )
            });
            cost = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                measure(probe, &device, &queue, set, at)
            }))
            .ok();
            if cost.is_none() {
                eprintln!("  `{label}` could not be measured; it will not be budgeted for");
            }

            // Compute estimate at the slot's target output resolution (ADR-0303).
            let target = unpacked(sizes.estimate_at.load(Ordering::Relaxed));
            prediction = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                estimate(probe, &device, &queue, set, target)
            }))
            .ok();
            if prediction.is_none() {
                eprintln!(
                    "  `{label}` could not be estimated; it will be budgeted on its \
                     measurement"
                );
            }

            // Flush the rewind uploads both the measurement and the estimate
            // left, so they do not stall the render thread upon installation.
            queue.submit([]);
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
        }

        if out
            .send(Done::Built(Built {
                id,
                label,
                result,
                cost,
                estimate: prediction,
            }))
            .is_err()
        {
            break;
        }
    }
}

/// Extracts a displayable message from a caught panic payload.
fn panic_detail(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "no message".to_string()
    }
}
