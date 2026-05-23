//! Code bundling and ignore logic
//!
//! This module handles .mdx-rustignore + .gitignore + built-in rules
//! to determine what gets included when we send code to the LLM.

use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Represents what files we will include when analyzing an agent.
#[derive(Debug, Clone)]
pub struct BundleScope {
    pub optimizable_paths: Vec<PathBuf>,
    pub read_only_paths: Vec<PathBuf>,
}

/// Builds the set of files we care about for a given project root.
pub fn build_bundle_scope(
    root: &Path,
    custom_ignore_file: Option<&Path>,
) -> anyhow::Result<BundleScope> {
    let mut walker = WalkBuilder::new(root);

    // Respect .gitignore
    walker.git_ignore(true);
    walker.git_global(true);
    walker.git_exclude(true);

    // Add our custom .mdx-rustignore if present
    if let Some(ignore_path) = custom_ignore_file {
        if ignore_path.exists() {
            walker.add_ignore(ignore_path);
        }
    } else {
        let default_ignore = root.join(".mdx-rust/.mdx-rustignore");
        if default_ignore.exists() {
            walker.add_ignore(default_ignore);
        }
    }

    walker.require_git(false);

    let mut optimizable = Vec::new();

    for entry in walker.build() {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            // For Phase 0/1 we treat everything walkable as optimizable.
            // Later we will add smarter classification (prompts, tools, logic).
            optimizable.push(path.to_path_buf());
        }
    }

    Ok(BundleScope {
        optimizable_paths: optimizable,
        read_only_paths: vec![],
    })
}