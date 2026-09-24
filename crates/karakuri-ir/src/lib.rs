//! Parsing, type checking, contract verification, and cost estimation for the Karakuri IR (`.kir`).

pub mod ast;
pub mod builtin;
pub mod check;
pub mod cost;
pub mod error;
pub mod layout;
mod lexer;
pub mod parse;
pub mod rate;
pub mod span;
pub mod typed;

pub use ast::{
    component_key, push_component_key, Ambient, AmplifyDecl, Attr, BinOp, Blend, Block, BlockKind,
    CapacityDecl, Derivation, Expr, Kind, Lit, Output, Param, Proc, SlotTy, Stmt, Topology, Ty,
    UnOp, UsesDecl, COMPONENTS, DEFAULT_CAPACITY,
};
pub use error::{CheckError, Diagnostic, DiagnosticReport, IrError, IrResult, Stage};
pub use parse::parse;
pub use span::{LineCol, Span};
