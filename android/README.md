# Coding Assistants - Android Remote Control

This Android app allows you to remotely control the Coding Assistants PC application over WiFi.

## Features

- Connect to your PC over local WiFi network
- Select LLM models and configure agent roles
- Submit tasks for execution
- Monitor task progress in real-time

## Requirements

- Android 7.0 (API 24) or higher
- PC and Android device on the same local network
- Coding Assistants PC app running with TCP server enabled

## Building the App

### Prerequisites

- Android Studio (latest version recommended)
- JDK 17 or higher

### Build Steps

1. Open Android Studio
2. Select "Open an Existing Project"
3. Navigate to the `android` directory
4. Wait for Gradle sync to complete
5. Click "Build" → "Build Bundle(s) / APK(s)" → "Build APK(s)"

The APK will be generated in `app/build/outputs/apk/debug/app-debug.apk`

## Usage

### 1. Start the PC Server

1. Launch the Coding Assistants app on your PC
2. Navigate to the "Remote Control" section
3. Click "Start Server"
4. Note the IP address displayed (e.g., `192.168.1.100:5555`)

### 2. Connect from Android

1. Launch the Remote Launcher app on your Android device
2. Enter the IP address from the PC app
3. Tap "Connect"

### 3. Configure Agents

After connecting, you'll see the model selection screen where you can:
- View available LLM models from all configured providers
- Add or remove agent roles
- Select provider and model for each role
- The default configuration includes Planner, Developer, and Reviewer roles

### 4. Execute Tasks

1. Tap "Next" to proceed to the task execution screen
2. Enter your task description
3. Tap "Launch Sequence" to start the task
4. Monitor progress on the PC app
5. The Android app will show status updates

## Network Requirements

- Both devices must be on the same local network
- Port 5555 must be accessible (no firewall blocking)
- For security, the connection is limited to local network only

## Troubleshooting

### Connection Failed

- Verify both devices are on the same WiFi network
- Check the IP address is correct
- Ensure the PC server is running
- Check firewall settings on the PC

### Models Not Loading

- Ensure you have OpenCode CLI installed on the PC
- Check that LLM API keys are configured correctly
- Verify internet connection for cloud providers

### Task Not Starting

- Check the PC app for error messages
- Ensure the workspace directory exists
- Verify MCP configuration is valid

## Protocol

The app uses a JSON-based TCP protocol on port 5555. Messages are newline-delimited JSON objects.

### Request Types

- `GetModels` - Fetch available LLM models
- `StartTask` - Start a task with agent configuration
- `CancelTask` - Cancel the running task
- `SubmitInput` - Submit user input when agent asks
- `GetStatus` - Get current task status

### Response Types

- `ModelsList` - List of available models
- `TaskStarted` - Confirmation that task started
- `TaskEvent` - Real-time agent events (thoughts, responses)
- `TaskComplete` - Task finished with result
- `Status` - Status update
- `Error` - Error message

## Development

### Project Structure

```
android/
├── app/
│   ├── src/main/kotlin/com/example/remotelauncher/
│   │   ├── MainActivity.kt           # Main activity
│   │   ├── network/                  # Network layer
│   │   │   └── TcpClient.kt          # TCP network client & Protocol definitions
│   │   ├── ui/                       # UI Components (Jetpack Compose)
│   │   │   ├── ConnectionScreen.kt
│   │   │   ├── ModelSelectionScreen.kt
│   │   │   └── TaskExecutionScreen.kt
│   │   └── viewmodel/                # App state management
│   │       └── MainViewModel.kt
│   └── src/main/res/
│       └── layout/
│           └── activity_main.xml
├── build.gradle.kts                  # Project build configuration
└── README.md                         # This file
```

### Tech Stack

- **Kotlin** - Programming language
- **Jetpack Compose** - Modern UI toolkit
- **Material 3** - UI components
- **Java Sockets** - TCP networking
- **Kotlinx Serialization** - JSON serialization
- **Coroutines & Flow** - Async programming

## Future Enhancements

- [ ] Authentication support
- [ ] Bluetooth connectivity option
- [ ] Save and load task templates
- [ ] View detailed agent logs on mobile
- [ ] Push notifications for task completion
- [ ] Multiple server connections
- [ ] Dark/Light theme toggle

## License

Same as parent project
