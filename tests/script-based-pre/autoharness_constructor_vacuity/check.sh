#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

set -eu

# The logs go outside the source tree, so a failing run leaves nothing behind.
log_dir=$(mktemp -d)
trap 'rm -rf "$log_dir"' EXIT

# Without --constructor-args, a harness gets no new property.
kani autoharness -Z autoharness --include-pattern boom_q --output-format=regular vacuous.rs \
    > "$log_dir/no_flag.log" 2>&1 || true
grep -q '| m::Q::boom_q *| #\[kani::proof\] *| Failure' "$log_dir/no_flag.log"
if grep -q 'autoharness: a generated input reaches' "$log_dir/no_flag.log"; then exit 2; fi

# Without assertion reachability checks, the cover before a call that no input reaches is
# UNSATISFIABLE rather than UNREACHABLE; the harness must be reported as vacuous all the same.
kani autoharness -Z autoharness --constructor-args --no-assertion-reach-checks \
    --include-pattern boom_q --output-format=terse vacuous.rs > "$log_dir/no_reach.log" 2>&1 || true
grep -q 'Vacuous harness: no generated input reaches `vacuous::m::Q::boom_q`' \
    "$log_dir/no_reach.log"
grep -q '| m::Q::boom_q *| #\[kani::proof\] (ctor) *| Failure' "$log_dir/no_reach.log"

# SARIF has one error result for each vacuous harness and none for a harness that reaches its
# function.
kani autoharness -Z autoharness --constructor-args --sarif "$log_dir/out.sarif" \
    --include-pattern boom --include-pattern R::get --include-pattern opt vacuous.rs \
    > "$log_dir/sarif.log" 2>&1 || true
[ "$(grep -c '"level": "error"' "$log_dir/out.sarif")" = 2 ]
[ "$(grep -c '"harness": ' "$log_dir/out.sarif")" = 2 ]
for harness in P::boom_p Q::boom_q; do
    grep -qF "[m::$harness] Vacuous harness: no generated input reaches \`vacuous::m::$harness\`" \
        "$log_dir/out.sarif"
done

kani autoharness -Z autoharness --constructor-args --output-format=regular vacuous.rs
