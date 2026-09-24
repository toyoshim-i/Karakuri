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
    /// True if this L4 procedure draws fullscreen without a vertex block.
    pub(crate) fullscreen: bool,
    /// Name of the declared secondary geometry slot (`uses far : Geometry`), if any.
    pub(crate) uses: Option<&'a str>,
    /// Declared field slot names (`uses <name> : Field`).
    pub(crate) fields: &'a [&'a str],
    /// Declared camera slot name (`uses view : Camera`), if any.
    pub(crate) camera: Option<&'a str>,
    /// Declared source slot names (`uses <name> : Source`).
    pub(crate) sources: &'a [&'a str],
    /// Declared texture slot names (`uses <name> : Texture`).
    pub(crate) textures: &'a [&'a str],
    /// True if the header declared `retains`.
    pub(crate) retains: bool,
    /// Declared secondary geometry slot name used to detect ambiguous `source` references.
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

    /// Resolves `name` to a texture reference (`src`, `held`, or a declared texture slot).
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

    /// Returns true if ray marching ambients (`eye`, `ray`) are allowed in this procedure.
    pub(crate) fn marching_only(&self, amb: Ambient) -> bool {
        !matches!(amb, Ambient::Eye | Ambient::Ray) || self.fullscreen
    }

    /// Returns true if element identity ambients (`seed`, `copy`) are allowed in this procedure.
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
