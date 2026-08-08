# Dependency Policy

1. **Prefer the standard library** before adding a new dependency.
2. **Pin exact or compatible-release versions** in `package.json`/`Cargo.toml`/Gradle manifests; never depend on a floating `latest`.
3. **One dependency, one purpose.** Don't add a second library that overlaps an existing one's functionality without removing the old one.
4. **License check.** New dependencies must use a license compatible with this repository's [AGPL-3.0 license](../LICENSE) (GPL-compatible, MIT, Apache-2.0, BSD are fine; proprietary or source-available-only licenses are not).
5. **Security.** Dependabot and the [security workflow](../.github/workflows/security.yml) run `cargo audit`/`pip-audit`/`npm audit` automatically; high-severity findings block merge.
6. **Major version bumps** get a dedicated PR with a changelog entry, reviewed separately from feature work.
