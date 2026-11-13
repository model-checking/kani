#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

import json
import os
import signal
import subprocess
import sys
import time

OUTCOME_MODES = ("completed", "crashed", "timeout", "oom")


def multibyte_text(prefix: str, chars: int) -> str:
    body = ("日本語" * chars)[: chars - len(prefix)]
    return prefix + body


def warnings_for_mode(mode: str) -> list[str]:
    if mode == "cap_under":
        return [f"e2e-warncap: filler warning {i}" for i in range(20)]
    if mode == "quantifier":
        return [
            "warning: ignoring forall over unbounded range (synthetic, structured-warnings probe)",
            "warning: ignoring exists over unbounded range (synthetic, structured-warnings probe)",
            "e2e-warncap: an unrelated normal warning",
        ]
    if mode in OUTCOME_MODES:
        short_multibyte = "e2e-warncap: café — 日本語 unicode ✅ boundary text"
        exactly_cap = multibyte_text("e2e-warncap:CAP:", 4096)
        over_cap = multibyte_text("e2e-warncap:OVER:", 4097)
        fillers = [f"e2e-warncap: filler warning {i}" for i in range(19)]
        return [short_multibyte, exactly_cap, over_cap] + fillers
    raise SystemExit(f"fake cbmc: unknown FAKE_WARNING_MODE {mode!r}")


def warning_item_text(message: str) -> str:
    # kani-driver's parser is line-oriented: an item ends at the first line starting with "  }".
    return '  {\n    "messageText": %s,\n    "messageType": "WARNING"\n  },\n' % json.dumps(message)


def drop_native_warnings(lines: list[str]) -> list[str]:
    kept: list[str] = []
    item: list[str] = []
    for line in lines:
        item.append(line)
        if line.startswith("[") or line.startswith("]"):
            kept.extend(item)
            item = []
        elif line.startswith("  }"):
            document = json.loads("".join(item).rstrip().rstrip(","))
            if str(document.get("messageType", "")).upper() != "WARNING":
                kept.extend(item)
            item = []
    kept.extend(item)
    return kept


def main() -> int:
    if len(sys.argv) == 3 and sys.argv[1] == "--print-warnings":
        print(json.dumps(warnings_for_mode(sys.argv[2])))
        return 0

    real_cbmc = os.environ["REAL_CBMC"]
    if len(sys.argv) == 2 and sys.argv[1] == "--version":
        os.execv(real_cbmc, [real_cbmc, "--version"])

    mode = os.environ["FAKE_WARNING_MODE"]
    inserted = "".join(warning_item_text(w) for w in warnings_for_mode(mode))

    if mode in ("crashed", "timeout", "oom"):
        sys.stdout.write(
            '[\n  {\n    "program": "structured-warnings fake cbmc"\n  },\n')
        sys.stdout.write(inserted)
        sys.stdout.flush()
        if mode == "timeout":
            time.sleep(100)
            return 0
        time.sleep(1)
        if mode == "crashed":
            return 1
        os.kill(os.getpid(), signal.SIGKILL)

    proc = subprocess.run([real_cbmc] + sys.argv[1:],
                          capture_output=True, text=True)
    lines = drop_native_warnings(proc.stdout.splitlines(keepends=True))
    insert_at = next((i + 1 for i, line in enumerate(lines)
                     if line.startswith("  }")), None)
    if insert_at is None:
        raise SystemExit("fake cbmc: no item in the real CBMC output")
    sys.stdout.writelines(lines[:insert_at])
    sys.stdout.write(inserted)
    sys.stdout.writelines(lines[insert_at:])
    sys.stderr.write(proc.stderr)
    return proc.returncode


if __name__ == "__main__":
    sys.exit(main())
