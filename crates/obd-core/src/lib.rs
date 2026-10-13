pub mod adapters;
pub mod connection;
pub mod dtc_db;
pub mod error;
pub mod garage;
pub mod manager;
pub mod profile;
pub mod protocols;
pub mod updater;

// Public re-exports for easy downstream consumption
pub use adapters::{BluetoothConnection, EnetConnection, MockConnection};
pub use connection::{ConnectionState, ObdConnection};
pub use dtc_db::{lookup_description, lookup_known_dtc};
pub use error::ObdError;
pub use garage::{
    append_datalog_entry, append_dtc_history, append_service_log, current_iso_timestamp,
    evaluate_compatibility, get_garage_directory, load_garage, load_vehicle_history, save_garage,
    save_vehicle_history, Garage, HardwareCompatibilityInfo, VehicleDatalogLogEntry,
    VehicleDtcHistoryEntry, VehicleHistory, VehicleRecord, VehicleServiceLogEntry,
};
pub use manager::VehicleManager;
pub use profile::{
    BatteryStatus, BmwEcuDefinition, BusType, CbsItem, CbsItemType, CbsReport, CbsStatus,
    DashboardSnapshot, DatalogRecordingSummary, DataloggerStatus, DtcRecord, DtcStatus, EcuHealth,
    EcuNode, EngineDatalogFrame, ExhaustFlapStatus, ParameterType, ParameterValue,
    RadiatorFanStatus, TopologyScanReport, TransmissionOilStatus, VehicleProfile,
    WaterPumpBleedStatus, BMW_F10_ECUS,
};
pub use updater::{check_for_updates, parse_clean_semver, UpdateInfo};
