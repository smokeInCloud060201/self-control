# RustRemote: Secure High-Performance Remote Desktop

RustRemote is a custom, TeamViewer-style remote access application built with **Rust** and **React**. It features ultra-low latency streaming, on-demand resource usage, and a secure pairing mechanism using hardware-based Machine IDs and temporary Session Passwords.

## 🏗 System Architecture

The project consists of three primary components:

```mermaid
graph TD
    A[Remote Agent (Target)] <--> |WebSockets| P(Relay Proxy)
    C[Web Client (Controller)] <--> |WebSockets| P
    
    subgraph Target Machine
    A --> |Screen Capture| S[OS Display API]
    A --> |Input Simulation| I[Enigo]
    end
    
    subgraph Relaying
    P --> |Binary Relay| C
    C --> |JSON Input| P
    P --> |Control Signals| A
    end
```

### 1. Agent (Host)
The Agent runs on the machine to be controlled.
- **On-Demand Streaming**: Remains idle until a client connects to save CPU/Network.
- **Screen Capture**: Uses the `scrap` crate for high-speed frame capture.
- **Compression**: Encodes frames to JPEG using `image` crate with quality-optimized settings (~20 FPS).
- **Input Simulation**: Simulates mouse and keyboard events using the `enigo` crate.
- **Observability**: Uses `tracing` for structured logs and performance monitoring.

### 2. Relay Proxy (Gateway)
A high-performance relay server built with `tokio`.
- **Session Management**: Pairs Agents and Clients based on Machine IDs.
- **Persistence**: Remembers session passwords across temporary disconnections.
- **Backpressure**: Uses bounded channels to handle network jitter without memory bloat.
- **Security**: Verifies 6-digit passwords before bridging connections.

### 3. Web Client (Frontend)
A modern React application for remote control.
- **Premium UI**: Built with Tailwind CSS, featuring glassmorphism and an intuitive dashboard.
- **Fast Rendering**: Uses HTML5 `<canvas>` and `createImageBitmap` for near-zero latency display.
- **Event Tracking**: Captures local mouse events and relays them as precision coordinates (0.0 to 1.0).

---

## 🚀 How It Works (The Flow)

1.  **Registration**: The Agent starts, generates a random 6-digit password, and connects to the Proxy using its hardware Machine ID.
2.  **Idle State**: The Agent enters a low-power "Idle" state, waiting for a signal.
3.  **Discovery**: The user enters the Agent's Machine ID and Password into the Web Client.
4.  **Handshake**:
    - Web Client connects to the Proxy.
    - Proxy verifies the password.
    - Proxy sends a `start_capture` command to the Agent.
5.  **Streaming**:
    - Agent starts capturing and sending binary JPEG frames.
    - Proxy relays these frames directly to the Web Client.
    - Web Client renders frames and sends back mouse/input JSON.
6.  **Termination**: When the Client disconnects, the Proxy signals the Agent to `stop_capture`, and the Agent returns to idle.

---

## 🛠 Tech Stack

- **Backend**: Rust 1.75+, Tokio (Async), Tungstenite (WebSockets), Tracing (Observability), Serde (JSON).
- **Frontend**: React (Vite), Tailwind CSS, Lucide React (Icons).
- **Security**: Machine-UID (Hardware fingerprinting), Random session tokens.

## 🏃 Running the Project

### 1. Start the Proxy
```bash
cd proxy
cargo run -- --port 8080
```

### 2. Start the Agent
#### Terminal Mode
```bash
cd agent
# Connect to a remote proxy (e.g., at IP B)
cargo run -- --server 1.2.3.4 --port 8080
```

#### macOS Desktop Mode
After running the build script:
```bash
./scripts/package-macos.sh
open agent/build/RustRemote.app
```

### 3. Start the Web Client
```bash
cd web
npm install
npm run dev
```

---

## 🔒 Security Model

- **Machine ID**: unique hardware-bound identifier.
- **Passkeys**: Randomly generated per-session secrets.
- **No Persistence**: Relays do not store image data; they are strictly pass-through.
- **Memory Safety**: Built entirely in Rust to eliminate buffer overflows and common memory vulnerabilities.
