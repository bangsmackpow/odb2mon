use crate::connection::ObdConnection;
use crate::error::ObdError;
use crate::profile::{BatteryStatus, DtcRecord, ParameterType, ParameterValue, VehicleProfile};
use crate::protocols::doip;
use crate::protocols::uds;
use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

/// BMW ENET (Ethernet-to-OBD2) adapter communicating via DoIP (ISO 13400) and UDS (ISO 14229).
pub struct EnetConnection {
    host: String,
    port: u16,
    tester_address: u16, // Diagnostic tool address, typically 0x0E00
    target_ecu: u16,     // BMW DME logical address: typically 0x0012 or 0x0001
    stream: Option<TcpStream>,
    session_elevated: bool,
}

impl EnetConnection {
    pub fn new(host: String, port: u16, target_ecu: u16) -> Self {
        Self {
            host,
            port,
            tester_address: 0x0E00,
            target_ecu,
            stream: None,
            session_elevated: false,
        }
    }

    /// Default configuration for 2011 BMW 535i xDrive (F10).
    pub fn default_f10() -> Self {
        Self::new("169.254.1.1".into(), 6801, 0x0012)
    }

    /// Sends a raw UDS payload wrapped in a DoIP diagnostic message frame and reads response.
    async fn send_uds_request(&mut self, payload: &[u8]) -> Result<Vec<u8>, ObdError> {
        let stream = self.stream.as_mut().ok_or(ObdError::Disconnected)?;

        let doip_frame =
            doip::build_diagnostic_message(self.tester_address, self.target_ecu, payload);

        stream.write_all(&doip_frame).await?;

        // Read DoIP 8-byte header
        let mut header_buf = [0u8; 8];
        timeout(
            Duration::from_millis(2000),
            stream.read_exact(&mut header_buf),
        )
        .await
        .map_err(|_| ObdError::Timeout(Duration::from_millis(2000)))??;

        let payload_len =
            u32::from_be_bytes([header_buf[4], header_buf[5], header_buf[6], header_buf[7]])
                as usize;

        let mut payload_buf = vec![0u8; payload_len];
        stream.read_exact(&mut payload_buf).await?;

        doip::extract_uds_payload(&payload_buf)
    }

    /// Switches ECU diagnostic session to Extended Session (0x10 0x03).
    async fn elevate_to_extended_session(&mut self) -> Result<(), ObdError> {
        let req = uds::build_session_control_request(0x03);
        let resp = self.send_uds_request(&req).await?;
        if resp.first() == Some(&0x50) {
            self.session_elevated = true;
            tracing::info!(
                "Successfully elevated BMW DME to Extended Diagnostic Session (0x10 0x03)"
            );
            Ok(())
        } else {
            Err(ObdError::Protocol(
                "ECU rejected extended diagnostic session".into(),
            ))
        }
    }
}

#[async_trait]
impl ObdConnection for EnetConnection {
    async fn connect(&mut self) -> Result<(), ObdError> {
        let addr = format!("{}:{}", self.host, self.port);
        tracing::info!("Connecting to BMW ENET at {}", addr);

        let mut stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| ObdError::ConnectionFailed(format!("Failed to connect to {addr}: {e}")))?;

        // DoIP Step 1: Send Routing Activation Request (0x0005)
        let activation_req = doip::build_routing_activation_request(self.tester_address);
        stream.write_all(&activation_req).await?;

        let mut ack_buf = [0u8; 13];
        timeout(Duration::from_millis(3000), stream.read_exact(&mut ack_buf))
            .await
            .map_err(|_| ObdError::Timeout(Duration::from_millis(3000)))??;

        doip::verify_routing_activation_ack(&ack_buf)?;

        self.stream = Some(stream);
        self.session_elevated = false;
        tracing::info!("BMW ENET DoIP Routing Activated successfully");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), ObdError> {
        if let Some(mut stream) = self.stream.take() {
            let _ = stream.shutdown().await;
        }
        self.session_elevated = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    async fn read_parameter(&mut self, param: ParameterType) -> Result<ParameterValue, ObdError> {
        let did = match param {
            ParameterType::EngineRpm => 0xD001,
            ParameterType::VehicleSpeed => 0xD002,
            ParameterType::CoolantTemperature => 0xD003,
            ParameterType::OilTemperature => 0xD004,
        };

        let request = uds::build_read_did_request(did);
        let response = match self.send_uds_request(&request).await {
            Ok(resp) => resp,
            Err(ObdError::NegativeResponse {
                service_id: 0x22,
                nrc: 0x22,
                ..
            }) => {
                // NRC 0x22 (ConditionsNotCorrect): Elevate session and retry once
                tracing::warn!(
                    "Received NRC 0x22 for DID {:#04X}; attempting session elevation...",
                    did
                );
                self.elevate_to_extended_session().await?;
                self.send_uds_request(&request).await?
            }
            Err(e) => return Err(e),
        };

        uds::parse_did_response(did, param, &response)
    }

    async fn read_dtcs(&mut self) -> Result<Vec<DtcRecord>, ObdError> {
        // UDS Service 0x19 0x02 0xFF (Report DTCs by Status Mask)
        let request = [0x19, 0x02, 0xFF];
        let response = self.send_uds_request(&request).await?;
        uds::parse_dtc_response(&response)
    }

    async fn clear_dtcs(&mut self) -> Result<(), ObdError> {
        // UDS Service 0x04 0xFF 0xFF 0xFF (Clear all DTC memory)
        let request = uds::build_clear_dtc_request();
        let response = self.send_uds_request(&request).await?;
        if response.first() == Some(&0x44) {
            Ok(())
        } else {
            Err(ObdError::Protocol("ECU rejected DTC clear request".into()))
        }
    }

    async fn read_battery_status(&mut self) -> Result<BatteryStatus, ObdError> {
        // Query IBS live data DID 0xD010
        let req = uds::build_read_did_request(0xD010);
        let resp = match self.send_uds_request(&req).await {
            Ok(r) => r,
            Err(ObdError::NegativeResponse {
                service_id: 0x22,
                nrc: 0x22,
                ..
            }) => {
                self.elevate_to_extended_session().await?;
                self.send_uds_request(&req).await?
            }
            Err(e) => return Err(e),
        };
        let mut status = uds::parse_battery_telemetry_response(0xD010, &resp)?;

        // Query battery replacement history DID 0x2036
        let hist_req = uds::build_read_did_request(0x2036);
        if let Ok(hist_resp) = self.send_uds_request(&hist_req).await {
            if let Ok(history) = uds::parse_battery_history_response(&hist_resp) {
                status.last_replacement_km = history;
            }
        }

        Ok(status)
    }

    async fn register_battery_replacement(&mut self) -> Result<String, ObdError> {
        // Ensure session is elevated to Extended Diagnostic Session (0x10 0x03)
        self.elevate_to_extended_session().await?;

        // Routine 0x0219: BMW DME Battery Replacement Registration
        let req = uds::build_start_routine(uds::ROUTINE_BMW_BATTERY_REGISTRATION, &[]);
        let resp = self.send_uds_request(&req).await?;
        uds::parse_routine_response(uds::ROUTINE_BMW_BATTERY_REGISTRATION, &resp)?;

        Ok("Battery replacement registered successfully in DME memory. Alternator charging curve & battery aging history reset to 0 km.".into())
    }

    fn profile(&self) -> VehicleProfile {
        VehicleProfile::BmwF10_535i
    }
}
