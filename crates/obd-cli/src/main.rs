use obd_core::{EnetConnection, MockConnection, ObdConnection, VehicleManager, VehicleProfile};
use std::env;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("=====================================================");
    println!("  odb2mon - Cross-Platform Vehicle Diagnostic System  ");
    let version = env!("CARGO_PKG_VERSION");
    println!("  Version: v{version} (Windows x64)");
    println!("=====================================================");

    // Spawn non-blocking background check for newer releases on GitHub
    tokio::spawn(async move {
        if let Ok(info) = obd_core::check_for_updates(env!("CARGO_PKG_VERSION")).await {
            if info.update_available {
                eprintln!(
                    "\n>>> UPDATE AVAILABLE: v{} is available! (Installed: v{}) <<<",
                    info.latest_version, info.current_version
                );
                if let Some(msi) = &info.msi_download_url {
                    eprintln!("    Download MSI: {msi}");
                }
                if let Some(exe) = &info.exe_download_url {
                    eprintln!("    Download EXE: {exe}");
                }
                eprintln!("    Release Page: {}\n", info.html_url);
            }
        }
    });

    let args: Vec<String> = env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("mock");

    let connection: Box<dyn ObdConnection> = match mode {
        "enet" => {
            let host = args
                .get(2)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "169.254.1.1".into());
            let port = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(6801);
            println!("Targeting BMW F10 via ENET DoIP at {host}:{port}...");
            Box::new(EnetConnection::new(host, port, 0x0012))
        }
        "gmc" => {
            println!("Targeting 2010 GMC Acadia via Simulator/BT...");
            Box::new(MockConnection::new(VehicleProfile::GmcAcadia_2010))
        }
        "bmw" => {
            println!("Targeting 2011 BMW 535i xDrive via Simulator...");
            Box::new(MockConnection::new(VehicleProfile::BmwF10_535i))
        }
        _ => {
            println!("Defaulting to Virtual Vehicle Simulator...");
            Box::new(MockConnection::new(VehicleProfile::Simulator))
        }
    };

    let profile = connection.profile();
    println!("Vehicle: {}", profile.name());
    println!("Interface: {}", profile.default_interface_description());
    println!("-----------------------------------------------------");

    let (manager, _event_rx) = VehicleManager::new(connection);
    let rx = manager.subscribe();

    let loop_handle = manager.spawn_polling_loop(Duration::from_millis(150));

    println!("Polling telemetry for 5 seconds...\n");
    for _ in 0..15 {
        tokio::time::sleep(Duration::from_millis(300)).await;
        let snap = rx.borrow().clone();

        let oil_str = if snap.oil_temp_supported {
            snap.oil_temp_c
                .map(|t| format!("{t:.1}°C"))
                .unwrap_or_else(|| "--".into())
        } else {
            "Unsupported (N/A)".into()
        };

        println!(
            "[{}] RPM: {:<5} | Speed: {:<3} km/h | Coolant: {:<5} | Oil: {}",
            snap.connection_status,
            snap.rpm
                .map(|r| r.to_string())
                .unwrap_or_else(|| "--".into()),
            snap.speed_kmh
                .map(|s| s.to_string())
                .unwrap_or_else(|| "--".into()),
            snap.coolant_temp_c
                .map(|t| format!("{t:.1}°C"))
                .unwrap_or_else(|| "--".into()),
            oil_str
        );

        if !snap.active_dtcs.is_empty() {
            println!("  Active Fault Codes:");
            for dtc in &snap.active_dtcs {
                println!(
                    "    * {} - {} [{:?}]",
                    dtc.code, dtc.description, dtc.status
                );
            }
        }
    }

    loop_handle.abort();
    println!("\nPolling stopped cleanly.");
    Ok(())
}
