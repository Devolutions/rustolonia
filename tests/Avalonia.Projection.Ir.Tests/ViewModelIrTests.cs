using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Avalonia.Projection.Ir;
using Xunit;

namespace Avalonia.Projection.Ir.Tests;

public class ViewModelIrTests
{
    [Fact]
    public void Checked_in_schema_is_valid_and_roundtrips()
    {
        var path = Path.Combine(FindRepositoryRoot(), "rust", "avalonia-sample", "view-model.ir.json");
        var ir = ViewModelIr.FromJson(File.ReadAllText(path));

        Assert.Equal(ViewModelIr.CurrentVersion, ir.Version);
        Assert.Equal(7, ir.Models.Count);
        var model = ir.Models.Single(candidate => candidate.Name == "SampleViewModel");
        Assert.Equal(17, model.Properties.Count);
        Assert.Equal(20, model.Commands.Count);
        Assert.True(model.Commands.Single(command => command.Name == "Save").IsAsync);
        Assert.True(model.Commands.Single(command => command.Name == "OpenFiles").IsAsync);
        Assert.Equal(9, model.Collections.Count);
        Assert.Equal(
            ViewModelValueKind.Double,
            model.Collections.Single(collection => collection.Name == "CoreLoads").ElementKind);
        Assert.Equal(
            ViewModelValueKind.Integer,
            model.Collections.Single(collection => collection.Name == "CoreTicks").ElementKind);
        var enumDefinition = Assert.Single(ir.Enums);
        Assert.Equal("Priority", enumDefinition.Name);
        Assert.Equal(3, enumDefinition.Members.Count);
        Assert.Equal(
            Normalize(File.ReadAllText(path)),
            Normalize(ir.ToJson()));
    }

    [Fact]
    public void Table_metadata_validates_nested_paths_selection_and_sort_contract()
    {
        var ir = ViewModelIr.FromJson(File.ReadAllText(Path.Combine(FindRepositoryRoot(), "rust", "avalonia-sample", "view-model.ir.json")));
        var table = ir.Models.Single(model => model.Name == "SampleViewModel")
            .Collections.Single(collection => collection.Name == "TraceRows").Table!;

        Assert.Equal(4, table.Columns.Count);
        Assert.Equal("Event.Source", table.Columns.Single(column => column.Name == "Source").Path);
        Assert.Equal("SelectedTraceIndex", table.Selection!.SelectedIndexProperty);
        Assert.Equal("SortTraceRows", table.Sort!.Command);
    }

    [Fact]
    public void Table_metadata_rejects_unknown_row_property_path()
    {
        var ir = new ViewModelIr
        {
            Models =
            [
                new() { Id = 1, Name = "Parent", ManagedNamespace = "Tests", Collections = [new()
                {
                    Id = 1, Name = "Rows", ElementKind = ViewModelValueKind.Model, ElementModelName = "Row",
                    Table = new ViewModelTable { Columns = [new() { Id = 1, Name = "Value", Header = "Value", Path = "Missing", Width = 10 }] },
                }] },
                new() { Id = 2, Name = "Row", ManagedNamespace = "Tests" },
            ],
        };

        Assert.Contains("unknown property", Assert.Throws<InvalidOperationException>(() => ir.Validate()).Message);
    }

    [Fact]
    public void Version_two_schema_without_table_metadata_remains_accepted()
    {
        var ir = CreateModel();
        ir = new ViewModelIr { Version = 2, Models = ir.Models };

        ir.Validate();
    }

    [Fact]
    public void Checked_in_schema_declares_the_stage_thirty_shapes()
    {
        var ir = ViewModelIr.FromJson(File.ReadAllText(Path.Combine(FindRepositoryRoot(), "rust", "avalonia-sample", "view-model.ir.json")));
        var model = ir.Models.Single(candidate => candidate.Name == "SampleViewModel");

        var window = model.Collections.Single(collection => collection.Name == "LogWindow");
        Assert.Equal(64, window.Window!.PageSize);
        Assert.Equal(8, window.Window.MaxLivePages);

        var tree = model.Collections.Single(collection => collection.Name == "LogTree");
        Assert.Equal("Children", tree.Tree!.ChildrenCollection);
        Assert.Equal("Label", tree.Tree.HeaderPath);
        Assert.True(ir.Models.Single(candidate => candidate.Name == "LogNodeViewModel")
            .Collections.Single().Recursive);

        Assert.Equal(2, model.Maps.Count);
        Assert.Equal(ViewModelValueKind.String, model.Maps[0].KeyKind);
        Assert.Equal(ViewModelValueKind.Model, model.Maps[1].ValueKind);
        Assert.Equal("TraceEventViewModel", model.Maps[1].ValueModelName);

        var save = model.Commands.Single(command => command.Name == "Save");
        Assert.True(save.SupportsProgress);
        Assert.True(save.SupportsCancellation);
        Assert.Equal("SaveReportViewModel", save.ResultModelName);
    }

    [Fact]
    public void Older_schema_versions_reject_the_stage_thirty_shapes()
    {
        var ir = CreateModel(maps: [new() { Id = 1, Name = "Counters", KeyKind = ViewModelValueKind.String, ValueKind = ViewModelValueKind.Integer }]);

        ir.Validate();
        Assert.Contains(
            "upgrade to version 4",
            Assert.Throws<InvalidOperationException>(
                () => new ViewModelIr { Version = 3, Models = ir.Models }.Validate()).Message);
    }

    [Fact]
    public void Maps_reject_unsupported_key_kinds_and_unknown_value_models()
    {
        Assert.Contains(
            "string or integer key",
            Assert.Throws<InvalidOperationException>(() => CreateModel(
                maps: [new() { Id = 1, Name = "Bad", KeyKind = ViewModelValueKind.Double, ValueKind = ViewModelValueKind.Integer }]).Validate()).Message);

        Assert.Contains(
            "unknown model",
            Assert.Throws<InvalidOperationException>(() => CreateModel(
                maps: [new() { Id = 1, Name = "Bad", KeyKind = ViewModelValueKind.String, ValueKind = ViewModelValueKind.Model, ValueModelName = "Missing" }]).Validate()).Message);

        Assert.Contains(
            "must declare 'valueModelName'",
            Assert.Throws<InvalidOperationException>(() => CreateModel(
                maps: [new() { Id = 1, Name = "Bad", KeyKind = ViewModelValueKind.String, ValueKind = ViewModelValueKind.Model }]).Validate()).Message);
    }

    [Fact]
    public void A_map_may_not_reuse_another_members_name()
    {
        var ir = CreateModel(
            collections: [new() { Id = 1, Name = "Shared", ElementKind = ViewModelValueKind.String }],
            maps: [new() { Id = 1, Name = "Shared", KeyKind = ViewModelValueKind.String, ValueKind = ViewModelValueKind.Integer }]);

        Assert.Contains("Duplicate", Assert.Throws<InvalidOperationException>(() => ir.Validate()).Message);
    }

    [Fact]
    public void Recursive_children_are_the_only_permitted_schema_cycle()
    {
        var valid = new ViewModelIr
        {
            Version = ViewModelIr.CurrentVersion,
            Models =
            [
                new()
                {
                    Id = 1,
                    Name = "Node",
                    ManagedNamespace = "Tests",
                    Properties = [new() { Id = 1, Name = "Label", Kind = ViewModelValueKind.String }],
                    Collections =
                    [
                        new() { Id = 1, Name = "Children", ElementKind = ViewModelValueKind.Model, ElementModelName = "Node", Recursive = true },
                    ],
                },
            ],
        };
        valid.Validate();

        var invalid = new ViewModelIr
        {
            Version = ViewModelIr.CurrentVersion,
            Models =
            [
                new()
                {
                    Id = 1,
                    Name = "Node",
                    ManagedNamespace = "Tests",
                    Collections = [new() { Id = 1, Name = "Children", ElementKind = ViewModelValueKind.Model, ElementModelName = "Node" }],
                },
            ],
        };
        Assert.Contains("Recursive view-model schema graph", Assert.Throws<InvalidOperationException>(() => invalid.Validate()).Message);
    }

    [Fact]
    public void A_tree_requires_a_recursive_children_collection_and_a_string_header()
    {
        Assert.Contains(
            "must declare 'recursive'",
            Assert.Throws<InvalidOperationException>(() => Tree(recursive: false, header: "Label").Validate()).Message);
        Assert.Contains(
            "must end in a String property",
            Assert.Throws<InvalidOperationException>(() => Tree(recursive: true, header: "Count").Validate()).Message);
        Tree(recursive: true, header: "Label").Validate();

        static ViewModelIr Tree(bool recursive, string header) => new()
        {
            Version = ViewModelIr.CurrentVersion,
            Models =
            [
                new()
                {
                    Id = 1,
                    Name = "Root",
                    ManagedNamespace = "Tests",
                    Collections =
                    [
                        new()
                        {
                            Id = 1,
                            Name = "Nodes",
                            ElementKind = ViewModelValueKind.Model,
                            ElementModelName = "Node",
                            Tree = new ViewModelTree { ChildrenCollection = "Children", HeaderPath = header, HasChildrenProperty = "Expandable" },
                        },
                    ],
                },
                new()
                {
                    Id = 2,
                    Name = "Node",
                    ManagedNamespace = "Tests",
                    Properties =
                    [
                        new() { Id = 1, Name = "Label", Kind = ViewModelValueKind.String },
                        new() { Id = 2, Name = "Count", Kind = ViewModelValueKind.Integer },
                        new() { Id = 3, Name = "Expandable", Kind = ViewModelValueKind.Boolean },
                    ],
                    Collections =
                    [
                        new() { Id = 1, Name = "Children", ElementKind = ViewModelValueKind.Model, ElementModelName = "Node", Recursive = recursive },
                    ],
                },
            ],
        };
    }

    [Fact]
    public void A_windowed_collection_rejects_invalid_paging_and_conflicting_shapes()
    {
        Assert.Contains(
            "pageSize must be positive",
            Assert.Throws<InvalidOperationException>(() => Window(new ViewModelCollectionWindow { PageSize = 0 }).Validate()).Message);
        Assert.Contains(
            "maxLivePages must be positive",
            Assert.Throws<InvalidOperationException>(() => Window(new ViewModelCollectionWindow { MaxLivePages = 0 }).Validate()).Message);
        Window(new ViewModelCollectionWindow()).Validate();

        static ViewModelIr Window(ViewModelCollectionWindow window) => new()
        {
            Version = ViewModelIr.CurrentVersion,
            Models =
            [
                new()
                {
                    Id = 1,
                    Name = "Root",
                    ManagedNamespace = "Tests",
                    Collections = [new() { Id = 1, Name = "Rows", ElementKind = ViewModelValueKind.String, Window = window }],
                },
            ],
        };
    }

    [Fact]
    public void Progress_cancellation_and_results_are_validated_against_the_command()
    {
        Assert.Contains(
            "must be asynchronous",
            Assert.Throws<InvalidOperationException>(() => CreateModel(
                commands: [new() { Id = 1, Name = "Go", SupportsProgress = true }]).Validate()).Message);
        Assert.Contains(
            "unknown result model",
            Assert.Throws<InvalidOperationException>(() => CreateModel(
                commands: [new() { Id = 1, Name = "Go", IsAsync = true, ResultModelName = "Missing" }]).Validate()).Message);
        CreateModel(commands: [new() { Id = 1, Name = "Go", IsAsync = true, SupportsProgress = true, SupportsCancellation = true }]).Validate();
    }

    [Fact]
    public void Duplicate_ids_are_rejected()
    {        var ir = new ViewModelIr
        {
            Models =
            [
                new() { Id = 1, Name = "One", ManagedNamespace = "Tests" },
                new() { Id = 1, Name = "Two", ManagedNamespace = "Tests" },
            ],
        };

        Assert.Throws<InvalidOperationException>(() => ir.Validate());
    }

    [Fact]
    public void Scalar_number_collections_are_accepted()
    {
        var ir = CreateModel(collections:
        [
            new() { Id = 1, Name = "Values", ElementKind = ViewModelValueKind.Integer },
            new() { Id = 2, Name = "Loads", ElementKind = ViewModelValueKind.Double },
        ]);

        ir.Validate();
    }

    [Fact]
    public void Scalar_number_collections_that_declare_an_element_model_are_rejected()
    {
        var ir = CreateModel(collections:
        [
            new()
            {
                Id = 1,
                Name = "Values",
                ElementKind = ViewModelValueKind.Integer,
                ElementModelName = "Model",
            },
        ]);

        Assert.Throws<InvalidOperationException>(() => ir.Validate());
    }

    [Fact]
    public void Boolean_element_collections_are_rejected()
    {
        var ir = CreateModel(collections:
        [
            new() { Id = 1, Name = "Flags", ElementKind = ViewModelValueKind.Boolean },
        ]);

        Assert.Throws<InvalidOperationException>(() => ir.Validate());
    }

    [Fact]
    public void Windowed_scalar_number_collections_are_rejected()
    {
        var ir = CreateModel(collections:
        [
            new()
            {
                Id = 1,
                Name = "Loads",
                ElementKind = ViewModelValueKind.Double,
                Window = new() { PageSize = 4, MaxLivePages = 2 },
            },
        ]);

        Assert.Throws<InvalidOperationException>(() => ir.Validate());
    }

    [Fact]
    public void Scalar_command_parameters_are_accepted()
    {
        var ir = CreateModel(
            properties:
        [
            new()
            {
                Id = 1,
                Name = "Count",
                Kind = ViewModelValueKind.Integer,
                Writable = true,
            },
        ],
            commands:
        [
            new() { Id = 1, Name = "SetCount", ParameterProperty = "Count" },
        ]);

        ir.Validate();
    }

    [Fact]
    public void Model_command_parameters_are_rejected()
    {
        var ir = new ViewModelIr
        {
            Models =
            [
                new()
                {
                    Id = 1,
                    Name = "Parent",
                    ManagedNamespace = "Tests",
                    Properties =
                    [
                        new()
                        {
                            Id = 1,
                            Name = "Child",
                            Kind = ViewModelValueKind.Model,
                            ModelName = "Child",
                            Writable = true,
                        },
                    ],
                    Commands = [new() { Id = 1, Name = "UseChild", ParameterProperty = "Child" }],
                },
                new() { Id = 2, Name = "Child", ManagedNamespace = "Tests" },
            ],
        };

        Assert.Throws<InvalidOperationException>(() => ir.Validate());
    }

    [Fact]
    public void Model_collections_are_accepted_when_the_element_model_exists()
    {
        var ir = new ViewModelIr
        {
            Models =
            [
                new()
                {
                    Id = 1,
                    Name = "Parent",
                    ManagedNamespace = "Tests",
                    Collections = [new() { Id = 1, Name = "Children", ElementKind = ViewModelValueKind.Model, ElementModelName = "Child" }],
                },
                new() { Id = 2, Name = "Child", ManagedNamespace = "Tests" },
            ],
        };

        ir.Validate(); // does not throw
    }

    [Fact]
    public void Model_collections_referencing_unknown_models_are_rejected()
    {
        var ir = CreateModel(collections:
        [
            new() { Id = 1, Name = "Children", ElementKind = ViewModelValueKind.Model, ElementModelName = "DoesNotExist" },
        ]);

        Assert.Throws<InvalidOperationException>(() => ir.Validate());
    }

    [Fact]
    public void Recursive_model_properties_are_rejected_before_descriptor_generation()
    {
        var ir = new ViewModelIr
        {
            Models =
            [
                new()
                {
                    Id = 1, Name = "Parent", ManagedNamespace = "Tests",
                    Properties = [new() { Id = 1, Name = "Child", Kind = ViewModelValueKind.Model, ModelName = "Child", Nullable = true }],
                },
                new()
                {
                    Id = 2, Name = "Child", ManagedNamespace = "Tests",
                    Properties = [new() { Id = 1, Name = "Parent", Kind = ViewModelValueKind.Model, ModelName = "Parent", Nullable = true }],
                },
            ],
        };

        var error = Assert.Throws<InvalidOperationException>(() => ir.Validate());
        Assert.Contains("Recursive view-model schema graph", error.Message);
        Assert.Contains("Parent -> Child -> Parent", error.Message);
    }

    [Fact]
    public void Recursive_model_collections_are_rejected_before_descriptor_generation()
    {
        var ir = new ViewModelIr
        {
            Models =
            [
                new()
                {
                    Id = 1, Name = "Node", ManagedNamespace = "Tests",
                    Collections = [new() { Id = 1, Name = "Children", ElementKind = ViewModelValueKind.Model, ElementModelName = "Node" }],
                },
            ],
        };

        Assert.Contains(
            "Recursive view-model schema graph",
            Assert.Throws<InvalidOperationException>(() => ir.Validate()).Message);
    }

    [Fact]
    public void Enum_properties_require_a_declared_enum()
    {
        var ir = CreateModel(properties:
        [
            new() { Id = 1, Name = "Priority", Kind = ViewModelValueKind.Enum, Writable = true },
        ]);

        Assert.Throws<InvalidOperationException>(() => ir.Validate());
    }

    [Fact]
    public void Enum_properties_reject_an_initial_value_outside_the_declared_members()
    {
        var ir = CreateModel(
            properties:
        [
            new()
            {
                Id = 1,
                Name = "Priority",
                Kind = ViewModelValueKind.Enum,
                EnumName = "Priority",
                Writable = true,
                InitialInteger = 99,
            },
        ],
            enums:
        [
            new() { Id = 1, Name = "Priority", ManagedNamespace = "Tests", Members = [new() { Name = "Low", Value = 0 }] },
        ]);

        Assert.Throws<InvalidOperationException>(() => ir.Validate());
    }

    [Fact]
    public void Enum_properties_round_trip_with_a_declared_enum()
    {
        var ir = CreateModel(
            properties:
        [
            new()
            {
                Id = 1,
                Name = "Priority",
                Kind = ViewModelValueKind.Enum,
                EnumName = "Priority",
                Writable = true,
                InitialInteger = 1,
            },
        ],
            enums:
        [
            new()
            {
                Id = 1,
                Name = "Priority",
                ManagedNamespace = "Tests",
                Members = [new() { Name = "Low", Value = 0 }, new() { Name = "High", Value = 1 }],
            },
        ]);

        ir.Validate(); // does not throw
    }

    [Fact]
    public void Model_properties_must_be_nullable_and_not_writable()
    {
        var writable = CreateModel(properties:
        [
            new() { Id = 1, Name = "Address", Kind = ViewModelValueKind.Model, ModelName = "Model", Writable = true, Nullable = true },
        ]);
        Assert.Throws<InvalidOperationException>(() => writable.Validate());

        var notNullable = CreateModel(properties:
        [
            new() { Id = 1, Name = "Address", Kind = ViewModelValueKind.Model, ModelName = "Model", Writable = false, Nullable = false },
        ]);
        Assert.Throws<InvalidOperationException>(() => notNullable.Validate());
    }

    [Fact]
    public void Nullable_is_only_supported_for_string_and_model_properties()
    {
        var ir = CreateModel(properties:
        [
            new() { Id = 1, Name = "Count", Kind = ViewModelValueKind.Integer, Nullable = true },
        ]);

        Assert.Throws<InvalidOperationException>(() => ir.Validate());
    }

    [Fact]
    public void Non_positive_member_ids_are_rejected()
    {
        var ir = CreateModel(properties:
        [
            new() { Id = 0, Name = "Name", Kind = ViewModelValueKind.String },
        ]);

        Assert.Throws<InvalidOperationException>(() => ir.Validate());
    }

    [Fact]
    public void Initializer_must_match_property_kind()
    {
        var ir = CreateModel(properties:
        [
            new()
            {
                Id = 1,
                Name = "Name",
                Kind = ViewModelValueKind.String,
                InitialInteger = 1,
            },
        ]);

        Assert.Throws<InvalidOperationException>(() => ir.Validate());
    }

    private static ViewModelIr CreateModel(
        IReadOnlyList<ViewModelProperty>? properties = null,
        IReadOnlyList<ViewModelCollection>? collections = null,
        IReadOnlyList<ViewModelCommand>? commands = null,
        IReadOnlyList<ViewModelEnumDefinition>? enums = null,
        IReadOnlyList<ViewModelMap>? maps = null) => new()
        {
            Enums = enums ?? [],
            Models =
        [
            new()
            {
                Id = 1,
                Name = "Model",
                ManagedNamespace = "Tests",
                Properties = properties ?? [],
                Collections = collections ?? [],
                Maps = maps ?? [],
                Commands = commands ?? [],
            },
        ],
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

    private static string Normalize(string value) =>
        value.Replace("\r\n", "\n", StringComparison.Ordinal).Trim();
}

