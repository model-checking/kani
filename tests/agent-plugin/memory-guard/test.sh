#!/usr/bin/env bash
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

# A harness that needs more memory than the cap is killed alone: the calling shell keeps running,
# the export says `killed: memory` with the peak, and the MCP tool reports the same.

source "$(dirname "$0")/../../../agent-plugin/tests/smoke-lib.sh"

cp "$PLUGIN/tests/fixtures/memory-hog/heavy.rs" "$WORK/heavy.rs"
cd "$WORK"
FLAGS="$(PYTHONPATH="$SCRIPTS" python3 -c "
from kani_plugin_lib import export_args
print(' '.join(export_args('$WORK/hog.json')))")"
set +e
# shellcheck disable=SC2086
"$SCRIPTS/guarded-run.py" --memory-limit-mb 1500 --timeout-s 900 --export "$WORK/hog.json" --harness heavy \
    -- kani heavy.rs $FLAGS > "$WORK/hog.log" 2>&1
status=$?
set -e
echo "guarded-run.py exit status: $status (this shell is still running)"
[ "$status" -eq 137 ] || fail "expected the memory kill (137), got $status: $(tail -5 "$WORK/hog.log")"
grep -q "guarded-run: killed: memory" "$WORK/hog.log" || fail "no kill message: $(tail -3 "$WORK/hog.log")"

"$SCRIPTS/summarize-results.py" "$WORK/hog.json" > "$WORK/hog-summary.json"
python3 - "$WORK/hog-summary.json" <<'PY' || fail "the summary does not report the memory kill"
import json, sys
d = json.load(open(sys.argv[1]))
assert d["run_state"] == "KILLED", d
assert d["by_verdict"] == {"OUT_OF_MEMORY": 1} and not d["all_ok"], d
[k] = d["killed"]
assert k["harness"] == "heavy" and k["reason"] == "memory" and k["message"].startswith("killed: memory (peak "), k
assert d["run"]["peak_memory_mb"] > 1000 and d["run"]["limit_mb"] == 1500, d["run"]
PY
echo "summarize-results.py reports: $(python3 -c "import json;print(json.load(open('$WORK/hog-summary.json'))['killed'][0]['message'])")"

build_mcp
"$PLUGIN/tests/mcp_client.py" "$KANI_MCP" kani_verify_harness \
    "{\"path\": \"$WORK/heavy.rs\", \"memory_limit_mb\": 1500, \"timeout_s\": 900, \"export_path\": \"$WORK/mcp-hog.json\"}" \
    > "$WORK/mcp-hog-result.json"
python3 - "$WORK/mcp-hog-result.json" <<'PY' || fail "kani_verify_harness does not report the memory kill"
import json, sys
r = json.load(open(sys.argv[1]))
assert r["is_error"] and "killed: memory (peak " in r["result"]["error"], r
PY
python3 - "$WORK/mcp-hog.json" <<'PY' || fail "the MCP tool wrote no stopped export"
import json, sys
d = json.load(open(sys.argv[1]))
assert d["run_state"] == "KILLED" and d["plugin_run"]["killed"] == "memory", d
PY
echo "kani_verify_harness: reports the kill and writes an export with run_state KILLED"

smoke_pass
