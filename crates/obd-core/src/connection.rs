use crate::error::ObdError;
use crate::profile::{
    BatteryStatus, CbsItemType, CbsReport, DtcRecord, ParameterType, ParameterValue, VehicleProfile,
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

    /// Returns the vehicle profile associated with this connection.
    fn profile(&self) -> VehicleProfile;
}
