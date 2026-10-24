#!/usr/bin/env python3
"""Capability comparison of FASTA validators on a small synthetic corpus.

HONEST SCOPE. The corpus is AUTHOR-CONSTRUCTED and was designed around the check list of
FastaGuard (a FASTA preflight-QC CLI), so it favours FastaGuard. The tools were NOT built
for the same purpose: py_fasta_validator is a minimal syntax checker, fa-lint a protein/
nucleotide FASTA linter, table2asn an NCBI submission converter whose validator is a side
effect, SeqKit a general toolkit whose `seq -v` checks residue letters. Expectations
(valid / error / advisory) were written down BEFORE any tool was run; advisory rows are
reported but never scored. Grounding: https://www.ncbi.nlm.nih.gov/genbank/fastaformat/
(F) and https://www.ncbi.nlm.nih.gov/genbank/genomesubmit/ (G), read 2026-10-24.

USAGE (stdlib only, Python 3.9+):
  run_comparison.py --corpus-dir DIR [--out-dir DIR] [--work-dir DIR] [--timeout SEC]
      [--fastaguard P] [--py-fasta-validator P] [--fa-lint P] [--table2asn P] [--seqkit P]
      [--archives DIR] [--generate-only] [--real [LABEL=]FASTA [[LABEL=]FASTA ...]]
      [--real-timeout SEC]
Tool location: flag, else env FASTAGUARD / PY_FASTA_VALIDATOR / FA_LINT / TABLE2ASN /
SEQKIT, else the same-named executable on PATH. A missing tool is reported as n.a.
--out-dir defaults to this script's directory (results.tsv, summary.tsv,
tool_versions.json, plus real_results.tsv with --real). --generate-only writes the corpus,
prints name/scope/expectation/sha256/rationale as TSV and exits. --archives DIR re-hashes
downloaded archives found anywhere below DIR against the pinned SHA-256 values below.
--real runs the same tools on real files (no expectation) into real_results.tsv; give each
file a unique LABEL=PATH (the label defaults to the file name) and a longer --real-timeout.

DECISION RULES (flagged = yes | no | crash | n.a.; strict is a second, stricter level)
  py_fasta_validator -f F: exit 1/2/4 (documented codes) -> yes; 0 -> no; Traceback,
      signal, timeout or any other code -> crash. Exit code recorded.
  fa-lint -fasta F: exit != 0 or a logrus level=error|fatal|warn|panic on stderr -> yes;
      "panic:"/"goroutine" in stderr, signal or timeout -> crash.
  seqkit seq -v -t dna F -o /dev/null: exit != 0 or [ERRO]/[WARN] on stderr -> yes;
      panic text, signal or timeout -> crash. (-t dna is explicit: with -t auto the
      alphabet is guessed from a prefix and tiny inputs are not validated.)
  table2asn: repo invocation of scripts/verify_ncbi_genome_policy.py (-t synthetic
      template -M n -j "[organism=synthetic construct][moltype=genomic DNA]" -V vb -Z).
      Every file gets two template-induced baseline warnings, so a bare "warning" is not
      attributable to the file. flagged = yes if (a) a console Error/Fatal/Critical block
      (FASTA reader rejections print there, exit 1, no .val), or (b) a .val/.stats
      message of level Error/Critical/Fatal/Reject (the repo's "rejected"), or (c) a
      Warning whose rule code is absent from the valid_simple control run, or a console
      Warning block with a Problem line (FASTA-reader warnings; added after the first run,
      it only affects gap_dash_in_sequence). strict = yes only for (a)/(b). Non-zero exit
      without such evidence, a console "Cannot open" (what it prints for an empty file),
      signal, timeout (it hangs on duplicate SeqIDs: idle, >240 s checked by hand), or no
      artifact at all -> crash (repo: tool_error).
  fastaguard_pipeline: --profile assembly --gate pipeline; fastaguard_ncbi: --profile
      assembly --gate submission --submission-target ncbi; defaults otherwise (no
      --min-contig-length). flagged = verdict.status != PASS; strict = not
      gate.can_continue ("blocked"). Missing/invalid JSON -> crash. FastaGuard exits 0
      even for FAIL, so the exit code carries no verdict.
For non-layered tools strict == flagged. Scoring: error rows -> flagged is TP (else miss),
valid rows -> flagged is FP (else TN), crash rows are counted separately, advisory rows
are reported only. Per-scope tallies: syntax, identifier, content, ncbi, control.
results.tsv columns: file, scope, expectation, tool, flagged, strict, exit_code, detail.
Reproduce: see tool_versions.json (pinned archives) and the command in the final report.
"""

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import zlib
from pathlib import Path

TWO_LEVEL = {"table2asn", "fastaguard_pipeline", "fastaguard_ncbi"}
TOOLS = ["py_fasta_validator", "fa-lint", "table2asn", "seqkit",
         "fastaguard_pipeline", "fastaguard_ncbi"]
ANSI = re.compile(r"\x1b\[[0-9;]*m")
ERR_RE = re.compile(r"(?i)(?:^|[\s\[])(?:FATAL|CRITICAL|ERROR)(?:\]|\s*:)")  # from the repo script
VAL_RE = re.compile(r"^\s*(\w+):\s+\S+\s+\[([^\]]+)\]\s*(.*)$")
ERR_LEVELS = {"error", "critical", "fatal", "reject", "rejected"}
BAD_FALINT = re.compile(r"level=(?:error|fatal|warn|warning|panic)", re.I)
T2A_ARGS = ["-M", "n", "-j", "[organism=synthetic construct][moltype=genomic DNA]", "-V", "vb", "-Z"]
T2A_TEMPLATE = """Submit-block ::= {
  contact {
    contact {
      name name {
        last "Validation",
        first "Local"
      }
    }
  },
  cit {
    authors {
      names std {
        {
          name name {
            last "Validation",
            first "Local",
            initials "L."
          }
        }
      },
      affil std {
        affil "Controlled local validation",
        city "Brussels",
        country "Belgium"
      }
    }
  },
  subtype new
}

Seqdesc ::= pub {
  pub {
    gen {
      cit "unpublished",
      authors {
        names std {
          {
            name name {
              last "Validation",
              first "Local",
              initials "L."
            }
          }
        }
      },
      title "Controlled FASTA validation fixture"
    }
  }
}
"""
# Pinned archives (macOS arm64 builds). Digests were checked at download time against the
# publisher's own value (GitHub release digest / PyPI JSON / repo evidence for table2asn).
SOURCES = {
    "py_fasta_validator": ("https://files.pythonhosted.org/packages/source/p/py_fasta_validator/"
                           "py_fasta_validator-0.6.tar.gz",
                           "ca29c20e1be31ef190f923b62ddb7fccda8c1c3ba9cf699a6ccaccce10ea011a",
                           ["-v"], "PyPI sdist of project py-fasta-validator; built in a python3.11 venv "
                           "with setuptools==80.9.0 because the shipped CLI imports pkg_resources"),
    "fa-lint": ("https://github.com/GallVp/fa-lint/releases/download/1.2.0/fa-lint-darwin-arm64.tar.gz",
                "6edcb792489b18bcfd03156912d05227ac05cdf657c517041c40489566470436",
                ["-version"], "official release binary"),
    "table2asn": ("https://ftp.ncbi.nlm.nih.gov/asn1-converters/by_program/table2asn/mac.table2asn.gz",
                  "348b9d5f1b05065f4e5e61b6c2350e5ff044de2d6dc14765be6ac02c00e59341",
                  ["-version"], "digest equals the one pinned in docs/evidence/"
                  "fastaguard-v0.7-ncbi-genome-policy.md; NCBI publishes no separate digest"),
    "seqkit": ("https://github.com/shenwei356/seqkit/releases/download/v2.14.0/seqkit_darwin_arm64.tar.gz",
               "7b3a6a7c9be07a06c5b308d1aad769c09762b36b387690eed8b91b00d01503b7",
               ["version"], "official release binary; md5 2b335eb27f69af97a252818a85145684 also matched"),
    "fastaguard": ("local build of `git archive v1.0.0` of https://github.com/ehsanestaji/FastaGuard",
                   None, ["--version"], "no archive; executable hash recorded"),
}


# ----------------------------------------------------------------------------- corpus
def seq(label, n):
    x = zlib.crc32(label.encode()) or 1
    out = []
    for _ in range(n):
        x = (x * 1103515245 + 12345) & 0x7FFFFFFF
        out.append("ACGT"[(x >> 16) & 3])
    return "".join(out)


def rec(name, s, w=60, desc=""):
    lines = [s] if w == 0 else [s[i:i + w] for i in range(0, len(s), w)]
    return "\n".join([">" + name + (" " + desc if desc else "")] + lines) + "\n"


def put(s, i, ch):
    return s[:i] + ch + s[i + 1:]


def build_corpus():
    """Return [(name, scope, expectation, rationale, bytes)]; order is the report order."""
    a, b = seq("a", 600), seq("b", 800)
    base = rec("ctg1", a) + rec("ctg2", b)
    iupac = a
    for pos, ch in ((100, "R"), (150, "Y"), (200, "K"), (250, "M"), (300, "S"), (350, "W"),
                    (400, "N"), (450, "B"), (480, "D"), (510, "H"), (540, "V")):
        iupac = put(iupac, pos, ch)
    soft = a[:100] + a[100:300].lower() + a[300:]
    chunks, i, k = [], 0, 0
    while i < 600:
        w = (60, 80, 45, 70)[k % 4]
        chunks.append(a[i:i + w])
        i, k = i + w, k + 1
    nn = "".join("N" if j % 5 in (1, 2, 3) else a[j] for j in range(600))
    c = []

    def add(name, scope, expect, why, data):
        c.append((name, scope, expect, why, data.encode() if isinstance(data, str) else data))

    add("valid_simple", "control", "valid", "F: '>' SeqID line plus IUPAC lines; 2 records, 600/800 nt, wrap 60", base)
    add("valid_single_line", "control", "valid", "F: sequence 'can contain returns', does not require them", rec("ctg1", a, 0) + rec("ctg2", b, 0))
    add("valid_softmasked", "control", "valid", "soft-masking is ubiquitous; F and G are silent on case, so no basis to reject", rec("ctg1", soft) + rec("ctg2", b))
    add("valid_iupac", "control", "valid", "F: 'only use IUPAC symbols'; R,Y,K,M,S,W,N,B,D,H,V all are, one each in 600 nt", rec("ctg1", iupac) + rec("ctg2", b))
    add("valid_no_final_newline", "control", "valid", "missing last newline is legal text; robustness check", base.rstrip("\n"))
    add("empty_file", "syntax", "error", "F/G: a file is a set of '>' records; zero bytes contains no sequence", b"")
    add("seq_before_header", "syntax", "error", "F: the definition line must precede the sequence", seq("pre", 60) + "\n" + base)
    add("empty_header", "syntax", "error", "F: '>' must be followed by a SeqID; a nameless record cannot be referenced", rec("ctg1", a) + ">\n" + "\n".join(wrap_(b)) + "\n")
    add("empty_record", "syntax", "error", "F: a definition line is followed by its sequence; zero residues defines nothing", rec("ctg1", a) + ">ctg2_empty\n" + rec("ctg3", b))
    add("dup_id_identical_header", "identifier", "error", "F: 'SeqID must be unique'; G: 'unique within a genome'", rec("ctg1", a, desc="isolate X") + rec("ctg1", b, desc="isolate X"))
    add("dup_first_token_id", "identifier", "error", "F/G: SeqID is the first token; differing descriptions do not make it unique", rec("ctg1", a, desc="isolate A") + rec("ctg1", b, desc="isolate B"))
    add("dup_sequence_diff_ids", "content", "advisory", "legal, but byte-identical records under two IDs usually signal a merge error", rec("ctg1", a) + rec("ctg2", a))
    add("digit_in_sequence", "syntax", "error", "F: 'only use IUPAC symbols'; a digit is not one", rec("ctg1", put(a, 300, "7")))
    add("stop_star_in_sequence", "syntax", "error", "F: '*' is a protein stop symbol, not an IUPAC nucleotide symbol", rec("ctg1", put(a, 300, "*")))
    add("gap_dash_in_sequence", "syntax", "advisory", "F: outside alignments '-' 'will be stripped'; not rejected, but silently altered", rec("ctg1", put(a, 300, "-")))
    add("non_iupac_letter_x", "syntax", "error", "F: 'only use IUPAC symbols'; X is not a nucleotide code", rec("ctg1", put(a, 300, "X")))
    add("space_in_sequence", "syntax", "advisory", "not IUPAC, but many parsers strip blanks, so reasonable tools differ", rec("ctg1", put(a, 30, " ")))
    add("blank_line_in_record", "syntax", "advisory", "F/G silent; some parsers skip blanks, indexers (faidx) object", ">ctg1\n" + "\n".join(wrap_(a)[:2] + [""] + wrap_(a)[2:]) + "\n")
    add("crlf_line_endings", "syntax", "advisory", "F/G silent; tolerated by most parsers, but stray CR leaks into IDs of some tools", base.replace("\n", "\r\n"))
    add("inconsistent_wrapping", "syntax", "advisory", "F: returns inside sequence are allowed (<=80 recommended); only line-indexed tools object", ">ctg1\n" + "\n".join(chunks) + "\n")
    add("utf8_bom", "syntax", "advisory", "BOM makes line 1 not start with '>' for byte-exact readers; common Windows artefact", b"\xef\xbb\xbf" + base.encode())
    add("high_n_record", "content", "advisory", "valid FASTA; 60% scattered N is a quality signal, not a format error", rec("ctg1", nn))
    add("all_n_record", "content", "error", "no residue information; also violates G 'remove any Ns from the beginning or end'", rec("ctg1", "N" * 600))
    add("tiny_contig_50", "content", "advisory", "G: 'Contigs should be >199nt' is a recommendation; dropping short contigs is a QC choice", rec("ctg1", a) + rec("tiny", seq("t", 50)))
    add("seqid_len_49", "ncbi", "valid", "G: SeqIDs must be <50 characters (F says <=25 for general GenBank; G governs genomes)", rec("ctg_" + "x" * 45, a))
    add("seqid_len_50", "ncbi", "error", "G: SeqIDs must be <50 characters", rec("ctg_" + "x" * 46, a))
    add("seqid_disallowed_char", "ncbi", "error", "F/G: SeqID 'can only include' letters, digits, - _ . : * #; '@' is not allowed", rec("ctg@1", a))
    add("contig_len_199", "ncbi", "advisory", "G: 'Contigs should be >199nt' (wording is a recommendation, so not scored)", rec("ctg1", seq("c", 199)))
    add("contig_len_200", "ncbi", "valid", "G: >199 nt satisfied exactly", rec("ctg1", seq("c", 200)))
    add("terminal_ns", "ncbi", "error", "G: 'Remove any Ns from the beginning or end of each sequence'", rec("ctg1", "N" * 10 + a[10:590] + "N" * 10))
    return c


def wrap_(s, w=60):
    return [s[i:i + w] for i in range(0, len(s), w)]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def corpus_sha(corpus):
    return sha("".join("%s\t%s\n" % (n, sha(d)) for n, _, _, _, d in corpus).encode())


# ----------------------------------------------------------------------------- runners
def sh(cmd, cwd, timeout, env=None):
    try:
        p = subprocess.run([str(x) for x in cmd], cwd=str(cwd), env=env, stdin=subprocess.DEVNULL,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout,
                           text=True, errors="replace")
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        return None, "", "timeout after %gs (process killed)" % timeout
    except OSError as e:
        return None, "", "cannot execute: %s" % e.strerror


def crashed(rc, err):
    return rc is None or rc < 0 or "panic:" in err or "goroutine " in err or "Traceback" in err


def run_pyfv(exe, path, wd, to):
    rc, out, err = sh([exe, "-f", path], wd, to, dict(os.environ, PYTHONWARNINGS="ignore"))
    msg = err or out or "valid"
    if crashed(rc, err) or rc not in (0, 1, 2, 4):
        return "crash", "crash", rc, "crash or undocumented exit: " + msg
    flag = "no" if rc == 0 else "yes"
    return flag, flag, rc, msg


def run_falint(exe, path, wd, to):
    rc, out, err = sh([exe, "-fasta", path], wd, to)
    if crashed(rc, err):
        return "crash", "crash", rc, err or out
    m = re.search(r'msg="?([^"]*)"?', err)
    flag = "yes" if rc != 0 or BAD_FALINT.search(err) else "no"
    return flag, flag, rc, (m.group(1) if m else (err or out))


def run_seqkit(exe, path, wd, to):
    rc, out, err = sh([exe, "seq", "-v", "-t", "dna", path, "-o", os.devnull], wd, to)
    err = ANSI.sub("", err)
    if crashed(rc, err):
        return "crash", "crash", rc, err
    flag = "yes" if rc != 0 or "[ERRO]" in err or "[WARN]" in err else "no"
    return flag, flag, rc, err or "no message"


def run_fg(exe, gate, path, wd, to):
    wd.mkdir(parents=True, exist_ok=True)
    for stale in ("fg.json", "fg.html", "fg.tsv", "fg_mqc.json"):
        (wd / stale).unlink(missing_ok=True)  # never parse a report left by an earlier run
    cmd = [exe, path, "--profile", "assembly"] + gate + [
        "--json", wd / "fg.json", "--out", wd / "fg.html", "--tsv", wd / "fg.tsv",
        "--multiqc", wd / "fg_mqc.json"]
    rc, out, err = sh(cmd, wd, to)
    if rc != 0:  # FastaGuard exits 0 for PASS, WARN and FAIL once the reports are written
        return "crash", "crash", rc, "fastaguard exit %s: %s" % (rc, (err or out).strip())
    try:
        rep = json.loads((wd / "fg.json").read_text())
        g = rep["gate"]
        verdict, blocked = rep["verdict"]["status"], not g["can_continue"]
    except (OSError, ValueError, KeyError, TypeError):
        return "crash", "crash", rc, "no usable JSON report: " + (err or out)
    det = "verdict=%s gate=%s can_continue=%s blocking=%s advisory=%s" % (
        verdict, g["status"], str(g["can_continue"]).lower(), ",".join(g["blocking_findings"]) or "none",
        ",".join(g["advisory_findings"]) or "none")
    return ("yes" if verdict != "PASS" else "no"), ("yes" if blocked else "no"), rc, det


class Table2asn:
    def __init__(self, exe, wd, to):
        self.exe, self.wd, self.to, self.baseline = exe, wd, to, set()
        self.template = wd / "table2asn-validation-template.sbt"
        self.template.write_text(T2A_TEMPLATE)

    def parse_val(self, d, stem):
        msgs = []
        val, stats = d / (stem + ".val"), d / (stem + ".stats")
        if val.is_file():
            for line in val.read_text(errors="replace").splitlines():
                m = VAL_RE.match(line)
                if m:
                    msgs.append((m.group(1).lower(), m.group(2), m.group(3)))
            return msgs, True
        if stats.is_file():
            txt = stats.read_text(errors="replace")
            for n, lvl in re.findall(r"(\d+)\s+([A-Za-z]+)-level", txt):
                if int(n) and lvl.lower() in ERR_LEVELS:
                    msgs.append((lvl.lower(), "stats", "%s %s-level message(s)" % (n, lvl)))
            for code in re.findall(r"^(\w+\.\w+):\s+\d+", txt, re.M):
                msgs.append(("warning", code, "(from .stats)"))
            return msgs, True
        return msgs, False

    def __call__(self, path, wd):
        d = self.wd / "table2asn" / wd.name
        d.mkdir(parents=True, exist_ok=True)
        name = path.name[:-3] if path.name.endswith(".gz") else path.name
        inp = d / name
        if path.name.endswith(".gz"):
            import gzip
            with gzip.open(path, "rb") as src, open(inp, "wb") as dst:
                shutil.copyfileobj(src, dst)
        else:
            shutil.copyfile(path, inp)
        rc, out, err = sh([self.exe, "-t", self.template, "-i", inp] + T2A_ARGS, d, self.to)
        console = ANSI.sub("", out + "\n" + err)
        if crashed(rc, err):
            return "crash", "crash", rc, err
        msgs, have = self.parse_val(d, inp.stem if inp.suffix else inp.name)
        errs = [m for m in msgs if m[0] in ERR_LEVELS]
        cerr = ERR_RE.search(console) is not None
        problem = " ".join(re.findall(r"Problem:\s*(.*)", console))
        if cerr and "Cannot open" in problem:
            return "crash", "crash", rc, "console: " + problem
        if cerr:
            return "yes", "yes", rc, "console Error: " + (problem or console)
        if errs:
            return "yes", "yes", rc, "; ".join("%s [%s] %s" % (m[0], m[1], m[2][:70]) for m in errs[:3])
        cwarn = problem if re.search(r"(?i)(?:^|[\s\[])WARNING\s*:", console) else ""
        new = [m for m in msgs if m[0].startswith("warn") and m[1] not in self.baseline]
        if (rc != 0 or not have) and not (new or cwarn):
            return "crash", "crash", rc, "no error evidence for non-zero exit / no artifact: " + console
        if new or cwarn:
            det = ["warning [%s] %s" % (m[1], m[2][:70]) for m in new[:3]]
            return "yes", "no", rc, "; ".join(det + (["console Warning: " + cwarn] if cwarn else []))
        return "no", "no", rc, "only baseline warnings (%d)" % len(msgs)

    def set_baseline(self, path, wd):
        self(path, wd)
        d = self.wd / "table2asn" / wd.name
        msgs, _ = self.parse_val(d, path.stem)
        self.baseline = {m[1] for m in msgs if m[0].startswith("warn")}


# ----------------------------------------------------------------------------- reporting
def cell(row):
    f = row["flagged"]
    return ("yes" if row["strict"] == "yes" else "warn") if f == "yes" else f


def tally(rows, tool, key, scope):
    t = {}
    for e in ("error", "valid", "advisory"):
        t.update({e + "_flagged": 0, e + "_total": 0, e + "_crash": 0})
    for r in rows:
        if r["tool"] != tool or r[key] == "n.a." or r["expectation"] not in ("error", "valid", "advisory"):
            continue
        if scope != "all" and r["scope"] != scope:
            continue
        t[r["expectation"] + "_total"] += 1
        t[r["expectation"] + "_flagged"] += r[key] == "yes"
        t[r["expectation"] + "_crash"] += r[key] == "crash"
    return t


def clean(v, maps=()):
    s = ANSI.sub("", str(v))
    for a, b in maps:
        s = s.replace(a, b)
    return " ".join(s.split())[:260].strip() or "-"


PREAMBLE = [
    "# NOTE: the corpus is author-constructed and favours FastaGuard's own check list.",
    "# Expectations (valid/error/advisory) were fixed before any tool ran; advisory rows are not scored.",
    "# The tools were not designed for the same purpose. flagged: yes|no|crash|n.a.;"
    " strict: blocked / error-level.",
]


def write_tsv(path, header, rows, preamble=()):
    lines = list(preamble) + ["\t".join(header)]
    lines += ["\t".join(clean(x) for x in r) for r in rows]
    path.write_text("\n".join(lines) + "\n")


def summary(rows, real_rows, tools_run):
    out = ["# Author-constructed corpus favouring FastaGuard's check list; tools differ in purpose.",
           "# block: scores (flagged = any report; strict = FastaGuard blocked / table2asn error-level)",
           "\t".join(["tool", "definition", "scope", "error_flagged", "error_total", "error_crash",
                      "valid_flagged", "valid_total", "valid_crash", "advisory_flagged",
                      "advisory_total", "advisory_crash"])]
    for tool in tools_run:
        for key in ("flagged", "strict") if tool in TWO_LEVEL else ("flagged",):
            for scope in ("all", "control", "syntax", "identifier", "content", "ncbi"):
                t = tally(rows, tool, key, scope)
                out.append("\t".join([tool, key, scope] + [str(t[k + s]) for k in ("error", "valid", "advisory")
                                                           for s in ("_flagged", "_total", "_crash")]))
    out += ["# block: capability_matrix (yes = flagged and blocked/error-level or single-level tool;"
            " warn = flagged below block/error level; no; crash; n.a.)",
            "\t".join(["file", "scope", "expectation"] + TOOLS)]
    for key in dict.fromkeys(r["file"] for r in rows):
        by = {r["tool"]: r for r in rows if r["file"] == key}
        first = by[TOOLS[0]]
        out.append("\t".join([key, first["scope"], first["expectation"]] + [cell(by[t]) for t in TOOLS]))
    if real_rows:
        out += ["# block: real_files (no expectation; see real_results.tsv for detail)",
                "\t".join(["file"] + TOOLS)]
        for key in dict.fromkeys(r["file"] for r in real_rows):
            by = {r["tool"]: r for r in real_rows if r["file"] == key}
            out.append("\t".join([key] + [cell(by[t]) for t in TOOLS]))
    return "\n".join(out) + "\n"


def tool_version(exe, args, maps):
    rc, out, err = sh([exe] + args, exe.parent, 30, dict(os.environ, PYTHONWARNINGS="ignore"))
    text = ANSI.sub("", (out or err).strip().splitlines()[0] if (out or err).strip() else "unknown")
    m = re.search(r"version\s+(\S+)", text)
    return clean(m.group(1) if m else text, maps)


def provenance(paths, archives, maps):
    found = {}
    if archives:
        for p in Path(archives).rglob("*"):
            if p.is_file():
                found.setdefault(p.name, []).append(p)
    entries = []
    for tool, (url, digest, vargs, note) in SOURCES.items():
        exe = paths.get(tool)
        verified = None
        if digest:
            hits = found.get(url.rsplit("/", 1)[-1], [])
            verified = (sha(hits[0].read_bytes()) == digest) if hits else None
        entries.append({
            "tool": tool, "version": tool_version(exe, vargs, maps) if exe else "not installed",
            "source_url": url, "archive_sha256_pinned": digest, "archive_sha256_verified": verified,
            "executable_sha256": sha(exe.read_bytes()) if exe else None, "notes": note})
    return entries


def resolve(flag, env, name):
    cand = flag or os.environ.get(env) or shutil.which(name)
    p = Path(cand).expanduser() if cand else None
    return p.resolve() if p and p.is_file() and os.access(p, os.X_OK) else None


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--corpus-dir", type=Path, required=True)
    ap.add_argument("--out-dir", type=Path, default=Path(__file__).resolve().parent)
    ap.add_argument("--work-dir", type=Path)
    ap.add_argument("--timeout", type=float, default=60.0)
    ap.add_argument("--archives", type=Path)
    ap.add_argument("--generate-only", action="store_true")
    ap.add_argument("--real", nargs="+", default=[], metavar="[LABEL=]FASTA")
    ap.add_argument("--real-timeout", type=float, default=600.0)
    for flag in ("fastaguard", "py-fasta-validator", "fa-lint", "table2asn", "seqkit"):
        ap.add_argument("--" + flag, type=Path)
    args = ap.parse_args()

    corpus = build_corpus()
    cdir = args.corpus_dir.resolve()
    cdir.mkdir(parents=True, exist_ok=True)
    for name, _, _, _, data in corpus:
        (cdir / (name + ".fa")).write_bytes(data)
    if args.generate_only:
        print("# corpus_sha256\t" + corpus_sha(corpus))
        for name, scope, expect, why, data in corpus:
            print("\t".join([name, scope, expect, sha(data)[:16], why]))
        return 0

    paths = {"py_fasta_validator": resolve(args.py_fasta_validator, "PY_FASTA_VALIDATOR", "py_fasta_validator"),
             "fa-lint": resolve(args.fa_lint, "FA_LINT", "fa-lint"),
             "table2asn": resolve(args.table2asn, "TABLE2ASN", "table2asn"),
             "seqkit": resolve(args.seqkit, "SEQKIT", "seqkit"),
             "fastaguard": resolve(args.fastaguard, "FASTAGUARD", "fastaguard")}
    tmp = None if args.work_dir else tempfile.mkdtemp(prefix="fg-compare-")
    work = (args.work_dir or Path(tmp)).resolve()
    work.mkdir(parents=True, exist_ok=True)
    maps = [(str(work), "$WORK"), (str(cdir), "$CORPUS")] + [
        (str(p.parent), "$TOOLDIR") for p in paths.values() if p]
    to = [args.timeout]  # one-element list so the real-file phase can raise it
    t2a = Table2asn(paths["table2asn"], work, to[0]) if paths["table2asn"] else None
    runners = {
        "py_fasta_validator": paths["py_fasta_validator"] and (lambda p, w: run_pyfv(paths["py_fasta_validator"], p, w, to[0])),
        "fa-lint": paths["fa-lint"] and (lambda p, w: run_falint(paths["fa-lint"], p, w, to[0])),
        "table2asn": t2a,
        "seqkit": paths["seqkit"] and (lambda p, w: run_seqkit(paths["seqkit"], p, w, to[0])),
        "fastaguard_pipeline": paths["fastaguard"] and (lambda p, w: run_fg(
            paths["fastaguard"], ["--gate", "pipeline"], p, w / "fgp", to[0])),
        "fastaguard_ncbi": paths["fastaguard"] and (lambda p, w: run_fg(
            paths["fastaguard"], ["--gate", "submission", "--submission-target", "ncbi"], p, w / "fgn", to[0])),
    }
    if t2a:
        wd = work / "case" / "valid_simple"
        wd.mkdir(parents=True, exist_ok=True)
        t2a.set_baseline(cdir / "valid_simple.fa", wd)

    def run_all(label, scope, expect, path, extra=()):
        wd = work / "case" / label
        wd.mkdir(parents=True, exist_ok=True)
        res = []
        for tool in TOOLS:
            fn = runners[tool]
            f, s, rc, det = fn(path, wd) if fn else ("n.a.", "n.a.", None, "tool not found")
            res.append({"file": label, "scope": scope, "expectation": expect, "tool": tool,
                        "flagged": f, "strict": s, "exit_code": "-" if rc is None else rc,
                        "detail": clean(det, maps + list(extra))})
        return res

    rows = []
    for name, scope, expect, _, _ in corpus:
        rows += run_all(name, scope, expect, cdir / (name + ".fa"))
    real_rows = []
    to[0] = args.real_timeout
    if t2a:
        t2a.to = to[0]
    seen = set()
    for spec in args.real:
        label, sep, rest = spec.partition("=")
        p = Path(rest if sep else spec)
        label = label if sep else p.name
        if label in seen:
            sys.exit("duplicate --real label %r: use LABEL=PATH to make labels unique" % label)
        seen.add(label)
        real_rows += run_all(label, "real", "n/a", p.resolve(), [(str(p.resolve().parent), "$REAL")])

    cols = ["file", "scope", "expectation", "tool", "flagged", "strict", "exit_code", "detail"]
    out = args.out_dir
    out.mkdir(parents=True, exist_ok=True)
    write_tsv(out / "results.tsv", cols, [[r[c] for c in cols] for r in rows], PREAMBLE)
    if real_rows:
        write_tsv(out / "real_results.tsv", cols, [[r[c] for c in cols] for r in real_rows],
                  ["# Real files: no expectation, nothing scored. Same decision rules as results.tsv."])
    tools_run = [t for t in TOOLS if runners[t]]
    (out / "summary.tsv").write_text(summary(rows, real_rows, tools_run))
    info = {"note": "Author-constructed corpus favouring FastaGuard's check list; tools differ in purpose.",
            "corpus_sha256": corpus_sha(corpus), "corpus_files": len(corpus),
            "platform": "%s %s" % (platform.system(), platform.machine()),
            "tools": provenance(paths, args.archives, maps)}
    (out / "tool_versions.json").write_text(json.dumps(info, indent=2, sort_keys=True) + "\n")
    if tmp:
        shutil.rmtree(tmp, ignore_errors=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
