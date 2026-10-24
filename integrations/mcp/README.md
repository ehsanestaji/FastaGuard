# FastaGuard MCP Server

This directory contains `fastaguard-mcp`, a lightweight Model Context Protocol
(MCP) stdio server built on top of the FastaGuard CLI contract.

The package requires Python 3.10 or newer and uses only the Python standard
library (`json`, `subprocess`, `sys`, `tempfile`, `pathlib`), so it runs in clean
environments without third-party runtime dependencies or external databases.

## Exposed Tools

`fastaguard-mcp` exposes six deterministic tools backed directly by the
`fastaguard` binary:

- `fastaguard_schema`: returns the parsed JSON Schema from `fastaguard --schema`.
- `fastaguard_finding_catalog`: returns the parsed finding catalog from
  `fastaguard --finding-catalog`.
- `fastaguard_explain_finding`: takes `id` and runs
  `fastaguard --explain-finding <id>`.
- `fastaguard_inspect_fasta`: runs `fastaguard <input>` (with optional
  `profile`, `gate`, `submission_target`, `fail_on`, `outdir`, `prefix`, `force`,
  and threshold overrides) and returns the structured single-FASTA JSON report
  (`verdict`, `gate.can_continue`, `readiness`, `findings`, `machine_summary`,
  `scope`).
- `fastaguard_compare`: runs `fastaguard compare <inputs...>` and returns the
  structured cohort JSON report.
- `fastaguard_reference`: runs `fastaguard reference <reference>` with optional
  companion declarations (`fai`, `dict`, `alignment`, `variants`, `annotation`,
  `alias_map`, `require`, `policy`, `outdir`, `prefix`, `write_lock`, `force`)
  and returns the structured reference JSON report.

## Binary Resolution and Exit Contract

The server resolves the FastaGuard CLI executable in this order:

1. Explicit `--fastaguard-bin <path>` CLI argument.
2. `FASTAGUARD_BIN` (or `FASTAGUARD_BINARY`) environment variable.
3. `fastaguard` on `PATH`.
4. Local build paths (`./target/release/fastaguard` or `./target/debug/fastaguard`).

Each `tools/call` response surfaces the CLI exit code in `exit_code` and
`_meta.exit_code`:

- `0`: completed report generation (including `PASS`, `WARN`, and `FAIL` reports)
  or contract discovery output.
- `2`: argument parsing error (`isError: true`).
- `3`: configuration, input-access/I/O, runtime, or output-write error
  (`isError: true`).

## Development Install and Usage

From this directory:

```bash
python3 -m pip install -e .
fastaguard-mcp --version
```

You can also run the server directly from a source checkout without installing:

```bash
PYTHONPATH=integrations/mcp/src FASTAGUARD_BIN=./target/release/fastaguard \
  python3 -m fastaguard_mcp
```

Both newline-delimited JSON-RPC 2.0 messages and `Content-Length` framed stdio
messages are supported.

## MCP Client Configuration

Example stdio client configuration using an installed `fastaguard-mcp` entry
point:

```json
{
  "mcpServers": {
    "fastaguard": {
      "command": "fastaguard-mcp",
      "args": [],
      "env": {
        "FASTAGUARD_BIN": "/path/to/fastaguard"
      }
    }
  }
}
```

Example configuration running directly from the repository checkout:

```json
{
  "mcpServers": {
    "fastaguard": {
      "command": "python3",
      "args": ["-m", "fastaguard_mcp"],
      "env": {
        "PYTHONPATH": "/path/to/FastaGuard/integrations/mcp/src",
        "FASTAGUARD_BIN": "/path/to/FastaGuard/target/release/fastaguard"
      }
    }
  }
}
```

## Scope Boundaries

- JSON emitted by the `fastaguard` CLI remains the sole source of truth.
- `fastaguard-mcp` never fabricates findings outside the CLI JSON report.
- `fastaguard-mcp` never modifies or overwrites input FASTA or companion files.
- No external biological database or network service is queried.
