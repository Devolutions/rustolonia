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
    let tray_icon = std::env::args().skip(1).next();
    let shutdown_result = std::sync::Arc::new(std::sync::Mutex::new(None));
    app.run({
        let shutdown_result = shutdown_result.clone();
        move |scope| {
            let tray = if let Some(icon_path) = tray_icon {
                scope.mount(
                    rustolonia::Window::el()
                        .title("Native tray regression")
                        .build(scope)?,
                )?;
                let tray = scope.create_tray(|_| Ok(())).map_err(|error| {
                    let host = rustolonia_sys::Host::load(
                        rustolonia::discover_host_path().expect("find diagnostic host"),
                    )
                    .expect("load diagnostic host");
                    eprintln!(
                        "create tray: {error}; {}",
                        host.last_error().unwrap_or_default()
                    );
                    error
                })?;
                tray.set_icon(&icon_path)?;
                tray.set_tool_tip("Rustolonia native tray regression")?;
                tray.set_menu(&[
                    rustolonia::TrayMenuItem::Command {
                        id: 1,
                        header: "Shutdown".into(),
                        enabled: true,
                    },
                    rustolonia::TrayMenuItem::Check {
                        id: 2,
                        header: "Enabled".into(),
                        checked: true,
                        enabled: true,
                    },
                    rustolonia::TrayMenuItem::Separator,
                    rustolonia::TrayMenuItem::Submenu {
                        header: "More".into(),
                        items: vec![rustolonia::TrayMenuItem::Command {
                            id: 3,
                            header: "Child".into(),
                            enabled: true,
                        }],
                    },
                ])?;
                tray.update_item(2, Some("Updated"), false, true)?;
                tray.set_visible(false)?;
                tray.set_visible(true)?;
                Some(tray)
            } else {
                None
            };
            let callback_scope = scope.clone();
            scope.post(move || {
                *shutdown_result
                    .lock()
                    .expect("shutdown result lock poisoned") = Some(
                    (|| {
                        if let Some(tray) = tray {
                            tray.remove()?;
                            // Leave a second icon attached to exercise shutdown cleanup.
                            callback_scope.create_tray(|_| Ok(()))?;
                        }
                        callback_scope.shutdown()
                    })()
                    .map_err(|error| error.to_string()),
                );
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
