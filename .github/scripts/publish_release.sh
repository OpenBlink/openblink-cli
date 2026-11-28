#!/usr/bin/env bash
set -euo pipefail

flags=(--verify-tag --generate-notes)
if [[ "${GITHUB_REF_NAME%%+*}" == *-* ]]; then
  flags+=(--prerelease --latest=false)
fi

assets=(dist/openblink-cli-*.tar.gz dist/openblink-cli-*.zip dist/checksums-sha256.txt)
gh release create "$GITHUB_REF_NAME" "${flags[@]}" "${assets[@]}"
