using System;
using System.Linq;
using Avalonia.Projection.Ir;
using Xunit;

namespace Avalonia.Projection.Generator.Tests;

/// <summary>
/// Stage 31 generated command surfaces: menus, accelerators, context menus,
/// recent files and the declared display projection.
/// </summary>
public class ViewModelMenuEmitterTests
{
    [Fact]
    public void Rejects_enum_name_colliding_with_model_menu_output()
    {
        var sample = MenuIr();
        var ir = new ViewModelIr
        {
            Models = sample.Models,
            Views = sample.Views,
            Enums = sample.Enums.Concat(
            [
                new ViewModelEnumDefinition
                {
                    Id = 2,
                    Name = "ShellViewModelMenus",
                    ManagedNamespace = "Other.Namespace",
                    Members = [new ViewModelEnumMember { Name = "Value", Value = 0 }],
                },
            ]).ToArray(),
        };

        var error = Assert.Throws<InvalidOperationException>(() => ViewModelSourceEmitter.EmitCSharp(ir));
        Assert.Contains("Duplicate generated output 'ShellViewModelMenus.g.cs'", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Emits_a_menu_file_only_for_models_that_declare_menus()
    {
        var files = ViewModelSourceEmitter.EmitCSharp(MenuIr());

        Assert.True(files.ContainsKey("ShellViewModelMenus.g.cs"));
        Assert.False(files.ContainsKey("RowViewModelMenus.g.cs"));
    }

    [Fact]
    public void Menu_generation_is_deterministic()
    {
        var ir = MenuIr();

        Assert.Equal(
            ViewModelSourceEmitter.EmitCSharp(ir)["ShellViewModelMenus.g.cs"],
            ViewModelSourceEmitter.EmitCSharp(ir)["ShellViewModelMenus.g.cs"]);
        Assert.Equal(ViewModelSourceEmitter.EmitRust(ir), ViewModelSourceEmitter.EmitRust(ir));
        Assert.Equal(ViewModelSourceEmitter.EmitContract(ir), ViewModelSourceEmitter.EmitContract(ir));
    }

    [Fact]
    public void Application_menu_uses_named_apis_not_raw_identifiers()
    {
        var source = ViewModelSourceEmitter.EmitCSharp(MenuIr())["ShellViewModelMenus.g.cs"];

        Assert.Contains("public static NativeMenu CreateMain(ShellViewModelAdapter model, RustMenuScope scope, IList<KeyBinding>? keyBindings = null)", source, StringComparison.Ordinal);
        Assert.Contains("public static RustMenuAttachment AttachMain(TopLevel target, ShellViewModelAdapter model)", source, StringComparison.Ordinal);
        Assert.Contains("model.OpenCommand.Execute(parameter);", source, StringComparison.Ordinal);
        Assert.Contains("new NativeMenuItemSeparator()", source, StringComparison.Ordinal);
    }

    [Fact]
    public void Declared_gestures_become_both_a_menu_gesture_and_a_key_binding()
    {
        var source = ViewModelSourceEmitter.EmitCSharp(MenuIr())["ShellViewModelMenus.g.cs"];

        Assert.Contains("openItem.Gesture = RustMenu.ParseGesture(\"Ctrl+O\");", source, StringComparison.Ordinal);
        Assert.Contains("keyBindings?.Add(new KeyBinding { Gesture = RustMenu.ParseGesture(\"Ctrl+O\")!, Command = openCommand });", source, StringComparison.Ordinal);
    }

    [Fact]
    public void Toggle_and_radio_items_write_and_mirror_their_declared_property()
    {
        var source = ViewModelSourceEmitter.EmitCSharp(MenuIr())["ShellViewModelMenus.g.cs"];

        Assert.Contains("model.ShowDetails = !model.ShowDetails;", source, StringComparison.Ordinal);
        Assert.Contains("scope.Observe(\"ShowDetails\", () => detailsItem.IsChecked = model.ShowDetails);", source, StringComparison.Ordinal);
        Assert.Contains("model.Level = global::Tests.Level.High;", source, StringComparison.Ordinal);
        Assert.Contains("highItem.IsChecked = model.Level == global::Tests.Level.High", source, StringComparison.Ordinal);
        Assert.Contains("MenuItemToggleType.Radio", source, StringComparison.Ordinal);
    }

    [Fact]
    public void An_enabled_property_gates_the_command_rather_than_assigning_is_enabled()
    {
        var source = ViewModelSourceEmitter.EmitCSharp(MenuIr())["ShellViewModelMenus.g.cs"];

        // NativeMenuItem/MenuItem recompute IsEnabled from the command, so the
        // declared Boolean must compose into CanExecute instead.
        Assert.Contains("model.SaveCommand.CanExecute(null) && model.CanSave", source, StringComparison.Ordinal);
        Assert.Contains("scope.Observe(\"CanSave\", saveCommand.RaiseCanExecuteChanged);", source, StringComparison.Ordinal);
        Assert.DoesNotContain("saveItem.IsEnabled =", source, StringComparison.Ordinal);
    }

    [Fact]
    public void A_context_menu_is_a_generated_control_usable_from_compiled_axaml()
    {
        var source = ViewModelSourceEmitter.EmitCSharp(MenuIr())["ShellViewModelMenus.g.cs"];

        Assert.Contains("public sealed class ShellViewModelRowsContextMenu : ContextMenu", source, StringComparison.Ordinal);
        Assert.Contains("protected override void OnDataContextChanged(EventArgs e)", source, StringComparison.Ordinal);
        Assert.Contains("ItemsSource = ShellViewModelMenus.CreateRowsItems(model, scope);", source, StringComparison.Ordinal);
        Assert.Contains("new MenuItem { Header = \"Copy\", Command = copyCommand }", source, StringComparison.Ordinal);
        Assert.Contains("new Separator()", source, StringComparison.Ordinal);
    }

    [Fact]
    public void An_accelerator_menu_produces_key_bindings_and_no_visual_items()
    {
        var source = ViewModelSourceEmitter.EmitCSharp(MenuIr())["ShellViewModelMenus.g.cs"];

        Assert.Contains("public static IReadOnlyList<KeyBinding> CreateShortcuts(ShellViewModelAdapter model, RustMenuScope scope)", source, StringComparison.Ordinal);
        Assert.Contains("public static IDisposable AttachShortcuts(InputElement target, ShellViewModelAdapter model)", source, StringComparison.Ordinal);
        Assert.Contains("keyBindings.Add(new KeyBinding { Gesture = RustMenu.ParseGesture(\"Ctrl+S\")!, Command = quickSaveCommand });", source, StringComparison.Ordinal);
        Assert.DoesNotContain("var quickSaveItem", source, StringComparison.Ordinal);
    }

    [Fact]
    public void Recent_files_reuse_the_string_collection_transport_with_named_rust_apis()
    {
        var ir = MenuIr();
        var source = ViewModelSourceEmitter.EmitCSharp(ir)["ShellViewModelMenus.g.cs"];
        var rust = ViewModelSourceEmitter.EmitRust(ir);

        Assert.Contains("public const int RecentFilesCapacity = 4;", source, StringComparison.Ordinal);
        Assert.Contains("RustMenu.FillRecentFiles(recentMenu.Items, model.Recent, recentActivate, RecentFilesEmptyHeader, RecentFilesCapacity)", source, StringComparison.Ordinal);
        Assert.Contains("pub const SHELL_VIEW_MODEL_RECENT_CAPACITY: usize = 4;", rust, StringComparison.Ordinal);
        Assert.Contains("pub fn publish_recent(&self, recent: &crate::RecentFileList) -> crate::Result<()> {", rust, StringComparison.Ordinal);
        Assert.Contains("pub fn set_recent(&mut self, recent: &crate::RecentFileList) { self.0.push_string_snapshot(2, recent.entries()); }", rust, StringComparison.Ordinal);

        // The activate command receives the chosen URI as its command parameter.
        Assert.Contains("fn open_recent(&mut self, value: String) -> crate::Result<()>;", rust, StringComparison.Ordinal);
    }

    [Fact]
    public void A_declared_display_path_becomes_a_to_string_override()
    {
        var files = ViewModelSourceEmitter.EmitCSharp(MenuIr());

        Assert.Contains("public override string ToString() => Label;", files["RowViewModelAdapter.g.cs"], StringComparison.Ordinal);
        Assert.DoesNotContain("public override string ToString()", files["ShellViewModelAdapter.g.cs"], StringComparison.Ordinal);
    }

    [Fact]
    public void A_traversing_display_path_stays_null_safe()
    {
        var ir = MenuIr();
        var models = ir.Models.ToList();
        models[1] = new ViewModelDefinition
        {
            Id = models[1].Id,
            Name = models[1].Name,
            ManagedNamespace = models[1].ManagedNamespace,
            Properties =
            [
                new ViewModelProperty { Id = 1, Name = "Label", Kind = ViewModelValueKind.String },
                new ViewModelProperty { Id = 2, Name = "Detail", Kind = ViewModelValueKind.Model, ModelName = "DetailViewModel", Nullable = true },
            ],
            DisplayPath = "Detail.Text",
        };
        models.Add(new ViewModelDefinition
        {
            Id = 3,
            Name = "DetailViewModel",
            ManagedNamespace = "Tests",
            Properties = [new ViewModelProperty { Id = 1, Name = "Text", Kind = ViewModelValueKind.String }],
        });
        var traversing = new ViewModelIr { Enums = ir.Enums, Models = models, Views = ir.Views };

        Assert.Contains(
            "public override string ToString() => Detail?.Text ?? \"\";",
            ViewModelSourceEmitter.EmitCSharp(traversing)["RowViewModelAdapter.g.cs"],
            StringComparison.Ordinal);
    }

    [Fact]
    public void Table_columns_project_declared_width_limits_into_compiled_axaml_columns()
    {
        var ir = TableIr();
        var metadata = ViewModelSourceEmitter.EmitCSharp(ir)["ShellViewModelMetadata.g.cs"];

        Assert.Contains("MinWidth = 80D", metadata, StringComparison.Ordinal);
        Assert.Contains("MaxWidth = 400D", metadata, StringComparison.Ordinal);
    }

    [Fact]
    public void The_contract_reports_menus_recent_files_and_the_display_path()
    {
        var contract = ViewModelSourceEmitter.EmitContract(MenuIr());

        Assert.Contains("### Application menu `Main` (`1`)", contract, StringComparison.Ordinal);
        Assert.Contains("### Context menu `Rows` (`2`)", contract, StringComparison.Ordinal);
        Assert.Contains("### Accelerators menu `Shortcuts` (`3`)", contract, StringComparison.Ordinal);
        Assert.Contains("### Recent files `Recent`", contract, StringComparison.Ordinal);
        Assert.Contains("Display projection (`ToString()`): `Label`.", contract, StringComparison.Ordinal);
        Assert.Contains("`Ctrl+O`", contract, StringComparison.Ordinal);
    }

    private static ViewModelIr TableIr() => new()
    {
        Models =
        [
            new ViewModelDefinition
            {
                Id = 1,
                Name = "ShellViewModel",
                ManagedNamespace = "Tests",
                Collections =
                [
                    new ViewModelCollection
                    {
                        Id = 1,
                        Name = "Rows",
                        ElementKind = ViewModelValueKind.Model,
                        ElementModelName = "RowViewModel",
                        Table = new ViewModelTable
                        {
                            Columns =
                            [
                                new ViewModelTableColumn
                                {
                                    Id = 1, Name = "Label", Header = "Label", Path = "Label",
                                    Star = true, MinWidth = 80, MaxWidth = 400,
                                },
                            ],
                        },
                    },
                ],
            },
            new ViewModelDefinition
            {
                Id = 2,
                Name = "RowViewModel",
                ManagedNamespace = "Tests",
                Properties = [new ViewModelProperty { Id = 1, Name = "Label", Kind = ViewModelValueKind.String }],
            },
        ],
    };

    private static ViewModelIr MenuIr() => new()
    {
        Enums =
        [
            new ViewModelEnumDefinition
            {
                Id = 1,
                Name = "Level",
                ManagedNamespace = "Tests",
                Members = [new() { Name = "Low", Value = 0 }, new() { Name = "High", Value = 1 }],
            },
        ],
        Models =
        [
            new ViewModelDefinition
            {
                Id = 1,
                Name = "ShellViewModel",
                ManagedNamespace = "Tests",
                Properties =
                [
                    new ViewModelProperty { Id = 1, Name = "ShowDetails", Kind = ViewModelValueKind.Boolean, Writable = true },
                    new ViewModelProperty { Id = 2, Name = "CanSave", Kind = ViewModelValueKind.Boolean },
                    new ViewModelProperty { Id = 3, Name = "Level", Kind = ViewModelValueKind.Enum, EnumName = "Level", Writable = true },
                ],
                Collections =
                [
                    new ViewModelCollection { Id = 1, Name = "Rows", ElementKind = ViewModelValueKind.Model, ElementModelName = "RowViewModel" },
                    new ViewModelCollection { Id = 2, Name = "Recent", ElementKind = ViewModelValueKind.String },
                ],
                Commands =
                [
                    new ViewModelCommand { Id = 1, Name = "Open" },
                    new ViewModelCommand { Id = 2, Name = "Save" },
                    new ViewModelCommand { Id = 3, Name = "OpenRecent" },
                    new ViewModelCommand { Id = 4, Name = "Copy" },
                    new ViewModelCommand { Id = 5, Name = "QuickSave" },
                ],
                RecentFiles = new ViewModelRecentFiles { Id = 1, Collection = "Recent", ActivateCommand = "OpenRecent", Capacity = 4 },
                Menus =
                [
                    new ViewModelMenu
                    {
                        Id = 1,
                        Name = "Main",
                        Kind = ViewModelMenuKind.Application,
                        Items =
                        [
                            new ViewModelMenuItem
                            {
                                Id = 1, Name = "File", Kind = ViewModelMenuItemKind.Submenu, Header = "File",
                                Items =
                                [
                                    new ViewModelMenuItem { Id = 2, Name = "Open", Kind = ViewModelMenuItemKind.Command, Header = "Open", Command = "Open", Gesture = "Ctrl+O" },
                                    new ViewModelMenuItem { Id = 3, Name = "Save", Kind = ViewModelMenuItemKind.Command, Header = "Save", Command = "Save", IsEnabledProperty = "CanSave" },
                                    new ViewModelMenuItem { Id = 4, Name = "Recent", Kind = ViewModelMenuItemKind.RecentFiles, Header = "Recent" },
                                    new ViewModelMenuItem { Id = 5, Name = "Gap", Kind = ViewModelMenuItemKind.Separator },
                                    new ViewModelMenuItem { Id = 6, Name = "Details", Kind = ViewModelMenuItemKind.Toggle, Header = "Details", ToggleProperty = "ShowDetails" },
                                    new ViewModelMenuItem { Id = 7, Name = "High", Kind = ViewModelMenuItemKind.Radio, Header = "High", RadioProperty = "Level", RadioValue = "High" },
                                ],
                            },
                        ],
                    },
                    new ViewModelMenu
                    {
                        Id = 2,
                        Name = "Rows",
                        Kind = ViewModelMenuKind.Context,
                        Items =
                        [
                            new ViewModelMenuItem { Id = 1, Name = "Copy", Kind = ViewModelMenuItemKind.Command, Header = "Copy", Command = "Copy" },
                            new ViewModelMenuItem { Id = 2, Name = "Gap", Kind = ViewModelMenuItemKind.Separator },
                            new ViewModelMenuItem { Id = 3, Name = "Details", Kind = ViewModelMenuItemKind.Toggle, Header = "Details", ToggleProperty = "ShowDetails" },
                        ],
                    },
                    new ViewModelMenu
                    {
                        Id = 3,
                        Name = "Shortcuts",
                        Kind = ViewModelMenuKind.Accelerators,
                        Items =
                        [
                            new ViewModelMenuItem { Id = 1, Name = "QuickSave", Kind = ViewModelMenuItemKind.Command, Header = "Quick save", Command = "QuickSave", Gesture = "Ctrl+S" },
                        ],
                    },
                ],
            },
            new ViewModelDefinition
            {
                Id = 2,
                Name = "RowViewModel",
                ManagedNamespace = "Tests",
                Properties = [new ViewModelProperty { Id = 1, Name = "Label", Kind = ViewModelValueKind.String }],
                DisplayPath = "Label",
            },
        ],
    };
}
