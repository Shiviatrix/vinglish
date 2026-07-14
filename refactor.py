import os
import glob
import re

files_to_check = []
for root, _, files in os.walk('crates'):
    for file in files:
        if file.endswith('.rs'):
            files_to_check.append(os.path.join(root, file))

for filepath in files_to_check:
    with open(filepath, 'r') as f:
        content = f.read()

    original = content
    # Replace block.instrs with block.stmts
    content = content.replace('block.instrs', 'block.stmts')
    
    # We also need to map iterators that yield instr into something that yields stmt,
    # or just rename `instr` to `stmt` and do `match &stmt.instr`.
    # Let's see if we can do this safely via simple string replaces or if it needs manual tweaks.

    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"Updated {filepath}")
