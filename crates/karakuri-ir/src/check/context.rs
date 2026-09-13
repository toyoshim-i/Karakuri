//! Checker context, scopes, and local variable environments.

use std::collections::{HashMap, HashSet};

use crate::ast::{Ambient, Attr, BlockKind, Kind, Output, Ty, TEXTURE_HELD, TEXTURE_SRC};
use crate::error::{IrError, Stage};
use crate::span::Span;
use crate::typed::{InputPort, TexRef};

// ---------------------------------------------------------------------------
// Scope: locals only. Params, attributes, and ambients are looked up through
// the `Checker` directly since they are proc- or block-scoped, not nested.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub(crate) struct LocalInfo {
    pub(crate) ty: Ty,
    pub(crate) mutable: bool,
}

pub(crate) struct Scope {
    pub(crate) frames: Vec<HashMap<String, LocalInfo>>,
}

impl Scope {
    pub(crate) fn new() -> Scope {
        Scope {
            frames: vec![HashMap::new()],
        }
    }

    pub(crate) fn push(&mut self) {
        self.frames.push(HashMap::new());
    }

    pub(crate) fn pop(&mut self) {
        self.frames.pop();
    }

    pub(crate) fn lookup(&self, name: &str) -> Option<LocalInfo> {
        self.frames.iter().rev().find_map(|f| f.get(name).copied())
    }

    pub(crate) fn declare(&mut self, name: String, info: LocalInfo) {
        self.frames
            .last_mut()
            .expect("at least one frame")
            .insert(name, info);
    }
}

// ---------------------------------------------------------------------------
// Checker: expression and statement checking for one block (or, with
// `block: None`, one header expression such as a param default).
// ---------------------------------------------------------------------------

pub(crate) struct Checker<'a> {
    pub(crate) kind: Kind,
    pub(crate) block: Option<BlockKind>,
    /// Whether this procedure draws the whole frame — an L4 with no `vertex` block,
    /// which is the only way to say so.
    ///
    /// Not derivable from `kind` and `block`, which is why it is carried here
    /// rather than folded into [`Ambient::available_in`]: it is a fact about the
    /// procedure, and a block checker otherwise sees only its own block. `eye` and
    /// `ray` need it — see [`Checker::marching_only`].
    pub(crate) fullscreen: bool,
    /// What this procedure calls the second geometry it takes, from `uses far :
    /// Geometry`, and `None` for a procedure that takes none.
    ///
    /// It is what makes `far.position` mean anything, and it is per *procedure*
    /// rather than per block for the reason [`Checker::fullscreen`] is: the
    /// declaration is in the header and a block checker sees only its own block.
    pub(crate) uses: Option<&'a str>,
    /// What this procedure calls the fields it evaluates, from every `uses <name> :
    /// Field` in its header.
    ///
    /// The list is what makes `shape(p)` mean anything, and it is consulted ahead
    /// of the builtin table: a call resolves against what the header declared,
    /// which is the whole of this notation. Several, because a procedure may want a
    /// shape and a cutter, and neither of them is "the" field.
    pub(crate) fields: &'a [&'a str],
    /// What this procedure calls the camera it draws from, from `uses view :
    /// Camera`, and `None` for one that declares none — which then reads the Set's
    /// camera as `camera`, `eye` and `ray`.
    ///
    /// It is what makes `view.clip` mean anything, and it is per procedure for the
    /// reason the two above are: the declaration is in the header and a block
    /// checker sees only its own block.
    pub(crate) camera: Option<&'a str>,
    /// What this procedure calls the sources it names, from every `uses <name> :
    /// Source` in its header.
    ///
    /// The list is what makes a bare `only` mean anything, and it is per procedure
    /// for the reason the three above are. Several, because a mask asking `source
    /// == a || source == b` is the ordinary case and each slot costs a `u32` in a
    /// uniform block that already exists.
    pub(crate) sources: &'a [&'a str],
    /// What this procedure calls the pictures it folds in, from every `uses <name>
    /// : Texture` in its header.
    ///
    /// The list is what makes `tap(<name>, uv)` mean anything, and it is per
    /// procedure for the reason every field beside it is. Empty on a chain slot's
    /// L5 and on every other kind, where the declaration is refused at the header.
    pub(crate) textures: &'a [&'a str],
    /// Whether the header declares `retains`, which is the whole of what makes
    /// [`TEXTURE_HELD`] readable.
    ///
    /// Carried rather than asked of the header for the reason the six lists above
    /// are: a block does not see its own header, and `held` outside `retains` has
    /// to be refused with a sentence about the declaration rather than about the
    /// name.
    pub(crate) retains: bool,
    /// What this procedure calls the second geometry it takes, if it takes one —
    /// the same declaration [`Checker::uses`] carries, kept a second time because
    /// this one is read where `source` is.
    ///
    /// It is what makes `source` ambiguous. In a pairing Set the far simulation
    /// lives inside the near `Source` and shares its uniform, so a node with a
    /// geometry slot has *two* geometries in hand and one salt to answer with — and
    /// the answer would silently be the near one. Refused with the slot's name in
    /// the sentence, because a hint that says which reading was ambiguous is better
    /// than a rule the author has to infer.
    pub(crate) paired: Option<&'a str>,
    pub(crate) params: &'a HashMap<String, Ty>,
    pub(crate) emit: &'a HashSet<Attr>,
    pub(crate) consumes: &'a HashSet<Attr>,
    pub(crate) scope: Scope,
    pub(crate) errors: Vec<IrError>,
}

pub(crate) enum TargetRes {
    Local(String, Ty, bool),
    Attr(Attr),
    Output(Output),
    /// Already reported; caller should not report again.
    Invalid,
}

impl<'a> Checker<'a> {
    // Thirteen, and each one is a fact about the *procedure* that a block
    // checker cannot see for itself — the header is not in the block. Bundling
    // them into a struct would be the same thirteen fields under one name, and
    // the struct would have exactly one constructor and one use.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        kind: Kind,
        block: Option<BlockKind>,
        fullscreen: bool,
        uses: Option<&'a str>,
        fields: &'a [&'a str],
        camera: Option<&'a str>,
        sources: &'a [&'a str],
        textures: &'a [&'a str],
        retains: bool,
        paired: Option<&'a str>,
        params: &'a HashMap<String, Ty>,
        emit: &'a HashSet<Attr>,
        consumes: &'a HashSet<Attr>,
    ) -> Checker<'a> {
        Checker {
            kind,
            block,
            fullscreen,
            uses,
            fields,
            camera,
            sources,
            textures,
            retains,
            paired,
            params,
            emit,
            consumes,
            scope: Scope::new(),
            errors: Vec::new(),
        }
    }

    /// Which texture `name` refers to, and `None` for a name that is not one here.
    ///
    /// Three sources and they are asked in this order because that is the order
    /// they are decided in: `src` is the language's, always present in a `frame`
    /// block; `held` is the header's, present under `retains`; a slot is the
    /// header's too, under whatever it was called. Nothing else can reach this —
    /// the two reserved names are refused as slot names and as locals, so one
    /// spelling never means two things.
    pub(crate) fn texture(&self, name: &str) -> Option<TexRef> {
        if self.block != Some(BlockKind::Frame) {
            return None;
        }
        if name == TEXTURE_SRC {
            return Some(TexRef::Src);
        }
        if name == TEXTURE_HELD && self.retains {
            return Some(TexRef::Held);
        }
        if self.textures.contains(&name) {
            return Some(TexRef::Slot(InputPort(name.to_string())));
        }
        None
    }

    /// `eye` and `ray` exist only where the lowering defines them, which is the ray
    /// prologue a fullscreen fragment stage opens with. A per-element L4 has a
    /// `vertex` block, gets no prologue, and reading either there lowered to a bare
    /// identifier nothing declared — so the `.kir` checked clean, `generate_l4`
    /// produced WGSL naga refuses, and wgpu's uncaptured error handler panicked the
    /// thread that built it. On the swap worker that is a `SetError::Panicked`; at
    /// startup it takes the process down.
    ///
    /// A rule about the *procedure* rather than the block, which is why it is not
    /// in [`Ambient::available_in`]: what makes an L4 a marcher is the absence of a
    /// `vertex` block, and a block does not know its siblings.
    pub(crate) fn marching_only(&self, amb: Ambient) -> bool {
        !matches!(amb, Ambient::Eye | Ambient::Ray) || self.fullscreen
    }

    /// The mirror of [`Checker::marching_only`], and it fails the same way. `seed`
    /// and `copy` are per-element identity; a fullscreen L4 has no element, no
    /// element buffer bound, and a vertex stage the procedure did not write.
    /// Reading either lowered to `in.seed` against a `VsOut` with no such field —
    /// WGSL naga refuses, and wgpu's uncaptured error handler takes the process
    /// down at startup or fails the swap worker.
    ///
    /// This is the same rule `consumes` already states for the same reason, and the
    /// reason it needed a second statement is that `seed` is not a `consumes`: it
    /// is available everywhere an element is, which is exactly the sentence a
    /// fullscreen procedure falsifies.
    pub(crate) fn element_only(&self, amb: Ambient) -> bool {
        !matches!(amb, Ambient::Seed | Ambient::Copy) || !self.fullscreen
    }

    pub(crate) fn err(&mut self, stage: Stage, span: Span, msg: impl Into<String>) {
        self.errors.push(IrError::new(stage, span, msg));
    }

    pub(crate) fn err_hint(
        &mut self,
        stage: Stage,
        span: Span,
        msg: impl Into<String>,
        hint: impl Into<String>,
    ) {
        self.errors
            .push(IrError::new(stage, span, msg).with_hint(hint));
    }
}
