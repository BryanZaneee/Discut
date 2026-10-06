#!/usr/bin/env python3
"""Read-only macOS process-family sampling. Python standard library only."""

import argparse
import csv
import ctypes
import datetime
import json
import math
import os
from pathlib import Path
import platform
import statistics
import subprocess
import time


class RUsage(ctypes.Structure):
    # macOS SDK sys/resource.h: rusage_info_v2; CPU counters are nanoseconds.
    _fields_ = [("uuid", ctypes.c_uint8 * 16)] + [
        (name, ctypes.c_uint64) for name in (
            "user", "system", "idle_wakeups", "interrupt_wakeups", "pageins",
            "wired", "rss", "footprint", "start", "exit", "child_user",
            "child_system", "child_idle", "child_interrupt", "child_pageins",
            "child_elapsed", "read_bytes", "written_bytes",
        )
    ]


def family(records, executable, bundle):
    """Match exact root executable/bundle membership, then descendant PIDs."""
    selected = {pid for pid, rec in records.items()
                if rec["path"] == executable or
                (bundle and rec["path"].startswith(bundle + "/"))}
    while True:
        expanded = selected | {pid for pid, rec in records.items()
                               if rec["ppid"] in selected}
        if expanded == selected:
            return sorted(selected)
        selected = expanded


def summary(values):
    values = sorted(value for value in values if value is not None)
    if not values:
        return {"count": 0, "mean": None, "median": None, "p95": None, "peak": None}
    return {"count": len(values), "mean": statistics.mean(values), "median": statistics.median(values),
            "p95": values[math.ceil(len(values) * .95) - 1], "peak": values[-1]}


def counter_rate(previous, current, seconds):
    # PID reuse and churn invalidate the interval rather than inventing a delta.
    if not current or previous.keys() != current.keys():
        return None
    deltas = [current[key] - previous[key] for key in current]
    return sum(deltas) / seconds if min(deltas) >= 0 and seconds > 0 else None


def cpu_percent(previous, current, seconds):
    rate = counter_rate(previous, current, seconds)
    return rate / 1e9 * 100 if rate is not None else None


def snapshot(lib):
    # Only PID/parent IDs; never collect command arguments, tokens or user data.
    output = subprocess.check_output(["/bin/ps", "-axo", "pid=,ppid="], text=True)
    records = {}
    for line in output.splitlines():
        pid, ppid = map(int, line.split())
        path = ctypes.create_string_buffer(4096)
        if lib.proc_pidpath(pid, path, len(path)) <= 0:
            continue
        records[pid] = {"ppid": ppid, "path": os.fsdecode(path.value)}
    return records


def target(name, executable, scenario, auth):
    path = Path(executable).resolve(strict=True)
    # Outermost bundle includes nested helper .apps, including orphan helpers.
    bundles = [parent for parent in path.parents if parent.suffix == ".app"]
    return {"name": name, "executable": str(path),
            "bundle": str(bundles[-1]) if bundles else None,
            "scenario": scenario, "auth_state": auth}


def sample(lib, app, records):
    if not any(rec["path"] == app["executable"] for rec in records.values()):
        return [], [{"reason": "Exact root executable absent or path unreadable; helper-only sample rejected"}]
    processes, errors = [], []
    for pid in family(records, app["executable"], app["bundle"]):
        usage = RUsage()
        if lib.proc_pid_rusage(pid, 2, ctypes.byref(usage)) != 0:
            errors.append({"pid": pid, "errno": ctypes.get_errno()})
            continue
        processes.append({"pid": pid, "path": records[pid]["path"],
                          "start": usage.start, "cpu_ns": usage.user + usage.system,
                          "idle_wakeups": usage.idle_wakeups,
                          "footprint_bytes": usage.footprint, "rss_bytes": usage.rss})
    return processes, errors


def self_test():
    records = {1: {"path": "/A.app/main", "ppid": 0},
               2: {"path": "/A.app/helper", "ppid": 0},
               3: {"path": "/external/helper", "ppid": 2},
               4: {"path": "/A.app.fake/main", "ppid": 0},
               5: {"path": "/external/child", "ppid": 3}}
    assert family(records, "/A.app/main", "/A.app") == [1, 2, 3, 5]
    assert family(records, "/A.app/main", None) == [1]
    assert cpu_percent({(1, 4): 1_000_000_000}, {(1, 4): 3_000_000_000}, 2) == 100
    assert cpu_percent({(1, 4): 1}, {(1, 5): 2}, 1) is None
    assert cpu_percent({(1, 4): 2}, {}, 1) is None
    assert counter_rate({(1, 4): 10}, {(1, 4): 16}, 2) == 3
    assert counter_rate({(1, 4): 10}, {(1, 4): 9}, 2) is None
    helpers_only = {pid: rec for pid, rec in records.items() if pid != 1}
    processes, errors = sample(None, {"executable": "/A.app/main", "bundle": "/A.app"}, helpers_only)
    assert not processes and "root executable absent" in errors[0]["reason"]
    assert summary([None, 4, 1, 2, 3]) == {"count": 4, "mean": 2.5, "median": 2.5, "p95": 4, "peak": 4}
    assert summary([0, 0, 12])["mean"] == 4
    assert ctypes.sizeof(RUsage) == 160
    # Cross-checked with offsetof/sizeof static assertions against local macOS SDK.
    assert [getattr(RUsage, key).offset for key in
            ("user", "idle_wakeups", "rss", "footprint", "start")] == [16, 32, 64, 72, 80]
    if platform.system() == "Darwin":
        lib = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
        usage = RUsage()
        assert lib.proc_pid_rusage(os.getpid(), 2, ctypes.byref(usage)) == 0
        assert usage.rss > 0 and usage.footprint > 0 and usage.start > 0
    print("PASS: family boundaries/root requirement, descendants, counter rates/PID reuse, burst mean, ABI size")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--app", nargs=4, action="append", metavar=("NAME", "EXECUTABLE", "SCENARIO", "AUTH"))
    parser.add_argument("--duration", type=float, default=60)
    parser.add_argument("--interval", type=float, default=1)
    parser.add_argument("--warmup", type=float, default=30)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--notes", default="")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if platform.system() != "Darwin":
        parser.error("live collection requires macOS")
    if not args.app or not args.output:
        parser.error("--app and --output are required")
    if not all(math.isfinite(v) for v in (args.duration, args.interval, args.warmup)) or not (
            args.duration >= args.interval >= .1 and args.warmup >= 0):
        parser.error("require duration >= interval >= 0.1 and warmup >= 0; values must be finite")
    apps = [target(*app) for app in args.app]
    if len({app["name"] for app in apps}) != len(apps):
        parser.error("application names must be unique")
    lib = ctypes.CDLL("/usr/lib/libproc.dylib", use_errno=True)
    lib.proc_pidpath.argtypes = [ctypes.c_int, ctypes.c_void_p, ctypes.c_uint32]
    lib.proc_pidpath.restype = ctypes.c_int
    lib.proc_pid_rusage.argtypes = [ctypes.c_int, ctypes.c_int, ctypes.c_void_p]
    lib.proc_pid_rusage.restype = ctypes.c_int
    hardware = {key: subprocess.check_output(["/usr/sbin/sysctl", "-n", key], text=True).strip()
                for key in ("hw.model", "hw.memsize", "hw.logicalcpu", "machdep.cpu.brand_string")}
    print(f"Read-only sampling: warmup {args.warmup}s, collection {args.duration}s", flush=True)
    time.sleep(args.warmup)
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    start = time.monotonic()
    rows, previous = [], {}
    # Baseline at t=0 supplies CPU counter origins; it is not a measured row.
    iterations = math.ceil(args.duration / args.interval)
    for index in range(iterations + 1):
        deadline = start + min(index * args.interval, args.duration)
        time.sleep(max(0, deadline - time.monotonic()))
        records = snapshot(lib)
        for app in apps:
            processes, errors = sample(lib, app, records)
            now = time.monotonic()
            counters = {(p["pid"], p["start"]): p["cpu_ns"] for p in processes}
            wakeups = {(p["pid"], p["start"]): p["idle_wakeups"] for p in processes}
            old_time, old_counters, old_wakeups = previous.get(app["name"], (now, {}, {}))
            valid_memory = bool(processes) and not errors
            row = {"app": app["name"], "scenario": app["scenario"], "auth_state": app["auth_state"],
                   "elapsed_seconds": now - start,
                   "interval_seconds": now - old_time, "process_count": len(processes),
                   "physical_footprint_bytes": sum(p["footprint_bytes"] for p in processes) if valid_memory else None,
                   "rss_bytes": sum(p["rss_bytes"] for p in processes) if valid_memory else None,
                   "cpu_percent_one_core": cpu_percent(old_counters, counters, now - old_time) if index and not errors else None,
                   "idle_wakeups_per_second": counter_rate(old_wakeups, wakeups, now - old_time) if index and not errors else None,
                   "processes": processes, "errors": errors}
            previous[app["name"]] = (now, counters if not errors else {}, wakeups if not errors else {})
            if index:
                rows.append(row)
    metrics = ("physical_footprint_bytes", "rss_bytes", "cpu_percent_one_core", "idle_wakeups_per_second")
    report = {"started_utc": started, "os": platform.platform(), "hardware": hardware,
              "duration_requested_seconds": args.duration, "duration_actual_seconds": time.monotonic() - start,
              "interval_seconds": args.interval, "warmup_seconds": args.warmup,
              "apps": apps, "notes": args.notes,
              "method": "proc_pid_rusage RUSAGE_INFO_V2, summed process families; CPU counter deltas / monotonic wall time; 100%=one core; nearest-rank p95",
              "limitations": ["Workloads/authentication are operator labels, not independently verified; no automatic improvement claim.",
                              "Summed RSS may double-count shared pages; footprint is a separate OS accounting metric.",
                              "Process churn/PID reuse invalidates CPU interval; missing processes or read errors yield null, not zero.",
                              "Short-lived processes between snapshots may be missed; external reparented helpers outside the bundle may be missed.",
                              "Concurrent applications contend for resources; run repeated isolated matched scenarios for stronger evidence."],
              "summary": {app["name"]: {metric: summary([row[metric] for row in rows if row["app"] == app["name"]])
                                         for metric in metrics} for app in apps}, "samples": rows}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.with_suffix(".json").write_text(json.dumps(report, indent=2) + "\n")
    with args.output.with_suffix(".csv").open("w", newline="") as file:
        fields = [key for key in rows[0] if key not in ("processes", "errors")]
        writer = csv.DictWriter(file, fieldnames=fields, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(rows)
    print(json.dumps(report["summary"], indent=2))
    if any(row["physical_footprint_bytes"] is None for row in rows):
        raise SystemExit("Incomplete collection: inspect errors/missing process samples in JSON")


if __name__ == "__main__":
    main()
