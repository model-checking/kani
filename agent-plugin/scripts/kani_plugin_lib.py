# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Shared helpers for the Kani agent plugin scripts.

The static checks here are deliberately lightweight: they scan Rust source text rather than
parse it, so they never need a toolchain and can run before anything compiles. They report
*suspicions* for the agent to review, not verdicts.
"""

import json
import re
from dataclasses import dataclass, field
from pathlib import Path

# Matches the attributes that mark a Kani harness, including `cfg_attr(kani, ...)` forms.
HARNESS_ATTR = re.compile(
    r"#\[\s*(?:cfg_attr\s*\(\s*kani\s*,\s*)?kani::(proof_for_contract|proof)\b\s*(?:\(([^)]*)\))?"
)
FN_DECL = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")
ATTR = re.compile(r"#\[\s*(?:cfg_attr\s*\(\s*kani\s*,\s*)?kani::([a-z_]+)\s*(?:\((.*?)\))?\s*\)?\s*\]", re.S)


def strip_comments_and_strings(src):
    """Blank out comments and the contents of string/char literals, preserving offsets and
    newlines, so later regexes and brace matching only see code."""
    out = list(src)
    i, n = 0, len(src)

    def blank(start, end):
        for k in range(start, end):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = src[i]
        nxt = src[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            end = src.find("\n", i)
            end = n if end == -1 else end
            blank(i, end)
            i = end
        elif c == "/" and nxt == "*":
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif c == "r" and re.match(r'r#*"', src[i:]) and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_")):
            hashes = re.match(r'r(#*)"', src[i:]).group(1)
            start = i + 2 + len(hashes)
            end = src.find('"' + hashes, start)
            end = n if end == -1 else end
            blank(start, end)
            i = end + 1 + len(hashes)
        elif c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            blank(i + 1, min(j, n))
            i = j + 1
        elif c == "'":
            # Char literal ('a', '\n', '\u{1F600}') vs lifetime ('a).
            m = re.match(r"'(\\u\{[0-9A-Fa-f]+\}|\\.|[^\\'])'", src[i:])
            if m:
                blank(i + 1, i + len(m.group(0)) - 1)
                i += len(m.group(0))
            else:
                i += 1
        else:
            i += 1
    return "".join(out)


def line_of(src, offset):
    return src.count("\n", 0, offset) + 1


def matching_brace(code, open_idx):
    depth = 0
    for k in range(open_idx, len(code)):
        if code[k] == "{":
            depth += 1
        elif code[k] == "}":
            depth -= 1
            if depth == 0:
                return k
    return len(code) - 1


@dataclass
class Harness:
    name: str
    file: str
    line: int
    kind: str  # "proof" or "proof_for_contract"
    target: str = None  # contract target for proof_for_contract
    attributes: dict = field(default_factory=dict)
    body: str = ""  # code with comments/strings blanked
    body_line: int = 0  # line number where `body` starts


def find_harnesses(path):
    """Return the harnesses defined in a Rust source file."""
    src = Path(path).read_text(encoding="utf-8", errors="replace")
    code = strip_comments_and_strings(src)
    harnesses = []
    for m in HARNESS_ATTR.finditer(code):
        fn = FN_DECL.search(code, m.end())
        if not fn:
            continue
        # Collect every attribute of the item: walk back from the harness attribute over the
        # preceding lines until reaching the end of the previous item (or a blank line).
        attr_start = code.rfind("\n", 0, m.start()) + 1
        while attr_start > 0:
            prev_start = code.rfind("\n", 0, attr_start - 1) + 1
            prev = code[prev_start: attr_start - 1].strip()
            if not prev or prev[-1] in ";{}":
                break
            attr_start = prev_start
        attr_region = src[attr_start: fn.start()]
        attributes = {}
        for a in ATTR.finditer(attr_region):
            name, arg = a.group(1), (a.group(2) or "").strip()
            attributes.setdefault(name, []).append(arg)
        open_idx = code.find("{", fn.end())
        if open_idx == -1:
            continue
        close_idx = matching_brace(code, open_idx)
        harnesses.append(
            Harness(
                name=fn.group(1),
                file=str(path),
                line=line_of(code, fn.start()),
                kind=m.group(1),
                target=(m.group(2) or "").strip() or None,
                attributes=attributes,
                body=code[open_idx: close_idx + 1],
                body_line=line_of(code, open_idx),
            )
        )
    return harnesses


def rust_files(paths):
    """Expand files and directories into Rust source files, skipping build outputs."""
    for p in map(Path, paths):
        if p.is_dir():
            for f in sorted(p.rglob("*.rs")):
                if not any(part in ("target", ".git") for part in f.parts):
                    yield f
        elif p.suffix == ".rs":
            yield p


def emit(obj):
    print(json.dumps(obj, indent=2, sort_keys=True))
