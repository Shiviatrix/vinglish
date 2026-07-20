import os

files_to_check = []
for root, _, files in os.walk('crates'):
    for file in files:
        if file.endswith('.rs'):
            files_to_check.append(os.path.join(root, file))

for filepath in files_to_check:
    with open(filepath, 'r') as f:
        content = f.read()

    original = content
    content = content.replace('for instr in &block.stmts', 'for stmt in &block.stmts')
    content = content.replace('for instr in &mut block.stmts', 'for stmt in &mut block.stmts')
    content = content.replace('match instr {', 'match &stmt.instr {')
    content = content.replace('match *instr {', 'match stmt.instr {')
    
    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)
        print(f"Updated {filepath}")
