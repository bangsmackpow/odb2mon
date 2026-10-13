use obd_core::{
    append_service_log, check_for_updates, current_iso_timestamp, evaluate_compatibility,
    load_garage, load_vehicle_history, save_garage, BatteryStatus, BluetoothConnection,
    CbsItemType, CbsReport, DashboardSnapshot, DatalogRecordingSummary, DataloggerStatus,
    DtcRecord, EnetConnection, EngineDatalogFrame, Garage, HardwareCompatibilityInfo,
    MockConnection, ObdConnection, TopologyScanReport, TransmissionOilStatus, UpdateInfo,
    VehicleHistory, VehicleManager, VehicleProfile, VehicleRecord, VehicleServiceLogEntry,
    WaterPumpBleedStatus,
};
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Manager, State};
use tokio::sync::Mutex;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DetectedVinResult {
    pub vin: String,
    pub matched_vehicle_id: Option<String>,
    pub message: String,
}

pub struct AppState {
    manager: Arc<Mutex<VehicleManager>>,
    active_profile: Arc<Mutex<VehicleProfile>>,
    garage: Arc<Mutex<Garage>>,
}

fn create_connection_for_vehicle(veh: &VehicleRecord) -> (VehicleProfile, Box<dyn ObdConnection>) {
    let profile = if veh.make.to_lowercase().contains("bmw") {
        VehicleProfile::BmwF10_535i
    } else if veh.make.to_lowercase().contains("gmc") || veh.make.to_lowercase().contains("gm") {
        VehicleProfile::GmcAcadia_2010
    } else {
        VehicleProfile::Simulator
    };

    let connection: Box<dyn ObdConnection> = match veh.interface_type.as_str() {
        "enet" => {
            let host = veh
                .interface_host
                .clone()
                .unwrap_or_else(|| "169.254.1.1".into());
            let port = veh.interface_port.unwrap_or(6801);
            Box::new(EnetConnection::new(host, port, 0x0012))
        }
        "bluetooth" | "usb" => {
            let port = veh.serial_port.clone().unwrap_or_else(|| "COM3".into());
            let baud = veh.serial_baud.unwrap_or(38400);
            Box::new(BluetoothConnection::new(port, baud))
        }
        _ => Box::new(MockConnection::new(profile)),
    };

    (profile, connection)
}

#[tauri::command]
async fn get_snapshot(state: State<'_, AppState>) -> Result<DashboardSnapshot, String> {
    let mgr = state.manager.lock().await;
    let rx = mgr.subscribe();
    let snap = rx.borrow().clone();
    Ok(snap)
}

#[tauri::command]
async fn get_garage(state: State<'_, AppState>) -> Result<Garage, String> {
    let garage = state.garage.lock().await;
    Ok(garage.clone())
}

#[tauri::command]
async fn get_vehicle_compatibility(
    make: String,
    model: String,
    year: u16,
    interface_type: String,
) -> Result<HardwareCompatibilityInfo, String> {
    Ok(evaluate_compatibility(&make, &model, year, &interface_type))
}

#[tauri::command]
async fn get_vehicle_history(vehicle_id: String) -> Result<VehicleHistory, String> {
    Ok(load_vehicle_history(&vehicle_id))
}

#[tauri::command]
async fn add_vehicle_service_note(
    vehicle_id: String,
    category: String,
    note: String,
    mileage: Option<u32>,
) -> Result<VehicleHistory, String> {
    append_service_log(
        &vehicle_id,
        VehicleServiceLogEntry {
            timestamp: current_iso_timestamp(),
            mileage,
            category,
            description: note,
            result: "Manual Log".into(),
        },
    )
    .map_err(|e| format!("Failed to record service note: {e}"))?;

    Ok(load_vehicle_history(&vehicle_id))
}

#[tauri::command]
async fn set_active_vehicle(
    vehicle_id: String,
    state: State<'_, AppState>,
) -> Result<Garage, String> {
    let mut garage_guard = state.garage.lock().await;
    let found = garage_guard
        .vehicles
        .iter()
        .find(|v| v.id == vehicle_id)
        .cloned();
    let Some(target_veh) = found else {
        return Err(format!("Vehicle not found with ID: {vehicle_id}"));
    };

    garage_guard.active_vehicle_id = vehicle_id.clone();
    let _ = save_garage(&garage_guard);

    let (profile, connection) = create_connection_for_vehicle(&target_veh);
    let (new_manager, _rx) = VehicleManager::new_with_vehicle(connection, vehicle_id.clone());
    new_manager.spawn_polling_loop(Duration::from_millis(100));

    let mut profile_guard = state.active_profile.lock().await;
    *profile_guard = profile;

    let mut mgr_guard = state.manager.lock().await;
    *mgr_guard = new_manager;

    Ok(garage_guard.clone())
}

#[tauri::command]
async fn save_vehicle_profile(
    mut vehicle: VehicleRecord,
    set_as_active: bool,
    state: State<'_, AppState>,
) -> Result<Garage, String> {
    let mut garage_guard = state.garage.lock().await;
    if vehicle.id.trim().is_empty() {
        let ts = chrono::Utc::now().timestamp_millis();
        vehicle.id = format!("veh_{ts}");
    }
    if vehicle.created_at.trim().is_empty() {
        vehicle.created_at = current_iso_timestamp();
    }

    if let Some(pos) = garage_guard
        .vehicles
        .iter()
        .position(|v| v.id == vehicle.id)
    {
        garage_guard.vehicles[pos] = vehicle.clone();
    } else {
        garage_guard.vehicles.push(vehicle.clone());
    }

    if set_as_active {
        garage_guard.active_vehicle_id = vehicle.id.clone();
    }

    save_garage(&garage_guard).map_err(|e| format!("Failed to save garage: {e}"))?;

    if set_as_active || garage_guard.active_vehicle_id == vehicle.id {
        let (profile, connection) = create_connection_for_vehicle(&vehicle);
        let (new_manager, _rx) = VehicleManager::new_with_vehicle(connection, vehicle.id.clone());
        new_manager.spawn_polling_loop(Duration::from_millis(100));

        let mut profile_guard = state.active_profile.lock().await;
        *profile_guard = profile;

        let mut mgr_guard = state.manager.lock().await;
        *mgr_guard = new_manager;
    }

    Ok(garage_guard.clone())
}

#[tauri::command]
async fn delete_vehicle_profile(
    vehicle_id: String,
    state: State<'_, AppState>,
) -> Result<Garage, String> {
    let mut garage_guard = state.garage.lock().await;
    garage_guard.vehicles.retain(|v| v.id != vehicle_id);
    if garage_guard.vehicles.is_empty() {
        let default_garage = Garage::default();
        *garage_guard = default_garage;
    } else if garage_guard.active_vehicle_id == vehicle_id {
        garage_guard.active_vehicle_id = garage_guard.vehicles[0].id.clone();
    }

    save_garage(&garage_guard).map_err(|e| format!("Failed to save garage: {e}"))?;

    let active_id = garage_guard.active_vehicle_id.clone();
    if let Some(target_veh) = garage_guard
        .vehicles
        .iter()
        .find(|v| v.id == active_id)
        .cloned()
    {
        let (profile, connection) = create_connection_for_vehicle(&target_veh);
        let (new_manager, _rx) = VehicleManager::new_with_vehicle(connection, active_id);
        new_manager.spawn_polling_loop(Duration::from_millis(100));

        let mut profile_guard = state.active_profile.lock().await;
        *profile_guard = profile;

        let mut mgr_guard = state.manager.lock().await;
        *mgr_guard = new_manager;
    }

    Ok(garage_guard.clone())
}

#[tauri::command]
async fn detect_vehicle_vin(state: State<'_, AppState>) -> Result<DetectedVinResult, String> {
    let mgr = state.manager.lock().await;
    let vin = mgr
        .read_vin()
        .await
        .map_err(|e| format!("Failed to read VIN: {e}"))?;
    drop(mgr);

    let garage = state.garage.lock().await;
    let matched_id = garage
        .vehicles
        .iter()
        .find(|v| {
            v.vin
                .as_ref()
                .map(|v_str| v_str.trim().eq_ignore_ascii_case(vin.trim()))
                .unwrap_or(false)
        })
        .map(|v| v.id.clone());

    let message = if let Some(ref mid) = matched_id {
        format!("VIN {vin} matched existing vehicle record ({mid})")
    } else {
        format!("VIN {vin} detected from vehicle bus")
    };

    Ok(DetectedVinResult {
        vin,
        matched_vehicle_id: matched_id,
        message,
    })
}

#[tauri::command]
async fn switch_profile(
    profile_type: String,
    enet_host: Option<String>,
    enet_port: Option<u16>,
    bt_port: Option<String>,
    bt_baud: Option<u32>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let (new_profile, connection): (VehicleProfile, Box<dyn ObdConnection>) =
        match profile_type.to_lowercase().as_str() {
            "bmw" => {
                let host = enet_host.unwrap_or_else(|| "169.254.1.1".into());
                let port = enet_port.unwrap_or(6801);
                (
                    VehicleProfile::BmwF10_535i,
                    Box::new(EnetConnection::new(host, port, 0x0012)),
                )
            }
            "gmc" => {
                let port = bt_port.unwrap_or_else(|| "COM3".into());
                let baud = bt_baud.unwrap_or(38400);
                (
                    VehicleProfile::GmcAcadia_2010,
                    Box::new(BluetoothConnection::new(port, baud)),
                )
            }
            "simulator_bmw" => (
                VehicleProfile::BmwF10_535i,
                Box::new(MockConnection::new(VehicleProfile::BmwF10_535i)),
            ),
            "simulator_gmc" => (
                VehicleProfile::GmcAcadia_2010,
                Box::new(MockConnection::new(VehicleProfile::GmcAcadia_2010)),
            ),
            _ => (
                VehicleProfile::Simulator,
                Box::new(MockConnection::new(VehicleProfile::Simulator)),
            ),
        };

    let mut current_profile = state.active_profile.lock().await;
    *current_profile = new_profile;

    let (new_manager, _rx) = VehicleManager::new(connection);
    let _handle = new_manager.spawn_polling_loop(Duration::from_millis(100));

    let mut mgr = state.manager.lock().await;
    *mgr = new_manager;

    Ok(format!("Switched to {}", new_profile.name()))
}

#[tauri::command]
async fn read_fault_codes(state: State<'_, AppState>) -> Result<Vec<DtcRecord>, String> {
    let mgr = state.manager.lock().await;
    let rx = mgr.subscribe();
    let snap = rx.borrow().clone();
    if !snap.active_dtcs.is_empty() {
        mgr.record_dtc_scan(snap.active_dtcs.clone()).await;
    }
    Ok(snap.active_dtcs)
}

#[tauri::command]
async fn clear_fault_codes(state: State<'_, AppState>) -> Result<String, String> {
    let mgr = state.manager.lock().await;
    mgr.clear_dtcs()
        .await
        .map_err(|e| format!("Failed to clear fault codes: {e}"))?;
    Ok("Fault codes successfully cleared".into())
}

#[tauri::command]
async fn check_update() -> Result<UpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION");
    check_for_updates(current_version)
        .await
        .map_err(|e| format!("Update check failed: {e}"))
}

#[tauri::command]
async fn get_battery_status(state: State<'_, AppState>) -> Result<BatteryStatus, String> {
    let mgr = state.manager.lock().await;
    mgr.read_battery_status().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn register_battery(state: State<'_, AppState>) -> Result<String, String> {
    let mgr = state.manager.lock().await;
    mgr.register_battery_replacement()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_cbs_report(state: State<'_, AppState>) -> Result<CbsReport, String> {
    let mgr = state.manager.lock().await;
    mgr.read_cbs_report().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn reset_cbs(item_type: String, state: State<'_, AppState>) -> Result<String, String> {
    let parsed_type: CbsItemType = item_type
        .parse()
        .map_err(|e: String| format!("Invalid CBS item: {e}"))?;
    let mgr = state.manager.lock().await;
    mgr.reset_cbs_item(parsed_type)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn scan_topology(state: State<'_, AppState>) -> Result<TopologyScanReport, String> {
    let mgr = state.manager.lock().await;
    mgr.scan_ecu_topology().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_water_pump_bleed(state: State<'_, AppState>) -> Result<String, String> {
    let mgr = state.manager.lock().await;
    mgr.start_water_pump_bleed()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_water_pump_bleed(state: State<'_, AppState>) -> Result<String, String> {
    let mgr = state.manager.lock().await;
    mgr.stop_water_pump_bleed().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_water_pump_bleed_status(
    state: State<'_, AppState>,
) -> Result<WaterPumpBleedStatus, String> {
    let mgr = state.manager.lock().await;
    mgr.get_water_pump_bleed_status()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_transmission_oil_status(
    state: State<'_, AppState>,
) -> Result<TransmissionOilStatus, String> {
    let mgr = state.manager.lock().await;
    mgr.get_transmission_oil_status()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_transmission_fill_routine(state: State<'_, AppState>) -> Result<String, String> {
    let mgr = state.manager.lock().await;
    mgr.start_transmission_fill_routine()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_radiator_fan_duty(
    duty_percent: u8,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let mgr = state.manager.lock().await;
    mgr.set_radiator_fan_duty(duty_percent)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_exhaust_flap(open: bool, state: State<'_, AppState>) -> Result<String, String> {
    let mgr = state.manager.lock().await;
    mgr.set_exhaust_flap(open).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_engine_datalog(state: State<'_, AppState>) -> Result<EngineDatalogFrame, String> {
    let mgr = state.manager.lock().await;
    mgr.read_engine_datalog().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_datalog_recording(state: State<'_, AppState>) -> Result<String, String> {
    let mgr = state.manager.lock().await;
    mgr.start_datalog_recording()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_datalog_recording(
    state: State<'_, AppState>,
) -> Result<DatalogRecordingSummary, String> {
    let mgr = state.manager.lock().await;
    mgr.stop_datalog_recording()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_datalog_status(state: State<'_, AppState>) -> Result<DataloggerStatus, String> {
    let mgr = state.manager.lock().await;
    mgr.get_datalog_status().await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_garage = load_garage();
    let active_id = initial_garage.active_vehicle_id.clone();
    let initial_veh = initial_garage
        .vehicles
        .iter()
        .find(|v| v.id == active_id)
        .cloned()
        .unwrap_or_else(|| {
            initial_garage
                .vehicles
                .first()
                .cloned()
                .unwrap_or_else(|| VehicleRecord {
                    id: "veh_bmw_f10_535i".into(),
                    nickname: "2011 BMW 535i xDrive".into(),
                    make: "BMW".into(),
                    model: "535i xDrive (F10)".into(),
                    year: 2011,
                    vin: Some("WBAFR7C58BC123456".into()),
                    mileage: Some(142500),
                    mileage_unit: "km".into(),
                    engine: Some("3.0L Turbo Inline-6 (N55B30M0)".into()),
                    interface_type: "enet".into(),
                    interface_host: Some("169.254.1.1".into()),
                    interface_port: Some(6801),
                    serial_port: None,
                    serial_baud: None,
                    notes: None,
                    created_at: current_iso_timestamp(),
                    last_connected_at: None,
                    active_dtc_count: 2,
                })
        });

    let (initial_profile, initial_conn) = create_connection_for_vehicle(&initial_veh);
    let (initial_manager, _event_rx) =
        VehicleManager::new_with_vehicle(initial_conn, initial_veh.id.clone());
    initial_manager.spawn_polling_loop(Duration::from_millis(100));

    let app_state = AppState {
        manager: Arc::new(Mutex::new(initial_manager)),
        active_profile: Arc::new(Mutex::new(initial_profile)),
        garage: Arc::new(Mutex::new(initial_garage)),
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            switch_profile,
            read_fault_codes,
            clear_fault_codes,
            check_update,
            get_battery_status,
            register_battery,
            get_cbs_report,
            reset_cbs,
            scan_topology,
            start_water_pump_bleed,
            stop_water_pump_bleed,
            get_water_pump_bleed_status,
            get_transmission_oil_status,
            start_transmission_fill_routine,
            set_radiator_fan_duty,
            set_exhaust_flap,
            get_engine_datalog,
            start_datalog_recording,
            stop_datalog_recording,
            get_datalog_status,
            get_garage,
            get_vehicle_compatibility,
            get_vehicle_history,
            add_vehicle_service_note,
            set_active_vehicle,
            save_vehicle_profile,
            delete_vehicle_profile,
            detect_vehicle_vin
        ])
        .setup(|app| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }

            let app_handle = app.handle().clone();

            // Background telemetry stream at 10 Hz pushed to frontend webview
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_millis(100));
                loop {
                    interval.tick().await;
                    if let Some(state) = app_handle.try_state::<AppState>() {
                        let mgr = state.manager.lock().await;
                        let rx = mgr.subscribe();
                        let snapshot = rx.borrow().clone();
                        drop(mgr);

                        let _ = app_handle.emit("telemetry-update", snapshot);
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
