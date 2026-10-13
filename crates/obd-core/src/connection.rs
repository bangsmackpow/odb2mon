use crate::error::ObdError;
use crate::profile::{
    BatteryStatus, CbsItemType, CbsReport, DtcRecord, ParameterType, ParameterValue,
    TopologyScanReport, TransmissionOilStatus, VehicleProfile, WaterPumpBleedStatus,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Connection lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u32 },
    Error(String),
}

/// Strict communication abstraction layer.
/// The UI, dashboard polling loop, and CLI are 100% agnostic to whether
/// the underlying transport is TCP/IP (ENET), Serial (ELM327 Bluetooth), or Mock.
#[async_trait]
pub trait ObdConnection: Send + Sync {
    /// Establishes transport connection and completes protocol handshakes
    /// (e.g., DoIP Routing Activation on BMW, or AT command setup on ELM327).
    async fn connect(&mut self) -> Result<(), ObdError>;

    /// Gracefully closes connection and shuts down active diagnostic sessions.
    async fn disconnect(&mut self) -> Result<(), ObdError>;

    /// Checks if the underlying transport is currently active and ready for I/O.
    fn is_connected(&self) -> bool;

    /// Queries a single parameter (e.g. RPM, Speed, Coolant, Oil Temp).
    async fn read_parameter(&mut self, param: ParameterType) -> Result<ParameterValue, ObdError>;

    /// Queries Diagnostic Trouble Codes (DTCs).
    async fn read_dtcs(&mut self) -> Result<Vec<DtcRecord>, ObdError>;

    /// Clears Diagnostic Trouble Codes.
    async fn clear_dtcs(&mut self) -> Result<(), ObdError>;

    /// Queries Intelligent Battery Sensor (IBS) and Power Management telemetry.
    async fn read_battery_status(&mut self) -> Result<BatteryStatus, ObdError> {
        Err(ObdError::Unsupported(
            "Battery status query is not supported on this vehicle profile".into(),
        ))
    }

    /// Registers a new battery exchange in DME/DDE memory.
    async fn register_battery_replacement(&mut self) -> Result<String, ObdError> {
        Err(ObdError::Unsupported(
            "Battery registration is only supported on BMW UDS profiles".into(),
        ))
    }

    /// Queries Condition Based Service (CBS) maintenance intervals and status.
    async fn read_cbs_report(&mut self) -> Result<CbsReport, ObdError> {
        Err(ObdError::Unsupported(
            "CBS monitoring is only supported on BMW UDS profiles".into(),
        ))
    }

    /// Resets a specific Condition Based Service (CBS) item interval.
    async fn reset_cbs_item(&mut self, _item: CbsItemType) -> Result<String, ObdError> {
        Err(ObdError::Unsupported(
            "CBS reset is only supported on BMW UDS profiles".into(),
        ))
    }

    /// Performs a full-network Electronic Control Unit (ECU) topology scan across all vehicle bus segments.
    async fn scan_ecu_topology(&mut self) -> Result<TopologyScanReport, ObdError> {
        Err(ObdError::Unsupported(
            "ECU topology scan is only supported on BMW UDS profiles".into(),
        ))
    }

    /// Starts the automated electric water pump coolant bleeding routine (Routine 0x0211 on DME).
    async fn start_water_pump_bleed(&mut self) -> Result<String, ObdError> {
        Err(ObdError::Unsupported(
            "Actuator routines are only supported on BMW UDS profiles".into(),
        ))
    }

    /// Stops / cancels the electric water pump coolant bleeding routine.
    async fn stop_water_pump_bleed(&mut self) -> Result<String, ObdError> {
        Err(ObdError::Unsupported(
            "Actuator routines are only supported on BMW UDS profiles".into(),
        ))
    }

    /// Queries live status and progress of the electric water pump bleeding routine.
    async fn get_water_pump_bleed_status(&mut self) -> Result<WaterPumpBleedStatus, ObdError> {
        Err(ObdError::Unsupported(
            "Actuator routines are only supported on BMW UDS profiles".into(),
        ))
    }

    /// Queries ZF 8HP transmission fluid sump temperature and fill level window assessment (DID 0x2045 on EGS).
    async fn get_transmission_oil_status(&mut self) -> Result<TransmissionOilStatus, ObdError> {
        Err(ObdError::Unsupported(
            "Transmission status is only supported on BMW UDS profiles".into(),
        ))
    }

    /// Executes ZF 8HP transmission fluid level check routine (Routine 0x0225 on EGS).
    async fn start_transmission_fill_routine(&mut self) -> Result<String, ObdError> {
        Err(ObdError::Unsupported(
            "Actuator routines are only supported on BMW UDS profiles".into(),
        ))
    }

    /// Sets radiator electric cooling fan test duty percent (Routine 0x0215: 0, 15, 50, 90).
    async fn set_radiator_fan_duty(&mut self, _duty_percent: u8) -> Result<String, ObdError> {
        Err(ObdError::Unsupported(
            "Actuator routines are only supported on BMW UDS profiles".into(),
        ))
    }

    /// Sets muffler exhaust flap actuator state (Routine 0x0218: true = open, false = closed).
    async fn set_exhaust_flap(&mut self, _open: bool) -> Result<String, ObdError> {
        Err(ObdError::Unsupported(
            "Actuator routines are only supported on BMW UDS profiles".into(),
        ))
    }

    /// Returns the vehicle profile associated with this connection.
    fn profile(&self) -> VehicleProfile;
}
