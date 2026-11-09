# rustolonia-bindgen

Code generator for [rustolonia](https://github.com/Devolutions/rustolonia). It
reads the projection IR (`projection.ir.json`) exported from the patched
Avalonia host and emits the raw vtables and GUIDs in `rustolonia-sys` and the
safe wrappers in `rustolonia`. It also provides the compile-time
AXAML-to-Rust transpiler for the limited standalone markup subset.

Applications using that subset can use this crate as a `build-dependency` and
call `compile_axaml_from_json` from `build.rs`; the matching projection IR is
required as input. The [AXAML template](../../templates/axaml/README.md)
demonstrates the standalone workflow. This is not Avalonia's managed AXAML
compiler and does not provide full AXAML compatibility.

```text
rustolonia-bindgen [--check] <projection.ir.json> <sys-output> [safe-output]
rustolonia-bindgen --compile-axaml <projection.ir.json> <input.axaml> <output.rs>
rustolonia-bindgen --audit-axaml <projection.ir.json> <input-directory> <report.json>
```

`--check` compares instead of writing and fails if the outputs are stale.

The AXAML audit recursively reports which files the transpiler accepted and
which it rejected. `transpilerAccepted` means only that Rust source was emitted;
it does not mean that generated Rust type-checks or that the view behaves like
Avalonia at runtime. Unsupported markup extensions and binding clauses fail
instead of being silently dropped. The audit is a source-tree report and does
not evaluate platform- or configuration-conditioned MSBuild items.

The Rust transpiler accepts `{UIRes Key}` on string properties and emits a
`Signal<String>` field in the generated bindings. Consumers can populate it
with `LocalizationResources::string_signal`; this is a Rust-owned localization
catalog. `StaticResource` and `DynamicResource` are accepted on scalar
properties as typed signal fields, which the app must populate. The Rust-owned
`ResourceScope` offers explicit hierarchy, merged-dictionary, and theme lookup;
the transpiler does not load or scope AXAML dictionaries automatically.

`ItemsSource` without an inline template maps to an application-supplied
`ItemsView`, optionally backed by a Rust `ViewRegistry`. An inline
`ItemTemplate` can be lowered through `El::items_for` when it contains one
`DataTemplate` with `x:DataType` set to a Rust type path and `x:Key` naming a
stable key field. Simple one-way item property bindings map PascalCase paths
to snake_case fields and update through each item's signal. This eagerly
materializes controls; it is not the managed template or virtualization system.
Named elements in an inline item template expose per-instance Rust `NodeRef`s
through an `ItemNameScope` binding. Entries are removed on item disposal; this
does not implement Avalonia name scopes or `ElementName` bindings.
`Content="{Binding ChildView}"` on a `ContentControl` maps to a `ControlSlot`
that Rust can bind to a `ViewRegistry`. Arbitrary object-content bindings and
interactive item-template behavior are not yet supported.

Root-model dotted bindings use an explicit Rust `PropertyPath`; signal reads
track intermediate replacements, and two-way paths require an application
writer. One-way command bindings use Rust `Command` values, with string
literal or one-way `Variant` parameters and reactive `CanExecute` support.

Scalar bindings can reference an application-registered converter via
`{x:Static Type.Member}` or `{StaticResource Key}`. Rust supplies the declared
source kind, dispatch, culture, and optional writer for `ConvertBack`. Literal
converter parameters and culture overrides are supported; `Unset` fallback,
converter-plus-`StringFormat`, and converters within item templates are not.
The [template documentation](../../templates/axaml/README.md#supported-markup)
describes the supported types and error behavior.

`compile_axaml_from_json_with_options` and `compile_axaml_to_rust_with_options`
also accept explicit application-owned root aliases, component builders,
generic type mappings, and verified binding defaults. Registration uses
resolved XML namespace URIs. Custom properties currently accept direct signal
bindings with explicitly declared modes; they do not load CLR controls. Grid
definition elements support literal lengths and shared-size groups through a
matching host. See the [component example](../../templates/axaml/README.md#rust-owned-components)
for the contract and its limitations.

Licensed under MIT.
