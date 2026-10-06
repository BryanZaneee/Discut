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

## Initial macOS MVP package (source commit `3f5d9b5`)

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

## Current running sessions — October 6, 2026, follow-up

At the owner's request, both already-running clients were sampled as-is for 60
one-second intervals after a five-second warmup. No Discut builds or UI interaction
ran during sampling. Account/call state and equal window visibility were not independently
confirmed. This is a current-session comparison, not a controlled feature-parity benchmark.

| Metric | Discord | Discut |
|---|---:|---:|
| Processes | 7 | 1 |
| Mean physical footprint | 620.23 MiB | 143.22 MiB |
| Peak during these 60 seconds | 621.45 MiB | 143.22 MiB |
| Mean summed RSS | 617.27 MiB | 136.15 MiB |
| Mean CPU, 100% = one core | 0.02346% | 0.00093% |

Discut's physical footprint was approximately 77% lower in these observed states.
Both were effectively idle; CPU ratios at this scale are not useful performance claims.
Discut's footprint stayed constant throughout the sample, which is not evidence of a
leak in this interval, but also does not exclude growth during longer active workloads.
[Raw JSON](discut-evidence/discut-current-session.json) and
[CSV](discut-evidence/discut-current-session.csv) retain the full measurements.

### Where further savings are plausible

A separate three-second macOS `sample` showed Discut's main thread waiting in the
AppKit event loop. `vmmap -summary` reported about 44.8 MiB of live allocations in the
default malloc zone, with 14.5 MiB of dirty/swapped allocator slack. Graphics categories
included 26.8 MiB of IOSurface mappings and 44.2 MiB of graphics-owned mappings. These
categories are OS accounting observations, not a guarantee of reclaimable memory or
an exact attribution to a particular cache. The process lifetime peak was 340.3 MiB;
the triggering workload was not recorded. Large reserved virtual-address ranges are
not actual RAM consumption. Raw system profiles remain local in ignored `target/metrics/`.

Priorities, preserving messaging, groups and media calls:

1. **Measure and trim offscreen decoded media first.** The UI has separate ceilings
   for static artwork (64 MiB), emoji (16 MiB), artwork animation (128 MiB), inline
   media stills (96 MiB), inline animation (128 MiB), and the viewer (128 MiB). These
   are limits, not current allocation measurements, and their accounting can overlap.
   A shared retained-media budget and eviction of old offscreen animation frames can
   constrain growth after browsing many channels. Keep posters/disk cache and visible
   media warm to avoid replacing RAM savings with repeated downloads or scrolling jank.
2. **Profile graphics retention across resize, hide and restore.** The observed graphics
   categories are substantial. Check drawable/font-atlas/texture retention using a fixed
   viewport and repeatable media workload before changing renderer settings. Do not
   recreate the GPU device on ordinary channel switches or sacrifice rendering quality
   merely to reduce a counter.
3. **Measure active text and calls separately.** Record typing/scrolling frame time and
   two-way audio latency/dropouts alongside memory and CPU. The current idle sample
   cannot justify cutting jitter buffers, message reconciliation, or reconnect state.
4. **Leave sleeping workers alone for now.** The main loop is already event-driven and
   the runtime has two workers. Game/extension/update integrations are already disabled.
   Deleting dormant code mainly affects package size; it is not a demonstrated runtime
   memory saving. A second macOS-only frontend is not justified by this profile yet.

No additional runtime-memory saving is claimed from the Spaces change below.

### macOS desktop behavior

The owner reported that Discut remained visible at the same position across desktops.
The running app was registered as a foreground application with bundle ID
`app.discut.desktop`; no Discut/Serein-specific all-desktop assignment was found in
the inspected Spaces preferences. The title-bar flag maps to transparency, not a
borderless window. The exact origin of the reported sticky behavior was not reproduced.

The targeted correction explicitly configures the main window at creation to use
`Managed`, clears `CanJoinAllSpaces`, `MoveToActiveSpace`, `Stationary` and `Transient`,
and enables moving. Fullscreen and cycling flags are preserved. Apple's
[collection-behavior documentation](https://developer.apple.com/documentation/appkit/nswindow/collectionbehavior-swift.struct)
defines these Spaces/Mission Control behaviors. The correction is applied once at
startup, without polling or extra background work. Desktop switching still requires
an interactive check after restarting into the new package; a bitmask regression test
does not substitute for that check.

`scripts/discut-build.sh xtask check` passed after the correction: formatting,
strict workspace Clippy, 1,168 reported passing test results (including subprocess
output), zero failures, 25 ignored cases, production compilation and policy checks.
The log is local at `target/discut-spaces-check.log`.

The corrected production package built successfully and passed strict code-signature
verification (`target/discut-spaces-package.log`). The running user session was left
open; quit and reopen `dist/Discut.app` to load the correction. No interactive Spaces
pass or post-change performance improvement is claimed.
