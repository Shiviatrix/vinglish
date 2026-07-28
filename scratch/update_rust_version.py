import os
import re

def update_cargo_toml(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # We want to add or update rust-version in [package] or [workspace.package]
    
    # First, handle [workspace.package]
    if '[workspace.package]' in content:
        if re.search(r'^rust-version\s*=', content, re.MULTILINE):
            content = re.sub(r'^rust-version\s*=.*', 'rust-version = "1.97.1"', content, flags=re.MULTILINE)
        else:
            content = content.replace('[workspace.package]', '[workspace.package]\nrust-version = "1.97.1"')
    
    # Next, handle [package]
    elif '[package]' in content:
        if re.search(r'^rust-version\s*=', content, re.MULTILINE):
            content = re.sub(r'^rust-version\s*=.*', 'rust-version = "1.97.1"', content, flags=re.MULTILINE)
        else:
            # Check if rust-version.workspace is used
            if re.search(r'^rust-version\.workspace\s*=', content, re.MULTILINE):
                content = re.sub(r'^rust-version\.workspace\s*=.*', 'rust-version = "1.97.1"', content, flags=re.MULTILINE)
            else:
                content = content.replace('[package]', '[package]\nrust-version = "1.97.1"')

    with open(filepath, 'w') as f:
        f.write(content)

if __name__ == '__main__':
    for root, dirs, files in os.walk('.'):
        for file in files:
            if file == 'Cargo.toml':
                filepath = os.path.join(root, file)
                update_cargo_toml(filepath)
                print(f"Updated {filepath}")
