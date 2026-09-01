use anyhow::{Context, Result};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use flate2::read::MultiGzDecoder;
use md5::Md5;
use noodles::{bam, bcf, cram, sam, vcf};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::cli::{ReferenceConfig, ReferencePolicy};
use crate::parser;
use crate::report;

const REFERENCE_SCHEMA_VERSION: &str = "1.0.0";
const SEQCOL_SCHEMA: &str = "https://w3id.org/ga4gh/seqcol/schema/extended/v1.0.0";

#[derive(Debug, Serialize)]
struct ReferenceReport {
    schema_version: &'static str,
    report_type: &'static str,
    tool: ReferenceTool,
    canonical_reference: CanonicalReference,
    reference_manifest: ReferenceManifest,
    comparisons: Vec<ReferenceComparison>,
    findings: Vec<ReferenceFinding>,
    verdict: ReferenceVerdict,
    gate: ReferenceGate,
    readiness: ReferenceReadiness,
    machine_summary: ReferenceMachineSummary,
    provenance: ReferenceProvenance,
}

#[derive(Debug, Serialize)]
struct ReferenceTool {
    name: &'static str,
    version: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct CanonicalReference {
    physical_sha256: String,
    seqcol_schema: &'static str,
    seqcol_digest: Option<String>,
    name_length_pairs_digest: Option<String>,
    sorted_name_length_pairs_digest: Option<String>,
    sequences: Vec<CanonicalSequence>,
    #[serde(skip)]
    critical_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CanonicalSequence {
    id: String,
    order: usize,
    length: usize,
    sam_md5: Option<String>,
    refget_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReferenceComparison {
    kind: &'static str,
    declaration_digest: String,
    relationship: &'static str,
    evidence: Vec<&'static str>,
    #[serde(skip)]
    source_path: PathBuf,
}

impl ReferenceComparison {
    fn new(
        kind: &'static str,
        relationship: &'static str,
        alias_used: bool,
        declaration_digest: String,
        source_path: &Path,
    ) -> Self {
        let mut evidence = vec![match (kind, relationship) {
            ("dict" | "alignment", "exact" | "content_equivalent") => "content_verified",
            ("fai", "exact") => "metadata_verified",
            ("alignment", "coordinate_compatible") => "header_asserted",
            ("variants", "exact") => "content_verified",
            ("variants", "coordinate_compatible" | "subset_compatible") => "header_asserted",
            ("annotation", "coordinate_compatible" | "subset_compatible") => "metadata_verified",
            _ => "insufficient",
        }];
        if alias_used {
            evidence.push("explicit_alias_mapping");
        }
        Self {
            kind,
            declaration_digest,
            relationship,
            evidence,
            source_path: source_path.to_path_buf(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct FaiEntry {
    id: String,
    length: usize,
    offset: usize,
    line_bases: usize,
    line_width: usize,
}

#[derive(Debug, Clone, Serialize)]
struct DictionaryEntry {
    id: String,
    length: usize,
    md5: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct VariantEntry {
    id: String,
    length: Option<usize>,
    md5: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AnnotationEntry {
    id: String,
    start: usize,
    end: usize,
    is_sequence_region: bool,
    is_circular_marker: bool,
}

#[derive(Debug, Serialize)]
struct ReferenceVerdict {
    status: &'static str,
    reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ReferenceGate {
    mode: &'static str,
    reference_policy: ReferencePolicyReport,
    status: &'static str,
    can_continue: bool,
    blocking_findings: Vec<&'static str>,
    advisory_findings: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct ReferencePolicyReport {
    id: &'static str,
    version: &'static str,
    required_kinds: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ReferenceMachineSummary {
    safe_for_downstream: bool,
}

#[derive(Debug, Serialize)]
struct ReferenceFinding {
    id: &'static str,
    artifact_kind: &'static str,
    relationship: &'static str,
    evidence: Vec<&'static str>,
    policy_effect: &'static str,
    affected_count: u64,
    original_name: Option<String>,
    resolved_name: Option<String>,
    expected_value: Option<String>,
    observed_value: Option<String>,
    examples: Vec<ReferenceFindingExample>,
    message: &'static str,
    suggested_next_step: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct ReferenceFindingExample {
    original_name: Option<String>,
    resolved_name: Option<String>,
    expected_value: Option<String>,
    observed_value: Option<String>,
}

#[derive(Debug, Clone)]
struct ReferenceFindingDetail {
    id: &'static str,
    example: ReferenceFindingExample,
}

#[derive(Debug, Serialize)]
struct ReferenceReadiness {
    overall: ReferenceReadinessOverall,
    categories: Vec<ReferenceReadinessCategory>,
}

#[derive(Debug, Serialize)]
struct ReferenceReadinessOverall {
    status: &'static str,
    blockers: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct ReferenceReadinessCategory {
    id: &'static str,
    label: &'static str,
    status: &'static str,
    findings: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct ReferenceProvenance {
    reference_path: String,
    companions: Vec<ReferenceCompanionProvenance>,
}

#[derive(Debug, Serialize)]
struct ReferenceCompanionProvenance {
    kind: &'static str,
    path: String,
    declaration_digest: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReferenceManifest {
    manifest_version: &'static str,
    semantic_digest: String,
    canonical_reference: CanonicalReference,
    artifacts: Vec<ReferenceArtifact>,
    comparisons: Vec<ReferenceComparison>,
    coverage: BTreeMap<String, serde_json::Value>,
    delegated_checks: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ReferenceLockManifest {
    manifest_version: &'static str,
    semantic_digest: String,
    canonical_reference: SemanticCanonicalReference,
    artifacts: Vec<ReferenceArtifact>,
    comparisons: Vec<ReferenceComparison>,
    coverage: BTreeMap<String, serde_json::Value>,
    delegated_checks: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SemanticCanonicalReference {
    seqcol_schema: &'static str,
    seqcol_digest: Option<String>,
    name_length_pairs_digest: Option<String>,
    sorted_name_length_pairs_digest: Option<String>,
    sequences: Vec<CanonicalSequence>,
}

impl From<&ReferenceManifest> for ReferenceLockManifest {
    fn from(manifest: &ReferenceManifest) -> Self {
        Self {
            manifest_version: manifest.manifest_version,
            semantic_digest: manifest.semantic_digest.clone(),
            canonical_reference: SemanticCanonicalReference {
                seqcol_schema: manifest.canonical_reference.seqcol_schema,
                seqcol_digest: manifest.canonical_reference.seqcol_digest.clone(),
                name_length_pairs_digest: manifest
                    .canonical_reference
                    .name_length_pairs_digest
                    .clone(),
                sorted_name_length_pairs_digest: manifest
                    .canonical_reference
                    .sorted_name_length_pairs_digest
                    .clone(),
                sequences: manifest.canonical_reference.sequences.clone(),
            },
            artifacts: manifest.artifacts.clone(),
            comparisons: manifest.comparisons.clone(),
            coverage: manifest.coverage.clone(),
            delegated_checks: manifest.delegated_checks.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ReferenceArtifact {
    kind: &'static str,
    declaration_digest: String,
    declaration: serde_json::Value,
}

#[derive(Debug, Clone)]
struct ArtifactSource {
    source_path: PathBuf,
    artifact: ReferenceArtifact,
}

pub fn run(config: ReferenceConfig) -> Result<i32> {
    let report = build_report(&config)?;
    let lock = config
        .outputs
        .lock
        .as_ref()
        .map(|_| ReferenceLockManifest::from(&report.reference_manifest));
    let outputs = reference_output_paths(&config);
    validate_reference_output_paths(&outputs, config.outputs.allow_overwrite)?;

    let write_html_report = |path: &Path| write_html(&report, path, true);
    let write_tsv_report = |path: &Path| write_tsv(&report, path, true);
    let write_multiqc_report = |path: &Path| write_multiqc(&report, path, true);
    let write_lock_report = |path: &Path| write_document(lock.as_ref().unwrap(), path, true);
    let write_json_report = |path: &Path| write_document(&report, path, true);
    let mut serializers: Vec<report::StagedSerializer<'_>> = Vec::new();
    if let Some(path) = &config.outputs.html {
        serializers.push((path, &write_html_report));
    }
    if let Some(path) = &config.outputs.tsv {
        serializers.push((path, &write_tsv_report));
    }
    if let Some(path) = &config.outputs.multiqc {
        serializers.push((path, &write_multiqc_report));
    }
    if let Some(path) = &config.outputs.lock {
        serializers.push((path, &write_lock_report));
    }
    if let Some(path) = &config.outputs.json {
        serializers.push((path, &write_json_report));
    }
    report::write_staged_set(&serializers, config.outputs.allow_overwrite)?;

    Ok(0)
}

fn reference_output_paths(config: &ReferenceConfig) -> Vec<&Path> {
    [
        config.outputs.html.as_deref(),
        config.outputs.tsv.as_deref(),
        config.outputs.multiqc.as_deref(),
        config.outputs.lock.as_deref(),
        config.outputs.json.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn validate_reference_output_paths(paths: &[&Path], allow_overwrite: bool) -> Result<()> {
    let mut seen = BTreeSet::new();
    for path in paths {
        let normalized = report::normalize_output_path(path)?;
        if !seen.insert(normalized) {
            anyhow::bail!("duplicate reference output path: {}", path.display());
        }
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        if !parent.is_dir() {
            anyhow::bail!(
                "parent directory for reference output path {} does not exist: {}",
                path.display(),
                parent.display()
            );
        }
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_dir() => {
                anyhow::bail!("reference output path {} is a directory", path.display());
            }
            Ok(_) if !allow_overwrite => {
                anyhow::bail!(
                    "reference output path {} already exists; use --force to replace it",
                    path.display()
                );
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("failed to inspect {}", path.display()))
            }
        }
    }
    Ok(())
}

fn write_html(report: &ReferenceReport, path: &Path, allow_overwrite: bool) -> Result<()> {
    let sequence_rows = report
        .canonical_reference
        .sequences
        .iter()
        .map(|sequence| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(&sequence.id),
                sequence.length,
                sequence.sam_md5.as_deref().unwrap_or("unavailable"),
                sequence.refget_id.as_deref().unwrap_or("unavailable"),
            )
        })
        .collect::<String>();
    let comparisons = report
        .comparisons
        .iter()
        .map(|comparison| format!("<li>{}: {}</li>", comparison.kind, comparison.relationship))
        .collect::<String>();
    let findings = report
        .findings
        .iter()
        .map(|finding| {
            format!(
                "<li><strong>{}</strong>: {} Suggested next step: {}</li>",
                escape_html(finding.id),
                escape_html(finding.message),
                escape_html(finding.suggested_next_step),
            )
        })
        .collect::<String>();
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>FastaGuard Reference</title></head><body><h1>FastaGuard Reference</h1><p>Verdict: {}</p><p>Policy: {}</p><h2>Compatibility</h2><ul>{}</ul><h2>Findings and next steps</h2><ul>{}</ul><h2>Canonical sequences</h2><table><thead><tr><th>ID</th><th>Length</th><th>SAM MD5</th><th>refget</th></tr></thead><tbody>{}</tbody></table></body></html>\n",
        report.verdict.status,
        report.gate.reference_policy.id,
        comparisons,
        findings,
        sequence_rows,
    );
    write_text(&html, path, allow_overwrite)
}

fn write_tsv(report: &ReferenceReport, path: &Path, allow_overwrite: bool) -> Result<()> {
    let mut output = String::from(
        "record_type\tkind\tdeclaration_digest\tinput_path\tid\tlength\tsam_md5\trefget_id\trelationship\tmessage\tsuggested_next_step\n",
    );
    for sequence in &report.canonical_reference.sequences {
        output.push_str(&format!(
            "sequence\t\t\t\t{}\t{}\t{}\t{}\t\t\t\n",
            sanitize_tsv(&sequence.id),
            sequence.length,
            sequence.sam_md5.as_deref().unwrap_or_default(),
            sequence.refget_id.as_deref().unwrap_or_default(),
        ));
    }
    for comparison in &report.comparisons {
        output.push_str(&format!(
            "comparison\t{}\t{}\t{}\t\t\t\t\t{}\t\t\n",
            comparison.kind,
            comparison.declaration_digest,
            sanitize_tsv(&comparison.source_path.display().to_string()),
            comparison.relationship,
        ));
    }
    for finding in &report.findings {
        output.push_str(&format!(
            "finding\t{}\t\t\t{}\t\t\t\t{}\t{}\t{}\n",
            finding.artifact_kind,
            finding.id,
            finding.relationship,
            sanitize_tsv(finding.message),
            sanitize_tsv(finding.suggested_next_step),
        ));
    }
    write_text(&output, path, allow_overwrite)
}

fn write_multiqc(report: &ReferenceReport, path: &Path, allow_overwrite: bool) -> Result<()> {
    let relationship_counts =
        report
            .comparisons
            .iter()
            .fold(BTreeMap::<&str, usize>::new(), |mut counts, comparison| {
                *counts.entry(comparison.relationship).or_default() += 1;
                counts
            });
    let mismatch_count = relationship_counts
        .get("incompatible")
        .copied()
        .unwrap_or(0)
        + relationship_counts
            .get("indeterminate")
            .copied()
            .unwrap_or(0);
    let row = serde_json::json!({
        "verdict": report.verdict.status,
        "gate_can_continue": report.gate.can_continue,
        "reference_policy": report.gate.reference_policy.id,
        "supplied_artifact_count": report.reference_manifest.artifacts.len(),
        "required_artifact_kind_count": report.gate.reference_policy.required_kinds.len(),
        "mismatch_count": mismatch_count,
        "exact_count": relationship_counts.get("exact").copied().unwrap_or(0),
        "content_equivalent_count": relationship_counts.get("content_equivalent").copied().unwrap_or(0),
        "coordinate_compatible_count": relationship_counts.get("coordinate_compatible").copied().unwrap_or(0),
        "subset_compatible_count": relationship_counts.get("subset_compatible").copied().unwrap_or(0),
        "incompatible_count": relationship_counts.get("incompatible").copied().unwrap_or(0),
        "indeterminate_count": relationship_counts.get("indeterminate").copied().unwrap_or(0),
    });
    let data = BTreeMap::from([(report.reference_manifest.semantic_digest.clone(), row)]);
    let document = serde_json::json!({
        "id": "fastaguard_reference",
        "section_name": "FastaGuard Reference",
        "description": "Canonical reference sequence identity and compatibility summary",
        "plot_type": "table",
        "pconfig": {
            "id": "fastaguard_reference",
            "title": "FastaGuard Reference",
            "headers": {
                "verdict": { "title": "Verdict" },
                "gate_can_continue": { "title": "Gate can continue" },
                "reference_policy": { "title": "Reference policy" },
                "supplied_artifact_count": { "title": "Supplied artefacts" },
                "required_artifact_kind_count": { "title": "Required artefact kinds" },
                "mismatch_count": { "title": "Mismatches" },
                "exact_count": { "title": "Exact" },
                "content_equivalent_count": { "title": "Content equivalent" },
                "coordinate_compatible_count": { "title": "Coordinate compatible" },
                "subset_compatible_count": { "title": "Subset compatible" },
                "incompatible_count": { "title": "Incompatible" },
                "indeterminate_count": { "title": "Indeterminate" }
            }
        },
        "data": data,
    });
    write_document(&document, path, allow_overwrite)
}

fn write_text(contents: &str, path: &Path, allow_overwrite: bool) -> Result<()> {
    if path.exists() && !allow_overwrite {
        anyhow::bail!("output path already exists: {}", path.display());
    }
    let mut file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    file.write_all(contents.as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))?;
    file.flush()
        .with_context(|| format!("failed to flush {}", path.display()))
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn sanitize_tsv(value: &str) -> String {
    value.replace(['\t', '\n', '\r'], " ")
}

fn build_manifest(
    canonical_reference: &CanonicalReference,
    artifacts: &[ReferenceArtifact],
    comparisons: &[ReferenceComparison],
    config: &ReferenceConfig,
    aliases: Option<&BTreeMap<String, String>>,
) -> Result<ReferenceManifest> {
    let coverage = BTreeMap::from([
        (
            "reference_policy".to_string(),
            serde_json::json!(policy_id(config.policy)),
        ),
        (
            "required_kinds".to_string(),
            serde_json::json!(config.required_artifacts),
        ),
        (
            "alias_map".to_string(),
            serde_json::to_value(aliases.cloned().unwrap_or_default())
                .context("failed to serialize reference aliases")?,
        ),
    ]);
    let mut semantic_comparisons = comparisons.to_vec();
    semantic_comparisons.sort_by(|left, right| {
        left.kind
            .cmp(right.kind)
            .then_with(|| left.declaration_digest.cmp(&right.declaration_digest))
            .then_with(|| left.relationship.cmp(right.relationship))
    });
    let semantic_payload = serde_json::json!({
        "manifest_version": "1.0.0",
        "canonical_reference": {
            "sequences": canonical_reference.sequences,
            "seqcol_schema": canonical_reference.seqcol_schema,
            "seqcol_digest": canonical_reference.seqcol_digest,
            "name_length_pairs_digest": canonical_reference.name_length_pairs_digest,
            "sorted_name_length_pairs_digest": canonical_reference.sorted_name_length_pairs_digest,
        },
        "artifacts": artifacts,
        "comparisons": semantic_comparisons,
        "coverage": coverage,
        "delegated_checks": [],
    });
    let canonical = serde_jcs::to_vec(&semantic_payload)
        .context("failed to canonicalize reference manifest payload")?;

    Ok(ReferenceManifest {
        manifest_version: "1.0.0",
        semantic_digest: format!("sha256:{}", hex::encode(Sha256::digest(canonical))),
        canonical_reference: canonical_reference.clone(),
        artifacts: artifacts.to_vec(),
        comparisons: semantic_comparisons,
        coverage,
        delegated_checks: Vec::new(),
    })
}

fn build_artifact_sources(config: &ReferenceConfig) -> Result<Vec<ArtifactSource>> {
    let mut sources = Vec::new();
    if let Some(path) = &config.fai {
        sources.push(build_artifact_source("fai", path)?);
    }
    if let Some(path) = &config.dict {
        sources.push(build_artifact_source("dict", path)?);
    }
    for path in &config.alignments {
        sources.push(build_artifact_source("alignment", path)?);
    }
    for path in &config.variants {
        sources.push(build_artifact_source("variants", path)?);
    }
    for path in &config.annotations {
        sources.push(build_artifact_source("annotation", path)?);
    }
    sources.sort_by(|left, right| {
        left.artifact.kind.cmp(right.artifact.kind).then_with(|| {
            left.artifact
                .declaration_digest
                .cmp(&right.artifact.declaration_digest)
        })
    });
    Ok(sources)
}

fn build_artifact_source(kind: &'static str, path: &Path) -> Result<ArtifactSource> {
    let declaration = normalised_declaration(kind, path);
    let payload = serde_json::json!({
        "kind": kind,
        "declaration": declaration,
    });
    let canonical =
        serde_jcs::to_vec(&payload).context("failed to canonicalize reference declaration")?;
    Ok(ArtifactSource {
        source_path: path.to_path_buf(),
        artifact: ReferenceArtifact {
            kind,
            declaration_digest: format!("sha256:{}", hex::encode(Sha256::digest(canonical))),
            declaration: payload["declaration"].clone(),
        },
    })
}

fn normalised_declaration(kind: &str, path: &Path) -> serde_json::Value {
    let declaration = match kind {
        "fai" => parse_fai(path).and_then(|entries| {
            serde_json::to_value(entries).context("failed to serialize FAI declaration")
        }),
        "dict" => parse_dictionary(path).and_then(|entries| {
            serde_json::to_value(entries).context("failed to serialize dictionary declaration")
        }),
        "alignment" => parse_alignment(path).and_then(|entries| {
            serde_json::to_value(entries).context("failed to serialize alignment declaration")
        }),
        "variants" => parse_variants(path).and_then(|entries| {
            serde_json::to_value(entries).context("failed to serialize variant declaration")
        }),
        "annotation" => parse_gff3(path).and_then(|entries| {
            serde_json::to_value(entries).context("failed to serialize annotation declaration")
        }),
        _ => unreachable!("reference artefact kind is validated by the CLI"),
    };
    declaration.unwrap_or_else(|_| serde_json::json!({ "state": "invalid" }))
}

fn artifact_for_path<'a>(
    sources: &'a [ArtifactSource],
    kind: &str,
    path: &Path,
) -> Result<&'a ReferenceArtifact> {
    sources
        .iter()
        .find(|source| source.artifact.kind == kind && source.source_path == path)
        .map(|source| &source.artifact)
        .context("reference artefact was not registered")
}

fn build_reference_findings(
    config: &ReferenceConfig,
    catalog: &CanonicalReference,
    aliases: Option<&BTreeMap<String, String>>,
    comparisons: &[ReferenceComparison],
    reasons: &[String],
) -> Vec<ReferenceFinding> {
    let mut findings = BTreeMap::new();
    for reason in reasons {
        let artifact_kind = artifact_kind_for_reason(reason);
        let (id, artifact_kind, relationship) =
            if reason.starts_with("required_") && reason.ends_with("_missing") {
                (
                    "reference_required_artifact_missing",
                    artifact_kind,
                    "indeterminate",
                )
            } else if reason.ends_with("_invalid") || reason.ends_with("_unusable") {
                (
                    "reference_malformed_declaration",
                    artifact_kind,
                    "incompatible",
                )
            } else if reason.starts_with("canonical_") {
                (
                    "reference_canonical_reference_invalid",
                    "reference",
                    "incompatible",
                )
            } else {
                (
                    "reference_insufficient_identity_evidence",
                    "reference",
                    "indeterminate",
                )
            };
        insert_reference_finding(
            &mut findings,
            ReferenceFinding::new(id, artifact_kind, relationship, &["insufficient"], true),
        );
    }
    for comparison in comparisons {
        let (detail, blocking) = match comparison.relationship {
            "incompatible" => (
                incompatible_finding_detail(config, catalog, aliases, comparison),
                config.policy != ReferencePolicy::Advisory,
            ),
            "indeterminate" => (
                ReferenceFindingDetail::generic("reference_insufficient_identity_evidence"),
                config.policy == ReferencePolicy::Strict,
            ),
            "content_equivalent"
                if !policy_accepts(config.policy, comparison.kind, comparison.relationship) =>
            {
                (
                    ReferenceFindingDetail::generic("reference_declaration_mismatch"),
                    true,
                )
            }
            "coordinate_compatible" | "subset_compatible"
                if !policy_accepts(config.policy, comparison.kind, comparison.relationship) =>
            {
                (
                    ReferenceFindingDetail::generic("reference_insufficient_identity_evidence"),
                    true,
                )
            }
            _ => continue,
        };
        insert_reference_finding(
            &mut findings,
            ReferenceFinding::new(
                detail.id,
                comparison.kind,
                comparison.relationship,
                &comparison.evidence,
                blocking,
            )
            .with_example(detail.example),
        );
    }
    findings.into_values().collect()
}

fn artifact_kind_for_reason(reason: &str) -> &'static str {
    ["fai", "dict", "alignment", "variants", "annotation"]
        .into_iter()
        .find(|kind| reason.contains(kind))
        .unwrap_or("reference")
}

impl ReferenceFinding {
    fn new(
        id: &'static str,
        artifact_kind: &'static str,
        relationship: &'static str,
        evidence: &[&'static str],
        blocking: bool,
    ) -> Self {
        let (message, suggested_next_step) = match id {
            "reference_required_artifact_missing" => (
                "A required reference companion was not supplied.",
                "Supply a readable declaration for every required companion kind.",
            ),
            "reference_malformed_declaration" => (
                "A supplied reference declaration could not be interpreted safely.",
                "Regenerate the declaration with its producing tool and rerun the reference gate.",
            ),
            "reference_length_mismatch" => (
                "A declared sequence length disagrees with the canonical reference.",
                "Use a companion generated from the same reference coordinate system.",
            ),
            "reference_digest_mismatch" => (
                "A declared sequence digest disagrees with the canonical reference.",
                "Use a companion generated from the same biological reference sequence.",
            ),
            "reference_contig_mismatch" => (
                "A declared sequence name does not resolve to the canonical reference.",
                "Correct the declaration or provide an explicit one-to-one alias map.",
            ),
            "reference_declaration_mismatch" => (
                "The declared reference coordinate system differs from the canonical reference.",
                "Use a companion generated from the same reference coordinate system.",
            ),
            "reference_canonical_reference_invalid" => (
                "The canonical FASTA cannot provide a safe reference identity.",
                "Fix the canonical FASTA before comparing companion declarations.",
            ),
            _ => (
                "The supplied declaration does not provide enough evidence to establish compatibility.",
                "Provide a complete, readable reference declaration with supported identity fields.",
            ),
        };
        Self {
            id,
            artifact_kind,
            relationship,
            evidence: evidence.to_vec(),
            policy_effect: if blocking { "blocking" } else { "advisory" },
            affected_count: 1,
            original_name: None,
            resolved_name: None,
            expected_value: None,
            observed_value: None,
            examples: Vec::new(),
            message,
            suggested_next_step,
        }
    }

    fn with_example(mut self, example: ReferenceFindingExample) -> Self {
        self.original_name = example.original_name.clone();
        self.resolved_name = example.resolved_name.clone();
        self.expected_value = example.expected_value.clone();
        self.observed_value = example.observed_value.clone();
        self.examples.push(example);
        self
    }
}

fn insert_reference_finding(
    findings: &mut BTreeMap<&'static str, ReferenceFinding>,
    finding: ReferenceFinding,
) {
    if let Some(existing) = findings.get_mut(finding.id) {
        existing.affected_count += 1;
        for example in finding.examples {
            if existing.examples.len() < 20
                && !existing.examples.iter().any(|current| {
                    current.original_name == example.original_name
                        && current.resolved_name == example.resolved_name
                        && current.expected_value == example.expected_value
                        && current.observed_value == example.observed_value
                })
            {
                existing.examples.push(example);
            }
        }
    } else {
        findings.insert(finding.id, finding);
    }
}

impl ReferenceFindingDetail {
    fn generic(id: &'static str) -> Self {
        Self {
            id,
            example: ReferenceFindingExample {
                original_name: None,
                resolved_name: None,
                expected_value: None,
                observed_value: None,
            },
        }
    }
}

fn incompatible_finding_detail(
    _config: &ReferenceConfig,
    catalog: &CanonicalReference,
    aliases: Option<&BTreeMap<String, String>>,
    comparison: &ReferenceComparison,
) -> ReferenceFindingDetail {
    if comparison.kind == "variants" {
        if let Ok(entries) = parse_variants(&comparison.source_path) {
            for entry in entries {
                let resolved = resolved_name(&entry.id, aliases);
                let Some(sequence) = catalog
                    .sequences
                    .iter()
                    .find(|sequence| sequence.id == resolved)
                else {
                    return ReferenceFindingDetail {
                        id: "reference_contig_mismatch",
                        example: ReferenceFindingExample {
                            original_name: Some(entry.id.clone()),
                            resolved_name: Some(resolved.to_string()),
                            expected_value: Some("canonical contig name".to_string()),
                            observed_value: Some(resolved.to_string()),
                        },
                    };
                };
                if entry.length.is_some_and(|length| length != sequence.length) {
                    return ReferenceFindingDetail {
                        id: "reference_length_mismatch",
                        example: ReferenceFindingExample {
                            original_name: Some(entry.id.clone()),
                            resolved_name: Some(resolved.to_string()),
                            expected_value: Some(sequence.length.to_string()),
                            observed_value: entry.length.map(|length| length.to_string()),
                        },
                    };
                }
                if entry.md5.as_deref().is_some_and(|md5| {
                    sequence
                        .sam_md5
                        .as_deref()
                        .is_some_and(|expected| expected != md5)
                }) {
                    return ReferenceFindingDetail {
                        id: "reference_digest_mismatch",
                        example: ReferenceFindingExample {
                            original_name: Some(entry.id.clone()),
                            resolved_name: Some(resolved.to_string()),
                            expected_value: sequence.sam_md5.clone(),
                            observed_value: entry.md5,
                        },
                    };
                }
            }
        }
    }
    ReferenceFindingDetail::generic("reference_declaration_mismatch")
}

fn build_reference_readiness(
    status: &'static str,
    findings: &[ReferenceFinding],
) -> ReferenceReadiness {
    let blocking_findings = findings
        .iter()
        .filter(|finding| finding.policy_effect == "blocking")
        .map(|finding| finding.id)
        .collect::<Vec<_>>();
    ReferenceReadiness {
        overall: ReferenceReadinessOverall {
            status,
            blockers: blocking_findings.clone(),
        },
        categories: vec![ReferenceReadinessCategory {
            id: "reference_compatibility",
            label: "Reference compatibility readiness",
            status,
            findings: findings.iter().map(|finding| finding.id).collect(),
        }],
    }
}

fn build_reference_provenance(
    config: &ReferenceConfig,
    sources: &[ArtifactSource],
) -> ReferenceProvenance {
    ReferenceProvenance {
        reference_path: config.reference.display().to_string(),
        companions: sources
            .iter()
            .map(|source| ReferenceCompanionProvenance {
                kind: source.artifact.kind,
                path: source.source_path.display().to_string(),
                declaration_digest: source.artifact.declaration_digest.clone(),
            })
            .collect(),
    }
}

fn build_report(config: &ReferenceConfig) -> Result<ReferenceReport> {
    let canonical_reference = build_catalog(&config.reference)?;
    let artifact_sources = build_artifact_sources(config)?;
    let artifacts = artifact_sources
        .iter()
        .map(|source| source.artifact.clone())
        .collect::<Vec<_>>();
    let policy = policy_id(config.policy);
    let aliases = config
        .alias_map
        .as_deref()
        .map(|path| parse_alias_map(path, &canonical_reference))
        .transpose()?;
    let mut comparisons = Vec::new();
    let mut reasons = canonical_reference.critical_reasons.clone();
    reasons.extend(missing_required_reasons(config));
    if let Some(path) = &config.fai {
        let artifact = artifact_for_path(&artifact_sources, "fai", path)?;
        match compare_fai(
            &canonical_reference,
            &config.reference,
            path,
            aliases.as_ref(),
        ) {
            Ok(relationship) => comparisons.push(ReferenceComparison::new(
                "fai",
                relationship,
                fai_uses_alias(path, aliases.as_ref())?,
                artifact.declaration_digest.clone(),
                path,
            )),
            Err(error) => {
                ensure_declaration_is_readable(path, error)?;
                comparisons.push(ReferenceComparison::new(
                    "fai",
                    "incompatible",
                    false,
                    artifact.declaration_digest.clone(),
                    path,
                ));
                reasons.push(required_or_optional_reason(config, "fai", "invalid"));
            }
        }
    }
    if let Some(path) = &config.dict {
        let artifact = artifact_for_path(&artifact_sources, "dict", path)?;
        match compare_dictionary(&canonical_reference, path, aliases.as_ref()) {
            Ok(relationship) => comparisons.push(ReferenceComparison::new(
                "dict",
                relationship,
                dictionary_uses_alias(path, aliases.as_ref())?,
                artifact.declaration_digest.clone(),
                path,
            )),
            Err(error) => {
                ensure_declaration_is_readable(path, error)?;
                comparisons.push(ReferenceComparison::new(
                    "dict",
                    "incompatible",
                    false,
                    artifact.declaration_digest.clone(),
                    path,
                ));
                reasons.push(required_or_optional_reason(config, "dict", "invalid"));
            }
        }
    }
    for path in &config.alignments {
        let artifact = artifact_for_path(&artifact_sources, "alignment", path)?;
        match compare_alignment(&canonical_reference, path, aliases.as_ref()) {
            Ok(relationship) => comparisons.push(ReferenceComparison::new(
                "alignment",
                relationship,
                alignment_uses_alias(path, aliases.as_ref())?,
                artifact.declaration_digest.clone(),
                path,
            )),
            Err(error) => {
                ensure_declaration_is_readable(path, error)?;
                comparisons.push(ReferenceComparison::new(
                    "alignment",
                    "incompatible",
                    false,
                    artifact.declaration_digest.clone(),
                    path,
                ));
                reasons.push(required_or_optional_reason(config, "alignment", "invalid"));
            }
        }
    }
    for path in &config.variants {
        let artifact = artifact_for_path(&artifact_sources, "variants", path)?;
        match compare_variants(&canonical_reference, path, aliases.as_ref()) {
            Ok(relationship) => {
                if relationship == "indeterminate" && config.required_artifacts.contains("variants")
                {
                    reasons.push("required_variants_unusable".to_string());
                }
                comparisons.push(ReferenceComparison::new(
                    "variants",
                    relationship,
                    variants_use_alias(path, aliases.as_ref())?,
                    artifact.declaration_digest.clone(),
                    path,
                ));
            }
            Err(error) => {
                ensure_declaration_is_readable(path, error)?;
                comparisons.push(ReferenceComparison::new(
                    "variants",
                    "incompatible",
                    false,
                    artifact.declaration_digest.clone(),
                    path,
                ));
                reasons.push(required_or_optional_reason(config, "variants", "invalid"));
            }
        }
    }
    for path in &config.annotations {
        let artifact = artifact_for_path(&artifact_sources, "annotation", path)?;
        match compare_annotation(&canonical_reference, path, aliases.as_ref()) {
            Ok(relationship) => {
                if relationship == "indeterminate"
                    && config.required_artifacts.contains("annotation")
                {
                    reasons.push("required_annotation_unusable".to_string());
                }
                comparisons.push(ReferenceComparison::new(
                    "annotation",
                    relationship,
                    annotation_uses_alias(path, aliases.as_ref())?,
                    artifact.declaration_digest.clone(),
                    path,
                ));
            }
            Err(error) => {
                ensure_declaration_is_readable(path, error)?;
                comparisons.push(ReferenceComparison::new(
                    "annotation",
                    "incompatible",
                    false,
                    artifact.declaration_digest.clone(),
                    path,
                ));
                reasons.push(required_or_optional_reason(config, "annotation", "invalid"));
            }
        }
    }
    let passes = reasons.is_empty()
        && !comparisons.is_empty()
        && comparisons.iter().all(|comparison| {
            policy_accepts(config.policy, comparison.kind, comparison.relationship)
        });
    if comparisons.is_empty() && reasons.is_empty() {
        reasons.push("no_reference_declarations".to_string());
    }
    let has_incompatible = comparisons
        .iter()
        .any(|comparison| comparison.relationship == "incompatible");
    let has_indeterminate = comparisons
        .iter()
        .any(|comparison| comparison.relationship == "indeterminate");
    let has_blocking_reason = reasons.iter().any(|reason| {
        reason.starts_with("required_")
            || reason.starts_with("canonical_")
            || reason.ends_with("_invalid")
            || reason.ends_with("_unusable")
    });
    let has_non_exact = comparisons
        .iter()
        .any(|comparison| comparison.relationship != "exact");
    let status = if has_blocking_reason || has_incompatible {
        "FAIL"
    } else if has_non_exact || has_indeterminate {
        "WARN"
    } else if passes {
        "PASS"
    } else {
        "WARN"
    };

    let reference_manifest = build_manifest(
        &canonical_reference,
        &artifacts,
        &comparisons,
        config,
        aliases.as_ref(),
    )?;
    let findings = build_reference_findings(
        config,
        &canonical_reference,
        aliases.as_ref(),
        &comparisons,
        &reasons,
    );
    let blocking_findings = findings
        .iter()
        .filter(|finding| finding.policy_effect == "blocking")
        .map(|finding| finding.id)
        .collect::<Vec<_>>();
    let advisory_findings = findings
        .iter()
        .filter(|finding| finding.policy_effect == "advisory")
        .map(|finding| finding.id)
        .collect::<Vec<_>>();
    let readiness = build_reference_readiness(status, &findings);
    let provenance = build_reference_provenance(config, &artifact_sources);
    Ok(ReferenceReport {
        schema_version: REFERENCE_SCHEMA_VERSION,
        report_type: "reference",
        tool: ReferenceTool {
            name: "FastaGuard",
            version: env!("CARGO_PKG_VERSION"),
        },
        canonical_reference,
        reference_manifest,
        comparisons,
        findings,
        verdict: ReferenceVerdict { status, reasons },
        gate: ReferenceGate {
            mode: "reference",
            reference_policy: ReferencePolicyReport {
                id: policy,
                version: "1.0.0",
                required_kinds: config.required_artifacts.iter().cloned().collect(),
            },
            status,
            can_continue: passes,
            blocking_findings,
            advisory_findings,
        },
        readiness,
        machine_summary: ReferenceMachineSummary {
            safe_for_downstream: status == "PASS",
        },
        provenance,
    })
}

fn fai_uses_alias(path: &Path, aliases: Option<&BTreeMap<String, String>>) -> Result<bool> {
    Ok(aliases.is_some_and(|aliases| {
        parse_fai(path)
            .map(|entries| entries.iter().any(|entry| aliases.contains_key(&entry.id)))
            .unwrap_or(false)
    }))
}

fn dictionary_uses_alias(path: &Path, aliases: Option<&BTreeMap<String, String>>) -> Result<bool> {
    Ok(aliases.is_some_and(|aliases| {
        parse_dictionary(path)
            .map(|entries| entries.iter().any(|entry| aliases.contains_key(&entry.id)))
            .unwrap_or(false)
    }))
}

fn alignment_uses_alias(path: &Path, aliases: Option<&BTreeMap<String, String>>) -> Result<bool> {
    Ok(aliases.is_some_and(|aliases| {
        parse_alignment(path)
            .map(|entries| entries.iter().any(|entry| aliases.contains_key(&entry.id)))
            .unwrap_or(false)
    }))
}

fn variants_use_alias(path: &Path, aliases: Option<&BTreeMap<String, String>>) -> Result<bool> {
    Ok(aliases.is_some_and(|aliases| {
        parse_variants(path)
            .map(|entries| entries.iter().any(|entry| aliases.contains_key(&entry.id)))
            .unwrap_or(false)
    }))
}

fn annotation_uses_alias(path: &Path, aliases: Option<&BTreeMap<String, String>>) -> Result<bool> {
    Ok(aliases.is_some_and(|aliases| {
        parse_gff3(path)
            .map(|entries| entries.iter().any(|entry| aliases.contains_key(&entry.id)))
            .unwrap_or(false)
    }))
}

fn ensure_declaration_is_readable(path: &Path, original_error: anyhow::Error) -> Result<()> {
    File::open(path)
        .with_context(|| {
            format!(
                "failed to open declared reference artefact {}",
                path.display()
            )
        })
        .map(|_| ())
        .map_err(|_| original_error)
}

fn required_or_optional_reason(config: &ReferenceConfig, kind: &str, state: &str) -> String {
    if config.required_artifacts.contains(kind) {
        format!("required_{kind}_{state}")
    } else {
        format!("{kind}_{state}")
    }
}

fn missing_required_reasons(config: &ReferenceConfig) -> Vec<String> {
    let mut reasons = Vec::new();
    if config.required_artifacts.contains("fai") && config.fai.is_none() {
        reasons.push("required_fai_missing".to_string());
    }
    if config.required_artifacts.contains("dict") && config.dict.is_none() {
        reasons.push("required_dict_missing".to_string());
    }
    if config.required_artifacts.contains("alignment") && config.alignments.is_empty() {
        reasons.push("required_alignment_missing".to_string());
    }
    if config.required_artifacts.contains("variants") && config.variants.is_empty() {
        reasons.push("required_variants_missing".to_string());
    }
    if config.required_artifacts.contains("annotation") && config.annotations.is_empty() {
        reasons.push("required_annotation_missing".to_string());
    }
    reasons
}

fn compare_dictionary(
    catalog: &CanonicalReference,
    path: &Path,
    aliases: Option<&BTreeMap<String, String>>,
) -> Result<&'static str> {
    let declared = parse_dictionary(path)?;
    compare_dictionary_entries(catalog, declared, aliases)
}

fn compare_alignment(
    catalog: &CanonicalReference,
    path: &Path,
    aliases: Option<&BTreeMap<String, String>>,
) -> Result<&'static str> {
    let declared = parse_alignment(path)?;
    compare_dictionary_entries(catalog, declared, aliases)
}

fn compare_dictionary_entries(
    catalog: &CanonicalReference,
    declared: Vec<DictionaryEntry>,
    aliases: Option<&BTreeMap<String, String>>,
) -> Result<&'static str> {
    if declared.len() != catalog.sequences.len() {
        return Ok("incompatible");
    }

    let same_order = declared
        .iter()
        .zip(&catalog.sequences)
        .all(|(entry, sequence)| resolved_name(&entry.id, aliases) == sequence.id);
    let matching = catalog
        .sequences
        .iter()
        .map(|sequence| {
            declared
                .iter()
                .find(|entry| resolved_name(&entry.id, aliases) == sequence.id)
                .map(|entry| (entry, sequence))
        })
        .collect::<Option<Vec<_>>>();
    let Some(matching) = matching else {
        return Ok("incompatible");
    };
    if matching
        .iter()
        .any(|(entry, sequence)| entry.length != sequence.length)
    {
        return Ok("incompatible");
    }

    if matching
        .iter()
        .all(|(entry, sequence)| entry.md5.as_deref() == sequence.sam_md5.as_deref())
    {
        return Ok(if same_order {
            "exact"
        } else {
            "content_equivalent"
        });
    }

    Ok("coordinate_compatible")
}

fn compare_variants(
    catalog: &CanonicalReference,
    path: &Path,
    aliases: Option<&BTreeMap<String, String>>,
) -> Result<&'static str> {
    let declared = parse_variants(path)?;
    if declared.is_empty() {
        return Ok("indeterminate");
    }

    let mut matching = Vec::with_capacity(declared.len());
    for entry in &declared {
        let resolved = resolved_name(&entry.id, aliases);
        let Some(sequence) = catalog
            .sequences
            .iter()
            .find(|sequence| sequence.id == resolved)
        else {
            return Ok("incompatible");
        };
        if entry.length.is_some_and(|length| length != sequence.length)
            || entry
                .md5
                .as_deref()
                .is_some_and(|md5| sequence.sam_md5.as_deref() != Some(md5))
        {
            return Ok("incompatible");
        }
        matching.push((entry, sequence));
    }

    if declared.len() < catalog.sequences.len() {
        return Ok("subset_compatible");
    }
    if declared.len() != catalog.sequences.len() {
        return Ok("incompatible");
    }
    if matching.iter().all(|(entry, _)| entry.md5.is_some()) {
        Ok("exact")
    } else if matching.iter().all(|(entry, _)| entry.length.is_some()) {
        Ok("coordinate_compatible")
    } else {
        Ok("indeterminate")
    }
}

fn compare_annotation(
    catalog: &CanonicalReference,
    path: &Path,
    aliases: Option<&BTreeMap<String, String>>,
) -> Result<&'static str> {
    let declared = parse_gff3(path)?;
    if declared.is_empty() {
        return Ok("indeterminate");
    }

    let mut circular_sequences = BTreeSet::new();
    for entry in &declared {
        if !entry.is_circular_marker {
            continue;
        }
        let resolved = resolved_name(&entry.id, aliases);
        let Some(sequence) = catalog
            .sequences
            .iter()
            .find(|sequence| sequence.id == resolved)
        else {
            return Ok("incompatible");
        };
        if entry.start != 1 || entry.end != sequence.length {
            return Ok("incompatible");
        }
        circular_sequences.insert(resolved.to_string());
    }

    let mut names = BTreeSet::new();
    for entry in &declared {
        let resolved = resolved_name(&entry.id, aliases);
        let Some(sequence) = catalog
            .sequences
            .iter()
            .find(|sequence| sequence.id == resolved)
        else {
            return Ok("incompatible");
        };
        let is_one_revolution = entry
            .end
            .checked_sub(entry.start)
            .and_then(|span| span.checked_add(1))
            .is_some_and(|span| span <= sequence.length);
        let valid_circular_overrun = entry.end > sequence.length
            && entry.start <= sequence.length
            && is_one_revolution
            && circular_sequences.contains(resolved);
        if entry.start == 0
            || entry.start > entry.end
            || (entry.end > sequence.length && !valid_circular_overrun)
        {
            return Ok("incompatible");
        }
        if entry.is_sequence_region && (entry.start != 1 || entry.end != sequence.length) {
            return Ok("incompatible");
        }
        names.insert(resolved);
    }

    if names.len() < catalog.sequences.len() {
        Ok("subset_compatible")
    } else {
        Ok("coordinate_compatible")
    }
}

fn parse_alias_map(path: &Path, catalog: &CanonicalReference) -> Result<BTreeMap<String, String>> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let mut lines = contents.lines();
    if lines.next() != Some("declared_name\treference_name") {
        anyhow::bail!("alias map must start with declared_name\\treference_name");
    }
    let canonical_names = catalog
        .sequences
        .iter()
        .map(|sequence| sequence.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut aliases = BTreeMap::new();
    let mut mapped_names = BTreeSet::new();
    for (line_index, line) in lines.enumerate() {
        if line.is_empty() {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 2
            || fields[0].is_empty()
            || fields[1].is_empty()
            || !canonical_names.contains(fields[1])
            || !mapped_names.insert(fields[1])
            || aliases
                .insert(fields[0].to_string(), fields[1].to_string())
                .is_some()
        {
            anyhow::bail!("invalid alias map entry at line {}", line_index + 2);
        }
    }
    Ok(aliases)
}

fn resolved_name<'a>(name: &'a str, aliases: Option<&'a BTreeMap<String, String>>) -> &'a str {
    aliases
        .and_then(|aliases| aliases.get(name))
        .map(String::as_str)
        .unwrap_or(name)
}

fn policy_accepts(policy: ReferencePolicy, kind: &str, relationship: &str) -> bool {
    match policy {
        ReferencePolicy::Strict => relationship == "exact",
        ReferencePolicy::Coordinate => {
            matches!(relationship, "exact" | "coordinate_compatible")
                || (matches!(kind, "variants" | "annotation")
                    && relationship == "subset_compatible")
        }
        ReferencePolicy::Advisory => true,
    }
}

fn parse_dictionary(path: &Path) -> Result<Vec<DictionaryEntry>> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    parse_dictionary_text(&contents)
}

fn parse_alignment(path: &Path) -> Result<Vec<DictionaryEntry>> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    let header = match extension.as_deref() {
        Some("bam") => {
            let file =
                File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
            let mut reader = bam::io::Reader::new(file);
            reader
                .read_header()
                .with_context(|| format!("failed to read BAM header from {}", path.display()))?
        }
        Some("cram") => {
            let mut reader = cram::io::reader::Builder::default()
                .build_from_path(path)
                .with_context(|| format!("failed to open CRAM header from {}", path.display()))?;
            reader
                .read_header()
                .with_context(|| format!("failed to read CRAM header from {}", path.display()))?
        }
        Some("sam") => return parse_dictionary(path),
        _ => anyhow::bail!(
            "unsupported alignment format for {}; expected BAM, CRAM or SAM",
            path.display()
        ),
    };
    parse_sam_header(&header)
}

fn parse_sam_header(header: &sam::Header) -> Result<Vec<DictionaryEntry>> {
    let mut contents = Vec::new();
    sam::io::Writer::new(&mut contents)
        .write_header(header)
        .context("failed to serialize alignment header")?;
    let contents = std::str::from_utf8(&contents).context("alignment header is not UTF-8")?;
    parse_dictionary_text(contents)
}

fn parse_variants(path: &Path) -> Result<Vec<VariantEntry>> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    let header = match extension.as_deref() {
        Some("bcf") => {
            let file =
                File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
            let mut reader = bcf::io::Reader::new(file);
            reader
                .read_header()
                .with_context(|| format!("failed to read BCF header from {}", path.display()))?
        }
        Some("vcf") => read_vcf_header(
            BufReader::new(
                File::open(path).with_context(|| format!("failed to open {}", path.display()))?,
            ),
            path,
        )?,
        Some("gz") => read_vcf_header(
            BufReader::new(MultiGzDecoder::new(
                File::open(path).with_context(|| format!("failed to open {}", path.display()))?,
            )),
            path,
        )?,
        _ => anyhow::bail!(
            "unsupported variant format for {}; expected VCF, VCF.GZ or BCF",
            path.display()
        ),
    };
    parse_vcf_header(&header)
}

fn read_vcf_header<R: BufRead>(reader: R, path: &Path) -> Result<vcf::Header> {
    vcf::io::Reader::new(reader)
        .read_header()
        .with_context(|| format!("failed to read VCF header from {}", path.display()))
}

fn parse_vcf_header(header: &vcf::Header) -> Result<Vec<VariantEntry>> {
    let mut contents = Vec::new();
    vcf::io::Writer::new(&mut contents)
        .write_header(header)
        .context("failed to serialize variant header")?;
    let contents = std::str::from_utf8(&contents).context("variant header is not UTF-8")?;
    let mut entries = Vec::new();
    for line in contents
        .lines()
        .filter(|line| line.starts_with("##contig=<"))
    {
        let fields = line
            .strip_prefix("##contig=<")
            .and_then(|line| line.strip_suffix('>'))
            .context("invalid VCF contig declaration")?
            .split(',')
            .filter_map(|field| field.split_once('='))
            .collect::<BTreeMap<_, _>>();
        let id = fields
            .get("ID")
            .filter(|id| !id.is_empty())
            .context("VCF contig declaration is missing ID")?;
        if entries.iter().any(|entry: &VariantEntry| entry.id == *id) {
            anyhow::bail!("VCF header has duplicate contig declaration for {id}");
        }
        let length = fields
            .get("length")
            .or_else(|| fields.get("Length"))
            .map(|length| length.parse::<usize>().context("invalid VCF contig length"))
            .transpose()?;
        if length == Some(0) {
            anyhow::bail!("invalid VCF contig length");
        }
        let md5 = fields
            .get("md5")
            .or_else(|| fields.get("MD5"))
            .map(|md5| md5.to_string());
        if md5.as_deref().is_some_and(|md5| {
            md5.len() != 32
                || !md5
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        }) {
            anyhow::bail!("invalid VCF contig MD5");
        }
        entries.push(VariantEntry {
            id: (*id).to_string(),
            length,
            md5,
        });
    }
    Ok(entries)
}

fn parse_gff3(path: &Path) -> Result<Vec<AnnotationEntry>> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut entries = Vec::new();
    let mut has_version = false;
    for (line_index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.with_context(|| format!("failed to read {}", path.display()))?;
        if line == "##gff-version 3" {
            has_version = true;
            continue;
        }
        if let Some(declaration) = line.strip_prefix("##sequence-region ") {
            let fields = declaration.split_ascii_whitespace().collect::<Vec<_>>();
            if fields.len() != 3 || fields[0].is_empty() {
                anyhow::bail!(
                    "invalid GFF3 sequence-region directive at line {}",
                    line_index + 1
                );
            }
            entries.push(AnnotationEntry {
                id: fields[0].to_string(),
                start: parse_gff3_coordinate(fields[1], line_index + 1)?,
                end: parse_gff3_coordinate(fields[2], line_index + 1)?,
                is_sequence_region: true,
                is_circular_marker: false,
            });
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 9 || fields[0].is_empty() {
            anyhow::bail!("invalid GFF3 feature at line {}", line_index + 1);
        }
        entries.push(AnnotationEntry {
            id: fields[0].to_string(),
            start: parse_gff3_coordinate(fields[3], line_index + 1)?,
            end: parse_gff3_coordinate(fields[4], line_index + 1)?,
            is_sequence_region: false,
            is_circular_marker: fields[8]
                .split(';')
                .any(|attribute| attribute == "Is_circular=true"),
        });
    }
    if !has_version {
        anyhow::bail!("annotation must be GFF3 with a ##gff-version 3 directive");
    }
    Ok(entries)
}

fn parse_gff3_coordinate(value: &str, line_index: usize) -> Result<usize> {
    let coordinate = value
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("invalid GFF3 coordinate at line {line_index}"))?;
    if coordinate == 0 {
        anyhow::bail!("invalid GFF3 coordinate at line {line_index}");
    }
    Ok(coordinate)
}

fn parse_dictionary_text(contents: &str) -> Result<Vec<DictionaryEntry>> {
    let mut entries = Vec::new();
    for (line_index, line) in contents.lines().enumerate() {
        if !line.starts_with("@SQ\t") {
            continue;
        }
        let mut id = None;
        let mut length = None;
        let mut md5 = None;
        for tag in line.split('\t').skip(1) {
            if let Some(value) = tag.strip_prefix("SN:") {
                id = Some(value.to_string());
            } else if let Some(value) = tag.strip_prefix("LN:") {
                length = Some(value.parse::<usize>().map_err(|_| {
                    anyhow::anyhow!("invalid SAM dictionary entry at line {}", line_index + 1)
                })?);
            } else if let Some(value) = tag.strip_prefix("M5:") {
                if value.len() != 32
                    || !value
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
                {
                    anyhow::bail!("invalid SAM dictionary entry at line {}", line_index + 1);
                }
                md5 = Some(value.to_string());
            }
        }
        let id = id.ok_or_else(|| {
            anyhow::anyhow!("invalid SAM dictionary entry at line {}", line_index + 1)
        })?;
        let length = length.ok_or_else(|| {
            anyhow::anyhow!("invalid SAM dictionary entry at line {}", line_index + 1)
        })?;
        if length == 0 || entries.iter().any(|entry: &DictionaryEntry| entry.id == id) {
            anyhow::bail!("invalid SAM dictionary entry at line {}", line_index + 1);
        }
        entries.push(DictionaryEntry { id, length, md5 });
    }
    if entries.is_empty() {
        anyhow::bail!("SAM dictionary contains no @SQ entries");
    }
    Ok(entries)
}

fn compare_fai(
    catalog: &CanonicalReference,
    fasta_path: &Path,
    fai_path: &Path,
    aliases: Option<&BTreeMap<String, String>>,
) -> Result<&'static str> {
    let declared = parse_fai(fai_path)?;
    let declared = declared
        .into_iter()
        .map(|entry| FaiEntry {
            id: resolved_name(&entry.id, aliases).to_string(),
            ..entry
        })
        .collect::<Vec<_>>();
    if declared.len() != catalog.sequences.len()
        || declared
            .iter()
            .zip(&catalog.sequences)
            .any(|(entry, sequence)| entry.id != sequence.id || entry.length != sequence.length)
    {
        return Ok("incompatible");
    }

    let Some(observed) = observed_fai_layout(fasta_path)? else {
        return Ok("coordinate_compatible");
    };
    if declared == observed {
        Ok("exact")
    } else {
        Ok("incompatible")
    }
}

fn parse_fai(path: &Path) -> Result<Vec<FaiEntry>> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let mut entries = Vec::new();
    for (line_index, line) in contents.lines().enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 5 || fields[0].is_empty() {
            anyhow::bail!("invalid FAI entry at line {}", line_index + 1);
        }
        let numbers = fields[1..]
            .iter()
            .map(|field| {
                field
                    .parse::<usize>()
                    .map_err(|_| anyhow::anyhow!("invalid FAI entry at line {}", line_index + 1))
            })
            .collect::<Result<Vec<_>>>()?;
        if numbers[0] == 0 || numbers[3] == 0 {
            anyhow::bail!("invalid FAI entry at line {}", line_index + 1);
        }
        if entries.iter().any(|entry: &FaiEntry| entry.id == fields[0]) {
            anyhow::bail!("invalid FAI entry at line {}", line_index + 1);
        }
        entries.push(FaiEntry {
            id: fields[0].to_string(),
            length: numbers[0],
            offset: numbers[1],
            line_bases: numbers[2],
            line_width: numbers[3],
        });
    }
    if entries.is_empty() {
        anyhow::bail!("FAI contains no entries");
    }
    Ok(entries)
}

fn observed_fai_layout(path: &Path) -> Result<Option<Vec<FaiEntry>>> {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("gz"))
    {
        return Ok(None);
    }

    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut entries = Vec::new();
    let mut current: Option<FaiEntry> = None;
    let mut offset = 0usize;

    for raw_line in bytes.split_inclusive(|byte| *byte == b'\n') {
        let line_offset = offset;
        offset += raw_line.len();
        let line = raw_line
            .strip_suffix(b"\n")
            .unwrap_or(raw_line)
            .strip_suffix(b"\r")
            .unwrap_or(raw_line.strip_suffix(b"\n").unwrap_or(raw_line));
        if let Some(header) = line.strip_prefix(b">") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let id = std::str::from_utf8(header)
                .ok()
                .and_then(|value| value.split_whitespace().next())
                .unwrap_or_default();
            current = Some(FaiEntry {
                id: id.to_string(),
                length: 0,
                offset: 0,
                line_bases: 0,
                line_width: 0,
            });
        } else if !line.is_empty() {
            let entry = current
                .as_mut()
                .context("sequence line before FASTA header while reading FAI layout")?;
            if entry.line_bases == 0 {
                entry.offset = line_offset;
                entry.line_bases = line.len();
                entry.line_width = raw_line.len();
            }
            entry.length += line.len();
        }
    }
    if let Some(entry) = current {
        entries.push(entry);
    }
    Ok(Some(entries))
}

fn build_catalog(path: &Path) -> Result<CanonicalReference> {
    let mut sequences = Vec::new();
    let mut seen_ids = BTreeSet::new();
    let mut critical_reasons = BTreeSet::new();
    parser::for_each_fasta_record(path, |record| {
        let sequence = record
            .sequence
            .iter()
            .map(u8::to_ascii_uppercase)
            .collect::<Vec<_>>();
        if !seen_ids.insert(record.id.clone()) {
            critical_reasons.insert("canonical_duplicate_sequence_id".to_string());
        }
        let valid_sequence = sequence.iter().all(|byte| is_iupac_base(*byte));
        if !valid_sequence {
            critical_reasons.insert("canonical_invalid_sequence_symbols".to_string());
        }
        sequences.push(CanonicalSequence {
            id: record.id,
            order: sequences.len(),
            length: sequence.len(),
            sam_md5: valid_sequence.then(|| hex::encode(Md5::digest(&sequence))),
            refget_id: valid_sequence.then(|| format!("ga4gh:SQ.{}", sha512t24u(&sequence))),
        });
        Ok(())
    })?;

    let (seqcol_digest, name_length_pairs_digest, sorted_name_length_pairs_digest) =
        if critical_reasons.is_empty() {
            let (seqcol, name_length, sorted_name_length) = seqcol_digests(&sequences)?;
            (Some(seqcol), Some(name_length), Some(sorted_name_length))
        } else {
            (None, None, None)
        };
    Ok(CanonicalReference {
        physical_sha256: file_sha256(path)?,
        seqcol_schema: SEQCOL_SCHEMA,
        seqcol_digest,
        name_length_pairs_digest,
        sorted_name_length_pairs_digest,
        sequences,
        critical_reasons: critical_reasons.into_iter().collect(),
    })
}

fn seqcol_digests(sequences: &[CanonicalSequence]) -> Result<(String, String, String)> {
    let names = sequences
        .iter()
        .map(|sequence| sequence.id.as_str())
        .collect::<Vec<_>>();
    let sequence_ids = sequences
        .iter()
        .map(|sequence| {
            sequence
                .refget_id
                .as_deref()
                .expect("valid canonical sequences must have refget identifiers")
                .trim_start_matches("ga4gh:")
        })
        .collect::<Vec<_>>();
    let names_digest = seqcol_attribute_digest(&names)?;
    let sequences_digest = seqcol_attribute_digest(&sequence_ids)?;
    let seqcol_digest = seqcol_attribute_digest(&serde_json::json!({
        "names": names_digest,
        "sequences": sequences_digest,
    }))?;

    let pairs = sequences
        .iter()
        .map(|sequence| serde_json::json!({ "name": sequence.id, "length": sequence.length }))
        .collect::<Vec<_>>();
    let name_length_pairs_digest = seqcol_attribute_digest(&pairs)?;
    let mut pair_digests = pairs
        .iter()
        .map(seqcol_attribute_digest)
        .collect::<Result<Vec<_>>>()?;
    pair_digests.sort();
    let sorted_name_length_pairs_digest = seqcol_attribute_digest(&pair_digests)?;

    Ok((
        seqcol_digest,
        name_length_pairs_digest,
        sorted_name_length_pairs_digest,
    ))
}

fn seqcol_attribute_digest(value: &impl Serialize) -> Result<String> {
    let canonical = serde_jcs::to_vec(value).context("failed to canonicalize SeqCol attribute")?;
    Ok(sha512t24u(&canonical))
}

fn sha512t24u(bytes: &[u8]) -> String {
    let sha512 = Sha512::digest(bytes);
    URL_SAFE_NO_PAD.encode(&sha512[..24])
}

fn is_iupac_base(byte: u8) -> bool {
    matches!(
        byte,
        b'A' | b'C'
            | b'G'
            | b'T'
            | b'U'
            | b'N'
            | b'M'
            | b'R'
            | b'W'
            | b'S'
            | b'Y'
            | b'K'
            | b'V'
            | b'H'
            | b'D'
            | b'B'
    )
}

fn file_sha256(path: &Path) -> Result<String> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn policy_id(policy: ReferencePolicy) -> &'static str {
    match policy {
        ReferencePolicy::Strict => "strict",
        ReferencePolicy::Coordinate => "coordinate",
        ReferencePolicy::Advisory => "advisory",
    }
}

fn write_document<T: Serialize>(report: &T, path: &Path, allow_overwrite: bool) -> Result<()> {
    if path.exists() && !allow_overwrite {
        anyhow::bail!("output path already exists: {}", path.display());
    }
    let file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, report)
        .with_context(|| format!("failed to serialize {}", path.display()))?;
    writer
        .write_all(b"\n")
        .with_context(|| format!("failed to write {}", path.display()))?;
    writer
        .flush()
        .with_context(|| format!("failed to flush {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_identifiers_are_stable() {
        assert_eq!(policy_id(ReferencePolicy::Strict), "strict");
        assert_eq!(policy_id(ReferencePolicy::Coordinate), "coordinate");
        assert_eq!(policy_id(ReferencePolicy::Advisory), "advisory");
    }
}
