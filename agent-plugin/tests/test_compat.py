#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Tests for compatibility with different Kani builds: toolchain lookup, the export flag, the two
export shapes, and the memory guard. No Kani or Rust toolchain is needed.

Run with: python3 agent-plugin/tests/test_compat.py
"""

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

TESTS = Path(__file__).resolve().parent
SCRIPTS = TESTS.parent / "scripts"
sys.path.insert(0, str(SCRIPTS))
import kani_export  # noqa: E402
import kani_plugin_lib as lib  # noqa: E402

EXPORTS = TESTS / "fixtures" / "exports"
PROBE = TESTS / "fixtures" / "export-probe"


def load(name):
    spec = importlib.util.spec_from_file_location(name.replace("-", "_"), SCRIPTS / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class ToolchainLookupTest(unittest.TestCase):
    def test_source_build_binary_four_levels_below_the_toolchain_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            checkout = Path(tmp).resolve() / "kani"
            binary = checkout / "target" / "kani" / "bin" / "kani"
            binary.parent.mkdir(parents=True)
            binary.write_text("")
            (checkout / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "nightly-2026-10-23"\n')
            self.assertEqual(lib.toolchain_channel_above(binary), "nightly-2026-10-23")

    def test_nearest_file_wins_and_search_stops_there(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp).resolve()
            (root / "rust-toolchain.toml").write_text('[toolchain]\nchannel = "outer"\n')
            inner = root / "a" / "bin"
            inner.mkdir(parents=True)
            (root / "a" / "rust-toolchain.toml").write_text("[toolchain]\ncomponents = []\n")
            self.assertIsNone(lib.toolchain_channel_above(inner / "kani"))

    def test_release_layout_has_no_toolchain_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            binary = Path(tmp).resolve() / "kani-0.68.0" / "bin" / "kani"
            binary.parent.mkdir(parents=True)
            self.assertIsNone(lib.toolchain_channel_above(binary))


class ExportShapesTest(unittest.TestCase):
    """The same crate's export, as Kani 0.68.0 (tree) and as current Kani (flat), gives the same answers."""

    @classmethod
    def setUpClass(cls):
        cls.flat = kani_export.load_export(EXPORTS / "probe-flat.json", workspace=PROBE)
        cls.tree = kani_export.load_export(EXPORTS / "probe-tree.json", workspace=PROBE)
        cls.summarize = load("summarize-results")
        cls.vacuity = load("check-vacuity")

    def test_shapes_are_detected(self):
        self.assertEqual((self.flat["export_shape"], self.tree["export_shape"]), ("flat", "tree"))
        self.assertTrue(self.flat["configuration_known"])
        self.assertFalse(self.tree["configuration_known"])

    def test_summaries_agree(self):
        flat, tree = (self.summarize.summarize(d) for d in (self.flat, self.tree))
        for key in ("total", "successful", "failed", "inconclusive", "by_verdict", "all_ok", "kani", "run_state"):
            self.assertEqual(flat[key], tree[key], key)
        self.assertEqual(tree["total"], 4)
        self.assertFalse(tree["all_ok"])
        for a, b in zip(flat["harnesses"], tree["harnesses"]):
            for key in ("name", "verdict", "location", "failed", "unreachable_checks", "unsatisfiable_covers",
                        "covers_total", "unsupported_constructs", "flags"):
                self.assertEqual(a[key], b[key], f"{a['name']}: {key}")

    def test_stubs_are_trusted_constructs_in_both(self):
        for doc, source in ((self.flat, None), (self.tree, "source-scan")):
            stubbed = next(h for h in self.summarize.summarize(doc)["harnesses"] if h["name"] == "proofs::stubbed")
            self.assertEqual(stubbed["trusted"]["stubs"], ["expensive -> stub_expensive"])
            self.assertEqual(stubbed["trusted"]["verified_stubs"][0].split("::")[-1], "checked_double")
            self.assertEqual(stubbed["trusted"].get("stubs_source"), source)

    def test_autoharness_harnesses_have_no_stubs_by_construction(self):
        doc = json.loads((EXPORTS / "probe-tree.json").read_text())
        doc["harness_metadata"][0]["is_automatically_generated"] = True
        converted = kani_export.normalize_export(doc, workspace=PROBE)
        self.assertEqual(converted["harnesses"][0]["attributes"]["stubs_source"], "autoharness-generated")

    def test_missing_source_is_reported_not_guessed(self):
        with tempfile.TemporaryDirectory() as empty:
            doc = kani_export.load_export(EXPORTS / "probe-tree.json", workspace=empty)
        stubbed = next(h for h in doc["harnesses"] if h["name"] == "proofs::stubbed")
        self.assertEqual(stubbed["attributes"]["stubs_source"], "unknown")
        trusted = self.summarize.trusted_constructs(stubbed)
        self.assertIn("unknown", trusted["stubs_source"])

    def test_weakened_checks_are_flagged_as_undetectable(self):
        flat, tree = (self.summarize.summarize(d) for d in (self.flat, self.tree))
        self.assertTrue(flat["weakened_checks_detectable"])
        self.assertNotIn("weakened_checks_note", flat)
        self.assertFalse(tree["weakened_checks_detectable"])
        self.assertIn("cannot be detected", tree["weakened_checks_note"])

    def test_vacuity_findings_agree(self):
        flat, tree = (self.vacuity.check(d) for d in (self.flat, self.tree))

        def key(f):
            return (f["harness"], f["rule"], f.get("property"))

        self.assertEqual(sorted(map(key, flat["findings"])), sorted(map(key, tree["findings"])))
        self.assertEqual(flat["skipped_unsuccessful"], tree["skipped_unsuccessful"])
        self.assertTrue(any(f["rule"] == "unsatisfiable-cover" or "cover" in f["rule"] for f in tree["findings"]),
                        tree["findings"])

    def test_kinds_and_contracts(self):
        for doc in (self.flat, self.tree):
            by_name = {h["name"]: h for h in doc["harnesses"]}
            kind = by_name["proofs::check_checked_double"]["attributes"]["kind"]
            self.assertEqual(kind["ProofForContract"]["target_fn"], "checked_double")
            self.assertIsNotNone(by_name["proofs::check_checked_double"]["contract"])
            self.assertIsNone(by_name["proofs::passes"]["contract"])

    def test_started_at_and_tools(self):
        self.assertRegex(self.tree["started_at"], r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$")
        self.assertEqual(self.tree["tools"]["kani"], "0.68.0")

    def test_evidence_records_the_shape_and_what_cannot_be_detected(self):
        import subprocess
        for shape, detectable in (("flat", True), ("tree", False)):
            with tempfile.TemporaryDirectory() as out:
                done = subprocess.run(
                    [sys.executable, str(SCRIPTS / "evidence.py"), "record", "--export",
                     str(EXPORTS / f"probe-{shape}.json"), "--command", "kani src/lib.rs", "--cwd", str(PROBE),
                     "--out", out], capture_output=True, text=True)
                self.assertEqual(done.returncode, 0, done.stderr)
                [bundle] = Path(out).iterdir()
                manifest = json.loads((bundle / "bundle.json").read_text())
                self.assertEqual(manifest["export_shape"], shape)
                self.assertEqual(manifest["weakened_checks_detectable"], detectable)
                self.assertEqual(manifest["verdicts"]["proofs::stubbed"], "SUCCESS")

    def test_map_reads_both_shapes_and_flags_the_unknown_configuration(self):
        import verifiability_map as vm
        for shape, known in (("flat", True), ("tree", False)):
            [run] = vm.load_results([EXPORTS / f"probe-{shape}.json"], PROBE)
            self.assertEqual(len(run["doc"]["harnesses"]), 4)
            self.assertEqual(run["doc"]["configuration_known"], known)
        harness = {"id": "h", "result": {"status": "verified", "fresh": True, "freshness": "ok", "verdict": "SUCCESS",
                                         "unwind": None, "bounded": False, "time_s": 1, "kani_version": "0.68.0",
                                         "weakened_checks": [], "configuration_known": False, "stubs": [],
                                         "verified_stubs": [], "stubs_source": "unknown"},
                   "reaches": {"reachable": ["f"]}, "assumptions": 0}
        verification = vm.function_verification("f", [harness])
        self.assertEqual(verification["status"], "verified")
        self.assertTrue(any("does not record the run configuration" in c for c in verification["caveats"]))
        self.assertTrue(any("stubs are unknown" in c for c in verification["caveats"]))

    def test_run_harnesses_merges_tree_exports_into_a_readable_document(self):
        run_harnesses = load("run-harnesses")
        doc = kani_export.load_export(EXPORTS / "probe-tree.json", workspace=PROBE)
        with tempfile.TemporaryDirectory() as tmp:
            merged = Path(tmp) / "export.json"
            entries = doc["harnesses"] + [run_harnesses.synthetic_entry("proofs::slow", "TIMEOUT", 9.0)]
            run_harnesses.write_merged(merged, entries, doc, doc["started_at"], "PARTIAL")
            again = kani_export.load_export(merged)
        summary = self.summarize.summarize(again)
        self.assertEqual((summary["total"], summary["successful"], summary["failed"], summary["inconclusive"]),
                         (5, 3, 1, 1))
        self.assertFalse(summary["weakened_checks_detectable"])
        self.assertEqual(summary["export_shape"], "tree")

    def test_unknown_shape_is_an_error(self):
        with self.assertRaises(kani_export.ExportError) as ctx:
            kani_export.normalize_export({"something": 1})
        self.assertIn("unknown Kani export shape", str(ctx.exception))

    def test_harness_without_a_result_is_not_run(self):
        doc = json.loads((EXPORTS / "probe-tree.json").read_text())
        doc["verification_results"]["results"].pop()
        doc["verification_results"]["summary"]["status"] = "interrupted"
        converted = kani_export.normalize_export(doc, workspace=PROBE)
        self.assertEqual(converted["harnesses"][-1]["outcome"], {"kind": "NOT_RUN"})
        self.assertEqual(converted["run_state"], "PARTIAL")
        self.assertFalse(self.summarize.summarize(converted)["all_ok"])


FAKE_VERIFYING_KANI = """#!/bin/bash
# A stand-in for `kani` that verifies a trivial harness and writes $KANI_FAKE_EXPORT as its export.
case "$1" in --version) echo "kani 0.68.0"; exit 0;; esac
prev=""
for a in "$@"; do
  if [ "$prev" = "--export-json" ] && [ -n "$KANI_FAKE_EXPORT" ]; then cp "$KANI_FAKE_EXPORT" "$a"; fi
  prev="$a"
done
echo "VERIFICATION:- SUCCESSFUL"
"""


class VersionExportCheckTest(unittest.TestCase):
    """check-kani-version.py fails when the installed Kani's export cannot be read."""

    def setUp(self):
        import os
        self.version = load("check-kani-version")
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        (self.dir / "bin").mkdir()
        (self.dir / "bin" / "kani").write_text(FAKE_VERIFYING_KANI)
        (self.dir / "bin" / "kani").chmod(0o755)
        self.saved = dict(os.environ)
        os.environ["PATH"] = f"{self.dir / 'bin'}{os.pathsep}/usr/bin{os.pathsep}/bin"
        os.environ["RUSTUP_TOOLCHAIN"] = "test"
        os.environ["KANI_PLUGIN_EXPORT_FLAG"] = "export-json"

    def tearDown(self):
        import os
        os.environ.clear()
        os.environ.update(self.saved)
        self.tmp.cleanup()

    def run_check(self, export):
        import contextlib
        import io
        import os
        path = self.dir / "export.json"
        path.write_text(json.dumps(export))
        os.environ["KANI_FAKE_EXPORT"] = str(path)
        out = io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
            status = self.version.main(["--kani-cmd", "kani"])
        return status, json.loads(out.getvalue())

    def one_harness(self, shape):
        doc = json.loads((EXPORTS / f"probe-{shape}.json").read_text())
        if shape == "flat":
            doc["harnesses"] = doc["harnesses"][:1]
        else:
            doc["harness_metadata"] = doc["harness_metadata"][:1]
            doc["verification_results"]["results"] = doc["verification_results"]["results"][:1]
        return doc

    def test_both_shapes_are_readable(self):
        for shape in ("flat", "tree"):
            status, result = self.run_check(self.one_harness(shape))
            self.assertEqual(status, 0, (shape, result))
            self.assertTrue(result["compatible"])
            self.assertTrue(result["export_readable"])
            self.assertEqual(result["export_flags"], "-Z export-json")

    def test_unreadable_export_is_not_compatible(self):
        status, result = self.run_check({"some": "other shape"})
        self.assertEqual(status, 1)
        self.assertFalse(result["compatible"])
        self.assertIn("unknown Kani export shape", result["export_error"])
        self.assertIn("cannot read its --export-json output", result["message"])

    def test_export_with_no_harnesses_is_not_compatible(self):
        doc = self.one_harness("flat")
        doc["harnesses"] = []
        status, result = self.run_check(doc)
        self.assertEqual(status, 1)
        self.assertIn("read 0 harness(es)", result["export_error"])


FAKE_KANI = """#!/bin/bash
# A stand-in for `kani`: MODE=old rejects `-Z export-json` as Kani 0.68.0 does.
if [ "$1 $2" = "-Z export-json" ] && [ "$KANI_FAKE_MODE" = old ]; then
  echo "error: invalid value 'export-json' for '--unstable <UNSTABLE_FEATURE>'" >&2
  exit 2
fi
echo "kani 0.68.0"
"""


class ExportFlagTest(unittest.TestCase):
    def setUp(self):
        import os
        self.tmp = tempfile.TemporaryDirectory()
        bindir = Path(self.tmp.name) / "bin"
        bindir.mkdir()
        (bindir / "kani").write_text(FAKE_KANI)
        (bindir / "kani").chmod(0o755)
        self.saved = dict(os.environ)
        os.environ["PATH"] = f"{bindir}{os.pathsep}/usr/bin{os.pathsep}/bin"
        os.environ["RUSTUP_TOOLCHAIN"] = "test"
        os.environ.pop("KANI_PLUGIN_EXPORT_FLAG", None)
        lib._EXPORT_FLAG_CACHE.clear()

    def tearDown(self):
        import os
        os.environ.clear()
        os.environ.update(self.saved)
        lib._EXPORT_FLAG_CACHE.clear()
        self.tmp.cleanup()

    def test_new_kani_takes_export_json(self):
        import os
        os.environ["KANI_FAKE_MODE"] = "new"
        self.assertEqual(lib.export_flag_args(), ["-Z", "export-json"])

    def test_old_kani_takes_unstable_options(self):
        import os
        os.environ["KANI_FAKE_MODE"] = "old"
        self.assertEqual(lib.export_args("r.json"), ["-Z", "unstable-options", "--export-json", "r.json"])

    def test_override(self):
        import os
        os.environ["KANI_FAKE_MODE"] = "new"
        os.environ["KANI_PLUGIN_EXPORT_FLAG"] = "unstable-options"
        self.assertEqual(lib.export_flag_args(), ["-Z", "unstable-options"])

    def test_recorded_commands_are_respelled(self):
        import os
        os.environ["KANI_FAKE_MODE"] = "old"
        recorded = ["cargo", "kani", "-Z", "export-json", "--export-json", "a.json", "--harness", "h"]
        self.assertEqual(lib.adapt_export_flag(recorded),
                         ["cargo", "kani", "-Z", "unstable-options", "--export-json", "a.json", "--harness", "h"])
        os.environ["KANI_FAKE_MODE"] = "new"
        lib._EXPORT_FLAG_CACHE.clear()
        recorded = ["kani", "x.rs", "--export-json", "a.json"]
        self.assertEqual(
            lib.adapt_export_flag(recorded), [
                "kani", "x.rs", "-Z", "export-json", "--export-json", "a.json"])
        self.assertEqual(lib.adapt_export_flag(["kani", "x.rs"]), ["kani", "x.rs"])


if __name__ == "__main__":
    unittest.main()
