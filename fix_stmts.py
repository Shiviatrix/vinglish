import os
import re

for root, _, files in os.walk('crates'):
    for file in files:
        if file.endswith('.rs'):
            filepath = os.path.join(root, file)
            with open(filepath, 'r') as f:
                content = f.read()

            original = content
            
            # The issue is that the loop variable was renamed to `stmt`, but `instr` was still used inside.
            # So I will find `for stmt in &block.stmts` and change it back to `for instr in &block.stmts`,
            # and then I will just change `match instr` to `match &instr.instr`
            # Wait, `instr.instr` is weird but it works! (The MirStmt variable is named `instr`, and its field is `instr`).
            
            content = content.replace('for stmt in &block.stmts', 'for instr in &block.stmts')
            content = content.replace('for stmt in &mut block.stmts', 'for instr in &mut block.stmts')
            
            # Now we have `instr: MirStmt`.
            # We must replace `match &stmt.instr` with `match &instr.instr`.
            content = content.replace('match &stmt.instr', 'match &instr.instr')
            content = content.replace('match stmt.instr', 'match instr.instr')
            
            # Any remaining `match instr {` or `match &instr {` should become `match instr.instr` or `match &instr.instr`
            content = re.sub(r'match\s+instr\s*\{', 'match instr.instr {', content)
            content = re.sub(r'match\s+&instr\s*\{', 'match &instr.instr {', content)
            
            # Also if we do `new_instrs.push(instr.clone())`, it remains `instr.clone()` (clones the MirStmt).
            
            if content != original:
                with open(filepath, 'w') as f:
                    f.write(content)
                print(f"Fixed {filepath}")
