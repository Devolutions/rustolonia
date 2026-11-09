#!/usr/bin/env python3
"""Collect notices from the exact pinned sources of the staged musl archives."""

import csv
import hashlib
import pathlib
import re
import sys
import tarfile


def fail(message):
    raise SystemExit(message)


def file_hash(path, algorithm):
    digest = hashlib.new(algorithm)
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def source_manifest(path):
    records = {}
    with path.open(encoding="utf-8", newline="") as stream:
        for row in csv.reader(stream, delimiter="\t"):
            if not row or row[0].startswith("#"):
                continue
            if len(row) != 5 or row[0] in records or not re.fullmatch(r"[0-9a-f]{128}", row[4]):
                fail(f"Invalid musl source manifest entry: {row}")
            records[row[0]] = row[1:]
    return records


def include_notice(source, member):
    path = pathlib.PurePosixPath(member.name)
    parts = path.parts
    if len(parts) < 2:
        return False
    name = path.name.upper()
    if len(parts) <= 4 and (
        name.startswith(("COPYING", "LICENSE", "LICENCE", "NOTICE", "COPYRIGHT"))
        or name in {"AUTHORS", "PATENTS"}
    ):
        return True
    if source.startswith("freetype-") and "/".join(parts[1:]) in {
        "docs/FTL.TXT",
        "docs/GPLv2.TXT",
        "src/bdf/README",
        "src/pcf/README",
    }:
        return True
    return False


def uuid_header(source, member):
    if not source.startswith("util-linux-"):
        return False
    parts = pathlib.PurePosixPath(member.name).parts
    relative = "/".join(parts[1:])
    return (
        relative.startswith("libuuid/src/")
        and relative.endswith((".c", ".h"))
    ) or relative in {"lib/randutils.c", "lib/md5.c", "lib/sha1.c"}


def collect(source, path):
    notices = []
    with tarfile.open(path, mode="r|*") as archive:
        for member in archive:
            if not member.isfile():
                continue
            full_notice = include_notice(source, member)
            header = uuid_header(source, member)
            if not full_notice and not header:
                continue
            if member.size > 5 * 1024 * 1024:
                fail(f"Unexpectedly large notice in {source}: {member.name}")
            stream = archive.extractfile(member)
            if stream is None:
                fail(f"Cannot read {member.name} from {source}")
            try:
                text = stream.read().decode("utf-8")
            except UnicodeDecodeError as error:
                fail(f"Notice is not valid UTF-8 in {source}: {member.name}: {error}")
            if header and not full_notice:
                match = re.match(r"\s*/\*.*?\*/", text, re.DOTALL)
                if match is None:
                    continue
                text = match.group(0)
            notices.append((member.name, text))
    if not notices:
        fail(f"No upstream copyright or license notices found in {source}")
    return sorted(notices)


def main():
    if len(sys.argv) != 5:
        fail("Usage: collect-linux-musl-notices.py <manifest> <provenance> <source-cache> <output>")
    manifest_path, provenance_path, cache, output = map(pathlib.Path, sys.argv[1:])
    manifest = source_manifest(manifest_path)
    used = {}
    with provenance_path.open(encoding="utf-8", newline="") as stream:
        for row in csv.reader(stream, delimiter="\t"):
            if not row or row[0].startswith("#"):
                continue
            if len(row) != 5 or row[0] not in manifest:
                fail(f"Unrecognized staged musl archive: {row}")
            _, _, source, digest = manifest[row[0]]
            if source in used and used[source] != digest:
                fail(f"Conflicting source hashes for {source}")
            used[source] = digest
    if not used:
        fail("No musl static archives were staged")
    with output.open("w", encoding="utf-8", newline="\n") as result:
        result.write("This software is based in part on the work of the FreeType Team.\n")
        for source, expected in sorted(used.items()):
            source_path = cache / source
            if not source_path.is_file() or file_hash(source_path, "sha512") != expected:
                fail(f"Missing or mismatched pinned source: {source_path}")
            result.write(f"\n===== {source} (SHA-512 {expected}) =====\n")
            for name, text in collect(source, source_path):
                result.write(f"\n--- {name} ---\n{text}\n")


if __name__ == "__main__":
    main()
