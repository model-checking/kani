#!/usr/bin/python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
import subprocess

def run_cargo_command(command, working_directory):

    try:
        process = subprocess.Popen(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            shell=True,
            text=True,
            cwd=working_directory)
        stdout, stderr = process.communicate()

        if process.returncode != 0:
            print(f"Command '{command}' executed unsuccesfully, in '{working_directory}'.")
            print(stderr)
        else:
            print(f"Command '{command}' executed succesfully, in '{working_directory}'.")
            print(stdout)
    except Exception as e:
        print(f"Error: {str(e)}")


def main():

    target_directory = "kani"
    run_cargo_command("cargo install cargo-outdated", working_directory)
    run_cargo_command("cargo outdated --workspace", working_directory)


if __name__ == "__main__":
    main()
