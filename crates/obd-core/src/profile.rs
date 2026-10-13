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

/// BMW Condition Based Service (CBS) item identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CbsItemType {
    EngineOil,
    FrontBrakes,
    RearBrakes,
    BrakeFluid,
    VehicleCheck,
    SparkPlugs,
    CabinFilter,
}

impl CbsItemType {
    pub fn id_byte(&self) -> u8 {
        match self {
            Self::EngineOil => 0x01,
            Self::FrontBrakes => 0x02,
            Self::RearBrakes => 0x03,
            Self::BrakeFluid => 0x04,
            Self::VehicleCheck => 0x05,
            Self::SparkPlugs => 0x06,
            Self::CabinFilter => 0x07,
        }
    }

    pub fn from_id_byte(byte: u8) -> Option<Self> {
        match byte {
            0x01 => Some(Self::EngineOil),
            0x02 => Some(Self::FrontBrakes),
            0x03 => Some(Self::RearBrakes),
            0x04 => Some(Self::BrakeFluid),
            0x05 => Some(Self::VehicleCheck),
            0x06 => Some(Self::SparkPlugs),
            0x07 => Some(Self::CabinFilter),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::EngineOil => "Engine Oil",
            Self::FrontBrakes => "Front Brake Pads",
            Self::RearBrakes => "Rear Brake Pads",
            Self::BrakeFluid => "Brake Fluid",
            Self::VehicleCheck => "Vehicle Inspection",
            Self::SparkPlugs => "Spark Plugs",
            Self::CabinFilter => "Microfilter / Cabin",
        }
    }
}

impl std::str::FromStr for CbsItemType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "engine_oil" | "oil" => Ok(Self::EngineOil),
            "front_brakes" | "front_brake" => Ok(Self::FrontBrakes),
            "rear_brakes" | "rear_brake" => Ok(Self::RearBrakes),
            "brake_fluid" | "fluid" => Ok(Self::BrakeFluid),
            "vehicle_check" | "inspection" => Ok(Self::VehicleCheck),
            "spark_plugs" | "plugs" => Ok(Self::SparkPlugs),
            "cabin_filter" | "microfilter" => Ok(Self::CabinFilter),
            _ => Err(format!("Unknown CBS item type: {s}")),
        }
    }
}

/// Service status of a Condition Based Service item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CbsStatus {
    Ok,
    Warning,
    Overdue,
}

/// Individual Condition Based Service item state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CbsItem {
    pub item_type: CbsItemType,
    pub name: String,
    pub status: CbsStatus,
    pub remaining_percent: f32,
    pub remaining_km: Option<i32>,
    pub due_date: Option<String>,
    pub reset_count: u32,
}

/// Full CBS service overview report for the vehicle.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CbsReport {
    pub supported: bool,
    pub items: Vec<CbsItem>,
}
