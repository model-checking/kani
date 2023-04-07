# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT


import dataclasses

import benchcomp.visualizers.utils as viz_utils


# TODO The doc comment should appear in the help output, which should list all
# available checks.

@dataclasses.dataclass
class error_on_regression:
    """Terminate benchcomp with a return code of 1 if any benchmark regressed.

    This visualization checks whether any benchmark regressed from one variant
    to another. Sample configuration:

    visualize:
    - type: error_on_regression
      variant_pairs:
      - [variant_1, variant_2]
      - [variant_1, variant_3]
      checks:
      - metric: runtime
        test: "lambda old, new: new / old > 1.1"
      - metric: passed
        test: "lambda old, new: False if not old else not new"

    This says to check whether any benchmark regressed when run under variant_2
    compared to variant_1. A benchmark is considered to have regressed if the
    value of the 'runtime' metric under variant_2 is 10% higher than the value
    under variant_1. Furthermore, the benchmark is also considered to have
    regressed if it was previously passing, but is now failing. These same
    checks are performed on all benchmarks run under variant_3 compared to
    variant_1. If any of those lambda functions returns True, then benchcomp
    will terminate with a return code of 1.
    """

    checks: list
    variant_pairs: list


    def __call__(self, results):
        for check in self.checks:
            any_benchmark_regressed = viz_utils.AnyBenchmarkRegressedChecker(
                    self.variant_pairs, **check)

        if any_benchmark_regressed(results):
            viz_utils.EXIT_CODE = 1
