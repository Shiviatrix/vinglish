use vinglish_lexer::Span;
use strsim::damerau_levenshtein;

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
    pub helps: Vec<String>,
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
            helps: vec![],
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
            helps: vec![],
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
            helps: vec![],
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

    pub fn add_note(&mut self, note: impl Into<String>) {
        self.notes.push(note.into());
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.helps.push(help.into());
        self
    }

    pub fn add_help(&mut self, help: impl Into<String>) {
        self.helps.push(help.into());
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

/// Calculate similarity score using multiple algorithms for better typo detection
fn similarity_score(s1: &str, s2: &str) -> f64 {
    // Use Damerau-Levenshtein which handles transpositions well
    let distance = damerau_levenshtein(s1, s2);
    let max_len = s1.len().max(s2.len()) as f64;

    if max_len == 0.0 {
        return 1.0;
    }

    // Convert distance to similarity (0-1 range, higher is better)
    let similarity = 1.0 - (distance as f64 / max_len);

    // Boost score for exact case-insensitive matches
    if s1.eq_ignore_ascii_case(s2) && s1.len() == s2.len() {
        return similarity.max(0.95);
    }

    similarity
}

/// Calculate calibrated confidence score for healing suggestions
/// This function maps raw similarity scores to more meaningful confidence percentages
/// based on empirical observations of typo patterns in programming languages
fn calibrated_confidence(raw_similarity: f64) -> f32 {
    // Apply a calibration curve to make confidence scores more meaningful
    // This prevents overconfidence in low-similarity matches while preserving
    // high confidence for clear typos

    // For very high similarity (>0.95), we're very confident
    if raw_similarity > 0.95 {
        return (95.0 + (raw_similarity - 0.95) * 100.0 * 0.5) as f32; // 95-100 range
    }

    // For high similarity (0.85-0.95), good confidence
    if raw_similarity > 0.85 {
        return (85.0 + (raw_similarity - 0.85) * 100.0 * 0.666) as f32; // 85-95 range
    }

    // For moderate similarity (0.7-0.85), lower confidence
    if raw_similarity > 0.7 {
        return (70.0 + (raw_similarity - 0.7) * 100.0) as f32; // 70-85 range
    }

    // For low similarity (<0.7), very low confidence (likely not a typo)
    return (raw_similarity * 50.0) as f32; // 0-35 range
}

/// Convert lex/parse/type errors into diagnostics, and enrich with intent suggestions.
/// `symbol_table` is a list of known symbol names for typo detection.
pub fn from_unknown_ident(name: &str, span: Span, symbol_table: &[&str]) -> Diagnostic {
    let mut scored: Vec<(&str, f64)> = symbol_table
        .iter()
        .map(|s| {
            let score = similarity_score(name, *s);
            (*s, score)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut diag = Diagnostic::error("E0001", format!("unknown identifier `{}`", name), span);

    for (candidate, score) in scored.iter().take(3) {
        // Use calibrated confidence with safety threshold
        let confidence = calibrated_confidence(*score);

        // Safety mechanism: Only suggest if confidence is above minimum threshold
        // This prevents suggesting corrections for completely unrelated words
        if confidence > 60.0 {
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