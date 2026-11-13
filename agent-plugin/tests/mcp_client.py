#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Minimal MCP stdio client for testing `kani-mcp`.

Performs the MCP handshake, then calls each tool given on the command line and prints one
JSON object per call: {"tool", "is_error", "result"}.

Usage:
    mcp_client.py SERVER_BINARY 'TOOL_NAME' '{"arg": "value"}' ['TOOL_NAME' '{...}' ...]
"""

import json
import subprocess
import sys


def main(argv):
    if len(argv) < 3 or len(argv) % 2 == 0:
        print(__doc__, file=sys.stderr)
        return 1
    server, calls = argv[0], list(zip(argv[1::2], argv[2::2]))
    proc = subprocess.Popen([server], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)

    def request(id_, method, params):
        proc.stdin.write(json.dumps({"jsonrpc": "2.0", "id": id_, "method": method, "params": params}) + "\n")
        proc.stdin.flush()
        response = json.loads(proc.stdout.readline())
        if response.get("id") != id_ or "error" in response:
            raise SystemExit(f"unexpected response to {method}: {response}")
        return response["result"]

    init = request(0, "initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                                     "clientInfo": {"name": "mcp_client.py", "version": "0"}})
    assert init["serverInfo"]["name"] == "kani-mcp", init
    proc.stdin.write(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}) + "\n")
    tools = {t["name"] for t in request(1, "tools/list", {})["tools"]}
    for i, (tool, args) in enumerate(calls, start=2):
        if tool not in tools:
            raise SystemExit(f"server does not advertise tool `{tool}`; it has {sorted(tools)}")
        result = request(i, "tools/call", {"name": tool, "arguments": json.loads(args)})
        print(json.dumps({"tool": tool, "is_error": result["isError"], "result": result["structuredContent"]}))
    proc.stdin.close()
    return proc.wait()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
