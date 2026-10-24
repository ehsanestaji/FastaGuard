"""Model Context Protocol (MCP) stdio server for FastaGuard."""

from .server import (
    EXIT_CODE_MEANINGS,
    SERVER_NAME,
    SERVER_VERSION,
    TOOL_DEFINITIONS,
    FastaguardMcpServer,
    main,
    resolve_fastaguard_binary,
)

__all__ = [
    "EXIT_CODE_MEANINGS",
    "SERVER_NAME",
    "SERVER_VERSION",
    "TOOL_DEFINITIONS",
    "FastaguardMcpServer",
    "main",
    "resolve_fastaguard_binary",
]
