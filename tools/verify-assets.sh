#!/usr/bin/env bash
# Verify the server's NX game data against the pinned checksums.
set -euo pipefail
cd "$(dirname "$0")/../assets/game-data"
missing=0
while read -r _ file; do
    if [[ ! -e "$file" ]]; then
        echo "missing: assets/game-data/$file" >&2
        missing=1
    fi
done < SHA256SUMS
if [[ $missing -ne 0 ]]; then
    exit 1
fi
sha256sum --check --strict SHA256SUMS
