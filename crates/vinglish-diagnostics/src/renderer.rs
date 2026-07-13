use crate::diagnostic::{Diagnostic, Severity};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const BLUE: &str = "\x1b[34m";
const GREEN: &str = "\x1b[32m";
const DIM: &str = "\x1b[2m";

fn use_color() -> bool {
    std::env::var("NO_COLOR").is_err() && std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true)
}

/// Render a list of diagnostics to a string, with full ANSI formatting.
pub fn render(diagnostics: &[Diagnostic], filename: &str) -> String {
    let color = use_color();
    let mut out = String::new();

    for diag in diagnostics {
        render_one(diag, filename, color, &mut out);
        out.push('\n');
    }

    out
}

fn render_one(diag: &Diagnostic, filename: &str, color: bool, out: &mut String) {
    let (sev_color, sev_label) = match diag.severity {
        Severity::Error => (RED, "error"),
        Severity::Warning => (YELLOW, "warning"),
        Severity::Hint => (CYAN, "hint"),
    };

    // ── Header: `error[E0001]: message` ──────────────────────────────────────
    if color {
        out.push_str(&format!(
            "{BOLD}{sev_color}{sev_label}[{}]{RESET}{BOLD}: {}{RESET}\n",
            diag.code, diag.message
        ));
    } else {
        out.push_str(&format!("{}[{}]: {}\n", sev_label, diag.code, diag.message));
    }

    // ── Location: `--> file:line:col` ────────────────────────────────────────
    if let (Some(line), Some(col)) = (diag.line_number, diag.col_number) {
        if color {
            out.push_str(&format!(
                "  {BLUE}-->{RESET} {DIM}{}{}:{}{RESET}\n",
                filename,
                format_args!(":{line}"),
                col
            ));
        } else {
            out.push_str(&format!("  --> {}:{}:{}\n", filename, line, col));
        }

        // ── Source snippet ────────────────────────────────────────────────────
        if let Some(source_line) = &diag.source_line {
            let line_str = line.to_string();
            let pad = line_str.len();
            let gutter = if color {
                format!("{BLUE}{:>pad$} |{RESET}", line)
            } else {
                format!("{:>pad$} |", line)
            };
            let empty_gutter = if color {
                format!("{BLUE}{:>pad$} |{RESET}", "")
            } else {
                format!("{:>pad$} |", "")
            };

            out.push_str(&format!("{empty_gutter}\n"));
            out.push_str(&format!("{gutter} {source_line}\n"));

            // Underline the span
            let col_start = (col as usize).saturating_sub(1);
            let span_len = (diag.span.end.saturating_sub(diag.span.start) as usize).max(1);
            let spaces = " ".repeat(col_start);
            let squig = "^".repeat(span_len);

            if color {
                out.push_str(&format!(
                    "{empty_gutter} {spaces}{sev_color}{BOLD}{squig}{RESET}\n"
                ));
            } else {
                out.push_str(&format!("{empty_gutter} {spaces}{squig}\n"));
            }
        }
    }

    // ── Suggestions ───────────────────────────────────────────────────────────
    for sug in &diag.suggestions {
        let conf = sug
            .confidence
            .map(|c| format!(" (confidence: {:.1}%)", c))
            .unwrap_or_default();
        if color {
            out.push_str(&format!(
                "  {GREEN}={RESET} {BOLD}suggestion{RESET}: {}{}\n",
                sug.label, conf
            ));
            if let Some(rep) = &sug.replacement {
                out.push_str(&format!("              `{}`\n", rep));
            }
        } else {
            out.push_str(&format!("  = suggestion: {}{}\n", sug.label, conf));
            if let Some(rep) = &sug.replacement {
                out.push_str(&format!("               `{}`\n", rep));
            }
        }
    }

    // ── Notes ─────────────────────────────────────────────────────────────────
    for note in &diag.notes {
        if color {
            out.push_str(&format!("  {CYAN}={RESET} {BOLD}note{RESET}: {}\n", note));
        } else {
            out.push_str(&format!("  = note: {}\n", note));
        }
    }
}
