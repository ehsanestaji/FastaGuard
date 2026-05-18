"""MultiQC plugin starter for FastaGuard."""

from .parser import load_custom_content_summary

__all__ = ["MultiqcModule", "load_custom_content_summary"]


def __getattr__(name):
    if name == "MultiqcModule":
        from .multiqc_module import MultiqcModule

        return MultiqcModule
    raise AttributeError(name)
