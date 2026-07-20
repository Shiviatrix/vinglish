#!/bin/bash
set -e

# Update dependencies in Cargo.toml files inside crates
find crates -name "Cargo.toml" -type f -exec sed -i '' -e 's/eng-/vinglish-/g' {} +

# Test it
cargo check --workspace
