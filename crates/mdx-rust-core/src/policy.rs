//! Structured project policy parsing.
//!
//! v0.4 keeps policy intentionally simple: markdown bullets and numbered lines
//! become versioned rules with coarse categories. The parser is deterministic so
//! reports can explain which rule a finding appears to relate to.

use crate::eval::stable_hash_hex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProjectPolicy {
    pub schema_version: String,
    pub path: String,
    pub hash: String,
    pub rules: Vec<PolicyRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PolicyRule {
    pub id: String,
    pub line: usize,
    pub category: PolicyCategory,
    pub severity: PolicySeverity,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum PolicyCategory {
    PanicSafety,
    ErrorContext,
    InputValidation,
    ProcessExecution,
    Environment,
    UnsafeCode,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum PolicySeverity {
    Low,
    Medium,
    High,
}

pub fn load_project_policy(
    root: &Path,
    policy_path: Option<&Path>,
) -> anyhow::Result<Option<ProjectPolicy>> {
    let Some(path) = resolve_policy_path(root, policy_path) else {
        return Ok(None);
    };
    let content = std::fs::read(&path)?;
    let text = String::from_utf8_lossy(&content);
    Ok(Some(ProjectPolicy {
        schema_version: "0.4".to_string(),
        path: path.display().to_string(),
        hash: stable_hash_hex(&content),
        rules: parse_policy_rules(&text),
    }))
}

fn resolve_policy_path(root: &Path, policy_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(policy_path) = policy_path {
        return Some(if policy_path.is_absolute() {
            policy_path.to_path_buf()
        } else {
            root.join(policy_path)
        });
    }

    let default = root.join(".mdx-rust/policies.md");
    default.exists().then_some(default)
}

fn parse_policy_rules(content: &str) -> Vec<PolicyRule> {
    content
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let text = extract_rule_text(line)?;
            let category = categorize_rule(&text);
            let severity = severity_for_category(&category);
            Some(PolicyRule {
                id: format!("policy-rule-{}", index + 1),
                line: index + 1,
                category,
                severity,
                text,
            })
        })
        .collect()
}

fn extract_rule_text(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("- ") {
        return normalize_rule_text(trimmed.trim_start_matches("- "));
    }

    let (prefix, rest) = trimmed.split_once(['.', ')'])?;
    if prefix.chars().all(|ch| ch.is_ascii_digit()) {
        let text = rest.trim();
        if !text.is_empty() {
            return normalize_rule_text(text);
        }
    }

    None
}

fn normalize_rule_text(text: &str) -> Option<String> {
    let text = text.trim();
    if text.is_empty() || text.contains("...") {
        return None;
    }
    Some(text.to_string())
}

fn categorize_rule(text: &str) -> PolicyCategory {
    let lower = text.to_ascii_lowercase();
    if lower.contains("unwrap") || lower.contains("expect") || lower.contains("panic") {
        PolicyCategory::PanicSafety
    } else if lower.contains("context") || lower.contains("error") || lower.contains("failure") {
        PolicyCategory::ErrorContext
    } else if lower.contains("input")
        || lower.contains("validate")
        || lower.contains("validation")
        || lower.contains("request")
    {
        PolicyCategory::InputValidation
    } else if lower.contains("command") || lower.contains("process") || lower.contains("shell") {
        PolicyCategory::ProcessExecution
    } else if lower.contains("environment") || lower.contains("env var") || lower.contains("env") {
        PolicyCategory::Environment
    } else if lower.contains("unsafe") {
        PolicyCategory::UnsafeCode
    } else {
        PolicyCategory::General
    }
}

fn severity_for_category(category: &PolicyCategory) -> PolicySeverity {
    match category {
        PolicyCategory::ProcessExecution | PolicyCategory::UnsafeCode => PolicySeverity::High,
        PolicyCategory::PanicSafety
        | PolicyCategory::ErrorContext
        | PolicyCategory::InputValidation
        | PolicyCategory::Environment => PolicySeverity::Medium,
        PolicyCategory::General => PolicySeverity::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn policy_parser_extracts_structured_rules() {
        let policy = parse_policy_rules(
            r#"# Policy

1. Avoid unwrap in request handlers.
- Validate external input before use.
- Command execution must be allowlisted.
"#,
        );

        assert_eq!(policy.len(), 3);
        assert_eq!(policy[0].category, PolicyCategory::PanicSafety);
        assert_eq!(policy[1].category, PolicyCategory::InputValidation);
        assert_eq!(policy[2].severity, PolicySeverity::High);
    }

    #[test]
    fn load_project_policy_uses_default_policy_path() {
        let dir = tempdir().unwrap();
        let policy_dir = dir.path().join(".mdx-rust");
        std::fs::create_dir_all(&policy_dir).unwrap();
        std::fs::write(policy_dir.join("policies.md"), "- Preserve error context.").unwrap();

        let policy = load_project_policy(dir.path(), None).unwrap().unwrap();

        assert_eq!(policy.schema_version, "0.4");
        assert_eq!(policy.rules[0].category, PolicyCategory::ErrorContext);
    }

    #[test]
    fn policy_parser_ignores_placeholders() {
        let policy = parse_policy_rules(
            r#"- ...
1. Never ...
2. Validate external inputs.
"#,
        );

        assert_eq!(policy.len(), 1);
        assert_eq!(policy[0].category, PolicyCategory::InputValidation);
    }
}
