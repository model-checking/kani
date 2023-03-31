Each file in this directory implements a 'parser' that is intended to
parse the results of a single suite x variant run. Each suite has a
different output format and exposes different metrics; the parsers' job
is to read the suites' output files and returns a dict in a unified
format.

Each file in this directory implements a `main` method that takes the
root directory where the suite was run as an argument. The parser will
attempt to read the suite's results from that directory, and return the
results in suite.json format (which `benchcomp collate` will
subsequently merge with other suites into a single result.json file).
