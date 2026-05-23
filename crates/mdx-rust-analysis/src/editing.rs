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
/// Returns the path to the worktree.
pub fn create_worktree(agent_path: &Path, worktree_name: &str) -> anyhow::Result<PathBuf> {
    let worktree_path = agent_path.join(".worktrees").join(worktree_name);
    std::fs::create_dir_all(worktree_path.parent().unwrap())?;

    let status = Command::new("git")
        .current_dir(agent_path)
        .args(["worktree", "add", worktree_path.to_str().unwrap(), "HEAD"])
        .status()?;

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to create git worktree"));
    }

    Ok(worktree_path)
}

/// Apply a unified diff patch inside a directory.
pub fn apply_patch(dir: &Path, patch: &str) -> anyhow::Result<()> {
    let patch_file = dir.join(".mdx_patch.diff");
    std::fs::write(&patch_file, patch)?;

    let status = Command::new("git")
        .current_dir(dir)
        .args(["apply", "--reject", "--whitespace=fix", patch_file.to_str().unwrap()])
        .status()?;

    let _ = std::fs::remove_file(&patch_file);

    if !status.success() {
        return Err(anyhow::anyhow!("Failed to apply patch"));
    }

    Ok(())
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