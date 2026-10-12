#!/usr/bin/env bash
# Convert a v83 GMS UI.wz into HeavenClient/nx/UIClassic.nx.
#
# usage: tools/convert-ui-classic.sh PATH/TO/UI.wz [OUT.nx]
#
# Builds NoLifeWzToNx at a pinned commit with two fixes for Linux, converts the
# file in a scratch directory, and checks both files against the checksums the
# client was written against. Needs git, g++, zlib, lz4 and libsquish.
set -euo pipefail

WZTONX_REPO=https://github.com/ryantpayton/NoLifeWzToNx
WZTONX_COMMIT=73bddde
UI_WZ_SHA256=d253934d986cd428ba914c37f1bfae18513e2830ec3ec5d0e055fd2cef05e47c
UI_CLASSIC_SHA256=d2e0e1ce6f1864e99ad6fdd1017e3495e72802744f68e9ffad54e61794b4d98a

if [[ $# -lt 1 || $# -gt 2 ]]; then
    echo "usage: $0 PATH/TO/UI.wz [OUT.nx]" >&2
    exit 2
fi
wz=$(realpath "$1")
out=${2:-"$(cd "$(dirname "$0")/.." && pwd)/HeavenClient/nx/UIClassic.nx"}

if ! echo "$UI_WZ_SHA256  $wz" | sha256sum --check --status; then
    echo "warning: $wz is not the UI.wz this client was checked against" >&2
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

git clone --quiet "$WZTONX_REPO" "$work/src"
git -C "$work/src" checkout --quiet "$WZTONX_COMMIT"

# The converter targets MSVC: it uses std::experimental::filesystem, and it
# treats UTF-16 strings as wchar_t, which is 4 bytes on Linux and makes every
# string conversion throw.
src="$work/src/NoLifeWzToNx.cpp"
sed -i 's/std::experimental::filesystem/std::filesystem/' "$src"
sed -i 's/codecvt_utf8_utf16<wchar_t>, wchar_t>/codecvt_utf8_utf16<char16_t>, char16_t>/' "$src"
sed -i '/auto ptr = reinterpret_cast<wchar_t const \*>(p_str.c_str());/d' "$src"
sed -i 's/convert.to_bytes(ptr, ptr + p_str.size())/convert.to_bytes(p_str.data(), p_str.data() + p_str.size())/' "$src"

g++ -O2 -std=c++17 -w -o "$work/wztonx" "$src" "$work/src/Keys.cpp" -lz -llz4 -lsquish

cp "$wz" "$work/UI.wz"
(cd "$work" && ./wztonx UI.wz -c >/dev/null)

if ! echo "$UI_CLASSIC_SHA256  $work/UI.nx" | sha256sum --check --status; then
    echo "warning: the converted file does not match the pinned UIClassic.nx" >&2
fi

mkdir -p "$(dirname "$out")"
mv "$work/UI.nx" "$out"
sha256sum "$out"
