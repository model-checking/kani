#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Record the evidence behind a reported Kani result, and check it later.

Every claim in a verification report should cite the run that established it. `record` saves
a bundle: the `--export-json` results, the exact command and directory, the Kani/CBMC/rustc
versions, the git state (commit, a hash of `git diff HEAD`, and the untracked files with their
hashes), and a SHA-256 of every source file of the package (`.rs` files, `Cargo.toml`,
`build.rs`) and of the workspace's `Cargo.toml` and `Cargo.lock`. `verify` checks that the
bundle is intact, reports every source that changed since, and re-runs the recorded command to
confirm every harness gets the same verdict.

Usage:
    evidence.py record --export results.json --command "cargo kani ... --export-json results.json"
                       [--cwd DIR] [--out DIR]
    evidence.py verify BUNDLE_DIR [--no-rerun]

The default output directory is `target/kani-evidence` in a Cargo project (already ignored by
git in most projects), and `.kani-evidence` otherwise. Exit status: 0 on success, 2 when
`verify` finds a difference, 1 on input errors.
"""

import argparse
import hashlib
import importlib.util
import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS))
from kani_plugin_lib import find_root  # noqa: E402
_spec = importlib.util.spec_from_file_location("summarize_results", SCRIPTS / "summarize-results.py")
summarize_results = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(summarize_results)


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


SOURCE_NAMES = {"Cargo.toml", "Cargo.lock", "build.rs", "rust-toolchain", "rust-toolchain.toml"}
SKIPPED_DIRS = {".git", "target", "node_modules"}


def source_files(cwd, doc):
    """The files a run depends on: every source of the package, the workspace manifests, and
    the harness files named in the export (whose paths may be relative to the workspace)."""
    root, package = find_root(cwd)
    files = set()
    if package is not None:
        for dirpath, dirnames, filenames in os.walk(package):
            dirnames[:] = [d for d in dirnames if d not in SKIPPED_DIRS and not d.startswith("target")]
            for name in filenames:
                if name.endswith(".rs") or name in SOURCE_NAMES:
                    files.add(Path(dirpath) / name)
        for name in ("Cargo.toml", "Cargo.lock"):
            if (root / name).exists():
                files.add(root / name)
    bases = [cwd, *cwd.parents]
    for h in doc.get("harnesses", []):
        found = next((b / h["file"] for b in bases if (b / h["file"]).exists()), None)
        if found:
            files.add(found)
    return sorted(f.resolve() for f in files)


def git_state(cwd):
    def git(*args, text=True):
        out = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=text)
        if out.returncode != 0:
            return None
        return out.stdout.strip() if text else out.stdout
    commit = git("rev-parse", "HEAD")
    if commit is None:
        return None
    top = Path(git("rev-parse", "--show-toplevel"))
    untracked = {}
    for rel in (git("ls-files", "--others", "--exclude-standard") or "").splitlines():
        f = top / rel
        if f.is_file():
            untracked[rel] = sha256(f)
    return {
        "commit": commit,
        "dirty": bool(git("status", "--porcelain", "--untracked-files=no")),
        "diff_sha256": hashlib.sha256(git("diff", "HEAD", "--binary", text=False) or b"").hexdigest(),
        "untracked_sha256": untracked,
    }


def verdicts(doc):
    out = {}
    for h in doc.get("harnesses", []):
        o = h.get("outcome", {})
        out[h["name"]] = o.get("verdict") if o.get("kind") == "COMPLETED" else o.get("kind")
    return out


def record(args):
    cwd = Path(args.cwd).resolve()
    export = Path(args.export)
    if not export.is_absolute():
        export = cwd / export
    doc = json.loads(export.read_text(encoding="utf-8"))
    out_root = Path(args.out) if args.out else cwd / ("target/kani-evidence" if (cwd / "Cargo.toml").exists()
                                                      else ".kani-evidence")
    digest = sha256(export)
    bundle = out_root / f"{time.strftime('%Y%m%dT%H%M%SZ', time.gmtime())}-{digest[:8]}"
    bundle.mkdir(parents=True, exist_ok=True)
    shutil.copy2(export, bundle / "results.json")
    sources = {os.path.relpath(f, cwd): sha256(f) for f in source_files(cwd, doc)}
    manifest = {
        "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "command": shlex.split(args.command),
        "cwd": str(cwd),
        "git": git_state(cwd),
        "tools": doc.get("tools", {}),
        "results_sha256": digest,
        "sources_sha256": sources,
        "verdicts": verdicts(doc),
        "weakened_checks": summarize_results.weakened_checks(doc),
    }
    (bundle / "bundle.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"bundle": str(bundle), "verdicts": manifest["verdicts"], "git": manifest["git"]},
                     indent=2, sort_keys=True))
    return 0


def verify(args):
    bundle = Path(args.bundle)
    manifest = json.loads((bundle / "bundle.json").read_text(encoding="utf-8"))
    cwd = Path(manifest["cwd"])
    differences = []
    if sha256(bundle / "results.json") != manifest["results_sha256"]:
        differences.append("results.json does not match the recorded hash: the bundle was modified")
    for rel, digest in manifest["sources_sha256"].items():
        f = cwd / rel
        if not f.exists():
            differences.append(f"source {rel} no longer exists")
        elif sha256(f) != digest:
            differences.append(f"source {rel} changed since the run")
    current = {os.path.relpath(f, cwd) for f in source_files(cwd, {})}
    for rel in sorted(current - set(manifest["sources_sha256"])):
        if rel.endswith(".rs"):
            differences.append(f"source {rel} was added since the run")
    git = git_state(cwd)
    recorded = manifest["git"]
    if recorded and git:
        if git["commit"] != recorded["commit"]:
            differences.append(f"git HEAD is {git['commit']}, the run was at {recorded['commit']}")
        elif git.get("diff_sha256") != recorded.get("diff_sha256"):
            differences.append("uncommitted changes differ from those at the time of the run")
        for rel, digest in recorded.get("untracked_sha256", {}).items():
            if git["untracked_sha256"].get(rel) != digest:
                differences.append(f"untracked file {rel} changed or disappeared since the run")
    rerun = None
    if not args.no_rerun:
        cmd = list(manifest["command"])
        with tempfile.TemporaryDirectory(prefix="kani-evidence-") as tmp:
            new_export = str(Path(tmp) / "results.json")
            if "--export-json" in cmd:
                cmd[cmd.index("--export-json") + 1] = new_export
            else:
                cmd += ["-Z", "export-json", "--export-json", new_export]
            subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
            if Path(new_export).exists():
                rerun = verdicts(json.loads(Path(new_export).read_text(encoding="utf-8")))
            else:
                differences.append("the re-run produced no results")
        if rerun is not None:
            for name in sorted(set(rerun) | set(manifest["verdicts"])):
                before, after = manifest["verdicts"].get(name), rerun.get(name)
                if before != after:
                    differences.append(f"harness {name}: recorded {before}, re-run {after}")
    print(json.dumps({"bundle": str(bundle), "consistent": not differences, "differences": differences,
                      "rerun_verdicts": rerun}, indent=2, sort_keys=True))
    return 0 if not differences else 2


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="action", required=True)
    rec = sub.add_parser("record", help="save an evidence bundle for a Kani run")
    rec.add_argument("--export", required=True, help="the `--export-json` file of the run")
    rec.add_argument("--command", required=True, help="the exact command that produced it")
    rec.add_argument("--cwd", default=".", help="directory the command ran in (default: current)")
    rec.add_argument("--out", help="directory to store bundles in")
    ver = sub.add_parser("verify", help="check a bundle and re-run its command")
    ver.add_argument("bundle", help="bundle directory printed by `record`")
    ver.add_argument("--no-rerun", action="store_true", help="only check hashes and git state")
    args = parser.parse_args(argv)
    try:
        return record(args) if args.action == "record" else verify(args)
    except (OSError, json.JSONDecodeError, KeyError) as e:
        print(f"error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
