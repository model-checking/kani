#!/usr/bin/python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
import re
import requests

class VersionUpdater:
    def __init__(self, file, dependency):
        self.file = file
        self.dependency = dependency
        self.dependencies = {}

    def get_latest_version(self, org_name, crate_name):
        url = f"https://github.com/{org_name}/{crate_name}/releases/latest"
        response = requests.get(url)
        if response.status_code == 404:
            raise ValueError(f"Failed to find latest version for crate '{crate_name}' on GitHub.")
        else:
            return re.search(r"v?(\d+\.\d+(\.\d+)?(-\S+)?)", response.url).group(1)

    def read_dependencies(self):

        with open(self.file, 'r') as f:
            contents = f.readlines()

        for line in contents:
            if "CBMC_VERSION" in line:
                version_number = line.split("=")[1].replace("\n", "")
                self.dependencies["CBMC_VERSION"] = version_number
            elif "CBMC_VIEWER_VERSION" in line:
                version_number = line.split("=")[1].replace("\n", "")
                self.dependencies["CBMC_VIEWER_VERSION"] = version_number
            elif "KISSAT_VERSION" in line:
                version_number = line.split("=")[1].replace("\n", "")
                self.dependencies["KISSAT_VERSION"] = version_number
            else:
                pass

    def update_dependencies(self):
        latest_version = self.get_latest_version(self.dependency["org_name"], self.dependency["dependency_name"])
        self.dependencies[self.dependency["dependency_string"]] = latest_version

    def write_dependencies(self):
        with open(self.file, 'r') as f:
            lines = f.readlines()
        with open(self.file, 'w') as f:
            for line in lines:
                match = re.search(r'(\w+)="\d+(\.\d+)+"', line.strip())
                if match and match.group(1).strip() == self.dependency["dependency_string"]:
                    f.write(
                        f'{self.dependency["dependency_string"]}="{self.dependencies[self.dependency["dependency_string"]]}"\n')
                else:
                    f.write(line)

    def run_process(self):
        self.read_dependencies()
        self.update_dependencies()
        self.write_dependencies()
