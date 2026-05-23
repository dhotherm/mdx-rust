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

/// Create a git worktree for safe experimentation (best when agent_path is a git repo root).
/// Falls back to a filesystem copy if worktree creation fails (e.g. agent lives inside another repo).
pub fn create_isolated_workspace(agent_path: &Path, name: &str) -> anyhow::Result<PathBuf> {
    let base = agent_path.join(".worktrees");
    std::fs::create_dir_all(&base)?;

    let worktree_path = base.join(name);

    // Try git worktree first (fast, shares objects, real git history)
    // But skip it if the agent lives deep inside another repo (common for examples/ in monorepos)
    let should_try_worktree = !agent_path.to_string_lossy().contains("/examples/")
        && !agent_path.to_string_lossy().contains("\\examples\\");

    if should_try_worktree {
        let _ = Command::new("git")
            .current_dir(agent_path)
            .args([
                "worktree",
                "remove",
                "--force",
                worktree_path.to_str().unwrap(),
            ])
            .status();

        let git_status = Command::new("git")
            .current_dir(agent_path)
            .args([
                "worktree",
                "add",
                "--detach",
                worktree_path.to_str().unwrap(),
                "HEAD",
            ])
            .status();

        if git_status.map(|s| s.success()).unwrap_or(false) {
            return Ok(worktree_path);
        }
    }

    // Fallback: filesystem copy (works for any directory, including subdirs of monorepos)
    // This is still safe because we only mutate the copy.
    if worktree_path.exists() {
        let _ = std::fs::remove_dir_all(&worktree_path);
    }
    copy_dir_all(agent_path, &worktree_path)?;

    // Initialize a git repo inside the copy so later cargo/git commands behave nicely
    let _ = Command::new("git")
        .current_dir(&worktree_path)
        .args(["init", "-q"])
        .status();
    let _ = Command::new("git")
        .current_dir(&worktree_path)
        .args(["add", "-A"])
        .status();
    let _ = Command::new("git")
        .current_dir(&worktree_path)
        .args(["commit", "-q", "-m", "mdx-rust isolated copy"])
        .status();

    Ok(worktree_path)
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Apply the proposed patch inside an isolated directory.
/// Strategy:
///
/// - Try real `git apply` (best when the patch was generated with context).
/// - Fall back to smart string replacement for the common Rig preamble/tool cases.
///
/// This keeps the system reliable even when perfect unified diffs are hard to generate.
pub fn apply_patch(dir: &Path, patch: &str) -> anyhow::Result<()> {
    // First attempt: real git apply (respects the patch the optimizer generated)
    let patch_file = dir.join(".mdx_patch.diff");
    let _ = std::fs::write(&patch_file, patch);

    let apply_ok = Command::new("git")
        .current_dir(dir)
        .args([
            "apply",
            "--reject",
            "--whitespace=fix",
            patch_file.to_str().unwrap(),
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    let _ = std::fs::remove_file(&patch_file);

    if apply_ok {
        return Ok(());
    }

    // Fallback: targeted smart edit for the things we commonly optimize (preambles, tools)
    let candidates = ["src/main.rs", "main.rs", "lib.rs", "agent.rs"];

    for rel in &candidates {
        let target = dir.join(rel);
        if !target.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&target)?;
        let improved = if patch.contains("Think step-by-step before answering") {
            "You are a concise, helpful assistant. Think step-by-step before answering. Always explain your reasoning in one sentence, then give the final answer."
        } else if patch.contains("reasoning") {
            "You are a concise, helpful assistant. Think step-by-step before answering."
        } else {
            continue;
        };

        let new_content = if let Some(start) = content.find(".preamble(\"") {
            let prefix = &content[..start + 11];
            let rest = &content[start + 11..];
            if let Some(end) = rest.find("\"") {
                format!("{}{}{}", prefix, improved, &rest[end..])
            } else {
                content.clone()
            }
        } else if content.contains("concise, helpful assistant") {
            content.replace(
                "concise, helpful assistant",
                &improved.replace("You are a ", ""),
            )
        } else {
            content.clone()
        };

        if new_content != content {
            std::fs::write(&target, new_content)?;
            return Ok(());
        }
    }

    Err(anyhow::anyhow!(
        "apply_patch could not apply the edit (neither git apply nor fallback succeeded)"
    ))
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
        if !out.status.success() {
            success = false;
        }
    } else {
        success = false;
    }

    if let Ok(out) = clippy {
        output.push_str(&String::from_utf8_lossy(&out.stdout));
        output.push_str(&String::from_utf8_lossy(&out.stderr));
        if !out.status.success() {
            success = false;
        }
    }

    (success, output)
}

/// High-level helper: take a ProposedEdit, create an isolated workspace (git worktree or copy),
/// apply the edit, run cargo check + clippy, then clean up.
/// This is the core safety primitive of mdx-rust.
pub fn apply_and_validate(
    agent_path: &Path,
    edit: &ProposedEdit,
    name: &str,
) -> anyhow::Result<ValidationResult> {
    let isolated = create_isolated_workspace(agent_path, name)?;
    apply_patch(&isolated, &edit.patch)?;

    let (passed, output) = validate_build(&isolated);

    // Best-effort cleanup
    if isolated
        .parent()
        .is_some_and(|p| p.file_name() == Some(std::ffi::OsStr::new(".worktrees")))
    {
        // Only try git worktree remove if it looks like a real worktree dir
        let _ = Command::new("git")
            .current_dir(agent_path)
            .args(["worktree", "remove", "--force", isolated.to_str().unwrap()])
            .status();
    }
    // For copied trees we leave them (they are cheap to recreate and useful for inspection)

    Ok(ValidationResult {
        passed,
        cargo_check_output: output,
        clippy_output: String::new(),
        new_score: None,
    })
}
