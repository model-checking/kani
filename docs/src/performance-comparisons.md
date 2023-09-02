# Performance comparisons with `benchcomp`

While Kani includes a [performance regression suite](https://github.com/model-checking/kani/tree/main/tests/perf), you may wish to test Kani's performance using your own benchmarks or with particular versions of Kani.
You can use the `benchcomp` tool in the Kani repository to run several 'variants' of a command on one or more benchmark suites; automatically parse the results of each of those suites; and take actions or emit visualizations based on those results.

## Example use-cases

1. Run one or more benchmark suites with the current and previous versions of Kani.
   Exit with a return code of 1 or print a custom summary to the terminal if any benchmark regressed by more than a user-configured amount.
1. Run benchmark suites using several historical versions of Kani and emit a graph of performance over time.
1. Run benchmark suites using different SAT solvers, command-line flags, or environment variables.

## Features

Benchcomp provides the following features to support your performance-comparison workflow:

* **Automatically copies benchmark suites into a fresh directories** before running with each variant, to ensure that built artifacts do not affect subsequent runtimes
* **Parses the results of different 'kinds' of benchmark suite** and combines those results into a single unified format.
  This allows you to run benchmarks from external repositories, suites of pre-compiled GOTO-binaries, and other kinds of benchmark all together and view their results in a single dashboard.
* **Driven by a single configuration file** that can be sent to colleagues or checked into a repository to be used in continuous integration.
* **Extensible,** allowing you to write your own parsers and visualizations.
* **Caches all previous runs** and allows you to re-create visualizations for the latest run without actually re-running the suites.

## Quick start

Here's how to run Kani's performance suite twice, comparing the last released version of Kani with the current HEAD.

```
cd $KANI_SRC_DIR
git worktree add new HEAD
git worktree add old $(git describe --tags --abbrev=0)

tools/benchcomp/bin/benchcomp --config tools/benchcomp/configs/perf-regression.yaml
```

This uses the [`perf-regression.yaml` configuration file](https://github.com/model-checking/kani/blob/main/tools/benchcomp/configs/perf-regression.yaml) that we use in continuous integration.
After running the suite twice, the configuration file terminates `benchcomp` with a return code of 1 if any of the benchmarks regressed on metrics such as `success` (a boolean), `solver_runtime`, and `number_vccs` (numerical).
Additionally, the config file directs benchcomp to print out a Markdown table that GitHub's CI summary page renders in to a table.

The rest of this documentation describes how to modify `benchcomp` for your own use cases, including writing a configuration file; writing a custom parser for your benchmark suite; and writing a custom visualization to examine the results of a performance comparison.
