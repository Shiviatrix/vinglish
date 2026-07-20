#!/bin/bash
set -e

sed -i '' -e 's/"eng"/"ving"/g' crates/vinglish-cli/src/main.rs
# But wait! I added `"eng" || ext == "c" ...` in cmd_benchmark! I should change `"eng"` to `"ving"` there too!
# Let's just sed it all, but let's review:

cargo check --workspace
