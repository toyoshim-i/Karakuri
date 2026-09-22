//! A Set groups nodes that form one video source, representing the unit of compilation and lifecycle.
//!
//! GPU state is owned by individual nodes in [`crate::node`]:
//! - L1 simulation nodes own element and alive buffers, counts, compaction scan, and spawn accumulators.
//! - L4 renderer nodes own render pipelines, uniform buffers, accumulation targets, and bind groups.
//!
//! The Set manages shared parameter values, bindings, viewport state, simulation clock, and execution order.
//! Parameter values are written via uniform buffers, while node compilation and pipeline generation
//! remain decoupled within their respective modules.

pub mod layers;
pub mod schedule;
pub mod types;

#[allow(unused_imports)]
pub use layers::*;
pub use schedule::*;
pub use types::*;

use std::collections::{HashMap, HashSet};

use karakuri_ir::typed::Checked;

use crate::camera::Orbit;
use crate::mix::Input;
use crate::node::{Deform, Renderer, Simulation};

impl Set {
    /// Compiles checked procedures into a runnable Set.
    pub fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        l1: &Checked,
        l4: &Checked,
        capacity: u32,
        seed_salt: u32,
    ) -> Result<Set, SetError> {
        Set::build_many(
            device,
            queue,
            &[(l1, capacity)],
            &[],
            &[],
            &[],
            &[l4],
            Layering::Overdraw,
            seed_salt,
            &[],
            Wiring::default(),
        )
    }

    /// Compiles multiple geometry sources, deformers, cameras, fields, and renderers into a runnable Set.
    ///
    /// Execution runs within a wgpu validation error scope to report driver-level errors gracefully.
    #[allow(clippy::too_many_arguments)]
    pub fn build_many(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        l1s: &[(&Checked, u32)],
        l2s: &[&Checked],
        l3s: &[&Checked],
        fields: &[&Checked],
        l4s: &[&Checked],
        layering: Layering,
        seed_salt: u32,
        salts: &[Option<u32>],
        wiring: Wiring<'_>,
    ) -> Result<Set, SetError> {
        let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let built = Set::build_inner(
            device, queue, l1s, l2s, l3s, fields, l4s, layering, seed_salt, salts, wiring,
        );
        let captured = pollster::block_on(scope.pop());

        match (built, captured) {
            (Err(e), _) => Err(e),
            (Ok(_), Some(e)) => Err(SetError::Invalid {
                proc: l1s
                    .first()
                    .map_or_else(String::new, |(p, _)| p.name.clone()),
                detail: e.to_string(),
            }),
            (Ok(set), None) => Ok(set),
        }
    }

    /// Compiles and instantiates GPU pipelines and buffers according to the validated plan.
    #[allow(clippy::too_many_arguments)]
    fn build_inner(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        l1s: &[(&Checked, u32)],
        l2s: &[&Checked],
        l3s: &[&Checked],
        fields: &[&Checked],
        l4s: &[&Checked],
        layering: Layering,
        seed_salt: u32,
        salts: &[Option<u32>],
        wiring: Wiring<'_>,
    ) -> Result<Set, SetError> {
        let Plan {
            names,
            cameras,
            camera_range,
            field_bound,
            camera_bound,
            source_bound,
            far_at,
            heads,
            source_salts,
            derived: derived_per_head,
            l1s: _,
            l2s: _,
            fields: _,
        } = Set::validate(
            l1s, l2s, l3s, fields, l4s, layering, seed_salt, salts, wiring,
        )?;
        let bound_at = |at: usize| -> Vec<(&str, &Checked)> {
            field_bound
                .iter()
                .filter(|(node, _, _)| *node == at)
                .map(|(_, slot, ordinal)| (slot.as_str(), fields[*ordinal]))
                .collect()
        };

        let camera_nodes: Vec<crate::node::Camera> = cameras
            .iter()
            .enumerate()
            .map(|(k, l3)| {
                crate::node::Camera::build(device, *l3, &bound_at(camera_range.start + k))
            })
            .collect();

        let renderer_count = l4s.len();
        let mut sources: Vec<Source> = Vec::with_capacity(heads.len());
        for (head, &at) in heads.iter().enumerate() {
            let (l1, capacity) = l1s[at];
            let salt = source_salts[at];
            let derived = &derived_per_head[head];

            let sim = Simulation::build(device, l1, capacity, salt, derived, &bound_at(at));

            let paired: Option<(Vec<karakuri_ir::Attr>, Simulation)> = match far_at {
                None => None,
                Some(far_at) => {
                    let (far, far_capacity) = l1s[far_at];
                    let far_salt = source_salts[far_at];
                    Some((
                        far.emit.clone(),
                        Simulation::build(
                            device,
                            far,
                            far_capacity,
                            far_salt,
                            derived,
                            &bound_at(far_at),
                        ),
                    ))
                }
            };

            let mut deforms: Vec<Deform> = Vec::new();
            let mut upstream: Vec<karakuri_ir::Attr> = l1.emit.clone();
            let mut synthetic = karakuri_ir::layout::Synthetic::NONE;
            let mut chain_capacity = capacity;
            let mut live: Option<usize> = None;
            for (k, l2) in l2s.iter().enumerate() {
                let node = {
                    let from = sim.geometry();
                    let (alive, counts) = match live {
                        None => (from.alive, from.counts),
                        Some(k) => {
                            let g = deforms[k].geometry(from.alive, from.counts);
                            (g.alive, g.counts)
                        }
                    };
                    let input = match deforms.last() {
                        None => sim.geometry(),
                        Some(prev) => prev.geometry(alive, counts),
                    };
                    let paired = paired.as_ref().filter(|_| l2.geometry_slot().is_some());
                    let far = paired.map(|(emits, sim): &(Vec<karakuri_ir::Attr>, Simulation)| {
                        (emits.as_slice(), sim.geometry())
                    });
                    Deform::build(
                        device,
                        l2,
                        &upstream,
                        synthetic,
                        derived,
                        far.as_ref().map(|(a, g)| (*a, g)),
                        &bound_at(l1s.len() + k),
                        &input,
                        chain_capacity,
                    )?
                };
                upstream = node.emits().to_vec();
                synthetic = node.synthetic();
                chain_capacity = chain_capacity.saturating_mul(node.amplify());
                if node.amplifies() {
                    live = Some(deforms.len());
                }
                deforms.push(node);
            }

            let renderers: Vec<Renderer> = {
                let from = sim.geometry();
                let (alive, counts) = match live {
                    None => (from.alive, from.counts),
                    Some(k) => {
                        let g = deforms[k].geometry(from.alive, from.counts);
                        (g.alive, g.counts)
                    }
                };
                let geometry = match deforms.last() {
                    None => sim.geometry(),
                    Some(last) => last.geometry(alive, counts),
                };
                let first_l4 = camera_range.end;
                l4s.iter()
                    .enumerate()
                    .map(|(k, l4)| {
                        let at = first_l4 + k;
                        let camera = camera_bound
                            .iter()
                            .find(|(node, _)| *node == at)
                            .map_or(0, |(_, ordinal)| *ordinal);
                        Renderer::build(device, l4, &geometry, &camera_nodes[camera], &bound_at(at))
                    })
                    .collect()
            };
            sources.push(Source {
                salt,
                procedures: std::iter::once(at).chain(far_at).collect(),
                sim,
                paired: paired.map(|(_, s)| s),
                deforms,
                renderers,
            });
        }

        let params = l1s
            .iter()
            .map(|(l1, _)| declared_defaults(l1))
            .chain(l2s.iter().map(|n| declared_defaults(n)))
            .chain(cameras.iter().map(|n| match n {
                Some(n) => declared_defaults(n),
                None => Orbit::default().placement_values().into_iter().collect(),
            }))
            .chain(l4s.iter().map(|n| declared_defaults(n)))
            .chain(fields.iter().map(|n| declared_defaults(n)))
            .collect();
        let param_values = l1s
            .iter()
            .map(|(l1, _)| declared_default_values(l1))
            .chain(l2s.iter().map(|n| declared_default_values(n)))
            .chain(cameras.iter().map(|n| {
                match n {
                    Some(n) => declared_default_values(n),
                    None => Orbit::default()
                        .placement_values()
                        .into_iter()
                        .map(|(k, v)| (k, karakuri_store::record::Value::Scalar(v)))
                        .collect(),
                }
            }))
            .chain(l4s.iter().map(|n| declared_default_values(n)))
            .chain(fields.iter().map(|n| declared_default_values(n)))
            .collect();
        let ranges = l1s
            .iter()
            .map(|(l1, _)| declared_ranges(l1))
            .chain(l2s.iter().map(|n| declared_ranges(n)))
            .chain(cameras.iter().map(|n| match n {
                Some(n) => declared_ranges(n),
                None => Orbit::placement_ranges().into_iter().collect(),
            }))
            .chain(l4s.iter().map(|n| declared_ranges(n)))
            .chain(fields.iter().map(|n| declared_ranges(n)))
            .collect();
        let authorities = vec![Authority::default(); names.len()];
        let moved = vec![HashSet::new(); names.len()];
        let declared_capacities = l1s
            .iter()
            .map(|(l1, at)| {
                l1.capacity
                    .map_or([*at, *at, *at], |decl| [decl.min, decl.max, decl.default])
            })
            .collect();
        let set = Set {
            names,
            authorities,
            seed_salt,
            source_salts,
            declared_capacities,
            steps_taken: 0,
            staged_delta: 0,
            staged_edges: None,
            dt: DT,
            last_beats: 0.0,
            viewport: [1.0, 1.0],
            closed_form: l1s.iter().all(|(n, _)| n.closed_form)
                && l2s.iter().all(|n| n.closed_form)
                && l3s.iter().all(|n| n.closed_form)
                && l4s.iter().all(|n| n.closed_form),
            reads_beats: l1s.iter().any(|(n, _)| n.reads_beats)
                || l2s.iter().any(|n| n.reads_beats)
                || l3s.iter().any(|n| n.reads_beats)
                || l4s.iter().any(|n| n.reads_beats),
            sources,
            params,
            param_values,
            moved,
            ranges,
            rate_bounds: l4s
                .iter()
                .map(|n| karakuri_ir::rate::point_rate_bound(n))
                .collect(),
            camera: Orbit::default(),
            cameras: camera_nodes,
            merge: (layering == Layering::Composite)
                .then(|| crate::node::Merge::build(device, renderer_count, 1, 1)),
            edges: vec![Input::default(); renderer_count],
            bindings: Vec::new(),
            interface: Vec::new(),
            l1_count: l1s.len(),
            field_count: fields.len(),
            field_declared: fields.iter().map(|f| declared_keys(f)).collect(),
            field_params: {
                let mut keys: Vec<String> = field_bound
                    .iter()
                    .flat_map(|(_, slot, ordinal)| {
                        fields[*ordinal]
                            .params
                            .iter()
                            .map(move |p| karakuri_codegen::layout::field_param_key(slot, &p.name))
                    })
                    .collect();
                keys.sort_unstable();
                keys.dedup();
                keys
            },
            field_bound,
            source_bound,
        };
        for source in &set.sources {
            source.sim.initialize(queue);
            if let Some(other) = &source.paired {
                other.initialize(queue);
            }
        }
        for camera in &set.cameras {
            camera.write_state(queue, &set.orbit().state(0.0));
            camera.write_canvas(queue, 1.0);
        }
        set.prime(device, queue);
        Ok(set)
    }

    /// Primes the deformation pipeline with initial simulation state during build.
    fn prime(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.sources.iter().all(|s| s.deforms.is_empty()) {
            return;
        }
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("prime the deformation chain"),
        });
        self.record_counts(&mut encoder);
        for source in &self.sources {
            let parity = source.sim.parity();
            let mut counts = source.sim.counts();
            for node in &source.deforms {
                node.record(&mut encoder, parity, counts);
                if let Some(own) = node.counts() {
                    counts = own;
                }
            }
        }
        queue.submit([encoder.finish()]);
    }
}

/// Returns declared default values as typed [`Value`](karakuri_store::record::Value)s.
fn declared_default_values(node: &Checked) -> HashMap<String, karakuri_store::record::Value> {
    node.params
        .iter()
        .filter_map(|p| {
            let comps = p.default_components()?;
            let val = match comps.len() {
                1 => karakuri_store::record::Value::Scalar(comps[0]),
                2 => karakuri_store::record::Value::Vec2([comps[0], comps[1]]),
                3 => karakuri_store::record::Value::Vec3([comps[0], comps[1], comps[2]]),
                4 => karakuri_store::record::Value::Vec4([comps[0], comps[1], comps[2], comps[3]]),
                _ => return None,
            };
            Some((p.name.clone(), val))
        })
        .collect()
}

fn declared_defaults(node: &Checked) -> HashMap<String, f32> {
    node.params
        .iter()
        .filter_map(|p| Some((p.keys(), p.default_components()?)))
        .flat_map(|(keys, values)| keys.into_iter().zip(values))
        .collect()
}

/// Returns all addressable parameter keys for `node`, expanding vector parameters.
pub(crate) fn declared_keys(node: &Checked) -> Vec<String> {
    node.params.iter().flat_map(|p| p.keys()).collect()
}

/// Returns the declared valid range for each addressable parameter key.
fn declared_ranges(node: &Checked) -> HashMap<String, [f32; 2]> {
    node.params
        .iter()
        .flat_map(|p| {
            p.keys()
                .into_iter()
                .map(move |key| (key, [p.min, p.max]))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
#[cfg(test)]
mod tests;
