use crate::connection::ObdConnection;
use crate::error::ObdError;
use crate::profile::{DtcRecord, DtcStatus, ParameterType, ParameterValue, VehicleProfile};
use async_trait::async_trait;
use std::time::Instant;

/// Realistic vehicle simulator for automated testing and CI/CD without physical vehicle hardware.
pub struct MockConnection {
    profile: VehicleProfile,
    connected: bool,
    start_time: Instant,
    active_dtcs: Vec<DtcRecord>,
}

impl MockConnection {
    pub fn new(profile: VehicleProfile) -> Self {
        let active_dtcs = match profile {
            VehicleProfile::BmwF10_535i => vec![
                DtcRecord {
                    code: "120408".into(),
                    description:
                        "Charging pressure control: Switch-off, pressure build-up disabled".into(),
                    status: DtcStatus::Confirmed,
                    ecu_source: Some("DME".into()),
                },
                DtcRecord {
                    code: "130104".into(),
                    description: "VANOS, intake: Camshaft angle not reached".into(),
                    status: DtcStatus::Pending,
                    ecu_source: Some("DME".into()),
                },
            ],
            VehicleProfile::GmcAcadia_2010 => vec![
                DtcRecord {
                    code: "P0008".into(),
                    description: "Engine Positions System Performance Bank 1 (Timing Chain Slack)"
                        .into(),
                    status: DtcStatus::Confirmed,
                    ecu_source: Some("ECM".into()),
                },
                DtcRecord {
                    code: "P0300".into(),
                    description: "Random/Multiple Cylinder Misfire Detected".into(),
                    status: DtcStatus::Pending,
                    ecu_source: Some("ECM".into()),
                },
            ],
            VehicleProfile::Simulator => vec![DtcRecord {
                code: "P0128".into(),
                description: "Coolant Thermostat (Coolant Temp Below Regulating Temp)".into(),
                status: DtcStatus::Confirmed,
                ecu_source: Some("ECU".into()),
            }],
        };

        Self {
            profile,
            connected: false,
            start_time: Instant::now(),
            active_dtcs,
        }
    }
}

#[async_trait]
impl ObdConnection for MockConnection {
    async fn connect(&mut self) -> Result<(), ObdError> {
        self.connected = true;
        self.start_time = Instant::now();
        tracing::info!(
            "Mock vehicle simulator connected for profile: {:?}",
            self.profile
        );
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), ObdError> {
        self.connected = false;
        tracing::info!("Mock vehicle simulator disconnected");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    async fn read_parameter(&mut self, param: ParameterType) -> Result<ParameterValue, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }

        let elapsed = self.start_time.elapsed().as_secs_f32();

        match param {
            ParameterType::EngineRpm => {
                // Simulates idle oscillation (750 RPM) and driving acceleration spikes
                let wave = (elapsed * 0.8).sin();
                let rpm = (2100.0 + wave * 1350.0).max(750.0) as u32;
                Ok(ParameterValue::Rpm(rpm))
            }
            ParameterType::VehicleSpeed => {
                let wave = (elapsed * 0.4).sin();
                let speed = ((wave + 1.0) * 45.0).max(0.0) as u32;
                Ok(ParameterValue::SpeedKmh(speed))
            }
            ParameterType::CoolantTemperature => {
                // Thermodynamic warmup from 20°C to 92°C
                let temp = (20.0 + (elapsed * 1.5)).min(92.0);
                Ok(ParameterValue::TemperatureCelsius(temp))
            }
            ParameterType::OilTemperature => {
                match self.profile {
                    VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                        // BMW N55 oil warms gradually to 102°C operating temp
                        let temp = (20.0 + (elapsed * 1.1)).min(102.0);
                        Ok(ParameterValue::TemperatureCelsius(temp))
                    }
                    VehicleProfile::GmcAcadia_2010 => {
                        // Accurately simulate GMC unsupported oil PID behavior
                        Ok(ParameterValue::Unsupported)
                    }
                }
            }
        }
    }

    async fn read_dtcs(&mut self) -> Result<Vec<DtcRecord>, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        Ok(self.active_dtcs.clone())
    }

    async fn clear_dtcs(&mut self) -> Result<(), ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        self.active_dtcs.clear();
        tracing::info!("Mock DTCs cleared successfully");
        Ok(())
    }

    fn profile(&self) -> VehicleProfile {
        self.profile
    }
}
