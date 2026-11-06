#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Unit tests for the Crate Verifiability Map. They need `cargo` (for `cargo metadata`) but neither
Kani nor cargo-callgraph: results are canned, and shipped as evidence bundles whose hashes are
computed from the fixture workspace.

Run with: python3 agent-plugin/tests/test_map.py
"""

import contextlib
import copy
import hashlib
import io
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

TESTS = Path(__file__).resolve().parent
sys.path.insert(0, str(TESTS.parent / "scripts"))
import verifiability_map as vm  # noqa: E402

WORKSPACE = TESTS / "fixtures" / "workspace"
SCHEMA = json.loads((TESTS.parent / "schemas" / "verifiability-map.schema.json").read_text())


def validate(value, schema, path="$"):
    """Errors of `value` against the JSON Schema subset the map schema uses."""
    errors = []
    if "const" in schema and value != schema["const"]:
        errors.append(f"{path}: expected {schema['const']!r}")
    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{path}: {value!r} not in {schema['enum']}")
    types = schema.get("type")
    if types:
        names = {"string": str, "integer": int, "number": (int, float), "boolean": bool, "array": list,
                 "object": dict, "null": type(None)}
        ok = any(isinstance(value, names[t]) and not (t in ("integer", "number") and isinstance(value, bool))
                 for t in ([types] if isinstance(types, str) else types))
        if not ok:
            return [f"{path}: expected {types}, got {type(value).__name__}"]
    if isinstance(value, dict):
        for key in schema.get("required", []):
            if key not in value:
                errors.append(f"{path}: missing `{key}`")
        for key, sub in schema.get("properties", {}).items():
            if key in value:
                errors += validate(value[key], sub, f"{path}.{key}")
    if isinstance(value, list) and "items" in schema:
        for i, item in enumerate(value):
            errors += validate(item, schema["items"], f"{path}[{i}]")
    return errors


def harness_entry(name, file, line, verdict="SUCCESS", kind="COMPLETED", stubs=(), verified_stubs=(), checks=None,
                  unsupported=0, bounded=False):
    outcome = {"kind": kind}
    if kind == "COMPLETED":
        outcome["verdict"] = verdict
    return {"name": name, "file": file, "line": line, "outcome": outcome, "resolved_unwind": 5,
            "is_bounded": bounded, "resources": {"verification_time_s": 1.5},
            "attributes": {"stubs": [{"original": a, "replacement": b} for a, b in stubs],
                           "verified_stubs": list(verified_stubs)},
            "unsupported_constructs": [{}] * unsupported, "failed_properties": []}


def write_bundle(directory, harnesses, cwd, sources, checks=None):
    """An evidence bundle of a (fictional) run over the workspace at `cwd`."""
    directory.mkdir(parents=True)
    doc = {"tools": {"kani": "0.68.0"}, "harnesses": harnesses, "kani_commit": None,
           "configuration": {"checks": checks or {}, "cbmc_args": []}}
    (directory / "results.json").write_text(json.dumps(doc))
    hashes = {rel: hashlib.sha256((cwd / rel).read_bytes()).hexdigest() for rel in sources
              if (cwd / rel).exists()}
    (directory / "bundle.json").write_text(json.dumps({"cwd": str(cwd), "sources_sha256": hashes}))
    return directory


STANDARD = [
    harness_entry("proofs::check_run", "app/src/lib.rs", 21),
    harness_entry("proofs::check_sum_to", "core/src/lib.rs", 56),
    harness_entry("proofs::check_fact", "core/src/lib.rs", 66, verdict="FAILURE"),
    harness_entry("math::proofs::check_midpoint", "core/src/math.rs", 21, kind="TIMEOUT"),
    harness_entry("math::proofs::check_div_contract", "core/src/math.rs", 29),
]


class MapTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = Path(tempfile.mkdtemp())
        cls.root = cls.tmp / "ws"
        shutil.copytree(WORKSPACE, cls.root, ignore=shutil.ignore_patterns("target", "Cargo.lock"))
        cls.sources = vm.package_sources(cls.root.resolve(), vm.discover_workspace(cls.root)[1])
        cls.bundle = write_bundle(cls.tmp / "b1", STANDARD, cls.root.resolve(), cls.sources)
        cls.map = vm.build_map(cls.root, [cls.bundle], callgraph="text")
        cls.by_name = {f["qualified_path"].split("::", 1)[1]: f for f in cls.map["functions"]}

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.tmp)

    def status(self, name):
        return self.by_name[name]["verification"]["status"]

    def test_schema(self):
        self.assertEqual(validate(json.loads(json.dumps(self.map, default=str)), SCHEMA), [])

    def test_workspace_discovery(self):
        pkgs = {p["name"]: p for p in self.map["workspace"]["packages"]}
        self.assertEqual(sorted(pkgs), ["ws-app", "ws-core"])
        self.assertEqual(pkgs["ws-app"]["workspace_dependencies"], ["ws-core"])
        self.assertEqual(sorted(t["kind"][0] for t in pkgs["ws-app"]["targets"]), ["bin", "lib"])
        self.assertEqual(pkgs["ws-core"]["features"], {"fast": []})

    def test_function_index_and_stable_ids(self):
        self.assertIn("math::div", self.by_name)
        self.assertIn("shapes::<Square as Shape>::area", self.by_name)
        self.assertEqual(self.by_name["shapes::largest"]["generic"], True)
        self.assertEqual(self.by_name["fast_path"]["cfg"], ['feature = "fast"'])
        ids = [f["id"] for f in self.map["functions"]]
        self.assertEqual(ids, sorted(ids))
        self.assertEqual(len(ids), len(set(ids)))
        self.assertNotIn("seven", [f["name"] for f in self.map["functions"]])  # macro-generated: invisible

    def test_harnesses_and_proof_code_are_not_functions(self):
        names = {f["name"] for f in self.map["functions"]}
        self.assertTrue(names.isdisjoint({"check_run", "check_sum_to", "check_fact", "check_midpoint"}))
        self.assertEqual(len(self.map["harnesses"]), 5)

    def test_call_graph_across_crates(self):
        run = self.by_name["run"]
        self.assertIn("ws_core::sum_to", run["callees"])
        self.assertIn("ws_core::math::midpoint", run["callees"])
        self.assertIn("ws_app::run", self.by_name["run_twice"]["callees"])

    def test_recursion_components(self):
        self.assertTrue(self.by_name["fact"]["recursive"])
        self.assertEqual(self.by_name["is_even"]["component"], self.by_name["is_odd"]["component"])
        self.assertTrue(self.by_name["is_even"]["recursive"])
        self.assertFalse(self.by_name["sum_to"]["recursive"])
        comp = next(c for c in self.map["components"] if c["id"] == self.by_name["is_even"]["component"])
        self.assertEqual(len(comp["members"]), 2)

    def test_levels_are_bottom_up(self):
        self.assertEqual(self.by_name["sum_to"]["level"], 0)
        self.assertEqual(self.by_name["run"]["level"], 1)
        self.assertEqual(self.by_name["run_twice"]["level"], 2)

    def test_dynamic_dispatch_is_not_resolved(self):
        total = self.by_name["shapes::total_area"]
        self.assertEqual(total["callees"], [])
        self.assertTrue(any(u["name"].endswith("area") for u in total["unresolved_callees"]))
        self.assertIn("unresolved-callees", [u["flag"] for u in total["uncertainty"]])

    def test_statuses_from_results(self):
        self.assertEqual(self.status("sum_to"), "verified")
        self.assertEqual(self.status("run_twice"), "verified")  # reached by check_run
        self.assertEqual(self.status("fact"), "failed")
        self.assertEqual(self.status("math::div"), "verified")  # through proof_for_contract
        self.assertEqual(self.status("is_even"), "not_attempted")
        # Its only harness timed out, but check_run also reaches it, and that run succeeded.
        self.assertEqual(self.status("math::midpoint"), "inconclusive")

    def test_inconclusive_beats_verified(self):
        ev = {e["harness"].split("::")[-1]: e["status"]
              for e in self.by_name["math::midpoint"]["verification"]["evidence"]}
        self.assertEqual(ev, {"check_midpoint": "inconclusive", "check_run": "verified"})

    def test_unsupported(self):
        bundle = write_bundle(self.tmp / "b-unsupported", [harness_entry(
            "proofs::check_sum_to", "core/src/lib.rs", 56, unsupported=1)], self.root.resolve(), self.sources)
        m = vm.build_map(self.root, [bundle], callgraph="text")
        sum_to = next(f for f in m["functions"] if f["name"] == "sum_to")
        self.assertEqual(sum_to["verification"]["status"], "unsupported")

    def test_stale_results(self):
        bundle = write_bundle(self.tmp / "b-stale", STANDARD, self.root.resolve(), self.sources)
        meta = json.loads((bundle / "bundle.json").read_text())
        meta["sources_sha256"]["core/src/lib.rs"] = "0" * 64
        (bundle / "bundle.json").write_text(json.dumps(meta))
        m = vm.build_map(self.root, [bundle], callgraph="text")
        sum_to = next(f for f in m["functions"] if f["name"] == "sum_to")
        self.assertEqual(sum_to["verification"]["status"], "stale")
        self.assertIn("changed since the run", sum_to["verification"]["evidence"][0]["freshness"])

    def test_no_results_means_not_attempted(self):
        m = vm.build_map(self.root, [], callgraph="text")
        self.assertEqual({f["verification"]["status"] for f in m["functions"]}, {"not_attempted"})
        self.assertEqual(m["summary"]["harnesses_with_results"], 0)

    def test_stub_verified_makes_the_result_conditional(self):
        root = self.tmp / "ws-stub"
        shutil.copytree(self.root, root)
        lib = root / "app" / "src" / "lib.rs"
        lib.write_text(lib.read_text().replace(
            "    #[kani::proof]\n    #[kani::unwind(5)]\n    fn check_run()",
            "    #[kani::proof]\n    #[kani::stub_verified(ws_core::math::div)]\n    fn check_stubbed() {\n"
            "        let _ = run(1);\n    }\n\n    #[kani::proof]\n    #[kani::unwind(5)]\n    fn check_run()"))
        sources = vm.package_sources(root.resolve(), vm.discover_workspace(root)[1])
        entries = [harness_entry("proofs::check_stubbed", "app/src/lib.rs", 21, verified_stubs=["ws_core::math::div"])]
        bundle = write_bundle(self.tmp / "b-stub", entries, root.resolve(), sources)
        m = vm.build_map(root, [bundle], callgraph="text")
        by = {f["name"]: f for f in m["functions"]}
        self.assertEqual(by["run"]["verification"]["status"], "conditional")  # check_stubbed calls run
        h = next(h for h in m["harnesses"] if h["name"] == "check_stubbed")
        self.assertEqual(h["reaches"]["contract_assumed"], ["ws_core::math::div"])
        self.assertIn("ws_core::sum_to", h["reaches"]["reachable"])
        self.assertEqual(by["sum_to"]["verification"]["status"], "conditional")
        self.assertEqual(by["sum_to"]["verification"]["conditional_on"], ["ws_core::math::div"])
        # Once a verified proof of the contract exists, the dependency is satisfied.
        entries.append(harness_entry("math::proofs::check_div_contract", "core/src/math.rs", 29))
        bundle = write_bundle(self.tmp / "b-stub2", entries, root.resolve(), sources)
        m = vm.build_map(root, [bundle], callgraph="text")
        by = {f["name"]: f for f in m["functions"]}
        self.assertEqual(by["sum_to"]["verification"]["status"], "verified")

    def test_weakened_checks_are_caveats(self):
        bundle = write_bundle(self.tmp / "b-weak", STANDARD[:2], self.root.resolve(), self.sources,
                              checks={"overflow": False})
        m = vm.build_map(self.root, [bundle], callgraph="text")
        sum_to = next(f for f in m["functions"] if f["name"] == "sum_to")
        self.assertTrue(any("checks disabled: overflow" in c for c in sum_to["verification"]["caveats"]))

    def test_complexity_is_reserved_not_computed(self):
        for f in self.map["functions"]:
            self.assertEqual(f["complexity"]["status"], "not_computed")
            self.assertEqual(set(f["complexity"]["metrics"]), set(vm.COMPLEXITY_METRICS))
            self.assertTrue(all(v is None for v in f["complexity"]["metrics"].values()))
        self.assertIn("complexity-not-computed", [u["flag"] for u in self.map["uncertainty"]])

    def test_uncertainty_is_explicit(self):
        flags = {u["flag"] for u in self.map["uncertainty"]}
        self.assertTrue({"approximate-call-graph", "cfg-ignored", "macro-generated-items"} <= flags)

    def test_deterministic_output(self):
        again = vm.build_map(self.root, [self.bundle], callgraph="text")
        self.assertEqual(json.dumps(again, sort_keys=True, default=str),
                         json.dumps(self.map, sort_keys=True, default=str))

    def test_plan_is_bottom_up_and_skips_verified(self):
        p = vm.plan(self.map)
        planned = {i["function"].split("::", 1)[1]: (s["stage"], i) for s in p["stages"] for i in s["functions"]}
        self.assertNotIn("sum_to", planned)
        self.assertIn("fact", planned)
        self.assertEqual(planned["fact"][1]["strategy"], "triage-failure")
        self.assertEqual(planned["is_even"][1]["strategy"], "contract-and-recursion")
        self.assertEqual(planned["math::midpoint"][1]["strategy"], "reduce-or-split")
        levels = [s["level"] for s in p["stages"]]
        self.assertEqual(levels, sorted(levels))
        self.assertEqual(p["scope"]["already_verified"], 4)

    def test_plan_goal_restricts_scope_to_dependencies(self):
        p = vm.plan(self.map, ["run_twice"])
        self.assertEqual(p["scope"]["functions"], 4)  # run_twice, run, sum_to, midpoint
        planned = {i["function"].split("::", 1)[1] for s in p["stages"] for i in s["functions"]}
        self.assertEqual(planned, {"math::midpoint"})
        with self.assertRaises(KeyError):
            vm.plan(self.map, ["nonexistent"])

    def test_update_merges_new_results_and_replans(self):
        m = copy.deepcopy(self.map)
        better = [harness_entry("proofs::check_fact", "core/src/lib.rs", 66),
                  harness_entry("math::proofs::check_midpoint", "core/src/math.rs", 21)]
        bundle = write_bundle(self.tmp / "b-update", better, self.root.resolve(), self.sources)
        vm.finalize(m, self.root.resolve(), [bundle])
        by = {f["qualified_path"].split("::", 1)[1]: f for f in m["functions"]}
        self.assertEqual(by["fact"]["verification"]["status"], "verified")
        self.assertEqual(by["math::midpoint"]["verification"]["status"], "verified")
        self.assertEqual(by["fact"]["strategy"]["primary"], "none")
        self.assertEqual(vm.check_fresh(m, self.root.resolve()), [])
        (self.root / "core" / "src" / "math.rs").write_text(
            (self.root / "core" / "src" / "math.rs").read_text() + "\n// edit\n")
        try:
            self.assertEqual(vm.check_fresh(m, self.root.resolve()), [("core/src/math.rs", "changed")])
        finally:
            shutil.copy2(WORKSPACE / "core" / "src" / "math.rs", self.root / "core" / "src" / "math.rs")

    def test_explain(self):
        [e] = vm.explain(self.map, "fact")
        self.assertEqual(e["verification"]["status"], "failed")
        self.assertEqual(e["complexity"]["status"], "not_computed")
        self.assertEqual(vm.explain(self.map, "ws_core::sum_to")[0]["id"], "ws_core::sum_to")
        with self.assertRaises(KeyError):
            vm.explain(self.map, "nope")

    def test_mermaid(self):
        text = vm.mermaid(self.map)
        self.assertTrue(text.startswith("flowchart TD"))
        self.assertIn("failed", text)
        self.assertIn("-->", text)


def auto_entry(name, file, line, contract=False, verdict="SUCCESS", unwind=20):
    entry = harness_entry(name, file, line, verdict=verdict)
    entry["is_automatically_generated"] = True
    entry["resolved_unwind"] = unwind
    entry["attributes"]["kind"] = {"ProofForContract": {"target_fn": name}} if contract else "Proof"
    return entry


class ContractFirstTest(unittest.TestCase):
    """The map steers toward contracts, loop contracts with decreases clauses, and AutoHarness."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = Path(tempfile.mkdtemp())
        cls.root = cls.tmp / "ws"
        shutil.copytree(WORKSPACE, cls.root, ignore=shutil.ignore_patterns("target", "Cargo.lock"))
        core = cls.root / "core" / "src" / "lib.rs"
        core.write_text(core.read_text() + """
/// Loop contract with an invariant but no decreases clause: termination is not shown.
pub fn invariant_only(n: u32) -> u32 {
    let mut x = n;
    #[kani::loop_invariant(x <= n)]
    while x > 0 {
        x -= 1;
    }
    x
}

/// A complete loop contract.
#[kani::requires(n <= 100)]
#[kani::ensures(|r: &u32| *r == 0)]
pub fn counted_down(n: u32) -> u32 {
    let mut x = n;
    #[kani::loop_invariant(x <= n)]
    #[kani::loop_decreases(x)]
    while x > 0 {
        x -= 1;
    }
    x
}
""")
        cls.map = vm.build_map(cls.root, [], callgraph="text")
        cls.by = {f["qualified_path"].split("::", 1)[1]: f for f in cls.map["functions"]}

    @classmethod
    def tearDownClass(cls):
        shutil.rmtree(cls.tmp)

    def primary(self, name):
        return self.by[name]["strategy"]["primary"]

    def test_strategy_asks_for_contracts_and_loop_contracts(self):
        self.assertEqual(self.primary("sum_to"), "loop-contract")
        self.assertIn("loop_decreases", " ".join(self.by["sum_to"]["strategy"]["rationale"]))
        self.assertEqual(self.primary("invariant_only"), "add-loop-decreases")
        self.assertEqual(self.primary("is_even"), "contract-and-recursion")
        self.assertEqual(self.primary("math::div"), "verify-contract-with-autoharness")
        self.assertEqual(self.primary("fast_path"), "write-contract-then-autoharness")
        self.assertEqual(self.primary("shapes::total_area"), "loop-contract")  # loop first, dispatch second
        self.assertIn("stub-or-contract-callees", self.by["shapes::total_area"]["strategy"]["also"])

    def test_loop_contract_signals_and_partial_termination(self):
        s = self.by["counted_down"]["signals"]
        self.assertEqual((s["loop_keywords"], s["loop_invariants"], s["loop_decreases"]), (1, 1, 1))
        self.assertNotEqual(self.primary("counted_down"), "loop-contract")
        flags = {u["flag"] for u in self.by["invariant_only"]["uncertainty"]}
        self.assertIn("partial-termination", flags)
        self.assertNotIn("partial-termination", {u["flag"] for u in self.by["counted_down"]["uncertainty"]})

    def test_plan_has_contract_workflow_and_autoharness_command(self):
        plan = vm.plan(self.map)
        self.assertIn("Contract-first", plan["approach"])
        progress = plan["contract_progress"]
        self.assertEqual(progress["with_contract"], 2)  # div and counted_down
        self.assertEqual(progress["loops_with_invariant"], 2)
        self.assertEqual(progress["loops_with_decreases"], 1)
        self.assertEqual(progress["partial_termination"], ["ws_core::invariant_only"])
        stage = next(st for st in plan["stages"] if st["level"] == 0)
        self.assertEqual(len(stage["workflow"]), 4)
        self.assertIn("ws_core::sum_to", stage["loop_contract_work"])
        auto = stage["autoharness"]
        self.assertIn("^ws_core::math::div$", auto["include_patterns"])
        self.assertIn("-Z loop-contracts", auto["cli"])
        self.assertEqual(auto["mcp"]["tool"], "kani_autoharness")
        # Functions with unresolved callees or generics are not in the sweep command.
        self.assertFalse(any("total_area" in p or "largest" in p for p in auto["include_patterns"]))

    def test_autoharness_results_are_evidence(self):
        sources = vm.package_sources(self.root.resolve(), vm.discover_workspace(self.root)[1])
        div = self.by["math::div"]
        entries = [auto_entry("div", "core/src/math.rs", div["first_line"], contract=True),
                   auto_entry("counted_down", "core/src/lib.rs", self.by["counted_down"]["first_line"], contract=True),
                   auto_entry("sum_to", "core/src/lib.rs", self.by["sum_to"]["first_line"]),
                   auto_entry("ghost", "core/src/lib.rs", 9999)]
        bundle = write_bundle(self.tmp / "auto", entries, self.root.resolve(), sources)
        m = vm.build_map(self.root, [bundle], callgraph="text")
        by = {f["qualified_path"].split("::", 1)[1]: f for f in m["functions"]}
        self.assertEqual(by["math::div"]["contract_status"], "verified")
        self.assertEqual(by["math::div"]["verification"]["status"], "verified")
        self.assertEqual(by["counted_down"]["contract_status"], "verified")
        self.assertEqual(by["sum_to"]["contract_status"], "none")
        # A plain automatic harness verifies the function, but its loop has no invariant: only
        # within the unwind bound, and the map says so.
        self.assertEqual(by["sum_to"]["verification"]["status"], "verified")
        self.assertTrue(any("no invariant" in c and "20" in c for c in by["sum_to"]["verification"]["caveats"]))
        self.assertEqual(by["counted_down"]["verification"]["caveats"], [])
        self.assertIn("unmatched-automatic-harnesses", {u["flag"] for u in m["uncertainty"]})
        self.assertEqual({h["id"] for h in m["harnesses"] if h.get("automatic")},
                         {"auto::ws_core::math::div", "auto::ws_core::counted_down", "auto::ws_core::sum_to"})
        self.assertEqual(validate(json.loads(json.dumps(m, default=str)), SCHEMA), [])
        # A second update replaces the automatic harnesses instead of duplicating them.
        vm.finalize(m, self.root.resolve(), [bundle])
        self.assertEqual(len([h for h in m["harnesses"] if h.get("automatic")]), 3)
        again = {f["qualified_path"]: f["harnesses"] for f in m["functions"]}
        self.assertEqual(again["ws_core::math::div"].count("auto::ws_core::math::div"), 1)

    def test_failed_contract_harness_marks_the_contract_failed(self):
        sources = vm.package_sources(self.root.resolve(), vm.discover_workspace(self.root)[1])
        div = self.by["math::div"]
        bundle = write_bundle(self.tmp / "auto-fail", [auto_entry("div", "core/src/math.rs", div["first_line"],
                                                                  contract=True, verdict="FAILURE")],
                              self.root.resolve(), sources)
        m = vm.build_map(self.root, [bundle], callgraph="text")
        f = next(f for f in m["functions"] if f["name"] == "div")
        self.assertEqual((f["contract_status"], f["verification"]["status"], f["strategy"]["primary"]),
                         ("failed", "failed", "triage-failure"))


class CallgraphJoinTest(unittest.TestCase):
    def test_edges_from_cargo_callgraph_output(self):
        root = Path("/w")
        functions = [{"id": "c::a", "file": "src/lib.rs", "line": 3, "name": "a"},
                     {"id": "c::b", "file": "src/lib.rs", "line": 7, "name": "b"}]

        def node(name, line, ws=True, trait=False, file="/w/src/lib.rs", qp=None):
            return {"id": f"{name}@{file}:{line}", "name": name, "qualified_path": qp or name, "file": file,
                    "line": line, "is_workspace": ws, "is_trait_method": trait}

        cg = {"nodes": [node("a", 2), node("b", 6), node("Shape::area", 1, trait=True), node("Sq", 9, trait=True),
                        node("sum", 5, ws=False, file="/rustc/core.rs")],
              "edges": [
                  {"source": "a@/w/src/lib.rs:2", "target": "b@/w/src/lib.rs:6", "kind": "DirectCall",
                   "call_site_file": "/w/src/lib.rs", "call_site_line": 3},
                  {"source": "a@/w/src/lib.rs:2", "target": "Shape::area@/w/src/lib.rs:1", "kind": "TraitDefCall",
                   "call_site_file": "/w/src/lib.rs", "call_site_line": 4},
                  {"source": "a@/w/src/lib.rs:2", "target": "Sq@/w/src/lib.rs:9", "kind": "TraitDefCall",
                   "call_site_file": "/w/src/lib.rs", "call_site_line": 4},
                  {"source": "a@/w/src/lib.rs:2", "target": "sum@/rustc/core.rs:5", "kind": "DirectCall",
                   "call_site_file": "/w/src/lib.rs", "call_site_line": 5}]}
        import unittest.mock as mock
        with mock.patch.object(vm, "rel_to", lambda r, p: p[len("/w/"):] if p.startswith("/w/") else None):
            edges, unmatched = vm.edges_from_callgraph(root, functions, cg)
        kinds = [(e["kind"], e["resolved"], e.get("to") or e.get("to_name")) for e in edges]
        self.assertEqual(kinds, [("direct", True, "c::b"), ("trait-dispatch", False, "Shape::area"),
                                 ("external", False, "sum")])
        self.assertEqual(edges[0]["call_site"]["line"], 4)  # 0-based in the tool, 1-based here
        self.assertEqual(unmatched, 2)  # the trait declaration and the constructor-like node


class CliTest(unittest.TestCase):
    def test_cli_round_trip(self):
        sys.path.insert(0, str(TESTS.parent / "scripts"))
        import importlib.util
        spec = importlib.util.spec_from_file_location("map_crate", TESTS.parent / "scripts" / "map-crate.py")
        cli = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cli)
        with tempfile.TemporaryDirectory() as tmp:
            map_file, plan_file = Path(tmp) / "map.json", Path(tmp) / "plan.json"
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                self.assertEqual(cli.main(["map", str(WORKSPACE), "--callgraph", "text", "--out", str(map_file)]), 0)
                self.assertEqual(cli.main(["plan", str(map_file), "--out", str(plan_file)]), 0)
                self.assertEqual(cli.main(["explain", str(map_file), "fact"]), 0)
            self.assertEqual(json.loads(plan_file.read_text())["kind"], "kani-verification-plan")
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(cli.main(["explain", str(map_file), "nope"]), 1)
                bad = Path(tmp) / "bad.json"
                bad.write_text("{}")
                self.assertEqual(cli.main(["plan", str(bad)]), 1)
        for leftover in (WORKSPACE / "target", WORKSPACE / "Cargo.lock"):
            self.assertFalse(leftover.exists(), f"{leftover} should not be created by mapping")


if __name__ == "__main__":
    unittest.main()
