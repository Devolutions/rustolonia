//! Embeds the program icon (see assets/icon_res.rs) and puts Rustolonia's
//! host next to `jiffshot.exe`, so the build output folder runs as it is and
//! can be zipped as the portable app.

include!("../assets/icon_res.rs");

use std::path::PathBuf;

fn main() {
    embed_icon("../assets/jiffshot.ico");
    copy_host();
}

/// Copies the host directory `rustolonia-sys` staged (its native libraries)
/// into the profile directory the executable is linked into.
fn copy_host() {
    let Some(host) = std::env::var_os("DEP_RUSTOLONIA_HOST_HOST_DIR").map(PathBuf::from) else {
        println!("cargo:warning=no Rustolonia host was staged; copy it next to jiffshot.exe yourself");
        return;
    };
    // OUT_DIR is <target>/<profile>/build/jiffshot-<hash>/out.
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let Some(profile) = out.ancestors().nth(3) else { return };
    let Ok(entries) = std::fs::read_dir(&host) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("dll") || e == "so" || e == "dylib") {
            println!("cargo:rerun-if-changed={}", path.display());
            if let Err(error) = std::fs::copy(&path, profile.join(entry.file_name())) {
                println!("cargo:warning=copying {}: {error}", path.display());
            }
        }
    }
    println!("cargo:rerun-if-env-changed=RUSTOLONIA_HOST_DIR");
}
