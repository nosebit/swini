//! Task engine abstractions, execution drivers, and metric sampling.
//!
//! Submodules:
//! - [`types`]: Core traits ([`TaskEngine`]) and metrics ([`TaskStats`]).
//! - [`engines`]: Concrete task engine drivers
//!   ([`ExecEngine`](engines::ExecEngine)).

pub mod engines;
pub mod types;

#[allow(unused_imports)]
pub use engines::*;
#[allow(unused_imports)]
pub use types::*;
