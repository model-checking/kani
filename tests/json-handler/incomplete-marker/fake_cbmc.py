#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

import json
import os
import sys
import time


def main():
    real_cbmc = os.environ["REAL_CBMC"]
    if sys.argv[1:] == ["--version"]:
        os.execv(real_cbmc, [real_cbmc, "--version"])

    ready = os.environ["FAKE_CBMC_READY"]
    record = {"pid": os.getpid(), "ppid": os.getppid()}
    temporary = f"{ready}.{os.getpid()}.tmp"
    with open(temporary, "w", encoding="utf-8") as output:
        json.dump(record, output)
    os.replace(temporary, ready)

    deadline = time.monotonic() + float(os.environ.get("FAKE_CBMC_TIMEOUT_S", "25"))
    release = os.environ["FAKE_CBMC_RELEASE"]
    while time.monotonic() < deadline:
        if os.path.exists(release):
            os.execv(real_cbmc, [real_cbmc, *sys.argv[1:]])
        time.sleep(0.02)
    print("fake cbmc timed out waiting for release", file=sys.stderr)
    return 124


if __name__ == "__main__":
    sys.exit(main())
