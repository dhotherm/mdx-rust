//! A minimal Rig-based agent used for testing mdx-rust itself.
//!
//! This agent is deliberately simple so we can dogfood the optimizer on it.

use rig::providers::openai;
use rig::completion::Prompt;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Deserialize, Serialize)]
struct AgentInput {
    pub query: String,
    pub context: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AgentOutput {
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

/// The main agent entrypoint (intentionally weak starting point for optimizer demo).
pub async fn run_agent(input: AgentInput) -> anyhow::Result<AgentOutput> {
    let api_key = env::var("OPENAI_API_KEY").ok();

    if api_key.is_some() {
        let client = openai::Client::from_env();
        let agent = client
            .agent("gpt-4o-mini")
            .preamble("You are a concise, helpful assistant. Think step-by-step before answering. Always explain your reasoning in one sentence, then give the final answer.")
            .build();

        let prompt = format!("Query: {}\nContext: {}", input.query, input.context.unwrap_or_default());
        let response = agent.prompt(prompt).await?;

        Ok(AgentOutput {
            answer: response,
            confidence: 0.8,
            reasoning: "LLM".to_string(),
        })
    } else {
        // Weak echo — the exact behavior the optimizer is supposed to detect and improve.
        Ok(AgentOutput {
            answer: format!("Echo: {}", input.query),
            confidence: 0.4,
            reasoning: "No key, echoing input.".to_string(),
        })
    }
}