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
