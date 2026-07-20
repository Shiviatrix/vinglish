import re
import os

def fix_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    original = content

    # Fix: block.stmts[x] = Instruction::... -> block.stmts[x].instr = Instruction::...
    content = re.sub(r'(block\.stmts\[[^\]]+\])\s*=\s*(Instruction::[a-zA-Z]+)', r'\1.instr = \2', content)

    # Fix: match &stmt.instr where the original was taking ownership:
    # Actually, in most optimization passes like constant_folding, they construct a new instruction.
    # In `gvn.rs`:
    # let (dest, expr) = match &stmt.instr {
    #     Instruction::Assign(dest, op) => ...
    # That creates a mismatch if it expects a MirStmt.
    # Let's revert match &stmt.instr to match stmt.instr in places where it complains about expected MirStmt.
    
    if content != original:
        with open(filepath, 'w') as f:
            f.write(content)

for root, _, files in os.walk('crates'):
    for file in files:
        if file.endswith('.rs'):
            fix_file(os.path.join(root, file))

