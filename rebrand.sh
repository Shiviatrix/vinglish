#!/bin/bash
set -e

echo "Starting rebrand..."

# 1. Bulk replace vinglish -> vinglish and Vinglish -> Vinglish
find . -type f \( -name "*.rs" -o -name "*.md" -o -name "*.toml" -o -name "*.html" -o -name "*.c" -o -name "*.h" -o -name "*.sh" -o -name "*.yml" -o -name "*.json" \) \
  -not -path "*/target/*" -not -path "*/.git/*" \
  -exec sed -i '' -e 's/vinglish/vinglish/g' -e 's/Vinglish/Vinglish/g' {} +

# 2. Bulk replace eng_ -> ving_ and ENG_ -> VING_
find . -type f \( -name "*.rs" -o -name "*.c" -o -name "*.h" \) \
  -not -path "*/target/*" -not -path "*/.git/*" \
  -exec sed -i '' -e 's/eng_/ving_/g' -e 's/ENG_/VING_/g' {} +

# 3. Rename .eng files to .ving
find . -name "*.eng" -type f -not -path "*/target/*" -not -path "*/.git/*" | while read f; do
  mv "$f" "${f%.eng}.ving"
done

# 4. Rename crates
for dir in crates/eng-*; do
  if [ -d "$dir" ]; then
    new_dir="crates/vinglish-${dir#crates/eng-}"
    mv "$dir" "$new_dir"
  fi
done

# 5. Fix Cargo.tomls package names
find crates -name "Cargo.toml" -type f -exec sed -i '' -e 's/name = "eng-/name = "vinglish-/g' {} +
sed -i '' -e 's/"crates\/eng-/"crates\/vinglish-/g' Cargo.toml

# 6. Change eng-cli binary to vng
sed -i '' -e 's/name = "eng"/name = "vng"/g' crates/vinglish-cli/Cargo.toml

# 7. Update eng_modules to ving_modules
find . -type f -name "*.rs" -not -path "*/target/*" -not -path "*/.git/*" \
  -exec sed -i '' -e 's/eng_modules/ving_modules/g' {} +
sed -i '' -e 's/eng_modules/ving_modules/g' .gitignore

# 8. Rename studio directory

# 9. Clean and build
cargo clean
cargo check --workspace

echo "Rebrand completed."
