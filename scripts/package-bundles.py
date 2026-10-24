#!/usr/bin/env python3
"""Package the bundles produced by `cargo xtask bundle` into release assets.

Each plugin format gets its own archive so a download is unambiguous:

    ReplicantPeak-1.0.1-linux-clap.zip
    ReplicantPeak-1.0.1-linux-vst3.zip
    ...

Unzipping an archive yields the bundle itself (`ReplicantPeak.clap` or
`ReplicantPeak.vst3`), which is what a host expects to be dropped into its
plugin directory, plus the MIT licence notice.

The script refuses to run if a bundle is missing: a matrix leg that silently
produced nothing would otherwise publish an incomplete release.
"""

from __future__ import annotations

import argparse
import stat
import sys
import time
import zipfile
from pathlib import Path

FORMATS = ("clap", "vst3")

# Everything a host dlopen()s must keep its executable bit inside the archive:
# the bundler copies the cdylib with a plain file copy, which drops the mode.
_EXECUTABLE_SUFFIXES = {".clap", ".so", ".dll", ".dylib", ".vst3", ".component", ".exe"}


def _is_executable(path: Path) -> bool:
    # macOS bundles keep the binary at <bundle>/Contents/MacOS/<name>.
    return path.suffix in _EXECUTABLE_SUFFIXES or path.parent.name == "MacOS"


def _add(zf: zipfile.ZipFile, path: Path, arcname: Path) -> None:
    if path.is_dir():
        for child in sorted(path.rglob("*")):
            if child.is_file():
                _add(zf, child, arcname / child.relative_to(path))
        return

    mode = 0o755 if _is_executable(path) else 0o644
    info = zipfile.ZipInfo(str(arcname), time.localtime(path.stat().st_mtime)[:6])
    info.external_attr = (stat.S_IFREG | mode) << 16
    info.compress_type = zipfile.ZIP_DEFLATED
    zf.writestr(info, path.read_bytes())


def find_standalone(bundled: Path, os_name: str) -> Path | None:
    candidates = []
    if os_name == "macos":
        candidates = [bundled / "ReplicantPeak.app", bundled / "ReplicantPeak"]
    elif os_name == "windows":
        candidates = [bundled / "ReplicantPeak.exe", bundled / "ReplicantPeak"]
    else:
        candidates = [bundled / "ReplicantPeak", bundled / "ReplicantPeak.bin"]

    for c in candidates:
        if c.exists():
            return c
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundled", type=Path, default=Path("target/bundled"))
    parser.add_argument("--out", type=Path, default=Path("dist"))
    parser.add_argument("--version", required=True, help="e.g. 1.0.1")
    parser.add_argument("--os", required=True, help="e.g. linux, windows, macos")
    parser.add_argument("--license", type=Path, default=Path("LICENSE"))
    args = parser.parse_args()

    args.out.mkdir(parents=True, exist_ok=True)

    for fmt in FORMATS:
        src = args.bundled / f"ReplicantPeak.{fmt}"
        if not src.exists():
            print(
                f"error: {src} was not produced; the plugin does not export "
                f"{fmt} for {args.os}",
                file=sys.stderr,
            )
            return 1

        dest = args.out / f"ReplicantPeak-{args.version}-{args.os}-{fmt}.zip"
        with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as zf:
            _add(zf, src, Path(src.name))
            if args.license.is_file():
                _add(zf, args.license, Path(args.license.name))

        size = dest.stat().st_size
        entries = len(zipfile.ZipFile(dest).namelist())
        print(f"{dest}  ({entries} entries, {size / 1024:.0f} KiB)")

    standalone_src = find_standalone(args.bundled, args.os)
    if standalone_src is not None:
        dest = args.out / f"ReplicantPeak-{args.version}-{args.os}-standalone.zip"
        with zipfile.ZipFile(dest, "w", zipfile.ZIP_DEFLATED) as zf:
            _add(zf, standalone_src, Path(standalone_src.name))
            if args.license.is_file():
                _add(zf, args.license, Path(args.license.name))

        size = dest.stat().st_size
        entries = len(zipfile.ZipFile(dest).namelist())
        print(f"{dest}  ({entries} entries, {size / 1024:.0f} KiB)")

    return 0


if __name__ == "__main__":
    sys.exit(main())
