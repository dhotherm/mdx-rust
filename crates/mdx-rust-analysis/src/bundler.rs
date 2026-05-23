//! Code bundling logic (will be expanded significantly in Phase 2)

/// Represents the scope of files that can be sent to an LLM for analysis
#[derive(Debug, Clone)]
pub struct BundleScope {
    pub optimizable_paths: Vec<std::path::PathBuf>,
    pub read_only_paths: Vec<std::path::PathBuf>,
}