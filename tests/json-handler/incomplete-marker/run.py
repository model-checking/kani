#!/usr/bin/env python3
# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT

import datetime
import json
import os
import pathlib
import shutil
import signal
import subprocess
import sys
import tempfile
import time


POLL_TIMEOUT_S = 30


def load_json(path):
    with path.open(encoding="utf-8") as source:
        return json.load(source)


def remove(path):
    try:
        path.unlink()
    except FileNotFoundError:
        pass


def cleanup_group(pid):
    try:
        os.killpg(pid, signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass


def wait_for_marker(process, ready, output):
    deadline = time.monotonic() + POLL_TIMEOUT_S
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise AssertionError(f"Kani exited before CBMC blocked: {process.returncode}")
        try:
            record = load_json(ready)
            marker = load_json(output)
        except (FileNotFoundError, json.JSONDecodeError):
            time.sleep(0.02)
            continue
        if marker.get("run_state") == "INCOMPLETE":
            return record, marker
        time.sleep(0.02)
    raise AssertionError("timed out waiting for fake CBMC and the INCOMPLETE marker")


def start_kani(command, environment, root, groups):
    process = subprocess.Popen(
        command,
        cwd=root,
        env=environment,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )
    groups.append(process.pid)
    return process


def wait_for_next_utc_second():
    previous = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    deadline = time.monotonic() + 2
    while time.monotonic() < deadline:
        current = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
        if current != previous:
            return
        time.sleep(0.01)
    raise AssertionError("UTC second did not advance")


def main():
    kani = pathlib.Path(sys.argv[1])
    real_cbmc = pathlib.Path(sys.argv[2]).resolve()
    suite_dir = pathlib.Path(sys.argv[3]).resolve()
    with tempfile.TemporaryDirectory(prefix="kani-incomplete-marker-") as temporary:
        groups = []
        root = pathlib.Path(temporary)
        output = root / "export.json"
        ready = root / "ready.json"
        release = root / "release"
        fake_bin = root / "bin"
        fake_bin.mkdir()
        os.symlink(suite_dir / "fake_cbmc.py", fake_bin / "cbmc")
        test = root / "test.rs"
        shutil.copyfile(suite_dir / "test.rs", test)

        base_command = [
            str(kani),
            "-Z",
            "export-json",
            str(test),
            "--export-json",
            str(output),
            "--quiet",
        ]
        seeded = subprocess.run(base_command, cwd=root, capture_output=True, text=True)
        assert seeded.returncode == 0, seeded.stderr
        seeded_export = load_json(output)
        assert seeded_export["run_state"] == "COMPLETE"
        wait_for_next_utc_second()

        environment = os.environ.copy()
        environment.update(
            {
                "PATH": f"{fake_bin}{os.pathsep}{environment['PATH']}",
                "REAL_CBMC": str(real_cbmc),
                "FAKE_CBMC_READY": str(ready),
                "FAKE_CBMC_RELEASE": str(release),
                "FAKE_CBMC_TIMEOUT_S": "25",
            }
        )

        try:
            # A killed driver leaves the INCOMPLETE marker bytes in place.
            remove(ready)
            remove(release)
            process = start_kani(base_command, environment, root, groups)
            record, marker = wait_for_marker(process, ready, output)
            assert record["ppid"] == process.pid, (record, process.pid)
            marker_bytes = output.read_bytes()
            os.kill(process.pid, signal.SIGKILL)
            assert process.wait(timeout=5) == -signal.SIGKILL
            assert output.read_bytes() == marker_bytes
            assert marker["run_state"] == "INCOMPLETE"
            assert marker["started_at"] != seeded_export["started_at"]
            for key in ["outcome", "wall_time_s", "summary", "harnesses"]:
                assert key not in marker, key
            cleanup_group(process.pid)

            # Releasing CBMC replaces the marker with a terminal document from the same context.
            remove(ready)
            remove(release)
            process = start_kani(base_command, environment, root, groups)
            record, marker = wait_for_marker(process, ready, output)
            assert record["ppid"] == process.pid, (record, process.pid)
            wait_for_next_utc_second()
            release.touch()
            assert process.wait(timeout=POLL_TIMEOUT_S) == 0
            terminal = load_json(output)
            assert terminal["run_state"] == "COMPLETE"
            assert terminal["started_at"] == marker["started_at"], (
                terminal["started_at"],
                marker["started_at"],
            )
            for key, value in marker.items():
                if key != "run_state":
                    assert terminal[key] == value, key

            # A marker write error prevents CBMC from starting.
            remove(ready)
            remove(release)
            regular_file = root / "regular-file"
            regular_file.write_text("not a directory", encoding="utf-8")
            bad_parent = regular_file / "child" / "export.json"
            command = [*base_command]
            command[command.index(str(output))] = str(bad_parent)
            result = subprocess.run(
                command,
                cwd=root,
                env=environment,
                capture_output=True,
                text=True,
                timeout=POLL_TIMEOUT_S,
            )
            diagnostic = result.stdout + result.stderr
            assert result.returncode != 0, diagnostic
            assert "Failed to create --export-json output directory" in diagnostic, diagnostic
            assert not ready.exists()

            # Rejected selection leaves the seeded terminal file unchanged.
            remove(ready)
            remove(release)
            seeded_bytes = output.read_bytes()
            command = [*base_command, "--harness", "does-not-exist"]
            result = subprocess.run(
                command,
                cwd=root,
                env=environment,
                capture_output=True,
                text=True,
                timeout=POLL_TIMEOUT_S,
            )
            diagnostic = result.stdout + result.stderr
            expected = (
                "Failed to match the following harness(es):\n"
                "does-not-exist\n"
                "Please specify the fully-qualified name of a harness."
            )
            assert result.returncode != 0, diagnostic
            assert expected in diagnostic, diagnostic
            assert output.read_bytes() == seeded_bytes
            assert not ready.exists()
        finally:
            for pid in groups:
                cleanup_group(pid)


if __name__ == "__main__":
    main()
