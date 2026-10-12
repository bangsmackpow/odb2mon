pub mod adapters;
pub mod connection;
pub mod dtc_db;
pub mod error;
pub mod manager;
pub mod profile;
pub mod protocols;
pub mod updater;

// Public re-exports for easy downstream consumption
pub use adapters::{BluetoothConnection, EnetConnection, MockConnection};
pub use connection::{ConnectionState, ObdConnection};
pub use dtc_db::{lookup_description, lookup_known_dtc};
pub use error::ObdError;
pub use manager::VehicleManager;
pub use profile::{
    DashboardSnapshot, DtcRecord, DtcStatus, ParameterType, ParameterValue, VehicleProfile,
};
pub use updater::{check_for_updates, parse_clean_semver, UpdateInfo};

