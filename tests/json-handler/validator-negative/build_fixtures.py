#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""
Builds fixture JSON documents for `scripts/validate_json_export.py`'s negative tests.

Starting point is the schema template itself, with its schema-only metadata keys
(`_comment`, `_optional`, `_nullable`) stripped out recursively: what remains is a document
built entirely from the template's own values, which validates cleanly against the template
(each fixture's "valid" baseline). Each malformed fixture then applies one targeted mutation --
each mutation is a case the old, leaf-type-blind validator accepted vacuously.
"""

import copy
import json
import sys


def strip_meta(node):
    if isinstance(node, dict):
        return {k: strip_meta(v) for k, v in node.items() if not k.startswith("_")}
    if isinstance(node, list):
        return [strip_meta(item) for item in node]
    return node


def write(out_dir, name, doc):
    with open(f"{out_dir}/{name}", "w") as f:
        json.dump(doc, f, indent=2)


def main():
    if len(sys.argv) != 3:
        print("usage: build_fixtures.py <schema_template.json> <output_dir>")
        sys.exit(1)
    schema_path, out_dir = sys.argv[1], sys.argv[2]

    with open(schema_path) as f:
        schema = json.load(f)

    valid = strip_meta(schema)
    write(out_dir, "valid_export.json", valid)

    # A negative count: well-typed (a real JSON integer), but meaningless. The
    # `"failed": -100`-style mutation from the issue report.
    negative_count = copy.deepcopy(valid)
    negative_count["verification_results"]["summary"]["failed"] = -1
    write(out_dir, "malformed_negative_count.json", negative_count)

    # A count reported as the wrong JSON type entirely.
    wrong_type = copy.deepcopy(valid)
    wrong_type["verification_results"]["summary"]["successful"] = "yes"
    write(out_dir, "malformed_wrong_type.json", wrong_type)

    # `executed` disagreeing with the actual number of entries in `results` -- well-typed and
    # individually non-negative, only wrong once the two are reconciled against each other.
    count_mismatch = copy.deepcopy(valid)
    count_mismatch["verification_results"]["summary"]["executed"] = 99
    write(out_dir, "malformed_count_mismatch.json", count_mismatch)

    # `run_state: "complete"` claiming a clean finish while `results` is empty -- exactly the
    # shape a zero-match or killed-before-verifying run must never be mistaken for. The rest of
    # the summary is kept internally consistent (0 executed, 0 successful/failed) so this fixture
    # isolates the `run_state == "complete"` invariant specifically.
    complete_with_empty_results = copy.deepcopy(valid)
    complete_with_empty_results["run_state"] = "complete"
    complete_with_empty_results["verification_results"]["results"] = []
    summary = complete_with_empty_results["verification_results"]["summary"]
    summary["executed"] = 0
    summary["successful"] = 0
    summary["failed"] = 0
    write(out_dir, "malformed_complete_with_empty_results.json", complete_with_empty_results)

    # The nulls that `--export-json` legitimately emits on degraded runs -- these must
    # *validate*, not be rejected. Each mirrors a real degraded case the schema's `_nullable`
    # markers were added for.

    # A harness that timed out or was OOM-killed before its properties were ever counted: the
    # exporter reports the counts as `null`, not `0`, so a degraded run is never mistaken for
    # "nothing failed".
    nullable_timeout = copy.deepcopy(valid)
    counts = nullable_timeout["property_details"][0]["property_details"]
    for field in (
        "total_properties",
        "passed",
        "failed",
        "unreachable",
        "undetermined",
        "solver_error",
        "satisfied",
        "unsatisfiable",
        "covered",
        "uncovered",
    ):
        counts[field] = None
    counts["error"] = "harness timed out before properties were measured"
    write(out_dir, "nullable_timeout.json", nullable_timeout)

    # An `--smt2` run names no solver and has no notion of object bits.
    nullable_smt2 = copy.deepcopy(valid)
    configuration = nullable_smt2["cbmc"][0]["configuration"]
    configuration["solver"] = None
    configuration["object_bits"] = None
    write(out_dir, "nullable_smt2.json", nullable_smt2)

    # Partial `cbmc_stats`: some statistics are unavailable (e.g. the run never reached the
    # decision procedure), while the rest of the stats are still reported normally.
    nullable_partial_stats = copy.deepcopy(valid)
    cbmc_stats = nullable_partial_stats["cbmc"][0]["cbmc_stats"]
    cbmc_stats["runtime_solver_s"] = None
    cbmc_stats["vccs_remaining"] = None
    cbmc_stats["runtime_decision_procedure_s"] = None
    write(out_dir, "nullable_partial_stats.json", nullable_partial_stats)

    # Failed version/info probes: the tool ran, but querying its version failed.
    nullable_missing_versions = copy.deepcopy(valid)
    tools = nullable_missing_versions["tools"]
    tools["rustc"] = None
    tools["cbmc"] = None
    tools["goto_cc"] = None
    tools["goto_instrument"] = None
    cbmc_metadata = nullable_missing_versions["cbmc"][0]["cbmc_metadata"]
    cbmc_metadata["version"] = None
    cbmc_metadata["os_info"] = None
    write(out_dir, "nullable_missing_versions.json", nullable_missing_versions)

    # A negative fixture proving `_nullable` is targeted, not blanket: a `null` in a field the
    # schema does *not* mark nullable must still be rejected.
    null_in_required_field = copy.deepcopy(valid)
    null_in_required_field["verification_results"]["summary"]["executed"] = None
    write(out_dir, "null_in_required_field.json", null_in_required_field)


if __name__ == "__main__":
    main()
