pub mod enet;
pub mod bluetooth;
pub mod mock;

pub use enet::EnetConnection;
pub use bluetooth::BluetoothConnection;
pub use mock::MockConnection;
