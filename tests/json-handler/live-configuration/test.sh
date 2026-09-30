#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Check that non-default configuration and resolved_* values are exported.

set -eu
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
VALIDATOR="$PROJECT_ROOT/scripts/validate_json_export.py"
FIXTURE="$SCRIPT_DIR/fixture.rs"

ARTIFACTS=(
    baseline.json cli_unwind.json cli_solver.json cbmc_args_solver.json
    cbmc_args_smt_override.json bare_smt2.json coverage.json coverage_baseline.json
    check_flags.json default_unwind_vs_attribute.json default_unwind_vs_cli.json
)
trap 'rm -f "${ARTIFACTS[@]}"' EXIT

kani -Z export-json "$FIXTURE" --harness configured_harness --export-json baseline.json
python3 "$VALIDATOR" baseline.json 2>&1 | tail -1

kani -Z export-json "$FIXTURE" --harness configured_harness --unwind 6 --export-json cli_unwind.json
python3 "$VALIDATOR" cli_unwind.json 2>&1 | tail -1

kani -Z export-json "$FIXTURE" --harness configured_harness --solver cadical --export-json cli_solver.json
python3 "$VALIDATOR" cli_solver.json 2>&1 | tail -1

kani -Z export-json -Z unstable-options "$FIXTURE" --harness configured_harness --solver cadical \
    --export-json cbmc_args_solver.json --cbmc-args --sat-solver minisat
python3 "$VALIDATOR" cbmc_args_solver.json 2>&1 | tail -1

kani -Z export-json -Z unstable-options "$FIXTURE" --harness configured_harness --solver cadical \
    --export-json cbmc_args_smt_override.json --cbmc-args --z3
python3 "$VALIDATOR" cbmc_args_smt_override.json 2>&1 | tail -1

kani -Z export-json "$FIXTURE" --harness configured_harness --default-unwind 99 \
    --export-json default_unwind_vs_attribute.json
python3 "$VALIDATOR" default_unwind_vs_attribute.json 2>&1 | tail -1

kani -Z export-json "$FIXTURE" --harness configured_harness --default-unwind 99 --unwind 6 \
    --export-json default_unwind_vs_cli.json
python3 "$VALIDATOR" default_unwind_vs_cli.json 2>&1 | tail -1

kani -Z export-json -Z unstable-options "$FIXTURE" --harness configured_harness \
    --export-json bare_smt2.json --cbmc-args --smt2
python3 "$VALIDATOR" bare_smt2.json 2>&1 | tail -1

kani -Z export-json -Z source-coverage "$FIXTURE" --harness coverage_harness --coverage \
    --export-json coverage.json
python3 "$VALIDATOR" coverage.json 2>&1 | tail -1
kani -Z export-json "$FIXTURE" --harness coverage_harness --export-json coverage_baseline.json
python3 "$VALIDATOR" coverage_baseline.json 2>&1 | tail -1

kani -Z export-json --no-overflow-checks --no-assertion-reach-checks \
    "$FIXTURE" --harness coverage_harness --export-json check_flags.json
python3 "$VALIDATOR" check_flags.json 2>&1 | tail -1

python3 << 'PYEOF'
import json
import sys

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)


def one_harness(path):
    with open(path) as f:
        doc = json.load(f)
    hs = doc["harnesses"]
    if len(hs) != 1:
        failures.append(f"{path}: expected 1 harness, got {len(hs)}")
        return None, doc
    return hs[0], doc

h, doc = one_harness("baseline.json")
if h is not None:
    check(h["resolved_unwind"] == 5,
          f"baseline: resolved_unwind should be 5 (the #[kani::unwind] attribute), got "
          f"{h['resolved_unwind']}")
    check(h["resolved_solver"] == "minisat",
          f"baseline: resolved_solver should be minisat (the #[kani::solver] attribute), got "
          f"{h['resolved_solver']}")
    check(doc["configuration"]["cbmc_args"] == [],
          f"baseline: cbmc_args should be empty, got {doc['configuration']['cbmc_args']}")

h, _ = one_harness("cli_unwind.json")
if h is not None:
    check(h["resolved_unwind"] == 6,
          f"cli_unwind: CLI --unwind 6 should win over the #[kani::unwind(5)] attribute, got "
          f"{h['resolved_unwind']}")

h, _ = one_harness("cli_solver.json")
if h is not None:
    check(h["resolved_solver"] == "cadical",
          f"cli_solver: CLI --solver cadical should win over the #[kani::solver(minisat)] "
          f"attribute, got {h['resolved_solver']}")

h, doc = one_harness("cbmc_args_solver.json")
if h is not None:
    check(h["resolved_solver"] == "cadical",
          f"cbmc_args_solver: CBMC's own --sat-solver handling is first-occurrence-wins, and "
          f"CLI --solver's --sat-solver is always positionally first, so a same-family "
          f"--cbmc-args --sat-solver is a no-op here; expected cadical, got "
          f"{h['resolved_solver']}")
    check(doc["configuration"]["cbmc_args"] == ["--sat-solver", "minisat"],
          f"cbmc_args_solver: cbmc_args should still be recorded verbatim even though it had no "
          f"effect, got {doc['configuration']['cbmc_args']}")

h, doc = one_harness("cbmc_args_smt_override.json")
if h is not None:
    check(h["resolved_solver"] == "z3",
          f"cbmc_args_smt_override: a --cbmc-args SMT-family flag (--z3) does override CLI "
          f"--solver's SAT-family flag, regardless of position, got {h['resolved_solver']}")
    check(doc["configuration"]["cbmc_args"] == ["--z3"],
          f"cbmc_args_smt_override: cbmc_args should be recorded verbatim, got "
          f"{doc['configuration']['cbmc_args']}")

h, _ = one_harness("default_unwind_vs_attribute.json")
if h is not None:
    check(h["resolved_unwind"] == 5,
          f"default_unwind_vs_attribute: the #[kani::unwind(5)] attribute should win over "
          f"--default-unwind 99, got {h['resolved_unwind']}")

h, _ = one_harness("default_unwind_vs_cli.json")
if h is not None:
    check(h["resolved_unwind"] == 6,
          f"default_unwind_vs_cli: CLI --unwind 6 should win over both the attribute and "
          f"--default-unwind 99, got {h['resolved_unwind']}")

h, _ = one_harness("bare_smt2.json")
if h is not None:
    check(h["resolved_solver"] is None,
          f"bare_smt2: bare --smt2 should yield resolved_solver: null, got "
          f"{h['resolved_solver']}")

h, doc = one_harness("coverage.json")
check(doc["configuration"]["coverage_enabled"] is True,
      f"coverage: configuration.coverage_enabled should be true, got "
      f"{doc['configuration']['coverage_enabled']}")
baseline_h, _ = one_harness("coverage_baseline.json")
if h is not None and baseline_h is not None:
    check(h["n_properties"] == baseline_h["n_properties"],
          f"coverage: n_properties should equal the same harness's non-coverage run (code_coverage "
          f"properties excluded), got {h['n_properties']} vs baseline "
          f"{baseline_h['n_properties']}")
    check(h["checks"]["total"] == baseline_h["checks"]["total"],
          f"coverage: checks.total should equal the same harness's non-coverage run, got "
          f"{h['checks']['total']} vs baseline {baseline_h['checks']['total']}")
    check(h["checks"]["total"] > 0,
          f"coverage: checks.total should be > 0 for a harness with real assertions, got "
          f"{h['checks']['total']}")

_, doc = one_harness("check_flags.json")
checks = doc["configuration"]["checks"]
check(checks["overflow"] is False,
      f"check_flags: checks.overflow should be false (--no-overflow-checks), got "
      f"{checks['overflow']}")
check(checks["assertion_reach_checks"] is False,
      f"check_flags: checks.assertion_reach_checks should be false "
      f"(--no-assertion-reach-checks), got {checks['assertion_reach_checks']}")
check(checks["memory_safety"] is True,
      f"check_flags: checks.memory_safety should stay at its default true, got "
      f"{checks['memory_safety']}")

if failures:
    for failure in failures:
        print(f"ERROR: {failure}")
    sys.exit(1)

print(
    "live-configuration: unwind/solver precedence, verbatim cbmc_args, bare --smt2, "
    "--coverage and live check-flag toggles all match RFC 0015"
)
PYEOF

echo "All live-configuration checks passed!"
