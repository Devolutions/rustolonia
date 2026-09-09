use super::desktop_files::{DesktopFiles, ACCEPTED_DROP_EFFECTS};
use super::rust_vm_model::{Converters, Model};
use avalonia::{App, AppScope, Result};
use std::sync::Arc;

/// Mounts the flagship Rust-owned view model and wires the platform-neutral
/// desktop file integration around it: pickers parented to the mounted window,
/// an incoming file drop target, and startup/open-with activation.
///
/// `App::run` forwards this process's own arguments to the managed desktop
/// lifetime by default, so launching the sample with file paths (or through a
/// registered "open with" association) populates the activation list.
pub fn run(
    mount: impl FnOnce(&AppScope, Model, Converters) -> Result<()> + Send + 'static,
) -> Result<()> {
    App::load_from_env()?.run(move |scope| {
        let files = DesktopFiles::new();
        mount(scope, Model::with_desktop_files(files.clone()), Converters)?;

        let window = scope
            .main_window()
            .expect("mounting the Rust view model creates its window");
        files.attach(scope, window.clone());

        // Incoming drag and drop. The accepted effect is declared here, once;
        // the host answers the platform drag loop with it synchronously and
        // delivers the notification to Rust asynchronously.
        let drop_files: Arc<DesktopFiles> = files.clone();
        scope.on_file_drop(&window, ACCEPTED_DROP_EFFECTS, move |event| {
            drop_files.publish_drop_event(&event);
        })?;

        // Startup "open with": the normalized activation items derived from the
        // process arguments this Rust executable forwarded to the host.
        files.publish_activation_items(&scope.activation_items()?);

        // Later activation, where the desktop lifetime supports it (macOS
        // "open with" while running, protocol activation, dock reopen).
        let activation_files: Arc<DesktopFiles> = files.clone();
        scope.on_activation(move |event| {
            activation_files.publish_activation_event(&event);
        })?;

        Ok(())
    })
}
