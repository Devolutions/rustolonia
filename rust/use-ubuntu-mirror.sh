#!/usr/bin/env bash
set -euo pipefail

architecture="$(dpkg --print-architecture)"
case "$architecture" in
    amd64) mirror="https://mirrors.mit.edu/ubuntu/" ;;
    arm64) mirror="https://mirrors.mit.edu/ubuntu-ports/" ;;
    *)
        echo "Unsupported Ubuntu architecture: $architecture" >&2
        exit 1
        ;;
esac

python3 - "$mirror" "${1:-/etc/apt}" <<'PY'
from pathlib import Path
import re
import sys

mirror = sys.argv[1]
apt_dir = Path(sys.argv[2])
source_files = [apt_dir / "sources.list"]
source_files.extend((apt_dir / "sources.list.d").glob("*.list"))
source_files.extend((apt_dir / "sources.list.d").glob("*.sources"))
ubuntu_uri = re.compile(
    r"https?://[^/\s]*ubuntu\.com/ubuntu(?:-ports)?/?"
    r"|mirror\+file:(?://)?/etc/apt/apt-mirrors\.txt(?=\s|$)"
)
selected_uri = re.compile(re.escape(mirror.rstrip("/")) + r"/?(?=\s|$)")

configured = False
for source_file in source_files:
    if source_file.is_file():
        content = source_file.read_text()
        updated = ubuntu_uri.sub(mirror, content)
        configured |= bool(selected_uri.search(updated))
        if updated != content:
            source_file.write_text(updated)

if not configured:
    sys.exit(f"No supported Ubuntu APT sources found in {apt_dir}")
PY
