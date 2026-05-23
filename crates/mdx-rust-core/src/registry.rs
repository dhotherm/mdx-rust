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

    /// Load registry from the .mdx-rust directory
    pub fn load_from(artifact_root: &std::path::Path) -> anyhow::Result<Self> {
        let registry_path = artifact_root.join("registry.json");
        if !registry_path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(registry_path)?;
        let reg: Registry = serde_json::from_str(&content)?;
        Ok(reg)
    }

    /// Save registry to the .mdx-rust directory
    pub fn save_to(&self, artifact_root: &std::path::Path) -> anyhow::Result<()> {
        let registry_path = artifact_root.join("registry.json");
        if let Some(parent) = registry_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(registry_path, content)?;
        Ok(())
    }

    pub fn register(&mut self, agent: RegisteredAgent) {
        self.agents.insert(agent.name.clone(), agent);
    }

    pub fn get(&self, name: &str) -> Option<&RegisteredAgent> {
        self.agents.get(name)
    }
}