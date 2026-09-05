//! Generated client and server definitions for the `PlotApi` gRPC service.
//!
//! Exposes protobuf message types and tonic RPC service/client definitions compiled
//! from `proto/plot.proto`. Used by the `PlotClerk` front-office and remote Plot clients
//! to coordinate cluster joins and inspect plot statuses across the Ranch.

tonic::include_proto!("swini.plot");
