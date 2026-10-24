# Cover letter — *Bioinformatics* Application Note (draft)

**To:** The Editors, *Bioinformatics* (Oxford University Press)  
**From:** Ehsan Estaji and Jian-Feng Mao  
**Re:** Application Note submission — *FastaGuard: FASTA preflight QC for modern bioinformatics pipelines*

Dear Editors,

Please consider our manuscript as an **Application Note** in *Bioinformatics*.

FastaGuard is an open-source (MIT), streaming-first Rust command-line tool that
provides **FASTA preflight quality control** for genome-assembly pipelines. It
validates FASTA structure, flags identifier and composition hazards, and emits a
stable multi-format report contract (HTML, JSON, TSV, and MultiQC-compatible
JSON) with explicit pipeline gates (`gate.can_continue`) before expensive
downstream steps such as QUAST, BUSCO, BlobToolKit, CheckM, annotation, or
official repository validators. The contribution is a pipeline-native preflight
*contract*—not interpretive assembly QC and not a substitute for those tools—so
workflow authors can stop or route early on FASTA-level blockers. The software
is published on Bioconda and BioContainers (v1.0.0) with merged nf-core module
and Snakemake wrapper integrations.

We believe *Bioinformatics* Application Notes are the appropriate venue for a
short, citable description of freely available bioinformatics software with clear
Availability and Implementation. The manuscript follows Application Note length
and abstract conventions (Summary / Availability and Implementation / Contact /
Supplementary information). Source and documentation:
https://github.com/ehsanestaji/FastaGuard

**Suggested topical area:** genome assembly / sequence analysis software tools  
(Editors may reassign as appropriate.)

Both corresponding authors can be reached at ehsan.estaji@umu.se and
jianfeng.mao@umu.se. We confirm that the work is original, is not under
consideration elsewhere, and that all authors approve submission.

Thank you for your consideration.

Sincerely,  
Ehsan Estaji  
Jian-Feng Mao  
Umeå Plant Science Centre, Department of Plant Physiology, Umeå University
