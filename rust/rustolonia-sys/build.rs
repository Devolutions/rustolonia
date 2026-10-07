use sha2::{Digest, Sha256};
use std::path::Path;

fn main() {
    let header = Path::new("include").join("avalonia-rust-abi.h");
    println!("cargo:rerun-if-changed={}", header.display());
    let bytes = std::fs::read(&header)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", header.display()));
    let fingerprint: String = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    println!("cargo:rustc-env=RUSTOLONIA_ABI_FINGERPRINT={fingerprint}");
}
