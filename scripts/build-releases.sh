#!/bin/bash
set -e

# Get the directory where the script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$( dirname "$SCRIPT_DIR" )"
RELEASE_DIR="${REPO_ROOT}/releases"

mkdir -p "$RELEASE_DIR"

echo "--- Building for macOS (native) ---"
cd "${REPO_ROOT}/agent"
cargo build --release
cp target/release/agent "${RELEASE_DIR}/agent-macos"

echo "--- Building for Windows (x86_64-mscv) ---"
# cargo-xwin handles the Windows SDK automatically
cargo xwin build --release --target x86_64-pc-windows-msvc
cp target/x86_64-pc-windows-msvc/release/agent.exe "${RELEASE_DIR}/agent-windows.exe"

echo "--- All builds complete! ---"
echo "Binaries are available in: ${RELEASE_DIR}"
ls -lh "$RELEASE_DIR"
