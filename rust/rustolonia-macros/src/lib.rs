//! Zero-dependency procedural macros for `rustolonia`.

extern crate proc_macro;

mod component;
mod lens;
mod literals;
mod rsx;
mod tokens;

use proc_macro::TokenStream;

/// Span-preserving declarative XAML-like builder for `El<T>` and `#[component]` views.
///
/// Supports:
/// - Properties (`width: 120.0,`) and struct-field shorthand (`spacing,`)
/// - Attached properties (`Grid.row: 1`, `DockPanel.dock: Dock::Top`, `ToolTip.tip: "Help"`)
/// - Node references (`ref: input_ref`)
/// - Static control flow (`if cond { ... } else { ... }`, `for item in iter { ... }`)
/// - Reactive control flow (`show cond { ... } else { ... }`, `for each row in rows key |r| r.id { ... }`)
/// - String/expression children (`"Submit"`, `{ expr }`) and custom `#[component]` elements.
#[proc_macro]
pub fn rsx(input: TokenStream) -> TokenStream {
    match rsx::expand_rsx(input) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error(),
    }
}

/// Transforms a Rust UI function into a declarative component usable inside `rsx!`.
///
/// Parameters become fluent builder methods accepting `IntoValue<T>`. Mark optional
/// defaults with `#[prop(default)]` or `#[prop(default = expr)]`; `Option<T>`
/// parameters default to `None` automatically, and a `children: Vec<AnyEl>`
/// parameter collects nested `rsx!` child elements.
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    match component::expand_component(attr, item) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error(),
    }
}

/// Compile-time validated CSS/Rust RGBA hex color literal.
///
/// Accepts `"#RGB"`, `"#RGBA"` (`R, G, B, A` order), `"#RRGGBB"`, or `"#RRGGBBAA"`
/// (`R, G, B, A` order) and expands to a `const`-compatible `::rustolonia::Color`
/// constructor.
#[proc_macro]
pub fn color(input: TokenStream) -> TokenStream {
    match literals::expand_color(input) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error(),
    }
}

/// Compile-time validated `GridTracks` builder.
///
/// Accepts comma-separated track tokens (`tracks![*, Auto, 120, 2.5*, Auto@Group]`)
/// or a string literal (`tracks!["*,Auto,120"]`).
#[proc_macro]
pub fn tracks(input: TokenStream) -> TokenStream {
    match literals::expand_tracks(input) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error(),
    }
}

/// Shorthand for a reactive formatted string closure (`move || ::std::format!(...)`).
#[proc_macro]
pub fn fmt(input: TokenStream) -> TokenStream {
    literals::expand_fmt(input)
}

/// Generates `lens_<field>(signal: Signal<Self>) -> PropertyPath<FieldType>`
/// helpers for every named field of a struct.
#[proc_macro_derive(Lens)]
pub fn derive_lens(input: TokenStream) -> TokenStream {
    match lens::expand_derive_lens(input) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error(),
    }
}

/// Builds a read-only (`lens!(sig => a.b)`) or writable (`lens!(mut sig => a.b)`)
/// reactive `PropertyPath` into a `Signal<Struct>`.
#[proc_macro]
pub fn lens(input: TokenStream) -> TokenStream {
    match lens::expand_lens(input) {
        Ok(ts) => ts,
        Err(err) => err.to_compile_error(),
    }
}
