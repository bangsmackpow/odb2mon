use crate::connection::ObdConnection;
use crate::error::ObdError;
use crate::profile::{
    BatteryStatus, CbsItem, CbsItemType, CbsReport, CbsStatus, DtcRecord, DtcStatus, EcuHealth,
    EcuNode, ParameterType, ParameterValue, TopologyScanReport, VehicleProfile, BMW_F10_ECUS,
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
    cbs_items: Vec<CbsItem>,
}

impl MockConnection {
    pub fn new(profile: VehicleProfile) -> Self {
        let default_bmw_cbs = || {
            vec![
                CbsItem {
                    item_type: CbsItemType::EngineOil,
                    name: CbsItemType::EngineOil.display_name().into(),
                    status: CbsStatus::Ok,
                    remaining_percent: 42.0,
                    remaining_km: Some(6200),
                    due_date: Some("11/2026".into()),
                    reset_count: 3,
                },
                CbsItem {
                    item_type: CbsItemType::FrontBrakes,
                    name: CbsItemType::FrontBrakes.display_name().into(),
                    status: CbsStatus::Ok,
                    remaining_percent: 78.0,
                    remaining_km: Some(34000),
                    due_date: None,
                    reset_count: 1,
                },
                CbsItem {
                    item_type: CbsItemType::RearBrakes,
                    name: CbsItemType::RearBrakes.display_name().into(),
                    status: CbsStatus::Warning,
                    remaining_percent: 15.0,
                    remaining_km: Some(3100),
                    due_date: None,
                    reset_count: 2,
                },
                CbsItem {
                    item_type: CbsItemType::BrakeFluid,
                    name: CbsItemType::BrakeFluid.display_name().into(),
                    status: CbsStatus::Ok,
                    remaining_percent: 88.0,
                    remaining_km: None,
                    due_date: Some("04/2027".into()),
                    reset_count: 2,
                },
                CbsItem {
                    item_type: CbsItemType::VehicleCheck,
                    name: CbsItemType::VehicleCheck.display_name().into(),
                    status: CbsStatus::Ok,
                    remaining_percent: 55.0,
                    remaining_km: Some(18500),
                    due_date: Some("09/2027".into()),
                    reset_count: 1,
                },
            ]
        };

        let (active_dtcs, battery_history, battery_soc, cbs_items) = match profile {
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
                default_bmw_cbs(),
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
                vec![],
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
                default_bmw_cbs(),
            ),
        };

        Self {
            profile,
            connected: false,
            start_time: Instant::now(),
            active_dtcs,
            battery_history,
            battery_soc,
            cbs_items,
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

    async fn read_cbs_report(&mut self) -> Result<CbsReport, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => Ok(CbsReport {
                supported: true,
                items: self.cbs_items.clone(),
            }),
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "CBS monitoring is only supported on BMW UDS profiles".into(),
            )),
        }
    }

    async fn reset_cbs_item(&mut self, item_type: CbsItemType) -> Result<String, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                if let Some(item) = self.cbs_items.iter_mut().find(|i| i.item_type == item_type) {
                    item.remaining_percent = 100.0;
                    item.status = CbsStatus::Ok;
                    item.reset_count += 1;
                    match item_type {
                        CbsItemType::EngineOil => {
                            item.remaining_km = Some(15000);
                            item.due_date = Some("10/2028".into());
                        }
                        CbsItemType::FrontBrakes => {
                            item.remaining_km = Some(50000);
                        }
                        CbsItemType::RearBrakes => {
                            item.remaining_km = Some(50000);
                        }
                        CbsItemType::BrakeFluid => {
                            item.due_date = Some("10/2028".into());
                        }
                        CbsItemType::VehicleCheck => {
                            item.remaining_km = Some(60000);
                            item.due_date = Some("10/2030".into());
                        }
                        _ => {}
                    }
                    Ok(format!(
                        "Successfully reset Condition Based Service item: {} (Simulated Routine 0x0220 executed)",
                        item_type.display_name()
                    ))
                } else {
                    Err(ObdError::Protocol(format!(
                        "CBS item {:?} not found in vehicle configuration",
                        item_type
                    )))
                }
            }
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "CBS reset is only supported on BMW UDS profiles".into(),
            )),
        }
    }

    async fn scan_ecu_topology(&mut self) -> Result<TopologyScanReport, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }

        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                let mut nodes = Vec::new();
                let mut fault_count = 0;

                for (idx, ecu_def) in BMW_F10_ECUS.iter().enumerate() {
                    let (health, dtcs, dtc_count) = if ecu_def.logical_address == 0x0012 {
                        if self.active_dtcs.is_empty() {
                            (EcuHealth::Normal, vec![], 0)
                        } else {
                            (
                                EcuHealth::FaultStored,
                                self.active_dtcs.clone(),
                                self.active_dtcs.len(),
                            )
                        }
                    } else {
                        (EcuHealth::Normal, vec![], 0)
                    };

                    fault_count += dtc_count;
                    // Realistic simulated response latency (12ms - 38ms)
                    let response_time_ms = Some(12 + ((idx as u32 * 7) % 25));

                    nodes.push(EcuNode {
                        logical_address: ecu_def.logical_address,
                        short_name: ecu_def.short_name.to_string(),
                        full_name: ecu_def.full_name.to_string(),
                        bus: ecu_def.bus,
                        health,
                        dtc_count,
                        dtcs,
                        response_time_ms,
                    });
                }

                Ok(TopologyScanReport {
                    supported: true,
                    total_ecus_scanned: BMW_F10_ECUS.len(),
                    online_count: BMW_F10_ECUS.len(),
                    fault_count,
                    nodes,
                })
            }
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "ECU network topology scan is only supported on BMW UDS profiles".into(),
            )),
        }
    }

    fn profile(&self) -> VehicleProfile {
        self.profile
    }
}
