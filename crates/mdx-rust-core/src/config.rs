//! Configuration loading and management for mdx-rust

use serde::{Deserialize, Serialize};

/// Root configuration for an mdx-rust project
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Default models to use for different roles
    pub models: ModelConfig,

    /// Artifact directory name (default: .mdx-rust)
    #[serde(default = "default_artifact_dir")]
    pub artifact_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelConfig {
    /// Model used for deep diagnosis and candidate generation
    pub analyzer: Option<String>,

    /// Model used for LLM-as-Judge scoring
    pub judge: Option<String>,

    /// Default model for lighter tasks
    pub default: Option<String>,
}

fn default_artifact_dir() -> String {
    ".mdx-rust".to_string()
}

impl Config {
    /// Load configuration from the standard location, with sensible defaults.
    pub fn load_from_project(root: &std::path::Path) -> anyhow::Result<Self> {
        let config_path = root.join(".mdx-rust/config.toml");

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(config_path)?;
        let mut cfg: Config = toml::from_str(&content)?;

        if cfg.artifact_dir.is_empty() {
            cfg.artifact_dir = default_artifact_dir();
        }

        Ok(cfg)
    }
}