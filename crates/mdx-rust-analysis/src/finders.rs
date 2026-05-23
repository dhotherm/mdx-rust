//! Basic source finders using tree-sitter and syn.
//!
//! Goal: locate prompts, tool definitions, agent entrypoints, etc. inside Rig (and other) agents.

use tree_sitter::{Parser, Tree};

pub fn parse_rust_source(source: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_rust::language()).ok()?;
    parser.parse(source, None)
}

/// Find functions that look like agent entrypoints.
pub fn find_run_agent_functions(source: &str) -> Vec<String> {
    let Some(tree) = parse_rust_source(source) else {
        return vec![];
    };

    let mut functions = vec![];
    let root = tree.root_node();

    let mut cursor = root.walk();
    for node in root.children(&mut cursor) {
        if node.kind() == "function_item" {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = name_node.utf8_text(source.as_bytes()).unwrap_or("");
                if name.contains("run_agent") || name == "run" {
                    functions.push(name.to_string());
                }
            }
        }
    }

    functions
}

/// Very rough heuristic to detect Rig-style agents by looking for common patterns.
/// This will be replaced with proper syn-based analysis of the AST.
pub fn looks_like_rig_agent(source: &str) -> bool {
    source.contains("rig::") || source.contains(".agent(") || source.contains("preamble(")
}