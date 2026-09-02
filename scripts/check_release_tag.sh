#!/usr/bin/env bash
set -euo pipefail

release_tag="${1:?release tag is required}"
package_id="$(cargo pkgid --quiet)"
package_version="${package_id##*@}"
expected_tag="v${package_version}"

if [[ "${release_tag}" != "${expected_tag}" ]]; then
  echo "release tag ${release_tag} does not match package version ${expected_tag}" >&2
  exit 1
fi

echo "release tag ${release_tag} matches package version ${package_version}"
