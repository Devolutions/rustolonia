#[cfg(target_os = "macos")]
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "macos")]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let host_path = rustolonia::discover_host_path()?;
    let diagnostic_host = rustolonia_sys::Host::load(host_path)?;
    let app = rustolonia::App::load_from_env()?;
    let shutdown_result = std::sync::Arc::new(std::sync::Mutex::new(None));
    app.run({
        let shutdown_result = shutdown_result.clone();
        move |scope| {
            let callback_scope = scope.clone();
            scope.post(move || {
                *shutdown_result
                    .lock()
                    .expect("shutdown result lock poisoned") =
                    Some(callback_scope.shutdown().map_err(|error| error.to_string()));
            })
        }
    })
    .map_err(|error| {
        let detail = diagnostic_host
            .last_error()
            .unwrap_or_else(|| "<none>".into());
        format!("run the native host: {error}; managed exception: {detail}")
    })?;
    match shutdown_result
        .lock()
        .expect("shutdown result lock poisoned")
        .take()
    {
        Some(Ok(())) => {}
        Some(Err(error)) => {
            let detail = diagnostic_host
                .last_error()
                .unwrap_or_else(|| "<none>".into());
            return Err(
                format!("shutdown the native host: {error}; managed exception: {detail}").into(),
            );
        }
        None => return Err("native host exited before the shutdown callback ran".into()),
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("native_host_shutdown is a macOS-only integration harness");
    std::process::exit(2);
}
