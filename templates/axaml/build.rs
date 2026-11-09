use std::env;
use std::fs;
use std::path::PathBuf;

#[path = "src/axaml_components.rs"]
mod axaml_components;

fn main() {
    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR")
            .expect("Cargo must provide the CARGO_MANIFEST_DIR variable"),
    );
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("projection.ir.json").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("src/axaml_components.rs").display()
    );
    for (name, module, output) in [
        ("MainWindow", "generated_main_window", "main_window.rs"),
        (
            "FilterEditor",
            "generated_filter_editor",
            "filter_editor.rs",
        ),
    ] {
        let axaml_path = manifest_dir.join(format!("src/{name}.axaml"));
        println!("cargo:rerun-if-changed={}", axaml_path.display());
        let axaml = fs::read_to_string(&axaml_path)
            .unwrap_or_else(|error| panic!("Failed to read {}: {error}", axaml_path.display()));
        let generated = rustolonia_bindgen::compile_axaml_from_json_with_options(
            include_str!("projection.ir.json"),
            &axaml,
            &axaml_components::options(),
        )
        .unwrap_or_else(|error| panic!("Failed to compile {}: {error}", axaml_path.display()));
        // The compiler emits a standalone module header; include its items inside a wrapper module.
        let module_body = generated
            .strip_prefix(
                "//! Generated from AXAML by rustolonia-bindgen. Do not edit.\n#![allow(dead_code, unused_imports)]\n",
            )
            .expect("rustolonia-bindgen output must start with its generated-file header");
        let generated_module =
            format!("mod {module} {{\n#![allow(dead_code, unused_imports)]\n{module_body}\n}}\n");
        let output_path = PathBuf::from(
            env::var_os("OUT_DIR").expect("Cargo must provide the OUT_DIR build-script variable"),
        )
        .join(output);
        fs::write(&output_path, generated_module)
            .unwrap_or_else(|error| panic!("Failed to write {}: {error}", output_path.display()));
    }
}
