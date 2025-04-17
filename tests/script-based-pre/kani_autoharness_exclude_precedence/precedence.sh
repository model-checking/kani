# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Test that --include-pattern and --exclude-pattern work as expected when provided together.

set -e

# Define all function paths
FUNCTIONS=(
    "foo::foo_function"
    "foo::bar::bar_function"
    "foo::bar::foo_bar_function"
    "foo::baz::foo_baz_function"
    "other::regular_function"
    "other::with_bar_name"
    "foo_top_level"
    "bar_top_level"
)

# Check if a function appears in the "Selected Function" table
check_selected() {
    local output="$1"
    local function_name="$2"
    if echo "$output" | grep -q "| $function_name *|"; then
        return 0
    else
        return 1
    fi
}

# Check if a function appears in the "Skipped Function" table
check_skipped() {
    local output="$1"
    local function_name="$2"
    if echo "$output" | grep -q "| $function_name *|.*Did not match provided filters"; then
        return 0
    else
        return 1
    fi
}

# Check that the warning message gets printed for the patterns that are mutually exclusive (no functions get selected)
check_warning() {
    local output="$1"
    local should_warn="$2"
    local warning_present=$(echo "$output" | grep -c "warning: Include pattern" || true)
    
    if [ "$should_warn" = true ] && [ "$warning_present" -eq 0 ]; then
        echo "ERROR: expected printed warning about conflicting --include-pattern and --exclude-pattern flags"
        return 1
    elif [ "$should_warn" = false ] && [ "$warning_present" -gt 0 ]; then
        echo "ERROR: Got unexpected warning message"
        return 1
    fi
    return 0
}


# Helper function to verify functions against include/exclude patterns
verify_functions() {
    local output="$1"
    local include_pattern="$2"
    local exclude_pattern="$3"
    
    for func in "${FUNCTIONS[@]}"; do
        # If the function name matches the include pattern and not the exclude pattern, it should be selected
        if echo "$func" | grep -q "$include_pattern" && ! echo "$func" | grep -q "$exclude_pattern"; then
            if ! check_selected "$output" "$func"; then
                echo "ERROR: Expected $func to be selected"
                exit 1
            fi
        # Otherwise, it should be skipped
        else
            if ! check_skipped "$output" "$func"; then
                echo "ERROR: Expected $func to be skipped"
                exit 1
            fi
        fi
    done
}

# Test cases
test_cases=(
    "include 'foo' exclude 'foo::bar'"
    "include 'foo' exclude 'bar'"
    "include 'foo::bar' exclude 'bar'"
    "include 'foo' exclude 'foo'"
)

include_patterns=(
    "foo"
    "foo"
    "foo::bar"
    "foo"
)

exclude_patterns=(
    "foo::bar"
    "bar"
    "bar"
    "foo"
)

# Whether each test case should produce a warning about no functions being selected
should_warn=(
    false
    false
    true
    true
)

for i in "${!test_cases[@]}"; do
    echo "Testing: ${test_cases[$i]}"
    output=$(kani autoharness -Z autoharness src/lib.rs --include-pattern "${include_patterns[$i]}" --exclude-pattern "${exclude_patterns[$i]}" --only-codegen)
    echo "$output"

    if ! check_warning "$output" "${should_warn[$i]}"; then
        exit 1
    fi
    
    verify_functions "$output" "${include_patterns[$i]}" "${exclude_patterns[$i]}"
done

echo "All tests passed!"