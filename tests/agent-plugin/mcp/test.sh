#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# Exercise every kani-mcp tool over the MCP stdio transport, and check that the
# kani_verify_harness summary matches summarize-results.py on the same run.

source "$(dirname "$0")/../../../agent-plugin/tests/smoke-lib.sh"
build_mcp
CLIENT="$PLUGIN/tests/mcp_client.py"
export KANI_PLUGIN_DIR="$PLUGIN"

"$CLIENT" "$KANI_MCP" \
    kani_get_version '{}' \
    kani_list_harnesses "{\"path\": \"$FIXTURE\", \"extra_args\": [\"-Z\", \"function-contracts\"]}" \
    kani_verify_harness "{\"path\": \"$FIXTURE\", \"export_path\": \"$WORK/mcp.json\", \"log_path\": \"$WORK/mcp.log\", \"extra_args\": [\"-Z\", \"function-contracts\"]}" \
    kani_get_counterexample "{\"path\": \"$FIXTURE\", \"harness\": \"check_naive_midpoint\", \"extra_args\": [\"-Z\", \"function-contracts\"]}" \
    kani_verify_harness "{\"path\": \"$FIXTURE\", \"harness\": \"check_midpoint\", \"extra_args\": [\"--no-such-flag\"]}" \
    kani_verify_harness "{\"path\": \"$FIXTURE\", \"harness\": \"check_midpoint\", \"timeout_s\": 0, \"extra_args\": [\"-Z\", \"function-contracts\"]}" \
    kani_list_harnesses "{\"path\": \"$FIXTURE\", \"filter\": \"midpoint\", \"extra_args\": [\"-Z\", \"function-contracts\"]}" \
    kani_autoharness "{\"path\": \"$FIXTURE\", \"list_only\": true, \"filter\": \"midpoint\"}" \
    kani_autoharness "{\"path\": \"$FIXTURE\", \"include_patterns\": [\"agent_plugin_fixture::midpoint\"], \"export_path\": \"$WORK/auto.json\"}" \
    kani_autoharness "{\"path\": \"$FIXTURE\", \"include_patterns\": [\"Mul<Foo> for Bar\"]}" \
    kani_verify_harness "{\"path\": \"$FIXTURE\", \"per_harness\": true, \"harness\": \"check_\", \"timeout_s\": 600, \"per_harness_timeout_s\": 300}" \
    kani_map_crate "{\"path\": \"$FIXTURE\", \"callgraph\": \"text\", \"map_path\": \"$WORK/map.json\"}" \
    kani_plan_verification "{\"map_path\": \"$WORK/map.json\", \"max_items\": 3}" \
    kani_explain_function "{\"map_path\": \"$WORK/map.json\", \"function\": \"naive_midpoint\"}" \
    kani_update_verification_map "{\"map_path\": \"$WORK/map.json\", \"results\": [\"$WORK/mcp.json\"]}" \
    kani_update_verification_map "{\"map_path\": \"$WORK/map.json\", \"results\": [\"$WORK/does-not-exist.json\"]}" \
    > "$WORK/mcp.jsonl" || fail "MCP session failed"

[ ! -e "$FIXTURE/kani-list.json" ] || fail "kani_list_harnesses left kani-list.json behind"

# Same run through the CLI, for comparison with kani_verify_harness.
(cd "$FIXTURE" && cargo kani -Z export-json --export-json "$WORK/cli.json" -Z function-contracts > /dev/null 2>&1) || true
"$SCRIPTS/summarize-results.py" "$WORK/cli.json" > "$WORK/cli-summary.json"

python3 - "$WORK/mcp.jsonl" "$WORK/cli-summary.json" <<'PY'
import json, sys
(version, listed, verified, cex, bad_flag, timed_out, filtered, auto_list, auto_run, auto_space,
 per_harness, mapped, planned, explained, updated, update_missing) = [json.loads(l) for l in open(sys.argv[1])]
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
assert r["run"]["command"].startswith("cargo kani -Z "), r
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

r = auto_list["result"]
assert not auto_list["is_error"] and r["selected"] == ["midpoint", "naive_midpoint"], auto_list
assert r["selected_count"] >= 2 and "skipped_by_reason" in r, auto_list
print("kani_autoharness: list_only reports the selected functions")

assert not auto_run["is_error"], auto_run
r = auto_run["result"]
assert r["automatic_harnesses"] == {"total": 1, "success": 1, "failure": 0, "other": 0}, r
assert r["manual_harnesses"]["total"] == 5 and r["manual_harnesses"]["failure"] == 1, r
assert "kani#5103" in r["note"] and r["run"]["export_path"].endswith("auto.json"), r
print("kani_autoharness: runs, and separates automatic from manual harnesses")

assert auto_space["is_error"] and "contains a space" in auto_space["result"]["error"], auto_space
print("kani_autoharness: rejects patterns with spaces")

r = per_harness["result"]
assert not per_harness["is_error"] and r["run_state"] == "COMPLETE", per_harness
assert r["total"] >= 4 and r["failed"] == 1 and r["not_run"] == [], r
assert r["per_harness"]["harnesses"] == r["total"] and "check_naive_midpoint" in str(r["harnesses"]), r
print("kani_verify_harness: per_harness runs each harness in its own process")

r = mapped["result"]
assert not mapped["is_error"] and r["call_graph"] == "text-scan", mapped
assert r["summary"]["functions"] >= 6 and r["summary"]["harnesses"] >= 5, r["summary"]
assert any(u["flag"] == "approximate-call-graph" for u in r["uncertainty"]), r["uncertainty"]
print("kani_map_crate: builds the map and returns a compact summary")

r = planned["result"]
assert not planned["is_error"] and "Contract-first" in r["approach"], planned
assert sum(len(s["functions"]) for s in r["stages"]) <= 3 and r.get("omitted_functions", 0) > 0, r["stages"]
print("kani_plan_verification: a contract-first plan, capped at max_items")

e = explained["result"][0]
assert not explained["is_error"] and e["complexity"]["status"] == "not_computed", explained
assert e["strategy"]["primary"] in ("triage-failure", "loop-contract", "write-contract-then-autoharness",
                                    "run-existing-harness"), e["strategy"]
print("kani_explain_function: strategy, evidence, and the reserved complexity field")

r = updated["result"]
assert not updated["is_error"], updated
assert r["summary"]["by_status"].get("verified", 0) >= 1 and r["summary"]["by_status"].get("failed", 0) >= 1, r["summary"]
assert update_missing["is_error"] and "does not exist" in update_missing["result"]["error"], update_missing
print("kani_update_verification_map: merges results; a missing results path is a clear error")
PY
[ ! -e "$FIXTURE/kani-list.json" ] || fail "kani_autoharness left kani-list.json behind"
smoke_pass
