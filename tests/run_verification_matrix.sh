#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "=========================================="
echo " Vinglish Semantic Verification Matrix"
echo "=========================================="

FAILURES=0

for file in tests/verify_*.ving; do
    if [ ! -f "$file" ]; then
        echo "No verify_*.ving files found."
        exit 0
    fi
    
    basename=$(basename "$file" .ving)
    echo -n "Testing $basename... "

    # Check for syntax/type errors first
    ./target/debug/vng check "$file" > tests/temp_check.out 2>&1
    if [ $? -ne 0 ]; then
        echo -e "${RED}CHECK FAILED${NC}"
        cat tests/temp_check.out
        FAILURES=$((FAILURES + 1))
        continue
    fi

    # Run Interpreter
    ./target/debug/vng run "$file" > tests/temp_interp.out 2> tests/temp_interp.err
    if [ $? -ne 0 ]; then
        if [[ "$basename" == *"oob"* ]]; then
            echo -e "${GREEN}INTERPRETER FAILED (EXPECTED) ${NC}"
            continue
        else
            echo -e "${RED}INTERPRETER FAILED${NC}"
            cat tests/temp_interp.err
            FAILURES=$((FAILURES + 1))
            continue
        fi
    fi

    # Run C Backend
    ./target/debug/vng build --backend c --output tests/temp_c_bin "$file" > tests/temp_build.out 2>&1
    if [ $? -ne 0 ]; then
        echo -e "${RED}C BUILD FAILED${NC}"
        cat tests/temp_build.out
        FAILURES=$((FAILURES + 1))
        continue
    fi

    ./tests/temp_c_bin > tests/temp_c.out 2> tests/temp_c.err
    if [ $? -ne 0 ]; then
        if [[ "$basename" == *"oob"* ]]; then
            echo -e "${GREEN}C EXECUTION FAILED (EXPECTED) ${NC}"
        else
            echo -e "${RED}C EXECUTION FAILED${NC}"
            cat tests/temp_c.err
            FAILURES=$((FAILURES + 1))
            continue
        fi
    fi

    # Compare Outputs
    diff tests/temp_interp.out tests/temp_c.out > tests/temp_diff.out
    if [ $? -ne 0 ]; then
        echo -e "${RED}SEMANTIC MISMATCH${NC}"
        echo "Interpreter Output:"
        cat tests/temp_interp.out
        echo "C Backend Output:"
        cat tests/temp_c.out
        echo "Diff:"
        cat tests/temp_diff.out
        FAILURES=$((FAILURES + 1))
        continue
    fi

    echo -e "${GREEN}PASS${NC}"
done

echo "=========================================="
if [ $FAILURES -eq 0 ]; then
    echo -e "${GREEN}All verified features match!${NC}"
    exit 0
else
    echo -e "${RED}$FAILURES features failed verification!${NC}"
    exit 1
fi
