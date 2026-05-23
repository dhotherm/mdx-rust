//! Code bundling and ignore logic
//!
//! This module handles .mdx-rustignore + .gitignore + built-in rules
//! to determine what gets included when we send code to the LLM.
//! Now also runs deep finders to extract prompts, tools, and entrypoints.

use crate::finders::{find_preambles, find_tools, looks_like_rig_agent, ExtractedPrompt, ExtractedTool};
use ignore::WalkBuilder;
use std::fs;
use std::path::{Path, PathBuf};

/// Basic file-level scope (what to send / edit).
#[derive(Debug, Clone, Default)]
pub struct BundleScope {
    pub optimizable_paths: Vec<PathBuf>,
    pub read_only_paths: Vec<PathBuf>,
}

/// Rich analysis result for an agent.
/// This is what gets summarized and sent to the LLM for high-quality diagnosis.
#[derive(Debug, Clone, Default)]
pub struct AgentBundle {
    pub scope: BundleScope,
    pub preambles: Vec<ExtractedPrompt>,
    pub tools: Vec<ExtractedTool>,
    pub is_rig_agent: bool,
    pub key_files: Vec<PathBuf>, // top files worth showing in full to the LLM
}

/// Builds the set of files we care about + runs finders on them.
pub fn build_bundle_scope(
    root: &Path,
    custom_ignore_file: Option<&Path>,
) -> anyhow::Result<BundleScope> {
    let mut walker = WalkBuilder::new(root);

    walker.git_ignore(true);
    walker.git_global(true);
    walker.git_exclude(true);

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
            optimizable.push(path.to_path_buf());
        }
    }

    Ok(BundleScope {
        optimizable_paths: optimizable,
        read_only_paths: vec![],
    })
}

/// Full analysis: walks the agent, extracts prompts/tools, identifies Rig usage.
pub fn analyze_agent(root: &Path, custom_ignore: Option<&Path>) -> anyhow::Result<AgentBundle> {
    let scope = build_bundle_scope(root, custom_ignore)?;
    let mut bundle = AgentBundle {
        scope,
        ..Default::default()
    };

    // Limit how many files we deeply analyze (keep bundles small for LLM)
    let candidates: Vec<_> = bundle
        .scope
        .optimizable_paths
        .iter()
        .filter(|p| p.extension().map_or(false, |e| e == "rs"))
        .take(12)
        .cloned()
        .collect();

    for path in &candidates {
        if let Ok(source) = fs::read_to_string(path) {
            if looks_like_rig_agent(&source) {
                bundle.is_rig_agent = true;

                // Extract preambles
                let mut ps = find_preambles(&source, path);
                bundle.preambles.append(&mut ps);

                // Extract tools
                let mut ts = find_tools(&source, path);
                bundle.tools.append(&mut ts);

                // Keep the most interesting files for full context
                if bundle.key_files.len() < 4 {
                    bundle.key_files.push(path.clone());
                }
            }
        }
    }

    // If we didn't find anything Rig-specific, still keep top .rs files
    if bundle.key_files.is_empty() {
        for p in candidates.iter().take(3) {
            bundle.key_files.push(p.clone());
        }
    }

    Ok(bundle)
}