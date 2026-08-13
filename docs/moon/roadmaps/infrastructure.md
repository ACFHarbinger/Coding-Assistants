# Infrastructure and Documentation Roadmap

Keep only infrastructure with a current local or prototype use.

| # | Capability | Status |
| --- | --- | --- |
| I1 | `infra/docker/` documentation-site stack | ✅ Done |
| I2 | `infra/terraform/` for future cloud synchronization prototypes | 📋 Pending |
| I3 | `infra/ansible/` for reproducible host setup | 📋 Pending |
| I4 | Firebase and Supabase authentication/private-storage prototypes | 📋 Pending · after Google Drive, Firebase then Supabase implement the shared encrypted-blob and trusted-identity contracts in [`cloud_sync.md`](cloud_sync.md); Drive remains the first replica path |
| I5 | Remove obsolete Kubernetes, Helm, serverless, AWS, Azure Pipelines, WordPress, Webpack, Nginx, and proxy scaffolding | 📋 Pending |
| I6 | Keep research and reports separate from active implementation roadmaps | 📋 Pending |
| I7 | Rename crate/package `tauri-app`/`tauri_app_lib` → `coding-assistants`/`ca` (`src-tauri/Cargo.toml`, root `package.json`, `tauri.conf.json`, capability configs, lockfiles) — owner-confirmed 2026-08-10; dropped from the roadmap during the capability-file restructure, re-added here (Claude verification pass) | 📋 Pending |
| I8 | Keep Rust source units in the Hub and CLI crates bounded to 500 lines, organized by responsibility, without changing their public API/CLI contracts | ✅ Done · 2026-08-13 |
