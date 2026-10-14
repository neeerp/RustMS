#!/usr/bin/env bash
# Put the original Maple Island into the v83 Map.nx and String.nx.
#
# usage: tools/graft-original-island.sh PATH/TO/V68 [GAME_DATA_DIR]
#
# v83 GMS already ships the revamped island. The original one, as in 2009
# footage, is in GMS v68 (April 2009), whose Map.wz and String.wz must be in
# PATH/TO/V68. This converts them (see tools/build-wztonx.sh) and copies the
# island's 12 changed maps, its world map, and their names over the v83 ones
# with tools/nx_graft.py. Every other map, and every tile, object, and
# background these maps use, is identical in both versions.
#
# GAME_DATA_DIR (default assets/game-data) must hold the v83 Map.nx and
# String.nx. They are replaced by the grafted files, which SHA256SUMS pins;
# the client loads the same files.
set -euo pipefail

V68_MAP_WZ_SHA256=924b2c4c2a4fa443a7a933e481aa89ed05172aeb629fb44c91bd4fd7eab61cf1
V68_STRING_WZ_SHA256=7bfeb8e84440601a8d8f2a6e12fbe0ab334513250bf0cd4a49229bea29f21587
V83_MAP_NX_SHA256=564a8f594cd609bf5796fb79af0f9496fc1adc54528274b1b390d6fecf264800
V83_STRING_NX_SHA256=ade1abe7b9af89634349aca0814d0ed80dc4e8e446f8ff73f5b86a93de94abfa
ISLAND_MAP_NX_SHA256=ac45984e2e3755273897f6c8ef69537944d05e92381b89164aeba08a93441b8a
ISLAND_STRING_NX_SHA256=50da85783aeadc36c885d257b4a3f8ad9a8072c63f27534f18ce0e0d752588e5

MAPS=(0 10000 20000 30000 40000 50000 1000000 1000001 1000002 1000003 1010000 1020000)

if [[ $# -lt 1 || $# -gt 2 ]]; then
    echo "usage: $0 PATH/TO/V68 [GAME_DATA_DIR]" >&2
    exit 2
fi
tools=$(cd "$(dirname "$0")" && pwd)
v68=$(realpath "$1")
data=$(realpath "${2:-$tools/../assets/game-data}")

check() {
    if ! echo "$1  $2" | sha256sum --check --status; then
        echo "$2 does not have the expected checksum" >&2
        exit 1
    fi
}
check "$V68_MAP_WZ_SHA256" "$v68/Map.wz"
check "$V68_STRING_WZ_SHA256" "$v68/String.wz"
check "$V83_MAP_NX_SHA256" "$data/Map.nx"
check "$V83_STRING_NX_SHA256" "$data/String.nx"

# The files are large, and /tmp is often a small tmpfs.
work=$(mktemp -d -p "$data" .graft.XXXXXX)
trap 'rm -rf "$work"' EXIT

"$tools/build-wztonx.sh" "$work"
mkdir "$work/v68"
cp "$v68/Map.wz" "$v68/String.wz" "$work/v68/"
(cd "$work/v68" && ../wztonx Map.wz String.wz -c >/dev/null)

map_paths=()
string_paths=()
for id in "${MAPS[@]}"; do
    map_paths+=("Map/Map0/$(printf %09d "$id").img")
    string_paths+=("Map.img/maple/$id")
done
python3 "$tools/nx_graft.py" "$data/Map.nx" "$work/v68/Map.nx" "$work/Map.nx" \
    "${map_paths[@]}" \
    WorldMap/WorldMap000.img=WorldMap/WorldMap00.img \
    WorldMap/WorldMap.img/MapList/0
python3 "$tools/nx_graft.py" "$data/String.nx" "$work/v68/String.nx" "$work/String.nx" \
    "${string_paths[@]}"

check "$ISLAND_MAP_NX_SHA256" "$work/Map.nx"
check "$ISLAND_STRING_NX_SHA256" "$work/String.nx"
mv "$work/Map.nx" "$work/String.nx" "$data/"
