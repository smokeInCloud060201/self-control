# RustRemote: Secure High-Performance Remote Desktop (v1.1)

RustRemote is a professional, high-performance remote access application built with **Rust** and **React**. It features ultra-low latency streaming, cross-platform support (macOS & Windows), and a secure pairing mechanism using hardware-based Machine IDs.

## 🌟 Key Features (v1.1)
- **🚀 Native Performance**: 100% Rust-based agent for maximum efficiency and security.
- **🖥️ Cross-Platform**: Full support for macOS (ScreenCaptureKit) and Windows (DXGI/GDI).
- **🔒 Secure Architecture**: Hardware fingerprinting (Machine ID) + 6-digit session passkeys.
- **⚡ Ultra-Low Latency**: High-speed frame capture with optimized JPEG compression and HTML5 Canvas rendering.
- **🔌 Integrated Service**: Automatic desktop switching on Windows for seamless login screen and system-level control.
- **📦 Zero-Config Release**: Built-in production configuration baked directly into binaries.
- **🎧 Audio Capture**: Native system audio streaming on macOS.

## 🏗 System Architecture

The project consists of three primary components:

```mermaid
graph TD
    A[Remote Agent (Target)] <--> |WebSockets| P(Relay Proxy)
    C[Web Client (Controller)] <--> |WebSockets| P
    
    subgraph Target Machine
    A --> |Screen Capture| S[OS Display API]
    A --> |Input Simulation| I[Enigo]
    A --> |Audio Capture| AU[SCK/WASAPI]
    end
    
    subgraph Relaying
    P --> |Binary Relay| C
    C --> |JSON Input| P
    P --> |Control Signals| A
    end
```

### 1. Agent (Host)
The Agent runs on any machine you want to control.
- **On-Demand Streaming**: Resource efficient; stops all capture when no client is connected.
- **Native Grabbers**: Uses `ScreenCaptureKit` on macOS and `DXGI` on Windows for GPU-accelerated capture.
- **Control**: Precision mouse & keyboard simulation. On Windows, it includes `AutoDesktop` switching to interact with secure desktops (UAC/Login).
- **Baked Config**: Production server details are baked into the binary at build time via `cargo:rustc-env`.

### 2. Relay Proxy (Gateway)
A high-performance relay server built with `tokio`.
- **Session Management**: Pairs Agents and Clients based on Machine IDs.
- **Binary Tunneling**: Transparently bridges binary frames and JSON control signals.
- **Secure Handshake**: Password-protected sessions with automatic reconnection handling.

### 3. Web Client (Frontend)
A modern dashboard for full remote control.
- **Premium UI**: Modern glassmorphic design with real-time status indicators.
- **Native Feel**: Full-screen support, clipboard synchronization, and dynamic resolution switching.

---

## 🏃 Getting Started

### 1. Building Releases
We use a unified build script to generate production-ready binaries for both platforms.

```bash
# Builds and packages macOS (.app) and Windows (.exe)
./scripts/build-releases.sh
```
Binaries will be available in the `releases/` directory.

### 2. Manual Development
**Start the Proxy:**
```bash
cd proxy && cargo run
```

**Start the Agent:**
```bash
cd agent && cargo run # Uses .env or baked-in defaults
```

**Start the Web Client:**
```bash
cd web && npm install && npm run dev
```

---

## 🛡 Security model
- **Hardware-bound Machine ID**: Prevents spoofing.
- **Transient Passkeys**: Passwords rotate or are wiped to ensure temporary access.
- **No Data Retention**: Frames are relayed in memory; no video data is ever written to disk.
- **Rust Safety**: Eliminates entire classes of memory safety bugs by design.
