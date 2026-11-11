#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
export IPHONEOS_DEPLOYMENT_TARGET=17.0
export CARGO_PROFILE_RELEASE_LTO=false
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
export CARGO_PROFILE_RELEASE_STRIP=debuginfo

target=aarch64-apple-ios
output="$PWD/target/ipad"
bundle="$output/Payload/Benilla-WotLK.app"
ipa="$output/Benilla-WotLK.ipa"

if [[ $# -eq 0 ]]; then
    rustup target add "$target"
    toolchain_bin="$(dirname "$(rustup which rustc)")"
    export PATH="$toolchain_bin:$PATH"
    cargo build --locked --release --target "$target" -p benilla --no-default-features
elif [[ $# -ne 1 || "$1" != "--package-only" ]]; then
    echo "Usage: $0 [--package-only]" >&2
    exit 2
fi

binary="target/$target/release/benilla"
test -x "$binary" || { echo "Build $binary before packaging." >&2; exit 1; }
mkdir -p "$bundle"
cp "$binary" "$bundle/benilla"
cp ios/Info.plist "$bundle/Info.plist"
cp LICENSE-APACHE LICENSE-MIT "$bundle/"

# Winit 0.30 uses UIKit's app-delegate lifecycle. When built using a newer
# SDK, keep the SDK 26 lifecycle behavior until winit adopts UIScene.
sdk_version="$(xcrun --sdk iphoneos --show-sdk-version)"
if [[ "${sdk_version%%.*}" -ge 27 ]]; then
    xcrun vtool -set-build-version ios "$IPHONEOS_DEPLOYMENT_TARGET" 26.0 -replace \
        -output "$bundle/benilla.compat" "$bundle/benilla"
    mv "$bundle/benilla.compat" "$bundle/benilla"
fi

icon_plist="$output/AppIcon-Info.plist"
xcrun --sdk iphoneos actool ios/Assets.xcassets \
    --compile "$bundle" --platform iphoneos --target-device ipad \
    --minimum-deployment-target "$IPHONEOS_DEPLOYMENT_TARGET" --app-icon AppIcon \
    --output-partial-info-plist "$icon_plist"

python3 - "$bundle/Info.plist" "$icon_plist" <<'PYTHON'
import plistlib
import sys
from pathlib import Path

info, icons = map(Path, sys.argv[1:])
metadata = plistlib.loads(info.read_bytes())
icon_metadata = plistlib.loads(icons.read_bytes())
assert any(key.startswith("CFBundleIcons") for key in icon_metadata), "App icon metadata missing"
metadata.update(icon_metadata)
info.write_bytes(plistlib.dumps(metadata))
PYTHON

plutil -lint "$bundle/Info.plist"
codesign --force --sign - --timestamp=none "$bundle"
codesign --verify --strict "$bundle"
staged_ipa="$output/.Benilla-WotLK.$$.ipa"
(cd "$output" && zip -q -r "$staged_ipa" Payload)
unzip -tq "$staged_ipa"
mv -f "$staged_ipa" "$ipa"
echo "$ipa"
