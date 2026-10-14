# obd2mon

[![Version](https://img.shields.io/badge/version-0.9.0-blue.svg)](file:///C:/Users/curtis/Desktop/dev/bimmermon/Cargo.toml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green.svg)](file:///C:/Users/curtis/Desktop/dev/bimmermon/README.md#license)
[![Tests](https://img.shields.io/badge/tests-49%20passed-brightgreen.svg)](file:///C:/Users/curtis/Desktop/dev/bimmermon/crates/obd-core/tests/simulator_test.rs)

High-performance, cross-platform automotive diagnostic suite and real-time telemetry engine engineered in pure Rust with asynchronous non-blocking I/O. 

Specifically architected for modern BMW UDS diagnostics over DoIP Ethernet (ENET) and universal SAE J1979 / GM Mode 22 diagnostics over ELM327 Bluetooth/Serial.

---

## Overview

`obd2mon` bridges vehicle communication buses into a unified telemetry stream for desktop and embedded targets. It delivers real-time dashboard instrumentation, dealer-level service functions (ISTA+ equivalent routines), fault memory management, multi-channel datalogging, and garage profile intelligence across heterogeneous vehicle platforms:

- **BMW F-Series / G-Series (e.g. 2011 BMW 535i xDrive F10 / N55):** Communicates via an ENET (RJ45-to-OBD2) cable over TCP/IP using Diagnostics over IP (DoIP, ISO 13400) and Unified Diagnostic Services (UDS, ISO 14229).
- **GM / GMC Platforms (e.g. 2010 GMC Acadia 3.6L LLT):** Communicates via ELM327-compatible Bluetooth adapters over serial streams using standard SAE J1979 / ISO 15765-4 (CAN 11-bit 500k) and GM manufacturer-specific Mode 22 PIDs.
- **Universal / Generic OBD-II:** Standard emissions diagnostics (Modes 01, 03, 04, 09) supported on any 1996+ gasoline or 2008+ CAN-bus equipped passenger vehicle.
- **Hardware-Free Simulation Engine:** Physics-based virtual vehicle simulator for automated testing and CI/CD without physical vehicle access.

---

## Key Features

### 1. Real-Time Telemetry Dashboard
- High-frequency engine speed (RPM) circular dial with animated redline indicators.
- Scalable digital speedometer with dynamic **KM/H & MPH** unit toggling.
- Engine coolant temperature bar gauge with safe operating zone tracking (75°C - 105°C).
- Engine oil temperature telemetry with automatic graceful fallback when unsupported.

### 2. Diagnostic Trouble Code (DTC) Scanner
- Read and decode stored, pending, and confirmed fault codes across all vehicle systems.
- Offline fault code database with detailed cause and failure descriptions.
- Safety-interlocked fault code clearing across all vehicle control units.

### 3. All-Module ECU Network Topology Map
- Interrogates 16 electronic control units across 4 discrete chassis communication buses:
  - **PT-CAN (500 kbps):** DME (`0x0012`), EGS (`0x0018`), DSC (`0x0029`), EKPS (`0x0017`), GWS (`0x005E`).
  - **FlexRay (10 Mbps):** EPS (`0x0030`), ICM (`0x001C`), EMF (`0x0026`).
  - **K-CAN (100 kbps):** CAS (`0x0040`), KOMBI (`0x0060`), IHKA (`0x0078`), FRM (`0x0072`), JBE (`0x0070`), PDC (`0x0067`).
  - **Ethernet (100 Mbps):** ZGW (`0x0010`), HU_NBT (`0x0063`).
- Visual status color coding (🟢 Healthy, 🟡 Stored Faults, 🔴 Comm Error, ⚪ No Response).
- Interactive ECU Inspector displaying address, bus segment, response latency, and active DTCs.

### 4. Power Management & Battery Registration
- Intelligent Battery Sensor (IBS) live telemetry via DID `0xD010` (SoC %, Terminal Voltage mV, Net Current mA, Core Temp °C).
- Historical battery replacement log via DID `0x2036` (odometer milestones).
- UDS Routine `0x0219` Battery Registration to stamp the odometer and reset alternator charging curves.

### 5. Condition Based Service (CBS) Monitor & Reset
- Real-time remaining life %, mileage distance, and expiration date via DID `0xD020` for:
  - Engine Oil (`0x01`), Front Brakes (`0x02`), Rear Brakes (`0x03`), Brake Fluid (`0x04`), Vehicle Check (`0x05`).
- One-click service interval reset via UDS Routine `0x0220`.

### 6. Actuator Service Routines
- **Coolant System Electric Water Pump Bleeding (Routine `0x0211`):** Automated 12-minute 4-stage purge cycle with safety prerequisites checklist.
- **ZF 8HP Transmission Fluid Temperature & Fill Level (DID `0x2045` & Routine `0x0225`):** Visual 40°C–50°C fill window with torque converter fill RPM hold.
- **Radiator Cooling Fan Step Test (Routine `0x0215`):** PWM duty cycle steps (15%, 50%, 90%, Stop).
- **Muffler Tailpipe Exhaust Flap Actuator (Routine `0x0218`):** Direct toggle between open and closed positions.

### 7. N55 High-Speed Datalogging & Oscilloscope
- Live acquisition (20–50 Hz) of 31 engine channels via DID `0xD100` (Boost Target, Boost Actual, WGDC %, HPFP Rail Pressure, LPFP, Lambda, MAF, IAT, Timing, Knock Cyl 1-6, Misfires, VANOS).
- Real-time multi-trace HTML5 canvas oscilloscope strip chart with customizable channel toggles.
- RFC 4180 compliant CSV recording saved directly to disk and downloadable in-browser for analysis in Virtual Dyno or MegaLogViewer.

### 8. Garage & Saved Vehicles Profile Management
- Multi-vehicle garage manager with persistent profile storage in `%APPDATA%\obd2mon\garage.json`.
- **Dynamic Hardware Compatibility Intelligence Engine:** Evaluates Make, Model, Year, and Interface to recommend optimal cables (ENET, K+DCAN, ELM327), outline supported capabilities, warn about limitations, and show wiring pinouts.
- **VIN Interrogation & Auto-Detection:** Automatically queries vehicle VIN over UDS DID `0xF190` or SAE Mode 09 PID `0x02` and matches against saved garage vehicles.
- **Persistent Per-Vehicle Logbooks (`vehicles/<id>/history.json`):** Automatically logs all battery registrations, CBS resets, bleed cycles, transmission fills, DTC scans, coding changes, datalog sessions, and manual maintenance notes.

### 9. Vehicle Coding & Feature Customization (FDL / UDS)
- **12 Configurable Parameters:** Warning gong tone (Rolls-Royce, BMW i, Mini), seat belt chimes, welcome light duration, pathway lighting, comfort blinker pulses, angel eye brightness, fog lights with high beams, auto-fold mirrors on lock, auto-unlock on engine shutdown, auto start/stop memory, digital speedometer in cluster, and iDrive legal disclaimer removal.
- **Target Modules:** Direct memory updates across KOMBI (`0x0060`), CAS (`0x0040`), FRM (`0x0072`), and HU_NBT (`0x0063`).
- **Safety Protocol & Voltage Interlock:** Enforces IBS battery voltage > 12.0V prior to write dispatch to prevent EEPROM corruption.
- **Automated Rollback Archive:** Generates timestamped pre-coding snapshots with one-click restore and immediate ECU soft reset (Service `0x11 0x03`).

### 10. In-App Technical Documentation & Auto-Updater
- 11-chapter technical reference built directly into the UI.
- Integrated GitHub release updater checking `bangsmackpow/obd2mon` releases.

### 11. Windows WiX Installer
- Generates an `obd2mon-installer.msi` package that installs `obd2mon-app.exe` (GUI) and `obd-cli.exe` (CLI), adds an `obd2mon` Desktop Shortcut, and registers the installation directory on the system PATH.

---

## Repository Layout

```text
obd2mon/
├── Cargo.toml                       # Workspace manifest
├── AGENTS.md                        # Operational instructions for AI agents & developers
├── README.md                        # Project documentation
├── docs/
│   └── FEATURES.md                  # Comprehensive feature & technical guide
├── wix/
│   └── main.wxs                     # WiX Toolset Windows installer definition
├── .github/
│   └── workflows/
│       └── build.yml                # Windows & Android CI/CD release pipeline
├── crates/
│   ├── obd-core/                    # Platform-agnostic diagnostic engine
│   │   ├── src/
│   │   │   ├── lib.rs               # Library root & public exports
│   │   │   ├── error.rs             # ObdError enum & ISO 14229 NRC translations
│   │   │   ├── connection.rs        # ObdConnection trait & state machine
│   │   │   ├── profile.rs           # VehicleProfile & telemetry models
│   │   │   ├── manager.rs           # VehicleManager background loop & telemetry channels
│   │   │   ├── garage.rs            # Garage, vehicle profiles, compatibility & logbooks
│   │   │   ├── dtc_db.rs            # Offline DTC database & translations
│   │   │   ├── updater.rs           # GitHub release auto-updater checker
│   │   │   ├── protocols/
│   │   │   │   ├── doip.rs          # ISO 13400 framing & Routing Activation
│   │   │   │   ├── uds.rs           # ISO 14229 Services (0x10, 0x19, 0x22, 0x2E, 0x11, 0x31, 0x04)
│   │   │   │   └── elm327.rs        # SAE J1979 Modes 01, 03, 04, 09, 22 parsers
│   │   │   └── adapters/
│   │   │       ├── enet.rs          # BMW ENET TCP client (169.254.1.1:6801)
│   │   │       ├── bluetooth.rs     # GMC ELM327 Bluetooth/Serial adapter
│   │   │       └── mock.rs          # Virtual physics & vehicle simulator
│   │   └── tests/
│   │       └── simulator_test.rs    # Core integration test suite
│   ├── app-gui/                     # Desktop GUI frontend (Tauri v2)
│   │   ├── src-tauri/
│   │   │   ├── Cargo.toml           # Binary package: obd2mon-app
│   │   │   ├── tauri.conf.json      # Tauri app configuration & permissions
│   │   │   └── src/
│   │   │       ├── lib.rs           # Tauri IPC commands & manager lifecycle
│   │   │       └── main.rs          # Application entrypoint
│   │   └── ui/
│   │       ├── index.html           # Single-page diagnostic application UI
│   │       ├── app.css              # Cyber-automotive styling & responsive layout
│   │       └── app.js               # Frontend controller, SVG gauges & chart canvas
│   └── obd-cli/                     # Headless command-line diagnostic tool
│       └── src/
│           └── main.rs              # Real-time console telemetry runner
```

---

## Quickstart

### Prerequisites
- **Rust 1.80+** (`cargo`, `rustc`)
- **Windows 10/11** or **Linux**
- **WebView2 Runtime** (pre-installed on Windows 10/11)

### Running the Desktop Application (GUI)

Launch the full graphical dashboard:

```bash
cargo run --bin obd2mon-app
```

Select your vehicle connection profile from the top toolbar:
- **BMW F10 (Simulated):** Simulates live N55 engine telemetry, fault codes, CBS monitors, battery sensors, and actuator routines.
- **GMC Acadia (Simulated):** Simulates GM OBD-II telemetry with graceful oil temperature fallback.
- **Physical BMW ENET:** Connect via RJ45 ENET cable to `169.254.1.1:6801`.
- **Physical ELM327:** Connect via paired Bluetooth/Serial COM port at 38400 baud.

### Running the Terminal Telemetry Runner (CLI)

```bash
# 1. Physics Simulation Mode (Default)
cargo run --bin obd-cli -- mock

# 2. Simulated BMW F10 Profile (DoIP / UDS)
cargo run --bin obd-cli -- bmw

# 3. Simulated GMC Acadia Profile (ELM327 / Mode 22)
cargo run --bin obd-cli -- gmc

# 4. Live Vehicle Connection over ENET Cable
cargo run --bin obd-cli -- enet 169.254.1.1 6801
```

---

## Developer & Verification Toolchain

Ensure all checks pass before submitting pull requests:

```bash
# Verify formatting
cargo fmt --all -- --check

# Run all 49+ unit and integration tests
cargo test --workspace

# Run strict clippy linting
cargo clippy --all-targets -- -D warnings

# Build optimized release binaries
cargo build --release --bin obd-cli --bin obd2mon-app
```

For AI agents and automated development instructions, refer to [AGENTS.md](AGENTS.md).  
For the complete technical feature specification, refer to [docs/FEATURES.md](docs/FEATURES.md).

---

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
