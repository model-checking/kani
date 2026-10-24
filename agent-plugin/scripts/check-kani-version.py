#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Check that the installed Kani is supported by this plugin release.

Reads the supported range from `compatibility.json` next to this plugin, runs `cargo kani
--version` (falling back to `kani --version`), and reports whether the installed version is
in range. Then it verifies a trivial harness, because `--version` answers even when the
verification toolchain (CBMC, goto-cc) is missing. Run this before relying on any Kani
feature the skill describes.

Usage:
    check-kani-version.py [--kani-cmd "cargo kani"] [--version-string "kani 0.68.0"]
                          [--no-toolchain-check]

Exit status: 0 when a supported, working Kani is installed, 1 otherwise.
"""

import argparse
import json
import re
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path

COMPATIBILITY = Path(__file__).resolve().parent.parent / "compatibility.json"
INSTALL_HINT = "cargo install --locked kani-verifier && cargo kani setup"


def parse_version(text):
    m = re.search(r"(\d+)\.(\d+)\.(\d+)", text)
    return tuple(int(x) for x in m.groups()) if m else None


def fmt(v):
    return ".".join(map(str, v))


def installed_version(cmds):
    for cmd in cmds:
        try:
            out = subprocess.run(shlex.split(cmd) + ["--version"], capture_output=True, text=True, timeout=60)
        except (OSError, subprocess.TimeoutExpired):
            continue
        if out.returncode == 0 and parse_version(out.stdout):
            return cmd, out.stdout.strip()
    return None, None


def check(version_text, compat, cmd=None):
    lo = parse_version(compat["min_kani"])
    hi = parse_version(compat["max_kani_exclusive"])
    result = {
        "command": cmd,
        "plugin_version": compat["plugin_version"],
        "supported_range": f">= {fmt(lo)}, < {fmt(hi)}",
    }
    version = parse_version(version_text or "")
    if version is None:
        result.update(compatible=False, installed=None,
                      message=f"Kani was not found. Install it with `{INSTALL_HINT}`.")
    elif version < lo:
        result.update(compatible=False, installed=fmt(version),
                      message=f"Kani {fmt(version)} is older than the minimum supported version "
                      f"{fmt(lo)} for this plugin. Upgrade with `{INSTALL_HINT}`.")
    elif version >= hi:
        result.update(compatible=False, installed=fmt(version),
                      message=f"Kani {fmt(version)} is newer than this plugin supports (< {fmt(hi)}). "
                      "Update the plugin, or install a supported Kani release.")
    else:
        result.update(compatible=True, installed=fmt(version),
                      message=f"Kani {fmt(version)} is supported.")
    return result


def toolchain_check(timeout=600):
    """Verify a trivial harness with `kani`. Returns (works, output tail)."""
    with tempfile.TemporaryDirectory(prefix="kani-check-") as tmp:
        Path(tmp, "check.rs").write_text("#[kani::proof]\nfn kani_plugin_toolchain_check() {}\n")
        try:
            out = subprocess.run(["kani", "check.rs"], cwd=tmp, capture_output=True, text=True, timeout=timeout)
        except (OSError, subprocess.TimeoutExpired) as e:
            return False, str(e)
        text = out.stdout + out.stderr
        return out.returncode == 0 and "VERIFICATION:- SUCCESSFUL" in text, "\n".join(text.splitlines()[-20:])


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--kani-cmd", action="append", help="command used to run Kani (repeatable)")
    parser.add_argument("--version-string", help="check this version text instead of running Kani")
    parser.add_argument("--no-toolchain-check", action="store_true",
                        help="only check the version, do not verify a trivial harness")
    args = parser.parse_args(argv)
    compat = json.loads(COMPATIBILITY.read_text(encoding="utf-8"))
    if args.version_string is not None:
        cmd, text = None, args.version_string
    else:
        cmd, text = installed_version(args.kani_cmd or ["cargo kani", "kani"])
    result = check(text, compat, cmd)
    if result["compatible"] and args.version_string is None and not args.no_toolchain_check:
        works, output = toolchain_check()
        result["toolchain_works"] = works
        if not works:
            result["compatible"] = False
            result["message"] = (f"Kani {result['installed']} is supported, but it cannot verify a trivial harness. "
                                 "If the output mentions goto-cc or cbmc, run `cargo kani setup` (or, for a "
                                 "development build, put CBMC on PATH).")
            result["output_tail"] = output
    print(json.dumps(result, indent=2, sort_keys=True))
    if not result["compatible"]:
        print(f"error: {result['message']}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
