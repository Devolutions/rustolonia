mod ui;

fn main() -> rustolonia::Result<()> {
    rustolonia::App::load_from_env()?.run(ui::build)
}
