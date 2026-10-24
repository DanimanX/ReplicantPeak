#!/usr/bin/env bash
# Render the README screenshot from the real panel binary.
#
# The panel is a normal egui window, so it needs a display. On a headless
# machine this runs it under Xvfb. `WAYLAND_DISPLAY` is cleared first: with a
# Wayland session in the environment winit would open the window on the host
# compositor instead of the Xvfb display, and the capture would come out at the
# wrong size.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

out="${1:-assets/panel.png}"
# Accept absolute output paths; only bare ones are relative to the repository.
[[ "$out" == /* ]] || out="$repo_root/$out"
firmware="${PEAK_FIRMWARE:-firmware/peak-firmware-2.2-build2108.syx}"

if [[ ! -f "$firmware" ]]; then
    echo "firmware image not found: $firmware" >&2
    echo "run scripts/fetch-firmware.sh first, or set PEAK_FIRMWARE" >&2
    exit 1
fi

cargo build --release -p peak-panel

run=(env -u WAYLAND_DISPLAY
    LIBGL_ALWAYS_SOFTWARE=1
    PEAK_FIRMWARE="$firmware"
    PEAK_PANEL_SHOT="$out"
    PEAK_PANEL_SHOT_DELAY_MS="${PEAK_PANEL_SHOT_DELAY_MS:-1200}"
    "$repo_root/target/release/peak-panel")

# Capture deterministically: the host display may be scaled or a Wayland
# session, either of which changes the pixel size of the window. Opt out with
# PEAK_PANEL_DISPLAY=host when you want a screenshot of your own desktop.
if [[ "${PEAK_PANEL_DISPLAY:-virtual}" == "host" && -n "${DISPLAY:-}" ]]; then
    "${run[@]}"
else
    xvfb-run -a -s '-screen 0 1400x1000x24' "${run[@]}"
fi

echo "wrote $out"
