using System;
using Avalonia.Projection.Ir;
using Xunit;

namespace Avalonia.Projection.Ir.Tests;

/// <summary>
/// Stage 33 notification-area (tray) icons: a top-level tray list naming a
/// model, a Tray menu on it, an icon and an optional activate command.
/// </summary>
public class ViewModelTrayIrTests
{
    [Fact]
    public void A_tray_with_a_tray_menu_validates_and_round_trips()
    {
        var ir = Tray();
        ir.Validate();
        var again = ViewModelIr.FromJson(ir.ToJson());
        var tray = Assert.Single(again.TrayIcons);
        Assert.Equal("Notify", tray.Name);
        Assert.Equal("avares://Tests/Assets/app.ico", tray.Icon);
        Assert.Equal("Show", tray.ActivateCommand);
    }

    [Fact]
    public void A_schema_without_trays_serializes_without_a_trays_member()
    {
        var ir = Tray();
        var plain = new ViewModelIr { Models = ir.Models };
        Assert.DoesNotContain("\"trays\"", plain.ToJson(), StringComparison.OrdinalIgnoreCase);
        Assert.Empty(ViewModelIr.FromJson(plain.ToJson()).TrayIcons);
    }

    [Fact]
    public void A_tray_must_name_a_tray_menu()
    {
        var error = Assert.Throws<InvalidOperationException>(() => Tray(menuKind: ViewModelMenuKind.Application).Validate());
        Assert.Contains("must be a Tray menu", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void A_tray_must_name_known_models_menus_and_commands()
    {
        Assert.Contains("unknown model", Assert.Throws<InvalidOperationException>(() => Tray(model: "Missing").Validate()).Message);
        Assert.Contains("unknown menu", Assert.Throws<InvalidOperationException>(() => Tray(menu: "Missing").Validate()).Message);
        Assert.Contains("unknown activate command", Assert.Throws<InvalidOperationException>(() => Tray(activate: "Missing").Validate()).Message);
    }

    [Fact]
    public void A_tray_needs_an_icon()
    {
        Assert.Throws<InvalidOperationException>(() => Tray(icon: " ").Validate());
    }

    [Fact]
    public void A_tray_menu_rejects_gestures_even_in_submenus()
    {
        var error = Assert.Throws<InvalidOperationException>(() => Tray(gesture: "Ctrl+Q").Validate());
        Assert.Contains("must not declare gestures", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Trays_need_schema_version_five()
    {
        // A model with no menus of its own, so only the tray needs version 5.
        var downgraded = new ViewModelIr
        {
            Version = 4,
            Models = [new() { Id = 1, Name = "Shell", ManagedNamespace = "Tests", Commands = [new() { Id = 1, Name = "Show" }] }],
            Trays = Tray().Trays,
        };
        Assert.Contains("upgrade to version 5", Assert.Throws<InvalidOperationException>(() => downgraded.Validate()).Message);
    }

    private static ViewModelIr Tray(
        string model = "Shell",
        string menu = "Notify",
        string activate = "Show",
        string icon = "avares://Tests/Assets/app.ico",
        ViewModelMenuKind menuKind = ViewModelMenuKind.Tray,
        string? gesture = null) => new()
        {
            Models =
            [
                new()
                {
                    Id = 1,
                    Name = "Shell",
                    ManagedNamespace = "Tests",
                    Commands = [new() { Id = 1, Name = "Show" }, new() { Id = 2, Name = "Exit" }],
                    Menus =
                    [
                        new()
                        {
                            Id = 1,
                            Name = "Notify",
                            Kind = menuKind,
                            Items =
                            [
                                new()
                                {
                                    Id = 1, Name = "More", Kind = ViewModelMenuItemKind.Submenu, Header = "More",
                                    Items = [new() { Id = 2, Name = "Exit", Kind = ViewModelMenuItemKind.Command, Header = "Exit", Command = "Exit", Gesture = gesture }],
                                },
                            ],
                        },
                    ],
                },
            ],
            Trays = [new() { Id = 1, Name = "Notify", Model = model, Menu = menu, Icon = icon, ToolTip = "Tests", ActivateCommand = activate }],
        };
}
