#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Guards the codegen cache's efficacy. The cache is a pure performance optimization, so nothing
# else in the suite notices when it stops working — a change elsewhere could reduce it to a
# fully-missing cache and every other test would still pass, just slower.
#
# Asserts a floor on the hit rate rather than an exact figure: the point is to catch the cache
# being broken outright, not to pin numbers that legitimately drift as codegen changes.
#
# Requires a debug build of kani-compiler, since the instrumented cache (and therefore
# `CACHE_STATS`) is compiled out when `debug_assertions` is off.

set -euo pipefail

# The cache reaches >98% on large crates; a small fixture like this one sits lower because a
# larger share of its queries are each type's first. Keep the floor well under what we observe so
# codegen churn does not make this flaky, while still failing loudly if caching stops happening.
MIN_HIT_RATE=70
# Every configured cache must be doing something. A row that silently stops being consulted would
# otherwise hide behind a healthy total.
EXPECTED_CACHES=2

output=$(CACHE_STATS=1 kani fixture.rs 2>&1)

if ! grep -q '\*\*\*CACHE STATS\*\*\*' <<<"$output"; then
    echo "FAIL: no cache statistics were printed."
    echo "The instrumented cache is only compiled in when debug_assertions is on;"
    echo "run this against a debug build (cargo build-dev), not a release one."
    exit 1
fi

# `TOTAL: <hits> hits / <queries> queries (<rate>%)`
total_line=$(grep -m1 '^TOTAL:' <<<"$output")
hits=$(sed -E 's/^TOTAL: ([0-9]+) hits.*/\1/' <<<"$total_line")
queries=$(sed -E 's@^TOTAL: [0-9]+ hits / ([0-9]+) queries.*@\1@' <<<"$total_line")

if [ -z "$hits" ] || [ -z "$queries" ] || [ "$queries" -eq 0 ]; then
    echo "FAIL: could not parse cache totals from: $total_line"
    exit 1
fi

# Integer arithmetic keeps the comparison free of locale-dependent decimal parsing.
hit_rate=$(( hits * 100 / queries ))
echo "queries recorded: $([ "$queries" -gt 0 ] && echo yes || echo no)"
if [ "$hit_rate" -ge "$MIN_HIT_RATE" ]; then
    echo "hit rate at or above ${MIN_HIT_RATE}%: yes"
else
    echo "hit rate at or above ${MIN_HIT_RATE}%: no (measured ${hit_rate}%)"
    echo "$output"
    exit 1
fi

# Each per-type line looks like `<type>:  <hits> hits / <queries> queries (<rate>%)...`.
caches_with_hits=0
while read -r line; do
    cache_hits=$(sed -E 's/^.*:[[:space:]]+([0-9]+) hits.*/\1/' <<<"$line")
    if [ -n "$cache_hits" ] && [ "$cache_hits" -gt 0 ]; then
        caches_with_hits=$(( caches_with_hits + 1 ))
    fi
done < <(grep -E '^[a-z_]+.*:[[:space:]]+[0-9]+ hits' <<<"$output")

if [ "$caches_with_hits" -eq "$EXPECTED_CACHES" ]; then
    echo "all ${EXPECTED_CACHES} caches serving hits: yes"
else
    echo "all ${EXPECTED_CACHES} caches serving hits: no (${caches_with_hits} were)"
    echo "$output"
    exit 1
fi
