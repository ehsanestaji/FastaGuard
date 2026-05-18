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
table {{ width: 100%; border-collapse: collapse; background: #ffffff; }}
th, td {{ border: 1px solid #d8d8d2; padding: 10px 12px; text-align: left; }}
th {{ background: #ecece6; }}
.verdict {{ display: inline-block; margin: 12px 0; padding: 6px 10px; font-weight: 700; border: 1px solid #202124; }}
.positioning {{ color: #4f565c; margin: 0 0 24px; }}
.finding {{ background: #ffffff; border: 1px solid #d8d8d2; padding: 16px; margin: 14px 0; }}
.finding h3 {{ margin: 0 0 10px; font-size: 18px; }}
.label {{ font-weight: 700; }}
pre {{ overflow-x: auto; background: #202124; color: #f7f7f4; padding: 16px; }}
</style>
</head>
<body>
<main>
<h1>FastaGuard Report</h1>
<div class="verdict">Verdict: {verdict}</div>
<p class="positioning">Before QUAST. Before BUSCO. Before BlobToolKit. Run FastaGuard first.</p>
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
<h2>Findings</h2>
{findings}
<h2>JSON</h2>
<pre>{json}</pre>
</main>
</body>
</html>
"#,
        verdict = escape_html(verdict_status(report.verdict.status)),
        sequence_count = summary.sequence_count,
        total_length = summary.total_length,
        n50 = summary.n50,
        n90 = summary.n90,
        gc_percent = summary.gc_percent,
        n_percent = summary.n_percent,
        findings = findings,
        json = escape_html(&json),
    ))
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
</section>"#,
                id = escape_html(&finding.id),
                severity = escape_html(severity(finding.severity.clone())),
                message = escape_html(&finding.message),
                why_it_matters = escape_html(&finding.why_it_matters),
                suggested_next_step = escape_html(&finding.suggested_next_step),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
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
        empty_evidence, Artifacts, FastaguardReport, Finding, InputInfo, MachineSummary,
        Provenance, ProvenanceThresholds, Scope, Severity, Summary, ToolInfo, Verdict,
        VerdictStatus,
    };

    #[test]
    fn escapes_finding_text_and_embedded_json() {
        let mut report = test_report();
        report.findings.push(Finding {
            id: "bad_<id>".to_string(),
            severity: Severity::Major,
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
            machine_summary: MachineSummary {
                verdict: VerdictStatus::Pass,
                safe_for_downstream: true,
                top_findings: Vec::new(),
                recommended_next_tools: Vec::new(),
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
            findings: Vec::new(),
            artifacts: Artifacts {
                html: "fastaguard_report.html".to_string(),
                tsv: "fastaguard.tsv".to_string(),
                multiqc: "fastaguard_multiqc.json".to_string(),
            },
        }
    }
}
