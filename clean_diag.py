import re

with open('crates/vinglish-own/src/diagnostics.rs', 'r') as f:
    content = f.read()

# Remove the println!
content = content.replace('println!("name={}, to_name={}", name, to_name); let mut clean_name = name.clone();', 'let mut clean_name = name.clone();')

# Update ownership transferred note
old_to_note = '    msg.push_str(&format!("  = note: Ownership transferred to `{}`.\\n", clean_to_name));'
new_to_note = '''    if clean_to_name.starts_with("_tmp") {
        msg.push_str("  = note: Ownership was transferred to another value or function call.\\n");
    } else {
        msg.push_str(&format!("  = note: Ownership transferred to `{}`.\\n", clean_to_name));
    }'''

content = content.replace(old_to_note, new_to_note)

with open('crates/vinglish-own/src/diagnostics.rs', 'w') as f:
    f.write(content)
