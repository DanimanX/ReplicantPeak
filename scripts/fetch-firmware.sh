#!/usr/bin/env bash
# Fetch the Novation Peak update package into `firmware/`.
#
# The files are proprietary Focusrite/Novation content and are never committed
# to this repository, so this script is how a checkout obtains them. It mirrors
# what the Components updater does: the download endpoint answers with a signed
# redirect and needs no account, cookies or connected instrument.
#
# Every image is checked against the SHA-256 the engine knows it by, so a
# truncated or substituted download fails here instead of inside the plugin.
#
# Usage:
#   scripts/fetch-firmware.sh               # latest known pair (fw 2.2/2108 + fpga 2.1/469)
#   PEAK_FIRMWARE_DIR=/tmp/fw scripts/fetch-firmware.sh
#   scripts/fetch-firmware.sh --list        # print the Peak entries of the catalogue
#
# The pinned ids match `KNOWN_BUILDS` in crates/peak-core/src/firmware.rs. A
# newer release means bumping both the ids and the hashes here.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dest="${PEAK_FIRMWARE_DIR:-$repo_root/firmware}"
base="https://components.novationmusic.com/api/v2/firmwares"

firmware_id="${PEAK_FIRMWARE_ID:-155}"
firmware_name="peak-firmware-2.2-build2108.syx"
firmware_sha="f9d50b4268a5c612b71d4cafd45dc01a02ecdf0902aa6e31784ad4d1bc4de26e"

fpga_id="${PEAK_FPGA_ID:-142}"
fpga_name="peak-fpga-2.1-build469.syx"
fpga_sha="317d48663036feb5bce51fc8c5448c618f0cf8673c4bef2e2a4b1631ad18924d"

if [[ "${1:-}" == "--list" ]]; then
    curl -fsSL "$base" | python3 -c '
import json, sys
for f in json.load(sys.stdin)["firmwares"]:
    if f["product"].lower().startswith("peak"):
        print("{:>4}  {:<8}  v{:<5}  build {}".format(
            f["id"], f["firmware_type"], f["version"], f["build"]))
'
    exit 0
fi

for tool in curl sha256sum; do
    command -v "$tool" >/dev/null || { echo "missing $tool" >&2; exit 1; }
done

mkdir -p "$dest"

# fetch <id> <filename> <expected sha256>
fetch() {
    local id="$1" name="$2" want="$3"
    local path="$dest/$name"

    if [[ -f "$path" ]] && [[ "$(sha256sum "$path" | cut -d' ' -f1)" == "$want" ]]; then
        echo "ok      $name (already present)"
        return
    fi

    echo "fetch   $name (id $id)"
    # -f turns the 4xx/5xx the endpoint returns for a retired build into an
    # error instead of saving an HTML body under a .syx name.
    curl -fsSL --retry 3 -o "$path.part" "$base/$id/file"
    local got
    got="$(sha256sum "$path.part" | cut -d' ' -f1)"
    if [[ "$got" != "$want" ]]; then
        rm -f "$path.part"
        echo "checksum mismatch for $name" >&2
        echo "  expected $want" >&2
        echo "  got      $got" >&2
        exit 1
    fi
    mv "$path.part" "$path"
    echo "ok      $name ($(stat -c%s "$path") bytes)"
}

fetch "$firmware_id" "$firmware_name" "$firmware_sha"
fetch "$fpga_id" "$fpga_name" "$fpga_sha"

# Catalogue snapshot: cheap, and it documents which builds existed on the day
# the pinned pair was chosen.
if curl -fsSL -o "$dest/catalog-snapshot.json.part" "$base"; then
    mv "$dest/catalog-snapshot.json.part" "$dest/catalog-snapshot.json"
    echo "ok      catalog-snapshot.json"
else
    echo "warn    catalogue unavailable; keeping any previous snapshot" >&2
    rm -f "$dest/catalog-snapshot.json.part"
fi

# Only the two images we just verified: the sums file is what `sha256sum -c`
# re-checks later, and it must not list files that are not there. The paths are
# relative to the repository root when the images live in the default place.
if [[ "$(cd "$dest" && pwd)" == "$repo_root/firmware" ]]; then
    prefix="firmware/"
    sums_hint="cd \"$repo_root\" && sha256sum -c firmware/SHA256SUMS"
else
    prefix=""
    sums_hint="cd \"$dest\" && sha256sum -c SHA256SUMS"
fi

cat > "$dest/SHA256SUMS" <<EOF
$firmware_sha  $prefix$firmware_name
$fpga_sha  $prefix$fpga_name
EOF

echo
echo "wrote $dest/SHA256SUMS"
echo "revalidate with: $sums_hint"
echo
echo "These files are Focusrite/Novation property: keep them out of git and out"
echo "of anything you redistribute."
