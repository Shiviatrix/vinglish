#!/bin/bash
export ENGLIST_ROOT=$(pwd)

cat << 'EOF' > test_intent_1.ving
function main() -> number
begin
    return 0
end
EOF

cat << 'EOF' > test_intent_2.ving
function main() returns number
begin
    while 1 > 0 do
        return 1
    end
    return 0
end
EOF

cat << 'EOF' > test_intent_3.ving
function main() returns number
begin
    number x = 5
    if x = 5 then
        return 1
    end
    return 0
end
EOF

cat << 'EOF' > test_intent_4.ving
function main(x: number) returns number
begin
    return x
end
EOF

echo "--- Test 1 (-> vs returns) ---"
cargo run --bin vng -- check test_intent_1.ving
echo ""

echo "--- Test 2 (while vs repeat while) ---"
cargo run --bin vng -- check test_intent_2.ving
echo ""

echo "--- Test 3 (= vs == in if) ---"
cargo run --bin vng -- check test_intent_3.ving
echo ""

echo "--- Test 4 (x: number vs number x) ---"
cargo run --bin vng -- check test_intent_4.ving
echo ""
