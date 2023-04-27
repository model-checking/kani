# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT


import dataclasses
import textwrap

import jinja2
import yaml

import benchcomp
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



class dump_yaml:
    """Print the YAML-formatted results to a file.

    The 'out_file' key is mandatory; specify '-' to print to stdout.

    Sample configuration:

    visualize:
    - type: dump_yaml
      out_file: '-'
    """


    def __init__(self, out_file):
        self.get_out_file = benchcomp.Outfile(out_file)


    def __call__(self, results):
        with self.get_out_file() as handle:
            print(
                yaml.dump(results, default_flow_style=False), file=handle)



class dump_markdown_results_table:
    """Print a Markdown-formatted table displaying benchmark results

    The 'out_file' key is mandatory; specify '-' to print to stdout.

    Sample configuration:

    visualize:
    - type: dump_markdown_results_table
      out_file: '-'
    """


    def __init__(self, out_file):
        self.get_out_file = benchcomp.Outfile(out_file)


    @staticmethod
    def _get_template():
        return textwrap.dedent("""\
            {% for metric, benchmarks in d["metrics"].items() %}
            ## {{ metric }}

            | Benchmark | {% for variant in d["variants"] %} {{ variant }} |{% endfor %}
            | --- | {% for variant in d["variants"] %}--- |{% endfor -%}
            {% for bench_name, bench_variants in benchmarks.items () %}
            | {{ bench_name }} {% for variant in d["variants"] -%}
             | {{ bench_variants[variant] }} {% endfor %}|
            {%- endfor %}
            {% endfor -%}
            """)


    @staticmethod
    def _get_variant_names(results):
        return results.values()[0]["variants"]


    @staticmethod
    def _organize_results_into_metrics(results):
        ret = {metric: {} for metric in results["metrics"]}
        for bench, bench_result in results["benchmarks"].items():
            for variant, variant_result in bench_result["variants"].items():
                for metric, value in variant_result["metrics"].items():
                    try:
                        ret[metric][bench][variant] = variant_result["metrics"][metric]
                    except KeyError:
                        ret[metric][bench] = {
                            variant: variant_result["metrics"][metric]
                    }
        return ret


    def __call__(self, results):
        data = {
            "metrics": self._organize_results_into_metrics(results),
            "variants": list(results["benchmarks"].values())[0]["variants"],
        }

        env = jinja2.Environment(
            loader=jinja2.BaseLoader, autoescape=jinja2.select_autoescape(
                enabled_extensions=("html"),
                default_for_string=True))
        template = env.from_string(self._get_template())
        output = template.render(d=data)[:-1]
        with self.get_out_file() as handle:
            print(output, file=handle)
