"""Native MultiQC module starter for FastaGuard."""

from __future__ import annotations

from pathlib import Path

from multiqc.base_module import BaseMultiqcModule, ModuleNoSamplesFound
from multiqc.plots import table

from .parser import find_custom_content_files, load_custom_content_summary


class MultiqcModule(BaseMultiqcModule):
    """Summarize FastaGuard FASTA preflight reports in MultiQC."""

    def __init__(self):
        super().__init__(
            name="FastaGuard",
            anchor="fastaguard",
            href="https://github.com/ehsanestaji/FastaGuard",
            info="FASTA preflight QC before downstream assembly analysis.",
        )

        data_by_sample = self._load_reports(Path.cwd())
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
                pconfig={
                    "id": "fastaguard_summary",
                    "title": "FastaGuard FASTA preflight summary",
                },
            ),
            statuses=self._statuses(data_by_sample),
        )
        self.write_data_file(data_by_sample, "multiqc_fastaguard")

    @staticmethod
    def _load_reports(root: Path) -> dict[str, dict]:
        data_by_sample: dict[str, dict] = {}
        for path in find_custom_content_files(root):
            try:
                data_by_sample.update(load_custom_content_summary(path))
            except ValueError:
                continue
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
