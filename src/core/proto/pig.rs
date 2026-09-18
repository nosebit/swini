//! Generated client and server definitions for the `PigApi` gRPC service.
//!
//! Exposes protobuf message types and tonic RPC service/client definitions
//! compiled from `proto/pig.proto`. Used by the `PigClerk` front-office, Croft
//! Gate gRPC server, and CLI `swini pig` commands to submit, inspect, and stop
//! workloads across the Ranch.

tonic::include_proto!("swini.pig");
