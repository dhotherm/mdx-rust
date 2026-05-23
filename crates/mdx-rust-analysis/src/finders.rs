//! Basic source finders using tree-sitter and syn.
//!
//! Goal in Phase 2: locate prompts, tool definitions, run_agent functions, etc.

use tree_sitter::{Parser, Tree};

pub fn parse_rust_source(source: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_rust::language()).ok()?;
    parser.parse(source, None)
}

/// Very naive finder for functions named "run_agent" or similar.
/// This will be greatly expanded.
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
                if name.contains("run_agent") || name.contains("run") {
                    functions.push(name.to_string());
                }
            }
        }
    }

    functions
}