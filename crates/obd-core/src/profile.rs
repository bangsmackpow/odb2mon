use serde::{Deserialize, Serialize};

/// Supported vehicle profiles with specific hardware, protocol, and DID/PID mappings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum VehicleProfile {
    /// 2011 BMW 535i xDrive (F10 chassis, N55 engine).
    /// Communicates via UDS over DoIP (TCP/IP on port 6801). Target ECU is DME (logical address 0x12 or 0x01).
    BmwF10_535i,

    /// 2010 GMC Acadia (3.6L LLT V6).
    /// Communicates via ISO 15765-4 (CAN 11-bit 500k) through an ELM327 Bluetooth/Serial adapter.
    GmcAcadia_2010,

    /// In-memory realistic physics simulator for CI/CD and offline testing.
    Simulator,
}

impl VehicleProfile {
    pub fn name(&self) -> &'static str {
        match self {
            Self::BmwF10_535i => "2011 BMW 535i xDrive (F10 / N55)",
            Self::GmcAcadia_2010 => "2010 GMC Acadia (3.6L LLT)",
            Self::Simulator => "Virtual Vehicle Simulator",
        }
    }

    pub fn default_interface_description(&self) -> &'static str {
        match self {
            Self::BmwF10_535i => "ENET Cable (TCP 169.254.1.1:6801, DoIP ISO 13400)",
            Self::GmcAcadia_2010 => "Bluetooth ELM327 (Serial AT commands, ISO 15765-4 CAN)",
            Self::Simulator => "In-Memory Physics Simulator",
        }
    }
}

/// Standardized telemetry parameter types monitored by the dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParameterType {
    EngineRpm,
    VehicleSpeed,
    CoolantTemperature,
    OilTemperature,
}

/// Parameter value variants with physical units.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParameterValue {
    /// Engine crankshaft revolutions per minute (RPM).
    Rpm(u32),
    /// Vehicle road speed in kilometers per hour (km/h).
    SpeedKmh(u32),
    /// Temperature in degrees Celsius (°C).
    TemperatureCelsius(f32),
    /// Critical requirement: If vehicle ECU rejects the PID (e.g., GMC Oil Temp),
    /// the system returns Unsupported without crashing or hanging.
    Unsupported,
    /// Parameter query returned no data on this cycle.
    NotAvailable,
}

/// Severity and confirmation status of Diagnostic Trouble Codes (DTCs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DtcStatus {
    Confirmed,
    Pending,
    Permanent,
}

/// Structured Diagnostic Trouble Code record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DtcRecord {
    pub code: String,
    pub description: String,
    pub status: DtcStatus,
    pub ecu_source: Option<String>,
}

/// Real-time dashboard telemetry snapshot for UI consumers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    pub rpm: Option<u32>,
    pub speed_kmh: Option<u32>,
    pub coolant_temp_c: Option<f32>,
    pub oil_temp_c: Option<f32>,
    pub oil_temp_supported: bool,
    pub connection_status: String,
    pub active_dtcs: Vec<DtcRecord>,
}

/// Detailed Intelligent Battery Sensor (IBS) and Power Management telemetry.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BatteryStatus {
    pub supported: bool,
    pub state_of_charge_percent: f32,
    pub voltage: f32,
    pub current_amps: f32,
    pub temperature_celsius: f32,
    pub capacity_ah: Option<u32>,
    pub battery_type: Option<String>,
    pub last_replacement_km: Vec<u32>,
}
