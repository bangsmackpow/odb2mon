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

/// A selectable option variant for a coding parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodingOption {
    pub label: String,
    pub raw_value: u8,
    pub is_factory_default: bool,
}

/// Definition and active state of an FDL vehicle customization parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodingParameterDefinition {
    pub id: String,
    pub name: String,
    pub category: String, // "Chimes & Warnings", "Lighting & Timing", "Mirrors & Convenience", "Engine & Start-Stop", "Displays & Cluster"
    pub target_ecu: String, // "KOMBI", "FRM", "CAS", "HU_NBT", "DME", "IHKA"
    pub ecu_address: u16,
    pub did: u16,
    pub byte_offset: usize,
    pub bit_mask: u8,
    pub description: String,
    pub active_value: String,
    pub active_raw_value: u8,
    pub options: Vec<CodingOption>,
}

/// Metadata record for a pre-coding parameter backup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodingBackupRecord {
    pub id: String,
    pub timestamp: String,
    pub vehicle_id: String,
    pub param_id: String,
    pub param_name: String,
    pub ecu_name: String,
    pub ecu_address: u16,
    pub previous_value_label: String,
    pub previous_raw_value: u8,
    pub new_value_label: String,
    pub new_raw_value: u8,
    pub payload_hex: String,
}

/// Returns the factory default coding parameter definitions for BMW F10 / F-series chassis.
pub fn get_default_bmw_f10_coding_parameters() -> Vec<CodingParameterDefinition> {
    vec![
        // 1. Warning Gong Sound Tone (KOMBI 0x0060)
        CodingParameterDefinition {
            id: "warning_gong".into(),
            name: "Warning Gong Tone".into(),
            category: "Chimes & Warnings".into(),
            target_ecu: "KOMBI".into(),
            ecu_address: 0x0060,
            did: 0x3000,
            byte_offset: 0,
            bit_mask: 0x0F,
            description: "Acoustic chime tone for door ajar, low temperature, and vehicle notifications.".into(),
            active_value: "BMW Standard".into(),
            active_raw_value: 0x00,
            options: vec![
                CodingOption { label: "BMW Standard".into(), raw_value: 0x00, is_factory_default: true },
                CodingOption { label: "Rolls-Royce (Gentle Chime)".into(), raw_value: 0x01, is_factory_default: false },
                CodingOption { label: "BMW i (Modern Electro)".into(), raw_value: 0x02, is_factory_default: false },
                CodingOption { label: "Mini Cooper".into(), raw_value: 0x03, is_factory_default: false },
            ],
        },
        // 2. Seat Belt Warning Chime (CAS 0x0040)
        CodingParameterDefinition {
            id: "seatbelt_reminder".into(),
            name: "Seat Belt Reminder Chime".into(),
            category: "Chimes & Warnings".into(),
            target_ecu: "CAS".into(),
            ecu_address: 0x0040,
            did: 0x3001,
            byte_offset: 0,
            bit_mask: 0x01,
            description: "Audible chime when driving without seat belt fastened (visual dash indicator remains).".into(),
            active_value: "Enabled".into(),
            active_raw_value: 0x01,
            options: vec![
                CodingOption { label: "Enabled".into(), raw_value: 0x01, is_factory_default: true },
                CodingOption { label: "Disabled".into(), raw_value: 0x00, is_factory_default: false },
            ],
        },
        // 3. Welcome Light Duration (FRM 0x0072)
        CodingParameterDefinition {
            id: "welcome_light_duration".into(),
            name: "Welcome Light Duration".into(),
            category: "Lighting & Timing".into(),
            target_ecu: "FRM".into(),
            ecu_address: 0x0072,
            did: 0x3010,
            byte_offset: 0,
            bit_mask: 0xFF,
            description: "Duration angel eyes and exterior LED lights stay lit upon key fob vehicle unlock.".into(),
            active_value: "20 Seconds".into(),
            active_raw_value: 0x14,
            options: vec![
                CodingOption { label: "Off".into(), raw_value: 0x00, is_factory_default: false },
                CodingOption { label: "10 Seconds".into(), raw_value: 0x0A, is_factory_default: false },
                CodingOption { label: "20 Seconds".into(), raw_value: 0x14, is_factory_default: true },
                CodingOption { label: "40 Seconds".into(), raw_value: 0x28, is_factory_default: false },
                CodingOption { label: "60 Seconds".into(), raw_value: 0x3C, is_factory_default: false },
            ],
        },
        // 4. Pathway / Follow-Me-Home Light Duration (FRM 0x0072)
        CodingParameterDefinition {
            id: "pathway_light_duration".into(),
            name: "Pathway (Follow-Me-Home) Lights".into(),
            category: "Lighting & Timing".into(),
            target_ecu: "FRM".into(),
            ecu_address: 0x0072,
            did: 0x3011,
            byte_offset: 0,
            bit_mask: 0xFF,
            description: "Duration headlights remain on after locking vehicle to illuminate walkway/driveway.".into(),
            active_value: "30 Seconds".into(),
            active_raw_value: 0x1E,
            options: vec![
                CodingOption { label: "Off".into(), raw_value: 0x00, is_factory_default: false },
                CodingOption { label: "30 Seconds".into(), raw_value: 0x1E, is_factory_default: true },
                CodingOption { label: "60 Seconds".into(), raw_value: 0x3C, is_factory_default: false },
                CodingOption { label: "90 Seconds".into(), raw_value: 0x5A, is_factory_default: false },
                CodingOption { label: "120 Seconds".into(), raw_value: 0x78, is_factory_default: false },
            ],
        },
        // 5. Comfort Turn Signal Blinks (FRM 0x0072)
        CodingParameterDefinition {
            id: "comfort_blinker_count".into(),
            name: "Comfort Turn Signal Blinks".into(),
            category: "Lighting & Timing".into(),
            target_ecu: "FRM".into(),
            ecu_address: 0x0072,
            did: 0x3012,
            byte_offset: 0,
            bit_mask: 0x0F,
            description: "Number of highway lane-change indicator blinks on a single soft tap of the turn stalk.".into(),
            active_value: "3 Blinks".into(),
            active_raw_value: 0x03,
            options: vec![
                CodingOption { label: "1 Blink".into(), raw_value: 0x01, is_factory_default: false },
                CodingOption { label: "3 Blinks".into(), raw_value: 0x03, is_factory_default: true },
                CodingOption { label: "5 Blinks".into(), raw_value: 0x05, is_factory_default: false },
            ],
        },
        // 6. Angel Eye Brightness with Low Beams (FRM 0x0072)
        CodingParameterDefinition {
            id: "angel_eyes_brightness".into(),
            name: "Angel Eye Halo Brightness (Low Beams)".into(),
            category: "Lighting & Timing".into(),
            target_ecu: "FRM".into(),
            ecu_address: 0x0072,
            did: 0x3013,
            byte_offset: 0,
            bit_mask: 0xFF,
            description: "PWM duty brightness intensity of front corona rings when low beam headlights are ON.".into(),
            active_value: "8% (US Factory)".into(),
            active_raw_value: 0x08,
            options: vec![
                CodingOption { label: "8% (US Factory)".into(), raw_value: 0x08, is_factory_default: true },
                CodingOption { label: "12% (Euro Spec)".into(), raw_value: 0x0C, is_factory_default: false },
                CodingOption { label: "50% (Enhanced)".into(), raw_value: 0x32, is_factory_default: false },
                CodingOption { label: "100% (Full DRL Brightness)".into(), raw_value: 0x64, is_factory_default: false },
            ],
        },
        // 7. Fog Lights with High Beams (FRM 0x0072)
        CodingParameterDefinition {
            id: "fog_lights_with_high_beams".into(),
            name: "Fog Lights with High Beams".into(),
            category: "Lighting & Timing".into(),
            target_ecu: "FRM".into(),
            ecu_address: 0x0072,
            did: 0x3014,
            byte_offset: 0,
            bit_mask: 0x01,
            description: "Allows front fog lights to stay illuminated simultaneously with high beams.".into(),
            active_value: "Disabled".into(),
            active_raw_value: 0x00,
            options: vec![
                CodingOption { label: "Disabled (Factory Cutoff)".into(), raw_value: 0x00, is_factory_default: true },
                CodingOption { label: "Enabled (Fog Lights Stay ON)".into(), raw_value: 0x01, is_factory_default: false },
            ],
        },
        // 8. Comfort Access Auto-Folding Mirrors (CAS 0x0040)
        CodingParameterDefinition {
            id: "auto_fold_mirrors".into(),
            name: "Comfort Access Auto-Folding Mirrors".into(),
            category: "Mirrors & Convenience".into(),
            target_ecu: "CAS".into(),
            ecu_address: 0x0040,
            did: 0x3020,
            byte_offset: 0,
            bit_mask: 0x03,
            description: "Folds exterior side mirrors when vehicle is locked and unfolds upon remote unlock.".into(),
            active_value: "Disabled".into(),
            active_raw_value: 0x00,
            options: vec![
                CodingOption { label: "Disabled".into(), raw_value: 0x00, is_factory_default: true },
                CodingOption { label: "Instant on Door Lock (0s delay)".into(), raw_value: 0x01, is_factory_default: false },
                CodingOption { label: "On Key Fob Lock Hold (1.5s delay)".into(), raw_value: 0x02, is_factory_default: false },
            ],
        },
        // 9. Auto-Unlock Doors on Engine Stop (CAS 0x0040)
        CodingParameterDefinition {
            id: "auto_unlock_engine_off".into(),
            name: "Auto-Unlock on Engine Stop".into(),
            category: "Mirrors & Convenience".into(),
            target_ecu: "CAS".into(),
            ecu_address: 0x0040,
            did: 0x3021,
            byte_offset: 0,
            bit_mask: 0x01,
            description: "Unlocks all vehicle passenger doors automatically when engine START/STOP is pressed to off.".into(),
            active_value: "Disabled".into(),
            active_raw_value: 0x00,
            options: vec![
                CodingOption { label: "Disabled (Factory)".into(), raw_value: 0x00, is_factory_default: true },
                CodingOption { label: "Unlock All Doors on Engine Stop".into(), raw_value: 0x01, is_factory_default: false },
            ],
        },
        // 10. Auto Start/Stop Memory (CAS 0x0040)
        CodingParameterDefinition {
            id: "auto_start_stop_memory".into(),
            name: "Auto Start/Stop Memory".into(),
            category: "Engine & Start-Stop".into(),
            target_ecu: "CAS".into(),
            ecu_address: 0x0040,
            did: 0x3030,
            byte_offset: 0,
            bit_mask: 0x03,
            description: "Remembers the Auto Start/Stop off-switch position between vehicle drive cycles.".into(),
            active_value: "Factory (Always ON)".into(),
            active_raw_value: 0x00,
            options: vec![
                CodingOption { label: "Factory (Always ON every start)".into(), raw_value: 0x00, is_factory_default: true },
                CodingOption { label: "Remember Last Setting (Recommended)".into(), raw_value: 0x01, is_factory_default: false },
                CodingOption { label: "Default to OFF".into(), raw_value: 0x02, is_factory_default: false },
            ],
        },
        // 11. Digital Speedometer in Cluster (KOMBI 0x0060)
        CodingParameterDefinition {
            id: "digital_speedometer".into(),
            name: "Digital Speedometer in Cluster".into(),
            category: "Displays & Cluster".into(),
            target_ecu: "KOMBI".into(),
            ecu_address: 0x0060,
            did: 0x3002,
            byte_offset: 0,
            bit_mask: 0x01,
            description: "Adds real-time digital numerical speed readout to the center BC gauge cluster display.".into(),
            active_value: "Disabled".into(),
            active_raw_value: 0x00,
            options: vec![
                CodingOption { label: "Disabled (Factory)".into(), raw_value: 0x00, is_factory_default: true },
                CodingOption { label: "Enabled (Digital Speedo in BC)".into(), raw_value: 0x01, is_factory_default: false },
            ],
        },
        // 12. iDrive Legal Disclaimer Removal (HU_NBT 0x0063)
        CodingParameterDefinition {
            id: "legal_disclaimer".into(),
            name: "iDrive Startup Legal Disclaimer".into(),
            category: "Displays & Cluster".into(),
            target_ecu: "HU_NBT".into(),
            ecu_address: 0x0063,
            did: 0x3040,
            byte_offset: 0,
            bit_mask: 0x01,
            description: "Removes the 10-second legal safety warning screen on iDrive system startup.".into(),
            active_value: "Enabled (Factory)".into(),
            active_raw_value: 0x01,
            options: vec![
                CodingOption { label: "Enabled (Factory 10s Screen)".into(), raw_value: 0x01, is_factory_default: true },
                CodingOption { label: "Instant Boot / Disabled".into(), raw_value: 0x00, is_factory_default: false },
            ],
        },
    ]
}
