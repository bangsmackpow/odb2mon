pub mod error;
pub mod profile;
pub mod connection;
pub mod manager;
pub mod protocols;
pub mod adapters;

// Public re-exports for easy downstream consumption
pub use error::ObdError;
pub use profile::{
    DashboardSnapshot, DtcRecord, DtcStatus, ParameterType, ParameterValue, VehicleProfile,
};
pub use connection::{ConnectionState, ObdConnection};
pub use manager::VehicleManager;
pub use adapters::{BluetoothConnection, EnetConnection, MockConnection};
