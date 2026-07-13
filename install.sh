#!/bin/bash
set -e

echo "========================================="
echo "   Vinglish Compiler Installation Script  "
echo "========================================="

# Check for prerequisites
if ! command -v git &> /dev/null; then
    echo "Error: git is not installed."
    exit 1
fi
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo (Rust) is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

ENGLIST_ROOT="$HOME/.vinglish"

# Create a temporary directory for cloning if we aren't already in the repo
WORK_DIR=$(pwd)
CLONED=false

if [ ! -f "Cargo.toml" ] || ! grep -q "eng-cli" "Cargo.toml"; then
    echo "Downloading Vinglish source code..."
    WORK_DIR=$(mktemp -d)
    git clone --quiet https://github.com/Shiviatrix/vinglish.git "$WORK_DIR"
    cd "$WORK_DIR"
    CLONED=true
fi

# 1. Build the compiler in release mode
echo "[1/4] Building Vinglish (cargo build --release)..."
cargo build --release

# 2. Create the global directory
echo "[2/4] Setting up global Vinglish directory (~/.vinglish)..."
mkdir -p "$ENGLIST_ROOT"

# 3. Copy standard library and runtime
echo "[3/4] Copying standard library and runtime..."
# Remove old ones if they exist to ensure clean update
rm -rf "$ENGLIST_ROOT/std" "$ENGLIST_ROOT/rt"
cp -r std "$ENGLIST_ROOT/"
cp -r rt "$ENGLIST_ROOT/"

# 4. Copy the binary to cargo's bin directory
echo "[4/4] Installing binary to ~/.cargo/bin..."
mkdir -p "$HOME/.cargo/bin"
cp target/release/eng "$HOME/.cargo/bin/eng"

# Cleanup if we cloned it
if [ "$CLONED" = true ]; then
    cd "$HOME"
    rm -rf "$WORK_DIR"
fi

echo "========================================="
echo "Installation Successful! 🎉"
echo ""
echo "To use Vinglish from any directory, you MUST set the ENGLIST_ROOT environment variable."
echo "Add the following line to your shell profile (e.g., ~/.bashrc, ~/.zshrc, or ~/.profile):"
echo ""
echo "    export ENGLIST_ROOT=\"$HOME/.vinglish\""
echo ""
echo "After adding it, restart your terminal or run: source ~/.zshrc"
echo "========================================="
