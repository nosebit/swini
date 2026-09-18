//! Workload domain models, configurations, and front-office services.
//!
//! Submodules organize workload management logic:
//! - [`types`]: Domain entities ([`Pig`](types::Pig),
//!   [`Piglet`](types::Piglet), [`Task`](types::Task), [`Yard`](types::Yard)).
//! - [`config`]: Declarative YAML workload configuration parser and normalizer.
//! - [`clerk`]: Domain clerk managing workload submissions and `PigApi` gRPC
//!   service.

pub mod clerk;
pub mod config;
pub mod types;

#[allow(unused_imports)]
pub use clerk::PigClerk;
#[allow(unused_imports)]
pub use config::*;
#[allow(unused_imports)]
pub use types::*;
