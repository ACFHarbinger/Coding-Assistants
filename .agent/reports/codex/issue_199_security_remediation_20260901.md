# #199 security remediation evidence

Prepared by Codex on 2026-09-01, completing the work reassigned from DeepSeek
with the repository owner's explicit authorization.

## Remediation

The Rust workspace had two committed lockfiles. Builds resolve the root
workspace `Cargo.lock`, which already contained patched versions, while the
Security Audit ran from `src-tauri` and scanned a stale, unused lock containing
all 11 vulnerable versions. This change removes `src-tauri/Cargo.lock` and
runs `cargo audit` against the canonical root workspace lock.

The final locked versions are:

| Crate | Required minimum | Final lock |
| --- | ---: | ---: |
| `bytes` | 1.11.1 | 1.12.1 |
| `h2` | 0.4.16 | 0.4.19 |
| `quick-xml` | 0.41.0 | 0.41.0 |
| `quinn-proto` | 0.11.15 | 0.11.17 |
| `rustls-webpki` | 0.103.13 | 0.103.15 |
| `time` | 0.3.47 | 0.3.55 |
| `anyhow` | patched | 1.0.104 |
| `event-listener` | patched | 5.4.2 |

The direct unmaintained `dotenv 0.15.0` dependency is replaced by maintained
`dotenvy 0.15.7`. No deferred GTK3/Tauri dependency was changed.

For Python, `pip-audit 2.10.1` is now a uv development dependency and
`git/uv.lock` commits the complete 50-package environment. CI uses
`uv sync --locked --dev` and `uv run --locked pip-audit`.

## Final local audit inventory

- `cargo audit` (cargo-audit 0.22.2): **0 vulnerabilities**, 23 allowed
  warnings (19 unmaintained, 3 unsound, 1 yanked). These are the previously
  deferred stack advisories plus `paste`, two `lru` advisories, and yanked
  `chacha20`; this remediation deliberately does not expand into that deferred
  dependency migration.
- `npm audit --audit-level=high`: **0 vulnerabilities**.
- `uv run --locked pip-audit`: **no known vulnerabilities** among the locked
  third-party packages. The local unpublished project itself is correctly
  reported as not present on PyPI and skipped.

## Build and test evidence

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (all tests passed; one explicitly ignored manual
  smoke test)
- `uv sync --locked --dev`
- `uv run --locked pip-audit`
- `npm audit --audit-level=high`

The pull request and final GitHub Security Audit run are linked from issue
#199. This branch is ready for Claude/owner review and is intentionally not
merged.
