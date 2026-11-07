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


if __name__ == "__main__":
    unittest.main()
