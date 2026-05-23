//! Lifecycle hooks for the safe optimization pipeline.
//!
//! Hooks are deliberately boring: deterministic inputs, deterministic
//! decisions, no shell execution. External hook runners can come later, after
//! the built-in contract has proven stable.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HookStage {
    PreEdit,
    PostEdit,
    PreCommand,
    PostValidation,
    PreAccept,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HookAction {
    Allow,
    Warn,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    pub stage: HookStage,
    pub agent_name: String,
    #[serde(default)]
    pub edit_description: Option<String>,
    #[serde(default)]
    pub patch_bytes: usize,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub validation_passed: Option<bool>,
    #[serde(default)]
    pub score_delta: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookDecision {
    pub stage: HookStage,
    pub action: HookAction,
    pub reason: String,
}

impl HookDecision {
    pub fn allow(stage: HookStage, reason: impl Into<String>) -> Self {
        Self {
            stage,
            action: HookAction::Allow,
            reason: reason.into(),
        }
    }

    pub fn warn(stage: HookStage, reason: impl Into<String>) -> Self {
        Self {
            stage,
            action: HookAction::Warn,
            reason: reason.into(),
        }
    }

    pub fn deny(stage: HookStage, reason: impl Into<String>) -> Self {
        Self {
            stage,
            action: HookAction::Deny,
            reason: reason.into(),
        }
    }

    pub fn denied(&self) -> bool {
        self.action == HookAction::Deny
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookPolicy {
    pub max_patch_bytes: usize,
    pub require_positive_delta: bool,
}

impl Default for HookPolicy {
    fn default() -> Self {
        Self {
            max_patch_bytes: 32 * 1024,
            require_positive_delta: true,
        }
    }
}

pub fn evaluate_builtin_hook(policy: &HookPolicy, context: &HookContext) -> HookDecision {
    match context.stage {
        HookStage::PreEdit if context.patch_bytes > policy.max_patch_bytes => HookDecision::deny(
            HookStage::PreEdit,
            format!(
                "patch is too large: {} bytes exceeds {}",
                context.patch_bytes, policy.max_patch_bytes
            ),
        ),
        HookStage::PostValidation if context.validation_passed == Some(false) => {
            HookDecision::deny(HookStage::PostValidation, "validation failed")
        }
        HookStage::PreAccept
            if policy.require_positive_delta
                && context.score_delta.is_some_and(|delta| delta <= 0.0) =>
        {
            HookDecision::deny(HookStage::PreAccept, "score delta is not positive")
        }
        HookStage::PreCommand => HookDecision::allow(
            HookStage::PreCommand,
            context
                .command
                .as_deref()
                .map(|command| format!("command allowed: {command}"))
                .unwrap_or_else(|| "no command supplied".to_string()),
        ),
        ref stage => HookDecision::allow(stage.clone(), "built-in policy allowed stage"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_patch_is_denied() {
        let context = HookContext {
            stage: HookStage::PreEdit,
            agent_name: "agent".to_string(),
            edit_description: None,
            patch_bytes: 99,
            command: None,
            validation_passed: None,
            score_delta: None,
        };
        let policy = HookPolicy {
            max_patch_bytes: 10,
            require_positive_delta: true,
        };

        let decision = evaluate_builtin_hook(&policy, &context);

        assert!(decision.denied());
    }

    #[test]
    fn non_positive_acceptance_delta_is_denied() {
        let context = HookContext {
            stage: HookStage::PreAccept,
            agent_name: "agent".to_string(),
            edit_description: None,
            patch_bytes: 0,
            command: None,
            validation_passed: None,
            score_delta: Some(0.0),
        };

        let decision = evaluate_builtin_hook(&HookPolicy::default(), &context);

        assert_eq!(decision.action, HookAction::Deny);
    }
}
