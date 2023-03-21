# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Entrypoint for `benchcomp collate`. This command turns a directory of
# `suite.yaml` files into a single `result.yaml` file. `suite.yaml` files are
# emitted by `benchcomp run` when it runs a single combination of suite x variant;
# the `collate` command is used to combine those files for all combinations.

import logging
import sys

import yaml


class _ResultsCollator:
    """Incrementally add suite x variant results, return combined results"""

    def __init__(self):
        self.result = {
            "metrics": {},
            "benchmarks": {},
        }

    def __call__(self):
        return self.result

    def _union_benchmarks(self, suite):
        for bench_name, suite_result in suite["benchmarks"].items():
            if bench_name not in self.result["benchmarks"]:
                self.result["benchmarks"][bench_name] = {"variants": {}}
            self.result["benchmarks"][bench_name]["variants"][suite["variant_id"]] = {
                **suite_result
            }

    def _union_metrics(self, suite):
        for metric, details in suite["metrics"].items():
            if metric not in self.result["metrics"]:
                self.result["metrics"][metric] = dict(details)
                continue
            if self.result["metrics"][metric] == details:
                continue
            logging.error(
                "two suite.yaml files inconsistently defined metric '%s'",
                metric)
            logging.error(
                "old definition: %s", str(self.result["metrics"][metric]))
            logging.error("new definition: %s", str(details))
            sys.exit(1)

    def add_suite(self, suite):
        self._union_metrics(suite)
        self._union_benchmarks(suite)


def main(args):
    results = _ResultsCollator()
    for suite_file in args.suites_dir.iterdir():
        with open(suite_file, encoding="utf-8") as handle:
            suite = yaml.safe_load(handle)
        results.add_suite(suite)

    with args.out_file() as handle:
        yaml.dump(results(), handle, default_flow_style=False)

    return results()
