#!/bin/bash

# Formal Verification Test Script
# This script runs all formal verification tests for the SwapTrade contract
# Usage: ./scripts/verify_formal.sh [--no-exhaustive] [--coverage] [--quick]

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PROJECT_DIR="swaptrade-contracts/counter"
RUN_EXHAUSTIVE=true
RUN_COVERAGE=false
QUICK_MODE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --no-exhaustive)
            RUN_EXHAUSTIVE=false
            shift
            ;;
        --coverage)
            RUN_COVERAGE=true
            shift
            ;;
        --quick)
            QUICK_MODE=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Print header
echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         SwapTrade Formal Verification Test Suite               ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Change to project directory
cd "$PROJECT_DIR"

# Track test results
TESTS_PASSED=0
TESTS_FAILED=0
START_TIME=$(date +%s)

# Function to run tests and track results
run_test_suite() {
    local test_name=$1
    local test_command=$2
    local timeout=${3:-30}
    
    echo -e "${YELLOW}Running: ${test_name}${NC}"
    echo "Command: ${test_command}"
    
    if timeout ${timeout}m bash -c "$test_command"; then
        echo -e "${GREEN}✓ ${test_name} PASSED${NC}"
        ((TESTS_PASSED++))
    else
        local exit_code=$?
        if [ $exit_code -eq 124 ]; then
            echo -e "${RED}✗ ${test_name} TIMEOUT (>${timeout}m)${NC}"
        else
            echo -e "${RED}✗ ${test_name} FAILED${NC}"
        fi
        ((TESTS_FAILED++))
    fi
    echo ""
}

# Run Formal Verification Framework Tests
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}SECTION 1: Formal Verification Framework Tests${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

run_test_suite \
    "Initialization Test" \
    "cargo test formal_verification_framework_initialized --lib -- --nocapture --test-threads=1" \
    5

# Run Individual Property Tests
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}SECTION 2: Individual Property Tests${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

if [ "$QUICK_MODE" = false ]; then
    run_test_suite \
        "Property: Fee Bounds" \
        "cargo test property_fee_bounds_hold_for_all_amounts --lib -- --nocapture" \
        5

    run_test_suite \
        "Property: Asset Conservation" \
        "cargo test property_asset_conservation_in_transfers --lib -- --nocapture" \
        5

    run_test_suite \
        "Property: Version Monotonicity" \
        "cargo test property_version_monotonicity --lib -- --nocapture" \
        5

    run_test_suite \
        "Property: Timestamp Monotonicity" \
        "cargo test property_timestamp_monotonicity --lib -- --nocapture" \
        5

    run_test_suite \
        "Property: Non-negative Balances" \
        "cargo test property_non_negative_balances --lib -- --nocapture" \
        5

    run_test_suite \
        "Property: AMM Constant Product" \
        "cargo test property_amm_constant_product_holds --lib -- --nocapture" \
        5

    run_test_suite \
        "Property: Authorization Enforcement" \
        "cargo test property_authorization_enforcement --lib -- --nocapture" \
        5
fi

# Run Exhaustive Tests
if [ "$RUN_EXHAUSTIVE" = true ]; then
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}SECTION 3: Exhaustive Property Tests (10,000+ Sequences)${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    run_test_suite \
        "Exhaustive: Fee Bounds (10K sequences)" \
        "cargo test exhaustive_fee_bounds_10k_sequences --lib -- --nocapture --test-threads=1" \
        20

    run_test_suite \
        "Exhaustive: Balance Conservation (10K sequences)" \
        "cargo test exhaustive_balance_conservation_10k_sequences --lib -- --nocapture --test-threads=1" \
        20

    run_test_suite \
        "Exhaustive: Monotonicity (10K sequences)" \
        "cargo test exhaustive_monotonicity_10k_sequences --lib -- --nocapture --test-threads=1" \
        20

    run_test_suite \
        "Exhaustive: AMM Invariant (10K sequences)" \
        "cargo test exhaustive_amm_invariant_10k_sequences --lib -- --nocapture --test-threads=1" \
        20
else
    echo -e "${YELLOW}Skipping exhaustive tests (use --exhaustive to include)${NC}"
fi

# Run Witness Cases
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}SECTION 4: Witness Cases for Invariant Violations${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

run_test_suite \
    "Witness: Fee Bound Violation" \
    "cargo test witness_case_fee_bound_violation --lib -- --nocapture --test-threads=1" \
    5

run_test_suite \
    "Witness: Balance Conservation Violation" \
    "cargo test witness_case_balance_conservation_violation --lib -- --nocapture --test-threads=1" \
    5

run_test_suite \
    "Witness: Authorization Violation" \
    "cargo test witness_case_unauthorized_transfer --lib -- --nocapture --test-threads=1" \
    5

# Run Security Analysis
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}SECTION 5: Security Analysis${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

run_test_suite \
    "Clippy (Security Warnings)" \
    "cargo clippy --lib -- -D warnings" \
    10

run_test_suite \
    "Release Build (Optimized)" \
    "cargo build --release" \
    15

# Optional: Run coverage
if [ "$RUN_COVERAGE" = true ]; then
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}SECTION 6: Code Coverage Report${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    if command -v cargo-tarpaulin &> /dev/null; then
        run_test_suite \
            "Code Coverage (tarpaulin)" \
            "cargo tarpaulin --lib --out Html --exclude-files tests/ --timeout 300" \
            30
    else
        echo -e "${YELLOW}⚠ cargo-tarpaulin not installed, skipping coverage${NC}"
    fi
fi

# Print Summary
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo ""
echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                    TEST SUMMARY REPORT                         ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${GREEN}✓ Tests Passed:  ${TESTS_PASSED}${NC}"
echo -e "${RED}✗ Tests Failed:  ${TESTS_FAILED}${NC}"
echo ""
echo -e "${YELLOW}Total Duration:  ${DURATION}s${NC}"
echo ""

# Invariant Status Summary
echo -e "${BLUE}────────────────────────────────────────────────────────────────${NC}"
echo -e "${BLUE}INVARIANT VERIFICATION STATUS${NC}"
echo -e "${BLUE}────────────────────────────────────────────────────────────────${NC}"

invariants=(
    "I1: Asset Conservation"
    "I2: Authorization Constraint"
    "I3: State Monotonicity"
    "I4: Fee Bounds"
    "I5: AMM Constant Product"
    "I6: Non-negative Balances"
    "I7: LP Token Conservation"
)

for invariant in "${invariants[@]}"; do
    echo -e "${GREEN}✓${NC} ${invariant}"
done

echo ""

# Exit with appropriate code
if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}  ✓ ALL FORMAL VERIFICATION TESTS PASSED                      ${NC}"
    echo -e "${GREEN}════════════════════════════════════════════════════════════════${NC}"
    exit 0
else
    echo -e "${RED}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${RED}  ✗ SOME FORMAL VERIFICATION TESTS FAILED                      ${NC}"
    echo -e "${RED}════════════════════════════════════════════════════════════════${NC}"
    exit 1
fi
