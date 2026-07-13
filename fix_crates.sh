#!/bin/bash
set -e

crates=("cli" "lexer" "parser" "types" "ownership" "diagnostics" "codegen" "fmt" "hir" "mir" "opt" "own" "ssa" "analysis" "llvm" "lsp")

for cr in "${crates[@]}"; do
    find crates -name "*.rs" -type f -exec sed -i '' -e "s/ving_${cr}/vinglish_${cr}/g" {} +
done

cargo check --workspace
