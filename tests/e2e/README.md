# Desktop end-to-end checks (WebDriver via tauri-driver)

Drives the **built** desktop app through the WebKitGTK webview's inspector
protocol. WebDriver talks to the webview directly, so it needs neither
compositor window focus nor synthetic OS input — it works where `xdotool` /
`spectacle` can't (a live Wayland session), and it runs under a throwaway
`Xvfb` display so a run never touches your desktop.

```
Xvfb :99  <-  tauri-driver :4444  ->  WebKitWebDriver :4445
                       |
                       +-> launches target/debug/coding-assistants
```

## One-time setup

```bash
sudo apt-get install -y webkitgtk-webdriver xvfb   # WebKitWebDriver + virtual display
cargo install tauri-driver --locked               # -> ~/.cargo/bin/tauri-driver
```

`selenium-webdriver` (the only npm dep) installs into `tests/e2e/node_modules`
on first run.

## Run

```bash
just test::e2e         # build the binary, smoke check, then run.mjs
just test::e2e-smoke   # transport proof only
```

Override paths with env vars if your layout differs: `E2E_APP_BINARY`,
`TAURI_DRIVER`, `WEBKIT_WEBDRIVER`, `E2E_DISPLAY`. `E2E_VERBOSE=1` surfaces
tauri-driver output; `E2E_NO_XVFB=1` uses the current `$DISPLAY`.

## What it covers

| Check | Reachable? |
|---|---|
| App launches, React root mounts, no blank window | ✔ |
| Top-level navigation controls render | ✔ |
| **#143** crash-recovery boundary catches a render throw, shows a reload view, leaks no stack trace | ✔ **only** when built with `VITE_E2E_CRASH_HOOK=1` (see below) |
| **#167** embedded-terminal resize frame default size | guarded — needs a mounted terminal |
| **#167** wheel scroll / focus-gated capture | ✘ needs a live PTY from a registered harness (real `claude`/`grok`/… CLIs) — verify by hand or in a harness-integration run |

### #143 forced-throw hook — the error-boundary test strategy

The app has no user-facing way to trigger a render exception, so
`src/e2eCrashProbe.tsx` provides one: a component that throws when
`window.__E2E_FORCE_RENDER_CRASH__()` is called. It is rendered from
`main.tsx` **only** when `import.meta.env.VITE_E2E_CRASH_HOOK` is set at build
time; that expression is a Vite compile-time constant, so a normal
`npm run build` folds the branch to `null` and tree-shakes the probe out
entirely (verified: the string `E2E forced render crash` is absent from
`dist/`). There is no runtime hook in a shipped build.

To exercise #143:

```bash
VITE_E2E_CRASH_HOOK=1 npx tauri build --debug --no-bundle
( cd tests/e2e && node run.mjs )
```

`run.mjs` then calls the hook, waits for `[role=alert]`, and asserts the
recovery copy plus a single "Reload" button and the absence of a
stack-trace-shaped string.
