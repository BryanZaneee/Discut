# Discut resource measurements

## Scope

Collected locally on 2026-10-06, Apple M3 Pro, 18 GiB RAM, macOS 27.0.1.
Official Discord 0.0.413 uses Electron 42.11.4. Its application bundle occupies
approximately 480 MiB on disk. Runtime measurements count its entire visible process
family, including helper executables, rather than just the parent process.

These observations are **not an equivalent-workload benchmark**. Signed-in Discord
was idle on Friends; the native baseline was a synthetic offline group preview.
Live chat, calls, video and sharing need separate matched measurements.

## Upstream baseline

Serein commit `ec75f729b11d5f3acca36b3563ee8db492b220aa`, unmodified release build,
`--no-default-features --features demo`, voice included. One native process versus
seven Discord processes. Both apps ran simultaneously with builds stopped. Sampling:
10-second warmup, 60 one-second intervals, macOS `proc_pid_rusage(RUSAGE_INFO_V2)`.
Visibility was not rigorously matched; this is exploratory evidence only.

| Metric | Official Discord, signed in | Upstream, offline preview |
|---|---:|---:|
| Median physical footprint | 665.49 MiB | 100.44 MiB |
| Mean physical footprint | 665.51 MiB | 105.78 MiB |
| Peak physical footprint | 666.34 MiB | 259.56 MiB |
| Mean CPU, 100% = one core | 0.0205% | 0.0030% |

The baseline preview remained on history-loading placeholders due to a discarded
fixture history request. Discut fixes that fixture integration. Therefore this
baseline cannot establish the cost of rendering populated chat history or a resource
improvement caused by the fork. Idle CPU is near measurement noise and short runs
cannot establish battery savings. No percentage saving is claimed.

[Raw JSON](discut-evidence/upstream-v-discord.json) and
[CSV](discut-evidence/upstream-v-discord.csv) retain samples and method metadata;
checkout paths have been replaced with `<checkout>` for sharing. Original files remain
in ignored `target/metrics/`.

## Reproduce

Follow [sampler instructions](../scripts/discut-benchmark/README.md). Record build,
authentication, foreground/background state, participant count and exact workload.
Warm up after downloads and history loading, stop compilation, and perform repeated
isolated matching text/call runs. Measure physical footprint separately from summed
RSS; neither is interchangeable with app bundle size or total machine RAM reclaimed.

## Discut optimized preview

Measured after native screenshot verification, using the same sampler and 60 one-second
intervals. Screenshot output confirmed populated group history with images, an attachment
and a reaction. The native accessibility tool stalled for 76 minutes before collection;
the app remained running. This unusually long settling period and uncontrolled window
visibility are additional reasons not to treat this run as an equivalent-workload test.
No compilation or interactive workload ran during collection.

| Metric | Official Discord, signed in | Discut, offline populated group |
|---|---:|---:|
| Processes | 7 | 1 |
| Median physical footprint | 656.79 MiB | 99.25 MiB |
| Mean physical footprint | 657.11 MiB | 99.25 MiB |
| Peak physical footprint | 659.96 MiB | 99.25 MiB |
| Mean CPU, 100% = one core | 0.0270% | 0.0002% |

[Raw JSON](discut-evidence/discut-demo-v-discord.json) and
[CSV](discut-evidence/discut-demo-v-discord.csv). Discord's Friends screen was its last
observed state, not independently reverified during collection. This measures an idle
fixture, not authenticated Discut performance. The call screenshot is visual evidence
only and was not used as a call-performance benchmark.

This preview predates the final conversation-selection changes.

## Final macOS package

The standard production build uses `--release --locked --no-default-features`, with
voice included. `cargo xtask package` completed, and `codesign --verify --strict`
passed. It is locally ad-hoc signed, not notarized. The retained upstream icon is
packaged as ICNS; no full-Xcode icon compiler is needed.

| Artifact | Size | Method |
|---|---:|---|
| Discut executable | 63.36 MiB | file length, 66,438,080 bytes |
| Discut app bundle | 68.29 MiB | `du -sk`, 69,924 KiB allocated |
| Discut ZIP | 43.80 MiB | file length, 45,923,386 bytes; `ditto -c -k --sequesterRsrc --keepParent` |
| Installed Discord bundle | 480.79 MiB | `du -sk`, 492,332 KiB allocated |

Disk size is separate from running memory. There is no equivalent full upstream
bundle baseline for this package comparison; no fork-attributable package saving
is asserted.

### Fresh packaged-app startup observation

The final production app was launched directly, without accessibility inspection or
demo mode. After 30 seconds of warmup, the sampler collected 60 one-second intervals.
Discut had not been signed into during this task; its initial screen/authentication
state was not independently inspected. Discord was signed in and last observed on
Friends. Discut compilation had stopped; unrelated machine activity and window
visibility were uncontrolled. This remains an unequal-workload observation.

| Metric | Official Discord, signed in | Discut, awaiting user sign-in |
|---|---:|---:|
| Median physical footprint | 655.77 MiB | 101.95 MiB |
| Mean physical footprint | 655.82 MiB | 101.95 MiB |
| Peak physical footprint | 656.76 MiB | 101.95 MiB |
| Mean CPU, 100% = one core | 0.01954% | 0.00005% |

[Raw JSON](discut-evidence/discut-package-v-discord.json) and
[CSV](discut-evidence/discut-package-v-discord.csv) contain all samples. There were
60 valid samples for each metric and no reported read failures. Tiny idle CPU values
are not a meaningful efficiency or battery-life claim. Signed-in text and two-way
call resource use remain unmeasured, and live interoperability remains unverified.
