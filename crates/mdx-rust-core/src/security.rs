//! Lightweight agent security audit checks.
//!
//! This module intentionally starts with deterministic static checks. The goal
//! is to surface risky agent surfaces early without executing untrusted code.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuditSeverity {
    Info,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AuditFinding {
    pub id: String,
    pub severity: AuditSeverity,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default)]
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecurityAuditReport {
    pub root: String,
    pub findings: Vec<AuditFinding>,
}

impl SecurityAuditReport {
    pub fn summary(&self) -> String {
        let high = self
            .findings
            .iter()
            .filter(|finding| finding.severity == AuditSeverity::High)
            .count();
        let medium = self
            .findings
            .iter()
            .filter(|finding| finding.severity == AuditSeverity::Medium)
            .count();
        format!(
            "{} finding(s), {} high, {} medium",
            self.findings.len(),
            high,
            medium
        )
    }
}

pub fn audit_agent(root: &Path) -> anyhow::Result<SecurityAuditReport> {
    let mut findings = Vec::new();
    let files = collect_rust_files(root)?;

    for file in files {
        let content = std::fs::read_to_string(&file)?;
        for (index, line) in content.lines().enumerate() {
            let line_no = index + 1;
            let trimmed = line.trim();

            if trimmed.contains("Command::new(") || trimmed.contains("std::process::Command") {
                findings.push(finding(
                    "unexpected-code-execution",
                    AuditSeverity::High,
                    "Process execution surface",
                    "Agent code starts external processes. Review command inputs, allowlists, and sandbox boundaries.",
                    &file,
                    line_no,
                ));
            }

            if trimmed.contains("unsafe ") || trimmed == "unsafe" || trimmed.contains("unsafe{") {
                findings.push(finding(
                    "unsafe-code",
                    AuditSeverity::Medium,
                    "Unsafe Rust block",
                    "Unsafe code increases the blast radius of agent-driven changes and should have a clear justification.",
                    &file,
                    line_no,
                ));
            }

            if contains_secret_literal(trimmed) {
                findings.push(finding(
                    "secret-literal",
                    AuditSeverity::High,
                    "Potential secret literal",
                    "A likely secret or token appears in source. Move secrets to environment or a managed secret store.",
                    &file,
                    line_no,
                ));
            }

            if trimmed.contains("MCP") || trimmed.contains("mcp") || trimmed.contains("A2A") {
                findings.push(finding(
                    "agent-interop-surface",
                    AuditSeverity::Low,
                    "Agent interop surface",
                    "MCP or A2A-style integration should validate tool schemas and trust boundaries before live execution.",
                    &file,
                    line_no,
                ));
            }
        }
    }

    if findings.is_empty() {
        findings.push(AuditFinding {
            id: "baseline".to_string(),
            severity: AuditSeverity::Info,
            title: "No obvious static risks found".to_string(),
            description: "Static audit found no process execution, unsafe code, obvious secret literals, or agent interop surfaces.".to_string(),
            file: None,
            line: None,
        });
    }

    Ok(SecurityAuditReport {
        root: root.display().to_string(),
        findings,
    })
}

fn collect_rust_files(root: &Path) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    collect_rust_files_inner(root, &mut files)?;
    Ok(files)
}

fn collect_rust_files_inner(
    root: &Path,
    files: &mut Vec<std::path::PathBuf>,
) -> anyhow::Result<()> {
    if !root.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if path.is_dir() {
            if matches!(
                name.as_ref(),
                "target" | ".git" | ".worktrees" | ".mdx-rust"
            ) {
                continue;
            }
            collect_rust_files_inner(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }

    Ok(())
}

fn finding(
    id: &str,
    severity: AuditSeverity,
    title: &str,
    description: &str,
    file: &Path,
    line: usize,
) -> AuditFinding {
    AuditFinding {
        id: id.to_string(),
        severity,
        title: title.to_string(),
        description: description.to_string(),
        file: Some(file.display().to_string()),
        line: Some(line),
    }
}

fn contains_secret_literal(line: &str) -> bool {
    if line.contains("env::var(")
        || line.contains("std::env::var(")
        || line.contains("option_env!(")
    {
        return false;
    }

    let lower = line.to_lowercase();
    let has_secret_name = ["api_key", "apikey", "secret", "token", "password"]
        .iter()
        .any(|needle| lower.contains(needle));
    has_secret_name && line.contains('"') && line.contains('=')
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn audit_flags_process_execution_and_secret_literals() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("main.rs"),
            r#"
            fn main() {
                let api_key = "secret";
                let _ = std::process::Command::new("sh");
            }
            "#,
        )
        .unwrap();

        let report = audit_agent(dir.path()).unwrap();

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "unexpected-code-execution"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.id == "secret-literal"));
    }

    #[test]
    fn audit_does_not_flag_environment_variable_names_as_secret_literals() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("main.rs"),
            r#"
            fn main() {
                let api_key = std::env::var("OPENAI_API_KEY").ok();
            }
            "#,
        )
        .unwrap();

        let report = audit_agent(dir.path()).unwrap();

        assert!(!report
            .findings
            .iter()
            .any(|finding| finding.id == "secret-literal"));
    }
}
