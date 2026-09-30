# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT


import pathlib
import unittest

import yaml

import benchcomp.visualizers.utils as utils


CONFIG = pathlib.Path(__file__).parents[2] / "configs" / "perf-regression.yaml"


def solver_runtime_check():
    """The `solver_runtime` check exactly as CI runs it"""

    with open(CONFIG) as handle:
        config = yaml.safe_load(handle)
    for viz in config["visualize"]:
        if viz["type"] != "error_on_regression":
            continue
        for check in viz["checks"]:
            if check["metric"] == "solver_runtime":
                return utils.SingleRegressionCheck(
                    check["metric"], check["test"], check.get("all_metrics", False))
    raise AssertionError("no solver_runtime check in perf-regression.yaml")


def metrics(solver_runtime, solver_calls=2, **overrides):
    result = {
        "number_vccs": 2399,
        "number_program_steps": 58765,
        "solver_variables": 20705,
        "solver_clauses": 30763,
        "solver_runtime": solver_runtime,
        "solver_calls": solver_calls,
        "solver_runtime_per_call": solver_runtime / solver_calls,
    }
    result.update(overrides)
    return result


class TestSolverRuntimeCheck(unittest.TestCase):
    def setUp(self):
        self.regressed = solver_runtime_check()

    def test_same_instance_noise_is_tolerated(self):
        """The widest same-instance spread seen on `main` (2.41x) must not fail a PR"""

        self.assertFalse(self.regressed(metrics(11.2), metrics(11.2 * 2.41)))

    def test_same_instance_large_slowdown_is_caught(self):
        """Identical counts do not exempt a harness: e.g. a solver-option change"""

        self.assertTrue(self.regressed(metrics(12.0), metrics(12.0 * 4.5)))

    def test_more_calls_at_the_same_per_call_time_is_caught(self):
        """The total is compared, so extra calls count even if each one is no slower"""

        self.assertTrue(self.regressed(
            metrics(12.0, solver_calls=2), metrics(60.0, solver_calls=10)))

    def test_changed_instance_uses_the_strict_threshold(self):
        for key, value in (("number_vccs", 2400), ("number_program_steps", 58766),
                           ("solver_variables", 20706), ("solver_clauses", 30764)):
            with self.subTest(changed=key):
                self.assertTrue(self.regressed(
                    metrics(12.0), metrics(12.0 * 1.6, **{key: value})))
                self.assertFalse(self.regressed(
                    metrics(12.0), metrics(12.0 * 1.4, **{key: value})))

    def test_short_runs_are_ignored(self):
        self.assertFalse(self.regressed(
            metrics(1.0, number_vccs=1), metrics(9.0, number_vccs=2)))

    def test_harness_without_solver_output(self):
        """All properties decided before solving: no solver metrics, no regression, no crash"""

        bare = {"number_vccs": 0, "number_program_steps": 10}
        self.assertFalse(self.regressed(bare, bare))
        self.assertFalse(self.regressed(metrics(30.0), bare))
        self.assertFalse(self.regressed(bare, metrics(9.0)))

    def test_newly_needing_the_solver_is_caught(self):
        """A harness that `main` decides without the solver but the change sends to it for 10s
        or more has regressed, although there is no old solver time to take a ratio of"""

        bare = {"number_vccs": 0, "number_program_steps": 10}
        self.assertTrue(self.regressed(bare, metrics(30.0)))
        self.assertTrue(self.regressed(metrics(0.0), metrics(30.0)))


class TestAllMetricsChecker(unittest.TestCase):
    def test_missing_label_metric_is_not_a_key_error(self):
        """With `all_metrics`, `metric` only labels the warning, so a benchmark that lacks it is
        still judged rather than aborting the whole check"""

        results = {"benchmarks": {
            "no_solver": {"variants": {
                "old": {"metrics": {"number_vccs": 0}},
                "new": {"metrics": {"number_vccs": 0}},
            }},
            "slow": {"variants": {
                "old": {"metrics": {"solver_runtime": 10.0}},
                "new": {"metrics": {"solver_runtime": 50.0}},
            }},
        }}
        checker = utils.AnyBenchmarkRegressedChecker(
            [["old", "new"]], "solver_runtime",
            "lambda old, new: new.get('solver_runtime', 0) > 2 * old.get('solver_runtime', 0)",
            all_metrics=True)
        with self.assertLogs(level="WARNING") as logs:
            self.assertTrue(checker(results))
        self.assertEqual(len(logs.output), 1)
        self.assertIn("'slow'", logs.output[0])
