#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -gt 2 ]; then
    printf 'usage: quickstart.sh [FASTAGUARD_EXECUTABLE] [OUTPUT_DIRECTORY]\n' >&2
    exit 2
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
binary=${1:-fastaguard}
outdir=${2:-target/reference-quickstart}
fixtures="$script_dir/fixtures"
helper="$script_dir/../workflows/check_fastaguard_gate.py"

"$binary" reference "$fixtures/reference.fa" \
    --dict "$fixtures/reference.dict" --require dict --policy coordinate \
    --outdir "$outdir/pass" --prefix reference --write-lock

"$binary" reference "$fixtures/reference.fa" \
    --dict "$fixtures/copied-length-mismatch.dict" --require dict --policy coordinate \
    --outdir "$outdir/mismatch" --prefix reference

python3 "$helper" "$outdir/pass/reference.fastaguard.json"

# The negative example must be blocked without losing its completed reports.
if python3 "$helper" "$outdir/mismatch/reference.fastaguard.json"; then
    printf 'The deliberately mismatched dictionary was unexpectedly accepted.\n' >&2
    exit 1
else
    status=$?
    if [ "$status" -ne 2 ]; then
        exit "$status"
    fi
fi

printf 'Matching reference accepted; copied length mismatch blocked. Reports: %s\n' "$outdir"
