//! Command-based benchmark harness for measured performance evidence.

use crate::eval::stable_hash_hex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BenchmarkRunConfig {
    pub spec_path: PathBuf,
    pub artifact_root: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BenchmarkSpec {
    pub version: String,
    pub commands: Vec<BenchmarkCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BenchmarkCommand {
    pub id: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<String>,
    #[serde(default = "default_runs")]
    pub runs: usize,
    #[serde(default)]
    pub warmup_runs: usize,
    pub timeout_seconds: Option<u64>,
    #[serde(default)]
    pub metric_hints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BenchmarkRun {
    pub schema_version: String,
    pub run_id: String,
    pub root: String,
    pub spec_path: String,
    pub spec_hash: String,
    pub status: BenchmarkStatus,
    pub total_commands: usize,
    pub passed_commands: usize,
    pub failed_commands: usize,
    pub total_measured_runs: usize,
    pub metrics: Vec<BenchmarkMetricSummary>,
    pub command_records: Vec<BenchmarkCommandRecord>,
    pub artifact_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum BenchmarkStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BenchmarkCommandRecord {
    pub id: String,
    pub command: String,
    pub run_index: usize,
    pub warmup: bool,
    pub success: bool,
    pub timed_out: bool,
    pub status_code: Option<i32>,
    pub duration_ms: u64,
    pub stdout: String,
    pub stderr: String,
    pub parsed_metrics: Vec<BenchmarkMetric>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BenchmarkMetric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub source_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BenchmarkMetricSummary {
    pub command_id: String,
    pub name: String,
    pub unit: String,
    pub samples: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
}

pub fn run_benchmarks(root: &Path, config: &BenchmarkRunConfig) -> anyhow::Result<BenchmarkRun> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let spec_path = if config.spec_path.is_absolute() {
        config.spec_path.clone()
    } else {
        root.join(&config.spec_path)
    };
    let content = std::fs::read(&spec_path)?;
    let spec: BenchmarkSpec = serde_json::from_slice(&content)?;
    let spec_hash = stable_hash_hex(&content);
    let mut command_records = Vec::new();

    for command in &spec.commands {
        let warmups = command.warmup_runs.min(10);
        let runs = command.runs.clamp(1, 50);
        for run_index in 0..warmups {
            command_records.push(run_benchmark_command(&root, command, run_index, true));
        }
        for run_index in 0..runs {
            command_records.push(run_benchmark_command(&root, command, run_index, false));
        }
    }

    let measured_records = command_records
        .iter()
        .filter(|record| !record.warmup)
        .collect::<Vec<_>>();
    let failed_commands = measured_records
        .iter()
        .filter(|record| !record.success)
        .count();
    let passed_commands = measured_records.len().saturating_sub(failed_commands);
    let metrics = summarize_metrics(&measured_records);
    let status = if failed_commands == 0 {
        BenchmarkStatus::Passed
    } else {
        BenchmarkStatus::Failed
    };
    let mut run = BenchmarkRun {
        schema_version: "1.5".to_string(),
        run_id: benchmark_run_id(&root, &spec_hash, &command_records),
        root: root.display().to_string(),
        spec_path: spec_path.display().to_string(),
        spec_hash,
        status,
        total_commands: spec.commands.len(),
        passed_commands,
        failed_commands,
        total_measured_runs: measured_records.len(),
        metrics,
        command_records,
        artifact_path: None,
    };

    if let Some(artifact_root) = &config.artifact_root {
        let path = persist_benchmark_run(artifact_root, &run)?;
        run.artifact_path = Some(path.display().to_string());
        std::fs::write(&path, serde_json::to_vec_pretty(&run)?)?;
    }

    Ok(run)
}

fn run_benchmark_command(
    root: &Path,
    spec: &BenchmarkCommand,
    run_index: usize,
    warmup: bool,
) -> BenchmarkCommandRecord {
    let started = std::time::Instant::now();
    if spec.command.trim().is_empty() {
        return failed_record(spec, run_index, warmup, started, false, "command is empty");
    }

    let cwd = spec
        .cwd
        .as_ref()
        .map(|cwd| root.join(cwd))
        .unwrap_or_else(|| root.to_path_buf());
    if !cwd.is_dir() {
        return failed_record(
            spec,
            run_index,
            warmup,
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
    let timeout = Duration::from_secs(spec.timeout_seconds.unwrap_or(120).clamp(1, 3600));
    let Some(output) = mdx_rust_analysis::editing::run_command_with_timeout(&mut command, timeout)
    else {
        return failed_record(
            spec,
            run_index,
            warmup,
            started,
            false,
            "command could not be started or observed",
        );
    };

    let failure_reason = if output.success() {
        None
    } else if output.timed_out {
        Some(format!(
            "command timed out after {} second(s)",
            timeout.as_secs()
        ))
    } else {
        Some("command exited unsuccessfully".to_string())
    };
    let stdout = truncate_output(&output.stdout);
    let stderr = truncate_output(&output.stderr);
    let parsed_metrics = parse_metrics(&stdout, &stderr, &spec.metric_hints);

    BenchmarkCommandRecord {
        id: spec.id.clone(),
        command: command_label(spec),
        run_index,
        warmup,
        success: failure_reason.is_none(),
        timed_out: output.timed_out,
        status_code: output.status.and_then(|status| status.code()),
        duration_ms: output.duration_ms,
        stdout,
        stderr,
        parsed_metrics,
        failure_reason,
    }
}

fn failed_record(
    spec: &BenchmarkCommand,
    run_index: usize,
    warmup: bool,
    started: std::time::Instant,
    timed_out: bool,
    reason: impl Into<String>,
) -> BenchmarkCommandRecord {
    BenchmarkCommandRecord {
        id: spec.id.clone(),
        command: command_label(spec),
        run_index,
        warmup,
        success: false,
        timed_out,
        status_code: None,
        duration_ms: started.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
        stdout: String::new(),
        stderr: String::new(),
        parsed_metrics: Vec::new(),
        failure_reason: Some(reason.into()),
    }
}

fn summarize_metrics(records: &[&BenchmarkCommandRecord]) -> Vec<BenchmarkMetricSummary> {
    let mut groups = std::collections::BTreeMap::<(String, String, String), Vec<f64>>::new();
    for record in records {
        groups
            .entry((record.id.clone(), "wall_time".to_string(), "ms".to_string()))
            .or_default()
            .push(record.duration_ms as f64);
        for metric in &record.parsed_metrics {
            groups
                .entry((record.id.clone(), metric.name.clone(), metric.unit.clone()))
                .or_default()
                .push(metric.value);
        }
    }

    groups
        .into_iter()
        .map(|((command_id, name, unit), mut values)| {
            values.sort_by(f64::total_cmp);
            let samples = values.len();
            let min = values.first().copied().unwrap_or(0.0);
            let max = values.last().copied().unwrap_or(0.0);
            let mean = if samples == 0 {
                0.0
            } else {
                values.iter().sum::<f64>() / samples as f64
            };
            BenchmarkMetricSummary {
                command_id,
                name,
                unit,
                samples,
                min,
                max,
                mean,
            }
        })
        .collect()
}

fn parse_metrics(stdout: &str, stderr: &str, hints: &[String]) -> Vec<BenchmarkMetric> {
    let mut metrics = Vec::new();
    for line in stdout.lines().chain(stderr.lines()) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(metric) = parse_metric_line(line, hints) {
            metrics.push(metric);
        }
    }
    metrics
}

fn parse_metric_line(line: &str, hints: &[String]) -> Option<BenchmarkMetric> {
    let lower = line.to_ascii_lowercase();
    let numbers = numbers_in_line(line);
    let value = numbers.first().copied()?;
    let hinted_name = hints
        .iter()
        .find(|hint| lower.contains(&hint.to_ascii_lowercase()))
        .cloned();

    if lower.contains("ops/sec") || lower.contains("op/s") {
        Some(metric(
            hinted_name.unwrap_or_else(|| "throughput".to_string()),
            value,
            "ops/sec",
            line,
        ))
    } else if lower.contains("req/s")
        || lower.contains("requests/sec")
        || lower.contains("requests per second")
    {
        Some(metric(
            hinted_name.unwrap_or_else(|| "throughput".to_string()),
            value,
            "req/s",
            line,
        ))
    } else if lower.contains("mb/s") {
        Some(metric(
            hinted_name.unwrap_or_else(|| "bandwidth".to_string()),
            value,
            "MB/s",
            line,
        ))
    } else if lower.contains("p95") && lower.contains("ms") {
        Some(metric(
            hinted_name.unwrap_or_else(|| "p95_latency".to_string()),
            numbers.last().copied().unwrap_or(value),
            "ms",
            line,
        ))
    } else if lower.contains("latency") && lower.contains("ms") {
        Some(metric(
            hinted_name.unwrap_or_else(|| "latency".to_string()),
            value,
            "ms",
            line,
        ))
    } else {
        None
    }
}

fn metric(name: String, value: f64, unit: &str, source_line: &str) -> BenchmarkMetric {
    BenchmarkMetric {
        name,
        value,
        unit: unit.to_string(),
        source_line: source_line.to_string(),
    }
}

fn numbers_in_line(line: &str) -> Vec<f64> {
    let mut numbers = Vec::new();
    for token in
        line.split(|ch: char| !(ch.is_ascii_digit() || matches!(ch, '.' | '-' | '_' | ',')))
    {
        let normalized = token.replace(['_', ','], "");
        if normalized.is_empty() || normalized == "." || normalized == "-" {
            continue;
        }
        if let Ok(value) = normalized.parse::<f64>() {
            numbers.push(value);
        }
    }
    numbers
}

fn truncate_output(value: &str) -> String {
    const MAX: usize = 16_384;
    if value.len() <= MAX {
        value.to_string()
    } else {
        let boundary = value
            .char_indices()
            .map(|(index, _)| index)
            .take_while(|index| *index <= MAX)
            .last()
            .unwrap_or(0);
        format!("{}...[truncated]", &value[..boundary])
    }
}

fn command_label(spec: &BenchmarkCommand) -> String {
    std::iter::once(spec.command.as_str())
        .chain(spec.args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ")
}

fn benchmark_run_id(root: &Path, spec_hash: &str, records: &[BenchmarkCommandRecord]) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(root.display().to_string().as_bytes());
    bytes.extend_from_slice(spec_hash.as_bytes());
    for record in records {
        bytes.extend_from_slice(record.id.as_bytes());
        bytes.extend_from_slice(record.duration_ms.to_string().as_bytes());
        bytes.extend_from_slice(record.success.to_string().as_bytes());
    }
    stable_hash_hex(&bytes)
}

fn persist_benchmark_run(artifact_root: &Path, run: &BenchmarkRun) -> anyhow::Result<PathBuf> {
    let dir = artifact_root.join("benchmarks");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join(format!(
        "benchmark-run-{}.json",
        run.run_id.replace(':', "-")
    )))
}

fn default_runs() -> usize {
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn benchmark_harness_parses_metrics_and_persists_artifact() {
        let dir = tempdir().unwrap();
        let spec_path = dir.path().join("benchmarks.json");
        std::fs::write(
            &spec_path,
            r#"{
  "version": "v1",
  "commands": [
    {
      "id": "echo-throughput",
      "command": "sh",
      "args": ["-c", "printf 'throughput: 1234 ops/sec\\nlatency p95: 4.5 ms\\n'"],
      "runs": 2,
      "warmup_runs": 1,
      "timeout_seconds": 5
    }
  ]
}"#,
        )
        .unwrap();

        let run = run_benchmarks(
            dir.path(),
            &BenchmarkRunConfig {
                spec_path,
                artifact_root: Some(dir.path().join(".mdx-rust")),
            },
        )
        .unwrap();

        assert_eq!(run.status, BenchmarkStatus::Passed);
        assert_eq!(run.total_measured_runs, 2);
        assert!(run.metrics.iter().any(|metric| {
            metric.name == "throughput" && metric.unit == "ops/sec" && metric.samples == 2
        }));
        assert!(run.metrics.iter().any(|metric| {
            metric.name == "p95_latency" && (metric.mean - 4.5).abs() < f64::EPSILON
        }));
        assert!(run
            .artifact_path
            .as_ref()
            .is_some_and(|path| Path::new(path).is_file()));
    }

    #[test]
    fn benchmark_harness_reports_invalid_commands_without_panicking() {
        let dir = tempdir().unwrap();
        let spec_path = dir.path().join("benchmarks.json");
        std::fs::write(
            &spec_path,
            r#"{
  "version": "v1",
  "commands": [
    {"id": "empty", "command": "", "runs": 1}
  ]
}"#,
        )
        .unwrap();

        let run = run_benchmarks(
            dir.path(),
            &BenchmarkRunConfig {
                spec_path,
                artifact_root: None,
            },
        )
        .unwrap();

        assert_eq!(run.status, BenchmarkStatus::Failed);
        assert_eq!(run.failed_commands, 1);
        assert_eq!(
            run.command_records[0].failure_reason.as_deref(),
            Some("command is empty")
        );
    }
}
