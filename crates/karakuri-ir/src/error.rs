//! Diagnostic and error representations for `.kir` compilation and analysis stages.
use serde::{Deserialize, Serialize};

use crate::span::{line_col, line_text, Span};

pub mod codes {
    // Stage::Parse (100 series)
    pub const E100_PARSE: &str = "KIR-E100-PARSE";
    pub const E100_COMPILE_FAILED: &str = "KIR-E100-COMPILE-FAILED";
    pub const E101_UNTERMINATED_BLOCK: &str = "KIR-E101-UNTERMINATED-BLOCK";
    pub const E102_UNKNOWN_KIND: &str = "KIR-E102-UNKNOWN-KIND";
    pub const E103_UNKNOWN_TOPOLOGY: &str = "KIR-E103-UNKNOWN-TOPOLOGY";
    pub const E104_UNKNOWN_BLEND: &str = "KIR-E104-UNKNOWN-BLEND";
    pub const E105_PARAM_SYNTAX: &str = "KIR-E105-PARAM-SYNTAX";
    pub const E106_MALFORMED_SWIZZLE: &str = "KIR-E106-MALFORMED-SWIZZLE";
    pub const E107_WHILE_NOT_SUPPORTED: &str = "KIR-E107-WHILE-NOT-SUPPORTED";
    pub const E108_FOR_BOUND: &str = "KIR-E108-FOR-BOUND";

    // Stage::Type (200 series)
    pub const E200_TYPE: &str = "KIR-E200-TYPE";
    pub const E201_UNDECLARED_NAME: &str = "KIR-E201-UNDECLARED-NAME";
    pub const E202_INVALID_SWIZZLE: &str = "KIR-E202-INVALID-SWIZZLE";
    pub const E203_TYPE_MISMATCH: &str = "KIR-E203-TYPE-MISMATCH";

    // Stage::Contract (300 series)
    pub const E101_UNDECLARED_PARAM: &str = "KIR-E101-UNDECLARED-PARAM";
    pub const E300_CONTRACT: &str = "KIR-E300-CONTRACT";
    pub const E300_LAYER_MISMATCH: &str = "KIR-E300-LAYER-MISMATCH";
    pub const E302_USES_CONTRACT: &str = "KIR-E302-USES-CONTRACT";
    pub const E303_AMPLIFY_CONTRACT: &str = "KIR-E303-AMPLIFY-CONTRACT";
    pub const E304_BLEND_CONTRACT: &str = "KIR-E304-BLEND-CONTRACT";
    pub const E305_TOPOLOGY_CONTRACT: &str = "KIR-E305-TOPOLOGY-CONTRACT";
    pub const E306_CAPACITY_CONTRACT: &str = "KIR-E306-CAPACITY-CONTRACT";
    pub const E307_SEED_CONTRACT: &str = "KIR-E307-SEED-CONTRACT";
    pub const E308_TEXTURE_CONTRACT: &str = "KIR-E308-TEXTURE-CONTRACT";
    pub const E309_CAMERA_CONTRACT: &str = "KIR-E309-CAMERA-CONTRACT";
    pub const E310_MISSING_OUTPUT: &str = "KIR-E310-MISSING-OUTPUT";

    // Stage::Cost (400 series)
    pub const E400_COST_EXCEEDED: &str = "KIR-E400-COST-EXCEEDED";
    pub const E401_EVAL_COST_EXCEEDED: &str = "KIR-E401-EVAL-COST-EXCEEDED";
    pub const E402_ELEMENT_COST_EXCEEDED: &str = "KIR-E402-ELEMENT-COST-EXCEEDED";
    pub const E403_SPAWN_COST_EXCEEDED: &str = "KIR-E403-SPAWN-COST-EXCEEDED";

    // Stage::Store / Tool (500 series)
    pub const E501_STORE_OPEN_FAILED: &str = "KIR-E501-STORE-OPEN-FAILED";
    pub const E502_SET_READ_FAILED: &str = "KIR-E502-SET-READ-FAILED";

    pub const ALL: &[&str] = &[
        E100_PARSE,
        E100_COMPILE_FAILED,
        E101_UNTERMINATED_BLOCK,
        E102_UNKNOWN_KIND,
        E103_UNKNOWN_TOPOLOGY,
        E104_UNKNOWN_BLEND,
        E105_PARAM_SYNTAX,
        E106_MALFORMED_SWIZZLE,
        E107_WHILE_NOT_SUPPORTED,
        E108_FOR_BOUND,
        E200_TYPE,
        E201_UNDECLARED_NAME,
        E202_INVALID_SWIZZLE,
        E203_TYPE_MISMATCH,
        E101_UNDECLARED_PARAM,
        E300_CONTRACT,
        E300_LAYER_MISMATCH,
        E302_USES_CONTRACT,
        E303_AMPLIFY_CONTRACT,
        E304_BLEND_CONTRACT,
        E305_TOPOLOGY_CONTRACT,
        E306_CAPACITY_CONTRACT,
        E307_SEED_CONTRACT,
        E308_TEXTURE_CONTRACT,
        E309_CAMERA_CONTRACT,
        E310_MISSING_OUTPUT,
        E400_COST_EXCEEDED,
        E401_EVAL_COST_EXCEEDED,
        E402_ELEMENT_COST_EXCEEDED,
        E403_SPAWN_COST_EXCEEDED,
        E501_STORE_OPEN_FAILED,
        E502_SET_READ_FAILED,
    ];
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,    // Machine-readable diagnostic code
    pub message: String, // Human-readable summary
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub remedy: Option<String>, // Actionable suggestion for LLM self-healing
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DiagnosticReport {
    pub diagnostics: Vec<Diagnostic>,
    pub success: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
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
            hint: None,
        }
    }

    pub fn code_for(err: &IrError) -> String {
        err.code.to_string()
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
            hint: None,
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
        let err = IrError::parse(codes::E100_PARSE, span, message);
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
            hash: None,
            hint: None,
        }
    }

    pub fn fail(diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            diagnostics,
            success: false,
            hash: None,
            hint: None,
        }
    }

    pub fn from_ir_errors(errors: &[IrError], src: &str) -> Self {
        Self {
            diagnostics: errors
                .iter()
                .map(|e| Diagnostic::from_ir_error(e, src))
                .collect(),
            success: errors.is_empty(),
            hash: None,
            hint: None,
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
    pub code: &'static str,
    pub message: String,
    pub span: Span,
    /// Shown under the message. Use it whenever there is a specific thing the
    /// author should have written instead.
    pub hint: Option<String>,
}

impl IrError {
    pub fn new(
        stage: Stage,
        code: &'static str,
        span: Span,
        message: impl Into<String>,
    ) -> IrError {
        IrError {
            stage,
            code,
            message: message.into(),
            span,
            hint: None,
        }
    }

    pub fn parse(code: &'static str, span: Span, message: impl Into<String>) -> IrError {
        IrError::new(Stage::Parse, code, span, message)
    }

    pub fn ty(code: &'static str, span: Span, message: impl Into<String>) -> IrError {
        IrError::new(Stage::Type, code, span, message)
    }

    pub fn contract(code: &'static str, span: Span, message: impl Into<String>) -> IrError {
        IrError::new(Stage::Contract, code, span, message)
    }

    pub fn cost(code: &'static str, span: Span, message: impl Into<String>) -> IrError {
        IrError::new(Stage::Cost, code, span, message)
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
        let err = IrError::parse(
            codes::E102_UNKNOWN_KIND,
            Span::new(16, 18),
            "unknown kind `L9`",
        )
        .with_hint("v0.2 defines `L1` and `L4`");
        let out = err.render(src);
        assert!(out.starts_with("2:8: parse: unknown kind `L9`"), "{out}");
        assert!(out.contains("^^"), "{out}");
        assert!(out.ends_with("hint: v0.2 defines `L1` and `L4`"), "{out}");
    }

    #[test]
    fn diagnostic_structured_report_and_serde() {
        let src = "proc a {\n  kind L9\n  undeclared = 1.0\n}\n";
        let err1 = IrError::parse(
            codes::E102_UNKNOWN_KIND,
            Span::new(16, 18),
            "unknown kind `L9`",
        )
        .with_hint("v0.2 defines `L1` and `L4`");
        let err2 = IrError::ty(
            codes::E201_UNDECLARED_NAME,
            Span::new(22, 32),
            "undeclared name",
        );

        let report = DiagnosticReport::from_ir_errors(&[err1, err2], src);
        assert!(!report.success);
        assert_eq!(report.diagnostics.len(), 2);

        let d1 = &report.diagnostics[0];
        assert_eq!(d1.code, codes::E102_UNKNOWN_KIND);
        assert!(d1.message.contains("parse: unknown kind `L9`"));
        assert_eq!(d1.line, Some(2));
        assert_eq!(d1.column, Some(8));
        assert_eq!(d1.remedy.as_deref(), Some("v0.2 defines `L1` and `L4`"));

        let d2 = &report.diagnostics[1];
        assert_eq!(d2.code, codes::E201_UNDECLARED_NAME);
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
