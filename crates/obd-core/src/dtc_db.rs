//! Diagnostic Trouble Code (DTC) Offline Database
//!
//! Provides embedded, fast, zero-allocation lookup of Diagnostic Trouble Codes
//! across standard SAE J2012 OBD-II codes, GM powertrain codes, and BMW ISTA / Rheingold
//! DME fault codes. Operates 100% offline without cellular or Wi-Fi access.

/// Look up a known plain-English description for a standardized OBD-II or BMW DTC.
///
/// Returns `Some(&'static str)` if the code is in the offline database,
/// or `None` if it is an unrecognized code.
pub fn lookup_known_dtc(code: &str) -> Option<&'static str> {
    let normalized = code.trim().to_ascii_uppercase();
    match normalized.as_str() {
        // =========================================================================
        // 1. SAE J2012 / Standard Powertrain Codes (P0xxx) & GM Codes (P0008, etc.)
        // =========================================================================
        // Camshaft / Crankshaft Timing & Correlation
        "P0008" => Some("Engine Position System Performance Bank 1 (Timing Chain Slack)"),
        "P0009" => Some("Engine Position System Performance Bank 2 (Timing Chain Slack)"),
        "P0010" => Some("Camshaft Position Actuator Circuit / Open (Bank 1)"),
        "P0011" => Some("Camshaft Position A - Timing Over-Advanced or System Performance (Bank 1)"),
        "P0012" => Some("Camshaft Position A - Timing Over-Retarded (Bank 1)"),
        "P0013" => Some("Camshaft Position B - Actuator Circuit / Open (Bank 1)"),
        "P0014" => Some("Camshaft Position B - Timing Over-Advanced or System Performance (Bank 1)"),
        "P0015" => Some("Camshaft Position B - Timing Over-Retarded (Bank 1)"),
        "P0016" => Some("Crankshaft Position - Camshaft Position Correlation (Bank 1 Sensor A)"),
        "P0017" => Some("Crankshaft Position - Camshaft Position Correlation (Bank 1 Sensor B)"),
        "P0018" => Some("Crankshaft Position - Camshaft Position Correlation (Bank 2 Sensor A)"),
        "P0019" => Some("Crankshaft Position - Camshaft Position Correlation (Bank 2 Sensor B)"),

        // Fuel Pressure & Delivery
        "P0087" => Some("Fuel Rail/System Pressure - Too Low"),
        "P0088" => Some("Fuel Rail/System Pressure - Too High"),
        "P0089" => Some("Fuel Pressure Regulator 1 Performance"),
        "P0090" => Some("Fuel Pressure Regulator 1 Control Circuit"),

        // Oxygen (O2) Sensor Heater & Circuits
        "P0030" => Some("HO2S Heater Control Circuit (Bank 1 Sensor 1)"),
        "P0031" => Some("HO2S Heater Control Circuit Low (Bank 1 Sensor 1)"),
        "P0032" => Some("HO2S Heater Control Circuit High (Bank 1 Sensor 1)"),
        "P0036" => Some("HO2S Heater Control Circuit (Bank 1 Sensor 2)"),
        "P0050" => Some("HO2S Heater Control Circuit (Bank 2 Sensor 1)"),
        "P0056" => Some("HO2S Heater Control Circuit (Bank 2 Sensor 2)"),

        // Mass Air Flow & Intake Sensors
        "P0100" => Some("Mass or Volume Air Flow Circuit"),
        "P0101" => Some("Mass or Volume Air Flow Circuit Range/Performance"),
        "P0102" => Some("Mass or Volume Air Flow Circuit Low Input"),
        "P0103" => Some("Mass or Volume Air Flow Circuit High Input"),
        "P0106" => Some("Manifold Absolute Pressure/Barometric Pressure Circuit Range/Performance"),
        "P0107" => Some("Manifold Absolute Pressure/Barometric Pressure Circuit Low Input"),
        "P0108" => Some("Manifold Absolute Pressure/Barometric Pressure Circuit High Input"),
        "P0111" => Some("Intake Air Temperature Sensor 1 Circuit Range/Performance"),
        "P0112" => Some("Intake Air Temperature Sensor 1 Circuit Low"),
        "P0113" => Some("Intake Air Temperature Sensor 1 Circuit High"),

        // Engine Coolant & Thermostat
        "P0115" => Some("Engine Coolant Temperature Circuit"),
        "P0116" => Some("Engine Coolant Temperature Circuit Range/Performance"),
        "P0117" => Some("Engine Coolant Temperature Circuit Low"),
        "P0118" => Some("Engine Coolant Temperature Circuit High"),
        "P0125" => Some("Insufficient Coolant Temperature for Closed Loop Fuel Control"),
        "P0128" => Some("Coolant Thermostat (Coolant Temp Below Regulating Temperature)"),

        // Throttle Position
        "P0120" => Some("Throttle/Pedal Position Sensor/Switch A Circuit"),
        "P0121" => Some("Throttle/Pedal Position Sensor/Switch A Circuit Range/Performance"),
        "P0122" => Some("Throttle/Pedal Position Sensor/Switch A Circuit Low"),
        "P0123" => Some("Throttle/Pedal Position Sensor/Switch A Circuit High"),

        // O2 Sensor Signals
        "P0130" => Some("O2 Sensor Circuit (Bank 1 Sensor 1)"),
        "P0131" => Some("O2 Sensor Circuit Low Voltage (Bank 1 Sensor 1)"),
        "P0132" => Some("O2 Sensor Circuit High Voltage (Bank 1 Sensor 1)"),
        "P0133" => Some("O2 Sensor Circuit Slow Response (Bank 1 Sensor 1)"),
        "P0134" => Some("O2 Sensor Circuit No Activity Detected (Bank 1 Sensor 1)"),
        "P0135" => Some("O2 Sensor Heater Circuit (Bank 1 Sensor 1)"),
        "P0137" => Some("O2 Sensor Circuit Low Voltage (Bank 1 Sensor 2)"),
        "P0138" => Some("O2 Sensor Circuit High Voltage (Bank 1 Sensor 2)"),
        "P0140" => Some("O2 Sensor Circuit No Activity Detected (Bank 1 Sensor 2)"),
        "P0141" => Some("O2 Sensor Heater Circuit (Bank 1 Sensor 2)"),
        "P0150" => Some("O2 Sensor Circuit (Bank 2 Sensor 1)"),
        "P0155" => Some("O2 Sensor Heater Circuit (Bank 2 Sensor 1)"),
        "P0161" => Some("O2 Sensor Heater Circuit (Bank 2 Sensor 2)"),

        // Fuel Trim (Lean / Rich)
        "P0170" => Some("Fuel Trim Malfunction (Bank 1)"),
        "P0171" => Some("System Too Lean (Bank 1)"),
        "P0172" => Some("System Too Rich (Bank 1)"),
        "P0173" => Some("Fuel Trim Malfunction (Bank 2)"),
        "P0174" => Some("System Too Lean (Bank 2)"),
        "P0175" => Some("System Too Rich (Bank 2)"),

        // Fuel Injectors
        "P0200" => Some("Injector Circuit / Open"),
        "P0201" => Some("Injector Circuit / Open - Cylinder 1"),
        "P0202" => Some("Injector Circuit / Open - Cylinder 2"),
        "P0203" => Some("Injector Circuit / Open - Cylinder 3"),
        "P0204" => Some("Injector Circuit / Open - Cylinder 4"),
        "P0205" => Some("Injector Circuit / Open - Cylinder 5"),
        "P0206" => Some("Injector Circuit / Open - Cylinder 6"),
        "P0207" => Some("Injector Circuit / Open - Cylinder 7"),
        "P0208" => Some("Injector Circuit / Open - Cylinder 8"),

        // Turbo / Supercharger Boost
        "P0234" => Some("Turbocharger/Supercharger Overboost Condition"),
        "P0235" => Some("Turbocharger/Supercharger Boost Sensor A Circuit"),
        "P0236" => Some("Turbocharger/Supercharger Boost Sensor A Circuit Range/Performance"),
        "P0299" => Some("Turbocharger/Supercharger Underboost"),

        // Misfires
        "P0300" => Some("Random/Multiple Cylinder Misfire Detected"),
        "P0301" => Some("Cylinder 1 Misfire Detected"),
        "P0302" => Some("Cylinder 2 Misfire Detected"),
        "P0303" => Some("Cylinder 3 Misfire Detected"),
        "P0304" => Some("Cylinder 4 Misfire Detected"),
        "P0305" => Some("Cylinder 5 Misfire Detected"),
        "P0306" => Some("Cylinder 6 Misfire Detected"),
        "P0307" => Some("Cylinder 7 Misfire Detected"),
        "P0308" => Some("Cylinder 8 Misfire Detected"),

        // Crankshaft / Camshaft Sensors & Knock Sensors
        "P0325" => Some("Knock Sensor 1 Circuit (Bank 1 or Single Sensor)"),
        "P0330" => Some("Knock Sensor 2 Circuit (Bank 2)"),
        "P0335" => Some("Crankshaft Position Sensor A Circuit"),
        "P0336" => Some("Crankshaft Position Sensor A Circuit Range/Performance"),
        "P0340" => Some("Camshaft Position Sensor A Circuit (Bank 1 or Single Sensor)"),
        "P0341" => Some("Camshaft Position Sensor A Circuit Range/Performance (Bank 1)"),
        "P0345" => Some("Camshaft Position Sensor A Circuit (Bank 2)"),

        // Catalytic Converter & Evaporative Emissions (EVAP)
        "P0420" => Some("Catalyst System Efficiency Below Threshold (Bank 1)"),
        "P0430" => Some("Catalyst System Efficiency Below Threshold (Bank 2)"),
        "P0440" => Some("Evaporative Emission Control System Malfunction"),
        "P0441" => Some("Evaporative Emission Control System Incorrect Purge Flow"),
        "P0442" => Some("Evaporative Emission Control System Leak Detected (Small Leak)"),
        "P0446" => Some("Evaporative Emission Control System Vent Control Circuit"),
        "P0455" => Some("Evaporative Emission Control System Leak Detected (Gross Leak)"),
        "P0456" => Some("Evaporative Emission Control System Leak Detected (Very Small Leak)"),

        // Vehicle Speed & Idle Speed
        "P0500" => Some("Vehicle Speed Sensor A"),
        "P0505" => Some("Idle Air Control System"),
        "P0506" => Some("Idle Air Control System RPM Lower Than Expected"),
        "P0507" => Some("Idle Air Control System RPM Higher Than Expected"),

        // Computer / ECU / Transmission
        "P0601" => Some("Internal Control Module Memory Checksum Error"),
        "P0602" => Some("Control Module Programming Error"),
        "P0606" => Some("ECM/PCM Processor Fault"),
        "P0700" => Some("Transmission Control System (MIL Request)"),
        "P0730" => Some("Incorrect Gear Ratio"),
        "P0741" => Some("Torque Converter Clutch Circuit Performance or Stuck Off"),

        // =========================================================================
        // 2. Chassis (C0xxx), Body (B0xxx), and Network (U0xxx) Codes
        // =========================================================================
        "C0035" => Some("Left Front Wheel Speed Sensor Circuit"),
        "C0040" => Some("Right Front Wheel Speed Sensor Circuit"),
        "C0045" => Some("Left Rear Wheel Speed Sensor Circuit"),
        "C0050" => Some("Right Rear Wheel Speed Sensor Circuit"),
        "B0001" => Some("Driver Frontal Stage 1 Deployment Control"),
        "B0028" => Some("Right Side Airbag Deployment Control"),
        "U0001" => Some("High Speed CAN Communication Bus"),
        "U0100" => Some("Lost Communication With ECM/PCM A"),
        "U0101" => Some("Lost Communication With TCM (Transmission Control Module)"),
        "U0121" => Some("Lost Communication With Anti-Lock Brake System (ABS) Control Module"),
        "U0140" => Some("Lost Communication With Body Control Module (BCM)"),
        "U0155" => Some("Lost Communication With Instrument Panel Cluster (IPC) Control Module"),

        // =========================================================================
        // 3. BMW ISTA / Rheingold DME Hex Fault Codes (F10 / N55 & Common DME)
        // =========================================================================
        // Boost Pressure & Charging Control
        "120408" => Some("Charging pressure control: Switch-off, pressure build-up disabled"),
        "120308" => Some("Charging pressure control: Pressure too low"),
        "120208" => Some("Charging pressure control: Pressure too high"),
        "123401" => Some("Electric wastegate: Learning limit stop not found"),
        "123402" => Some("Electric wastegate: Learning limit stop out of range"),
        "123431" => Some("Electric wastegate: Plausibility, position not reached"),

        // VANOS Variable Valve Timing
        "130104" => Some("VANOS, intake: Camshaft angle not reached"),
        "130108" => Some("VANOS, intake: Control fault, position not reached"),
        "130304" => Some("VANOS, exhaust: Camshaft angle not reached"),
        "130308" => Some("VANOS, exhaust: Control fault, position not reached"),
        "130E11" => Some("Inlet camshaft: Sensor gear not detected"),
        "130F11" => Some("Exhaust camshaft: Sensor gear not detected"),

        // Valvetronic System
        "133010" => Some("Valvetronic system: Deactivated, adjustment fault"),
        "133011" => Some("Valvetronic system: Warning threshold exceeded"),
        "133013" => Some("Valvetronic system: No movement detected"),
        "135608" => Some("Valvetronic eccentric shaft sensor: Plausibility error"),
        "135704" => Some("Valvetronic servo motor: Position sensor signal implausible"),

        // Air Mass & Intake Pressure
        "101F01" => Some("Throttle valve angle - absolute pressure intake pipe, comparison: Pressure too high"),
        "101F02" => Some("Throttle valve angle - absolute pressure intake pipe, comparison: Pressure too low"),
        "102001" => Some("Air mass, plausibility: Air mass too high"),
        "102002" => Some("Air mass, plausibility: Air mass too low"),

        // Fuel Injection & High Pressure Pump
        "118001" => Some("Mixture control: Mixture too lean"),
        "118002" => Some("Mixture control: Mixture too rich"),
        "11A002" => Some("High-pressure fuel system: Fuel pressure too low"),
        "11A031" => Some("High-pressure fuel system: Low pressure below threshold"),

        // Combustion Misfires (BMW DME)
        "140001" => Some("Combustion misfire, several cylinders: Fuel injection is switched off"),
        "140010" => Some("Combustion misfires, more cylinders: recognized"),
        "140101" => Some("Combustion misfire, cylinder 1: Fuel injection is switched off"),
        "140110" => Some("Combustion misfire, cylinder 1: recognized"),
        "140201" => Some("Combustion misfire, cylinder 2: Fuel injection is switched off"),
        "140210" => Some("Combustion misfire, cylinder 2: recognized"),
        "140301" => Some("Combustion misfire, cylinder 3: Fuel injection is switched off"),
        "140310" => Some("Combustion misfire, cylinder 3: recognized"),
        "140401" => Some("Combustion misfire, cylinder 4: Fuel injection is switched off"),
        "140410" => Some("Combustion misfire, cylinder 4: recognized"),
        "140501" => Some("Combustion misfire, cylinder 5: Fuel injection is switched off"),
        "140510" => Some("Combustion misfire, cylinder 5: recognized"),
        "140601" => Some("Combustion misfire, cylinder 6: Fuel injection is switched off"),
        "140610" => Some("Combustion misfire, cylinder 6: recognized"),

        // Cooling & Thermostat (BMW DME)
        "1D2404" => Some("Map thermostat: Activation line disconnected / open circuit"),
        "1D2402" => Some("Map thermostat: Activation line short-circuit to ground"),
        "10D000" => Some("Coolant pump: Missing transmitter signal"),

        // Legacy 4-Hex Digit BMW DME Codes
        "2C57" => Some("Charging pressure control, plausibility: Pressure too low"),
        "2C58" => Some("Charging pressure control: Switch-off as consequential reaction"),
        "2A82" => Some("VANOS intake: Solenoid valve stiff / jammed"),
        "2A87" => Some("VANOS exhaust: Solenoid valve stiff / jammed"),
        "2EE0" => Some("Coolant temperature sensor: Signal implausible"),
        "2EF4" => Some("Map thermostat: Control line interruption"),
        "2FBC" => Some("High-pressure fuel pump: Delivery pressure plausibility"),

        _ => None,
    }
}

/// Returns a human-readable description for a given trouble code.
///
/// If the code is cataloged in the offline dictionary, returns the verified description.
/// Otherwise, provides an intelligent structural classification based on SAE J2012 / BMW standards.
pub fn lookup_description(code: &str) -> &'static str {
    if let Some(desc) = lookup_known_dtc(code) {
        return desc;
    }

    let trimmed = code.trim().to_ascii_uppercase();
    if trimmed.len() == 5 && trimmed[1..].chars().all(|c| c.is_ascii_hexdigit()) {
        match trimmed.chars().next() {
            Some('P') => {
                if trimmed.starts_with("P0") {
                    return "Standard SAE Powertrain Trouble Code (SAE J2012)";
                } else {
                    return "Manufacturer-Specific Powertrain Trouble Code";
                }
            }
            Some('C') => {
                if trimmed.starts_with("C0") {
                    return "Standard SAE Chassis Trouble Code (ABS / Suspension)";
                } else {
                    return "Manufacturer-Specific Chassis Trouble Code";
                }
            }
            Some('B') => {
                if trimmed.starts_with("B0") {
                    return "Standard SAE Body Trouble Code (Airbag / Climate)";
                } else {
                    return "Manufacturer-Specific Body Trouble Code";
                }
            }
            Some('U') => {
                if trimmed.starts_with("U0") {
                    return "Standard SAE Network & Communication Trouble Code";
                } else {
                    return "Manufacturer-Specific Network Trouble Code";
                }
            }
            _ => {}
        }
    }

    if (trimmed.len() == 6 || trimmed.len() == 4) && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        if trimmed.len() == 6 {
            return "BMW Electronic Control Unit (DME/DDE) Fault Code";
        } else {
            return "BMW DME Hex Fault Code";
        }
    }

    "Unclassified Diagnostic Trouble Code"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_sae_codes() {
        assert_eq!(
            lookup_description("P0300"),
            "Random/Multiple Cylinder Misfire Detected"
        );
        assert_eq!(
            lookup_description("p0008"),
            "Engine Position System Performance Bank 1 (Timing Chain Slack)"
        );
        assert_eq!(
            lookup_description("P0128"),
            "Coolant Thermostat (Coolant Temp Below Regulating Temperature)"
        );
        assert_eq!(
            lookup_description("P0171"),
            "System Too Lean (Bank 1)"
        );
    }

    #[test]
    fn test_known_bmw_codes() {
        assert_eq!(
            lookup_description("120408"),
            "Charging pressure control: Switch-off, pressure build-up disabled"
        );
        assert_eq!(
            lookup_description("130104"),
            "VANOS, intake: Camshaft angle not reached"
        );
        assert_eq!(
            lookup_description("2c57"),
            "Charging pressure control, plausibility: Pressure too low"
        );
    }

    #[test]
    fn test_unrecognized_fallbacks() {
        assert_eq!(
            lookup_description("P0999"),
            "Standard SAE Powertrain Trouble Code (SAE J2012)"
        );
        assert_eq!(
            lookup_description("P1234"),
            "Manufacturer-Specific Powertrain Trouble Code"
        );
        assert_eq!(
            lookup_description("U0199"),
            "Standard SAE Network & Communication Trouble Code"
        );
        assert_eq!(
            lookup_description("ABCDEF"),
            "BMW Electronic Control Unit (DME/DDE) Fault Code"
        );
        assert_eq!(
            lookup_description("UNKNOWN_CODE"),
            "Unclassified Diagnostic Trouble Code"
        );
    }
}
