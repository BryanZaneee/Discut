# Discut MVP verification

Discut is a local experimental fork of [Serein](https://github.com/ViceVerse-cz/Serein),
starting at `ec75f729b11d5f3acca36b3563ee8db492b220aa`. Upstream licenses and notices
remain in the repository and package. This is not an independently rewritten Discord
implementation, and it is not endorsed by Discord or Serein.

## Required product scope

Native Rust/egui messaging on macOS, Windows and Linux: server channels, direct and
group conversations, organization, attachments, and native voice/video/screen sharing.
No Nitro storefront, shop, Quests, embedded Activities, or game overlays. Discut disables
upstream self-updating, game detection/Rich Presence and the extension host/catalog.
These product restrictions must apply at runtime, not just hide their settings pages.

## Existing account and conversation choices

The sign-in flow uses the existing Discord account. The service's READY navigation
catalog supplies servers, channels, DMs and groups; users do not manually migrate chats.
All supplied conversations appear by default. Messages load in bounded pages on demand,
so signing in does not download an account's entire message or attachment history.
Live completeness and account restoration still require owner verification.

The sidebar's **Choose conversations** opens Chat settings with searchable server,
direct-message and group lists. Checkboxes affect navigation on this device only;
**Restore all conversations** removes exclusions. Existing memberships, drafts and
remote conversations are preserved. Choices use existing account-isolated storage,
including load/save failure recovery. Notifications remain enabled as configured, and
direct links or notifications may open excluded conversations; this is not a metadata
privacy boundary. Exclusions and existing shortcuts share a bounded 256-entry budget.

## Evidence rules

- A working build or synthetic fixture is not proof of Discord interoperability.
- Keep live-account and offline-demo resource samples separate and label authentication,
  workload, visibility, build, process set and sampling method.
- Collect after compilation ends. Count helper processes. Keep physical footprint and
  resident memory distinct. CPU is expressed as a percentage of one logical core.
- No published claims of chat/call savings until equivalent live scenarios are measured.
- macOS is the local validation platform. Windows/Linux remain source targets until
  their native builds and runtime checks pass.

## Live acceptance checklist

Use a private owner-controlled test space and an explicitly willing second participant.
The owner signs in inside Discut. Never paste credentials into chat or extract them
from another application's storage.

1. Sign in, load server/channel and existing DM/group navigation, and receive normal-user
   READY. Compare the visible catalog with official Discord; verify local selection,
   restore-all and persistence across restart/account switching.
2. Send a labeled test message; verify in official Discord. Reply there and verify in Discut.
3. Verify reply, edit, delete, reaction, attachment, pagination and draft restoration.
4. Create a group, add a willing participant, rename it, exchange messages and remove/leave
   as permitted. Distinguish REST success from reflected service state.
5. Join a private voice channel on separate endpoints; verify audible two-way audio,
   mute/deafen, device selection and hangup. Repeat in a group DM.
6. Verify camera and screen sharing, then ensure capture ends when stopped.
7. Verify reconnect and app restart without duplicate sends or lost drafts.
8. Measure matching text-idle and call workloads in both clients with the sampler.

Record pass/fail and redacted failure categories only. Do not save account/channel IDs,
message contents, credentials, signed attachment URLs, or screenshots of private chats.

## Reproducible local commands

`scripts/discut-build.sh` defaults to the native package task and caps build concurrency.
The Rust toolchain is pinned. CMake is required for bundled Opus. On this machine CMake
is installed in the ignored `target/build-tools` Python environment.

```sh
scripts/discut-build.sh xtask check
scripts/discut-build.sh build --release --locked --no-default-features --features demo -p serein
target/release/serein --demo --demo-group-dm
python3 scripts/discut-benchmark/benchmark.py --self-test
```

The internal Cargo package/binary name remains `serein` to keep upstream integration small;
the packaged application, credential namespace and user-data directory are Discut-specific.
See [benchmark instructions](../scripts/discut-benchmark/README.md) and the final measurement
report for actual results. Nothing in this checklist alone is a completion claim.

## Implementation roadmap and remaining gates

1. **Native core and local preview:** fork the existing Rust implementation; isolate
   Discut's identity/data; disable optional background integrations; add group creation
   and membership controls; build the macOS app and run synthetic behavior checks.
2. **Live MVP gate:** owner-controlled sign-in, bidirectional text/attachments, group
   management, and audible two-way calls. The recommended destination is a private
   `Discut Test` server and a willing participant or second account controlled by the owner.
   This gate is pending; offline transport tests do not satisfy it.
3. **Comparable resource evidence:** repeat identical signed-in chat and call scenarios,
   with the same account/channel history and media settings. Measure whole process families
   after warmup with compilation stopped. Report failures and variance, not just a best run.
4. **Cross-platform delivery:** validate Windows and Linux on those systems, then finish
   install/update identities. Linux distro package names and internal `serein` executable
   names are inherited; current distro packages are not intended to coexist with an
   installed Serein package. Public release/update infrastructure is not configured.
5. **Further optimization:** profile actual call/video and large-account hot paths before
   considering an AppKit/SwiftUI frontend. The shared Rust app already uses native OS media
   APIs; a separate macOS frontend's additional savings have not been measured.

## Automated validation on this Mac

The complete `scripts/discut-build.sh xtask check` passed on 2026-10-06: formatting,
strict workspace Clippy, workspace tests/doc-tests, the production no-default-features
check and persistence/authentication policy checks. Cargo reported 1,167 passing test
results (including subprocess-isolation output), zero failures and 25 ignored cases.
Ignored/live cases are not counted as validated functionality. The full log is in ignored
`target/discut-check-selection.log`.

The authentication handoff's synthetic JavaScript checks and the resource sampler's
self-test also passed. Local transport tests exercise encrypted Opus voice, group membership,
audio/video delivery, reconnection/state handling and bounded parsing. They contact only
synthetic local endpoints, not the Discord service. Group UI tests verify an explicit click
is required before creation. Direct-routing regression tests verify disabled extensions
cannot leave initial connection/reconnection waiting indefinitely.

Conversation-selection tests exercise actual checkbox clicks, navigation/switcher
filtering, restoration, draft/catalog preservation, legacy preference loading and
per-account storage. Account switching waits for the latest asynchronous preference
save; save failure retains the current account and offers a retry.

The [conversation-selector screenshot](screenshots/discut-selection.png) was captured
and visually inspected using the final source's native renderer with synthetic data.
That image uses a debug demo build; it is not the build used for resource measurements.
Group controls below the pictured viewport are covered by the UI click tests. Native
accessibility automation stalled on this machine, so no live click-through is claimed.

The macOS production app was packaged successfully with voice, its icon and license
resources, then verified with `codesign --verify --strict`. It is available locally at
`dist/Discut.app`, with a ZIP at `dist/Discut-macos-arm64.zip`; neither is a notarized
public release. Windows/Linux native packaging and live-account checks remain pending.

Inherited macOS UI test assumptions were adjusted for egui's inert selection notification;
clipboard-write and external-link commands remain rejected. Removed game settings now have
exclusion tests instead of tests that expect those controls to be present.
