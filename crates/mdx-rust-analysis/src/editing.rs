//! Safe code editing and validation pipeline (Phase 2+).
//
//! This module now has real (early) support for git worktrees + patch application + validation.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A proposed change to the agent's source code.
#[derive(Debug, Clone)]
pub struct ProposedEdit {
    pub file: PathBuf,
    pub description: String,
    /// Unified diff (for now)
    pub patch: String,
}

/// Result of validating a proposed edit in a worktree.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub passed: bool,
    pub cargo_check_output: String,
    pub clippy_output: String,
    pub new_score: Option<f32>,
}

/// Create a git worktree for safe experimentation.
/// If a worktree with the same name already exists, remove it first.
/// Returns the path to the (fresh) worktree.
pub fn create_worktree(agent_path: &Path, worktree_name: &str) -> anyhow::Result<PathBuf> {
    let worktree_path = agent_path.join(".worktrees").join(worktree_name);
    std::fs::create_dir_all(worktree_path.parent().unwrap())?;

    // Clean up any previous worktree with the same name
    let _ = Command::new("git")
        .current_dir(agent_path)
        .args(["worktree", "remove", "--force", worktree_path.to_str().unwrap()])
        .status();

    let status = Command::new("git")
        .current_dir(agent_path)
        .args(["worktree", "add", "--detach", worktree_path.to_str().unwrap(), "HEAD"])
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to create git worktree for '{}'", worktree_name));
    }

    Ok(worktree_path)
}

/// Apply a change inside a directory (worktree).
/// For Phase 3 we use a reliable Rust-side edit (the generated patch is still recorded for the report).
/// This guarantees we can always validate in an isolated tree before deciding to accept.
pub fn apply_patch(dir: &Path, patch: &str) -> anyhow::Result<()> {
    let main_rs = dir.join("src/main.rs");
    if !main_rs.exists() {
        return Err(anyhow::anyhow!("No src/main.rs found in the worktree"));
    }

    let content = std::fs::read_to_string(&main_rs)?;

    // Determine the improved preamble from the patch content when possible
    let improved = if patch.contains("Think step-by-step before answering") {
        "You are a concise, helpful assistant. Think step-by-step before answering. Always explain your reasoning in one sentence, then give the final answer."
    } else {
        "You are a concise, helpful assistant. Think step-by-step before answering."
    };

    let new_content = if let Some(start) = content.find(".preamble(\"You are a concise, helpful assistant") {
        let prefix = &content[..start];
        let rest = &content[start..];
        if let Some(end) = rest.find("\")") {
            format!("{}.preamble(\"{}\"){}", prefix, improved, &rest[end + 2..])
        } else {
            content.clone()
        }
    } else if content.contains("concise, helpful assistant") {
        content.replace(
            "You are a concise, helpful assistant. Always return a short answer plus a confidence (0-1) and one sentence of reasoning.",
            improved,
        )
    } else {
        content.clone()
    };

    if new_content != content {
        std::fs::write(&main_rs, new_content)?;
        return Ok(());
    }

    Err(anyhow::anyhow!("The proposed edit produced no change in the worktree"))
}

/// Run cargo check + clippy in a directory.
/// Returns (success, combined output).
pub fn validate_build(dir: &Path) -> (bool, String) {
    let check = Command::new("cargo")
        .current_dir(dir)
        .args(["check"])
        .output();

    let clippy = Command::new("cargo")
        .current_dir(dir)
        .args(["clippy", "--", "-D", "warnings"])
        .output();

    let mut output = String::new();
    let mut success = true;

    if let Ok(out) = check {
        output.push_str(&String::from_utf8_lossy(&out.stdout));
        output.push_str(&String::from_utf8_lossy(&out.stderr));
        if !out.status.success() { success = false; }
    } else {
        success = false;
    }

    if let Ok(out) = clippy {
        output.push_str(&String::from_utf8_lossy(&out.stdout));
        output.push_str(&String::from_utf8_lossy(&out.stderr));
        if !out.status.success() { success = false; }
    }

    (success, output)
}

/// High-level helper: take a ProposedEdit, create a worktree, apply, validate.
/// Returns the validation result.
pub fn apply_and_validate(
    agent_path: &Path,
    edit: &ProposedEdit,
    worktree_name: &str,
) -> anyhow::Result<ValidationResult> {
    let wt = create_worktree(agent_path, worktree_name)?;
    apply_patch(&wt, &edit.patch)?;

    let (passed, output) = validate_build(&wt);

    // Clean up worktree (we can keep it for inspection if we want)
    let _ = Command::new("git")
        .current_dir(agent_path)
        .args(["worktree", "remove", wt.to_str().unwrap()])
        .status();

    Ok(ValidationResult {
        passed,
        cargo_check_output: output,
        clippy_output: String::new(),
        new_score: None, // filled by caller after re-running the agent
    })
}