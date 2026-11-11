# rustolonia-macros

Zero-dependency procedural macros re-exported by [`rustolonia`](../rustolonia):

- `rsx!`: Span-preserving declarative view builder supporting shorthand properties, attached properties (`Grid.row`, `DockPanel.dock`, `ToolTip.tip`), static `if`/`for`, reactive `show`/`for each` blocks, and user-defined components.
- `#[component]`: Turns a Rust function into a reusable declarative component usable directly in `rsx!`.
- `color!`: Compile-time validated CSS/RGBA hex color literals (`color!("#1e293b")`, `color!("#0f172acc")`).
- `tracks!`: Compile-time validated `GridTracks` definitions (`tracks![*, Auto@Labels, 120, 2.5*]`).
- `fmt!`: Reactive format closure (`move || format!(...)`) for `TextBlock` and string properties.
- `#[derive(Lens)]` and `lens!`: Reactive `PropertyPath` projections for nested Rust structs.
