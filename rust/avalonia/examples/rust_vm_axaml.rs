mod support;

fn main() -> avalonia::Result<()> {
    support::sample_app::run(|scope, model, converters| {
        avalonia_sample::mount_rust_vm_window_with_converters(scope, model, converters)
    })
}
