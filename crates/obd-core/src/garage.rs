use crate::error::ObdError;
use crate::profile::DtcRecord;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Returns current timestamp in RFC 3339 / ISO 8601 format.
pub fn current_iso_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Complete persistent vehicle record representing a car saved in the user's garage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleRecord {
    pub id: String,
    pub nickname: String,
    pub make: String,
    pub model: String,
    pub year: u16,
    pub vin: Option<String>,
    pub mileage: Option<u32>,
    pub mileage_unit: String, // "mi" or "km"
    pub engine: Option<String>,
    pub interface_type: String, // "enet", "bluetooth", "usb", "simulator"
    pub interface_host: Option<String>,
    pub interface_port: Option<u16>,
    pub serial_port: Option<String>,
    pub serial_baud: Option<u32>,
    pub notes: Option<String>,
    pub created_at: String,
    pub last_connected_at: Option<String>,
    pub active_dtc_count: usize,
}

/// Dynamic compatibility analysis detailing supported capabilities and hardware interface requirements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwareCompatibilityInfo {
    pub make: String,
    pub model: String,
    pub year: u16,
    pub interface_type: String,
    pub recommended_interface: String,
    pub compatible_interfaces: Vec<String>,
    pub supported_features: Vec<String>,
    pub unsupported_features: Vec<String>,
    pub cable_notes: String,
}

/// Service event log entry (e.g. Battery registration, CBS reset, actuator routine).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleServiceLogEntry {
    pub timestamp: String,
    pub mileage: Option<u32>,
    pub category: String, // "battery", "cbs_reset", "actuator_test", "dtc_clear", "note"
    pub description: String,
    pub result: String,
}

/// Diagnostic trouble code historical scan snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleDtcHistoryEntry {
    pub timestamp: String,
    pub mileage: Option<u32>,
    pub dtc_count: usize,
    pub dtcs: Vec<DtcRecord>,
}

/// High-speed datalog session record saved under a specific vehicle profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleDatalogLogEntry {
    pub filename: String,
    pub file_path: String,
    pub timestamp: String,
    pub sample_count: usize,
    pub duration_seconds: f32,
    pub file_size_bytes: usize,
}

/// Complete historical activity and telemetry archives recorded for a specific vehicle.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VehicleHistory {
    pub service_logs: Vec<VehicleServiceLogEntry>,
    pub dtc_history: Vec<VehicleDtcHistoryEntry>,
    pub datalog_logs: Vec<VehicleDatalogLogEntry>,
}

/// Persistent garage state storing all vehicles and active selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Garage {
    pub active_vehicle_id: String,
    pub vehicles: Vec<VehicleRecord>,
}

impl Default for Garage {
    fn default() -> Self {
        let bmw = VehicleRecord {
            id: "veh_bmw_f10_535i".into(),
            nickname: "2011 BMW 535i xDrive".into(),
            make: "BMW".into(),
            model: "535i xDrive (F10)".into(),
            year: 2011,
            vin: Some("WBAFR7C58BC123456".into()),
            mileage: Some(142500),
            mileage_unit: "km".into(),
            engine: Some("3.0L Turbo Inline-6 (N55B30M0)".into()),
            interface_type: "enet".into(),
            interface_host: Some("169.254.1.1".into()),
            interface_port: Some(6801),
            serial_port: None,
            serial_baud: None,
            notes: Some(
                "Factory M-Sport, ZF 8HP45 transmission, pneumatic wastegate (PWG).".into(),
            ),
            created_at: "2026-10-12T00:00:00Z".into(),
            last_connected_at: None,
            active_dtc_count: 2,
        };

        let gmc = VehicleRecord {
            id: "veh_gmc_acadia_2010".into(),
            nickname: "2010 GMC Acadia".into(),
            make: "GMC".into(),
            model: "Acadia SLE".into(),
            year: 2010,
            vin: Some("1GKKVRED9AJ123456".into()),
            mileage: Some(168200),
            mileage_unit: "mi".into(),
            engine: Some("3.6L V6 Direct Injection (LLT)".into()),
            interface_type: "bluetooth".into(),
            interface_host: None,
            interface_port: None,
            serial_port: Some("COM3".into()),
            serial_baud: Some(38400),
            notes: Some(
                "Connected via ELM327 Bluetooth v1.5. Uses GM Mode 22 enhanced PIDs.".into(),
            ),
            created_at: "2026-10-12T00:00:00Z".into(),
            last_connected_at: None,
            active_dtc_count: 2,
        };

        let generic = VehicleRecord {
            id: "veh_generic_obd2".into(),
            nickname: "Generic / General OBD-II".into(),
            make: "General / Generic".into(),
            model: "Standard SAE J1979 Vehicle".into(),
            year: 2015,
            vin: None,
            mileage: None,
            mileage_unit: "mi".into(),
            engine: None,
            interface_type: "bluetooth".into(),
            interface_host: None,
            interface_port: None,
            serial_port: Some("COM3".into()),
            serial_baud: Some(38400),
            notes: Some(
                "Universal emissions diagnostics for any 1996+ gasoline or 2008+ CAN vehicle."
                    .into(),
            ),
            created_at: "2026-10-12T00:00:00Z".into(),
            last_connected_at: None,
            active_dtc_count: 0,
        };

        Self {
            active_vehicle_id: "veh_bmw_f10_535i".into(),
            vehicles: vec![bmw, gmc, generic],
        }
    }
}

/// Evaluates hardware and protocol compatibility for any make, model, year, and interface.
pub fn evaluate_compatibility(
    make: &str,
    model: &str,
    year: u16,
    selected_interface: &str,
) -> HardwareCompatibilityInfo {
    let make_lower = make.to_lowercase();
    let model_lower = model.to_lowercase();
    let is_bmw = make_lower.contains("bmw");
    let is_gm =
        make_lower.contains("gmc") || make_lower.contains("gm") || make_lower.contains("chevrolet");

    if is_bmw {
        let is_f_or_g_series = year >= 2010
            || model_lower.contains("f10")
            || model_lower.contains("f30")
            || model_lower.contains("f80")
            || model_lower.contains("g30")
            || model_lower.contains("g20");

        if is_f_or_g_series {
            if selected_interface == "enet" || selected_interface == "simulator" {
                HardwareCompatibilityInfo {
                    make: make.into(),
                    model: model.into(),
                    year,
                    interface_type: selected_interface.into(),
                    recommended_interface: "BMW ENET (Ethernet-to-OBD2 via ISO 13400 DoIP)".into(),
                    compatible_interfaces: vec!["enet".into(), "simulator".into()],
                    supported_features: vec![
                        "High-Speed Live Telemetry (10 - 50 Hz)".into(),
                        "Full ECU Network Topology Tree Scan (PT-CAN, FlexRay, K-CAN, Ethernet)".into(),
                        "Intelligent Battery Sensor (IBS) & Battery Registration (0x0219)".into(),
                        "Condition Based Service (CBS) Monitor & Reset (0x0220)".into(),
                        "Electric Water Pump Coolant Bleeding Routine (0x0211)".into(),
                        "ZF 8HP Transmission Fluid Temperature (0x2045) & Level Fill (0x0225)".into(),
                        "Radiator Cooling Fan Step Override (0x0215)".into(),
                        "Muffler Exhaust Flap Valve Actuator (0x0218)".into(),
                        "High-Speed N55 Multi-Channel Datalogging & Oscilloscope Chart".into(),
                        "BMW FDL Vehicle Coding & Customizations (Gong, Lights, Mirrors, Start/Stop)".into(),
                    ],
                    unsupported_features: vec![],
                    cable_notes: "Connect using a standard RJ45-to-OBD2 ENET cable. Configure computer network adapter to DHCP (Autoconfig 169.254.x.x) or static IP 169.254.1.2 / 255.255.0.0. ZGW responds at 169.254.1.1:6801.".into(),
                }
            } else {
                HardwareCompatibilityInfo {
                    make: make.into(),
                    model: model.into(),
                    year,
                    interface_type: selected_interface.into(),
                    recommended_interface: "BMW ENET (Ethernet-to-OBD2) - SWITCH TO ENET FOR FULL ISTA FEATURES".into(),
                    compatible_interfaces: vec!["enet".into(), "bluetooth".into(), "usb".into()],
                    supported_features: vec![
                        "Basic SAE J1979 Mode 01 Telemetry (RPM, Speed, Coolant)".into(),
                        "Emissions Fault Codes Read (Mode 03) & Clear (Mode 04)".into(),
                    ],
                    unsupported_features: vec![
                        "All UDS Service Routines (Battery Registration, CBS Reset, Bleed Routine)".into(),
                        "ECU Network Topology Scan (Requires multi-bus DoIP access)".into(),
                        "High-Speed N55 Multi-Channel Datalogging (Bandwidth insufficient)".into(),
                        "ZF 8HP Transmission Fluid Temperature Monitoring".into(),
                    ],
                    cable_notes: "ELM327 Bluetooth/USB adapters only access the engine DME on standard pin 6/14 CAN and cannot communicate with FlexRay, K-CAN, or Ethernet gateways.".into(),
                }
            }
        } else {
            // E-Series BMW (pre-2010, e.g. E90, E60, E46)
            HardwareCompatibilityInfo {
                make: make.into(),
                model: model.into(),
                year,
                interface_type: selected_interface.into(),
                recommended_interface: "BMW K+DCAN USB Interface with Pin 7-8 Switch".into(),
                compatible_interfaces: vec!["usb".into(), "bluetooth".into()],
                supported_features: vec![
                    "Standard SAE J1979 Mode 01 Telemetry".into(),
                    "Emissions Diagnostic Trouble Codes Read & Clear".into(),
                    "K+DCAN EDIABAS Protocol Communication".into(),
                ],
                unsupported_features: vec![
                    "ISO 13400 DoIP (Ethernet is not equipped on E-series OBD2 port)".into(),
                    "N55 High-Speed UDS Datalogging via ENET".into(),
                ],
                cable_notes: "E-Series vehicles require a K+DCAN USB cable. ENET cables will not link because OBD2 pin 8 is Ethernet activation on F-series but second K-line on pre-2007 E-series.".into(),
            }
        }
    } else if is_gm {
        HardwareCompatibilityInfo {
            make: make.into(),
            model: model.into(),
            year,
            interface_type: selected_interface.into(),
            recommended_interface: "ELM327 Bluetooth (v1.5) or OBDLink MX+".into(),
            compatible_interfaces: vec!["bluetooth".into(), "usb".into(), "simulator".into()],
            supported_features: vec![
                "SAE J1979 Mode 01 Live Engine Parameters".into(),
                "Mode 03 Confirmed Diagnostic Trouble Codes".into(),
                "Mode 04 Diagnostic Trouble Codes Clear".into(),
                "Mode 09 Vehicle Identification Number (VIN) Detection".into(),
                "GM Enhanced Mode 22 Engine Sump Temperature (with graceful fallback)".into(),
            ],
            unsupported_features: vec![
                "BMW-specific UDS Service Routines (Battery, CBS, Water Pump Bleed)".into(),
                "BMW Multi-Bus Topology Map".into(),
            ],
            cable_notes: "Pair ELM327 Bluetooth device in Windows Bluetooth settings (default PIN 1234). Set COM port to assigned Outgoing Serial Port at 38400 baud.".into(),
        }
    } else {
        // Universal / Generic OBD-II
        HardwareCompatibilityInfo {
            make: make.into(),
            model: model.into(),
            year,
            interface_type: selected_interface.into(),
            recommended_interface: "Universal ELM327 Bluetooth / USB Scan Tool".into(),
            compatible_interfaces: vec!["bluetooth".into(), "usb".into(), "simulator".into()],
            supported_features: vec![
                "Standard SAE J1979 Mode 01 Emissions Gauges (RPM, Speed, Coolant)".into(),
                "Mode 03 Stored Diagnostic Trouble Codes".into(),
                "Mode 04 Trouble Codes Clearing & Memory Reset".into(),
                "Mode 09 VIN & Calibration ID Auto-Query".into(),
                "State Inspection Readiness Monitor Status".into(),
            ],
            unsupported_features: vec![
                "Proprietary OEM Manufacturer Calibration Routines".into(),
                "Multi-Bus Engineering Topology Scan".into(),
            ],
            cable_notes: "Compatible with any 1996+ gasoline or 2008+ CAN-bus equipped passenger vehicle using standard ELM327 hardware.".into(),
        }
    }
}

/// Resolves user application data directory for persistent garage storage.
pub fn get_garage_directory() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let primary = PathBuf::from(&appdata).join("obd2mon");
        let legacy = PathBuf::from(&appdata).join("odb2mon");
        if !primary.exists() && legacy.exists() {
            legacy
        } else {
            primary
        }
    } else if let Ok(home) = std::env::var("HOME") {
        let primary = PathBuf::from(&home).join(".obd2mon");
        let legacy = PathBuf::from(&home).join(".odb2mon");
        if !primary.exists() && legacy.exists() {
            legacy
        } else {
            primary
        }
    } else {
        PathBuf::from("./data/obd2mon")
    }
}

/// Loads persistent garage state from disk, or initializes default garage if not present.
pub fn load_garage() -> Garage {
    let dir = get_garage_directory();
    let file_path = dir.join("garage.json");

    if let Ok(data) = fs::read_to_string(&file_path) {
        if let Ok(garage) = serde_json::from_str::<Garage>(&data) {
            return garage;
        }
    }

    let default_garage = Garage::default();
    let _ = save_garage(&default_garage);
    default_garage
}

/// Saves persistent garage state to disk.
pub fn save_garage(garage: &Garage) -> Result<(), ObdError> {
    let dir = get_garage_directory();
    fs::create_dir_all(&dir)?;
    let file_path = dir.join("garage.json");
    let json =
        serde_json::to_string_pretty(garage).map_err(|e| ObdError::ParseError(e.to_string()))?;
    fs::write(&file_path, json)?;
    Ok(())
}

/// Retrieves persistent history (service actions, DTC scans, and datalogs) for a specific vehicle.
pub fn load_vehicle_history(vehicle_id: &str) -> VehicleHistory {
    let dir = get_garage_directory().join("vehicles").join(vehicle_id);
    let file_path = dir.join("history.json");

    if let Ok(data) = fs::read_to_string(&file_path) {
        if let Ok(history) = serde_json::from_str::<VehicleHistory>(&data) {
            return history;
        }
    }

    VehicleHistory::default()
}

/// Saves persistent vehicle history to disk.
pub fn save_vehicle_history(vehicle_id: &str, history: &VehicleHistory) -> Result<(), ObdError> {
    let dir = get_garage_directory().join("vehicles").join(vehicle_id);
    fs::create_dir_all(&dir)?;
    let file_path = dir.join("history.json");
    let json =
        serde_json::to_string_pretty(history).map_err(|e| ObdError::ParseError(e.to_string()))?;
    fs::write(&file_path, json)?;
    Ok(())
}

/// Records a service event into the active vehicle profile's history log.
pub fn append_service_log(vehicle_id: &str, entry: VehicleServiceLogEntry) -> Result<(), ObdError> {
    let mut history = load_vehicle_history(vehicle_id);
    history.service_logs.insert(0, entry);
    if history.service_logs.len() > 100 {
        history.service_logs.truncate(100);
    }
    save_vehicle_history(vehicle_id, &history)
}

/// Records a diagnostic trouble code scan into the active vehicle profile's history.
pub fn append_dtc_history(vehicle_id: &str, entry: VehicleDtcHistoryEntry) -> Result<(), ObdError> {
    let mut history = load_vehicle_history(vehicle_id);
    history.dtc_history.insert(0, entry);
    if history.dtc_history.len() > 50 {
        history.dtc_history.truncate(50);
    }
    save_vehicle_history(vehicle_id, &history)
}

/// Records a completed datalog session into the active vehicle profile's archive.
pub fn append_datalog_entry(
    vehicle_id: &str,
    entry: VehicleDatalogLogEntry,
) -> Result<(), ObdError> {
    let mut history = load_vehicle_history(vehicle_id);
    history.datalog_logs.insert(0, entry);
    if history.datalog_logs.len() > 100 {
        history.datalog_logs.truncate(100);
    }
    save_vehicle_history(vehicle_id, &history)
}

/// Saves a coding backup record to persistent storage under the vehicle's directory.
pub fn save_coding_backup(
    vehicle_id: &str,
    record: &crate::profile::CodingBackupRecord,
) -> Result<String, ObdError> {
    let dir = get_garage_directory()
        .join("vehicles")
        .join(vehicle_id)
        .join("coding_backups");
    fs::create_dir_all(&dir)?;

    let file_path = dir.join(format!("{}.json", record.id));
    let json =
        serde_json::to_string_pretty(record).map_err(|e| ObdError::ParseError(e.to_string()))?;
    fs::write(&file_path, json)?;
    Ok(file_path.to_string_lossy().to_string())
}

/// Retrieves all saved coding backup records for a vehicle, sorted newest first.
pub fn list_coding_backups(vehicle_id: &str) -> Vec<crate::profile::CodingBackupRecord> {
    let dir = get_garage_directory()
        .join("vehicles")
        .join(vehicle_id)
        .join("coding_backups");
    if !dir.exists() {
        return Vec::new();
    }

    let mut backups = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(data) = fs::read_to_string(&path) {
                    if let Ok(record) =
                        serde_json::from_str::<crate::profile::CodingBackupRecord>(&data)
                    {
                        backups.push(record);
                    }
                }
            }
        }
    }
    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    backups
}

/// Loads a specific coding backup record by ID.
pub fn load_coding_backup(
    vehicle_id: &str,
    backup_id: &str,
) -> Result<crate::profile::CodingBackupRecord, ObdError> {
    let dir = get_garage_directory()
        .join("vehicles")
        .join(vehicle_id)
        .join("coding_backups");
    let file_path = dir.join(format!("{backup_id}.json"));
    let data = fs::read_to_string(&file_path)
        .map_err(|e| ObdError::ConnectionFailed(format!("Backup file not found: {e}")))?;
    serde_json::from_str(&data).map_err(|e| ObdError::ParseError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_garage_has_three_vehicles() {
        let garage = Garage::default();
        assert_eq!(garage.vehicles.len(), 3);
        assert_eq!(garage.active_vehicle_id, "veh_bmw_f10_535i");
        assert_eq!(garage.vehicles[0].make, "BMW");
        assert_eq!(garage.vehicles[1].make, "GMC");
        assert_eq!(garage.vehicles[2].make, "General / Generic");
    }

    #[test]
    fn test_bmw_f10_enet_compatibility() {
        let compat = evaluate_compatibility("BMW", "535i F10", 2011, "enet");
        assert!(compat.compatible_interfaces.contains(&"enet".to_string()));
        assert!(compat
            .supported_features
            .iter()
            .any(|f| f.contains("Battery")));
        assert!(compat.supported_features.iter().any(|f| f.contains("CBS")));
        assert!(compat
            .supported_features
            .iter()
            .any(|f| f.contains("Coolant Bleeding")));
        assert!(compat.unsupported_features.is_empty());
    }

    #[test]
    fn test_bmw_f10_bluetooth_warning() {
        let compat = evaluate_compatibility("BMW", "535i", 2011, "bluetooth");
        assert!(compat.recommended_interface.contains("SWITCH TO ENET"));
        assert!(compat
            .unsupported_features
            .iter()
            .any(|f| f.contains("Battery Registration")));
    }

    #[test]
    fn test_gmc_acadia_compatibility() {
        let compat = evaluate_compatibility("GMC", "Acadia", 2010, "bluetooth");
        assert!(compat
            .compatible_interfaces
            .contains(&"bluetooth".to_string()));
        assert!(compat
            .supported_features
            .iter()
            .any(|f| f.contains("Mode 01")));
        assert!(compat
            .unsupported_features
            .iter()
            .any(|f| f.contains("BMW-specific")));
    }

    #[test]
    fn test_generic_obd2_compatibility() {
        let compat = evaluate_compatibility("Toyota", "Camry", 2016, "bluetooth");
        assert!(compat
            .compatible_interfaces
            .contains(&"bluetooth".to_string()));
        assert!(compat
            .supported_features
            .iter()
            .any(|f| f.contains("SAE J1979")));
    }
}
