# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# Entrypoint for `benchcomp run`


def get_default_out_symlink():
    return "latest"


def get_default_out_dir():
    return str(uuid.uuid4())


def get_default_out_prefix():
    return pathlib.Path("tmp")/"benchcomp"/"suites"


def main(_):
    pass
