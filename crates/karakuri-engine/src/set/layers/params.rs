use karakuri_ir::Kind;

use super::common::{kind_of_layer, parse_component_key, update_value_component};
use crate::binding::ParamWrite;
use crate::camera::Orbit;
use crate::set::types::{
    Authority, Bound, CrossesAuthority, Layering, PublishError, Published, Set,
};

impl Set {
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
    pub(crate) fn control_position(&self, name: &str) -> Option<f32> {
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
}
