use std::{env, fs, path::PathBuf};

mod axaml_components {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../templates/axaml/src/axaml_components.rs"
    ));
}

fn main() {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../..");
    let ir = root.join("rust/projection.ir.json");
    println!("cargo:rerun-if-changed={}", ir.display());
    println!(
        "cargo:rerun-if-changed={}",
        root.join("templates/axaml/src/axaml_components.rs")
            .display()
    );
    let projection = fs::read_to_string(&ir).unwrap();
    for (name, module, output) in [
        ("MainWindow", "generated_main_window", "main_window.rs"),
        (
            "FilterEditor",
            "generated_filter_editor",
            "filter_editor.rs",
        ),
    ] {
        let axaml = root.join(format!("templates/axaml/src/{name}.axaml"));
        println!("cargo:rerun-if-changed={}", axaml.display());
        let source = fs::read_to_string(&axaml).unwrap();
        let generated = rustolonia_bindgen::compile_axaml_from_json_with_options(
            &projection,
            &source,
            &axaml_components::options(),
        )
        .unwrap_or_else(|error| panic!("{}: {error}", axaml.display()));
        let body = generated
            .strip_prefix(
                "//! Generated from AXAML by rustolonia-bindgen. Do not edit.\n#![allow(dead_code, unused_imports)]\n",
            )
            .expect("generated module header");
        let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join(output);
        fs::write(
            output,
            format!("mod {module} {{\n#![allow(dead_code, unused_imports)]\n{body}\n}}\n"),
        )
        .unwrap();
    }
}
