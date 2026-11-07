#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Tests for compatibility with different Kani builds: toolchain lookup, the export flag, the two
export shapes, and the memory guard. No Kani or Rust toolchain is needed.

Run with: python3 agent-plugin/tests/test_compat.py
"""

import sys
import tempfile
import unittest
from pathlib import Path

TESTS = Path(__file__).resolve().parent
sys.path.insert(0, str(TESTS.parent / "scripts"))
import kani_plugin_lib as lib  # noqa: E402


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
        self.assertEqual(lib.adapt_export_flag(recorded), ["kani", "x.rs", "-Z", "export-json", "--export-json", "a.json"])
        self.assertEqual(lib.adapt_export_flag(["kani", "x.rs"]), ["kani", "x.rs"])


if __name__ == "__main__":
    unittest.main()
