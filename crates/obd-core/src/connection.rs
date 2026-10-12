use crate::error::ObdError;
use crate::profile::{DtcRecord, ParameterType, ParameterValue, VehicleProfile};
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

    /// Returns the vehicle profile associated with this connection.
    fn profile(&self) -> VehicleProfile;
}
