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
//!
//! ## Stability contract
//!
//! The CLI is the supported product surface for `0.2.x`. The re-exported
//! library crates are available for inspection and experiments, but their APIs
//! remain unstable before `1.0`.

pub use mdx_rust_analysis as analysis;
pub use mdx_rust_core as core;
