# FastaGuard Release Readiness Plan

> Date: 2026-05-18
> Scope: Prepare the assembly MVP for dependable CI, packaging, examples, report review, benchmarks, and schema validation.

## Recommendation

Ship the next project increment as release infrastructure, not new biological features. The core assembly preflight is already useful; the highest leverage now is making it easy to trust, install, inspect, benchmark, and integrate.

## Product Goal

FastaGuard should become the boringly reliable FASTA preflight layer:

```text
Before QUAST. Before BUSCO. Before BlobToolKit. Before annotation.
Run FastaGuard first.
```

This release-readiness pass makes that promise more credible by adding automation and artifacts around the existing Rust CLI.

## Work Items

1. Add CI
   - Create `.github/workflows/ci.yml`.
   - Run `cargo fmt --check`.
   - Run `cargo test`.
   - Run `cargo clippy --all-targets --all-features -- -D warnings`.
   - Run `git diff --check`.

2. Add JSON Schema Validation
   - Add a Rust integration test that compiles `schema/fastaguard.schema.json`.
   - Validate all committed golden JSON reports in `tests/golden/`.
   - Keep schema validation inside `cargo test` so local and CI checks match.

3. Add Packaging Assets
   - Add a production-minded `Dockerfile`.
   - Add `.dockerignore`.
   - Document local binary, Docker, GitHub release, and Bioconda packaging paths.
   - Keep Bioconda as a day-one strategy for tagged releases rather than blocking local development.

4. Add Example Outputs
   - Generate stable example output artifacts from existing tiny test FASTA files.
   - Include JSON, TSV, HTML, and MultiQC JSON examples.
   - Link examples from the README so users and pipeline authors can inspect the output contract quickly.

5. Upgrade HTML Report
   - Surface verdict, scope, machine summary, recommended next tools, finding actions, and per-record evidence.
   - Keep the HTML static and self-contained.
   - Preserve the existing JSON embed for complete auditability.

6. Add Large-FASTA Benchmark Tooling
   - Add a deterministic synthetic FASTA benchmark script.
   - Measure runtime and output artifact size without requiring external databases.
   - Document smoke and larger benchmark commands.

## Verification

Run these before declaring the bundle complete:

```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
python3 scripts/benchmark_large_fasta.py --records 10 --length 100 --binary target/debug/fastaguard --out-dir target/bench-smoke
```

## Done Means

- CI exists and mirrors the local quality gate.
- Golden JSON reports are validated against the JSON Schema.
- Users can build or run FastaGuard in Docker.
- Example outputs are committed and easy to find.
- HTML reports explain machine-readable findings without requiring JSON inspection.
- Benchmark tooling gives the project a repeatable way to prove the "fast preflight" promise.
