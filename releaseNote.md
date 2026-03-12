# Release Notes - v1.1.0

This release marks a significant milestone for RustRemote, introducing full Windows support, improved performance, and enhanced security features for production environments.

## 🚀 New Features

### 💻 Windows Native Support
- **DXGI Desktop Capture**: High-performance screen capture on Windows using Desktop Duplication API.
- **Integrated Service Logic**: Seamlessly handle Windows login screens and UAC prompts via automatic desktop switching.
- **MSVC Build Support**: Added cross-compilation support from macOS to Windows using `cargo-xwin`.

### 📦 Production-Ready Builds
- **Baked-in Configuration**: Production proxy server and port are now baked into the binary at build time. No more manual `.env` files for end-users.
- **Unified Build Pipeline**: A single script (`build-releases.sh`) now generates release artifacts for both macOS and Windows.

### 🛠 Improvements & Fixes
- **Refined Display Logic**: Standardized resolution switching and multi-monitor handling.
- **Enhanced Logging**: Proxy connection errors now include the target host/port for easier troubleshooting.
- **Memory & Safety**: Resolved `Send` trait violations in the networking layer, improving stability under high load.
- **Audio Improvements**: Updated macOS capture to use `ScreenCaptureKit` natives.

## 📦 Artifacts
- `agent-macos-bin`: Standalone macOS Agent binary.
- `RustRemote.app`: Bundled macOS Application.
- `agent-windows.exe`: Native Windows Agent executable.

---
*Built with ❤️ using Rust and React.*
