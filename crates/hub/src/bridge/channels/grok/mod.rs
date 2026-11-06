//! C14.6: opt-in Grok leader connect/spawn.
//!
//! This module starts or resumes a documented `grok --leader` session so
//! `bridge::grok`'s ACP delivery path has a live socket. It never mutates
//! that C12 inject/refuse logic and never writes a PTY or invented socket.

#![allow(unused_imports)]
mod leader;

pub use leader::{connect_grok_leader_session, grok_leader_status, GrokConnectResult};
