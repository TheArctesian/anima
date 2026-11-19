# Android APK and F-Droid build

This procedure serves M5-7 of the [mobile plan](../../plans/mobile-plan.md). It builds an unsigned APK for `ing.anima.mobile` on x86-64 Linux.

## Build on Linux

1. Use a fresh Git checkout of the commit to build. Install Git, a C compiler, `make`, `pkg-config`, and JDK 21.
2. Install Bun 1.3.14 and Go 1.27.4. Keep both on `PATH`. The repository lockfiles pin the application dependencies and Tauri CLI 2.12.4.
3. Install Rust through `rustup`. Run `rustup target add aarch64-linux-android` in the checkout. The repository pins the compiler version in [`rust-toolchain.toml`](../../../rust-toolchain.toml).
4. Set `JAVA_HOME` to JDK 21. Set `ANDROID_HOME` to the Android SDK. Tauri compiles the app against SDK 37.0. The keyboard plugin compiles against SDK 36. Use `sdkmanager` to install both.

   ```sh
   sdkmanager --sdk_root="$ANDROID_HOME" \
     'platforms;android-36' 'platforms;android-37.0' 'build-tools;36.0.0' 'ndk;28.2.13676358'
   ```

5. Pass the version name and Android version code for the release. The following values are for a local build check.

   ```sh
   apps/desktop/src-tauri/build-android-apk.sh 0.1.0 1000
   ```

   Expected output file is `target/android/Anima-unsigned.apk`. Check the package and the two native libraries.

   ```sh
   "$ANDROID_HOME/build-tools/36.0.0/aapt" dump badging target/android/Anima-unsigned.apk
   unzip -Z1 target/android/Anima-unsigned.apk | grep '^lib/arm64-v8a/'
   ```

   Expected package is `ing.anima.mobile` at the chosen version. The archive contains `libanima_app_lib.so` and `libphonelink.so` for `arm64-v8a`.

## Prepare the F-Droid submission

Use the draft [F-Droid recipe](../../../apps/desktop/fdroid/ing.anima.mobile.yml). The inputs are the release commit, version, upstream APK URL and signing certificate fingerprint. The draft build stays disabled until these inputs are available and the release meets the phone checks in the mobile plan.

The checkout includes the [project license](../../../LICENSE), [font notices](../../../apps/client/src/styles/fonts/) and the [Carbon icon license](../../../apps/client/src/lib/icons/LICENSE-Carbon.txt). The F-Droid source scanner flagged a Go test fixture. The recipe removes it from the recipe checkout because the production build does not use it.

1. Copy the recipe to `metadata/ing.anima.mobile.yml` in an `fdroiddata` checkout. Set `commit`, `versionName` and `versionCode` to the first release.
2. Set `binary` in the build entry to the versioned URL of the signed APK. Run `apksigner verify --print-certs` on that APK. Put the signing certificate SHA-256 in `AllowedAPKSigningKeys` at the top level.
3. Remove `disable` from the build entry. Run `fdroid lint ing.anima.mobile` and `fdroid build --server ing.anima.mobile` using the F-Droid build server. Review the scanner output and the dependency licenses.
4. Compare the signed APK to the unsigned APK from that build. Put the Android build-tools folder on `PATH` so the comparison can run `apksigner`.

   ```sh
   apksigcopier compare --unsigned /path/to/signed-release.apk /path/to/fdroid-rebuild.apk
   ```

   Expected result is exit code zero. Keep the same release signing key for both channels. F-Droid uses the APK signed by Anima when its rebuild matches. Submit the metadata after the checks pass.

See the [F-Droid submission guide](https://f-droid.org/docs/Submitting_to_F-Droid_Quick_Start_Guide/) and the [reproducible builds guide](https://f-droid.org/docs/Reproducible_Builds/).
