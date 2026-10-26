# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Shared helpers for the Kani agent plugin scripts.

The static checks here are deliberately lightweight: they scan Rust source text rather than
parse it, so they never need a toolchain and can run before anything compiles. They report
*suspicions* for the agent to review, not verdicts.
"""

import json
import os
import re
import sys
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


QUALIFIERS = re.compile(r'(?:\bpub\s*(?:\([^)]*\))?|\bconst|\bunsafe|\basync|\bdefault|\bextern\s*(?:"[^"]*")?)\s*$')


def item_attr_start(code, item_start):
    """Walk back from an item's keyword over its qualifiers (`pub(crate)`, `const`, `unsafe`, ...)
    and its attributes, matching brackets, so multi-line attributes such as
    `#[kani::ensures(|r| { ... })]` are included."""
    i = item_start
    while True:
        m = QUALIFIERS.search(code[max(0, i - 80):i])
        if not m or m.start() == m.end():
            break
        i -= len(m.group(0))
    while True:
        k = i
        while k > 0 and code[k - 1].isspace():
            k -= 1
        if k == 0 or code[k - 1] != "]":
            return i
        depth, j = 0, k - 1
        while j >= 0:
            if code[j] == "]":
                depth += 1
            elif code[j] == "[":
                depth -= 1
                if depth == 0:
                    break
            j -= 1
        if j <= 0 or code[j - 1] != "#":
            return i
        i = j - 1


def find_root(path):
    """The Cargo workspace root containing `path`, and its package directory; (None, None)
    outside a Cargo project."""
    p = Path(path).resolve()
    start = p if p.is_dir() else p.parent
    package = next((d for d in [start, *start.parents] if (d / "Cargo.toml").exists()), None)
    if package is None:
        return None, None
    # A package only owns its source directories: a loose file elsewhere under the package
    # directory (a fixture, a script) is a single-file target.
    rel = p.relative_to(package).parts
    if rel and rel[0] not in ("src", "tests", "benches", "examples", "build.rs"):
        return None, None
    root = package
    for d in package.parents:
        manifest = d / "Cargo.toml"
        if manifest.exists() and re.search(r"^\s*\[workspace\]", manifest.read_text(errors="replace"), re.M):
            root = d
    return root, package


MOD_DECL = re.compile(r"\bmod\s+(\w+)\s*;")


def kani_only_files(files):
    """Files that are only compiled for Kani: modules declared out of line as
    `#[cfg(kani)] mod name;`, and the modules they declare in turn."""
    files = {Path(f).resolve() for f in files}
    decls = {}  # file -> [(module file candidates, under cfg(kani))]
    for f in files:
        src = f.read_text(encoding="utf-8", errors="replace")
        code = strip_comments_and_strings(src)
        base = f.parent if f.name in ("mod.rs", "lib.rs", "main.rs") else f.parent / f.stem
        out = []
        for m in MOD_DECL.finditer(code):
            start = item_attr_start(code, m.start())
            attrs = src[start:m.start()]
            path_attr = re.search(r'#\[\s*path\s*=\s*"([^"]+)"\s*\]', attrs)
            if path_attr:
                candidates = [(f.parent / path_attr.group(1)).resolve()]
            else:
                candidates = [(base / f"{m.group(1)}.rs").resolve(), (base / m.group(1) / "mod.rs").resolve()]
            out.append((candidates, bool(re.search(r"#\[\s*cfg\s*\(\s*kani\s*\)\s*\]", attrs))))
        decls[f] = out
    kani_only = set()
    changed = True
    while changed:
        changed = False
        for f, entries in decls.items():
            for candidates, cfg_kani in entries:
                if cfg_kani or f in kani_only:
                    for c in candidates:
                        if c in files and c not in kani_only:
                            kani_only.add(c)
                            changed = True
    return kani_only


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
    scope_header: str  # source text before the enclosing block's `{` ("" at file level)
    scope_end: int  # offset just past the enclosing block's `}` (len(src) at file level)
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


def parse_functions(src, path="<src>", all_in_kani=False):
    """Every function with a body in a Rust source text. `all_in_kani` marks every function as
    Kani-only (for files included through `#[cfg(kani)] mod name;`)."""
    code = strip_comments_and_strings(src)
    spans = _scopes(code)
    kani_mods = []
    for open_idx, close_idx, kind, header_start in spans:
        # A module's or impl's header (the text since the previous item) includes its attributes.
        if kind in ("mod", "impl"):
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
        inner = max(enclosing, key=lambda s: s[0]) if enclosing else None
        scope = inner[2] if inner else "file"
        scope_header = src[inner[3]:inner[0]] if inner else ""
        scope_end = inner[1] + 1 if inner else len(src)
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
        in_kani = all_in_kani or any(a < fn.start() < b for a, b in kani_mods) or any(
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
            scope_header=scope_header,
            scope_end=scope_end,
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
                attributes.setdefault(n[len("kani::"):], []).append(re.sub(r"\s+", " ", arg))
        target = (f.attrs("kani::proof_for_contract") or [None])[0]
        out.append(Harness(name=f.name, file=f.file, line=f.line, kind=kind, target=target or None,
                           attributes=attributes, body=f.body, body_line=f.body_line))
    return out


CALL = re.compile(r"(?<![!\w])((?:[A-Za-z_][A-Za-z0-9_]*::)*[A-Za-z_][A-Za-z0-9_]*)\s*(?:::<[^>]*>)?\s*\(")


def is_harness_fn(f):
    return any(n in ("kani::proof", "kani::proof_for_contract") for n, _, _ in f.attributes)


def helper_index(functions):
    """Kani-only helper functions (not harnesses) by name, for `helper_closure`."""
    index = {}
    for f in functions:
        if f.in_kani and not is_harness_fn(f):
            index.setdefault(f.name, []).append(f)
    return index


EXTERNAL_ROOTS = {"kani", "std", "core", "alloc"}
IMPL_TYPE = re.compile(r"\bimpl\b(?:\s*<[^{]*?>)?\s+(?:[^{]*?\bfor\s+)?([A-Za-z_]\w*)")


def resolve_call(caller, path, index):
    """The Kani-only helpers a call `path(...)` in `caller` may refer to. Calls into Kani and
    the standard library never resolve; a qualified call must match the helper's module (file
    or directory name) or, for `Type::f`, its impl block; an unqualified call resolves to free
    functions, preferring those in the caller's file."""
    segments = path.split("::")
    if segments[0] in EXTERNAL_ROOTS:
        return []
    candidates = index.get(segments[-1], [])
    if len(segments) >= 2:
        qualifier = segments[-2]
        if qualifier in ("self", "super", "crate"):
            return [h for h in candidates if h.scope != "impl"]
        return [h for h in candidates
                if (h.scope == "impl" and re.search(rf"\b{re.escape(qualifier)}\b", h.scope_header))
                or (h.scope != "impl" and (qualifier in Path(h.file).parts or Path(h.file).stem == qualifier))]
    free = [h for h in candidates if h.scope != "impl"]
    same_file = [h for h in free if h.file == caller.file]
    return same_file or free


def helper_closure(fn, index):
    """The Kani-only helpers `fn` calls, transitively, and the replacement functions of its
    `#[kani::stub]` attributes: assumptions and nondeterministic inputs in them belong to the
    harness. Method calls (`x.f()`) are not followed: their receiver type is unknown."""
    stub_paths = [arg.split(",")[-1].strip() for n, arg, _ in fn.attributes if n == "kani::stub"]
    seen, out, todo = {id(fn)}, [], [fn]
    while todo:
        f = todo.pop()
        paths = list(stub_paths) if f is fn else []
        for m in CALL.finditer(f.body):
            before = f.body[:m.start(1)].rstrip()
            if before.endswith("."):
                continue
            paths.append(m.group(1))
        for path in paths:
            for h in resolve_call(f, path, index):
                if id(h) not in seen:
                    seen.add(id(h))
                    out.append(h)
                    todo.append(h)
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


def kani_toolchain():
    """An installed Rust toolchain to run Kani's `cargo` proxy with: Kani's own toolchain if it
    can be found, otherwise rustup's default toolchain. None if rustup is not used."""
    import shutil
    import subprocess
    kani = shutil.which("kani") or shutil.which("cargo-kani")
    if kani:
        # A development build: `scripts/kani` in a Kani checkout, next to its rust-toolchain.toml.
        toolchain_file = Path(kani).resolve().parent.parent / "rust-toolchain.toml"
        if toolchain_file.exists():
            m = re.search(r'^\s*channel\s*=\s*"([^"]+)"', toolchain_file.read_text(), re.M)
            if m:
                return m.group(1)
        # A release: `$KANI_HOME/kani-<version>/rust-toolchain-version`.
        home = Path(os.environ.get("KANI_HOME", Path.home() / ".kani"))
        for version_file in sorted(home.glob("kani-*/rust-toolchain-version"), reverse=True):
            return version_file.read_text().strip()
    if shutil.which("rustup"):
        out = subprocess.run(["rustup", "default"], capture_output=True, text=True,
                             env={**os.environ, "RUSTUP_AUTO_INSTALL": "0"})
        if out.returncode == 0 and out.stdout.split():
            return out.stdout.split()[0]
    return None


def kani_env():
    """The environment for running Kani in a user's project. rustup's `cargo` proxy reads the
    project's `rust-toolchain.toml` before Kani starts, and installs that toolchain if it is
    missing; Kani does not need it (it uses its own toolchain). Pin the proxy to an installed
    toolchain and disable auto-install, unless the user already chose a toolchain."""
    env = dict(os.environ)
    env["RUSTUP_AUTO_INSTALL"] = "0"
    if "RUSTUP_TOOLCHAIN" not in env:
        toolchain = kani_toolchain()
        if toolchain:
            env["RUSTUP_TOOLCHAIN"] = toolchain
    return env


IGNORED_DIRS = re.compile(r"^(target.*|\.git|node_modules)$")


def sync_tree(src, dst):
    """Make `dst` a copy of `src` (without build outputs), copying only files whose size or
    modification time differ, so the copy's build cache stays valid between runs."""
    import shutil
    src, dst = Path(src), Path(dst)
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


PATH_DEP = re.compile(r'\bpath\s*=\s*"([^"]+)"')


def external_path_deps(root):
    """Directories of `path` dependencies outside `root`, transitively: the copy must include
    them at the same relative place, or their relative paths would not resolve."""
    deps, todo = set(), [Path(root)]
    while todo:
        top = todo.pop()
        for dirpath, dirnames, filenames in os.walk(top):
            dirnames[:] = [d for d in dirnames if not (d.startswith("target") or d.startswith("."))]
            if "Cargo.toml" not in filenames:
                continue
            text = (Path(dirpath) / "Cargo.toml").read_text(errors="replace")
            for m in PATH_DEP.finditer(text):
                dep = (Path(dirpath) / m.group(1)).resolve()
                if dep.is_dir() and not dep.is_relative_to(root) and dep not in deps \
                        and not any(dep.is_relative_to(d) for d in deps):
                    deps.add(dep)
                    todo.append(dep)
    return deps


def mirror_workspace(root, work):
    """Copy the workspace `root` into `work` (incrementally, see `sync_tree`), together with its
    external path dependencies, under their common ancestor, so that relative
    `path = "../..."` dependencies still resolve. Returns the copy of `root`."""
    root, work = Path(root), Path(work)
    deps = external_path_deps(root)
    anchor = Path(os.path.commonpath([root, *deps])) if deps else root
    copy = work / root.relative_to(anchor)
    sync_tree(root, copy)
    for dep in deps:
        sync_tree(dep, work / dep.relative_to(anchor))
    return copy


def test_only_ranges(src, keep):
    """Source ranges of test-only code, other than the function `keep`: `#[test]` functions and
    `#[cfg(test)]` modules. Removing them lets a single test build when the crate's other tests
    do not compile under `cfg(kani)`."""
    code = strip_comments_and_strings(src)
    keep_span = next(((f.start, f.end) for f in parse_functions(src) if f.name == keep), None)
    ranges = []
    for open_idx, close_idx, kind, header_start in _scopes(code):
        if kind != "mod":
            continue
        attrs = parse_attributes(src, code, header_start, open_idx)
        if any(n == "cfg" and re.fullmatch(r"\s*test\s*", a) for n, a, _ in attrs):
            start = next(off for n, a, off in attrs if n == "cfg")
            if not (keep_span and start <= keep_span[0] < close_idx):
                ranges.append((start, close_idx + 1))
    for f in parse_functions(src):
        if f.name != keep and re.search(r"#\[\s*test\s*\]", src[f.start:f.end]):
            ranges.append((f.start, f.end))
    # Drop ranges nested in others.
    ranges.sort()
    out = []
    for r in ranges:
        if out and r[0] < out[-1][1]:
            continue
        out.append(r)
    return out


def join_option_values(argv, options):
    """Rewrite `--opt VALUE` as `--opt=VALUE` for pass-through options, so that values starting
    with `-` (like `-Z`) are not mistaken for options by argparse."""
    argv = list(sys.argv[1:] if argv is None else argv)
    out, i = [], 0
    while i < len(argv):
        if argv[i] in options and i + 1 < len(argv):
            out.append(f"{argv[i]}={argv[i + 1]}")
            i += 2
        else:
            out.append(argv[i])
            i += 1
    return out


def emit(obj):
    print(json.dumps(obj, indent=2, sort_keys=True))
