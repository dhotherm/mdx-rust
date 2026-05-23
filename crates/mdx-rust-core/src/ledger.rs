//! Experiment budgeting and prompt variant ledger primitives.
//!
//! These records make optimization runs explainable without requiring a
//! database. They are append-friendly JSON structures that can later be moved
//! behind a richer storage layer.

use crate::eval::{stable_hash_hex, EvaluationDataset, EvaluationSample};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum OptimizationBudget {
    Light,
    #[default]
    Medium,
    Heavy,
}

impl OptimizationBudget {
    pub fn from_label(value: &str) -> anyhow::Result<Self> {
        match value {
            "light" => Ok(Self::Light),
            "medium" => Ok(Self::Medium),
            "heavy" => Ok(Self::Heavy),
            other => anyhow::bail!("unknown optimization budget: {other}"),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Medium => "medium",
            Self::Heavy => "heavy",
        }
    }

    pub fn candidate_limit(self, requested: u32) -> usize {
        let cap = match self {
            Self::Light => 2,
            Self::Medium => 4,
            Self::Heavy => 8,
        };
        requested.max(1).min(cap) as usize
    }

    pub fn holdout_percent(self) -> usize {
        match self {
            Self::Light => 20,
            Self::Medium => 25,
            Self::Heavy => 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSplit {
    pub train: Vec<EvaluationSample>,
    pub holdout: Vec<EvaluationSample>,
    pub holdout_percent: usize,
}

pub fn split_dataset(dataset: &EvaluationDataset, budget: OptimizationBudget) -> DatasetSplit {
    if dataset.samples.len() < 2 {
        return DatasetSplit {
            train: dataset.samples.clone(),
            holdout: Vec::new(),
            holdout_percent: 0,
        };
    }

    let holdout_percent = budget.holdout_percent();
    let holdout_len = ((dataset.samples.len() * holdout_percent).div_ceil(100))
        .max(1)
        .min(dataset.samples.len() - 1);
    let split_at = dataset.samples.len() - holdout_len;

    DatasetSplit {
        train: dataset.samples[..split_at].to_vec(),
        holdout: dataset.samples[split_at..].to_vec(),
        holdout_percent,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptVariantRecord {
    pub id: String,
    pub strategy: String,
    pub target_file: String,
    pub patch_hash: String,
    pub description: String,
}

impl PromptVariantRecord {
    pub fn from_patch(
        strategy: impl Into<String>,
        target_file: impl Into<String>,
        description: impl Into<String>,
        patch: &str,
    ) -> Self {
        let patch_hash = stable_hash_hex(patch.as_bytes());
        Self {
            id: patch_hash.replace(':', "_"),
            strategy: strategy.into(),
            target_file: target_file.into(),
            patch_hash,
            description: description.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentLedger {
    pub budget: OptimizationBudget,
    pub dataset_version: String,
    pub dataset_hash: String,
    pub train_samples: usize,
    pub holdout_samples: usize,
    pub variants: Vec<PromptVariantRecord>,
}

impl ExperimentLedger {
    pub fn new(
        budget: OptimizationBudget,
        dataset: &EvaluationDataset,
        split: &DatasetSplit,
    ) -> Self {
        Self {
            budget,
            dataset_version: dataset.version.clone(),
            dataset_hash: dataset.content_hash(),
            train_samples: split.train.len(),
            holdout_samples: split.holdout.len(),
            variants: Vec::new(),
        }
    }

    pub fn record_variant(&mut self, variant: PromptVariantRecord) {
        self.variants.push(variant);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_dataset_keeps_at_least_one_train_sample() {
        let dataset = EvaluationDataset::synthetic_v1();
        let split = split_dataset(&dataset, OptimizationBudget::Heavy);

        assert!(!split.train.is_empty());
        assert!(!split.holdout.is_empty());
        assert_eq!(
            split.train.len() + split.holdout.len(),
            dataset.samples.len()
        );
    }

    #[test]
    fn variant_ids_are_stable_hashes() {
        let first = PromptVariantRecord::from_patch("schema", "src/main.rs", "desc", "patch");
        let second = PromptVariantRecord::from_patch("schema", "src/main.rs", "desc", "patch");

        assert_eq!(first.id, second.id);
        assert!(first.patch_hash.starts_with("fnv1a64:"));
    }

    #[test]
    fn budget_caps_candidates_but_never_to_zero() {
        assert_eq!(OptimizationBudget::Light.candidate_limit(99), 2);
        assert_eq!(OptimizationBudget::Medium.candidate_limit(99), 4);
        assert_eq!(OptimizationBudget::Heavy.candidate_limit(99), 8);
        assert_eq!(OptimizationBudget::Light.candidate_limit(0), 1);
    }

    #[test]
    fn ledger_recording_variants_does_not_record_acceptance() {
        let dataset = EvaluationDataset::synthetic_v1();
        let split = split_dataset(&dataset, OptimizationBudget::Medium);
        let mut ledger = ExperimentLedger::new(OptimizationBudget::Medium, &dataset, &split);

        ledger.record_variant(PromptVariantRecord::from_patch(
            "schema",
            "src/main.rs",
            "candidate only",
            "patch",
        ));

        assert_eq!(ledger.variants.len(), 1);
        assert_eq!(
            ledger.train_samples + ledger.holdout_samples,
            dataset.samples.len()
        );
    }
}
