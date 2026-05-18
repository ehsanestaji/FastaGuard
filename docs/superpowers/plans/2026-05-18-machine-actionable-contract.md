# Machine-Actionable Contract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make FastaGuard's output contract discoverable by tools and future LLM agents through repo memory, a JSON Schema, a finding catalog, and CLI contract-discovery commands.

**Architecture:** Keep the report schema and finding catalog as static repository artifacts under `schema/`, then expose them through lightweight CLI modes that do not require an input FASTA. This keeps the current analysis path unchanged while making the machine contract easy to inspect programmatically.

**Tech Stack:** Rust, clap, serde_json, static JSON assets loaded with `include_str!`, integration tests with assert_cmd.

---

### Task 1: Project Memory

**Files:**
- Create: `AGENTS.md`

- [x] **Step 1: Add durable Codex project memory**

Capture the product thesis, tool landscape, machine-actionable vision, and recommendation-first collaboration preference in `AGENTS.md`.

- [x] **Step 2: Keep scope separate from runtime behavior**

Do not change Rust behavior in this task.

### Task 2: Contract Discovery Tests

**Files:**
- Modify: `tests/cli.rs`

- [x] **Step 1: Add failing CLI tests**

Add tests for:

- `fastaguard --schema`
- `fastaguard --finding-catalog`
- `fastaguard --explain-finding high_n_rate`
- `fastaguard --explain-finding unknown_rule`

- [x] **Step 2: Run tests to verify RED**

Run:

```bash
cargo test --test cli contract_
```

Expected: tests fail because the CLI flags do not exist yet.

### Task 3: Static Contract Assets

**Files:**
- Create: `schema/fastaguard.schema.json`
- Create: `schema/finding-catalog.json`
- Create: `src/contract.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Add JSON Schema**

Create a draft 2020-12 JSON Schema that documents the current `FastaguardReport` shape.

- [x] **Step 2: Add finding catalog**

Create a machine-readable finding catalog for current finding IDs:

- `duplicate_ids`
- `invalid_chars`
- `high_n_rate`
- `tiny_contigs`
- `gap_runs`
- `duplicate_sequences`
- `invalid_fasta_structure`

- [x] **Step 3: Add Rust accessors**

Add `src/contract.rs` with functions that return the schema, full catalog, and one catalog entry by ID.

### Task 4: CLI Contract Commands

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/lib.rs`

- [x] **Step 1: Add discovery flags**

Add:

- `--schema`
- `--finding-catalog`
- `--explain-finding <id>`

- [x] **Step 2: Allow discovery flags without input**

Make the positional FASTA input optional at parse time, but keep it required for normal QC runs.

- [x] **Step 3: Print contract output to stdout**

Return exit code `0` for known contract requests and exit code `3` for unknown finding IDs.

### Task 5: Verification And Docs

**Files:**
- Modify: `README.md`
- Modify: `docs/output-contract.md`
- Modify: `docs/llm-tooling-vision.md`

- [x] **Step 1: Document discovery commands**

Add examples for `--schema`, `--finding-catalog`, and `--explain-finding`.

- [x] **Step 2: Run verification**

Run:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
```

- [x] **Step 3: Commit**

Commit with:

```bash
git commit -m "feat: expose machine-readable QC contract"
```
