# Creating an XRbox release

A Git tag identifies the exact source to build. The APK version name must equal
the tag without `v`: **`v0.0.1` → APK `0.0.1`**. Tags may use two or three numeric
components (`vX.X` or `vX.X.X`); prerelease suffixes do not match the build trigger.
The Rust core has an internal version, currently `0.1.10`, recorded separately in
artifact metadata. Do not reset that library version to match a public app tag.

The first release is an **experimental Quest 3S test build**, not a claim of
headset acceptance. Linux artifacts currently contain only the native ARM64
extension, not a complete Linux streaming app. Linux live video remains disabled.

## 1. Prepare and merge the source

1. Create a new `feat/<description>` or `bug/<description>` branch.
2. Update `xr/project.godot` → `application/config/version` and the Android preset
   in `xr/export_presets.cfg` → `version/name` and `export_path`. Keep the APK
   version equal to the intended tag without `v`. The packaging helper derives
   its filename and manifest check from the Godot project version.
3. Increase Android `version/code` for each subsequent APK release. It must not
   decrease even when the display version changes. For `0.0.1`, retain **11003**
   to upgrade the earlier prototype (**11002**) using the same signing key.
4. Update release notes and relevant [validation evidence](validation.md). State
   what was tested, what is implemented and which device checks remain unrun.
5. Run appropriate local checks from [development.md](development.md), including
   `make test-python` for build-helper changes and `make xr-check` for UI changes.
   `make xr-apk` builds/signs/verifies locally without operating a device.
6. Push the branch, open a PR against `master`, and require the **Test** workflow
   to pass before merging. Do not claim hosted checks passed from local lint alone.

The Test workflow runs formatting/full workspace lint and the synthetic Rust,
Python, Godot and software-GPU suites. It does not authorize accounts or test a
console/headset. Follow [the Quest procedure](android.md#install-and-test) for
actual device acceptance.

## 2. Tag the tested commit and collect build artifacts

After merging, update the local `master` normally, verify the source/version,
and create the tag yourself. Do not overwrite an existing published tag.

```sh
git switch master
git pull --ff-only origin master
git tag -a v0.0.1 -m 'XRbox v0.0.1 — Quest 3S test build'
git push origin refs/tags/v0.0.1
```

A new matching tag starts the **Build** workflow. It builds:

| Actions artifact | Contents |
| --- | --- |
| `xrbox-v0.0.1-linux-arm64-extension` | Optimized native `.so`, runtime dependency listing, notices and `build.json` |
| `xrbox-v0.0.1-android-development-apk` | Verified development APK, notices and `build.json` |

Each `build.json` records the source commit, public tag, app/core versions and
SHA256 values. Both build jobs must succeed. These artifacts are retained for
30 days; attaching files to a release makes them available beyond that window.

Download both artifacts from the Actions run, or use the GitHub CLI:

```sh
gh run list --workflow build.yml --event push --limit 10
# Select the run for the exact v0.0.1 tag and merged source commit:
gh run watch RUN_ID --exit-status
gh run download RUN_ID --dir .tools/releases/v0.0.1
```

Verify that the run's commit matches the tag and both `build.json` files. Check
that the APK metadata says `app_version: 0.0.1` and its SHA256 matches the file.
Do not mix artifacts from different runs.

## 3. Create the release with downloadable assets

Use GitHub's Releases page or prepare a draft through `gh`. The prepared [v0.0.1 notes](release-notes/v0.0.1.md) can be used with this test release. Suggested content:

- First experimental Android ARM64 console Remote Play build for Quest 3S.
- Shared controller-first UI; Meta thumbstick/A-X/B-Y/stick-click menu controls.
- Bluetooth Xbox controller required for gameplay; no relay PC or cloud gaming.
- Actual sign-in, hardware video/audio, controllers, Keystore and XR acceptance
  are pending unless supported by recorded device results.
- Linux attachment is the native extension only; Linux streaming is not ready.

Create an archive of the Linux artifact and attach the APK directly so Quest users
can install it without unpacking the Actions artifact. Include build metadata,
checksums and licensing files. Keep debug signing keys out of every attachment.
For example, from the repository root after downloading the matching run:

```sh
artifacts=.tools/releases/v0.0.1
linux="$artifacts/xrbox-v0.0.1-linux-arm64-extension"
android="$artifacts/xrbox-v0.0.1-android-development-apk"
tar -czf "$artifacts/XRbox-v0.0.1-linux-arm64-extension.tar.gz" -C "$linux" .
cp "$android/XRbox-0.0.1-quest-debug.apk" "$artifacts/"
cp "$android/build.json" "$artifacts/android-build.json"
(
  cd "$artifacts"
  shasum -a 256 XRbox-0.0.1-quest-debug.apk \
    XRbox-v0.0.1-linux-arm64-extension.tar.gz android-build.json > SHA256SUMS
)
```

Save the checksum output and exact release notes, then create a draft:

```sh
gh release create v0.0.1 --verify-tag --draft --prerelease \
  --title 'XRbox v0.0.1 — Quest 3S test build' \
  --notes-file docs/release-notes/v0.0.1.md \
  .tools/releases/v0.0.1/XRbox-0.0.1-quest-debug.apk \
  .tools/releases/v0.0.1/XRbox-v0.0.1-linux-arm64-extension.tar.gz \
  .tools/releases/v0.0.1/android-build.json \
  .tools/releases/v0.0.1/SHA256SUMS \
  LICENSE THIRD_PARTY_NOTICES.md
```

Review the draft's notes/assets/checksums, then publish it through GitHub or
`gh release edit v0.0.1 --draft=false`. Publishing also triggers Build for the
same tag; it does not replace attached assets automatically. Keep the originally
reviewed artifact set together rather than mixing files from that later run.

The workflow uploads **Actions artifacts only**. It never creates releases,
attaches release assets, installs APKs or operates devices on its own. Android
CI builds use a fresh development signing key each run; they may not upgrade an
APK signed by another run or by your local key. Keep the local ignored debug key
for local updates. Stable public signing is a separate decision before distributing
ongoing updates; no production signing key is used by this test release.

## Test on Quest 3S

Follow [android.md](android.md#install-and-test). For the local APK:

```sh
adb install -r dist/xr/android/XRbox-0.0.1-quest-debug.apk
```

Confirm **0.0.1**, stereo output and head tracking in the headset. Test Meta-only menu
navigation first, then supported sign-in, console discovery and actual moving
video/audio/gamepad input. Test menu resume, focus loss, reconnect and secure
restoration/sign-out separately. Share sanitized results; never share sign-in
codes, tokens, raw SDP/ICE or account IDs. Packaging success is not device acceptance.
