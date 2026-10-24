#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Check specifications with Kani itself, instead of trusting a reading of the source text.

Generates small harnesses in a temporary copy of the crate (the user's files are never
modified), runs Kani on them once, and reports:

- requires-unsatisfiable (error): no input satisfies the function's `#[kani::requires]`
  clauses together, so its contract proof is vacuous (a `kani::cover!` of the conjunction is
  UNSATISFIABLE).
- requires-always-true (info): one `requires` clause holds for every input of its type, so it
  constrains nothing.
- requires-stronger / requires-incomparable (warning), requires-weaker / requires-equivalent
  (info), with `--compare-base` or `--compare-base-dir`: whether the current preconditions
  admit fewer inputs than the baseline's. A stronger precondition shrinks what the contract
  proof covers and needs the user's approval.
- harness-end-unreachable (error), with `--probe-harnesses`: Kani cannot reach the end of the
  harness, so its assumptions (or an earlier panic) exclude every execution; harness-end-reached
  (ok) otherwise, and harness-fails (info) when the harness fails anyway.
- unchecked (warning): a check could not be generated (unsupported parameter types, methods,
  generics) or did not compile. A check that cannot run never counts as passing.

Free functions, associated functions, and methods of non-generic impl blocks are checked; for a
method, the self type must derive or implement `kani::Arbitrary`. Parameters must be named by
plain identifiers, with types Kani can generate: primitives, arrays, tuples, `Option`,
`Result`, `NonZero*`, references to those, and crate types that derive or implement
`kani::Arbitrary`. Use `--kani-arg` for extra Kani flags (e.g. `-Z
quantifiers`).

Usage:
    check-specs.py PATH... [--function NAME] [--probe-harnesses [--harness NAME]]
                   [--compare-base REV | --compare-base-dir DIR] [--kani-arg ARG]... [--work-dir DIR]

The crate is copied to a persistent work directory under `target/` and only changed files are
re-copied, so dependencies are built once. Do not run two checks on the same crate at once.

Exit status: 0 when every check passed (only `ok`/`info`), 2 when there is a `warning` or
`error`, 1 on input errors.
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kani_plugin_lib import (  # noqa: E402
    emit, find_root, join_option_values, kani_only_files, parse_functions, rust_files, split_top_level,
    strip_comments_and_strings)

PRIMITIVES = {f"{s}{w}" for s in "iu" for w in ("8", "16", "32", "64", "128", "size")} | {
    "bool", "char", "f32", "f64", "()"}
NONZERO = {f"NonZero{s}{w}" for s in "IU" for w in ("8", "16", "32", "64", "128", "size")}
WRAPPERS = {"Option", "Result", "std::option::Option", "core::option::Option",
            "std::result::Result", "core::result::Result"}
PREFIX = "__kani_spec"
END_MARKER = f"{PREFIX}_end_reached"
CMP_MARKER = f"{PREFIX}_implies"
OUTPUT_TAIL_LINES = 40


def arbitrary_types(files):
    """Names of crate types that derive or implement `kani::Arbitrary`."""
    names = set()
    for f in files:
        code = strip_comments_and_strings(f.read_text(encoding="utf-8", errors="replace"))
        for m in re.finditer(r"derive\s*\(([^)]*)\)", code):
            if "Arbitrary" in m.group(1):
                item = re.compile(r"\b(?:struct|enum|union)\s+(\w+)").search(code, m.end())
                if item:
                    names.add(item.group(1))
        names.update(re.findall(r"\bimpl\b[^{;]*?\bArbitrary\s+for\s+(\w+)", code))
    return names


def supported(ty, extra, top=True):
    ty = ty.strip()
    if top and ty.startswith("&"):
        inner = re.sub(r"^&\s*(?:'\w+\s+)?(?:mut\s+)?", "", ty)
        return supported(inner, extra, top=False)
    if ty in PRIMITIVES or ty.split("::")[-1] in NONZERO or ty in extra:
        return True
    m = re.fullmatch(r"\[(.*);\s*([0-9_]+|[A-Z_][A-Z0-9_]*)\s*\]", ty, re.S)
    if m:
        return supported(m.group(1), extra, top=False)
    if ty.startswith("(") and ty.endswith(")"):
        return all(supported(t, extra, top=False) for t in split_top_level(ty[1:-1]))
    m = re.fullmatch(r"([\w:]+)\s*<(.*)>", ty, re.S)
    if m and m.group(1) in WRAPPERS:
        return all(supported(t, extra, top=False) for t in split_top_level(m.group(2)))
    return False


LIFETIME = re.compile(r"'\w+\s*")
IMPL_HEADER = re.compile(r"^\s*(?:unsafe\s+)?impl\s+(?:!?[\w:]+(?:<[^{]*>)?\s+for\s+)?([A-Za-z_][\w:]*)\s*$", re.S)


def strip_attrs(header):
    """The impl header without its attributes and doc comments."""
    code = re.sub(r"#\[[^\]]*\]", " ", header)
    return re.sub(r"//[^\n]*", " ", code).strip()


def context(fn, extra):
    """How to evaluate `fn`'s preconditions: (kind, self type, receiver) or (None, reason).

    kind is "free" for free functions, "assoc" for associated functions without `self`, and
    "method" for methods; the predicates of the last two go into an `impl` of the self type."""
    if fn.generic:
        return None, f"`{fn.name}` is generic"
    receiver = next((pat for pat, ty in fn.params if ty is None), None)
    if fn.scope in ("file", "mod"):
        return ("free", None, None), None
    if fn.scope != "impl":
        return None, f"`{fn.name}` is inside a `{fn.scope}` block; free functions and impl methods are checked"
    m = IMPL_HEADER.match(strip_attrs(fn.scope_header))
    if not m:
        return None, f"`{fn.name}` is in a generic or unsupported impl (`{strip_attrs(fn.scope_header)}`)"
    self_ty = m.group(1)
    if receiver is None:
        return ("assoc", self_ty, None), None
    if not supported(self_ty, extra, top=False):
        return None, (f"`{fn.name}` takes `{receiver}`, but `{self_ty}` does not derive or implement "
                      "kani::Arbitrary as far as the scan can tell")
    return ("method", self_ty, LIFETIME.sub("", receiver)), None


def bindings(fn, extra):
    """Statements binding each non-self parameter to a nondeterministic value, the arguments to
    pass, and the predicate's parameter list; or a reason why not."""
    lines, call_args, params = [], [], []
    for pat, ty in fn.params:
        if ty is None:
            continue
        name = re.sub(r"^mut\s+", "", pat)
        if not re.fullmatch(r"[A-Za-z_]\w*", name):
            return None, f"parameter pattern `{pat}` is not a plain identifier"
        if not supported(ty, extra):
            return None, f"parameter `{name}: {ty}`: Kani may not be able to generate this type"
        ty = LIFETIME.sub("", ty)
        params.append(f"{name}: {ty}")
        if ty.startswith("&"):
            inner = re.sub(r"^&\s*(?:mut\s+)?", "", ty)
            mut = "mut " if re.match(r"^&\s*mut\b", ty) else ""
            lines.append(f"let mut {PREFIX}_v_{name}: {inner} = kani::any();")
            call_args.append(f"&{mut}{PREFIX}_v_{name}")
        else:
            lines.append(f"let {name}: {ty} = kani::any();")
            call_args.append(name)
    return (lines, call_args, params), None


def conj(clauses):
    return " && ".join(f"({c})" for c in clauses) if clauses else "true"


class Plan:
    """The generated checks, grouped by the file they are inserted into."""

    def __init__(self):
        self.insertions = {}  # file -> list of (offset, text)
        self.checks = []  # dicts with kind, function, file, line, harness, and kind-specific keys
        self.unchecked = []
        self.harness_filters = [PREFIX]

    def insert(self, file, offset, text):
        self.insertions.setdefault(file, []).append((offset, text))


def harness(name, decls, stmts):
    lines = "\n    ".join(decls + stmts)
    return f"#[kani::proof]\nfn {name}() {{\n    {lines}\n}}"


def plan_function(plan, fn, base_requires, extra):
    """Plan the precondition checks of one function. All clauses are evaluated by one generated
    predicate with the function's own parameters (and receiver), so they mean exactly what they
    mean in the contract; each check is a separate harness that calls the predicate once."""
    requires = fn.attrs("kani::requires")
    comparing = base_requires is not None and sorted(base_requires) != sorted(requires)
    if not requires and not comparing:
        return
    ctx, reason = context(fn, extra)
    bound = None
    if ctx is not None:
        bound, reason = bindings(fn, extra)
    if ctx is None or bound is None:
        plan.unchecked.append({"function": fn.name, "file": fn.file, "line": fn.line, "reason": reason})
        return
    kind, self_ty, receiver = ctx
    decls, args, params = bound
    tag = f"{fn.name}_{fn.line}"
    pred = f"{PREFIX}_pred_{tag}"
    values = [conj(requires)] + [f"({c})" for c in requires]
    if comparing:
        values.append(conj(base_requires))
    sig = ", ".join(([receiver] if receiver else []) + params)
    pred_fn = (f"#[allow(non_snake_case, unused, clippy::all)]\n"
               f"fn {pred}({sig}) -> [bool; {len(values)}] {{ [{', '.join(values)}] }}")
    if kind == "free":
        call = f"{pred}({', '.join(args)})"
    elif kind == "assoc":
        call = f"{self_ty}::{pred}({', '.join(args)})"
    else:
        decls = decls + [f"let mut {PREFIX}_self: {self_ty} = kani::any();"]
        call = f"{PREFIX}_self.{pred}({', '.join(args)})"
    decls = decls + [f"let {PREFIX}_r = {call};"]
    r = f"{PREFIX}_r"
    harnesses = []

    def add(check_kind, stmts, **info):
        name = f"{PREFIX}_{check_kind}_{len(plan.checks)}_{tag}"
        harnesses.append(harness(name, decls, stmts))
        plan.checks.append({"kind": check_kind, "harness": name, "function": fn.name, "file": fn.file,
                            "line": fn.line, **info})

    if requires:
        add("sat", [f'kani::cover!({r}[0], "{PREFIX}_requires_satisfiable");'])
        for i, clause in enumerate(requires):
            add("clause", [f'kani::cover!(!{r}[{i + 1}], "{PREFIX}_clause_can_fail");'], clause=clause)
    if comparing:
        old = f"{r}[{len(values) - 1}]"
        add("new_implies_old", [f"kani::assume({r}[0]);", f'kani::assert({old}, "{CMP_MARKER}");'],
            old=base_requires, new=requires)
        add("old_implies_new", [f"kani::assume({old});", f'kani::assert({r}[0], "{CMP_MARKER}");'],
            old=base_requires, new=requires)
    mod = (f"\n#[cfg(kani)]\n#[allow(non_snake_case, unused, clippy::all)]\nmod {PREFIX}_{tag} {{\n"
           f"use super::*;\n{pred_fn if kind == 'free' else ''}\n" + "\n".join(harnesses) + "\n}\n")
    if kind == "free":
        plan.insert(fn.file, fn.end, mod)
    else:
        # The predicate needs `self`/`Self`: put it in an inherent impl after the enclosing impl.
        plan.insert(fn.file, fn.scope_end, f"\n#[cfg(kani)]\nimpl {self_ty} {{\n{pred_fn}\n}}\n" + mod)


def plan_probe(plan, fn):
    if any(n == "kani::should_panic" for n, _, _ in fn.attributes):
        plan.unchecked.append({"function": fn.name, "file": fn.file, "line": fn.line,
                               "reason": "should_panic harness: its end is not meant to be reached"})
        return
    # Insert before the closing brace. The leading `;` ends a trailing expression, if any.
    plan.insert(fn.file, fn.end - 1, f'\n;kani::assert(false, "{END_MARKER}");\n')
    plan.checks.append({"kind": "probe", "harness": fn.name, "function": fn.name, "file": fn.file,
                        "line": fn.line})
    plan.harness_filters.append(fn.name)


def base_source(args, path):
    if args.compare_base_dir:
        f = Path(args.compare_base_dir) / path
        return f.read_text(encoding="utf-8", errors="replace") if f.exists() else ""
    top = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=path.resolve().parent,
                         capture_output=True, text=True, check=True).stdout.strip()
    rel = path.resolve().relative_to(Path(top).resolve()).as_posix()
    out = subprocess.run(["git", "show", f"{args.compare_base}:{rel}"], cwd=top, capture_output=True, text=True)
    return out.stdout if out.returncode == 0 else ""


IGNORED_DIRS = re.compile(r"^(target.*|\.git|node_modules)$")


def sync_tree(src, dst):
    """Make `dst` a copy of `src` (without build outputs), copying only files whose size or
    modification time differ, so the copy's build cache stays valid between runs."""
    for dirpath, dirnames, filenames in os.walk(src):
        dirnames[:] = [d for d in dirnames if not IGNORED_DIRS.match(d)]
        rel = Path(dirpath).relative_to(src)
        (dst / rel).mkdir(parents=True, exist_ok=True)
        for name in filenames:
            s_file, d_file = Path(dirpath) / name, dst / rel / name
            st = s_file.stat()
            if d_file.exists():
                dt = d_file.stat()
                if dt.st_size == st.st_size and dt.st_mtime_ns == st.st_mtime_ns:
                    continue
            shutil.copy2(s_file, d_file, follow_symlinks=False)


def run_kani(plan, files, kani_args, work_dir=None):
    """Apply the plan to a copy of the crate and run Kani once. Cargo projects are copied to a
    persistent work directory (default `<workspace>/target/kani-plugin/spec-checks`) so that
    dependencies are only built once. Returns (export or None, output)."""
    first = files[0]
    root, package = find_root(first)
    with tempfile.TemporaryDirectory(prefix="kani-spec-") as tmp:
        tmp = Path(tmp)
        if root is None:
            if len({f.resolve() for f in files}) != 1:
                raise SystemExit("error: files outside a Cargo project must be checked one at a time")
            work_file = tmp / first.name
            shutil.copy2(first, work_file)
            mapping = {first.resolve(): work_file}
            cwd, cmd = tmp, ["kani", work_file.name]
        else:
            copy = Path(work_dir) if work_dir else root / "target" / "kani-plugin" / "spec-checks"
            sync_tree(root, copy)
            mapping = {f.resolve(): copy / f.resolve().relative_to(root) for f in files}
            cwd, cmd = copy / package.relative_to(root), ["cargo", "kani"]
        for file, edits in plan.insertions.items():
            target = mapping[Path(file).resolve()]
            text = Path(file).read_text(encoding="utf-8")
            for offset, snippet in sorted(edits, key=lambda e: e[0], reverse=True):
                text = text[:offset] + snippet + text[offset:]
            target.write_text(text, encoding="utf-8")
        export = tmp / "results.json"
        cmd += ["-Z", "function-contracts", "-Z", "export-json", "--export-json", str(export)]
        for h in plan.harness_filters:
            cmd += ["--harness", h]
        cmd += kani_args
        out = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
        text = out.stdout + out.stderr
        doc = json.loads(export.read_text()) if export.exists() else None
        return doc, text


def property_descriptions(h):
    return [p.get("description", "") for p in h.get("failed_properties", [])]


def interpret(plan, doc):
    by_name = {}
    for h in doc.get("harnesses", []):
        by_name.setdefault(h["name"].split("::")[-1], h)
    findings, results = [], {}
    for c in plan.checks:
        h = by_name.get(c["harness"])
        loc = {"file": c["file"], "line": c["line"], "function": c["function"]}
        if h is None:
            findings.append({**loc, "severity": "warning", "rule": "unchecked",
                             "message": f"generated check `{c['harness']}` did not run"})
            continue
        outcome = h.get("outcome", {})
        verdict = outcome.get("verdict") if outcome.get("kind") == "COMPLETED" else outcome.get("kind")
        covers = h.get("covers", {})
        if c["kind"] in ("sat", "clause"):
            satisfied = bool(covers.get("satisfied"))
            if verdict not in ("SUCCESS", "FAILURE") or not (satisfied or covers.get("unsatisfiable")):
                findings.append({**loc, "severity": "warning", "rule": "unchecked",
                                 "message": f"check `{c['kind']}` was inconclusive ({verdict})"})
            elif c["kind"] == "sat" and not satisfied:
                findings.append({**loc, "severity": "error", "rule": "requires-unsatisfiable",
                                 "message": "no input satisfies the preconditions together; the contract "
                                            "proof is vacuous."})
            elif c["kind"] == "sat":
                findings.append({**loc, "severity": "ok", "rule": "requires-satisfiable",
                                 "message": "some input satisfies the preconditions."})
            elif not satisfied:
                findings.append({**loc, "severity": "info", "rule": "requires-always-true",
                                 "message": f"`{c['clause']}` holds for every input; it constrains nothing."})
        elif c["kind"] in ("new_implies_old", "old_implies_new"):
            results[(c["function"], c["file"], c["line"], c["kind"])] = (verdict, c)
        elif c["kind"] == "probe":
            reached = any(END_MARKER in d for d in property_descriptions(h))
            other_failures = [d for d in property_descriptions(h) if END_MARKER not in d]
            if other_failures:
                findings.append({**loc, "severity": "info", "rule": "harness-fails",
                                 "message": f"the harness fails ({len(other_failures)} failed properties, e.g. "
                                 f"{other_failures[0]}); its end is "
                                 f"{'reachable' if reached else 'not reached'}. Fix the failure "
                                 "first; vacuity only matters for passing harnesses."})
            elif reached:
                findings.append({**loc, "severity": "ok", "rule": "harness-end-reached",
                                 "message": "Kani reaches the end of the harness."})
            elif verdict == "SUCCESS":
                findings.append({**loc, "severity": "error", "rule": "harness-end-unreachable",
                                 "message": "no execution reaches the end of the harness: its assumptions "
                                            "or an earlier panic exclude everything after them. The proof "
                                            "is vacuous."})
            else:
                findings.append({**loc, "severity": "warning", "rule": "unchecked",
                                 "message": f"reachability probe was inconclusive ({verdict})"})
    pairs = {}
    for (fn, file, line, kind), (verdict, c) in results.items():
        pairs.setdefault((fn, file, line), {})[kind] = (verdict, c)
    for (fn, file, line), p in pairs.items():
        loc = {"file": file, "line": line, "function": fn}
        if set(p) != {"new_implies_old", "old_implies_new"} or any(v not in ("SUCCESS", "FAILURE")
                                                                   for v, _ in p.values()):
            findings.append({**loc, "severity": "warning", "rule": "unchecked",
                             "message": "precondition comparison was inconclusive"})
            continue
        new_old = p["new_implies_old"][0] == "SUCCESS"
        old_new = p["old_implies_new"][0] == "SUCCESS"
        c = p["new_implies_old"][1]
        detail = f"before: {c['old']}; after: {c['new']}"
        if new_old and old_new:
            findings.append({**loc, "severity": "info", "rule": "requires-equivalent",
                             "message": f"the preconditions admit the same inputs ({detail})."})
        elif new_old:
            findings.append({**loc, "severity": "warning", "rule": "requires-stronger",
                             "message": "the new preconditions exclude inputs the old ones admitted: the "
                                        f"contract proof covers less. Needs the user's approval ({detail})."})
        elif old_new:
            findings.append({**loc, "severity": "info", "rule": "requires-weaker",
                             "message": f"the new preconditions admit more inputs ({detail})."})
        else:
            findings.append({**loc, "severity": "warning", "rule": "requires-incomparable",
                             "message": "the old and new preconditions each admit inputs the other "
                                        f"excludes. Needs the user's approval ({detail})."})
    return findings


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("paths", nargs="+", help="Rust files or directories of one crate")
    parser.add_argument("--function", action="append", default=[], help="only check these functions")
    parser.add_argument("--probe-harnesses", action="store_true", help="check that each harness's end is reachable")
    parser.add_argument("--harness", action="append", default=[], help="with --probe-harnesses, only these")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--compare-base", help="git revision whose preconditions to compare against")
    group.add_argument("--compare-base-dir", help="directory with a copy of the sources made before editing")
    parser.add_argument("--kani-arg", action="append", default=[],
                        help="extra argument for Kani, e.g. `--kani-arg -Z --kani-arg quantifiers` (repeatable)")
    parser.add_argument("--work-dir", help="where to keep the crate copy (default: target/kani-plugin/spec-checks)")
    args = parser.parse_args(join_option_values(argv, {"--kani-arg"}))

    files = list(rust_files(args.paths))
    if not files:
        print("error: no Rust files given", file=sys.stderr)
        return 1
    _, package = find_root(files[0])
    crate = list(rust_files([package])) if package else files
    kani_only = kani_only_files(crate)
    extra = arbitrary_types(crate)
    comparing = bool(args.compare_base or args.compare_base_dir)
    plan = Plan()

    def key(fn):
        return (strip_attrs(fn.scope_header), fn.name)

    for f in files:
        src = f.read_text(encoding="utf-8", errors="replace")
        base = {}
        if comparing:
            for bf in parse_functions(base_source(args, f), f):
                base.setdefault(key(bf), bf.attrs("kani::requires"))
        for fn in parse_functions(src, f, f.resolve() in kani_only):
            is_harness = any(n in ("kani::proof", "kani::proof_for_contract") for n, _, _ in fn.attributes)
            if is_harness:
                if args.probe_harnesses and (not args.harness or fn.name in args.harness):
                    plan_probe(plan, fn)
                continue
            if fn.in_kani or (args.function and fn.name not in args.function):
                continue
            plan_function(plan, fn, base.get(key(fn), []) if comparing else None, extra)

    findings = [{"file": u["file"], "line": u["line"], "function": u["function"], "severity": "warning",
                 "rule": "unchecked", "message": u["reason"]} for u in plan.unchecked]
    output_tail = None
    if plan.checks:
        doc, text = run_kani(plan, files, args.kani_arg, args.work_dir)
        if doc is None:
            output_tail = "\n".join(text.splitlines()[-OUTPUT_TAIL_LINES:])
            findings += [{"file": c["file"], "line": c["line"], "function": c["function"],
                          "severity": "warning", "rule": "unchecked",
                          "message": "the generated checks did not compile or Kani failed; see output_tail"}
                         for c in plan.checks]
        else:
            findings += interpret(plan, doc)
    result = {"checks_generated": len(plan.checks), "findings": findings}
    if output_tail:
        result["output_tail"] = output_tail
    emit(result)
    return 2 if any(f["severity"] in ("warning", "error") for f in findings) else 0


if __name__ == "__main__":
    sys.exit(main())
