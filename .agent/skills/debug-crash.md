# Skill: Debug a Crash

1. Get a reliable repro command and the full crash output (stack trace, exit code, panic message).
2. For the Rust backend (`src-tauri/`), rebuild with debug symbols and reproduce under `rust-gdb`/`lldb`, or re-run with `RUST_BACKTRACE=1`.
3. For the React/TypeScript frontend (`src/`), capture the full browser/devtools console output — do not summarize it.
4. For the Android companion app (`android/`), capture the full `adb logcat` output around the crash.
5. Bisect via `git bisect` if the crash is a regression against a known-good commit.
6. Once fixed, add a regression test that would have caught it, and note the root cause in the commit message.
