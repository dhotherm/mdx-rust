//! Conservative refactor planning analysis for Rust modules.
//!
//! This module is intentionally plan-only. It summarizes module shape and
//! likely refactor pressure without producing edits.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefactorAnalysis {
    pub root: PathBuf,
    pub target: Option<PathBuf>,
    pub files_scanned: usize,
    pub files: Vec<RefactorFileSummary>,
    pub module_edges: Vec<ModuleEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefactorFileSummary {
    pub file: PathBuf,
    pub line_count: usize,
    pub function_count: usize,
    pub public_item_count: usize,
    pub largest_function_lines: usize,
    pub has_tests: bool,
    pub public_items: Vec<PublicItemSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PublicItemSummary {
    pub kind: String,
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModuleEdge {
    pub from: PathBuf,
    pub to: String,
    pub line: usize,
    pub kind: ModuleEdgeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum ModuleEdgeKind {
    ModDeclaration,
    CrateUse,
    SuperUse,
}

#[derive(Debug, Clone, Copy)]
pub struct RefactorAnalyzeConfig<'a> {
    pub target: Option<&'a Path>,
    pub max_files: usize,
}

pub fn analyze_refactor(
    root: &Path,
    config: RefactorAnalyzeConfig<'_>,
) -> anyhow::Result<RefactorAnalysis> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let files = collect_rust_files(&root, config.target)?;
    let mut summaries = Vec::new();
    let mut module_edges = Vec::new();

    for file in files.iter().take(config.max_files) {
        let content = std::fs::read_to_string(file)?;
        let rel = relative_path(&root, file);
        module_edges.extend(find_module_edges(&rel, &content));
        summaries.push(summarize_file(&rel, &content));
    }

    Ok(RefactorAnalysis {
        root,
        target: config.target.map(Path::to_path_buf),
        files_scanned: summaries.len(),
        files: summaries,
        module_edges,
    })
}

fn summarize_file(file: &Path, content: &str) -> RefactorFileSummary {
    let line_count = content.lines().count();
    let has_tests = content.contains("#[cfg(test)]") || content.contains("#[test]");
    let function_ranges = find_function_ranges(content);
    let largest_function_lines = function_ranges
        .iter()
        .map(|range| range.line_count)
        .max()
        .unwrap_or(0);
    let public_items = public_items(content);

    RefactorFileSummary {
        file: file.to_path_buf(),
        line_count,
        function_count: function_ranges.len(),
        public_item_count: public_items.len(),
        largest_function_lines,
        has_tests,
        public_items,
    }
}

fn public_items(content: &str) -> Vec<PublicItemSummary> {
    let parsed = match syn::parse_file(content) {
        Ok(parsed) => parsed,
        Err(_) => return Vec::new(),
    };

    parsed
        .items
        .iter()
        .filter_map(|item| public_item_summary(item, content))
        .collect()
}

fn public_item_summary(item: &syn::Item, content: &str) -> Option<PublicItemSummary> {
    let (kind, name, token) = match item {
        syn::Item::Const(item) if is_public(&item.vis) => (
            "const",
            item.ident.to_string(),
            format!("const {}", item.ident),
        ),
        syn::Item::Enum(item) if is_public(&item.vis) => (
            "enum",
            item.ident.to_string(),
            format!("enum {}", item.ident),
        ),
        syn::Item::Fn(item) if is_public(&item.vis) => (
            "fn",
            item.sig.ident.to_string(),
            format!("fn {}", item.sig.ident),
        ),
        syn::Item::Struct(item) if is_public(&item.vis) => (
            "struct",
            item.ident.to_string(),
            format!("struct {}", item.ident),
        ),
        syn::Item::Trait(item) if is_public(&item.vis) => (
            "trait",
            item.ident.to_string(),
            format!("trait {}", item.ident),
        ),
        syn::Item::Type(item) if is_public(&item.vis) => (
            "type",
            item.ident.to_string(),
            format!("type {}", item.ident),
        ),
        syn::Item::Mod(item) if is_public(&item.vis) => {
            ("mod", item.ident.to_string(), format!("mod {}", item.ident))
        }
        _ => return None,
    };

    Some(PublicItemSummary {
        kind: kind.to_string(),
        name,
        line: line_for_token(content, &token),
    })
}

fn is_public(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

#[derive(Debug)]
struct FunctionRange {
    line_count: usize,
}

fn find_function_ranges(content: &str) -> Vec<FunctionRange> {
    let lines: Vec<&str> = content.lines().collect();
    let mut ranges = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let trimmed = lines[index].trim_start();
        let is_fn = trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("pub(crate) fn ")
            || trimmed.starts_with("pub(super) fn ")
            || trimmed.starts_with("async fn ")
            || trimmed.starts_with("pub async fn ");
        if !is_fn {
            index += 1;
            continue;
        }

        let mut brace_depth: isize = 0;
        let mut saw_open = false;
        let mut end_index = index;
        for (offset, line) in lines[index..].iter().enumerate() {
            let code = line_without_strings(line);
            brace_depth += code.matches('{').count() as isize;
            if code.contains('{') {
                saw_open = true;
            }
            brace_depth -= code.matches('}').count() as isize;
            end_index = index + offset;
            if saw_open && brace_depth <= 0 {
                break;
            }
        }

        ranges.push(FunctionRange {
            line_count: end_index.saturating_sub(index) + 1,
        });
        index = end_index.saturating_add(1);
    }

    ranges
}

fn find_module_edges(file: &Path, content: &str) -> Vec<ModuleEdge> {
    let mut edges = Vec::new();
    for (index, line) in content.lines().enumerate() {
        let line_no = index + 1;
        let trimmed = line.trim_start();
        if let Some(rest) = module_declaration_rest(trimmed) {
            let module = rest
                .trim_end_matches(';')
                .split_whitespace()
                .next()
                .unwrap_or_default();
            if !module.is_empty() {
                edges.push(ModuleEdge {
                    from: file.to_path_buf(),
                    to: module.to_string(),
                    line: line_no,
                    kind: ModuleEdgeKind::ModDeclaration,
                });
            }
        }

        if let Some(rest) = trimmed.strip_prefix("use crate::") {
            edges.push(ModuleEdge {
                from: file.to_path_buf(),
                to: rest.trim_end_matches(';').to_string(),
                line: line_no,
                kind: ModuleEdgeKind::CrateUse,
            });
        } else if let Some(rest) = trimmed.strip_prefix("use super::") {
            edges.push(ModuleEdge {
                from: file.to_path_buf(),
                to: rest.trim_end_matches(';').to_string(),
                line: line_no,
                kind: ModuleEdgeKind::SuperUse,
            });
        }
    }
    edges
}

fn module_declaration_rest(trimmed: &str) -> Option<&str> {
    trimmed
        .strip_prefix("mod ")
        .or_else(|| trimmed.strip_prefix("pub mod "))
        .or_else(|| trimmed.strip_prefix("pub(crate) mod "))
        .or_else(|| trimmed.strip_prefix("pub(super) mod "))
}

fn line_for_token(content: &str, token: &str) -> usize {
    content
        .lines()
        .position(|line| line.contains(token))
        .map(|index| index + 1)
        .unwrap_or(1)
}

fn line_without_strings(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut in_string = false;
    let mut escaped = false;
    for ch in line.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            output.push(' ');
        } else if ch == '"' {
            in_string = true;
            output.push(' ');
        } else {
            output.push(ch);
        }
    }
    output
}

fn collect_rust_files(root: &Path, target: Option<&Path>) -> anyhow::Result<Vec<PathBuf>> {
    let requested_scan_root = target
        .map(|path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            }
        })
        .unwrap_or_else(|| root.to_path_buf());
    if target.is_some() && !requested_scan_root.exists() {
        anyhow::bail!(
            "refactor target does not exist: {}",
            requested_scan_root.display()
        );
    }
    let scan_root = requested_scan_root
        .canonicalize()
        .unwrap_or(requested_scan_root);
    if !scan_root.starts_with(root) {
        anyhow::bail!("refactor target is outside root: {}", scan_root.display());
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
    fn refactor_analysis_summarizes_public_api_and_modules() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"pub mod api;
use crate::api::Handler;

pub struct Config {
    value: String,
}

pub fn load() -> anyhow::Result<String> {
    Ok(String::new())
}
"#,
        )
        .unwrap();

        let analysis = analyze_refactor(
            dir.path(),
            RefactorAnalyzeConfig {
                target: Some(Path::new("src/lib.rs")),
                max_files: 10,
            },
        )
        .unwrap();

        assert_eq!(analysis.files_scanned, 1);
        assert_eq!(analysis.files[0].public_item_count, 3);
        assert!(analysis.files[0]
            .public_items
            .iter()
            .any(|item| item.name == "load"));
        assert_eq!(analysis.module_edges.len(), 2);
    }

    #[test]
    fn refactor_analysis_rejects_missing_target() {
        let dir = tempdir().unwrap();
        let err = analyze_refactor(
            dir.path(),
            RefactorAnalyzeConfig {
                target: Some(Path::new("src/missing.rs")),
                max_files: 10,
            },
        )
        .unwrap_err();

        assert!(err.to_string().contains("refactor target does not exist"));
    }
}
