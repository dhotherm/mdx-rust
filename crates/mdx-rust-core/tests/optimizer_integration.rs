//! Integration tests for the full optimization loop.
//! These exercise tracing, scoring, diagnosis, candidate generation, and safe editing.

use mdx_rust_core::mechanical_score;
use mdx_rust_core::optimizer::{run_optimization, OptimizeConfig};
use mdx_rust_core::registry::{AgentContract, RegisteredAgent};
use mdx_rust_core::runner::run_agent;
use mdx_rust_core::{HookPolicy, OptimizationBudget};
use tempfile::tempdir;

#[tokio::test]
async fn test_optimizer_runs_without_crashing_on_temp_agent() {
    let tmp = tempdir().unwrap();
    let agent_dir = tmp.path().join("test-agent");
    std::fs::create_dir_all(&agent_dir).unwrap();

    // Minimal agent structure so the runner and analysis don't explode
    std::fs::write(
        agent_dir.join("Cargo.toml"),
        r#"
        [package]
        name = "test-agent"
        version = "0.1.0"
        edition = "2021"
    "#,
    )
    .unwrap();

    std::fs::create_dir_all(agent_dir.join("src")).unwrap();
    std::fs::write(
        agent_dir.join("src/main.rs"),
        r#"
        use serde::{Deserialize, Serialize};

        #[derive(Deserialize)]
        struct AgentInput { query: String }

        #[derive(Serialize)]
        struct AgentOutput { answer: String, confidence: f32, reasoning: String }

        pub async fn run_agent(input: AgentInput) -> anyhow::Result<AgentOutput> {
            Ok(AgentOutput {
                answer: format!("Echo: {}", input.query),
                confidence: 0.4,
                reasoning: "weak fallback".to_string(),
            })
        }
    "#,
    )
    .unwrap();

    let agent = RegisteredAgent {
        name: "integration-test-agent".to_string(),
        path: agent_dir,
        contract: AgentContract::NativeRust,
        registered_at: "2026-05-23".to_string(),
    };

    let cfg = OptimizeConfig {
        max_iterations: 1,
        candidates_per_iteration: 1,
        use_llm_judge: false,
        budget: OptimizationBudget::Light,
        hook_policy: HookPolicy::default(),
        review_before_apply: false,
        quiet: false,
        candidate_timeout: std::time::Duration::from_secs(300),
    };

    let result = run_optimization(&agent, &cfg).await;
    assert!(
        result.is_ok(),
        "optimizer should not crash on a minimal agent"
    );

    let runs = result.unwrap();
    assert!(!runs.is_empty());
    assert_eq!(runs[0].iteration, 0);
}

#[tokio::test]
async fn optimizer_loop_denied_candidate_cannot_land_or_accept() {
    let tmp = tempdir().unwrap();
    let agent_dir = tmp.path().join("echo-agent");
    std::fs::create_dir_all(agent_dir.join("src")).unwrap();

    std::fs::write(
        agent_dir.join("Cargo.toml"),
        r#"
        [package]
        name = "echo-agent"
        version = "0.1.0"
        edition = "2021"
    "#,
    )
    .unwrap();

    let source = r#"
use std::io::BufRead;

fn main() {
    let mut input = String::new();
    std::io::stdin().lock().read_line(&mut input).unwrap();
    println!("{{\"answer\":\"Echo: unchanged\",\"confidence\":0.35,\"reasoning\":\"weak fallback\"}}");
}
"#;
    let source_path = agent_dir.join("src/main.rs");
    std::fs::write(&source_path, source).unwrap();

    let agent = RegisteredAgent {
        name: "echo-agent".to_string(),
        path: agent_dir,
        contract: AgentContract::Process,
        registered_at: "2026-05-23".to_string(),
    };

    let cfg = OptimizeConfig {
        max_iterations: 1,
        candidates_per_iteration: 2,
        use_llm_judge: false,
        budget: OptimizationBudget::Medium,
        hook_policy: HookPolicy {
            max_patch_bytes: 1,
            require_positive_delta: true,
        },
        review_before_apply: false,
        quiet: true,
        candidate_timeout: std::time::Duration::from_secs(300),
    };

    let runs = run_optimization(&agent, &cfg).await.unwrap();
    let run = &runs[0];

    assert_eq!(run.validated_changes, 0);
    assert_eq!(run.landed_changes, 0);
    assert_eq!(run.accepted_changes, 0);
    assert!(run.hook_decisions.iter().any(|decision| decision.denied()));
    assert_eq!(std::fs::read_to_string(source_path).unwrap(), source);
}

#[tokio::test]
async fn optimizer_loop_accepts_real_improvement_with_complete_audit_packet() {
    let tmp = tempdir().unwrap();
    let agent_dir = tmp.path().join("improvable-agent");
    std::fs::create_dir_all(agent_dir.join("src")).unwrap();

    std::fs::write(
        agent_dir.join("Cargo.toml"),
        r#"
        [package]
        name = "improvable-agent"
        version = "0.1.0"
        edition = "2021"
    "#,
    )
    .unwrap();

    let source_path = agent_dir.join("src/main.rs");
    std::fs::write(
        &source_path,
        r#"
use std::io::BufRead;

fn main() {
    let mut input = String::new();
    std::io::stdin().lock().read_line(&mut input).unwrap();
    let query = input.trim().replace('"', "'");
    println!(
        "{{\"answer\":\"Echo: {}\",\"confidence\":0.35,\"reasoning\":\"weak fallback\"}}",
        query
    );
}
"#,
    )
    .unwrap();

    let agent = RegisteredAgent {
        name: "improvable-agent".to_string(),
        path: agent_dir.clone(),
        contract: AgentContract::Process,
        registered_at: "2026-05-23".to_string(),
    };

    let baseline = run_agent(&agent, serde_json::json!({"query":"What is 2+2?"}))
        .await
        .unwrap();
    let baseline_score = mechanical_score(&baseline);

    let cfg = OptimizeConfig {
        max_iterations: 1,
        candidates_per_iteration: 2,
        use_llm_judge: false,
        budget: OptimizationBudget::Medium,
        hook_policy: HookPolicy::default(),
        review_before_apply: false,
        quiet: true,
        candidate_timeout: std::time::Duration::from_secs(300),
    };

    let runs = run_optimization(&agent, &cfg).await.unwrap();
    let run = &runs[0];

    assert_eq!(run.validated_changes, 1);
    assert_eq!(run.landed_changes, 1);
    assert_eq!(run.accepted_changes, 1);
    assert!(run.score_delta.unwrap() > 0.0);

    let final_result = run_agent(&agent, serde_json::json!({"query":"What is 2+2?"}))
        .await
        .unwrap();
    assert!(mechanical_score(&final_result) > baseline_score);
    assert!(std::fs::read_to_string(source_path)
        .unwrap()
        .contains("Best-effort answer after reasoning"));

    let packet = run
        .audit_packet
        .as_ref()
        .expect("accepted run should include an audit packet");
    assert_eq!(packet.schema_version, "0.2");
    assert_eq!(packet.edit_scope_contract, "single-file-v0.2");
    assert!(packet.accepted_edit.diff_hash.starts_with("fnv1a64:"));
    assert_eq!(packet.provenance.dataset_version, "synthetic_v1");
    assert_eq!(packet.provenance.scorer_id, "mechanical");
    assert_eq!(packet.provenance.scorer_version, "v1");
    assert!(!packet.validation_command_records.is_empty());
    assert!(!packet.final_validation_command_records.is_empty());
    assert!(packet.scores.score_delta > 0.0);
}
