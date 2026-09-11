//! Loading a `.kir` pair and putting it through the validation pipeline.
//!
//! Stages 1 through 4 live in `karakuri-ir` and stage 5 in `karakuri-codegen`;
//! all this does is read files, run them, and render any diagnostic against the
//! source it came from. Failure at any stage means no artifact, so there is no
//! partial success to report — the whole point of one severity.

use std::path::{Path, PathBuf};

use karakuri_ir::typed::Checked;

use crate::meta::put_meta;

/// A name for every node of a slot, in the shape the procedures themselves are
/// passed in.
///
/// **Per layer rather than one list in node order**, because the node order is
/// the engine's — `slot_of` and `nodes_of` decide it — and a caller that
/// reproduced it here would be a second place for a fact this project has
/// already been bitten by twice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Names {
    pub l1s: Vec<Option<String>>,
    pub l2s: Vec<Option<String>>,
    /// **A list, like the renderers'.** A slot holds as many cameras as its
    /// files declare, and the built-in orbit is a node beside them — one
    /// nobody can name from the command line, since a name is written beside
    /// a path and the built-in has none. It is called `orbit`; see
    /// `karakuri_engine::set::BUILTIN_CAMERA`.
    pub l3s: Vec<Option<String>>,
    pub l4s: Vec<Option<String>>,
    pub fields: Vec<Option<String>>,
}

impl Names {
    /// Every name that was actually written, which is the only thing worth
    /// checking for a collision: a derived one is disambiguated where it is
    /// derived, in `Set::build_many`.
    fn written(&self) -> impl Iterator<Item = &String> {
        self.l1s
            .iter()
            .flatten()
            .chain(self.l2s.iter().flatten())
            .chain(self.l3s.iter().flatten())
            .chain(self.l4s.iter().flatten())
            .chain(self.fields.iter().flatten())
    }

    /// **Unique within a slot**, which is the scope a name resolves in: a Set is
    /// what holds the nodes, so two slots may each have a `near` and neither is
    /// ambiguous.
    ///
    /// Refused rather than disambiguated. A derived name is disambiguated
    /// where it is derived — that is what `-2` is for — so a collision reaching
    /// here is two *written* names, and picking one for the author would leave
    /// a `--param` pointing at whichever the tie-break preferred.
    pub fn check_unique(&self) -> Result<(), String> {
        let mut seen: Vec<&str> = Vec::new();
        for name in self.written() {
            if seen.contains(&name.as_str()) {
                return Err(format!(
                    "two nodes are both called `{name}` — a name addresses one node in a slot"
                ));
            }
            seen.push(name);
        }
        Ok(())
    }
}

/// **The compiled procedure and the text it was compiled from**, together.
///
/// The source is handed back rather than dropped, and that is the whole reason
/// this returns a pair. A `.kir` is read here exactly once per run, and what
/// the run goes on to say about that node — the address a live save writes, the
/// hash a `procedure` record names, the artifact a replay resolves — is a
/// function of *these* bytes. A second reader asking the path again is a second
/// answer to "what is this node running", and it is a different answer the
/// moment anything has rewritten the file in between: an editor, a model over
/// MCP, a formatter. See [`Placed`], which is where these bytes
/// are kept — beside the pipeline that read them, since the last slice of
/// ADR-0214's move brought it here.
pub fn load(path: &Path) -> Result<(Checked, String), String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let checked = compile(&src).map_err(|report| format!("{}:\n{report}", path.display()))?;
    Ok((checked, src))
}

/// **Every diagnostic from one refused file, in the two forms it has readers
/// for.**
///
/// A terminal takes the whole of it — the message, the line under it and the
/// caret, and the hint where there is one — and that is what every caller
/// before this one wanted. A *row* cannot: the Staging lane draws one line per
/// candidate, so what it can hold is the head of the first diagnostic and a
/// count of the rest ([`karakuri_engine::swap::Refusal`], and
/// `docs/principles/0083-…`, whose one-round-trip half is what keeps the
/// whole report on the terminal and on the MCP surface rather than trimming it
/// everywhere).
///
/// **[`Diagnostics::said`] is the first line of each rendered diagnostic and
/// not a second rendering of it.** `IrError::render` writes *line:col: stage:
/// message* and then the source line, the caret and the hint, so the head of
/// what a terminal reads and the whole of what a row reads are one derivation
/// rather than two that can drift
/// (`docs/principles/0087-name-the-property-never-the-shape.md`).
pub struct Diagnostics {
    /// Rendered against the source it came from, diagnostics separated by a
    /// blank line — what was printed before this type existed, unchanged.
    pub report: String,
    /// The same diagnostics, one line each: where, which stage, and what.
    pub said: Vec<String>,
}

impl std::fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.report)
    }
}

/// The same five stages, over source already in hand rather than a path.
///
/// A Set file's procedures arrive as bytes out of the store, so they have no
/// path to name in a diagnostic; everything else about validating them is
/// identical, and it is the same function.
pub fn check(src: &str) -> Result<Checked, String> {
    compile(src).map_err(|diagnostics| diagnostics.report)
}

/// [`check`], with the diagnostics kept apart rather than joined.
///
/// **For the one caller that has a row to draw them on**, which is
/// [`crate::watch::Watch`]: a refusal it used to print and drop is now an
/// outcome the deck reports and the Staging lane draws, and the lane needs the
/// first diagnostic on its own. Everything else about the two is the same
/// call.
pub fn diagnose(src: &str) -> Result<Checked, Diagnostics> {
    compile(src)
}

fn compile(src: &str) -> Result<Checked, Diagnostics> {
    let proc = karakuri_ir::parse(src).map_err(|e| render(&e, src))?;
    let checked = karakuri_ir::check::check(&proc).map_err(|e| render(&e, src))?;
    let cost = karakuri_ir::cost::estimate(&checked).map_err(|e| render(&e, src))?;
    // **A field is reported on its own terms**, because neither figure beside
    // it means anything for one: it has no elements to have a per-element cost
    // and nothing to spawn. Printing those zeroes beside it was a line that read
    // as a measurement and was not one.
    //
    // **And no bytes/element from anything, because this is one procedure and
    // a procedure does not decide the layout it is allocated under.** The
    // struct the engine sizes a buffer by is built from everything that reached
    // the node, plus whatever a downstream consumer asked to be carried — both
    // properties of a Set, and there is no Set anywhere in this file. A built
    // Set is what reports the figure now; see the module doc on
    // `karakuri_ir::cost` for why nowhere earlier could.
    if proc.kind == karakuri_ir::Kind::Field {
        eprintln!(
            "  {} — {} ops/evaluation",
            proc.name, cost.ops_per_evaluation
        );
    } else {
        eprintln!(
            "  {} — {} ops/element, {} ops/spawn",
            proc.name, cost.ops_per_element, cost.ops_per_spawn
        );
    }
    Ok(checked)
}

fn render(errors: &[karakuri_ir::IrError], src: &str) -> Diagnostics {
    let rendered: Vec<String> = errors.iter().map(|e| e.render(src)).collect();
    // **The head of each rendered diagnostic**, which `IrError::render` writes
    // as `line:col: stage: message` before the source line and the caret. Taken
    // from the render rather than re-derived, so the sentence a row draws
    // cannot come to disagree with the one a terminal prints — there is one
    // formatting of a diagnostic in this workspace and this is a slice of it.
    let said = rendered
        .iter()
        .map(|d| d.lines().next().unwrap_or_default().to_owned())
        .collect();
    Diagnostics {
        report: rendered.join("\n\n"),
        said,
    }
}

/// A `.kir` named on the command line, with the name the operator gave it.
///
/// **A name belongs to the *use*, not to the procedure** — see `docs/ir-spec.md`,
/// "Naming a source, on the terms HTML gives an `id`". The same lattice twice is
/// one `proc` name and two nodes, so the name is written where the file is
/// spelled and travels with that mention of it.
///
/// `None` is a path written bare. Something still has to address that node, so a
/// name is derived from the procedure once it has compiled, and from the moment
/// it is recorded — derived or written — it *is* the address.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Named {
    pub name: Option<String>,
    pub path: PathBuf,
}

impl Named {
    pub fn bare(path: impl Into<PathBuf>) -> Named {
        Named {
            name: None,
            path: path.into(),
        }
    }

    /// `name=path`, or a bare path.
    ///
    /// **The separator is `=` and not `:`**, which `--param` and `--publish`
    /// already use to mean "a layer and an index follow". `=` is not a path
    /// character anywhere, where `:` is one on Windows.
    pub fn parse(spelled: &str) -> Result<Named, String> {
        let Some((name, path)) = spelled.split_once('=') else {
            return Ok(Named::bare(spelled));
        };
        check_node_name(name)?;
        if path.is_empty() {
            return Err(format!("`{spelled}` — a name with no file after it"));
        }
        Ok(Named {
            name: Some(name.to_string()),
            path: PathBuf::from(path),
        })
    }
}

/// What a node may be called.
///
/// **Refused rather than mangled**, because a name is an address: something
/// silently renamed is something a mask or a `--param` written against it stops
/// finding, and the author is the only one who can pick the replacement.
fn check_node_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("a node name cannot be empty".to_string());
    }
    // **The layer spellings are reserved**, so that `--param near:radius=1` and
    // `--param L4:0:exposure=1` can never be the same sentence about different
    // things. The two forms are told apart by counting colons, and a node called
    // `L4` would make that count a lie.
    if crate::meta::layer_named(name).is_some() {
        return Err(format!(
            "`{name}` is a layer, so it cannot also be a node's name — a `--param` is told \
             which of the two it names by the shape of what follows"
        ));
    }
    if let Some(bad) = name
        .chars()
        .find(|c| !c.is_ascii_alphanumeric() && *c != '_' && *c != '-')
    {
        return Err(format!(
            "`{name}` cannot be a node name: `{bad}` is not allowed. Letters, digits, `_` and `-`"
        ));
    }
    Ok(())
}

/// One deck slot's material, sorted into the chain a `Set` is built from: the
/// procedure that simulates, the deformations between, and the renderers drawn
/// over the result.
///
/// **The sorting happens here rather than on the command line**, because every
/// `.kir` declares its own `kind` — so `--set` is one comma-separated list and
/// the loader reads which is which off the files. Order *within* a kind is list
/// order, which is chain order for L2s and draw order for L4s.
pub struct Material {
    /// **The geometry sources**, in the order their paths appeared. At least
    /// one; several is a merge.
    pub l1s: Vec<karakuri_ir::typed::Checked>,
    pub l2s: Vec<karakuri_ir::typed::Checked>,
    /// **The cameras, in the order their paths appeared.** Empty leaves the
    /// slot looking from the built-in orbit, which is a node all the same — so
    /// a Set has at least one camera however this list comes out.
    pub l3s: Vec<karakuri_ir::typed::Checked>,
    /// **The fields, in the order their paths appeared.** Empty for a slot that
    /// evaluates none.
    pub fields: Vec<karakuri_ir::typed::Checked>,
    pub l4s: Vec<karakuri_ir::typed::Checked>,
    /// What each of those is called, in the same per-layer shape.
    pub names: Names,
}

/// Where one of a slot's files ended up: the layer its own `kind` declaration
/// puts it on, and which node of that layer it is, beside the name and path it
/// was spelled with.
///
/// **Kept beside the compiled [`Material`] rather than worked out again.** A
/// Set file records a node's layer and its index, and a run that answered "what
/// layer is this file on" once for the engine and once for the file it saves
/// would hold two answers to one question — the shape this project has been
/// bitten by twice. See [`crate::setfile::Node`], which is this as the record.
// **`Eq` and not merely `PartialEq` is what the metadata card costs**: a record
// carries the declared range as `f32`, so the lines are comparable and not
// totally so. Nothing asks for `Eq` — no `Placed` is a map key — and the
// comparison that is used, in the tests below, is unchanged: two nodes with the
// same source have the same card, because the card is a function of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub named: Named,
    /// The name the procedure itself declares, which is not the name in
    /// `named`: that one belongs to the *use* and is `None` for a bare path.
    ///
    /// **Read where the file was compiled, not scanned for again.** The edit
    /// history files a version under the procedure's name and a session's
    /// `procedure` record carries it, and both of them are looking at a node
    /// this already knows the layer and index of — asking a second reader would
    /// be a second answer to a question already settled here.
    pub(crate) proc: String,
    pub layer: karakuri_ir::Kind,
    pub index: u32,
    /// **The text this node was compiled from**, carried from the read that
    /// produced the `Checked` beside it.
    ///
    /// This is the *one* derivation of "what bytes is this node running", and
    /// everything downstream is a function of it: [`Placed::hash`] is the
    /// address a live save writes and a `procedure` record names,
    /// [`Placed::put`] is how those bytes reach the store, and the edit history
    /// files this same buffer. It used to be re-read from `named.path` by each
    /// of them, which made the answer whatever the disk happened to hold at the
    /// moment they asked — and between the compile and the first frame sit the
    /// adapter request, the deck build, `measure_slots`, and the audio, MIDI,
    /// tempo and **MCP server** starts. Anything rewriting a `.kir` in that
    /// window moved the launch hash onto bytes the deck had never compiled; if
    /// the rewrite did not compile, no watcher ever corrected it, and `k` wrote
    /// a Set naming a procedure that had never reached the screen and did not
    /// load back.
    ///
    /// **Shared rather than copied**, because a slot's `Placed` list is cloned
    /// into the surface's `Live::startup` — one crate over — and crosses onto a
    /// save thread; the bytes
    /// themselves are read once and never again.
    pub source: std::sync::Arc<str>,
    /// **This node's metadata card**, built from the `Checked` the compile
    /// produced — see [`crate::meta::card`].
    ///
    /// It rides here for the reason `source` does, and it is the same reason
    /// twice: this is the one compile the run will do of these bytes, and the
    /// card is a function of it. The alternative was to build it where the
    /// artifact is written, which would mean **re-compiling the source at save
    /// time** — a second pass whose answer can differ from the one on screen
    /// the moment anything about the checker is version-dependent, and a
    /// compile on the path of a keypress besides. `Checked` itself is not
    /// carried: what a card says is decided once, and holding the whole checked
    /// tree per node to re-derive it would be holding the question instead of
    /// the answer.
    ///
    /// Shared, because a slot's list is cloned onto the save thread.
    pub meta: std::sync::Arc<[karakuri_store::ndjson::Line]>,
}

impl Placed {
    /// **Where this node's source is addressed**, derived from the bytes above
    /// and from nothing else.
    ///
    /// A method rather than a field beside `source`, because a hash is a pure
    /// function of the bytes: computed here it cannot disagree with them, where
    /// a stored copy would be a second thing to keep true. It is a few
    /// kilobytes hashed at launch and again per save, which is not a rate
    /// anything here is bounded by.
    pub fn hash(&self) -> karakuri_store::hash::Hash {
        karakuri_store::hash::Hash::of(self.source.as_bytes())
    }

    /// **Put this node's source in `store`**, so that a file or a record naming
    /// [`Placed::hash`] resolves on the way back in.
    ///
    /// **Separate from [`Placed::node`], and called later than it.** Knowing
    /// what a slot is running costs nothing and every windowed run needs it;
    /// writing the bytes down creates a directory and a file, and only two
    /// callers need that — a save that actually happened, and a run recording a
    /// session, whose `procedure` records a replay has to resolve. Folding the
    /// two together is what made a plain windowed run create a store it was
    /// never asked for; see the surface's `Running::at_launch`.
    ///
    /// **The card goes down beside the artifact, and a card that will not write
    /// does not fail the put.** The artifact is the thing; its metadata is
    /// derived from the `.kir` plus a compile pass and regenerates on the next
    /// one, so a store holding the source and no card holds everything that
    /// cannot be recovered. Failing here instead would mean an operator losing
    /// a save — or a session losing a `procedure` record's source — over a file
    /// nothing has read yet. It is still said out loud: silence would leave a
    /// library quietly thinning out as it grew.
    pub fn put(
        &self,
        store: &karakuri_store::store::Store,
    ) -> Result<karakuri_store::hash::Hash, String> {
        let hash = store
            .put_artifact(self.source.as_bytes())
            .map_err(|e| format!("{}: {e}", self.named.path.display()))?;
        put_meta(store, &hash, &self.meta);
        Ok(hash)
    }
}

/// **Sort one slot's compiled procedures by the `kind` each declares**, keeping
/// list order within a kind.
///
/// **The one place that answers "which layer is this file on, and which node of
/// it".** Both ways into a slot come through here: [`sort_slot`] compiles from
/// paths at startup, and [`crate::watch::Watch`]'s `poll` compiles from text it
/// has already read — it needs the bytes for the edit history and for the artifacts
/// a session stores. Loading is the only thing they do differently, so the seam
/// is after the compile and this takes procedures rather than paths. The two
/// used to hold a copy each of this match, and they had already drifted: the
/// rebuild still took its head for the L1 whatever the file declared.
///
/// **The first procedure is the first source**, and every later `kind L1` is
/// another one. Each simulates independently — its own `seed` from zero, its own
/// hash salt, its own compaction — and the renderers draw all of them. See
/// `docs/ir-spec.md`, "Multiple L1 sources".
///
/// The head is sorted by its declaration like everything after it. It used to
/// be taken as the L1 whatever it said, which cost nothing while the engine was
/// the only reader — it refuses a non-L1 there with `WrongKind` — and starts
/// costing as soon as a Set file records the layer: an L2 written down as
/// `slot L1` is a file that reads back as a Set nobody assembled.
///
/// **`Err` is a sentence and not an exit**, because the two callers answer a
/// refusal differently and that difference is the only reason there were ever
/// two of these: a startup that cannot assemble its slot has nothing to run and
/// stops, while a rebuild that cannot leaves the Set that *is* running alone.
/// Each prefixes the slot it is about and decides.
pub fn sort_compiled(
    compiled: Vec<(Named, karakuri_ir::typed::Checked, std::sync::Arc<str>)>,
) -> Result<(Material, Vec<Placed>), String> {
    // Taken before the loop consumes the list. The "nothing draws" refusal
    // names the file the slot was given, which is the one an operator looks at
    // first — and by then it has been moved from.
    let head = compiled
        .first()
        .map(|(named, ..)| named.path.display().to_string());
    // **A name per node, in the same per-layer shape the engine takes its
    // procedures in.** Not one flat list in node order: that order is the
    // engine's, and a caller that reproduced it would be the second place a
    // fact this project has already been bitten by lives.
    let mut names = Names::default();
    let mut l1s = Vec::new();
    let mut l2s = Vec::new();
    let mut l3s: Vec<karakuri_ir::typed::Checked> = Vec::new();
    let mut fields: Vec<karakuri_ir::typed::Checked> = Vec::new();
    let mut l4s = Vec::new();
    let mut placed = Vec::new();
    for (named, checked, source) in compiled {
        let name = named.name.clone();
        let proc = checked.name.clone();
        // **Each arm hands back the `Checked` it just filed**, because the
        // card below is read off it and the arm is where it otherwise goes out
        // of reach. This loop is the last point in the run holding both the
        // checked procedure and the bytes it came from — see [`Placed::meta`]
        // for why the card is kept and the tree is not.
        let (layer, index, filed) = match checked.kind {
            karakuri_ir::Kind::L2 => {
                names.l2s.push(name);
                l2s.push(checked);
                (karakuri_ir::Kind::L2, l2s.len() - 1, l2s.last())
            }
            karakuri_ir::Kind::L4 => {
                names.l4s.push(name);
                l4s.push(checked);
                (karakuri_ir::Kind::L4, l4s.len() - 1, l4s.last())
            }
            // **A second camera is a second camera**, and this was the last
            // refusal in this file that said otherwise. It said a slot looks
            // from one viewpoint, which was true of the plumbing and not of
            // the material: a renderer declares `uses view : Camera` and an
            // `edge` names which node fills it, so two cameras are two nodes
            // with two names and nothing has to arbitrate between them. A
            // renderer that names none draws from the first, which is what it
            // has always drawn from.
            karakuri_ir::Kind::L3 => {
                names.l3s.push(name);
                l3s.push(checked);
                (karakuri_ir::Kind::L3, l3s.len() - 1, l3s.last())
            }
            // **A second field is a second field**, not a mistake — the last
            // refusal in this file that said otherwise, and it said so about
            // the plumbing rather than about the material. A caller declares
            // `uses shape : Field` and an `edge` names which node fills it, so
            // two fields are two nodes with two names and nothing has to
            // arbitrate between them. What it took was a `Vec` here, in
            // `Loaded`, in `Wiring` and in the engine — an `Option` apiece was
            // all that was left of "there is exactly one, so it needs no name".
            karakuri_ir::Kind::Field => {
                names.fields.push(name);
                fields.push(checked);
                (karakuri_ir::Kind::Field, fields.len() - 1, fields.last())
            }
            // **A second L1 is a second source**, not a mistake. Each one
            // simulates independently — its own `seed` from zero, its own hash
            // salt, its own compaction — and the renderers draw all of them,
            // and rebuilding one needs nothing a first does not: a request
            // carries a list, and every record of a build addresses a node as
            // `(slot, layer, index)`. The rebuild's copy of this used to refuse
            // — a slot with two geometries started, then printed a refusal on
            // every save for the rest of the run with the picture frozen at its
            // startup build.
            karakuri_ir::Kind::L1 => {
                names.l1s.push(name);
                l1s.push(checked);
                (karakuri_ir::Kind::L1, l1s.len() - 1, l1s.last())
            }
            // **Refused, and refused here rather than filed into a list
            // nothing reads.** `kind L5` parses, checks, costs and lowers as of
            // M5.16's IR/codegen pass — a `.kir` is a complete artifact and can
            // be stored. What does not exist yet is anywhere to *put* one: the
            // master chain is still three fixed passes, and a Set has no L5
            // node for an `edge` to bind. Accepting it into a slot would build
            // a Set with a procedure in it that never runs, which is the one
            // shape
            // [ADR-0032](../../../docs/adr/0032-nothing-checks-clean-and-comes-up-short-at-runtime.md)
            // exists to refuse.
            karakuri_ir::Kind::L5 => {
                return Err(format!(
                    "{} declares `kind L5`, and a Set has nowhere to put one yet — a frame \
                     effect runs in the master chain, which is still three fixed passes. The \
                     file compiles and can be stored; what is missing is the chain's slots",
                    named.path.display()
                ));
            }
        };
        let checked = filed.expect("the procedure was pushed onto that layer's list above");
        // **The node first, and its card read off the node.** A card names the
        // artifact it describes by its hash, and [`Placed::hash`] is where a
        // node's address is derived — spelling `Hash::of(source)` here as well
        // would be a second derivation of one fact, which is the defect this
        // file has been bitten by twice and the reason `hash` is a method
        // rather than a field beside `source`. `meta` is therefore empty for
        // exactly one statement, and nothing can observe a `Placed` in that
        // state: the node is not reachable until it is pushed.
        let mut node = Placed {
            named,
            proc,
            layer,
            index: index as u32,
            source,
            meta: Vec::new().into(),
        };
        node.meta = crate::meta::card(&node.hash(), checked).into();
        placed.push(node);
    }
    if l4s.is_empty() {
        return Err(match head {
            Some(head) => format!(
                "nothing here draws — {head} names no L4, and a Set with no \
                 renderer has no frame to give"
            ),
            // Nothing at all to sort is the same refusal with no file to point
            // at. Neither caller can reach it — a slot is spelled with a head —
            // and answering it here is cheaper than making that a precondition.
            None => "nothing here draws — a Set with no renderer has no frame to give".to_string(),
        });
    }
    names.check_unique()?;
    Ok((
        Material {
            l1s,
            l2s,
            l3s,
            fields,
            l4s,
            names,
        },
        placed,
    ))
}

/// **Compile one slot's files and sort them**, which is [`sort_compiled`] with
/// the loading in front of it and the exit behind it.
///
/// Fatal on anything the sort refuses, and fatal here rather than at the build,
/// because the file is what an operator can fix. A run that cannot assemble a
/// slot has no picture to keep showing — which is exactly what the rebuild
/// path, over the same sort, does instead.
pub fn sort_slot(slot: usize, l1: &Named, rest: &[Named]) -> (Material, Vec<Placed>) {
    let named: Vec<String> = rest.iter().map(|p| p.path.display().to_string()).collect();
    eprintln!(
        "  slot {slot}: {} + {}",
        l1.path.display(),
        named.join(" + ")
    );
    // **The text each file was compiled from travels with it**, because it is
    // the only copy this run will ever make of those bytes — see [`Placed`],
    // which is where it lands and why re-reading the path later was wrong.
    let compiled: Vec<(Named, karakuri_ir::typed::Checked, std::sync::Arc<str>)> =
        std::iter::once(l1)
            .chain(rest)
            .map(|named| match load(&named.path) {
                Ok((checked, src)) => (named.clone(), checked, std::sync::Arc::from(src.as_str())),
                Err(report) => {
                    eprintln!("{report}");
                    std::process::exit(1);
                }
            })
            .collect();
    match sort_compiled(compiled) {
        Ok(sorted) => sorted,
        Err(e) => {
            eprintln!("slot {slot}: {e}");
            std::process::exit(1);
        }
    }
}
