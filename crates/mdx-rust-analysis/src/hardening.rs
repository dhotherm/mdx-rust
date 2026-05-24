//! Conservative Rust hardening analysis for ordinary Rust modules.
//!
//! This module intentionally starts with high-confidence static patterns. It
//! can inspect normal Rust crates without requiring agent registration.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HardeningAnalysis {
    pub root: PathBuf,
    pub target: Option<PathBuf>,
    pub files_scanned: usize,
    pub findings: Vec<HardeningFinding>,
    pub changes: Vec<HardeningFileChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HardeningFinding {
    pub id: String,
    pub title: String,
    pub description: String,
    pub file: PathBuf,
    pub line: usize,
    pub strategy: HardeningStrategy,
    pub patchable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum HardeningStrategy {
    BorrowParameterTightening,
    ClonePressureReview,
    ErrorContextPropagation,
    IteratorCloned,
    LenCheckIsEmpty,
    LongFunctionReview,
    MechanicalTier1Cleanup,
    MustUsePublicReturn,
    OptionContextPropagation,
    RepeatedStringLiteralConst,
    ResultUnwrapContext,
    ProcessExecutionReview,
    UnsafeReview,
    EnvAccessReview,
    FileIoReview,
    HttpSurfaceReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HardeningFileChange {
    pub file: PathBuf,
    pub old_content: String,
    pub new_content: String,
    pub strategy: HardeningStrategy,
    pub finding_ids: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, Copy)]
pub struct HardeningAnalyzeConfig<'a> {
    pub target: Option<&'a Path>,
    pub max_files: usize,
    pub max_recipe_tier: u8,
    pub evidence_depth: HardeningEvidenceDepth,
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
pub enum HardeningEvidenceDepth {
    Basic,
    Tested,
    Covered,
    Hardened,
    Proven,
}

pub fn analyze_hardening(
    root: &Path,
    config: HardeningAnalyzeConfig<'_>,
) -> anyhow::Result<HardeningAnalysis> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let files = collect_rust_files(&root, config.target)?;
    let mut findings = Vec::new();
    let mut changes = Vec::new();

    for file in files.iter().take(config.max_files) {
        let content = std::fs::read_to_string(file)?;
        let rel = relative_path(&root, file);
        let function_ranges = find_function_ranges(&content);

        for (index, line) in content.lines().enumerate() {
            let line_no = index + 1;
            let pattern_line = line_without_comments_or_strings(line);
            let trimmed = pattern_line.trim();

            if trimmed.contains("Command::new(") || trimmed.contains("std::process::Command") {
                findings.push(HardeningFinding {
                    id: format!("process-execution:{}:{line_no}", rel.display()),
                    title: "Process execution surface".to_string(),
                    description:
                        "External process execution should have explicit input validation or allowlisting."
                            .to_string(),
                    file: rel.clone(),
                    line: line_no,
                    strategy: HardeningStrategy::ProcessExecutionReview,
                    patchable: false,
                });
            }

            if trimmed.contains("unsafe ") || trimmed == "unsafe" || trimmed.contains("unsafe{") {
                findings.push(HardeningFinding {
                    id: format!("unsafe-rust:{}:{line_no}", rel.display()),
                    title: "Unsafe Rust requires review".to_string(),
                    description:
                        "Unsafe code should be isolated and documented before automated edits touch it."
                            .to_string(),
                    file: rel.clone(),
                    line: line_no,
                    strategy: HardeningStrategy::UnsafeReview,
                    patchable: false,
                });
            }

            if trimmed.contains("std::env::var(") || trimmed.contains("env::var(") {
                findings.push(HardeningFinding {
                    id: format!("env-access:{}:{line_no}", rel.display()),
                    title: "Environment variable access".to_string(),
                    description:
                        "Environment-derived configuration should return contextual errors at boundaries."
                            .to_string(),
                    file: rel.clone(),
                    line: line_no,
                    strategy: HardeningStrategy::EnvAccessReview,
                    patchable: false,
                });
            }

            let filesystem_call = trimmed.contains("std::fs::read_to_string(")
                || trimmed.contains("fs::read_to_string(")
                || trimmed.contains("std::fs::write(")
                || trimmed.contains("fs::write(");
            let has_visible_error_handling = trimmed.contains('?')
                || trimmed.contains(".unwrap(")
                || trimmed.contains(".expect(");
            if filesystem_call && !has_visible_error_handling {
                findings.push(HardeningFinding {
                    id: format!("file-io:{}:{line_no}", rel.display()),
                    title: "Filesystem boundary".to_string(),
                    description:
                        "Filesystem access should preserve contextual errors and validated paths."
                            .to_string(),
                    file: rel.clone(),
                    line: line_no,
                    strategy: HardeningStrategy::FileIoReview,
                    patchable: false,
                });
            }

            if trimmed.contains("Router::new(")
                || trimmed.contains(".route(")
                || trimmed.contains("#[get(")
                || trimmed.contains("#[post(")
            {
                findings.push(HardeningFinding {
                    id: format!("http-surface:{}:{line_no}", rel.display()),
                    title: "HTTP or route surface".to_string(),
                    description:
                        "HTTP-facing surfaces should validate inputs and preserve typed errors."
                            .to_string(),
                    file: rel.clone(),
                    line: line_no,
                    strategy: HardeningStrategy::HttpSurfaceReview,
                    patchable: false,
                });
            }
        }

        if config.evidence_depth >= HardeningEvidenceDepth::Hardened {
            add_hardened_evidence_findings(&rel, &content, &function_ranges, &mut findings);
        }

        if let Some(change) =
            build_mechanical_change(&root, file, &content, &function_ranges, &config)?
        {
            findings.extend(change.findings);
            changes.push(change.change);
        }
    }

    Ok(HardeningAnalysis {
        root,
        target: config.target.map(Path::to_path_buf),
        files_scanned: files.len().min(config.max_files),
        findings,
        changes,
    })
}

struct MechanicalChange {
    change: HardeningFileChange,
    findings: Vec<HardeningFinding>,
}

fn build_mechanical_change(
    root: &Path,
    file: &Path,
    content: &str,
    function_ranges: &[FunctionRange],
    config: &HardeningAnalyzeConfig<'_>,
) -> anyhow::Result<Option<MechanicalChange>> {
    let rel = relative_path(root, file);
    let mut lines: Vec<String> = content.lines().map(ToString::to_string).collect();
    let mut finding_ids = Vec::new();
    let mut findings = Vec::new();

    apply_result_context_recipe(
        &rel,
        &mut lines,
        function_ranges,
        &mut finding_ids,
        &mut findings,
    );
    apply_error_context_recipe(
        &rel,
        &mut lines,
        function_ranges,
        &mut finding_ids,
        &mut findings,
    );
    apply_borrow_parameter_recipe(
        &rel,
        &mut lines,
        function_ranges,
        &mut finding_ids,
        &mut findings,
    );
    apply_borrowed_vec_literal_recipe(&rel, &mut lines, &mut finding_ids, &mut findings);
    apply_iterator_cloned_recipe(&rel, &mut lines, &mut finding_ids, &mut findings);
    apply_must_use_recipe(
        &rel,
        &mut lines,
        function_ranges,
        &mut finding_ids,
        &mut findings,
    );
    if config.max_recipe_tier >= 2 {
        apply_len_check_is_empty_recipe(&rel, &mut lines, &mut finding_ids, &mut findings);
        apply_option_context_recipe(
            &rel,
            &mut lines,
            function_ranges,
            &mut finding_ids,
            &mut findings,
        );
        apply_repeated_string_literal_const_recipe(
            &rel,
            &mut lines,
            &mut finding_ids,
            &mut findings,
        );
    }

    if finding_ids.is_empty() {
        return Ok(None);
    }

    let mut new_content = lines.join("\n");
    if content.ends_with('\n') {
        new_content.push('\n');
    }
    if findings.iter().any(|finding| {
        matches!(
            finding.strategy,
            HardeningStrategy::ErrorContextPropagation
                | HardeningStrategy::ResultUnwrapContext
                | HardeningStrategy::OptionContextPropagation
        )
    }) {
        new_content = ensure_anyhow_context_import(&new_content);
    }
    if syn::parse_file(&new_content).is_err() {
        return Ok(None);
    }

    Ok(Some(MechanicalChange {
        change: HardeningFileChange {
            file: rel,
            old_content: content.to_string(),
            new_content,
            strategy: HardeningStrategy::MechanicalTier1Cleanup,
            finding_ids,
            description:
                "Apply enabled mechanical hardening recipes under compile and clippy validation."
                    .to_string(),
        },
        findings,
    }))
}

fn add_hardened_evidence_findings(
    rel: &Path,
    content: &str,
    function_ranges: &[FunctionRange],
    findings: &mut Vec<HardeningFinding>,
) {
    let mut clone_lines = Vec::new();
    for (index, line) in content.lines().enumerate() {
        let pattern_line = line_without_comments_or_strings(line);
        if pattern_line.contains(".clone()") {
            clone_lines.push(index + 1);
        }
    }
    if clone_lines.len() >= 3 {
        findings.push(HardeningFinding {
            id: format!("clone-pressure-review:{}:{}", rel.display(), clone_lines[0]),
            title: "Clone pressure review".to_string(),
            description: format!(
                "Hardened evidence unlocks deeper clone-pressure analysis; this file has {} visible clone callsites for future semantic cleanup.",
                clone_lines.len()
            ),
            file: rel.to_path_buf(),
            line: clone_lines[0],
            strategy: HardeningStrategy::ClonePressureReview,
            patchable: false,
        });
    }

    for range in function_ranges {
        let function_len = range.end_line.saturating_sub(range.start_line) + 1;
        if function_len >= 50 {
            findings.push(HardeningFinding {
                id: format!(
                    "long-function-review:{}:{}",
                    rel.display(),
                    range.signature_start_line
                ),
                title: "Long function refactor candidate".to_string(),
                description: format!(
                    "Hardened evidence unlocks deeper function-shape analysis; `{}` spans {function_len} lines and may be ready for extract-function planning.",
                    range.name
                ),
                file: rel.to_path_buf(),
                line: range.signature_start_line,
                strategy: HardeningStrategy::LongFunctionReview,
                patchable: false,
            });
        }
    }
}

fn apply_result_context_recipe(
    rel: &Path,
    lines: &mut [String],
    function_ranges: &[FunctionRange],
    finding_ids: &mut Vec<String>,
    findings: &mut Vec<HardeningFinding>,
) {
    for range in function_ranges {
        if !range.returns_anyhow_result {
            continue;
        }

        for line_index in range.start_line.saturating_sub(1)..range.end_line.min(lines.len()) {
            let original = lines[line_index].clone();
            if original.trim_start().starts_with("//") {
                continue;
            }

            let mut rewritten = original.clone();
            if rewritten.contains(".unwrap()") {
                rewritten = rewritten.replace(
                    ".unwrap()",
                    &format!(".context(\"{} failed instead of panicking\")?", range.name),
                );
            }
            rewritten = replace_expect_calls(&rewritten);

            if rewritten != original {
                lines[line_index] = rewritten;
                let line = line_index + 1;
                let id = format!("unwrap-in-result:{}:{line}", rel.display());
                finding_ids.push(id.clone());
                findings.push(HardeningFinding {
                    id,
                    title: "Panic-prone unwrap in anyhow Result function".to_string(),
                    description: "Replace unwrap/expect with anyhow Context and ? so failure is reported instead of panicking.".to_string(),
                    file: rel.to_path_buf(),
                    line,
                    strategy: HardeningStrategy::ResultUnwrapContext,
                    patchable: true,
                });
            }
        }
    }
}

fn apply_error_context_recipe(
    rel: &Path,
    lines: &mut [String],
    function_ranges: &[FunctionRange],
    finding_ids: &mut Vec<String>,
    findings: &mut Vec<HardeningFinding>,
) {
    for range in function_ranges {
        if !range.returns_anyhow_result {
            continue;
        }

        for line_index in range.start_line.saturating_sub(1)..range.end_line.min(lines.len()) {
            let original = lines[line_index].clone();
            if original.trim_start().starts_with("//")
                || original.contains(".context(")
                || original.contains(".with_context(")
            {
                continue;
            }

            let pattern_line = line_without_comments_or_strings(&original);
            let Some(boundary) = boundary_call_kind(&pattern_line) else {
                continue;
            };
            if !pattern_line.contains('?') {
                continue;
            }

            let Some(rewritten) = add_context_before_question_mark(
                &original,
                &format!("{} failed at {boundary} boundary", range.name),
            ) else {
                continue;
            };
            if rewritten == original {
                continue;
            }

            lines[line_index] = rewritten;
            let line = line_index + 1;
            let id = format!("error-context-propagation:{}:{line}", rel.display());
            finding_ids.push(id.clone());
            findings.push(HardeningFinding {
                id,
                title: "Propagate boundary errors with context".to_string(),
                description: "Add anyhow Context to fallible boundary calls that already use ? so failures explain where they came from.".to_string(),
                file: rel.to_path_buf(),
                line,
                strategy: HardeningStrategy::ErrorContextPropagation,
                patchable: true,
            });
        }
    }
}

fn boundary_call_kind(line: &str) -> Option<&'static str> {
    if line.contains("std::fs::")
        || line.contains("fs::read")
        || line.contains("fs::write")
        || line.contains("File::open(")
    {
        Some("filesystem")
    } else if line.contains("std::env::var(") || line.contains("env::var(") {
        Some("environment")
    } else {
        None
    }
}

fn add_context_before_question_mark(line: &str, message: &str) -> Option<String> {
    let question = line.find('?')?;
    let (before, after) = line.split_at(question);
    Some(format!(
        "{}.context(\"{}\"){}",
        before,
        escape_string(message),
        after
    ))
}

fn apply_borrow_parameter_recipe(
    rel: &Path,
    lines: &mut [String],
    function_ranges: &[FunctionRange],
    finding_ids: &mut Vec<String>,
    findings: &mut Vec<HardeningFinding>,
) {
    for range in function_ranges {
        if range.is_public {
            continue;
        }

        let start = range.signature_start_line.saturating_sub(1);
        let end = range.signature_end_line.min(lines.len());
        let mut changed = false;
        for line in &mut lines[start..end] {
            let original = line.clone();
            let tightened = tighten_borrow_parameters(&original);
            if tightened != original {
                *line = tightened;
                changed = true;
            }
        }

        if changed {
            let id = format!(
                "borrow-parameter-tightening:{}:{}",
                rel.display(),
                range.signature_start_line
            );
            finding_ids.push(id.clone());
            findings.push(HardeningFinding {
                id,
                title: "Tighten private borrowed parameter type".to_string(),
                description: "Prefer &str and slices over borrowed owned containers in private functions when compile gates prove the change.".to_string(),
                file: rel.to_path_buf(),
                line: range.signature_start_line,
                strategy: HardeningStrategy::BorrowParameterTightening,
                patchable: true,
            });
        }
    }
}

fn apply_must_use_recipe(
    rel: &Path,
    lines: &mut Vec<String>,
    function_ranges: &[FunctionRange],
    finding_ids: &mut Vec<String>,
    findings: &mut Vec<HardeningFinding>,
) {
    let mut inserted = 0usize;
    for range in function_ranges {
        if !range.is_public || !range.returns_value || range.returns_common_must_use {
            continue;
        }
        if has_nearby_must_use(lines, range.signature_start_line + inserted) {
            continue;
        }

        let insert_at = range.signature_start_line.saturating_sub(1) + inserted;
        let indent: String = lines
            .get(insert_at)
            .map(|line| line.chars().take_while(|ch| ch.is_whitespace()).collect())
            .unwrap_or_default();
        lines.insert(insert_at, format!("{indent}#[must_use]"));
        inserted += 1;

        let id = format!(
            "must-use-public-return:{}:{}",
            rel.display(),
            range.signature_start_line
        );
        finding_ids.push(id.clone());
        findings.push(HardeningFinding {
            id,
            title: "Public return value should be marked must_use".to_string(),
            description: "Add #[must_use] to public value-returning functions so ignored results are visible to callers.".to_string(),
            file: rel.to_path_buf(),
            line: range.signature_start_line,
            strategy: HardeningStrategy::MustUsePublicReturn,
            patchable: true,
        });
    }
}

fn apply_iterator_cloned_recipe(
    rel: &Path,
    lines: &mut [String],
    finding_ids: &mut Vec<String>,
    findings: &mut Vec<HardeningFinding>,
) {
    for (line_index, line) in lines.iter_mut().enumerate() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        let original = line.clone();
        let rewritten = replace_map_clone_calls(&original);
        if rewritten == original {
            continue;
        }

        *line = rewritten;
        let line_no = line_index + 1;
        let id = format!("iterator-cloned:{}:{line_no}", rel.display());
        finding_ids.push(id.clone());
        findings.push(HardeningFinding {
            id,
            title: "Simplify iterator clone collection".to_string(),
            description: "Replace clone-mapping collection with a simpler form when compile gates prove the iterator item type.".to_string(),
            file: rel.to_path_buf(),
            line: line_no,
            strategy: HardeningStrategy::IteratorCloned,
            patchable: true,
        });
    }
}

fn apply_borrowed_vec_literal_recipe(
    rel: &Path,
    lines: &mut [String],
    finding_ids: &mut Vec<String>,
    findings: &mut Vec<HardeningFinding>,
) {
    for (line_index, line) in lines.iter_mut().enumerate() {
        if line.trim_start().starts_with("//") || !line.contains("&vec![") {
            continue;
        }

        *line = line.replace("&vec![", "&[");
        let line_no = line_index + 1;
        let id = format!("borrowed-vec-literal:{}:{line_no}", rel.display());
        finding_ids.push(id.clone());
        findings.push(HardeningFinding {
            id,
            title: "Use a borrowed slice literal".to_string(),
            description: "Replace &vec![..] with a borrowed slice literal when validation proves the callsite.".to_string(),
            file: rel.to_path_buf(),
            line: line_no,
            strategy: HardeningStrategy::BorrowParameterTightening,
            patchable: true,
        });
    }
}

fn apply_len_check_is_empty_recipe(
    rel: &Path,
    lines: &mut [String],
    finding_ids: &mut Vec<String>,
    findings: &mut Vec<HardeningFinding>,
) {
    for (line_index, line) in lines.iter_mut().enumerate() {
        if line.trim_start().starts_with("//") || !line.contains(".len() == 0") {
            continue;
        }
        let original = line.clone();
        let rewritten = original.replace(".len() == 0", ".is_empty()");
        if rewritten == original {
            continue;
        }

        *line = rewritten;
        let line_no = line_index + 1;
        let id = format!("len-check-is-empty:{}:{line_no}", rel.display());
        finding_ids.push(id.clone());
        findings.push(HardeningFinding {
            id,
            title: "Use is_empty for zero-length check".to_string(),
            description: "Replace len() == 0 with is_empty() under Tier 2 evidence gates and compile validation.".to_string(),
            file: rel.to_path_buf(),
            line: line_no,
            strategy: HardeningStrategy::LenCheckIsEmpty,
            patchable: true,
        });
    }
}

fn apply_option_context_recipe(
    rel: &Path,
    lines: &mut [String],
    function_ranges: &[FunctionRange],
    finding_ids: &mut Vec<String>,
    findings: &mut Vec<HardeningFinding>,
) {
    for range in function_ranges {
        if !range.returns_anyhow_result {
            continue;
        }

        for line_index in range.start_line.saturating_sub(1)..range.end_line.min(lines.len()) {
            let original = lines[line_index].clone();
            if original.trim_start().starts_with("//")
                || original.contains(".context(")
                || original.contains(".with_context(")
            {
                continue;
            }

            let Some(rewritten) = replace_ok_or_string_with_context(&original) else {
                continue;
            };
            if rewritten == original {
                continue;
            }

            lines[line_index] = rewritten;
            let line = line_index + 1;
            let id = format!("option-context-propagation:{}:{line}", rel.display());
            finding_ids.push(id.clone());
            findings.push(HardeningFinding {
                id,
                title: "Propagate Option failure with context".to_string(),
                description: "Replace ok_or string boundaries with anyhow Context so missing values preserve useful diagnostics.".to_string(),
                file: rel.to_path_buf(),
                line,
                strategy: HardeningStrategy::OptionContextPropagation,
                patchable: true,
            });
        }
    }
}

fn replace_ok_or_string_with_context(line: &str) -> Option<String> {
    let start = line.find(".ok_or(\"")?;
    let message_start = start + ".ok_or(\"".len();
    let after_message = &line[message_start..];
    let message_end = after_message.find("\")?")?;
    let message = &after_message[..message_end];
    if message.is_empty() || message.contains('\\') {
        return None;
    }
    let suffix = &after_message[message_end + "\")?".len()..];
    let mut output = String::new();
    output.push_str(&line[..start]);
    output.push_str(&format!(".context(\"{}\")?", escape_string(message)));
    output.push_str(suffix);
    Some(output)
}

fn apply_repeated_string_literal_const_recipe(
    rel: &Path,
    lines: &mut Vec<String>,
    finding_ids: &mut Vec<String>,
    findings: &mut Vec<HardeningFinding>,
) {
    let content = lines.join("\n");
    let Some((literal, count, first_line)) = repeated_safe_string_literal(&content) else {
        return;
    };
    let const_name = format!("MDX_LITERAL_{}", short_literal_hash(&literal));
    if content.contains(&const_name) {
        return;
    }

    let quoted = format!("\"{}\"", escape_string(&literal));
    let mut replacement_count = 0usize;
    for line in lines.iter_mut() {
        let should_rewrite = !line.trim_start().starts_with("//") && line.contains(&quoted);
        if should_rewrite {
            *line = line.replace(&quoted, &const_name);
            replacement_count += 1;
        }
    }
    if replacement_count < 3 {
        return;
    }

    let insert_at = const_insert_index(lines);
    lines.insert(insert_at, format!("const {const_name}: &str = {quoted};"));

    let id = format!(
        "repeated-string-literal-const:{}:{first_line}",
        rel.display()
    );
    finding_ids.push(id.clone());
    findings.push(HardeningFinding {
        id,
        title: "Extract repeated string literal".to_string(),
        description: format!(
            "Extract repeated private string literal used {count} times into a file-local const under Tier 2 evidence gates."
        ),
        file: rel.to_path_buf(),
        line: first_line,
        strategy: HardeningStrategy::RepeatedStringLiteralConst,
        patchable: true,
    });
}

fn repeated_safe_string_literal(content: &str) -> Option<(String, usize, usize)> {
    let mut counts = std::collections::BTreeMap::<String, (usize, usize)>::new();
    for (line_index, line) in content.lines().enumerate() {
        if line.trim_start().starts_with("//") || line.trim_start().starts_with("const ") {
            continue;
        }
        for literal in string_literals_in_line(line) {
            if !is_safe_extractable_literal(&literal) {
                continue;
            }
            let entry = counts.entry(literal).or_insert((0, line_index + 1));
            entry.0 += 1;
        }
    }

    counts
        .into_iter()
        .filter(|(_, (count, _))| *count >= 3)
        .max_by(|left, right| {
            left.1
                 .0
                .cmp(&right.1 .0)
                .then_with(|| left.0.len().cmp(&right.0.len()))
        })
        .map(|(literal, (count, line))| (literal, count, line))
}

fn string_literals_in_line(line: &str) -> Vec<String> {
    let mut literals = Vec::new();
    let mut chars = line.char_indices().peekable();
    while let Some((_, ch)) = chars.next() {
        if ch != '"' {
            continue;
        }
        let mut literal = String::new();
        let mut escaped = false;
        for (_, next) in chars.by_ref() {
            if escaped {
                literal.push(next);
                escaped = false;
                continue;
            }
            if next == '\\' {
                escaped = true;
                continue;
            }
            if next == '"' {
                literals.push(literal);
                break;
            }
            literal.push(next);
        }
    }
    literals
}

fn is_safe_extractable_literal(value: &str) -> bool {
    value.len() >= 8
        && value.len() <= 80
        && !value.contains('{')
        && !value.contains('}')
        && !value.contains('\n')
        && value.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(ch, ' ' | '-' | '_' | '.' | '/' | ':' | ',' | '(' | ')')
        })
}

fn const_insert_index(lines: &[String]) -> usize {
    let mut index = 0usize;
    while index < lines.len() {
        let trimmed = lines[index].trim_start();
        if trimmed.starts_with("#![") || trimmed.starts_with("//!") || trimmed.is_empty() {
            index += 1;
            continue;
        }
        if trimmed.starts_with("use ") {
            index += 1;
            continue;
        }
        break;
    }
    index
}

fn short_literal_hash(value: &str) -> String {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:08X}", hasher.finish() as u32)
}

fn replace_map_clone_calls(line: &str) -> String {
    let mut output = String::new();
    let mut rest = line;
    while let Some(start) = rest.find(".map(|") {
        let (before, after_start) = rest.split_at(start);
        output.push_str(before);
        let Some((variable, after_variable)) = after_start[".map(|".len()..].split_once('|') else {
            output.push_str(after_start);
            return output;
        };
        let variable = variable.trim();
        if variable.is_empty()
            || !variable
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            output.push_str(after_start);
            return output;
        }

        let expected = format!(" {}.clone())", variable);
        let trimmed_expected = format!("{}.clone())", variable);
        if let Some(next) = after_variable.strip_prefix(&expected) {
            rest = push_clone_replacement(&mut output, next);
        } else if let Some(next) = after_variable.strip_prefix(&trimmed_expected) {
            rest = push_clone_replacement(&mut output, next);
        } else {
            output.push_str(".map(|");
            rest = &after_start[".map(|".len()..];
        }
    }
    output.push_str(rest);
    output
}

fn push_clone_replacement<'a>(output: &mut String, next: &'a str) -> &'a str {
    if next.starts_with(".collect()") && output.ends_with(".iter()") {
        output.truncate(output.len() - ".iter()".len());
        output.push_str(".to_vec()");
        &next[".collect()".len()..]
    } else {
        output.push_str(".cloned()");
        next
    }
}

fn tighten_borrow_parameters(line: &str) -> String {
    replace_borrowed_vec(&line.replace("&String", "&str"))
}

fn replace_borrowed_vec(line: &str) -> String {
    let mut output = String::new();
    let mut index = 0usize;
    while let Some(relative_start) = line[index..].find("&Vec<") {
        let start = index + relative_start;
        output.push_str(&line[index..start]);
        let generic_start = start + "&Vec<".len();
        let Some(generic_end) = matching_angle_end(line, generic_start) else {
            output.push_str(&line[start..]);
            return output;
        };
        output.push_str("&[");
        output.push_str(&line[generic_start..generic_end]);
        output.push(']');
        index = generic_end + 1;
    }
    output.push_str(&line[index..]);
    output
}

fn matching_angle_end(value: &str, start: usize) -> Option<usize> {
    let mut depth = 1isize;
    for (offset, ch) in value[start..].char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return Some(start + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn has_nearby_must_use(lines: &[String], signature_line: usize) -> bool {
    let signature_index = signature_line.saturating_sub(1);
    let start = signature_index.saturating_sub(4);
    lines[start..signature_index.min(lines.len())]
        .iter()
        .any(|line| line.contains("must_use"))
}

fn replace_expect_calls(line: &str) -> String {
    let mut output = String::new();
    let mut rest = line;
    while let Some(start) = rest.find(".expect(\"") {
        let (before, after_start) = rest.split_at(start);
        output.push_str(before);
        let msg_start = ".expect(\"".len();
        let after_msg_start = &after_start[msg_start..];
        if let Some(end) = after_msg_start.find("\")") {
            let message = &after_msg_start[..end];
            output.push_str(&format!(".context(\"{}\")?", escape_string(message)));
            rest = &after_msg_start[end + 2..];
        } else {
            output.push_str(after_start);
            rest = "";
        }
    }
    output.push_str(rest);
    output
}

fn escape_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn line_without_comments_or_strings(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if !in_string && ch == '/' && chars.peek() == Some(&'/') {
            break;
        }

        if ch == '"' && !escaped {
            in_string = !in_string;
            output.push(' ');
            continue;
        }

        if in_string {
            escaped = ch == '\\' && !escaped;
            output.push(' ');
            continue;
        }

        escaped = false;
        output.push(ch);
    }

    output
}

fn ensure_anyhow_context_import(content: &str) -> String {
    if has_anyhow_context_import(content) {
        return content.to_string();
    }

    let mut lines: Vec<&str> = content.lines().collect();
    let insert_at = lines
        .iter()
        .position(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("#![")
                && !trimmed.starts_with("//!")
                && !trimmed.starts_with("/*!")
                && !trimmed.is_empty()
        })
        .unwrap_or(0);
    lines.insert(insert_at, "use anyhow::Context;");
    let mut result = lines.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

fn has_anyhow_context_import(content: &str) -> bool {
    content.lines().any(|line| {
        let trimmed = line.trim();
        if !(trimmed.starts_with("use anyhow::") || trimmed.starts_with("pub use anyhow::")) {
            return false;
        }
        trimmed == "use anyhow::Context;"
            || trimmed == "pub use anyhow::Context;"
            || trimmed.starts_with("use anyhow::{") && import_group_contains_context(trimmed)
            || trimmed.starts_with("pub use anyhow::{") && import_group_contains_context(trimmed)
    })
}

fn import_group_contains_context(line: &str) -> bool {
    line.split_once('{')
        .and_then(|(_, rest)| rest.split_once('}').map(|(inside, _)| inside))
        .is_some_and(|inside| {
            inside
                .split(',')
                .any(|item| item.trim().split(" as ").next() == Some("Context"))
        })
}

#[derive(Debug)]
struct FunctionRange {
    name: String,
    start_line: usize,
    end_line: usize,
    signature_start_line: usize,
    signature_end_line: usize,
    is_public: bool,
    returns_anyhow_result: bool,
    returns_value: bool,
    returns_common_must_use: bool,
}

fn find_function_ranges(content: &str) -> Vec<FunctionRange> {
    let lines: Vec<&str> = content.lines().collect();
    let has_anyhow_result_alias =
        content.contains("use anyhow::Result") || content.contains("use anyhow::{Result");
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if !line.contains("fn ") {
            index += 1;
            continue;
        }

        let mut signature = line.to_string();
        let start_line = index + 1;
        let mut open_line = index;
        while !signature.contains('{') && open_line + 1 < lines.len() {
            open_line += 1;
            signature.push(' ');
            signature.push_str(lines[open_line]);
        }

        if !signature.contains('{') {
            index += 1;
            continue;
        }

        let Some(name) = function_name(&signature) else {
            index += 1;
            continue;
        };

        let mut depth = 0isize;
        let mut end_line = open_line + 1;
        for (body_index, body_line) in lines.iter().enumerate().skip(open_line) {
            depth += body_line.matches('{').count() as isize;
            depth -= body_line.matches('}').count() as isize;
            end_line = body_index + 1;
            if depth == 0 {
                break;
            }
        }

        let return_text = signature
            .split_once("->")
            .map(|(_, rest)| rest.split('{').next().unwrap_or_default().trim())
            .unwrap_or_default();
        let returns_anyhow_result = return_text.starts_with("anyhow::Result")
            || (has_anyhow_result_alias && return_text.starts_with("Result<"));
        let returns_value = !return_text.is_empty() && return_text != "()";
        let returns_common_must_use = return_text.starts_with("Result<")
            || return_text.starts_with("anyhow::Result")
            || return_text.starts_with("Option<")
            || signature.contains("async fn ");
        ranges.push(FunctionRange {
            name,
            start_line,
            end_line,
            signature_start_line: start_line,
            signature_end_line: open_line + 1,
            is_public: signature.trim_start().starts_with("pub "),
            returns_anyhow_result,
            returns_value,
            returns_common_must_use,
        });
        index = end_line;
    }
    ranges
}

fn function_name(signature: &str) -> Option<String> {
    let rest = signature.split_once("fn ")?.1;
    let name = rest
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .next()?;
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn collect_rust_files(root: &Path, target: Option<&Path>) -> anyhow::Result<Vec<PathBuf>> {
    let requested_scan_root = target
        .map(|path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            }
        })
        .unwrap_or_else(|| root.to_path_buf());
    if target.is_some() && !requested_scan_root.exists() {
        anyhow::bail!(
            "hardening target does not exist: {}",
            requested_scan_root.display()
        );
    }
    let scan_root = requested_scan_root
        .canonicalize()
        .unwrap_or(requested_scan_root);
    if !scan_root.starts_with(root) {
        anyhow::bail!("hardening target is outside root: {}", scan_root.display());
    }

    if scan_root.is_file() {
        return Ok(if scan_root.extension().is_some_and(|ext| ext == "rs") {
            vec![scan_root]
        } else {
            Vec::new()
        });
    }

    let mut files = Vec::new();
    for result in ignore::WalkBuilder::new(scan_root)
        .hidden(false)
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            !matches!(
                name.as_ref(),
                "target" | ".git" | ".worktrees" | ".mdx-rust"
            )
        })
        .build()
    {
        let entry = result?;
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path.to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

fn relative_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn hardening_rewrites_unwrap_in_anyhow_result_function() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"pub fn load() -> anyhow::Result<String> {
    let value = std::fs::read_to_string("config.toml").unwrap();
    Ok(value)
}
"#,
        )
        .unwrap();

        let analysis = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
                max_recipe_tier: 1,
                evidence_depth: HardeningEvidenceDepth::Basic,
            },
        )
        .unwrap();

        assert_eq!(analysis.changes.len(), 1);
        let change = &analysis.changes[0];
        assert!(change.new_content.contains("use anyhow::Context;"));
        assert!(change
            .new_content
            .contains(".context(\"load failed instead of panicking\")?"));
        assert!(syn::parse_file(&change.new_content).is_ok());
    }

    #[test]
    fn hardening_adds_context_to_question_mark_boundaries() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"pub fn load(path: &str) -> anyhow::Result<String> {
    let value = std::fs::read_to_string(path)?;
    Ok(value)
}
"#,
        )
        .unwrap();

        let analysis = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
                max_recipe_tier: 1,
                evidence_depth: HardeningEvidenceDepth::Basic,
            },
        )
        .unwrap();

        assert_eq!(analysis.changes.len(), 1);
        let change = &analysis.changes[0];
        assert!(change.new_content.contains("use anyhow::Context;"));
        assert!(change
            .new_content
            .contains(".context(\"load failed at filesystem boundary\")?"));
        assert!(change
            .finding_ids
            .iter()
            .any(|id| id.contains("error-context-propagation")));
        assert!(syn::parse_file(&change.new_content).is_ok());
    }

    #[test]
    fn hardening_context_import_ignores_context_named_types_and_preserves_inner_docs() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"//! Crate docs must stay before regular items.

pub struct CandidateEvidenceContext;

pub fn load(path: &str) -> anyhow::Result<String> {
    let value = std::fs::read_to_string(path)?;
    Ok(value)
}
"#,
        )
        .unwrap();

        let analysis = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
                max_recipe_tier: 1,
                evidence_depth: HardeningEvidenceDepth::Basic,
            },
        )
        .unwrap();

        assert_eq!(analysis.changes.len(), 1);
        let change = &analysis.changes[0];
        assert!(change
            .new_content
            .starts_with("//! Crate docs must stay before regular items.\n\nuse anyhow::Context;"));
        assert!(syn::parse_file(&change.new_content).is_ok());
    }

    #[test]
    fn hardening_does_not_rewrite_plain_result_without_anyhow_alias() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"pub fn load() -> Result<String, std::io::Error> {
    let value = std::fs::read_to_string("config.toml").unwrap();
    Ok(value)
}
"#,
        )
        .unwrap();

        let analysis = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
                max_recipe_tier: 1,
                evidence_depth: HardeningEvidenceDepth::Basic,
            },
        )
        .unwrap();

        assert!(analysis.changes.is_empty());
    }

    #[test]
    fn hardening_tightens_private_borrowed_owned_parameters() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"fn score(name: &String, values: &Vec<u8>) -> usize {
    name.len() + values.len()
}
"#,
        )
        .unwrap();

        let analysis = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
                max_recipe_tier: 1,
                evidence_depth: HardeningEvidenceDepth::Basic,
            },
        )
        .unwrap();

        assert_eq!(analysis.changes.len(), 1);
        let change = &analysis.changes[0];
        assert!(change
            .new_content
            .contains("fn score(name: &str, values: &[u8])"));
        assert!(change
            .finding_ids
            .iter()
            .any(|id| id.contains("borrow-parameter-tightening")));
        assert!(syn::parse_file(&change.new_content).is_ok());
    }

    #[test]
    fn hardening_marks_public_value_returns_must_use() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"pub fn total(values: &[u8]) -> usize {
    values.iter().map(|value| *value as usize).sum()
}
"#,
        )
        .unwrap();

        let analysis = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
                max_recipe_tier: 1,
                evidence_depth: HardeningEvidenceDepth::Basic,
            },
        )
        .unwrap();

        assert_eq!(analysis.changes.len(), 1);
        let change = &analysis.changes[0];
        assert!(change.new_content.contains("#[must_use]\npub fn total"));
        assert!(change
            .finding_ids
            .iter()
            .any(|id| id.contains("must-use-public-return")));
        assert!(syn::parse_file(&change.new_content).is_ok());
    }

    #[test]
    fn hardening_replaces_map_clone_collect_with_to_vec() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"pub fn copy_values(values: &[String]) -> Vec<String> {
    values.iter().map(|value| value.clone()).collect()
}
"#,
        )
        .unwrap();

        let analysis = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
                max_recipe_tier: 1,
                evidence_depth: HardeningEvidenceDepth::Basic,
            },
        )
        .unwrap();

        assert_eq!(analysis.changes.len(), 1);
        let change = &analysis.changes[0];
        assert!(change.new_content.contains("values.to_vec()"));
        assert!(change
            .finding_ids
            .iter()
            .any(|id| id.contains("iterator-cloned")));
        assert!(syn::parse_file(&change.new_content).is_ok());
    }

    #[test]
    fn tier2_extracts_repeated_private_string_literal_when_enabled() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"fn labels() -> Vec<&'static str> {
    vec![
        "shared boundary label",
        "shared boundary label",
        "shared boundary label",
    ]
}
"#,
        )
        .unwrap();

        let tier1 = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
                max_recipe_tier: 1,
                evidence_depth: HardeningEvidenceDepth::Basic,
            },
        )
        .unwrap();
        assert!(tier1.changes.is_empty());

        let tier2 = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
                max_recipe_tier: 2,
                evidence_depth: HardeningEvidenceDepth::Covered,
            },
        )
        .unwrap();

        assert_eq!(tier2.changes.len(), 1);
        let change = &tier2.changes[0];
        assert!(change.new_content.contains("const MDX_LITERAL_"));
        assert!(change
            .finding_ids
            .iter()
            .any(|id| id.contains("repeated-string-literal-const")));
        assert!(syn::parse_file(&change.new_content).is_ok());
    }

    #[test]
    fn tier2_rewrites_len_zero_checks_when_enabled() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"pub fn empty(items: &[String]) -> bool {
    items.len() == 0
}
"#,
        )
        .unwrap();

        let tier2 = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
                max_recipe_tier: 2,
                evidence_depth: HardeningEvidenceDepth::Covered,
            },
        )
        .unwrap();

        assert_eq!(tier2.changes.len(), 1);
        let change = &tier2.changes[0];
        assert!(change.new_content.contains("items.is_empty()"));
        assert!(change
            .finding_ids
            .iter()
            .any(|id| id.contains("len-check-is-empty")));
        assert!(syn::parse_file(&change.new_content).is_ok());
    }

    #[test]
    fn tier2_rewrites_option_ok_or_to_context_when_enabled() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"pub fn load(value: Option<String>) -> anyhow::Result<String> {
    let value = value.ok_or("missing value")?;
    Ok(value)
}
"#,
        )
        .unwrap();

        let tier2 = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
                max_recipe_tier: 2,
                evidence_depth: HardeningEvidenceDepth::Covered,
            },
        )
        .unwrap();

        assert_eq!(tier2.changes.len(), 1);
        let change = &tier2.changes[0];
        assert!(change.new_content.contains("use anyhow::Context;"));
        assert!(change.new_content.contains(".context(\"missing value\")?"));
        assert!(change
            .finding_ids
            .iter()
            .any(|id| id.contains("option-context-propagation")));
        assert!(syn::parse_file(&change.new_content).is_ok());
    }

    #[test]
    fn hardened_evidence_adds_deeper_review_findings() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        let mut body = String::from("pub fn clone_pressure(values: &[String]) -> Vec<String> {\n");
        body.push_str("    let a = values[0].clone();\n");
        body.push_str("    let b = values[1].clone();\n");
        body.push_str("    let c = values[2].clone();\n");
        for index in 0..50 {
            body.push_str(&format!("    let _v{index} = {index};\n"));
        }
        body.push_str("    vec![a, b, c]\n}\n");
        std::fs::write(src.join("lib.rs"), body).unwrap();

        let basic = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
                max_recipe_tier: 1,
                evidence_depth: HardeningEvidenceDepth::Basic,
            },
        )
        .unwrap();
        assert!(!basic.findings.iter().any(|finding| matches!(
            finding.strategy,
            HardeningStrategy::ClonePressureReview | HardeningStrategy::LongFunctionReview
        )));

        let hardened = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
                max_recipe_tier: 1,
                evidence_depth: HardeningEvidenceDepth::Hardened,
            },
        )
        .unwrap();
        assert!(hardened.findings.iter().any(|finding| {
            finding.strategy == HardeningStrategy::ClonePressureReview && !finding.patchable
        }));
        assert!(hardened.findings.iter().any(|finding| {
            finding.strategy == HardeningStrategy::LongFunctionReview && !finding.patchable
        }));
    }

    #[test]
    fn hardening_does_not_flag_patterns_inside_strings_or_comments() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"fn describe() -> &'static str {
    // Command::new("ignored")
    "unsafe std::process::Command env::var("
}
"#,
        )
        .unwrap();

        let analysis = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: None,
                max_files: 10,
                max_recipe_tier: 1,
                evidence_depth: HardeningEvidenceDepth::Basic,
            },
        )
        .unwrap();

        assert!(analysis.findings.is_empty(), "{:?}", analysis.findings);
    }

    #[test]
    fn hardening_rejects_missing_target() {
        let dir = tempdir().unwrap();
        let err = analyze_hardening(
            dir.path(),
            HardeningAnalyzeConfig {
                target: Some(Path::new("src/missing.rs")),
                max_files: 10,
                max_recipe_tier: 1,
                evidence_depth: HardeningEvidenceDepth::Basic,
            },
        )
        .unwrap_err();

        assert!(err.to_string().contains("hardening target does not exist"));
    }
}
