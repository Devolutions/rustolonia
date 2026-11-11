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

python3 - "$mirror" <<'PY'
from pathlib import Path
import re
import sys

mirror = sys.argv[1]
source_files = [Path("/etc/apt/sources.list")]
source_files.extend(Path("/etc/apt/sources.list.d").glob("*.list"))
source_files.extend(Path("/etc/apt/sources.list.d").glob("*.sources"))
ubuntu_uri = re.compile(r"https?://[^/\s]*ubuntu\.com/ubuntu(?:-ports)?/?")

for source_file in source_files:
    if source_file.is_file():
        content = source_file.read_text()
        updated = ubuntu_uri.sub(mirror, content)
        if updated != content:
            source_file.write_text(updated)
PY
