# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT


import pathlib
import tempfile
import textwrap
import unittest

import benchcomp.parsers.kani_perf as kani_perf


class TestKaniPerfParser(unittest.TestCase):
    def parse(self, expected_out):
        """Run the parser over a single fake `expected.out` and return its benchmarks"""

        with tempfile.TemporaryDirectory() as tmp:
            out_dir = pathlib.Path(tmp) / "build" / "tests" / "perf" / "demo" / "expected"
            out_dir.mkdir(parents=True)
            (out_dir / "expected.out").write_text(textwrap.dedent(expected_out))
            return kani_perf.main(pathlib.Path(tmp))["benchmarks"]

    def test_solver_calls_are_counted(self):
        """CBMC solves a harness in several calls; count them and average the time over them.

        `solver_runtime` sums a number of calls that is not stable across runs of the same code
        (https://github.com/model-checking/kani/issues/4821), so the per-call time is what two
        runs can be compared on.
        """

        benchmarks = self.parse("""\
            Checking harness demo::check...
            Solving with CaDiCaL 3.0.0
            Runtime Solver: 2.0s
            Solving with CaDiCaL 3.0.0
            Runtime Solver: 4.0s
            size of program expression: 100 steps
            slicing removed 10 assignments
            Generated 20 VCC(s), 7 remaining after simplification
            Runtime Symex: 1.5s
            Verification Time: 8.0s
            VERIFICATION:- SUCCESSFUL
            """)

        metrics = benchmarks["demo/demo::check"]["metrics"]
        self.assertEqual(metrics["solver_calls"], 2)
        self.assertEqual(metrics["solver_runtime"], 6.0)
        self.assertEqual(metrics["solver_runtime_per_call"], 3.0)
        # unchanged by this addition
        self.assertEqual(metrics["number_program_steps"], 90)
        self.assertEqual(metrics["number_vccs"], 7)
        self.assertTrue(metrics["success"])

    def test_harness_without_solver_output(self):
        """A harness whose properties are all decided before solving has no solver metrics"""

        benchmarks = self.parse("""\
            Checking harness demo::check...
            size of program expression: 10 steps
            slicing removed 2 assignments
            Generated 4 VCC(s), 0 remaining after simplification
            Runtime Symex: 0.1s
            Verification Time: 0.2s
            VERIFICATION:- SUCCESSFUL
            """)

        metrics = benchmarks["demo/demo::check"]["metrics"]
        self.assertNotIn("solver_calls", metrics)
        self.assertNotIn("solver_runtime_per_call", metrics)
