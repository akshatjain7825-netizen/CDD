use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceSpan {
    pub file_id: u32,
    pub start_byte: u32,
    pub end_byte: u32,
    pub line: u32,
    pub column: u32,
}

impl SourceSpan {
    pub const fn empty() -> Self {
        Self {
            file_id: 0,
            start_byte: 0,
            end_byte: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn merge(self, other: Self) -> Self {
        Self {
            file_id: self.file_id,
            start_byte: self.start_byte.min(other.start_byte),
            end_byte: self.end_byte.max(other.end_byte),
            line: self.line.min(other.line),
            column: if self.line <= other.line {
                self.column
            } else {
                other.column
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub span: SourceSpan,
    pub message: String,
    pub notes: Vec<String>,
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            span,
            message: message.into(),
            notes: Vec::new(),
            suggestion: None,
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Warning,
            span,
            message: message.into(),
            notes: Vec::new(),
            suggestion: None,
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DiagnosticSink {
    diagnostics: Vec<Diagnostic>,
    has_errors: bool,
}

impl DiagnosticSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diag: Diagnostic) {
        if diag.severity == Severity::Error {
            self.has_errors = true;
        }
        self.diagnostics.push(diag);
    }

    pub fn has_errors(&self) -> bool {
        self.has_errors
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    pub fn render_human(&self, source_name: &str, source: &str) -> String {
        let mut out = String::new();
        let lines: Vec<&str> = source.lines().collect();

        for diag in &self.diagnostics {
            let sev_str = match diag.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            };
            out.push_str(&format!(
                "{}[{}]: {} at {}:{}:{}\n",
                sev_str, diag.code, diag.message, source_name, diag.span.line, diag.span.column
            ));

            if diag.span.line > 0 && (diag.span.line as usize) <= lines.len() {
                let line_idx = (diag.span.line - 1) as usize;
                let line_content = lines[line_idx];
                out.push_str(&format!("  {:4} | {}\n", diag.span.line, line_content));
                let col = diag.span.column.saturating_sub(1) as usize;
                let indent = " ".repeat(col);
                let width = (diag.span.end_byte.saturating_sub(diag.span.start_byte)).max(1) as usize;
                let underline = "^".repeat(width);
                out.push_str(&format!("       | {}{}\n", indent, underline));
            }

            for note in &diag.notes {
                out.push_str(&format!("  = note: {}\n", note));
            }
            if let Some(sug) = &diag.suggestion {
                out.push_str(&format!("  = help: {}\n", sug));
            }
            out.push('\n');
        }

        out
    }

    pub fn render_json(&self) -> String {
        serde_json::to_string_pretty(&self.diagnostics).unwrap_or_else(|_| "[]".to_string())
    }
}
