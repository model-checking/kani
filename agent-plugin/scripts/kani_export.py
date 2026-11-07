# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Reads Kani's `--export-json` document, whichever Kani wrote it: the one place that knows
the shapes. Every script that reads an export calls `load_export` (or `normalize_export`) and then
works with the *flat* shape only (the document that Kani builds since the RFC 0015 export, #4941):
top level `harnesses`, `summary`, `run_state`, `started_at`, `tools`, `configuration`.

Two shapes exist:

- flat: Kani since #4941. Has `harnesses[]` and, per run, the `configuration` (which checks were
  disabled) and, per harness, its `stubs`. Returned as it is.
- tree: Kani 0.68.0 and builds before #4941. Top level `metadata`, `project`, `tools`,
  `harness_metadata`, `error_details`, `property_details`, `cbmc`, `verification_results`,
  `coverage`. Converted to the flat shape. What the tree does not record is marked, never
  guessed:
    * the run configuration (`--no-*-checks`, `--cbmc-args`, `--prove-safety-only`):
      `configuration_known` is false, and `configuration` is empty. Weakened checks cannot be
      detected from such an export, and every summary says so.
    * stubs and verified stubs: read from the `#[kani::stub(..)]` / `#[kani::stub_verified(..)]`
      attributes above the harness function in the source (`attributes.stubs_source` is
      `source-scan`, or `unknown` when the source cannot be found). Harnesses that AutoHarness
      generates have no stubs by construction (`autoharness-generated`).
    * a harness listed without a result is `NOT_RUN`.

`tests/test_compat.py` holds the same crate's export in both shapes and checks they agree.
"""

import json
import os
import re
from pathlib import Path

NO_CONFIGURATION_NOTE = (
    "This export does not record the run's configuration, so weakened checks (--no-*-checks, "
    "--cbmc-args, --prove-safety-only) cannot be detected from it. Compare with the command that "
    "produced it, or use a Kani that writes the flat export.")

_VERDICTS = {"success": "SUCCESS", "failure": "FAILURE"}
_KINDS = {"timeout": "TIMEOUT", "outofmemory": "OUT_OF_MEMORY", "oom": "OUT_OF_MEMORY",
          "crashed": "CRASHED", "crash": "CRASHED", "killed": "KILLED"}
_STUB = re.compile(r"#\[\s*kani::stub\s*\(\s*([^,()]+?)\s*,\s*([^()]+?)\s*\)\s*\]")
_VERIFIED_STUB = re.compile(r"#\[\s*kani::stub_verified\s*\(\s*([^()]+?)\s*\)\s*\]")


class ExportError(ValueError):
    """The document is not an export that this plugin can read."""


def shape_of(doc):
    """`flat`, `tree`, or None."""
    if not isinstance(doc, dict):
        return None
    if "harnesses" in doc:
        return "flat"
    if "harness_metadata" in doc and "verification_results" in doc:
        return "tree"
    return None


def load_export(path, workspace=None):
    """Read and normalize the export at `path`. Raises ExportError if it is unreadable or has an
    unknown shape; `workspace` helps to find the sources for stubs (see `normalize_export`)."""
    try:
        with open(path, encoding="utf-8") as f:
            doc = json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        raise ExportError(f"cannot read Kani JSON export `{path}`: {e}") from e
    return normalize_export(doc, workspace=workspace, export_path=path)


def normalize_export(doc, workspace=None, export_path=None):
    """The flat shape of `doc`, with `export_shape` (`flat` or `tree`) and `configuration_known`
    added. Flat documents are returned unchanged apart from those two keys."""
    shape = shape_of(doc)
    if shape is None:
        keys = sorted(doc) if isinstance(doc, dict) else type(doc).__name__
        raise ExportError(
            f"unknown Kani export shape (top-level keys: {keys}). This plugin reads the flat export "
            "(`harnesses`, Kani since #4941) and the tree export (`harness_metadata`, Kani 0.68.0 and older "
            "builds)")
    if shape == "flat":
        doc.setdefault("export_shape", "flat")
        doc.setdefault("configuration_known", "configuration" in doc)
        return doc
    return _from_tree(doc, workspace, export_path)


# ---- Tree -> flat ----------------------------------------------------------------------


def _norm(text):
    return (text or "").replace("_", "").replace("-", "").replace(" ", "").lower()


def _outcome(status):
    s = _norm(status)
    if s in _VERDICTS:
        return {"kind": "COMPLETED", "verdict": _VERDICTS[s]}
    if s in _KINDS:
        return {"kind": _KINDS[s]}
    return {"kind": "COMPLETED", "verdict": (status or "UNKNOWN").upper()}


def _where(check):
    loc = check.get("location") or {}
    line = loc.get("line")
    # The flat export spells line numbers as strings.
    return loc.get("file"), str(line) if str(line).isdigit() else None


def _property_id(check):
    # The flat shape's property ids are `<function>.<class>.<n>`.
    return f"{check.get('function')}.{check.get('category')}.{check.get('id')}"


def stubs_from_source(roots, file, line, name):
    """(stubs, verified stubs, found) from the attributes above the harness function `name`. The
    harness starts at `line` (its `fn` line, or an attribute line above it): scan down to the `fn`,
    then up until the end of the previous item. A candidate file counts only if the `fn` is where
    the export says (a relative path may exist in the wrong directory)."""
    function = re.compile(rf"\bfn\s+{re.escape(name.split('::')[-1])}\b")
    for root in roots:
        path = Path(file) if os.path.isabs(file) else Path(root) / file
        try:
            lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        except OSError:
            continue
        if not 1 <= line <= len(lines):
            continue
        fn_index = next((k for k in range(line - 1, min(len(lines), line + 31)) if function.search(lines[k])), None)
        if fn_index is None:
            continue
        block, i = [], fn_index - 1
        while i >= 0:
            code = re.sub(r"//.*$", "", lines[i]).rstrip()
            if code.endswith((";", "}", "{")) and not code.lstrip().startswith("#"):
                break
            block.append(lines[i])
            i -= 1
        text = "\n".join(reversed(block))
        stubs = [{"original": x, "replacement": y} for x, y in _STUB.findall(text)]
        return stubs, _VERIFIED_STUB.findall(text), True
    return [], [], False


def _kind(raw):
    """The harness kind as the flat export spells it: "Proof", or {"ProofForContract": {...}}."""
    if isinstance(raw, str) and raw.startswith("ProofForContract"):
        m = re.search(r'target_fn:\s*"([^"]*)"', raw)
        return {"ProofForContract": {"target_fn": m.group(1) if m else None}}
    return raw


def _from_tree(doc, workspace, export_path):
    meta = doc.get("metadata", {})
    roots = [r for r in (workspace, doc.get("project", {}).get("workspace_root"),
                         Path(export_path).resolve().parent if export_path else None, os.getcwd()) if r]
    results = {r.get("harness_id"): r for r in doc.get("verification_results", {}).get("results", [])}
    solvers = {c.get("harness_id"): (c.get("configuration") or {}).get("solver") for c in doc.get("cbmc", [])}
    harnesses = []
    for m in doc.get("harness_metadata", []):
        name = m["pretty_name"]
        r = results.get(name)
        checks = (r or {}).get("checks", [])
        plain = [c for c in checks if c.get("category") != "cover"]
        covers = [c for c in checks if c.get("category") == "cover"]

        def ids(items, *statuses):
            return [_property_id(c) for c in items if _norm(c.get("status")) in statuses]

        failed, unsupported = [], []
        for c in plain:
            if _norm(c.get("status")) == "failure":
                file, line = _where(c)
                failed.append({"id": _property_id(c), "description": c.get("description"),
                               "class": c.get("category"), "file": file, "line": line, "status": "FAILURE"})
        for c in checks:
            if "unsupported" in (c.get("category") or "").lower() or _norm(c.get("status")) == "unsupported":
                file, line = _where(c)
                unsupported.append({"description": c.get("description"), "file": file, "line": line})
        if m.get("is_automatically_generated"):
            stubs, verified, stubs_source = [], [], "autoharness-generated"
        else:
            stubs, verified, found = stubs_from_source(roots, m["source"]["file"], m["source"]["start_line"], name)
            stubs_source = "source-scan" if found else "unknown"
        contract = m.get("contract") or {}
        harnesses.append({
            "name": name, "crate_name": m.get("crate_name", ""),
            "file": m["source"]["file"], "line": m["source"]["start_line"],
            "contract": contract if contract.get("contracted_function_name") else None,
            "is_automatically_generated": bool(m.get("is_automatically_generated")),
            "has_loop_contracts": bool(m.get("has_loop_contracts")),
            "is_bounded": bool(m.get("is_bounded")),
            "attributes": {"kind": _kind((m.get("attributes") or {}).get("kind")),
                           "should_panic": bool((m.get("attributes") or {}).get("should_panic")),
                           "solver": None, "unwind_value": None,
                           "stubs": stubs, "verified_stubs": verified, "stubs_source": stubs_source},
            "outcome": _outcome(r.get("status")) if r else {"kind": "NOT_RUN"},
            "resolved_solver": solvers.get(name), "resolved_unwind": None,
            "generated_concrete_test": False,
            "resources": {"verification_time_s": ((r or {}).get("duration_ms") or 0) / 1000.0},
            "n_properties": len(plain), "n_failed": len(failed),
            "failed_properties": failed, "unsupported_constructs": unsupported,
            "warnings": [], "warnings_truncated": 0,
            "checks": {"total": len(plain), "success": len(ids(plain, "success")),
                       "failure": ids(plain, "failure"), "unreachable": ids(plain, "unreachable"),
                       "undetermined": ids(plain, "undetermined"), "error": [], "unknown": [], "other": []},
            "covers": {"total": len(covers), "satisfied": ids(covers, "satisfied"),
                       "unsatisfiable": ids(covers, "unsatisfiable"), "unreachable": ids(covers, "unreachable"),
                       "undetermined": ids(covers, "undetermined"), "error": [], "unknown": [], "other": []},
        })
    verdicts = [h["outcome"].get("verdict") for h in harnesses if h["outcome"]["kind"] == "COMPLETED"]
    completed = (doc.get("verification_results", {}).get("summary", {}).get("status") == "completed"
                 and all(h["outcome"]["kind"] != "NOT_RUN" for h in harnesses))
    timestamp = meta.get("timestamp")
    # The tree-only sections are consumed above; the flat document carries what remains.
    dropped = ("harness_metadata", "verification_results", "property_details", "error_details", "cbmc", "coverage")
    out = {k: v for k, v in doc.items() if k not in dropped}
    out.update({
        "export_shape": "tree",
        "configuration_known": False,
        "configuration": {},
        "export_notes": [NO_CONFIGURATION_NOTE],
        "harnesses": harnesses,
        "summary": {"total": len(harnesses), "successful": verdicts.count("SUCCESS"),
                    "failed": verdicts.count("FAILURE")},
        "run_state": "COMPLETE" if completed else "PARTIAL",
        "started_at": (timestamp[:19] + "Z") if timestamp else None,
        "tools": {**doc.get("tools", {}), "kani": meta.get("kani_version", doc.get("tools", {}).get("kani"))},
    })
    return out


def weakened_checks_detectable(doc):
    """Whether `doc` records the run configuration (so that weakened checks can be reported)."""
    return bool(doc.get("configuration_known", "configuration" in doc))
