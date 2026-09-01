# FastaGuard 1.0 Reference Contract

Status: implementation-ready; contract freeze pending release version and publication approval
Date: 2026-08-28

## Summary

FastaGuard 1.0 adds a reference-contract gate alongside the existing assembly
preflight commands.

> Know exactly which reference genome you have, show what each related file
> proves about compatibility, and stop incompatible workflows before expensive
> analysis begins.

The new `reference` command treats one FASTA as the canonical reference and
compares explicitly supplied indexes, dictionaries, alignments, variants and
annotations against it. It produces the same report-first workflow contract as
FastaGuard 0.7: PASS, WARN and FAIL are data in the reports, while process exit
codes describe whether the command completed.

This is a preflight compatibility check. It does not repair files, replace
format-specific validators, manage reference downloads or claim biological
correctness.

## Product goals

FastaGuard 1.0 must:

1. identify the physical FASTA file and its biological sequence content;
2. produce portable, content-derived reference and coordinate identities;
3. compare companion files through one explicit compatibility vocabulary;
4. distinguish proven compatibility from header assertions and missing evidence;
5. provide deterministic machine output suitable for workflow gates;
6. remain useful offline with loose files from collaborators or archives;
7. preserve all documented FastaGuard 0.7 assembly and compare behaviour; and
8. support later nf-core and Snakemake integration through a stable CLI, schema
   and container package.

The primary users are bioinformaticians receiving references and related files
from multiple sources, core facilities standardising projects, and workflow
maintainers who need an early, explainable stop/go decision.

## Non-goals

Version 1.0 will not:

- rename contigs or rewrite, repair or normalise user files;
- infer reference identity from filenames;
- silently treat names such as `1` and `chr1` as equivalent;
- download, register or manage reference assets;
- build aligner indexes;
- perform complete BAM, CRAM, VCF, BCF or GFF3 validation;
- validate GFF3 feature semantics or attributes;
- accept GTF as a reference-declaration format;
- prove biological completeness, taxonomic purity or submission acceptance;
- provide a hosted service, shared database or online registry; or
- support compressed-FASTA byte-layout validation that requires `.gzi` data.

These boundaries keep the command a small, auditable compatibility gate rather
than another reference-management system.

## User contract

### Command

The stable command shape is:

```bash
fastaguard reference reference.fa \
  --fai reference.fa.fai \
  --dict reference.dict \
  --alignment sample.bam \
  --variants known-sites.vcf.gz \
  --annotation genes.gff3 \
  --alias-map aliases.tsv \
  --policy coordinate \
  --outdir reports \
  --prefix project-reference
```

The canonical FASTA is required. The following options are optional and must be
explicit; FastaGuard does not discover companions from filenames:

| Option | Cardinality | Meaning |
| --- | ---: | --- |
| `--fai <path>` | zero or one | FASTA index to compare |
| `--dict <path>` | zero or one | SAM sequence dictionary to compare |
| `--alignment <path>` | repeatable | BAM or CRAM header to compare |
| `--variants <path>` | repeatable | VCF or BCF declarations to compare |
| `--annotation <path>` | repeatable | GFF3 declarations and feature bounds to compare |
| `--alias-map <path>` | zero or one | explicit global contig-name mapping |
| `--policy <mode>` | one | `strict`, `coordinate` or `advisory`; default `coordinate` |
| `--require <kinds>` | repeatable | comma-separated required kinds: `fai`, `dict`, `alignment`, `variants`, `annotation` |
| `--format <format>` | repeatable | requested bundle formats: `html`, `json`, `tsv`, `multiqc` |
| `--write-lock <path>` | zero or one | write a portable reference lockfile |

Repeated artefacts are evaluated independently. A required kind means at least
one readable, structurally parseable artefact of that kind must supply a usable
declaration. For `.fai`, `.dict`, BAM, CRAM, VCF and BCF, that means at least one
valid sequence entry; for GFF3, it means at least one valid `##sequence-region`
or feature sequence identifier. Missing or unusable required input is a reported
policy failure, not a process error, and blocks every policy including
`advisory`.

Reference mode does not accept assembly-only `--profile`, `--gate`, submission
target or assembly-threshold options. Its gate kind is fixed to `reference`, and
`--policy` selects the compatibility policy within that gate.

The command reuses the FastaGuard 0.7 `--outdir`, `--prefix`, direct report-path
and `--force` conventions. Reference mode adds selective output as a new,
subcommand-local capability: with no `--format`, all four reports are produced;
when `--format` is present, only the named formats are produced. `--format`
cannot be combined with direct report-path options. JSON is the source of truth
and is published last whenever it is among the requested reference outputs.
Assembly and compare output selection and publication order remain unchanged.

### Alias map

The alias map is a UTF-8 TSV with this exact header:

```text
declared_name	reference_name
```

Each side must be unique, making the map one-to-one. Names are matched as exact,
case-sensitive strings. Wildcards, prefixes, regular expressions and chained
aliases are rejected. The map applies to every supplied companion, is recorded
in the manifest, and never changes an input file. Invalid or ambiguous mappings
produce a blocking structured finding.

Version 1.0 has no per-artefact alias maps. A name absent from the map must match
the canonical FASTA exactly. Adapter-specific alias claims, including SAM `AN`
tags, are recorded but never applied automatically. Explicit aliases translate
names before set, subset and order comparisons; order is then evaluated using
the resolved canonical names. Aliases resolve names only and cannot repair order
drift or change any length, digest or coordinate.

### Outputs

Reference mode produces the established report family:

- HTML for human review;
- JSON as the complete machine contract;
- TSV as a compact artefact summary; and
- MultiQC-compatible JSON for run-level visibility.

Every reference JSON report has `report_type: "reference"`. Version 1.0 adds the
additive `report_type: "assembly"` field to the existing single-FASTA report;
compare reports retain their existing `report_type: "compare"` field. All other
documented fields remain available. `fastaguard --schema`,
`--finding-catalog` and `--explain-finding` continue to expose the public
contract.

The optional lockfile contains the semantic reference manifest only. It excludes
local paths and timestamps and is safe to move between systems. It is an
identity record, not a cryptographic signature or provenance attestation.

Reference-mode MultiQC output remains one custom-content table. It uses
`id: "fastaguard_reference"`, section name `FastaGuard Reference`, and one row
per canonical reference. The row contains `verdict`, `gate_can_continue`,
`reference_policy`, supplied and required artefact counts, total mismatch count,
and counts for each relationship. Paths, per-companion rows and contig examples
are omitted; the full evidence remains in JSON and the one-row-per-companion TSV.
Existing assembly and compare MultiQC shapes do not change.

## Architecture

Reference mode is an isolated layer beside the existing assembly path:

```text
reference command
    -> canonical FASTA catalogue
    -> format adapters
    -> normalised declarations
    -> compatibility engine
    -> reference policy
    -> findings and reports
```

The existing assembly and compare execution paths are not routed through the
new compatibility engine.

### Canonical FASTA catalogue

`ReferenceCatalog` is built in a single streaming pass and contains:

- FASTA record order;
- exact sequence identifiers, defined as the first whitespace-delimited token
  after `>` on each FASTA definition line;
- sequence lengths;
- SAM-compatible MD5 values;
- GA4GH refget sequence identifiers;
- the GA4GH refget Sequence Collections digest plus the level-1
  `name_length_pairs` and `sorted_name_length_pairs` digests; and
- byte-layout observations needed for plain-FASTA `.fai` verification.

Sequence-collection encoding is pinned to the GA4GH
`seqcol_extended_v1.0.0` schema at
`https://ga4gh.github.io/refget/schemas/seqcol_extended_v1.0.0.json`. Its
additional coordinate attributes are non-inherent, so its top-level digest is
compatible with the minimal version 1.0 schema. The manifest records this exact
schema identifier. `name_length_pairs_digest` is the ordered level-1 attribute
digest; `sorted_name_length_pairs_digest` is the order-invariant level-1
attribute digest.

The raw input SHA-256 is calculated over the FASTA file bytes. Sequence and
collection identities are calculated from decoded biological sequence according
to their published specifications. Physical, sequence, collection and
coordinate identities remain separate facts.

The existing assembly preflight runs while the canonical catalogue is built.
FastaGuard never removes an invalid sequence character merely to obtain a
digest. A record that cannot safely satisfy the nucleotide identity contract has
null sequence identities with `insufficient` evidence, and no collection digest
is claimed for an incomplete catalogue. Duplicate identifiers, invalid
structure or unsafe alphabet findings remain visible and block every reference
policy when they are critical.

Plain and gzip-compressed FASTA are supported for sequence identities. For a
compressed canonical FASTA, `.fai` names and lengths can be compared, but byte
offsets, line geometry and random-access behaviour are reported as unverified.
This is declaration comparison, not validation that the compressed index is
usable. Strict policy blocks on that missing layout evidence. BGZF `.gzi`
validation is outside version 1.0.

### Format adapters

Each adapter reads one artefact and returns a neutral `DeclaredDictionary`.
Adapters collect evidence; they do not assign PASS, WARN or FAIL.

All declarations retain their original values and source locations. Parsers are
streaming where practical, reject unsafe structural input, and bound displayed
examples while preserving complete mismatch counts.

Binary readers sit behind private adapter interfaces and use a mature,
tightly-pinned implementation. They are not exposed in FastaGuard's public data
model, so a parser can be replaced without changing report semantics. The core
version 1.0 command has no external validator runtime dependency. The manifest's
`delegated_checks` array remains empty; a later opt-in delegate must record its
name, version and result separately and may not replace core evidence.

#### FASTA index

The `.fai` adapter compares sequence name, length, byte offset, line bases, line
width and order. A complete plain-FASTA match supplies layout evidence. Stale
lengths, offsets or line geometry are contradictions even if names match.

#### SAM sequence dictionary

The `.dict` adapter reads `@SQ` records and compares `SN`, `LN`, order and `M5`
when present. Other tags are retained as provenance but are not treated as
identity evidence. A missing `M5` weakens evidence; it does not imply a digest
match.

#### BAM and CRAM

Alignment adapters inspect the SAM header only. They compare `@SQ` names,
lengths, order and available `M5` values. They do not scan alignments, validate
records or prove that a CRAM can be decoded with the supplied FASTA. Those
limitations are stated in the report.

#### VCF and BCF

Variant adapters inspect header contig declarations and their available length
and digest fields. A declared subset can be valid because a variant file may use
only part of a reference. If no usable contig declarations exist, the result is
`indeterminate`; records are not scanned to infer a dictionary. REF-allele
validation remains a separate deep check and is not part of the core gate.

#### GFF3

The GFF3 adapter reads `##sequence-region` directives and streams feature
coordinates. It verifies that encountered sequence names resolve to the
canonical catalogue and that start/end coordinates lie within reference bounds.
It does not interpret attributes, ontology, phase correctness or biological
feature relationships. This single cross-reference pass is not general GFF3
validation. A valid annotation subset can be accepted. GTF is not accepted in
version 1.0 because it has no equivalent portable sequence dictionary.

The sole attribute-level exception is the prescribed `Is_circular=true` marker
on a landmark feature. The marker is valid for this gate only when that feature
uses the same sequence identifier, starts at 1 and ends at the canonical
reference length. For that sequence, an origin-crossing feature may have an end
greater than the reference length when its start is within the reference and its
span covers no more than one complete revolution. Without a valid circular
landmark marker, a coordinate beyond the reference length is incompatible. No
other GFF3 attribute affects the reference gate.

#### Alias map

The alias adapter validates the explicit one-to-one mapping before any name
translation. Both original and resolved names remain in evidence. An alias can
resolve a naming difference, but it cannot conceal a length, digest or bounds
contradiction.

### Compatibility engine

`CompatibilityEngine` is a pure deterministic comparison over a
`ReferenceCatalog`, a `DeclaredDictionary` and an optional validated alias map.
It emits facts and findings without reading files, writing reports or selecting
a policy outcome. This separation keeps format parsing, biological comparison
and workflow policy independently testable.

### Reference policy

`ReferencePolicy` consumes the comparison result and decides whether findings
block the workflow. It does not modify evidence or compatibility labels.

## Evidence model

### Identity layers

FastaGuard reports four distinct identity layers:

| Layer | Evidence | Question answered |
| --- | --- | --- |
| Physical | SHA-256 of original FASTA bytes | Is this the same stored file? |
| Sequence | per-contig SAM MD5 and GA4GH refget identifier | Is each biological sequence the same? |
| Collection | GA4GH SeqCol digest | Is this the same named, ordered sequence collection? |
| Coordinate | SeqCol `name_length_pairs` and `sorted_name_length_pairs` digests | Is this the same ordered or order-invariant coordinate system? |

No layer is substituted for another. In particular, matching byte hashes do not
replace coordinate evidence, and matching names and lengths do not prove
sequence content.

### Relationship

Each companion receives exactly one top-level relationship:

| Relationship | Definition |
| --- | --- |
| `exact` | The full declared set, names, lengths and required order match, and the strongest identity or layout evidence that the artefact type carries also matches. |
| `content_equivalent` | All declared biological sequences are proven equal, but coordinate labels or required order differ. |
| `coordinate_compatible` | The required full coordinate declaration matches, but sequence-content proof is unavailable. |
| `subset_compatible` | Every declared/used sequence resolves and matches, no extra contradiction exists, and the declaration is a proper subset. |
| `incompatible` | At least one unresolved name or one length, digest, layout or bounds fact contradicts the canonical reference, and content equivalence cannot explain the naming difference. |
| `indeterminate` | Available evidence cannot establish compatibility or a contradiction. |

Order requirements are artefact-aware and always reported explicitly.
`subset_compatible` is accepted by the default policy only for VCF, BCF and
GFF3. For `.fai`, `.dict`, BAM and CRAM, an incomplete dictionary is blocking
under the default policy.

The engine assigns relationships in this deterministic precedence order after
applying the explicit alias map:

1. no usable declaration produces `indeterminate`;
2. a length, digest, layout or bounds contradiction on a resolved coordinate
   produces `incompatible`;
3. a full set with proven-equal sequence content produces `exact` when resolved
   names and required order also match, otherwise `content_equivalent`;
4. any remaining declared name or sequence that cannot resolve to the canonical
   catalogue produces `incompatible`;
5. a contradiction-free proper subset produces `subset_compatible`;
6. a full set with the strongest layout evidence supported by `.fai` produces
   `exact`;
7. a full matching coordinate declaration without content proof produces
   `coordinate_compatible`; and
8. every remaining case produces `indeterminate`.

For `.fai`, complete verified byte layout is the strongest native evidence. For
`.dict`, BAM, CRAM, VCF and BCF, supported sequence digests are required for
`exact`. GFF3 can reach coordinate or subset compatibility but not exact content
identity. After explicit alias resolution, a name difference is reconciled; an
order difference remains an order difference.

### Evidence strength

Relationship and evidence strength are separate axes:

| Strength | Meaning |
| --- | --- |
| `content_verified` | Published sequence-content identifiers were calculated or matched. |
| `metadata_verified` | Structural metadata was checked against the canonical FASTA, including lengths, layout or feature bounds. |
| `header_asserted` | Compatibility relies on declarations that were not independently content-verified. |
| `explicit_alias_mapping` | A validated user-supplied one-to-one alias was needed. |
| `insufficient` | Evidence is missing or unusable. |

These labels are not a total ordering. An artefact reports one primary evidence
level plus every applicable qualifier; `explicit_alias_mapping` is always an
additional qualifier rather than a substitute for content or metadata proof.
Missing checksums never become `content_verified`. Aliases never become implicit
evidence. For dictionary-bearing formats, an omitted supported digest prevents
an `exact` result.

## Policy and gate behaviour

### Policy modes

`coordinate` is the default because it is useful with common laboratory files
that lack content digests while still blocking contradictions.

| Result | `strict` | `coordinate` | `advisory` |
| --- | --- | --- | --- |
| `exact` | continue | continue | continue |
| `content_equivalent` | block | block because the coordinate system still differs | continue |
| `coordinate_compatible` | block for missing content proof | continue | continue |
| `subset_compatible` | block | continue for VCF/BCF/GFF3; block for other kinds | continue |
| `incompatible` | block | block | continue |
| `indeterminate` | block | warn when optional; block when required | continue when optional; block when required |

In advisory mode, biological compatibility findings never block, but process
errors still prevent successful completion. Advisory is deliberately a
report-only policy: `gate.can_continue` may be true while `verdict.status` and
`gate.status` are FAIL for a known incompatibility. Optional artefacts that were
not provided are recorded as coverage gaps. They block only when named by
`--require`.

Advisory mode does not override critical findings in the canonical FASTA. This
preserves the existing rule that a structurally unusable reference cannot be
declared safe for downstream work.

The required-input rule has precedence over the relationship table. Supplying a
readable VCF with no usable contig declaration, for example, does not satisfy
`--require variants` and makes `gate.can_continue` false under every policy.

### Gate output

Reference reports retain the established gate fields:

- `gate.mode` is always `reference`;
- `gate.reference_policy.id` is `strict`, `coordinate` or `advisory`;
- `gate.reference_policy.version` is the policy-contract version;
- `gate.reference_policy.required_kinds` records the effective requirements;
- `gate.status` mirrors `verdict.status`, as in FastaGuard 0.7;
- `gate.can_continue` is the policy decision;
- `gate.blocking_findings` contains stable finding IDs; and
- `gate.advisory_findings` contains non-blocking finding IDs.

Workflow engines must route on `gate.can_continue` or the documented JSON
fields, not on process exit status. A completed FAIL report exits zero.

### Verdict, safety and readiness

Reference verdicts describe evidence independently of the chosen policy:

- FAIL means at least one incompatibility, malformed supplied artefact or
  missing required artefact exists;
- WARN means there is no FAIL condition, but at least one relationship is not
  exact, an alias was required, or optional coverage/evidence is missing; and
- PASS means the canonical FASTA passes preflight, all supplied companions are
  exact, all required kinds are present, and there are no coverage warnings.

`machine_summary.safe_for_downstream` remains true only for PASS. A reference
report reuses the file, structure and alphabet readiness categories for the
canonical FASTA and adds a `reference_compatibility` category. Submission
readiness is not evaluated in reference mode. `readiness.overall.status` mirrors
the verdict, while `gate.can_continue` remains the selected policy decision.
This preserves the documented distinction between evidence status and workflow
permission.

### Findings

Reference finding IDs use the `reference_` namespace. The catalogue covers at
least these stable categories before the release candidate:

- missing required artefact;
- malformed declaration;
- missing or extra contig;
- length, digest or order mismatch;
- FASTA-index layout mismatch or unverified layout;
- invalid or ambiguous alias mapping;
- insufficient identity evidence; and
- annotation name or bounds mismatch.

Each finding records the artefact, original and resolved contig name where
applicable, expected and observed values, relationship, evidence strength,
policy effect, total affected count and at most 20 deterministic examples. It
also states one concrete corrective action without claiming that the input was
automatically repaired.

## Process errors and report publication

Reference mode keeps the FastaGuard 0.7 process contract:

```text
0 = requested reports completed, including PASS, WARN and FAIL
2 = command-line usage error
3 = configuration, unreadable input, unavailable explicitly requested capability,
    runtime or output-publication error
```

An existing but structurally malformed canonical FASTA or companion is a
reported blocking finding and exits zero when enough input can be consumed to
produce honest reports. A path that cannot be opened, an interrupted compressed
stream, or another read failure that prevents complete analysis is a process
error and exits three. This distinction ensures that users receive evidence for
bad biological files but do not mistake incomplete execution for a QC result.

All requested reference reports and the optional lockfile are collision-checked
and staged before publication. The lockfile is a separate optional fifth output,
not part of the default four-report bundle. Final publication remains sequential
rather than transactionally atomic across files. Human-readable, TSV and
MultiQC outputs are published first; the lockfile is next; requested JSON is
published last so its presence can serve as the completion signal. If JSON is
not requested, the lockfile is last. Existing no-clobber and `--force` behaviour
applies to every selected path. Assembly and compare publication order is not
changed by this feature.

## Reference manifest

The JSON report contains a versioned `reference_manifest` with this logical
shape:

```json
{
  "manifest_version": "1.0.0",
  "semantic_digest": "sha256:<digest>",
  "canonical_reference": {
    "physical_sha256": "<digest>",
    "sequences": [],
    "seqcol_schema": "https://ga4gh.github.io/refget/schemas/seqcol_extended_v1.0.0.json",
    "seqcol_digest": "<GA4GH SeqCol digest>",
    "name_length_pairs_digest": "<ordered coordinate digest>",
    "sorted_name_length_pairs_digest": "<order-invariant coordinate digest>"
  },
  "artifacts": [],
  "comparisons": [],
  "coverage": {},
  "delegated_checks": []
}
```

The JSON field is named `artifacts` to remain consistent with the existing
machine contract, while prose uses British spelling.

Version 1.0 always records the raw SHA-256 of the canonical FASTA because that
file is already streamed in full. It does not hash complete BAM, CRAM, VCF or
BCF companions by default; doing so would turn a header gate into a full-file
I/O pass. The semantic lock records their normalised declarations and comparison
evidence instead. Report provenance retains each companion path outside the
portable semantic payload.

The semantic digest is SHA-256 over the RFC 8785 canonical JSON representation
of the semantic manifest payload; the `semantic_digest` field itself is not part
of that payload. The payload includes reference sequence and collection
identities, explicit aliases, companion declarations, comparison relationships,
evidence strengths, selected policy, required artefact kinds and policy-relevant
limitations. It excludes:

- physical file hashes, including `physical_sha256`;
- absolute and relative paths;
- file modification times;
- command text;
- start, completion and duration values;
- output destinations; and
- presentation-only report fields.

Only declarations that affect identity, coordinates or policy enter the
semantic payload. Provenance-only header claims such as SAM `UR`/`AS` and VCF
`URL`/`assembly` remain in the full report but do not affect the digest.

Rewrapping or recompressing an otherwise identical FASTA therefore changes its
physical SHA-256 but not the semantic digest.

Each companion has a `declaration_digest`: SHA-256 over the RFC 8785 canonical
JSON form of its normalised identity and coordinate declaration, excluding its
path and provenance-only tags.

Arrays whose order is biologically meaningful retain that order. Sets and maps
are serialised in a documented canonical order. Repeated companions are sorted
by artefact kind and declaration digest, retaining duplicate occurrences; CLI
order and filenames do not affect the semantic digest. Identical content and
inputs therefore produce the same semantic digest across directories, machines
and runs. Runtime provenance remains available outside the semantic payload.

## Backward compatibility

Version 1.0 is additive around the existing product:

- `fastaguard <fasta>` remains the assembly preflight command;
- `fastaguard compare ...` remains the cohort command;
- existing flags, default outputs, finding meanings and exit codes remain
  unchanged;
- documented v0.7 JSON paths remain present with the same types and meanings;
- the schema becomes a report-type-discriminated contract containing assembly,
  compare and reference reports;
- new findings are confined to the `reference_` namespace; and
- the nf-core module and Snakemake wrapper for the established command remain
  unchanged.

Reference-mode workflow components will be proposed separately only after the
CLI, packages, containers, schema and finding catalogue are stable. They will
call the public CLI rather than embed FastaGuard implementation code.

Migration fixtures must demonstrate that every documented v0.7 consumer query
continues to work against version 1.0 assembly output. Any unavoidable breaking
change delays the final release and requires an explicit migration document.

## Verification strategy

### Identity and policy tests

- official SAM MD5 examples and GA4GH refget/SeqCol test vectors;
- every relationship and evidence-strength combination;
- all three policies, required-kind usability combinations and alias outcomes;
- canonical-manifest and semantic-digest determinism across paths and runs; and
- bounded, deterministic finding examples.

### Format corpus

The committed and generated fixture corpus covers:

- plain and gzip FASTA, descriptive definition lines, wrapping differences,
  CRLF, duplicate records and structural corruption;
- `.fai` with stale length, offset, line-bases, line-width, missing, extra and
  reordered entries;
- `.dict`, BAM and CRAM with missing, extra, reordered, length-mismatched and
  digest-mismatched sequence declarations;
- VCF and BCF with complete, partial, absent and contradictory contig headers;
- GFF3 with valid subsets, missing declarations, unknown sequence names,
  out-of-bounds features and valid/invalid circular landmarks; and
- malformed, truncated and adversarial inputs for every adapter.

### Differential checks

Expected identities and declarations are cross-checked against authoritative
implementations:

- GA4GH refget/SeqCol for sequence and collection identities;
- samtools and Picard for SAM dictionaries and header behaviour; and
- bcftools or GATK for selected deep variant-reference fixtures.

External tools are test oracles, not silent runtime dependencies. A disagreement
is investigated and recorded; expected values are never changed merely to make
a differential test pass.

### Compatibility and integration checks

- the complete FastaGuard 0.7 command and golden-report suite;
- schema migration fixtures from 0.7 to 1.0;
- real nf-core and Snakemake smoke workflows;
- Linux and macOS builds;
- Bioconda and BioContainer installation and execution; and
- reproducible time and peak-memory measurements on human, microbial and
  non-model reference bundles.

Large-file behaviour is streaming: one FASTA pass for sequence identities,
header-only reads for BAM/CRAM/VCF/BCF, and one GFF3 feature pass. Memory scales
with sequence declarations and bounded evidence, not full file size.

## Release stages

### 1.0 alpha

Ship the isolated command, canonical catalogue, FASTA/FAI/dictionary adapters,
identity calculations, compatibility engine, policies and deterministic
manifest. CLI and schema may still change between alpha releases.

### 1.0 beta

Add BAM, CRAM, VCF, BCF and GFF3 adapters; complete report parity; publish the
fixture corpus and differential results; and distribute prerelease packages and
containers.

### Optional adoption feedback

Report-only pilots are optional after the release. They may cover human,
microbial, or non-model work and can record time to first useful result, false
or confusing classifications, and workflow changes. They do not gate the 1.0
release and do not alter a laboratory's workflow policy without its own review.

### 1.0 release candidate

Freeze the CLI, JSON schema, lockfile, policy meanings and finding IDs. Only
defect corrections and documentation clarification are accepted after the
freeze.

### 1.0 final

The final release is made only when all go/no-go criteria below pass. Otherwise
the project remains on an alpha, beta or release-candidate version.

## Go/no-go criteria

FastaGuard 1.0 is ready only when:

1. there is no known incorrect compatibility classification in the accepted
   corpus;
2. every documented FastaGuard 0.7 workflow remains operational;
3. report and semantic-manifest output is deterministic;
4. the accepted automated corpus covers human, microbial, and non-model
   references, with expected compatibility outcomes for each supported
   declaration format;
5. a new user can install the tool and obtain an interpretable result within
   five minutes using the quickstart;
6. performance and memory behaviour are documented with reproducible commands;
7. packages, containers, schema discovery and finding discovery pass release
   checks; and
8. documentation contains no unsupported acceptance, correctness or biological
   claims.

## Privacy and operational safety

Reference mode performs no network requests. It sends no filenames, sequences,
headers, statistics or findings to an external service. Reports retain explicit
local input paths for provenance, while the portable lockfile and semantic
digest exclude them. Examples in findings are bounded to prevent unmanageable
reports, and input files are never modified.

## Authoritative specifications

- [SAM/BAM specification](https://samtools.github.io/hts-specs/SAMv1.pdf)
- [CRAM specification](https://samtools.github.io/hts-specs/CRAMv3.pdf)
- [VCF specification](https://samtools.github.io/hts-specs/VCFv4.5.pdf)
- [GA4GH refget Sequences](https://ga4gh.github.io/refget/sequences/)
- [GA4GH refget Sequence Collections](https://ga4gh.github.io/refget/seqcols/)
- [GFF3 specification](https://github.com/The-Sequence-Ontology/Specifications/blob/master/gff3.md)
- [JSON Canonicalization Scheme, RFC 8785](https://www.rfc-editor.org/rfc/rfc8785)
