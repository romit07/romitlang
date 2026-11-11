use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub snippet: String,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        file: impl Into<String>,
        line: usize,
        column: usize,
        snippet: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            file: file.into(),
            line,
            column,
            snippet: snippet.into(),
            help: None,
        }
    }

    pub fn warning(
        code: impl Into<String>,
        message: impl Into<String>,
        file: impl Into<String>,
        line: usize,
        column: usize,
        snippet: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Warning,
            code: code.into(),
            message: message.into(),
            file: file.into(),
            line,
            column,
            snippet: snippet.into(),
            help: None,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sev = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        writeln!(f, "{}[{}]: {}", sev, self.code, self.message)?;
        writeln!(f, "  --> {}:{}:{}", self.file, self.line, self.column)?;
        for line in self.snippet.lines() {
            writeln!(f, "   | {}", line)?;
        }
        if let Some(h) = &self.help {
            writeln!(f, "   = help: {}", h)?;
        }
        Ok(())
    }
}

pub fn snippet_at(source: &str, line: usize, column: usize, highlight: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let idx = line.saturating_sub(1);
    let mut out = String::new();
    if idx > 0 && idx < lines.len() {
        out.push_str(&format!("{} | {}\n", idx, lines[idx - 1]));
    }
    if idx < lines.len() {
        out.push_str(&format!("{} | {}\n", line, lines[idx]));
        let pointer = " ".repeat(column.saturating_sub(1).min(80)) + "^" + highlight;
        out.push_str(&format!("   | {}", pointer));
    }
    out
}
