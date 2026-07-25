//! Diagnostics.
//!
//! Errors carry the stage that produced them, because the validation pipeline is
//! ordered and a failure at any stage means no artifact. They carry a span so
//! the message can point at source, and an optional hint, which is where a
//! known LLM mistake gets its correction: `id` suggesting `seed`, or a signal
//! name suggesting a `param` plus a `bind` record.

use crate::span::{line_col, line_text, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Parse,
    Type,
    Contract,
    Cost,
}

impl Stage {
    pub fn name(self) -> &'static str {
        match self {
            Stage::Parse => "parse",
            Stage::Type => "type",
            Stage::Contract => "contract",
            Stage::Cost => "cost",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}: {message}", stage.name())]
pub struct IrError {
    pub stage: Stage,
    pub message: String,
    pub span: Span,
    /// Shown under the message. Use it whenever there is a specific thing the
    /// author should have written instead.
    pub hint: Option<String>,
}

impl IrError {
    pub fn new(stage: Stage, span: Span, message: impl Into<String>) -> IrError {
        IrError {
            stage,
            message: message.into(),
            span,
            hint: None,
        }
    }

    pub fn parse(span: Span, message: impl Into<String>) -> IrError {
        IrError::new(Stage::Parse, span, message)
    }

    pub fn ty(span: Span, message: impl Into<String>) -> IrError {
        IrError::new(Stage::Type, span, message)
    }

    pub fn contract(span: Span, message: impl Into<String>) -> IrError {
        IrError::new(Stage::Contract, span, message)
    }

    pub fn cost(span: Span, message: impl Into<String>) -> IrError {
        IrError::new(Stage::Cost, span, message)
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> IrError {
        self.hint = Some(hint.into());
        self
    }

    /// Render against the source it came from, with a caret under the span.
    pub fn render(&self, src: &str) -> String {
        let pos = line_col(src, self.span.start);
        let line = line_text(src, self.span.start);
        let width = (self.span.end.saturating_sub(self.span.start)).max(1) as usize;
        let gutter = format!("{} | ", pos.line);
        let mut out = format!(
            "{}:{}: {}: {}\n{}{}\n{}{}{}",
            pos.line,
            pos.col,
            self.stage.name(),
            self.message,
            gutter,
            line,
            " ".repeat(gutter.len() + pos.col.saturating_sub(1) as usize),
            "^".repeat(width),
            String::new(),
        );
        if let Some(hint) = &self.hint {
            out.push_str(&format!("\nhint: {hint}"));
        }
        out
    }
}

/// Errors are collected rather than returned one at a time: a generated
/// procedure with four mistakes should report four, so a repair prompt can fix
/// them in one pass instead of four round trips.
pub type IrResult<T> = Result<T, Vec<IrError>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_points_at_the_span() {
        let src = "proc a {\n  kind L9\n}\n";
        let err = IrError::parse(Span::new(16, 18), "unknown kind `L9`")
            .with_hint("v0.2 defines `L1` and `L4`");
        let out = err.render(src);
        assert!(out.starts_with("2:8: parse: unknown kind `L9`"), "{out}");
        assert!(out.contains("^^"), "{out}");
        assert!(out.ends_with("hint: v0.2 defines `L1` and `L4`"), "{out}");
    }
}
