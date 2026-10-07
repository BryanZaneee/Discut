# Discut roadmap

The goal is a fast, lightweight native Discord client with familiar sign-in, existing
conversations, groups and calls. Extras must earn their resource cost. This is a plan,
not a claim that live compatibility or every platform is already verified.

Source: owner requests on October 6–7, 2026, the [MVP checklist](docs/discut-mvp.md),
[resource measurements](docs/discut-performance.md) and the
[open-source readiness review](docs/development/open-source-review.md).

| ID | Status | Goal | Acceptance criteria |
|---|---|---|---|
| D-01 | Next: live verification | Seamless existing-account sign-in and conversation list | Owner verifies server, DM and group completeness, on-demand history, local selection and restart/account switching. No credential extraction. |
| D-02 | Next: live verification | Messages and calls that work reliably | Bidirectional text, attachments, group management, audible two-way server/group calls, reconnect and capture stop pass the private acceptance checklist. |
| D-03 | Next: benchmark | Comparable resource evidence | Same signed-in account/workload, viewport, warmup and media settings; repeated text-idle, scrolling, voice and sharing runs; report footprint, CPU, responsiveness and limitations. |
| D-04 | Needs reproduction/verification | Normal macOS Spaces behavior | Restart corrected app; move it with Mission Control, switch desktops, test fullscreen and tray restore. It does not follow every desktop unexpectedly. |
| D-05 | Proposed optimization | Shared retained-media budget | Instrument current pool use; evict old offscreen frames while keeping visible content and still previews. Show reduced retained memory after media browsing without scrolling/call regressions. |
| D-06 | Next: packaging | Windows/Linux build and install identity | Native CI/runtime checks pass; packages, shortcuts and update destinations identify Discut and coexist safely with Serein. |
| D-07 | Before binary release | Independent, reviewable release pipeline | Discut artifact paths, notices, signing/update policy and destinations verified; no inherited Serein publisher runs. Source publication does not imply a supported binary release. |
| D-08 | Before release | Security and dependency review | Review authentication and storage boundaries; license/dependency checks pass; confirm private vulnerability reporting and tested support policy. |

## Implemented, with validation limits

- Native Rust foundation and isolated Discut application/data identity.
- Optional extension, game-scanning, Spotify presence and upstream update workers disabled.
- Group creation/member management with bounded requests and stale-response handling.
- All service-provided conversations included by default; optional per-account navigation choices.
- Mac package, offline screenshots, automated checks and read-only process-family sampler.
- Targeted managed-Spaces correction packaged; interactive acceptance remains D-04.
- Open-source documentation, issue forms, PR guidance and initial improvement backlog.

## Not planned

Nitro commerce, the shop, Quests, embedded Activities, game overlays, telemetry,
a separate messaging backend, and replacing the Rust app with an Electron wrapper.
A separate macOS-only frontend is not planned without evidence that it materially
improves real workloads beyond the shared implementation.
