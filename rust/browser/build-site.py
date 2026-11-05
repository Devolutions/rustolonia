#!/usr/bin/env python3
"""Links a Rust app into the prebuilt Rustolonia browser host.

Usage:
    python3 build-site.py <rust-staticlib.a> <output-dir> <rustolonia-version> <abi-sha256>

The Rust app is a wasm32-unknown-emscripten staticlib exporting
`rustolonia_main`. This script relinks the host's dotnet.native.wasm with it
using the Emscripten SDK, at the exact version recorded in host-manifest.json
(`emcc` on PATH, as set up by emsdk_env, or EMCC), and writes a static site
to <output-dir>. It needs no .NET SDK: the managed host, the .NET runtime
archives and every other link input ship in this directory.

The output directory is owned by this script: it may be replaced on later
builds, and an existing directory without its ownership marker is refused.
"""

import base64
import gzip
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import sys
import tempfile

KIND = "rustolonia-browser-host"
OWNER_FILE = ".rustolonia-browser-site"
OWNER_TEXT = "rustolonia-browser-site-v1"
JSON_START = "/*json-start*/"
JSON_END = "/*json-end*/"


def fail(message):
    raise SystemExit(f"error: {message}")


def sha256_file(path):
    hasher = hashlib.sha256()
    with open(path, "rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            hasher.update(block)
    return hasher


def load_manifest(root, version, abi):
    try:
        manifest = json.loads((root / "host-manifest.json").read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        fail(f"cannot read the browser host manifest: {error}")
    if manifest.get("schemaVersion") != 1 or manifest.get("kind") != KIND:
        fail("unsupported browser host manifest")
    if manifest.get("version") != version:
        fail(f"host version {manifest.get('version')!r} does not match rustolonia-sys {version!r}")
    if not re.fullmatch(r"[0-9a-f]{64}", abi) or manifest.get("abiFingerprint") != abi:
        fail("the host ABI fingerprint does not match rustolonia-sys")
    return manifest


def verify_inventory(root, manifest):
    """Every file is listed with its SHA-256; nothing else is present."""
    entries = manifest.get("files")
    if not isinstance(entries, list) or not entries:
        fail("the browser host manifest has no file inventory")
    expected = set()
    for entry in entries:
        name, digest = entry.get("name"), entry.get("sha256")
        if not isinstance(name, str) or not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
            fail("invalid browser host file entry")
        relative = PurePosixPath(name)
        if relative.is_absolute() or "\\" in name or any(part in {"", ".", ".."} for part in relative.parts):
            fail(f"invalid browser host file path: {name!r}")
        if name in expected or name == "host-manifest.json":
            fail(f"duplicate or reserved browser host file: {name!r}")
        expected.add(name)
        path = root.joinpath(*relative.parts)
        if any(root.joinpath(*relative.parts[:i]).is_symlink() for i in range(1, len(relative.parts) + 1)):
            fail(f"symbolic links are not allowed in the browser host: {name!r}")
        try:
            actual = sha256_file(path).hexdigest()
        except OSError as error:
            fail(f"cannot read browser host file {name!r}: {error}")
        if actual != digest:
            fail(f"checksum mismatch for browser host file {name!r}")
    actual = set()
    for directory, dirs, files in os.walk(root, followlinks=False):
        for name in dirs + files:
            path = Path(directory) / name
            if path.is_symlink():
                fail(f"symbolic links are not allowed in the browser host: {path}")
        for name in files:
            relative = (Path(directory) / name).relative_to(root).as_posix()
            if relative != "host-manifest.json" and not relative.startswith("__pycache__/"):
                actual.add(relative)
    if actual != expected:
        fail(f"browser host file inventory differs: missing={sorted(expected - actual)}, "
             f"extra={sorted(actual - expected)}")


def find_emcc():
    emcc = os.environ.get("EMCC") or shutil.which("emcc")
    if not emcc:
        fail("emcc was not found; install the Emscripten SDK and run emsdk_env, or set EMCC")
    return Path(emcc)


def emscripten_version(emcc):
    output = subprocess.run([str(emcc), "--version"], check=True, capture_output=True, text=True).stdout
    match = re.search(r"\) (\d+\.\d+\.\d+)", output.splitlines()[0] if output else "")
    if not match:
        fail(f"cannot read the Emscripten version from: {output.strip()!r}")
    return match.group(1)


def find_tool(emcc, name):
    """LLVM and Binaryen tools ship in emsdk's upstream/bin, beside upstream/emscripten."""
    exe = name + (".exe" if os.name == "nt" else "")
    candidates = []
    for variable in ("EM_LLVM_ROOT", "EM_BINARYEN_ROOT"):
        if os.environ.get(variable):
            root = Path(os.environ[variable])
            candidates += [root / exe, root / "bin" / exe]
    candidates.append(emcc.resolve().parent.parent / "bin" / exe)
    for candidate in candidates:
        if candidate.is_file():
            return candidate
    found = shutil.which(name)
    if found:
        return Path(found)
    fail(f"{name} was not found beside {emcc}; set EM_LLVM_ROOT to the emsdk upstream/bin directory")


def run(command, cwd=None, env=None):
    print("==> " + " ".join(str(part) for part in command), file=sys.stderr)
    subprocess.run([str(part) for part in command], check=True, cwd=cwd, env=env)


def fingerprint(digest):
    return base64.b32encode(digest).decode("ascii").lower()[:10]


def integrity(digest):
    return "sha256-" + base64.b64encode(digest).decode("ascii")


def write_gzip(path):
    with open(path, "rb") as source, open(str(path) + ".gz", "wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, compresslevel=9, mtime=0) as target:
            shutil.copyfileobj(source, target)


def patch_boot_config(path, js_name, wasm_name, wasm_digest):
    text = path.read_text(encoding="utf-8")
    start, end = text.find(JSON_START), text.find(JSON_END)
    if start < 0 or end < start:
        fail(f"{path.name} has no embedded boot configuration")
    config = json.loads(text[start + len(JSON_START):end])
    resources = config.get("resources")
    if not isinstance(resources, dict) or len(resources.get("wasmNative", [])) != 1:
        fail("unexpected boot configuration: expected one wasmNative resource")
    resources["jsModuleNative"] = [{"name": js_name}]
    native = resources["wasmNative"][0]
    native["name"] = wasm_name
    native["hash"] = integrity(wasm_digest)
    # An opaque version of the resource set (cache keys); it must change with it.
    previous = str(resources.get("hash", ""))
    resources["hash"] = integrity(hashlib.sha256((previous + native["hash"]).encode("utf-8")).digest())
    body = json.dumps(config, indent=2, ensure_ascii=False)
    path.write_text(text[:start + len(JSON_START)] + body + text[end:], encoding="utf-8", newline="\n")


def is_owned(path):
    marker = path / OWNER_FILE
    return (path.is_dir() and not path.is_symlink() and marker.is_file() and not marker.is_symlink()
            and marker.read_text(encoding="utf-8").strip() == OWNER_TEXT)


def main(argv):
    if len(argv) != 5:
        print(__doc__, file=sys.stderr)
        return 2
    app_library, output, version, abi = Path(argv[1]).resolve(), Path(argv[2]).resolve(), argv[3], argv[4]
    root = Path(__file__).resolve().parent
    manifest = load_manifest(root, version, abi)
    verify_inventory(root, manifest)
    if not app_library.is_file():
        fail(f"the Rust static library does not exist: {app_library}")
    if (output.exists() or output.is_symlink()) and not is_owned(output):
        fail(f"refusing to replace a directory not created by this script: {output}")

    emcc = find_emcc()
    wanted = manifest["emscriptenVersion"]
    actual = emscripten_version(emcc)
    if actual != wanted:
        fail(f"this host was linked with Emscripten {wanted}, but {emcc} is {actual}; "
             f"install it with: emsdk install {wanted} && emsdk activate {wanted}")
    objcopy = find_tool(emcc, "llvm-objcopy")
    link = manifest["link"]

    output.parent.mkdir(parents=True, exist_ok=True)
    stage = Path(tempfile.mkdtemp(prefix=".rustolonia-browser-stage-", dir=output.parent))
    try:
        work = stage / "link"
        work.mkdir()
        # The Rust archive records wasm features newer than this Emscripten's
        # LLVM knows; the host's glue objects declare the ones it needs.
        app = work / "rustolonia_app.a"
        run([objcopy, "--remove-section=target_features", app_library, app])
        native_js = work / "dotnet.native.js"
        native_wasm = work / "dotnet.native.wasm"
        posix = lambda path: path.resolve().as_posix()
        rsp = (root / link["responseFile"]).read_text(encoding="utf-8")
        rsp = rsp.replace("@KIT@", posix(root)).replace("@APP@", posix(app)).replace("@OUT@", posix(native_js))
        (work / "link.rsp").write_text(rsp, encoding="utf-8", newline="\n")
        # The .NET runtime's JavaScript library reads its feature switches
        # (WASM_ENABLE_SIMD, WASM_ENABLE_EH, ...) from emcc's environment.
        environment = link.get("environment", {})
        if not isinstance(environment, dict) or not all(
                isinstance(k, str) and isinstance(v, str) for k, v in environment.items()):
            fail("invalid link environment in the browser host manifest")
        run([emcc, "@" + str(work / "link.rsp")], cwd=work, env={**os.environ, **environment})
        if link.get("runWasmOpt"):
            wasm_opt = find_tool(emcc, "wasm-opt")
            run([wasm_opt, "--enable-simd", "--enable-exception-handling", "--enable-bulk-memory",
                 *link.get("wasmOptFlags", []), native_wasm, "-o", native_wasm])

        site = stage / "site"
        shutil.copytree(root / manifest["site"]["directory"], site)
        framework = site / "_framework"
        wasm_digest = sha256_file(native_wasm).digest()
        js_digest = sha256_file(native_js).digest()
        wasm_name = f"dotnet.native.{fingerprint(wasm_digest)}.wasm"
        js_name = f"dotnet.native.{fingerprint(js_digest)}.js"
        shutil.copyfile(native_wasm, framework / wasm_name)
        shutil.copyfile(native_js, framework / js_name)
        boot = site / manifest["site"]["bootConfig"]
        patch_boot_config(boot, js_name, wasm_name, wasm_digest)
        for path in (framework / wasm_name, framework / js_name, boot):
            write_gzip(path)
        (site / OWNER_FILE).write_text(OWNER_TEXT + "\n", encoding="utf-8")

        previous = stage / "previous"
        if output.exists():
            output.rename(previous)
        try:
            site.rename(output)
        except OSError:
            if previous.exists():
                previous.rename(output)
            raise
    finally:
        shutil.rmtree(stage, ignore_errors=True)
    print(f"Browser app: {output}")
    print("Serve it with any static file server, for example: "
          f"python3 -m http.server 8080 --directory \"{output}\"")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
