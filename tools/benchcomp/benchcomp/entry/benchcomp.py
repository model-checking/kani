# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Entrypoint when running `benchcomp` with no arguments. This runs the other
# subcommands in sequence, for a single-command way of running, comparing, and
# post-processing the suites from a single reproducible config file.


import benchcomp.entry.collate
import benchcomp.entry.run


def main(args):
    run_result = benchcomp.entry.run.main(args)

    args.suites_dir = run_result.out_prefix / run_result.out_symlink
    results = benchcomp.entry.collate.main(args)

    benchcomp.entry.visualize.main(args)
