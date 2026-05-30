//! Lightweight Rust contract scanner.
//!
//! This module records documented preconditions, postconditions, invariants,
//! safety notes, panic docs, and assertion hints. It is intentionally read-only.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContractScanConfig {
    pub target: Option<PathBuf>,
    pub max_files: usize,
}

impl Default for ContractScanConfig {
    fn default() -> Self {
        Self {
            target: None,
            max_files: 250,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContractRun {
    pub schema_version: String,
    pub root: PathBuf,
    pub target: PathBuf,
    pub summary: ContractSummary,
    pub functions: Vec<ContractFunction>,
    pub recommendations: Vec<ContractRecommendation>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContractSummary {
    pub scanned_files: usize,
    pub function_count: usize,
    pub public_function_count: usize,
    pub functions_with_contracts: usize,
    pub public_functions_missing_contracts: usize,
    pub assertion_count: usize,
    pub test_context_functions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContractFunction {
    pub file: PathBuf,
    pub line: usize,
    pub name: String,
    pub visibility: String,
    pub requires: Vec<String>,
    pub ensures: Vec<String>,
    pub invariants: Vec<String>,
    pub safety_notes: Vec<String>,
    pub panics: Vec<String>,
    pub assertion_count: usize,
    pub test_context: bool,
    pub contract_grade: ContractGrade,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum ContractGrade {
    Strong,
    Documented,
    AssertionBacked,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContractRecommendation {
    pub file: PathBuf,
    pub line: usize,
    pub severity: String,
    pub message: String,
}

/// Requires: `root` points at the workspace that owns the optional scan target.
/// Ensures: returns a read-only contract report without writing source files.
pub fn scan_contracts(root: &Path, config: &ContractScanConfig) -> anyhow::Result<ContractRun> {
    let target = match &config.target {
        Some(target) => root.join(target),
        None => root.to_path_buf(),
    };
    if !target.exists() {
        anyhow::bail!("contracts target does not exist: {}", target.display());
    }

    let mut warnings = Vec::new();
    let files = rust_files(root, &target, config.max_files.max(1), &mut warnings)?;
    let mut functions = Vec::new();
    for file in &files {
        functions.extend(scan_file(root, file)?);
    }

    let mut recommendations = Vec::new();
    for function in &functions {
        if function.visibility == "public"
            && function.contract_grade == ContractGrade::Missing
            && !function.test_context
        {
            recommendations.push(ContractRecommendation {
                file: function.file.clone(),
                line: function.line,
                severity: "medium".to_string(),
                message: format!(
                    "public function `{}` has no visible contract docs or assertion hints",
                    function.name
                ),
            });
        }
    }

    let public_function_count = functions
        .iter()
        .filter(|function| function.visibility == "public")
        .count();
    let functions_with_contracts = functions
        .iter()
        .filter(|function| {
            matches!(
                function.contract_grade,
                ContractGrade::Strong | ContractGrade::Documented | ContractGrade::AssertionBacked
            )
        })
        .count();
    let public_functions_missing_contracts = functions
        .iter()
        .filter(|function| {
            function.visibility == "public"
                && function.contract_grade == ContractGrade::Missing
                && !function.test_context
        })
        .count();
    let assertion_count = functions
        .iter()
        .map(|function| function.assertion_count)
        .sum();
    let test_context_functions = functions
        .iter()
        .filter(|function| function.test_context)
        .count();

    Ok(ContractRun {
        schema_version: "1.0".to_string(),
        root: root.to_path_buf(),
        target,
        summary: ContractSummary {
            scanned_files: files.len(),
            function_count: functions.len(),
            public_function_count,
            functions_with_contracts,
            public_functions_missing_contracts,
            assertion_count,
            test_context_functions,
        },
        functions,
        recommendations,
        warnings,
    })
}

fn scan_file(root: &Path, file: &Path) -> anyhow::Result<Vec<ContractFunction>> {
    let source = std::fs::read_to_string(file)?;
    let lines: Vec<&str> = source.lines().collect();
    let function_lines = function_line_indices(&lines);
    let mut functions = Vec::new();

    for (position, line_index) in function_lines.iter().enumerate() {
        let line = lines[*line_index];
        let Some((name, visibility)) = parse_function_line(line) else {
            continue;
        };
        let docs = preceding_doc_lines(&lines, *line_index);
        let sections = contract_sections(&docs);
        let end = function_lines
            .get(position + 1)
            .copied()
            .unwrap_or(lines.len());
        let assertion_count = lines[*line_index..end]
            .iter()
            .filter(|line| contains_assertion(line))
            .count();
        let test_context =
            is_test_context(root, file) || docs.iter().any(|line| line.contains("# Examples"));
        let contract_grade = contract_grade(&sections, assertion_count);

        functions.push(ContractFunction {
            file: relative_path(root, file),
            line: *line_index + 1,
            name,
            visibility,
            requires: sections.requires,
            ensures: sections.ensures,
            invariants: sections.invariants,
            safety_notes: sections.safety_notes,
            panics: sections.panics,
            assertion_count,
            test_context,
            contract_grade,
        });
    }

    Ok(functions)
}

#[derive(Default)]
struct ContractSections {
    requires: Vec<String>,
    ensures: Vec<String>,
    invariants: Vec<String>,
    safety_notes: Vec<String>,
    panics: Vec<String>,
}

fn contract_sections(docs: &[String]) -> ContractSections {
    let mut sections = ContractSections::default();
    for doc in docs {
        let trimmed = doc.trim();
        let lower = trimmed.to_ascii_lowercase();
        if let Some(value) = strip_contract_prefix(trimmed, &lower, &["requires:", "precondition:"])
        {
            sections.requires.push(value.to_string());
        } else if let Some(value) =
            strip_contract_prefix(trimmed, &lower, &["ensures:", "postcondition:"])
        {
            sections.ensures.push(value.to_string());
        } else if let Some(value) = strip_contract_prefix(trimmed, &lower, &["invariant:"]) {
            sections.invariants.push(value.to_string());
        } else if let Some(value) = strip_contract_prefix(trimmed, &lower, &["safety:"]) {
            sections.safety_notes.push(value.to_string());
        } else if let Some(value) = strip_contract_prefix(trimmed, &lower, &["panics:"]) {
            sections.panics.push(value.to_string());
        }
    }
    sections
}

fn strip_contract_prefix<'a>(original: &'a str, lower: &str, prefixes: &[&str]) -> Option<&'a str> {
    prefixes.iter().find_map(|prefix| {
        lower
            .starts_with(prefix)
            .then(|| original[prefix.len()..].trim())
    })
}

fn contract_grade(sections: &ContractSections, assertion_count: usize) -> ContractGrade {
    let documented_count = sections.requires.len()
        + sections.ensures.len()
        + sections.invariants.len()
        + sections.safety_notes.len()
        + sections.panics.len();
    if documented_count >= 2 {
        ContractGrade::Strong
    } else if documented_count == 1 {
        ContractGrade::Documented
    } else if assertion_count > 0 {
        ContractGrade::AssertionBacked
    } else {
        ContractGrade::Missing
    }
}

fn function_line_indices(lines: &[&str]) -> Vec<usize> {
    let mut indices = Vec::new();
    let mut in_block_comment = false;
    let mut raw_string_hashes: Option<usize> = None;

    for (index, line) in lines.iter().enumerate() {
        if let Some(hashes) = raw_string_hashes {
            if line.contains(&raw_string_end_marker(hashes)) {
                raw_string_hashes = None;
            }
            continue;
        }

        let trimmed = line.trim_start();
        if in_block_comment {
            if trimmed.contains("*/") {
                in_block_comment = false;
            }
            continue;
        }
        if trimmed.starts_with("/*") {
            if !trimmed.contains("*/") {
                in_block_comment = true;
            }
            continue;
        }
        if let Some(hashes) = raw_string_start_hashes(line) {
            if !raw_string_closes_on_line(line, hashes) {
                raw_string_hashes = Some(hashes);
            }
            continue;
        }

        if parse_function_line(line).is_some() {
            indices.push(index);
        }
    }

    indices
}

fn parse_function_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return None;
    }
    let visibility = if trimmed.starts_with("pub ") || trimmed.starts_with("pub(") {
        "public"
    } else {
        "private"
    };
    let fn_pos = trimmed.find("fn ")?;
    let before_fn = &trimmed[..fn_pos];
    if !before_fn.is_empty()
        && !before_fn
            .split_whitespace()
            .all(|token| matches!(token, "pub" | "async" | "unsafe" | "const" | "extern"))
        && !before_fn.trim_start().starts_with("pub(")
    {
        return None;
    }
    let after_fn = &trimmed[fn_pos + 3..];
    let name: String = after_fn
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect();
    (!name.is_empty()).then(|| (name, visibility.to_string()))
}

fn preceding_doc_lines(lines: &[&str], line_index: usize) -> Vec<String> {
    let mut docs = Vec::new();
    let mut index = line_index;
    while index > 0 {
        index -= 1;
        let trimmed = lines[index].trim_start();
        if let Some(doc) = trimmed.strip_prefix("///") {
            docs.push(doc.trim().to_string());
            continue;
        }
        if trimmed.starts_with("#[") || trimmed.is_empty() {
            continue;
        }
        break;
    }
    docs.reverse();
    docs
}

fn contains_assertion(line: &&str) -> bool {
    let trimmed = line.trim_start();
    !trimmed.starts_with("//")
        && (trimmed.contains("assert!(")
            || trimmed.contains("debug_assert!(")
            || trimmed.contains("assert_eq!(")
            || trimmed.contains("assert_ne!("))
}

fn raw_string_start_hashes(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'r' {
            let mut cursor = index + 1;
            let mut hashes = 0;
            while cursor < bytes.len() && bytes[cursor] == b'#' {
                hashes += 1;
                cursor += 1;
            }
            if cursor < bytes.len() && bytes[cursor] == b'"' {
                return Some(hashes);
            }
        }
        index += 1;
    }
    None
}

fn raw_string_closes_on_line(line: &str, hashes: usize) -> bool {
    let Some(start) = line.find(&raw_string_start_marker(hashes)) else {
        return false;
    };
    line[start + raw_string_start_marker(hashes).len()..].contains(&raw_string_end_marker(hashes))
}

fn raw_string_start_marker(hashes: usize) -> String {
    format!("r{}\"", "#".repeat(hashes))
}

fn raw_string_end_marker(hashes: usize) -> String {
    format!("\"{}", "#".repeat(hashes))
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
    if files.len() >= max_files {
        return Ok(());
    }
    if is_noise_path(root, dir) && dir != root {
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
                    "contract scan stopped at max_files={max_files}; raise --max-files for a larger scan"
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
    fn contract_scan_finds_doc_contracts_and_missing_public_contracts() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/lib.rs"),
            r#"/// Requires: input is non-empty
/// Ensures: returns a trimmed copy
pub fn normalize(input: &str) -> String {
    debug_assert!(!input.is_empty());
    input.trim().to_string()
}

pub fn undocumented(value: usize) -> usize {
    value + 1
}
"#,
        )
        .unwrap();

        let run = scan_contracts(
            dir.path(),
            &ContractScanConfig {
                target: Some(PathBuf::from("src/lib.rs")),
                ..ContractScanConfig::default()
            },
        )
        .unwrap();

        assert_eq!(run.summary.function_count, 2);
        assert_eq!(run.summary.functions_with_contracts, 1);
        assert_eq!(run.summary.public_functions_missing_contracts, 1);
        assert!(run
            .functions
            .iter()
            .any(|function| function.name == "normalize"
                && function.contract_grade == ContractGrade::Strong));
        assert!(run
            .recommendations
            .iter()
            .any(|recommendation| recommendation.message.contains("undocumented")));
    }

    #[test]
    fn contract_scan_ignores_raw_string_fixtures() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/lib.rs"),
            r##"pub fn real() {}

const FIXTURE: &str = r#"
pub fn fake_inside_fixture() {}
"#;
"##,
        )
        .unwrap();

        let run = scan_contracts(
            dir.path(),
            &ContractScanConfig {
                target: Some(PathBuf::from("src/lib.rs")),
                ..ContractScanConfig::default()
            },
        )
        .unwrap();

        assert_eq!(run.summary.function_count, 1);
        assert!(run.functions.iter().any(|function| function.name == "real"));
        assert!(!run
            .functions
            .iter()
            .any(|function| function.name == "fake_inside_fixture"));
    }
}
