//! Thin LLM client abstraction for the optimizer.
//!
//! Currently backed by Rig for convenience. Later we can support direct HTTP
//! for more control over "heavy reasoning" models.

use rig::completion::Prompt;
use rig::providers::openai;
use serde::Serialize;

/// Very simple diagnosis request.
#[derive(Serialize)]
pub struct DiagnosisRequest {
    pub policy: String,
    pub bundle_summary: String, // path count + key files
    pub traces_summary: String,
    pub scores: Vec<f32>,
}

/// Structured candidate returned by the LLM.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct StructuredCandidate {
    pub focus: String, // "system_prompt", "tool_descriptions", "reasoning", "logic"
    pub description: String,
    pub expected_improvement: String,
}

/// Result of asking the model for diagnosis + candidates.
#[derive(Debug, Clone)]
pub struct DiagnosisResult {
    pub summary: String,
    pub candidates: Vec<StructuredCandidate>,
}

/// Basic LLM client for diagnosis.
pub struct LlmClient {
    model: String,
}

impl LlmClient {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
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
                 Always respond with a JSON object: {\"summary\": \"short diagnosis\", \"candidates\": [{\"focus\": \"system_prompt|tools|reasoning\", \"description\": \"...\", \"expected_improvement\": \"...\"}]}. \
                 Be concise and actionable. Focus on minimal, high-impact changes to prompts or control flow.",
            )
            .build();

        let prompt = format!(
            "Policy:\n{}\n\nCode analysis (extracted preambles, tools, structure):\n{}\n\nTraces summary:\n{}\n\nCurrent scores: {:?}\n\n\
             Return ONLY a JSON object with keys \"summary\" and \"candidates\" (array of objects with focus, description, expected_improvement). No prose outside the JSON.",
            req.policy, req.bundle_summary, req.traces_summary, req.scores
        );

        let response = agent.prompt(prompt).await?;

        // Try to parse structured JSON from the model response.
        // We instruct the model to return a JSON object with "summary" and "candidates".
        #[derive(serde::Deserialize)]
        struct LlmOutput {
            summary: Option<String>,
            candidates: Option<Vec<StructuredCandidate>>,
        }

        let parsed: Option<LlmOutput> = serde_json::from_str(&response).ok().or_else(|| {
            // Sometimes the model wraps it in ```json ... ```
            if let Some(start) = response.find('{') {
                if let Some(end) = response.rfind('}') {
                    serde_json::from_str(&response[start..=end]).ok()
                } else {
                    None
                }
            } else {
                None
            }
        });

        if let Some(out) = parsed {
            let summary = out
                .summary
                .unwrap_or_else(|| "LLM diagnosis completed.".to_string());
            let candidates = out.candidates.unwrap_or_default();
            if !candidates.is_empty() {
                return Ok(DiagnosisResult {
                    summary,
                    candidates,
                });
            }
        }

        // Fallback: treat the whole response as a single textual candidate
        let summary = response
            .lines()
            .next()
            .unwrap_or("LLM returned free text")
            .to_string();
        let fallback = vec![StructuredCandidate {
            focus: "system_prompt".to_string(),
            description: response.chars().take(280).collect(),
            expected_improvement: "Model suggestion".to_string(),
        }];

        Ok(DiagnosisResult {
            summary,
            candidates: fallback,
        })
    }
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new("gpt-4o")
    }
}
