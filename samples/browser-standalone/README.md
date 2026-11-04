# Standalone browser app (no .NET)

This code-first app builds its UI in Rust and runs in the browser on the
prebuilt Rustolonia browser host. Building it needs Cargo and the Emscripten
SDK; it does **not** compile C# or need a .NET SDK. The design is described in
[rust/PLATFORMS.md](../../rust/PLATFORMS.md#prebuilt-browser-host).

The browser host is a release asset from the first release that includes it;
earlier releases do not have one. This in-repository copy uses the local
crates, so it can also test a host produced from this checkout.

## Requirements

- Rust 1.88 or newer with `rustup target add wasm32-unknown-emscripten`.
- The [Emscripten SDK](https://emscripten.org/docs/getting_started/downloads.html)
  at the version the host was linked with (3.1.56 for .NET 10 hosts), active
  in the shell:

  ```bash
  ./emsdk install 3.1.56 && ./emsdk activate 3.1.56
  source ./emsdk_env.sh        # emsdk_env.ps1 / emsdk_env.bat on Windows
  ```

- Python 3 (Emscripten requires it too).

## Build and run

```bash
python3 build.py --serve          # http://localhost:8080
```

`build.py`:

1. reads the version, ABI fingerprint and `host-checksums.txt` of the
   `rustolonia-sys` crate Cargo resolves for this app;
2. downloads `rustolonia-host-<version>-browser-wasm.tar.gz` from the matching
   GitHub release into `target/rustolonia-browser-host/` and checks its SHA-256
   against that list before extracting it;
3. builds the staticlib for `wasm32-unknown-emscripten` with `panic = "abort"`;
4. runs the host's `build-site.py`, which checks every packaged file and the
   Emscripten version, relinks the .NET runtime's `dotnet.native.wasm` with
   the Rust library, and writes a static site to `target/browser-site`
   (`--out` to change it).

Serve the site with any static file server. The output directory is owned by
the script: later builds replace it, and it refuses to replace a directory it
did not create. `--debug` builds the debug profile.

`python3 smoke-test.py target/browser-site` loads the site in headless Chrome
(or `CHROME`) and waits for the app's "main view ready" console line.

## Use it in your own project

Copy `Cargo.toml`, `src/`, `build.py` and (optionally) `smoke-test.py`, then
replace the path dependency with the tag of a release that has the browser
host:

```toml
[dependencies]
rustolonia = { git = "https://github.com/Devolutions/rustolonia", tag = "vX.Y.Z", default-features = false }
```

Run `cargo generate-lockfile` once and commit `Cargo.lock`; `build.py` builds
with `--locked`. The crate must be `crate-type = ["staticlib"]` and export
`rustolonia_main`, which the host calls once the page has loaded.

For an offline or mirrored archive, set `RUSTOLONIA_BROWSER_HOST_ARCHIVE` to
its path (the release checksum is still required), or
`RUSTOLONIA_HOST_BASE_URL` to a mirror laid out like the release downloads.
For a host produced locally before a release exists, additionally set
`RUSTOLONIA_BROWSER_HOST_SHA256` to its lowercase SHA-256; this explicit
override replaces the release-pinned checksum, so obtain it from a trusted
source:

```pwsh
pwsh ./rust/package-browser-host.ps1 -OutputRoot ./rust/artifacts/host
```

The page has a single view; see the browser limitations in
[rust/PLATFORMS.md](../../rust/PLATFORMS.md#limitations).
