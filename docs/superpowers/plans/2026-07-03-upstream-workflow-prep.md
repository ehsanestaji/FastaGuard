# Upstream Workflow Prep Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prepare FastaGuard's local nf-core and Snakemake workflow starters in the safest order before any external upstream submission.

**Architecture:** Keep FastaGuard's stable CLI and JSON contract unchanged. Add upstream-shaped local metadata, fixture layouts, test harness files, and evidence-preserving gate examples around the existing v0.5.0 Bioconda/BioContainers package. Treat external nf-core and Snakemake PRs as a final gate after local assets and repository tests are green.

**Tech Stack:** Rust CLI, Python unittest, Nextflow/nf-test starter assets, Snakemake wrapper starter assets, Bioconda/BioContainers v0.5.0 package metadata.

---

## Safe Order

1. Create a plan and guard tests in the FastaGuard repository.
2. Harden the nf-core starter because it has the highest workflow adoption value and strictest module shape.
3. Add an evidence-preserving collect-then-gate helper for workflow engines.
4. Harden the Snakemake wrapper starter with official-wrapper-style metadata and tests.
5. Update readiness docs with what is ready locally and what remains external.
6. Run local verification and publish a normal project PR.
7. Only after that, decide whether to open external upstream PRs.

## File Map

- Modify: `tests/python/test_adoption_assets.py`
  - Add repository-level assertions for nf-core starter tests, Snakemake starter tests, collect-then-gate behavior, and safe-order documentation.
- Modify: `examples/nf-core/modules/local/fastaguard/main.nf`
  - Add `topic: versions` to the versions output.
- Modify: `examples/nf-core/modules/local/fastaguard/meta.yml`
  - Document versions as a topic output and keep HTML/JSON/TSV/MultiQC outputs clear.
- Create: `examples/nf-core/modules/local/fastaguard/tests/main.nf.test`
  - Provide nf-test starter cases for pass, warn, fail, and invalid FASTA fixtures.
- Create: `examples/nf-core/modules/local/fastaguard/tests/data/pass.fa`
- Create: `examples/nf-core/modules/local/fastaguard/tests/data/warn.fa`
- Create: `examples/nf-core/modules/local/fastaguard/tests/data/fail.fa`
- Create: `examples/nf-core/modules/local/fastaguard/tests/data/invalid.fa`
- Create: `examples/workflows/check_fastaguard_gate.py`
  - Read FastaGuard JSON and return 0 for PASS, 1 for WARN, 2 for FAIL, 3 for malformed tool input.
- Create: `tests/python/test_workflow_gate_helper.py`
  - Verify the gate helper exits correctly from small JSON fixtures.
- Create: `examples/snakemake/wrapper/environment.linux-64.pin.yaml`
  - Local starter pin file for upstream wrapper preparation.
- Create: `examples/snakemake/wrapper/test/Snakefile`
  - Official-wrapper-style local test Snakefile.
- Create: `examples/snakemake/wrapper/test/test_wrappers.py`
  - Copy-pasteable upstream test wiring snippet.
- Create: `examples/snakemake/wrapper/test/data/pass.fa`
- Create: `examples/snakemake/wrapper/test/data/warn.fa`
- Create: `examples/snakemake/wrapper/test/data/fail.fa`
- Create: `examples/snakemake/wrapper/test/data/invalid.fa`
- Modify: `examples/nf-core/README.md`
  - Add safe execution order and local/external validation boundary.
- Modify: `examples/snakemake/wrapper/README.md`
  - Add safe execution order and local/external validation boundary.
- Modify: `docs/workflow-readiness.md`
  - Mark local starter hardening items and leave upstream PRs as a final external step.
- Modify: `docs/adoption-plan.md`
  - Keep the safe order aligned with the readiness document.

### Task 1: Add Failing Adoption Tests For Safe-Order Assets

**Files:**
- Modify: `tests/python/test_adoption_assets.py`

- [ ] **Step 1: Write the failing tests**

Append these methods inside `AdoptionAssetsTest` before `test_benchmarking_docs_include_v0_2_evidence_topics`:

```python
    def test_nf_core_starter_has_upstream_prep_test_layout(self):
        module = ROOT / "examples" / "nf-core" / "modules" / "local" / "fastaguard"
        main_nf = (module / "main.nf").read_text()
        meta_yml = (module / "meta.yml").read_text()
        nf_test = (module / "tests" / "main.nf.test").read_text()

        self.assertIn('path "versions.yml", emit: versions, topic: versions', main_nf)
        self.assertIn("versions:", meta_yml)
        self.assertIn("topic", meta_yml)
        self.assertIn("process \"FASTAGUARD\"", nf_test)
        self.assertIn("pass.fa", nf_test)
        self.assertIn("warn.fa", nf_test)
        self.assertIn("fail.fa", nf_test)
        self.assertIn("invalid.fa", nf_test)
        self.assertIn("fastaguard_mqc.json", nf_test)
        for name in ("pass.fa", "warn.fa", "fail.fa", "invalid.fa"):
            self.assertTrue((module / "tests" / "data" / name).exists(), name)

    def test_snakemake_wrapper_has_upstream_prep_test_layout(self):
        wrapper = ROOT / "examples" / "snakemake" / "wrapper"
        readme = (wrapper / "README.md").read_text()
        test_snakefile = (wrapper / "test" / "Snakefile").read_text()
        test_py = (wrapper / "test" / "test_wrappers.py").read_text()
        pin = (wrapper / "environment.linux-64.pin.yaml").read_text()

        self.assertIn("safe local order", readme)
        self.assertIn("fastaguard=0.5.0", pin)
        self.assertIn("rule fastaguard_pass", test_snakefile)
        self.assertIn("rule fastaguard_warn", test_snakefile)
        self.assertIn("rule fastaguard_fail", test_snakefile)
        self.assertIn("rule fastaguard_invalid", test_snakefile)
        self.assertIn("pytest", test_py)
        self.assertIn("snakemake", test_py)
        for name in ("pass.fa", "warn.fa", "fail.fa", "invalid.fa"):
            self.assertTrue((wrapper / "test" / "data" / name).exists(), name)

    def test_workflow_readiness_safe_order_is_explicit(self):
        readiness = (ROOT / "docs" / "workflow-readiness.md").read_text()
        adoption = (ROOT / "docs" / "adoption-plan.md").read_text()

        self.assertIn("Safe Order", readiness)
        self.assertIn("local repository tests first", readiness)
        self.assertIn("external upstream PRs last", readiness)
        self.assertIn("check_fastaguard_gate.py", readiness)
        self.assertIn("nf-core module PR ready", adoption)
        self.assertIn("Snakemake wrapper PR ready", adoption)
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets.AdoptionAssetsTest.test_nf_core_starter_has_upstream_prep_test_layout tests.python.test_adoption_assets.AdoptionAssetsTest.test_snakemake_wrapper_has_upstream_prep_test_layout tests.python.test_adoption_assets.AdoptionAssetsTest.test_workflow_readiness_safe_order_is_explicit
```

Expected: FAIL because the new test layouts and safe-order text are missing.

### Task 2: Harden nf-core Starter Layout

**Files:**
- Modify: `examples/nf-core/modules/local/fastaguard/main.nf`
- Modify: `examples/nf-core/modules/local/fastaguard/meta.yml`
- Create: `examples/nf-core/modules/local/fastaguard/tests/main.nf.test`
- Create data fixtures under `examples/nf-core/modules/local/fastaguard/tests/data/`

- [ ] **Step 1: Add topic metadata to the versions output**

Change this output line:

```nextflow
path "versions.yml", emit: versions
```

to:

```nextflow
path "versions.yml", emit: versions, topic: versions
```

- [ ] **Step 2: Add versions topic metadata to `meta.yml`**

Add this block below the existing `output:` entries and before `authors:`:

```yaml
topics:
  versions:
    - - process:
          type: string
          description: The process the versions were collected from.
      - tool:
          type: string
          description: The tool name.
      - version:
          type: string
          description: The version reported by the tool.
```

- [ ] **Step 3: Add nf-test starter file**

Create `examples/nf-core/modules/local/fastaguard/tests/main.nf.test`:

```groovy
nextflow_process {

    name "Test FASTAGUARD"
    script "../main.nf"
    process "FASTAGUARD"

    tag "fastaguard"
    tag "fastaguard_single"

    test("pass FASTA emits all reports") {
        input[0] = [
            [id: "pass"],
            file("data/pass.fa")
        ]

        when {
            process {
                """
                input[0] = Channel.of(input[0])
                """
            }
        }

        then {
            assert process.success
            assert process.out.html[0][1].name.endsWith(".fastaguard.html")
            assert process.out.json[0][1].name.endsWith(".fastaguard.json")
            assert process.out.tsv[0][1].name.endsWith(".fastaguard.tsv")
            assert process.out.mqc[0][1].name.endsWith(".fastaguard_mqc.json")
            assert process.out.versions
        }
    }

    test("warn FASTA preserves reports") {
        input[0] = [
            [id: "warn"],
            file("data/warn.fa")
        ]

        when {
            process {
                """
                input[0] = Channel.of(input[0])
                """
            }
        }

        then {
            assert process.out.json
            assert process.out.mqc
        }
    }

    test("fail FASTA preserves reports for gate review") {
        input[0] = [
            [id: "fail"],
            file("data/fail.fa")
        ]

        when {
            process {
                """
                input[0] = Channel.of(input[0])
                """
            }
        }

        then {
            assert process.out.json
            assert process.out.mqc
        }
    }

    test("invalid FASTA is represented in the evidence path") {
        input[0] = [
            [id: "invalid"],
            file("data/invalid.fa")
        ]

        when {
            process {
                """
                input[0] = Channel.of(input[0])
                """
            }
        }

        then {
            assert process.out.json
            assert process.out.mqc
        }
    }
}
```

- [ ] **Step 4: Add nf-core test fixtures**

Create these files:

`examples/nf-core/modules/local/fastaguard/tests/data/pass.fa`

```fasta
>contig1
ACGTACGTACGTACGT
>contig2
GCGCGCATATAT
```

`examples/nf-core/modules/local/fastaguard/tests/data/warn.fa`

```fasta
>tiny1
ACGT
>tiny2
NNNNNNNN
```

`examples/nf-core/modules/local/fastaguard/tests/data/fail.fa`

```fasta
>dup
ACGTACGT
>dup
ACGTACGT
>bad
ACGTXYZ
```

`examples/nf-core/modules/local/fastaguard/tests/data/invalid.fa`

```fasta
>empty_record
>next_record
ACGT
```

- [ ] **Step 5: Run targeted adoption tests**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets.AdoptionAssetsTest.test_nf_core_starter_has_upstream_prep_test_layout
```

Expected: PASS.

### Task 3: Add Evidence-Preserving Gate Helper

**Files:**
- Create: `examples/workflows/check_fastaguard_gate.py`
- Create: `tests/python/test_workflow_gate_helper.py`

- [ ] **Step 1: Write failing tests**

Create `tests/python/test_workflow_gate_helper.py`:

```python
import json
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory


ROOT = Path(__file__).resolve().parents[2]
HELPER = ROOT / "examples" / "workflows" / "check_fastaguard_gate.py"


class WorkflowGateHelperTest(unittest.TestCase):
    def write_report(self, directory, status):
        path = Path(directory) / f"{status.lower()}.json"
        path.write_text(json.dumps({"gate": {"status": status}}))
        return path

    def run_helper(self, path):
        return subprocess.run(
            [sys.executable, str(HELPER), str(path)],
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def test_pass_exits_zero(self):
        with TemporaryDirectory() as temp_dir:
            result = self.run_helper(self.write_report(temp_dir, "PASS"))
        self.assertEqual(result.returncode, 0)
        self.assertIn("PASS", result.stdout)

    def test_warn_exits_one(self):
        with TemporaryDirectory() as temp_dir:
            result = self.run_helper(self.write_report(temp_dir, "WARN"))
        self.assertEqual(result.returncode, 1)
        self.assertIn("WARN", result.stdout)

    def test_fail_exits_two(self):
        with TemporaryDirectory() as temp_dir:
            result = self.run_helper(self.write_report(temp_dir, "FAIL"))
        self.assertEqual(result.returncode, 2)
        self.assertIn("FAIL", result.stdout)

    def test_malformed_report_exits_three(self):
        with TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "bad.json"
            path.write_text("{}")
            result = self.run_helper(path)
        self.assertEqual(result.returncode, 3)
        self.assertIn("gate.status", result.stderr)


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
python3 -m unittest tests.python.test_workflow_gate_helper
```

Expected: FAIL because the helper does not exist.

- [ ] **Step 3: Add helper implementation**

Create `examples/workflows/check_fastaguard_gate.py`:

```python
#!/usr/bin/env python3
import json
import sys
from pathlib import Path


EXIT_BY_STATUS = {
    "PASS": 0,
    "WARN": 1,
    "FAIL": 2,
}


def main(argv):
    if len(argv) != 2:
        print("usage: check_fastaguard_gate.py fastaguard.json", file=sys.stderr)
        return 3

    report_path = Path(argv[1])
    try:
        report = json.loads(report_path.read_text())
    except OSError as exc:
        print(f"could not read {report_path}: {exc}", file=sys.stderr)
        return 3
    except json.JSONDecodeError as exc:
        print(f"could not parse {report_path}: {exc}", file=sys.stderr)
        return 3

    status = report.get("gate", {}).get("status")
    if status not in EXIT_BY_STATUS:
        print("missing or unsupported gate.status", file=sys.stderr)
        return 3

    print(f"FastaGuard gate status: {status}")
    return EXIT_BY_STATUS[status]


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
```

- [ ] **Step 4: Run helper tests**

Run:

```bash
python3 -m unittest tests.python.test_workflow_gate_helper
```

Expected: PASS.

### Task 4: Harden Snakemake Wrapper Starter

**Files:**
- Create: `examples/snakemake/wrapper/environment.linux-64.pin.yaml`
- Create: `examples/snakemake/wrapper/test/Snakefile`
- Create: `examples/snakemake/wrapper/test/test_wrappers.py`
- Create data fixtures under `examples/snakemake/wrapper/test/data/`
- Modify: `examples/snakemake/wrapper/README.md`

- [ ] **Step 1: Add starter pin file**

Create `examples/snakemake/wrapper/environment.linux-64.pin.yaml`:

```yaml
channels:
  - conda-forge
  - bioconda
dependencies:
  - fastaguard=0.5.0
```

- [ ] **Step 2: Add official-wrapper-style test Snakefile**

Create `examples/snakemake/wrapper/test/Snakefile`:

```python
rule fastaguard_pass:
    input:
        fasta="data/pass.fa"
    output:
        html="pass/fastaguard_report.html",
        json="pass/fastaguard.json",
        tsv="pass/fastaguard.tsv",
        multiqc="pass/fastaguard_mqc.json"
    params:
        profile="assembly",
        gate="pipeline",
        extra=""
    wrapper:
        "file:../wrapper/fastaguard"


rule fastaguard_warn:
    input:
        fasta="data/warn.fa"
    output:
        html="warn/fastaguard_report.html",
        json="warn/fastaguard.json",
        tsv="warn/fastaguard.tsv",
        multiqc="warn/fastaguard_mqc.json"
    params:
        profile="assembly",
        gate="none",
        extra=""
    wrapper:
        "file:../wrapper/fastaguard"


rule fastaguard_fail:
    input:
        fasta="data/fail.fa"
    output:
        html="fail/fastaguard_report.html",
        json="fail/fastaguard.json",
        tsv="fail/fastaguard.tsv",
        multiqc="fail/fastaguard_mqc.json"
    params:
        profile="assembly",
        gate="none",
        extra=""
    wrapper:
        "file:../wrapper/fastaguard"


rule fastaguard_invalid:
    input:
        fasta="data/invalid.fa"
    output:
        html="invalid/fastaguard_report.html",
        json="invalid/fastaguard.json",
        tsv="invalid/fastaguard.tsv",
        multiqc="invalid/fastaguard_mqc.json"
    params:
        profile="assembly",
        gate="none",
        extra=""
    wrapper:
        "file:../wrapper/fastaguard"
```

- [ ] **Step 3: Add copy-pasteable test runner snippet**

Create `examples/snakemake/wrapper/test/test_wrappers.py`:

```python
import subprocess
from pathlib import Path


def test_fastaguard_wrapper():
    snakefile = Path(__file__).with_name("Snakefile")
    subprocess.run(
        ["snakemake", "-s", str(snakefile), "--cores", "1", "--use-conda"],
        check=True,
    )
```

- [ ] **Step 4: Add Snakemake test fixtures**

Create these files:

`examples/snakemake/wrapper/test/data/pass.fa`

```fasta
>contig1
ACGTACGTACGTACGT
>contig2
GCGCGCATATAT
```

`examples/snakemake/wrapper/test/data/warn.fa`

```fasta
>tiny1
ACGT
>tiny2
NNNNNNNN
```

`examples/snakemake/wrapper/test/data/fail.fa`

```fasta
>dup
ACGTACGT
>dup
ACGTACGT
>bad
ACGTXYZ
```

`examples/snakemake/wrapper/test/data/invalid.fa`

```fasta
>empty_record
>next_record
ACGT
```

- [ ] **Step 5: Update README safe local order**

Add this section to `examples/snakemake/wrapper/README.md` before `The wrapper emits:`:

```markdown
Safe local order before upstream submission:

1. Run repository Python tests that inspect this wrapper layout.
2. Install Snakemake in a workflow test environment.
3. Run `snakemake -s test/Snakefile --cores 1 --use-conda`.
4. Generate a real upstream `environment.linux-64.pin.yaml` if the upstream
   wrapper repository requires a solver-produced pin file.
5. Adapt `test/test_wrappers.py` into the upstream wrapper repository test
   harness.
```

- [ ] **Step 6: Run targeted adoption tests**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets.AdoptionAssetsTest.test_snakemake_wrapper_has_upstream_prep_test_layout
```

Expected: PASS.

### Task 5: Update Readiness Docs In Safe Order

**Files:**
- Modify: `docs/workflow-readiness.md`
- Modify: `docs/adoption-plan.md`
- Modify: `examples/nf-core/README.md`

- [ ] **Step 1: Update `docs/workflow-readiness.md`**

Add a `## Safe Order` section after `## Current State`:

```markdown
## Safe Order

1. Run local repository tests first.
2. Harden the nf-core starter with topic-aware versions, fixtures, and nf-test
   starter coverage.
3. Add collect-then-gate examples with `examples/workflows/check_fastaguard_gate.py`.
4. Harden the Snakemake wrapper starter with metadata, fixture tests, and a
   starter pin file.
5. Run external `nf-core modules lint`, `nf-core modules test`, and Snakemake
   wrapper tests in dedicated upstream checkouts.
6. Open external upstream PRs last.
```

In the `nf-core Readiness` checklist, mark local assets as present and keep external commands as remaining validation gates.

In the `Snakemake Readiness` checklist, mark local metadata and tests as present and keep the upstream generated pin as an external validation gate if needed.

- [ ] **Step 2: Update `examples/nf-core/README.md`**

Add this section after the opening checklist link:

```markdown
Safe local order before upstream submission:

1. Run repository Python tests that inspect this module layout.
2. Install nf-core/tools, Nextflow, and nf-test in a workflow test environment.
3. Run `nf-core modules lint fastaguard`.
4. Run `nf-core modules test fastaguard`.
5. Port the local starter into an upstream nf-core/modules checkout only after
   those checks pass.
```

- [ ] **Step 3: Run safe-order adoption test**

Run:

```bash
python3 -m unittest tests.python.test_adoption_assets.AdoptionAssetsTest.test_workflow_readiness_safe_order_is_explicit
```

Expected: PASS.

### Task 6: Final Verification And PR

**Files:**
- All modified files in this plan.

- [ ] **Step 1: Run all repository Python tests**

Run:

```bash
python3 -m unittest discover -s tests/python
```

Expected: all tests pass.

- [ ] **Step 2: Run Rust tests**

Run:

```bash
cargo test --locked
```

Expected: all Rust, CLI, schema, and doctests pass.

- [ ] **Step 3: Run whitespace and public naming checks**

Run:

```bash
git diff --check
python3 - <<'PY'
from pathlib import Path

terms = ["co" + "dex", "Co" + "dex", "AI " + "agent", "ai " + "agent"]
paths = [
    Path("README.md"),
    Path("docs/adoption-plan.md"),
    Path("docs/workflow-readiness.md"),
    Path("examples"),
    Path("tests/python"),
]
matches = []
for root in paths:
    files = [root] if root.is_file() else [p for p in root.rglob("*") if p.is_file()]
    for path in files:
        text = path.read_text(errors="ignore")
        for line_no, line in enumerate(text.splitlines(), start=1):
            if any(term in line for term in terms):
                matches.append(f"{path}:{line_no}:{line}")
if matches:
    print("\n".join(matches))
    raise SystemExit(1)
PY
```

Expected: `git diff --check` exits 0. The naming scan returns no matches.

- [ ] **Step 4: Commit and publish**

Run:

```bash
git status --short
git add tests/python/test_adoption_assets.py tests/python/test_workflow_gate_helper.py docs/workflow-readiness.md docs/adoption-plan.md examples/nf-core/README.md examples/nf-core/modules/local/fastaguard/main.nf examples/nf-core/modules/local/fastaguard/meta.yml examples/nf-core/modules/local/fastaguard/tests examples/workflows/check_fastaguard_gate.py examples/snakemake/wrapper/README.md examples/snakemake/wrapper/environment.linux-64.pin.yaml examples/snakemake/wrapper/test docs/superpowers/plans/2026-07-03-upstream-workflow-prep.md
git commit -m "docs: prepare upstream workflow starters"
git push -u origin feature/upstream-workflow-prep
```

Expected: branch pushes successfully.

- [ ] **Step 5: Open PR**

Open a normal PR against `main` titled:

```text
Prepare upstream workflow starters
```

PR body should include:

```markdown
## Summary

- harden local nf-core starter metadata and test layout for upstream preparation
- add a workflow gate helper for collect-then-gate examples
- harden local Snakemake wrapper starter metadata and test layout
- document the safe order before external upstream PRs

## Validation

- `python3 -m unittest discover -s tests/python`
- `cargo test --locked`
- `git diff --check`
- public naming scan
```

Expected: PR opens cleanly and remote checks pass before merge.

## Self-Review

Spec coverage:

- nf-core starter hardening: Task 2.
- evidence-preserving workflow pattern: Task 3.
- Snakemake wrapper hardening: Task 4.
- safe order and external PR gate: Task 5.
- verification and publish: Task 6.

Placeholder scan:

- No `TBD`, `TODO`, or `implement later` placeholders are present.
- External upstream PRs are intentionally not performed in this plan; they are gated behind local verification and dedicated upstream checkouts.

Type and path consistency:

- `check_fastaguard_gate.py` is referenced consistently in docs and tests.
- nf-core fixture names match across tests and nf-test starter.
- Snakemake fixture names match across tests and wrapper test Snakefile.
