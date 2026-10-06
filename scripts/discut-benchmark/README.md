# Discut resource sampler (macOS)

Python 3 standard library only. This script observes already-running processes. It does
not launch apps, log in, inspect credentials/messages, change settings, or initiate calls.

```sh
python3 scripts/discut-benchmark/benchmark.py --self-test
python3 scripts/discut-benchmark/benchmark.py \
  --app Discord /Applications/Discord.app/Contents/MacOS/Discord friends-idle signed-in \
  --app Discut /absolute/path/to/discut demo-idle offline-synthetic \
  --warmup 30 --duration 60 --interval 1 \
  --notes 'Both windows visible; compilation stopped; same display scale.' \
  --output /tmp/discut-resource-sample
```

Replace the Discut path with the actual release executable. Every `--app` requires
name, exact executable path, workload label, and authentication-state label. Multiple
instances of the same executable/bundle are intentionally counted together. Close
unwanted instances before measuring. JSON records process paths/IDs, never command
arguments. Review paths before sharing; they may include your local username.

Outputs `/tmp/discut-resource-sample.json` and `.csv` (existing files at those names
are overwritten). JSON includes hardware/OS, sampling method, full process samples,
errors, and arithmetic mean, median, nearest-rank p95, and peak for each metric. CSV includes workload
and authentication labels alongside aggregate samples. No percentage improvement is
automatically calculated: the example compares an authenticated app against synthetic
data and **cannot establish equivalent-workload savings or a working Discord client**.

## Method

- Process identity comes from `proc_pidpath`, matching the exact executable and its
  outermost `.app` directory (with a path boundary), plus recursively discovered child
  processes, including children outside the bundle. PID/parent IDs come from `ps`.
- Memory is the sum of macOS `proc_pid_rusage(RUSAGE_INFO_V2)` physical footprint.
  Resident bytes are retained separately; summed RSS may double-count shared pages.
- Idle wakeups per second use deltas of `ri_pkg_idle_wkups` with the same interval
  and process-identity validation as CPU. This counter is not a battery-energy measurement.
- CPU uses deltas of cumulative user+system CPU nanoseconds divided by actual monotonic
  elapsed time. **100% means one fully occupied core**; multi-core values may exceed 100%.
  The first counter read is a baseline, not a measured sample. No decaying `ps %cpu` values.
- PID start times distinguish reused PIDs. Process churn invalidates that CPU interval
  instead of inventing usage. Missing processes/read errors are null, never a zero-memory
  result. Memory-read failures cause nonzero exit after preserving evidence. JSON `count`
  tells you how many valid samples contributed to each statistic.
- An exact root executable must be visible in every snapshot. If it is absent or its
  path cannot be read, that sample is invalid even when bundle helpers remain running.
- Means are arithmetic means of valid sample values; inspect interval lengths before
  interpreting a run with scheduling delays. Median idle CPU alone can conceal bursts.
- Short-lived processes between snapshots and external helpers reparented before
  discovery may be missed. Footprint is an OS accounting metric, not a measure of
  total machine RAM reclaimed on app exit. This does not measure GPU utilization,
  frame latency, battery energy, audio quality, or memory pressure independently.

For a fair comparison, stop builds, allow both apps to settle, match window visibility,
channel history, participant counts, media quality, and authentication state. Measure
chat, background idle, voice, and screen sharing separately. Obtain live-session
authorization before preparing live workloads. Prefer several repeated isolated runs;
simultaneous runs are useful observations but can interfere with each other. Do not
extrapolate macOS results to Windows/Linux. If the sandbox denies process access, run
the same read-only command with the environment's approved process-observation access;
never report permission failures as resource savings.

The self-test exercises path-boundary exclusion, transitive external descendants, CPU
units, PID reuse, missing-process handling, summary statistics, and native ABI size.
On macOS it also verifies that the current Python process returns plausible resource
counters; this sanity check is not a performance benchmark.

## Reproducible synthetic native screenshots

With a release build that includes the `demo` feature:

```sh
/absolute/path/to/discut --demo --demo-group-dm --demo-screenshot=/tmp/discut-group.png
```

The option requires `--demo`, a demo-enabled build, and a `.png` filename. The parent
directory must already exist; an existing destination image is overwritten. It uses
egui's native WGPU screenshot callback after frame 15, with repaint requests bounded
to frame 45. Standard error confirms successful saving or reports a write failure.
The app stays open. Inspect the saved image before treating it as verified evidence.
Only the application's rendered viewport is captured, without other desktop windows.
Do not use screenshot mode during resource measurements: its bounded forced repaints
and image encoding add work. Screenshots show synthetic data, not live compatibility.
`EFRAME_SCREENSHOT_TO` is unsupported by this project's pinned WGPU backend.
