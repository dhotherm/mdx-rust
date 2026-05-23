//! Public documentation facade for the `mdx-rust` CLI crate.
//!
//! Most reusable APIs live in [`mdx_rust_core`] and [`mdx_rust_analysis`].
//! This crate primarily ships the `mdx-rust` binary, but it also exposes those
//! lower-level crates so docs.rs has a stable library target to document.
//!
//! Install the CLI with:
//!
//! ```text
//! cargo install mdx-rust
//! ```

pub use mdx_rust_analysis as analysis;
pub use mdx_rust_core as core;
