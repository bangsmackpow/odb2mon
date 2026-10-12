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
    let oil_val = conn.read_parameter(ParameterType::OilTemperature).await.unwrap();
    if let ParameterValue::TemperatureCelsius(temp) = oil_val {
        assert!(temp >= 15.0 && temp <= 110.0, "Unexpected oil temp: {temp}");
    } else {
        panic!("Expected TemperatureCelsius variant for BMW, got {:?}", oil_val);
    }

    // Test BMW DTC reading
    let dtcs = conn.read_dtcs().await.unwrap();
    assert!(!dtcs.is_empty(), "Expected BMW mock to return trouble codes");
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

    let coolant = conn.read_parameter(ParameterType::CoolantTemperature).await.unwrap();
    assert!(matches!(coolant, ParameterValue::TemperatureCelsius(_)));

    // CRITICAL: Verify GMC Oil Temp returns Unsupported gracefully without failing
    let oil = conn.read_parameter(ParameterType::OilTemperature).await.unwrap();
    assert_eq!(oil, ParameterValue::Unsupported, "GMC Acadia must report Unsupported for oil temp");

    // Verify GM DTCs
    let dtcs = conn.read_dtcs().await.unwrap();
    assert!(dtcs.iter().any(|d| d.code == "P0008"));
}

#[tokio::test]
async fn test_vehicle_manager_decoupled_snapshots() {
    let conn = Box::new(MockConnection::new(VehicleProfile::Simulator));
    let (manager, _event_rx) = VehicleManager::new(conn);
    let mut subscriber = manager.subscribe();

    let join_handle = manager.spawn_polling_loop(Duration::from_millis(20));

    // Wait a brief moment to receive populated snapshot
    tokio::time::sleep(Duration::from_millis(80)).await;

    let snapshot = subscriber.borrow().clone();
    assert_eq!(snapshot.connection_status, "Connected");
    assert!(snapshot.rpm.is_some());
    assert!(snapshot.coolant_temp_c.is_some());

    join_handle.abort();
}
