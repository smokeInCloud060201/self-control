#!/bin/bash
set -e

# Get the directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( dirname "$SCRIPT_DIR" )"
RELEASE_DIR="${REPO_ROOT}/releases"

mkdir -p "$RELEASE_DIR"

echo "--- Building for macOS (.app bundle) ---"
cd "${REPO_ROOT}/agent"
cargo build --release

APP_NAME="SelfControl"
APP_BUNDLE="${RELEASE_DIR}/${APP_NAME}.app"

mkdir -p "${APP_BUNDLE}/Contents/MacOS"
# Package, Sign and Zip
cp target/release/agent "${APP_BUNDLE}/Contents/MacOS/"
cp build/Info.plist "${APP_BUNDLE}/Contents/"

echo "--- Signing and verifying macOS app bundle ---"
codesign --force --deep --sign - --entitlements build/agent.entitlements "${APP_BUNDLE}"

echo "--- Zipping macOS app bundle for GitHub releases ---"
cd "${RELEASE_DIR}"
zip -qr "${APP_NAME}-macos.zip" "${APP_NAME}.app"
cd - > /dev/null

# Also keep a standalone binary for convenience
cp target/release/agent "${RELEASE_DIR}/agent-macos-bin"

echo "--- Building for Windows (x86_64-mscv) ---"
# cargo-xwin handles the Windows SDK automatically
cargo xwin build --release --target x86_64-pc-windows-msvc
cp target/x86_64-pc-windows-msvc/release/agent.exe "${RELEASE_DIR}/agent-windows.exe"

echo "--- All builds complete! ---"
echo "Binaries are available in: ${RELEASE_DIR}"
ls -lh "$RELEASE_DIR"
