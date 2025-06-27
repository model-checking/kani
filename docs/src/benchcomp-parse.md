# Custom parsers

Benchcomp ships with built-in *parsers* that retrieve the results of a benchmark suite after the run has completed.
You can also create your own parser, either to run locally or to check into the Kani codebase.

## Built-in parsers

You specify which parser should run for each benchmark suite in `benchcomp.yaml`.
For example, if you're running the kani performance suite, you would use the built-in `kani_perf` parser to parse the results:

```yaml
suites:
    my_benchmark_suite:
      variants: [variant_1, variant_2]
      parser:
        module: kani_perf
```

## Custom parsers

A parser is a program that benchcomp runs inside the root directory of a benchmark suite, after the suite run has completed.
The parser should retrieve the results of the run (by parsing output files etc.) and print the results out as a YAML document.
You can use your executable parser by specifying the `command` key rather than the `module` key in your `benchconf.yaml` file:

```yaml
suites:
    my_benchmark_suite:
      variants: [variant_1, variant_2]
      parser:
        command: ./my-cool-parser.sh
```

The `kani_perf` parser mentioned above, in `tools/benchcomp/benchcomp/parsers/kani_perf.py`, is a good starting point for writing a custom parser, as it also works as a standalone executable.
Here is an example output from an executable parser:

```yaml
metrics:
    runtime: {}
    success: {}
    errors: {}
benchmarks:
    bench_1:
        metrics:
            runtime: 32
            success: true
            errors: []
    bench_2:
        metrics:
            runtime: 0
            success: false
            errors: ["compilation failed"]
```

The above format is different from the final `result.yaml` file that benchcomp writes, because the above file represents the output of running a single benchmark suite using a single variant.
Your parser will run once for each variant, and benchcomp combines the dictionaries into the final `result.yaml` file.


## Contributing custom parsers to Kani

To turn your executable parser into one that benchcomp can invoke as a module, ensure that it has a `main(working_directory)` method that returns a dict (the same dict that it would print out as a YAML file to stdout).
Save the file in `tools/benchcomp/benchcomp/parsers` using python module naming conventions (filename should be an identifier and end in `.py`).
