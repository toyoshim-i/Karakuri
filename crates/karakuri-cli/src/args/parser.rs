use super::*;

/// The real entry point: reads the process's own arguments, then hands them to
/// [`parse_args_from`] and turns its `Result` into the exit this binary
/// actually makes. Kept this thin so the parsing logic itself takes any
/// iterator of strings and returns rather than exits — which is what makes it
/// possible to drive adversarial input through it in a test.
pub(crate) fn parse_args() -> Args {
    match parse_args_from(std::env::args().skip(1)) {
        Ok(ParseOutcome::Run(args)) => *args,
        Ok(ParseOutcome::Help) => {
            print!("{USAGE}\n{BINDINGS}");
            std::process::exit(0);
        }
        // **Nothing is built to answer this.** The store is opened, the `sets/`
        // directory is read, each file in it is read, and the process ends —
        // see [`listed_sets`]. On stdout, because it is the answer to the
        // question that was asked rather than a note about a run.
        Ok(ParseOutcome::ListSets(root)) => match listed_sets_at(&root) {
            Ok(said) => {
                print!("{said}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("karakuri-cli: {e}");
                std::process::exit(1);
            }
        },
        // **To standard output, so a shell can redirect it.** What packaging
        // produces is a thing you *send somebody* — a single self-contained
        // file that works in their store — rather than something the library
        // keeps, so it goes where `> patch.ndjson` puts it and needs no naming
        // rule of its own in `sets/`. The store already holds this Set under an
        // id; a second copy of it there under some derived name would be a
        // second answer to which file is the Set.
        Ok(ParseOutcome::Package { store, id }) => match packaged_set(&store, &id) {
            Ok(said) => {
                print!("{said}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("karakuri-cli: {e}");
                std::process::exit(1);
            }
        },
        // On stdout for the reason a listing is: it is the answer to the
        // question that was asked. Nothing is compiled to *run* — the checker
        // is reached only to write each artifact's metadata card.
        Ok(ParseOutcome::TakeIn { store, file }) => match taken_in_file(&store, &file) {
            Ok(said) => {
                print!("{said}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("karakuri-cli: {e}");
                std::process::exit(1);
            }
        },
        Err(message) => fail(&message),
    }
}

/// The whole of argument parsing, as a pure function: no I/O, no exit, just
/// strings in and either an [`Args`] or an error message out. Every message a
/// person can act on is produced here rather than by falling back to a default
/// that hides a typo — `--set` and `--tonemap` already worked this way;
/// `--exposure` now validates on the same terms rather than silently keeping
/// 1.0 for a negative, zero, or unparsable value. The value belonging to
/// `flag`, refused rather than defaulted.
///
/// Two silences this removes. A flag at the end of the line with nothing after
/// it used to take `None` and fall back — `--render` with the path forgotten
/// opened a *window*, so a batch script with a typo hung waiting for one
/// instead of failing. And a flag whose value was missing used to swallow the
/// next flag: `--set --render out.png` read `--render` as a pair of paths and
/// then blamed `--render` for not being one.
///
/// A leading `-` is treated as another option unless the whole token parses as
/// a number, so a negative value is still a value.
pub(crate) fn value_for(
    flag: &str,
    it: &mut impl Iterator<Item = String>,
) -> Result<String, String> {
    match it.next() {
        Some(v) if !v.starts_with('-') || v.parse::<f64>().is_ok() => Ok(v),
        Some(v) => Err(format!(
            "`{flag}` was given no value — `{v}` is an option, not one"
        )),
        None => Err(format!("`{flag}` needs a value")),
    }
}

/// A number for `flag`, refused rather than defaulted.
///
/// `--frames`, `--capacity` and `--budget-ms` used to keep their default on
/// anything unparsable, so `--frames 24O` (a letter O) rendered 240 frames and
/// said nothing. A run that quietly used the default is indistinguishable from
/// one that honoured what was typed, which is the same failure this codebase
/// keeps finding in other clothes.
pub(crate) fn number_for<T: std::str::FromStr>(
    flag: &str,
    what: &str,
    it: &mut impl Iterator<Item = String>,
) -> Result<T, String> {
    let value = value_for(flag, it)?;
    value
        .parse()
        .map_err(|_| format!("`{flag} {value}` — expected {what}"))
}

/// A `WIDTHxHEIGHT` pair for `flag`.
///
/// Zero is refused rather than clamped, which is the difference between a typo
/// and a picture: everything downstream takes `max(1)` to keep a texture
/// descriptor legal, so `--canvas 1920x0` would have rendered a one-texel-tall
/// frame and reported the size it was asked for.
pub(crate) fn extent(flag: &str, value: String) -> Result<(u32, u32), String> {
    let bad = || format!("`{flag} {value}` — expected `WIDTHxHEIGHT`");
    let (w, h) = value.split_once('x').ok_or_else(bad)?;
    match (w.parse::<u32>(), h.parse::<u32>()) {
        (Ok(w), Ok(h)) if w > 0 && h > 0 => Ok((w, h)),
        (Ok(_), Ok(_)) => Err(format!("`{flag} {value}` — neither side may be zero")),
        _ => Err(bad()),
    }
}

/// One `--bind` value, as the `bind` record's own fields.
///
/// The grammar is `field=value`, comma separated, and every field name is the
/// record's. The single deviation is `range`, which the record writes as a
/// two-element array and this writes as `LOW..HIGH` — a comma inside a value
/// would be indistinguishable from the separator between fields, and quoting
/// rules to fix that would be a second grammar rather than a smaller one.
///
/// Unknown fields are refused rather than ignored. The record format ignores an
/// unknown `t` for forward compatibility between engine versions; a typo on a
/// command line has no such excuse, and `curv=pow2` silently taking the default
/// curve is the exact silence every other flag here was fixed for. `--edge
/// <node>.<slot>=<node>` — bind one procedure's declared input slot to a node
/// of the Set, whatever type the slot was declared with.
///
/// `.` between the node and the slot, `=` before the node it is bound to. The
/// `=` is `--set`'s already and means "the thing on the left is a name for the
/// thing on the right"; the `.` is the same dot the procedure reads the slot
/// through, so `--edge morph.far=sphere_shell` and `far.position` in the
/// `deform` are visibly one spelling. A Field slot is read as a call rather
/// than through a dot — `--edge field_lens.shape=melt_blob`, then `shape(p)` —
/// and still writes the same record, because what an edge says is the same fact
/// whatever fills the slot. `:` was not available — `--param` and `--publish`
/// use it for a layer and an index, and it is a path character on Windows.
///
/// `--bind` was taken, by signals, which is the other reason the word here is
/// `edge`: it is what the record has always been going to be called, since what
/// it writes down is one edge of the graph a Set describes.
///
/// Every part is refused empty rather than accepted and resolved to nothing: an
/// edge with no slot in it is a sentence about a node, and there is no such
/// sentence.
pub(crate) fn parse_edge(value: &str) -> Result<karakuri_engine::set::Edge, String> {
    let bad = |what: &str| format!("`--edge {value}` — {what}");
    let Some((from, to)) = value.split_once('=') else {
        return Err(bad(
            "expected `<node>.<slot>=<node>`, e.g. `morph.far=sphere_shell` or \
             `field_lens.shape=melt_blob`",
        ));
    };
    // **The last dot, not the first.** A node name may hold one — nothing
    // refuses `--set my.morph=morph.kir` — and the slot is a `.kir` identifier,
    // which cannot.
    let Some((node, slot)) = from.rsplit_once('.') else {
        return Err(bad(
            "expected a `.` between the node and the slot it declares, e.g. \
             `morph.far=sphere_shell`",
        ));
    };
    if node.is_empty() || slot.is_empty() || to.is_empty() {
        return Err(bad("every part names something: `<node>.<slot>=<node>`"));
    }
    Ok(karakuri_engine::set::Edge {
        node: node.to_string(),
        slot: slot.into(),
        to: to.to_string(),
    })
}

pub(crate) fn parse_bind(value: &str) -> Result<Binding, String> {
    let bad = |what: &str| format!("`--bind {value}` — {what}");

    let mut layer = None;
    // **Absent is a wildcard, not zero.** A binding with no `index` is the
    // layer's — every node declaring the key — which is what `--bind` has
    // always meant and what one published control would drive. See
    // `Binding::index`.
    let mut index: Option<u32> = None;
    let mut key = None;
    let mut signal = None;
    let mut curve = Curve::Lin;
    let mut range = None;
    // Built whether or not it is used: a `noise.*` field on a binding whose
    // signal is not `noise` is a mistake worth reporting, and that check needs
    // to know one was given.
    let mut noise = NoiseConfig::default();
    let mut noise_given = false;
    // Kept as what was written rather than folded into `noise.kind` as it is
    // read. `NoiseKind` carries the octave count inside the `fbm` variant, so
    // assigning either field as it arrives lets the later one decide the
    // other: `noise.octaves` would turn a `white` that was asked for into an
    // `fbm` that was not. Both are resolved once, after the loop, where the
    // pair can be checked against each other.
    let mut noise_kind: Option<&str> = None;
    let mut noise_octaves: Option<u32> = None;

    for field in value.split(',') {
        let (name, v) = field
            .split_once('=')
            .ok_or_else(|| bad(&format!("`{field}` is not `field=value`")))?;
        let number = |what: &str| -> Result<f32, String> {
            v.parse::<f32>()
                .map_err(|_| bad(&format!("`{name}` expects {what}, got `{v}`")))
        };
        match name.trim() {
            "layer" => {
                layer = Some(layer_named(v).ok_or_else(|| {
                    bad(&format!("`layer={v}` — expected L1, L2, L3, L4 or Field"))
                })?)
            }
            "index" => {
                index = Some(v.parse::<u32>().map_err(|_| {
                    bad(&format!(
                        "`index={v}` — expected a node number, 0 for the first"
                    ))
                })?)
            }
            "key" => key = Some(v.to_string()),
            "signal" => signal = Some(v.to_string()),
            "curve" => {
                curve = Curve::parse(v).ok_or_else(|| {
                    let names: Vec<&str> = CURVES.iter().map(|c| c.name()).collect();
                    bad(&format!("`curve={v}` — expected {}", names.join(", ")))
                })?
            }
            "range" => {
                let (low, high) = v
                    .split_once("..")
                    .ok_or_else(|| bad(&format!("`range={v}` — expected `LOW..HIGH`")))?;
                match (low.parse::<f32>(), high.parse::<f32>()) {
                    (Ok(low), Ok(high)) => range = Some([low, high]),
                    _ => return Err(bad(&format!("`range={v}` — expected `LOW..HIGH`"))),
                }
            }
            "noise.kind" => {
                noise_given = true;
                if !NOISE_KINDS.contains(&v) {
                    return Err(bad(&format!(
                        "`noise.kind={v}` — expected white, value, perlin or fbm"
                    )));
                }
                noise_kind = Some(v);
            }
            "noise.rate" => {
                noise_given = true;
                noise.rate = number("a number of cycles per beat")?;
            }
            "noise.stream" => {
                noise_given = true;
                noise.stream = v
                    .parse::<u64>()
                    .map_err(|_| bad(&format!("`noise.stream={v}` — expected a whole number")))?;
            }
            "noise.octaves" => {
                noise_given = true;
                noise_octaves =
                    Some(v.parse::<u32>().map_err(|_| {
                        bad(&format!("`noise.octaves={v}` — expected a whole number"))
                    })?);
            }
            other => {
                return Err(bad(&format!(
                    "unknown field `{other}` — expected layer, key, signal, curve, range, \
                     or noise.kind / noise.rate / noise.stream / noise.octaves"
                )))
            }
        }
    }

    let layer = layer.ok_or_else(|| bad("no `layer=`"))?;
    let key = key.ok_or_else(|| bad("no `key=`"))?;
    let signal = signal.ok_or_else(|| bad("no `signal=`"))?;
    // Required, unlike `curve`: there is no defensible default range. A param
    // declares its own in the `.kir`, and silently binding across all of it
    // would be an aesthetic decision made by the argument parser.
    let range = range.ok_or_else(|| bad("no `range=LOW..HIGH`"))?;

    // **Built as the record and decoded back**, so the flag is what its
    // documentation always claimed: a way to write a `bind` record. Every
    // semantic rule — the `bpm` refusal, `octaves` needing `fbm`, a generator
    // needing `signal=noise` — lives in `setfile::binding_from_record` and
    // cannot differ between a command line and a Set file. That debt against
    // the decoder is paid by having one rule rather than two copies of it —
    // see `docs/adr/0066-a-flag-becomes-a-record-writer.md`.
    //
    // The one check that stays here is the one the record cannot express.
    // `BindNoise::octaves` has a serde default, deliberately — "a generator
    // omitted field by field is under-specified, not refused" — so a record
    // cannot say whether `octaves` was *named*. The flag knows, and an
    // `octaves` named beside a kind that has no octaves is an operator
    // expecting a generator they did not ask for.
    if noise_octaves.is_some() && noise_kind != Some("fbm") {
        return Err(bad(&format!(
            "`noise.octaves` needs `noise.kind=fbm`, and this asks for `{}`",
            noise_kind.unwrap_or("perlin")
        )));
    }

    let record = Record::Bind {
        layer: record_layer(layer),
        index,
        key,
        signal,
        curve: curve.name().to_string(),
        range,
        noise: noise_given.then(|| BindNoise {
            kind: noise_kind.unwrap_or("perlin").to_string(),
            rate: noise.rate,
            stream: noise.stream,
            octaves: noise_octaves.unwrap_or(setfile::DEFAULT_OCTAVES),
        }),
    };
    setfile::binding_from_record(&record).map_err(|e| bad(&e))
}

/// The `noise.kind` names, in the order the spec lists them. One list, so the
/// check and the message cannot drift apart.
pub(crate) const NOISE_KINDS: [&str; 4] = ["white", "value", "perlin", "fbm"];

/// `--param [L4:N:]name=value`.
///
/// The address is optional and is `layer:index:` when it is there. A bare name
/// is a wildcard — every node declaring it, "the Set's `exposure`", one knob
/// moving both renderers — which is what this flag has always meant and is the
/// useful default. The prefix is what sets two renderers apart, and it is
/// present or absent as a unit for the reason `ParamWrite::at` gives: a layer
/// alone stopped naming a node when a Set gained a list of them, so a
/// half-address would be a wish rather than an address.
///
/// `:` cannot occur in a param name — identifiers are alphanumerics and
/// underscores — so splitting on it is unambiguous and needs no quoting. A
/// malformed override is refused rather than dropped, on the same terms every
/// other flag here is: silence looks exactly like a parameter that was applied
/// and had no visible effect. A name that no procedure declares is still only a
/// warning at build time — that one is a question about the `.kir`. A layer
/// name as an operator writes it. One reader, so `--param` and `--bind` cannot
/// disagree about which layers a Set has — they did, and the disagreement was
/// silent: `--param L2:…` was refused as a malformed address while `--bind
/// layer=L2` was refused with a sentence saying L2 did not exist yet, both long
/// after it did. A layer as the record vocabulary spells it.
///
/// Exhaustive on purpose. A sixth `Kind` stops compiling here rather than
/// falling to a default, which is what the two places that used to map this by
/// hand could not promise.
pub(crate) fn record_layer(kind: karakuri_ir::Kind) -> Layer {
    karakuri_environment::meta::layer_of(kind)
}

/// `--publish name=L4:0:exposure[0.2..0.8]`, or `--publish
/// level=exposure[0..2]` for every node that declares it.
///
/// The address is the `--param` one and the range is the `--bind` one, which is
/// why neither half needed a grammar of its own: what a published control is,
/// is a name in front of an address and a range behind it — and the address is
/// `layer:index:` present or absent as a unit, meaning the same thing it means
/// there. The range is mandatory: publishing without one would mean "over the
/// declared range", and spelling that as an absence would make the common
/// narrowing case look like the exception.
pub(crate) fn parse_publish(value: &str) -> Result<karakuri_engine::set::Published, String> {
    let bad = || {
        format!(
            "`--publish {value}` — expected `name=key[LOW..HIGH]` or `name=L4:0:key[LOW..HIGH]`"
        )
    };
    let (name, rest) = value.split_once('=').ok_or_else(bad)?;
    if name.is_empty() {
        return Err(bad());
    }
    let (at, rest) = match rest.split_once(':') {
        Some((layer, tail)) => {
            let layer = layer_named(layer).ok_or_else(bad)?;
            let (index, tail) = tail.split_once(':').ok_or_else(bad)?;
            let index: u32 = index.parse().map_err(|_| bad())?;
            (Some((layer, index)), tail)
        }
        None => (None, rest),
    };
    let (key, range) = rest.split_once('[').ok_or_else(bad)?;
    let range = range.strip_suffix(']').ok_or_else(bad)?;
    let (low, high) = range.split_once("..").ok_or_else(bad)?;
    let low: f32 = low.parse().map_err(|_| bad())?;
    let high: f32 = high.parse().map_err(|_| bad())?;
    if key.is_empty() {
        return Err(bad());
    }
    Ok(karakuri_engine::set::Published {
        name: name.to_string(),
        at,
        key: key.to_string(),
        range: [low, high],
    })
}

pub(crate) fn parse_param(value: &str) -> Result<ParamWrite, String> {
    let bad = || format!("`--param {value}` — expected `name=number` or `L4:1:name=number`");
    let (addressed, rest) = match value.split_once(':') {
        Some((layer, rest)) => {
            let kind = layer_named(layer).ok_or_else(bad)?;
            // **The index is required, even where a layer can hold only one
            // node.** `L3:0:radius` for a camera a Set has exactly one of is
            // two characters of ceremony, and the rule it keeps is worth more:
            // the address is `layer:index:` present or absent as a *unit*, so
            // there is exactly one wildcard spelling — a bare name. Making the
            // index optional would give `L4:exposure` a third meaning, sitting
            // between "every renderer" and "renderer 0".
            let (index, rest) = rest.split_once(':').ok_or_else(bad)?;
            let index: u32 = index.parse().map_err(|_| bad())?;
            (Some((kind, index)), rest)
        }
        None => (None, value),
    };
    let (key, number) = rest.split_once('=').ok_or_else(bad)?;
    let number: f32 = number.parse().map_err(|_| bad())?;
    if key.is_empty() {
        return Err(bad());
    }
    Ok(ParamWrite {
        at: addressed,
        key: key.to_string(),
        value: number,
    })
}

pub(crate) fn parse_args_from(args: impl Iterator<Item = String>) -> Result<ParseOutcome, String> {
    let mut args_out = Args {
        overrides: Vec::new(),
        bindings: Vec::new(),
        edges: Vec::new(),
        bpm: DEFAULT_BPM,
        sets: Vec::new(),
        capacity: karakuri_ir::DEFAULT_CAPACITY,
        capacity_given: false,
        render_to: None,
        seq_to: None,
        frames: 240,
        size: (1280, 720),
        canvas: (1920, 1080),
        canvas_given: false,
        watch: false,
        merge: Vec::new(),
        published: Vec::new(),
        mcp: None,
        tempo_source: None,
        audio_in: None,
        midi_in: None,
        midi_map: None,
        latency_offset_ms: audio::DEFAULT_LATENCY_OFFSET_MS,
        budget_ms: DEFAULT_BUDGET_MS,
        demo: None,
        store: PathBuf::from(places::STORE),
        save_set: None,
        load_set: None,
        record_session: None,
        replay: None,
        from_set: None,
        look: Look {
            op: TonemapOp::Aces,
            exposure: 1.0,
            white_point: 1.0,
        },
    };
    // Whether this was *typed*, not what it holds: it has a default, so "is it
    // still 1280x720" cannot tell a flag that was given from one that was not,
    // and the refusal below is about the giving. `--canvas` needs the same fact
    // for longer and carries it on `Args` instead.
    let mut size_given = false;
    let mut list_sets = false;
    let mut package: Option<String> = None;
    let mut take_in: Option<PathBuf> = None;
    let mut positional = Vec::new();
    let mut it = args;
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(ParseOutcome::Help),
            "--set" => {
                let value = value_for("--set", &mut it)?;
                // **The first is the L1 and the rest are renderers**, drawn in
                // the order given. A third part used to be refused: it could
                // only be a typo when a Set was a pair, and taking `b,c` as one
                // literal filename would have blamed a missing file for a stray
                // comma. It is now what asking for two renderers looks like, and
                // there is no new syntax for it — one comma-separated list, read
                // as one L1 and however many L4s.
                let parts: Vec<&str> = value.split(',').collect();
                match parts.split_first() {
                    Some((l1, l4s))
                        if !l1.is_empty()
                            && !l4s.is_empty()
                            && l4s.iter().all(|p| !p.is_empty()) =>
                    {
                        // **Each part may carry a name** — `near=lattice.kir`.
                        // A name is what everything downstream addresses the
                        // node by; see `Named`.
                        let head = Named::parse(l1)?;
                        let rest: Vec<Named> = l4s
                            .iter()
                            .map(|p| Named::parse(p))
                            .collect::<Result<_, _>>()?;
                        args_out.sets.push((head, rest))
                    }
                    _ => {
                        return Err(format!(
                            "`--set {value}` — expected `L1.kir,L4.kir`, or \
                             `L1.kir,L4.kir,L4.kir` for several renderers over one geometry. \
                             Any part may be written `name=file.kir`"
                        ))
                    }
                }
            }
            "--render" => args_out.render_to = Some(PathBuf::from(value_for("--render", &mut it)?)),
            "--seq" => args_out.seq_to = Some(PathBuf::from(value_for("--seq", &mut it)?)),
            "--param" => {
                // A malformed override used to be dropped in silence, which
                // looks exactly like a parameter that was applied and had no
                // visible effect. A name that no procedure declares is still
                // only a warning at build time — that one is a question about
                // the `.kir`, not about the command line.
                let value = value_for("--param", &mut it)?;
                args_out.overrides.push(parse_param(&value)?);
            }
            "--bind" => {
                let value = value_for("--bind", &mut it)?;
                args_out.bindings.push(parse_bind(&value)?);
            }
            "--edge" => {
                let value = value_for("--edge", &mut it)?;
                args_out.edges.push(parse_edge(&value)?);
            }
            "--bpm" => {
                let value = value_for("--bpm", &mut it)?;
                // Refused rather than clamped here, on the same terms as
                // `--exposure`: the oscillator clamps a bad tempo so that a
                // zero cannot freeze every noise signal at once, but a tempo
                // typed at the command line and quietly changed is a run that
                // did not do what it was told.
                match value.parse::<f32>() {
                    Ok(v) if v.is_finite() && v > 0.0 => args_out.bpm = v,
                    _ => return Err(format!("`--bpm {value}` — expected a positive number")),
                }
            }
            "--tonemap" => {
                let value = value_for("--tonemap", &mut it)?;
                args_out.look.op = parse_op(&value)
                    .ok_or_else(|| format!("`--tonemap {value}` — expected {}", op_wire_names()))?;
            }
            "--exposure" => {
                let value = value_for("--exposure", &mut it)?;
                // Validated rather than defaulted on failure — a negative,
                // zero, NaN, infinite, or unparsable exposure used to be
                // silently swapped for 1.0, which reads as "the flag was
                // ignored" rather than "the value was wrong". `--tonemap` and
                // `--set` already fail loudly on a bad value; this does the
                // same. **What it does not do is clamp**: `100` is accepted and
                // lands at 100, well outside what `-`/`=` reach, because a
                // batch render is allowed to ask for something extreme. See
                // `exposure_positive_is_accepted_unclamped` and
                // [`EXPOSURE_MIN`], whose comment claimed the opposite.
                match value.parse::<f32>() {
                    Ok(v) if v.is_finite() && v > 0.0 => args_out.look.exposure = v,
                    _ => return Err(format!("`--exposure {value}` — expected a positive number")),
                }
            }
            "--mcp" => args_out.mcp = Some(number_for("--mcp", "a port number", &mut it)?),
            "--tempo-source" => {
                args_out.tempo_source = Some(value_for("--tempo-source", &mut it)?);
            }
            "--audio-in" => {
                args_out.audio_in = Some(value_for("--audio-in", &mut it)?);
            }
            "--midi-in" => {
                args_out.midi_in = Some(value_for("--midi-in", &mut it)?);
            }
            "--midi-map" => {
                args_out.midi_map = Some(PathBuf::from(value_for("--midi-map", &mut it)?));
            }
            "--latency-offset-ms" => {
                let value = value_for("--latency-offset-ms", &mut it)?;
                // Refused rather than clamped, on the same terms as
                // `--exposure`: an offset silently changed is an offset the
                // operator will spend the first song chasing.
                match value.parse::<f32>() {
                    Ok(v) if v.is_finite() && audio::LATENCY_OFFSET_RANGE.contains(&v) => {
                        args_out.latency_offset_ms = v
                    }
                    _ => {
                        return Err(format!(
                            "`--latency-offset-ms {value}` — expected {} to {} milliseconds",
                            audio::LATENCY_OFFSET_RANGE.start(),
                            audio::LATENCY_OFFSET_RANGE.end()
                        ))
                    }
                }
            }
            "--watch" => args_out.watch = true,
            "--publish" => {
                let v = value_for("--publish", &mut it)?;
                args_out.published.push(parse_publish(&v)?);
            }
            "--merge" => {
                let v = value_for("--merge", &mut it)?;
                let slot: usize = v.parse().map_err(|_| {
                    format!("`--merge {v}` — expected a slot number, 0 for the first")
                })?;
                if !args_out.merge.contains(&slot) {
                    args_out.merge.push(slot);
                }
            }
            "--demo" => {
                let name = value_for("--demo", &mut it)?;
                args_out.demo = Some(Demo::from_name(&name).ok_or_else(|| {
                    format!("no demonstration named `{name}` — `transport` or `lines`")
                })?);
            }
            "--store" => args_out.store = PathBuf::from(value_for("--store", &mut it)?),
            // Read into a local rather than onto `Args`: a listing is not a
            // run, and the answer is returned below once the whole command line
            // has been seen — `--list-sets --store DIR` and `--store DIR
            // --list-sets` are the same request.
            "--list-sets" => list_sets = true,
            // Locals rather than `Args` fields, for `--list-sets`'s reason:
            // neither is a run, and both are answered below once the whole
            // command line has been seen, so `--store` may be on either side.
            "--package" => package = Some(value_for("--package", &mut it)?),
            "--take-in" => take_in = Some(PathBuf::from(value_for("--take-in", &mut it)?)),
            "--save-set" => args_out.save_set = Some(value_for("--save-set", &mut it)?),
            "--load-set" => args_out.load_set = Some(value_for("--load-set", &mut it)?),
            "--record-session" => {
                args_out.record_session = Some(value_for("--record-session", &mut it)?)
            }
            "--replay" => args_out.replay = Some(value_for("--replay", &mut it)?),
            "--budget-ms" => {
                args_out.budget_ms = number_for("--budget-ms", "a number of milliseconds", &mut it)?
            }
            "--frames" => args_out.frames = number_for("--frames", "a frame count", &mut it)?,
            "--capacity" => {
                args_out.capacity = number_for("--capacity", "an element count", &mut it)?;
                args_out.capacity_given = true;
            }
            "--size" => {
                args_out.size = extent("--size", value_for("--size", &mut it)?)?;
                size_given = true;
            }
            "--canvas" => {
                args_out.canvas = extent("--canvas", value_for("--canvas", &mut it)?)?;
                args_out.canvas_given = true;
            }
            // An unknown option used to become a path, so `--wtach` looked
            // like a `.kir` that did not exist and the error blamed the file.
            other if other.starts_with('-') => return Err(format!("unknown option `{other}`")),
            _ => positional.push(PathBuf::from(arg)),
        }
    }

    // **Answered before a word is said about material**, and above every rule
    // below: none of them is about a listing. A `.kir` pair this run will not
    // read, a default pair nobody asked for, a refusal about two ways of naming
    // material — all of it belongs to a run, and refusing `--list-sets
    // --load-set x` for "two descriptions of the material" would be a refusal
    // about a Set nothing here is going to build.
    if list_sets {
        return Ok(ParseOutcome::ListSets(args_out.store));
    }
    // The same, and above the material rules for the same reason. Answered in
    // a fixed order rather than refused as a pair: each of the three prints and
    // stops, so the only thing a second one could change is which answer is
    // printed, and none of them is a run whatever the other says.
    if let Some(id) = package {
        return Ok(ParseOutcome::Package {
            store: args_out.store,
            id,
        });
    }
    if let Some(file) = take_in {
        return Ok(ParseOutcome::TakeIn {
            store: args_out.store,
            file,
        });
    }

    // The bare positional pair still means what it always meant, and now it is
    // simply the last slot: `--set a,b c.kir d.kir` is a deck of two.
    match positional.len() {
        0 => {}
        2 => args_out.sets.push((
            Named::bare(positional[0].clone()),
            vec![Named::bare(positional[1].clone())],
        )),
        n => {
            return Err(format!(
                "{n} file argument(s) — a Set is an L1 and an L4, so give two, or use --set"
            ))
        }
    }
    // The default pair, for a run that named no material at all. **Not when a
    // Set file is loaded**: that file *is* the material, and adding the default
    // beside it would put a second Set on the deck nobody asked for — which is
    // not merely extra, it is a `--bind` from the file landing on material that
    // has no such parameter and saying so.
    validate_args(&mut args_out, size_given)?;
    Ok(ParseOutcome::Run(Box::new(args_out)))
}
