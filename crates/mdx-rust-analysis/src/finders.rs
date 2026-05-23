//! Source finders using tree-sitter + syn for deep understanding of Rig agents.
//!
//! Extracts high-signal artifacts (preambles, tools, entrypoints) to feed the optimizer LLM.

use std::path::Path;
use tree_sitter::{Parser, Tree};

#[derive(Debug, Clone)]
pub struct ExtractedPrompt {
    pub file: String,
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ExtractedTool {
    pub file: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentEntrypoint {
    pub name: String,
    pub file: String,
    pub line: usize,
}

/// Parse Rust source into a tree-sitter tree.
pub fn parse_rust_source(source: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser.set_language(tree_sitter_rust::language()).ok()?;
    parser.parse(source, None)
}

/// Find functions that look like agent entrypoints (run_agent, run, main agent fn).
pub fn find_run_agent_functions(source: &str) -> Vec<AgentEntrypoint> {
    let Some(tree) = parse_rust_source(source) else {
        return vec![];
    };

    let mut entries = vec![];
    let root = tree.root_node();

    for node in root.children(&mut root.walk()) {
        if node.kind() == "function_item" {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = name_node.utf8_text(source.as_bytes()).unwrap_or("");
                if name.contains("run_agent") || name == "run" || name.contains("agent") {
                    let line = node.start_position().row + 1;
                    entries.push(AgentEntrypoint {
                        name: name.to_string(),
                        file: "<unknown>".to_string(),
                        line,
                    });
                }
            }
        }
    }
    entries
}

/// Extract preamble strings from Rig-style `.preamble("...")` calls.
/// Uses simple but effective tree-sitter + string heuristics for now.
pub fn find_preambles(source: &str, file_path: &Path) -> Vec<ExtractedPrompt> {
    let mut prompts = vec![];

    // Tree-sitter walk for call expressions containing "preamble"
    if let Some(tree) = parse_rust_source(source) {
        let root = tree.root_node();
        let cursor = root.walk();

        fn walk(node: tree_sitter::Node, source: &str, file_path: &Path, prompts: &mut Vec<ExtractedPrompt>) {
            if node.kind() == "call_expression" {
                let text = node.utf8_text(source.as_bytes()).unwrap_or("");
                if text.contains("preamble(") {
                    // Try to extract the string literal argument
                    if let Some(start) = text.find("preamble(\"") {
                        let after = &text[start + 10..];
                        if let Some(end) = after.find('"') {
                            let content = &after[..end];
                            if !content.is_empty() && content.len() > 3 {
                                let line = node.start_position().row + 1;
                                prompts.push(ExtractedPrompt {
                                    file: file_path.display().to_string(),
                                    line,
                                    text: content.to_string(),
                                });
                            }
                        }
                    }
                }
            }
            let mut c = node.walk();
            for child in node.children(&mut c) {
                walk(child, source, file_path, prompts);
            }
        }

        walk(root, source, file_path, &mut prompts);
    }

    // Fallback: regex-style scan for any .preamble("...")
    if prompts.is_empty() {
        for (_i, line) in source.lines().enumerate() {
            if line.contains(".preamble(\"") {
                if let Some(start) = line.find(".preamble(\"") {
                    let after = &line[start + 11..];
                    if let Some(end) = after.find('"') {
                        let content = &after[..end];
                        if !content.is_empty() {
                            prompts.push(ExtractedPrompt {
                                file: file_path.display().to_string(),
                                line: _i + 1,
                                text: content.to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    prompts
}

/// Very rough tool extraction (looks for .tool( or tool definitions).
pub fn find_tools(source: &str, file_path: &Path) -> Vec<ExtractedTool> {
    let mut tools = vec![];

    for (_i, line) in source.lines().enumerate() {
        if line.contains(".tool(") || line.contains("tool(") {
            // Try to grab a name near the call
            let name = if let Some(start) = line.find(".tool(") {
                let s = &line[start + 6..];
                s.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .find(|p| !p.is_empty())
                    .unwrap_or("tool")
                    .to_string()
            } else {
                "tool".to_string()
            };

            tools.push(ExtractedTool {
                file: file_path.display().to_string(),
                name,
                description: None,
            });
        }
    }

    tools
}

/// Heuristic: does this source contain a Rig agent?
pub fn looks_like_rig_agent(source: &str) -> bool {
    source.contains("rig::") || source.contains(".agent(") || source.contains("preamble(")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_preambles_basic() {
        let source = r#"
            let agent = client.agent("gpt-4o").preamble("You are helpful. Think step by step.").build();
        "#;
        let prompts = find_preambles(source, std::path::Path::new("test.rs"));
        assert!(!prompts.is_empty());
        assert!(prompts[0].text.contains("Think step by step"));
    }

    #[test]
    fn test_looks_like_rig_agent() {
        assert!(looks_like_rig_agent("let x = client.agent(\"..\").preamble(\"hi\")"));
        assert!(!looks_like_rig_agent("fn main() {}"));
    }

    #[test]
    fn test_find_run_agent_functions() {
        let source = "pub async fn run_agent(input: Input) -> Result<Output> { Ok(()) }";
        let fns = find_run_agent_functions(source);
        assert!(!fns.is_empty());
    }
}