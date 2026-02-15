pub mod bridge;
pub mod config;
pub mod database;
pub mod matrix;
pub mod qq;

pub const NAME: &str = "matrix-bridge-qq";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
