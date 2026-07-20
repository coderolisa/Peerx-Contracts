// Library exports for testing and external usage

pub mod commands;
pub mod config;
pub mod error;
pub mod health;
pub mod output;

// Re-export commonly used types
pub use error::{CliError, Result};
pub use config::Config;
pub use health::{HealthChecker, HealthReport, HealthStatus, CheckStatus};
