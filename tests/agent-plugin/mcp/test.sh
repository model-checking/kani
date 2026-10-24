#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Exercise every kani-mcp tool over the MCP stdio transport, and check that the
# kani_verify_harness summary matches summarize-results.py on the same run.

source "$(dirname "$0")/../../../agent-plugin/tests/smoke-lib.sh"
build_mcp
CLIENT="$PLUGIN/tests/mcp_client.py"

"$CLIENT" "$KANI_MCP" \
    kani_get_version '{}' \
    kani_list_harnesses "{\"path\": \"$FIXTURE\", \"extra_args\": [\"-Z\", \"function-contracts\"]}" \
    kani_verify_harness "{\"path\": \"$FIXTURE\", \"export_path\": \"$WORK/mcp.json\", \"log_path\": \"$WORK/mcp.log\", \"extra_args\": [\"-Z\", \"function-contracts\"]}" \
    kani_get_counterexample "{\"path\": \"$FIXTURE\", \"harness\": \"check_naive_midpoint\", \"extra_args\": [\"-Z\", \"function-contracts\"]}" \
    kani_verify_harness "{\"path\": \"$FIXTURE\", \"harness\": \"check_midpoint\", \"extra_args\": [\"--no-such-flag\"]}" \
    kani_verify_harness "{\"path\": \"$FIXTURE\", \"harness\": \"check_midpoint\", \"timeout_s\": 0, \"extra_args\": [\"-Z\", \"function-contracts\"]}" \
    kani_list_harnesses "{\"path\": \"$FIXTURE\", \"filter\": \"midpoint\", \"extra_args\": [\"-Z\", \"function-contracts\"]}" \
    > "$WORK/mcp.jsonl" || fail "MCP session failed"

[ ! -e "$FIXTURE/kani-list.json" ] || fail "kani_list_harnesses left kani-list.json behind"

# Same run through the CLI, for comparison with kani_verify_harness.
(cd "$FIXTURE" && cargo kani -Z export-json --export-json "$WORK/cli.json" -Z function-contracts > /dev/null 2>&1) || true
"$SCRIPTS/summarize-results.py" "$WORK/cli.json" > "$WORK/cli-summary.json"

python3 - "$WORK/mcp.jsonl" "$WORK/cli-summary.json" <<'PY'
import json, sys
version, listed, verified, cex, bad_flag, timed_out, filtered = [json.loads(l) for l in open(sys.argv[1])]
cli = json.load(open(sys.argv[2]))

assert not version["is_error"] and version["result"]["compatible"], version
assert version["result"]["toolchain"]["works"], version
print("kani_get_version: compatible", version["result"]["installed"])

assert not listed["is_error"], listed
assert listed["result"]["totals"]["standard-harnesses"] == 3, listed
assert listed["result"]["totals"]["contract-harnesses"] == 2, listed
assert "standard-harnesses" not in listed["result"], "listing should be a summary by default"
print("kani_list_harnesses: 3 harnesses")

assert not verified["is_error"], verified
r = verified["result"]
assert r["total"] == 5 and r["failed"] == 1 and r["exit_code"] != 0, r
assert r["run"]["export_path"].endswith("mcp.json") and r["run"]["log_path"].endswith("mcp.log"), r
assert r["run"]["command"].startswith("cargo kani -Z export-json"), r
r = {k: v for k, v in r.items() if k != "run"}

def strip_times(summary):
    return [{k: v for k, v in h.items() if k != "time_s"} for h in summary["harnesses"]]

assert strip_times(r) == strip_times(cli), (r, cli)
for key in ("kani", "run_state", "total", "successful", "failed"):
    assert r[key] == cli[key], key
print("kani_verify_harness: matches summarize-results.py")

assert not cex["is_error"], cex
c = cex["result"]
assert [h["name"].split("::")[-1] for h in c["harnesses"]] == ["check_naive_midpoint"], c
assert c["failure_witnesses"] and "concrete_playback_run" in c["failure_witnesses"][0], c
assert all("Check for `cover`" not in t for t in c["failure_witnesses"]), c
print("kani_get_counterexample: returns a concrete playback test")

assert bad_flag["is_error"] and "no-such-flag" in bad_flag["result"]["error"], bad_flag
print("kani_verify_harness: reports invalid arguments as a tool error")

assert timed_out["is_error"] and "timeout_s" in timed_out["result"]["error"], timed_out
print("kani_verify_harness: stops at timeout_s")

assert filtered["result"]["matching_standard_harnesses"] == [
    "proofs::check_midpoint", "proofs::check_naive_midpoint"], filtered
print("kani_list_harnesses: filter")
PY
smoke_pass
