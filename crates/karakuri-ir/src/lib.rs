//! The Karakuri IR: parsing, type checking, and cost estimation for `.kir`.
//!
//! `.kir` is written by LLMs, validated here, and lowered to WGSL by
//! `karakuri-codegen`. The language is small on purpose — only what type
//! checking and cost estimation can verify statically — and the specification
//! in `docs/ir-spec.md` is the authority on every rule this crate enforces.
//!
//! The pipeline stages this crate owns are the first four:
//!
//! 1. parse
//! 2. type check
//! 3. contract check
//! 4. cost estimation
//!
//! Stages 5 through 8 (WGSL generation, background compilation, probe
//! measurement, promotion) belong to `karakuri-codegen` and `karakuri-engine`.

pub mod ast;
pub mod builtin;
pub mod check;
pub mod cost;
pub mod error;
pub mod layout;
mod lexer;
pub mod parse;
pub mod span;
pub mod typed;

pub use ast::{
    component_key, push_component_key, Ambient, AmplifyDecl, Attr, BinOp, Blend, Block, BlockKind,
    CapacityDecl, Derivation, Expr, Kind, Lit, Output, Param, Proc, SlotTy, Stmt, Topology, Ty,
    UnOp, UsesDecl, COMPONENTS, DEFAULT_CAPACITY,
};
pub use error::{IrError, IrResult, Stage};
pub use parse::parse;
pub use span::{LineCol, Span};
