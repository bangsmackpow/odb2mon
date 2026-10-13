use obd_core::{
    check_for_updates, BatteryStatus, BluetoothConnection, DashboardSnapshot, DtcRecord,
    EnetConnection, MockConnection, ObdConnection, UpdateInfo, VehicleManager, VehicleProfile,
};
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Manager, State};
use tokio::sync::Mutex;

pub struct AppState {
    manager: Arc<Mutex<VehicleManager>>,
    active_profile: Arc<Mutex<VehicleProfile>>,
}

#[tauri::command]
async fn get_snapshot(state: State<'_, AppState>) -> Result<DashboardSnapshot, String> {
    let mgr = state.manager.lock().await;
    let rx = mgr.subscribe();
    let snap = rx.borrow().clone();
    Ok(snap)
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Default initial connection to realistic simulator
    let initial_conn = Box::new(MockConnection::new(VehicleProfile::BmwF10_535i));
    let (initial_manager, _event_rx) = VehicleManager::new(initial_conn);
    initial_manager.spawn_polling_loop(Duration::from_millis(100));

    let app_state = AppState {
        manager: Arc::new(Mutex::new(initial_manager)),
        active_profile: Arc::new(Mutex::new(VehicleProfile::BmwF10_535i)),
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
            register_battery
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
