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
        let mut cfg_test_pending = false;
        let mut in_cfg_test_region = false;
        for (index, line) in content.lines().enumerate() {
            let line_no = index + 1;
            let code_line = line_without_comments_or_strings(line);
            let trimmed = code_line.trim();
            let literal_line = line_without_comments(line);
            if trimmed == "#[cfg(test)]" {
                cfg_test_pending = true;
            } else if cfg_test_pending && trimmed.starts_with("mod tests") {
                in_cfg_test_region = true;
                cfg_test_pending = false;
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                cfg_test_pending = false;
            }
            let test_context = in_cfg_test_region || is_test_path(&file);

            if trimmed.contains("Command::new(") || trimmed.contains("std::process::Command") {
                findings.push(finding(
                    "unexpected-code-execution",
                    contextual_severity(test_context, AuditSeverity::High),
                    "Process execution surface",
                    "Agent code starts external processes. Review command inputs, allowlists, and sandbox boundaries.",
                    &file,
                    line_no,
                ));
            }

            if trimmed.contains("unsafe ") || trimmed == "unsafe" || trimmed.contains("unsafe{") {
                findings.push(finding(
                    "unsafe-code",
                    contextual_severity(test_context, AuditSeverity::Medium),
                    "Unsafe Rust block",
                    "Unsafe code increases the blast radius of agent-driven changes and should have a clear justification.",
                    &file,
                    line_no,
                ));
            }

            if contains_secret_literal(literal_line.trim()) {
                findings.push(finding(
                    "secret-literal",
                    contextual_severity(test_context, AuditSeverity::High),
                    "Potential secret literal",
                    "A likely secret or token appears in source. Move secrets to environment or a managed secret store.",
                    &file,
                    line_no,
                ));
            }

            if trimmed.contains("MCP") || trimmed.contains("mcp") || trimmed.contains("A2A") {
                findings.push(finding(
                    "agent-interop-surface",
                    contextual_severity(test_context, AuditSeverity::Low),
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
    if root.is_file() {
        if root.extension().is_some_and(|extension| extension == "rs") {
            files.push(root.to_path_buf());
        }
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

    let Some((left, _right)) = line.split_once('=') else {
        return false;
    };
    line.contains('"')
        && left
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .any(is_secret_identifier)
}

fn contextual_severity(test_context: bool, severity: AuditSeverity) -> AuditSeverity {
    if !test_context {
        return severity;
    }

    match severity {
        AuditSeverity::High => AuditSeverity::Low,
        AuditSeverity::Medium => AuditSeverity::Low,
        other => other,
    }
}

fn is_secret_identifier(identifier: &str) -> bool {
    matches!(
        identifier,
        "api_key" | "apikey" | "secret" | "token" | "password"
    ) || identifier.ends_with("_api_key")
        || identifier.ends_with("_secret")
        || identifier.ends_with("_token")
        || identifier.ends_with("_password")
}

fn is_test_path(file: &Path) -> bool {
    file.components()
        .any(|component| component.as_os_str() == "tests")
        || file.file_name().is_some_and(|name| {
            let name = name.to_string_lossy();
            name.ends_with("_test.rs") || name == "test.rs"
        })
}

fn line_without_comments_or_strings(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if !in_string && ch == '/' && chars.peek().is_some_and(|next| *next == '/') {
            break;
        }

        if ch == '"' && !escaped {
            in_string = !in_string;
            out.push(' ');
        } else if in_string {
            out.push(' ');
        } else {
            out.push(ch);
        }

        escaped = in_string && ch == '\\' && !escaped;
        if ch != '\\' {
            escaped = false;
        }
    }

    out
}

fn line_without_comments(line: &str) -> &str {
    line.split_once("//")
        .map(|(before_comment, _)| before_comment)
        .unwrap_or(line)
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

    #[test]
    fn audit_does_not_flag_risky_patterns_inside_strings_or_comments() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"
            fn describe() -> &'static str {
                // std::process::Command::new("ignored")
                "unsafe std::process::Command::new(\"sh\") MCP"
            }
            "#,
        )
        .unwrap();

        let report = audit_agent(dir.path()).unwrap();

        assert!(!report
            .findings
            .iter()
            .any(|finding| finding.id == "unexpected-code-execution"));
        assert!(!report
            .findings
            .iter()
            .any(|finding| finding.id == "unsafe-code"));
        assert!(!report
            .findings
            .iter()
            .any(|finding| finding.id == "agent-interop-surface"));
    }

    #[test]
    fn audit_downgrades_process_execution_in_test_files() {
        let dir = tempdir().unwrap();
        let tests = dir.path().join("tests");
        std::fs::create_dir_all(&tests).unwrap();
        std::fs::write(
            tests.join("cli.rs"),
            r#"
            #[test]
            fn runs_cli() {
                let _ = std::process::Command::new("cargo");
            }
            "#,
        )
        .unwrap();

        let report = audit_agent(dir.path()).unwrap();
        let finding = report
            .findings
            .iter()
            .find(|finding| finding.id == "unexpected-code-execution")
            .expect("process execution finding");

        assert_eq!(finding.severity, AuditSeverity::Low);
    }
}
