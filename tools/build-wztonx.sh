#!/usr/bin/env bash
# Build NoLifeWzToNx for Linux.
#
# usage: tools/build-wztonx.sh DIR
#
# Clones the converter at a pinned commit into DIR/src, applies two fixes for
# Linux, and writes the binary to DIR/wztonx. Needs git, g++, zlib, lz4 and
# libsquish.
set -euo pipefail

WZTONX_REPO=https://github.com/ryantpayton/NoLifeWzToNx
WZTONX_COMMIT=73bddde

if [[ $# -ne 1 ]]; then
    echo "usage: $0 DIR" >&2
    exit 2
fi
dir=$1

git clone --quiet "$WZTONX_REPO" "$dir/src"
git -C "$dir/src" checkout --quiet "$WZTONX_COMMIT"

# The converter targets MSVC: it uses std::experimental::filesystem, and it
# treats UTF-16 strings as wchar_t, which is 4 bytes on Linux and makes every
# string conversion throw.
src="$dir/src/NoLifeWzToNx.cpp"
sed -i 's/std::experimental::filesystem/std::filesystem/' "$src"
sed -i 's/codecvt_utf8_utf16<wchar_t>, wchar_t>/codecvt_utf8_utf16<char16_t>, char16_t>/' "$src"
sed -i '/auto ptr = reinterpret_cast<wchar_t const \*>(p_str.c_str());/d' "$src"
sed -i 's/convert.to_bytes(ptr, ptr + p_str.size())/convert.to_bytes(p_str.data(), p_str.data() + p_str.size())/' "$src"

g++ -O2 -std=c++17 -w -o "$dir/wztonx" "$src" "$dir/src/Keys.cpp" -lz -llz4 -lsquish
