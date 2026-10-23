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
