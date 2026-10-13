use crate::connection::ObdConnection;
use crate::error::ObdError;
use crate::profile::{
    BatteryStatus, CbsItemType, CbsReport, DtcRecord, EcuHealth, EcuNode, EngineDatalogFrame,
    ParameterType, ParameterValue, TopologyScanReport, TransmissionOilStatus, VehicleProfile,
    WaterPumpBleedStatus,
};
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

    /// Sends a raw UDS payload wrapped in a DoIP diagnostic message frame to a specified ECU address.
    async fn send_uds_request_to_ecu(
        &mut self,
        target_ecu: u16,
        payload: &[u8],
    ) -> Result<Vec<u8>, ObdError> {
        let stream = self.stream.as_mut().ok_or(ObdError::Disconnected)?;

        let doip_frame = doip::build_diagnostic_message(self.tester_address, target_ecu, payload);

        stream.write_all(&doip_frame).await?;

        // Read DoIP 8-byte header
        let mut header_buf = [0u8; 8];
        timeout(
            Duration::from_millis(1500),
            stream.read_exact(&mut header_buf),
        )
        .await
        .map_err(|_| ObdError::Timeout(Duration::from_millis(1500)))??;

        let payload_len =
            u32::from_be_bytes([header_buf[4], header_buf[5], header_buf[6], header_buf[7]])
                as usize;

        let mut payload_buf = vec![0u8; payload_len];
        stream.read_exact(&mut payload_buf).await?;

        doip::extract_uds_payload(&payload_buf)
    }

    /// Sends a raw UDS payload wrapped in a DoIP diagnostic message frame to default DME.
    async fn send_uds_request(&mut self, payload: &[u8]) -> Result<Vec<u8>, ObdError> {
        self.send_uds_request_to_ecu(self.target_ecu, payload).await
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

    async fn read_cbs_report(&mut self) -> Result<CbsReport, ObdError> {
        let req = uds::build_read_did_request(uds::DID_BMW_CBS_OVERVIEW);
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
        uds::parse_cbs_report_response(uds::DID_BMW_CBS_OVERVIEW, &resp)
    }

    async fn reset_cbs_item(&mut self, item: CbsItemType) -> Result<String, ObdError> {
        self.elevate_to_extended_session().await?;
        let req = uds::build_cbs_reset_routine(item);
        let resp = self.send_uds_request(&req).await?;
        uds::parse_routine_response(uds::ROUTINE_BMW_CBS_RESET, &resp)?;
        Ok(format!(
            "Successfully reset Condition Based Service item: {} (Routine 0x0220 executed)",
            item.display_name()
        ))
    }

    async fn scan_ecu_topology(&mut self) -> Result<TopologyScanReport, ObdError> {
        let mut nodes = Vec::new();
        let mut online_count = 0;
        let mut fault_count = 0;

        for ecu_def in crate::profile::BMW_F10_ECUS {
            let start = std::time::Instant::now();
            let req = [0x19, 0x02, 0xFF];
            match self
                .send_uds_request_to_ecu(ecu_def.logical_address, &req)
                .await
            {
                Ok(resp) => {
                    let elapsed = start.elapsed().as_millis() as u32;
                    let dtcs = uds::parse_dtc_response(&resp).unwrap_or_default();
                    let dtcs: Vec<DtcRecord> = dtcs
                        .into_iter()
                        .map(|mut d| {
                            d.ecu_source = Some(ecu_def.short_name.to_string());
                            d
                        })
                        .collect();

                    let health = if dtcs.is_empty() {
                        EcuHealth::Normal
                    } else {
                        EcuHealth::FaultStored
                    };

                    online_count += 1;
                    if !dtcs.is_empty() {
                        fault_count += dtcs.len();
                    }

                    nodes.push(EcuNode {
                        logical_address: ecu_def.logical_address,
                        short_name: ecu_def.short_name.to_string(),
                        full_name: ecu_def.full_name.to_string(),
                        bus: ecu_def.bus,
                        health,
                        dtc_count: dtcs.len(),
                        dtcs,
                        response_time_ms: Some(elapsed),
                    });
                }
                Err(ObdError::NegativeResponse {
                    service_id: 0x19, ..
                }) => {
                    let elapsed = start.elapsed().as_millis() as u32;
                    online_count += 1;
                    nodes.push(EcuNode {
                        logical_address: ecu_def.logical_address,
                        short_name: ecu_def.short_name.to_string(),
                        full_name: ecu_def.full_name.to_string(),
                        bus: ecu_def.bus,
                        health: EcuHealth::CommError,
                        dtc_count: 0,
                        dtcs: Vec::new(),
                        response_time_ms: Some(elapsed),
                    });
                }
                Err(_) => {
                    nodes.push(EcuNode {
                        logical_address: ecu_def.logical_address,
                        short_name: ecu_def.short_name.to_string(),
                        full_name: ecu_def.full_name.to_string(),
                        bus: ecu_def.bus,
                        health: EcuHealth::NoResponse,
                        dtc_count: 0,
                        dtcs: Vec::new(),
                        response_time_ms: None,
                    });
                }
            }
        }

        Ok(TopologyScanReport {
            supported: true,
            total_ecus_scanned: crate::profile::BMW_F10_ECUS.len(),
            online_count,
            fault_count,
            nodes,
        })
    }

    async fn start_water_pump_bleed(&mut self) -> Result<String, ObdError> {
        self.elevate_to_extended_session().await?;
        let req = uds::build_start_routine(uds::ROUTINE_BMW_WATER_PUMP_BLEED, &[]);
        let resp = self.send_uds_request(&req).await?;
        uds::parse_routine_response(uds::ROUTINE_BMW_WATER_PUMP_BLEED, &resp)?;
        Ok("Electric water pump automated bleeding sequence initiated (Routine 0x0211 running). Purging air pockets across N55 cooling circuit.".into())
    }

    async fn stop_water_pump_bleed(&mut self) -> Result<String, ObdError> {
        self.elevate_to_extended_session().await?;
        let req = uds::build_stop_routine(uds::ROUTINE_BMW_WATER_PUMP_BLEED);
        let resp = self.send_uds_request(&req).await?;
        uds::parse_routine_response(uds::ROUTINE_BMW_WATER_PUMP_BLEED, &resp)?;
        Ok(
            "Electric water pump bleeding routine stopped. Normal DME coolant regulation restored."
                .into(),
        )
    }

    async fn get_water_pump_bleed_status(&mut self) -> Result<WaterPumpBleedStatus, ObdError> {
        let req = uds::build_routine_control_request(
            uds::ROUTINE_CONTROL_REQUEST_RESULTS,
            uds::ROUTINE_BMW_WATER_PUMP_BLEED,
            &[],
        );
        if let Ok(resp) = self.send_uds_request(&req).await {
            if let Ok(data) = uds::parse_routine_response(uds::ROUTINE_BMW_WATER_PUMP_BLEED, &resp)
            {
                if !data.is_empty() {
                    let is_running = data[0] == 0x01;
                    let progress = data.get(1).copied().unwrap_or(0) as f32;
                    let speed = data.get(2).copied().unwrap_or(0);
                    let elapsed = data.get(3).copied().unwrap_or(0) as u32 * 10;
                    return Ok(WaterPumpBleedStatus {
                        is_running,
                        progress_percent: progress.clamp(0.0, 100.0),
                        pump_speed_percent: speed,
                        elapsed_seconds: elapsed,
                        remaining_seconds: 720usize.saturating_sub(elapsed as usize) as u32,
                        stage_description: if is_running {
                            "Automated pulsing active".into()
                        } else {
                            "Idle / Completed".into()
                        },
                    });
                }
            }
        }
        Ok(WaterPumpBleedStatus::default())
    }

    async fn get_transmission_oil_status(&mut self) -> Result<TransmissionOilStatus, ObdError> {
        let req = uds::build_read_did_request(uds::DID_BMW_TRANSMISSION_SUMP_TEMP);
        // Query EGS directly at address 0x0018
        let resp = self.send_uds_request_to_ecu(0x0018, &req).await?;
        let temp_c = uds::parse_transmission_temp_response(&resp)?;
        let temp_f = (temp_c * 9.0 / 5.0) + 32.0;
        let is_within_fill_window = temp_c >= 40.0 && temp_c <= 50.0;
        let status_text = if temp_c < 40.0 {
            "COLD - Run engine to warm fluid to 40°C - 50°C".into()
        } else if temp_c <= 50.0 {
            "OPTIMAL FILL WINDOW (40°C - 50°C) - Ready to check/fill level".into()
        } else {
            "OVER-TEMPERATURE (> 50°C) - Allow transmission to cool before filling".into()
        };

        Ok(TransmissionOilStatus {
            supported: true,
            temperature_c: temp_c,
            temperature_f: temp_f,
            is_within_fill_window,
            status_text,
        })
    }

    async fn start_transmission_fill_routine(&mut self) -> Result<String, ObdError> {
        let req = uds::build_start_routine(uds::ROUTINE_BMW_EGS_OIL_LEVEL, &[]);
        let resp = self.send_uds_request_to_ecu(0x0018, &req).await?;
        uds::parse_routine_response(uds::ROUTINE_BMW_EGS_OIL_LEVEL, &resp)?;
        Ok("ZF 8HP fluid level preparation routine initiated on EGS (Routine 0x0225). Verify engine idling in P/N.".into())
    }

    async fn set_radiator_fan_duty(&mut self, duty_percent: u8) -> Result<String, ObdError> {
        self.elevate_to_extended_session().await?;
        let req = uds::build_radiator_fan_request(duty_percent);
        let resp = self.send_uds_request(&req).await?;
        uds::parse_routine_response(uds::ROUTINE_BMW_RADIATOR_FAN, &resp)?;
        if duty_percent == 0 {
            Ok("Electric cooling fan returned to automatic DME control.".into())
        } else {
            Ok(format!(
                "Electric cooling fan commanded to {duty_percent}% duty cycle."
            ))
        }
    }

    async fn set_exhaust_flap(&mut self, open: bool) -> Result<String, ObdError> {
        self.elevate_to_extended_session().await?;
        let req = uds::build_exhaust_flap_request(open);
        let resp = self.send_uds_request(&req).await?;
        uds::parse_routine_response(uds::ROUTINE_BMW_EXHAUST_FLAP, &resp)?;
        if open {
            Ok("Exhaust flapper valve forced OPEN (Sport sound).".into())
        } else {
            Ok("Exhaust flapper valve commanded CLOSED (Quiet sound).".into())
        }
    }

    async fn read_engine_datalog(&mut self) -> Result<EngineDatalogFrame, ObdError> {
        let req = uds::build_read_did_request(uds::DID_BMW_N55_DATALOG);
        let resp = self.send_uds_request(&req).await?;
        let mut frame = uds::parse_n55_datalog_response(&resp)?;
        frame.timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Ok(frame)
    }

    fn profile(&self) -> VehicleProfile {
        VehicleProfile::BmwF10_535i
    }
}
