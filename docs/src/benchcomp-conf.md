# `benchcomp` configuration file

`benchcomp`'s operation is controlled through a YAML file---`benchcomp.yaml` by default or a file passed to the `-c/--config` option.
This section documents the top-level schema and the different visualizations
that are available.

## Configuration file

The schema for `benchcomp.yaml` is shown below:

{{#include ../gen_src/BenchcompYaml.md}}

A minimal example of such a file is:

```yaml
run:
  suites:
    suite_1:
      parser:
        command: ./parse_suite1.py
      variants:
        - suite_1_old
        - suite_1_new
    suite_2:
      parser:
        module: kani_perf
      variants:
        - suite_2_env
        - suite_2_noenv
variants:
  suite_1_old:
    config:
      command: ./scripts/run_benchmarks.sh
      directory: suites/suite_1_old
      env: {}
  suite_1_new:
    config:
      command: ./scripts/run_benchmarks.sh
      directory: suites/suite_1_new
      env: {}
  suite_2_env:
    config:
      command: make benchmarks
      directory: suites/suite_2
      env:
        RUN_FAST: "true"
  suite_2_noenv:
    config:
      command: make benchmarks
      directory: suites/suite_2
      env: {}
visualize:
- type: run_command
  command: ./my_visualization.py
```

## Built-in visualizations

The following visualizations are available; these can be added to the `visualize` list of `benchcomp.yaml`.

{{#include ../gen_src/visualization_list.txt}}

Detailed documentation for these visualizations follows.

{{#include ../gen_src/visualization_docs.txt}}
