//! Conservative Rust hardening analysis for ordinary Rust modules.
//!
//! This module intentionally starts with high-confidence static patterns. It
//! can inspect normal Rust crates without requiring agent registration.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HardeningAnalysis {
    pub root: PathBuf,
    pub target: Option<PathBuf>,
    pub files_scanned: usize,
    pub findings: Vec<HardeningFinding>,
    pub changes: Vec<HardeningFileChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HardeningFinding {
    pub id: String,
    pub title: String,
    pub description: String,
    pub file: PathBuf,
    pub line: usize,
    pub strategy: HardeningStrategy,
    pub patchable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum HardeningStrategy {
    ResultUnwrapContext,
    ProcessExecutionReview,
    UnsafeReview,
    EnvAccessReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HardeningFileChange {
    pub file: PathBuf,
    pub old_content: String,
    pub new_content: String,
    pub strategy: HardeningStrategy,
    pub finding_ids: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, Copy)]
pub struct HardeningAnalyzeConfig<'a> {
    pub target: Option<&'a Path>,
    pub max_files: usize,
}

pub fn analyze_hardening(
    root: &Path,
    config: HardeningAnalyzeConfig<'_>,
) -> anyhow::Result<HardeningAnalysis> {
    let files = collect_rust_files(root, config.target)?;
    let mut findings = Vec::new();
    let mut changes = Vec::new();

    for file in files.iter().take(config.max_files) {
        let content = std::fs::read_to_string(file)?;
        let rel = relative_path(root, file);
        let function_ranges = find_function_ranges(&content);

        for (index, line) in content.lines().enumerate() {
            let line_no = index + 1;
            let pattern_line = line_without_comments_or_strings(line);
            let trimmed = pattern_line.trim();

            if trimmed.contains("Command::new(") || trimmed.contains("std::process::Command") {
                findings.push(HardeningFinding {
                    id: format!("process-execution:{}:{line_no}", rel.display()),
                    title: "Process execution surface".to_string(),
                    description:
                        "External process execution should have explicit input validation or allowlisting."
                            .to_string(),
                    file: rel.clone(),
                    line: line_no,
                    strategy: HardeningStrategy::ProcessExecutionReview,
                    patchable: false,
                });
            }

            if trimmed.contains("unsafe ") || trimmed == "unsafe" || trimmed.contains("unsafe{") {
                findings.push(HardeningFinding {
                    id: format!("unsafe-rust:{}:{line_no}", rel.display()),
                    title: "Unsafe Rust requires review".to_string(),
                    description:
                        "Unsafe code should be isolated and documented before automated edits touch it."
                            .to_string(),
                    file: rel.clone(),
                    line: line_no,
                    strategy: HardeningStrategy::UnsafeReview,
                    patchable: false,
                });
            }

            if trimmed.contains("std::env::var(") || trimmed.contains("env::var(") {
                findings.push(HardeningFinding {
                    id: format!("env-access:{}:{line_no}", rel.display()),
                    title: "Environment variable access".to_string(),
                    description:
                        "Environment-derived configuration should return contextual errors at boundaries."
                            .to_string(),
                    file: rel.clone(),
                    line: line_no,
                    strategy: HardeningStrategy::EnvAccessReview,
                    patchable: false,
                });
            }
        }

        if let Some(change) = build_result_context_change(root, file, &content, &function_ranges)? {
            for id in &change.finding_ids {
                if !findings.iter().any(|finding| &finding.id == id) {
                    let line = id
                        .rsplit(':')
                        .next()
                        .and_then(|line| line.parse::<usize>().ok())
                        .unwrap_or(1);
                    findings.push(HardeningFinding {
                        id: id.clone(),
                        title: "Panic-prone unwrap in anyhow Result function".to_string(),
                        description: "Replace unwrap/expect with anyhow Context and ? so failure is reported instead of panicking.".to_string(),
                        file: rel.clone(),
                        line,
                        strategy: HardeningStrategy::ResultUnwrapContext,
                        patchable: true,
                    });
                }
            }
            changes.push(change);
        }
    }

    Ok(HardeningAnalysis {
        root: root.to_path_buf(),
        target: config.target.map(Path::to_path_buf),
        files_scanned: files.len().min(config.max_files),
        findings,
        changes,
    })
}

fn build_result_context_change(
    root: &Path,
    file: &Path,
    content: &str,
    function_ranges: &[FunctionRange],
) -> anyhow::Result<Option<HardeningFileChange>> {
    let rel = relative_path(root, file);
    let mut lines: Vec<String> = content.lines().map(ToString::to_string).collect();
    let mut changed = false;
    let mut finding_ids = Vec::new();

    for range in function_ranges {
        if !range.returns_anyhow_result {
            continue;
        }

        for line_index in range.start_line.saturating_sub(1)..range.end_line.min(lines.len()) {
            let original = lines[line_index].clone();
            if original.trim_start().starts_with("//") {
                continue;
            }

            let mut rewritten = original.clone();
            if rewritten.contains(".unwrap()") {
                rewritten = rewritten.replace(
                    ".unwrap()",
                    &format!(".context(\"{} failed instead of panicking\")?", range.name),
                );
            }
            rewritten = replace_expect_calls(&rewritten);

            if rewritten != original {
                changed = true;
                lines[line_index] = rewritten;
                finding_ids.push(format!(
                    "unwrap-in-result:{}:{}",
                    rel.display(),
                    line_index + 1
                ));
            }
        }
    }

    if !changed {
        return Ok(None);
    }

    let mut new_content = lines.join("\n");
    if content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content = ensure_anyhow_context_import(&new_content);
    if syn::parse_file(&new_content).is_err() {
        return Ok(None);
    }

    Ok(Some(HardeningFileChange {
        file: rel,
        old_content: content.to_string(),
        new_content,
        strategy: HardeningStrategy::ResultUnwrapContext,
        finding_ids,
        description:
            "Replace panic-prone unwrap/expect calls in anyhow Result functions with Context and ?."
                .to_string(),
    }))
}

fn replace_expect_calls(line: &str) -> String {
    let mut output = String::new();
    let mut rest = line;
    while let Some(start) = rest.find(".expect(\"") {
        let (before, after_start) = rest.split_at(start);
        output.push_str(before);
        let msg_start = ".expect(\"".len();
        let after_msg_start = &after_start[msg_start..];
        if let Some(end) = after_msg_start.find("\")") {
            let message = &after_msg_start[..end];
            output.push_str(&format!(".context(\"{}\")?", escape_string(message)));
            rest = &after_msg_start[end + 2..];
        } else {
            output.push_str(after_start);
            rest = "";
        }
    }
    output.push_str(rest);
    output
}

fn escape_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn line_without_comments_or_strings(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if !in_string && ch == '/' && chars.peek() == Some(&'/') {
            break;
        }

        if ch == '"' && !escaped {
            in_string = !in_string;
            output.push(' ');
            continue;
        }

        if in_string {
            escaped = ch == '\\' && !escaped;
            output.push(' ');
            continue;
        }

        escaped = false;
        output.push(ch);
    }

    output
}

fn ensure_anyhow_context_import(content: &str) -> String {
    if content.contains("anyhow::Context") || content.contains("Context,") {
        return content.to_string();
    }

    let mut lines: Vec<&str> = content.lines().collect();
    let insert_at = lines
        .iter()
        .position(|line| !line.starts_with("#![") && !line.trim().is_empty())
        .unwrap_or(0);
    lines.insert(insert_at, "use anyhow::Context;");
    let mut result = lines.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

#[derive(Debug)]
struct FunctionRange {
    name: String,
    start_line: usize,
    end_line: usize,
    returns_anyhow_result: bool,
}

fn find_function_ranges(content: &str) -> Vec<FunctionRange> {
    let lines: Vec<&str> = content.lines().collect();
    let has_anyhow_result_alias =
        content.contains("use anyhow::Result") || content.contains("use anyhow::{Result");
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if !line.contains("fn ") {
            index += 1;
            continue;
        }

        let mut signature = line.to_string();
        let start_line = index + 1;
        let mut open_line = index;
        while !signature.contains('{') && open_line + 1 < lines.len() {
            open_line += 1;
            signature.push(' ');
            signature.push_str(lines[open_line]);
        }

        if !signature.contains('{') {
            index += 1;
            continue;
        }

        let Some(name) = function_name(&signature) else {
            index += 1;
            continue;
        };

        let mut depth = 0isize;
        let mut end_line = open_line + 1;
        for (body_index, body_line) in lines.iter().enumerate().skip(open_line) {
            depth += body_line.matches('{').count() as isize;
            depth -= body_line.matches('}').count() as isize;
            end_line = body_index + 1;
            if depth == 0 {
                break;
            }
        }

        let returns_anyhow_result = signature.contains("-> anyhow::Result")
            || (has_anyhow_result_alias && signature.contains("-> Result<"));
        ranges.push(FunctionRange {
            name,
            start_line,
            end_line,
            returns_anyhow_result,
        });
        index = end_line;
    }
    ranges
}

fn function_name(signature: &str) -> Option<String> {
    let rest = signature.split_once("fn ")?.1;
    let name = rest
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .next()?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn collect_rust_files(root: &Path, target: Option<&Path>) -> anyhow::Result<Vec<PathBuf>> {
    let scan_root = target
        .map(|path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            }
        })
        .unwrap_or_else(|| root.to_path_buf());
    if !scan_root.starts_with(root) {
        anyhow::bail!("hardening target is outside root: {}", scan_root.display());
    }

    if scan_root.is_file() {
        return Ok(if scan_root.extension().is_some_and(|ext| ext == "rs") {
            vec![scan_root]
        } else {
            Vec::new()
        });
    }

    let mut files = Vec::new();
    for result in ignore::WalkBuilder::new(scan_root)
        .hidden(false)
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            !matches!(
                name.as_ref(),
                "target" | ".git" | ".worktrees" | ".mdx-rust"
            )
        })
        .build()
    {
        let entry = result?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn relative_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn hardening_rewrites_unwrap_in_anyhow_result_function() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"pub fn load() -> anyhow::Result<String> {
    let value = std::fs::read_to_string("config.toml").unwrap();
    Ok(value)
}
"#,
        )
        .unwrap();

        let analysis = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
            },
        )
        .unwrap();

        assert_eq!(analysis.changes.len(), 1);
        let change = &analysis.changes[0];
        assert!(change.new_content.contains("use anyhow::Context;"));
        assert!(change
            .new_content
            .contains(".context(\"load failed instead of panicking\")?"));
        assert!(syn::parse_file(&change.new_content).is_ok());
    }

    #[test]
    fn hardening_does_not_rewrite_plain_result_without_anyhow_alias() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"pub fn load() -> Result<String, std::io::Error> {
    let value = std::fs::read_to_string("config.toml").unwrap();
    Ok(value)
}
"#,
        )
        .unwrap();

        let analysis = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
            },
        )
        .unwrap();

        assert!(analysis.changes.is_empty());
    }

    #[test]
    fn hardening_does_not_flag_patterns_inside_strings_or_comments() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"pub fn describe() -> &'static str {
    // Command::new("ignored")
    "unsafe std::process::Command env::var("
}
"#,
        )
        .unwrap();

        let analysis = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
            },
        )
        .unwrap();

        assert!(analysis.findings.is_empty(), "{:?}", analysis.findings);
    }
}
