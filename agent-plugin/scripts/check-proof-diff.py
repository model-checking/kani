#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Compare proofs and code against a baseline, and flag changes that weaken what is proven.

The property the user agreed to is pinned: once a harness and its contracts are approved, any
change that makes the proof cover less must be approved by the user again. And while verifying
existing code, the agent adds proofs, it does not change the code under verification: a fix is
proposed separately, with a concrete playback witness (see `check-witness.py`).

Compares every function in the given Rust files with the same function in the baseline (a git
revision, or a copy of the sources made before editing) and reports:

- harness-removed (error), harness-attribute-removed (error: the function is still there but
  is no longer a harness), harness-added (info)
- assertion-removed (error), cover-removed (warning), assertion-added (info)
- assumption-added / assumption-changed (warning): the inputs covered may have shrunk
  (`kani::assume` and `kani::any_where` predicates, in the harness or in the Kani-only helpers
  it calls)
- should-panic-added (error); stub-added, stub-verified-added (warning); unwind-lowered (warning)
- requires-added / requires-changed (warning), ensures-removed / ensures-changed (warning),
  modifies-changed (warning); `check-specs.py --compare-base` decides with Kani whether a changed
  precondition is stronger or weaker
- code-changed (error): an executable function outside harnesses and `#[cfg(kani)]` code changed
  its signature or body; code-added (warning), code-removed (error)

This is a text-based comparison of items: it reports what changed for review and never decides
that a change is safe. Exit status: 0 when only `info` findings are reported, 2 when there is a
`warning` or `error`, 1 on input errors.

Also flagged: item-position macro invocations that changed (`macro-invocation-changed`: their
expansion is invisible to the scan, so executable code may have changed), and stubs on new
harnesses (`new-harness-stub`). Helpers that are only compiled for Kani (`#[cfg(kani)]`,
`#[cfg(any(kani, test))]`, files included that way) are never reported as new executable code.
Only files that changed since the base (or mention Kani) are read, so large crates are quick;
files over `--max-file-kb` (generated bindings) are skipped and counted.

Usage:
    check-proof-diff.py PATH... [--base REV | --base-dir DIR] [--allow-code-change FUNCTION]...
"""

import argparse
import os
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kani_plugin_lib import (  # noqa: E402
    emit, find_root, helper_closure, helper_index, kani_only_files, parse_functions, rust_files,
    strip_comments_and_strings)

# `kani::any_where(|x| cond)` restricts its value like an assumption.
ASSUME = re.compile(r"\bkani::assume\s*\(|\bkani::any_where\s*\(")
ASSERT = re.compile(
    r"\b(?:assert|assert_eq|assert_ne|debug_assert|debug_assert_eq|debug_assert_ne)!\s*\(|\bkani::assert\s*\(")
COVER = re.compile(r"\bkani::cover!\s*\(")


def norm(text):
    return re.sub(r"\s+", " ", text).strip()


def calls(fn, pattern, index=None):
    """Normalized text of each call matching `pattern` in a function body and, given a helper
    index, in the Kani-only helpers it calls, as a multiset."""
    out = Counter()
    if index is not None:
        for h in helper_closure(fn, index):
            out += calls(h, pattern)
    for m in pattern.finditer(fn.body):
        open_paren = m.end() - 1
        depth = 0
        for k in range(open_paren, len(fn.body)):
            if fn.body[k] == "(":
                depth += 1
            elif fn.body[k] == ")":
                depth -= 1
                if depth == 0:
                    out[norm(fn.body_src[m.start():k + 1])] += 1
                    break
    return out


def key_functions(functions):
    """Key functions by name and occurrence, so overloaded method names stay distinct."""
    seen = Counter()
    keyed = {}
    for f in functions:
        seen[f.name] += 1
        keyed[f"{f.name}#{seen[f.name]}" if seen[f.name] > 1 else f.name] = f
    return keyed


def is_harness(f):
    return has_attr(f, "kani::proof") or has_attr(f, "kani::proof_for_contract")


def has_attr(f, name):
    return any(n == name for n, _, _ in f.attributes)


def finding(file, line, item, severity, rule, message):
    return {"file": file, "line": line, "item": item, "severity": severity, "rule": rule, "message": message}


def diff_harness(old, new, old_index=None, new_index=None):
    """Compare two versions of a harness. Assertions, covers, and assumptions in the Kani-only
    helpers each version calls count as part of the harness."""
    out = []
    loc = (new.file, new.line, new.name)
    old_asserts, new_asserts = calls(old, ASSERT, old_index), calls(new, ASSERT, new_index)
    for a in (old_asserts - new_asserts).elements():
        out.append(finding(*loc, "error", "assertion-removed",
                           f"assertion removed or changed: `{a}`. The proof no longer checks it."))
    for a in (new_asserts - old_asserts).elements():
        out.append(finding(*loc, "info", "assertion-added", f"assertion added: `{a}`."))
    for c in (calls(old, COVER, old_index) - calls(new, COVER, new_index)).elements():
        out.append(finding(*loc, "warning", "cover-removed",
                           f"cover removed or changed: `{c}`. Vacuity probes must stay."))
    old_assumes, new_assumes = calls(old, ASSUME, old_index), calls(new, ASSUME, new_index)
    added = new_assumes - old_assumes
    for a in added.elements():
        rule = "assumption-changed" if old_assumes - new_assumes else "assumption-added"
        out.append(finding(*loc, "warning", rule,
                           f"`{a}` restricts the inputs the proof covers; it needs the user's approval."))
    old_attrs = {n: Counter(map(norm, old.attrs(n))) for n in ("kani::stub", "kani::stub_verified")}
    for name, rule in (("kani::stub", "stub-added"), ("kani::stub_verified", "stub-verified-added")):
        for a in (Counter(map(norm, new.attrs(name))) - old_attrs[name]).elements():
            out.append(finding(*loc, "warning", rule,
                               f"`#[{name}({a})]` replaces real code with an assumption about it."))
    if has_attr(new, "kani::should_panic") and not has_attr(old, "kani::should_panic"):
        out.append(finding(*loc, "error", "should-panic-added",
                           "`#[kani::should_panic]` added: the harness now passes only if the code panics."))
    old_unwind, new_unwind = old.attrs("kani::unwind"), new.attrs("kani::unwind")
    if old_unwind and new_unwind and old_unwind[0].isdigit() and new_unwind[0].isdigit() \
            and int(new_unwind[0]) < int(old_unwind[0]):
        out.append(finding(*loc, "warning", "unwind-lowered",
                           f"unwind bound lowered from {old_unwind[0]} to {new_unwind[0]}."))
    return out


def diff_contract(old, new):
    out = []
    loc = (new.file, new.line, new.name)
    for name, rule_added, rule_removed in (
        ("kani::requires", "requires-added", "requires-removed"),
        ("kani::ensures", "ensures-added", "ensures-removed"),
        ("kani::modifies", "modifies-added", "modifies-removed"),
    ):
        o, n = Counter(map(norm, old.attrs(name))), Counter(map(norm, new.attrs(name)))
        if o == n:
            continue
        section = name.split("::")[1]
        rule = rule_added if not (o - n) else (f"{section}-changed" if n - o else rule_removed)
        change = {rule_added: "added", rule_removed: "removed"}.get(rule, "changed")
        if section == "requires":
            sev = "warning"
            msg = (f"precondition {change}: a stronger `requires` shrinks what the contract proof covers. "
                   "Run `check-specs.py --compare-base` to classify it, and get the user's approval.")
        elif section == "ensures":
            sev = "info" if change == "added" else "warning"
            msg = (f"postcondition {change}." if change == "added" else
                   f"postcondition {change}: callers may rely on what was guaranteed before.")
        else:
            sev = "warning"
            msg = f"`modifies` {change}: the frame of the contract is different."
        out.append(finding(*loc, sev, rule, f"{msg} Before: {sorted(o.elements())}; after: {sorted(n.elements())}."))
    return out


def code_fingerprint(f):
    """Signature and body with comments and whitespace normalized; Kani contract attributes are
    excluded because `diff_contract` reports them separately."""
    signature = norm(strip_comments_and_strings(f.signature))
    return signature, norm(f.body)


# Macros of the standard library and Kani: they do not define items.
STD_MACROS = {"println", "print", "eprintln", "eprint", "format", "write", "writeln", "vec", "panic", "assert",
              "assert_eq", "assert_ne", "debug_assert", "debug_assert_eq", "debug_assert_ne", "unreachable",
              "todo", "unimplemented", "matches", "dbg", "include", "include_str", "include_bytes", "concat",
              "stringify", "env", "option_env", "line", "file", "column", "cfg", "compile_error", "thread_local"}
MACRO_CALL = re.compile(r"^[ \t]*([A-Za-z_]\w*)!\s*([(\[{])", re.M)


def macro_invocations(src, functions):
    """Normalized text of item-position macro invocations (outside function bodies): their expansion
    is invisible to the source scan, so a change inside one can change executable code."""
    code = strip_comments_and_strings(src)
    spans = [(f.start, f.end) for f in functions]
    out = Counter()
    for m in MACRO_CALL.finditer(code):
        if m.group(1) in STD_MACROS or any(a <= m.start() < b for a, b in spans):
            continue
        close = {"(": ")", "[": "]", "{": "}"}[m.group(2)]
        depth, k = 0, m.end() - 1
        while k < len(code):
            if code[k] == m.group(2):
                depth += 1
            elif code[k] == close:
                depth -= 1
                if depth == 0:
                    break
            k += 1
        out[norm(src[m.start():k + 1])] += 1
    return out


def diff_file(path, old_src, new_src, old_index=None, new_index=None, kani_only=False, allow=frozenset()):
    """Compare two versions of a file. The helper indexes cover the whole crate, so that helpers
    defined in other files count; `kani_only` marks a file compiled only for Kani. Functions in
    `allow` have approved code changes."""
    out = []
    old = key_functions(parse_functions(old_src, path, kani_only)) if old_src is not None else {}
    new = key_functions(parse_functions(new_src, path, kani_only))
    if not kani_only and old_src is not None:
        old_macros = macro_invocations(old_src, old.values())
        new_macros = macro_invocations(new_src, new.values())
        for text in (new_macros - old_macros).elements():
            out.append(finding(str(path), 0, text.split("!")[0], "warning", "macro-invocation-changed",
                               f"an item-generating macro invocation changed or was added: `{text[:100]}`. "
                               "Its expansion is invisible to this scan, so executable code may have changed: "
                               "review it by hand."))
        for text in (old_macros - new_macros).elements():
            out.append(finding(str(path), 0, text.split("!")[0], "warning", "macro-invocation-removed",
                               f"an item-generating macro invocation was removed or changed: `{text[:100]}`."))
    if new_index is None:
        new_index = helper_index(new.values())
    if old_index is None:
        old_index = helper_index(old.values())
    for key, n in new.items():
        o = old.get(key)
        if o is not None and is_harness(o) and not is_harness(n):
            out.append(finding(n.file, n.line, n.name, "error", "harness-attribute-removed",
                               "`#[kani::proof]` (or `proof_for_contract`) was removed: Kani no longer runs "
                               "this harness, and its property is no longer checked."))
            continue
        if is_harness(n):
            if o is None or not is_harness(o):
                assumes = sorted(calls(n, ASSUME, new_index).elements())
                detail = (f" Its assumptions, including those in the helpers it calls: {assumes}. "
                          "Confirm the user approved each one, then snapshot a new baseline."
                          if assumes else " It has no assumptions.")
                out.append(finding(n.file, n.line, n.name, "info", "harness-added", "new harness." + detail))
                for kind in ("kani::stub", "kani::stub_verified"):
                    for spec in n.attrs(kind):
                        out.append(finding(n.file, n.line, n.name, "warning", "new-harness-stub",
                                           f"`#[{kind}({norm(spec)})]` replaces real code with a model or a "
                                           "contract: it is an assumption about that code. Confirm the user "
                                           "approved it, and list it in the report."))
            else:
                out.extend(diff_harness(o, n, old_index, new_index))
            continue
        if o is not None:
            out.extend(diff_contract(o, n))
        if n.in_kani:
            continue
        if o is None:
            out.append(finding(n.file, n.line, n.name, "warning", "code-added",
                               "new executable function: check that it was requested."))
        elif code_fingerprint(o) != code_fingerprint(n) and n.name in allow:
            out.append(finding(n.file, n.line, n.name, "info", "approved-code-change",
                               "executable code changed; listed in --allow-code-change. Confirm it is "
                               "confirmed by a witness (check-witness.py --base)."))
        elif code_fingerprint(o) != code_fingerprint(n):
            out.append(finding(
                n.file, n.line, n.name, "error", "code-changed",
                "executable code under verification changed. Proofs must not change the code; propose a "
                "fix separately, with a concrete playback witness that fails before and passes after."))
    for key, o in old.items():
        if key in new:
            continue
        if is_harness(o):
            out.append(finding(str(path), o.line, o.name, "error", "harness-removed",
                               "harness removed: its property is no longer checked."))
        elif not o.in_kani:
            out.append(finding(str(path), o.line, o.name, "error", "code-removed",
                               "executable function removed."))
    return out


def changed_files(args, sample):
    """Resolved paths of files that differ from the base (or are new), or None if that cannot be
    determined cheaply (then every file is compared). Unchanged files are not read from git."""
    if args.base_dir or not sample:
        return None
    top = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=Path(sample).resolve().parent,
                         capture_output=True, text=True)
    if top.returncode != 0:
        return None
    top = Path(top.stdout.strip())
    changed = set()
    for cmd in (["git", "diff", "--name-only", args.base, "--"], ["git", "ls-files", "--others", "--exclude-standard"]):
        out = subprocess.run(cmd, cwd=top, capture_output=True, text=True)
        if out.returncode != 0:
            return None
        changed.update((top / line).resolve() for line in out.stdout.splitlines())
    return changed


def base_source(args, f):
    if args.base_dir:
        base_file = Path(args.base_dir) / os.path.relpath(f)
        return base_file.read_text(encoding="utf-8", errors="replace") if base_file.exists() else None
    return git_show(args.base, f)


def git_show(rev, path):
    top = subprocess.run(["git", "rev-parse", "--show-toplevel"], cwd=path.parent,
                         capture_output=True, text=True, check=True).stdout.strip()
    rel = path.resolve().relative_to(Path(top).resolve())
    out = subprocess.run(["git", "show", f"{rev}:{rel.as_posix()}"], cwd=top, capture_output=True, text=True)
    return out.stdout if out.returncode == 0 else None


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("paths", nargs="+", help="Rust files or directories to compare")
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--base", default="HEAD", help="git revision to compare against (default: HEAD)")
    group.add_argument("--base-dir", help="directory holding a copy of the sources made before editing; "
                       "PATHs are resolved relative to the current directory and looked up under it")
    parser.add_argument("--allow-code-change", action="append", default=[], metavar="FUNCTION",
                        help="a function whose code change the user approved (a fix confirmed by a witness); "
                             "reported as info instead of an error (repeatable)")
    parser.add_argument("--max-file-kb", type=int, default=2048,
                        help="skip source files larger than this (generated bindings); default 2048")
    args = parser.parse_args(argv)
    findings = []
    files = skipped_large = unchanged = harness_count = 0
    try:
        targets = list(rust_files(args.paths))
        _, package = find_root(targets[0]) if targets else (None, None)
        crate = list(rust_files([package])) if package else targets
        big = {f.resolve() for f in crate + targets if f.stat().st_size > args.max_file_kb * 1024}
        skipped_large = len(big)
        crate = [f for f in crate if f.resolve() not in big]
        targets = [f for f in targets if f.resolve() not in big]
        changed = changed_files(args, targets[0] if targets else None)
        new_srcs = {f.resolve(): f.read_text(encoding="utf-8", errors="replace") for f in crate + targets}
        # Only files that mention Kani, or changed, matter for the helper index and the diff.
        old_srcs = {f: (base_source(args, Path(f)) if changed is None or f in changed else src)
                    for f, src in new_srcs.items() if "kani" in src or changed is None or f in changed}
        kani_only = kani_only_files([f for f in crate if f.resolve() in old_srcs])
        new_index = helper_index([fn for f in old_srcs for fn in parse_functions(new_srcs[f], f, f in kani_only)])
        old_index = helper_index([fn for f, src in old_srcs.items() if src is not None
                                  for fn in parse_functions(src, f, f in kani_only)])
        allow = frozenset(args.allow_code_change)
        for f in targets:
            r = f.resolve()
            if r not in old_srcs or old_srcs[r] == new_srcs[r]:
                unchanged += 1
                continue
            files += 1
            harness_count += sum(1 for fn in parse_functions(new_srcs[r], f, r in kani_only) if is_harness(fn))
            findings.extend(diff_file(f, old_srcs[r], new_srcs[r], old_index, new_index, r in kani_only, allow))
    except (OSError, subprocess.CalledProcessError) as e:
        print(f"error: {e}", file=sys.stderr)
        return 1
    out = {"base": args.base_dir or args.base, "files_compared": files, "files_unchanged": unchanged,
           "harnesses_in_compared_files": harness_count, "findings": findings}
    if skipped_large:
        out["skipped_large_files"] = skipped_large
    emit(out)
    return 2 if any(f["severity"] != "info" for f in findings) else 0


if __name__ == "__main__":
    sys.exit(main())
