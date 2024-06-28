`benchcomp` allows you to:

* Run two or more sets of benchmark suites under different configurations;
* declaratively define visualizations to be generated from the resulting data;
* run the tool with sensible defaults, or hook into every stage of the pipeline when you need more flexibility.

This documentation contains three sections.
The [user walkthrough](#user-walkthrough) takes you through the process of setting up an entirely new benchmark run and dashboard.
The [developer reference](#developer-reference) describes `benchcomp`'s architecture and the different data formats that it uses, enabling you to author a benchmark run and dashboard of your own.
The [visualization viewer guide](#benchcomp-visualization-viewer-guide) describes how to use dashboards that `benchcomp` generates.
Although dashboards are highly customizable, the guide describes the common elements.


# User Walkthrough

This section presents a single user story, showing how the user can quickly get started with a simple benchmark comparison run, through to a more complex setup with extensive customization.
This section explains the syntax in prose without attempting to exhaustively document it; the [Development Reference](#developer-reference) below is a full reference for the syntax.


## Comparing two variants

Alice wants to compare the performance of two versions of CBMC: the latest release, with and without a new optimization that she's implemented.
Initially, Alice tries to do this entirely using `benchcomp`'s built-in parsers and filters.
She performs the following steps:

* Create a directory containing all AWS CBMC proofs, with a top-level script that runs all of them called `run-all-cbmc-proofs.py`
* Create a configuration file (shown below)
* Run `benchcomp -o result.json`

The initial configuration file is:

<div class="twocolumn">
<div class="col col-66">

```yaml
# benchcomp.yaml

run:
  suites:
    all_cbmc_proofs:
      parser:
        type: built_in
        name: litani_to_benchcomp
        directory: all_cbmc_proofs
      variants:
        optimized:
          provenance: inline
          config:
            command_line: ./run-cbmc-proofs.py
            env:
              CBMC: ~/src/cbmc/build/bin/cbmc
        release:
          provenance: inline
          config:
            command_line: ./run-cbmc-proofs.py
            env:
              CBMC: /usr/local/bin/cbmc
```

</div>
<div class="col col-33">

This says: run two *variants* (`optimized` and `release`) of a single benchmark *suite* (`all_cbmc_proofs`).
The variants are distinguished in this case by the `CBMC` environment variable, which the AWS [proof build system](https://github.com/model-checking/cbmc-starter-kit) uses when invoking CBMC.

</div>
</div>

`benchcomp` copies the `all_cbmc_proofs` directory to two temporary directories, one for each variant, and runs the command line. It uses the built-in `litani_to_benchcomp` parser to assemble the results. `benchcomp` then writes this data to the output file in JSON format (here in YAML for readability):

<div class="twocolumn">
<div class="col col-75">

```yaml
# result.json

metrics:
  runtime:
    lower_is_better: true
    unit: s
  passed: {}
  coverage:
    unit: %
benchmarks:
  s2n_init:
    tags: [s2n, crypto, cbmc]
    variants:
        optimized:
          metrics:
            runtime: 3
            passed: true
            coverage: 100
          path: ./tmp/benchcomp/runs/abcd1234/optimized/all_cbmc_proofs/s2n/tests/cbmc/proofs/s2n_init
        release:
          metrics:
            runtime: 4
            passed: true
            coverage: 100
          path: ./tmp/benchcomp/runs/abcd1234/release/all_cbmc_proofs/s2n/tests/cbmc/proofs/s2n_init
  freertos_list_replace:
    tags: [cbmc, freertos, freertos_kernel]
    variants:
      optimized:
        metrics:
          # ...
```
</div>
<div class="col col-25">

The output file first declares the various *metrics* that the union of all the parsers emitted.
In later sections, we will see how Alice writes her own parser or extends an existing one with new metrics.
The file then maps benchmark names to values for each of the metrics for each of the variants that the benchmark ran under.

</div>
</div>

## Visualizing the results

Alice now wants to visualize the difference in runtime between the two versions of CBMC, ignoring other metrics for now.
Alice adds the `visualize` top-level key to her `benchcomp.yaml` file:

<div class="twocolumn">
<div class="col col-66">

```yaml
# add to benchcomp.yaml

visualize:
  - type: dashboard
    output: comparison.html
    graphs:
      - type: pairwise_box_whisker
        metric: runtime
        pairs:
          - [release, optimized]
```

</div>
<div class="col col-33">

This says to generate a single graph in a single HTML dashboard visualization.
The graph compares the runtimes of all benchmarks run under the `release` variant pairwise with the same benchmarks run under the `optimized` variant.

</div>
</div>

Alice runs the entire suite, together with generating and writing out the visualization, by running `benchcomp` again.
Alternatively, she can run `benchcomp visualize < result.json`, which loads the result of the run she did in the previous section.

The resulting dashboard looks like this:

<div class="subpage">
<div class="sidebar">
<div class="side-header">

Run ID: `abc123`

`2023-01-01T18:42:54`

[JSON version](/) of this dashboard

</div>
<div class="tags-bar">
<div class="tags-header">

**Filter dashboards by tags**

</div>
<div class="tags-container">

* [cbmc](/) <span class="n_proofs">(833)</span>
  * [s2n](/) <span class="n_proofs">(128)</span>
  * [freertos](/) <span class="n_proofs">(547)</span>
  * [e-sdk](/) <span class="n_proofs">(49)</span>
  * [uses-function-contracts](/) <span class="n_proofs">(49)</span>

</div>
</div>
</div>
<div class="central-container">
<div class="central-view">


~include tmp/box_whisker/cbmc.html

</div>
</div>
</div>

Benchmarks whose runtime did not change between the two versions have a 'runtime increase' of 1.0.
Benchmarks that took twice as long have a value of 2, while benchmarks that took half as long have a value of 0.5.
The graph shows that the optimization made the median benchmark speed up by a modest amount, with no benchmarks regressing and a few outliers with significant speedups.
Alice clicks on any of the 'tag' hyperlinks to view the graph for only tagged benchmarks.


## Filtering before visualizing

Alice realizes that the default parser for CBMC returns runtime at a granularity of 1 second.
This might skew the resulting visualisation because a proof that previously took 1.49 seconds and now takes 1.5, would be represented as having doubled its runtime (because the parser would report the runtimes as 1 and 2) despite having only increased by a small amount.
Alice decides to ignore all benchmarks whose runtime is less than 10 seconds, to ensure that the visualizations aren't noisy.
She also decides to ignore benchmarks that failed, as their runtime is not relevant.

Alice adds a new top-level `filter` key to `benchcomp.yaml`:

<div class="twocolumn">
<div class="col col-75">

```yaml
# add to benchcomp.yaml

filter:
  - type: predicate
    expression: > 
      lambda bench: all([
        variant["metrics"]["runtime"] > 9
        and variant["metrics"]["passed"]
        for variant in bench["variants"].values()
      ])
```

</div>
<div class="col col-25">


This filters results after running the benchmarks but before generating any visualisations.
The Python lambda filters out benchmarks for which the lambda expression returns `false`.
</div>
</div>

The benchmarks that are passed to the filter are each of the values of the `benchmarks` key in the `results.json` file above, whose full syntax is described in [`result.json` schema](#result.json-schema).

<div class="twocolumn">
<div class="col col-33">


Alice can alternatively write a script that reads `results.json` from stdin, and prints out a JSON file in the same format on stdout.

</div>
<div class="col col-66">

```yaml
filter:
  - type: executable
    path: ./scripts/drop_fast_benchmarks.py
```
</div>
</div>

With either of these examples, `benchcomp` will automatically invoke the filter whenever Alice runs `benchcomp` or `benchcomp` visualize.


## Adding extra metrics

Alice's optimization was targeted at CBMC's solver encoding.
Alice is therefore interested in seeing the number of verification conditions that CBMC generates per benchmark, but this metric is not emitted by the default CBMC parser.

Alice writes a filter which uses the `"path"` key from each benchmark to browse to the directory where the benchmark ran; read the log file to find the number of VCCs; and add this data to the list of metrics.

Here is an example of such a filter:

```python
#!/usr/bin/env python

import json
import pathlib
import re
import sys

results = json.read(sys.stdin)

results["metrics"]["vccs"] = {"lower_is_better": True}
pat = re.compile(
    r".+Generated \d+ VCC(s), "
    r"(?P<vccs>\d+) remaining after simplification")

for _, bench in results["benchmarks"].items():
    for _, variant in bench["variants"].items():
        proof_dir = pathlib.Path(variant["path"])
        log_file = proof_dir/"cbmc.xml"
        with open(log_file) as handle:
            for line in handle:
                if m := pat.match(line):
                    variant["metrics"]["vccs"] = int(m["vccs"])
                    break

print(json.dumps(results, indent=2))
```

<div class="twocolumn">
<div class="col col-25">

Alice can now add a second graph to her dashboard, showing a comparison of VCCs between the two CBMC versions.
Again, she can run `benchcomp visualize` to re-generate the dashboard without re-running the benchmarks again.


</div>
<div class="col col-75">

```yaml
visualize:
  - type: dashboard
    output: comparison.html
    graphs:
      - type: pairwise_box_whisker
        metric: runtime
        pairs:
          - [release, optimized]

      # add to benchcomp.yaml

      - type: pairwise_box_whisker
        metric: vccs
        pairs:
          - [release, optimized]
```

</div>
</div>


## Adding another benchmark suite

Alice wants to run a completely new set of benchmarks together with the CBMC ones to ensure that her new optimization works for other model checkers.
She decides to use a codebase that contains Kani proofs that run using the standalone `kani` command.
However, `benchcomp` does not yet have a parser for Kani, so Alice has a bit more work to do.

First of all, Alice creates `./firecracker/run-kani-proofs.sh`:

```sh
#!/bin/sh

out_dir="kani_output/$(uuidgen)";
mkdir -p "${out_dir}";
grep -re '#[kani::proof]' \
    | sort -u \
    | while read -r proof_file; do
        out_file="${out_dir}/${proof_file}.out";
        time_file="${out_dir}${proof_file}.time";
        PATH="${CBMC_DIR}:${PATH}" \
            time -e kani ${proof_file} > "${out_file}" 2> "${time_file}";
    done;
ln -sf "${out_dir}" "kani_output/latest";
```

Next, Alice creates a parser that reads the output files from the latest run and prints them out in the format that `benchcomp` expects from a single benchmark suite run, which is documented in [suite.json schema](#suite.json-schema) below.

```sh
#!/bin/sh
# kani-parser.sh

find kani_output/latest -name '*.out' \
    | while read -r out_file; do
        bench_name="$(echo ${out_file} | sed -e 's/\.out//')";
        line='{"bench_name": "${bench_name}", "passed": '
        grep -e "VERIFICATION:- SUCCESSFUL" < "${out_file}"
        if [ $? -eq 0 ]; then
            line="${line}true, "; else line="${line}false, ";
        fi;
        line="${line} \"tags\": [\"kani\", \"firecracker\"], "
        time_file="$(echo ${out_file} | sed -e 's/\.out/\.time/')";
        line="${line}\"runtime\": $(cat time_file)\}";
        echo "${line}";
    done \
    | jq '{
        "metrics": {
            "passed": {},
            "runtime": {
                "unit": "s",
                "lower_is_better": true,
            },
        },
        "benchmarks": {
            (.bench_name): .|del(.bench_name)
        }
    }'
```

Finally, Alice adds a new suite under the `suites` key to `benchcomp.yaml` and re-runs the `benchcomp` tool.

<div class="twocolumn">
<div class="col col-66">

```yaml
# Add to benchcomp.yaml

  suites:
    kani_firecracker:
      parser:
        type: executable
        path: ./kani-parser.sh
        directory: .
      variants:
        optimized:
          provenance: inline
          config:
            timeout: 7200
            memout: 48G
            command_line: ./run-kani-proofs.sh
            env:
              CBMC_DIR: ~/src/cbmc/build/bin
        release:
          provenance: inline
          config:
            timeout: 7200
            memout: 48G
            command_line: ./kani-proofs.sh
            env:
              CBMC_DIR: /usr/local/bin
```

</div>
<div class="col col-33">

The `kani-parser.sh` script prepends the `$CBMC_DIR` environment variable to the `$PATH` before running Kani, so setting that environment variable to a different value for each variant will make Kani invoke a different version of CBMC.

These variants also include timeouts and memouts.
The `benchcomp` run as a whole can also include a global timeout and memout in the top-level `run` key.

</div>
</div>

## Adding a custom visualization

Alice now wants to add a table to the dashboard, containing the name of each benchmark in both the Kani and CBMC benchmark suites together with their `optimized` and `release` runtimes.

`benchcomp` doesn't have a built-in way to do this, so Alice writes a script that reads a [`result.json` file](#result.json-schema) on stdin, and prints a HTML table (or really any arbitrary HTML or SVG) to stdout.
Alice decides to create a new output file to avoid cluttering up the graphs, so she adds a new visualization to the `visualizations` top-level key in `benchcomp.yaml`:

<div class="twocolumn">
<div class="col col-50">

```yaml
visualizations:
  - type: executable
    path: ./results-to-table.sh
```

</div>
<div class="col col-50">

`benchcomp visualize` automatically invokes the `results-to-table.sh` script once for every set of results filtered by tag, so that it can generate all the different per-tag pages on the dashboard.

</div>
</div>

Because the `kani-parser.sh` script gave the `kani` and `firecracker` tags to each benchmark, these benchmarks appear in the dashboard along with the CBMC ones.
Alice can click on the hyperlinks to view the results for just the Kani benchmarks.

<div class="subpage">
<div class="sidebar">
<div class="side-header">

Run ID: `abc123`

`2023-01-01T18:42:54`

[JSON version](/) of this dashboard

</div>
<div class="tags-bar">
<div class="tags-header">

**Filter dashboards by tags**

</div>
<div class="tags-container">

* [cbmc](/) <span class="n_proofs">(833)</span>
  * [s2n](/) <span class="n_proofs">(128)</span>
  * [freertos](/) <span class="n_proofs">(547)</span>
  * [e-sdk](/) <span class="n_proofs">(49)</span>
  * [uses-function-contracts](/) <span class="n_proofs">(49)</span>
* [kani](/) <span class="n_proofs">(27)</span>
  * [firecracker](/) <span class="n_proofs">(27)</span>

</div>
</div>
</div>
<div class="central-container">
<div class="central-view">
<table>
<tr>
    <th>Benchmark name</th>
    <th>Release runtime</th>
    <th>Optimized runtime</th>
</tr>
<tr>
    <td>s2n_init</td>
    <td>4</td>
    <td>3</td>
</tr>
<tr>
    <td>firecracker_xen_teardown</td>
    <td>19</td>
    <td>12</td>
</tr>
<tr>
    <td>...</td>
</tr>
</table>

</div>
</div>
</div>

# Development Reference

This development reference describes `benchcomp`'s architecture and gives complete schemas for each of its data formats.


## Architecture and overview

This section gives a high-level sequence diagram of the entire tool pipeline; describes the components of a single run; and then describes how benchmarks are run in more detail.

At the highest level, users invoke `benchcomp`, a unified front-end that runs several other sub-tools in the background.
`benchcomp` first executes `benchcomp run`, which runs one or more _benchmark suites_ several times (each time using a different _variant_).
`bc run` eventually returns a JSON document in the [`result.json`](#result.json-schema) format, containing the union of results from all benchmark runs under every variant.
`benchcomp` then sends the results to `benchcomp visualize`, which writes out whichever visualizations the user configured.
Users can optionally _filter_ the results before visualizing using their own filter, which must act as a composable pipe, reading and writing the same [`result.json`](#result.json-schema) format.

<div class="ascii-art">

```
Zoomed-out view: `benchcomp` front-end invoking other tools
-----------------------------------------------------------

┌───────────┐  ┌────────┐┌────────┐┌──────────────┐ ┌────────────┐┌──────────────┐
│ benchcomp │  │ bc run ││ filter ││ bc visualize │ │ custom viz ││ built-in viz │
└───────────┘  └────────┘└────────┘└──────────────┘ └────────────┘└──────────────┘
      │ invoke       │       │              │             │               │
      o------------->│       │              │             │               │
      │  result.json │       │              │             │               │
      │<-------------o       │              │             │               │
      │              │       │              │             │               │
      │ (optional) result.json              │             │               │
      o--------------┼------>│              │             │               │
      │          result.json │              │             │               │
      │<-------------┼-------o              │             │               │
      │              │       │              │             │               │
      │ result.json  │       │              │             │   out.html    │
      o--------------┼-------┼------------->│             │     ^         │
      │              │       │              │ result.json │    /          │  out.html
      │              │       │              o------------>│---'           │     ^
      │              │       │              │ result.json │               │    /
      │              │       │              o-------------┼-------------->│---'
      │              │       │              │             │               │
```

</div>

To help understand how `bc run` works in more detail, the diagram below describes the different components of a run.
A _benchmark suite_ is a set of benchmarks that can be run all together with a single command and whose results can be interpreted by a single _parser_ script.
The parser is responsible for reading whatever metrics the user is interested in gathering, and emitting them in a standard [`suite.json`](#suite.json-schema) format (different but related to [`result.json`](#result.json-schema) mentioned above).
Each benchmark suite may be run under two or more _variants_; `benchcomp`'s main utility is in comparing the metrics from such runs.
Variants define how to invoke the benchmark suite, including how to customise the invocation through command-line flags and environment variables.

Users must define one or more benchmark suites, and two or more variants.

<div class="ascii-art">

```
Components of a single benchmark "run", including variant details
-----------------------------------------------------------------

┌───────────────┐  ┌───────────────┐  ┌───────────────┐  ┌────────────────┐
│ ┌───────────┐ │  │ ┌───────────┐ │  │ ┌───────────┐ │  │ variant1a.yaml │
│ │  suite 1  │ │  │ │  suite 2  │ │  │ │  suite 3  │ │  │                │
│ │-----------│ │  │ │---------- │ │  │ │---------- │ │  │command: ./run-a│
│ │ b b b b b │ │  │ │  b b b b  │ │  │ │ b b b b b │ │  │directory: ./all│
│ └───────────┘ │  │ └───────────┘ │  │ └───────────┘ │  │env:            │
│ ┌───────────┐ │  │ ┌───────────┐ │  │ ┌───────────┐ │  │  USE_KISSAT: "1│
│ │ parser 1  │ │  │ │ parser 2  │ │  │ │ parser 3  │ │  │patches:        │
│ └───────────┘ │  │ └───────────┘ │  │ └───────────┘ │  │  - ./patches/li│
│               │  │               │  │               │  │  - ./patches/xe│
│ ┌───────────┐ │  │ ┌───────────┐ │  │ ┌───────────┐ │  │timeout: 7200   │
│ │ variant1a │ │  │ │ variant2a │ │  │ │ variant3a │ │  │memout: 48G     │
│ └───────────┘ │  │ └───────────┘ │  │ └───────────┘ │  │                │
│ ┌───────────┐ │  │ ┌───────────┐ │  │ ┌───────────┐ │  │                │
│ │ variant1b │ │  │ │ variant2b │ │  │ │ variant3b │ │  │                │
│ └───────────┘ │  │ └───────────┘ │  │ └───────────┘ │  │                │
│ ┌───────────┐ │  │ ┌───────────┐ │  │ ┌───────────┐ │  │                │
│ │ variant1c │ │  │ │ variant2c │ │  │ │ variant3c │ │  │                │
│ └───────────┘ │  │ └───────────┘ │  │ └───────────┘ │  │                │
└───────────────┘  └───────────────┘  └───────────────┘  └────────────────┘
```

</div>

The diagram below zooms into `benchcomp run`'s sequence diagram.
Here, `bc run` is running a single benchmark suites ("suite 1") sequentially using two different variants ("a" and "b").
Once the suite-running command terminates, `bc run` runs the suite's parser, which uses the suite's log files and other artefacts to return the benchmark metrics for that variants in [`suite.json`](#suite.json-schema) format.
`bc run` saves each suite file until it has run all benchmark suites using all variants.
`bc run` then invokes `bc collate`, which takes a set of `suite.json` files and prints out a [`result.json`](#result.json-schema) file, ready for further filtering and visualization.

<div class="ascii-art">

```
Zoomed-in view: `benchcomp run` and `benchcomp collate`
------------------------------------------------------------------

┌───────────────┐ ┌──────────┐ ┌───────────────┐ ┌───────────────┐ ┌────────────┐
│ benchcomp run │ │ parser 1 │ │ suite 1 copy 1│ │ suite 1 copy 2│ │ bc collate │
└───────────────┘ └──────────┘ └───────────────┘ └───────────────┘ └────────────┘
      │                │           │                   │                  │
      │ apply patches to fresh copy│                   │                  │
      o----------------┼---------->│                   │                  │
      │                │           │                   │                  │
      │ run suite with variant 1a  │                   │                  │
      o----------------┼---------->│                   │                  │
      │       wait for termination │                   │                  │
      │<---------------┼-----------o                   │                  │
      │                │           │                   │                  │
      │   run parser   │           │                   │                  │
      o--------------->│           │                   │                  │
      │             read native result from suite      │                  │
      │                o---------->│                   │                  │
      │              return result in suite.json format│                  │
      │<---------------o           │                   │                  │
      │                │           │                   │                  │
      │                │           │                   │                  │
      │ apply patches to fresh copy│                   │                  │
      o----------------┼-----------┼------------------>│                  │
      │                │           │                   │                  │
      │ run suite with variant 1b  │                   │                  │
      o----------------┼-----------┼------------------>│                  │
      │                │          wait for termination │                  │
      │<---------------┼-----------┼-------------------o                  │
      │                │           │                   │                  │
      │   run parser   │           │                   │                  │
      o--------------->│           │                   │                  │
      │             read native result from suite      │                  │
      │                o-----------┼------------------>│                  │
      │              return result in suite.json format│                  │
      │<---------------o           │                   │                  │
      │                │           │                   │                  │
      :                :           :                   :                  :
      :                :           :                   :                  :
      :      [ All combinations of suites x variants are run ]            :
      :      [         :  serially or in parallel      :     ]            :
      :                :           :                   :                  :
      :                :           :                   :                  :
      │                │           │                   │                  │
      │ all suite.json result files│                   │                  │
      o----------------┼-----------┼-------------------┼----------------->│
      │                │           │            single result.json file   │
      │<---------------┼-----------┼-------------------┼------------------o
      │                │           │                   │                  │
```

</div>



## Command line

```
benchcomp run
    Run benchmark suites using different variants; print result.json

benchcomp collate  [non user-facing command]
    Print out a result.json file given a directory of suite.json files

benchcomp visualize
    Emit visualizations derived from a result.json file

benchcomp
    Invoke each of the above tools in turn using a configuration file, described below.
```

## Configuration file

A `benchcomp` configuration file is used to orchestrate an entire benchmark run, filtering, and visualization by running a single command.
Each top-level key in the `benchcomp` file corresponds to one of `benchcomp`'s sub-commands.

Running `benchcomp` with the file below as input makes the entire process run from end-to-end.
Alternatively, users can invoke subcommands individually, saving the intermediate files.

```yaml
run:
  suites:
    suite_1:
      # This parser parses the results at the end of each run
      # of each of the 3 variants below
      parser:
        type: executable
        command: ./scripts/parse_results.py
        directory: ./suite_1

      # Each variant defines a command line, environment variables, and
      # other aspects to be changed on each run of this benchmark suite.
      variants:
        a:
          provenance: inline
          config:
            command_line: ./scripts/run_benchmark.py
            directory: ./suite_1
            timeout: 7200
            memout: 48G
            patches:
              - ./patches/foo.diff
              - ./patches/bar.diff
            env:
              USE_KISSAT: "1"
        b:
          provenance: file
          path: ./suite_1/variants/b.yaml
        c:
          provenance: file
          path: ./suite_1/variants/c.yaml

    suite_2:
      parser:
        type: built-in
        name: litani_to_benchcomp
        directory: ./suite_2
      variants:
        a:
          provenance: file
          path: ./suite_2/variants/a.yaml
        b:
          provenance: file
          path: ./suite_2/variants/b.yaml
        c:
          provenance: file
          path: ./suite_2/variants/c.yaml

# When all benchmark suites have been run under all variants, the
# results are collated and passed through each of these filters.
# The filters each read the results on stdin, in results.json format,
# and write to stdout in the same format.
filter:
  - command: ./filters/drop-fast-proofs.py
  - predicate: "lambda bench: bench['metrics']['runtime'] > 10"

# After filtering, the results are passed to one or more visualizations
visualize:
  - type: dashboard
    output: "comparison.html"
    graphs:
      - type: pairwise-box-whisker
        output: "comparison.html"
        metric: runtime
        pairs:
          - [a, b]
          - [b, c]
          - [a, b]

  - type: script
    command: ./scripts/visualize-data.py --result-file %r
    # A single visualization can have further filters applied
    filters:
      - ./filters/viz-specific-filter.py
```


## Suite Parsers

A _parser_ is responsible for reading benchmark results after a benchmark suite has finished executing.
Parsers must emit the benchmark suite results in a particular format (`suite.json`), documented below.

Parser writers are responsible for deciding what information they wish to include in a `suite.json`.
Each benchmark can be associated with arbitrary different metrics and tags, which can then be used by the subsequent 'filter' and 'visualize' steps.

### `suite.json` schema

Parsers are responsible for emitting the results of a single benchmark suite run in the following format.

#### Brief schema

```yaml
"metrics":
  str:
    [optional] "lower_is_better": bool
    [optional] "unit": str
    [optional] "derivative": str
[optional] "tags":
  - str
"benchmarks":
  str:
    "metrics":
      str: any-type
    [optional] "tags":
      - str
    [optional] "aux": dict
[optional] "aux": dict
```

#### Detailed schema

```yaml
# The metrics that each benchmark will have results for
"metrics":
  str:                              # metric identifier

    [optional] "lower_is_better": bool  # for comparison graphs

    [optional] "unit": str          # for axes on graphs

    [optional] "derivative": str    # human-readable term for
                                    # "difference between two metrics"
"benchmarks":
  str:                              # benchmark identifier

    "metrics":
      str: any-type                 # map from metric name to result
                                    # for this benchmark

    [optional] "tags":              # arbitrary tags for this benchmark
      - str

    [optional] "aux": dict          # arbitrary user-provided data

[optional] "aux": dict              # arbitrary user-provided data
```


#### Example `suite.json`

```yaml
metrics:
  runtime:
    lower_is_better: true
    unit: s
    differential: speedup
  passed: {}
  symex_time:
    lower_is_better: true
    unit: s
    differential: speedup
  coverage:
    unit: %
benchmarks:
  s2n_init:
    tags: [cbmc, crypto, s2n]
    metrics:
      runtime: 3
      passed: true
      symex_time: 1
      coverage: 100
  freertos_list_replace:
    tags: [cbmc, freertos, freertos_kernel]
    metrics:
      runtime: 97
      passed: true
      symex_time: 44
```

### Built-in result parsers

`benchcomp` includes a number of convenient built-in parsers that parse commonly-used benchmark formats.
Here is a list:

* `litani_to_benchcomp`: parses a [Litani `run.json`](https://awslabs.github.io/aws-build-accumulator/#litani-man_litani-run.json) file for CBMC benchmarks using the [CBMC starter kit](https://github.com/model-checking/cbmc-starter-kit/).

### Writing your own result parser

Result parsers must:

* Be executable
* Print to stdout whichever metrics the user is interested in in `suite.json` format


## Result filters

`benchcomp` collates the results of all benchmark runs using all variants into a format called `result.json`.
Users can transform the results in this data format before it is used to generate visualizations.
This section documents the schema and filter feature.

### `result.json` schema

This data represents the union of all benchmark suite executions that `benchcomp` has run.

#### Brief schema

```yaml
"metrics": <same as in suite.json>
"benchmarks":
  str:
    "variants":
      "tags": [str]
      str:
        "metrics":
          str: any-type
```


### Executable filters

### Python predicate filters


## Visualizations

### Built-in visualizations

### Writing your own visualizations


# Visualization Viewer Guide

## Overview: two example dashboards


.

## Dashboard features

.

### Tags

### Individual benchmark result pages

### Machine-readable data

### Arbitrary HTML
