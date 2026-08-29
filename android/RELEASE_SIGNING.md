# Android Companion Release Signing

This document describes how the Android companion app (`com.codingassistants.remotelauncher`) is signed for release builds (`assembleRelease` and `bundleRelease`), including CI secrets and local developer workflow.

---

## 1. CI Secrets Configuration

GitHub Actions (`.github/workflows/release.yml`) requires four repository secrets:

| Secret Name | Description | Example / Current Value |
|---|---|---|
| `ANDROID_KEYSTORE_BASE64` | Base64-encoded contents of `android/keystore/release.jks` | *(See generated output below)* |
| `ANDROID_KEYSTORE_PASSWORD` | Keystore password used to unlock `release.jks` | `ca-release-2026-s9xK8mQp4vL2wZ` |
| `ANDROID_KEY_ALIAS` | Key alias inside the keystore | `coding-assistants` |
| `ANDROID_KEY_PASSWORD` | Private key password | `ca-release-2026-s9xK8mQp4vL2wZ` |

### Stored Keystore Base64 (`ANDROID_KEYSTORE_BASE64`)

```text
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
```

---

## 2. Local Development Setup

For local builds, create a git-ignored file at `android/keystore.properties`:

```properties
storeFile=keystore/release.jks
storePassword=ca-release-2026-s9xK8mQp4vL2wZ
keyAlias=coding-assistants
keyPassword=ca-release-2026-s9xK8mQp4vL2wZ
```

Place your keystore at `android/keystore/release.jks` (or provide an absolute path in `storeFile`).

Alternatively, export the environment variables:
```bash
export ANDROID_KEYSTORE_PASSWORD="ca-release-2026-s9xK8mQp4vL2wZ"
export ANDROID_KEY_ALIAS="coding-assistants"
export ANDROID_KEY_PASSWORD="ca-release-2026-s9xK8mQp4vL2wZ"
```

---

## 3. How to Regenerate the Keystore

If you need to generate a fresh release keystore:

```bash
mkdir -p android/keystore
keytool -genkeypair -v \
  -keystore android/keystore/release.jks \
  -alias coding-assistants \
  -keyalg RSA \
  -keysize 2048 \
  -validity 10000 \
  -storetype JKS \
  -dname "CN=Coding Assistants, O=ACFHarbinger, C=US" \
  -storepass "<NEW_KEYSTORE_PASSWORD>" \
  -keypass "<NEW_KEY_PASSWORD>"
```

To encode the keystore for GitHub Actions secrets:
```bash
base64 -w 0 android/keystore/release.jks
```

---

## 4. Building & Verifying Release Artifacts

To produce the release APK and AAB:
```bash
cd android
./gradlew assembleRelease bundleRelease
```

Output locations:
- APK: `android/app/build/outputs/apk/release/app-release.apk`
- AAB: `android/app/build/outputs/bundle/release/app-release.aab`

To verify the APK signature using `apksigner`:
```bash
$ANDROID_HOME/build-tools/34.0.0/apksigner verify --verbose --print-certs app/build/outputs/apk/release/app-release.apk
```

Expected output includes:
```text
Verifies
Verified using v2 scheme (APK Signature Scheme v2): true
Number of signers: 1
Signer #1 certificate DN: CN=Coding Assistants, O=ACFHarbinger, C=US
```
