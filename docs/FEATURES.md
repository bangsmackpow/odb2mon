# odb2mon — Feature & Technical Reference Guide

A modern, high-performance cross-platform vehicle diagnostics application built with Rust and Tauri. Specifically engineered for BMW F-series vehicles (e.g. 2011 BMW 535i xDrive F10 / N55) communicating over ENET (Ethernet-to-OBD2 via DoIP and UDS), with fallback support for standard SAE J1979 / ELM327 Bluetooth serial adapters.

---

## 1. Architecture & Protocol Engine

### ENET / DoIP (ISO 13400)
- **Physical Interface:** RJ45 Ethernet to OBD2 port connector (Pins 3, 11, 12, 8).
- **Transport:** TCP socket connecting to the Central Gateway Module (ZGW) at `169.254.1.1:6801`.
- **DoIP Handshake:**
  - Routing Activation Request (`0x0005`) sent with source tester address `0x0E00`.
  - Routing Activation Response (`0x0006`) validates active socket connection.
- **Diagnostic Message Wrapper (`0x8001`):**
  - All UDS diagnostic payloads are framed inside DoIP diagnostic message frames targeting specific ECU addresses (e.g., DME `0x0012`, EGS `0x0018`, DSC `0x0029`).

### Unified Diagnostic Services (UDS / ISO 14229)
- **Session Control (`0x10`):** Seamless automatic elevation from Default Session (`0x01`) to Extended Diagnostic Session (`0x03`) for service routines and protected DIDs.
- **Read Data by Identifier (`0x22`):** Direct querying of proprietary BMW manufacturer DIDs.
- **Routine Control (`0x31`):** Execution of start (`0x01`), stop (`0x02`), and request results (`0x03`) subfunctions for calibration and actuation routines.
- **Fault Memory Management (`0x19` & `0x04`):** Read DTC information with status availability masks (`0x19 0x02`) and group fault clearing (`0x04 0xFFFFFF`).

### ELM327 / SAE J1979 (Emissions OBD2)
- **Transport:** Bluetooth RFCOMM or USB Serial port (default `COM3`, 38400 baud).
- **Standard Services:** Mode 01 (Live Current Data), Mode 03 (Confirmed DTCs), Mode 04 (Clear DTCs), Mode 22 (Manufacturer Enhanced PIDs).
- **Graceful Fallbacks:** Non-blocking support for ECUs lacking specific sensor PIDs (e.g. GMC Acadia oil temp fallback).

---

## 2. Live Dashboard Telemetry
- **Engine Speed (RPM):** High-frequency 10 Hz polling via UDS DID or SAE Mode 01 PID 0x0C. Visual geometric circular SVG gauge with redline indicator.
- **Vehicle Speed:** Scalable digital speedometer supporting dynamic KM/H and MPH toggle.
- **Coolant Temperature:** Digital readout and bar gauge with safe operating zone tracking (75°C - 105°C).
- **Engine Oil Temperature:** Live oil temperature telemetry with automatic graceful fallback when unsupported.

---

## 3. Diagnostic Trouble Codes (DTC)
- **ISO 14229 & SAE DTC Parsing:** 3-byte hex DTC translation to standard codes (e.g., `120408`, `130104`, `P0008`).
- **Status Mask Filtering:** Distinguishes between `Confirmed`, `Pending`, and `Permanent` faults.
- **Embedded Database:** Offline lookup database providing detailed failure descriptions.
- **Safety Interlocked Clearing:** Clear operation resets DTC memory across all controllers with safety confirmation modal.

---

## 4. All-Module ECU Network Topology Scan
- **Multi-Bus Interrogation:** Scans 16 electronic control units across 4 discrete chassis communication buses:
  - **PT-CAN (Powertrain, 500 kbps):** DME (`0x0012`), EGS (`0x0018`), DSC (`0x0029`), EKPS (`0x0017`), GWS (`0x005E`).
  - **FlexRay (Chassis & Dynamics, 10 Mbps):** EPS (`0x0030`), ICM (`0x001C`), EMF (`0x0026`).
  - **K-CAN (Body & Interior, 100 kbps):** CAS (`0x0040`), KOMBI (`0x0060`), IHKA (`0x0078`), FRM (`0x0072`), JBE (`0x0070`), PDC (`0x0067`).
  - **Ethernet (Gateway & Multimedia, 100 Mbps):** ZGW (`0x0010`), HU_NBT (`0x0063`).
- **ISTA-Style Visual Tree:**
  - 🟢 **Normal (Green):** Responding with zero stored faults.
  - 🟡 **Fault Stored (Amber):** Responding with persistent diagnostic trouble codes.
  - 🔴 **Comm Error (Red):** Returned Negative Response Code (NRC) or protocol framing error.
  - ⚪ **No Response (Grey):** Unresponsive or unequipped controller.
- **ECU Inspector Modal:** Click any module block to view its hex address, bus segment, latency in milliseconds, and active fault memory.

---

## 5. Intelligent Battery Sensor (IBS) & Power Management
- **DID 0xD010 Live Telemetry:**
  - State of Charge (SoC %): Battery capacity percentage.
  - Terminal Voltage: Precise mV resolution (Resting ~12.6V, Alternator charging ~14.2V).
  - Net Current: Signed amperage showing net vehicle discharge (-) or alternator charge (+).
  - Core Temperature: Sensor temperature measured at negative battery terminal.
- **DID 0x2036 Replacement History:**
  - 3-entry historical odometer log recording previous replacement milestones.
- **UDS Routine 0x0219 Battery Registration:**
  - Commands the BMW DME to register a new battery installation.
  - Resets internal battery aging metrics, restores aggressive alternator charging curves to prevent premature wear on new AGM plates, and stamps the current odometer.

---

## 6. Condition Based Service (CBS) Monitor & Reset
- **DID 0xD020 Maintenance Tracking:**
  - Queries live remaining life percentage, remaining mileage distance, and due calendar date for:
    - **Engine Oil** (Routine Target `0x01`)
    - **Front Brake Pads** (Routine Target `0x02`)
    - **Rear Brake Pads** (Routine Target `0x03`)
    - **Brake Fluid** (Routine Target `0x04`)
    - **Vehicle Check / Inspection** (Routine Target `0x05`)
- **UDS Routine 0x0220 Individual Consumable Reset:**
  - Directly resets individual maintenance items to 100% life after service.
  - Advances calendar inspection intervals and increments vehicle reset counters.

---

## 7. Actuator Service Routines
- **Coolant System Electric Water Pump Bleed (Routine 0x0211 on DME):**
  - Automated 12-minute pulsing cycle running the 400W electric coolant pump through 4 distinct stages (priming, intermediate displacement, high-velocity purge, stabilization) to extract air pockets.
  - Includes safety verification: Ignition ON, Engine OFF, expansion tank cap sealed, battery charger connected, and climate control set to MAX heat with lowest fan.
- **ZF 8HP Transmission Fluid Temperature & Fill Level (EGS 0x0018):**
  - Reads sump fluid temperature via DID `0x2045`.
  - Visual target window bar highlights the required **40°C to 50°C** fill window to prevent incorrect oil level from thermal fluid expansion.
  - Executes Routine `0x0225` on EGS to hold torque converter fill RPM during fluid level adjustment.
- **Radiator Electric Cooling Fan Step Test (Routine 0x0215 on DME):**
  - Overrides engine cooling logic to test variable-speed PWM fan at 15% (Low), 50% (Medium), and 90% (High), with one-click return to automatic control.
- **Muffler Exhaust Flap Actuator (Routine 0x0218 on DME):**
  - Actuates tailpipe flap between forced open (sport exhaust sound) and forced closed (quiet cold-start).
