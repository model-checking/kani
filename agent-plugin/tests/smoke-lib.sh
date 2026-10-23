# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Shared setup for the agent plugin smoke tests in `tests/agent-plugin/`. Source it from a
# test script; it defines:
#   KANI_ROOT, PLUGIN    repository root and plugin directory
#   SCRIPTS              the plugin's helper scripts
#   WORK                 a scratch directory, removed on exit
#   FIXTURE              a fresh copy of the fixture crate inside $WORK
#   build_mcp            builds kani-mcp and sets $KANI_MCP to its path
#   fail MSG             prints MSG and exits 1

set -eu

KANI_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PLUGIN="$KANI_ROOT/agent-plugin"
SCRIPTS="$PLUGIN/scripts"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cp -R "$PLUGIN/tests/fixtures/crate" "$WORK/crate"
FIXTURE="$WORK/crate"

fail() {
    echo "FAIL: $*"
    exit 1
}

build_mcp() {
    KANI_MCP="$(cargo build --quiet --manifest-path "$KANI_ROOT/Cargo.toml" -p kani-mcp \
        --message-format=json | python3 -c '
import json, sys
for line in sys.stdin:
    msg = json.loads(line)
    if msg.get("reason") == "compiler-artifact" and msg.get("executable"):
        print(msg["executable"])
')"
    [ -x "$KANI_MCP" ] || fail "could not build kani-mcp"
}

# Print the JSON field at a Python expression `$2` of file `$1`.
jq_py() {
    python3 -c "import json,sys; d=json.load(open(sys.argv[1])); print($2)" "$1"
}
