use crate::profile::ParameterType;
use thiserror::Error;

/// Unified error enum for vehicle diagnostics, transport failures, and protocol parsing.
#[derive(Debug, Error)]
pub enum ObdError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Transport disconnected or closed")]
    Disconnected,

    #[error("I/O operation timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("UDS Negative Response: Service {service_id:#02X}, NRC {nrc:#02X} ({description})")]
    NegativeResponse {
        service_id: u8,
        nrc: u8,
        description: &'static str,
    },

    #[error("Diagnostic session elevation required (NRC 0x22 or 0x7E)")]
    SessionElevationRequired,

    #[error("Parameter '{0:?}' is unsupported by the target vehicle")]
    ParameterUnsupported(ParameterType),

    #[error("Parsing error: {0}")]
    ParseError(String),

    #[error("ELM327 error: {0}")]
    Elm327(String),

    #[error("DTC operation failed: {0}")]
    DtcError(String),

    #[error("Operation unsupported: {0}")]
    Unsupported(String),
}

impl ObdError {
    /// Translates ISO 14229-1 Negative Response Codes (NRC) to human-readable explanations.
    pub fn nrc_to_str(nrc: u8) -> &'static str {
        match nrc {
            0x10 => "General Reject",
            0x11 => "Service Not Supported",
            0x12 => "Sub-Function Not Supported",
            0x13 => "Incorrect Message Length Or Invalid Format",
            0x14 => "Response Too Long",
            0x21 => "Busy Repeat Request",
            0x22 => "Conditions Not Correct",
            0x24 => "Request Sequence Error",
            0x25 => "No Response From Subnet Component",
            0x26 => "Failure Prevents Execution Of Requested Action",
            0x31 => "Request Out Of Range",
            0x33 => "Security Access Denied",
            0x35 => "Invalid Key",
            0x36 => "Exceeded Number Of Attempts",
            0x37 => "Required Time Delay Not Expired",
            0x70 => "Upload / Download Not Accepted",
            0x71 => "Transfer Data Suspended",
            0x72 => "General Programming Failure",
            0x73 => "Wrong Block Sequence Counter",
            0x78 => "Request Correctly Received - Response Pending",
            0x7E => "Sub-Function Not Supported In Active Session",
            0x7F => "Service Not Supported In Active Session",
            _ => "Unknown Negative Response Code",
        }
    }

    /// Constructs a NegativeResponse variant directly from service ID and NRC byte.
    pub fn from_nrc(service_id: u8, nrc: u8) -> Self {
        ObdError::NegativeResponse {
            service_id,
            nrc,
            description: Self::nrc_to_str(nrc),
        }
    }
}
