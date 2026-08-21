# Contributing to FastaGuard

Thanks for improving FastaGuard. The project focuses on fast, explainable,
machine-readable FASTA preflight QC. Please keep contributions within that
scope: validate and describe FASTA-level signals without claiming biological
completeness, contamination confirmation, or repository acceptance.

## Before opening a change

- Search existing issues and pull requests to avoid duplicate work.
- Keep changes focused and include tests for changed behavior.
- Preserve stable JSON and finding contracts unless a deliberate, documented
  contract change is needed.
- Do not include private sequence data in issues, tests, or examples. Use a
  minimal synthetic FASTA reproducer or a safe description instead.

## Local verification

Run the Rust checks from the repository root:

```bash
cargo fmt --check
cargo test --locked
```

Run the Python contract and documentation checks:

```bash
python3 -m pytest -q
```

Add or update focused tests when changing a behavior, report shape, or public
workflow example. Mention any commands you could not run in the pull request.

## Pull requests

Describe the user-facing change, tests run, contract impact, and documentation
impact. Update the relevant documentation and fixtures when a public interface
changes. The pull request template lists the review checks used by this
repository.

## Developer Certificate of Origin

All commits must include a Developer Certificate of Origin (DCO) sign-off.
Use Git's `-s` option when committing:

```bash
git commit -s -m "type: concise summary"
```

This adds a `Signed-off-by:` line using your configured Git identity. By
submitting the sign-off, you certify the contribution under the terms of the
[Developer Certificate of Origin](https://developercertificate.org/).

## Code of conduct and security

Please follow the [Code of Conduct](CODE_OF_CONDUCT.md). For a security issue,
follow the private reporting instructions in [SECURITY.md](SECURITY.md) rather
than opening a public issue.
