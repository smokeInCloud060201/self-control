#!/bin/bash
set -e

APP_NAME="RustRemote"
BUILD_DIR="agent/build"
APP_BUNDLE="${BUILD_DIR}/${APP_NAME}.app"

echo "Building Agent binary..."
cd agent
cargo build --release
cd ..

echo "Creating App Bundle structure..."
mkdir -p "${APP_BUNDLE}/Contents/MacOS"
mkdir -p "${APP_BUNDLE}/Contents/Resources"

echo "Copying binary and metadata..."
cp agent/target/release/agent "${APP_BUNDLE}/Contents/MacOS/"
cp "${BUILD_DIR}/Info.plist" "${APP_BUNDLE}/Contents/"
cp "${BUILD_DIR}/RustRemote.icns" "${APP_BUNDLE}/Contents/Resources/AppIcon.icns"

echo "Success! Application created at ${APP_BUNDLE}"
echo "You can now run it: open ${APP_BUNDLE}"
