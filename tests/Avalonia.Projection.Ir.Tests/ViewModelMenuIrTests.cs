using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Avalonia.Projection.Ir;
using Xunit;

namespace Avalonia.Projection.Ir.Tests;

/// <summary>
/// Schema-version 5 command surfaces: menus, keyboard accelerators, recent-file
/// lists and declared display paths.
/// </summary>
public class ViewModelMenuIrTests
{
    [Fact]
    public void Checked_in_schema_declares_the_stage_thirty_one_command_surfaces()
    {
        var ir = ViewModelIr.FromJson(File.ReadAllText(
            Path.Combine(FindRepositoryRoot(), "rust", "avalonia-sample", "view-model.ir.json")));
        var model = ir.Models.Single(candidate => candidate.Name == "SampleViewModel");

        Assert.Equal(5, ir.Version);
        Assert.Equal(3, model.Menus.Count);

        var main = model.Menus.Single(menu => menu.Name == "Main");
        Assert.Equal(ViewModelMenuKind.Application, main.Kind);
        var file = main.Items.Single(item => item.Name == "File");
        Assert.Equal(ViewModelMenuItemKind.Submenu, file.Kind);
        Assert.Equal("Ctrl+O", file.Items.Single(item => item.Name == "Open").Gesture);
        Assert.Contains(file.Items, item => item.Kind == ViewModelMenuItemKind.RecentFiles);
        Assert.Contains(file.Items, item => item.Kind == ViewModelMenuItemKind.Separator);

        var view = main.Items.Single(item => item.Name == "View");
        Assert.Equal("ShowTraceDetails", view.Items.Single(item => item.Name == "ShowDetails").ToggleProperty);
        Assert.Equal(3, view.Items.Count(item => item.Kind == ViewModelMenuItemKind.Radio));

        Assert.Equal(ViewModelMenuKind.Context, model.Menus.Single(menu => menu.Name == "TraceRows").Kind);
        Assert.Equal(ViewModelMenuKind.Accelerators, model.Menus.Single(menu => menu.Name == "Shortcuts").Kind);

        Assert.Equal("RecentFiles", model.RecentFiles!.Collection);
        Assert.Equal("OpenRecentFile", model.RecentFiles.ActivateCommand);
        Assert.Equal(8, model.RecentFiles.Capacity);
        Assert.Equal("Message", ir.Models.Single(candidate => candidate.Name == "TraceRowViewModel").DisplayPath);
    }

    [Fact]
    public void Version_four_hard_rejects_every_new_member()
    {
        AssertUpgradeRejected(Menu());
        AssertUpgradeRejected(new ViewModelIr { Models = [WithRecentFiles()] });
        AssertUpgradeRejected(new ViewModelIr
        {
            Models = [new()
            {
                Id = 1,
                Name = "Model",
                ManagedNamespace = "Tests",
                Properties = [new() { Id = 1, Name = "Label", Kind = ViewModelValueKind.String }],
                DisplayPath = "Label",
            }],
        });

        static void AssertUpgradeRejected(ViewModelIr ir)
        {
            ir.Validate();
            var downgraded = new ViewModelIr { Version = 4, Enums = ir.Enums, Models = ir.Models };
            Assert.Contains(
                "upgrade to version 5",
                Assert.Throws<InvalidOperationException>(() => downgraded.Validate()).Message);
        }
    }

    [Fact]
    public void A_menu_item_must_reference_a_declared_command()
    {
        var ir = Menu(item: Item(1, "Open", ViewModelMenuItemKind.Command, "Open", command: "Missing"));

        Assert.Contains(
            "unknown command 'Missing'",
            Assert.Throws<InvalidOperationException>(() => ir.Validate()).Message);
    }

    [Fact]
    public void A_separator_may_not_declare_anything_else()
    {
        var ir = Menu(item: Item(1, "Gap", ViewModelMenuItemKind.Separator, header: "Nope"));

        Assert.Contains(
            "separator",
            Assert.Throws<InvalidOperationException>(() => ir.Validate()).Message);
    }

    [Fact]
    public void A_toggle_must_bind_a_writable_boolean_property()
    {
        Assert.Contains(
            "must be a Boolean property",
            Assert.Throws<InvalidOperationException>(() => Menu(
                item: Item(1, "Toggle", ViewModelMenuItemKind.Toggle, "Toggle", toggleProperty: "Label")).Validate()).Message);

        Assert.Contains(
            "must be writable",
            Assert.Throws<InvalidOperationException>(() => Menu(
                item: Item(1, "Toggle", ViewModelMenuItemKind.Toggle, "Toggle", toggleProperty: "ReadOnlyFlag")).Validate()).Message);
    }

    [Fact]
    public void A_radio_value_must_be_a_member_of_the_bound_enum()
    {
        var ir = Menu(item: Item(1, "High", ViewModelMenuItemKind.Radio, "High", radioProperty: "Level", radioValue: "Missing"));

        Assert.Contains(
            "is not a member of enum",
            Assert.Throws<InvalidOperationException>(() => ir.Validate()).Message);
    }

    [Fact]
    public void An_accelerator_only_menu_requires_a_gesture_on_every_item()
    {
        var ir = Menu(
            kind: ViewModelMenuKind.Accelerators,
            item: Item(1, "Run", ViewModelMenuItemKind.Command, "Run", command: "Run"));

        Assert.Contains(
            "must declare a gesture",
            Assert.Throws<InvalidOperationException>(() => ir.Validate()).Message);
    }

    [Fact]
    public void A_recent_file_item_requires_a_declared_recent_file_list()
    {
        var ir = Menu(item: Item(1, "Recent", ViewModelMenuItemKind.RecentFiles, "Recent"));

        Assert.Contains(
            "declares no recentFiles",
            Assert.Throws<InvalidOperationException>(() => ir.Validate()).Message);
    }

    [Fact]
    public void Recent_files_must_reference_a_plain_string_collection_and_a_parameterless_command()
    {
        Assert.Contains(
            "unknown collection",
            Assert.Throws<InvalidOperationException>(() => new ViewModelIr
            {
                Models = [WithRecentFiles(collection: "Missing")],
            }.Validate()).Message);

        Assert.Contains(
            "must contain strings",
            Assert.Throws<InvalidOperationException>(() => new ViewModelIr
            {
                Models =
                [
                    new()
                    {
                        Id = 1,
                        Name = "Model",
                        ManagedNamespace = "Tests",
                        Collections = [new() { Id = 1, Name = "Recent", ElementKind = ViewModelValueKind.Model, ElementModelName = "Model" }],
                        Commands = [new() { Id = 1, Name = "OpenRecent" }],
                        RecentFiles = new() { Id = 1, Collection = "Recent", ActivateCommand = "OpenRecent" },
                    },
                ],
            }.Validate()).Message);

        Assert.Contains(
            "must not use parameterProperty",
            Assert.Throws<InvalidOperationException>(() => new ViewModelIr
            {
                Models =
                [
                    new()
                    {
                        Id = 1,
                        Name = "Model",
                        ManagedNamespace = "Tests",
                        Properties = [new() { Id = 1, Name = "Chosen", Kind = ViewModelValueKind.String, Writable = true }],
                        Collections = [new() { Id = 1, Name = "Recent", ElementKind = ViewModelValueKind.String }],
                        Commands = [new() { Id = 1, Name = "OpenRecent", ParameterProperty = "Chosen" }],
                        RecentFiles = new() { Id = 1, Collection = "Recent", ActivateCommand = "OpenRecent" },
                    },
                ],
            }.Validate()).Message);
    }

    [Fact]
    public void A_display_path_must_end_in_a_string_property()
    {
        var ir = new ViewModelIr
        {
            Models =
            [
                new()
                {
                    Id = 1,
                    Name = "Model",
                    ManagedNamespace = "Tests",
                    Properties = [new() { Id = 1, Name = "Count", Kind = ViewModelValueKind.Integer }],
                    DisplayPath = "Count",
                },
            ],
        };

        Assert.Contains(
            "must end in a String property",
            Assert.Throws<InvalidOperationException>(() => ir.Validate()).Message);
    }

    [Fact]
    public void Menu_item_identifiers_are_unique_within_one_menu()
    {
        var ir = Menu(items:
        [
            Item(1, "First", ViewModelMenuItemKind.Command, "First", command: "Run"),
            Item(1, "Second", ViewModelMenuItemKind.Command, "Second", command: "Run"),
        ]);

        Assert.Contains(
            "Duplicate menu item ID",
            Assert.Throws<InvalidOperationException>(() => ir.Validate()).Message);
    }

    [Fact]
    public void Menu_item_names_that_would_share_a_generated_local_are_rejected()
    {
        // Every item of a menu is emitted into one method body whose locals are
        // the item name with a lower-cased first character, so `Run` and `run`
        // would collide there. That has to be a schema error, not a compile
        // error in generated code.
        var ir = Menu(items:
        [
            Item(1, "Run", ViewModelMenuItemKind.Command, "Run", command: "Run"),
            Item(2, "run", ViewModelMenuItemKind.Command, "Run again", command: "Run"),
        ]);

        Assert.Contains(
            "Duplicate menu item name",
            Assert.Throws<InvalidOperationException>(() => ir.Validate()).Message);
    }

    [Fact]
    public void A_nested_submenu_item_shares_the_menus_name_space()
    {
        var ir = Menu(items:
        [
            Item(1, "File", ViewModelMenuItemKind.Submenu, "File", items:
            [
                Item(2, "Run", ViewModelMenuItemKind.Command, "Run", command: "Run"),
            ]),
            Item(3, "Run", ViewModelMenuItemKind.Command, "Run", command: "Run"),
        ]);

        Assert.Contains(
            "Duplicate menu item name",
            Assert.Throws<InvalidOperationException>(() => ir.Validate()).Message);
    }

    private static ViewModelMenuItem Item(
        int id,
        string name,
        ViewModelMenuItemKind kind,
        string? header = null,
        string? command = null,
        string? gesture = null,
        string? toggleProperty = null,
        string? radioProperty = null,
        string? radioValue = null,
        IReadOnlyList<ViewModelMenuItem>? items = null) => new()
        {
            Id = id,
            Name = name,
            Kind = kind,
            Header = header,
            Command = command,
            Gesture = gesture,
            ToggleProperty = toggleProperty,
            RadioProperty = radioProperty,
            RadioValue = radioValue,
            Items = items ?? [],
        };

    private static ViewModelIr Menu(
        ViewModelMenuItem? item = null,
        IReadOnlyList<ViewModelMenuItem>? items = null,
        ViewModelMenuKind kind = ViewModelMenuKind.Application) => new()
        {
            Enums =
            [
                new()
                {
                    Id = 1,
                    Name = "Level",
                    ManagedNamespace = "Tests",
                    Members = [new() { Name = "Low", Value = 0 }, new() { Name = "High", Value = 1 }],
                },
            ],
            Models =
            [
                new()
                {
                    Id = 1,
                    Name = "Model",
                    ManagedNamespace = "Tests",
                    Properties =
                    [
                        new() { Id = 1, Name = "Label", Kind = ViewModelValueKind.String },
                        new() { Id = 2, Name = "Flag", Kind = ViewModelValueKind.Boolean, Writable = true },
                        new() { Id = 3, Name = "ReadOnlyFlag", Kind = ViewModelValueKind.Boolean },
                        new() { Id = 4, Name = "Level", Kind = ViewModelValueKind.Enum, EnumName = "Level", Writable = true },
                    ],
                    Commands = [new() { Id = 1, Name = "Run" }],
                    Menus =
                    [
                        new()
                        {
                            Id = 1,
                            Name = "Main",
                            Kind = kind,
                            Items = items ?? [item ?? Item(1, "Run", ViewModelMenuItemKind.Command, "Run", command: "Run", gesture: "Ctrl+R")],
                        },
                    ],
                },
            ],
        };

    private static ViewModelDefinition WithRecentFiles(string collection = "Recent") => new()
    {
        Id = 1,
        Name = "Model",
        ManagedNamespace = "Tests",
        Collections = [new() { Id = 1, Name = "Recent", ElementKind = ViewModelValueKind.String }],
        Commands = [new() { Id = 1, Name = "OpenRecent" }],
        RecentFiles = new() { Id = 1, Collection = collection, ActivateCommand = "OpenRecent" },
    };

    private static string FindRepositoryRoot()
    {
        for (var current = new DirectoryInfo(AppContext.BaseDirectory);
             current is not null;
             current = current.Parent)
        {
            var gitPath = Path.Combine(current.FullName, ".git");
            if (Directory.Exists(gitPath) || File.Exists(gitPath))
                return current.FullName;
        }

        throw new DirectoryNotFoundException("Could not locate repository root.");
    }
}
