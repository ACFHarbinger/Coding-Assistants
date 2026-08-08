# Kotlin Rules (Android Companion App)

- Target Kotlin/JVM per `android/build.gradle.kts`, built via Gradle Kotlin DSL.
- Format/lint with `ktlint` (`./gradlew ktlintCheck`, `./gradlew ktlintFormat`) from `android/`.
- Prefer immutable data classes and `val` over `var`; avoid platform types leaking from Java interop without an explicit null-check.
- Use coroutines (`kotlinx.coroutines`) for async work (e.g. TCP communication with the desktop app) instead of raw threads or callbacks.
- Tests live under `android/app/src/test/` (unit) and `android/app/src/androidTest/` (instrumented).
- Keep Gradle build logic and dependency versions in `android/build.gradle.kts`/`android/app/build.gradle.kts`, not scattered as hardcoded literals.
