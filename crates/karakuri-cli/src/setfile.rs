//! Set files: the material, through the record stream at last.
//!
//! Everything else an operator moves — audio, tempo, the mix, the transport —
//! reaches the engine as a record. The *material* did not: two `.kir` paths and
//! a handful of flags went straight into `Set::build`, so `README.md`'s
//! invariant had to say "the performance is on the record path and the material
//! is not". This is the other half.
//!
//! Two directions, and both are needed or neither is worth anything:
//!
//! - [`save`] puts the two `.kir` sources into the store as content-addressed
//!   artifacts and writes a Set file that references them by hash, with the
//!   capacity, parameters, bindings, camera and seed the run was using.
//! - [`load`] reads one back and returns everything `Set::build` and the flags
//!   used to supply.
//!
//! ## Where the flag went
//!
//! `--bind`'s fields are `Record::Bind`'s fields, and `docs/roadmap.md` records
//! the debt that came with that: **two diagnostics guarding the flag — a `bpm`
//! binding, and `noise.octaves` on a kind that has no octaves — lived only in
//! the flag, and the decoder owed them too.** Paying that by writing them a
//! second time would be two copies of a rule that must not differ.
//!
//! So the flag is now what its documentation always claimed: **a way to write
//! the record**. `parse_bind` turns a `--bind` string into a [`Record::Bind`]
//! and hands it to [`binding_from_record`], which is where every semantic check
//! lives. One rule, one place, and a Set file and a command line cannot disagree
//! about what a binding means.
//!
//! ## Three places the format is finer than the engine
//!
//! The Set file keys `seed`, `capacity` and `param` by **layer**; the engine
//! holds one seed, one capacity, and one flat parameter map per Set. A `param`
//! may also be a vector, and the engine's map holds `f32`.
//!
//! None of that is resolved here and none of it is silently dropped. Loading
//! reports what it could not carry — see [`Loaded::notes`] — because a Set file
//! that half-applies is the failure mode this repository keeps refusing:
//! checking clean and coming up short later. The gaps themselves are the
//! format's and the engine's to settle, and `Set::build` refusing a colliding
//! param name is the same disagreement seen from the other side.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use karakuri_engine::binding::{Curve, NOISE_SIGNAL};
use karakuri_engine::camera::Orbit;
use karakuri_engine::{Binding, ParamWrite};
use karakuri_ir::typed::Checked;
use karakuri_ir::Kind;
use karakuri_signal::{NoiseConfig, NoiseKind};
use karakuri_store::hash::Hash;
use karakuri_store::ndjson::Line;
use karakuri_store::record::{BindNoise, Layer, Record, Value};
use karakuri_store::store::Store;

/// The Set file format version this build writes. One number for the whole
/// file, on `Record::Set`.
const VERSION: u32 = 1;

/// The octave count an `fbm` binding gets when it does not say. Matches
/// `karakuri-store`'s `BindNoise` default, which is the record this stands for.
pub const DEFAULT_OCTAVES: u32 = 4;

/// What a Set file said, in the terms the engine takes.
#[cfg_attr(test, derive(Debug))]
pub struct Loaded {
    pub id: String,
    pub l1: Checked,
    /// The renderers, in the order their `slot` records appeared — which is
    /// draw order. **Several `slot` records on L4 is how a file says a stack**;
    /// the format already allowed it and nothing new had to be added.
    pub l4s: Vec<Checked>,
    /// Every procedure as text — the L1, then each renderer in draw order.
    ///
    /// **Carried because a Set file has no `.kir` on disk and an editable run
    /// needs one.** A Set names its procedures by hash; the sources come out of
    /// the store, or out of the file when it was bundled. Before the scratch
    /// existed there was nowhere to put them and `--mcp` with `--load-set` was
    /// refused for exactly that reason. See `scratch::place`.
    pub l1_src: String,
    pub l4_srcs: Vec<String>,
    /// `None` when the file gave no `capacity` record, which means the `.kir`
    /// default applies — the spec's own wording.
    pub capacity: Option<u32>,
    pub params: Vec<ParamWrite>,
    pub bindings: Vec<Binding>,
    pub camera: Option<Orbit>,
    pub seed: Option<u32>,
    /// **What could not be carried across, in the operator's words.**
    ///
    /// Not warnings to be counted and not errors: a Set file that mentions a
    /// second layer's seed is a valid file this engine cannot honour in full,
    /// and the honest response is to load it and say so. Silence here would be
    /// the load succeeding and the material being subtly not what was saved.
    pub notes: Vec<String>,
}

/// Convert one [`Record::Bind`] into the binding the engine applies.
///
/// **The one place a binding's semantics live.** `--bind` reaches here too, so
/// the flag and a Set file cannot mean different things by the same fields.
///
/// The two diagnostics `docs/roadmap.md` says the decoder owes are here and
/// nowhere else:
///
/// - **`signal=bpm` is refused.** A tempo is not a `[0, 1]` signal, so the
///   curve clamps it and the binding sits pinned at the top of its range for
///   the whole run. From the outside a pinned binding and a working one are the
///   same number on a status line, which is exactly why this cannot be a
///   silent clamp. `beat` and `bar` carry the same tempo in the range a binding
///   is defined over.
/// - **`noise.octaves` needs `kind=fbm`.** The other three generators have no
///   layers, so an octave count on one of them is asking for a generator nobody
///   named. Refused rather than ignored, for the same reason.
///
/// And one more that is the same shape: `noise` on a binding whose signal is
/// not `noise` is refused, because accepting it leaves an operator re-reading
/// the noise fields to find out why the parameter does not move.
pub fn binding_from_record(record: &Record) -> Result<Binding, String> {
    let Record::Bind {
        layer,
        index,
        key,
        signal,
        curve,
        range,
        noise,
    } = record
    else {
        return Err("not a `bind` record".to_string());
    };

    let bad = |what: String| format!("bind {}={key}: {what}", layer_name(*layer));

    let kind = match layer {
        Layer::L1 => Kind::L1,
        Layer::L4 => Kind::L4,
        other => {
            return Err(bad(format!(
                "layer {} — this engine builds L1 and L4 only",
                layer_name(*other)
            )))
        }
    };
    let curve = Curve::parse(curve)
        .ok_or_else(|| bad(format!("curve `{curve}` — expected lin, pow2, sqrt or smooth")))?;

    if signal == "bpm" {
        return Err(bad(
            "signal `bpm` — a tempo is not a [0, 1] signal, so the curve clamps it and this \
             binding would sit at the top of its range for the whole run; bind `beat` or \
             `bar` instead"
                .to_string(),
        ));
    }

    let mut binding = Binding::new(kind, key.clone(), signal.clone(), curve, *range);
    // Absent stays absent: a binding with no `index` is the layer's, every node
    // declaring the key — see `Binding::index`.
    if let Some(at) = index {
        binding = binding.at(*at);
    }
    if signal != NOISE_SIGNAL {
        if noise.is_some() {
            return Err(bad(format!(
                "a generator needs `signal={NOISE_SIGNAL}`, and this binds `{signal}`"
            )));
        }
        return Ok(binding);
    }
    // **Absent means the default generator, not the absence of one**, which is
    // what `Record::Bind::noise` says and the only thing the name can mean: a
    // binding to `noise` with nothing else said is a binding to the default
    // generator. Materialised here rather than left as `None` so that what the
    // binding carries is what it will use.
    let noise = noise.clone().unwrap_or_default();
    let noise = &noise;
    // Read before the kind is folded, because `NoiseKind` carries the octave
    // count inside the `fbm` variant: once folded there is nothing left to
    // check against, and an octave count on a `white` would have turned it into
    // an `fbm` on the way past.
    let octaves_named = noise.octaves != DEFAULT_OCTAVES;
    if octaves_named && noise.kind != "fbm" {
        return Err(bad(format!(
            "`octaves` needs kind `fbm`, and this asks for `{}`",
            noise.kind
        )));
    }
    let kind = match noise.kind.as_str() {
        "white" => NoiseKind::White,
        "value" => NoiseKind::Value,
        "perlin" => NoiseKind::Perlin,
        "fbm" => NoiseKind::Fbm {
            octaves: noise.octaves,
        },
        other => {
            return Err(bad(format!(
                "noise kind `{other}` — expected white, value, perlin or fbm"
            )))
        }
    };
    Ok(binding.with_noise(NoiseConfig {
        kind,
        rate: noise.rate,
        stream: noise.stream,
    }))
}

/// A written param's fold key: its address, then its name. `None` sorts first,
/// which puts the Set-wide value above the narrower ones that override it.
type ParamKey<'a> = (Option<(u8, u32)>, &'a str);

/// `Layer` as a number, so an address can sort. Only the two a Set builds.
fn layer_ordinal(layer: Kind) -> u8 {
    match layer {
        Kind::L1 => 0,
        Kind::L2 => 1,
        Kind::L4 => 2,
    }
}

fn layer_from_ordinal(n: u8) -> Layer {
    match n {
        0 => Layer::L1,
        1 => Layer::L2,
        _ => Layer::L4,
    }
}

/// The engine `Kind` a record `Layer` names, or `None` for one this engine has
/// no node for.
fn kind_of(layer: Layer) -> Option<Kind> {
    match layer {
        Layer::L1 => Some(Kind::L1),
        Layer::L2 => Some(Kind::L2),
        Layer::L4 => Some(Kind::L4),
        _ => None,
    }
}

/// The record a binding is. The inverse of [`binding_from_record`], and what
/// [`save`] writes.
pub fn record_from_binding(binding: &Binding) -> Record {
    Record::Bind {
        layer: match binding.layer {
            Kind::L1 => Layer::L1,
            Kind::L2 => Layer::L2,
            Kind::L4 => Layer::L4,
        },
        index: binding.index,
        key: binding.key.clone(),
        signal: binding.signal.clone(),
        curve: binding.curve.name().to_string(),
        range: binding.range,
        noise: binding.noise.map(|n| BindNoise {
            kind: match n.kind {
                NoiseKind::White => "white".to_string(),
                NoiseKind::Value => "value".to_string(),
                NoiseKind::Perlin => "perlin".to_string(),
                NoiseKind::Fbm { .. } => "fbm".to_string(),
            },
            rate: n.rate,
            stream: n.stream,
            octaves: match n.kind {
                NoiseKind::Fbm { octaves } => octaves,
                _ => DEFAULT_OCTAVES,
            },
        }),
    }
}

fn layer_name(layer: Layer) -> &'static str {
    match layer {
        Layer::L1 => "L1",
        Layer::L2 => "L2",
        Layer::L3 => "L3",
        Layer::L4 => "L4",
    }
}

/// Everything a Set file records, gathered so [`save`] takes one argument for
/// the Set rather than seven for its parts. The fields are the records, in the
/// order they are written.
pub struct Saving<'a> {
    pub l1_path: &'a Path,
    /// The renderers, in draw order. One is the ordinary case.
    pub l4_paths: &'a [PathBuf],
    pub capacity: u32,
    pub params: &'a [ParamWrite],
    pub bindings: &'a [Binding],
    pub camera: &'a Orbit,
    pub seed: u32,
}

/// **Write a Set file, and the artifacts it references.**
///
/// The sources go into the store first and the file references them by hash,
/// so a Set file is a few dozen lines a human can read rather than a copy of
/// the material. Content addressing means saving the same procedure twice
/// stores it once.
pub fn save(store: &Store, id: &str, set: Saving<'_>) -> Result<(), String> {
    let Saving {
        l1_path,
        l4_paths,
        capacity,
        params,
        bindings,
        camera,
        seed,
    } = set;
    let put = |path: &Path| -> Result<Hash, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        store
            .put_artifact(&bytes)
            .map_err(|e| format!("{}: {e}", path.display()))
    };
    let l1_hash = put(l1_path)?;
    let l4_hashes = l4_paths.iter().map(|p| put(p)).collect::<Result<Vec<_>, _>>()?;

    let mut lines = vec![
        Line::new(Record::Set {
            id: id.to_string(),
            v: VERSION,
        }),
        Line::new(Record::Slot {
            layer: Layer::L1,
            index: 0,
            proc_hash: l1_hash,
        }),
        // On L1, because that is the layer whose element buffers it sizes. The
        // format keys capacity by layer and the engine holds one per Set; see
        // the module doc.
        Line::new(Record::Capacity {
            layer: Layer::L1,
            value: capacity,
        }),
    ];
    // **One `slot` record per renderer, in draw order.** Several on L4 is how
    // the format says a stack, and it needed no new record to say it — a file
    // with one reads exactly as it always did.
    for (index, proc_hash) in l4_hashes.into_iter().enumerate() {
        lines.insert(
            lines.len() - 1,
            Line::new(Record::Slot {
                layer: Layer::L4,
                index: index as u32,
                proc_hash,
            }),
        );
    }
    // Sorted, so saving the same state twice produces the same file. A
    // `HashMap`'s order is not a property anything should depend on, and a Set
    // file that differed run to run would make every diff meaningless.
    // Keyed by the address as well as the name, so a wildcard write and a
    // write addressed at one node are two lines rather than one overwriting the
    // other. `None` sorts first, which puts the Set-wide value above the
    // narrower ones that override it — the order a reader wants.
    let ordered: BTreeMap<ParamKey<'_>, f32> = params
        .iter()
        .map(|w| {
            let at = w.at.map(|(layer, i)| (layer_ordinal(layer), i));
            ((at, w.key.as_str()), w.value)
        })
        .collect();
    for ((at, key), value) in ordered {
        lines.push(Line::new(Record::Param {
            // **`layer` is load-bearing exactly when `index` is beside it.** It
            // was a placeholder before the address existed — written as `L1` on
            // everything and ignored on read — so an unaddressed write still
            // says `L1` and still means every node declaring the name. See
            // `Record::Param`.
            layer: at.map_or(Layer::L1, |(l, _)| layer_from_ordinal(l)),
            index: at.map(|(_, i)| i),
            key: key.to_string(),
            value: Value::Scalar(value),
        }));
    }
    for binding in bindings {
        lines.push(Line::new(record_from_binding(binding)));
    }
    lines.push(Line::new(Record::Camera {
        kind: "orbit".to_string(),
        radius: camera.radius,
        speed: camera.speed,
    }));
    lines.push(Line::new(Record::Seed {
        stream: Layer::L1,
        value: u64::from(seed),
    }));

    store
        .write_set(id, &lines)
        .map_err(|e| format!("writing set `{id}`: {e}"))
}

/// **Read a Set file back into what the engine takes.**
///
/// A source is resolved from the inlined `src` records when the file carries
/// them — the bundled form — and from the store otherwise. Bundling wins
/// because a file that carries its own source is meant to be readable on a
/// machine whose store has never seen it.
pub fn load(store: &Store, id: &str) -> Result<Loaded, String> {
    let lines = store
        .read_set(id)
        .map_err(|e| format!("reading set `{id}`: {e}"))?;
    from_lines(store, id, &lines)
}

/// The decode, over lines that are already in hand. Split out so a test can
/// build a file in memory and so a session stream's head can be loaded the same
/// way once anything writes one.
pub fn from_lines(store: &Store, id: &str, lines: &[Line]) -> Result<Loaded, String> {
    let mut notes = Vec::new();
    let mut slots: BTreeMap<&str, Hash> = BTreeMap::new();
    // The renderers, by index. `None` is a gap — an index nothing claimed —
    // which is refused below rather than silently closed up.
    let mut l4_slots: Vec<Option<Hash>> = Vec::new();
    let mut inlined: BTreeMap<Hash, BTreeMap<u32, String>> = BTreeMap::new();
    let mut capacity = None;
    let mut params = Vec::new();
    let mut bindings = Vec::new();
    let mut camera = None;
    let mut seed = None;
    let mut file_id = id.to_string();

    for line in lines {
        match line.record() {
            Record::Set { id, v } => {
                if *v != VERSION {
                    notes.push(format!(
                        "the file says version {v} and this build writes {VERSION}; \
                         reading it anyway"
                    ));
                }
                file_id = id.clone();
            }
            Record::Slot {
                layer,
                index,
                proc_hash,
            } => match layer {
                Layer::L1 => {
                    slots.insert("L1", *proc_hash);
                }
                // **Placed by index, not appended.** A second L4 `slot` record
                // is a second renderer over the same geometry rather than a
                // correction of the first — and the index says which, so the
                // records need not arrive in order and the projection can fold
                // them without one. A file from before stacks existed carries
                // one L4 at index 0 and lands where it always did.
                Layer::L4 => {
                    let at = *index as usize;
                    if l4_slots.len() <= at {
                        l4_slots.resize(at + 1, None);
                    }
                    if l4_slots[at].is_some() {
                        notes.push(format!(
                            "two L4 slots both claim index {at}; the later one is used"
                        ));
                    }
                    l4_slots[at] = Some(*proc_hash);
                }
                other => notes.push(format!(
                    "slot {} was skipped: this engine builds L1 and L4 only",
                    layer_name(*other)
                )),
            },
            Record::Src { hash, line, s } => {
                inlined.entry(*hash).or_default().insert(*line, s.clone());
            }
            Record::Capacity { layer, value } => match layer {
                Layer::L1 => capacity = Some(*value),
                other => notes.push(format!(
                    "capacity on {} was skipped: a Set has one capacity and it is L1's",
                    layer_name(*other)
                )),
            },
            Record::Param {
                layer,
                index,
                key,
                value,
            } => match value {
                // The address is `(layer, index)` present or absent as a unit,
                // so a record with no index is a wildcard whatever its `layer`
                // says — which is what keeps every file written before the
                // address existed meaning what it meant.
                Value::Scalar(v) => params.push(match index {
                    Some(at) => match kind_of(*layer) {
                        Some(kind) => ParamWrite::at(kind, *at, key.clone(), *v),
                        None => {
                            notes.push(format!(
                                "param `{key}` on {} was skipped: this engine builds L1 and L4 only",
                                layer_name(*layer)
                            ));
                            continue;
                        }
                    },
                    None => ParamWrite::everywhere(key.clone(), *v),
                }),
                _ => notes.push(format!(
                    "param `{key}` was skipped: it is a vector and the engine holds \
                     scalar parameter values only"
                )),
            },
            record @ Record::Bind { .. } => match binding_from_record(record) {
                Ok(binding) => bindings.push(binding),
                // Reported and skipped rather than failing the load: one
                // unusable binding is not a reason to refuse the material, and
                // the note says exactly which parameter will not move.
                Err(message) => notes.push(format!("{message} — skipped")),
            },
            Record::Camera { kind, radius, speed } => {
                if kind != "orbit" {
                    notes.push(format!(
                        "camera kind `{kind}` is not one this engine has; using an orbit"
                    ));
                }
                // The record carries two of the six fields an `Orbit` has, so
                // the rest take their defaults. Said out loud because a saved
                // camera and a loaded one are then not the same camera unless
                // the other four were already default.
                camera = Some(Orbit {
                    radius: *radius,
                    speed: *speed,
                    ..Orbit::default()
                });
            }
            Record::Seed { stream, value } => match stream {
                Layer::L1 => seed = Some(*value as u32),
                other => notes.push(format!(
                    "seed on {} was skipped: a Set is salted from one seed and it is L1's",
                    layer_name(*other)
                )),
            },
            // Not a Set file's, and each for its own reason — see
            // `Record::is_set_state`.
            Record::Tick { .. }
            | Record::Audio { .. }
            | Record::Tempo { .. }
            | Record::Gain { .. }
            | Record::Opacity { .. }
            | Record::Blend { .. }
            | Record::Residency { .. }
            | Record::Look { .. }
            | Record::Canvas { .. }
            | Record::Procedure { .. }
            | Record::Transport { .. }
            | Record::Preview { .. }
            | Record::Transition { .. }
            | Record::Mask { .. } => notes.push(
                "a record that belongs to a session rather than to a Set was skipped".to_string(),
            ),
            Record::Unknown => notes.push(
                "a record type this build does not know was skipped, as the format says to"
                    .to_string(),
            ),
        }
    }

    let source_of = |layer: &str, hash: Option<&Hash>| -> Result<String, String> {
        let hash = hash.ok_or_else(|| format!("set `{file_id}` has no {layer} slot"))?;
        if let Some(lines) = inlined.get(hash) {
            // Bundled: the file carries its own source, so it reads on a
            // machine whose store has never seen this artifact.
            return Ok(lines.values().cloned().collect::<Vec<_>>().join("\n"));
        }
        let bytes = store.get_artifact(hash).map_err(|e| {
            format!(
                "set `{file_id}`: {layer} is `{}` and the store does not have it ({e}); \
                 a Set file references its procedures by hash, so the artifact has to be \
                 in the store or inlined in the file",
                hash.short(12)
            )
        })?;
        String::from_utf8(bytes).map_err(|e| format!("set `{file_id}`: {layer} is not UTF-8: {e}"))
    };

    let l1_src = source_of("L1", slots.get("L1"))?;
    if l4_slots.is_empty() {
        return Err(format!("set `{file_id}` has no L4 slot"));
    }
    let l4_srcs = l4_slots
        .iter()
        .enumerate()
        .map(|(at, h)| {
            source_of("L4", h.as_ref()).map_err(|e| match h {
                Some(_) => e,
                // A gap rather than a missing artifact: index 2 without index 1
                // describes a stack with a hole in it, and closing it up would
                // silently change draw order.
                None => format!("set `{file_id}` names an L4 at index {at} but none before it"),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let l1 = crate::compile::check(&l1_src)?;
    let l4s = l4_srcs
        .iter()
        .map(|src| crate::compile::check(src))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Loaded {
        id: file_id,
        l1,
        l4s,
        l1_src,
        l4_srcs,
        capacity,
        params,
        bindings,
        camera,
        seed,
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const L1: &str = r#"
proc ring {
  kind     L1
  topology points
  capacity [1024, 262144] = 4096

  param radius : float [0.1, 8.0] = 2.5
  param spin   : float [0.0, 4.0] = 1.0

  emit position, age

  element {
    let a = t * spin + hash1(seed) * 1.2;
    position = vec3(cos(a) * radius, sin(a) * radius, 0.0);
    age      = t;
  }
}
"#;

    const L4: &str = r#"
proc points {
  kind  L4
  blend additive

  consumes position

  vertex {
    clip       = camera * vec4(position, 1.0);
    point_size = 4.0;
  }

  fragment {
    color = vec4(1.0, 1.0, 1.0, 1.0 - length(point_coord * 2.0 - 1.0));
  }
}
"#;

    /// A store with the two procedures written out beside it, and the paths.
    fn fixture() -> (tempfile::TempDir, Store, std::path::PathBuf, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let l1 = dir.path().join("l1.kir");
        let l4 = dir.path().join("l4.kir");
        std::fs::write(&l1, L1).expect("write l1");
        std::fs::write(&l4, L4).expect("write l4");
        let store = Store::open(dir.path().join("store")).expect("store");
        (dir, store, l1, l4)
    }

    /// A Set with nothing but its material and whatever bindings are given.
    /// `Orbit::default()` is not `const`, so this is the one place a test names
    /// its fields; `LazyLock` keeps that to one place rather than one per call.
    static DEFAULT_CAMERA: std::sync::LazyLock<Orbit> =
        std::sync::LazyLock::new(Orbit::default);

    fn plain<'a>(
        l1: &'a std::path::Path,
        l4: &'a [PathBuf],
        bindings: &'a [Binding],
    ) -> Saving<'a> {
        Saving {
            l1_path: l1,
            l4_paths: l4,
            capacity: 4096,
            params: &[],
            bindings,
            camera: &DEFAULT_CAMERA,
            seed: 1,
        }
    }

    fn a_binding() -> Binding {
        Binding::new(Kind::L1, "spin", "beat", Curve::Pow2, [0.5, 3.0])
    }

    /// **Everything a Set file is for, in one assertion**: what went in comes
    /// back out. A format that carried the material and lost the parameters
    /// would still load, still render, and still be the wrong Set.
    #[test]
    fn a_saved_set_loads_back_as_what_was_saved() {
        let (_dir, store, l1, l4) = fixture();
        let params = vec![ParamWrite::everywhere("radius", 3.25)];
        let camera = Orbit {
            radius: 11.5,
            speed: 0.42,
            ..Orbit::default()
        };
        save(
            &store,
            "s1",
            Saving {
                l1_path: &l1,
                l4_paths: std::slice::from_ref(&l4),
                capacity: 65_536,
                params: &params,
                bindings: &[a_binding()],
                camera: &camera,
                seed: 4242,
            },
        )
        .expect("save");

        let loaded = load(&store, "s1").expect("load");
        assert_eq!(loaded.id, "s1");
        assert_eq!(loaded.capacity, Some(65_536));
        assert_eq!(loaded.params, params);
        assert_eq!(loaded.seed, Some(4242));
        assert_eq!(loaded.camera.map(|c| (c.radius, c.speed)), Some((11.5, 0.42)));
        assert_eq!(loaded.bindings.len(), 1);
        let back = &loaded.bindings[0];
        assert_eq!(back.layer, Kind::L1);
        assert_eq!(back.key, "spin");
        assert_eq!(back.signal, "beat");
        assert_eq!(back.curve, Curve::Pow2);
        assert_eq!(back.range, [0.5, 3.0]);
        // And the procedures themselves came back through the store, compiled.
        assert!(loaded.notes.is_empty(), "unexpected notes: {:?}", loaded.notes);
    }

    /// **The material is resolved by hash out of the store**, which is what
    /// makes a Set file a few dozen lines rather than a copy of the source. A
    /// file whose artifacts are missing says so instead of loading something
    /// else.
    #[test]
    fn a_set_whose_artifacts_are_missing_says_which_and_why() {
        let (_dir, store, l1, l4) = fixture();
        save(&store, "s1", plain(&l1, std::slice::from_ref(&l4), &[])).expect("save");
        let lines = store.read_set("s1").expect("read");

        // A second store that has the file but not the artifacts — a Set file
        // carried to a machine that has never seen the procedures.
        let elsewhere = tempfile::tempdir().expect("tempdir");
        let bare = Store::open(elsewhere.path()).expect("store");
        let err = from_lines(&bare, "s1", &lines).expect_err("the artifacts are not there");
        assert!(err.contains("the store does not have it"), "{err}");
        assert!(err.contains("inlined"), "{err}");
    }

    /// The bundled form: a file carrying its own source reads on a machine
    /// whose store has never seen the artifact. **Inlined source wins over the
    /// store**, so a bundle is self-contained rather than half-resolved.
    #[test]
    fn inlined_source_loads_without_a_store_that_knows_the_artifact() {
        let (_dir, store, l1, l4) = fixture();
        save(&store, "s1", plain(&l1, std::slice::from_ref(&l4), &[])).expect("save");
        let mut lines = store.read_set("s1").expect("read");
        // Bundle it: every slot's source inlined, line by line, as `src`.
        let mut bundled = Vec::new();
        for line in &lines {
            if let Record::Slot { proc_hash, layer, .. } = line.record() {
                let src = match layer {
                    Layer::L1 => L1,
                    _ => L4,
                };
                for (n, text) in src.lines().enumerate() {
                    bundled.push(Line::new(Record::Src {
                        hash: *proc_hash,
                        line: n as u32,
                        s: text.to_string(),
                    }));
                }
            }
        }
        lines.append(&mut bundled);

        let elsewhere = tempfile::tempdir().expect("tempdir");
        let bare = Store::open(elsewhere.path()).expect("store");
        let loaded = from_lines(&bare, "s1", &lines).expect("the file carries its own source");
        assert_eq!(loaded.l1.name, "ring");
        assert_eq!(loaded.l4s[0].name, "points");
    }

    /// **A file written before the address existed still means what it meant.**
    ///
    /// `layer` on a `param` record was a placeholder: the writer put `L1` on
    /// everything and said so in a comment, and the loader ignored it. So
    /// honouring `layer` now would silently retarget every Set file ever
    /// written — an `exposure` that reached the renderer would start reaching
    /// the L1 and doing nothing.
    ///
    /// What stops that is the address being `(layer, index)` present or absent
    /// **as a unit**: no `index`, no address, whatever `layer` says. This reads
    /// a hand-written old-style file to prove it, rather than one this build
    /// produced — a round trip through the new writer would agree with itself
    /// however wrong both halves were.
    #[test]
    fn a_param_record_without_an_index_is_a_wildcard_whatever_its_layer_says() {
        let (_dir, store, l1, l4) = fixture();
        let hashes: Vec<String> = [&l1, &l4]
            .iter()
            .map(|p| {
                let bytes = std::fs::read(p).expect("read");
                store.put_artifact(&bytes).expect("put").to_string()
            })
            .collect();
        let text = format!(
            r#"{{"t":"set","id":"old","v":1}}
{{"t":"slot","layer":"L1","proc":"{}"}}
{{"t":"slot","layer":"L4","proc":"{}"}}
{{"t":"param","layer":"L1","key":"exposure","value":0.4}}
{{"t":"param","layer":"L4","index":1,"key":"exposure","value":0.9}}
"#,
            hashes[0], hashes[1]
        );
        // Through the file reader, so the bytes above are genuinely parsed
        // rather than hand-built into records that could not have been written.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("old.set.ndjson");
        std::fs::write(&path, text).expect("write");
        let lines = karakuri_store::ndjson::read(&path).expect("an old-style file still parses");

        let loaded = from_lines(&store, "old", &lines).expect("an old-style file still loads");
        assert_eq!(
            loaded.params,
            vec![
                // No index: a wildcard, even though the record says `L1`.
                ParamWrite::everywhere("exposure", 0.4),
                // An index: an address, and `layer` is load-bearing beside it.
                ParamWrite::at(Kind::L4, 1, "exposure", 0.9),
            ]
        );
    }

    /// **What could not be carried is said, not dropped.** Three shapes, and
    /// each is a real disagreement between a format keyed by layer and an
    /// engine that holds one value per Set.
    #[test]
    fn what_the_engine_cannot_carry_is_reported_rather_than_dropped() {
        let (_dir, store, l1, l4) = fixture();
        save(&store, "s1", plain(&l1, std::slice::from_ref(&l4), &[])).expect("save");
        let mut lines = store.read_set("s1").expect("read");
        lines.push(Line::new(Record::Seed {
            stream: Layer::L4,
            value: 7,
        }));
        lines.push(Line::new(Record::Capacity {
            layer: Layer::L4,
            value: 128,
        }));
        lines.push(Line::new(Record::Param {
            layer: Layer::L1,
            index: None,
            key: "tint".to_string(),
            value: Value::Vec3([1.0, 0.0, 0.0]),
        }));

        let loaded = from_lines(&store, "s1", &lines).expect("load");
        let notes = loaded.notes.join("\n");
        assert!(notes.contains("seed on L4"), "{notes}");
        assert!(notes.contains("capacity on L4"), "{notes}");
        assert!(notes.contains("`tint`"), "{notes}");
        // The L1 values are still the ones applied: a note is not a refusal.
        assert_eq!(loaded.seed, Some(1));
        assert_eq!(loaded.capacity, Some(4096));
    }

    /// **A binding the engine cannot honour is reported and skipped**, and the
    /// load still succeeds. One unusable binding is not a reason to refuse the
    /// material, and the note names the parameter that will not move.
    #[test]
    fn an_unusable_binding_is_named_and_the_rest_of_the_set_still_loads() {
        let (_dir, store, l1, l4) = fixture();
        save(&store, "s1", plain(&l1, std::slice::from_ref(&l4), &[a_binding()])).expect("save");
        let mut lines = store.read_set("s1").expect("read");
        lines.push(Line::new(Record::Bind {
            layer: Layer::L1,
            index: None,
            key: "radius".to_string(),
            signal: "bpm".to_string(),
            curve: "lin".to_string(),
            range: [0.0, 1.0],
            noise: None,
        }));

        let loaded = from_lines(&store, "s1", &lines).expect("load");
        assert_eq!(loaded.bindings.len(), 1, "the good binding survived");
        let notes = loaded.notes.join("\n");
        assert!(notes.contains("bpm"), "{notes}");
        assert!(notes.contains("skipped"), "{notes}");
    }

    /// The two diagnostics `docs/roadmap.md` says the decoder owes, asserted
    /// against the decoder rather than against the flag that used to hold them.
    #[test]
    fn the_decoder_carries_the_diagnostics_the_flag_used_to_hold_alone() {
        let bpm = Record::Bind {
            layer: Layer::L1,
            index: None,
            key: "radius".to_string(),
            signal: "bpm".to_string(),
            curve: "lin".to_string(),
            range: [0.0, 1.0],
            noise: None,
        };
        let err = binding_from_record(&bpm).expect_err("a tempo is not a [0,1] signal");
        assert!(err.contains("bar"), "{err}");

        let octaves = Record::Bind {
            layer: Layer::L1,
            index: None,
            key: "radius".to_string(),
            signal: NOISE_SIGNAL.to_string(),
            curve: "lin".to_string(),
            range: [0.0, 1.0],
            noise: Some(BindNoise {
                kind: "white".to_string(),
                octaves: 6,
                ..BindNoise::default()
            }),
        };
        let err = binding_from_record(&octaves).expect_err("white has no octaves");
        assert!(err.contains("fbm"), "{err}");
    }

    /// A `bind` record and a `Binding` are the same thing in two shapes, and
    /// `save` writes one from the other. **Every generator kind survives**, so
    /// a saved `fbm` does not come back as the perlin the default would give.
    #[test]
    fn every_noise_generator_survives_the_record_it_is_written_as() {
        for kind in [
            NoiseKind::White,
            NoiseKind::Value,
            NoiseKind::Perlin,
            NoiseKind::Fbm { octaves: 6 },
        ] {
            let binding = Binding::new(Kind::L1, "radius", NOISE_SIGNAL, Curve::Lin, [0.0, 1.0])
                .with_noise(NoiseConfig {
                    kind,
                    rate: 2.5,
                    stream: 3,
                });
            let back = binding_from_record(&record_from_binding(&binding))
                .unwrap_or_else(|e| panic!("{kind:?}: {e}"));
            assert_eq!(back.noise.map(|n| n.kind), Some(kind), "{kind:?}");
            assert_eq!(back.noise.map(|n| (n.rate, n.stream)), Some((2.5, 3)));
        }
    }
}
