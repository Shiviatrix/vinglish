#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

cargo test --workspace --quiet
cargo run --quiet --bin vng -- check examples/basics/hello.ving
cargo run --quiet --bin vng -- build examples/basics/hello.ving --output /tmp/vinglish-public-check

test -x /tmp/vinglish-public-check
/tmp/vinglish-public-check > /tmp/vinglish-public-check.out

grep -q "Hello from Vinglish!" /tmp/vinglish-public-check.out

echo "Public validation passed."
