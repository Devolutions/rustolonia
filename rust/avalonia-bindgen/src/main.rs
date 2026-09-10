use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().collect();
    if args
        .get(1)
        .is_some_and(|argument| argument == "--write-baseline")
    {
        return write_baseline(&args[2..]);
    }
    let check = args.get(1).is_some_and(|argument| argument == "--check");
    let positional: Vec<_> = if check {
        args[2..].to_vec()
    } else {
        args[1..].to_vec()
    };
    if positional.len() != 2 && positional.len() != 3 {
        eprintln!(
            "Usage: avalonia-bindgen [--check] <projection.ir.json> <sys-output> [safe-output]"
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
    let sys = match avalonia_bindgen::generate_from_json(&json) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Failed to generate Rust bindings: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut outputs = vec![(PathBuf::from(&positional[1]), sys)];
    if positional.len() == 3 {
        let safe = match avalonia_bindgen::generate_safe_from_json(&json) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("Failed to generate safe Rust bindings: {error}");
                return ExitCode::FAILURE;
            }
        };
        outputs.push((PathBuf::from(&positional[2]), safe));
    }

    if check {
        let report =
            match avalonia_bindgen::check_outputs(avalonia_bindgen::BINDGEN_GENERATOR_ID, &outputs)
            {
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
        avalonia_bindgen::write_outputs(avalonia_bindgen::BINDGEN_GENERATOR_ID, &outputs)
    {
        eprintln!("Failed to write generated Rust: {error}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

/// Rewrites the frozen released-ABI snapshot from the current header and IR.
/// Run only as part of an intentional ABI release wave; the regenerated
/// baseline makes the change auditable in review and keeps the identity
/// contract test green afterwards.
fn write_baseline(args: &[std::ffi::OsString]) -> ExitCode {
    if args.len() != 3 {
        eprintln!(
            "Usage: avalonia-bindgen --write-baseline <projection.ir.json> <abi-header> <abi-baseline.json>"
        );
        return ExitCode::from(2);
    }
    let ir = match fs::read_to_string(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Failed to read projection IR: {error}");
            return ExitCode::FAILURE;
        }
    };
    let header = match fs::read_to_string(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Failed to read native ABI header: {error}");
            return ExitCode::FAILURE;
        }
    };
    let existing = match fs::read_to_string(&args[2]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Failed to read existing ABI baseline: {error}");
            return ExitCode::FAILURE;
        }
    };
    match avalonia_bindgen::regenerate_baseline(&ir, &header, &existing) {
        Ok(json) => {
            if let Err(error) = fs::write(&args[2], json) {
                eprintln!("Failed to write ABI baseline: {error}");
                return ExitCode::FAILURE;
            }
            println!("Regenerated the released ABI baseline.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Failed to regenerate ABI baseline: {error}");
            ExitCode::FAILURE
        }
    }
}
