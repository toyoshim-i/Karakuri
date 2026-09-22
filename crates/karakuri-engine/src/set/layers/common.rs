use std::collections::HashMap;

use karakuri_ir::Kind;

use crate::binding::Binding;
use crate::set::types::Set;

impl Set {
    /// Returns simulation time in seconds after `n` steps.
    #[inline]
    pub(crate) fn t_at(&self, n: u64) -> f32 {
        n as f32 * self.dt
    }

    /// Returns the index of the built-in camera within the L3 layer, if present.
    pub(crate) fn builtin_camera(&self) -> Option<usize> {
        self.cameras.iter().position(|c| c.is_builtin())
    }

    /// Returns the slot index of the built-in camera in [`Set::params`].
    ///
    /// The built-in camera is only addressable explicitly and is excluded from wildcard writes.
    pub(crate) fn addressed_only(&self) -> Option<usize> {
        self.builtin_camera()
            .and_then(|at| self.nodes_of(Kind::L3).nth(at))
    }

    /// Returns the number of procedures of the given `layer` in this set.
    ///
    /// A procedure chain is instantiated once per source, but procedures are shared.
    pub(crate) fn procedures(&self, layer: Kind) -> usize {
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
    pub(crate) fn slot_of(&self, layer: Kind) -> usize {
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
    pub(crate) fn nodes_of(&self, layer: Kind) -> std::ops::Range<usize> {
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
    pub(crate) fn declared_names(&self, layer: Kind) -> Vec<&[String]> {
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

    /// Returns the declared parameter range, taking the intersection over all matching nodes.
    pub(crate) fn declared_range(&self, at: Option<(Kind, u32)>, key: &str) -> Option<[f32; 2]> {
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
}

/// Returns the effective value for `name`, resolving bindings before manual values.
pub(crate) fn effective(
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
pub(crate) fn effective_vector(
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
pub(crate) fn field_value(
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

pub(crate) fn kind_of_layer(layer: karakuri_store::record::Layer) -> Kind {
    match layer {
        karakuri_store::record::Layer::L1 => Kind::L1,
        karakuri_store::record::Layer::L2 => Kind::L2,
        karakuri_store::record::Layer::L3 => Kind::L3,
        karakuri_store::record::Layer::L4 => Kind::L4,
        karakuri_store::record::Layer::Field => Kind::Field,
        karakuri_store::record::Layer::L5 => Kind::L5,
    }
}

pub(crate) fn parse_component_key(key: &str) -> Option<(&str, usize)> {
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

pub(crate) fn update_value_component(
    vec_val: &mut karakuri_store::record::Value,
    comp_idx: usize,
    val: f32,
) {
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
pub(crate) fn source_value(
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
