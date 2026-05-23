//! Thin LLM client abstraction for the optimizer.
//!
//! Currently backed by Rig for convenience. Later we can support direct HTTP
//! for more control over "heavy reasoning" models.

use rig::providers::openai;
use rig::completion::Prompt;
use serde::Serialize;

/// Very simple diagnosis request.
#[derive(Serialize)]
pub struct DiagnosisRequest {
    pub policy: String,
    pub bundle_summary: String,   // path count + key files
    pub traces_summary: String,
    pub scores: Vec<f32>,
}

/// Result of asking the model for diagnosis + candidates.
#[derive(Debug, Clone)]
pub struct DiagnosisResult {
    pub summary: String,
    pub candidates: Vec<String>,   // textual description of each candidate
}

/// Basic LLM client for diagnosis.
pub struct LlmClient {
    model: String,
}

impl LlmClient {
    pub fn new(model: impl Into<String>) -> Self {
        Self { model: model.into() }
    }

    /// Ask a strong model for diagnosis and candidate ideas.
    pub async fn diagnose(&self, req: DiagnosisRequest) -> anyhow::Result<DiagnosisResult> {
        // Only attempt if we have a key (avoids panic on Client::from_env)
        if std::env::var("OPENAI_API_KEY").is_err() {
            return Err(anyhow::anyhow!("No OPENAI_API_KEY"));
        }

        let client = openai::Client::from_env();
        let agent = client
            .agent(&self.model)
            .preamble(
                "You are an expert at debugging and improving LLM agents written in Rust. \
                 Be concise, specific, and actionable. Suggest 2-3 targeted improvements \
                 (prompt changes, tool description changes, or small logic changes).",
            )
            .build();

        let prompt = format!(
            "Policy:\n{}\n\nCode analysis (extracted preambles, tools, structure):\n{}\n\nTraces summary:\n{}\n\nCurrent scores: {:?}\n\n\
             You are optimizing a Rust LLM agent. Give a short diagnosis and 2-3 concrete, minimal, safe candidate improvements \
             (e.g. stronger system prompt, better tool descriptions, added reasoning step). Be specific.",
            req.policy, req.bundle_summary, req.traces_summary, req.scores
        );

        let response = agent.prompt(prompt).await?;

        // Very naive parsing for now — in real version we would ask for JSON.
        let lines: Vec<&str> = response.lines().collect();
        let summary = lines.first().unwrap_or(&"Diagnosis unavailable").to_string();
        let candidates = lines
            .iter()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .take(3)
            .map(|s| s.trim().to_string())
            .collect();

        Ok(DiagnosisResult { summary, candidates })
    }
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new("gpt-4o")
    }
}