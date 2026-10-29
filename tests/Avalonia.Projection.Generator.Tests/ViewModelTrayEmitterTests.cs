using System;
using Avalonia.Projection.Ir;
using Xunit;

namespace Avalonia.Projection.Generator.Tests;

/// <summary>
/// Stage 33 generated tray icons: the tray menu factory, the attach method the
/// registry calls, and the Rust mount API.
/// </summary>
public class ViewModelTrayEmitterTests
{
    [Fact]
    public void A_tray_menu_builds_a_native_menu_without_key_bindings()
    {
        var menus = ViewModelSourceEmitter.EmitCSharp(TrayIr())["ShellViewModelMenus.g.cs"];

        Assert.Contains("public static NativeMenu CreateNotify(ShellViewModelAdapter model, RustMenuScope scope)", menus, StringComparison.Ordinal);
        Assert.DoesNotContain("keyBindings", Section(menus, "CreateNotify("), StringComparison.Ordinal);
    }

    [Fact]
    public void A_tray_attaches_its_icon_tooltip_and_activate_command()
    {
        var menus = ViewModelSourceEmitter.EmitCSharp(TrayIr())["ShellViewModelMenus.g.cs"];

        Assert.Contains("public static RustTrayAttachment AttachNotifyTray(ShellViewModelAdapter model)", menus, StringComparison.Ordinal);
        Assert.Contains(
            "RustTrayAttachment.Attach(menu, \"avares://Tests/Assets/app.ico\", \"Tests & co\", model.ShowCommand, scope, model)",
            menus,
            StringComparison.Ordinal);
    }

    [Fact]
    public void The_registry_creates_trays_by_id_with_their_own_adapter()
    {
        var registry = ViewModelSourceEmitter.EmitCSharp(TrayIr())["RustViewRegistry.g.cs"];

        Assert.Contains("internal static IDisposable CreateTray(int trayId, IAvnRustViewModel model) => trayId switch", registry, StringComparison.Ordinal);
        Assert.Contains("3 => global::Tests.ShellViewModelMenus.AttachNotifyTray(new global::Tests.ShellViewModelAdapter(model)),", registry, StringComparison.Ordinal);
    }

    [Fact]
    public void A_registry_without_trays_still_declares_create_tray()
    {
        var plain = new ViewModelIr { Models = TrayIr().Models };
        var registry = ViewModelSourceEmitter.EmitCSharp(plain)["RustViewRegistry.g.cs"];

        Assert.Contains("internal static IDisposable CreateTray(int trayId, IAvnRustViewModel model) => trayId switch", registry, StringComparison.Ordinal);
    }

    [Fact]
    public void Rust_gets_a_named_mount_for_each_tray()
    {
        var ir = TrayIr();

        Assert.Contains(
            "pub fn mount_notify(&self, model: impl ShellViewModel) -> crate::Result<()> { self.mount_dynamic_tray(3, ShellViewModelDispatch { model }) }",
            ViewModelSourceEmitter.EmitRust(ir),
            StringComparison.Ordinal);
        Assert.Contains(
            "pub fn mount_notify(scope: &crate::AppScope, model: impl ShellViewModel) -> crate::Result<()> { scope.mount_dynamic_tray(3, ShellViewModelDispatch { model }) }",
            ViewModelSourceEmitter.EmitRust(ir, externalConsumer: true),
            StringComparison.Ordinal);
    }

    [Fact]
    public void Each_view_also_gets_a_hidden_mount()
    {
        var ir = new ViewModelIr
        {
            Models = TrayIr().Models,
            Views = [new ViewDefinition { Id = 1, Name = "MainWindow", Model = "ShellViewModel", ManagedTypeName = "Tests.MainWindow" }],
        };

        Assert.Contains(
            "pub fn mount_main_window_hidden(scope: &crate::AppScope, model: impl ShellViewModel) -> crate::Result<()> { scope.mount_dynamic_view_model_hidden(1, ShellViewModelDispatch { model }).map(drop) }",
            ViewModelSourceEmitter.EmitRust(ir, externalConsumer: true),
            StringComparison.Ordinal);
    }

    /// The text of the generated method starting at <paramref name="signature"/>.
    private static string Section(string source, string signature)
    {
        var start = source.IndexOf(signature, StringComparison.Ordinal);
        Assert.True(start >= 0, signature);
        var end = source.IndexOf("\n    }\n", start, StringComparison.Ordinal);
        return source[start..(end < 0 ? source.Length : end)];
    }

    private static ViewModelIr TrayIr() => new()
    {
        Models =
        [
            new ViewModelDefinition
            {
                Id = 1,
                Name = "ShellViewModel",
                ManagedNamespace = "Tests",
                Properties = [new ViewModelProperty { Id = 1, Name = "Paused", Kind = ViewModelValueKind.Boolean, Writable = true }],
                Commands = [new ViewModelCommand { Id = 1, Name = "Show" }, new ViewModelCommand { Id = 2, Name = "Exit" }],
                Menus =
                [
                    new ViewModelMenu
                    {
                        Id = 1,
                        Name = "Notify",
                        Kind = ViewModelMenuKind.Tray,
                        Items =
                        [
                            new ViewModelMenuItem { Id = 1, Name = "Paused", Kind = ViewModelMenuItemKind.Toggle, Header = "Paused", ToggleProperty = "Paused" },
                            new ViewModelMenuItem { Id = 2, Name = "Gap", Kind = ViewModelMenuItemKind.Separator },
                            new ViewModelMenuItem { Id = 3, Name = "Exit", Kind = ViewModelMenuItemKind.Command, Header = "E_xit", Command = "Exit" },
                        ],
                    },
                ],
            },
        ],
        Trays =
        [
            new TrayDefinition { Id = 3, Name = "Notify", Model = "ShellViewModel", Menu = "Notify", Icon = "avares://Tests/Assets/app.ico", ToolTip = "Tests & co", ActivateCommand = "Show" },
        ],
    };
}
