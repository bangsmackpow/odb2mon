# AGENTS.md — Agent & Developer Operational Guide

Welcome to the **obd2mon** repository. This document serves as the primary technical specification, architectural guide, and operational runbook for AI agents and human developers maintaining and extending this codebase.

---

## 1. Project Mission & System Architecture

`obd2mon` is a high-performance, cross-platform automotive diagnostic suite written in pure Rust. It targets modern European vehicles (specifically BMW F-series / G-series communicating via ISO 13400 DoIP and ISO 14229 UDS over Ethernet/ENET) and American/universal OBD-II vehicles (such as GM / GMC communicating via SAE J1979 and manufacturer-enhanced Mode 22 PIDs over ELM327 Bluetooth/Serial).

### Core Architectural Directives
1. **Strict Protocol Abstraction:** The core library (`crates/obd-core`) defines the `ObdConnection` trait. Diagnostic dialect logic (UDS vs. SAE J1979) and physical transport mechanisms (TCP/DoIP vs. Serial/Bluetooth vs. Simulation) are strictly isolated.
2. **Decoupled Asynchronous I/O:** Bus interrogation runs on dedicated background worker tasks (`tokio`). Bus latency or vehicle timeouts never block the UI or command processors. State is propagated using `tokio::sync::watch` (latest state) and `tokio::sync::mpsc` (event streams).
3. **Graceful PID Degradation:** On platforms or controllers where specific telemetry channels are unsupported (e.g., Mode 22 Oil Temperature on 2010 GM platforms), the engine traps Negative Response Codes (NRCs) and `NO DATA` responses, returning `ParameterValue::Unsupported` without thread stalls or panics.
4. **Session Elevation Automation:** For BMW UDS DIDs requiring elevated privileges (NRC `0x22` - Conditions Not Correct), the adapter automatically negotiates Extended Diagnostic Session (`0x10 0x03`) and retries the transaction.
5. **Strict Naming Standard:** Always use **`obd`** (On-Board Diagnostics) and **`obd2mon`** across all code, identifiers, binaries, and documentation. Never use `odb`.

---

## 2. Workspace Organization

```text
obd2mon/
├── Cargo.toml                       # Workspace root manifest
├── AGENTS.md                        # Agent & developer operational guidelines
├── README.md                        # Project overview & quickstart
├── docs/
│   └── FEATURES.md                  # Comprehensive feature & technical guide
├── wix/
│   └── main.wxs                     # WiX Toolset Windows installer definition
├── .github/
│   └── workflows/
│       └── build.yml                # Windows & Android CI/CD release pipeline
├── crates/
│   ├── obd-core/                    # Platform-agnostic diagnostic engine
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs               # Library root & public re-exports
│   │   │   ├── error.rs             # ObdError enum & ISO 14229 NRC translations
│   │   │   ├── connection.rs        # ObdConnection trait & state machine
│   │   │   ├── profile.rs           # VehicleProfile & telemetry models
│   │   │   ├── manager.rs           # VehicleManager background loop & telemetry channels
│   │   │   ├── garage.rs            # Garage, vehicle profiles, compatibility & logbooks
│   │   │   ├── dtc_db.rs            # Offline DTC database & translations
│   │   │   ├── updater.rs           # GitHub release auto-updater checker
│   │   │   ├── protocols/
│   │   │   │   ├── doip.rs          # ISO 13400 framing & Routing Activation
│   │   │   │   ├── uds.rs           # ISO 14229 Services (0x10, 0x19, 0x22, 0x31, 0x04)
│   │   │   │   └── elm327.rs        # SAE J1979 Modes 01, 03, 04, 09, 22 parsers
│   │   │   └── adapters/
│   │   │       ├── enet.rs          # BMW ENET TCP client (169.254.1.1:6801)
│   │   │       ├── bluetooth.rs     # GMC ELM327 Bluetooth/Serial adapter
│   │   │       └── mock.rs          # Virtual physics & vehicle simulator
│   │   └── tests/
│   │       └── simulator_test.rs    # End-to-end integration test suite
│   ├── app-gui/                     # Desktop GUI frontend (Tauri v2)
│   │   ├── Cargo.toml
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
│       ├── Cargo.toml
│       └── src/
│           └── main.rs              # Real-time console telemetry runner
```

---

## 3. Essential Commands & Verification Workflows

Always verify modifications through this standard toolchain before committing or completing work:

### 1. Code Formatting
```bash
cargo fmt --all -- --check
```
*To automatically format:*
```bash
cargo fmt --all
```

### 2. Unit & Integration Testing
```bash
cargo test --workspace
```
*Expected result: 49+ tests passing across `obd-core`, `simulator_test.rs`, and binary crates.*

### 3. Clippy & Linter Verification
```bash
cargo clippy --all-targets -- -D warnings
```
*Note: GitHub Actions enforces `-D warnings`. Zero warnings are permitted.*

### 4. Running the Desktop Application
```bash
cargo run --bin obd2mon-app
```

### 5. Running the Command-Line Interface
```bash
# Physics Simulation Mode
cargo run --bin obd-cli -- mock

# Simulated BMW F10 (ENET / UDS)
cargo run --bin obd-cli -- bmw

# Simulated GMC Acadia (ELM327 / Mode 22)
cargo run --bin obd-cli -- gmc

# Live Vehicle Connection over ENET
cargo run --bin obd-cli -- enet 169.254.1.1 6801
```

### 6. Compiling Release Executables
```bash
cargo build --release --bin obd-cli --bin obd2mon-app
```
Binaries will be placed in `target/release/obd2mon-app.exe` and `target/release/obd-cli.exe`.

---

## 4. Vehicle Protocols & Reference Specifications

### Unified Diagnostic Services (UDS / ISO 14229)
- **DoIP Port & IP:** `169.254.1.1:6801` (BMW Central Gateway Module ZGW).
- **Session Elevation:** Service `0x10 0x03` (Extended Diagnostic Session).
- **Key DIDs:**
  - `0xD001`: Engine Speed (RPM, 2 bytes, `val / 4.0`).
  - `0xD002`: Vehicle Road Speed (km/h, 1 byte).
  - `0xD003`: Engine Coolant Temperature (1 byte, `raw - 48°C`).
  - `0xD004`: Engine Oil Temperature (1 byte, `raw - 48°C`).
  - `0xD010`: Intelligent Battery Sensor (SoC %, Voltage mV, Net Current mA, Temp °C).
  - `0x2036`: Battery Replacement History (3 odometer milestones).
  - `0xD020`: Condition Based Service (CBS) Consumables status.
  - `0x2045`: ZF 8HP Transmission Sump Temperature (`raw - 40°C`).
  - `0xD100`: N55 High-Speed Datalog Frame (Boost, WGDC, HPFP, LPFP, Knock, Misfires).
  - `0xF190`: Vehicle Identification Number (17-character ASCII VIN).
- **Key Service Routines (Service 0x31):**
  - `0x0219`: Battery Registration (stamps odometer, resets aging curve).
  - `0x0220`: CBS Consumable Life Reset (Oil `0x01`, Front Brakes `0x02`, Rear Brakes `0x03`, Brake Fluid `0x04`, Vehicle Check `0x05`).
  - `0x0211`: Coolant System Electric Water Pump Bleeding (12-minute cycle).
  - `0x0225`: ZF 8HP Transmission Fluid Fill Level Check (RPM elevation).
  - `0x0215`: Radiator Electric Cooling Fan Step Test (15%, 50%, 90%, Stop).
  - `0x0218`: Muffler Tailpipe Exhaust Flap Actuator (Open / Closed).
- **Fault Codes:** Service `0x19 0x02 0xFF` (Read DTCs by Status Mask); Service `0x04 FF FF FF` (Clear All DTCs).
- **Vehicle Coding (FDL Customization):**
  - UDS Services: `0x22` (Read DID), `0x2E` (Write DID), `0x11 0x03` (ECU Soft Reset), `0x10 0x03` (Extended Session).
  - Target ECUs: `KOMBI (0x0060)`, `CAS (0x0040)`, `FRM (0x0072)`, `HU_NBT (0x0063)`.
  - Voltage Interlock: Requires voltage >= 12.0V prior to write dispatch to prevent EEPROM corruption.
  - Automatic Rollback Backups: Pre-coding state archived under `coding_backups/`.

### SAE J1979 / ELM327 Standard
- **Transport:** Bluetooth RFCOMM or Serial COM port (e.g. `COM3`, default 38400 baud).
- **Modes:**
  - Mode `01`: Live Current Telemetry (`0C` RPM, `0D` Speed, `05` Coolant).
  - Mode `03`: Read Confirmed Diagnostic Trouble Codes.
  - Mode `04`: Clear Diagnostic Trouble Codes & Reset Freeze Frame.
  - Mode `09 PID 02`: Read Vehicle Identification Number (VIN).
  - Mode `22 PID 1154`: GM Enhanced Transmission / Oil Sump Temperature.

---

## 5. Storage & Persistence Layout

User configuration and per-vehicle logbooks are persisted under `%APPDATA%\obd2mon\`:

- **Garage Manifest:** `%APPDATA%\obd2mon\garage.json`
  - Stores all saved vehicle records (Make, Model, Year, VIN, Mileage, Interface, Notes).
  - Tracks `active_vehicle_id`.
- **Per-Vehicle Logbook:** `%APPDATA%\obd2mon\vehicles\<vehicle_id>\history.json`
  - `service_logs`: Automated timestamps for battery registrations, CBS resets, bleed routines, transmission fills, DTC clears, coding changes, and manual notes.
  - `dtc_history`: Historical DTC scans and fault codes.
  - `datalog_logs`: CSV recording archives (duration, sample counts, disk paths).
- **Pre-Coding Backups:** `%APPDATA%\obd2mon\vehicles\<vehicle_id>\coding_backups\`
  - Individual timestamped JSON snapshots recording previous values, byte offsets, and hex payloads for instant one-click rollback.
- **Compatibility Engine:** Evaluates hardware requirements dynamically without network access (`evaluate_compatibility()`).
- **Legacy Fallback:** If `%APPDATA%\odb2mon` exists from older test builds, `get_garage_directory()` transparently migrates or reads from it.

---

## 6. Coding & Clippy Conventions

1. **Strict Range Checks:** Clippy requires `(min..=max).contains(&val)` over `val >= min && val <= max`.
2. **Path Handling:** Use `PathBuf` and `std::env::var("APPDATA")` for Windows user paths.
3. **Markdown Links:** In responses and documentation, use clickable GitHub-style links with `file:///` format for files and symbols.
4. **Preserve Comments:** Retain existing comments, protocol references, and docstrings when editing files.
