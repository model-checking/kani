# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Entrypoint for `benchcomp visualize`


import sys

import yaml

import benchcomp.visualizers.utils

def main(args):
    with open(args.result_file, encoding="utf-8") as handle:
        results = yaml.safe_load(handle)

    generate_visualizations = benchcomp.visualizers.utils.Generator(args.config)
    generate_visualizations(results)
    sys.exit(benchcomp.visualizers.utils.EXIT_CODE)
