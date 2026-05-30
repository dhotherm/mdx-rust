//! Agent-first repository context artifacts.
//!
//! These records help external coding agents orient in large Rust workspaces
//! before planning or mutating code. They are read-only by design.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RepoMapConfig {
    pub target: Option<PathBuf>,
    pub max_depth: usize,
    pub max_dirs: usize,
}

impl Default for RepoMapConfig {
    fn default() -> Self {
        Self {
            target: None,
            max_depth: 3,
            max_dirs: 80,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RepoMap {
    pub schema_version: String,
    pub root: PathBuf,
    pub target: PathBuf,
    pub summary: RepoMapSummary,
    pub directories: Vec<RepoMapDirectory>,
    pub key_files: Vec<RepoMapKeyFile>,
    pub instruction_files: Vec<RepoMapKeyFile>,
    pub noise_filter: NoiseFilter,
    pub agent_intake: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RepoMapSummary {
    pub total_directories_listed: usize,
    pub total_rust_files: usize,
    pub total_files: usize,
    pub detected_crates: usize,
    pub likely_generated_or_build_dirs: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RepoMapDirectory {
    pub path: PathBuf,
    pub role: String,
    pub total_files: usize,
    pub rust_files: usize,
    pub child_dirs: usize,
    pub notable_files: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RepoMapKeyFile {
    pub path: PathBuf,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NoiseFilter {
    pub schema_version: String,
    pub root: PathBuf,
    pub rules: Vec<NoiseFilterRule>,
    pub generated_artifacts: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NoiseFilterRule {
    pub pattern: String,
    pub reason: String,
    pub default_action: String,
    pub agents: Vec<String>,
}

pub fn build_repo_map(root: &Path, config: &RepoMapConfig) -> anyhow::Result<RepoMap> {
    let target = match &config.target {
        Some(target) => root.join(target),
        None => root.to_path_buf(),
    };
    if !target.exists() {
        anyhow::bail!("repo-map target does not exist: {}", target.display());
    }
    let scan_target = if target.is_file() {
        target.parent().unwrap_or(root).to_path_buf()
    } else {
        target.clone()
    };

    let max_depth = config.max_depth.max(1);
    let max_dirs = config.max_dirs.max(1);
    let noise_filter = build_noise_filter(root);
    let mut directories = Vec::new();
    collect_directories(root, &scan_target, 0, max_depth, max_dirs, &mut directories)?;

    let mut warnings = Vec::new();
    if target.is_file() {
        warnings.push(format!(
            "target is a file; repo-map scanned parent directory {}",
            relative_path(root, &scan_target).display()
        ));
    }
    if directories.len() >= max_dirs {
        warnings.push(format!(
            "directory listing stopped at max_dirs={max_dirs}; raise --max-dirs for a larger map"
        ));
    }

    let total_rust_files = directories.iter().map(|dir| dir.rust_files).sum();
    let total_files = directories.iter().map(|dir| dir.total_files).sum();
    let detected_crates = directories
        .iter()
        .filter(|dir| dir.notable_files.iter().any(|file| file == "Cargo.toml"))
        .count();
    let likely_generated_or_build_dirs = directories
        .iter()
        .filter(|dir| dir.role == "generated-or-build")
        .count();
    let summary = RepoMapSummary {
        total_directories_listed: directories.len(),
        total_rust_files,
        total_files,
        detected_crates,
        likely_generated_or_build_dirs,
    };

    Ok(RepoMap {
        schema_version: "1.0".to_string(),
        root: root.to_path_buf(),
        target,
        summary,
        directories,
        key_files: key_files(root),
        instruction_files: instruction_files(root),
        noise_filter,
        agent_intake: vec![
            "Read AGENTS.md and SAFETY_INVARIANTS.md before proposing source changes.".to_string(),
            "Run mdx-rust --json agent-contract before choosing commands.".to_string(),
            "Run mdx-rust --json scorecard <target> for safety, evidence, and next actions."
                .to_string(),
            "Respect noise_filter rules before searching or loading context.".to_string(),
            "Do not add --apply unless the human explicitly asks for mutation.".to_string(),
        ],
        warnings,
    })
}

pub fn build_noise_filter(root: &Path) -> NoiseFilter {
    NoiseFilter {
        schema_version: "1.0".to_string(),
        root: root.to_path_buf(),
        rules: default_noise_rules(),
        generated_artifacts: vec![
            PathBuf::from(".mdx-rust/agent-pack/noise-filter.json"),
            PathBuf::from(".mdx-rust/agent-pack/noise-filter.md"),
        ],
    }
}

pub fn noise_filter_markdown(filter: &NoiseFilter) -> String {
    let mut out = String::from("# mdx-rust Noise Filter\n\n");
    out.push_str(
        "Use these defaults before searching, reading, or summarizing the repository.\n\n",
    );
    for rule in &filter.rules {
        out.push_str(&format!(
            "- `{}`: {} ({})\n",
            rule.pattern, rule.reason, rule.default_action
        ));
    }
    out
}

fn collect_directories(
    root: &Path,
    path: &Path,
    depth: usize,
    max_depth: usize,
    max_dirs: usize,
    directories: &mut Vec<RepoMapDirectory>,
) -> anyhow::Result<()> {
    if directories.len() >= max_dirs {
        return Ok(());
    }
    if is_noise_path(root, path) && depth > 0 {
        directories.push(describe_directory(root, path, "generated-or-build")?);
        return Ok(());
    }

    let role = infer_directory_role(path);
    directories.push(describe_directory(root, path, &role)?);
    if depth >= max_depth {
        return Ok(());
    }

    let mut children = read_child_dirs(path)?;
    children.sort();
    for child in children {
        if directories.len() >= max_dirs {
            break;
        }
        collect_directories(root, &child, depth + 1, max_depth, max_dirs, directories)?;
    }
    Ok(())
}

fn describe_directory(root: &Path, path: &Path, role: &str) -> anyhow::Result<RepoMapDirectory> {
    let mut total_files = 0;
    let mut rust_files = 0;
    let mut child_dirs = 0;
    let mut notable_files = Vec::new();

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if file_type.is_dir() {
            child_dirs += 1;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        total_files += 1;
        if name.ends_with(".rs") {
            rust_files += 1;
        }
        if is_notable_file(&name) {
            notable_files.push(name);
        }
    }
    notable_files.sort();
    notable_files.dedup();

    let mut notes = Vec::new();
    if notable_files.iter().any(|file| file == "Cargo.toml") {
        notes.push("Rust crate boundary".to_string());
    }
    if role == "generated-or-build" {
        notes.push("Do not load unless explicitly debugging generated/build output".to_string());
    }
    if role == "tests" || path.ends_with("tests") {
        notes.push("Behavior evidence lives here".to_string());
    }

    Ok(RepoMapDirectory {
        path: relative_path(root, path),
        role: role.to_string(),
        total_files,
        rust_files,
        child_dirs,
        notable_files,
        notes,
    })
}

fn infer_directory_role(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if has_file(path, "Cargo.toml") {
        "crate".to_string()
    } else if name == "src" {
        "source".to_string()
    } else if name == "tests" {
        "tests".to_string()
    } else if name == "examples" {
        "examples".to_string()
    } else if name == "docs" {
        "docs".to_string()
    } else if is_noise_name(name) {
        "generated-or-build".to_string()
    } else {
        "support".to_string()
    }
}

fn read_child_dirs(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut children = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() && !file_type.is_symlink() {
            children.push(entry.path());
        }
    }
    Ok(children)
}

fn key_files(root: &Path) -> Vec<RepoMapKeyFile> {
    [
        ("Cargo.toml", "workspace manifest"),
        ("Cargo.lock", "locked dependency graph"),
        ("README.md", "human product overview"),
        ("SAFETY_INVARIANTS.md", "mutation safety contract"),
        ("ROADMAP.md", "release direction"),
        ("Justfile", "validation entrypoints"),
        ("deny.toml", "supply-chain policy"),
        ("clippy.toml", "lint policy"),
    ]
    .into_iter()
    .filter_map(|(path, role)| {
        let absolute = root.join(path);
        absolute.exists().then(|| RepoMapKeyFile {
            path: PathBuf::from(path),
            role: role.to_string(),
        })
    })
    .collect()
}

fn instruction_files(root: &Path) -> Vec<RepoMapKeyFile> {
    [
        ("AGENTS.md", "root coding-agent instructions"),
        ("CLAUDE.md", "Claude-specific instructions"),
        (".codex/skills/mdx-rust-evolution/SKILL.md", "Codex skill"),
        (".claude/skills/mdx-rust-evolution/SKILL.md", "Claude skill"),
        (".cursor/rules/mdx-rust-evolution.mdc", "Cursor rule"),
        (".mdx-rust/agent-pack/goosehints.md", "Goose hints"),
    ]
    .into_iter()
    .filter_map(|(path, role)| {
        let absolute = root.join(path);
        absolute.exists().then(|| RepoMapKeyFile {
            path: PathBuf::from(path),
            role: role.to_string(),
        })
    })
    .collect()
}

fn default_noise_rules() -> Vec<NoiseFilterRule> {
    let agents = vec![
        "codex".to_string(),
        "claude".to_string(),
        "cursor".to_string(),
        "aider".to_string(),
        "goose".to_string(),
        "generic".to_string(),
    ];
    [
        ("target/", "Rust build output is large and generated."),
        (".git/", "Git object storage is not source context."),
        (
            ".mdx-rust/",
            "mdx-rust artifacts are generated unless explicitly inspecting a run.",
        ),
        (
            "node_modules/",
            "Node dependency output is too noisy for Rust analysis.",
        ),
        ("dist/", "Distribution build output is generated."),
        ("build/", "Build output is generated or tool-owned."),
        (
            "coverage/",
            "Coverage reports are generated evidence artifacts.",
        ),
        ("*.profraw", "LLVM coverage profile output is generated."),
        ("*.lcov", "Coverage report output is generated."),
        (
            "*.snap",
            "Snapshot files should be loaded only when test behavior requires it.",
        ),
    ]
    .into_iter()
    .map(|(pattern, reason)| NoiseFilterRule {
        pattern: pattern.to_string(),
        reason: reason.to_string(),
        default_action: "exclude-from-default-context".to_string(),
        agents: agents.clone(),
    })
    .collect()
}

fn is_noise_path(root: &Path, path: &Path) -> bool {
    relative_path(root, path)
        .components()
        .any(|component| component.as_os_str().to_str().is_some_and(is_noise_name))
}

fn is_noise_name(name: &str) -> bool {
    matches!(
        name,
        "target" | ".git" | ".mdx-rust" | "node_modules" | "dist" | "build" | "coverage"
    )
}

fn is_notable_file(name: &str) -> bool {
    matches!(
        name,
        "Cargo.toml"
            | "Cargo.lock"
            | "README.md"
            | "AGENTS.md"
            | "CLAUDE.md"
            | "SAFETY_INVARIANTS.md"
            | "Justfile"
            | "deny.toml"
            | "clippy.toml"
    )
}

fn has_file(path: &Path, name: &str) -> bool {
    path.join(name).exists()
}

fn relative_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn repo_map_lists_crates_and_noise_rules() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[workspace]\n").unwrap();
        std::fs::write(dir.path().join("AGENTS.md"), "rules").unwrap();
        std::fs::create_dir_all(dir.path().join("crates/app/src")).unwrap();
        std::fs::write(dir.path().join("crates/app/Cargo.toml"), "[package]\n").unwrap();
        std::fs::write(
            dir.path().join("crates/app/src/lib.rs"),
            "pub fn app() {}\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("target/debug")).unwrap();

        let map = build_repo_map(dir.path(), &RepoMapConfig::default()).unwrap();

        assert_eq!(map.schema_version, "1.0");
        assert!(map.summary.detected_crates >= 2);
        assert!(map
            .instruction_files
            .iter()
            .any(|file| file.path == PathBuf::from("AGENTS.md")));
        assert!(map
            .noise_filter
            .rules
            .iter()
            .any(|rule| rule.pattern == "target/"));
    }

    #[test]
    fn repo_map_rejects_missing_target() {
        let dir = tempdir().unwrap();
        let error = build_repo_map(
            dir.path(),
            &RepoMapConfig {
                target: Some(PathBuf::from("missing")),
                ..RepoMapConfig::default()
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("target does not exist"));
    }

    #[test]
    fn repo_map_file_target_scans_parent_directory() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "pub fn lib() {}\n").unwrap();

        let map = build_repo_map(
            dir.path(),
            &RepoMapConfig {
                target: Some(PathBuf::from("src/lib.rs")),
                ..RepoMapConfig::default()
            },
        )
        .unwrap();

        assert_eq!(map.target, dir.path().join("src/lib.rs"));
        assert!(map
            .directories
            .iter()
            .any(|directory| directory.path == PathBuf::from("src")));
        assert!(map
            .warnings
            .iter()
            .any(|warning| warning.contains("target is a file")));
    }
}
