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
import os
import re
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from kani_export import ExportError, load_export  # noqa: E402
from kani_plugin_lib import export_args, export_flag_args, kani_env, kani_toolchain  # noqa: E402

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
            out = subprocess.run(shlex.split(cmd) + ["--version"], capture_output=True, text=True, timeout=60,
                                 env=kani_env())
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
    """Verify a trivial harness with `kani`, and read the export that the run writes. Returns
    (works, export_error, output tail): `works` is whether the harness verified; `export_error` is
    None when this plugin can read Kani's `--export-json` document (the harness must be in it, with
    a SUCCESS verdict), otherwise why not."""
    with tempfile.TemporaryDirectory(prefix="kani-check-") as tmp:
        Path(tmp, "check.rs").write_text("#[kani::proof]\nfn kani_plugin_toolchain_check() {}\n")
        export = Path(tmp, "export.json")
        try:
            out = subprocess.run(["kani", "check.rs", *export_args(export)], cwd=tmp, capture_output=True,
                                 text=True, timeout=timeout, env=kani_env())
        except (OSError, subprocess.TimeoutExpired) as e:
            return False, None, str(e)
        text = out.stdout + out.stderr
        works = out.returncode == 0 and "VERIFICATION:- SUCCESSFUL" in text
        tail = "\n".join(text.splitlines()[-20:])
        if not works:
            return False, None, tail
        return True, export_problem(export), tail


def export_problem(export):
    """Why this plugin cannot read the export at `export` (None if it can)."""
    if not export.exists():
        return "Kani wrote no export for a successful run"
    try:
        doc = load_export(export)
    except ExportError as e:
        return str(e)
    harnesses = doc.get("harnesses", [])
    if len(harnesses) != 1 or harnesses[0].get("outcome", {}).get("verdict") != "SUCCESS":
        return (f"the plugin read {len(harnesses)} harness(es) from the export of a run that verified one: "
                "the export has a shape it does not understand")
    return None


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
    toolchain = os.environ.get("RUSTUP_TOOLCHAIN") or kani_toolchain()
    if toolchain:
        # Run `cargo kani` with this environment, so that rustup does not install the toolchain
        # a project pins in its rust-toolchain.toml (Kani does not use it).
        result["cli_env"] = f"RUSTUP_TOOLCHAIN={toolchain} RUSTUP_AUTO_INSTALL=0"
    if result["compatible"] and args.version_string is None:
        result["export_flags"] = " ".join(export_flag_args())
    if result["compatible"] and args.version_string is None and not args.no_toolchain_check:
        works, export_error, output = toolchain_check()
        result["toolchain_works"] = works
        if works:
            result["export_readable"] = export_error is None
        if works and export_error:
            result["compatible"] = False
            result["export_error"] = export_error
            result["message"] = (f"Kani {result['installed']} is supported, but this plugin cannot read its "
                                 f"--export-json output ({export_error}). Update the plugin, or use a Kani "
                                 "build whose export it supports.")
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
