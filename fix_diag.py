import re

with open('crates/vinglish-own/src/diagnostics.rs', 'r') as f:
    content = f.read()

clean_orig = """    let clean_name = name.split('_').next().unwrap_or(&name).to_string();
    let display_name = if clean_name.starts_with("tmp") { "temporary value".to_string() } else { format!("`{}`", clean_name) };"""

clean_new = """    // Remove trailing _NUMBER
    let mut clean_name = name.clone();
    if let Some(pos) = name.rfind('_') {
        if name[pos + 1..].chars().all(|c| c.is_ascii_digit()) {
            clean_name = name[..pos].to_string();
        }
    }
    let display_name = if clean_name.starts_with("_tmp") || clean_name.starts_with("tmp") {
        "temporary value".to_string()
    } else {
        format!("`{}`", clean_name)
    };"""

content = content.replace(clean_orig, clean_new)

with open('crates/vinglish-own/src/diagnostics.rs', 'w') as f:
    f.write(content)

