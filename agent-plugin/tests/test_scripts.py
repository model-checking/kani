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


summarize = load("summarize-results")
vacuity = load("check-vacuity")
assumptions = load("check-assumptions")
inspect = load("inspect-harness")
version = load("check-kani-version")
proof_diff = load("check-proof-diff")
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
        self.assertNotIn("unreachable_checks", h)

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
        self.assertIn(("h", "unreachable-assertion", "error"), rules)
        self.assertIn(("h", "all-checks-unreachable", "error"), rules)

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
        self.assertIn("kani::cover!((a != 0) && (b < 10)", text)
        self.assertIn("let b: &u8 = &__kani_spec_v_b;", text)

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
