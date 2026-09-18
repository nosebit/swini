//! Concrete platform host system driver variants.
//!
//! Submodules organize OS-specific implementations of [`Sys`](super::Sys):
//! - [`unix`]: Base Unix driver using standard libc and process APIs.
//! - [`macos`]: macOS Darwin adapter.
//! - [`linux`]: Generic Linux and Ubuntu distribution adapters.

pub mod linux;
pub mod macos;
pub mod unix;

pub use linux::{LinuxSys, UbuntuSys};
pub use macos::MacSys;
pub use unix::UnixSys;
