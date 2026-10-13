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

/// Vehicle communication bus segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BusType {
    PtCan,    // Powertrain CAN
    KCan,     // Body / Comfort CAN
    FlexRay,  // Chassis FlexRay
    Ethernet, // High-speed DoIP / Gateway
}

impl BusType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::PtCan => "PT-CAN (Powertrain)",
            Self::KCan => "K-CAN (Body / Comfort)",
            Self::FlexRay => "FlexRay (Chassis / Dynamics)",
            Self::Ethernet => "Ethernet (Gateway / Multimedia)",
        }
    }
}

/// Operational health state of an Electronic Control Unit (ECU).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EcuHealth {
    Normal,      // Responding normally, 0 DTCs (Green)
    FaultStored, // Responding, 1 or more DTCs stored (Amber)
    NoResponse,  // Unresponsive / Not equipped (Grey)
    CommError,   // Negative response code / framing error (Red)
}

/// Individual Electronic Control Unit (ECU) node in the vehicle network topology.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EcuNode {
    pub logical_address: u16,
    pub short_name: String,
    pub full_name: String,
    pub bus: BusType,
    pub health: EcuHealth,
    pub dtc_count: usize,
    pub dtcs: Vec<DtcRecord>,
    pub response_time_ms: Option<u32>,
}

/// Full network topology report across all vehicle controllers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TopologyScanReport {
    pub supported: bool,
    pub total_ecus_scanned: usize,
    pub online_count: usize,
    pub fault_count: usize,
    pub nodes: Vec<EcuNode>,
}

/// Static metadata defining an ECU node in the BMW F-Series architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BmwEcuDefinition {
    pub logical_address: u16,
    pub short_name: &'static str,
    pub full_name: &'static str,
    pub bus: BusType,
}

pub const BMW_F10_ECUS: &[BmwEcuDefinition] = &[
    // PT-CAN (Powertrain)
    BmwEcuDefinition {
        logical_address: 0x0012,
        short_name: "DME",
        full_name: "Digital Motor Electronics (N55 Engine)",
        bus: BusType::PtCan,
    },
    BmwEcuDefinition {
        logical_address: 0x0018,
        short_name: "EGS",
        full_name: "Electronic Transmission Control (ZF 8HP)",
        bus: BusType::PtCan,
    },
    BmwEcuDefinition {
        logical_address: 0x0029,
        short_name: "DSC",
        full_name: "Dynamic Stability Control / ABS",
        bus: BusType::PtCan,
    },
    BmwEcuDefinition {
        logical_address: 0x0017,
        short_name: "EKPS",
        full_name: "Electronic Fuel Pump Control",
        bus: BusType::PtCan,
    },
    BmwEcuDefinition {
        logical_address: 0x005E,
        short_name: "GWS",
        full_name: "Gear Selector Switch",
        bus: BusType::PtCan,
    },
    // FlexRay (Chassis / Dynamics)
    BmwEcuDefinition {
        logical_address: 0x0030,
        short_name: "EPS",
        full_name: "Electronic Power Steering",
        bus: BusType::FlexRay,
    },
    BmwEcuDefinition {
        logical_address: 0x001C,
        short_name: "ICM",
        full_name: "Integrated Chassis Management",
        bus: BusType::FlexRay,
    },
    BmwEcuDefinition {
        logical_address: 0x0026,
        short_name: "EMF",
        full_name: "Electromechanical Parking Brake",
        bus: BusType::FlexRay,
    },
    // K-CAN (Body / Comfort)
    BmwEcuDefinition {
        logical_address: 0x0040,
        short_name: "CAS",
        full_name: "Car Access System (Immobilizer / Ignition)",
        bus: BusType::KCan,
    },
    BmwEcuDefinition {
        logical_address: 0x0060,
        short_name: "KOMBI",
        full_name: "Instrument Cluster",
        bus: BusType::KCan,
    },
    BmwEcuDefinition {
        logical_address: 0x0078,
        short_name: "IHKA",
        full_name: "Integrated Automatic Heating & A/C",
        bus: BusType::KCan,
    },
    BmwEcuDefinition {
        logical_address: 0x0072,
        short_name: "FRM",
        full_name: "Footwell Lighting & Windows Module",
        bus: BusType::KCan,
    },
    BmwEcuDefinition {
        logical_address: 0x0070,
        short_name: "JBE",
        full_name: "Junction Box Electronics / Gateway",
        bus: BusType::KCan,
    },
    BmwEcuDefinition {
        logical_address: 0x0067,
        short_name: "PDC",
        full_name: "Park Distance Control",
        bus: BusType::KCan,
    },
    // Ethernet (High-speed / Multimedia / Gateway)
    BmwEcuDefinition {
        logical_address: 0x0010,
        short_name: "ZGW",
        full_name: "Central Gateway Module",
        bus: BusType::Ethernet,
    },
    BmwEcuDefinition {
        logical_address: 0x0063,
        short_name: "HU_NBT",
        full_name: "Head Unit / Navigation System",
        bus: BusType::Ethernet,
    },
];

/// Status of the automated electric water pump coolant bleeding routine (Routine 0x0211).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WaterPumpBleedStatus {
    pub is_running: bool,
    pub progress_percent: f32,
    pub pump_speed_percent: u8,
    pub elapsed_seconds: u32,
    pub remaining_seconds: u32,
    pub stage_description: String,
}

impl Default for WaterPumpBleedStatus {
    fn default() -> Self {
        Self {
            is_running: false,
            progress_percent: 0.0,
            pump_speed_percent: 0,
            elapsed_seconds: 0,
            remaining_seconds: 720,
            stage_description: "Idle / Ready to start".into(),
        }
    }
}

/// ZF 8HP transmission fluid temperature and fill level window assessment (DID 0x2045).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransmissionOilStatus {
    pub supported: bool,
    pub temperature_c: f32,
    pub temperature_f: f32,
    pub is_within_fill_window: bool, // 40.0°C <= temp <= 50.0°C
    pub status_text: String,
}

impl Default for TransmissionOilStatus {
    fn default() -> Self {
        Self {
            supported: false,
            temperature_c: 0.0,
            temperature_f: 32.0,
            is_within_fill_window: false,
            status_text: "Uninitialized".into(),
        }
    }
}

/// Status of the variable-speed electric radiator cooling fan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadiatorFanStatus {
    pub active_duty_percent: u8,
    pub is_overridden: bool,
}

/// Status of the muffler exhaust flapper valve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExhaustFlapStatus {
    pub is_open: bool,
    pub is_overridden: bool,
}

/// Individual high-speed engine datalog frame capturing dynamic combustion and turbocharger parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineDatalogFrame {
    pub timestamp_ms: u64,
    pub engine_rpm: u16,
    pub vehicle_speed_kmh: u8,
    pub boost_target_hpa: f32,
    pub boost_actual_hpa: f32,
    pub boost_target_psi: f32,
    pub boost_actual_psi: f32,
    pub wastegate_duty_percent: f32,
    pub rail_pressure_target_bar: f32,
    pub rail_pressure_actual_bar: f32,
    pub low_pressure_fuel_bar: f32,
    pub lambda: f32,
    pub intake_air_temp_c: f32,
    pub mass_air_flow_gps: f32,
    pub ignition_timing_cyl1: f32,
    pub knock_retard: [f32; 6],
    pub misfire_counts: [u16; 6],
    pub vanos_intake_target_deg: f32,
    pub vanos_intake_actual_deg: f32,
    pub vanos_exhaust_target_deg: f32,
    pub vanos_exhaust_actual_deg: f32,
}

impl Default for EngineDatalogFrame {
    fn default() -> Self {
        Self {
            timestamp_ms: 0,
            engine_rpm: 650,
            vehicle_speed_kmh: 0,
            boost_target_hpa: 1013.0,
            boost_actual_hpa: 1013.0,
            boost_target_psi: 0.0,
            boost_actual_psi: 0.0,
            wastegate_duty_percent: 0.0,
            rail_pressure_target_bar: 50.0,
            rail_pressure_actual_bar: 50.0,
            low_pressure_fuel_bar: 5.2,
            lambda: 1.0,
            intake_air_temp_c: 25.0,
            mass_air_flow_gps: 4.5,
            ignition_timing_cyl1: 5.0,
            knock_retard: [0.0; 6],
            misfire_counts: [0; 6],
            vanos_intake_target_deg: 85.0,
            vanos_intake_actual_deg: 85.0,
            vanos_exhaust_target_deg: 115.0,
            vanos_exhaust_actual_deg: 115.0,
        }
    }
}

/// Datalogger recording status and session metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DataloggerStatus {
    pub supported: bool,
    pub is_recording: bool,
    pub sample_count: usize,
    pub elapsed_seconds: f32,
}

impl Default for DataloggerStatus {
    fn default() -> Self {
        Self {
            supported: false,
            is_recording: false,
            sample_count: 0,
            elapsed_seconds: 0.0,
        }
    }
}

/// Result returned when a recording session is completed and exported.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatalogRecordingSummary {
    pub sample_count: usize,
    pub duration_seconds: f32,
    pub csv_content: String,
    pub saved_file_path: Option<String>,
}
