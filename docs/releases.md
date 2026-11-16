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

1. In GitHub, create a new `feat/<description>` or `bug/<description>` branch
   and use the file editor or web editor to prepare the changes.
2. Edit only the root **`release.json`** for version settings. For example:

   ```json
   {
     "version": "0.0.2",
     "android_version_code": 11004
   }
   ```

   `version` must match the intended tag without `v` (`v0.0.2` → `0.0.2`).
   Increase `android_version_code` for each later release; the prepared `0.0.1`
   uses **11003**, so the next code is **11004**. Do not put credentials here.
3. Do not manually edit version fields in `xr/project.godot` or
   `xr/export_presets.cfg`: they are derived copies. Build/preview/check helpers
   automatically synchronize the app/footer version, APK version/code and export
   filename. CI validates tags and generates metadata from `release.json`.
   Local editor users can run `make version` to sync without building or launching.
4. Update release notes and relevant [validation evidence](validation.md). State
   what was tested, what is implemented and which device checks remain unrun.
5. Commit the changes through the GitHub editor and open a PR against `master`.
   Require the **Test** workflow to pass before merging. Local development checks
   are documented in [development.md](development.md), but are not required to
   operate the release flow from the UI.

The Test workflow runs formatting/full workspace lint and the synthetic Rust,
Python, Godot and software-GPU suites. It does not authorize accounts or test a
console/headset. Follow [the Quest procedure](android.md#install-and-test) for
actual device acceptance.

## 2. Publish through the GitHub UI

No local Git commands or artifact downloads are required for the release flow.

1. Merge the feature/fix PR into `master` after the **Test** workflow passes.
   Make sure the merged source includes the intended APK version and release notes.
2. Open the repository's **Releases** page, then **Draft a new release**.
3. Choose **Choose a tag**, enter a new version such as `v0.0.1`, and select
   **Create new tag on publish**. Set **Target** to `master`.
4. Enter the release title and notes. The prepared [v0.0.1 notes](release-notes/v0.0.1.md)
   can be pasted into the description. Mark experimental builds as a prerelease;
   use a numeric tag even for prereleases (no `-rc` suffix).
5. Select **Publish release**. Saving a draft alone does not start a build.
6. Open **Actions → Build** to follow the run. When all jobs pass, refresh the
   release page: the files appear under **Assets** automatically.

The workflow validates the tag format, APK version match and that the tagged
commit belongs to `master` history. Both builds use that exact validated commit.
It runs only when a release is published; creating/pushing a tag alone does not
start another build. Keep published tags fixed.

## 3. Automatically attached assets

The final attachment job runs only after both platform builds succeed. It verifies
both outputs' source revision, tag, version and file checksums before packaging.
Only that final job receives permission to write release assets.

| Release asset | Contents |
| --- | --- |
| `XRbox-0.0.1-quest-debug.apk` | Verified Android ARM64 development APK, directly installable |
| `XRbox-v0.0.1-linux-arm64-extension.tar.gz` | Optimized native `.so`, runtime dependency listing, notices and build metadata |
| `android-build.json`, `linux-build.json` | Source revision, tag, app/core versions and artifact SHA256 values |
| `SHA256SUMS` | Checksums of all attached build files and notices |
| `LICENSE`, `THIRD_PARTY_NOTICES.md` | License and dependency notices |

Release attachments persist beyond the 30-day Actions artifact retention period.
Linux is an extension artifact, not a complete Linux streaming application.
Publishing the release makes its notes visible immediately; assets appear after
building finishes. A published release with missing assets is not a successful build.

If a build fails, inspect its logs in **Actions → Build** and use **Re-run failed
jobs** after resolving any transient runner problem. The attachment job waits for
both successful builds. A full rerun replaces assets of the same names with the
new run's outputs and regenerated checksums. Asset uploads are not atomic: if
attachment fails partway, rerun the failed job to complete the set. Source fixes
require merging a new PR and publishing a new version, not moving the old tag.

CI uses a fresh Android development signing key for each build. A rebuilt APK may
not upgrade one signed by an earlier run or your local key; stable public signing
remains separate work. No production signing key is used or attached. This
workflow does not create releases, install APKs or operate devices itself.

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
