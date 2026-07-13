#!/bin/bash
set -e

echo "========================================="
echo "   Englist Compiler Installation Script  "
echo "========================================="

# 1. Build the compiler in release mode
echo "[1/4] Building Englist (cargo build --release)..."
cargo build --release

# 2. Create the global directory
echo "[2/4] Setting up global Englist directory (~/.englist)..."
ENGLIST_ROOT="$HOME/.englist"
mkdir -p "$ENGLIST_ROOT"

# 3. Copy standard library and runtime
echo "[3/4] Copying standard library and runtime..."
cp -r std "$ENGLIST_ROOT/"
cp -r rt "$ENGLIST_ROOT/"

# 4. Copy the binary to cargo's bin directory
echo "[4/4] Installing binary to ~/.cargo/bin..."
mkdir -p "$HOME/.cargo/bin"
cp target/release/eng "$HOME/.cargo/bin/eng"

echo "========================================="
echo "Installation Successful! 🎉"
echo ""
echo "To use Englist from any directory, you MUST set the ENGLIST_ROOT environment variable."
echo "Add the following line to your shell profile (e.g., ~/.bashrc, ~/.zshrc, or ~/.profile):"
echo ""
echo "    export ENGLIST_ROOT=\"$HOME/.englist\""
echo ""
echo "After adding it, restart your terminal or run: source ~/.zshrc"
echo "========================================="
