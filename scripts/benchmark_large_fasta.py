#!/usr/bin/env python3
"""Generate a deterministic FASTA and benchmark a FastaGuard binary."""

from __future__ import annotations

import argparse
import json
import subprocess
import time
from pathlib import Path


BASES = "ACGT"
LINE_WIDTH = 80
INDEX_TAG_WIDTH = 32


def main() -> int:
    args = parse_args()
    ensure_unique_capacity(args.records, args.length)
    binary = args.binary.resolve()
    out_dir = args.out_dir.resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    if not binary.exists():
        raise SystemExit(
            f"FastaGuard binary not found at {binary}. "
            "Run `cargo build --release --locked` or pass --binary."
        )

    fasta_path = out_dir / "synthetic.fa"
    json_path = out_dir / "fastaguard.json"
    html_path = out_dir / "fastaguard_report.html"
    tsv_path = out_dir / "fastaguard.tsv"
    multiqc_path = out_dir / "fastaguard_multiqc.json"

    generated_bytes = write_fasta(fasta_path, args.records, args.length)
    command = [
        str(binary),
        str(fasta_path),
        "--profile",
        "assembly",
        "--min-contig-length",
        "1",
        "--out",
        str(html_path),
        "--json",
        str(json_path),
        "--tsv",
        str(tsv_path),
        "--multiqc",
        str(multiqc_path),
    ]

    started = time.perf_counter()
    completed = subprocess.run(command, capture_output=True, text=True, check=False)
    elapsed = time.perf_counter() - started

    if completed.returncode != 0:
        raise SystemExit(
            "FastaGuard benchmark command failed with exit code "
            f"{completed.returncode}\nSTDOUT:\n{completed.stdout}\nSTDERR:\n{completed.stderr}"
        )

    report = json.loads(json_path.read_text())
    summary = {
        "records": args.records,
        "length_per_record": args.length,
        "total_bases": args.records * args.length,
        "fasta_bytes": generated_bytes,
        "elapsed_seconds": round(elapsed, 4),
        "bases_per_second": round((args.records * args.length) / elapsed, 2)
        if elapsed > 0
        else None,
        "exit_code": completed.returncode,
        "verdict": report["verdict"]["status"],
        "reported_total_length": report["summary"]["total_length"],
        "fasta_removed_after_run": not args.keep_fasta,
        "outputs": {
            "fasta": str(fasta_path) if args.keep_fasta else None,
            "json": str(json_path),
            "html": str(html_path),
            "tsv": str(tsv_path),
            "multiqc": str(multiqc_path),
        },
    }

    print(json.dumps(summary, indent=2, sort_keys=True))

    if not args.keep_fasta:
        fasta_path.unlink()

    return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Benchmark FastaGuard on a deterministic synthetic FASTA."
    )
    parser.add_argument(
        "--records",
        type=positive_int,
        default=10_000,
        help="Number of FASTA records to generate.",
    )
    parser.add_argument(
        "--length",
        type=positive_int,
        default=1_000,
        help="Bases per FASTA record.",
    )
    parser.add_argument(
        "--binary",
        type=Path,
        default=Path("target/release/fastaguard"),
        help="Path to the FastaGuard binary to benchmark.",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("target/benchmarks/large-fasta"),
        help="Directory for synthetic inputs and benchmark outputs.",
    )
    parser.add_argument(
        "--keep-fasta",
        action="store_true",
        help="Keep the generated synthetic FASTA after the benchmark.",
    )
    return parser.parse_args()


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be greater than zero")
    return parsed


def write_fasta(path: Path, records: int, length: int) -> int:
    with path.open("w", encoding="utf-8", newline="\n") as handle:
        for record_index in range(records):
            handle.write(f">synthetic_{record_index:08d}\n")
            write_sequence(handle, record_index, length)

    return path.stat().st_size


def write_sequence(handle, record_index: int, length: int) -> None:
    written = 0
    while written < length:
        chunk_length = min(LINE_WIDTH, length - written)
        chunk = "".join(
            synthetic_base(record_index, written + offset)
            for offset in range(chunk_length)
        )
        handle.write(chunk)
        handle.write("\n")
        written += chunk_length


def synthetic_base(record_index: int, position: int) -> str:
    if position < INDEX_TAG_WIDTH:
        return BASES[(record_index >> (position * 2)) & 0b11]

    return BASES[(record_index + position) % len(BASES)]


def ensure_unique_capacity(records: int, length: int) -> None:
    tag_width = min(length, INDEX_TAG_WIDTH)
    unique_capacity = len(BASES) ** tag_width
    if records > unique_capacity:
        raise SystemExit(
            f"Cannot generate {records} unique records with length {length}. "
            f"Increase --length or reduce --records below {unique_capacity}."
        )


if __name__ == "__main__":
    raise SystemExit(main())
