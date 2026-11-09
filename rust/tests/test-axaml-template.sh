#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
template="$root/templates/axaml"
mode="${1:-check}"
if [[ $# -gt 1 || ( "$mode" != "check" && "$mode" != "--native-smoke" ) ]]; then
    echo "Usage: $0 [--native-smoke]" >&2
    exit 2
fi
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

app="$scratch/app"
mkdir -p "$app" "$scratch/no-dotnet"
find "$template" -mindepth 1 -maxdepth 1 ! -name target -exec cp -R {} "$app/" \;
cat >>"$app/.cargo/config.toml" <<TOML

[patch."https://github.com/Devolutions/rustolonia"]
rustolonia = { path = "$root/rust/rustolonia" }
rustolonia-bindgen = { path = "$root/rust/rustolonia-bindgen" }
rustolonia-sys = { path = "$root/rust/rustolonia-sys" }
TOML

cat >"$scratch/no-dotnet/dotnet" <<'SH'
#!/usr/bin/env sh
echo "dotnet must not run when building the Rust-only AXAML template" >&2
exit 1
SH
chmod +x "$scratch/no-dotnet/dotnet"

export PATH="$scratch/no-dotnet:$HOME/.cargo/bin:/usr/bin:/bin:/usr/sbin:/sbin"
export RUSTOLONIA_NO_DOWNLOAD=1
cd "$app"
if [[ "$mode" == "--native-smoke" ]]; then
    : "${RUSTOLONIA_HOST_LIB:?Set RUSTOLONIA_HOST_LIB to the matching native host}"
    cp "$root/rust/tests/axaml-native/src/main.rs" "$app/src/main.rs"
    cargo run
else
    cargo check
fi
