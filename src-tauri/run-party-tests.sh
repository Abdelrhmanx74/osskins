#!/bin/bash
# Party Mode Test Runner with formatted output

echo -e "\n\033[36m=== Running Party Mode Tests ===\033[0m"
echo -e "\033[33mPlease wait...\033[0m\n"

# Run tests and capture output
OUTPUT=$(cargo test --lib -- party_mode 2>&1)

# Parse test results
PASSED=$(echo "$OUTPUT" | grep -E "^test .* \.\.\. ok$" | wc -l)
FAILED=$(echo "$OUTPUT" | grep -E "^test .* \.\.\. FAILED$" | wc -l)
TOTAL=$((PASSED + FAILED))

if [ $TOTAL -gt 0 ]; then
    PASS_RATE=$(echo "scale=1; ($PASSED * 100) / $TOTAL" | bc)
else
    PASS_RATE=0
fi

# Display summary
echo -e "\n\033[1m╔════════════════════════════════════════════════════════════╗\033[0m"
echo -e "\033[1m║           PARTY MODE TEST SUMMARY                          ║\033[0m"
echo -e "\033[1m╚════════════════════════════════════════════════════════════╝\033[0m"

echo -e "\n\033[36mTotal Tests: $TOTAL\033[0m"
echo -e "\033[32m✓ Passed: $PASSED ($PASS_RATE%)\033[0m"
echo -e "\033[31m✗ Failed: $FAILED\033[0m"

# Show failed tests with details
if [ $FAILED -gt 0 ]; then
    echo -e "\n\033[1;31m╔════════════════════════════════════════════════════════════╗\033[0m"
    echo -e "\033[1;31m║           FAILED TESTS DETAILS                             ║\033[0m"
    echo -e "\033[1;31m╚════════════════════════════════════════════════════════════╝\033[0m"
    
    # Extract failed test names
    FAILED_TESTS=$(echo "$OUTPUT" | grep -E "^test .* \.\.\. FAILED$" | sed 's/^test //' | sed 's/ \.\.\. FAILED$//')
    
    INDEX=1
    while IFS= read -r TEST; do
        if [ -n "$TEST" ]; then
            TEST_NAME=$(echo "$TEST" | awk -F'::' '{print $NF}')
            MODULE=$(echo "$TEST" | sed 's/::[^:]*$//')
            
            echo -e "\n\033[33m[$INDEX/$FAILED]\033[0m \033[31m$TEST_NAME\033[0m"
            echo -e "  \033[90mModule: $MODULE\033[0m"
            
            # Try to extract failure details
            SECTION=$(echo "$OUTPUT" | sed -n "/---- $TEST stdout ----/,/^----/p")
            
            if echo "$SECTION" | grep -q "left:"; then
                ACTUAL=$(echo "$SECTION" | grep "left:" | head -1 | sed 's/.*left: //')
                echo -e "    \033[33mActual:   \033[0m$ACTUAL"
            fi
            
            if echo "$SECTION" | grep -q "right:"; then
                EXPECTED=$(echo "$SECTION" | grep "right:" | head -1 | sed 's/.*right: //')
                echo -e "    \033[33mExpected: \033[0m$EXPECTED"
            fi
            
            if echo "$SECTION" | grep -q "assertion"; then
                ERROR=$(echo "$SECTION" | grep "assertion" | head -1 | xargs)
                echo -e "    \033[31mError: $ERROR\033[0m"
            fi
            
            INDEX=$((INDEX + 1))
        fi
    done <<< "$FAILED_TESTS"
fi

# Show test categories summary
echo -e "\n\033[36m╔════════════════════════════════════════════════════════════╗\033[0m"
echo -e "\033[36m║           TEST CATEGORIES                                   ║\033[0m"
echo -e "\033[36m╚════════════════════════════════════════════════════════════╝\033[0m"

declare -A CATEGORIES=(
    ["Helper Tests"]="test_helpers"
    ["Timing Edge Cases"]="test_timing_edge_cases"
    ["ARAM Mode"]="test_aram_mode"
    ["Swift Play"]="test_swift_play"
    ["Session State"]="test_session_state"
    ["Party Detection"]="test_party_detection"
    ["Injection Logic"]="test_injection_logic"
    ["Race Conditions"]="test_race_conditions"
)

for CATEGORY in "${!CATEGORIES[@]}"; do
    PATTERN="${CATEGORIES[$CATEGORY]}"
    CAT_PASSED=$(echo "$OUTPUT" | grep -E "^test .*${PATTERN}.* \.\.\. ok$" | wc -l)
    CAT_FAILED=$(echo "$OUTPUT" | grep -E "^test .*${PATTERN}.* \.\.\. FAILED$" | wc -l)
    CAT_TOTAL=$((CAT_PASSED + CAT_FAILED))
    
    if [ $CAT_TOTAL -gt 0 ]; then
        if [ $CAT_FAILED -eq 0 ]; then
            echo -e "\033[32m✓ $CATEGORY: $CAT_PASSED/$CAT_TOTAL passed\033[0m"
        else
            echo -e "\033[33m✗ $CATEGORY: $CAT_PASSED/$CAT_TOTAL passed\033[0m"
        fi
    fi
done | sort

echo ""

# Return exit code
if [ $FAILED -gt 0 ]; then
    exit 1
else
    exit 0
fi
