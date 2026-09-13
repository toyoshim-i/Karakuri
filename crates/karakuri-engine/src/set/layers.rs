//! Pass execution and runtime frame processing for Set instances.

use std::collections::HashMap;

use karakuri_ir::layout::ElementLayout;
use karakuri_ir::Kind;

use crate::binding::{Binding, ParamWrite, Signals, CONTROL_PREFIX};
use crate::camera::Orbit;
use crate::mix::Input;
use crate::node::{Deform, Simulation};
use crate::set::types::{
    Authority, Bound, Clock, CrossesAuthority, ElementStorage, Layering, PublishError, Published,
    Set, Source, MAX_STEPS,
};
use crate::video_source::VideoSource;

impl Set {
    /// Resizes renderer viewports and accumulation targets to match new dimensions.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.viewport = [width.max(1) as f32, height.max(1) as f32];
        for renderer in self.sources.iter_mut().flat_map(|s| &mut s.renderers) {
            renderer.resize(device, width, height);
        }
        if let Some(merge) = &mut self.merge {
            merge.resize(device, width, height);
        }
    }

    /// Returns the currently clamped viewport dimensions `(width, height)`.
    pub fn viewport(&self) -> (u32, u32) {
        (self.viewport[0] as u32, self.viewport[1] as u32)
    }

    /// Returns the committed simulation steps elapsed.
    pub fn steps_taken(&self) -> u64 {
        self.steps_taken
    }

    /// Returns the uncommitted steps staged for the current frame.
    pub fn staged_delta(&self) -> u64 {
        self.staged_delta
    }

    /// Returns the active ping-pong buffer index for the primary geometry.
    pub fn parity(&self) -> usize {
        self.sources.first().map_or(0, |s| s.sim.parity())
    }

    /// Returns the committed ping-pong buffer index for the primary geometry.
    pub fn committed_parity(&self) -> usize {
        self.sources.first().map_or(0, |s| s.sim.committed_parity())
    }

    /// Commits staged clock steps, buffer parities, and composite input edges upon submission.
    pub fn commit(&mut self) {
        self.steps_taken += self.staged_delta;
        self.staged_delta = 0;
        if let Some(edges) = self.staged_edges.take() {
            self.edges = edges;
        }
        for source in &mut self.sources {
            source.sim.commit();
            if let Some(other) = &mut source.paired {
                other.commit();
            }
        }
    }

    /// Discards staged simulation clock advancement and ping-pong parities.
    pub fn discard(&mut self) {
        self.staged_delta = 0;
        self.staged_edges = None;
        for source in &mut self.sources {
            source.sim.discard();
            if let Some(other) = &mut source.paired {
                other.discard();
            }
        }
    }

    /// Returns elapsed simulation time in seconds.
    pub fn time(&self) -> f32 {
        self.t_at(self.steps_taken)
    }

    /// Returns simulation time in seconds after `n` steps.
    fn t_at(&self, n: u64) -> f32 {
        n as f32 * self.dt
    }

    /// Reads back the total active element count across all sources. Blocks GPU queue.
    pub fn live_count(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> u32 {
        self.sources
            .iter()
            .map(|s| s.sim.live_count(device, queue))
            .sum()
    }

    /// Reads back raw element buffer bytes decoded against the layout. Blocks GPU queue.
    pub fn read_elements(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Vec<u8> {
        self.sources[0].sim.read_elements(device, queue)
    }

    /// Returns the primary geometry's element buffer layout.
    pub fn element_layout(&self) -> &ElementLayout {
        self.sources[0].sim.element_layout()
    }

    /// Returns the total element capacity across all geometry sources.
    pub fn capacity(&self) -> u32 {
        self.sources.iter().map(|s| s.sim.capacity()).sum()
    }

    /// Returns true if all Set procedures are closed-form functions of seed, time, and params.
    pub fn is_closed_form(&self) -> bool {
        self.closed_form
    }

    /// Returns true if any procedure in the Set references the ambient beat count.
    pub fn reads_beats(&self) -> bool {
        self.reads_beats
    }

    /// Seeks the simulation clock directly to `steps_taken` without running intermediate steps.
    pub fn seek(&mut self, steps_taken: u64) {
        self.discard();
        self.steps_taken = steps_taken;
    }

    /// Resets simulation buffers and clock state back to initial post-build conditions.
    pub fn rewind(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue) {
        self.discard();
        self.steps_taken = 0;
        for source in &mut self.sources {
            source.sim.rewind(queue);
            if let Some(other) = &mut source.paired {
                other.rewind(queue);
            }
        }
    }

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

    /// Returns the element storage allocation for each node instance in the Set.
    pub fn element_storage(&self) -> Vec<ElementStorage> {
        self.sources
            .iter()
            .flat_map(|source| {
                std::iter::once(source.sim.element_storage())
                    .chain(source.paired.iter().map(Simulation::element_storage))
                    .chain(source.deforms.iter().map(Deform::element_storage))
            })
            .collect()
    }

    /// Returns the total bytes allocated across all element buffers in the Set.
    pub fn element_storage_bytes(&self) -> u64 {
        self.element_storage().iter().map(|e| e.bytes).sum()
    }

    /// Returns canonical names for each node in execution order.
    pub fn node_names(&self) -> &[String] {
        &self.names
    }

    /// Returns hash salts assigned to each geometry source.
    pub fn source_salts(&self) -> &[u32] {
        &self.source_salts
    }

    /// Returns allocated element capacities for each geometry source.
    pub fn source_capacities(&self) -> Vec<u32> {
        let mut out = vec![0; self.l1_count];
        for source in &self.sources {
            for (k, sim) in std::iter::once(&source.sim)
                .chain(source.paired.iter())
                .enumerate()
            {
                out[source.procedures[k]] = sim.capacity();
            }
        }
        out
    }

    /// Returns declared `[min, max, default]` capacity specifications for each geometry.
    pub fn declared_capacities(&self) -> &[[u32; 3]] {
        &self.declared_capacities
    }

    /// Looks up a node by name, returning its `(layer, index)` address if found.
    pub fn node_named(&self, name: &str) -> Option<(Kind, u32)> {
        let at = self.names.iter().position(|n| n == name)?;
        Kind::ALL.into_iter().find_map(|kind| {
            let range = self.nodes_of(kind);
            range
                .contains(&at)
                .then(|| (kind, (at - range.start) as u32))
        })
    }

    /// Returns the number of procedures of the given `layer` in this set.
    ///
    /// A procedure chain is instantiated once per source, but procedures are shared.
    fn procedures(&self, layer: Kind) -> usize {
        let first = &self.sources[0];
        match layer {
            Kind::L1 => self.l1_count,
            Kind::L2 => first.deforms.len(),
            Kind::L3 => self.cameras.len(),
            Kind::L4 => first.renderers.len(),
            Kind::Field => self.field_count,
            // Nested L5 nodes are not yet supported.
            Kind::L5 => 0,
        }
    }

    /// Returns the starting index of a layer's parameter maps in [`Set::params`].
    fn slot_of(&self, layer: Kind) -> usize {
        match layer {
            Kind::L1 => 0,
            Kind::L2 => self.l1_count,
            Kind::L3 => self.l1_count + self.procedures(Kind::L2),
            Kind::L4 => self.l1_count + self.procedures(Kind::L2) + self.procedures(Kind::L3),
            Kind::Field => {
                self.l1_count
                    + self.procedures(Kind::L2)
                    + self.procedures(Kind::L3)
                    + self.procedures(Kind::L4)
            }
            Kind::L5 => self.params.len(),
        }
    }

    /// Returns the range of indices in [`Set::params`] belonging to `layer`, in node order.
    fn nodes_of(&self, layer: Kind) -> std::ops::Range<usize> {
        let start = self.slot_of(layer);
        match layer {
            Kind::L1 => start..start + self.l1_count,
            Kind::L2 => start..start + self.procedures(Kind::L2),
            Kind::L3 => start..start + self.procedures(Kind::L3),
            Kind::L4 => start..self.params.len() - self.field_count,
            Kind::Field => start..start + self.field_count,
            Kind::L5 => start..start,
        }
    }

    /// Returns the declared parameter keys for each node of `layer`, in declaration order.
    ///
    /// Entries align with [`Set::nodes_of`]. Vector parameters are expanded to individual
    /// component keys (`x`, `y`, `z`).
    fn declared_names(&self, layer: Kind) -> Vec<&[String]> {
        match layer {
            Kind::L1 => {
                let mut out: Vec<&[String]> = vec![&[]; self.l1_count];
                for source in &self.sources {
                    for (k, sim) in std::iter::once(&source.sim)
                        .chain(source.paired.iter())
                        .enumerate()
                    {
                        out[source.procedures[k]] = sim.param_keys();
                    }
                }
                out
            }
            Kind::L2 => self.sources[0]
                .deforms
                .iter()
                .map(|d| d.param_keys())
                .collect(),
            Kind::L3 => self.cameras.iter().map(|c| c.param_keys()).collect(),
            Kind::Field => self.field_declared.iter().map(Vec::as_slice).collect(),
            Kind::L4 => self.sources[0]
                .renderers
                .iter()
                .map(|r| r.param_keys())
                .collect(),
            Kind::L5 => Vec::new(),
        }
    }

    /// Sets every declaration of `name` across all applicable nodes, returning the count written.
    ///
    /// The built-in camera is excluded from bare name writes; see [`Set::addressed_only`].
    pub fn set_param(&mut self, name: &str, value: f32) -> usize {
        let mut written = 0;
        let addressed_only = self.addressed_only();
        for (at, (node, moved)) in self
            .params
            .iter_mut()
            .zip(self.moved.iter_mut())
            .enumerate()
        {
            if Some(at) == addressed_only {
                continue;
            }
            if let Some(slot) = node.get_mut(name) {
                *slot = value;
                moved.insert(name.to_string());
                written += 1;
            }
        }
        if written > 0 {
            for (slot, node_values) in self.param_values.iter_mut().enumerate() {
                if Some(slot) == addressed_only {
                    continue;
                }
                if let Some((base, comp_idx)) = parse_component_key(name) {
                    if let Some(vec_val) = node_values.get_mut(base) {
                        update_value_component(vec_val, comp_idx, value);
                    }
                } else if let Some(karakuri_store::record::Value::Scalar(s)) =
                    node_values.get_mut(name)
                {
                    *s = value;
                }
            }
        }
        written
    }

    /// Sets a specific node's declaration of `name`.
    ///
    /// Returns `false` if the node does not exist or does not declare `name`.
    pub fn set_param_at(&mut self, layer: Kind, index: u32, name: &str, value: f32) -> bool {
        let Some(slot) = self.nodes_of(layer).nth(index as usize) else {
            return false;
        };
        match self.params.get_mut(slot).and_then(|n| n.get_mut(name)) {
            Some(held) => {
                *held = value;
                self.moved[slot].insert(name.to_string());
                if let Some((base, comp_idx)) = parse_component_key(name) {
                    if let Some(vec_val) = self.param_values[slot].get_mut(base) {
                        update_value_component(vec_val, comp_idx, value);
                    }
                } else if let Some(karakuri_store::record::Value::Scalar(s)) =
                    self.param_values[slot].get_mut(name)
                {
                    *s = value;
                }
                true
            }
            None => false,
        }
    }

    /// Sets a parameter value atomically, either addressed to a node or across all matching nodes.
    ///
    /// Setting a vector parameter under its bare name updates all components atomically.
    pub fn set_param_value(
        &mut self,
        at: Option<karakuri_store::record::NodeAddress>,
        key: &str,
        value: karakuri_store::record::Value,
    ) -> usize {
        match at {
            Some(addr) => {
                let layer = kind_of_layer(addr.layer);
                usize::from(self.set_param_value_at(layer, addr.index, key, value))
            }
            None => {
                let mut written = 0;
                let addressed_only = self.addressed_only();
                let len = self.param_values.len();
                for slot in 0..len {
                    if Some(slot) == addressed_only {
                        continue;
                    }
                    if self.set_param_value_in_slot(slot, key, value) {
                        written += 1;
                    }
                }
                written
            }
        }
    }

    /// Sets one node's declaration of `name` to `value` atomically.
    pub fn set_param_value_at(
        &mut self,
        layer: Kind,
        index: u32,
        name: &str,
        value: karakuri_store::record::Value,
    ) -> bool {
        let Some(slot) = self.nodes_of(layer).nth(index as usize) else {
            return false;
        };
        self.set_param_value_in_slot(slot, name, value)
    }

    fn set_param_value_in_slot(
        &mut self,
        slot: usize,
        key: &str,
        value: karakuri_store::record::Value,
    ) -> bool {
        // Case 1: `key` matches a declaration in `param_values[slot]`
        if let Some(decl_val) = self
            .param_values
            .get(slot)
            .and_then(|m| m.get(key).copied())
        {
            if decl_val.len() == value.len() {
                self.param_values[slot].insert(key.to_string(), value);
                if value.len() == 1 {
                    if let Some(s) = value.get(0) {
                        self.params[slot].insert(key.to_string(), s);
                        self.moved[slot].insert(key.to_string());
                    }
                } else {
                    for (i, v) in value.components().iter().enumerate() {
                        let comp_key = karakuri_ir::component_key(key, i);
                        self.params[slot].insert(comp_key.clone(), *v);
                        self.moved[slot].insert(comp_key);
                    }
                    self.moved[slot].insert(key.to_string());
                }
                return true;
            }
            return false;
        }

        // Case 2: `key` is a single component key (e.g. "glow.y") and `value` is Scalar
        if value.len() == 1 {
            let scalar = value.get(0).unwrap();
            if let Some(held) = self.params.get_mut(slot).and_then(|m| m.get_mut(key)) {
                *held = scalar;
                self.moved[slot].insert(key.to_string());

                if let Some((base, comp_idx)) = parse_component_key(key) {
                    if let Some(vec_val) = self.param_values[slot].get_mut(base) {
                        update_value_component(vec_val, comp_idx, scalar);
                    }
                }
                return true;
            }
        }

        false
    }

    /// Returns the index of the built-in camera within the L3 layer, if present.
    fn builtin_camera(&self) -> Option<usize> {
        self.cameras.iter().position(|c| c.is_builtin())
    }

    /// Returns the slot index of the built-in camera in [`Set::params`].
    ///
    /// The built-in camera is only addressable explicitly and is excluded from wildcard writes.
    fn addressed_only(&self) -> Option<usize> {
        self.builtin_camera()
            .and_then(|at| self.nodes_of(Kind::L3).nth(at))
    }

    /// Returns the current state of the built-in camera orbit.
    pub fn orbit(&self) -> Orbit {
        let slot = self.addressed_only();
        self.camera
            .with_placement(|key| slot.and_then(|slot| self.params[slot].get(key).copied()))
    }

    /// Updates the built-in camera orbit state and its parameter values.
    pub fn aim_camera(&mut self, orbit: Orbit) {
        self.camera = orbit;
        let Some(slot) = self.addressed_only() else {
            return;
        };
        for (key, value) in orbit.placement_values() {
            if let Some(held) = self.params[slot].get_mut(&key) {
                *held = value;
            }
            if let Some(held_val) = self.param_values[slot].get_mut(&key) {
                *held_val = karakuri_store::record::Value::Scalar(value);
            }
        }
    }

    /// Returns the [`Authority`] of the node at `(layer, index)`, or `None` if it does not exist.
    pub fn authority(&self, layer: Kind, index: u32) -> Option<Authority> {
        let slot = self.nodes_of(layer).nth(index as usize)?;
        self.authorities.get(slot).copied()
    }

    /// Sets the [`Authority`] of the node at `(layer, index)`.
    ///
    /// Returns `false` if the node does not exist.
    pub fn set_authority(&mut self, layer: Kind, index: u32, authority: Authority) -> bool {
        let Some(slot) = self.nodes_of(layer).nth(index as usize) else {
            return false;
        };
        match self.authorities.get_mut(slot) {
            Some(held) => {
                *held = authority;
                true
            }
            None => false,
        }
    }

    /// Returns the `(layer, index)` coordinates of every node declaring `key`.
    pub fn landing_of(&self, key: &str) -> Vec<(Kind, u32)> {
        self.landing(key)
            .into_iter()
            .map(|(layer, index, _)| (layer, index))
            .collect()
    }

    /// Returns every node declaring `key`, along with its effective authority.
    fn landing(&self, key: &str) -> Vec<(Kind, u32, Authority)> {
        Kind::ALL
            .into_iter()
            .flat_map(|layer| {
                self.nodes_of(layer)
                    .enumerate()
                    .map(move |(index, slot)| (layer, index as u32, slot))
            })
            .filter(|(_, _, slot)| Some(*slot) != self.addressed_only())
            .filter(|(_, _, slot)| self.params[*slot].contains_key(key))
            .map(|(layer, index, slot)| {
                (
                    layer,
                    index,
                    self.authorities.get(slot).copied().unwrap_or_default(),
                )
            })
            .collect()
    }

    /// Applies a [`ParamWrite`], addressed or wildcarded across nodes.
    ///
    /// Returns the number of nodes updated, or an error if a wildcard write
    /// crosses conflicting node authorities.
    pub fn write_param(&mut self, write: &ParamWrite) -> Result<usize, CrossesAuthority> {
        match write.at {
            None => {
                if let Some(refused) = CrossesAuthority::over(&write.key, &self.landing(&write.key))
                {
                    return Err(refused);
                }
                Ok(self.set_param(&write.key, write.value))
            }
            Some((layer, index)) => Ok(usize::from(self.set_param_at(
                layer,
                index,
                &write.key,
                write.value,
            ))),
        }
    }

    /// Returns whether `key` at `(layer, index)` was modified after initialization.
    fn moved_at(&self, layer: Kind, index: u32, key: &str) -> bool {
        self.nodes_of(layer)
            .nth(index as usize)
            .is_some_and(|slot| self.moved[slot].contains(key))
    }

    /// Copies modified parameter values from an `outgoing` set to this set.
    ///
    /// Only parameters modified in `outgoing` and still declared in `self` are copied.
    /// Keys already modified in `self` are preserved. Returns the count of carried values.
    pub fn carry_moved_from(&mut self, outgoing: &Set) -> usize {
        let mut carried = 0;
        let moved: Vec<(Kind, u32, &str, f32)> = Kind::ALL
            .into_iter()
            .flat_map(|layer| {
                outgoing
                    .nodes_of(layer)
                    .enumerate()
                    .map(move |(index, slot)| (layer, index as u32, slot))
            })
            .flat_map(|(layer, index, slot)| {
                outgoing.moved[slot]
                    .iter()
                    .filter_map(move |key| {
                        Some((layer, index, key.as_str(), *outgoing.params[slot].get(key)?))
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        for (layer, index, key, value) in moved {
            if self.moved_at(layer, index, key) {
                continue;
            }
            if self.set_param_at(layer, index, key, value) {
                carried += 1;
            }
        }
        carried
    }

    /// Copies dynamic signal bindings from an `outgoing` set to this set.
    ///
    /// Preserves bindings already stated on this set. Returns the count of carried bindings.
    pub fn carry_bound_from(&mut self, outgoing: &Set) -> usize {
        let mut carried = 0;
        for binding in &outgoing.bindings {
            let stated = self.bindings.iter().any(|b| {
                b.layer == binding.layer && b.index == binding.index && b.key == binding.key
            });
            if stated {
                continue;
            }
            if self.bind(binding.clone()) == Bound::Yes {
                carried += 1;
            }
        }
        carried
    }

    /// Adds a control to this set's published interface.
    ///
    /// Calling this method switches the set from publishing all declared parameters
    /// by default to publishing only explicitly added controls.
    pub fn publish(&mut self, control: Published) -> Result<(), PublishError> {
        let Some([min, max]) = self.declared_range(control.at, &control.key) else {
            return Err(PublishError::NoSuchControl {
                key: control.key,
                at: match control.at {
                    Some((layer, index)) => format!(" at {layer:?}:{index}"),
                    None => String::new(),
                },
            });
        };
        let [low, high] = control.range;
        if low < min || high > max || low > high {
            return Err(PublishError::RangeNotASubset {
                name: control.name,
                key: control.key,
                low,
                high,
                min,
                max,
            });
        }
        if self.interface.iter().any(|p| p.name == control.name) {
            return Err(PublishError::DuplicateName(control.name));
        }
        self.interface.push(control);
        Ok(())
    }

    /// Returns the declared parameter range, taking the intersection over all matching nodes.
    fn declared_range(&self, at: Option<(Kind, u32)>, key: &str) -> Option<[f32; 2]> {
        let mut found: Option<[f32; 2]> = None;
        for layer in Kind::ALL {
            for (index, slot) in self.nodes_of(layer).enumerate() {
                if at.is_some_and(|(l, i)| l != layer || i != index as u32) {
                    continue;
                }
                // Wildcard ranges do not narrow against the built-in camera.
                if at.is_none() && Some(slot) == self.addressed_only() {
                    continue;
                }
                let Some([min, max]) = self.ranges.get(slot).and_then(|n| n.get(key)).copied()
                else {
                    continue;
                };
                found = Some(match found {
                    None => [min, max],
                    Some([lo, hi]) => [lo.max(min), hi.min(max)],
                });
            }
        }
        found
    }

    /// Returns the published control interface for this set.
    ///
    /// If no controls were explicitly published via [`Set::publish`], returns the
    /// full declared interface in definition order.
    pub fn published(&self) -> Vec<Published> {
        if !self.interface.is_empty() {
            return self.interface.clone();
        }
        self.declared_interface()
    }

    /// Returns the full list of declared parameters as a published interface.
    ///
    /// The returned controls maintain a stable, deterministic order based on declaration order.
    pub fn declared_interface(&self) -> Vec<Published> {
        let mut keys: Vec<&String> = Vec::new();
        let mut out: Vec<Published> = Vec::new();
        for layer in Kind::ALL {
            for (index, names) in self.declared_names(layer).into_iter().enumerate() {
                let addressed = self.nodes_of(layer).nth(index) == self.addressed_only();
                for key in names {
                    if addressed {
                        let at = Some((layer, index as u32));
                        if let Some(range) = self.declared_range(at, key) {
                            out.push(Published {
                                name: key.clone(),
                                at,
                                key: key.clone(),
                                range,
                            });
                        }
                        continue;
                    }
                    if keys.contains(&key) {
                        continue;
                    }
                    keys.push(key);
                    if let Some(range) = self.declared_range(None, key) {
                        out.push(Published {
                            name: key.clone(),
                            at: None,
                            key: key.clone(),
                            range,
                        });
                    }
                }
            }
        }
        out
    }

    /// Sets a published control by name, clamping `value` to its published range.
    ///
    /// Returns `Ok(false)` if no control publishes `name`, or an error if the write
    /// crosses conflicting node authorities.
    pub fn set_published(&mut self, name: &str, value: f32) -> Result<bool, CrossesAuthority> {
        let Some(control) = self.published().into_iter().find(|p| p.name == name) else {
            return Ok(false);
        };
        let clamped = value.clamp(control.range[0], control.range[1]);
        Ok(self.write_param(&ParamWrite {
            at: control.at,
            key: control.key,
            value: clamped,
        })? > 0)
    }

    /// Returns a published control's normalized position in `[0.0, 1.0]`.
    fn control_position(&self, name: &str) -> Option<f32> {
        let (at, key, [low, high]) = match self.interface.iter().find(|p| p.name == name) {
            Some(control) => (control.at, control.key.as_str(), control.range),
            None if self.interface.is_empty() => (None, name, self.declared_range(None, name)?),
            None => return None,
        };
        let value = self.value_at(at, key)?;
        if high <= low {
            return Some(1.0);
        }
        Some(((value - low) / (high - low)).clamp(0.0, 1.0))
    }

    /// Returns the current value of a published control by name.
    pub fn published_value(&self, name: &str) -> Option<f32> {
        let control = self.published().into_iter().find(|p| p.name == name)?;
        self.value_at(control.at, &control.key)
    }

    /// Returns the parameter value for `key` at the specified node, or the first matching node for wildcards.
    pub fn value_at(&self, at: Option<(Kind, u32)>, key: &str) -> Option<f32> {
        for layer in Kind::ALL {
            for (index, slot) in self.nodes_of(layer).enumerate() {
                if at.is_some_and(|(l, i)| l != layer || i != index as u32) {
                    continue;
                }
                if let Some(v) = self.params.get(slot).and_then(|n| n.get(key)) {
                    return Some(*v);
                }
            }
        }
        None
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

    /// Returns the rendered primitive topologies in draw order across all sources.
    pub fn drawn_topologies(&self) -> Vec<karakuri_ir::Topology> {
        self.sources
            .iter()
            .flat_map(|s| &s.renderers)
            .map(|r| r.topology())
            .collect()
    }

    /// Returns the conservative primitive rate bounds for each L4 procedure.
    pub fn rate_bounds(&self) -> &[karakuri_ir::rate::RateBound] {
        &self.rate_bounds
    }

    /// Returns the first parameter whose current value violates its rate bound range.
    pub fn rate_bound_contradicted(&self) -> Option<(String, f32, [f32; 2])> {
        let l4s = self.nodes_of(Kind::L4);
        for (bound, slot) in self.rate_bounds.iter().zip(l4s) {
            let karakuri_ir::rate::Bound::AtLeast { over, .. } = &bound.bound else {
                continue;
            };
            let (Some(values), Some(ranges)) = (self.params.get(slot), self.ranges.get(slot))
            else {
                continue;
            };
            for name in over {
                for (key, declared) in ranges {
                    if key != name && !key.strip_prefix(name).is_some_and(|r| r.starts_with('.')) {
                        continue;
                    }
                    let Some(&value) = values.get(key) else {
                        continue;
                    };
                    if value < declared[0] || value > declared[1] {
                        return Some((key.clone(), value, *declared));
                    }
                }
            }
        }
        None
    }

    /// Returns the layering strategy used by this set.
    pub fn layering(&self) -> Layering {
        if self.merge.is_some() {
            Layering::Composite
        } else {
            Layering::Overdraw
        }
    }

    /// Returns the scalar parameter value of `name` from the first declaring node.
    pub fn param(&self, name: &str) -> Option<f32> {
        self.params.iter().find_map(|node| node.get(name).copied())
    }

    /// Returns the typed parameter [`Value`](karakuri_store::record::Value) of `name` from the first declaring node.
    pub fn param_value(&self, name: &str) -> Option<karakuri_store::record::Value> {
        self.param_values
            .iter()
            .find_map(|node| node.get(name).copied())
    }

    /// Returns the typed parameter [`Value`](karakuri_store::record::Value) at `(layer, index)`.
    pub fn param_value_at(
        &self,
        layer: Kind,
        index: u32,
        name: &str,
    ) -> Option<karakuri_store::record::Value> {
        let slot = self.nodes_of(layer).nth(index as usize)?;
        self.param_values
            .get(slot)
            .and_then(|node| node.get(name).copied())
    }

    /// Returns the typed parameter [`Value`](karakuri_store::record::Value) at `address`.
    pub fn param_value_at_address(
        &self,
        address: karakuri_store::record::NodeAddress,
        name: &str,
    ) -> Option<karakuri_store::record::Value> {
        let layer = kind_of_layer(address.layer);
        self.param_value_at(layer, address.index, name)
    }

    /// Returns an iterator over all parameter values with their node coordinates.
    pub fn params(&self) -> impl Iterator<Item = (Kind, u32, &str, f32)> + '_ {
        let addressed: Vec<(Kind, u32, usize)> = Kind::ALL
            .into_iter()
            .flat_map(|layer| {
                self.nodes_of(layer)
                    .enumerate()
                    .map(move |(index, slot)| (layer, index as u32, slot))
            })
            .collect();
        addressed.into_iter().flat_map(move |(layer, index, slot)| {
            self.params[slot]
                .iter()
                .map(move |(k, v)| (layer, index, k.as_str(), *v))
        })
    }

    /// Returns an iterator over all bound parameters and their most recent evaluated values.
    pub fn bound(&self) -> impl Iterator<Item = (&str, f32)> {
        self.bindings.iter().map(|b| (b.key.as_str(), b.value()))
    }

    /// Returns the active parameter signal bindings.
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
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

    /// Advances simulation and deformation passes for the current frame without rasterizing.
    pub fn step(&mut self, encoder: &mut wgpu::CommandEncoder, steps: u8) {
        let steps = if self
            .sources
            .iter()
            .flat_map(|s| &s.renderers)
            .all(|r| r.is_fullscreen())
        {
            0
        } else {
            steps
        };
        for source in &mut self.sources {
            source.sim.record(encoder, steps);
            if let Some(other) = &mut source.paired {
                other.record(encoder, steps);
            }
        }
        self.record_counts(encoder);
        for source in &self.sources {
            let parity = source.sim.parity();
            let mut counts = source.sim.counts();
            for node in &source.deforms {
                node.record(encoder, parity, counts);
                if let Some(own) = node.counts() {
                    counts = own;
                }
            }
        }
    }

    /// Records amplifier compute passes to derive instance counts.
    pub(crate) fn record_counts(&self, encoder: &mut wgpu::CommandEncoder) {
        for node in self.sources.iter().flat_map(|s| &s.deforms) {
            node.record_counts(encoder);
        }
    }

    /// Returns the output count buffer from the last amplifier in the deformation chain.
    pub(crate) fn output_counts<'a>(&self, source: &'a Source) -> &'a wgpu::Buffer {
        source
            .deforms
            .iter()
            .rev()
            .find_map(|node| node.counts())
            .unwrap_or_else(|| source.sim.counts())
    }

    /// Records rasterization passes for all renderers into `target`.
    pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::TextureView) {
        for camera in &self.cameras {
            camera.record(encoder);
        }
        let merge = self.merge.as_ref();
        for (source_at, source) in self.sources.iter().enumerate() {
            let (parity, counts) = (source.sim.parity(), self.output_counts(source));
            for (i, renderer) in source.renderers.iter().enumerate() {
                match merge {
                    None => {
                        renderer.draw(encoder, target, parity, counts, source_at == 0 && i == 0)
                    }
                    Some(merge) => {
                        renderer.draw(encoder, merge.target(i), parity, counts, source_at == 0)
                    }
                }
            }
        }
        if let Some(merge) = merge {
            merge.record(encoder, target);
        }
    }
}

impl VideoSource for Set {
    fn render(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        steps: u8,
    ) {
        self.step(encoder, steps);
        self.draw(encoder, target);
    }

    fn commit(&mut self) {
        self.commit();
    }

    fn discard(&mut self) {
        self.discard();
    }
}

/// Returns the effective value for `name`, resolving bindings before manual values.
fn effective(
    bindings: &[Binding],
    params: &HashMap<String, f32>,
    layer: Kind,
    index: usize,
    name: &str,
) -> Option<f32> {
    match bindings
        .iter()
        .find(|b| b.layer == layer && b.key == name && b.covers(index))
    {
        Some(binding) => Some(binding.value()),
        None => params.get(name).copied(),
    }
}

/// Returns the effective vector value, falling back to `None` if any component is driven by a binding.
fn effective_vector(
    bindings: &[Binding],
    param_values: &HashMap<String, karakuri_store::record::Value>,
    layer: Kind,
    index: usize,
    name: &str,
) -> Option<karakuri_store::record::Value> {
    let has_binding = bindings.iter().any(|b| {
        b.layer == layer
            && b.covers(index)
            && (b.key == name
                || (b.key.starts_with(name) && b.key.as_bytes().get(name.len()) == Some(&b'.')))
    });
    if has_binding {
        return None;
    }
    param_values.get(name).copied()
}

/// Resolves a spliced field parameter value for `node`.
fn field_value(
    bound: &[(usize, String, usize)],
    maps: &[HashMap<String, f32>],
    node: usize,
    key: &str,
) -> Option<f32> {
    let (slot, declared) = key.strip_prefix("field\u{1}")?.split_once('\u{1}')?;
    let (_, _, ordinal) = bound
        .iter()
        .find(|(at, name, _)| *at == node && name == slot)?;
    maps.get(*ordinal)?.get(declared).copied()
}

fn kind_of_layer(layer: karakuri_store::record::Layer) -> Kind {
    match layer {
        karakuri_store::record::Layer::L1 => Kind::L1,
        karakuri_store::record::Layer::L2 => Kind::L2,
        karakuri_store::record::Layer::L3 => Kind::L3,
        karakuri_store::record::Layer::L4 => Kind::L4,
        karakuri_store::record::Layer::Field => Kind::Field,
        karakuri_store::record::Layer::L5 => Kind::L5,
    }
}

fn parse_component_key(key: &str) -> Option<(&str, usize)> {
    let (base, comp) = key.rsplit_once('.')?;
    let idx = match comp {
        "x" | "r" => 0,
        "y" | "g" => 1,
        "z" | "b" => 2,
        "w" | "a" => 3,
        _ => return None,
    };
    Some((base, idx))
}

fn update_value_component(vec_val: &mut karakuri_store::record::Value, comp_idx: usize, val: f32) {
    match vec_val {
        karakuri_store::record::Value::Scalar(s) => {
            if comp_idx == 0 {
                *s = val;
            }
        }
        karakuri_store::record::Value::Vec2(arr) => {
            if comp_idx < 2 {
                arr[comp_idx] = val;
            }
        }
        karakuri_store::record::Value::Vec3(arr) => {
            if comp_idx < 3 {
                arr[comp_idx] = val;
            }
        }
        karakuri_store::record::Value::Vec4(arr) | karakuri_store::record::Value::Color(arr) => {
            if comp_idx < 4 {
                arr[comp_idx] = val;
            }
        }
    }
}

/// Resolves a declared source slot salt for `node`.
fn source_value(
    bound: &[(usize, String, usize)],
    salts: &[u32],
    node: usize,
    key: &str,
) -> Option<u32> {
    let slot = key.strip_prefix("source\u{1}")?;
    let (_, _, at) = bound
        .iter()
        .find(|(node_at, name, _)| *node_at == node && name == slot)?;
    salts.get(*at).copied()
}
