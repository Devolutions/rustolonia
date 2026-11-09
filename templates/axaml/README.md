# Standalone AXAML app

This template transpiles a limited AXAML subset into Rust during Cargo's build
step and runs it against a matching prebuilt Avalonia NativeAOT host. Building
the app does not require the .NET SDK, an Avalonia checkout, a C# project, or
an application-specific managed host.

## Development setup

The template's `v12.1.5` dependency tag predates the compiler and runtime APIs
used here. Until a release includes these changes, use a checkout containing
this revision and override the tagged dependencies. The checkout needs neither
an initialized Avalonia submodule nor .NET tooling.

Copy this directory, including `.cargo`, `.gitignore`, `Cargo.lock`, and
`projection.ir.json`, to your app. In the copy, append the following to
`.cargo/config.toml`, replacing `/path/to/rustolonia` with the absolute path
to that checkout:

```toml
[patch."https://github.com/Devolutions/rustolonia"]
rustolonia = { path = "/path/to/rustolonia/rust/rustolonia" }
rustolonia-bindgen = { path = "/path/to/rustolonia/rust/rustolonia-bindgen" }
rustolonia-sys = { path = "/path/to/rustolonia/rust/rustolonia-sys" }
```

Install Rust 1.88 or newer and the native linker toolchain. In the copied
directory, update the lockfile to select the checkout's crates:

```sh
cargo update -p rustolonia -p rustolonia-bindgen -p rustolonia-sys
```

Obtain a prebuilt host built from the same revision as the checkout; the
published v12.1.5 host predates these changes. Point `RUSTOLONIA_HOST_DIR` at
the directory containing the host library before running `cargo run` from the
copied directory. For example:

```sh
export RUSTOLONIA_HOST_DIR=/absolute/path/to/matching-host
cargo run
```

In PowerShell, use `$env:RUSTOLONIA_HOST_DIR = 'C:\path\to\matching-host'`.
Unset `RUSTOLONIA_HOST_LIB` if it points to an older host. After updating the
lockfile, use `cargo run --locked`. Obtaining or building the matching host is
separate from building the Rust application.

Once a release includes these APIs, replace both git tags in `Cargo.toml` with
that release, copy its AXAML template's `projection.ir.json`, remove the local
overrides, and regenerate `Cargo.lock`. After unsetting `RUSTOLONIA_HOST_DIR`,
Cargo can download the checksum-verified release host. Keep the compiler, Rust
bindings, projection catalog, and host from the same revision; the host ABI
fingerprint is checked at load time.

## Supported markup

`src/MainWindow.axaml` and `src/FilterEditor.axaml` are compiled by `build.rs`
using the included projection catalog. The compiler supports projected controls and writable properties,
simple literals, `x:Name`, supported attached properties, event handlers, and
`{Binding}` or `{CompiledBinding}` expressions. Binding paths map to Rust
snake_case fields. Properties with a projected two-way builder
require an explicit `Mode=OneWay` or `Mode=TwoWay` unless the application
registers a default for the exact projected control and property. The projection
does not encode Avalonia's default binding mode, so the application must verify
any registered default. The example registers `TextBox.Text` as two-way.
Two-way bindings on other properties are rejected. Nullable Boolean properties use `Signal<Option<bool>>`; the
existing string builders normalize nullable strings to `String`. `StringFormat`
is limited to a string binding with exactly one `{0}` placeholder and cannot
be combined with `TwoWay`.

For a dotted root-model path such as `Profile.Title`, the generated
`profile_title` field is a `PropertyPath<String>`, not a reflected property
lookup. The application supplies a reader that accesses its Rust model and
signals; re-evaluation follows changed intermediate models and unsubscribes
from the old signals. Return an error for a missing intermediate value, or
choose an explicit fallback. For `Mode=TwoWay`, supply a writable
`PropertyPath` whose writer updates the current model.

One-way `Command` bindings use an application-supplied
`Signal<Option<Command>>`, or a `PropertyPath<Option<Command>>` for a dotted
path. `CommandParameter` accepts a literal string or a one-way `Variant`
binding; numeric-looking literals such as `3389` stay strings. A `Command`
can have a reactive global `CanExecute` gate and a parameter-specific
predicate. Call `command.notify()?` when dependencies of the predicate other
than the reactive gate change. Commands and parameters inside item templates
are not supported.

Converters accept registered `{x:Static Type.Member}` or
`{StaticResource Key}` references. The application supplies a `ScalarPath`
declaring the source type, a `RegisteredValueConverter`, and a
`ValueConverterDispatch` implementation; no CLR member or AXAML resource is
loaded automatically. The supported source kinds are string, Boolean, 64-bit
integer and double, and supported target properties are `String`, `bool`,
`Option<bool>`, `i32`, `i64` and `f64`. `ConverterParameter` is a literal
string, and `ConverterCulture` is a literal override of the registered
culture signal. Without an override, culture changes re-evaluate the binding.
Two-way conversion requires a writable source and a `ConvertBack`
implementation. `DoNothing` skips an update; `Unset`, type mismatches and
conversion errors are reported. Runtime update errors are logged; a rejected
control edit can remain visible and does not implement Avalonia validation or
fallback-value semantics. Converters cannot be combined with `StringFormat`
or used in item templates.

`x:Class` names the Rust view; root `x:DataType` and `x:CompileBindings` do not
load a C# viewmodel or perform managed compiled-binding checks. Declared
`mc:Ignorable` namespaces can contain design-only attributes and elements;
registered runtime namespaces are not ignored. `Design.DataContext` is skipped
as a complete design-only subtree.
`x:Name` sets the control's name and exposes a Rust `NodeRef`; it does not
implement Avalonia name scopes or `ElementName` bindings.

### Rust-owned components

`src/axaml_components.rs` supplies `AxamlCompileOptions` to
`compile_axaml_from_json_with_options`. Its registrations match the resolved
XML namespace URI and element name, not the chosen prefix. The example maps
one self-typed view root to `UserControl`, maps `FormGrid` and `EnumPicker` to
Rust builders backed by projected `Grid` and `ComboBox`, and maps the enum
QName in `x:TypeArguments` to a Rust type. A root alias substitutes a control;
it does not reproduce the original base class or code-behind.

Custom properties in this subset accept direct model `Signal` bindings only.
Each registration explicitly declares the Rust value type, default binding
mode, and builder method for each supported mode. The Rust component is
responsible for implementing that behavior. Unknown elements, generic types,
properties, binding modes and unsupported binding clauses fail compilation.
The compiler validates the declared contract but cannot prove the replacement
matches an original CLR control.

The sample picker implements two-way selection, an observable filter, ordered
localized labels, and a reactive culture supplied through `ComponentContext`.
An empty filter permits all values; if a filter excludes the selected value,
the picker shows no selection but preserves the model. This is the sample's
explicit contract, not verified behavior of the RDM third-party picker for
excluded selections, default binding modes, or culture changes.

The sample's `FormGrid` replacement classifies direct children across multiple
rows and columns, including spans, overlaps, hidden children and ancestors,
definition changes, and child removal. It uses raw grid positions: hidden rows
are not renumbered, and the first visible child occupies each overlapping
cell. `Label`, `CheckBox`, and `TextBlock` are label-like; add the
`form-grid-label-like` class to a replacement for a custom label-like control.
As in the RDM algorithm, a fully shadowed child can retain previous classes.
Removal clears the grid classes and restores sample metrics immediately, even
when the grid is detached, so the child can be reparented.

Classification runs after a layout pass through `Grid.LayoutUpdated`, not
before measurement as in RDM, and changing the sample metrics can schedule
another pass. The sample uses local property values rather than Avalonia
styles: it can override application-local settings and restores the values
captured when a child was first tracked. `ComponentContext` supplies compact
height and right margin. `finish_form_grid_with_options` accepts additional
margins, `treat_empty_cells_as_controls`, and `apply_sample_styles = false`
when the application supplies its own styles. These options are Rust-side;
the example registration does not accept a `TreatEmptyCellsAsControls` AXAML
property. The replacement does not implement `Wrap`, all RDM input styles,
dynamic theme tokens, or the original control's full visual behavior.

`Grid.ColumnDefinitions` and `Grid.RowDefinitions` property elements support
literal `Width`/`Height` and optional `SharedSizeGroup`; other definition
properties are rejected. The matching host transports groups using
`Auto@Group,1*`-style definition strings. A shared group needs an ancestor
with `Grid.IsSharedSizeScope="True"` to affect layout. This host contract
requires the matching updated native host, even for an application built
without .NET.

`{UIRes Key}` on a string property becomes a `Signal<String>` supplied by
`LocalizationResources::string_signal`. The Rust catalog supports culture
changes, default-culture fallback, and explicit missing-key errors. Retain the
catalog with `AppScope::retain_localization_resources` while it is in use.

Scalar `StaticResource` and `DynamicResource` expressions become typed signal
fields. `StaticResource` reads a snapshot when the generated builder is
created; `DynamicResource` follows its signal. `ResourceScope` supplies
Rust-owned dictionaries with local, parent, theme, and reverse-order merged
lookup. Retain the relevant resource scope while its signals are in use. The
app must synchronize its theme with the host if desired, as the example does;
AXAML resource dictionaries are not loaded automatically. Signals created
inside a `ReactiveScope` are disconnected when that scope is disposed.

`Content="{Binding Child}"` on a `ContentControl` produces a `ControlSlot`.
After materializing the view, Rust can mount a child explicitly or call
`ControlSlot::bind` with a `ViewRegistry`. A model update with the same
registry key updates its existing view; changing keys replaces it. One
generated slot cannot be attached to multiple controls. Events attached while
building a view are unsubscribed when its reactive scope is disposed.

For `ItemsSource` without an inline template, supply an `ItemsView` built with
a Rust view factory or `ViewRegistry`. An inline `DataTemplate` instead needs
`x:DataType="crate::Model"` and `x:Key="Id"` as this compiler's explicit Rust
model and stable-key contract. The generated list materializes keyed controls
and binds simple item fields; it is not Avalonia's managed `DataTemplate` or a
virtualization mechanism. Names inside an item template
produce a separate `ItemNameScope` binding. Its `entries()` returns the item
signals and per-instance Rust references, which are removed when an item is
unmounted. Entries follow creation order rather than visual order; identify an
item by its model instead of its position. The example initializes
`contacts_item_refs_1` for the named contact text. After removal, a saved
`NodeRef` returns `None`; its item signal has been disposed and must not be
read.

An inline template can use an explicit `Mode=TwoWay` binding to a simple item
field when that Rust model field is itself a `Signal<T>` matching the control
property. The generated view binds directly to that signal; it does not mutate
a copied item field or infer Avalonia's managed property-change behavior.
Nested item paths, styles, item commands/events, managed code-behind, automatic
CLR control loading, and markup extensions beyond this subset are not
implemented. Unsupported
markup is rejected rather than silently treated as literal text or ignored.
Transpilation alone does not establish that generated Rust type-checks or that
a view behaves like its managed counterpart.

Maintainers can inventory a source tree with:

```sh
cargo run --manifest-path rust/Cargo.toml -p rustolonia-bindgen -- \
  --audit-axaml rust/projection.ir.json path/to/views axaml-audit.json
```

The report records transpiler acceptance only; it does not type-check output,
launch views, or evaluate MSBuild platform and configuration conditions.
