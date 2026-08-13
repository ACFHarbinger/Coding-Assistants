# Troubleshooting

[![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://v2.tauri.app)

Common issues and their solutions when developing or running Coding Assistants.

---

## Build Issues

### `npm run tauri dev` fails to compile Rust

**Symptom**: Cargo compilation errors when starting the app.

**Solutions**:

1. Ensure Rust is up to date:
   ```bash
   rustup update stable
   ```

2. Verify Tauri system dependencies are installed. On Linux:
   ```bash
   sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
     libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
   ```

3. Clean and rebuild:
   ```bash
   cd src-tauri && cargo clean
   npm run tauri dev
   ```

### `npm install` fails

**Symptom**: Dependency resolution or download errors.

**Solutions**:

1. Clear npm cache:
   ```bash
   npm cache clean --force
   rm -rf node_modules package-lock.json
   npm install
   ```

2. Verify Node.js version (LTS recommended):
   ```bash
   node --version  # Should be 20+
   ```

### TypeScript compilation errors

**Symptom**: `tsc` reports type errors during `npm run build`.

**Solutions**:

1. Verify TypeScript types are installed:
   ```bash
   npm install -D @types/react @types/react-dom
   ```

2. Check for version mismatches between React and its type definitions in `package.json`.

---

## Runtime Issues

### App window is blank or shows white screen

**Symptom**: Tauri window opens but shows nothing.

**Solutions**:

1. Check the terminal for frontend compilation errors
2. Verify the Vite dev server is running on the configured development port
   (1420 by default):
   ```bash
   curl "localhost:${VITE_PORT:-1420}"
   ```
3. Open DevTools (right-click -> Inspect in debug builds) and check the console for JavaScript errors
4. Ensure `devUrl` in `tauri.conf.json` matches the Vite port:
   ```json
   "devUrl": "<local Vite server URL>"
   ```

### `invoke` calls fail with "command not found"

**Symptom**: Frontend errors like `invoke("command_name") failed: command command_name not found`.

**Solutions**:

1. Verify the command is registered in the Tauri builder in `lib.rs`:
   ```rust
   .invoke_handler(tauri::generate_handler![
       run_agent_task,
       submit_user_input,
       // ...ensure your command is listed here
   ])
   ```

2. Ensure the command name in `invoke()` exactly matches the Rust function name (underscores, not camelCase)

3. Restart `npm run tauri dev` -- the backend must recompile to pick up new commands

### Serialization errors on IPC calls

**Symptom**: `invoke` calls fail with serialization or deserialization errors.

**Solutions**:

1. Ensure TypeScript types match Rust struct fields exactly (field names, types, optionality)
2. Check that Rust structs derive both `Serialize` and `Deserialize`:
   ```rust
   #[derive(Serialize, Deserialize)]
   pub struct MyPayload { ... }
   ```
3. Optional fields in Rust (`Option<T>`) should use `?` in TypeScript or be passed as `null`

---

## LLM Provider Issues

### "No models found" for Ollama

**Symptom**: Ollama provider shows no available models.

**Solutions**:

1. Verify Ollama is running:
   ```bash
   ollama list
   ```

2. If no models appear, pull one:
   ```bash
   ollama pull llama3.2
   ```

3. Ensure `ollama` is on your `$PATH`:
   ```bash
   which ollama
   ```

### OpenCode provider not working

**Symptom**: OpenCode Zen provider fails or shows no models.

**Solutions**:

1. Verify `opencode` CLI is installed and on `$PATH`:
   ```bash
   opencode --version
   opencode models
   ```

2. Check that `opencode run` works from the terminal:
   ```bash
   echo "test" | opencode run <model-name>
   ```

### API key not recognized

**Symptom**: Cloud providers (OpenAI, Google, Anthropic) return authentication errors.

**Solutions**:

1. Check that `env/.env` exists and contains valid keys:
   ```bash
   cat env/.env
   ```

2. Ensure keys are properly formatted (no trailing whitespace, correct quotes):
   ```
   OPENAI_API_KEY='sk-...'
   GOOGLE_GENAI_API_KEY='...'
   ```

3. Restart the app after modifying `.env` -- the `dotenv` crate loads at startup

### Agent hangs or produces no output

**Symptom**: Task appears to be running but no events appear.

**Solutions**:

1. Check the terminal for Rust backend errors or panics
2. Verify the selected model exists and the provider is accessible
3. Try a simpler task to isolate whether it's a model or configuration issue
4. Use the Cancel button and try again

---

## Remote Control Issues

### Android app cannot connect

**Symptom**: Android app shows "Connection failed" when trying to connect to the desktop app.

**Solutions**:

1. Verify the TCP server is started (green status in Remote Control section)
2. Both devices must be on the **same local network** (same WiFi)
3. Check the displayed IP address matches what you enter in the Android app
4. Verify port 5555 is not blocked by a firewall:
   ```bash
   # Linux: check if port is listening
   ss -tlnp | grep 5555
   ```
5. On Linux, allow the port through the firewall:
   ```bash
   sudo ufw allow 5555/tcp
   ```

### TCP server won't start

**Symptom**: Clicking "Start Server" fails or shows an error.

**Solutions**:

1. Check if port 5555 is already in use:
   ```bash
   ss -tlnp | grep 5555
   ```

2. Kill any existing process on that port:
   ```bash
   kill $(lsof -t -i:5555)
   ```

3. Verify your network interface is up and has an IP address:
   ```bash
   ip addr show
   ```

---

## Workspace Issues

### Directory picker doesn't open

**Symptom**: Clicking the workspace folder icon does nothing.

**Solutions**:

1. Verify `dialog:default` permission is granted in `src-tauri/capabilities/default.json`
2. On Linux, ensure the file dialog backend is installed:
   ```bash
   sudo apt install zenity  # or kdialog for KDE
   ```

### Agent resources not loading

**Symptom**: Prompt, rule, and workflow dropdowns are empty.

**Solutions**:

1. Verify the workspace directory contains a `.agent/` folder with the expected structure:
   ```
   .agent/
   ├── prompts/
   │   └── *.md
   ├── rules/
   │   └── *.md
   └── workflows/
       └── *.md
   ```

2. Check file permissions -- the app needs read access to these files

3. Select the workspace directory again (the resource scan happens on directory selection)

---

## Performance Issues

### UI feels sluggish during task execution

**Symptom**: The interface becomes unresponsive when agents are running.

**Solutions**:

1. This typically happens when many events accumulate. The event log grows unbounded during long tasks.
2. Large agent responses can cause render bottlenecks. This is a known area for improvement (see [ROADMAP.md](moon/ROADMAP.md)).
3. Avoid running agents with extremely verbose output settings.

### High memory usage

**Symptom**: Application memory grows significantly during use.

**Solutions**:

1. Agent events are stored in React state and accumulate over time. Restarting the app clears them.
2. Large workspaces with many files in `.agent/` may use more memory during resource scanning.
3. Multiple concurrent TCP connections can increase memory usage from broadcast channel buffers.

---

## Platform-Specific Issues

### Linux: WebView rendering issues

Ensure WebKitGTK is up to date:
```bash
sudo apt update && sudo apt upgrade libwebkit2gtk-4.1-dev
```

### macOS: "App is damaged" warning

For unsigned development builds, remove the quarantine attribute:
```bash
xattr -cr /path/to/Coding\ Assistants.app
```

### Windows: WebView2 missing

Install the Microsoft Edge WebView2 Runtime from Microsoft's official download
page. Search for **Microsoft Edge WebView2 Runtime** if the vendor's download
URL redirects in your browser or network environment.

---

## Getting Help

If your issue isn't covered here:

1. Check the terminal output for error messages from both the Vite dev server and the Rust backend
2. Open browser DevTools in the Tauri window for JavaScript errors
3. Search existing GitHub issues
4. Open a new issue with:
   - Your OS and version
   - Node.js, Rust, and Tauri CLI versions
   - Steps to reproduce
   - Terminal and console error output
