use std::collections::HashMap;

use karakuri_ir::Kind;

use super::common::{effective, effective_vector, field_value, source_value};
use crate::binding::{Binding, Signals, CONTROL_PREFIX};
use crate::mix::Input;
use crate::set::types::{Bound, Clock, Set, MAX_STEPS};

impl Set {
    /// Attaches a dynamic signal to a parameter, replacing any existing binding on that target.
    pub fn bind(&mut self, binding: Binding) -> Bound {
        let declares = |names: &[String], map: &HashMap<String, f32>| {
            names.contains(&binding.key) && map.contains_key(&binding.key)
        };
        if let Some(name) = binding.signal.strip_prefix(CONTROL_PREFIX) {
            if !self.published().iter().any(|p| p.name == name) {
                return Bound::NoSuchControl;
            }
        }
        let range = self.nodes_of(binding.layer);
        let names = self.declared_names(binding.layer);
        let found = names.iter().enumerate().any(|(at, n)| {
            binding.covers(at)
                && self
                    .params
                    .get(range.start + at)
                    .is_some_and(|map| declares(n, map))
        });
        if !found {
            return Bound::NoSuchParam;
        }
        self.bindings.retain(|b| {
            b.layer != binding.layer || b.key != binding.key || b.index != binding.index
        });
        self.bindings.push(binding);
        Bound::Yes
    }

    /// Detaches a dynamic signal from a parameter.
    pub fn unbind(&mut self, layer: Kind, index: Option<u32>, key: &str) -> bool {
        let before = self.bindings.len();
        self.bindings
            .retain(|b| b.layer != layer || b.key != key || b.index != index);
        self.bindings.len() != before
    }

    /// Returns an iterator over all bound parameters and their most recent evaluated values.
    pub fn bound(&self) -> impl Iterator<Item = (&str, f32)> {
        self.bindings.iter().map(|b| (b.key.as_str(), b.value()))
    }

    /// Returns the active parameter signal bindings.
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }

    /// Returns the input edges to the composite pass in draw order.
    pub fn inputs(&self) -> &[Input] {
        &self.edges
    }

    /// Sets the composite input edge at index `at`. Returns `false` if out of bounds.
    pub fn set_input(&mut self, at: usize, input: Input) -> bool {
        match self.edges.get_mut(at) {
            Some(edge) => {
                *edge = input;
                true
            }
            None => false,
        }
    }

    /// Selects the renderer at index `at` as the live composite input.
    ///
    /// Returns `false` if `at` is out of bounds.
    pub fn select_renderer(&mut self, at: usize) -> bool {
        crate::mix::select(&mut self.edges, at)
    }

    /// Stages selection of the live renderer during an uncommitted frame.
    pub fn stage_select_renderer(&mut self, at: usize) -> bool {
        if self.staged_edges.is_none() {
            self.staged_edges = Some(self.edges.clone());
        }
        crate::mix::select(self.staged_edges.as_mut().unwrap(), at)
    }

    /// Returns the active or staged inputs to the merge pass.
    pub fn edges(&self) -> &[Input] {
        self.staged_edges.as_deref().unwrap_or(&self.edges)
    }

    /// Prepares simulation uniforms and advances time on the session clock.
    pub fn prepare(&mut self, queue: &wgpu::Queue, steps: u8, signals: &Signals) {
        self.prepare_on(queue, steps, signals, Clock::Session);
    }

    /// Prepares simulation uniforms for an off-air set on its local clock.
    pub fn prepare_warming(&mut self, queue: &wgpu::Queue, steps: u8, signals: &Signals) {
        self.prepare_on(queue, steps, signals, Clock::Local);
    }

    /// Common preparation routine for both on-air and warming sets.
    fn prepare_on(&mut self, queue: &wgpu::Queue, steps: u8, signals: &Signals, clock: Clock) {
        let steps = steps.min(MAX_STEPS);
        self.staged_delta = u64::from(steps);
        let next_steps_taken = self.steps_taken + self.staged_delta;
        let view = match clock {
            Clock::Session => *signals,
            Clock::Local => {
                let lag = signals
                    .oscillator()
                    .steps_taken()
                    .saturating_sub(next_steps_taken);
                signals.behind(lag as f64 * f64::from(self.dt))
            }
        };
        self.resolve_bindings(&view);

        let first = next_steps_taken - u64::from(steps) + 1;
        let mut instants = [(0.0f32, 0.0f32); MAX_STEPS as usize];
        for (k, slot) in instants.iter_mut().enumerate().take(usize::from(steps)) {
            let t = self.t_at(first + k as u64);
            *slot = (t, view.oscillator().at_time(f64::from(t)).beats() as f32);
        }

        {
            let bindings = &self.bindings;
            let field_range = self.nodes_of(Kind::Field);
            let field_maps = &self.params[field_range.clone()];
            let field_bound = &self.field_bound;
            let field_params = &self.field_params;
            let params = &self.params;
            let param_values = &self.param_values;
            let source_bound = &self.source_bound;
            let source_salts = &self.source_salts;

            for source in &mut self.sources {
                let procedures = source.procedures.clone();
                for (k, sim) in std::iter::once(&mut source.sim)
                    .chain(source.paired.as_mut())
                    .enumerate()
                {
                    let at = procedures[k];
                    let own = &params[at];
                    let own_values = &param_values[at];
                    let param = |name: &str| effective(bindings, own, Kind::L1, at, name);
                    let param_val =
                        |name: &str| effective_vector(bindings, own_values, Kind::L1, at, name);
                    let tick = crate::node::Tick {
                        steps,
                        dt: self.dt,
                        instants,
                        param: &param,
                        param_value: Some(&param_val),
                        field_params,
                        field_value: &|name: &str| field_value(field_bound, field_maps, at, name),
                        source_value: &|key: &str| {
                            source_value(source_bound, source_salts, at, key)
                        },
                    };
                    sim.prepare(queue, &tick);
                }
            }
        }

        let t = self.t_at(next_steps_taken);
        self.last_beats = view.oscillator().at_time(f64::from(t)).beats() as f32;
        self.write_l4_uniforms(queue, t);
    }

    /// Writes uniform buffers for L4 renderers and camera nodes.
    fn write_l4_uniforms(&mut self, queue: &wgpu::Queue, t: f32) {
        self.write_l2_uniforms(queue, t);
        if let Some(merge) = &self.merge {
            merge.write_uniform(queue, self.edges());
        }
        {
            let first = self.slot_of(Kind::L3);
            let aspect = self.viewport[0] / self.viewport[1];
            let field_range = self.nodes_of(Kind::Field);
            let (bindings, dt, field_params, field_bound, field_maps, viewport, beats, salt) = (
                &self.bindings,
                self.dt,
                &self.field_params,
                &self.field_bound,
                &self.params[field_range.clone()],
                self.viewport,
                self.last_beats,
                self.seed_salt,
            );
            let no_sources = |_: &str| None;
            let stated = self.camera;
            let params = &self.params[first..];
            let param_values = &self.param_values[first..];
            for (at, (camera, params)) in self.cameras.iter_mut().zip(params).enumerate() {
                camera.write_canvas(queue, aspect);
                let orbit = match camera.is_builtin() {
                    true => {
                        stated.with_placement(|key| effective(bindings, params, Kind::L3, at, key))
                    }
                    false => stated,
                };
                let fallback = orbit.state(t);
                let own_values = &param_values[at];
                let param_val =
                    |name: &str| effective_vector(bindings, own_values, Kind::L3, at, name);
                let view = crate::node::View {
                    t,
                    beats,
                    seed_salt: salt,
                    viewport,
                    field_params,
                    field_value: &|name: &str| {
                        field_value(field_bound, field_maps, first + at, name)
                    },
                    source_value: &no_sources,
                    param: &|name: &str| effective(bindings, params, Kind::L3, at, name),
                    param_value: Some(&param_val),
                };
                camera.prepare(queue, &view, dt, &fallback);
            }
        }
        let field_range = self.nodes_of(Kind::Field);
        let (bindings, beats, viewport, field_params, field_bound, field_maps) = (
            &self.bindings,
            self.last_beats,
            self.viewport,
            &self.field_params,
            &self.field_bound,
            &self.params[field_range.clone()],
        );
        let (source_bound, source_salts) = (&self.source_bound, &self.source_salts);
        let first = self.slot_of(Kind::L4);
        let params = &self.params[first..];
        let param_values = &self.param_values[first..];
        for source in &mut self.sources {
            let salt = source.salt;
            for (at, (renderer, params)) in source.renderers.iter_mut().zip(params).enumerate() {
                let own_values = &param_values[at];
                let param_val =
                    |name: &str| effective_vector(bindings, own_values, Kind::L4, at, name);
                let view = crate::node::View {
                    t,
                    beats,
                    seed_salt: salt,
                    viewport,
                    field_params,
                    field_value: &|name: &str| {
                        field_value(field_bound, field_maps, first + at, name)
                    },
                    source_value: &|key: &str| {
                        source_value(source_bound, source_salts, first + at, key)
                    },
                    param: &|name: &str| effective(bindings, params, Kind::L4, at, name),
                    param_value: Some(&param_val),
                };
                renderer.write_uniforms(queue, &view);
            }
        }
    }

    /// Writes uniform buffers for L2 deformation nodes.
    fn write_l2_uniforms(&mut self, queue: &wgpu::Queue, t: f32) {
        let field_range = self.nodes_of(Kind::Field);
        let (bindings, beats, viewport, dt, field_params, field_bound, field_maps) = (
            &self.bindings,
            self.last_beats,
            self.viewport,
            self.dt,
            &self.field_params,
            &self.field_bound,
            &self.params[field_range.clone()],
        );
        let (source_bound, source_salts) = (&self.source_bound, &self.source_salts);
        let capacity = self.sources[0].sim.capacity();
        let range = self.nodes_of(Kind::L2);
        let first = range.start;
        let params = &self.params[range.clone()];
        let param_values = &self.param_values[range];
        for source in &mut self.sources {
            let salt = source.salt;
            for (at, (node, params)) in source.deforms.iter_mut().zip(params).enumerate() {
                let own_values = &param_values[at];
                let param_val =
                    |name: &str| effective_vector(bindings, own_values, Kind::L2, at, name);
                let view = crate::node::View {
                    t,
                    beats,
                    seed_salt: salt,
                    viewport,
                    field_params,
                    field_value: &|name: &str| {
                        field_value(field_bound, field_maps, first + at, name)
                    },
                    source_value: &|key: &str| {
                        source_value(source_bound, source_salts, first + at, key)
                    },
                    param: &|name: &str| effective(bindings, params, Kind::L2, at, name),
                    param_value: Some(&param_val),
                };
                node.write_uniforms(queue, &view, dt, capacity);
            }
        }
    }

    /// Evaluates each parameter binding against input signals and manual fallback values.
    fn resolve_bindings(&mut self, signals: &Signals) {
        let ranges: Vec<(usize, std::ops::Range<usize>)> = Kind::ALL
            .into_iter()
            .map(|k| (self.slot_of(k), self.nodes_of(k)))
            .collect();
        let mut bindings = std::mem::take(&mut self.bindings);
        let params = &self.params;
        for binding in &mut bindings {
            let (base, range) = match binding.layer {
                Kind::L1 => ranges[0].clone(),
                Kind::L2 => ranges[1].clone(),
                Kind::L3 => ranges[2].clone(),
                Kind::L4 => ranges[3].clone(),
                Kind::Field => (0, 0..0),
                Kind::L5 => (0, 0..0),
            };
            let manual = range
                .clone()
                .filter(|slot| binding.covers(slot - base))
                .filter_map(|slot| params.get(slot))
                .find_map(|node| node.get(&binding.key).copied())
                .unwrap_or(0.0);
            match binding.signal.strip_prefix(CONTROL_PREFIX) {
                Some(name) => {
                    match self.control_position(name) {
                        Some(at) => binding.drive(at),
                        None => binding.hold(manual),
                    };
                }
                None => {
                    binding.resolve(signals, manual);
                }
            }
        }
        self.bindings = bindings;
    }
}
