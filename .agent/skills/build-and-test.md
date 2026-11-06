# Skill: Build and Test Everything

Run the full build + test cycle across the frontend, backend, and Android companion app.

```bash
npm run build          # frontend build (Vite)
(cd src-tauri && cargo test && cargo clippy)   # Rust backend
npm test                # Vitest, if configured
(cd android && ./gradlew ktlintCheck test)     # Android companion app
just docs               # builds the MkDocs documentation site
```

Use this before opening a PR, or whenever asked to "make sure everything still works."
Report which target(s) failed and the first failing assertion/error, not just "tests failed."
