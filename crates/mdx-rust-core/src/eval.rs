//! Evaluation dataset and scorer metadata.
//!
//! These types make experiment records explicit about what was measured and
//! how. The current scorer is intentionally simple, but the optimizer now has
//! a stable place to grow policy-aligned and LLM-judge scoring.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvaluationSample {
    pub id: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvaluationDataset {
    pub version: String,
    pub samples: Vec<EvaluationSample>,
}

impl EvaluationDataset {
    pub fn synthetic_v1() -> Self {
        let samples = (0..5)
            .map(|i| EvaluationSample {
                id: format!("synthetic-addition-{i}"),
                input: serde_json::json!({
                    "query": format!("What is {} + {}?", i, i + 1),
                    "context": null
                }),
            })
            .collect();

        Self {
            version: "synthetic_v1".to_string(),
            samples,
        }
    }

    pub fn content_hash(&self) -> String {
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        stable_hash_hex(&bytes)
    }

    /// Load a dataset from JSON.
    ///
    /// Accepted shapes:
    /// - `{ "version": "...", "samples": [{ "id": "...", "input": {...} }] }`
    /// - `[{ "id": "...", "input": {...} }]`
    /// - `[{...}, {...}]` where each object is treated directly as an input.
    pub fn load_from_path(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;

        if let Ok(dataset) = serde_json::from_str::<EvaluationDataset>(&content) {
            return Ok(dataset);
        }

        let value: serde_json::Value = serde_json::from_str(&content)?;
        let Some(items) = value.as_array() else {
            anyhow::bail!("dataset must be an EvaluationDataset object or JSON array");
        };

        let mut samples = Vec::with_capacity(items.len());
        for (index, item) in items.iter().enumerate() {
            if let Some(input) = item.get("input") {
                let id = item
                    .get("id")
                    .and_then(|id| id.as_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("sample-{index}"));
                samples.push(EvaluationSample {
                    id,
                    input: input.clone(),
                });
            } else {
                samples.push(EvaluationSample {
                    id: format!("sample-{index}"),
                    input: item.clone(),
                });
            }
        }

        Ok(Self {
            version: dataset_version_from_path(path),
            samples,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScorerMetadata {
    pub id: String,
    pub version: String,
}

impl ScorerMetadata {
    pub fn mechanical_v1() -> Self {
        Self {
            id: "mechanical".to_string(),
            version: "v1".to_string(),
        }
    }

    pub fn label(&self) -> String {
        format!("{}_{}", self.id, self.version)
    }
}

pub fn stable_hash_hex(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn dataset_version_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .map(|stem| format!("file:{stem}"))
        .unwrap_or_else(|| "file:dataset".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_dataset_from_raw_input_array() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("dataset.json");
        std::fs::write(
            &path,
            r#"[{"query":"hello"},{"query":"world","context":null}]"#,
        )
        .unwrap();

        let dataset = EvaluationDataset::load_from_path(&path).unwrap();

        assert_eq!(dataset.samples.len(), 2);
        assert_eq!(dataset.samples[0].id, "sample-0");
        assert_eq!(dataset.version, "file:dataset");
    }

    #[test]
    fn load_dataset_from_structured_object() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("evals.json");
        std::fs::write(
            &path,
            r#"{"version":"v9","samples":[{"id":"a","input":{"query":"hello"}}]}"#,
        )
        .unwrap();

        let dataset = EvaluationDataset::load_from_path(&path).unwrap();

        assert_eq!(dataset.version, "v9");
        assert_eq!(dataset.samples[0].id, "a");
    }
}
