# `benchcomp` configuration file

`benchcomp`'s operation is controlled through a YAML file---`benchcomp.yaml` by default or a file passed to the `-c/--config` option.
This page lists the different visualizations that are available.


## Variants

A *variant* is a single invocation of a benchmark suite. Benchcomp runs several
variants, so that their performance can be compared later. A variant consists of
a command-line argument, working directory, and environment. Benchcomp invokes
the command using the operating system environment, updated with the keys and
values in `env`. If any values in `env` contain strings of the form `${var}`,
Benchcomp expands them to the value of the environment variable `$var`.

```yaml
variants:
    variant_1:
        config:
            command_line: echo "Hello, world"
            directory: /tmp
            env:
              PATH: /my/local/directory:${PATH}
```


## Filters

After benchcomp has finished parsing the results, it writes the results to `results.yaml` by default.
Before visualizing the results (see below), benchcomp can *filter* the results by piping them into an external program.

To filter results before visualizing them, add `filters` to the configuration file.

```yaml
filters:
    - command_line: ./scripts/remove-redundant-results.py
    - command_line: cat
```

The value of `filters` is a list of dicts.
Currently the only legal key for each of the dicts is `command_line`.
Benchcomp invokes each `command_line` in order, passing the results as a JSON file on stdin, and interprets the stdout as a YAML-formatted modified set of results.
Filter scripts can emit either YAML (which might be more readable while developing the script), or JSON (which benchcomp will parse as a subset of YAML).


## Built-in visualizations

The following visualizations are available; these can be added to the `visualize` list of `benchcomp.yaml`.

{{#include ../gen_src/visualization_list.txt}}

Detailed documentation for these visualizations follows.

{{#include ../gen_src/visualization_docs.txt}}
