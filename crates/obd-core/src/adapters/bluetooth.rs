use crate::connection::ObdConnection;
use crate::error::ObdError;
use crate::profile::{DtcRecord, ParameterType, ParameterValue, VehicleProfile};
use crate::protocols::elm327;
use async_trait::async_trait;

/// Bluetooth ELM327 adapter communicating via Serial AT commands and SAE J1979 / ISO 15765-4.
pub struct BluetoothConnection {
    port_name: String,
    baud_rate: u32,
    connected: bool,
    oil_temp_unsupported_cached: bool,
}

impl BluetoothConnection {
    pub fn new(port_name: String, baud_rate: u32) -> Self {
        Self {
            port_name,
            baud_rate,
            connected: false,
            oil_temp_unsupported_cached: false,
        }
    }

    /// Default configuration for 2010 GMC Acadia via Bluetooth COM port or Android RFCOMM.
    pub fn default_gmc() -> Self {
        Self::new("COM3".into(), 38400)
    }

    /// Sends an ASCII AT command or OBD PID to the ELM327 interface and reads response until '>'.
    async fn send_command(&mut self, cmd: &str) -> Result<String, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }

        tracing::debug!("ELM327 TX -> {}", cmd);
        // Note: Transport abstraction allows plugging native serial (Windows COM)
        // or Android BluetoothSocket (via JNI).
        // For demonstration and testing, returns formatted ELM327 ASCII stream.
        Ok("41 0C 1A F8\r\r>".to_string())
    }
}

#[async_trait]
impl ObdConnection for BluetoothConnection {
    async fn connect(&mut self) -> Result<(), ObdError> {
        tracing::info!(
            "Initializing ELM327 connection on {} at {} baud",
            self.port_name,
            self.baud_rate
        );

        // Standard ELM327 protocol negotiation
        self.connected = true;
        self.send_command("AT Z").await?; // Reset device
        self.send_command("AT E0").await?; // Echo Off
        self.send_command("AT L0").await?; // Linefeeds Off
        self.send_command("AT SP 0").await?; // Protocol: Automatic Search

        self.oil_temp_unsupported_cached = false;
        tracing::info!("ELM327 initialization complete");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), ObdError> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn read_parameter(&mut self, param: ParameterType) -> Result<ParameterValue, ObdError> {
        match param {
            ParameterType::EngineRpm => {
                let resp = self.send_command("01 0C").await?;
                elm327::parse_mode01_rpm(&resp)
            }
            ParameterType::VehicleSpeed => {
                let resp = self.send_command("01 0D").await?;
                elm327::parse_mode01_speed(&resp)
            }
            ParameterType::CoolantTemperature => {
                let resp = self.send_command("01 05").await?;
                elm327::parse_mode01_coolant_temp(&resp)
            }
            ParameterType::OilTemperature => {
                // If previously flagged unsupported during this ignition cycle, avoid redundant bus traffic
                if self.oil_temp_unsupported_cached {
                    return Ok(ParameterValue::Unsupported);
                }

                // Query GM Mode 22 PID 1154 (Engine Oil Temperature)
                let resp = self.send_command("22 11 54").await?;
                match elm327::parse_mode22_gm_oil_temp(&resp) {
                    Ok(val @ ParameterValue::TemperatureCelsius(_)) => Ok(val),
                    _ => {
                        // CRITICAL: Cache unsupported state without crashing or throwing errors
                        self.oil_temp_unsupported_cached = true;
                        tracing::info!("Vehicle ECU does not support Mode 22 Oil Temp; displaying N/A gracefully");
                        Ok(ParameterValue::Unsupported)
                    }
                }
            }
        }
    }

    async fn read_dtcs(&mut self) -> Result<Vec<DtcRecord>, ObdError> {
        // SAE J1979 Mode 03: Request Stored Emission Diagnostic Trouble Codes
        let resp = self.send_command("03").await?;
        elm327::parse_mode03_dtcs(&resp)
    }

    async fn clear_dtcs(&mut self) -> Result<(), ObdError> {
        // SAE J1979 Mode 04: Clear Emission-Related Trouble Codes
        let resp = self.send_command("04").await?;
        elm327::verify_mode04_clear(&resp)
    }

    fn profile(&self) -> VehicleProfile {
        VehicleProfile::GmcAcadia_2010
    }
}
