"""Native MultiQC module starter for FastaGuard."""

from __future__ import annotations

from pathlib import Path

from multiqc.base_module import BaseMultiqcModule, ModuleNoSamplesFound
from multiqc.plots import table

from .parser import load_custom_content_summary


GENERAL_STATS_FIELDS = (
    "verdict",
    "gate_can_continue",
    "sequence_count",
    "total_length",
    "finding_count",
    "n50",
    "n_percent",
)

COMPACT_SUMMARY_FIELDS = (
    "verdict",
    "gate_can_continue",
    "gate_status",
    "readiness_status",
    "submission_target",
    "submission_policy_id",
    "submission_status",
    "sequence_count",
    "total_length",
    "n50",
    "gc_percent",
    "n_percent",
    "finding_count",
)


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
                self._summary_data(data_by_sample),
                headers=self._summary_headers(),
                pconfig={
                    "id": "fastaguard_summary",
                    "title": "FastaGuard FASTA preflight summary",
                },
            ),
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
    def _general_stats_data(data_by_sample: dict[str, dict]) -> dict[str, dict]:
        return {
            sample_name: {
                field: row.get(field)
                for field in GENERAL_STATS_FIELDS
                if row.get(field) is not None
            }
            for sample_name, row in data_by_sample.items()
        }

    @staticmethod
    def _general_stats_headers() -> dict:
        return {
            "verdict": {
                "title": "FG verdict",
                "description": "FastaGuard FASTA preflight verdict",
            },
            "gate_can_continue": {
                "title": "FG continue",
                "description": "Whether the selected FastaGuard gate permits continuation",
            },
            "sequence_count": {
                "title": "FG sequences",
                "description": "Number of FASTA records",
                "hidden": True,
                "min": 0,
                "scale": "Blues",
            },
            "total_length": {
                "title": "FG total length",
                "description": "Total sequence length",
                "hidden": True,
                "min": 0,
                "scale": "Blues",
            },
            "finding_count": {
                "title": "FG findings",
                "description": "Number of FastaGuard findings",
                "min": 0,
                "scale": "OrRd",
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
    def _summary_data(data_by_sample: dict[str, dict]) -> dict[str, dict]:
        return {
            sample_name: {
                field: row.get(field)
                for field in COMPACT_SUMMARY_FIELDS
                if row.get(field) is not None
            }
            for sample_name, row in data_by_sample.items()
        }

    @staticmethod
    def _summary_headers() -> dict:
        return {
            "verdict": {
                "title": "Verdict",
                "description": "FastaGuard FASTA preflight verdict",
            },
            "gate_can_continue": {
                "title": "Gate can continue",
                "description": "Whether the selected FastaGuard gate permits continuation",
            },
            "gate_status": {
                "title": "Gate status",
                "description": "Status of the selected FastaGuard gate",
            },
            "readiness_status": {
                "title": "Readiness",
                "description": "FastaGuard readiness status",
            },
            "submission_target": {
                "title": "Submission target",
                "description": "Submission target profile used by FastaGuard",
            },
            "submission_policy_id": {
                "title": "Submission policy",
                "description": "FastaGuard submission-policy snapshot identifier",
            },
            "submission_status": {
                "title": "Submission status",
                "description": "FASTA-level submission readiness status",
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
        }
