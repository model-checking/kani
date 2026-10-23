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

FN_DECL = re.compile(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)")


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


def matching(code, open_idx, open_ch, close_ch):
    """Index of the delimiter closing the one at `open_idx` (or the end of `code`)."""
    depth = 0
    for k in range(open_idx, len(code)):
        if code[k] == open_ch:
            depth += 1
        elif code[k] == close_ch:
            depth -= 1
            if depth == 0:
                return k
    return len(code) - 1


def split_top_level(text, sep=","):
    """Split `text` on `sep` outside (), [], {}, and <> nesting."""
    parts, depth, cur = [], 0, ""
    for i, c in enumerate(text):
        if c in "([{<":
            depth += 1
        elif c in ")]}" or (c == ">" and (i == 0 or text[i - 1] != "-")):
            depth -= 1
        if c == sep and depth == 0:
            parts.append(cur)
            cur = ""
        else:
            cur += c
    if cur.strip():
        parts.append(cur)
    return [p.strip() for p in parts]


ATTR_PATH = re.compile(r"\s*(?:cfg_attr\s*\(\s*kani\s*,\s*)?(kani::[a-z_]+|cfg)\s*")


def parse_attributes(src, code, start, end):
    """The attributes in `src[start:end]` as a list of (name, argument text, start offset).
    Names are `kani::<attr>` or `cfg`; other attributes are skipped. Arguments are sliced from
    `src`, so string literals inside them are preserved."""
    attrs = []
    i = code.find("#[", start, end)
    while i != -1:
        close = matching(code, i + 1, "[", "]")
        m = ATTR_PATH.match(code, i + 2)
        if m:
            name = m.group(1)
            arg = ""
            if code.startswith("(", m.end()):
                arg_close = matching(code, m.end(), "(", ")")
                arg = src[m.end() + 1: arg_close].strip()
            attrs.append((name, arg, i))
        i = code.find("#[", close + 1, end)
    return attrs


def item_attr_start(code, item_start):
    """Walk back from an item over its attribute lines, until the end of the previous item or a
    blank line."""
    attr_start = code.rfind("\n", 0, item_start) + 1
    while attr_start > 0:
        prev_start = code.rfind("\n", 0, attr_start - 1) + 1
        prev = code[prev_start: attr_start - 1].strip()
        if not prev or prev[-1] in ";{}":
            break
        attr_start = prev_start
    return attr_start


@dataclass
class Function:
    name: str
    file: str
    line: int
    start: int  # offset of the first attribute
    end: int  # offset just past the closing brace
    attributes: list  # (name, argument, offset) from parse_attributes
    signature: str  # source text from `fn` to the body's `{`
    params: list  # (pattern, type) pairs; `self` receivers have type None
    generic: bool
    scope: str  # "file", "mod", "impl", "trait", "fn", or "other"
    in_kani: bool  # inside `#[cfg(kani)]` code, or a harness
    body: str  # body code, comments and strings blanked
    body_src: str  # body source
    body_line: int

    def attrs(self, name):
        return [arg for n, arg, _ in self.attributes if n == name]


def _scopes(code):
    """For every `{`, its matching `}` and the kind of the block it opens."""
    spans, stack = [], []
    for i, c in enumerate(code):
        if c == "{":
            boundary = max(code.rfind(ch, 0, i) for ch in ";{}")
            header = code[boundary + 1: i]
            if re.search(r"\bimpl\b", header):
                kind = "impl"
            elif re.search(r"\btrait\b", header):
                kind = "trait"
            elif re.search(r"\bmod\s+\w+\s*$", header):
                kind = "mod"
            elif re.search(r"\bfn\b", header):
                kind = "fn"
            else:
                kind = "other"
            stack.append((i, kind, boundary + 1))
        elif c == "}" and stack:
            open_idx, kind, header_start = stack.pop()
            spans.append((open_idx, i, kind, header_start))
    return spans


def parse_functions(src, path="<src>"):
    """Every function with a body in a Rust source text."""
    code = strip_comments_and_strings(src)
    spans = _scopes(code)
    kani_mods = []
    for open_idx, close_idx, kind, header_start in spans:
        # A module's header (the text since the previous item) includes its attributes.
        if kind == "mod":
            if any(n == "cfg" and re.fullmatch(r"\s*kani\s*", a)
                   for n, a, _ in parse_attributes(src, code, header_start, open_idx)):
                kani_mods.append((open_idx, close_idx))
    functions = []
    for fn in FN_DECL.finditer(code):
        # The body starts at the first `{` outside parentheses/brackets; a `;` first means a
        # declaration without a body.
        depth, k = 0, fn.end()
        while k < len(code):
            c = code[k]
            if c in "([":
                depth += 1
            elif c in ")]":
                depth -= 1
            elif depth == 0 and c in "{;":
                break
            k += 1
        if k >= len(code) or code[k] == ";":
            continue
        open_idx = k
        close_idx = matching(code, open_idx, "{", "}")
        enclosing = [s for s in spans if s[0] < fn.start() < s[1]]
        scope = max(enclosing, key=lambda s: s[0])[2] if enclosing else "file"
        attr_start = item_attr_start(code, fn.start())
        attributes = parse_attributes(src, code, attr_start, fn.start())
        signature = src[fn.start():open_idx]
        paren = code.find("(", fn.end())
        params = []
        if paren != -1 and paren < open_idx:
            for p in split_top_level(src[paren + 1: matching(code, paren, "(", ")")]):
                pat, sep, ty = p.partition(":")
                if not sep:
                    params.append((pat.strip(), None))  # `self`, `&self`, `&mut self`
                else:
                    params.append((pat.strip(), ty.strip()))
        in_kani = any(a < fn.start() < b for a, b in kani_mods) or any(
            n == "cfg" and re.fullmatch(r"\s*kani\s*", a) or n in ("kani::proof", "kani::proof_for_contract")
            for n, a, _ in attributes)
        functions.append(Function(
            name=fn.group(1),
            file=str(path),
            line=line_of(code, fn.start()),
            start=attr_start,
            end=close_idx + 1,
            attributes=attributes,
            signature=signature,
            params=params,
            generic=code[fn.end():paren].strip().startswith("<") if paren != -1 else False,
            scope=scope,
            in_kani=in_kani,
            body=code[open_idx: close_idx + 1],
            body_src=src[open_idx: close_idx + 1],
            body_line=line_of(code, open_idx),
        ))
    return functions


@dataclass
class Harness:
    name: str
    file: str
    line: int
    kind: str  # "proof" or "proof_for_contract"
    target: str = None  # contract target for proof_for_contract
    attributes: dict = field(default_factory=dict)  # Kani attribute name -> list of arguments
    body: str = ""  # code with comments/strings blanked
    body_line: int = 0  # line number where `body` starts


def harnesses_of(functions):
    out = []
    for f in functions:
        kinds = [n for n, _, _ in f.attributes if n in ("kani::proof", "kani::proof_for_contract")]
        if not kinds:
            continue
        kind = "proof_for_contract" if "kani::proof_for_contract" in kinds else "proof"
        attributes = {}
        for n, arg, _ in f.attributes:
            if n.startswith("kani::"):
                attributes.setdefault(n[len("kani::"):], []).append(arg)
        target = (f.attrs("kani::proof_for_contract") or [None])[0]
        out.append(Harness(name=f.name, file=f.file, line=f.line, kind=kind, target=target or None,
                           attributes=attributes, body=f.body, body_line=f.body_line))
    return out


def find_harnesses(path):
    """Return the harnesses defined in a Rust source file."""
    src = Path(path).read_text(encoding="utf-8", errors="replace")
    return harnesses_of(parse_functions(src, path))


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
