use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().collect();
    if args
        .get(1)
        .is_some_and(|argument| argument == "--compile-axaml")
    {
        return compile_axaml_cli(&args[2..]);
    }
    if args
        .get(1)
        .is_some_and(|argument| argument == "--audit-axaml")
    {
        return audit_axaml_cli(&args[2..]);
    }
    let check = args.get(1).is_some_and(|argument| argument == "--check");
    let positional: Vec<_> = if check {
        args[2..].to_vec()
    } else {
        args[1..].to_vec()
    };
    if positional.len() != 2 && positional.len() != 3 {
        eprintln!(
            "Usage:\n  rustolonia-bindgen [--check] <projection.ir.json> <sys-output> [safe-output]\n  rustolonia-bindgen --compile-axaml <projection.ir.json> <input.axaml> <output.rs>\n  rustolonia-bindgen --audit-axaml <projection.ir.json> <input-directory> <report.json>"
        );
        return ExitCode::from(2);
    }

    let json = match fs::read_to_string(&positional[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Failed to read projection IR: {error}");
            return ExitCode::FAILURE;
        }
    };
    let sys = match rustolonia_bindgen::generate_from_json(&json) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Failed to generate Rust bindings: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut outputs = vec![(PathBuf::from(&positional[1]), sys)];
    if positional.len() == 3 {
        let safe = match rustolonia_bindgen::generate_safe_from_json(&json) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("Failed to generate safe Rust bindings: {error}");
                return ExitCode::FAILURE;
            }
        };
        outputs.push((PathBuf::from(&positional[2]), safe));
    }

    if check {
        let report = match rustolonia_bindgen::check_outputs(
            rustolonia_bindgen::BINDGEN_GENERATOR_ID,
            &outputs,
        ) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("Failed to check generated Rust: {error}");
                return ExitCode::FAILURE;
            }
        };
        if report.success() {
            println!(
                "Generation check passed for {} output file(s).",
                outputs.len()
            );
            return ExitCode::SUCCESS;
        }
        for mismatch in report.mismatches {
            eprintln!("{mismatch}");
        }
        return ExitCode::FAILURE;
    }

    if let Err(error) =
        rustolonia_bindgen::write_outputs(rustolonia_bindgen::BINDGEN_GENERATOR_ID, &outputs)
    {
        eprintln!("Failed to write generated Rust: {error}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn audit_axaml_cli(args: &[std::ffi::OsString]) -> ExitCode {
    if args.len() != 3 {
        eprintln!(
            "Usage: rustolonia-bindgen --audit-axaml <projection.ir.json> <input-directory> <report.json>"
        );
        return ExitCode::from(2);
    }
    let ir_json = match fs::read_to_string(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Failed to read projection IR: {error}");
            return ExitCode::FAILURE;
        }
    };
    let ir: rustolonia_bindgen::ProjectionIr = match serde_json::from_str(&ir_json) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Failed to parse projection IR: {error}");
            return ExitCode::FAILURE;
        }
    };
    let input_directory = PathBuf::from(&args[1]);
    if !input_directory.is_dir() {
        eprintln!(
            "AXAML audit input is not a directory: {}",
            input_directory.display()
        );
        return ExitCode::FAILURE;
    }

    let mut files = Vec::new();
    if let Err(error) = collect_axaml_files(&input_directory, &mut files) {
        eprintln!(
            "Failed to enumerate AXAML files under {}: {error}",
            input_directory.display()
        );
        return ExitCode::FAILURE;
    }
    files.sort();

    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut results = Vec::with_capacity(files.len());
    for path in files {
        let relative = path
            .strip_prefix(&input_directory)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let result = fs::read_to_string(&path)
            .map_err(|error| format!("Failed to read AXAML: {error}"))
            .and_then(|source| {
                rustolonia_bindgen::compile_axaml_to_rust(&ir, &source)
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(()) => {
                accepted += 1;
                results.push(serde_json::json!({
                    "path": relative,
                    "status": "transpilerAccepted",
                    "diagnostic": null,
                }));
            }
            Err(diagnostic) => {
                rejected += 1;
                results.push(serde_json::json!({
                    "path": relative,
                    "status": "rejected",
                    "diagnostic": diagnostic,
                }));
            }
        }
    }

    let report = serde_json::json!({
        "reportVersion": 1,
        "projectionIrSchemaVersion": ir.version,
        "validationStage": "transpiler-only",
        "limitations": [
            "transpilerAccepted means Rust source was emitted; it was not type-checked or run",
            "source-tree results do not account for MSBuild platform/configuration item conditions"
        ],
        "summary": {
            "total": accepted + rejected,
            "transpilerAccepted": accepted,
            "rejected": rejected
        },
        "files": results
    });
    let report_path = PathBuf::from(&args[2]);
    let json = match serde_json::to_string_pretty(&report) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Failed to serialize AXAML audit report: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = fs::write(&report_path, format!("{json}\n")) {
        eprintln!(
            "Failed to write AXAML audit report {}: {error}",
            report_path.display()
        );
        return ExitCode::FAILURE;
    }
    println!(
        "AXAML audit wrote {}: {} transpiler-accepted, {} rejected. Rust type-check and runtime behavior were not tested.",
        report_path.display(),
        accepted,
        rejected
    );
    ExitCode::SUCCESS
}

fn collect_axaml_files(directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_axaml_files(&path, files)?;
        } else if file_type.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("axaml"))
        {
            files.push(path);
        }
    }
    Ok(())
}

fn compile_axaml_cli(args: &[std::ffi::OsString]) -> ExitCode {
    if args.len() != 3 {
        eprintln!(
            "Usage: rustolonia-bindgen --compile-axaml <projection.ir.json> <input.axaml> <output.rs>"
        );
        return ExitCode::from(2);
    }
    let ir_json = match fs::read_to_string(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Failed to read projection IR: {error}");
            return ExitCode::FAILURE;
        }
    };
    let axaml = match fs::read_to_string(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!(
                "Failed to read AXAML file {}: {error}",
                args[1].to_string_lossy()
            );
            return ExitCode::FAILURE;
        }
    };
    match rustolonia_bindgen::compile_axaml_from_json(&ir_json, &axaml) {
        Ok(rust_source) => {
            if let Err(error) = fs::write(&args[2], rust_source) {
                eprintln!("Failed to write compiled AXAML Rust output: {error}");
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!(
                "Failed to compile AXAML {}: {error}",
                args[1].to_string_lossy()
            );
            ExitCode::FAILURE
        }
    }
}
