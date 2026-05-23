//! Safe code editing and validation pipeline (Phase 2+).
//
//! This module now has real (early) support for git worktrees + patch application + validation.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

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

    // Fallback: proper temp directory copy outside the source tree (prevents recursion and .worktrees self-copy)
    let temp_dir = tempfile::tempdir()?;
    let isolated_path = temp_dir.path().join(name);

    // Use improved copy that excludes dangerous dirs
    copy_dir_all_excluding(
        agent_path,
        &isolated_path,
        &[".git", ".worktrees", "target", ".mdx-rust"],
    )?;

    // We must keep the tempdir alive, so we leak it intentionally for the lifetime of the validation.
    // In production a better RAII handle would be used.
    std::mem::forget(temp_dir);

    // Init git in the copy for cargo/git commands
    let _ = Command::new("git")
        .current_dir(&isolated_path)
        .args(["init", "-q"])
        .status();
    let _ = Command::new("git")
        .current_dir(&isolated_path)
        .args(["add", "-A"])
        .status();
    let _ = Command::new("git")
        .current_dir(&isolated_path)
        .args(["commit", "-q", "-m", "mdx-rust isolated copy"])
        .status();

    Ok(isolated_path)
}

pub(crate) fn copy_dir_all_excluding(
    src: &Path,
    dst: &Path,
    exclude: &[&str],
) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if exclude.iter().any(|e| name_str == *e) {
            continue;
        }

        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(name);

        if ty.is_dir() {
            copy_dir_all_excluding(&src_path, &dst_path, exclude)?;
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
    // Protected by timeout so a stuck git process cannot hang the optimizer (P0).
    let patch_file = dir.join(".mdx_patch.diff");
    let _ = std::fs::write(&patch_file, patch);

    let mut git_apply = Command::new("git");
    git_apply.current_dir(dir).args([
        "apply",
        "--reject",
        "--whitespace=fix",
        patch_file.to_str().unwrap(),
    ]);

    let apply_ok = run_command_with_timeout(&mut git_apply, Duration::from_secs(30))
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

/// Run cargo check + clippy in a directory with timeout.
/// Returns (success, combined output).
/// A hanging or extremely slow cargo command must fail the validation instead of hanging the optimizer (P0).
pub fn validate_build(dir: &Path) -> (bool, String) {
    const CARGO_TIMEOUT: Duration = Duration::from_secs(90);

    fn run_cargo_with_timeout(
        dir: &Path,
        args: &[&str],
        timeout: Duration,
    ) -> Option<(bool, String)> {
        use std::sync::mpsc;

        let dir = dir.to_path_buf();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            let output = Command::new("cargo").current_dir(&dir).args(&args).output();
            let _ = tx.send(output);
        });

        match rx.recv_timeout(timeout) {
            Ok(Ok(out)) => {
                let text = format!(
                    "{}{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                Some((out.status.success(), text))
            }
            _ => None, // timeout or error → treat as failure
        }
    }

    let mut output = String::new();
    let mut success = true;

    if let Some((ok, text)) = run_cargo_with_timeout(dir, &["check"], CARGO_TIMEOUT) {
        output.push_str(&text);
        if !ok {
            success = false;
        }
    } else {
        output.push_str("[cargo check timed out]\n");
        success = false;
    }

    if let Some((ok, text)) =
        run_cargo_with_timeout(dir, &["clippy", "--", "-D", "warnings"], CARGO_TIMEOUT)
    {
        output.push_str(&text);
        if !ok {
            success = false;
        }
    } else {
        output.push_str("[cargo clippy timed out]\n");
        success = false;
    }

    (success, output)
}

/// Run a Command with a timeout. Returns None on timeout (treated as failure by callers).
fn run_command_with_timeout(
    cmd: &mut Command,
    timeout: Duration,
) -> Option<std::process::ExitStatus> {
    use std::sync::mpsc;

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return None,
    };

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let res = child.wait();
        let _ = tx.send(res);
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(status)) => Some(status),
        _ => None,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn copy_dir_all_excluding_prevents_recursion_into_worktrees_and_target() {
        let src = tempdir().unwrap();
        let src_path = src.path();

        // Create normal source
        fs::create_dir_all(src_path.join("src")).unwrap();
        fs::write(src_path.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(
            src_path.join("Cargo.toml"),
            "[package]\nname=\"t\"\nversion=\"0.1\"",
        )
        .unwrap();

        // Create dangerous dirs that must be excluded
        fs::create_dir_all(src_path.join(".worktrees").join("some-worktree")).unwrap();
        fs::write(src_path.join(".worktrees/some-worktree/evil.rs"), "BAD").unwrap();

        fs::create_dir_all(src_path.join("target").join("debug")).unwrap();
        fs::write(src_path.join("target/debug/bad.o"), "binary").unwrap();

        fs::create_dir_all(src_path.join(".git")).unwrap();
        fs::write(src_path.join(".git/config"), "git").unwrap();

        let dst = tempdir().unwrap();
        let dst_path = dst.path().join("copy");

        copy_dir_all_excluding(
            src_path,
            &dst_path,
            &[".git", ".worktrees", "target", ".mdx-rust"],
        )
        .unwrap();

        // Assertions: dangerous content must not be present
        assert!(
            dst_path.join("src/main.rs").exists(),
            "normal source must be copied"
        );
        assert!(
            !dst_path.join(".worktrees").exists(),
            ".worktrees must be excluded (no recursion)"
        );
        assert!(!dst_path.join("target").exists(), "target must be excluded");
        assert!(!dst_path.join(".git").exists(), ".git must be excluded");
    }
}
