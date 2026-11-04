//! Desktop entry point, sharing `ui::build` with the Android library.

fn main() -> rustolonia::Result<()> {
    rustolonia::App::load_from_env()?.run(android_hello::ui::build)
}
