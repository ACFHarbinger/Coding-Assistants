# Benchmarks

> **TODO:** Fill in with real benchmark results once the project has measurable workloads.

| Target | Tool | Location |
| --- | --- | --- |
| `src-tauri/` (Rust backend) | criterion | `src-tauri/benches/` |

Run the suite with `just bench`. CI runs benchmarks on pushes to `main` that
touch `src-tauri/benches/` — see [`.github/workflows/benchmark.yml`](../.github/workflows/benchmark.yml).
