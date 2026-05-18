#!/usr/bin/env bash
set -euo pipefail

target="${1:?target triple is required}"
version="${2:-${GITHUB_REF_NAME:-dev}}"
binary_path="${3:-target/${target}/release/fastaguard}"
dist_dir="${4:-dist}"
name="fastaguard-${version}-${target}"
staging_dir="${dist_dir}/${name}"

if [[ ! -x "${binary_path}" ]]; then
  echo "missing executable binary: ${binary_path}" >&2
  exit 1
fi

rm -rf "${staging_dir}"
mkdir -p "${staging_dir}"
cp "${binary_path}" "${staging_dir}/fastaguard"
cp README.md "${staging_dir}/README.md"
cp -R schema "${staging_dir}/schema"

tar -C "${dist_dir}" -czf "${dist_dir}/${name}.tar.gz" "${name}"
rm -rf "${staging_dir}"

echo "${dist_dir}/${name}.tar.gz"
