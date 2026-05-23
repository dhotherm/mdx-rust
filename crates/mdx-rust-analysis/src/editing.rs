//! Safe code editing and validation pipeline (Phase 2+).
//
//! This module now has real (early) support for git worktrees + patch application + validation.

use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

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
    pub command_records: Vec<ValidationCommandRecord>,
}

/// Auditable record for a validation command.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationCommandRecord {
    pub command: String,
    pub success: bool,
    pub timed_out: bool,
    pub status_code: Option<i32>,
    pub duration_ms: u64,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub passed: bool,
    pub combined_output: String,
    pub command_records: Vec<ValidationCommandRecord>,
}

/// Captured command execution result.
#[derive(Debug, Clone)]
pub struct CapturedCommand {
    pub status: Option<ExitStatus>,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub duration_ms: u64,
}

impl CapturedCommand {
    pub fn success(&self) -> bool {
        self.status.is_some_and(|status| status.success()) && !self.timed_out
    }

    pub fn combined_output(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

/// Create a git worktree for safe experimentation (best when agent_path is a git repo root).
/// Falls back to a filesystem copy if worktree creation fails (e.g. agent lives inside another repo).
pub fn create_isolated_workspace(agent_path: &Path, name: &str) -> anyhow::Result<PathBuf> {
    // Try git worktree first (fast, shares objects, real git history)
    // But skip it if the agent lives deep inside another repo (common for examples/ in monorepos)
    let should_try_worktree = !agent_path.to_string_lossy().contains("/examples/")
        && !agent_path.to_string_lossy().contains("\\examples\\");

    if should_try_worktree {
        let mut rev_parse = Command::new("git");
        rev_parse
            .current_dir(agent_path)
            .args(["rev-parse", "--show-toplevel"]);
        let is_git_repo = run_command_with_timeout(&mut rev_parse, Duration::from_secs(10))
            .map(|output| output.success())
            .unwrap_or(false);

        if !is_git_repo {
            return create_temp_workspace_copy(agent_path, name);
        }

        let base = agent_path.join(".worktrees");
        std::fs::create_dir_all(&base)?;
        let worktree_path = base.join(name);

        let mut remove = Command::new("git");
        remove.current_dir(agent_path).args([
            "worktree",
            "remove",
            "--force",
            worktree_path.to_str().unwrap(),
        ]);
        let _ = run_command_with_timeout(&mut remove, Duration::from_secs(20));

        let mut add = Command::new("git");
        add.current_dir(agent_path).args([
            "worktree",
            "add",
            "--detach",
            worktree_path.to_str().unwrap(),
            "HEAD",
        ]);

        if run_command_with_timeout(&mut add, Duration::from_secs(30))
            .map(|output| output.success())
            .unwrap_or(false)
        {
            return Ok(worktree_path);
        }
    }

    create_temp_workspace_copy(agent_path, name)
}

fn create_temp_workspace_copy(agent_path: &Path, name: &str) -> anyhow::Result<PathBuf> {
    // Fallback: proper temp directory copy outside the source tree (prevents recursion and .worktrees self-copy)
    let isolated_parent = tempfile::Builder::new()
        .prefix("mdx-rust-workspace-")
        .tempdir()?
        .keep();
    let isolated_path = isolated_parent.join(name);

    // Use improved copy that excludes dangerous dirs
    copy_dir_all_excluding(
        agent_path,
        &isolated_path,
        &[".git", ".worktrees", "target", ".mdx-rust"],
    )?;

    // Init git in the copy for cargo/git commands
    let mut init = Command::new("git");
    init.current_dir(&isolated_path).args(["init", "-q"]);
    let _ = run_command_with_timeout(&mut init, Duration::from_secs(20));
    let mut add = Command::new("git");
    add.current_dir(&isolated_path).args(["add", "-A"]);
    let _ = run_command_with_timeout(&mut add, Duration::from_secs(20));
    let mut commit = Command::new("git");
    commit
        .current_dir(&isolated_path)
        .args(["commit", "-q", "-m", "mdx-rust isolated copy"]);
    let _ = run_command_with_timeout(&mut commit, Duration::from_secs(20));

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
    apply_patch_with_target(dir, None, patch)
}

/// Apply a proposed edit to an isolated workspace or the real agent tree.
///
/// The edit's `file` field is authoritative for fallback edits. The unified
/// diff is attempted first, but string-based fallback is constrained to the
/// resolved target file so a patch can never drift into an unrelated source.
pub fn apply_edit(
    agent_root: &Path,
    workspace_root: &Path,
    edit: &ProposedEdit,
) -> anyhow::Result<()> {
    let rel = relative_edit_path(agent_root, &edit.file)?;
    apply_patch_with_target(workspace_root, Some(&rel), &edit.patch)
}

pub fn apply_edit_to_agent(agent_root: &Path, edit: &ProposedEdit) -> anyhow::Result<()> {
    apply_edit(agent_root, agent_root, edit)
}

fn apply_patch_with_target(dir: &Path, target: Option<&Path>, patch: &str) -> anyhow::Result<()> {
    // First attempt: real git apply (respects the patch the optimizer generated)
    // Protected by timeout so a stuck git process cannot hang the optimizer (P0).
    let patch_file = dir.join(".mdx_patch.diff");
    let _ = std::fs::write(&patch_file, patch);

    let mut git_apply = Command::new("git");
    git_apply
        .current_dir(dir)
        .args(["apply", "--whitespace=fix", patch_file.to_str().unwrap()]);

    let apply_ok = run_command_with_timeout(&mut git_apply, Duration::from_secs(30))
        .map(|output| output.success())
        .unwrap_or(false);

    let _ = std::fs::remove_file(&patch_file);

    if apply_ok {
        return Ok(());
    }

    // Fallback: targeted smart edit for the things we commonly optimize.
    // In real edit application, this is constrained to ProposedEdit.file.
    let candidates: Vec<PathBuf> = if let Some(target) = target {
        vec![target.to_path_buf()]
    } else {
        ["src/main.rs", "main.rs", "lib.rs", "agent.rs"]
            .into_iter()
            .map(PathBuf::from)
            .collect()
    };

    for rel in &candidates {
        let target_path = dir.join(rel);
        if !target_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(&target_path)?;
        if patch.contains("Best-effort answer after reasoning")
            && apply_syn_guarded_echo_rewrite(&target_path, &content)?
        {
            return Ok(());
        }

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
            std::fs::write(&target_path, new_content)?;
            return Ok(());
        }
    }

    Err(anyhow::anyhow!(
        "apply_patch could not apply the edit (neither git apply nor fallback succeeded)"
    ))
}

fn apply_syn_guarded_echo_rewrite(target_path: &Path, content: &str) -> anyhow::Result<bool> {
    syn::parse_file(content)
        .map_err(|err| anyhow::anyhow!("source did not parse before fallback rewrite: {err}"))?;

    let new_content = content
        .replace("Echo: {}", "Best-effort answer after reasoning: {}")
        .replace("Echo: ", "Best-effort answer after reasoning: ");

    if new_content == content {
        return Ok(false);
    }

    syn::parse_file(&new_content)
        .map_err(|err| anyhow::anyhow!("source did not parse after fallback rewrite: {err}"))?;
    std::fs::write(target_path, new_content)?;
    Ok(true)
}

fn relative_edit_path(agent_root: &Path, file: &Path) -> anyhow::Result<PathBuf> {
    let rel = if file.is_absolute() {
        file.strip_prefix(agent_root)
            .map_err(|_| {
                anyhow::anyhow!("edit target is outside the agent root: {}", file.display())
            })?
            .to_path_buf()
    } else {
        file.to_path_buf()
    };

    if rel.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        anyhow::bail!(
            "edit target contains unsafe path components: {}",
            rel.display()
        );
    }

    Ok(rel)
}

/// Run cargo check + clippy in a directory with timeout.
/// Returns (success, combined output).
/// A hanging or extremely slow cargo command must fail the validation instead of hanging the optimizer (P0).
pub fn validate_build(dir: &Path) -> (bool, String) {
    let report = validate_build_detailed(dir);
    (report.passed, report.combined_output)
}

pub fn validate_build_detailed(dir: &Path) -> ValidationReport {
    validate_build_detailed_with_budget(dir, Duration::from_secs(180))
}

pub fn validate_build_detailed_with_budget(dir: &Path, budget: Duration) -> ValidationReport {
    let started = Instant::now();

    fn run_cargo_with_timeout(
        dir: &Path,
        args: &[&str],
        timeout: Duration,
    ) -> Option<CapturedCommand> {
        let mut command = Command::new("cargo");
        command.current_dir(dir).args(args);
        run_command_with_timeout(&mut command, timeout)
    }

    let mut output = String::new();
    let mut success = true;
    let mut command_records = Vec::new();

    for (label, args) in [
        ("cargo check", &["check"][..]),
        (
            "cargo clippy -- -D warnings",
            &["clippy", "--", "-D", "warnings"][..],
        ),
    ] {
        let Some(remaining) = budget.checked_sub(started.elapsed()) else {
            output.push_str(&format!("[{label} skipped: validation budget exhausted]\n"));
            success = false;
            command_records.push(ValidationCommandRecord {
                command: label.to_string(),
                success: false,
                timed_out: true,
                status_code: None,
                duration_ms: started.elapsed().as_millis() as u64,
                stdout: String::new(),
                stderr: "validation budget exhausted before command started".to_string(),
            });
            continue;
        };

        if remaining.is_zero() {
            output.push_str(&format!("[{label} skipped: validation budget exhausted]\n"));
            success = false;
            command_records.push(ValidationCommandRecord {
                command: label.to_string(),
                success: false,
                timed_out: true,
                status_code: None,
                duration_ms: started.elapsed().as_millis() as u64,
                stdout: String::new(),
                stderr: "validation budget exhausted before command started".to_string(),
            });
            continue;
        }

        if let Some(result) = run_cargo_with_timeout(dir, args, remaining) {
            output.push_str(&result.combined_output());
            if !result.success() {
                success = false;
            }
            command_records.push(ValidationCommandRecord {
                command: label.to_string(),
                success: result.success(),
                timed_out: result.timed_out,
                status_code: result.status.and_then(|status| status.code()),
                duration_ms: result.duration_ms,
                stdout: result.stdout,
                stderr: result.stderr,
            });
        } else {
            output.push_str(&format!("[{label} failed to start]\n"));
            success = false;
            command_records.push(ValidationCommandRecord {
                command: label.to_string(),
                success: false,
                timed_out: false,
                status_code: None,
                duration_ms: 0,
                stdout: String::new(),
                stderr: "failed to start validation command".to_string(),
            });
        }
    }

    ValidationReport {
        passed: success,
        combined_output: output,
        command_records,
    }
}

/// Run a Command with a timeout. Returns None on timeout (treated as failure by callers).
pub fn run_command_with_timeout(cmd: &mut Command, timeout: Duration) -> Option<CapturedCommand> {
    configure_process_group(cmd);

    let mut child = match cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return None,
    };

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                let output = child.wait_with_output().ok()?;
                return Some(CapturedCommand {
                    status: Some(output.status),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    timed_out: false,
                    duration_ms,
                });
            }
            Ok(None) if start.elapsed() >= timeout => {
                terminate_process_group(child.id());
                let _ = child.kill();
                let duration_ms = start.elapsed().as_millis() as u64;
                let output = child.wait_with_output().ok()?;
                return Some(CapturedCommand {
                    status: Some(output.status),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    timed_out: true,
                    duration_ms,
                });
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(_) => {
                terminate_process_group(child.id());
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

#[cfg(unix)]
fn configure_process_group(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    cmd.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_cmd: &mut Command) {}

#[cfg(unix)]
fn terminate_process_group(pid: u32) {
    let group = format!("-{pid}");
    for signal in ["-TERM", "-KILL"] {
        let _ = Command::new("kill")
            .arg(signal)
            .arg(&group)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(not(unix))]
fn terminate_process_group(_pid: u32) {}

#[derive(Debug)]
pub struct FileSnapshot {
    path: PathBuf,
    content: Option<Vec<u8>>,
}

pub fn snapshot_file(path: &Path) -> anyhow::Result<FileSnapshot> {
    let content = if path.exists() {
        Some(std::fs::read(path)?)
    } else {
        None
    };

    Ok(FileSnapshot {
        path: path.to_path_buf(),
        content,
    })
}

pub fn restore_file(snapshot: &FileSnapshot) -> anyhow::Result<()> {
    if let Some(parent) = snapshot.path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    match &snapshot.content {
        Some(content) => std::fs::write(&snapshot.path, content)?,
        None if snapshot.path.exists() => std::fs::remove_file(&snapshot.path)?,
        None => {}
    }

    Ok(())
}

#[derive(Debug)]
pub struct TransactionSnapshot {
    files: Vec<FileSnapshot>,
}

pub fn snapshot_transaction(paths: &[PathBuf]) -> anyhow::Result<TransactionSnapshot> {
    let mut files = Vec::with_capacity(paths.len());
    for path in paths {
        files.push(snapshot_file(path)?);
    }
    Ok(TransactionSnapshot { files })
}

pub fn restore_transaction(snapshot: &TransactionSnapshot) -> anyhow::Result<()> {
    for file in snapshot.files.iter().rev() {
        restore_file(file)?;
    }
    Ok(())
}

/// High-level helper: take a ProposedEdit, create an isolated workspace (git worktree or copy),
/// apply the edit, run cargo check + clippy, then clean up.
/// This is the core safety primitive of mdx-rust.
pub fn apply_and_validate(
    agent_path: &Path,
    edit: &ProposedEdit,
    name: &str,
) -> anyhow::Result<ValidationResult> {
    apply_and_validate_with_budget(agent_path, edit, name, Duration::from_secs(180))
}

pub fn apply_and_validate_with_budget(
    agent_path: &Path,
    edit: &ProposedEdit,
    name: &str,
    validation_budget: Duration,
) -> anyhow::Result<ValidationResult> {
    let isolated = create_isolated_workspace(agent_path, name)?;
    apply_edit(agent_path, &isolated, edit)?;

    let report = validate_build_detailed_with_budget(&isolated, validation_budget);

    cleanup_isolated_workspace(agent_path, &isolated);

    Ok(ValidationResult {
        passed: report.passed,
        cargo_check_output: report.combined_output,
        clippy_output: String::new(),
        new_score: None,
        command_records: report.command_records,
    })
}

pub fn cleanup_isolated_workspace(agent_path: &Path, isolated: &Path) {
    if isolated
        .parent()
        .is_some_and(|p| p.file_name() == Some(std::ffi::OsStr::new(".worktrees")))
    {
        // Only try git worktree remove if it looks like a real worktree dir
        let mut remove = Command::new("git");
        remove.current_dir(agent_path).args([
            "worktree",
            "remove",
            "--force",
            isolated.to_str().unwrap(),
        ]);
        let _ = run_command_with_timeout(&mut remove, Duration::from_secs(20));
    } else if let Some(parent) = isolated.parent() {
        if parent
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("mdx-rust-workspace-"))
        {
            let _ = std::fs::remove_dir_all(parent);
        } else {
            let _ = std::fs::remove_dir_all(isolated);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use std::time::{Duration, Instant};
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

    #[test]
    fn temp_workspace_for_non_git_repo_does_not_create_source_worktrees_dir() {
        let src = tempdir().unwrap();
        fs::create_dir_all(src.path().join("src")).unwrap();
        fs::write(src.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(
            src.path().join("Cargo.toml"),
            "[package]\nname=\"t\"\nversion=\"0.1.0\"\nedition=\"2021\"",
        )
        .unwrap();

        let isolated = create_isolated_workspace(src.path(), "no-git").unwrap();
        assert!(isolated.exists());
        assert!(
            !src.path().join(".worktrees").exists(),
            "temp-copy fallback must not mutate the source tree"
        );
        cleanup_isolated_workspace(src.path(), &isolated);
    }

    #[test]
    fn apply_edit_fallback_only_changes_requested_file() {
        let root = tempdir().unwrap();
        let src = root.path().join("src");
        fs::create_dir_all(&src).unwrap();

        let main = src.join("main.rs");
        let agent = src.join("agent.rs");
        let weak =
            r#"client.agent("m").preamble("You are a concise, helpful assistant.").build();"#;
        fs::write(&main, weak).unwrap();
        fs::write(&agent, weak).unwrap();

        let edit = ProposedEdit {
            file: agent.clone(),
            description: "strengthen prompt".to_string(),
            patch: "not a real diff, but Think step-by-step before answering".to_string(),
        };

        apply_edit(root.path(), root.path(), &edit).unwrap();

        let main_after = fs::read_to_string(main).unwrap();
        let agent_after = fs::read_to_string(agent).unwrap();

        assert!(
            !main_after.contains("Think step-by-step"),
            "fallback must not drift into unrelated files"
        );
        assert!(
            agent_after.contains("Think step-by-step"),
            "requested edit target should be changed"
        );
    }

    #[test]
    fn apply_edit_fallback_can_replace_echo_response_prefix() {
        let root = tempdir().unwrap();
        let src = root.path().join("src");
        fs::create_dir_all(&src).unwrap();

        let main = src.join("main.rs");
        fs::write(
            &main,
            r#"fn main() { println!("{}", format!("Echo: {}", "hello")); }"#,
        )
        .unwrap();

        let edit = ProposedEdit {
            file: main.clone(),
            description: "replace echo fallback".to_string(),
            patch: "not a real diff, but Best-effort answer after reasoning".to_string(),
        };

        apply_edit(root.path(), root.path(), &edit).unwrap();

        let main_after = fs::read_to_string(main).unwrap();
        assert!(main_after.contains("Best-effort answer after reasoning"));
        assert!(!main_after.contains("Echo:"));
    }

    #[test]
    fn echo_fallback_rewrite_requires_parseable_rust_before_writing() {
        let root = tempdir().unwrap();
        let src = root.path().join("src");
        fs::create_dir_all(&src).unwrap();

        let main = src.join("main.rs");
        let broken = r#"fn main( { println!("Echo: {}", "hello"); }"#;
        fs::write(&main, broken).unwrap();

        let edit = ProposedEdit {
            file: main.clone(),
            description: "replace echo fallback".to_string(),
            patch: "not a real diff, but Best-effort answer after reasoning".to_string(),
        };

        let error = apply_edit(root.path(), root.path(), &edit).unwrap_err();

        assert!(error.to_string().contains("source did not parse"));
        assert_eq!(fs::read_to_string(main).unwrap(), broken);
    }

    #[test]
    fn snapshot_restore_puts_file_back() {
        let root = tempdir().unwrap();
        let file = root.path().join("src/main.rs");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "before").unwrap();

        let snapshot = snapshot_file(&file).unwrap();
        fs::write(&file, "after").unwrap();
        restore_file(&snapshot).unwrap();

        assert_eq!(fs::read_to_string(file).unwrap(), "before");
    }

    #[test]
    fn transaction_restore_rolls_back_multiple_files() {
        let root = tempdir().unwrap();
        let first = root.path().join("src/main.rs");
        let second = root.path().join("src/lib.rs");
        fs::create_dir_all(first.parent().unwrap()).unwrap();
        fs::write(&first, "first-before").unwrap();
        fs::write(&second, "second-before").unwrap();

        let snapshot = snapshot_transaction(&[first.clone(), second.clone()]).unwrap();
        fs::write(&first, "first-after").unwrap();
        fs::write(&second, "second-after").unwrap();

        restore_transaction(&snapshot).unwrap();

        assert_eq!(fs::read_to_string(first).unwrap(), "first-before");
        assert_eq!(fs::read_to_string(second).unwrap(), "second-before");
    }

    #[test]
    fn command_timeout_kills_and_captures_without_leaking() {
        let start = Instant::now();
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg("printf noisy-output; while true; do :; done");

        let result = run_command_with_timeout(&mut command, Duration::from_millis(100)).unwrap();

        assert!(result.timed_out);
        assert!(start.elapsed() < Duration::from_secs(2));
        assert_eq!(result.stdout, "noisy-output");
        assert!(result.duration_ms > 0);
    }

    #[test]
    fn validate_build_records_command_outcomes() {
        let src = tempdir().unwrap();
        fs::create_dir_all(src.path().join("src")).unwrap();
        fs::write(src.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(
            src.path().join("Cargo.toml"),
            "[package]\nname=\"t\"\nversion=\"0.1.0\"\nedition=\"2021\"",
        )
        .unwrap();

        let report = validate_build_detailed(src.path());

        assert!(report.passed);
        assert_eq!(report.command_records.len(), 2);
        assert!(report
            .command_records
            .iter()
            .all(|record| record.duration_ms > 0));
    }

    #[test]
    fn validate_build_budget_exhaustion_records_timeout() {
        let src = tempdir().unwrap();
        fs::create_dir_all(src.path().join("src")).unwrap();
        fs::write(src.path().join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(
            src.path().join("Cargo.toml"),
            "[package]\nname=\"t\"\nversion=\"0.1.0\"\nedition=\"2021\"",
        )
        .unwrap();

        let report = validate_build_detailed_with_budget(src.path(), Duration::from_secs(0));

        assert!(!report.passed);
        assert_eq!(report.command_records.len(), 2);
        assert!(report.command_records.iter().all(|record| record.timed_out));
    }
}
