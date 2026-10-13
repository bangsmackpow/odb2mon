use crate::connection::ObdConnection;
use crate::error::ObdError;
use crate::profile::{
    BatteryStatus, CbsItem, CbsItemType, CbsReport, CbsStatus, CodingParameterDefinition,
    DtcRecord, DtcStatus, EcuHealth, EcuNode, EngineDatalogFrame, ParameterType, ParameterValue,
    TopologyScanReport, TransmissionOilStatus, VehicleProfile, WaterPumpBleedStatus, BMW_F10_ECUS,
};
use async_trait::async_trait;
use std::collections::HashMap;
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
    water_pump_bleed_start: Option<Instant>,
    water_pump_bleed_stopped: bool,
    trans_fluid_temp_c: f32,
    fan_duty_percent: u8,
    exhaust_flap_open: bool,
    coding_values: HashMap<String, u8>,
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

        let mut coding_values = HashMap::new();
        for param in crate::profile::get_default_bmw_f10_coding_parameters() {
            coding_values.insert(param.id, param.active_raw_value);
        }

        Self {
            profile,
            connected: false,
            start_time: Instant::now(),
            active_dtcs,
            battery_history,
            battery_soc,
            cbs_items,
            water_pump_bleed_start: None,
            water_pump_bleed_stopped: false,
            trans_fluid_temp_c: 44.5,
            fan_duty_percent: 0,
            exhaust_flap_open: false,
            coding_values,
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

    async fn start_water_pump_bleed(&mut self) -> Result<String, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                self.water_pump_bleed_start = Some(Instant::now());
                self.water_pump_bleed_stopped = false;
                Ok("Electric water pump automated bleeding sequence initiated (Simulated Routine 0x0211 running). Purging air pockets across N55 cooling circuit.".into())
            }
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "Electric water pump bleeding routine is only supported on BMW UDS profiles".into(),
            )),
        }
    }

    async fn stop_water_pump_bleed(&mut self) -> Result<String, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                self.water_pump_bleed_start = None;
                self.water_pump_bleed_stopped = true;
                Ok("Electric water pump bleeding routine stopped. Normal DME coolant regulation restored.".into())
            }
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "Electric water pump bleeding routine is only supported on BMW UDS profiles".into(),
            )),
        }
    }

    async fn get_water_pump_bleed_status(&mut self) -> Result<WaterPumpBleedStatus, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                if let Some(start) = self.water_pump_bleed_start {
                    if !self.water_pump_bleed_stopped {
                        let elapsed_sec = start.elapsed().as_secs() as u32;
                        let demo_duration = 60u32;
                        if elapsed_sec >= demo_duration {
                            return Ok(WaterPumpBleedStatus {
                                is_running: false,
                                progress_percent: 100.0,
                                pump_speed_percent: 0,
                                elapsed_seconds: demo_duration,
                                remaining_seconds: 0,
                                stage_description: "Bleeding cycle complete. Air pockets purged."
                                    .into(),
                            });
                        }

                        let progress_percent = (elapsed_sec as f32 / demo_duration as f32) * 100.0;
                        let remaining_seconds = demo_duration.saturating_sub(elapsed_sec);

                        let (pump_speed_percent, stage_description) = if elapsed_sec < 10 {
                            (15, "Stage 1: Priming & bubble extraction (15% PWM)".into())
                        } else if elapsed_sec < 25 {
                            (
                                50,
                                "Stage 2: Intermediate coolant displacement (50% PWM)".into(),
                            )
                        } else if elapsed_sec < 45 {
                            (
                                85,
                                "Stage 3: High-velocity air pocket purge (85% PWM)".into(),
                            )
                        } else {
                            (
                                100,
                                "Stage 4: Maximum flow & thermal equalization (100% PWM)".into(),
                            )
                        };

                        return Ok(WaterPumpBleedStatus {
                            is_running: true,
                            progress_percent,
                            pump_speed_percent,
                            elapsed_seconds: elapsed_sec,
                            remaining_seconds,
                            stage_description,
                        });
                    }
                }

                if self.water_pump_bleed_stopped {
                    Ok(WaterPumpBleedStatus {
                        is_running: false,
                        progress_percent: 0.0,
                        pump_speed_percent: 0,
                        elapsed_seconds: 0,
                        remaining_seconds: 720,
                        stage_description: "Cancelled / Stopped by technician".into(),
                    })
                } else {
                    Ok(WaterPumpBleedStatus::default())
                }
            }
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "Electric water pump bleeding routine is only supported on BMW UDS profiles".into(),
            )),
        }
    }

    async fn get_transmission_oil_status(&mut self) -> Result<TransmissionOilStatus, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                let temp_c = self.trans_fluid_temp_c;
                let temp_f = (temp_c * 9.0 / 5.0) + 32.0;
                let is_within_fill_window = (40.0..=50.0).contains(&temp_c);
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
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "Transmission fluid temperature is only supported on BMW UDS profiles".into(),
            )),
        }
    }

    async fn start_transmission_fill_routine(&mut self) -> Result<String, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                self.trans_fluid_temp_c = 44.5;
                Ok("ZF 8HP fluid level preparation routine initiated on EGS (Simulated Routine 0x0225). Holding torque converter fill RPM.".into())
            }
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "Transmission fill routine is only supported on BMW UDS profiles".into(),
            )),
        }
    }

    async fn set_radiator_fan_duty(&mut self, duty_percent: u8) -> Result<String, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                self.fan_duty_percent = duty_percent;
                if duty_percent == 0 {
                    Ok("Electric cooling fan returned to automatic DME control.".into())
                } else {
                    Ok(format!(
                        "Electric cooling fan commanded to {duty_percent}% duty cycle."
                    ))
                }
            }
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "Radiator fan actuator is only supported on BMW UDS profiles".into(),
            )),
        }
    }

    async fn set_exhaust_flap(&mut self, open: bool) -> Result<String, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                self.exhaust_flap_open = open;
                if open {
                    Ok("Exhaust flapper valve forced OPEN (Sport sound).".into())
                } else {
                    Ok("Exhaust flapper valve commanded CLOSED (Quiet sound).".into())
                }
            }
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "Exhaust flap actuator is only supported on BMW UDS profiles".into(),
            )),
        }
    }

    async fn read_engine_datalog(&mut self) -> Result<EngineDatalogFrame, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;

                let elapsed = self.start_time.elapsed().as_secs_f32();
                let cycle_t = elapsed % 20.0;

                let (
                    rpm,
                    speed,
                    boost_tgt_hpa,
                    boost_act_hpa,
                    wgdc,
                    rail_tgt,
                    rail_act,
                    lpfp,
                    lambda,
                    iat,
                    maf,
                    timing,
                    knock_retard,
                    vanos_in_tgt,
                    vanos_in_act,
                    vanos_ex_tgt,
                    vanos_ex_act,
                ) = if cycle_t < 3.0 {
                    // Stage 1: Idle
                    (
                        720, 0, 1013.0, 1011.0, 0.0, 50.0, 50.2, 5.3, 1.00, 32.0, 4.6, 5.5,
                        [0.0; 6], 85.0, 85.1, 115.0, 114.9,
                    )
                } else if cycle_t < 11.0 {
                    // Stage 2: 3rd Gear WOT Dyno / Acceleration Pull (3s to 11s)
                    let p = (cycle_t - 3.0) / 8.0; // 0.0 -> 1.0
                    let r = 2200.0 + p * 4400.0; // 2200 to 6600 RPM
                    let s = 45.0 + p * 110.0; // 45 to 155 km/h
                    let b_tgt = 2150.0; // ~16.5 psi
                                        // Spool simulation with 0.15s lag curve
                    let spool = (p * 1.25).min(1.0);
                    let b_act = 1013.0 + spool * (b_tgt - 1013.0) - (p * 15.0).sin() * 25.0;
                    let w = if p < 0.25 { 85.0 } else { 62.0 + (p * 5.0) };
                    let r_tgt = 190.0;
                    let r_act = 186.0 + (p * 3.5);
                    let l = 0.94 - (p * 0.10); // 0.94 -> 0.84 Lambda
                    let i = 33.0 + (p * 11.0); // 33°C -> 44°C IAT
                    let m = 35.0 + (p * 220.0); // 35 -> 255 g/s MAF
                    let t = 8.0 + (p * 4.5); // 8.0° -> 12.5° timing advance
                    let mut kr = [0.0; 6];
                    if p > 0.4 && p < 0.7 {
                        kr[1] = -1.2; // Minor Cyl 2 knock correction
                        kr[4] = -0.6; // Minor Cyl 5 knock correction
                    }
                    (
                        r as u16, s as u8, b_tgt, b_act, w, r_tgt, r_act, 5.1, l, i, m, t, kr,
                        122.0, 121.7, 98.0, 98.2,
                    )
                } else if cycle_t < 15.0 {
                    // Stage 3: Throttle Lift / Shift
                    (
                        3200, 140, 820.0, 810.0, 12.0, 120.0, 122.0, 5.3, 1.28, 38.0, 12.0, 2.0,
                        [0.0; 6], 90.0, 90.2, 110.0, 109.8,
                    )
                } else {
                    // Stage 4: Steady Cruise
                    (
                        1950, 95, 1060.0, 1058.0, 18.0, 85.0, 85.4, 5.2, 1.00, 34.0, 24.0, 18.0,
                        [0.0; 6], 100.0, 100.1, 105.0, 105.2,
                    )
                };

                let boost_tgt_psi = (boost_tgt_hpa - 1013.25) * 0.0145038;
                let boost_act_psi = (boost_act_hpa - 1013.25) * 0.0145038;

                Ok(EngineDatalogFrame {
                    timestamp_ms: now_ms,
                    engine_rpm: rpm,
                    vehicle_speed_kmh: speed,
                    boost_target_hpa: boost_tgt_hpa,
                    boost_actual_hpa: boost_act_hpa,
                    boost_target_psi: boost_tgt_psi,
                    boost_actual_psi: boost_act_psi,
                    wastegate_duty_percent: wgdc,
                    rail_pressure_target_bar: rail_tgt,
                    rail_pressure_actual_bar: rail_act,
                    low_pressure_fuel_bar: lpfp,
                    lambda,
                    intake_air_temp_c: iat,
                    mass_air_flow_gps: maf,
                    ignition_timing_cyl1: timing,
                    knock_retard,
                    misfire_counts: [0, 0, 0, 0, 0, 0],
                    vanos_intake_target_deg: vanos_in_tgt,
                    vanos_intake_actual_deg: vanos_in_act,
                    vanos_exhaust_target_deg: vanos_ex_tgt,
                    vanos_exhaust_actual_deg: vanos_ex_act,
                })
            }
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "High-speed engine datalogging is only supported on BMW N55 UDS profiles".into(),
            )),
        }
    }

    async fn read_vin(&mut self) -> Result<String, ObdError> {
        if !self.connected {
            return Err(ObdError::Disconnected);
        }
        match self.profile {
            VehicleProfile::BmwF10_535i => Ok("WBAFR7C58BC123456".into()),
            VehicleProfile::GmcAcadia_2010 => Ok("1GKKVRED9AJ123456".into()),
            VehicleProfile::Simulator => Ok("WBAFR7C58BC123456".into()),
        }
    }

    async fn get_coding_parameters(&mut self) -> Result<Vec<CodingParameterDefinition>, ObdError> {
        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                let mut params = crate::profile::get_default_bmw_f10_coding_parameters();
                for param in &mut params {
                    if let Some(&val) = self.coding_values.get(&param.id) {
                        param.active_raw_value = val;
                        if let Some(opt) = param.options.iter().find(|o| o.raw_value == val) {
                            param.active_value = opt.label.clone();
                        }
                    }
                }
                Ok(params)
            }
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "Vehicle coding is only supported on BMW F/G-Series via ENET".into(),
            )),
        }
    }

    async fn write_coding_parameter(
        &mut self,
        param_id: &str,
        raw_value: u8,
    ) -> Result<String, ObdError> {
        match self.profile {
            VehicleProfile::BmwF10_535i | VehicleProfile::Simulator => {
                let mut params = crate::profile::get_default_bmw_f10_coding_parameters();
                let Some(param) = params.iter_mut().find(|p| p.id == param_id) else {
                    return Err(ObdError::Unsupported(format!(
                        "Unknown coding parameter ID: {param_id}"
                    )));
                };

                let prev_raw = self
                    .coding_values
                    .get(param_id)
                    .copied()
                    .unwrap_or(param.active_raw_value);
                let prev_label = param
                    .options
                    .iter()
                    .find(|o| o.raw_value == prev_raw)
                    .map(|o| o.label.clone())
                    .unwrap_or_else(|| format!("{:#04X}", prev_raw));

                let new_label = param
                    .options
                    .iter()
                    .find(|o| o.raw_value == raw_value)
                    .map(|o| o.label.clone())
                    .unwrap_or_else(|| format!("{:#04X}", raw_value));

                // Save simulated backup to persistent garage
                let garage = crate::garage::load_garage();
                let backup_record = crate::profile::CodingBackupRecord {
                    id: format!(
                        "backup_{}_{}_{}",
                        param.id,
                        param.target_ecu.to_lowercase(),
                        chrono::Utc::now().timestamp_millis()
                    ),
                    timestamp: crate::garage::current_iso_timestamp(),
                    vehicle_id: garage.active_vehicle_id.clone(),
                    param_id: param.id.clone(),
                    param_name: param.name.clone(),
                    ecu_name: param.target_ecu.clone(),
                    ecu_address: param.ecu_address,
                    previous_value_label: prev_label,
                    previous_raw_value: prev_raw,
                    new_value_label: new_label.clone(),
                    new_raw_value: raw_value,
                    payload_hex: format!("{prev_raw:02X} 00 00 00"),
                };
                let _ =
                    crate::garage::save_coding_backup(&garage.active_vehicle_id, &backup_record);

                // Update simulated memory
                self.coding_values.insert(param_id.to_string(), raw_value);

                Ok(format!(
                    "Successfully coded '{}' to '{}' on {} ({:#04X}). Simulated ECU soft reset performed.",
                    param.name, new_label, param.target_ecu, param.ecu_address
                ))
            }
            VehicleProfile::GmcAcadia_2010 => Err(ObdError::Unsupported(
                "Vehicle coding is only supported on BMW F/G-Series via ENET".into(),
            )),
        }
    }

    async fn restore_coding_backup(&mut self, backup_id: &str) -> Result<String, ObdError> {
        let garage = crate::garage::load_garage();
        let backup = crate::garage::load_coding_backup(&garage.active_vehicle_id, backup_id)?;

        self.coding_values
            .insert(backup.param_id.clone(), backup.previous_raw_value);

        Ok(format!(
            "Successfully restored '{}' to previous state '{}' from backup.",
            backup.param_name, backup.previous_value_label
        ))
    }

    fn profile(&self) -> VehicleProfile {
        self.profile
    }
}
