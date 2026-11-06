# #199 §3 security-audit disposition

Prepared by Codex on 2026-09-01, completing DeepSeek's assigned audit review
on the repository owner's explicit authorization. This is a disposition for
release governance; it does not authorize dependency changes or publication.

## Decision

Keep #199 blocked. The latest completed Security Audit reviewed here is run
`33504647997` at `24fb9e3`: `npm-audit` passed, `cargo-audit` failed, and
`pip-audit` failed before it audited anything.

The inventories must not be conflated:

- `cargo audit` found **11 vulnerabilities** plus **26 allowed warnings**.
- Dependabot has **26 open alerts**: 9 high, 11 medium, and 6 low (23 Rust,
  3 npm). Those are not “26 cargo-audit vulnerabilities.”
- `pip-audit` produced **no vulnerability result**. `uv sync` succeeded, but
  `uv run pip-audit` could not spawn `pip-audit` because the executable is not
  declared/installed.

## Cargo vulnerabilities: fix before release

All 11 have patched versions and therefore have a **fix now** disposition:

| Advisory | Locked crate | Minimum patched version | Disposition |
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

## Cargo allowed warnings: per-advisory disposition

“Accept with reason” below is a recommendation for the release owner to record,
not an acceptance made on the owner's behalf. “Defer” requires a tracked
post-1.0 migration item and a written release exception.

| Advisory | Locked crate | Kind | Disposition and reason |
| --- | --- | --- | --- |
| RUSTSEC-2024-0413 | `atk 0.18.2` | unmaintained | Defer: GTK3/Tauri Linux stack migration; regression-test tray, dialogs, and WebKit together. |
| RUSTSEC-2024-0416 | `atk-sys 0.18.2` | unmaintained | Defer: same GTK3/Tauri migration. |
| RUSTSEC-2025-0012 | `backoff 0.4.0` | unmaintained | Defer: transitive through `async-openai 0.26`; replace or upgrade that client as one tested change. |
| RUSTSEC-2021-0141 | `dotenv 0.15.0` | unmaintained | Fix now: direct dependency; replace with maintained `dotenvy` or remove if unused. |
| RUSTSEC-2025-0057 | `fxhash 0.2.1` | unmaintained | Defer: transitive through `selectors` in the WebKit stack; resolve with that upstream stack. |
| RUSTSEC-2024-0412 | `gdk 0.18.2` | unmaintained | Defer: GTK3/Tauri Linux stack migration. |
| RUSTSEC-2024-0418 | `gdk-sys 0.18.2` | unmaintained | Defer: GTK3/Tauri Linux stack migration. |
| RUSTSEC-2024-0411 | `gdkwayland-sys 0.18.2` | unmaintained | Defer: GTK3/Tauri Linux stack migration. |
| RUSTSEC-2024-0417 | `gdkx11 0.18.2` | unmaintained | Defer: GTK3/Tauri Linux stack migration. |
| RUSTSEC-2024-0414 | `gdkx11-sys 0.18.2` | unmaintained | Defer: GTK3/Tauri Linux stack migration. |
| RUSTSEC-2024-0415 | `gtk 0.18.2` | unmaintained | Defer: GTK3/Tauri Linux stack migration. |
| RUSTSEC-2024-0420 | `gtk-sys 0.18.2` | unmaintained | Defer: GTK3/Tauri Linux stack migration. |
| RUSTSEC-2024-0419 | `gtk3-macros 0.18.2` | unmaintained | Defer: GTK3/Tauri Linux stack migration. |
| RUSTSEC-2024-0384 | `instant 0.1.13` | unmaintained | Defer: transitive through `backoff`/`async-openai`; resolve with that client migration. |
| RUSTSEC-2024-0370 | `proc-macro-error 1.0.4` | unmaintained | Defer: build-time dependency of GTK macros; resolve with GTK/Tauri migration. |
| RUSTSEC-2025-0081 | `unic-char-property 0.9.0` | unmaintained | Defer: transitive through `unic-ucd-ident`/`urlpattern`/`tauri-utils`; resolve through Tauri upstream. |
| RUSTSEC-2025-0075 | `unic-char-range 0.9.0` | unmaintained | Defer: same Tauri `urlpattern` chain. |
| RUSTSEC-2025-0080 | `unic-common 0.9.0` | unmaintained | Defer: same Tauri `urlpattern` chain. |
| RUSTSEC-2025-0100 | `unic-ucd-ident 0.9.0` | unmaintained | Defer: same Tauri `urlpattern` chain. |
| RUSTSEC-2025-0098 | `unic-ucd-version 0.9.0` | unmaintained | Defer: same Tauri `urlpattern` chain. |
| RUSTSEC-2026-0190 | `anyhow 1.0.100` | unsound | Fix now: lock to the patched release and rerun the Rust suite. |
| RUSTSEC-2026-0221 | `event-listener 5.4.1` | unsound | Fix now: lock to the patched release and rerun async/network tests. |
| RUSTSEC-2024-0429 | `glib 0.18.5` | unsound | Defer only with written exception: the patched line requires the GTK/Tauri migration above. |
| RUSTSEC-2026-0097 | `rand 0.7.3` | unsound | Accept with reason: no custom `log::Log` installation or `rand::rng()`/`thread_rng()` call exists in repository Rust code; old build-time `phf` chain. |
| RUSTSEC-2026-0097 | `rand 0.8.5` | unsound | Accept with the same no-custom-logger reachability finding, or update the lock while upgrading `async-openai`. |
| RUSTSEC-2026-0097 | `rand 0.9.2` | unsound | Fix now with the `quinn-proto` remediation; do not retain the stale lock solely on the reachability argument. |

## Dependabot inventory

The 26 open alerts remain 9 high / 11 medium / 6 low. Release-blocking “fix
now” alerts are the patched Rust vulnerability families (`bytes`, `time`,
`quinn-proto`, `rustls-webpki`, OpenSSL, Tauri, and `serde_with`) plus the
high-severity website `extract-zip` alert (replace/remove because no patched
version is listed). The GTK `glib` pair, root-lock `lru`, and documentation-only
`uuid`/`qs` alerts may be deferred only with written scope and follow-up. The
two `rand` alerts require the custom-logger reachability rationale above if not
fixed. The full alert-by-alert table is retained in
`.agent/reports/codex/release_1.0.0_governance_security_review_20260901.md`.

## Required closure evidence

1. Apply the authorized lock/dependency remediation and rerun `cargo audit`.
2. Declare `pip-audit` reproducibly (for example as a uv dev dependency) and
   audit a committed, reproducible Python dependency set. Until then its result
   is “tool missing,” not pass and not vulnerabilities found.
3. Record owner acceptance and linked follow-up for every Accept/Defer row.
4. Rerun the whole Security Audit; attach the run URL and final inventories to
   #199 before release sign-off.
