#!/usr/bin/env bash
set -euo pipefail

target="${1:?target triple is required}"
version="${2:-${GITHUB_REF_NAME:-dev}}"
binary_path="${3:-}"
dist_dir="${4:-dist}"
name="fastaguard-${version}-${target}"
staging_dir="${dist_dir}/${name}"

if [[ -z "${binary_path}" ]]; then
  binary_path="target/${target}/release/fastaguard"
  if [[ ! -x "${binary_path}" && -x "target/release/fastaguard" ]]; then
    host_target="$(rustc -vV | sed -n 's/^host: //p')"
    if [[ -z "${host_target}" ]]; then
      echo "could not detect the host target with rustc -vV" >&2
      exit 1
    fi
    if [[ "${target}" != "${host_target}" ]]; then
      echo "missing target-specific release binary: ${binary_path}; refusing to package" \
        "target/release/fastaguard for requested target ${target} (detected host ${host_target});" \
        "build with --target ${target} or pass its binary path as the third argument" >&2
      exit 1
    fi
    binary_path="target/release/fastaguard"
  fi
fi

if [[ ! -x "${binary_path}" ]]; then
  echo "missing executable binary: ${binary_path}" >&2
  exit 1
fi

rm -rf "${staging_dir}"
mkdir -p "${staging_dir}"
cp "${binary_path}" "${staging_dir}/fastaguard"
cp README.md "${staging_dir}/README.md"
cp LICENSE "${staging_dir}/LICENSE"
cp -R schema "${staging_dir}/schema"

COPYFILE_DISABLE=1 tar -C "${dist_dir}" -czf "${dist_dir}/${name}.tar.gz" "${name}"
rm -rf "${staging_dir}"

echo "${dist_dir}/${name}.tar.gz"
