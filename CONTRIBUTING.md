# Contributing

Discut is a small project built around a simple goal: a fast, lightweight Discord
client that keeps normal conversations, groups and calls. Contributions are welcome;
small changes with clear evidence are easiest to review.

## Before writing code

For anything beyond a small fix, open an issue describing the problem, the proposed
change and why it belongs in Discut. Check [ROADMAP.md](ROADMAP.md) first. Discuss
changes to authentication, protocol behavior, dependencies or resource budgets before
building them.

## New features: small, optional and findable

Extras that change how the app looks or behaves should be:

- **Minimal:** the smallest useful version, in the app's existing style.
- **Optional and off by default:** no new background work or interface clutter for
  someone who never enables the feature.
- **Findable:** a clear Settings control, with onboarding guidance for major additions.

Bug fixes, accessibility, reliable sign-in, existing chats and groups, notifications,
and working calls are core behavior. They do not need an opt-in switch. Existing
conversations should appear by default; local visibility choices remain optional.
Nitro commerce, the shop, Quests, Activities and game overlays are outside this project.

## Where things are tracked

- [ROADMAP.md](ROADMAP.md) records goals, their source, status and acceptance criteria.
- [The initial review](docs/development/open-source-review.md) records confirmed gaps
  and issue-ready follow-ups. A gap is not automatically a confirmed runtime bug.
- [CHANGELOG.md](CHANGELOG.md) records user-visible changes under **Unreleased**.
  Update the relevant roadmap entry when a change lands; preserve its history.

## What tends to get merged

- **Small, focused changes.** One problem per pull request, with a useful reproduction.
- **Existing tools first.** Prefer Rust's standard library, native platform APIs and
  dependencies already in the project. A new dependency needs a concrete reason,
  license review and an explanation of its build/runtime cost.
- **Matches the existing style.** Read nearby code first. Comments explain why.
  Handle fallible network, file and credential operations explicitly; do not add
  panicking unwraps on untrusted data.
- **Bounded resource use.** Bound queues and caches by bytes and item counts. Keep
  expensive work off rendering/audio callbacks. Preserve drafts and reconnect state.
- **Builds clean.** Use the pinned toolchain and lockfile. Run
  `scripts/discut-build.sh xtask check` with no new warnings from your changes.

See [validation commands](docs/development/validation.md) for authentication,
licenses, fuzzing and workload-specific checks. Use a separate target directory
for each worktree.

## What doesn't

- Rewrites of working code for style alone or speculative abstractions.
- Telemetry, analytics, a hidden backend, or undisclosed new network destinations.
- Replacing the native Rust client with an Electron or embedded Discord web client.
- Extracting credentials from other apps, bypassing service challenges or silently
  sending messages. Live tests use an owner-controlled account and private test space.
- Real credentials, private conversations, signed attachment URLs or unreviewed logs
  in code, tests, screenshots or issues. Use obviously synthetic fixtures. See
  [SECURITY.md](SECURITY.md) before reporting a vulnerability.
- Performance claims from unlike workloads, or claims that offline tests prove live calls.

## Pull requests and review

Use a short-lived branch and target the repository's default branch. Keep commits
focused, with titles such as `fix(macos): restore desktop movement`. No develop branch
or release tooling is required to contribute. Use the PR template, describe what
changed and why, and list checks actually run and anything still unverified.

UI changes need reviewed synthetic screenshots. Runtime changes need appropriate
measurements with the revision, build, workload, platform and metric recorded. Docs-only
changes do not need an app rebuild. Never include private account screenshots.

Maintainers review contributions when available. There is no guaranteed response time
or automated-review service; a polite follow-up after a couple of weeks is welcome.
Contributions use the project's MIT OR Apache-2.0 licensing; preserve upstream notices.

## Reporting a bug

Open an issue with what you did, what you expected, what happened, your Discut version
or commit, OS/version, and whether you were using a live account or an offline fixture.
For performance reports, include the workload, warmup, sampling duration, process set
and memory metric. [The sampler](scripts/discut-benchmark/README.md) is available on Mac.
Review and redact attachments yourself. Do not assume there is a universal crash-log
path, and do not post security reports in public issues.
