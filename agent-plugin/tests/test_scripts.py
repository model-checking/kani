#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Unit tests for the plugin scripts. They need neither Kani nor a Rust toolchain: they run on
the RFC 0015 fixtures in `tests/json-handler/schema-validation` and on Rust sources in
`fixtures/rust`.

Run with: python3 agent-plugin/tests/test_scripts.py
"""

import contextlib
import importlib.util
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

TESTS = Path(__file__).resolve().parent
SCRIPTS = TESTS.parent / "scripts"
JSON_FIXTURES = TESTS.parent.parent / "tests" / "json-handler" / "schema-validation"
RUST = TESTS / "fixtures" / "rust" / "assumptions.rs"


def load(name):
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), SCRIPTS / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def run_main(module, argv):
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        status = module.main(argv)
    return status, json.loads(out.getvalue())


sys.path.insert(0, str(SCRIPTS))
import kani_plugin_lib as lib  # noqa: E402

summarize = load("summarize-results")
coverage = load("summarize-coverage")
vacuity = load("check-vacuity")
assumptions = load("check-assumptions")
inspect = load("inspect-harness")
version = load("check-kani-version")
proof_diff = load("check-proof-diff")
witness = load("check-witness")
vacuity_main = vacuity
specs = load("check-specs")
DIFF = TESTS / "fixtures" / "rust" / "diff"


class SummarizeTest(unittest.TestCase):
    def test_failure_fixture(self):
        status, s = run_main(summarize, [str(JSON_FIXTURES / "fixture-completed-failure.json"), "--max-failed", "1"])
        self.assertEqual(status, 0)
        [h] = s["harnesses"]
        self.assertEqual(h["verdict"], "FAILURE")
        self.assertEqual(h["failed"],
                         [{"class": "assertion",
                           "description": "attempt to add with overflow",
                           "location": "test.rs:10"}])
        self.assertEqual(h["failed_omitted"], 1)
        self.assertEqual(h["unsatisfiable_covers"], 1)
        self.assertEqual(h["warnings"], 4)
        self.assertIn("contract", h["flags"])
        self.assertEqual(h["unreachable_checks"], 0)
        self.assertEqual(h["unsupported"], [
                         {"description": "not currently supported by Kani", "location": "test.rs:9"}])
        self.assertEqual((s["failed"], s["inconclusive"], s["all_ok"]), (1, 0, False))

    def test_non_completed_outcomes(self):
        _, s = run_main(summarize, [str(JSON_FIXTURES / "fixture-non-completed-outcomes.json")])
        verdicts = {h["verdict"] for h in s["harnesses"]}
        self.assertTrue(verdicts <= {"CRASHED", "OUT_OF_MEMORY", "TIMEOUT", "SUCCESS", "FAILURE"}, verdicts)
        self.assertTrue(verdicts & {"CRASHED", "OUT_OF_MEMORY", "TIMEOUT"})

    def test_no_harnesses(self):
        _, s = run_main(summarize, [str(JSON_FIXTURES / "fixture-no-harnesses-selected.json")])
        self.assertEqual(s["harnesses"], [])
        self.assertEqual(s["run_state"], "NO_HARNESSES_SELECTED")

    def test_failures_only(self):
        doc = {"harnesses": [
            {"name": "a", "file": "f.rs", "line": 1, "outcome": {"kind": "COMPLETED", "verdict": "SUCCESS"}},
            {"name": "b", "file": "f.rs", "line": 2, "outcome": {"kind": "TIMEOUT"}},
        ]}
        self.assertEqual([h["name"] for h in summarize.summarize(doc, failures_only=True)["harnesses"]], ["b"])

    def test_weakened_checks_and_trusted(self):
        doc = {"configuration": {"checks": {"overflow": False, "unwinding": True, "prove_safety_only": True},
                                 "cbmc_args": ["--unwind", "2"]},
               "harnesses": [{"name": "h", "file": "f.rs", "line": 1,
                              "outcome": {"kind": "COMPLETED", "verdict": "SUCCESS"},
                              "attributes": {"stubs": [{"original": "a", "replacement": "b"}],
                                             "verified_stubs": ["c"]}}]}
        s = summarize.summarize(doc)
        self.assertEqual(s["weakened_checks"], ["--no-overflow-checks",
                         "--prove-safety-only", "--cbmc-args --unwind 2"])
        self.assertEqual(s["harnesses"][0]["trusted"], {"stubs": ["a -> b"], "verified_stubs": ["c"]})

    def test_default_checks_are_not_weakened(self):
        # The fixture enables every default check, but passes raw CBMC arguments.
        _, s = run_main(summarize, [str(JSON_FIXTURES / "fixture-completed-failure.json")])
        self.assertEqual(s["weakened_checks"], ["--cbmc-args --object-bits 16"])

    def test_unreadable_input(self):
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(summarize.main(["/nonexistent.json"]), 1)


def harness(name, verdict="SUCCESS", unreachable=(), covers=None, total=1):
    return {
        "name": name,
        "outcome": {"kind": "COMPLETED", "verdict": verdict},
        "checks": {"total": total, "unreachable": list(unreachable)},
        "covers": covers or {"total": 1, "satisfied": [f"{name}.cover.1"], "unsatisfiable": [], "unreachable": []},
    }


class VacuityTest(unittest.TestCase):
    def rules(self, doc):
        return {(f["harness"], f["rule"], f["severity"]) for f in vacuity.check(doc)["findings"]}

    def test_clean(self):
        self.assertEqual(self.rules({"harnesses": [harness("h", total=3)]}), set())

    def test_unreachable_assertion(self):
        rules = self.rules({"harnesses": [harness("h", unreachable=["h.assertion.1"], total=1)]})
        self.assertIn(("h", "unreachable-assertion", "warning"), rules)
        self.assertIn(("h", "all-checks-unreachable", "warning"), rules)

    def test_log_names_with_spaces_and_locations(self):
        log = ("Check 1: <impl std::ops::Neg for Duration>::neg.assertion.1\n\t - Status: UNREACHABLE\n"
               "\t - Description: \"attempt to negate with overflow\"\n\t - Location: src/ops.rs:3:4 in function neg\n"
               "Check 2: h::{closure#0}.assertion.1\n\t - Status: UNREACHABLE\n"
               "\t - Description: \"assertion failed: ok\"\n\t - Location: src/lib.rs:9:9 in function h::{closure#0}\n"
               "Check 3: callee.assertion.1\n\t - Status: UNREACHABLE\n"
               "\t - Description: \"assertion failed: y\"\n\t - Location: src/lib.rs:20:5 in function callee\n"
               "Check 4: ptr.assertion.1\n\t - Status: UNREACHABLE\n"
               "\t - Description: \"Cannot compute offset of a ZST\"\n"
               "\t - Location: /x/kani/library/kani_core/src/mem.rs:1:1 in function kani::mem::offset\n")
        ids = ["<impl std::ops::Neg for Duration>::neg.assertion.1", "h::{closure#0}.assertion.1",
               "callee.assertion.1", "ptr.assertion.1"]
        doc = {"harnesses": [harness("h", unreachable=ids, total=9)]}
        found = {(f.get("property"), f["rule"], f["severity"])
                 for f in vacuity.check(doc, vacuity.parse_log(log))["findings"]}
        self.assertEqual(found, {
            (ids[0], "unreachable-builtin-check", "info"),
            (ids[1], "unreachable-assertion", "error"),
            (ids[2], "unreachable-callee-assertion", "warning"),
            (ids[3], "unreachable-builtin-check", "info"),
        })

    def test_log_classifies_unreachable_checks(self):
        log = ('Check 1: h.assertion.1\n\t - Status: UNREACHABLE\n\t - Description: "attempt to negate with overflow"\n'
               'Check 2: h.assertion.2\n\t - Status: UNREACHABLE\n\t - Description: "assertion failed: x > 0"\n'
               '\t - Location: f.rs:1:1 in function h\n')
        doc = {"harnesses": [harness("h", unreachable=["h.assertion.1", "h.assertion.2"], total=3)]}
        found = {(f.get("property"), f["rule"], f["severity"])
                 for f in vacuity.check(doc, vacuity.parse_log(log))["findings"]}
        self.assertEqual(found, {("h.assertion.1", "unreachable-builtin-check", "info"),
                                 ("h.assertion.2", "unreachable-assertion", "error")})

    def test_unreachable_default_check_is_not_an_assertion(self):
        rules = self.rules({"harnesses": [harness("h", unreachable=["h.arithmetic_overflow.1"], total=2)]})
        self.assertEqual(rules, set())

    def test_unsatisfiable_cover(self):
        covers = {"total": 1, "satisfied": [], "unsatisfiable": ["h.cover.1"], "unreachable": []}
        self.assertIn(("h", "unsatisfiable-cover", "warning"), self.rules({"harnesses": [harness("h", covers=covers)]}))

    def test_no_probe_is_info_only(self):
        doc = {"harnesses": [harness("h", covers={"total": 0})]}
        self.assertEqual(self.rules(doc), {("h", "no-vacuity-probe", "info")})

    def test_failures_are_skipped(self):
        result = vacuity.check({"harnesses": [harness("h", verdict="FAILURE", unreachable=["h.assertion.1"])]})
        self.assertEqual(result, {"findings": [], "skipped_unsuccessful": ["h"]})


class AssumptionsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.status, cls.result = run_main(assumptions, [str(RUST)])
        cls.rules = {(f["harness"], f["rule"]) for f in cls.result["findings"]}

    def test_exit_status(self):
        self.assertEqual(self.status, 2)

    def test_counts_all_harnesses(self):
        self.assertEqual(self.result["harnesses_checked"], 7)

    def test_rules(self):
        expected = {
            ("contradictory", "contradictory-bounds"),
            ("assume_false", "assume-false"),
            ("pinned_and_late", "assume-success"),
            ("pinned_and_late", "pinned-value"),
            ("pinned_and_late", "assume-after-assert"),
            (None, "contradictory-bounds"),
        }
        self.assertEqual(self.rules, expected)

    def test_contract_line(self):
        [f] = [f for f in self.result["findings"] if f["harness"] is None]
        self.assertEqual(f["line"], 7)

    def test_harness_filter(self):
        status, result = run_main(assumptions, [str(RUST), "--harness", "clean"])
        self.assertEqual(result["harnesses_checked"], 1)
        # The contract finding is file-level, so it is still reported.
        self.assertEqual({f["rule"] for f in result["findings"]}, {"contradictory-bounds"})


class InspectTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        _, result = run_main(inspect, [str(RUST)])
        cls.by_name = {h["name"]: h for h in result["harnesses"]}

    def test_finds_cfg_attr_and_contract_harnesses(self):
        self.assertIn("pinned_and_late", self.by_name)
        self.assertEqual(self.by_name["contract_harness"]["kind"], "proof_for_contract")
        self.assertEqual(self.by_name["contract_harness"]["contract_target"], "contract_ok")
        self.assertEqual(self.by_name["contract_harness"]["attributes"], {"stub": ["a::b, c::d"]})

    def test_flags(self):
        self.assertEqual(
            self.by_name["no_inputs"]["flags"], [
                "no-nondeterminism", "no-assertions", "loop-without-unwind"])
        self.assertEqual(self.by_name["pinned_and_late"]["flags"], [])
        self.assertEqual(self.by_name["clean"]["flags"], [])

    def test_body_ends_at_matching_brace(self):
        # The `'}'` char literal in `no_inputs` must not end the body early.
        self.assertEqual(self.by_name["no_inputs"]["calls"], ["helper", "len"])

    def test_counts(self):
        self.assertEqual(
            self.by_name["clean"]["counts"], {
                "nondet_inputs": 1, "assumptions": 1, "assertions": 1, "covers": 1})


class CoverageTest(unittest.TestCase):
    def raw(self, directory, name, regions):
        """Write a `*_kaniraw.json` file; regions are (file, start line, end line, function, status)."""
        data = {}
        for file, start, end, function, status in regions:
            data.setdefault(file, []).append({
                "function": function, "term": {"Counter": 0}, "status": status,
                "region": {"file": file, "start": [start, 1], "end": [end, 2]}})
        (Path(directory) / f"{name}_kaniraw.json").write_text(json.dumps({"data": data}))

    def test_merge_covers_a_region_when_any_harness_covers_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.raw(tmp, "h1", [("src/a.rs", 1, 3, "f", "COVERED"), ("src/a.rs", 5, 6, "f", "UNCOVERED"),
                                 ("src/b.rs", 1, 2, "g", "UNCOVERED")])
            self.raw(tmp, "h2", [("src/a.rs", 1, 3, "f", "UNCOVERED"), ("src/a.rs", 5, 6, "f", "COVERED"),
                                 ("src/b.rs", 1, 2, "g", "UNCOVERED")])
            regions, n, bad = coverage.merge([tmp])
            self.assertEqual((n, bad), (2, []))
            s = coverage.summarize(regions)
            self.assertEqual(s["total"], {"covered": 2, "regions": 3, "percent": 67})
            [a, b] = s["files"]
            self.assertEqual((a["file"], a["covered"], a["total"], a["uncovered"]), ("src/a.rs", 2, 2, []))
            self.assertEqual(b["uncovered"], [{"line": 1, "end_line": 2, "function": "g"}])
            self.assertEqual(b["uncovered_omitted"], 0)
            self.assertEqual([f["file"] for f in coverage.summarize(regions, "../b.rs")["files"]], ["src/b.rs"])
            self.assertIn("not the whole crate", s["scope"])

    def test_generic_instances_are_merged_by_source_span(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.raw(tmp, "h1", [("src/a.rs", 5, 6, "closure_of_instance_1", "COVERED")])
            self.raw(tmp, "h2", [("src/a.rs", 5, 6, "closure_of_instance_2", "UNCOVERED")])
            regions, _, _ = coverage.merge([tmp])
            self.assertEqual(coverage.summarize(regions)["files"][0]["uncovered"], [])

    def test_dependencies_corrupt_files_and_unreached_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.raw(tmp, "h1", [("src/a.rs", 1, 2, "f", "COVERED"),
                                 ("/home/u/.cargo/registry/src/x/lib.rs", 1, 2, "g", "UNCOVERED")])
            (Path(tmp) / "killed_kaniraw.json").write_text("")
            regions, n, bad = coverage.merge([tmp])
            self.assertEqual((n, bad), (1, ["killed_kaniraw.json"]))
            s = coverage.summarize(regions, src_files=["src/a.rs", "src/never_touched.rs"])
            self.assertEqual([f["file"] for f in s["files"]], ["src/a.rs"])
            self.assertEqual(s["excluded_dependency_files"], 1)
            self.assertEqual(s["files_without_regions"], ["src/never_touched.rs"])
            self.assertEqual(len(coverage.summarize(regions, all_files=True)["files"]), 2)

    def test_uncovered_list_is_limited(self):
        regions = {("a.rs", (i, 1), (i, 2)): {"covered": False, "functions": {"f"}} for i in range(1, 6)}
        f = coverage.summarize(regions, max_uncovered=2)["files"][0]
        self.assertEqual((len(f["uncovered"]), f["uncovered_omitted"]), (2, 3))

    def test_compare_and_log(self):
        with tempfile.TemporaryDirectory() as one, tempfile.TemporaryDirectory() as two:
            self.raw(one, "h", [("a.rs", 1, 2, "f", "COVERED"), ("a.rs", 4, 5, "f", "UNCOVERED")])
            self.raw(two, "h", [("a.rs", 1, 2, "f", "UNCOVERED"), ("a.rs", 4, 5, "f", "COVERED")])
            a, _, _ = coverage.merge([one])
            b, _, _ = coverage.merge([two])
            diff = coverage.compare(a, b)
            self.assertEqual([r["line"] for r in diff["covered_only_in_first"]], [1])
            self.assertEqual([r["line"] for r in diff["covered_only_in_second"]], [4])
            log = Path(one) / "kani.log"
            log.write_text(f"[info] Coverage results saved to {two}\n")
            self.assertEqual(coverage.dirs_from_log(log), [two])

    def test_no_data_is_an_error(self):
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(coverage.main(["/nonexistent"]), 1)


class ReportedIssuesTest(unittest.TestCase):
    """Regressions from testing on public crates."""

    def test_summary_counts_inconclusive_separately(self):
        def entry(name, outcome):
            return {"name": name, "file": "f.rs", "line": 1, "outcome": outcome}
        doc = {"harnesses": [entry("a", {"kind": "TIMEOUT"}), entry("b", {"kind": "OUT_OF_MEMORY"}),
                             entry("c", {"kind": "COMPLETED", "verdict": "SUCCESS"})]}
        s = summarize.summarize(doc)
        self.assertEqual((s["total"], s["successful"], s["failed"],
                         s["inconclusive"], s["all_ok"]), (3, 1, 0, 2, False))
        self.assertEqual(s["by_verdict"], {"OUT_OF_MEMORY": 1, "SUCCESS": 1, "TIMEOUT": 1})
        self.assertTrue(summarize.summarize(
            {"harnesses": [entry("c", {"kind": "COMPLETED", "verdict": "SUCCESS"})]})["all_ok"])

    def test_log_diagnoses_causes(self):
        log = ("Checking harness proofs::a...\nFailed Checks: unwinding assertion loop 0\nVERIFICATION:- FAILED\n"
               "Checking harness proofs::b...\nCBMC timed out after 300s\n"
               "Thread 3: Autoharness: Checking function count_down's contract against all possible inputs...\n"
               "Thread 3: Not unwinding recursion drop\n")
        d = summarize.diagnose_log(log)
        self.assertEqual({k: v["cause"] for k, v in d.items()},
                         {"proofs::a": "unwind-bound", "proofs::b": "timeout", "count_down": "recursion"})
        doc = {"harnesses": [{"name": "proofs::a", "file": "f.rs", "line": 1, "outcome": {"kind": "TIMEOUT"}}]}
        self.assertEqual(summarize.summarize(doc, diagnoses=d)["harnesses"][0]["diagnosis"]["cause"], "unwind-bound")
        finished, unfinished = summarize.verdicts_from_log(log)
        self.assertEqual([(f["name"], f["verdict"]) for f in finished], [("proofs::a", "FAILURE")])
        self.assertEqual(unfinished, ["proofs::b", "count_down"])

    def test_missing_export_is_explained(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "k.log"
            log.write_text("Checking harness h...\nVERIFICATION:- SUCCESSFUL\n")
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                self.assertEqual(summarize.main([str(Path(tmp) / "none.json"), "--log", str(log)]), 1)
            msg = json.loads(out.getvalue())
            self.assertIn("killed", msg["why"])
            self.assertEqual(msg["verdicts_in_log"], [{"name": "h", "verdict": "SUCCESS"}])

    def test_vacuity_counts_info_and_exempts_library_checks_and_should_panic(self):
        doc = {"harnesses": [
            {"name": "h", "outcome": {"kind": "COMPLETED", "verdict": "SUCCESS"}, "covers": {"total": 0},
             "checks": {"total": 3, "unreachable": ["core::fmt::Arguments::<'_>::from_str.assertion.1"]}},
            {"name": "sp", "outcome": {"kind": "COMPLETED", "verdict": "SUCCESS"}, "attributes": {"should_panic": True},
             "covers": {"total": 1, "unreachable": ["sp.cover.1"]}, "checks": {"total": 1, "unreachable": []}},
            {"name": "auto", "is_automatically_generated": True, "outcome": {"kind": "COMPLETED", "verdict": "SUCCESS"},
             "covers": {"total": 0}, "checks": {"total": 1, "unreachable": []}}]}
        raw = vacuity.check(doc)["findings"]
        self.assertEqual({(f["harness"], f["rule"]) for f in raw},
                         {("h", "unreachable-builtin-check"), ("h", "no-vacuity-probe")})
        self.assertTrue(all("description" in f for f in raw))
        condensed = vacuity.condense({"findings": raw, "skipped_unsuccessful": []}, show_info=False)
        self.assertEqual(condensed["findings"], [])
        self.assertEqual(condensed["info_summary"]["unreachable-builtin-check"], {"count": 1, "harnesses": 1})
        self.assertEqual(len(vacuity.condense({"findings": raw}, show_info=True)["findings"]), 2)

    def test_harness_filters_accept_qualified_names_and_modules(self):
        src = "#[cfg(kani)]\nmod verification {\n    mod proofs {\n        #[kani::proof]\n        fn check_x() {}\n    }\n}\n"
        fns = [f for f in lib.parse_functions(src, "src/lib.rs") if lib.harnesses_of([f])]
        self.assertEqual(lib.qualified_name(fns[0]), "verification::proofs::check_x")
        for flt in ("check_x", "proofs::check_x", "verification::proofs::check_x", "proofs", "verification"):
            self.assertTrue(lib.harness_matches(fns[0], flt), flt)
        self.assertFalse(lib.harness_matches(fns[0], "check"))
        self.assertIsNone(lib.unmatched_message(["proofs"], fns))
        self.assertIn("no harness matches ['nope']", lib.unmatched_message(["nope"], fns))
        self.assertIn("verification::proofs::check_x", lib.unmatched_message(["nope"], fns))

    def test_cfg_any_kani_is_kani_only(self):
        for arg, expected in [("kani", True), ("any(kani, test)", True), ("all(kani, feature = \"x\")", True),
                              ("not(kani)", False), ("test", False), ("feature = \"kani_extra\"", False)]:
            self.assertEqual(lib.cfg_mentions_kani(arg), expected, arg)
        src = "#[cfg(any(kani, test))]\nfn helper() {}\n#[cfg(not(kani))]\nfn real() {}\n"
        by = {f.name: f.in_kani for f in lib.parse_functions(src)}
        self.assertEqual(by, {"helper": True, "real": False})

    def test_hidden_and_target_dirs_are_not_scanned(self):
        with tempfile.TemporaryDirectory() as tmp:
            for d in ("src", ".kani-out/specs/src", "target/debug", ".git"):
                (Path(tmp) / d).mkdir(parents=True)
                (Path(tmp) / d / "x.rs").write_text("fn f() {}")
            found = [str(f.relative_to(tmp)) for f in lib.rust_files([tmp])]
            self.assertEqual(found, ["src/x.rs"])

    def test_witness_matches_panics_by_class(self):
        def panic(msg):
            return ("src/lib.rs:5", msg)
        m = witness.matches
        self.assertTrue(m(panic("assertion `left == right` failed\n  left: 1"),
                          "This is a placeholder message; Kani doesn't support message formatted at runtime"))
        self.assertTrue(m(panic("index out of bounds: the len is 2 but the index is 2"),
                          "index out of bounds: the length is less than or equal to the given index"))
        self.assertTrue(m(panic("attempt to add with overflow"), "attempt to add with overflow"))
        self.assertTrue(m(panic("assertion failed: x < 3"), "assertion failed: x < 3"))
        self.assertFalse(m(panic("attempt to add with overflow"), "index out of bounds"))
        self.assertFalse(m(panic("explicit panic"), "assertion failed: x < 3"))
        self.assertIsNone(m(panic("anything"), None))
        self.assertFalse(m(None, "anything"))

    def test_witness_build_problems_and_std_imports(self):
        self.assertEqual(witness.build_problem("error: cannot find macro `vec` in this scope"), "needs-std")
        self.assertEqual(witness.build_problem("error: unexpected `cfg` condition name: `kani`"), "deny-warnings")
        self.assertIsNone(witness.build_problem("error[E0308]: mismatched types"))
        self.assertEqual(witness.file_top_insert("#![no_std]\n//! doc\n\nfn f() {}", "extern crate std;"),
                         "#![no_std]\n//! doc\n\nextern crate std;\nfn f() {}")
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "src"
            src.mkdir()
            (src /
             "lib.rs").write_text("#![no_std]\n#[cfg(kani)]\nmod w {\n    #[test]\n    fn t() { let _ = vec![1]; }\n}\n")
            witness.add_std_imports(tmp, "t")
            text = (src / "lib.rs").read_text()
            self.assertIn("extern crate std;", text)
            self.assertIn("use std::{vec, vec::Vec};", text)
            witness.relax_warnings(tmp)

    def test_proof_diff_new_harness_stubs_macros_and_allowed_changes(self):
        base = "pub fn f(x: u8) -> u8 { x }\nmake_things!(a, 1);\n"
        new = ("pub fn f(x: u8) -> u8 { x + 1 }\nmake_things!(a, 2);\n#[cfg(any(kani, test))]\nfn helper() {}\n"
               "#[cfg(kani)]\nmod p {\n    #[kani::proof]\n    #[kani::stub(f, g)]\n    fn h() { f(kani::any()); }\n"
               "    fn g(_x: u8) -> u8 { kani::assume(false); 0 }\n}\n")
        rules = {(f["item"], f["rule"], f["severity"]) for f in proof_diff.diff_file(Path("a.rs"), base, new)}
        self.assertIn(("f", "code-changed", "error"), rules)
        self.assertIn(("make_things", "macro-invocation-changed", "warning"), rules)
        self.assertIn(("h", "new-harness-stub", "warning"), rules)
        self.assertFalse(any(item == "helper" for item, _, _ in rules))
        allowed = {(f["item"], f["rule"])
                   for f in proof_diff.diff_file(Path("a.rs"), base, new, allow=frozenset({"f"}))}
        self.assertIn(("f", "approved-code-change"), allowed)

    def test_proof_diff_reports_stub_body_assumptions(self):
        new = (
            "#[cfg(kani)]\nmod p {\n    #[kani::proof]\n    #[kani::stub(real, stubs::fake)]\n    fn h() { real(1); }\n"
            "    mod stubs { pub fn fake(_x: u8) -> u8 { kani::assume(true); 0 } }\n}\nfn real(_x: u8) -> u8 { 0 }\n")
        [f] = [
            f for f in proof_diff.diff_file(
                Path("a.rs"),
                "fn real(_x: u8) -> u8 { 0 }\n",
                new) if f["rule"] == "harness-added"]
        self.assertIn("assume(true)", f["message"])


class ProofDiffTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        findings = proof_diff.diff_file(DIFF / "after.rs", (DIFF / "before.rs").read_text(),
                                        (DIFF / "after.rs").read_text())
        cls.rules = {(f["item"], f["rule"], f["severity"]) for f in findings}

    def test_harness_weakening(self):
        for rule, severity in [("assertion-removed", "error"), ("cover-removed", "warning"),
                               ("assumption-changed", "warning"), ("stub-added", "warning"),
                               ("should-panic-added", "error"), ("unwind-lowered", "warning")]:
            self.assertIn(("check_helper", rule, severity), self.rules)

    def test_harnesses_added_and_removed(self):
        self.assertIn(("check_gone", "harness-removed", "error"), self.rules)
        self.assertIn(("check_new", "harness-added", "info"), self.rules)

    def test_contract_changes(self):
        self.assertIn(("div", "requires-changed", "warning"), self.rules)
        self.assertIn(("div", "ensures-removed", "warning"), self.rules)

    def test_code_changes(self):
        self.assertIn(("helper", "code-changed", "error"), self.rules)
        self.assertIn(("added", "code-added", "warning"), self.rules)
        self.assertIn(("removed_later", "code-removed", "error"), self.rules)

    def test_unchanged_and_kani_only_code_is_quiet(self):
        items = {item for item, _, _ in self.rules}
        self.assertNotIn("method", items)
        self.assertNotIn("kani_only_helper", items)
        self.assertFalse(any(item == "div" and rule == "code-changed" for item, rule, _ in self.rules))

    def test_no_changes(self):
        src = (DIFF / "before.rs").read_text()
        self.assertEqual(proof_diff.diff_file(DIFF / "before.rs", src, src), [])


class ParserRegressionTest(unittest.TestCase):
    """Cases from testing on real crates."""

    def test_multiline_contract_attributes(self):
        before = """impl Neg for Duration {
    #[cfg_attr(kani, kani::ensures(|result: &Duration| {
        result.total_nanoseconds() == -self.total_nanoseconds()
    }))]
    fn neg(self) -> Self { self }
}
"""
        after = before.replace("== -self", "<= -self")
        rules = {f["rule"] for f in proof_diff.diff_file(Path("ops.rs"), before, after)}
        self.assertEqual(rules, {"ensures-changed"})

    def test_assumptions_in_helpers_count(self):
        before = """#[cfg(kani)]
mod proofs {
    fn any_small() -> u8 { let x: u8 = kani::any(); x }
    #[kani::proof]
    fn check() { let x = any_small(); assert!(x < 255 || x == 255); }
}
"""
        after = before.replace("let x: u8 = kani::any(); x", "let x: u8 = kani::any(); kani::assume(x < 3); x")
        rules = {(f["item"], f["rule"]) for f in proof_diff.diff_file(Path("p.rs"), before, after)}
        self.assertIn(("check", "assumption-added"), rules)
        self.assertNotIn(("any_small", "code-changed"), rules)
        fns = proof_diff.parse_functions(after, "p.rs")
        index = lib.helper_index(fns)
        [check] = [f for f in fns if f.name == "check"]
        [h] = lib.harnesses_of([check])
        found = {f["rule"] for f in assumptions.check_harness(h, lib.helper_closure(check, index))}
        self.assertEqual(found, set())  # one bound on x is not suspicious by itself
        described = inspect.describe(h, lib.helper_closure(check, index))
        self.assertEqual(described["counts"]["assumptions"], 1)
        self.assertEqual(described["counts"]["nondet_inputs"], 1)
        self.assertNotIn("no-nondeterminism", described["flags"])

    def test_out_of_line_kani_modules(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "src"
            (src / "kani_helpers").mkdir(parents=True)
            (src / "lib.rs").write_text("#[cfg(kani)]\nmod proofs;\nmod real;\n")
            (src / "proofs.rs").write_text("mod nested;\nfn clamp_total() {}\n")
            (src / "proofs").mkdir()
            (src / "proofs" / "nested.rs").write_text("fn any_string() {}\n")
            (src / "real.rs").write_text("fn real() {}\n")
            files = list(lib.rust_files([src]))
            kani_only = {f.name for f in lib.kani_only_files(files)}
            self.assertEqual(kani_only, {"proofs.rs", "nested.rs"})

    def test_multiline_stub_is_normalized(self):
        src = "#[kani::proof]\n#[kani::stub(\n    a::b,\n    c::d\n)]\nfn h() {}\n"
        [h] = lib.harnesses_of(lib.parse_functions(src))
        self.assertEqual(h.attributes["stub"], ["a::b, c::d"])

    def test_harness_attribute_removed(self):
        before = "#[cfg(kani)]\nmod p {\n    #[kani::proof]\n    fn check() { assert!(true); }\n}\n"
        after = before.replace("    #[kani::proof]\n", "")
        rules = {(f["item"], f["rule"], f["severity"]) for f in proof_diff.diff_file(Path("p.rs"), before, after)}
        self.assertEqual(rules, {("check", "harness-attribute-removed", "error")})

    def test_helpers_resolve_by_path(self):
        src = """#[cfg(kani)]
mod proofs {
    #[kani::proof]
    fn check() { let x = kani::any::<u32>(); let q = Queue::any(); let y = small(); let z = q.any(); }
    fn small() -> u32 { kani::any() }
}
#[cfg(kani)]
impl kani::Arbitrary for Queue { fn any() -> Self { kani::assume(true); Queue } }
#[cfg(kani)]
impl kani::Arbitrary for Other { fn any() -> Self { kani::assume(false); Other } }
"""
        fns = lib.parse_functions(src, "q.rs")
        index = lib.helper_index(fns)
        [check] = [f for f in fns if f.name == "check"]
        found = sorted((h.name, lib.IMPL_TYPE.search(h.scope_header).group(1) if h.scope == "impl" else "")
                       for h in lib.helper_closure(check, index))
        self.assertEqual(found, [("any", "Queue"), ("small", "")])

    def test_any_where_is_an_assumption(self):
        before = "#[cfg(kani)]\nmod p {\n    #[kani::proof]\n    fn check() { let l: usize = kani::any(); }\n}\n"
        after = before.replace("kani::any()", "kani::any_where(|l| *l <= 4)")
        rules = {f["rule"] for f in proof_diff.diff_file(Path("p.rs"), before, after)}
        self.assertEqual(rules, {"assumption-added"})

    def test_added_ensures_is_described_as_added(self):
        before = "fn f(x: u8) -> u8 { x }\n"
        after = "#[kani::ensures(|r: &u8| *r == x)]\n" + before
        [f] = proof_diff.diff_file(Path("f.rs"), before, after)
        self.assertEqual((f["rule"], f["severity"]), ("ensures-added", "info"))
        self.assertIn("postcondition added", f["message"])

    def test_inspect_recognizes_more_generators(self):
        src = "#[kani::proof]\nfn h() { let a: [u8; 4] = kani::Arbitrary::any_array::<4>(); assert!(a.len() == 4); }\n"
        [h] = lib.harnesses_of(lib.parse_functions(src))
        self.assertNotIn("no-nondeterminism", inspect.describe(h)["flags"])

    def test_loose_files_are_not_owned_by_an_enclosing_package(self):
        self.assertEqual(lib.find_root(RUST), (None, None))


class SpecsPlanTest(unittest.TestCase):
    def test_supported_types(self):
        for ty in ["u8", "&u32", "&mut [u8; 4]", "(u8, bool)", "Option<i64>", "core::num::NonZeroU32", "Point"]:
            self.assertTrue(specs.supported(ty, {"Point"}), ty)
        for ty in ["&[u8]", "&str", "Vec<u8>", "*const u8", "&&u8", "Box<u8>"]:
            self.assertFalse(specs.supported(ty, set()), ty)

    def test_unsupported_functions_are_reported_unchecked(self):
        src = """
#[kani::requires(x.len() > 0)]
fn slice(x: &[u8]) {}
impl S { #[kani::requires(self.0 > 0)] fn m(&self) {} }
#[kani::requires(true)]
fn generic<T>(t: T) {}
"""
        plan = specs.Plan()
        for fn in specs.parse_functions(src, "f.rs"):
            specs.plan_function(plan, fn, None, set())
        self.assertEqual(plan.checks, [])
        self.assertEqual(len(plan.unchecked), 3)

    def test_generated_checks(self):
        src = "#[kani::requires(a != 0)]\n#[kani::requires(b < 10)]\nfn f(a: u8, b: &u8) -> u8 { a }\n"
        plan = specs.Plan()
        [fn] = specs.parse_functions(src, "f.rs")
        specs.plan_function(plan, fn, ["a != 0"], set())
        self.assertEqual([c["kind"] for c in plan.checks],
                         ["sat", "clause", "clause", "new_implies_old", "old_implies_new"])
        [(offset, text)] = plan.insertions["f.rs"]
        self.assertEqual(offset, len(src) - 1)
        self.assertIn("fn __kani_spec_pred_f_3(a: u8, b: &u8) -> [bool; 4] { [(a != 0) && (b < 10), (a != 0), "
                      "(b < 10), (a != 0)] }", text)
        self.assertIn("__kani_spec_pred_f_3(a, &__kani_spec_v_b);", text)

    def test_methods_use_an_inherent_impl(self):
        src = """impl Neg for Duration {
    #[cfg_attr(kani, kani::requires(self.total() != 0))]
    fn neg(self) -> Self { self }
}
impl Duration {
    #[kani::requires(days < 10.0)]
    pub fn from_days(days: f64) -> Self { todo!() }
}
"""
        plan = specs.Plan()
        for fn in specs.parse_functions(src, "f.rs"):
            specs.plan_function(plan, fn, None, {"Duration"})
        self.assertEqual(plan.unchecked, [])
        texts = [t for _, t in plan.insertions["f.rs"]]
        self.assertIn("impl Duration {", texts[0])
        self.assertIn("fn __kani_spec_pred_neg_3(self) -> [bool; 2]", texts[0])
        self.assertIn("__kani_spec_self.__kani_spec_pred_neg_3()", texts[0])
        self.assertIn("Duration::__kani_spec_pred_from_days_7(days)", texts[1])
        self.assertEqual({o for o, _ in plan.insertions["f.rs"]}, {src.index("}\nimpl Duration") + 1, len(src) - 1})

    def test_join_option_values(self):
        self.assertEqual(specs.join_option_values(["src", "--kani-arg", "-Z", "--kani-arg", "quantifiers"],
                                                  {"--kani-arg"}),
                         ["src", "--kani-arg=-Z", "--kani-arg=quantifiers"])

    def test_arbitrary_types_are_discovered(self):
        self.assertEqual(specs.arbitrary_types([TESTS / "fixtures" / "rust" / "assumptions.rs"]), set())


class VersionTest(unittest.TestCase):
    compat = {"plugin_version": "0.1.0", "min_kani": "0.68.0", "max_kani_exclusive": "1.0.0"}

    def test_range(self):
        self.assertTrue(version.check("kani 0.68.0", self.compat)["compatible"])
        self.assertTrue(version.check("cargo-kani 0.99.3", self.compat)["compatible"])
        self.assertFalse(version.check("kani 0.67.1", self.compat)["compatible"])
        self.assertFalse(version.check("kani 1.0.0", self.compat)["compatible"])

    def test_missing(self):
        r = version.check(None, self.compat)
        self.assertFalse(r["compatible"])
        self.assertIn("cargo install --locked kani-verifier", r["message"])


if __name__ == "__main__":
    unittest.main()
