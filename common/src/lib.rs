//! BlindHop Common — Shared types, transport traits, and utilities.
//!
//! This crate provides the core abstractions for the BlindHop mixnet privacy layer:
//! - [`MixnetTransport`] trait for pluggable mixnet backends (Nym, custom Sphinx, etc.)
//! - Configuration types ([`BlindHopConfig`], [`PrivacyMode`])
//! - JSON-RPC message handling for Substrate chain communication
//! - Error types and metrics collection

pub mod config;
pub mod error;
pub mod metrics;
pub mod rpc;
pub mod transport;
