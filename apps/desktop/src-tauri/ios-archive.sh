#!/bin/sh
# Makes the archive of the iOS application for one version from the commit that is checked out
# (mobile plan M5-4; OD-M5, OD-M8; RLS-010, RLS-011). It uploads nothing: the upload to
# TestFlight is a step of the release workflow (task B8i, R-203) or an act of the operator.
#
#   ios-archive.sh [version]          with no version: the version of the commit that is checked
#                                     out, from deploy/bin/commit-version (R-183, R-201)
#   ios-archive.sh --plan [version]   prints what a run with this environment makes, and builds
#                                     nothing (for the test of the two modes)
#
# The build number (CFBundleVersion) is never typed: it comes from the version by
# deploy/bin/phone-build-number (M5-3).
#
# Environment:
#   IOS_SIGN_IDENTITY   Unset or empty: an archive with no signature and no IPA. It proves
#                       that the application builds; no phone installs it. Set: the name of the Apple Distribution identity,
#                       as `security find-identity` prints it. The certificate and its key are
#                       in a keychain of the search list of this user: the login keychain of
#                       the Mac of the operator, or the keychain that the job of the release
#                       workflow makes (as its job `macos` does, BLD-083). The script reads no
#                       certificate file and no password.
#   IOS_PROFILE         Needed with IOS_SIGN_IDENTITY: the path of the App Store provisioning
#                       profile of ing.anima.mobile (a .mobileprovision file). The script copies
#                       it to the folders where Xcode looks for profiles.
#
# The script writes the signature settings into the Xcode project that Tauri generates
# (gen/apple/project.yml), because Tauri gives xcodebuild no setting from the caller and reads
# a variable such as XCODE_XCCONFIG_FILE not at all (tried on 2026-11-19: Xcode then signed
# with the development identity of the Mac and made a new development profile).
# - With no identity the project says that nothing is signed.
# - With an identity the project says: a signature by hand, that identity, that profile. Xcode
#   then makes no certificate and no profile, and changes nothing in the developer account. No value of the environment, no identity and no profile content is
# printed, but the name of the identity and the UUID of the profile, which are not secrets.
#
# It prints, as its last lines:
#   version=<version>
#   build=<build number>
#   signed=yes|no
#   archive=<path of the .xcarchive>
#   ipa=<path of the .ipa>           only with a signature
#
# Tauri makes the archive only (--archive-only). Its own export is not used: with --no-sign it
# looks for Anima.app where the application is Anima.ing.app (tauri-cli 2.12.4), and with a
# signature it would let Xcode change profiles (-allowProvisioningUpdates).
#
# Needs: Xcode with the iOS platform, xcodegen, bun with the packages installed, Go (for the
# link, link/build-phone.sh) and the Rust target aarch64-apple-ios.
set -eu

refuse() { echo "ios-archive: $1" >&2; exit 1; }

PLAN_ONLY=false
if [ "${1:-}" = --plan ]; then PLAN_ONLY=true; shift; fi
[ $# -le 1 ] || refuse "give one version, or none for the version of the commit"
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../../.." && pwd)
APP_ID=ing.anima.mobile
VERSION=${1:-$(cd "$ROOT" && sh deploy/bin/commit-version)}
echo "$VERSION" | grep -Eq '^(0|[1-9][0-9]{0,8})\.(0|[1-9][0-9]{0,8})\.(0|[1-9][0-9]{0,8})$' || refuse "the version is not like 0.1.0"
BUILD=$(sh "$ROOT/deploy/bin/phone-build-number" "$VERSION") || refuse "the version has no build number"
IDENTITY=${IOS_SIGN_IDENTITY:-}
PROFILE=${IOS_PROFILE:-}
# The identity goes into a settings file of Xcode and into a property list: one plain line.
case "$IDENTITY" in *\"* | *\\* | *'
'* | *\$* | *\<* | *\&*) refuse "IOS_SIGN_IDENTITY holds a character that a name of an identity does not have" ;; esac
if [ -n "$IDENTITY" ]; then
    SIGNED=yes
    [ -n "$PROFILE" ] || refuse "IOS_SIGN_IDENTITY is set, so IOS_PROFILE must name the provisioning profile"
else
    SIGNED=no
    [ -z "$PROFILE" ] || refuse "IOS_PROFILE is set, so IOS_SIGN_IDENTITY must name the identity"
fi
PROJECT=$HERE/gen/apple
ARCHIVE=$PROJECT/build/anima-app_iOS.xcarchive
OUT=$PROJECT/build/arm64
IPA=$OUT/Anima.ing.ipa
if $PLAN_ONLY; then
    printf 'version=%s\nbuild=%s\nsigned=%s\n' "$VERSION" "$BUILD" "$SIGNED"
    exit 0
fi

ROOM=$(mktemp -d)
trap 'rm -rf "$ROOM"' EXIT
plist() { /usr/libexec/PlistBuddy -c "Print :$1" "$2"; }

if [ "$SIGNED" = yes ]; then
    [ -f "$PROFILE" ] || refuse "IOS_PROFILE is not a file"
    security cms -D -i "$PROFILE" > "$ROOM/profile.plist" 2> /dev/null || refuse "IOS_PROFILE is not a provisioning profile"
    UUID=$(plist UUID "$ROOM/profile.plist")
    TEAM=$(plist TeamIdentifier:0 "$ROOM/profile.plist")
    echo "$UUID" | grep -Eq '^[0-9A-Fa-f-]{36}$' || refuse "the profile has no UUID"
    echo "$TEAM" | grep -Eq '^[A-Z0-9]{10}$' || refuse "the profile has no team"
    [ "$(plist Entitlements:application-identifier "$ROOM/profile.plist")" = "$TEAM.$APP_ID" ] || refuse "the profile is not for $APP_ID"
    [ "$(plist Entitlements:get-task-allow "$ROOM/profile.plist")" = false ] || refuse "the profile is a development profile, not one for App Store Connect"
    security find-identity -v -p codesigning | grep -Fq "\"$IDENTITY\"" || refuse "no keychain of the search list holds the identity that IOS_SIGN_IDENTITY names"
    for folder in "$HOME/Library/MobileDevice/Provisioning Profiles" "$HOME/Library/Developer/Xcode/UserData/Provisioning Profiles"; do
        mkdir -p "$folder"
        cp "$PROFILE" "$folder/$UUID.mobileprovision"
    done
    echo "ios-archive: a signature by hand with the identity $IDENTITY and the profile $UUID"
fi

# The signature settings of the project, as lines of the group `app` of project.yml.
signature_settings() {
    if [ "$SIGNED" = yes ]; then
        echo "      CODE_SIGN_STYLE: Manual"
        echo "      CODE_SIGN_IDENTITY: \"$IDENTITY\""
        echo "      DEVELOPMENT_TEAM: $TEAM"
        echo "      PROVISIONING_PROFILE_SPECIFIER: $UUID"
    else
        echo "      CODE_SIGNING_ALLOWED: \"NO\""
        echo "      CODE_SIGNING_REQUIRED: \"NO\""
        echo "      CODE_SIGN_IDENTITY: \"\""
    fi
}

sh "$ROOT/link/build-phone.sh" aarch64-apple-ios
cd "$HERE/.."
# Xcode runs Tauri's Cargo build in a child process, which can otherwise find a system Rust.
RUST_CARGO=$(rustup which cargo) || refuse "install the Rust toolchain of the repository with rustup"
export PATH="$(dirname "$RUST_CARGO"):$PATH"
export CI=true
# A new project each time, so that no setting of an earlier run stays in it.
rm -rf "$PROJECT/anima-app.xcodeproj" "$PROJECT/project.yml" "$ARCHIVE" "$IPA"
bunx tauri ios init --ci
grep -q '^      PRODUCT_BUNDLE_IDENTIFIER: ' "$PROJECT/project.yml" || refuse "project.yml of Tauri has no group of settings where the script looks for it"
cp -R "$HERE/ios/AnimaAppIcon.appiconset" "$PROJECT/Assets.xcassets/"
{
    signature_settings
    echo '      ASSETCATALOG_COMPILER_APPICON_NAME: AnimaAppIcon'
    echo '      TARGETED_DEVICE_FAMILY: "1,2"'
} > "$ROOM/settings"
# After the line of the identifier, which each generated project has in its group `app`.
# The team of the profile takes the place of the team that Tauri wrote.
if [ "$SIGNED" = yes ]; then sed -i '' -e '/^      DEVELOPMENT_TEAM: /d' "$PROJECT/project.yml"; fi
sed -i '' -e "/^      PRODUCT_BUNDLE_IDENTIFIER: /r $ROOM/settings" "$PROJECT/project.yml"
(cd "$PROJECT" && xcodegen generate --spec project.yml --quiet)
CONFIG="{\"version\":\"$VERSION\",\"bundle\":{\"iOS\":{\"bundleVersion\":\"$BUILD\"}}}"
bunx tauri ios build --target aarch64 --ci --archive-only --config "$CONFIG"

APP=$ARCHIVE/Products/Applications/Anima.ing.app
[ -d "$APP" ] || refuse "the build made no archive at $ARCHIVE"
[ "$(plist CFBundleIdentifier "$APP/Info.plist")" = "$APP_ID" ] || refuse "the application of the archive is not $APP_ID"
[ "$(plist CFBundleShortVersionString "$APP/Info.plist")" = "$VERSION" ] || refuse "the archive does not have the version $VERSION"
[ "$(plist CFBundleVersion "$APP/Info.plist")" = "$BUILD" ] || refuse "the archive does not have the build number $BUILD"
for family in CFBundleIcons 'CFBundleIcons~ipad'; do
    [ "$(plist "$family:CFBundlePrimaryIcon:CFBundleIconName" "$APP/Info.plist")" = AnimaAppIcon ] ||
        refuse "the archive does not have the Anima icon for $family"
done

if [ "$SIGNED" = no ]; then
    if codesign --display "$APP" > /dev/null 2>&1; then refuse "the application of the archive has a signature, and this run asked for none"; fi
    printf 'version=%s\nbuild=%s\nsigned=no\narchive=%s\n' "$VERSION" "$BUILD" "$ARCHIVE"
    exit 0
fi
codesign --verify --deep --strict "$APP" || refuse "the signature of the application in the archive is not valid"
cat > "$ROOM/export.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>method</key><string>app-store-connect</string>
    <key>destination</key><string>export</string>
    <key>signingStyle</key><string>manual</string>
    <key>teamID</key><string>$TEAM</string>
    <key>signingCertificate</key><string>$IDENTITY</string>
    <key>provisioningProfiles</key><dict><key>$APP_ID</key><string>$UUID</string></dict>
    <key>manageAppVersionAndBuildNumber</key><false/>
    <key>uploadSymbols</key><true/>
</dict>
</plist>
EOF
mkdir -p "$OUT"
xcodebuild -exportArchive -archivePath "$ARCHIVE" -exportOptionsPlist "$ROOM/export.plist" -exportPath "$OUT"
[ -f "$IPA" ] || refuse "the export made no IPA at $IPA"
printf 'version=%s\nbuild=%s\nsigned=yes\narchive=%s\nipa=%s\n' "$VERSION" "$BUILD" "$ARCHIVE" "$IPA"
