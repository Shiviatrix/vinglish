use vinglish_lexer::Span;

/// Severity level for a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Hint,
    Warning,
    Error,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hint => write!(f, "hint"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// A fix suggestion shown alongside the error.
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub label: String,
    pub replacement: Option<String>,
    pub confidence: Option<f32>,
}

impl Suggestion {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            replacement: None,
            confidence: None,
        }
    }
    pub fn with_replacement(mut self, r: impl Into<String>) -> Self {
        self.replacement = Some(r.into());
        self
    }
    pub fn with_confidence(mut self, c: f32) -> Self {
        self.confidence = Some(c);
        self
    }
}

/// A single diagnostic message.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub span: Span,
    pub source_line: Option<String>, // The actual source line for display
    pub line_number: Option<u32>,
    pub col_number: Option<u32>,
    pub suggestions: Vec<Suggestion>,
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            span,
            source_line: None,
            line_number: None,
            col_number: None,
            suggestions: vec![],
            notes: vec![],
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Warning,
            code: code.into(),
            message: message.into(),
            span,
            source_line: None,
            line_number: None,
            col_number: None,
            suggestions: vec![],
            notes: vec![],
        }
    }

    pub fn hint(message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Hint,
            code: "H0000".into(),
            message: message.into(),
            span,
            source_line: None,
            line_number: None,
            col_number: None,
            suggestions: vec![],
            notes: vec![],
        }
    }

    pub fn with_suggestion(mut self, s: Suggestion) -> Self {
        self.suggestions.push(s);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Fill in line/column information from the source text.
    pub fn enrich(&mut self, src: &str) {
        let target = self.span.start as usize;
        let mut line_num = 1u32;
        let mut col_num = 1u32;
        let mut line_start = 0usize;

        for (i, ch) in src.char_indices() {
            if i == target {
                break;
            }
            if ch == '\n' {
                line_num += 1;
                col_num = 1;
                line_start = i + 1;
            } else {
                col_num += 1;
            }
        }

        self.line_number = Some(line_num);
        self.col_number = Some(col_num);

        // Extract the source line text
        let rest = &src[line_start..];
        let line_text = rest.lines().next().unwrap_or("").to_string();
        self.source_line = Some(line_text);
    }
}

/// Convert lex/parse/type errors into diagnostics, and enrich with intent suggestions.
/// `symbol_table` is a list of known symbol names for typo detection.
pub fn from_unknown_ident(name: &str, span: Span, symbol_table: &[&str]) -> Diagnostic {
    use strsim::jaro_winkler;

    let mut scored: Vec<(&str, f64)> = symbol_table
        .iter()
        .map(|s| (*s, jaro_winkler(name, s)))
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut diag = Diagnostic::error("E0001", format!("unknown identifier `{}`", name), span);

    for (candidate, score) in scored.iter().take(3) {
        if *score > 0.8 {
            let confidence = (score * 100.0) as f32;
            diag.suggestions.push(
                Suggestion::new(format!("did you mean `{}`?", candidate))
                    .with_replacement(candidate.to_string())
                    .with_confidence(confidence),
            );
        }
    }

    if diag.suggestions.is_empty() {
        diag.notes
            .push("check the identifier spelling or import the relevant module".into());
    }

    diag
}
