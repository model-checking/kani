# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT


import dataclasses
import json
import subprocess
import textwrap

import jinja2
import yaml

import benchcomp
import benchcomp.visualizers.utils as viz_utils



@dataclasses.dataclass
class run_command:
    """Run an executable command, passing the performance metrics as JSON on stdin.

    This allows you to write your own visualization, which reads a result file
    on stdin and does something with it, e.g. writing out a graph or other
    output file.

    Sample configuration:

    ```
    visualize:
    - type: run_command
      command: ./my_visualization.py
    ```
    """

    command: str


    def __call__(self, results):
        results = json.dumps(results, indent=2)
        try:
            proc = subprocess.Popen(
                self.command, shell=True, text=True, stdin=subprocess.PIPE)
            _, _ = proc.communicate(input=results)
        except (OSError, subprocess.SubprocessError) as exe:
            logging.error(
                "visualization command '%s' failed: %s", self.command, str(exe))
            viz_utils.EXIT_CODE = 1
        if proc.returncode:
            logging.error(
                "visualization command '%s' exited with code %d",
                self.command, proc.returncode)
            viz_utils.EXIT_CODE = 1



@dataclasses.dataclass
class error_on_regression:
    """Terminate benchcomp with a return code of 1 if any benchmark regressed.

    This visualization checks whether any benchmark regressed from one variant
    to another. Sample configuration:

    ```
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
    ```

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

    ```
    visualize:
    - type: dump_yaml
      out_file: '-'
    ```
    """


    def __init__(self, out_file):
        self.get_out_file = benchcomp.Outfile(out_file)


    def __call__(self, results):
        with self.get_out_file() as handle:
            print(
                yaml.dump(results, default_flow_style=False), file=handle)



class dump_markdown_results_table:
    """Print Markdown-formatted tables displaying benchmark results

    For each metric, this visualization prints out a table of benchmarks,
    showing the value of the metric for each variant.

    The 'out_file' key is mandatory; specify '-' to print to stdout.

    'extra_colums' can be an empty dict. The sample configuration below assumes
    that each benchmark result has a 'success' and 'runtime' metric for both
    variants, 'variant_1' and 'variant_2'. It adds a 'ratio' column to the table
    for the 'runtime' metric, and a 'change' column to the table for the
    'success' metric. The 'text' lambda is called once for each benchmark. The
    'text' lambda accepts a single argument---a dict---that maps variant
    names to the value of that variant for a particular metric. The lambda
    returns a string that is rendered in the benchmark's row in the new column.
    This allows you to emit arbitrary text or markdown formatting in response to
    particular combinations of values for different variants, such as
    regressions or performance improvements.

    Sample configuration:

    ```
    visualize:
    - type: dump_markdown_results_table
      out_file: "-"
      extra_columns:
        runtime:
        - column_name: ratio
          text: >
            lambda b: str(b["variant_2"]/b["variant_1"])
            if b["variant_2"] < (1.5 * b["variant_1"])
            else "**" + str(b["variant_2"]/b["variant_1"]) + "**"
        success:
        - column_name: change
          text: >
            lambda b: "" if b["variant_2"] == b["variant_1"]
            else "newly passing" if b["variant_2"]
            else "regressed"
    ```

    Example output:

    ```
    ## runtime

    | Benchmark |  variant_1 | variant_2 | ratio |
    | --- | --- | --- | --- |
    | bench_1 | 5 | 10 | **2.0** |
    | bench_2 | 10 | 5 | 0.5 |

    ## success

    | Benchmark |  variant_1 | variant_2 | change |
    | --- | --- | --- | --- |
    | bench_1 | True | True |  |
    | bench_2 | True | False | regressed |
    | bench_3 | False | True | newly passing |
    ```
    """


    def __init__(self, out_file, extra_columns=None):
        self.get_out_file = benchcomp.Outfile(out_file)
        self.extra_columns = self._eval_column_text(extra_columns or {})


    @staticmethod
    def _eval_column_text(column_spec):
        for columns in column_spec.values():
            for column in columns:
                try:
                    column["text"] = eval(column["text"])
                except SyntaxError:
                    logging.error(
                        "This column text is not a valid python program: '%s'",
                        column["text"])
                    sys.exit(1)
        return column_spec


    @staticmethod
    def _get_template():
        return textwrap.dedent("""\
            {% for metric, benchmarks in d["metrics"].items() %}
            ## {{ metric }}

            | Benchmark | {% for variant in d["variants"][metric] %} {{ variant }} |{% endfor %}
            | --- |{% for variant in d["variants"][metric] %} --- |{% endfor -%}
            {% for bench_name, bench_variants in benchmarks.items () %}
            | {{ bench_name }} {% for variant in d["variants"][metric] -%}
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


    def _add_extra_columns(self, metrics):
        for metric, benches in metrics.items():
            try:
                columns = self.extra_columns[metric]
            except KeyError:
                continue
            for bench, variants in benches.items():
                tmp_variants = dict(variants)
                for column in columns:
                    variants[column["column_name"]] = column["text"](tmp_variants)


    @staticmethod
    def _get_variants(metrics):
        ret = {}
        for metric, benches in metrics.items():
            for bench, variants in benches.items():
                ret[metric] = list(variants.keys())
                break
        return ret


    def __call__(self, results):
        metrics = self._organize_results_into_metrics(results)
        self._add_extra_columns(metrics)

        data = {
            "metrics": metrics,
            "variants": self._get_variants(metrics),
        }

        env = jinja2.Environment(
            loader=jinja2.BaseLoader, autoescape=jinja2.select_autoescape(
                enabled_extensions=("html"),
                default_for_string=True))
        template = env.from_string(self._get_template())
        output = template.render(d=data)[:-1]
        with self.get_out_file() as handle:
            print(output, file=handle)
