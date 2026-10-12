use crate::connection::ObdConnection;
use crate::profile::{DashboardSnapshot, ParameterType, ParameterValue};
use std::sync::Arc;
use tokio::sync::{mpsc, watch, Mutex};
use tokio::time::{sleep, Duration};

/// Asynchronous dashboard engine that decouples vehicle bus I/O from UI rendering.
pub struct VehicleManager {
    connection: Arc<Mutex<Box<dyn ObdConnection>>>,
    snapshot_tx: watch::Sender<DashboardSnapshot>,
    snapshot_rx: watch::Receiver<DashboardSnapshot>,
    event_tx: mpsc::Sender<String>,
}

impl VehicleManager {
    pub fn new(connection: Box<dyn ObdConnection>) -> (Self, mpsc::Receiver<String>) {
        let (snapshot_tx, snapshot_rx) = watch::channel(DashboardSnapshot::default());
        let (event_tx, event_rx) = mpsc::channel(64);

        let manager = Self {
            connection: Arc::new(Mutex::new(connection)),
            snapshot_tx,
            snapshot_rx,
            event_tx,
        };

        (manager, event_rx)
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

        tokio::spawn(async move {
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
        })
    }

    /// Clears vehicle diagnostic trouble codes across the abstraction layer.
    pub async fn clear_dtcs(&self) -> Result<(), crate::error::ObdError> {
        let mut conn = self.connection.lock().await;
        conn.clear_dtcs().await
    }
}
