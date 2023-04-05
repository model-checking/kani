# Benchcomp

This directory contains `bin/benchcomp`, a tool for comparing one or
more suites of benchmarks using two or more 'variants' (command line
arguments and environment variables).

`benchcomp` runs all combinations of suite x variant, parsing the unique
output formats of each of these runs. `benchcomp` then combines the
parsed outputs and writes them into a single file. `benchcomp` can
post-process that combined file to create visualizations, exit if the
results are not as expected, or perform other actions.
