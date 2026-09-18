#![doc = include_str!("README.md")]

pub mod format;
pub mod proto;
pub mod sys;
pub mod telemetry;

#[cfg(test)]
pub static TEST_ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
