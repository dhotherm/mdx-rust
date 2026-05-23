//! Trace diagnosis primitives.
//!
//! This is the bridge between raw runner traces and future targeted fixes.
//! Today it summarizes obvious run-level failures. As trace spans become
//! richer, this module should become the place that maps span failures to
//! candidate edit strategies.

use crate::runner::AgentRunResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FailureKind {
    Timeout,
    ProcessError,
    InvalidJson,
    EchoFallback,
    LowConfidence,
    MissingReasoning,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureSignal {
    pub kind: FailureKind,
    pub severity: u8,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraceDiagnosis {
    pub signals: Vec<FailureSignal>,
}

impl TraceDiagnosis {
    pub fn has_failures(&self) -> bool {
        !self.signals.is_empty()
    }

    pub fn compact_summary(&self) -> String {
        if self.signals.is_empty() {
            return "no obvious trace failures".to_string();
        }

        self.signals
            .iter()
            .map(|signal| format!("{:?}:{}", signal.kind, signal.severity))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub fn diagnose_run(result: &AgentRunResult) -> TraceDiagnosis {
    let mut signals = Vec::new();

    if !result.success {
        let error = result
            .error
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let kind = if error.to_lowercase().contains("timeout") {
            FailureKind::Timeout
        } else {
            FailureKind::ProcessError
        };
        signals.push(FailureSignal {
            kind,
            severity: 3,
            evidence: truncate(&error, 240),
        });
    }

    if result.output.get("raw").is_some() {
        signals.push(FailureSignal {
            kind: FailureKind::InvalidJson,
            severity: 2,
            evidence: "agent stdout was not valid JSON".to_string(),
        });
    }

    if let Some(answer) = result.output.get("answer").and_then(|value| value.as_str()) {
        if answer.starts_with("Echo:") {
            signals.push(FailureSignal {
                kind: FailureKind::EchoFallback,
                severity: 2,
                evidence: truncate(answer, 160),
            });
        }
    }

    if let Some(confidence) = result
        .output
        .get("confidence")
        .and_then(|value| value.as_f64())
    {
        if confidence < 0.5 {
            signals.push(FailureSignal {
                kind: FailureKind::LowConfidence,
                severity: 1,
                evidence: format!("confidence={confidence:.2}"),
            });
        }
    }

    let reasoning = result
        .output
        .get("reasoning")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if reasoning.trim().is_empty() {
        signals.push(FailureSignal {
            kind: FailureKind::MissingReasoning,
            severity: 1,
            evidence: "reasoning field missing or empty".to_string(),
        });
    }

    TraceDiagnosis { signals }
}

fn truncate(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        value.to_string()
    } else {
        let truncated: String = value.chars().take(limit).collect();
        format!("{truncated}...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::AgentRunResult;

    #[test]
    fn diagnose_echo_fallback_and_low_confidence() {
        let result = AgentRunResult {
            output: serde_json::json!({
                "answer": "Echo: hello",
                "confidence": 0.2,
                "reasoning": ""
            }),
            duration_ms: 1,
            success: true,
            error: None,
            traces: vec![],
        };

        let diagnosis = diagnose_run(&result);

        assert!(diagnosis.has_failures());
        assert!(diagnosis
            .signals
            .iter()
            .any(|signal| signal.kind == FailureKind::EchoFallback));
        assert!(diagnosis
            .signals
            .iter()
            .any(|signal| signal.kind == FailureKind::LowConfidence));
    }
}
