#!/usr/bin/python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
import toml
import sys

def update_version(file_path, new_version):

    try:
        # load the toml file
        with open(file_path, 'r') as f:
            data = toml.load(f)

        # update the version number
        data['package']['version'] = new_version

        with open(file_path, 'w') as f:
            toml.dump(data, f)

        print(f"Version updated succesfully to '{version_number}' in '{file_path}'.")
    except Exception as e:
        print(f"Error updating kani version to '{version_number}' in '{file_path}'.")


def main():
    if len(sys.argv) != 2:
        print("Usage: python update_versions.py <new_version>")
        sys.exit(1)

    # The new version number specified by the user
    new_version = sys.argv[1]

    # The list of Cargo.toml files to update
    cargo_toml_files = [
        "Cargo.toml",
        "cprover_bindings/Cargo.toml",
        "kani-compiler/Cargo.toml",
        "kani-compiler/kani_queries/Cargo.toml",
        "kani-driver/Cargo.toml",
        "kani_metadata/Cargo.toml",
        "library/kani/Cargo.toml",
        "library/kani_macros/Cargo.toml",
        "library/std/Cargo.toml",
        "tools/build-kani/Cargo.toml",
    ]
