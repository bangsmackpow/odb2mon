use crate::connection::ObdConnection;
use crate::profile::{
    DashboardSnapshot, DatalogRecordingSummary, DataloggerStatus, EngineDatalogFrame,
    ParameterType, ParameterValue,
};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, watch, Mutex};
use tokio::time::{sleep, Duration};

static FALLBACK_RUNTIME: std::sync::OnceLock<tokio::runtime::Runtime> = std::sync::OnceLock::new();

#[derive(Default)]
struct DatalogSession {
    is_recording: bool,
    start_time: Option<Instant>,
    frames: Vec<EngineDatalogFrame>,
}

/// Asynchronous dashboard engine that decouples vehicle bus I/O from UI rendering.
pub struct VehicleManager {
    connection: Arc<Mutex<Box<dyn ObdConnection>>>,
    snapshot_tx: watch::Sender<DashboardSnapshot>,
    snapshot_rx: watch::Receiver<DashboardSnapshot>,
    event_tx: mpsc::Sender<String>,
    datalog_session: Arc<Mutex<DatalogSession>>,
    active_vehicle_id: Arc<Mutex<String>>,
}

impl VehicleManager {
    pub fn new(connection: Box<dyn ObdConnection>) -> (Self, mpsc::Receiver<String>) {
        Self::new_with_vehicle(connection, "veh_bmw_f10_535i".into())
    }

    pub fn new_with_vehicle(
        connection: Box<dyn ObdConnection>,
        vehicle_id: String,
    ) -> (Self, mpsc::Receiver<String>) {
        let (snapshot_tx, snapshot_rx) = watch::channel(DashboardSnapshot::default());
        let (event_tx, event_rx) = mpsc::channel(64);

        let manager = Self {
            connection: Arc::new(Mutex::new(connection)),
            snapshot_tx,
            snapshot_rx,
            event_tx,
            datalog_session: Arc::new(Mutex::new(DatalogSession::default())),
            active_vehicle_id: Arc::new(Mutex::new(vehicle_id)),
        };

        (manager, event_rx)
    }

    /// Returns currently active vehicle ID.
    pub async fn get_active_vehicle_id(&self) -> String {
        self.active_vehicle_id.lock().await.clone()
    }

    /// Sets active vehicle ID for persistent logging.
    pub async fn set_active_vehicle_id(&self, id: String) {
        let mut cur = self.active_vehicle_id.lock().await;
        *cur = id;
    }

    /// Reads vehicle VIN across active adapter protocol.
    pub async fn read_vin(&self) -> Result<String, crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        conn.read_vin().await
    }

    /// Subscribes to real-time telemetry snapshots (suitable for UI watchers/events).
    pub fn subscribe(&self) -> watch::Receiver<DashboardSnapshot> {
        self.snapshot_rx.clone()
    }

    /// Starts the asynchronous polling loop at the specified polling interval (e.g. 100ms for 10 Hz).
    pub fn spawn_polling_loop(&self, poll_interval: Duration) -> tokio::task::JoinHandle<()> {
        let connection = Arc::clone(&self.connection);
        let snapshot_tx = self.snapshot_tx.clone();
        let event_tx = self.event_tx.clone();

        let task = async move {
            tracing::info!(
                "Starting background vehicle polling task at {:?}",
                poll_interval
            );

            // Connect if not already connected
            {
                let mut conn = connection.lock().await;
                if !conn.is_connected() {
                    if let Err(e) = conn.connect().await {
                        let _ = event_tx.send(format!("Connection error: {e}")).await;
                    }
                }
            }

            loop {
                let mut conn = connection.lock().await;
                if conn.is_connected() {
                    let mut snapshot = DashboardSnapshot {
                        connection_status: "Connected".into(),
                        ..Default::default()
                    };

                    // Query RPM
                    if let Ok(ParameterValue::Rpm(rpm)) =
                        conn.read_parameter(ParameterType::EngineRpm).await
                    {
                        snapshot.rpm = Some(rpm);
                    }

                    // Query Speed
                    if let Ok(ParameterValue::SpeedKmh(spd)) =
                        conn.read_parameter(ParameterType::VehicleSpeed).await
                    {
                        snapshot.speed_kmh = Some(spd);
                    }

                    // Query Coolant
                    if let Ok(ParameterValue::TemperatureCelsius(c)) =
                        conn.read_parameter(ParameterType::CoolantTemperature).await
                    {
                        snapshot.coolant_temp_c = Some(c);
                    }

                    // Query Oil Temp
                    match conn.read_parameter(ParameterType::OilTemperature).await {
                        Ok(ParameterValue::TemperatureCelsius(c)) => {
                            snapshot.oil_temp_c = Some(c);
                            snapshot.oil_temp_supported = true;
                        }
                        Ok(ParameterValue::Unsupported) => {
                            snapshot.oil_temp_supported = false;
                        }
                        _ => {}
                    }

                    // Fetch active DTCs
                    if let Ok(dtcs) = conn.read_dtcs().await {
                        snapshot.active_dtcs = dtcs;
                    }

                    let _ = snapshot_tx.send(snapshot);
                } else {
                    let snapshot = DashboardSnapshot {
                        connection_status: "Disconnected".into(),
                        ..Default::default()
                    };
                    let _ = snapshot_tx.send(snapshot);
                }

                drop(conn);
                sleep(poll_interval).await;
            }
        };

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(task)
        } else {
            let rt = FALLBACK_RUNTIME.get_or_init(|| {
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .thread_name("obd2mon-poller")
                    .build()
                    .expect("Failed to initialize background Tokio runtime")
            });
            rt.spawn(task)
        }
    }

    /// Clears vehicle diagnostic trouble codes across the abstraction layer.
    pub async fn clear_dtcs(&self) -> Result<(), crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        conn.clear_dtcs().await?;
        let active_id = self.active_vehicle_id.lock().await.clone();
        let _ = crate::garage::append_service_log(
            &active_id,
            crate::garage::VehicleServiceLogEntry {
                timestamp: crate::garage::current_iso_timestamp(),
                mileage: None,
                category: "dtc_clear".into(),
                description: "Cleared diagnostic trouble codes across all ECUs".into(),
                result: "Success".into(),
            },
        );
        Ok(())
    }

    /// Queries Intelligent Battery Sensor (IBS) telemetry from vehicle controller.
    pub async fn read_battery_status(
        &self,
    ) -> Result<crate::profile::BatteryStatus, crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        conn.read_battery_status().await
    }

    /// Registers a newly installed battery in engine controller memory.
    pub async fn register_battery_replacement(&self) -> Result<String, crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        let res = conn.register_battery_replacement().await?;
        let active_id = self.active_vehicle_id.lock().await.clone();
        let _ = crate::garage::append_service_log(
            &active_id,
            crate::garage::VehicleServiceLogEntry {
                timestamp: crate::garage::current_iso_timestamp(),
                mileage: None,
                category: "battery".into(),
                description: "Registered 90Ah AGM battery replacement into DME (0x0219)".into(),
                result: "Success".into(),
            },
        );
        Ok(res)
    }

    /// Queries Condition Based Service (CBS) maintenance intervals and item health.
    pub async fn read_cbs_report(
        &self,
    ) -> Result<crate::profile::CbsReport, crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        conn.read_cbs_report().await
    }

    /// Resets an individual Condition Based Service (CBS) item routine.
    pub async fn reset_cbs_item(
        &self,
        item: crate::profile::CbsItemType,
    ) -> Result<String, crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        let res = conn.reset_cbs_item(item).await?;
        let active_id = self.active_vehicle_id.lock().await.clone();
        let _ = crate::garage::append_service_log(
            &active_id,
            crate::garage::VehicleServiceLogEntry {
                timestamp: crate::garage::current_iso_timestamp(),
                mileage: None,
                category: "cbs_reset".into(),
                description: format!("Reset Condition Based Service item: {item:?}"),
                result: "Success".into(),
            },
        );
        Ok(res)
    }

    /// Performs an all-module ECU network topology scan across all vehicle bus segments.
    pub async fn scan_ecu_topology(
        &self,
    ) -> Result<crate::profile::TopologyScanReport, crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        conn.scan_ecu_topology().await
    }

    /// Starts the automated electric water pump coolant bleeding routine (Routine 0x0211 on DME).
    pub async fn start_water_pump_bleed(&self) -> Result<String, crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        let res = conn.start_water_pump_bleed().await?;
        let active_id = self.active_vehicle_id.lock().await.clone();
        let _ = crate::garage::append_service_log(
            &active_id,
            crate::garage::VehicleServiceLogEntry {
                timestamp: crate::garage::current_iso_timestamp(),
                mileage: None,
                category: "actuator_test".into(),
                description: "Electric water pump coolant bleeding routine initiated (0x0211)"
                    .into(),
                result: "Active".into(),
            },
        );
        Ok(res)
    }

    /// Stops / cancels the electric water pump coolant bleeding routine.
    pub async fn stop_water_pump_bleed(&self) -> Result<String, crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        conn.stop_water_pump_bleed().await
    }

    /// Queries live status and progress of the electric water pump bleeding routine.
    pub async fn get_water_pump_bleed_status(
        &self,
    ) -> Result<crate::profile::WaterPumpBleedStatus, crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        conn.get_water_pump_bleed_status().await
    }

    /// Queries ZF 8HP transmission fluid sump temperature and fill level window assessment.
    pub async fn get_transmission_oil_status(
        &self,
    ) -> Result<crate::profile::TransmissionOilStatus, crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        conn.get_transmission_oil_status().await
    }

    /// Executes ZF 8HP transmission fluid level check routine (Routine 0x0225 on EGS).
    pub async fn start_transmission_fill_routine(&self) -> Result<String, crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        let res = conn.start_transmission_fill_routine().await?;
        let active_id = self.active_vehicle_id.lock().await.clone();
        let _ = crate::garage::append_service_log(
            &active_id,
            crate::garage::VehicleServiceLogEntry {
                timestamp: crate::garage::current_iso_timestamp(),
                mileage: None,
                category: "actuator_test".into(),
                description: "ZF 8HP transmission fluid level check routine initiated (0x0225)"
                    .into(),
                result: "Active".into(),
            },
        );
        Ok(res)
    }

    /// Sets radiator electric cooling fan test duty percent (Routine 0x0215: 0, 15, 50, 90).
    pub async fn set_radiator_fan_duty(
        &self,
        duty_percent: u8,
    ) -> Result<String, crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        conn.set_radiator_fan_duty(duty_percent).await
    }

    /// Sets muffler exhaust flap actuator state (Routine 0x0218: true = open, false = closed).
    pub async fn set_exhaust_flap(&self, open: bool) -> Result<String, crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        conn.set_exhaust_flap(open).await
    }

    /// Queries high-speed live engine datalog frame. If recording is active, buffers frame.
    pub async fn read_engine_datalog(&self) -> Result<EngineDatalogFrame, crate::error::ObdError> {
        let frame = {
            let mut conn = self.connection.lock().await;
            if !conn.is_connected() {
                conn.connect().await?;
            }
            conn.read_engine_datalog().await?
        };

        let mut session = self.datalog_session.lock().await;
        if session.is_recording {
            session.frames.push(frame.clone());
        }
        Ok(frame)
    }

    /// Starts in-memory datalog recording session.
    pub async fn start_datalog_recording(&self) -> Result<String, crate::error::ObdError> {
        let mut session = self.datalog_session.lock().await;
        session.is_recording = true;
        session.start_time = Some(Instant::now());
        session.frames.clear();
        Ok("High-speed datalog recording initiated.".into())
    }

    /// Stops datalog recording, generates CSV representation, and returns summary.
    pub async fn stop_datalog_recording(
        &self,
    ) -> Result<DatalogRecordingSummary, crate::error::ObdError> {
        let mut session = self.datalog_session.lock().await;
        session.is_recording = false;
        let duration = session
            .start_time
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0);
        let sample_count = session.frames.len();

        let mut csv = String::from("timestamp_ms,rpm,speed_kmh,boost_target_hpa,boost_actual_hpa,boost_target_psi,boost_actual_psi,wgdc_pct,rail_target_bar,rail_actual_bar,lpfp_bar,lambda,iat_c,maf_gps,timing_cyl1,knock_cyl1,knock_cyl2,knock_cyl3,knock_cyl4,knock_cyl5,knock_cyl6,misfire_cyl1,misfire_cyl2,misfire_cyl3,misfire_cyl4,misfire_cyl5,misfire_cyl6,vanos_in_tgt,vanos_in_act,vanos_ex_tgt,vanos_ex_act\n");

        for f in &session.frames {
            csv.push_str(&format!(
                "{},{},{},{:.1},{:.1},{:.2},{:.2},{:.1},{:.1},{:.1},{:.1},{:.2},{:.1},{:.1},{:.1},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{},{},{},{},{},{},{:.1},{:.1},{:.1},{:.1}\n",
                f.timestamp_ms,
                f.engine_rpm,
                f.vehicle_speed_kmh,
                f.boost_target_hpa,
                f.boost_actual_hpa,
                f.boost_target_psi,
                f.boost_actual_psi,
                f.wastegate_duty_percent,
                f.rail_pressure_target_bar,
                f.rail_pressure_actual_bar,
                f.low_pressure_fuel_bar,
                f.lambda,
                f.intake_air_temp_c,
                f.mass_air_flow_gps,
                f.ignition_timing_cyl1,
                f.knock_retard[0],
                f.knock_retard[1],
                f.knock_retard[2],
                f.knock_retard[3],
                f.knock_retard[4],
                f.knock_retard[5],
                f.misfire_counts[0],
                f.misfire_counts[1],
                f.misfire_counts[2],
                f.misfire_counts[3],
                f.misfire_counts[4],
                f.misfire_counts[5],
                f.vanos_intake_target_deg,
                f.vanos_intake_actual_deg,
                f.vanos_exhaust_target_deg,
                f.vanos_exhaust_actual_deg,
            ));
        }

        let now_sec = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let file_path = format!("obd2mon_n55_log_{now_sec}.csv");
        let saved_path = if std::fs::write(&file_path, &csv).is_ok() {
            Some(file_path)
        } else {
            None
        };

        let active_id = self.active_vehicle_id.lock().await.clone();
        if let Some(ref path) = saved_path {
            let file_size = std::fs::metadata(path)
                .map(|m| m.len() as usize)
                .unwrap_or(csv.len());
            let _ = crate::garage::append_datalog_entry(
                &active_id,
                crate::garage::VehicleDatalogLogEntry {
                    filename: path.clone(),
                    file_path: std::fs::canonicalize(path)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| path.clone()),
                    timestamp: crate::garage::current_iso_timestamp(),
                    sample_count,
                    duration_seconds: duration,
                    file_size_bytes: file_size,
                },
            );
        }

        Ok(DatalogRecordingSummary {
            sample_count,
            duration_seconds: duration,
            csv_content: csv,
            saved_file_path: saved_path,
        })
    }

    /// Records a diagnostic trouble code scan into the active vehicle profile's history.
    pub async fn record_dtc_scan(&self, dtcs: Vec<crate::profile::DtcRecord>) {
        let active_id = self.active_vehicle_id.lock().await.clone();
        let count = dtcs.len();
        let _ = crate::garage::append_dtc_history(
            &active_id,
            crate::garage::VehicleDtcHistoryEntry {
                timestamp: crate::garage::current_iso_timestamp(),
                mileage: None,
                dtc_count: count,
                dtcs,
            },
        );
    }

    /// Queries datalogger recording state and elapsed metrics.
    pub async fn get_datalog_status(&self) -> Result<DataloggerStatus, crate::error::ObdError> {
        let conn = self.connection.lock().await;
        let supported = match conn.profile() {
            crate::profile::VehicleProfile::BmwF10_535i
            | crate::profile::VehicleProfile::Simulator => true,
            crate::profile::VehicleProfile::GmcAcadia_2010 => false,
        };
        drop(conn);

        let session = self.datalog_session.lock().await;
        let elapsed = session
            .start_time
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(0.0);

        Ok(DataloggerStatus {
            supported,
            is_recording: session.is_recording,
            sample_count: session.frames.len(),
            elapsed_seconds: if session.is_recording { elapsed } else { 0.0 },
        })
    }
}
