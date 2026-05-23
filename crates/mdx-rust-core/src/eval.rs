//! Evaluation dataset and scorer metadata.
//!
//! These types make experiment records explicit about what was measured and
//! how. The current scorer is intentionally simple, but the optimizer now has
//! a stable place to grow policy-aligned and LLM-judge scoring.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BehaviorEvalSpec {
    pub version: String,
    pub commands: Vec<BehaviorCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BehaviorCommand {
    pub id: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
    #[serde(default = "default_expect_success")]
    pub expect_success: bool,
    #[serde(default)]
    pub expect_stdout_contains: Vec<String>,
    #[serde(default)]
    pub expect_stderr_contains: Vec<String>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BehaviorEvalReport {
    pub schema_version: String,
    pub spec_path: String,
    pub spec_hash: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub command_records: Vec<BehaviorCommandRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BehaviorCommandRecord {
    pub id: String,
    pub command: String,
    pub success: bool,
    pub timed_out: bool,
    pub status_code: Option<i32>,
    pub duration_ms: u64,
    pub stdout: String,
    pub stderr: String,
    pub failure_reason: Option<String>,
}

impl BehaviorEvalReport {
    pub fn passed(&self) -> bool {
        self.failed == 0
    }
}

pub fn run_behavior_evals(root: &Path, spec_path: &Path) -> anyhow::Result<BehaviorEvalReport> {
    let path = if spec_path.is_absolute() {
        spec_path.to_path_buf()
    } else {
        root.join(spec_path)
    };
    let content = std::fs::read(&path)?;
    let spec: BehaviorEvalSpec = serde_json::from_slice(&content)?;
    let mut command_records = Vec::with_capacity(spec.commands.len());

    for command_spec in &spec.commands {
        let record = run_behavior_command(root, command_spec);
        command_records.push(record);
    }

    let passed = command_records
        .iter()
        .filter(|record| record.success)
        .count();
    let failed = command_records.len().saturating_sub(passed);

    Ok(BehaviorEvalReport {
        schema_version: "0.4".to_string(),
        spec_path: path.display().to_string(),
        spec_hash: stable_hash_hex(&content),
        total: command_records.len(),
        passed,
        failed,
        command_records,
    })
}

fn run_behavior_command(root: &Path, spec: &BehaviorCommand) -> BehaviorCommandRecord {
    let started = std::time::Instant::now();
    if spec.command.trim().is_empty() {
        return failed_behavior_record(spec, started, false, "command is empty");
    }

    let cwd = spec
        .cwd
        .as_ref()
        .map(|cwd| root.join(cwd))
        .unwrap_or_else(|| root.to_path_buf());
    if !cwd.is_dir() {
        return failed_behavior_record(
            spec,
            started,
            false,
            format!(
                "cwd does not exist or is not a directory: {}",
                cwd.display()
            ),
        );
    }

    let mut command = std::process::Command::new(&spec.command);
    command.args(&spec.args);
    command.current_dir(&cwd);

    let timeout = Duration::from_secs(spec.timeout_seconds.unwrap_or(120));
    let Some(output) = mdx_rust_analysis::editing::run_command_with_timeout(&mut command, timeout)
    else {
        return failed_behavior_record(
            spec,
            started,
            false,
            "command could not be started or observed",
        );
    };

    let mut failure_reason = None;
    if output.success() != spec.expect_success {
        failure_reason = Some(format!(
            "expected success={} but command success={}",
            spec.expect_success,
            output.success()
        ));
    }
    if failure_reason.is_none() {
        if let Some(missing) = spec
            .expect_stdout_contains
            .iter()
            .find(|needle| !output.stdout.contains(*needle))
        {
            failure_reason = Some(format!("stdout did not contain {missing:?}"));
        }
    }
    if failure_reason.is_none() {
        if let Some(missing) = spec
            .expect_stderr_contains
            .iter()
            .find(|needle| !output.stderr.contains(*needle))
        {
            failure_reason = Some(format!("stderr did not contain {missing:?}"));
        }
    }

    BehaviorCommandRecord {
        id: spec.id.clone(),
        command: command_label(spec),
        success: failure_reason.is_none(),
        timed_out: output.timed_out,
        status_code: output.status.and_then(|status| status.code()),
        duration_ms: output.duration_ms,
        stdout: output.stdout,
        stderr: output.stderr,
        failure_reason,
    }
}

fn failed_behavior_record(
    spec: &BehaviorCommand,
    started: std::time::Instant,
    timed_out: bool,
    reason: impl Into<String>,
) -> BehaviorCommandRecord {
    BehaviorCommandRecord {
        id: spec.id.clone(),
        command: command_label(spec),
        success: false,
        timed_out,
        status_code: None,
        duration_ms: elapsed_millis_u64(started),
        stdout: String::new(),
        stderr: String::new(),
        failure_reason: Some(reason.into()),
    }
}

fn command_label(spec: &BehaviorCommand) -> String {
    std::iter::once(spec.command.as_str())
        .chain(spec.args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ")
}

fn elapsed_millis_u64(started: std::time::Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn default_expect_success() -> bool {
    true
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

    #[test]
    fn behavior_eval_runs_command_specs() {
        let dir = tempdir().unwrap();
        let spec_path = dir.path().join("evals.json");
        std::fs::write(
            &spec_path,
            r#"{
  "version": "v1",
  "commands": [
    {
      "id": "hello",
      "command": "cargo",
      "args": ["--version"],
      "expect_stdout_contains": ["cargo"],
      "timeout_seconds": 30
    }
  ]
}"#,
        )
        .unwrap();

        let report = run_behavior_evals(dir.path(), &spec_path).unwrap();

        assert!(report.passed());
        assert_eq!(report.total, 1);
        assert_eq!(report.command_records[0].id, "hello");
    }

    #[test]
    fn behavior_eval_reports_malformed_commands_without_process_errors() {
        let dir = tempdir().unwrap();
        let spec_path = dir.path().join("evals.json");
        std::fs::write(
            &spec_path,
            r#"{
  "version": "v1",
  "commands": [
    {
      "id": "empty",
      "command": "",
      "timeout_seconds": 30
    }
  ]
}"#,
        )
        .unwrap();

        let report = run_behavior_evals(dir.path(), &spec_path).unwrap();

        assert!(!report.passed());
        assert_eq!(report.failed, 1);
        assert_eq!(
            report.command_records[0].failure_reason.as_deref(),
            Some("command is empty")
        );
        assert!(!report.command_records[0].timed_out);
    }
}
