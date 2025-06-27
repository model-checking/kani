# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT


import dataclasses
import logging
import typing

import benchcomp.visualizers


EXIT_CODE = 0


class SingleRegressionCheck:
    """Check whether a single benchmark has regressed on a single metric

    Instances of this class are constructed with the name of a metric to check,
    and a test function that figures out whether that metric has
    regressed. Instances of this class can then be called on pairs of
    benchmarks values. The instance returns true if the second benchmark
    regressed compared to the first.
    """

    metric: str
    test: typing.Callable


    def __init__(self, metric, test_program):
        self.metric = metric
        try:
            self.test = eval(test_program)
        except SyntaxError:
            logging.error(
                "This test program is not valid Python: '%s'", test_program)
            logging.error(
                "Regression test programs should be Python lambda functions that "
                "take two arguments (the value of a metric when run under two "
                "variants) and returns true if the second value regressed with "
                "respect to the first.")
            sys.exit(1)


    def __call__(self, old_value, new_value):
        return self.test(old_value, new_value)



class AnyBenchmarkRegressedChecker:
    """Check whether any benchmark has regressed on a particular metric

    Instances of this class are constructed with the name of a metric to check,
    and the name of a comparison function that figures out whether one variant
    of a benchmark has regressed compared to another variant.

    When called, instances of this class return True iff any of the benchmarks
    regressed.
    """

    def __init__(self, variant_pairs, metric, test, **test_args):
        self.variant_pairs = variant_pairs
        self.metric = metric
        self.test = test
        self.test_args = test_args


    def __call__(self, results):
        ret = False
        has_regressed = SingleRegressionCheck(
            self.metric, self.test, **self.test_args)

        for bench_name, bench in results["benchmarks"].items():
            for old_variant, new_variant in self.variant_pairs:
                for variant in (old_variant, new_variant):
                    if variant not in bench["variants"]:
                        logging.warning(
                            "benchmark '%s' did not have a value for metric '%s' "
                            "when run under variant '%s'",
                            bench_name, self.metric, variant)
                        continue

                old = bench["variants"][old_variant]["metrics"][self.metric]
                new = bench["variants"][new_variant]["metrics"][self.metric]

                if has_regressed(old, new):
                    logging.warning(
                        "Benchmark '%s' regressed on metric '%s' (%s -> %s)",
                        bench_name, self.metric, old, new)
                    ret = True
        return ret



@dataclasses.dataclass
class Generator:
    """Generate all visualizations in a config file given a dict of results"""

    config: benchcomp.ConfigFile
    except_for: list
    only: list


    def __call__(self, results):
        visualizations = self.config["visualize"]

        if self.except_for:
            for viz_name in self.except_for:
                vizs = [v for v in visualizations if v["type"] == viz_name]
                for viz in vizs:
                    visualizations.remove(viz)
        if self.only:
            for viz_name in [v["type"] for v in visualizations]:
                if viz_name not in self.only:
                    vizs = [v for v in visualizations if v["type"] == viz_name]
                    for viz in vizs:
                        visualizations.remove(viz)

        for viz in visualizations:
            viz_type = viz.pop("type")
            klass = getattr(benchcomp.visualizers, viz_type)
            visualize = klass(**viz)
            visualize(results)
