#!/bin/bash
set -e

echo "Building vng..."
cargo build

VNG="$(pwd)/target/debug/vng"

TEST_DIR="tests/tmp_fix_test"
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

echo "====================================="
echo "1. Integration test for git-clean safety"
echo "====================================="

git init . > /dev/null
git config user.email "test@example.com"
git config user.name "Test User"
cat << 'CODE' > src.ving
public function main() returns text
begin
    return "number: " + 1
end
CODE
git add src.ving
git commit -m "init" > /dev/null

# Make tree dirty
cat << 'CODE' > src.ving
public function main() returns text
begin
    return "number: " + 1 + 2
end
CODE

echo "Running vng fix without --allow-dirty on a dirty tree..."
if $VNG fix src.ving --yes 2>&1 | grep -q "Git working tree is dirty"; then
    echo "[PASS] vng fix correctly blocked by dirty tree."
else
    echo "[FAIL] vng fix did not block dirty tree."
    exit 1
fi

echo "====================================="
echo "2. Test for multiple fix application (span-offset order)"
echo "====================================="

# Clean up
git reset --hard HEAD > /dev/null

# Create file with multiple errors on the same line
cat << 'CODE' > multiple.ving
public function main() returns text
begin
    return "number: " + 1 + 2
end
CODE

git add multiple.ving
git commit -m "add multiple" > /dev/null

echo "Running vng fix --yes on multiple.ving..."
$VNG fix multiple.ving --yes

echo "Contents after fix:"
cat multiple.ving

if grep -q '"number: " + to_text(1) + to_text(2)' multiple.ving; then
    echo "[PASS] multiple.ving was fixed correctly!"
else
    echo "[FAIL] multiple.ving was not fixed correctly."
    exit 1
fi

echo "====================================="
echo "3. Validation of vng build --deny-heal"
echo "====================================="

# Create a file with a type mismatch
cat << 'CODE' > deny.ving
public function main() returns text
begin
    return 42
end
CODE

git add deny.ving
git commit -m "add deny" > /dev/null

echo "Running vng build --deny-heal deny.ving..."
if $VNG build deny.ving --deny-heal 2>&1 | grep -q "auto-fixable"; then
    echo "[FAIL] --deny-heal should NOT suggest fixes."
    exit 1
else
    echo "[PASS] --deny-heal did not suggest fixes."
fi

if $VNG build deny.ving --heal 2>&1 | grep -q "auto-fixable"; then
    echo "[FAIL] --heal should not just suggest, it should apply."
    exit 1
else
    echo "[PASS] --heal applied and didn't just suggest."
fi

# test normal build suggests
if $VNG check deny.ving 2>&1 | grep -q "auto-fixable"; then
    echo "[PASS] Normal check suggests fixes."
else
    echo "[FAIL] Normal check did not suggest fixes."
    exit 1
fi

echo "ALL TESTS PASSED."
