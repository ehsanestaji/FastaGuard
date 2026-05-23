"""Native MultiQC module starter for FastaGuard."""

from __future__ import annotations

from pathlib import Path

from multiqc.base_module import BaseMultiqcModule, ModuleNoSamplesFound
from multiqc.plots import table

from .parser import load_custom_content_summary


class MultiqcModule(BaseMultiqcModule):
    """Summarize FastaGuard FASTA preflight reports in MultiQC."""

    def __init__(self):
        super().__init__(
            name="FastaGuard",
            anchor="fastaguard",
            href="https://github.com/ehsanestaji/FastaGuard",
            info="FASTA preflight QC before downstream assembly analysis.",
        )

        data_by_sample = self._load_reports()
        if not data_by_sample:
            raise ModuleNoSamplesFound

        self.general_stats_addcols(
            self._general_stats_data(data_by_sample),
            self._general_stats_headers(),
        )
        self.add_section(
            name="FastaGuard summary",
            anchor="fastaguard-summary",
            description="FASTA preflight verdicts and core assembly metrics.",
            plot=table.plot(
                data_by_sample,
                headers=self._summary_headers(),
                pconfig={
                    "id": "fastaguard_summary",
                    "title": "FastaGuard FASTA preflight summary",
                },
            ),
            statuses=self._statuses(data_by_sample),
        )
        self.write_data_file(data_by_sample, "multiqc_fastaguard")

    def _load_reports(self) -> dict[str, dict]:
        data_by_sample: dict[str, dict] = {}
        for file_match in self.find_log_files("fastaguard", filecontents=False):
            path = Path(file_match["root"]) / file_match["fn"]
            file_data = load_custom_content_summary(path)
            data_by_sample.update(file_data)
            for sample_name in file_data:
                self.add_data_source(file_match, sample_name)
        return data_by_sample

    @staticmethod
    def _statuses(data_by_sample: dict[str, dict]) -> dict[str, list[str]]:
        statuses = {"pass": [], "warn": [], "fail": []}
        for sample_name, row in data_by_sample.items():
            verdict = str(row.get("verdict", "")).lower()
            if verdict in statuses:
                statuses[verdict].append(sample_name)
        return statuses

    @staticmethod
    def _general_stats_data(data_by_sample: dict[str, dict]) -> dict[str, dict]:
        visible_fields = (
            "finding_count",
            "gc_outlier_count",
            "length_outlier_count",
            "composite_anomaly_count",
            "n50",
            "n_percent",
        )
        return {
            sample_name: {
                field: row.get(field)
                for field in visible_fields
                if row.get(field) is not None
            }
            for sample_name, row in data_by_sample.items()
        }

    @staticmethod
    def _general_stats_headers() -> dict:
        return {
            "finding_count": {
                "title": "FG findings",
                "description": "Number of FastaGuard findings",
                "min": 0,
                "scale": "OrRd",
            },
            "gc_outlier_count": {
                "title": "FG GC outliers",
                "description": "Number of records flagged as GC composition outliers",
                "min": 0,
                "scale": "OrRd",
            },
            "length_outlier_count": {
                "title": "FG length outliers",
                "description": "Number of records flagged as length distribution outliers",
                "min": 0,
                "scale": "YlOrBr",
            },
            "composite_anomaly_count": {
                "title": "FG composite anomalies",
                "description": "Number of records flagged by composite anomaly checks",
                "min": 0,
                "scale": "Reds",
            },
            "n50": {
                "title": "FG N50",
                "description": "FastaGuard assembly N50",
                "hidden": True,
                "min": 0,
                "scale": "Blues",
            },
            "n_percent": {
                "title": "FG N%",
                "description": "FastaGuard global N percentage",
                "hidden": True,
                "min": 0,
                "max": 100,
                "suffix": "%",
                "scale": "OrRd",
            },
        }

    @staticmethod
    def _summary_headers() -> dict:
        return {
            "verdict": {
                "title": "Verdict",
                "description": "FastaGuard FASTA preflight verdict",
            },
            "sequence_count": {
                "title": "Sequences",
                "description": "Number of FASTA records",
                "min": 0,
                "scale": "Blues",
            },
            "total_length": {
                "title": "Total length",
                "description": "Total sequence length",
                "min": 0,
                "suffix": " bp",
                "scale": "Blues",
            },
            "n50": {
                "title": "N50",
                "description": "Assembly N50",
                "min": 0,
                "suffix": " bp",
                "scale": "Blues",
            },
            "n90": {
                "title": "N90",
                "description": "Assembly N90",
                "min": 0,
                "suffix": " bp",
                "scale": "Blues",
            },
            "gc_percent": {
                "title": "GC",
                "description": "Global GC percentage",
                "min": 0,
                "max": 100,
                "suffix": "%",
                "scale": "RdYlBu",
            },
            "n_percent": {
                "title": "N",
                "description": "Global N percentage",
                "min": 0,
                "max": 100,
                "suffix": "%",
                "scale": "OrRd",
            },
            "finding_count": {
                "title": "Findings",
                "description": "Number of FastaGuard findings",
                "min": 0,
                "scale": "OrRd",
            },
            "duplicate_id_count": {
                "title": "Duplicate IDs",
                "description": "Number of duplicate FASTA record IDs",
                "min": 0,
                "scale": "OrRd",
            },
            "invalid_sequence_count": {
                "title": "Invalid sequences",
                "description": "Number of records with invalid sequence characters",
                "min": 0,
                "scale": "Reds",
            },
            "high_n_sequence_count": {
                "title": "High-N sequences",
                "description": "Number of records exceeding the high-N threshold",
                "min": 0,
                "scale": "OrRd",
            },
            "tiny_contig_count": {
                "title": "Tiny contigs",
                "description": "Number of records below the tiny-contig threshold",
                "min": 0,
                "scale": "YlOrBr",
            },
            "max_gap_run": {
                "title": "Max gap run",
                "description": "Longest consecutive N run",
                "min": 0,
                "suffix": " bp",
                "scale": "OrRd",
            },
            "gc_outlier_count": {
                "title": "GC outliers",
                "description": "Number of records flagged as GC composition outliers",
                "min": 0,
                "scale": "OrRd",
            },
            "length_outlier_count": {
                "title": "Length outliers",
                "description": "Number of records flagged as length distribution outliers",
                "min": 0,
                "scale": "YlOrBr",
            },
            "composite_anomaly_count": {
                "title": "Composite anomalies",
                "description": "Number of records flagged by composite anomaly checks",
                "min": 0,
                "scale": "Reds",
            },
        }
