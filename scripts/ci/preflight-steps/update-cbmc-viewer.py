#!/usr/bin/python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
from job_runner import dependencies_links
from dependency_updater import VersionUpdater

# Set the name of the dependency you want to update
kani_dependencies_path = "/Users/jaisnan/kani/kani-dependencies"

if __name__ == "__main__":
    updater = VersionUpdater(kani_dependencies_path, dependencies_links["cbmc_viewer"])
    updater.run_process()
