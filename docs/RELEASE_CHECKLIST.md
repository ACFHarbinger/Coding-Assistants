# Application Release Checklist

A manual pass for cutting a desktop (`tauri-app`) / Android (`android/`
RemoteLauncher) release from this repository. It pairs with the automated
`.github/workflows/release.yml` job; this checklist covers the steps that need
a human decision or a real machine. The docs website has its own checklist in
[`website/RELEASE_CHECKLIST.md`](website/RELEASE_CHECKLIST.md).

## 1. Pre-release

- [ ] **Version bump.** `just release::bump <semver>` writes `package.json`,
      `src-tauri/Cargo.toml`, and `android/app/build.gradle.kts`
      (`versionName` + derived `versionCode`). It does **not** commit — review
      the diff before committing.
- [ ] **Changelog freeze.** Move the `## [1.0.0]` marker from
      `- Unreleased` to a dated `[X.Y.Z] - YYYY-MM-DD` heading in
      `docs/moon/CHANGELOG.md`. Do not delete history.
- [ ] **CI green on `main`.** Confirm `ci.yml` passes before tagging. Note:
      `lint-test-rust` (clippy `-D warnings`) and `lint-test-android` are
      pre-existing red on `main` (handled in the bugs phase, not a packaging
      blocker). The release job itself does **not** re-run clippy.

## 2. Tag convention

- Cut a `vX.Y.Z` **annotated** tag on `main`, e.g. `git tag -a v1.0.0 -m "v1.0.0"`.
- Push the tag: `git push origin vX.Y.Z`. Pushing it triggers
  `.github/workflows/release.yml`.

## 3. Dry run

- Dispatch `.github/workflows/release.yml` manually with `dry_run: true`
  (GitHub Actions → Workflow → Run workflow). This builds every bundle and
  uploads the artifacts but **skips publishing** the GitHub Release — equivalent
  to `workflow_dispatch` with the publish step gated off.
- Confirm all 6 artifacts build and the job goes green before a real tag push.

## 4. Artifact review

The `release.yml` job publishes a **draft** GitHub Release. Confirm all 6
artifacts are attached:

- [ ] `.deb`
- [ ] `.AppImage`
- [ ] `.msi`
- [ ] NSIS `*-setup.exe`
- [ ] `.apk`
- [ ] `.aab`

Artifacts are written under repo-root `target/release/bundle/{deb,appimage,msi,nsis}/`
desktop side and `android/app/build/outputs/**` Android side.

## 5. Smoke-test matrix

Test each artifact on a real machine before publishing the draft:

- [ ] **AppImage** on an older-glibc distro (e.g. Ubuntu 22.04, not 24.04 —
      a 24.04 AppImage requires glibc 2.39 and won't run on older distros).
- [ ] **`.deb`** install and remove (`dpkg -i` then `dpkg -r`, confirm no
      leftover config/units).
- [ ] **`.msi`** on Windows 10 and 11.
- [ ] **NSIS `-setup.exe`** on Windows 10 and 11.
- [ ] **APK** sideload (a real device with USB debugging / `adb install`).
- [ ] **AAB** via
      `bundletool build-apks --bundle <app>.aab --output <app>.apks --ks <release.jks> --ks-key-alias <alias> --ks-pass pass:<pass> --key-pass pass:<pass>`.

## 6. Publish and post-publish

- [ ] Promote the draft GitHub Release to public (or keep as a pre-release).
- [ ] Announce the release.
- [ ] Close the GitHub milestone associated with `v1.0.0`.

## 7. Known v1.0 caveats

- **Windows bundles are unsigned** for v1.0. Expect a SmartScreen warning on
  `.msi` / `.exe` download and run; the signing pipeline is deferred.
- **Android signing requires secrets.** The release build reads a signed
  `assembleRelease`/`bundleRelease` from CI secrets
  (`ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`,
  `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`). A local unsigned
  `assembleRelease` will not produce a store-signed artifact — set
  `keystore.properties` (git-ignored) locally or rely on CI.
