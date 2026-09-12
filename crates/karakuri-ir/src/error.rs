//! Diagnostics.
//!
//! Errors carry the stage that produced them, because the validation pipeline
//! is ordered and a failure at any stage means no artifact. They carry a span
//! so the message can point at source, and an optional hint, which is where a
//! known LLM mistake gets its correction: `id` suggesting `seed`, or a signal
//! name suggesting a `param` plus a `bind` record.
use serde::{Deserialize, Serialize};

use crate::span::{line_col, line_text, Span};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,    // e.g. "KIR-E101-UNDECLARED-PARAM"
    pub message: String, // Human-readable summary
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub remedy: Option<String>, // Actionable suggestion for LLM self-healing
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DiagnosticReport {
    pub diagnostics: Vec<Diagnostic>,
    pub success: bool,
}

pub type CheckError = IrError;

impl Diagnostic {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        line: Option<usize>,
        column: Option<usize>,
        remedy: Option<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            line,
            column,
            remedy,
        }
    }

    pub fn code_for(err: &IrError) -> String {
        let msg = err.message.to_lowercase();
        match err.stage {
            Stage::Parse => {
                if msg.contains("unterminated") {
                    "KIR-E101-UNTERMINATED-BLOCK".to_string()
                } else if msg.contains("unknown kind") {
                    "KIR-E102-UNKNOWN-KIND".to_string()
                } else if msg.contains("unknown topology") {
                    "KIR-E103-UNKNOWN-TOPOLOGY".to_string()
                } else if msg.contains("unknown blend") {
                    "KIR-E104-UNKNOWN-BLEND".to_string()
                } else if msg.contains("range") || msg.contains("param") {
                    "KIR-E105-PARAM-SYNTAX".to_string()
                } else if msg.contains("swizzle") {
                    "KIR-E106-MALFORMED-SWIZZLE".to_string()
                } else if msg.contains("while") {
                    "KIR-E107-WHILE-NOT-SUPPORTED".to_string()
                } else if msg.contains("for") {
                    "KIR-E108-FOR-BOUND".to_string()
                } else {
                    "KIR-E100-PARSE".to_string()
                }
            }
            Stage::Type => {
                if msg.contains("undeclared")
                    || msg.contains("undefined")
                    || msg.contains("unknown name")
                {
                    "KIR-E201-UNDECLARED-NAME".to_string()
                } else if msg.contains("swizzle") {
                    "KIR-E202-INVALID-SWIZZLE".to_string()
                } else if msg.contains("int and float") || msg.contains("type mismatch") {
                    "KIR-E203-TYPE-MISMATCH".to_string()
                } else {
                    "KIR-E200-TYPE".to_string()
                }
            }
            Stage::Contract => {
                if msg.contains("undeclared") || msg.contains("param") {
                    "KIR-E101-UNDECLARED-PARAM".to_string()
                } else if msg.contains("uses") {
                    "KIR-E302-USES-CONTRACT".to_string()
                } else if msg.contains("amplify") {
                    "KIR-E303-AMPLIFY-CONTRACT".to_string()
                } else if msg.contains("blend") {
                    "KIR-E304-BLEND-CONTRACT".to_string()
                } else if msg.contains("topology") {
                    "KIR-E305-TOPOLOGY-CONTRACT".to_string()
                } else if msg.contains("capacity") {
                    "KIR-E306-CAPACITY-CONTRACT".to_string()
                } else if msg.contains("seed") {
                    "KIR-E307-SEED-CONTRACT".to_string()
                } else if msg.contains("texture") {
                    "KIR-E308-TEXTURE-CONTRACT".to_string()
                } else if msg.contains("camera") {
                    "KIR-E309-CAMERA-CONTRACT".to_string()
                } else if msg.contains("required") || msg.contains("missing") {
                    "KIR-E310-MISSING-OUTPUT".to_string()
                } else {
                    "KIR-E300-CONTRACT".to_string()
                }
            }
            Stage::Cost => {
                if msg.contains("evaluation") {
                    "KIR-E401-EVAL-COST-EXCEEDED".to_string()
                } else if msg.contains("element") {
                    "KIR-E402-ELEMENT-COST-EXCEEDED".to_string()
                } else if msg.contains("spawn") {
                    "KIR-E403-SPAWN-COST-EXCEEDED".to_string()
                } else {
                    "KIR-E400-COST-EXCEEDED".to_string()
                }
            }
        }
    }

    pub fn from_ir_error(err: &IrError, src: &str) -> Self {
        let pos = line_col(src, err.span.start);
        let code = Self::code_for(err);
        Self {
            code,
            message: format!("{}: {}", err.stage.name(), err.message),
            line: Some(pos.line as usize),
            column: Some(pos.col as usize),
            remedy: err.hint.clone(),
        }
    }

    pub fn from_check_error(err: &CheckError, src: &str) -> Self {
        Self::from_ir_error(err, src)
    }

    pub fn from_parse_error(
        span: Span,
        message: impl Into<String>,
        remedy: Option<String>,
        src: &str,
    ) -> Self {
        let err = IrError::parse(span, message);
        let mut d = Self::from_ir_error(&err, src);
        if remedy.is_some() {
            d.remedy = remedy;
        }
        d
    }
}

impl DiagnosticReport {
    pub fn ok() -> Self {
        Self {
            diagnostics: Vec::new(),
            success: true,
        }
    }

    pub fn fail(diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            diagnostics,
            success: false,
        }
    }

    pub fn from_ir_errors(errors: &[IrError], src: &str) -> Self {
        Self {
            diagnostics: errors
                .iter()
                .map(|e| Diagnostic::from_ir_error(e, src))
                .collect(),
            success: errors.is_empty(),
        }
    }

    pub fn from_check_errors(errors: &[CheckError], src: &str) -> Self {
        Self::from_ir_errors(errors, src)
    }
}

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

    #[test]
    fn diagnostic_structured_report_and_serde() {
        let src = "proc a {\n  kind L9\n  undeclared = 1.0\n}\n";
        let err1 = IrError::parse(Span::new(16, 18), "unknown kind `L9`")
            .with_hint("v0.2 defines `L1` and `L4`");
        let err2 = IrError::ty(Span::new(22, 32), "undeclared name");

        let report = DiagnosticReport::from_ir_errors(&[err1, err2], src);
        assert!(!report.success);
        assert_eq!(report.diagnostics.len(), 2);

        let d1 = &report.diagnostics[0];
        assert_eq!(d1.code, "KIR-E102-UNKNOWN-KIND");
        assert!(d1.message.contains("parse: unknown kind `L9`"));
        assert_eq!(d1.line, Some(2));
        assert_eq!(d1.column, Some(8));
        assert_eq!(d1.remedy.as_deref(), Some("v0.2 defines `L1` and `L4`"));

        let d2 = &report.diagnostics[1];
        assert_eq!(d2.code, "KIR-E201-UNDECLARED-NAME");
        assert_eq!(d2.line, Some(3));

        // Test serde serialization / deserialization round trip
        let json = serde_json::to_string(&report).expect("serialize");
        let deserialized: DiagnosticReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(report, deserialized);

        let ok_report = DiagnosticReport::ok();
        assert!(ok_report.success);
        assert!(ok_report.diagnostics.is_empty());
    }
}
