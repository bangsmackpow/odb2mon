# obd2mon

Cross-platform automotive diagnostics engine and dashboard targeting modern BMW UDS (over DoIP Ethernet) and legacy GM OBD2 (over ELM327 Bluetooth/Serial). Engineered in pure Rust with asynchronous I/O and zero UI coupling.

---

## Overview

`obd2mon` bridges vehicle communication buses into a unified telemetry stream for desktop and mobile targets. The system provides real-time dashboard instrumentation (RPM, road speed, coolant temperature, engine oil temperature) and Diagnostic Trouble Code (DTC) management across heterogeneous vehicle protocols:

- **2011 BMW 535i xDrive (F10 chassis, N55 engine):** Communicates via an ENET (RJ45-to-OBD2) cable over TCP/IP using Diagnostics over IP (DoIP, ISO 13400) and Unified Diagnostic Services (UDS, ISO 14229).
- **2010 GMC Acadia (3.6L LLT V6):** Communicates via ELM327-compatible Bluetooth adapters over serial streams using standard SAE J1979 / ISO 15765-4 (CAN 11-bit 500k) and GM manufacturer-specific Mode 22 PIDs.
- **Hardware-Free Simulator:** Physics-based virtual vehicle simulator for automated testing and CI/CD without physical vehicle access.

---

## Architectural Principles

1. **Strict Protocol Abstraction:** The core library (`obd-core`) defines the `ObdConnection` trait. The presentation layer and business logic remain completely agnostic to transport mechanics (TCP vs. Serial) and diagnostic dialects (UDS vs. standard OBD2).
2. **Asynchronous Non-Blocking I/O:** Built on `tokio`. Network and bus transactions run in background worker loops decoupled from UI rendering threads via `tokio::sync::watch` and `tokio::sync::mpsc` channels.
3. **Graceful PID Degradation:** On platforms where specific parameters are unsupported (e.g., Mode 22 Oil Temperature on 2010 GM platforms), the engine traps Negative Response Codes (NRCs) and `NO DATA` responses, returning `ParameterValue::Unsupported` without thread stalls or panics.
4. **Session Elevation Automation:** For BMW UDS DIDs requiring diagnostic privileges (NRC `0x22` - Conditions Not Correct), the adapter automatically negotiates Extended Diagnostic Session (`0x10 0x03`) and retries the transaction.

---

## Repository Layout

```text
obd2mon/
├── Cargo.toml                       # Workspace manifest
├── .github/
│   └── workflows/
│       └── build.yml                # Windows & Android CI/CD pipeline
├── crates/
│   ├── obd-core/                    # Platform-agnostic core library
│   │   ├── src/
│   │   │   ├── lib.rs               # Library root & public exports
│   │   │   ├── error.rs             # ObdError enum & ISO 14229 NRC translations
│   │   │   ├── connection.rs        # ObdConnection trait & state machine
│   │   │   ├── profile.rs           # VehicleProfile & telemetry value models
│   │   │   ├── manager.rs           # Decoupled polling loop & broadcast channels
│   │   │   ├── protocols/
│   │   │   │   ├── doip.rs          # ISO 13400 framing & Routing Activation
│   │   │   │   ├── uds.rs           # ISO 14229 Services 0x10, 0x19, 0x22, 0x04
│   │   │   │   └── elm327.rs        # SAE J1979 Modes 01, 22, 03, 04 parsers
│   │   │   └── adapters/
│   │   │       ├── enet.rs          # BMW ENET TCP client
│   │   │       ├── bluetooth.rs     # GMC ELM327 Bluetooth adapter
│   │   │       └── mock.rs          # Physics-based simulation engine
│   │   └── tests/
│   │       └── simulator_test.rs    # Core integration test suite
│   └── obd-cli/                     # Headless command-line diagnostic tool
│       └── src/
│           └── main.rs              # Real-time console telemetry runner
└── README.md
```

---

## Vehicle Profiles & Parameter Matrix

| Parameter | BMW 535i (F10 / N55) | 2010 GMC Acadia (LLT) | Fallback Behavior |
| :--- | :--- | :--- | :--- |
| **Engine RPM** | UDS DID `0xD001` via DME `0x0012` | Mode `01` PID `0C` | Returns `NotAvailable` if bus idle |
| **Vehicle Speed** | UDS DID `0xD002` | Mode `01` PID `0D` | Returns `0 km/h` |
| **Coolant Temp** | UDS DID `0xD003` (`raw - 48°C`) | Mode `01` PID `05` (`A - 40°C`) | Plateau warning at >105°C |
| **Oil Temp** | UDS DID `0xD004` (`raw - 48°C`) | Mode `22` PID `1154` | Traps `NO DATA` / `7F 22` -> `Unsupported` |
| **Read DTCs** | UDS Service `0x19 0x02 0xFF` | Mode `03` Request | Parses standard SAE / BMW hex codes |
| **Clear DTCs** | UDS Service `0x04 FF FF FF` | Mode `04` Clear | Requires explicit user safety confirmation |

---

## Quickstart

### Prerequisites

- Rust 1.80+ (`cargo`, `rustc`)
- Windows 10/11 or Linux for desktop development
- Android NDK (r26b+) for mobile cross-compilation

### Running the Test Suite

Run unit and integration tests across the workspace:

```bash
cargo test --workspace
```

### Running the Terminal Telemetry Runner

#### 1. Simulation Mode (Default)
Simulates active engine acceleration, thermodynamic coolant/oil warmup, and realistic fault codes:

```bash
cargo run --bin obd-cli -- mock
```

#### 2. Simulated BMW F10 Profile
```bash
cargo run --bin obd-cli -- bmw
```

#### 3. Simulated GMC Acadia Profile (Demonstrating Graceful Oil Temp Fallback)
```bash
cargo run --bin obd-cli -- gmc
```

#### 4. Live BMW ENET Hardware
Connect your laptop to the vehicle OBD2 port using an ENET RJ45 cable (standard vehicle IP: `169.254.1.1`, port `6801`):

```bash
cargo run --bin obd-cli -- enet 169.254.1.1 6801
```

---

## Cross-Platform UI Architecture

The presentation layer is decoupled from the Rust core using **Tauri v2**:

- **Core (`obd-core`):** Runs the background `VehicleManager` loop, polling vehicle buses at 10–20 Hz.
- **IPC Bridge:** Telemetry snapshots (`DashboardSnapshot`) are broadcast to the frontend via `app_handle.emit("telemetry", snapshot)`.
- **Frontend:** Built with Svelte 5 and SVG for high-contrast geometric dials, digital readouts, and responsive scaling on both Windows laptops and Android dashboard mounts.

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
