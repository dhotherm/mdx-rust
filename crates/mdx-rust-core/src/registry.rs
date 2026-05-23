//! Agent registry management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents a registered agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredAgent {
    pub name: String,
    pub path: PathBuf,
    pub contract: AgentContract,
    pub registered_at: String, // ISO8601 for simplicity in early phases
}

/// The kind of agent contract detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentContract {
    /// Native Rust function or trait (preferred for Rig agents)
    NativeRust,
    /// Separate process speaking JSON over stdin/stdout
    Process,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Registry {
    pub agents: HashMap<String, RegisteredAgent>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }
}