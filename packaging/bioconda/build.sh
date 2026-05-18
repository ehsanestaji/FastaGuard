#!/usr/bin/env bash
set -euo pipefail

cargo-bundle-licenses --format yaml --output THIRDPARTY.yml
cargo install -v --locked --no-track --root "${PREFIX}" --path .
rm -f "${PREFIX}/.crates.toml" "${PREFIX}/.crates2.json"

install -Dm644 schema/fastaguard.schema.json \
  "${PREFIX}/share/${PKG_NAME}/schema/fastaguard.schema.json"
install -Dm644 schema/finding-catalog.json \
  "${PREFIX}/share/${PKG_NAME}/schema/finding-catalog.json"
