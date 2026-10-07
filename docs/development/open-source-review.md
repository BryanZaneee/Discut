# Open-source readiness review

Reviewed October 7, 2026. Scope: contributor entry points, inherited automation,
existing runtime/resource findings and release readiness. This is a bounded source
review, not a complete security audit or proof of live Discord interoperability.

| Finding | Evidence | Disposition |
|---|---|---|
| Contributor docs and issue links still directed users to Serein, including an Updates settings page disabled in Discut | Prior CONTRIBUTING.md and issue forms | Fixed in this setup: project-specific guidance and neutral redacted evidence fields |
| Inherited publishing workflows could publish catalogs/releases or use Serein destinations | release.yml, package-repositories.yml, extension-catalog.yml | Moved outside Actions discovery; retained in upstream-workflows as reference |
| Linux/Nix packaging retains upstream identities | nix.yml expects cz.viceverse.serein.desktop and Serein.app; internal/distro names are inherited | D-06; do not claim independent cross-platform release readiness |
| Public release scripts retain upstream assumptions | .github/release and packaging scripts | D-07; publishing workflows stay disabled until audited/adapted |
| Live sign-in completeness, text/group operations and audible calls lack recorded owner acceptance | docs/discut-mvp.md | D-01/D-02 validation gaps, not confirmed failures |
| Memory comparison uses unmatched running states | docs/discut-evidence/discut-current-session.json | README labels the observation; D-03 controls workloads before general savings claims |
| Multiple independent media pools can retain substantial decoded data | crates/ui/src/avatars.rs and avatars/media.rs | D-05 proposed optimization; limits are not current allocation measurements |
| Spaces correction has a bitmask regression test but no interactive desktop-switching pass | crates/platform/src/window.rs | D-04, retain pending status |

## Publication boundaries

Publish source, reviewed fixtures and redacted aggregate process evidence only. Do not
publish target/, dist/, live account data, credentials, raw system profiles or private
chat screenshots. Preserve original authorship, licenses, vendored notices and history.
The owner designated BryanZaneee/Discut as the public destination. Never push to upstream.

## Hosted setup

Use the repository's Issues for D-01 through D-08. Keep ROADMAP.md as the concise status
index and CHANGELOG.md for user-visible changes. Prefer squash merging and deleting
merged branches. Enable private vulnerability reporting if available. Confirm hosted
settings after applying them; do not require CI statuses until their actual names and
successful runs are established. Do not invent a CODEOWNERS identity or review service.

CI still contains inherited cross-platform checks and is not presumed green. Local
Rust checks passed before this documentation pass; this pass validates docs/templates
and repository wiring. Any initial hosted CI failure should become a concrete follow-up,
not be hidden by removing the check.

## Confirmed hosted state

Source is public at [BryanZaneee/Discut](https://github.com/BryanZaneee/Discut), with
original Git ancestry preserved. Eight issues are linked from the roadmap and grouped
into three goal milestones. Issues are enabled and the wiki is disabled. Squash merging
and deletion of merged branches are enabled; merge commits/rebase merges are disabled.
GitHub secret scanning, push protection and private vulnerability reporting are enabled.
No binary release or signing secret was created.

Initial publication checks passed: local doc links, issue-form YAML, shell syntax, skill
frontmatter, focused credential-pattern scan of fork changes, and native screenshot
reproduction/visual review. The pattern scan is not a full security audit. The prior
1,168-result Rust check applies to the unchanged runtime source; no new full Rust
suite was needed for these documentation/template changes. Hosted CI is not yet
verified and no status check has been made mandatory.
