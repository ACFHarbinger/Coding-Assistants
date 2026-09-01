# Release 1.0.0 governance and security review

Reviewed by Codex on 2026-09-01 WEST. This is evidence for Claude, who owns
GitHub issue truth. It is **not** authorization to publish or close #199.

## Decision

The release-remediation work is approved. Publication/sign-off (#199) is
**blocked**: security disposition requires owner action, the draft release has
an empty body, and the required hardware/live acceptance evidence is not yet
complete. No dependency or workflow change was made by this review.

## Remediation review

| Item | Result | Evidence |
| --- | --- | --- |
| #200 candidate and changelog | Approved | PR #200 merged as `568a4e3`; the pre-remediation tag target `41c47cf` is an ancestor of `f8e0479`; tag-time changelog has an empty `Unreleased` section and retains the released entries under `1.0.0` dated 2026-09-01. The PR is additive relative to the old tag; no history was dropped. |
| #201 CI sidecars | Approved | PR #201 (`432cf96`) adds the same seven-package release build and staging sequence before `src-tauri` fmt/clippy/test. The tag-triggered Release workflow `33493385015` completed successfully at `f8e0479`. Future parity remains a follow-up: the commands are duplicated, not yet a shared action. |
| #203 Debian identity | Approved | PR #203 (`f8e0479`) replaces `authors = ["you"]` and the placeholder description. The downloaded release `.deb` has SHA-256 `2a9a0e998c8e2d5f51986e40dd8b4c7d22bb728d2a329b132f14b6ffc4c88e14`, version `1.0.0`, maintainer `ACFHarbinger <afonso.fernandes100@gmail.com>`, and all seven MCP sidecars. |
| Retag and draft inventory | Approved, bounded | `v1.0.0` is an annotated tag resolving to `f8e0479f9f75a888db3ecd8919879294e3001558`. GitHub lists exactly one draft `v1.0.0` release with six assets. Current API state establishes the re-cut; it cannot independently prove deletion of a historical draft. |
| `RELEASE_1.0.0_BLOCKER.md` | Accurate with one needed context note | Its resolved state, candidate, workflow run, and artifact table match the current tag/release evidence. The record predates #203's re-cut in its resolved summary, but the current bus correctly supersedes it with final candidate `f8e0479`. |

## Publication/sign-off review (#199)

The draft remains draft and has all six expected assets, but its release body is
empty. Before publication, Claude must add release notes and the known caveats:
Windows MSI/NSIS are unsigned; Android acceptance requires a real device; the
platform acceptance issues (#193--#198) require their recorded live/N/A/blocked
dispositions. Do not publish while the security items below are unresolved.

## Security-audit disposition

### Observed audit state

Security workflow `33494162468` failed. `cargo audit` scanned
`src-tauri/Cargo.lock` and found 11 vulnerabilities. `pip-audit` did **not**
scan dependencies: `uv run pip-audit` failed with `No such file or directory`.
There is no Python lockfile in the repository. Therefore a claim that
`pip-audit` is "red" is an audit-tooling failure, not a vulnerability result.
`npm-audit` passed.

Every `cargo-audit` vulnerability is **fix now**: it has a stated patched
version and release sign-off cannot accept known, fixable Rust vulnerabilities.

| Advisory | Locked package | Minimum remediation | Disposition |
| --- | --- | --- | --- |
| RUSTSEC-2026-0007 | `bytes 1.11.0` | `1.11.1` | Fix now |
| RUSTSEC-2026-0258 | `h2 0.4.13` | `0.4.16` | Fix now |
| RUSTSEC-2026-0194 | `quick-xml 0.38.4` | `0.41.0` | Fix now |
| RUSTSEC-2026-0195 | `quick-xml 0.38.4` | `0.41.0` | Fix now |
| RUSTSEC-2026-0037 | `quinn-proto 0.11.13` | `0.11.14` | Fix now |
| RUSTSEC-2026-0185 | `quinn-proto 0.11.13` | `0.11.15` | Fix now |
| RUSTSEC-2026-0099 | `rustls-webpki 0.103.8` | `0.103.12` | Fix now |
| RUSTSEC-2026-0049 | `rustls-webpki 0.103.8` | `0.103.10` | Fix now |
| RUSTSEC-2026-0104 | `rustls-webpki 0.103.8` | `0.103.13` | Fix now |
| RUSTSEC-2026-0098 | `rustls-webpki 0.103.8` | `0.103.12` | Fix now |
| RUSTSEC-2026-0009 | `time 0.3.44` | `0.3.47` | Fix now |

The 26 open Dependabot alerts are a different inventory: 9 high, 11 medium,
and 6 low. The table gives a release disposition per alert. "Accept" requires
Claude's written acceptance with the stated scope; it is not pre-approved.

| Alert | Package / path | Severity | Disposition |
| --- | --- | --- | --- |
| GHSA-wrw7-89jp-8q8g | `glib`, root `Cargo.lock` | medium | Defer post-1.0.0: GTK3 transitive upgrade needs platform regression coverage. |
| GHSA-wrw7-89jp-8q8g | `glib`, `src-tauri/Cargo.lock` | medium | Defer post-1.0.0: same tracked GTK3 migration. |
| GHSA-434x-w66g-qw3r | `bytes`, `src-tauri/Cargo.lock` | medium | Fix now: also reported by cargo-audit. |
| GHSA-r6v5-fh4h-64xc | `time`, `src-tauri/Cargo.lock` | medium | Fix now: also reported by cargo-audit. |
| GHSA-6xvm-j4wr-6v98 | `quinn-proto`, `src-tauri/Cargo.lock` | high | Fix now: remote DoS. |
| GHSA-pwjx-qhcg-rvj4 | `rustls-webpki`, `src-tauri/Cargo.lock` | medium | Fix now: TLS/CRL validation. |
| GHSA-965h-392x-2mh5 | `rustls-webpki`, `src-tauri/Cargo.lock` | low | Defer post-1.0.0 with the same Rustls update. |
| GHSA-xgp8-3hg3-c2mh | `rustls-webpki`, `src-tauri/Cargo.lock` | low | Defer post-1.0.0 with the same Rustls update. |
| GHSA-cq8v-f236-94qc | `rand 0.9`, `src-tauri/Cargo.lock` | low | Accept only with a written no-custom-logger reachability review; otherwise fix. |
| GHSA-cq8v-f236-94qc | `rand 0.8`, `src-tauri/Cargo.lock` | low | Accept only with a written no-custom-logger reachability review; otherwise fix. |
| GHSA-hppc-g8h3-xhp3 | `openssl`, `src-tauri/Cargo.lock` | high | Fix now: peer-adjacent memory disclosure. |
| GHSA-ghm9-cr32-g9qj | `openssl`, `src-tauri/Cargo.lock` | high | Fix now: caller-buffer overflow. |
| GHSA-8c75-8mhr-p7r9 | `openssl`, `src-tauri/Cargo.lock` | high | Fix now: key-wrap bounds error. |
| GHSA-xmgf-hq76-4vx2 | `openssl`, `src-tauri/Cargo.lock` | low | Defer only with written proof PEM password callbacks are unreachable; otherwise fix. |
| GHSA-pqf5-4pqq-29f5 | `openssl`, `src-tauri/Cargo.lock` | high | Fix now: short-buffer overflow. |
| GHSA-82j2-j2ch-gfr8 | `rustls-webpki`, `src-tauri/Cargo.lock` | high | Fix now: malformed-CRL DoS. |
| GHSA-xp3w-r5p5-63rr | `openssl`, `src-tauri/Cargo.lock` | high | Fix now: undefined behavior on peer data. |
| GHSA-7gmj-67g7-phm9 | `tauri`, `src-tauri/Cargo.lock` | medium | Fix now: local IPC origin-confusion boundary. |
| GHSA-xv59-967r-8726 | `openssl`, `src-tauri/Cargo.lock` | medium | Fix now: AES-KW-PAD heap overflow. |
| GHSA-phqj-4mhp-q6mq | `openssl`, `src-tauri/Cargo.lock` | medium | Fix now: AES-KW-PAD out-of-bounds write. |
| GHSA-4w2j-m93h-cj5j | `quinn-proto`, `src-tauri/Cargo.lock` | high | Fix now: remote memory exhaustion. |
| GHSA-w5hq-g745-h8pq | `uuid`, website lockfile | medium | Defer post-1.0.0: documentation build dependency; update in website maintenance PR. |
| GHSA-q8mj-m7cp-5q26 | `qs`, website lockfile | medium | Defer post-1.0.0: documentation build dependency; update in website maintenance PR. |
| GHSA-jmr9-qjv8-65gv | `extract-zip`, website lockfile | high | Fix now: no patched version; replace/remove the transitive dependency. |
| GHSA-rhfx-m35p-ff5j | `lru`, root `Cargo.lock` | low | Defer post-1.0.0: root workspace lock only; include in workspace dependency update. |
| GHSA-7gcf-g7xr-8hxj | `serde_with`, `src-tauri/Cargo.lock` | medium | Fix now: malformed input can panic serialization. |

## Required owner follow-up

1. Authorize one dependency-update PR, including regeneration of both lockfiles
   and focused regression tests; no dependency bump was authorized in this task.
2. Repair the Python audit job with a reproducible, explicitly installed audit
   tool and lockfile, then run it. Its current failure is not evidence of Python
   vulnerabilities.
3. Resolve or write the acceptance for every `Accept`/`Defer` row above, link
   the work to #199, and rerun the security workflow before sign-off.
4. Populate the draft release notes, collect the child acceptance evidence, and
   only then promote the draft and perform the post-publication smoke test.
