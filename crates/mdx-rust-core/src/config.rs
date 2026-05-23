//! Configuration loading and management for mdx-rust

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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