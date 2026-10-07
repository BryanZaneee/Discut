# Discut

An experimental native Rust Discord client focused on conversations, groups and calls.

<img src="docs/screenshots/discut-chat.png" alt="Discut native server chat with synthetic attachments and group controls" width="1000" />

*Native offline fixture with populated history; no live Discord session or messages.*

<img src="docs/screenshots/discut-call.png" alt="Discut native call layout using synthetic participants" width="1000" />

*Synthetic call layout only; no microphone capture, network call or audio-quality validation.*

[More screenshots and reproduction commands](docs/screenshots/README.md).

Discut is an open-source fork of [Serein](https://github.com/ViceVerse-cz/Serein), using egui/wgpu
and its existing Discord transport and media stack. It is unofficial. Live account
interoperability is still awaiting owner-controlled verification; a synthetic preview
is not proof of working Discord calls.

## Measured resource use

Mac observation on October 6, 2026: Apple M3 Pro, 18 GiB RAM, macOS 27.0.1;
Discut optimized build and installed Discord 0.0.413. RAM/CPU are a 60-second sample
of both running process families. Disk sizes were measured separately that day.

| Metric | Official Discord | Discut |
| --- | ---: | ---: |
| Average physical memory footprint | 620.23 MiB | **143.22 MiB** |
| Peak footprint during this sample | 621.45 MiB | **143.22 MiB** |
| Average CPU (100% = one core) | 0.02346% | 0.00093% |
| Processes | 7 | 1 |
| Installed app bundle on disk | 480.79 MiB | **68.29 MiB** |
| Discut compressed Mac package | Not measured | 43.80 MiB |

**About 77% lower memory footprint in these observed states, and 86% smaller app
bundle.** This is a comparison of different clients, not a controlled before/after
optimization benchmark. Both were effectively idle; account, channel, call state and
visibility were not independently matched. Tiny idle CPU values do not establish
battery savings. These numbers do not yet establish savings during equivalent live
chats or calls. [Method, limitations and raw samples](docs/discut-performance.md).

## Build and run

Install the pinned Rust toolchain and CMake. macOS needs Xcode command-line tools.
See [upstream platform prerequisites](README.upstream.md#quick-start) for Windows/Linux.

```sh
git clone https://github.com/BryanZaneee/Discut.git
cd Discut
scripts/discut-build.sh xtask package
open dist/Discut.app                       # macOS
```

The build wrapper also recognizes an optional local CMake installation in `target/build-tools`.
The package is locally ad-hoc signed, not a notarized public release.
Windows and Linux remain source targets pending native validation.

For a synthetic preview that does not sign in or contact Discord:

```sh
scripts/discut-build.sh build --release --locked --no-default-features --features demo -p serein
target/release/serein --demo --demo-group-dm
```

The internal Cargo package/executable remains `serein`. The app display name,
credential namespace and local data directory are separate from upstream Serein.
Sign in only inside the app; never paste tokens into a terminal or chat.

## Usage and validation

- Sign in with an existing Discord account; service-provided servers, DMs and groups
  populate automatically. History loads as you open conversations and scroll.
- **Choose conversations** in the sidebar or Chat settings controls which servers,
  DMs and groups appear, with everything included by default. Choices persist per
  account on this device and can be restored. They do not change memberships or mute
  notifications; direct links and notifications can still open hidden conversations.
- Server/channel navigation, DMs, group conversations, messages, attachments and drafts.
- Native voice, camera and screen-sharing implementation inherited from Serein.
- Added group creation and member management with bounded requests and stale-response checks.
- No Nitro/shop, Quests, embedded Activities or game overlays. Optional extension hosting,
  game scanning and Spotify presence workers are disabled, as is the upstream updater.

Read the [MVP acceptance checklist](docs/discut-mvp.md) for implemented versus verified
scope. The [measurement report](docs/discut-performance.md) contains actual observations;
[benchmark instructions](scripts/discut-benchmark/README.md) explain process-tree memory
and CPU sampling. Live text and call comparisons remain required.

```sh
scripts/discut-build.sh xtask check
node tests/login-handoff.cjs
python3 scripts/discut-benchmark/benchmark.py --self-test
```

## Architecture

Rust state/reconciliation and bounded local SQLite caches are separate from the native
UI, Discord REST/gateway connections and audio/video workers. There is no Electron
runtime; a temporary authentication webview handles sign-in. See
[compatibility](docs/discord-compatibility.md) and [authentication](docs/authentication.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for small, focused changes, optional extras,
review expectations and checks. [ROADMAP.md](ROADMAP.md) tracks acceptance goals;
[CHANGELOG.md](CHANGELOG.md) records changes. Use
[Issues](https://github.com/BryanZaneee/Discut/issues) for bugs and proposals, and
[SECURITY.md](SECURITY.md) for private vulnerability reporting.

Keep credentials, private chats, raw diagnostics, `target/` and `dist/` out of git.
Inherited publishing workflows are disabled pending a Discut-specific release pipeline.

## License and attribution

[MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), with original copyright notices
preserved. See [third-party notices](THIRD_PARTY_NOTICES.md). The current icon and
bundled artwork are inherited from Serein. [Original upstream README](README.upstream.md)
contains upstream documentation, screenshots and claims, not Discut validation results.
