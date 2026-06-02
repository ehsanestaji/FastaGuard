use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

use crate::models::{CohortFinding, CompareReport, CompareSample, Severity, VerdictStatus};

const READINESS_MATRIX_CATEGORIES: [(&str, &str); 7] = [
    ("file", "File readiness"),
    ("structure", "Structure readiness"),
    ("alphabet", "Alphabet readiness"),
    ("index", "Index readiness"),
    ("assembly", "Assembly readiness"),
    ("submission", "Submission readiness"),
    ("machine", "Machine readiness"),
];

pub fn write(report: &CompareReport, path: &Path) -> Result<()> {
    let html = render(report)?;
    fs::write(path, html).with_context(|| format!("failed to write HTML report {}", path.display()))
}

fn render(report: &CompareReport) -> Result<String> {
    let readiness_matrix = render_readiness_matrix(report);
    let charts = render_charts(&report.samples);
    let cohort_findings = render_cohort_findings(&report.cohort_findings)?;
    let suggested_tools = render_suggested_tools(&report.samples);
    let json = serde_json::to_string_pretty(report).context("failed to serialize report JSON")?;

    Ok(format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>FastaGuard Compare Report</title>
<style>
:root {{ color-scheme: light; font-family: Arial, Helvetica, sans-serif; }}
body {{ margin: 0; background: #f7f7f4; color: #202124; }}
main {{ max-width: 1180px; margin: 0 auto; padding: 32px 20px 48px; }}
h1 {{ margin: 0 0 8px; font-size: 34px; }}
h2 {{ margin-top: 32px; border-bottom: 1px solid #d8d8d2; padding-bottom: 8px; }}
h3 {{ margin: 0 0 10px; font-size: 18px; }}
table {{ width: 100%; border-collapse: collapse; background: #ffffff; }}
th, td {{ border: 1px solid #d8d8d2; padding: 10px 12px; text-align: left; vertical-align: top; }}
th {{ background: #ecece6; }}
.positioning {{ color: #4f565c; margin: 0 0 24px; }}
.summary {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 12px; }}
.panel {{ background: #ffffff; border: 1px solid #d8d8d2; padding: 14px; }}
.metric {{ margin: 0; font-size: 24px; font-weight: 700; }}
.label {{ margin: 0 0 6px; color: #596066; }}
.table-scroll {{ overflow-x: auto; }}
.table-scroll table {{ min-width: 960px; }}
.status-pass {{ color: #1f7a3f; font-weight: 700; }}
.status-warn {{ color: #9a6a00; font-weight: 700; }}
.status-fail {{ color: #a32020; font-weight: 700; }}
.plot-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 18px; }}
.plot {{ background: #ffffff; border: 1px solid #d8d8d2; padding: 14px; }}
.plot svg {{ width: 100%; height: auto; display: block; }}
.bar {{ fill: #437c90; }}
.axis {{ stroke: #5f6368; stroke-width: 1; }}
.axis-label {{ fill: #4f565c; font-size: 12px; }}
.muted {{ color: #596066; }}
.finding {{ background: #ffffff; border: 1px solid #d8d8d2; padding: 16px; margin: 14px 0; }}
pre {{ overflow-x: auto; background: #202124; color: #f7f7f4; padding: 16px; }}
</style>
</head>
<body>
<main>
<h1>FastaGuard Compare Report</h1>
<p class="positioning">Before QUAST. Before BUSCO. Before BlobToolKit. Run FastaGuard first.</p>
<section class="summary">
<div class="panel"><p class="label">Samples</p><p class="metric">{sample_count}</p></div>
<div class="panel"><p class="label">PASS</p><p class="metric">{pass_count}</p></div>
<div class="panel"><p class="label">WARN</p><p class="metric">{warn_count}</p></div>
<div class="panel"><p class="label">FAIL</p><p class="metric">{fail_count}</p></div>
</section>
<h2>Readiness Matrix</h2>
{readiness_matrix}
<h2>Cohort Metrics</h2>
{charts}
<h2>Cohort Findings</h2>
{cohort_findings}
<h2>Suggested Next Tools</h2>
{suggested_tools}
<h2>JSON</h2>
<pre>{json}</pre>
</main>
</body>
</html>
"#,
        sample_count = report.summary.sample_count,
        pass_count = report.summary.pass_count,
        warn_count = report.summary.warn_count,
        fail_count = report.summary.fail_count,
        readiness_matrix = readiness_matrix,
        charts = charts,
        cohort_findings = cohort_findings,
        suggested_tools = suggested_tools,
        json = escape_html(&json),
    ))
}

fn render_readiness_matrix(report: &CompareReport) -> String {
    let rows = report
        .samples
        .iter()
        .map(|sample| {
            let verdict = verdict_status(sample.verdict);
            let gate_status = verdict_status(sample.gate_status);
            let readiness_status = readiness_status(sample.readiness_status);
            let category_cells = render_readiness_category_cells(sample);
            format!(
                r#"<tr>
<td>{sample_id}</td>
<td>{input_path}</td>
<td class="status-{verdict_class}">{verdict}</td>
<td class="status-{gate_class}">{gate_status}</td>
<td class="status-{readiness_class}">{readiness_status}</td>
{category_cells}
<td>{sequence_count}</td>
<td>{total_length}</td>
<td>{n50}</td>
<td>{n90}</td>
<td>{gc_percent:.2}</td>
<td>{n_percent:.2}</td>
<td>{finding_count}</td>
<td>{readiness_blockers}</td>
</tr>"#,
                sample_id = escape_html(&sample.sample_id),
                input_path = escape_html(&sample.input_path),
                verdict = verdict,
                verdict_class = verdict.to_ascii_lowercase(),
                gate_status = gate_status,
                gate_class = gate_status.to_ascii_lowercase(),
                readiness_status = readiness_status,
                readiness_class = readiness_status.to_ascii_lowercase(),
                category_cells = category_cells,
                sequence_count = sample.sequence_count,
                total_length = sample.total_length,
                n50 = sample.n50,
                n90 = sample.n90,
                gc_percent = sample.gc_percent,
                n_percent = sample.n_percent,
                finding_count = sample.finding_count,
                readiness_blockers = escape_html(&sample.readiness_blockers.join(",")),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let category_headers = READINESS_MATRIX_CATEGORIES
        .iter()
        .map(|(_, label)| format!("<th>{}</th>", escape_html(label)))
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<div class="table-scroll">
<table>
<thead><tr><th>Sample</th><th>Input</th><th>Verdict</th><th>Gate</th><th>Readiness</th>{category_headers}<th>Sequences</th><th>Total length</th><th>N50</th><th>N90</th><th>GC%</th><th>N%</th><th>Findings</th><th>Blockers</th></tr></thead>
<tbody>{rows}</tbody>
</table>
</div>"#
    )
}

fn render_readiness_category_cells(sample: &CompareSample) -> String {
    READINESS_MATRIX_CATEGORIES
        .iter()
        .map(|(id, _)| {
            if let Some(category) = sample
                .readiness_categories
                .iter()
                .find(|category| category.id == *id)
            {
                let status = readiness_status(category.status);
                format!(
                    r#"<td class="status-{status_class}">{status}</td>"#,
                    status_class = status.to_ascii_lowercase(),
                )
            } else {
                r#"<td class="muted">.</td>"#.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_charts(samples: &[CompareSample]) -> String {
    format!(
        r#"<div class="plot-grid">
<section class="plot">{total_length}</section>
<section class="plot">{n50}</section>
<section class="plot">{gc}</section>
<section class="plot">{n}</section>
<section class="plot">{sequence_count}</section>
</div>"#,
        total_length =
            render_bar_chart("Total length", samples, |sample| sample.total_length as f64),
        n50 = render_bar_chart("N50", samples, |sample| sample.n50 as f64),
        gc = render_bar_chart("GC%", samples, |sample| sample.gc_percent),
        n = render_bar_chart("N%", samples, |sample| sample.n_percent),
        sequence_count = render_bar_chart("Sequence count", samples, |sample| {
            sample.sequence_count as f64
        }),
    )
}

fn render_bar_chart(
    title: &str,
    samples: &[CompareSample],
    value: impl Fn(&CompareSample) -> f64,
) -> String {
    if samples.is_empty() {
        return format!(
            r#"<h3>{}</h3><p class="muted">No samples available.</p>"#,
            escape_html(title)
        );
    }

    let width = 720.0;
    let height = 260.0;
    let left = 48.0;
    let right = 16.0;
    let top = 18.0;
    let bottom = 54.0;
    let plot_width = width - left - right;
    let plot_height = height - top - bottom;
    let values = samples.iter().map(&value).collect::<Vec<_>>();
    let max_value = values
        .iter()
        .copied()
        .filter(|number| number.is_finite())
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let gap = 4.0;
    let bar_width = ((plot_width - gap * (samples.len().saturating_sub(1) as f64))
        / samples.len() as f64)
        .max(1.0);
    let bars = samples
        .iter()
        .zip(values.iter())
        .enumerate()
        .map(|(index, (sample, metric))| {
            let normalized = if metric.is_finite() {
                (*metric / max_value).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let bar_height = normalized * plot_height;
            let x = left + index as f64 * (bar_width + gap);
            let y = top + plot_height - bar_height;
            format!(
                r#"<rect class="bar" x="{x:.2}" y="{y:.2}" width="{bar_width:.2}" height="{bar_height:.2}"><title>{sample_id}: {value:.2}</title></rect>"#,
                sample_id = escape_html(&sample.sample_id),
                value = metric,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<h3>{title}</h3>
<svg viewBox="0 0 {width:.0} {height:.0}" role="img" aria-label="{aria_label}">
<line class="axis" x1="{left:.0}" y1="{top:.0}" x2="{left:.0}" y2="{axis_y:.0}"/>
<line class="axis" x1="{left:.0}" y1="{axis_y:.0}" x2="{axis_x:.0}" y2="{axis_y:.0}"/>
{bars}
<text class="axis-label" x="{left:.0}" y="{label_y:.0}">samples</text>
<text class="axis-label" x="4" y="{top:.0}">value</text>
</svg>"#,
        title = escape_html(title),
        aria_label = escape_html(title),
        axis_y = top + plot_height,
        axis_x = left + plot_width,
        label_y = height - 14.0,
    )
}

fn render_cohort_findings(findings: &[CohortFinding]) -> Result<String> {
    if findings.is_empty() {
        return Ok("<p>No cohort findings.</p>".to_string());
    }

    findings
        .iter()
        .map(render_cohort_finding)
        .collect::<Result<Vec<_>>>()
        .map(|sections| sections.join("\n"))
}

fn render_cohort_finding(finding: &CohortFinding) -> Result<String> {
    let evidence =
        serde_json::to_string_pretty(&finding.evidence).context("failed to serialize evidence")?;
    Ok(format!(
        r#"<section class="finding">
<h3>{id}</h3>
<p><span class="label">Severity:</span> {severity}</p>
<p><span class="label">Affected samples:</span> {affected_count}</p>
<pre>{evidence}</pre>
</section>"#,
        id = escape_html(&finding.id),
        severity = escape_html(severity(&finding.severity)),
        affected_count = finding.affected_count,
        evidence = escape_html(&evidence),
    ))
}

fn render_suggested_tools(samples: &[CompareSample]) -> String {
    let rows = samples
        .iter()
        .map(|sample| {
            let tools = if sample.recommended_next_tools.is_empty() {
                "None".to_string()
            } else {
                escape_html(&sample.recommended_next_tools.join(","))
            };
            format!(
                "<tr><td>{sample_id}</td><td>{tools}</td></tr>",
                sample_id = escape_html(&sample.sample_id),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<div class="table-scroll">
<table>
<thead><tr><th>Sample</th><th>Tools</th></tr></thead>
<tbody>{rows}</tbody>
</table>
</div>"#
    )
}

fn verdict_status(status: VerdictStatus) -> &'static str {
    match status {
        VerdictStatus::Pass => "PASS",
        VerdictStatus::Warn => "WARN",
        VerdictStatus::Fail => "FAIL",
    }
}

fn readiness_status(status: crate::readiness::ReadinessStatus) -> &'static str {
    match status {
        crate::readiness::ReadinessStatus::Pass => "PASS",
        crate::readiness::ReadinessStatus::Warn => "WARN",
        crate::readiness::ReadinessStatus::Fail => "FAIL",
    }
}

fn severity(severity: &Severity) -> &'static str {
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
        CohortFinding, CompareInputInfo, CompareSummary, ToolInfo, SCHEMA_VERSION,
    };

    #[test]
    fn writes_compare_html_sections_and_charts() {
        let file = NamedTempFile::new().unwrap();

        write(&test_report(), file.path()).unwrap();

        let output = fs::read_to_string(file.path()).unwrap();
        assert!(output.contains("FastaGuard Compare Report"), "{output}");
        assert!(output.contains("Readiness Matrix"), "{output}");
        assert!(output.contains("<th>File readiness</th>"), "{output}");
        assert!(output.contains("<th>Index readiness</th>"), "{output}");
        assert!(output.contains("<th>Machine readiness</th>"), "{output}");
        assert!(output.contains("Cohort Findings"), "{output}");
        assert!(output.contains("Suggested Next Tools"), "{output}");
        assert!(output.matches("<svg").count() >= 5, "{output}");
    }

    #[test]
    fn escapes_compare_html_content() {
        let mut report = test_report();
        report.samples[0].sample_id = "sample_<script>".to_string();
        report.samples[0].input_path = "bad<&>.fa".to_string();
        report.samples[0].recommended_next_tools = vec!["tool<bad>".to_string()];

        let output = render(&report).unwrap();

        assert!(output.contains("sample_&lt;script&gt;"), "{output}");
        assert!(output.contains("bad&lt;&amp;&gt;.fa"), "{output}");
        assert!(output.contains("tool&lt;bad&gt;"), "{output}");
        assert!(!output.contains("sample_<script>"), "{output}");
    }

    fn test_report() -> CompareReport {
        CompareReport {
            schema_version: SCHEMA_VERSION.to_string(),
            report_type: "compare".to_string(),
            tool: ToolInfo {
                name: "FastaGuard".to_string(),
                version: "0.4.0".to_string(),
            },
            input: CompareInputInfo {
                profile: "assembly".to_string(),
                sample_count: 1,
            },
            summary: CompareSummary {
                sample_count: 1,
                pass_count: 1,
                warn_count: 0,
                fail_count: 0,
            },
            samples: vec![CompareSample {
                sample_id: "sample_a".to_string(),
                input_path: "sample_a.fa".to_string(),
                verdict: VerdictStatus::Pass,
                gate_status: VerdictStatus::Pass,
                readiness_status: crate::readiness::ReadinessStatus::Pass,
                readiness_categories: crate::readiness::build_readiness(
                    VerdictStatus::Pass,
                    &[],
                    &[],
                    crate::readiness::ReadinessScope::Single,
                )
                .categories,
                sequence_count: 2,
                total_length: 100,
                n50: 60,
                n90: 40,
                gc_percent: 50.0,
                n_percent: 0.0,
                duplicate_id_count: 0,
                invalid_sequence_count: 0,
                high_n_sequence_count: 0,
                tiny_contig_count: 0,
                max_gap_run: 0,
                gc_outlier_count: 0,
                length_outlier_count: 0,
                finding_count: 1,
                finding_ids: vec!["duplicate_ids".to_string()],
                readiness_blockers: vec!["duplicate_ids".to_string()],
                recommended_next_tools: vec!["seqkit".to_string(), "QUAST".to_string()],
                input_sha256: "0".repeat(64),
            }],
            cohort_findings: vec![CohortFinding {
                id: "cohort_total_length_outliers".to_string(),
                severity: Severity::Minor,
                affected_count: 1,
                evidence: serde_json::json!({
                    "records": [{
                        "sample_id": "sample_a",
                        "total_length": 100,
                        "reason": "total length is unusual relative to the cohort",
                    }],
                }),
            }],
        }
    }
}
