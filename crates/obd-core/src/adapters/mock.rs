use crate::connection::ObdConnection;
use crate::error::ObdError;
use crate::profile::{
    BatteryStatus, DtcRecord, DtcStatus, ParameterType, ParameterValue, VehicleProfile,
};
use async_trait::async_trait;
use std::time::Instant;

/// Realistic vehicle simulator for automated testing and CI/CD without physical vehicle hardware.
pub struct MockConnection {
    profile: VehicleProfile,
    connected: bool,
    start_time: Instant,
    active_dtcs: Vec<DtcRecord>,
    battery_history: Vec<u32>,
    battery_soc: f32,
}

impl MockConnection {
    pub fn new(profile: VehicleProfile) -> Self {
        let (active_dtcs, battery_history, battery_soc) = match profile {
            VehicleProfile::BmwF10_535i => (
                vec![
                    DtcRecord {
                        code: "120408".into(),
                        description: crate::dtc_db::lookup_description("120408").into(),
                        status: DtcStatus::Confirmed,
                        ecu_source: Some("DME".into()),
                    },
                    DtcRecord {
                        code: "130104".into(),
                        description: crate::dtc_db::lookup_description("130104").into(),
                        status: DtcStatus::Pending,
                        ecu_source: Some("DME".into()),
                    },
                ],
                vec![142300, 78100],
                86.5,
            ),
            VehicleProfile::GmcAcadia_2010 => (
                vec![
                    DtcRecord {
                        code: "P0008".into(),
                        description: crate::dtc_db::lookup_description("P0008").into(),
                        status: DtcStatus::Confirmed,
                        ecu_source: Some("ECM".into()),
                    },
                    DtcRecord {
                        code: "P0300".into(),
                        description: crate::dtc_db::lookup_description("P0300").into(),
                        status: DtcStatus::Pending,
                        ecu_source: Some("ECM".into()),
                    },
                ],
                vec![],
                0.0,
            ),
            VehicleProfile::Simulator => (
                vec![DtcRecord {
                    code: "P0128".into(),
                    description: crate::dtc_db::lookup_description("P0128").into(),
                    status: DtcStatus::Confirmed,
                    ecu_source: Some("ECU".into()),
                }],
                vec![142300, 78100],
                86.5,
            ),
        };

        Self {
            profile,
            connected: false,
            start_time: Instant::now(),
            active_dtcs,
            battery_history,
            battery_soc,
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

    async fn read_battery_status(&mut self) -> Result<BatteryStatus, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                let elapsed = self.start_time.elapsed().as_secs_f32();
                // Terminal voltage: 12.6V resting or 14.2V charging when running
                let voltage = 14.15 + (elapsed * 0.2).sin() * 0.15;
                let current = 12.5 - (elapsed * 0.1).sin() * 5.0; // Positive charging current
                let temp = (22.0 + (elapsed * 0.1)).min(32.0);

                Ok(BatteryStatus {
                    supported: true,
                    state_of_charge_percent: self.battery_soc,
                    voltage,
                    current_amps: current,
                    temperature_celsius: temp,
                    capacity_ah: Some(90),
                    battery_type: Some("AGM (90 Ah)".into()),
                    last_replacement_km: self.battery_history.clone(),
                })
            }
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "Battery registration is only supported on BMW UDS profiles".into(),
            )),
        }
    }

    async fn register_battery_replacement(&mut self) -> Result<String, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                let current_km = 168450;
                self.battery_history.insert(0, current_km);
                if self.battery_history.len() > 3 {
                    self.battery_history.truncate(3);
                }
                self.battery_soc = 100.0;
                Ok(format!(
                    "Battery replacement successfully registered in simulated DME memory at {current_km} km. Alternator aging curves reset to 0 km."
                ))
            }
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "Battery registration is only supported on BMW UDS profiles".into(),
            )),
        }
    }

    fn profile(&self) -> VehicleProfile {
        self.profile
    }
}
