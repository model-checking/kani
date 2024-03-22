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


## Built-in visualizations

The following visualizations are available; these can be added to the `visualize` list of `benchcomp.yaml`.

{{#include ../gen_src/visualization_list.txt}}

Detailed documentation for these visualizations follows.

{{#include ../gen_src/visualization_docs.txt}}
