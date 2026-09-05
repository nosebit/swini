//! Generated client and server definitions for the `CroftApi` gRPC service.
//!
//! Exposes protobuf message types and tonic RPC service/client definitions compiled
//! from `proto/croft.proto`. Used by the `CroftClerk` front-office and remote Croft clients
//! to coordinate cluster joins and inspect croft statuses across the Ranch.

tonic::include_proto!("swini.croft");
