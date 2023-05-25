# `benchcomp` configuration file

`benchcomp`'s operation is controlled through a YAML file---`benchcomp.yaml` by default or a file passed to the `-c/--config` option.
This page describes the file's schema and lists the different parsers and visualizations that are available.


## Built-in visualizations

The following visualizations are available; these can be added to the `visualize` list of `benchcomp.yaml`.

{{#include ../gen_src/visualization_list.txt}}

Detailed documentation for these visualizations follows.

{{#include ../gen_src/visualization_docs.txt}}
