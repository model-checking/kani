# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Crate Verifiability Map: the verification structure of a Rust workspace, for planning.

The map joins four sources into one deterministic, versioned JSON document:

- `cargo metadata`: packages, targets, features, workspace dependencies;
- a call graph: from `cargo-callgraph` (rust-analyzer based) when it is installed, otherwise a
  text-scan approximation. Both are approximations of the compiler's call graph, and every edge
  says how it was resolved;
- the plugin's source scan: functions, contracts, harnesses, and cheap structural signals;
- Kani results (`--export-json` files or evidence bundles), merged per harness with the exact
  configuration they were produced under, and a freshness check.

The map is a planning aid, not a proof certificate. A function is `verified` only through
fresh evidence of a harness that provably reaches it (through resolved edges, not through stubs),
and the scope of that evidence (harness, bounds, stubs, weakened checks) is kept with it. A
callee that was not verified is never treated as proven.

The five complexity metrics of the proposed `cargo kani complexity` command are reserved in
`complexity` and reported as `not_computed`; nothing in this module defines or estimates them.
The `signals` are different, text-based counts with their own names and provenance.
"""

import collections
import hashlib
import json
import os
import re
import shutil
import subprocess
from pathlib import Path

from kani_plugin_lib import (
    CALL, _scopes, harnesses_of, kani_env, kani_only_files, parse_functions, rust_files,
    strip_comments_and_strings, test_only_ranges)

SCHEMA_VERSION = "0.1.0"
KIND = "kani-verifiability-map"
# `conditional`: verified, but only under assumptions (stubs, callee contracts) that nothing verifies.
STATUSES = ("verified", "conditional", "failed", "inconclusive", "unsupported", "not_attempted", "stale")
COMPLEXITY_METRICS = ("branches", "loops", "function_calls", "arithmetic_operations", "memory_operations")

LOOP_KW = re.compile(r"\b(?:for|while|loop)\b")
BRANCH_KW = re.compile(r"\b(?:if|match)\b")
UNSAFE_KW = re.compile(r"\bunsafe\b")
ASSUME_KW = re.compile(r"\bkani::assume\s*\(|\bkani::any_where\s*\(")
UNBOUNDED_TYPES = re.compile(
    r"&\s*(?:mut\s+)?(?:str|\[)|\bVec<|\bString\b|\bBox<|\bdyn\b|\bimpl\b|\bHashMap<|\bHashSet<")
SKIPPED_CALLS = {"if", "while", "for", "match", "return", "Some", "Ok", "Err", "loop", "in", "Box::new", "Vec::new"}
INTERNAL_PREFIXES = ("kani::", "std::", "core::", "alloc::")
AMBIGUOUS_LIMIT = 5


# --------------------------------------------------------------------------------------
# Discovery
# --------------------------------------------------------------------------------------

def run(cmd, cwd, timeout=600):
    return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout, env=kani_env())


def discover_workspace(root):
    """Packages, targets, features, and workspace dependencies from `cargo metadata`."""
    out = run(["cargo", "metadata", "--no-deps", "--format-version", "1"], root)
    if out.returncode != 0:
        raise RuntimeError(f"`cargo metadata` failed: {out.stderr.strip()[-300:]}")
    meta = json.loads(out.stdout)
    ws_root = Path(meta["workspace_root"]).resolve()
    members = set(meta["workspace_members"])
    names = {p["name"] for p in meta["packages"] if p["id"] in members}
    packages = []
    for p in sorted(meta["packages"], key=lambda p: p["name"]):
        if p["id"] not in members:
            continue
        manifest = Path(p["manifest_path"]).resolve()
        packages.append({
            "name": p["name"],
            "crate": p["name"].replace("-", "_"),
            "version": p["version"],
            "edition": p["edition"],
            "manifest": str(manifest.relative_to(ws_root)),
            "dir": str(manifest.parent.relative_to(ws_root)) or ".",
            "targets": sorted(({"name": t["name"], "kind": sorted(t["kind"]),
                                "src": str(Path(t["src_path"]).resolve().relative_to(ws_root)),
                                "required_features": sorted(t.get("required-features", []))}
                               for t in p["targets"]), key=lambda t: (t["src"], t["name"])),
            "features": {k: sorted(v) for k, v in sorted(p["features"].items())},
            "workspace_dependencies": sorted(d["name"] for d in p["dependencies"] if d["name"] in names),
        })
    return ws_root, packages


def single_file_workspace(path):
    path = Path(path).resolve()
    return path.parent, [{
        "name": path.stem, "crate": path.stem.replace("-", "_"), "version": "0.0.0", "edition": None,
        "manifest": None, "dir": ".",
        "targets": [{"name": path.stem, "kind": ["bin"], "src": path.name, "required_features": []}],
        "features": {}, "workspace_dependencies": []}]


def tool_versions():
    versions = {"cargo-callgraph": None, "kani": None}
    if shutil.which("cargo-callgraph"):
        listing = subprocess.run(["cargo", "install", "--list"], capture_output=True, text=True, env=kani_env())
        m = re.search(r"^cargo-callgraph v(\S+)", listing.stdout, re.M)
        versions["cargo-callgraph"] = m.group(1) if m else "unknown"
    for cmd in (["cargo", "kani", "--version"], ["kani", "--version"]):
        try:
            out = subprocess.run(cmd, capture_output=True, text=True, timeout=60, env=kani_env())
        except (OSError, subprocess.TimeoutExpired):
            continue
        m = re.search(r"(\d+\.\d+\.\d+)", out.stdout)
        if out.returncode == 0 and m:
            versions["kani"] = m.group(1)
            break
    return versions


# --------------------------------------------------------------------------------------
# Function index
# --------------------------------------------------------------------------------------

def module_chain(src, code, offset):
    """Names of the inline `mod` blocks enclosing `offset`, outermost first."""
    chain = []
    for open_idx, close_idx, kind, header_start in sorted(_scopes(code), key=lambda s: s[0]):
        if kind == "mod" and open_idx < offset < close_idx:
            m = re.search(r"\bmod\s+(\w+)\s*$", code[header_start:open_idx])
            if m:
                chain.append(m.group(1))
    return chain


def file_modules(rel_path, target_srcs):
    """Module path implied by a file's location (`src/a/b.rs` -> a::b; roots -> none)."""
    p = Path(rel_path)
    if rel_path in target_srcs:
        return []
    parts = list(p.parts)
    if "src" in parts:
        parts = parts[parts.index("src") + 1:]
    parts[-1] = p.stem
    if parts[-1] in ("mod", "lib", "main"):
        parts = parts[:-1]
    return parts


def impl_parts(header):
    """(self type, trait or None) of an impl header; (None, None) if it is not one."""
    h = re.sub(r"#\[[^\]]*\]", " ", header).strip()
    m = re.match(r"(?:unsafe\s+)?impl\b(?:\s*<.*?>)?\s+(?:(!?[\w:]+(?:<.*?>)?)\s+for\s+)?([A-Za-z_][\w:]*)", h, re.S)
    if not m:
        return None, None
    return m.group(2), m.group(1)


def source_dirs(root, pkg):
    """The directories holding the code of a package's library and binary targets. Tests,
    examples, and benchmarks are not verification targets, and a package may sit at the
    repository root (so its directory also holds unrelated trees)."""
    pkg_dir = (root / pkg["dir"]).resolve()
    dirs = set()
    for t in pkg["targets"]:
        if set(t["kind"]) & {"lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro", "bin"}:
            dirs.add((root / t["src"]).resolve().parent)
    # Keep only the outermost directories.
    return [d for d in sorted(dirs) if not any(o != d and d.is_relative_to(o) for o in dirs)] or [pkg_dir]


def build_index(root, packages):
    """All functions with bodies in each package's sources, with structural signals."""
    functions = []
    for pkg in packages:
        files = sorted({f for d in source_dirs(root, pkg) for f in rust_files([d])})
        kani_only = kani_only_files(files)
        target_srcs = {t["src"] for t in pkg["targets"]}
        for f in files:
            rel = str(f.resolve().relative_to(root))
            src = f.read_text(encoding="utf-8", errors="replace")
            code = strip_comments_and_strings(src)
            tests = test_only_ranges(src, "")
            for fn in parse_functions(src, f, f.resolve() in kani_only):
                if any(a <= fn.start < b for a, b in tests):
                    continue
                kw = src.find(fn.signature, fn.start)
                prefix = code[max(0, kw - 60):kw]
                pre = re.search(
                    r"((?:pub(?:\s*\([^)]*\))?\s+)?(?:(?:const|unsafe|async|extern(?:\s+\"[^\"]*\")?)\s+)*)$", prefix)
                qualifiers = pre.group(1) if pre else ""
                self_ty, trait = impl_parts(fn.scope_header) if fn.scope == "impl" else (None, None)
                trait_ty = (self_ty, trait)
                inline = module_chain(src, code, kw)
                path = [pkg["crate"], *file_modules(rel, target_srcs), *inline]
                if self_ty:
                    path.append(f"<{self_ty} as {trait}>" if trait else self_ty)
                path.append(fn.name)
                body = fn.body
                unbounded = sorted({p[1] for p in fn.params if p[1] and UNBOUNDED_TYPES.search(p[1])})
                functions.append({
                    "_fn": fn,
                    "package": pkg["name"], "file": rel, "line": fn.line,
                    "end_line": fn.line + src.count("\n", kw, fn.end),
                    "qualified_path": "::".join(path), "name": fn.name, "self_type": self_ty, "trait": trait,
                    "in_trait_impl": trait_ty[1] is not None,
                    "scope": fn.scope, "generic": fn.generic,
                    "public": "pub" in qualifiers, "unsafe_fn": "unsafe" in qualifiers,
                    "kani_only": fn.in_kani,
                    "signals": {
                        "loop_keywords": len(LOOP_KW.findall(body)),
                        "branch_keywords": len(BRANCH_KW.findall(body)),
                        "unsafe_blocks": len(UNSAFE_KW.findall(body)),
                        "parameters": len(fn.params),
                        "unbounded_input_types": unbounded,
                    },
                    "contracts": {k: [re.sub(r"\s+", " ", a) for a in fn.attrs(f"kani::{k}")]
                                  for k in ("requires", "ensures", "modifies") if fn.attrs(f"kani::{k}")},
                    "has_loop_contract": bool(fn.attrs("kani::loop_invariant")),
                    "cfg": sorted(a for n, a, _ in fn.attributes if n == "cfg" and a.strip() not in ("kani", "test")),
                })
    # Stable IDs: crate path, disambiguated by file, then by order of appearance.
    functions.sort(key=lambda f: (f["file"], f["line"], f["name"]))
    seen = collections.Counter(f["qualified_path"] for f in functions)
    used = collections.Counter()
    for f in functions:
        base = f["qualified_path"]
        if seen[base] > 1:
            base = f"{base}~{f['file']}"
        used[base] += 1
        f["id"] = base if used[base] == 1 else f"{base}#{used[base]}"
    functions.sort(key=lambda f: f["id"])
    return functions


# --------------------------------------------------------------------------------------
# Call graph
# --------------------------------------------------------------------------------------

def run_callgraph_tool(root, manifest, timeout=900):
    """Run `cargo callgraph`; returns the parsed JSON or raises."""
    out_file = Path(root) / "target" / "kani-plugin" / "callgraph.json"
    out_file.parent.mkdir(parents=True, exist_ok=True)
    cmd = ["cargo", "callgraph", "--json", str(out_file)]
    if manifest:
        cmd += ["--manifest-path", str(manifest)]
    out = run(cmd, root, timeout)
    if out.returncode != 0 or not out_file.exists():
        raise RuntimeError(f"`cargo callgraph` failed: {(out.stderr or out.stdout).strip()[-300:]}")
    return json.loads(out_file.read_text())


def rel_to(root, path):
    try:
        return str(Path(path).resolve().relative_to(root))
    except ValueError:
        return None


EDGE_KINDS = {"DirectCall": "direct", "StaticTraitCall": "trait-impl-candidate",
              "TraitDefCall": "trait-dispatch"}


def edges_from_callgraph(root, functions, cg):
    """Join cargo-callgraph nodes to indexed functions by (file, line, name). The tool numbers
    lines from 0 (and calls the name's line, which is the `fn` line for ordinary functions)."""
    by_key = {(f["file"], f["line"], f["name"]): f["id"] for f in functions}
    nodes = {}
    unmatched = 0
    for n in cg["nodes"]:
        rel = rel_to(root, n["file"]) if n.get("is_workspace") else None
        fid = by_key.get((rel, n["line"] + 1, n["name"])) if rel else None
        nodes[n["id"]] = {"function": fid, "name": n["qualified_path"], "workspace": bool(n.get("is_workspace")),
                          "trait_method": bool(n.get("is_trait_method"))}
        if n.get("is_workspace") and not fid:
            unmatched += 1
    edges = []
    for e in cg["edges"]:
        src, dst = nodes.get(e["source"]), nodes.get(e["target"])
        if not src or not src["function"]:
            continue
        if dst and dst["workspace"] and not dst["function"] and (
                dst["name"] == "impl" or ("::" not in dst["name"] and dst["name"][:1].isupper())):
            continue  # tuple-struct constructors and impl blocks are not functions
        kind = EDGE_KINDS.get(e["kind"], "unknown")
        edge = {"from": src["function"], "kind": kind, "source": "cargo-callgraph",
                "call_site": {"file": rel_to(root, e["call_site_file"]), "line": e["call_site_line"] + 1},
                "raw_kind": e["kind"]}
        if dst and dst["function"] and kind != "trait-dispatch":
            edge.update(to=dst["function"], resolved=kind == "direct")
        else:
            external = bool(dst) and not dst["workspace"]
            edge.update(to=None, to_name=dst["name"] if dst else e["target"].split("@")[0],
                        external=external, resolved=False)
            if external and kind == "direct":
                edge["kind"] = "external"
        edges.append(edge)
    return edges, unmatched


def resolve_name(path, caller, functions, by_name):
    """Candidate callees of a textual call `path(...)`: (function ids, how)."""
    segs = path.split("::")
    if segs[0] in ("kani", "std", "core", "alloc"):
        return [], "library"
    cands = by_name.get(segs[-1], [])
    if len(segs) >= 2:
        q = segs[-2]
        cands = [c for c in cands if (c["self_type"] == q) or q in c["qualified_path"].split("::")
                 or q in ("self", "Self", "super", "crate")]
        how = "path-match"
    else:
        free = [c for c in cands if c["scope"] != "impl"]
        same = [c for c in free if c["file"] == caller["file"]]
        cands = same or [c for c in free if c["package"] == caller["package"]] or free
        how = "name-match"
    return [c["id"] for c in cands], how


def edges_from_text(functions):
    """Fallback call graph: calls found by scanning function bodies, resolved by name only.
    Every edge is a candidate (`resolved: false`) except unique path-qualified matches."""
    by_name = collections.defaultdict(list)
    for f in functions:
        by_name[f["name"]].append(f)
    edges = []
    for f in functions:
        fn = f["_fn"]
        for m in CALL.finditer(fn.body):
            path, is_method = m.group(1), fn.body[:m.start(1)].rstrip().endswith(".")
            if path in SKIPPED_CALLS or path.startswith(INTERNAL_PREFIXES):
                continue
            if "::" not in path and path[:1].isupper() and not is_method:
                continue  # tuple-struct or enum-variant constructor, not a function call
            if is_method:
                cands = [c["id"] for c in by_name.get(path, []) if c["self_type"]]
                how = "method-name-match"
                if len(cands) > AMBIGUOUS_LIMIT:
                    # `new`, `len`, `from`: without types the receiver is unknown, and the callee is most
                    # likely a library method. One external edge, not hundreds of spurious candidates.
                    edges.append({"from": f["id"], "to": None, "to_name": path, "kind": "external",
                                  "resolved": False, "external": True, "source": "text-scan",
                                  "ambiguous": len(cands), "call_site": {"file": f["file"], "line": fn.body_line
                                  + fn.body.count("\n", 0, m.start(1))}})
                    continue
            else:
                cands, how = resolve_name(path, f, functions, by_name)
            line = fn.body_line + fn.body.count("\n", 0, m.start(1))
            site = {"file": f["file"], "line": line}
            if not cands:
                edges.append({"from": f["id"], "to": None, "to_name": path, "kind": "external", "resolved": False,
                              "external": True, "source": "text-scan", "call_site": site})
            for c in cands:
                unique = len(cands) == 1 and how == "path-match" or (len(cands) == 1 and how == "name-match")
                edges.append({"from": f["id"], "to": c, "kind": "direct" if unique else "name-candidate",
                              "resolved": unique, "source": "text-scan", "match": how, "call_site": site})
    return edges


def dedupe_edges(edges):
    seen, out = set(), []
    for e in edges:
        key = (e["from"], e.get("to"), e.get("to_name"), e["kind"], e["call_site"]["file"], e["call_site"]["line"])
        if key not in seen:
            seen.add(key)
            out.append(e)
    return sorted(out, key=lambda e: (e["from"], e["call_site"]["file"] or "", e["call_site"]["line"],
                                      e.get("to") or "", e.get("to_name") or "", e["kind"]))


# --------------------------------------------------------------------------------------
# Graph analysis
# --------------------------------------------------------------------------------------

def sccs(ids, succ):
    """Tarjan's algorithm (iterative). Returns components in reverse topological order
    (callees before callers), each as a sorted list."""
    index, low, on, stack, comps, counter = {}, {}, set(), [], [], [0]
    for root in sorted(ids):
        if root in index:
            continue
        work = [(root, iter(sorted(succ.get(root, ()))))]
        index[root] = low[root] = counter[0]
        counter[0] += 1
        stack.append(root)
        on.add(root)
        while work:
            v, it = work[-1]
            advanced = False
            for w in it:
                if w not in index:
                    index[w] = low[w] = counter[0]
                    counter[0] += 1
                    stack.append(w)
                    on.add(w)
                    work.append((w, iter(sorted(succ.get(w, ())))))
                    advanced = True
                    break
                if w in on:
                    low[v] = min(low[v], index[w])
            if advanced:
                continue
            work.pop()
            if work:
                low[work[-1][0]] = min(low[work[-1][0]], low[v])
            if low[v] == index[v]:
                comp = []
                while True:
                    w = stack.pop()
                    on.discard(w)
                    comp.append(w)
                    if w == v:
                        break
                comps.append(sorted(comp))
    return comps


def transitive(start, adj):
    seen, todo = set(), list(start)
    while todo:
        v = todo.pop()
        for w in adj.get(v, ()):
            if w not in seen:
                seen.add(w)
                todo.append(w)
    return seen


def analyze_graph(functions, edges):
    """Callers/callees, strongly connected components, and levels (leaves are level 0)."""
    ids = {f["id"] for f in functions}
    succ, pred = collections.defaultdict(set), collections.defaultdict(set)
    for e in edges:
        if e.get("to") in ids and e["resolved"]:
            succ[e["from"]].add(e["to"])
            pred[e["to"]].add(e["from"])
    maybe = collections.defaultdict(set)
    for e in edges:
        if e.get("to") in ids and not e["resolved"]:
            maybe[e["from"]].add(e["to"])
    comps = sccs(ids, succ)
    comp_of = {}
    for i, comp in enumerate(comps):
        for v in comp:
            comp_of[v] = i
    level = {}
    for i, comp in enumerate(comps):  # callees come first
        deps = {comp_of[w] for v in comp for w in succ.get(v, ()) if comp_of[w] != i}
        level[i] = 1 + max((level[d] for d in deps), default=-1)
    components = []
    for i, comp in enumerate(comps):
        recursive = len(comp) > 1 or comp[0] in succ.get(comp[0], ())
        components.append({"id": f"scc{i}", "members": comp, "recursive": recursive, "level": level[i],
                           "depends_on": sorted(f"scc{d}" for d in {comp_of[w] for v in comp
                                                                    for w in succ.get(v, ())} - {i})})
    return succ, pred, maybe, components, comp_of


# --------------------------------------------------------------------------------------
# Harnesses and relations to functions
# --------------------------------------------------------------------------------------

def scan_harnesses(root, functions):
    out = []
    for f in functions:
        hs = harnesses_of([f["_fn"]])
        if not hs:
            continue
        h = hs[0]
        a = h.attributes
        out.append({
            "id": f"{f['package']}::{f['qualified_path'].split('::', 1)[1]}",
            "name": h.name, "package": f["package"], "file": f["file"], "line": f["line"],
            "kind": h.kind, "contract_target": h.target,
            "attributes": {"unwind": a.get("unwind", [None])[0], "stubs": a.get("stub", []),
                           "verified_stubs": a.get("stub_verified", []), "should_panic": "should_panic" in a,
                           "solver": a.get("solver", [None])[0]},
            "assumptions": len(ASSUME_KW.findall(f["_fn"].body)),
            "_function_id": f["id"],
        })
    return sorted(out, key=lambda h: (h["file"], h["line"]))


def relations(harnesses, functions, edges, succ):
    """For each harness: the functions it targets, provably reaches, may reach, stubs, or assumes."""
    ids = {f["id"] for f in functions}
    by_name = collections.defaultdict(list)
    for f in functions:
        if not f["kani_only"]:
            by_name[f["name"]].append(f)
    fn_by_id = {f["id"]: f for f in functions}
    edges_from = collections.defaultdict(list)
    for e in edges:
        edges_from[e["from"]].append(e)

    def lookup(path, caller):
        segs = re.sub(r"<[^>]*>", "", path).split("::")
        cands = by_name.get(segs[-1], [])
        if len(segs) >= 2:
            q = segs[-2]
            cands = [c for c in cands if c["self_type"] == q or q in c["qualified_path"].split("::")]
        return [c["id"] for c in cands]

    for h in harnesses:
        caller = fn_by_id[h["_function_id"]]
        body = caller["_fn"].body
        stubbed, assumed, targets = set(), set(), set()
        for spec in h["attributes"]["stubs"]:
            stubbed.update(lookup(spec.split(",")[0].strip(), caller))
        for spec in h["attributes"]["verified_stubs"]:
            assumed.update(lookup(spec.strip(), caller))
        if h["contract_target"]:
            targets.update(lookup(h["contract_target"], caller))
        direct = set()
        for m in CALL.finditer(body):
            p = m.group(1)
            if p in SKIPPED_CALLS or p.startswith(INTERNAL_PREFIXES) or p == h["name"]:
                continue
            direct.update(lookup(p, caller))
        roots = (targets | direct) - stubbed - assumed
        sure, unsure = set(), set()
        todo = list(roots)
        sure.update(roots)
        while todo:
            v = todo.pop()
            for w in succ.get(v, ()):
                if w not in sure and w not in stubbed and w not in assumed:
                    sure.add(w)
                    todo.append(w)
        for v in sure:
            for e in edges_from.get(v, ()):
                if e.get("to") in ids and not e["resolved"] and e["to"] not in sure \
                        and e["to"] not in stubbed and e["to"] not in assumed:
                    unsure.add(e["to"])
        h["reaches"] = {
            "contract_target": sorted(targets & ids), "reachable": sorted(sure & ids),
            "maybe_reachable": sorted(unsure), "stubbed": sorted(stubbed & ids),
            "contract_assumed": sorted(assumed & ids)}
        # Only a name-matched direct callee is certain if the match was unique.
        ambiguous = [p for p in {m.group(1) for m in CALL.finditer(body)}
                     if len(lookup(p, caller)) > 1 and not p.startswith(INTERNAL_PREFIXES)]
        if ambiguous:
            h["reaches"]["ambiguous_calls"] = sorted(ambiguous)
    return harnesses


# --------------------------------------------------------------------------------------
# Kani results
# --------------------------------------------------------------------------------------

def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load_results(paths):
    """Load `--export-json` files and evidence bundle directories. Returns a list of runs."""
    runs = []
    for p in paths:
        p = Path(p)
        bundle = None
        if p.is_dir():
            bundle = json.loads((p / "bundle.json").read_text())
            export = p / "results.json"
        else:
            export = p
        doc = json.loads(export.read_text())
        runs.append({"path": str(p), "doc": doc, "bundle": bundle, "mtime": export.stat().st_mtime})
    return runs


def run_freshness(run, root, package_files):
    """(fresh: True/False/None, reason). Bundles are compared by source hashes; bare exports by
    comparing the start of the run with the modification time of the package sources."""
    bundle = run["bundle"]
    if bundle:
        cwd = Path(bundle["cwd"])
        for rel, digest in bundle["sources_sha256"].items():
            f = cwd / rel
            if not f.exists() or sha256(f) != digest:
                return False, f"source {rel} changed since the run"
        return True, "source hashes match the evidence bundle"
    started = run["doc"].get("started_at")
    if not started:
        return None, "the export has no start time; freshness is unknown"
    import calendar
    import time
    start = calendar.timegm(time.strptime(started, "%Y-%m-%dT%H:%M:%SZ"))
    newer = [str(f.relative_to(root)) for f in package_files if f.stat().st_mtime > start + 1]
    if newer:
        return False, f"{newer[0]} was modified after the run started"
    return True, "no source was modified after the run started"


def outcome_status(h):
    o = h.get("outcome", {})
    if o.get("kind") != "COMPLETED":
        return "inconclusive"
    if h.get("unsupported_constructs"):
        return "unsupported"
    return "verified" if o.get("verdict") == "SUCCESS" else "failed"


def merge_results(harnesses, runs, root, package_files):
    """Attach the newest matching result of each harness, with its configuration and freshness."""
    for h in harnesses:
        h.pop("result", None)
    by_name = collections.defaultdict(list)
    for h in harnesses:
        by_name[h["name"]].append(h)
    for run in sorted(runs, key=lambda r: r["mtime"]):
        fresh, why = run_freshness(run, root, package_files)
        doc = run["doc"]
        cfg = doc.get("configuration", {})
        for eh in doc.get("harnesses", []):
            name = eh["name"].split("::")[-1]
            for h in by_name.get(name, []):
                if not eh.get("file") or not (h["file"].endswith(eh["file"]) or eh["file"].endswith(h["file"])):
                    continue
                checks = cfg.get("checks", {})
                h["result"] = {
                    "status": outcome_status(eh), "verdict": eh.get("outcome", {}).get("verdict")
                    or eh.get("outcome", {}).get("kind"),
                    "fresh": fresh, "freshness": why, "source": os.path.basename(run["path"]),
                    "kani_version": doc.get("tools", {}).get("kani"),
                    "kani_commit": doc.get("kani_commit"),
                    "unwind": eh.get("resolved_unwind"), "solver": eh.get("resolved_solver"),
                    "bounded": bool(eh.get("is_bounded")), "time_s": eh.get("resources", {}).get("verification_time_s"),
                    "weakened_checks": sorted(k for k, bad in (("memory_safety", False), ("overflow", False),
                                                               ("unwinding", False), ("undefined_function", False),
                                                               ("assertion_reach_checks", False),
                                                               ("assert_contracts", False),
                                                               ("prove_safety_only", True)) if checks.get(k) is bad),
                    "cbmc_args": cfg.get("cbmc_args", []),
                    "unsupported_constructs": len(eh.get("unsupported_constructs", [])),
                    "failed_properties": [p.get("description") for p in eh.get("failed_properties", [])][:3],
                    "stubs": [f"{s['original']} -> {s['replacement']}" for s in eh.get("attributes", {}).get("stubs", [])],
                    "verified_stubs": eh.get("attributes", {}).get("verified_stubs", []),
                }
    return harnesses


PRECEDENCE = ("failed", "inconclusive", "unsupported", "verified")


def function_verification(fid, harnesses, fn_status_by_id=None):
    """Summarize the evidence about one function. Only fresh results of harnesses that provably
    reach the function (or target its contract) count toward `verified`."""
    evidence, caveats = [], []
    for h in harnesses:
        r = h.get("reaches", {})
        relation = None
        if fid in r.get("contract_target", []):
            relation = "contract-target"
        elif fid in r.get("reachable", []):
            relation = "reachable"
        elif fid in r.get("maybe_reachable", []):
            relation = "maybe-reachable"
        elif fid in r.get("stubbed", []):
            relation = "stubbed-out"
        elif fid in r.get("contract_assumed", []):
            relation = "contract-assumed"
        if not relation:
            continue
        res = h.get("result")
        item = {"harness": h["id"], "relation": relation, "status": res["status"] if res else "not_attempted"}
        if res:
            item.update(fresh=res["fresh"], freshness=res["freshness"], verdict=res["verdict"],
                        unwind=res["unwind"], bounded=res["bounded"], time_s=res["time_s"],
                        kani_version=res["kani_version"], weakened_checks=res["weakened_checks"],
                        assumptions=h["assumptions"], stubs=res["stubs"], verified_stubs=res["verified_stubs"])
        evidence.append(item)
    counting = [e for e in evidence if e["relation"] in ("contract-target", "reachable") and "verdict" in e]
    fresh = [e for e in counting if e.get("fresh") is not False]
    if not evidence or not any(e["relation"] in ("contract-target", "reachable") for e in evidence):
        status = "not_attempted"
        if any(e["relation"] == "maybe-reachable" for e in evidence):
            caveats.append("only reachable through an unresolved or name-matched call edge: not counted")
    elif not counting:
        status = "not_attempted"
    elif not fresh:
        status = "stale"
    else:
        statuses = {e["status"] for e in fresh}
        status = next(s for s in PRECEDENCE if s in statuses)
        if status == "verified":
            for e in fresh:
                if e["status"] != "verified":
                    continue
                if e.get("weakened_checks"):
                    caveats.append(f"{e['harness']}: checks disabled: {', '.join(e['weakened_checks'])}")
                if e.get("freshness") and e.get("fresh") is None:
                    caveats.append(f"{e['harness']}: freshness unknown")
                if e.get("bounded"):
                    caveats.append(f"{e['harness']}: holds only up to the bounds of its bounded inputs")
                if e.get("stubs"):
                    caveats.append(f"{e['harness']}: relies on stubs {e['stubs']}")
                for v in e.get("verified_stubs", []):
                    caveats.append(
                        f"{e['harness']}: assumes the contract of {v} (verified only if {v} has a verified contract proof)")
                if e.get("assumptions"):
                    caveats.append(f"{e['harness']}: {e['assumptions']} assumption(s) restrict its inputs")
    return {"status": status, "scope": "harness-bounded" if status == "verified" else None,
            "evidence": evidence, "caveats": sorted(set(caveats))}


# --------------------------------------------------------------------------------------
# Strategy and planning
# --------------------------------------------------------------------------------------

def strategy_for(f, comp, status, callees_unverified):
    """Documented heuristic: which Kani workflow to try first. Not a guarantee of success."""
    s = f["signals"]
    reasons, tags = [], []
    if comp["recursive"]:
        tags.append("contract-and-recursion")
        reasons.append(f"in a recursive component of {len(comp['members'])} function(s): needs a function contract "
                       "for every member and recursion handling")
    if s["loop_keywords"]:
        tags.append("unwind-or-loop-contract")
        reasons.append(f"{s['loop_keywords']} loop keyword(s): choose an unwind bound or a loop invariant")
    if f["generic"]:
        tags.append("instantiate-concrete-types")
        reasons.append("generic: verify concrete instantiations")
    if f["unresolved_callees"]:
        tags.append("stub-or-contract-callees")
        reasons.append(f"{len(f['unresolved_callees'])} callee(s) not resolved exactly (trait dispatch, dynamic, "
                       "external): stub them or give them contracts")
    if s["unbounded_input_types"]:
        tags.append("bounded-inputs")
        reasons.append(f"unbounded input types {s['unbounded_input_types']}: bound them (bounded_any, "
                       "--bounded-arguments)")
    if s["unsafe_blocks"] or f["unsafe_fn"]:
        tags.append("ub-focused-harness")
        reasons.append("unsafe code: also use -Z valid-value-checks / -Z uninit-checks")
    if status == "failed":
        primary = "triage-failure"
        reasons.insert(0, "a harness that reaches it failed: replay the counterexample first")
    elif status == "inconclusive":
        primary = "reduce-or-split"
        reasons.insert(0, "a run was inconclusive (timeout, out of memory, crash): shrink bounds, stub callees, split")
    elif status == "unsupported":
        primary = "avoid-unsupported-construct"
        reasons.insert(0, "Kani reported unsupported constructs: stub them or restructure")
    elif status == "stale":
        primary = "re-run"
        reasons.insert(0, "the evidence is stale: the sources changed after it was produced")
    elif status == "verified":
        primary = "none"
    elif tags:
        primary = tags[0]
    elif f["has_harness"]:
        primary = "run-existing-harness"
        reasons.append("a harness already exists")
    elif not f["callees"] and not f["unresolved_callees"]:
        primary = "autoharness-candidate"
        reasons.append("leaf function with no structural obstacles: try AutoHarness or a direct harness")
    else:
        primary = "harness-with-callee-contracts"
        reasons.append("call the function from a harness; use verified contracts (stub_verified) for its callees")
    if callees_unverified and primary not in ("none",):
        reasons.append(f"{len(callees_unverified)} resolved callee(s) are not verified: prove them first or "
                       "cover them with a contract")
    return {"primary": primary, "also": [t for t in tags if t != primary], "rationale": reasons}


def priority_of(f, transitive_callers):
    """priority = impact / effort, a heuristic (documented in the design doc)."""
    s = f["signals"]
    effort = (1 + 2 * s["loop_keywords"] + 0.5 * s["branch_keywords"] + 0.5 * len(f["callees"])
              + 3 * f["recursive"] + 2 * len(f["unresolved_callees"]) + f["generic"]
              + len(s["unbounded_input_types"]))
    impact = 1 + transitive_callers + (2 if f["public"] else 0)
    return round(impact / effort, 3), {"impact": impact, "effort": effort}


# --------------------------------------------------------------------------------------
# Assembly
# --------------------------------------------------------------------------------------

def package_sources(root, packages):
    """Relative path -> SHA-256 of every Rust source and manifest of the workspace packages."""
    files = {}
    for pkg in packages:
        for f in {f for d in source_dirs(root, pkg) for f in rust_files([d])}:
            files[str(f.resolve().relative_to(root))] = sha256(f)
        if pkg["manifest"] and (root / pkg["manifest"]).exists():
            files[pkg["manifest"]] = sha256(root / pkg["manifest"])
    return dict(sorted(files.items()))


def global_uncertainty(root, packages, source, unmatched, sources):
    notes = []
    if source == "text-scan":
        notes.append({"flag": "approximate-call-graph", "reason": "calls were found by scanning source text and "
                      "resolved by name; method calls, trait dispatch, and macros are not resolved exactly. "
                      "Install cargo-callgraph for a rust-analyzer based graph."})
        notes.append({"flag": "cfg-ignored", "reason": "the text scan does not evaluate #[cfg] attributes: "
                      "code for every feature and target is indexed"})
    else:
        notes.append({"flag": "approximate-call-graph", "reason": "cargo-callgraph is rust-analyzer based: "
                      "dynamic dispatch is reported as trait-level edges, closures and macro-generated calls "
                      "may be missing, and it cannot be given a feature set"})
        notes.append({"flag": "features-not-controllable", "reason": "the call graph uses the tool's default "
                      "workspace configuration; other features or targets may have other call edges"})
    if unmatched:
        notes.append({"flag": "unmatched-callgraph-nodes", "reason": f"{unmatched} workspace node(s) of the call "
                      "graph (tuple-struct constructors, impl blocks) matched no function and were dropped"})
    macros = [rel for rel in sources if rel.endswith(".rs")
              and "macro_rules!" in (root / rel).read_text(encoding="utf-8", errors="replace")]
    if macros:
        notes.append({"flag": "macro-generated-items", "reason": "macro_rules! definitions found; functions and "
                      f"harnesses generated by macros are not visible ({', '.join(macros[:3])})"})
    ffi = [rel for rel in sources if rel.endswith(".rs")
           and re.search(r'extern\s+"C"\s*\{', (root / rel).read_text(encoding="utf-8", errors="replace"))]
    if ffi:
        notes.append({"flag": "ffi", "reason": f"extern \"C\" blocks found ({', '.join(ffi[:3])}): foreign functions "
                      "cannot be verified by Kani and are not in the map"})
    notes.append({"flag": "complexity-not-computed", "reason": "the five complexity metrics need the proposed "
                  "`cargo kani complexity` command; `signals` are text-based counts, not those metrics"})
    return notes


def build_map(path, results=(), callgraph="auto", callgraph_json=None, timeout=900):
    """Build the Crate Verifiability Map of the Cargo workspace (or single file) at `path`."""
    path = Path(path).resolve()
    if path.is_file():
        root, packages = single_file_workspace(path)
        manifest = None
    else:
        root, packages = discover_workspace(path)
        manifest = root / "Cargo.toml"
    all_functions = build_index(root, packages)
    functions = [f for f in all_functions if not f["kani_only"]]
    cg, source, unmatched, note = None, "text-scan", 0, None
    if callgraph_json:
        cg = json.loads(Path(callgraph_json).read_text())
    elif callgraph != "text" and manifest and shutil.which("cargo-callgraph"):
        try:
            cg = run_callgraph_tool(root, manifest, timeout)
        except (RuntimeError, subprocess.TimeoutExpired) as e:
            if callgraph == "tool":
                raise
            note = f"cargo-callgraph failed ({e}); fell back to the text scan"
    elif callgraph == "tool":
        raise RuntimeError("cargo-callgraph is not installed (cargo install cargo-callgraph; AGPL-3.0, used only "
                           "as an external program)")
    if cg is not None:
        edges, unmatched = edges_from_callgraph(root, functions, cg)
        source = "cargo-callgraph"
    else:
        edges = edges_from_text(functions)
    edges = dedupe_edges(edges)
    succ, pred, maybe, components, comp_of = analyze_graph(functions, edges)
    harnesses = relations(scan_harnesses(root, all_functions), all_functions, edges, succ)
    sources = package_sources(root, packages)
    comp_by_index = {i: c for i, c in enumerate(components)}
    edges_from = collections.defaultdict(list)
    for e in edges:
        edges_from[e["from"]].append(e)
    fn_records = []
    for f in functions:
        fid = f["id"]
        out_edges = edges_from.get(fid, [])
        unresolved = sorted({(e.get("to_name") or e["to"], e["kind"]) for e in out_edges
                             if not e["resolved"] and e["kind"] != "external"})
        external = sorted({e["to_name"] for e in out_edges if e["kind"] == "external"})
        comp = comp_by_index[comp_of[fid]]
        rec = {k: f[k] for k in ("id", "name", "qualified_path", "package", "file", "line", "end_line", "scope",
                                 "self_type", "trait", "generic", "public", "unsafe_fn", "signals", "contracts",
                                 "has_loop_contract", "cfg")}
        rec.update({
            "callees": sorted(succ.get(fid, ())), "callers": sorted(pred.get(fid, ())),
            "candidate_callees": sorted(maybe.get(fid, ())),
            "unresolved_callees": [{"name": n, "kind": k} for n, k in unresolved],
            "external_callees": external,
            "component": comp["id"], "recursive": comp["recursive"], "level": comp["level"],
            "transitive_callers": len(transitive([fid], pred)),
            "has_harness": False, "harnesses": [],
            "complexity": {"source": "cargo kani complexity (proposed)", "status": "not_computed",
                           "metrics": {m: None for m in COMPLEXITY_METRICS}},
        })
        fn_records.append(rec)
    by_id = {r["id"]: r for r in fn_records}
    for h in harnesses:
        for key in ("contract_target", "reachable"):
            for fid in h["reaches"][key]:
                if fid in by_id and h["id"] not in by_id[fid]["harnesses"]:
                    by_id[fid]["harnesses"].append(h["id"])
                    by_id[fid]["has_harness"] = True
        h["function"] = h.pop("_function_id")
    m = {
        "schema_version": SCHEMA_VERSION, "kind": KIND,
        "generator": {"name": "kani-agent-plugin map-crate.py", "tools": tool_versions()},
        "configuration": {"call_graph": source, "features": "default", "root": ".",
                          "note": note},
        "workspace": {"packages": packages},
        "functions": fn_records, "edges": edges, "components": components, "harnesses": harnesses,
        "sources": sources,
        "uncertainty": global_uncertainty(root, packages, source, unmatched, sources),
    }
    finalize(m, root, results)
    return m


def finalize(m, root, results=()):
    """(Re)compute everything that depends on Kani results: per-harness results, per-function
    verification status, strategy, and priority. Works from the map alone."""
    root = Path(root)
    package_files = [root / rel for rel in m["sources"] if rel.endswith(".rs") and (root / rel).exists()]
    merge_results(
        m["harnesses"],
        load_results(results) if results and not isinstance(
            results[0],
            dict) else list(results),
        root,
        package_files)
    by_id = {f["id"]: f for f in m["functions"]}
    comps = {c["id"]: c for c in m["components"]}
    harness_by_id = {h["id"]: h for h in m["harnesses"]}
    for f in m["functions"]:
        f["verification"] = function_verification(f["id"], m["harnesses"])

    def contract_proven(fid):
        return any(e["relation"] == "contract-target" and e["status"] == "verified" and e.get("fresh") is not False
                   for e in by_id[fid]["verification"]["evidence"])

    for f in m["functions"]:
        v = f["verification"]
        if v["status"] != "verified":
            continue
        conditions = set()
        for e in v["evidence"]:
            if e["relation"] not in ("contract-target", "reachable") or e["status"] != "verified":
                continue
            h = harness_by_id[e["harness"]]
            conditions.update(g for g in h["reaches"]["contract_assumed"] if g in by_id and not contract_proven(g))
            conditions.update(g for g in h["reaches"]["stubbed"]
                              if g in by_id and by_id[g]["verification"]["status"] != "verified")
        if conditions:
            v["status"] = "conditional"
            v["conditional_on"] = sorted(conditions)
            v["caveats"] = sorted(set(v["caveats"] + [
                f"depends on unverified: {', '.join(sorted(conditions))} (stubbed, or assumed through a contract "
                "that no verified harness proves)"]))
    for f in m["functions"]:
        v = f["verification"]
        f["uncertainty"] = []
        if f["unresolved_callees"]:
            f["uncertainty"].append({"flag": "unresolved-callees", "reason": f"{len(f['unresolved_callees'])} call(s) "
                                     "not resolved to a workspace function (trait dispatch, external, unknown)"})
        if f["candidate_callees"]:
            f["uncertainty"].append({"flag": "name-matched-callees", "reason": "edges matched by name only; "
                                     "the real callee may be any of them"})
        if f["generic"]:
            f["uncertainty"].append({"flag": "generic", "reason": "monomorphized instances are not tracked"})
        if f["cfg"]:
            f["uncertainty"].append({"flag": "cfg-gated", "reason": f"only compiled under {f['cfg']}: results hold "
                                     "for that configuration only"})
        if f["trait"]:
            f["uncertainty"].append({"flag": "trait-method", "reason": f"implements {f['trait']}; callers may "
                                     "reach it through dynamic dispatch the graph cannot see"})
        if f["unsafe_fn"] or f["signals"]["unsafe_blocks"]:
            f["uncertainty"].append({"flag": "unsafe", "reason": "contains unsafe code"})
        if any(e.get("fresh") is None for e in v["evidence"] if "verdict" in e):
            f["uncertainty"].append({"flag": "freshness-unknown", "reason": "a result has no start time or hash"})
    status = {f["id"]: f["verification"]["status"] for f in m["functions"]}
    for f in m["functions"]:
        comp = comps[f["component"]]
        unverified = sorted(c for c in f["callees"] if status[c] != "verified" and c not in comp["members"])
        f["unverified_callees"] = unverified
        f["strategy"] = strategy_for(f, comp, status[f["id"]], unverified)
        f["priority"], f["priority_inputs"] = priority_of(f, f["transitive_callers"])
    m["summary"] = {"functions": len(m["functions"]),
                    "by_status": dict(sorted(collections.Counter(status.values()).items())),
                    "harnesses": len(m["harnesses"]),
                    "harnesses_with_results": sum(1 for h in m["harnesses"] if h.get("result")),
                    "recursive_components": sum(1 for c in m["components"] if c["recursive"]),
                    "edges": {"resolved": sum(1 for e in m["edges"] if e["resolved"]),
                              "unresolved": sum(1 for e in m["edges"] if not e["resolved"])}}
    return m


def check_fresh(m, root):
    """Source files that changed since the map was built: [(path, reason)]."""
    root = Path(root)
    problems = []
    for rel, digest in m["sources"].items():
        f = root / rel
        if not f.exists():
            problems.append((rel, "removed"))
        elif sha256(f) != digest:
            problems.append((rel, "changed"))
    return problems


def plan(m, goals=()):
    """A staged, bottom-up verification plan from the map. Goals restrict it to the goal
    functions and everything they depend on, and override the heuristic order of stages only
    by scope: ordering inside the scope still follows dependencies."""
    by_id = {f["id"]: f for f in m["functions"]}
    wanted = set(by_id)
    if goals:
        resolved = []
        for g in goals:
            hits = resolve_function(m, g)
            if not hits:
                raise KeyError(f"no function matches `{g}`")
            resolved.extend(hits)
        adj = {f["id"]: set(f["callees"]) | set(f["candidate_callees"]) for f in m["functions"]}
        wanted = set(resolved) | transitive(resolved, adj)
    done = {f["id"] for f in m["functions"] if f["verification"]["status"] == "verified"}
    stages = {}
    for f in m["functions"]:
        if f["id"] not in wanted or f["id"] in done:
            continue
        blocked = sorted(c for c in f["unverified_callees"])
        stages.setdefault(f["level"], []).append({
            "function": f["id"], "status": f["verification"]["status"], "component": f["component"],
            "recursive": f["recursive"], "strategy": f["strategy"]["primary"], "also": f["strategy"]["also"],
            "rationale": f["strategy"]["rationale"], "priority": f["priority"], "blocked_by": blocked,
            "unresolved_callees": [c["name"] for c in f["unresolved_callees"]],
            "harnesses": f["harnesses"], "conditional_on": f["verification"].get("conditional_on", []),
            "uncertainty": [u["flag"] for u in f["uncertainty"]]})
    out_stages = []
    for n, level in enumerate(sorted(stages)):
        items = sorted(stages[level], key=lambda i: (-i["priority"], i["function"]))
        out_stages.append({"stage": n + 1, "level": level, "functions": items})
    reusable = sorted(f["id"] for f in m["functions"]
                      if f["id"] in wanted and len(f["callers"]) >= 2 and f["verification"]["status"] != "verified")
    return {
        "schema_version": SCHEMA_VERSION, "kind": "kani-verification-plan",
        "goals": list(goals) or None,
        "scope": {"functions": len(wanted), "already_verified": len(wanted & done), "to_do": len(wanted - done)},
        "stages": out_stages,
        "reusable_contract_candidates": reusable,
        "notes": ["Stages run bottom-up: callees before callers. A function is `blocked_by` its callees that are "
                  "not verified; prove them, or cover them with a contract (stub_verified), first.",
                  "Recursive components must be handled together (contracts for every member).",
                  "`verified` means a fresh harness that provably reaches the function succeeded, under the "
                  "recorded bounds, assumptions, and stubs; it is not a proof of the whole crate.",
                  "Priority and strategy are heuristics (impact / effort); they do not predict solver time.",
                  *[u["reason"] for u in m["uncertainty"] if u["flag"] == "approximate-call-graph"]],
    }


def resolve_function(m, query):
    """Functions whose id, qualified path, or name matches `query` (exact id first)."""
    exact = [f["id"] for f in m["functions"] if f["id"] == query]
    if exact:
        return exact
    tail = [f["id"] for f in m["functions"] if f["qualified_path"]
            == query or f["qualified_path"].endswith("::" + query)]
    return tail or [f["id"] for f in m["functions"] if f["name"] == query]


def explain(m, query):
    ids = resolve_function(m, query)
    if not ids:
        raise KeyError(f"no function matches `{query}`")
    by_id = {f["id"]: f for f in m["functions"]}
    out = []
    for fid in ids:
        f = by_id[fid]
        out.append({
            **{k: f[k] for k in ("id", "file", "line", "signals", "contracts", "complexity", "strategy", "priority",
                                 "priority_inputs", "uncertainty", "harnesses", "component", "recursive", "level")},
            "verification": f["verification"],
            "callees": {c: by_id[c]["verification"]["status"] for c in f["callees"]},
            "callers": {c: by_id[c]["verification"]["status"] for c in f["callers"]},
            "unresolved_callees": f["unresolved_callees"],
            "how_to_read": "signals are text-based counts; the five complexity metrics are not computed "
                           "until `cargo kani complexity` exists; priority = impact / effort (heuristic)",
        })
    return out


STATUS_STYLE = {"verified": "#c8e6c9", "conditional": "#fff9c4", "failed": "#ffcdd2", "inconclusive": "#ffe0b2",
                "unsupported": "#e1bee7", "stale": "#cfd8dc", "not_attempted": "#ffffff"}


def mermaid(m, max_nodes=60):
    """A Mermaid flowchart of the call graph: node color is the verification status, dashed
    edges are unresolved, thick borders are recursive components."""
    funcs = sorted(m["functions"], key=lambda f: (-len(f["callers"]) - len(f["callees"]), f["id"]))[:max_nodes]
    keep = {f["id"] for f in funcs}
    name = {f["id"]: f"n{i}" for i, f in enumerate(sorted(funcs, key=lambda f: f["id"]))}
    lines = ["flowchart TD"]
    for f in sorted(funcs, key=lambda f: f["id"]):
        label = f["qualified_path"].split("::", 1)[-1].replace('"', "'")
        lines.append(f'    {name[f["id"]]}["{label}<br/>{f["verification"]["status"]}"]')
    for st, color in STATUS_STYLE.items():
        ids = [name[f["id"]] for f in funcs if f["verification"]["status"] == st]
        if ids:
            lines.append(f"    style {','.join(ids)} fill:{color}")
    for e in m["edges"]:
        if e["from"] in keep and e.get("to") in keep:
            arrow = "-->" if e["resolved"] else "-.->"
            lines.append(f"    {name[e['from']]} {arrow} {name[e['to']]}")
    if len(m["functions"]) > len(funcs):
        lines.append(f"    %% {len(m['functions']) - len(funcs)} less connected function(s) omitted")
    return "\n".join(lines) + "\n"
