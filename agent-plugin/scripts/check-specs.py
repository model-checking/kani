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
  (ok) otherwise.
- unchecked (warning): a check could not be generated (unsupported parameter types, methods,
  generics) or did not compile. A check that cannot run never counts as passing.

Parameters must be named by plain identifiers, with types Kani can generate: primitives,
arrays, tuples, `Option`, `Result`, `NonZero*`, references to those, and crate types that
derive or implement `kani::Arbitrary`. Use `--kani-arg` for extra Kani flags (e.g. `-Z
quantifiers`).

Usage:
    check-specs.py PATH... [--function NAME] [--probe-harnesses [--harness NAME]]
                   [--compare-base REV | --compare-base-dir DIR] [--kani-arg ARG]...

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
from kani_plugin_lib import emit, parse_functions, rust_files, split_top_level, strip_comments_and_strings  # noqa: E402

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


def declarations(fn, extra):
    """Statements binding each parameter to a nondeterministic value, or a reason why not."""
    if fn.scope not in ("file", "mod"):
        return None, f"`{fn.name}` is inside an `{fn.scope}` block; only free functions are checked"
    if fn.generic:
        return None, f"`{fn.name}` is generic"
    lines = []
    for pat, ty in fn.params:
        if ty is None:
            return None, f"`{fn.name}` takes `self`"
        name = re.sub(r"^mut\s+", "", pat)
        if not re.fullmatch(r"[A-Za-z_]\w*", name):
            return None, f"parameter pattern `{pat}` is not a plain identifier"
        if not supported(ty, extra):
            return None, f"parameter `{name}: {ty}`: Kani may not be able to generate this type"
        if ty.startswith("&"):
            inner = re.sub(r"^&\s*(?:'\w+\s+)?(?:mut\s+)?", "", ty)
            mut = "mut " if re.match(r"^&\s*(?:'\w+\s+)?mut\b", ty) else ""
            lines.append(f"let {mut}{PREFIX}_v_{name}: {inner} = kani::any();")
            lines.append(f"let {name}: {ty} = &{mut}{PREFIX}_v_{name};")
        else:
            lines.append(f"let {name}: {ty} = kani::any();")
    return lines, None


def conj(clauses):
    return " && ".join(f"({c})" for c in clauses) if clauses else "true"


def find_root(path):
    """The directory to copy (the top Cargo workspace) and the package directory inside it."""
    p = path.resolve()
    start = p if p.is_dir() else p.parent
    package = next((d for d in [start, *start.parents] if (d / "Cargo.toml").exists()), None)
    if package is None:
        return None, None
    root = package
    for d in package.parents:
        manifest = d / "Cargo.toml"
        if manifest.exists() and re.search(r"^\s*\[workspace\]", manifest.read_text(errors="replace"), re.M):
            root = d
    return root, package


class Plan:
    """The generated checks, grouped by the file they are inserted into."""

    def __init__(self):
        self.insertions = {}  # file -> list of (offset, text)
        self.checks = []  # dicts with id, kind, function, file, line, harness, and kind-specific keys
        self.unchecked = []
        self.harness_filters = [PREFIX]

    def insert(self, file, offset, text):
        self.insertions.setdefault(file, []).append((offset, text))

    def module(self, fn, harnesses):
        """A `#[cfg(kani)]` module of `harnesses` placed right after `fn`, in the same scope."""
        mod = f"{PREFIX}_{fn.name}_{fn.line}"
        body = "\n".join(harnesses)
        return (f"\n#[cfg(kani)]\n#[allow(non_snake_case, unused, clippy::all)]\nmod {mod} {{\n"
                f"use super::*;\n{body}\n}}\n")


def harness(name, decls, stmts):
    lines = "\n    ".join(decls + stmts)
    return f"#[kani::proof]\nfn {name}() {{\n    {lines}\n}}"


def plan_function(plan, fn, base_requires, extra):
    requires = fn.attrs("kani::requires")
    if not requires and base_requires is None:
        return
    decls, reason = declarations(fn, extra)
    if decls is None:
        plan.unchecked.append({"function": fn.name, "file": fn.file, "line": fn.line, "reason": reason})
        return
    tag = f"{fn.name}_{fn.line}"
    harnesses = []

    def add(kind, stmts, **info):
        name = f"{PREFIX}_{kind}_{len(plan.checks)}_{tag}"
        harnesses.append(harness(name, decls, stmts))
        plan.checks.append({"kind": kind, "harness": name, "function": fn.name, "file": fn.file,
                            "line": fn.line, **info})

    if requires:
        add("sat", [f'kani::cover!({conj(requires)}, "{PREFIX}_requires_satisfiable");'])
        for clause in requires:
            add("clause", [f'kani::cover!(!({clause}), "{PREFIX}_clause_can_fail");'], clause=clause)
    if base_requires is not None and sorted(base_requires) != sorted(requires):
        old, new = conj(base_requires), conj(requires)
        add("new_implies_old", [f"kani::assume({new});", f'kani::assert({old}, "{CMP_MARKER}");'],
            old=base_requires, new=requires)
        add("old_implies_new", [f"kani::assume({old});", f'kani::assert({new}, "{CMP_MARKER}");'],
            old=base_requires, new=requires)
    if harnesses:
        plan.insert(fn.file, fn.end, plan.module(fn, harnesses))


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


def run_kani(plan, files, kani_args):
    """Apply the plan to a temporary copy and run Kani once. Returns (export or None, output)."""
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
            copy = tmp / "src"
            shutil.copytree(root, copy, symlinks=True,
                            ignore=shutil.ignore_patterns("target", "target-*", ".git", "kani-list.json"))
            mapping = {f.resolve(): copy / f.resolve().relative_to(root) for f in files}
            cwd, cmd = copy / package.relative_to(root), ["cargo", "kani"]
        for file, edits in plan.insertions.items():
            target = mapping[Path(file).resolve()]
            text = target.read_text(encoding="utf-8")
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
            if reached:
                findings.append({**loc, "severity": "ok", "rule": "harness-end-reached",
                                 "message": "Kani reaches the end of the harness."})
            elif other_failures:
                findings.append({**loc, "severity": "info", "rule": "harness-fails-before-end",
                                 "message": "the harness fails before its end; fix the failure first."})
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
    parser.add_argument("--kani-arg", action="append", default=[], help="extra argument for Kani (repeatable)")
    args = parser.parse_args(argv)

    files = list(rust_files(args.paths))
    if not files:
        print("error: no Rust files given", file=sys.stderr)
        return 1
    root, _ = find_root(files[0])
    all_files = list(rust_files([root])) if root else files
    extra = arbitrary_types(all_files)
    comparing = bool(args.compare_base or args.compare_base_dir)
    plan = Plan()
    for f in files:
        src = f.read_text(encoding="utf-8", errors="replace")
        base = {}
        if comparing:
            for bf in parse_functions(base_source(args, f), f):
                base.setdefault(bf.name, bf.attrs("kani::requires"))
        for fn in parse_functions(src, f):
            is_harness = any(n in ("kani::proof", "kani::proof_for_contract") for n, _, _ in fn.attributes)
            if is_harness:
                if args.probe_harnesses and (not args.harness or fn.name in args.harness):
                    plan_probe(plan, fn)
                continue
            if fn.in_kani or (args.function and fn.name not in args.function):
                continue
            plan_function(plan, fn, base.get(fn.name) if comparing else None, extra)

    findings = [{"file": u["file"], "line": u["line"], "function": u["function"], "severity": "warning",
                 "rule": "unchecked", "message": u["reason"]} for u in plan.unchecked]
    output_tail = None
    if plan.checks:
        doc, text = run_kani(plan, files, args.kani_arg)
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
