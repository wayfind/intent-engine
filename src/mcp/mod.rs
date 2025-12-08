//! MCP (Model Context Protocol) server implementation
//!
//! This module provides the MCP server functionality that allows AI assistants
//! to interact with Intent-Engine through the JSON-RPC 2.0 protocol.

pub mod server;
pub mod ws_client;

pub use server::run;
