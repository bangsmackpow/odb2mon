use obd_core::{
    MockConnection, ObdConnection, ParameterType, ParameterValue, VehicleManager, VehicleProfile,
};
use std::time::Duration;

#[tokio::test]
async fn test_bmw_f10_mock_simulation() {
    let mut conn = MockConnection::new(VehicleProfile::BmwF10_535i);
    assert!(!conn.is_connected());

    conn.connect().await.expect("Failed to connect mock");
    assert!(conn.is_connected());

    // Test RPM reading
    let rpm_val = conn.read_parameter(ParameterType::EngineRpm).await.unwrap();
    if let ParameterValue::Rpm(rpm) = rpm_val {
        assert!(rpm >= 700, "Expected idle RPM >= 700, got {rpm}");
    } else {
        panic!("Expected Rpm variant, got {:?}", rpm_val);
    }

    // Test BMW Oil Temperature reading (Supported)
    let oil_val = conn
        .read_parameter(ParameterType::OilTemperature)
        .await
        .unwrap();
    if let ParameterValue::TemperatureCelsius(temp) = oil_val {
        assert!(
            (15.0..=110.0).contains(&temp),
            "Unexpected oil temp: {temp}"
        );
    } else {
        panic!(
            "Expected TemperatureCelsius variant for BMW, got {:?}",
            oil_val
        );
    }

    // Test BMW DTC reading
    let dtcs = conn.read_dtcs().await.unwrap();
    assert!(
        !dtcs.is_empty(),
        "Expected BMW mock to return trouble codes"
    );
    assert_eq!(dtcs[0].code, "120408");

    // Test DTC clear
    conn.clear_dtcs().await.unwrap();
    let dtcs_after = conn.read_dtcs().await.unwrap();
    assert!(dtcs_after.is_empty(), "Expected DTCs to be cleared");

    conn.disconnect().await.expect("Failed to disconnect mock");
    assert!(!conn.is_connected());
}

#[tokio::test]
async fn test_gmc_acadia_unsupported_oil_temp_graceful() {
    let mut conn = MockConnection::new(VehicleProfile::GmcAcadia_2010);
    conn.connect().await.unwrap();

    // Verify RPM and Coolant work
    let rpm = conn.read_parameter(ParameterType::EngineRpm).await.unwrap();
    assert!(matches!(rpm, ParameterValue::Rpm(_)));

    let coolant = conn
        .read_parameter(ParameterType::CoolantTemperature)
        .await
        .unwrap();
    assert!(matches!(coolant, ParameterValue::TemperatureCelsius(_)));

    // CRITICAL: Verify GMC Oil Temp returns Unsupported gracefully without failing
    let oil = conn
        .read_parameter(ParameterType::OilTemperature)
        .await
        .unwrap();
    assert_eq!(
        oil,
        ParameterValue::Unsupported,
        "GMC Acadia must report Unsupported for oil temp"
    );

    // Verify GM DTCs
    let dtcs = conn.read_dtcs().await.unwrap();
    assert!(dtcs.iter().any(|d| d.code == "P0008"));
}

#[tokio::test]
async fn test_vehicle_manager_decoupled_snapshots() {
    let conn = Box::new(MockConnection::new(VehicleProfile::Simulator));
    let (manager, _event_rx) = VehicleManager::new(conn);
    let subscriber = manager.subscribe();

    let join_handle = manager.spawn_polling_loop(Duration::from_millis(20));

    // Wait a brief moment to receive populated snapshot
    tokio::time::sleep(Duration::from_millis(80)).await;

    let snapshot = subscriber.borrow().clone();
    assert_eq!(snapshot.connection_status, "Connected");
    assert!(snapshot.rpm.is_some());
    assert!(snapshot.coolant_temp_c.is_some());

    join_handle.abort();
}

#[tokio::test]
async fn test_bmw_battery_registration_and_telemetry() {
    let mut conn = MockConnection::new(VehicleProfile::BmwF10_535i);
    conn.connect().await.unwrap();

    let battery = conn.read_battery_status().await.unwrap();
    assert!(battery.supported);
    assert_eq!(battery.state_of_charge_percent, 86.5);
    assert_eq!(battery.capacity_ah, Some(90));
    assert_eq!(battery.last_replacement_km, vec![142300, 78100]);

    // Register a new battery
    let reg_msg = conn.register_battery_replacement().await.unwrap();
    assert!(reg_msg.contains("168450 km"));

    // Verify history and SoC were updated
    let battery_updated = conn.read_battery_status().await.unwrap();
    assert_eq!(battery_updated.state_of_charge_percent, 100.0);
    assert_eq!(battery_updated.last_replacement_km[0], 168450);
    assert_eq!(battery_updated.last_replacement_km[1], 142300);
}

#[tokio::test]
async fn test_gmc_battery_registration_unsupported() {
    let mut conn = MockConnection::new(VehicleProfile::GmcAcadia_2010);
    conn.connect().await.unwrap();

    let err = conn.read_battery_status().await.unwrap_err();
    assert!(matches!(err, obd_core::ObdError::Unsupported(_)));

    let reg_err = conn.register_battery_replacement().await.unwrap_err();
    assert!(matches!(reg_err, obd_core::ObdError::Unsupported(_)));
}

#[tokio::test]
async fn test_bmw_cbs_monitor_and_reset() {
    let mut conn = MockConnection::new(VehicleProfile::BmwF10_535i);
    conn.connect().await.unwrap();

    let report = conn.read_cbs_report().await.unwrap();
    assert!(report.supported);
    assert_eq!(report.items.len(), 5);

    // Verify initial rear brakes in warning state (15%)
    let rear_brakes = report
        .items
        .iter()
        .find(|i| i.item_type == obd_core::CbsItemType::RearBrakes)
        .unwrap();
    assert_eq!(rear_brakes.status, obd_core::CbsStatus::Warning);
    assert_eq!(rear_brakes.remaining_percent, 15.0);

    // Reset rear brakes
    let reset_msg = conn
        .reset_cbs_item(obd_core::CbsItemType::RearBrakes)
        .await
        .unwrap();
    assert!(reset_msg.contains("Rear Brake Pads"));

    // Verify rear brakes are now 100% and OK
    let updated_report = conn.read_cbs_report().await.unwrap();
    let updated_rear = updated_report
        .items
        .iter()
        .find(|i| i.item_type == obd_core::CbsItemType::RearBrakes)
        .unwrap();
    assert_eq!(updated_rear.status, obd_core::CbsStatus::Ok);
    assert_eq!(updated_rear.remaining_percent, 100.0);
    assert_eq!(updated_rear.remaining_km, Some(50000));
    assert_eq!(updated_rear.reset_count, 3);
}

#[tokio::test]
async fn test_gmc_cbs_unsupported() {
    let mut conn = MockConnection::new(VehicleProfile::GmcAcadia_2010);
    conn.connect().await.unwrap();

    let err = conn.read_cbs_report().await.unwrap_err();
    assert!(matches!(err, obd_core::ObdError::Unsupported(_)));

    let reset_err = conn
        .reset_cbs_item(obd_core::CbsItemType::EngineOil)
        .await
        .unwrap_err();
    assert!(matches!(reset_err, obd_core::ObdError::Unsupported(_)));
}

#[tokio::test]
async fn test_bmw_topology_scan() {
    let mut conn = MockConnection::new(VehicleProfile::BmwF10_535i);
    conn.connect().await.unwrap();

    let scan = conn.scan_ecu_topology().await.unwrap();
    assert!(scan.supported);
    assert_eq!(scan.total_ecus_scanned, 16);
    assert_eq!(scan.online_count, 16);
    assert_eq!(scan.fault_count, 2);

    let dme = scan
        .nodes
        .iter()
        .find(|n| n.logical_address == 0x0012)
        .unwrap();
    assert_eq!(dme.short_name, "DME");
    assert_eq!(dme.health, obd_core::EcuHealth::FaultStored);
    assert_eq!(dme.dtc_count, 2);
    assert_eq!(dme.dtcs.len(), 2);

    let egs = scan
        .nodes
        .iter()
        .find(|n| n.logical_address == 0x0018)
        .unwrap();
    assert_eq!(egs.short_name, "EGS");
    assert_eq!(egs.health, obd_core::EcuHealth::Normal);
    assert_eq!(egs.dtc_count, 0);

    // Clear DTCs and verify topology scan reflects clean state
    conn.clear_dtcs().await.unwrap();
    let clean_scan = conn.scan_ecu_topology().await.unwrap();
    assert_eq!(clean_scan.fault_count, 0);
    let dme_clean = clean_scan
        .nodes
        .iter()
        .find(|n| n.logical_address == 0x0012)
        .unwrap();
    assert_eq!(dme_clean.health, obd_core::EcuHealth::Normal);
    assert_eq!(dme_clean.dtc_count, 0);
}

#[tokio::test]
async fn test_gmc_topology_scan_unsupported() {
    let mut conn = MockConnection::new(VehicleProfile::GmcAcadia_2010);
    conn.connect().await.unwrap();

    let err = conn.scan_ecu_topology().await.unwrap_err();
    assert!(matches!(err, obd_core::ObdError::Unsupported(_)));
}

#[tokio::test]
async fn test_bmw_water_pump_bleed_cycle() {
    let mut conn = MockConnection::new(VehicleProfile::BmwF10_535i);
    conn.connect().await.unwrap();

    // Verify initial idle state
    let initial_status = conn.get_water_pump_bleed_status().await.unwrap();
    assert!(!initial_status.is_running);

    // Start bleed routine
    let start_msg = conn.start_water_pump_bleed().await.unwrap();
    assert!(start_msg.contains("0x0211"));

    // Query running status
    let running_status = conn.get_water_pump_bleed_status().await.unwrap();
    assert!(running_status.is_running);
    assert!(running_status.pump_speed_percent > 0);

    // Stop routine
    let stop_msg = conn.stop_water_pump_bleed().await.unwrap();
    assert!(stop_msg.contains("stopped"));

    let stopped_status = conn.get_water_pump_bleed_status().await.unwrap();
    assert!(!stopped_status.is_running);
}

#[tokio::test]
async fn test_bmw_transmission_temp_and_fill() {
    let mut conn = MockConnection::new(VehicleProfile::BmwF10_535i);
    conn.connect().await.unwrap();

    let oil_status = conn.get_transmission_oil_status().await.unwrap();
    assert!(oil_status.supported);
    assert!(oil_status.temperature_c >= 40.0 && oil_status.temperature_c <= 50.0);
    assert!(oil_status.is_within_fill_window);

    let fill_msg = conn.start_transmission_fill_routine().await.unwrap();
    assert!(fill_msg.contains("0x0225"));
}

#[tokio::test]
async fn test_bmw_fan_and_exhaust_flap_actuators() {
    let mut conn = MockConnection::new(VehicleProfile::BmwF10_535i);
    conn.connect().await.unwrap();

    let fan_msg = conn.set_radiator_fan_duty(50).await.unwrap();
    assert!(fan_msg.contains("50%"));

    let fan_off_msg = conn.set_radiator_fan_duty(0).await.unwrap();
    assert!(fan_off_msg.contains("returned to automatic"));

    let flap_open_msg = conn.set_exhaust_flap(true).await.unwrap();
    assert!(flap_open_msg.contains("OPEN"));

    let flap_close_msg = conn.set_exhaust_flap(false).await.unwrap();
    assert!(flap_close_msg.contains("CLOSED"));
}

#[tokio::test]
async fn test_gmc_actuators_unsupported() {
    let mut conn = MockConnection::new(VehicleProfile::GmcAcadia_2010);
    conn.connect().await.unwrap();

    let bleed_err = conn.start_water_pump_bleed().await.unwrap_err();
    assert!(matches!(bleed_err, obd_core::ObdError::Unsupported(_)));

    let trans_err = conn.get_transmission_oil_status().await.unwrap_err();
    assert!(matches!(trans_err, obd_core::ObdError::Unsupported(_)));

    let fan_err = conn.set_radiator_fan_duty(50).await.unwrap_err();
    assert!(matches!(fan_err, obd_core::ObdError::Unsupported(_)));

    let flap_err = conn.set_exhaust_flap(true).await.unwrap_err();
    assert!(matches!(flap_err, obd_core::ObdError::Unsupported(_)));
}

#[tokio::test]
async fn test_bmw_n55_datalog_and_recording() {
    let conn = Box::new(MockConnection::new(VehicleProfile::BmwF10_535i));
    let (manager, _event_rx) = VehicleManager::new(conn);

    // Initial status check
    let status = manager.get_datalog_status().await.unwrap();
    assert!(status.supported);
    assert!(!status.is_recording);
    assert_eq!(status.sample_count, 0);

    // Start recording session
    let start_msg = manager.start_datalog_recording().await.unwrap();
    assert!(start_msg.contains("recording initiated"));

    let status_active = manager.get_datalog_status().await.unwrap();
    assert!(status_active.is_recording);

    // Read 5 datalog frames
    for _ in 0..5 {
        let frame = manager.read_engine_datalog().await.unwrap();
        assert!(frame.engine_rpm > 500);
        assert!(frame.boost_target_hpa >= 1000.0);
        assert!(frame.rail_pressure_target_bar >= 40.0);
        assert!(frame.low_pressure_fuel_bar > 4.0);
        assert!(frame.vanos_intake_target_deg > 50.0);
        assert_eq!(frame.misfire_counts.len(), 6);
        assert_eq!(frame.knock_retard.len(), 6);
    }

    // Stop recording and inspect CSV summary
    let summary = manager.stop_datalog_recording().await.unwrap();
    assert_eq!(summary.sample_count, 5);
    assert!(summary
        .csv_content
        .contains("timestamp_ms,rpm,speed_kmh,boost_target_hpa"));
    assert!(summary.csv_content.lines().count() >= 6); // header + 5 lines

    // Status should be idle now
    let status_idle = manager.get_datalog_status().await.unwrap();
    assert!(!status_idle.is_recording);
    assert_eq!(status_idle.sample_count, 5);
}

#[tokio::test]
async fn test_gmc_datalog_unsupported() {
    let conn = Box::new(MockConnection::new(VehicleProfile::GmcAcadia_2010));
    let (manager, _event_rx) = VehicleManager::new(conn);

    let status = manager.get_datalog_status().await.unwrap();
    assert!(!status.supported);

    let datalog_err = manager.read_engine_datalog().await.unwrap_err();
    assert!(matches!(datalog_err, obd_core::ObdError::Unsupported(_)));
}
