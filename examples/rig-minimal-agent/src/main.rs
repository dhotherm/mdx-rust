//! A minimal Rig-based agent used for testing mdx-rust itself.
//!
//! This agent is deliberately simple so we can dogfood the optimizer on it.

use rig::completion::Prompt;
use rig::providers::openai;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentInput {
    pub query: String,
    pub context: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentOutput {
    pub answer: String,
    pub confidence: f32,
    pub reasoning: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Clean Process contract: always read JSON AgentInput from stdin, write JSON output.
    let input: AgentInput = serde_json::from_reader(std::io::stdin())?;
    let output = run_agent(input).await?;
    println!("{}", serde_json::to_string(&output)?);
    Ok(())
}

/// The main agent entrypoint (deliberately weak starting point for mdx-rust optimizer demo).
pub async fn run_agent(input: AgentInput) -> anyhow::Result<AgentOutput> {
    let api_key = env::var("OPENAI_API_KEY").ok();

    if api_key.is_some() {
        let client = openai::Client::from_env();
        let agent = client
            .agent("gpt-4o-mini")
            .preamble("You are a concise, helpful assistant.")
            .build();
        // Note: .tool(echo_tool) would be here in a fuller agent — the finder will still detect tool usage patterns in richer code.

        let prompt = format!(
            "Query: {}\nContext: {}",
            input.query,
            input.context.unwrap_or_default()
        );
        let response = agent.prompt(prompt).await?;

        Ok(AgentOutput {
            answer: response,
            confidence: 0.7,
            reasoning: "LLM response".to_string(),
        })
    } else {
        // Intentionally poor fallback. The optimizer should notice the echo pattern
        // and replace it with a more useful best-effort answer.
        let is_improved = false;

        Ok(AgentOutput {
            answer: if is_improved {
                format!("Better answer after reasoning step: {}", input.query)
            } else {
                format!("Echo: {}", input.query)
            },
            confidence: if is_improved { 0.72 } else { 0.35 },
            reasoning: if is_improved {
                "Applied step-by-step reasoning improvement from mdx-rust.".to_string()
            } else {
                "No API key, just repeating the question.".to_string()
            },
        })
    }
}
