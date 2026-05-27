use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::models::{FastaguardReport, Severity, VerdictStatus};

pub fn write(report: &FastaguardReport, path: &Path) -> Result<()> {
    let html = render(report)?;
    fs::write(path, html).with_context(|| format!("failed to write HTML report {}", path.display()))
}

fn render(report: &FastaguardReport) -> Result<String> {
    let summary = &report.summary;
    let machine_summary = render_machine_summary(report);
    let scope = render_scope(report);
    let plots = render_plots(report);
    let findings = render_findings(report);
    let json = serde_json::to_string_pretty(report).context("failed to serialize report JSON")?;

    Ok(format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>FastaGuard Report</title>
<style>
:root {{ color-scheme: light; font-family: Arial, Helvetica, sans-serif; }}
body {{ margin: 0; background: #f7f7f4; color: #202124; }}
main {{ max-width: 980px; margin: 0 auto; padding: 32px 20px 48px; }}
h1 {{ margin: 0 0 8px; font-size: 34px; }}
h2 {{ margin-top: 32px; border-bottom: 1px solid #d8d8d2; padding-bottom: 8px; }}
h3 {{ margin-top: 0; }}
h4 {{ margin: 18px 0 8px; }}
table {{ width: 100%; border-collapse: collapse; background: #ffffff; }}
th, td {{ border: 1px solid #d8d8d2; padding: 10px 12px; text-align: left; }}
th {{ background: #ecece6; }}
.verdict {{ display: inline-block; margin: 12px 0; padding: 6px 10px; font-weight: 700; border: 1px solid #202124; }}
.positioning {{ color: #4f565c; margin: 0 0 24px; }}
.finding {{ background: #ffffff; border: 1px solid #d8d8d2; padding: 16px; margin: 14px 0; }}
.finding h3 {{ margin: 0 0 10px; font-size: 18px; }}
.label {{ font-weight: 700; }}
.grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 12px; }}
.panel {{ background: #ffffff; border: 1px solid #d8d8d2; padding: 14px; }}
.muted {{ color: #596066; }}
.nowrap {{ white-space: nowrap; }}
.table-scroll {{ overflow-x: auto; }}
.table-scroll table {{ min-width: 760px; }}
.plot-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 18px; }}
.plot {{ background: #ffffff; border: 1px solid #d8d8d2; padding: 14px; }}
.plot svg {{ width: 100%; height: auto; display: block; }}
.bar {{ fill: #437c90; }}
.point {{ fill: #5b7f3a; opacity: 0.78; }}
.point.gc-outlier {{ fill: #b23a48; opacity: 0.9; }}
.axis {{ stroke: #5f6368; stroke-width: 1; }}
.axis-label {{ fill: #4f565c; font-size: 12px; }}
pre {{ overflow-x: auto; background: #202124; color: #f7f7f4; padding: 16px; }}
</style>
</head>
<body>
<main>
<h1>FastaGuard Report</h1>
<div class="verdict">Verdict: {verdict}</div>
<p class="positioning">Before QUAST. Before BUSCO. Before BlobToolKit. Run FastaGuard first.</p>
<h2>Machine Summary</h2>
{machine_summary}
<h2>Summary</h2>
<table>
<thead><tr><th>Metric</th><th>Value</th></tr></thead>
<tbody>
<tr><td>Sequences</td><td>{sequence_count}</td></tr>
<tr><td>Total length</td><td>{total_length}</td></tr>
<tr><td>N50</td><td>{n50}</td></tr>
<tr><td>N90</td><td>{n90}</td></tr>
<tr><td>GC%</td><td>{gc_percent}</td></tr>
<tr><td>N%</td><td>{n_percent}</td></tr>
</tbody>
</table>
<h2>Scope</h2>
{scope}
<h2>Plots</h2>
{plots}
<h2>Findings</h2>
{findings}
<h2>JSON</h2>
<pre>{json}</pre>
</main>
</body>
</html>
"#,
        verdict = escape_html(verdict_status(report.verdict.status)),
        machine_summary = machine_summary,
        sequence_count = summary.sequence_count,
        total_length = summary.total_length,
        n50 = summary.n50,
        n90 = summary.n90,
        gc_percent = summary.gc_percent,
        n_percent = summary.n_percent,
        scope = scope,
        plots = plots,
        findings = findings,
        json = escape_html(&json),
    ))
}

fn render_machine_summary(report: &FastaguardReport) -> String {
    let summary = &report.machine_summary;
    let top_findings = if summary.top_findings.is_empty() {
        "None".to_string()
    } else {
        summary
            .top_findings
            .iter()
            .map(|finding| escape_html(finding))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let safe = if summary.safe_for_downstream {
        "Yes"
    } else {
        "No"
    };

    format!(
        r#"<div class="grid">
<section class="panel">
<h3>Verdict</h3>
<p><span class="label">Machine verdict:</span> {verdict}</p>
<p><span class="label">Safe for downstream:</span> {safe}</p>
<p><span class="label">Top findings:</span> {top_findings}</p>
</section>
<section class="panel">
<h3>Recommended Next Tools</h3>
{tools}
</section>
</div>"#,
        verdict = escape_html(verdict_status(summary.verdict)),
        safe = safe,
        top_findings = top_findings,
        tools = render_recommended_tools(report),
    )
}

fn render_recommended_tools(report: &FastaguardReport) -> String {
    let tools = &report.machine_summary.recommended_next_tools;
    if tools.is_empty() {
        return "<p>No tool recommendations.</p>".to_string();
    }

    let rows = tools
        .iter()
        .map(|tool| {
            format!(
                "<tr><td>{tool}</td><td>{reason}</td></tr>",
                tool = escape_html(&tool.tool),
                reason = escape_html(&tool.reason),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<table>
<thead><tr><th>Tool</th><th>Reason</th></tr></thead>
<tbody>{rows}</tbody>
</table>"#
    )
}

fn render_scope(report: &FastaguardReport) -> String {
    format!(
        r#"<div class="grid">
<section class="panel">
<h3>Can Conclude</h3>
{can_conclude}
</section>
<section class="panel">
<h3>Cannot Conclude</h3>
{cannot_conclude}
</section>
</div>"#,
        can_conclude = render_string_list(&report.scope.can_conclude),
        cannot_conclude = render_string_list(&report.scope.cannot_conclude),
    )
}

fn render_plots(report: &FastaguardReport) -> String {
    format!(
        r#"<div class="plot-grid">
<section class="plot">
<h3>Length Histogram</h3>
{histogram}
</section>
<section class="plot">
<h3>GC vs Length</h3>
{gc_length}
</section>
</div>"#,
        histogram = render_length_histogram(report),
        gc_length = render_gc_length_plot(report),
    )
}

fn render_length_histogram(report: &FastaguardReport) -> String {
    let bins = &report.plots.length_histogram;
    if bins.is_empty() {
        return r#"<p class="muted">No length histogram data available.</p>"#.to_string();
    }

    let width = 720.0;
    let height = 260.0;
    let left = 48.0;
    let right = 16.0;
    let top = 18.0;
    let bottom = 42.0;
    let plot_width = width - left - right;
    let plot_height = height - top - bottom;
    let max_count = bins
        .iter()
        .map(|bin| bin.sequence_count)
        .max()
        .unwrap_or(1)
        .max(1) as f64;
    let gap = 4.0;
    let bar_width =
        ((plot_width - gap * (bins.len().saturating_sub(1) as f64)) / bins.len() as f64).max(1.0);
    let bars = bins
        .iter()
        .enumerate()
        .map(|(index, bin)| {
            let bar_height = if bin.sequence_count == 0 {
                0.0
            } else {
                (bin.sequence_count as f64 / max_count) * plot_height
            };
            let x = left + index as f64 * (bar_width + gap);
            let y = top + plot_height - bar_height;
            format!(
                r#"<rect class="bar" x="{x:.2}" y="{y:.2}" width="{bar_width:.2}" height="{bar_height:.2}"><title>{min}-{max}: {count} sequences, {total} bases</title></rect>"#,
                min = bin.min_length,
                max = bin.max_length,
                count = bin.sequence_count,
                total = bin.total_length,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<svg viewBox="0 0 {width:.0} {height:.0}" role="img" aria-label="Length Histogram">
<line class="axis" x1="{left:.0}" y1="{top:.0}" x2="{left:.0}" y2="{axis_y:.0}"/>
<line class="axis" x1="{left:.0}" y1="{axis_y:.0}" x2="{axis_x:.0}" y2="{axis_y:.0}"/>
{bars}
<text class="axis-label" x="{left:.0}" y="{label_y:.0}">sequence length bins</text>
<text class="axis-label" x="4" y="{top:.0}">count</text>
</svg>"#,
        axis_y = top + plot_height,
        axis_x = left + plot_width,
        label_y = height - 10.0,
    )
}

fn render_gc_length_plot(report: &FastaguardReport) -> String {
    let points = &report.plots.gc_length_plot;
    if points.is_empty() {
        return r#"<p class="muted">No GC-vs-length data available.</p>"#.to_string();
    }

    let width = 720.0;
    let height = 260.0;
    let left = 56.0;
    let right = 16.0;
    let top = 18.0;
    let bottom = 42.0;
    let plot_width = width - left - right;
    let plot_height = height - top - bottom;
    let min_length = points.iter().map(|point| point.length).min().unwrap_or(0);
    let max_length = points.iter().map(|point| point.length).max().unwrap_or(0);
    let length_span = (max_length.saturating_sub(min_length)).max(1) as f64;
    let circles = points
        .iter()
        .map(|point| {
            let x = left + ((point.length.saturating_sub(min_length) as f64 / length_span) * plot_width);
            let y = top + plot_height - ((point.gc_percent / 100.0) * plot_height);
            let class = if point.flags.iter().any(|flag| flag == "gc_outlier") {
                "point gc-outlier"
            } else {
                "point"
            };
            format!(
                r#"<circle class="{class}" cx="{x:.2}" cy="{y:.2}" r="4"><title>{id}: length {length}, GC {gc:.2}%, N {n:.2}%{flags}</title></circle>"#,
                id = escape_html(&point.id),
                length = point.length,
                gc = point.gc_percent,
                n = point.n_percent,
                flags = render_plot_flags(&point.flags),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<svg viewBox="0 0 {width:.0} {height:.0}" role="img" aria-label="GC vs Length">
<line class="axis" x1="{left:.0}" y1="{top:.0}" x2="{left:.0}" y2="{axis_y:.0}"/>
<line class="axis" x1="{left:.0}" y1="{axis_y:.0}" x2="{axis_x:.0}" y2="{axis_y:.0}"/>
{circles}
<text class="axis-label" x="{left:.0}" y="{label_y:.0}">length</text>
<text class="axis-label" x="4" y="{top:.0}">GC%</text>
</svg>"#,
        axis_y = top + plot_height,
        axis_x = left + plot_width,
        label_y = height - 10.0,
    )
}

fn render_findings(report: &FastaguardReport) -> String {
    if report.findings.is_empty() {
        return "<p>No findings.</p>".to_string();
    }

    report
        .findings
        .iter()
        .map(|finding| {
            format!(
                r#"<section class="finding">
<h3>{id}</h3>
<p><span class="label">Severity:</span> {severity}</p>
<p><span class="label">Message:</span> {message}</p>
<p><span class="label">Why it matters:</span> {why_it_matters}</p>
<p><span class="label">Suggested next step:</span> {suggested_next_step}</p>
{evidence}
{actions}
</section>"#,
                id = escape_html(&finding.id),
                severity = escape_html(severity(finding.severity.clone())),
                message = escape_html(&finding.message),
                why_it_matters = escape_html(&finding.why_it_matters),
                suggested_next_step = escape_html(&finding.suggested_next_step),
                evidence = render_evidence(finding),
                actions = render_actions(finding),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_evidence(finding: &crate::models::Finding) -> String {
    if finding.evidence.total_records == 0 && finding.evidence.records.is_empty() {
        return r#"<h4>Finding Evidence</h4><p class="muted">No record-level evidence captured.</p>"#
            .to_string();
    }

    let rows = finding
        .evidence
        .records
        .iter()
        .map(|record| {
            format!(
                r#"<tr>
<td>{id}</td>
<td class="nowrap">{length}</td>
<td>{reason}</td>
<td class="nowrap">{invalid_count}</td>
<td class="nowrap">{n_percent}</td>
<td class="nowrap">{max_gap_run}</td>
<td class="nowrap">{gc_percent}</td>
</tr>"#,
                id = escape_html(&record.id),
                length = record.length,
                reason = escape_html(&record.reason),
                invalid_count = render_optional_u64(record.invalid_count),
                n_percent = render_optional_f64(record.n_percent),
                max_gap_run = render_optional_u64(record.max_gap_run),
                gc_percent = render_optional_f64(record.gc_percent),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let truncated = if finding.evidence.truncated {
        "yes"
    } else {
        "no"
    };

    format!(
        r#"<h4>Finding Evidence</h4>
<p class="muted">Showing {shown} of {total} affected records. Truncated: {truncated}.</p>
<div class="table-scroll">
<table>
<thead><tr><th>ID</th><th>Length</th><th>Reason</th><th>Invalid</th><th>N%</th><th>Max gap</th><th>GC%</th></tr></thead>
<tbody>{rows}</tbody>
</table>
</div>"#,
        shown = finding.evidence.records.len(),
        total = finding.evidence.total_records,
        truncated = truncated,
        rows = rows,
    )
}

fn render_actions(finding: &crate::models::Finding) -> String {
    if finding.actions.is_empty() {
        return r#"<h4>Suggested Actions</h4><p class="muted">No structured actions available.</p>"#
            .to_string();
    }

    let rows = finding
        .actions
        .iter()
        .map(|action| {
            let external_database = if action.requires_external_database {
                "yes"
            } else {
                "no"
            };
            format!(
                r#"<tr>
<td>{action_type}</td>
<td>{target}</td>
<td>{reason}</td>
<td>{recommended_tool}</td>
<td>{external_database}</td>
</tr>"#,
                action_type = escape_html(&action.action_type),
                target = escape_html(&action.target),
                reason = escape_html(&action.reason),
                recommended_tool = escape_html(&action.recommended_tool),
                external_database = external_database,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<h4>Suggested Actions</h4>
<div class="table-scroll">
<table>
<thead><tr><th>Action</th><th>Target</th><th>Reason</th><th>Tool</th><th>External DB</th></tr></thead>
<tbody>{rows}</tbody>
</table>
</div>"#
    )
}

fn render_string_list(values: &[String]) -> String {
    if values.is_empty() {
        return "<p>None.</p>".to_string();
    }

    let items = values
        .iter()
        .map(|value| format!("<li>{}</li>", escape_html(value)))
        .collect::<Vec<_>>()
        .join("\n");
    format!("<ul>{items}</ul>")
}

fn render_optional_u64(value: Option<u64>) -> String {
    value
        .map(|number| number.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn render_optional_f64(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.2}"))
        .unwrap_or_else(|| "-".to_string())
}

fn render_plot_flags(flags: &[String]) -> String {
    if flags.is_empty() {
        String::new()
    } else {
        format!("; flags {}", escape_html(&flags.join(", ")))
    }
}

fn verdict_status(status: VerdictStatus) -> &'static str {
    match status {
        VerdictStatus::Pass => "PASS",
        VerdictStatus::Warn => "WARN",
        VerdictStatus::Fail => "FAIL",
    }
}

fn severity(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Minor => "minor",
        Severity::Major => "major",
        Severity::Critical => "critical",
    }
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::NamedTempFile;

    use super::*;
    use crate::models::{
        empty_evidence, empty_plots, Artifacts, EvidenceRecord, FastaguardReport, Finding,
        FindingAction, FindingCategory, FindingConfidence, FindingEvidence, GateDecision,
        InputInfo, MachineSummary, Provenance, ProvenanceThresholds, RecommendedTool, Scope,
        Severity, Summary, ToolInfo, Verdict, VerdictStatus,
    };

    #[test]
    fn escapes_finding_text_and_embedded_json() {
        let mut report = test_report();
        report.findings.push(Finding {
            id: "bad_<id>".to_string(),
            category: FindingCategory::Validity,
            severity: Severity::Major,
            confidence: FindingConfidence::High,
            requires_followup_tool: false,
            profile: "assembly".to_string(),
            affected_count: 1,
            affected_fraction: 0.5,
            message: "contains <script>alert(\"x\")</script> & more".to_string(),
            why_it_matters: "breaks <downstream> reports".to_string(),
            suggested_next_step: "replace \"bad\" bases".to_string(),
            evidence: empty_evidence(),
            actions: Vec::new(),
        });
        let file = NamedTempFile::new().unwrap();

        write(&report, file.path()).unwrap();

        let output = fs::read_to_string(file.path()).unwrap();
        assert!(output.contains("&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt; &amp; more"));
        assert!(!output.contains("<script>alert"));
        assert!(output.contains("&quot;bad_&lt;id&gt;&quot;"));
    }

    #[test]
    fn renders_machine_summary_scope_actions_and_evidence() {
        let mut report = test_report();
        report.verdict.status = VerdictStatus::Fail;
        report.verdict.reasons = vec!["high_n_rate".to_string()];
        report.machine_summary = MachineSummary {
            verdict: VerdictStatus::Fail,
            safe_for_downstream: false,
            top_findings: vec!["high_n_rate".to_string()],
            recommended_next_tools: vec![RecommendedTool {
                tool: "QUAST".to_string(),
                reason: "inspect assembly-level effects".to_string(),
            }],
            routing_hints: Vec::new(),
        };
        report.scope.can_conclude = vec!["FASTA parse validity".to_string()];
        report.scope.cannot_conclude = vec!["biological completeness".to_string()];
        report.findings.push(Finding {
            id: "high_n_rate".to_string(),
            category: FindingCategory::Composition,
            severity: Severity::Major,
            confidence: FindingConfidence::High,
            requires_followup_tool: false,
            profile: "assembly".to_string(),
            affected_count: 1,
            affected_fraction: 0.5,
            message: "50% of sequences contain more than 20% Ns.".to_string(),
            why_it_matters: "High ambiguity can reduce annotation quality.".to_string(),
            suggested_next_step: "Inspect high-N scaffolds.".to_string(),
            evidence: FindingEvidence {
                total_records: 1,
                truncated: false,
                records: vec![EvidenceRecord {
                    id: "scaffold_1".to_string(),
                    length: 1200,
                    reason: "per-sequence N fraction exceeded threshold".to_string(),
                    invalid_count: None,
                    n_fraction: Some(0.42),
                    n_percent: Some(42.0),
                    max_gap_run: Some(240),
                    gc_percent: None,
                    gc_zscore: None,
                    signals: Vec::new(),
                }],
            },
            actions: vec![FindingAction {
                action_type: "inspect_records".to_string(),
                target: "high-N scaffolds".to_string(),
                reason: "High ambiguity may indicate unresolved assembly regions.".to_string(),
                recommended_tool: "seqkit".to_string(),
                requires_external_database: false,
            }],
        });
        let file = NamedTempFile::new().unwrap();

        write(&report, file.path()).unwrap();

        let output = fs::read_to_string(file.path()).unwrap();
        assert!(output.contains("Machine Summary"));
        assert!(output.contains("Safe for downstream"));
        assert!(output.contains("Recommended Next Tools"));
        assert!(output.contains("QUAST"));
        assert!(output.contains("Scope"));
        assert!(output.contains("biological completeness"));
        assert!(output.contains("Finding Evidence"));
        assert!(output.contains("scaffold_1"));
        assert!(output.contains("42"));
        assert!(output.contains("Suggested Actions"));
        assert!(output.contains("inspect_records"));
        assert!(output.contains("seqkit"));
    }

    fn test_report() -> FastaguardReport {
        FastaguardReport {
            schema_version: "0.1.0".to_string(),
            tool: ToolInfo {
                name: "FastaGuard".to_string(),
                version: "0.1.0".to_string(),
            },
            input: InputInfo {
                path: "input.fa".to_string(),
                profile: "assembly".to_string(),
                compressed: false,
            },
            verdict: Verdict {
                status: VerdictStatus::Pass,
                reasons: Vec::new(),
            },
            gate: GateDecision {
                mode: "none".to_string(),
                status: VerdictStatus::Pass,
                blocking_findings: Vec::new(),
                advisory_findings: Vec::new(),
                fail_on: Vec::new(),
            },
            machine_summary: MachineSummary {
                verdict: VerdictStatus::Pass,
                safe_for_downstream: true,
                top_findings: Vec::new(),
                recommended_next_tools: Vec::new(),
                routing_hints: Vec::new(),
            },
            scope: Scope {
                level: "fasta_preflight".to_string(),
                can_conclude: Vec::new(),
                cannot_conclude: Vec::new(),
            },
            provenance: Provenance {
                profile: "assembly".to_string(),
                threads: 1,
                fail_on: Vec::new(),
                thresholds: ProvenanceThresholds {
                    high_n_sequence_fraction: 0.2,
                    high_global_n_fraction: 0.05,
                    min_contig_length: 200,
                    max_gap_run: 100,
                    gc_outlier_zscore: 3.0,
                },
                command: "fastaguard input.fa".to_string(),
                started_at: "2026-05-23T00:00:00Z".to_string(),
                completed_at: "2026-05-23T00:00:00Z".to_string(),
                duration_ms: 0,
                input_size_bytes: 100,
                input_sha256: "0".repeat(64),
            },
            summary: Summary {
                sequence_count: 2,
                total_length: 100,
                min_length: 40,
                max_length: 60,
                mean_length: 50.0,
                median_length: 50.0,
                n50: 60,
                n90: 40,
                l50: 1,
                l90: 2,
                gc_percent: 48.5,
                at_percent: 50.0,
                n_percent: 1.5,
                ambiguity_percent: 1.5,
                duplicate_id_count: 0,
                duplicate_sequence_count: 0,
                invalid_sequence_count: 0,
                high_n_sequence_count: 0,
                tiny_contig_count: 0,
                max_gap_run: 1,
            },
            plots: empty_plots(),
            findings: Vec::new(),
            artifacts: Artifacts {
                html: "fastaguard_report.html".to_string(),
                tsv: "fastaguard.tsv".to_string(),
                multiqc: "fastaguard_mqc.json".to_string(),
            },
        }
    }
}
