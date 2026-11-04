#!/usr/bin/env python3
"""Builds this crate into a static browser site against the prebuilt
Rustolonia browser host, with Cargo and the Emscripten SDK only (no .NET).

Usage: python3 build.py [--debug] [--out DIR] [--serve [PORT]]

1. Reads the rustolonia-sys version, ABI fingerprint and host-checksums.txt
   of the crate Cargo resolves for this app.
2. Downloads rustolonia-host-<version>-browser-wasm.tar.gz from the matching
   GitHub release (or uses RUSTOLONIA_BROWSER_HOST_ARCHIVE) and checks its
   SHA-256 against that list before extracting it. For a locally produced
   host, RUSTOLONIA_BROWSER_HOST_SHA256 explicitly replaces the pinned hash.
3. Builds the staticlib for wasm32-unknown-emscripten (panic = abort).
4. Runs the host's build-site.py, which relinks dotnet.native.wasm with it
   using emcc and writes the site (default target/browser-site).
"""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request

TARGET = "wasm32-unknown-emscripten"
RELEASES = os.environ.get("RUSTOLONIA_HOST_BASE_URL",
                          "https://github.com/Devolutions/rustolonia/releases/download")


def fail(message):
    raise SystemExit(f"error: {message}")


def cargo():
    return os.environ.get("CARGO", "cargo")


def resolve_bindings(manifest):
    output = subprocess.run([cargo(), "metadata", "--locked", "--format-version", "1",
                             "--manifest-path", str(manifest)],
                            check=True, capture_output=True, text=True).stdout
    metadata = json.loads(output)
    packages = [p for p in metadata["packages"] if p["name"] == "rustolonia-sys"]
    if len(packages) != 1:
        fail("expected exactly one rustolonia-sys package in the Cargo dependency graph")
    root = Path(packages[0]["manifest_path"]).parent
    abi = hashlib.sha256((root / "include" / "avalonia-rust-abi.h").read_bytes()).hexdigest()
    library = None
    for package in metadata["packages"]:
        if Path(package["manifest_path"]).resolve() == manifest.resolve():
            for target in package["targets"]:
                if "staticlib" in target["crate_types"]:
                    library = target["name"].replace("-", "_")
    if not library:
        fail("this crate has no staticlib target; set crate-type = [\"staticlib\"] under [lib]")
    return packages[0]["version"], abi, root / "host-checksums.txt", library, Path(metadata["target_directory"])


def expected_hash(checksums, asset):
    override = os.environ.get("RUSTOLONIA_BROWSER_HOST_SHA256")
    if override:
        if not re.fullmatch(r"[0-9a-f]{64}", override):
            fail("RUSTOLONIA_BROWSER_HOST_SHA256 must be a lowercase SHA-256 hash")
        return override
    matches = []
    for line in checksums.read_text(encoding="utf-8").splitlines():
        match = re.fullmatch(r"\s*([0-9a-f]{64})\s+\*?(\S+)\s*", line)
        if match and match.group(2) == asset:
            matches.append(match.group(1))
    if len(matches) != 1:
        fail(f"no unique trusted checksum for {asset} in {checksums}. Use a release that includes "
             "the browser host, or set RUSTOLONIA_BROWSER_HOST_ARCHIVE and "
             "RUSTOLONIA_BROWSER_HOST_SHA256 for a locally produced one")
    return matches[0]


def fetch_archive(version, asset, digest, cache):
    local = os.environ.get("RUSTOLONIA_BROWSER_HOST_ARCHIVE")
    path = Path(local) if local else cache / asset
    if not local and not path.is_file():
        url = f"{RELEASES.rstrip('/')}/v{version}/{asset}"
        print(f"Downloading {url}", file=sys.stderr)
        cache.mkdir(parents=True, exist_ok=True)
        partial = path.with_name(path.name + ".partial")
        with urllib.request.urlopen(url) as response, open(partial, "wb") as target:
            shutil.copyfileobj(response, target)
        partial.replace(path)
    if not path.is_file():
        fail(f"the browser host archive does not exist: {path}")
    hasher = hashlib.sha256()
    with open(path, "rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(block)
    if hasher.hexdigest() != digest:
        if not local:
            path.unlink()
        fail(f"SHA-256 mismatch for {path}: expected {digest}, got {hasher.hexdigest()}")
    return path


def extract(archive, destination):
    """Extracts regular files only, refusing absolute or escaping paths."""
    with tarfile.open(archive, "r:gz") as source:
        for member in source:
            relative = PurePosixPath(member.name)
            if (not member.isfile() or relative.is_absolute() or "\\" in member.name
                    or any(part in {"", ".", ".."} for part in relative.parts)):
                fail(f"unsafe browser host archive entry: {member.name!r}")
            target = destination.joinpath(*relative.parts)
            target.parent.mkdir(parents=True, exist_ok=True)
            with source.extractfile(member) as data, open(target, "xb") as out:
                shutil.copyfileobj(data, out)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--debug", action="store_true", help="build the debug profile")
    parser.add_argument("--out", type=Path, help="site directory (default: <target>/browser-site)")
    parser.add_argument("--serve", nargs="?", const=8080, type=int, metavar="PORT",
                        help="serve the site on http://localhost:PORT afterwards")
    args = parser.parse_args()

    here = Path(__file__).resolve().parent
    manifest = here / "Cargo.toml"
    version, abi, checksums, library, target_dir = resolve_bindings(manifest)
    asset = f"rustolonia-host-{version}-browser-wasm.tar.gz"
    archive = fetch_archive(version, asset, expected_hash(checksums, asset),
                            target_dir / "rustolonia-browser-host")

    profile = "debug" if args.debug else "release"
    command = [cargo(), "build", "--locked", "--manifest-path", str(manifest), "--target", TARGET]
    if not args.debug:
        command.append("--release")
    # Unwinding needs Emscripten exception support the host's link does not enable.
    env = dict(os.environ, CARGO_PROFILE_RELEASE_PANIC="abort", CARGO_PROFILE_DEV_PANIC="abort")
    subprocess.run(command, check=True, env=env)
    staticlib = target_dir / TARGET / profile / f"lib{library}.a"

    out = (args.out or target_dir / "browser-site").resolve()
    with tempfile.TemporaryDirectory(prefix="rustolonia-browser-host-") as host:
        extract(archive, Path(host))
        subprocess.run([sys.executable, str(Path(host) / "build-site.py"),
                        str(staticlib), str(out), version, abi], check=True)
    if args.serve is not None:
        print(f"Serving http://localhost:{args.serve} (Ctrl+C to stop)")
        subprocess.run([sys.executable, "-m", "http.server", str(args.serve), "--directory", str(out)])
    return 0


if __name__ == "__main__":
    sys.exit(main())
