//! Read-only performance signal scanner.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PerformanceScanConfig {
    pub target: Option<PathBuf>,
    pub max_files: usize,
}

impl Default for PerformanceScanConfig {
    fn default() -> Self {
        Self {
            target: None,
            max_files: 250,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PerformanceRun {
    pub schema_version: String,
    pub root: PathBuf,
    pub target: PathBuf,
    pub summary: PerformanceSummary,
    pub findings: Vec<PerformanceFinding>,
    pub recommendations: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PerformanceSummary {
    pub scanned_files: usize,
    pub finding_count: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PerformanceFinding {
    pub id: String,
    pub file: PathBuf,
    pub line: usize,
    pub severity: String,
    pub category: String,
    pub title: String,
    pub evidence: String,
    pub recommendation: String,
}

/// Requires: `root` points at the workspace that owns the optional scan target.
/// Ensures: returns performance signals without writing source files.
pub fn scan_performance(
    root: &Path,
    config: &PerformanceScanConfig,
) -> anyhow::Result<PerformanceRun> {
    let target = match &config.target {
        Some(target) => root.join(target),
        None => root.to_path_buf(),
    };
    if !target.exists() {
        anyhow::bail!("perf target does not exist: {}", target.display());
    }

    let mut warnings = Vec::new();
    let files = rust_files(root, &target, config.max_files.max(1), &mut warnings)?;
    let mut findings = Vec::new();
    for file in &files {
        findings.extend(scan_file(root, file)?);
    }
    findings.sort_by(|left, right| {
        severity_rank(&left.severity)
            .cmp(&severity_rank(&right.severity))
            .then_with(|| left.file.cmp(&right.file))
            .then_with(|| left.line.cmp(&right.line))
    });

    let high = findings
        .iter()
        .filter(|finding| finding.severity == "high")
        .count();
    let medium = findings
        .iter()
        .filter(|finding| finding.severity == "medium")
        .count();
    let low = findings
        .iter()
        .filter(|finding| finding.severity == "low")
        .count();

    Ok(PerformanceRun {
        schema_version: "1.0".to_string(),
        root: root.to_path_buf(),
        target,
        summary: PerformanceSummary {
            scanned_files: files.len(),
            finding_count: findings.len(),
            high,
            medium,
            low,
        },
        recommendations: performance_recommendations(&findings),
        findings,
        warnings,
    })
}

fn scan_file(root: &Path, file: &Path) -> anyhow::Result<Vec<PerformanceFinding>> {
    let source = std::fs::read_to_string(file)?;
    let mut findings = Vec::new();
    let mut in_async_fn = false;
    let mut brace_depth = 0isize;
    let mut loop_depth = 0isize;
    let mut in_raw_string = false;
    let test_context = is_test_context(root, file);

    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if in_raw_string {
            if trimmed.contains("\"#") {
                in_raw_string = false;
            }
            continue;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        if trimmed.contains("r#\"") && !trimmed.contains("\"#") {
            in_raw_string = true;
            continue;
        }
        let code = strip_string_literals(line);
        let code_trimmed = code.trim_start();
        if code_trimmed.starts_with("async fn ")
            || code_trimmed.starts_with("pub async fn ")
            || code_trimmed.contains(" async fn ")
        {
            in_async_fn = true;
            brace_depth = 0;
        }
        if in_async_fn {
            brace_depth += count_char(&code, '{') as isize;
            brace_depth -= count_char(&code, '}') as isize;
            if brace_depth <= 0 && code.contains('}') {
                in_async_fn = false;
            }
        }
        if starts_loop(code_trimmed) {
            loop_depth += 1;
        }
        if loop_depth > 0 && code.contains('}') {
            loop_depth -= count_char(&code, '}') as isize;
            if loop_depth < 0 {
                loop_depth = 0;
            }
        }

        if !test_context && code.contains(".clone()") {
            findings.push(finding(
                root,
                file,
                index + 1,
                line,
                FindingSpec {
                    severity: if loop_depth > 0 { "medium" } else { "low" },
                    category: "clone-pressure",
                    title: "Clone pressure",
                    recommendation: if loop_depth > 0 {
                        "Review whether the clone can become a borrow before this loop becomes hot."
                    } else {
                        "Review whether ownership or borrowing can remove this clone."
                    },
                },
            ));
        }
        if !test_context
            && loop_depth > 0
            && (code.contains(".to_string()") || code.contains("format!("))
        {
            findings.push(finding(
                root,
                file,
                index + 1,
                line,
                FindingSpec {
                    severity: "medium",
                    category: "allocation-in-loop",
                    title: "Allocation inside loop",
                    recommendation:
                        "Consider precomputing, borrowing, or reusing buffers if this loop is hot.",
                },
            ));
        }
        if in_async_fn && (code.contains("std::fs::") || code.contains("std::thread::sleep")) {
            findings.push(finding(
                root,
                file,
                index + 1,
                line,
                FindingSpec {
                    severity: "high",
                    category: "blocking-in-async",
                    title: "Blocking operation inside async function",
                    recommendation:
                        "Use async-aware filesystem, timers, or spawn_blocking when behavior evidence allows it.",
                },
            ));
        }
        if in_async_fn && (code.contains("std::sync::Mutex") || code.contains("Arc<Mutex<")) {
            findings.push(finding(
                root,
                file,
                index + 1,
                line,
                FindingSpec {
                    severity: "medium",
                    category: "sync-lock-in-async",
                    title: "Synchronous lock hint inside async function",
                    recommendation:
                        "Check whether an async-aware lock or narrower critical section is appropriate.",
                },
            ));
        }
    }

    Ok(findings)
}

struct FindingSpec<'a> {
    severity: &'a str,
    category: &'a str,
    title: &'a str,
    recommendation: &'a str,
}

fn finding(
    root: &Path,
    file: &Path,
    line: usize,
    evidence: &str,
    spec: FindingSpec<'_>,
) -> PerformanceFinding {
    PerformanceFinding {
        id: format!("{}-{line}", spec.category),
        file: relative_path(root, file),
        line,
        severity: spec.severity.to_string(),
        category: spec.category.to_string(),
        title: spec.title.to_string(),
        evidence: evidence.trim().to_string(),
        recommendation: spec.recommendation.to_string(),
    }
}

fn performance_recommendations(findings: &[PerformanceFinding]) -> Vec<String> {
    let mut recommendations = Vec::new();
    if findings
        .iter()
        .any(|finding| finding.category == "blocking-in-async")
    {
        recommendations.push(
            "Prioritize blocking-in-async findings before enabling deeper autonomous refactors."
                .to_string(),
        );
    }
    if findings
        .iter()
        .any(|finding| finding.category == "clone-pressure")
    {
        recommendations.push(
            "Use evidence and benchmarks before replacing clones in public behavior paths."
                .to_string(),
        );
    }
    if recommendations.is_empty() {
        recommendations.push(
            "No obvious static performance pressure found in the scanned target.".to_string(),
        );
    }
    recommendations
}

fn starts_loop(trimmed: &str) -> bool {
    trimmed.starts_with("for ") || trimmed.starts_with("while ") || trimmed.starts_with("loop ")
}

fn strip_string_literals(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    while let Some(ch) = chars.next() {
        if in_string {
            if ch == '\\' {
                let _ = chars.next();
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            out.push(' ');
            continue;
        }
        if ch == '"' {
            in_string = true;
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    out
}

fn count_char(value: &str, needle: char) -> usize {
    value.chars().filter(|ch| *ch == needle).count()
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "high" => 0,
        "medium" => 1,
        _ => 2,
    }
}

fn rust_files(
    root: &Path,
    target: &Path,
    max_files: usize,
    warnings: &mut Vec<String>,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if target.is_file() {
        if target
            .extension()
            .is_some_and(|extension| extension == "rs")
        {
            files.push(target.to_path_buf());
        }
        return Ok(files);
    }
    collect_rust_files(root, target, max_files, warnings, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_rust_files(
    root: &Path,
    dir: &Path,
    max_files: usize,
    warnings: &mut Vec<String>,
    files: &mut Vec<PathBuf>,
) -> anyhow::Result<()> {
    if files.len() >= max_files || is_noise_path(root, dir) && dir != root {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            collect_rust_files(root, &path, max_files, warnings, files)?;
        } else if file_type.is_file() && path.extension().is_some_and(|extension| extension == "rs")
        {
            if files.len() >= max_files {
                warnings.push(format!(
                    "performance scan stopped at max_files={max_files}; raise --max-files for a larger scan"
                ));
                return Ok(());
            }
            files.push(path);
        }
    }
    Ok(())
}

fn is_test_context(root: &Path, path: &Path) -> bool {
    relative_path(root, path)
        .components()
        .any(|component| component.as_os_str() == "tests")
}

fn is_noise_path(root: &Path, path: &Path) -> bool {
    relative_path(root, path).components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some("target" | ".git" | ".mdx-rust" | "node_modules")
        )
    })
}

fn relative_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn performance_scan_finds_async_blocking_and_loop_allocation() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/lib.rs"),
            r#"pub async fn load() -> String {
    let text = std::fs::read_to_string("config.toml").unwrap();
    text
}

pub fn copy(values: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        out.push(value.clone().to_string());
    }
    out
}
"#,
        )
        .unwrap();

        let run = scan_performance(
            dir.path(),
            &PerformanceScanConfig {
                target: Some(PathBuf::from("src/lib.rs")),
                ..PerformanceScanConfig::default()
            },
        )
        .unwrap();

        assert_eq!(run.summary.high, 1);
        assert!(run
            .findings
            .iter()
            .any(|finding| finding.category == "blocking-in-async"));
        assert!(run
            .findings
            .iter()
            .any(|finding| finding.category == "clone-pressure"));
        assert!(run
            .findings
            .iter()
            .any(|finding| finding.category == "allocation-in-loop"));
    }

    #[test]
    fn performance_scan_ignores_raw_string_fixtures() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/lib.rs"),
            r##"pub fn real(values: &[String]) -> usize {
    values.len()
}

const FIXTURE: &str = r#"
pub async fn fake() {
    let text = std::fs::read_to_string("config.toml").unwrap();
    for value in values {
        out.push(value.clone().to_string());
    }
}
"#;
"##,
        )
        .unwrap();

        let run = scan_performance(
            dir.path(),
            &PerformanceScanConfig {
                target: Some(PathBuf::from("src/lib.rs")),
                ..PerformanceScanConfig::default()
            },
        )
        .unwrap();

        assert!(run.findings.is_empty());
    }

    #[test]
    fn performance_scan_ignores_plain_string_literals() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/lib.rs"),
            r#"pub fn detector_text() -> &'static str {
    "value.clone() and std::fs::read_to_string should stay text"
}
"#,
        )
        .unwrap();

        let run = scan_performance(
            dir.path(),
            &PerformanceScanConfig {
                target: Some(PathBuf::from("src/lib.rs")),
                ..PerformanceScanConfig::default()
            },
        )
        .unwrap();

        assert!(run.findings.is_empty());
    }
}
