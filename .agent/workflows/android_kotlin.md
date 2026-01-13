---
description: Procedures for working with the Android Remote Launcher app.
---

The Android app is located in the `android/` directory. It is built using modern Android standards:
- **UI**: Jetpack Compose with Material 3.
- **Networking**: Ktor for asynchronous TCP communication.
- **Serialization**: `kotlinx-serialization` for JSON protocol alignment.
- **Architecture**: MVVM using `ViewModel` and `MutableSharedFlow`.

### Core Files
- `android/app/src/main/kotlin/com/example/remotelauncher/`: Core source code.
- `Models.kt`: Defines the shared JSON protocol used with the PC backend.
- `TcpClient.kt`: Handles the raw socket communication and event broadcasting.
- `MainViewModel.kt`: Manages UI state and connects the client to the Compose views.

### Working on the App
1. **Adding Features**:
   - Update `Models.kt` if the TCP protocol changes.
   - Update `TcpClient.kt` if new networking capabilities are needed.
   - Implement UI changes in `MainActivity.kt` or split into new Compose files.

2. **Common Tasks**:
   - **Build**: Run `./gradlew assembleDebug` in the `android/` directory (ensure local Android SDK/Gradle setup).
   - **Protocol Sync**: Ensure any changes to `src-tauri/src/tcp_server.rs` are mirrored in `Models.kt`.