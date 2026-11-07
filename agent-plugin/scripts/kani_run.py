# Copyright Kani Contributors
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Run Kani (or any command) under a memory guard, and record what the run used.

CBMC can use all the memory of a machine, and a memory kill that takes down the agent's own process
loses much more than the run. `run_guarded` puts the run where only it can be killed:

- `scope`: on Linux with systemd, the command runs in its own transient scope
  (`systemd-run --user --scope -p MemoryMax=... -p MemorySwapMax=0`). The kernel kills only that
  scope's processes when the cap is exceeded, and its `memory.peak` and `cpu.stat` give the run's
  real peak memory and CPU time.
- `watchdog`: elsewhere (macOS, or Linux without a user systemd), the process group is polled every
  two seconds and killed when its resident memory exceeds the cap. The peak is the largest sample,
  so it can miss a spike. (`RLIMIT_AS` is not used: Rust and CBMC reserve far more address space
  than they use, so it kills healthy runs.)

The cap is, in this order: the `memory_mb` argument (the `--memory-limit-mb` flag of the scripts),
`KANI_PLUGIN_MEMORY_MB` (`0` or `none` for no limit), or half of the memory available to this process
(the smaller of the machine's memory and the limit of its cgroup). `KANI_PLUGIN_MEMORY_GUARD` chooses
the method: `scope`, `watchdog`, or `off`.

A run that is stopped (memory or time) writes no export of its own. `write_stopped_export` writes one
that says so: `run_state` `KILLED` or `TIMEOUT`, the harness that was running, the time, and the peak.
"""

import json
import os
import re
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

MIN_DEFAULT_MB = 2048
POLL_S = 0.5
WATCHDOG_EVERY_S = 2.0

# Runs in the scope after the command: the scope is gone when systemd-run returns, so the cgroup's
# counters are read from inside it.
SCOPE_WRAPPER = r'''
"$@"; rc=$?
cg=/sys/fs/cgroup$(cut -d: -f3 /proc/self/cgroup)
{
  echo "peak_bytes=$(cat "$cg/memory.peak" 2>/dev/null)"
  echo "cpu_usec=$(awk '/^usage_usec/{print $2}' "$cg/cpu.stat" 2>/dev/null)"
  echo "oom_kill=$(awk '/^oom_kill /{print $2}' "$cg/memory.events" 2>/dev/null)"
} > "$KANI_PLUGIN_RES" 2>/dev/null
exit $rc
'''


def _physical_memory_mb():
    try:
        with open("/proc/meminfo") as f:
            for line in f:
                if line.startswith("MemTotal:"):
                    return int(line.split()[1]) // 1024
    except OSError:
        pass
    try:
        out = subprocess.run(["sysctl", "-n", "hw.memsize"], capture_output=True, text=True)
        return int(out.stdout.strip()) // (1024 * 1024)
    except (OSError, ValueError):
        return None


def _cgroup_limit_mb():
    """The smallest finite `memory.max` on the cgroup path of this process (cgroup v2), or None."""
    try:
        with open("/proc/self/cgroup") as f:
            relative = next(line.strip().split(":", 2)[2] for line in f if line.startswith("0::"))
    except (OSError, StopIteration):
        return None
    best = None
    path = Path("/sys/fs/cgroup") / relative.lstrip("/")
    for directory in [path, *path.parents]:
        if not str(directory).startswith("/sys/fs/cgroup"):
            break
        try:
            value = (directory / "memory.max").read_text().strip()
        except OSError:
            continue
        if value.isdigit():
            mb = int(value) // (1024 * 1024)
            best = mb if best is None else min(best, mb)
    return best


def available_memory_mb():
    """The memory this process may use: the machine's, or its cgroup's limit if that is smaller."""
    candidates = [m for m in (_physical_memory_mb(), _cgroup_limit_mb()) if m]
    return min(candidates) if candidates else None


def default_limit_mb():
    available = available_memory_mb()
    return max(MIN_DEFAULT_MB, available // 2) if available else None


def resolve_limit_mb(explicit=None):
    """The memory cap in MB, or None for no limit."""
    if explicit is None:
        explicit = os.environ.get("KANI_PLUGIN_MEMORY_MB")
    if explicit is None or explicit == "":
        return default_limit_mb()
    if str(explicit).strip().lower() in ("0", "none", "off", "unlimited"):
        return None
    try:
        value = int(float(explicit))
    except ValueError:
        raise ValueError(f"memory limit `{explicit}` is not a number of MB") from None
    return value if value > 0 else None


_SCOPE_OK = {}


def scope_available():
    if "ok" not in _SCOPE_OK:
        ok = False
        if sys.platform.startswith("linux") and shutil.which("systemd-run") and shutil.which("bash"):
            try:
                ok = subprocess.run(["systemd-run", "--user", "--scope", "--quiet", "-p", "MemoryMax=1G", "true"],
                                    capture_output=True, timeout=20).returncode == 0
            except (OSError, subprocess.TimeoutExpired):
                ok = False
        _SCOPE_OK["ok"] = ok
    return _SCOPE_OK["ok"]


def guard_method(limit_mb):
    forced = os.environ.get("KANI_PLUGIN_MEMORY_GUARD", "").lower()
    if forced == "off" or limit_mb is None:
        return "none"
    if forced == "watchdog":
        return "watchdog"
    if forced == "scope":
        return "scope" if scope_available() else "watchdog"
    return "scope" if scope_available() else "watchdog"


def group_rss_mb(pgid):
    """Resident memory (MB) of every process in a process group."""
    try:
        out = subprocess.run(["ps", "-axo", "pgid=,rss="], capture_output=True, text=True)
    except OSError:
        return 0
    total = 0
    for line in out.stdout.splitlines():
        parts = line.split()
        if len(parts) == 2 and parts[0] == str(pgid) and parts[1].isdigit():
            total += int(parts[1])
    return total / 1024


def _kill_group(proc):
    try:
        os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
    except (ProcessLookupError, PermissionError):
        pass
    try:
        proc.kill()
    except OSError:
        pass


class RunResult:
    """What a guarded run did. `killed` is `memory`, `timeout`, or None."""

    def __init__(self, cmd, returncode, killed, elapsed_s, peak_memory_mb, cpu_s, method, limit_mb, text, started):
        self.cmd, self.returncode, self.killed = list(cmd), returncode, killed
        self.elapsed_s, self.peak_memory_mb, self.cpu_s = elapsed_s, peak_memory_mb, cpu_s
        self.method, self.limit_mb, self.text, self.started = method, limit_mb, text, started

    def record(self):
        """The resource record stored in exports and summaries (`plugin_run`)."""
        out = {"elapsed_s": round(self.elapsed_s, 1), "guard": self.method, "limit_mb": self.limit_mb,
               "peak_memory_mb": None if self.peak_memory_mb is None else round(self.peak_memory_mb),
               "cpu_s": None if self.cpu_s is None else round(self.cpu_s, 1), "exit_code": self.returncode}
        if self.killed:
            out["killed"] = self.killed
            out["killed_message"] = self.killed_message()
        peak_is_sample = self.method == "watchdog"
        if peak_is_sample:
            out["peak_is_sampled"] = True
        return out

    def killed_message(self):
        if not self.killed:
            return None
        peak = "peak unknown" if self.peak_memory_mb is None else f"peak {round(self.peak_memory_mb)} MB"
        limit = f", limit {self.limit_mb} MB" if self.killed == "memory" and self.limit_mb else ""
        return f"killed: {self.killed} ({peak}{limit}) after {round(self.elapsed_s, 1)}s"


def run_guarded(cmd, cwd=None, env=None, timeout_s=None, memory_mb=None, log_path=None, echo=None):
    """Run `cmd`, capturing stdout and stderr together (also appended to `log_path` as they come, so
    a killed run leaves its log; also written to the binary stream `echo`, if given). Returns a RunResult."""
    limit = resolve_limit_mb(memory_mb)
    method = guard_method(limit)
    environment = dict(os.environ if env is None else env)
    argv = list(cmd)
    res_file = None
    if method == "scope":
        fd, res_file = tempfile.mkstemp(prefix="kani-run-", suffix=".res")
        os.close(fd)
        environment["KANI_PLUGIN_RES"] = res_file
        argv = ["systemd-run", "--user", "--scope", "--quiet", "-p", f"MemoryMax={limit}M", "-p", "MemorySwapMax=0",
                "--", "bash", "-c", SCOPE_WRAPPER, "_", *cmd]
    before = os.times()
    started = time.time()
    try:
        proc = subprocess.Popen(argv, cwd=cwd, env=environment, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                stdin=subprocess.DEVNULL, start_new_session=True)
    except OSError as e:
        if res_file:
            os.unlink(res_file)
        return RunResult(cmd, 127, None, 0.0, None, None, method, limit, f"cannot run `{cmd[0]}`: {e}", started)
    chunks = []
    log = open(log_path, "wb") if log_path else None

    def pump():
        for chunk in iter(lambda: proc.stdout.read(8192), b""):
            chunks.append(chunk)
            if log:
                log.write(chunk)
                log.flush()
            if echo:
                echo.write(chunk)
                echo.flush()
    reader = threading.Thread(target=pump, daemon=True)
    reader.start()
    killed, peak, last_check = None, 0.0, 0.0
    while proc.poll() is None:
        time.sleep(POLL_S)
        elapsed = time.time() - started
        if timeout_s is not None and elapsed > timeout_s:
            killed = "timeout"
            _kill_group(proc)
            break
        if method == "watchdog" and elapsed - last_check >= WATCHDOG_EVERY_S:
            last_check = elapsed
            rss = group_rss_mb(proc.pid)
            peak = max(peak, rss)
            if limit and rss > limit:
                killed = "memory"
                _kill_group(proc)
                break
    proc.wait()
    reader.join(timeout=10)
    proc.stdout.close()
    if log:
        log.close()
    elapsed = time.time() - started
    text = b"".join(chunks).decode("utf-8", errors="replace")
    peak_mb, cpu_s = (peak or None), None
    if method == "scope":
        values = {}
        try:
            for line in Path(res_file).read_text().splitlines():
                key, _, value = line.partition("=")
                if value.strip().isdigit():
                    values[key] = int(value)
        except OSError:
            pass
        finally:
            os.unlink(res_file)
        peak_mb = values["peak_bytes"] / (1024 * 1024) if "peak_bytes" in values else None
        cpu_s = values["cpu_usec"] / 1e6 if "cpu_usec" in values else None
        if not killed and values.get("oom_kill", 0) > 0:
            killed = "memory"
        elif not killed and proc.returncode in (137, -9) and peak_mb and limit and peak_mb >= 0.9 * limit:
            killed = "memory"
    else:
        after = os.times()
        cpu_s = (after.children_user + after.children_system) - (before.children_user + before.children_system)
        if killed:
            cpu_s = None  # processes that were killed are not counted by the kernel for their parent
    return RunResult(cmd, proc.returncode, killed, elapsed, peak_mb, cpu_s, method, limit, text, started)


# ---- Exports ---------------------------------------------------------------------------


def _summarize_results_module():
    import importlib.util
    path = Path(__file__).resolve().parent / "summarize-results.py"
    spec = importlib.util.spec_from_file_location("summarize_results", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def annotate_export(path, result):
    """Add the run's resource record to an export Kani wrote (`plugin_run`)."""
    path = Path(path)
    try:
        doc = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return False
    doc["plugin_run"] = result.record()
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(json.dumps(doc, indent=2) + "\n", encoding="utf-8")
    os.replace(tmp, path)
    return True


def stopped_export(result, harness=None, log_text=None):
    """The export document for a run that was stopped (memory or time) before it wrote its own."""
    from kani_export import NO_CONFIGURATION_NOTE
    summarize = _summarize_results_module()
    finished, unfinished = summarize.verdicts_from_log(log_text if log_text is not None else result.text)
    stopped_kind = "OUT_OF_MEMORY" if result.killed == "memory" else "TIMEOUT"
    entries = []

    def entry(name, outcome, failed=()):
        return {"name": name, "crate_name": "", "file": "", "line": 0, "outcome": outcome,
                "resources": {"verification_time_s": round(result.elapsed_s, 1) if outcome["kind"] == stopped_kind
                              else 0.0},
                "failed_properties": [{"class": None, "description": d, "file": None, "line": None}
                                      for d in failed],
                "checks": {}, "covers": {}, "attributes": {}, "is_automatically_generated": False,
                "is_bounded": False, "generated_concrete_test": False, "from_console_log": True}

    for item in finished:
        entries.append(entry(item["name"], {"kind": "COMPLETED", "verdict": item["verdict"]}, item.get("failed", [])))
    stopped = harness or (unfinished[-1] if unfinished else None)
    if stopped is None and not entries:
        stopped = "(the whole run)"
    if stopped and stopped not in [e["name"] for e in entries]:
        entries.append({**entry(stopped, {"kind": stopped_kind}), "plugin_run": result.record()})
    entries.extend(entry(n, {"kind": "NOT_RUN"}) for n in unfinished if n != stopped)
    state = "KILLED" if result.killed == "memory" else "TIMEOUT"
    verdicts = [e["outcome"].get("verdict") for e in entries if e["outcome"]["kind"] == "COMPLETED"]
    return {
        "export_shape": "flat",
        "configuration_known": False,
        "configuration": {},
        "export_notes": [
            NO_CONFIGURATION_NOTE,
            "Kani was stopped before it wrote an export: this document was written by the plugin from "
            "its console output."],
        "tools": {
            "kani": None},
        "run_state": state,
        "started_at": time.strftime(
            "%Y-%m-%dT%H:%M:%SZ",
            time.gmtime(
                result.started)),
        "summary": {
            "total": len(entries),
            "successful": verdicts.count("SUCCESS"),
            "failed": verdicts.count("FAILURE")},
        "harnesses": entries,
        "plugin_run": result.record(),
    }


def write_stopped_export(path, result, harness=None):
    """Write `stopped_export` to `path` unless Kani wrote one (never overwrites Kani's own)."""
    path = Path(path)
    if path.exists() or not result.killed:
        return False
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(stopped_export(result, harness), indent=2) + "\n", encoding="utf-8")
    return True


def add_memory_arg(parser):
    parser.add_argument("--memory-limit-mb", help="memory cap for each Kani run in MB (0: none). Default: "
                        "KANI_PLUGIN_MEMORY_MB, else half of the available memory (see kani_run.py)")


def apply_memory_arg(args):
    """Make `--memory-limit-mb` the cap of every guarded run of this process."""
    if getattr(args, "memory_limit_mb", None) is not None:
        os.environ["KANI_PLUGIN_MEMORY_MB"] = str(args.memory_limit_mb)


def safe_name(name):
    return re.sub(r"[^A-Za-z0-9_.-]", "_", name)
