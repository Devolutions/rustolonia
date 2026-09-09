using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using Avalonia.Projection.Ir;
using Xunit;

namespace Avalonia.Projection.Ir.Tests;

public class ProjectionIrValidationTests
{
    [Fact]
    public void Checked_in_projection_ir_is_valid()
    {
        var path = Path.Combine(FindRepositoryRoot(), "rust", "projection.ir.json");
        var ir = ProjectionIr.FromJson(File.ReadAllText(path));

        Assert.Equal(ProjectionIr.CurrentVersion, ir.Version);
        ir.Validate();
    }

    [Fact]
    public void Version_one_schema_remains_accepted()
    {
        new ProjectionIr { Version = ProjectionIr.MinimumVersion }.Validate();
    }

    [Fact]
    public void Current_schema_version_is_accepted()
    {
        new ProjectionIr { Version = ProjectionIr.CurrentVersion }.Validate();
    }

    [Theory]
    [InlineData(0)]
    [InlineData(-1)]
    [InlineData(ProjectionIr.CurrentVersion + 1)]
    public void Unsupported_versions_are_rejected(int version)
    {
        var error = Assert.Throws<InvalidOperationException>(() => new ProjectionIr { Version = version }.Validate());
        Assert.Contains($"Unsupported projection IR version {version}", error.Message, StringComparison.Ordinal);
        Assert.Contains($"{ProjectionIr.MinimumVersion}..={ProjectionIr.CurrentVersion}", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Void_return_kind_is_rejected()
    {
        var error = Assert.Throws<InvalidOperationException>(() => new ProjectionIr
        {
            Types =
            [
                Interface("IAvnVoid", methods:
                [
                    new ProjectedMethod { Name = "Noop", ReturnKind = MarshallingKind.Void },
                ]),
            ],
        }.Validate());
        Assert.Contains("Only PreserveSig I32 HRESULT", error.Message);
    }

    [Theory]
    [InlineData(false, ParameterDirection.In)]
    [InlineData(true, ParameterDirection.InOut)]
    public void Unsupported_method_abi_is_rejected(bool preserveSig, ParameterDirection direction)
    {
        Assert.Throws<InvalidOperationException>(() => new ProjectionIr
        {
            Types = [Interface("IAvnBad", methods:
            [
                new ProjectedMethod
                {
                    Name = "Bad",
                    ReturnKind = MarshallingKind.I32,
                    PreserveSig = preserveSig,
                    Parameters = [new() { Name = "value", Kind = MarshallingKind.I32, Direction = direction }],
                },
            ])],
        }.Validate());
    }

    [Fact]
    public void Unsupported_marshalling_kind_is_rejected()
    {
        var error = Assert.Throws<InvalidOperationException>(() => new ProjectionIr
        {
            Types =
            [
                Interface("IAvnBad", methods:
                [
                    new ProjectedMethod { Name = "Ping", ReturnKind = MarshallingKind.Unsupported },
                ]),
            ],
        }.Validate());
        Assert.Contains("Unknown marshalling kind 'Unsupported'", error.Message, StringComparison.Ordinal);
        Assert.Contains("type 'Tests.IAvnBad' method 'Ping'", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Undefined_marshalling_kind_is_rejected()
    {
        var error = Assert.Throws<InvalidOperationException>(() => new ProjectionIr
        {
            Types =
            [
                Interface("IAvnBad", methods:
                [
                    new ProjectedMethod { Name = "Ping", ReturnKind = (MarshallingKind)999 },
                ]),
            ],
        }.Validate());
        Assert.Contains("Unknown marshalling kind '999'", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Unknown_json_marshalling_kind_fails_to_deserialize()
    {
        const string json = """
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnBad",
                  "fullName": "Tests.IAvnBad",
                  "kind": "Interface",
                  "methods": [
                    { "name": "Ping", "returnKind": "FutureKind", "parameters": [] }
                  ]
                }
              ]
            }
            """;

        Assert.Throws<JsonException>(() => ProjectionIr.FromJson(json));
    }

    [Fact]
    public void Unknown_parameter_direction_is_rejected()
    {
        var error = Assert.Throws<InvalidOperationException>(() => new ProjectionIr
        {
            Types =
            [
                Interface("IAvnBad", methods:
                [
                    new ProjectedMethod
                    {
                        Name = "Ping",
                        ReturnKind = MarshallingKind.Void,
                        Parameters =
                        [
                            new ProjectedParameter
                            {
                                Name = "value",
                                Kind = MarshallingKind.I32,
                                Direction = (ParameterDirection)99,
                            },
                        ],
                    },
                ]),
            ],
        }.Validate());
        Assert.Contains("Unknown parameter direction '99'", error.Message, StringComparison.Ordinal);
        Assert.Contains("parameter 'value'", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Missing_base_reference_is_rejected()
    {
        var error = Assert.Throws<InvalidOperationException>(() => new ProjectionIr
        {
            Types = [Interface("IAvnChild", baseFullName: "Tests.IAvnMissing")],
        }.Validate());
        Assert.Contains("Type 'Tests.IAvnChild' references missing base 'Tests.IAvnMissing'", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Self_base_reference_is_rejected()
    {
        var error = Assert.Throws<InvalidOperationException>(() => new ProjectionIr
        {
            Types = [Interface("IAvnSelf", baseFullName: "Tests.IAvnSelf")],
        }.Validate());
        Assert.Contains("Type 'Tests.IAvnSelf' has a self base reference.", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Inheritance_cycle_is_rejected()
    {
        var error = Assert.Throws<InvalidOperationException>(() => new ProjectionIr
        {
            Types =
            [
                Interface("IAvnA", baseFullName: "Tests.IAvnB"),
                Interface("IAvnB", baseFullName: "Tests.IAvnA"),
            ],
        }.Validate());
        Assert.Contains("Inheritance cycle: Tests.IAvnA -> Tests.IAvnB -> Tests.IAvnA", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Valid_base_lineage_is_accepted()
    {
        new ProjectionIr
        {
            Types =
            [
                Interface("IAvnBase"),
                Interface("IAvnChild", baseFullName: "Tests.IAvnBase"),
            ],
        }.Validate();
    }

    [Fact]
    public void Empty_base_full_name_is_rejected_by_from_json()
    {
        const string json = """{"version":1,"types":[{"name":"IAvnA","fullName":"Tests.IAvnA","kind":"Interface","baseFullName":""}]}""";
        var error = Assert.Throws<InvalidOperationException>(() => ProjectionIr.FromJson(json));
        Assert.Contains("Type 'Tests.IAvnA' has an empty baseFullName.", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Collection_i32_element_kind_is_rejected_by_from_json()
    {
        const string json = """
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnA",
                  "fullName": "Tests.IAvnA",
                  "kind": "Interface",
                  "properties": [
                    {
                      "name": "Items",
                      "kind": "ComCollection",
                      "interfaceName": "Tests.IAvnList",
                      "interfaceIid": "00000000-0000-0000-0000-000000000001",
                      "elementKind": "I32"
                    }
                  ]
                }
              ]
            }
            """;
        var error = Assert.Throws<InvalidOperationException>(() => ProjectionIr.FromJson(json));
        Assert.Contains("Unsupported collection element kind 'I32'", error.Message, StringComparison.Ordinal);
        Assert.Contains("type 'Tests.IAvnA' property 'Items'", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Collection_missing_interface_name_is_rejected_by_from_json()
    {
        const string json = """
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnA",
                  "fullName": "Tests.IAvnA",
                  "kind": "Interface",
                  "properties": [
                    { "name": "Items", "kind": "ComCollection", "elementKind": "Variant" }
                  ]
                }
              ]
            }
            """;
        var error = Assert.Throws<InvalidOperationException>(() => ProjectionIr.FromJson(json));
        Assert.Contains("Missing interfaceName at type 'Tests.IAvnA' property 'Items'.", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Collection_missing_element_kind_is_rejected_by_from_json()
    {
        const string json = """
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnA",
                  "fullName": "Tests.IAvnA",
                  "kind": "Interface",
                  "properties": [
                    {
                      "name": "Items",
                      "kind": "ComCollection",
                      "interfaceName": "Tests.IAvnList",
                      "interfaceIid": "00000000-0000-0000-0000-000000000001"
                    }
                  ]
                }
              ]
            }
            """;
        var error = Assert.Throws<InvalidOperationException>(() => ProjectionIr.FromJson(json));
        Assert.Contains("Missing elementKind at type 'Tests.IAvnA' property 'Items'.", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Collection_com_interface_missing_element_interface_is_rejected_by_from_json()
    {
        const string json = """
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnA",
                  "fullName": "Tests.IAvnA",
                  "kind": "Interface",
                  "properties": [
                    {
                      "name": "Items",
                      "kind": "ComCollection",
                      "interfaceName": "Tests.IAvnList",
                      "interfaceIid": "00000000-0000-0000-0000-000000000001",
                      "elementKind": "ComInterface"
                    }
                  ]
                }
              ]
            }
            """;
        var error = Assert.Throws<InvalidOperationException>(() => ProjectionIr.FromJson(json));
        Assert.Contains("Missing elementInterfaceName at type 'Tests.IAvnA' property 'Items'.", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Fields_payload_without_parameters_is_rejected_by_from_json()
    {
        const string json = """
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnA",
                  "fullName": "Tests.IAvnA",
                  "kind": "Interface",
                  "events": [
                    {
                      "name": "Changed",
                      "handlerInterfaceName": "Tests.IAvnChangedHandler",
                      "handlerInterfaceIid": "00000000-0000-0000-0000-000000000002",
                      "payloadKind": "Fields",
                      "parameters": []
                    }
                  ]
                }
              ]
            }
            """;
        var error = Assert.Throws<InvalidOperationException>(() => ProjectionIr.FromJson(json));
        Assert.Contains("Fields payload requires parameters at type 'Tests.IAvnA' event 'Changed'.", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Args_payload_without_interface_name_is_rejected_by_from_json()
    {
        const string json = """
            {
              "version": 1,
              "types": [
                {
                  "name": "IAvnA",
                  "fullName": "Tests.IAvnA",
                  "kind": "Interface",
                  "events": [
                    {
                      "name": "Changed",
                      "handlerInterfaceName": "Tests.IAvnChangedHandler",
                      "handlerInterfaceIid": "00000000-0000-0000-0000-000000000002",
                      "payloadKind": "Args"
                    }
                  ]
                }
              ]
            }
            """;
        var error = Assert.Throws<InvalidOperationException>(() => ProjectionIr.FromJson(json));
        Assert.Contains("Missing argsInterfaceName at type 'Tests.IAvnA' event 'Changed'.", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Null_types_collection_is_rejected_by_from_json()
    {
        const string json = """{"version":1,"types":null}""";
        var error = Assert.Throws<InvalidOperationException>(() => ProjectionIr.FromJson(json));
        Assert.Contains("Projection IR 'types' must not be null.", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Null_types_element_is_rejected_by_from_json()
    {
        const string json = """{"version":1,"types":[null]}""";
        var error = Assert.Throws<InvalidOperationException>(() => ProjectionIr.FromJson(json));
        Assert.Contains("Projection IR 'types[0]' must not be null.", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Null_methods_collection_is_rejected_by_from_json()
    {
        const string json = """
            {
              "version": 1,
              "types": [
                { "name": "IAvnA", "fullName": "Tests.IAvnA", "kind": "Interface", "methods": null }
              ]
            }
            """;
        var error = Assert.Throws<InvalidOperationException>(() => ProjectionIr.FromJson(json));
        Assert.Contains("Projection IR 'types[0].methods' must not be null.", error.Message, StringComparison.Ordinal);
    }

    [Fact]
    public void Null_attached_properties_collection_is_rejected_by_from_json()
    {
        const string json = """{"version":1,"types":[],"attachedProperties":null}""";
        var error = Assert.Throws<InvalidOperationException>(() => ProjectionIr.FromJson(json));
        Assert.Contains("Projection IR 'attachedProperties' must not be null.", error.Message, StringComparison.Ordinal);
    }

    private static ProjectedType Interface(
        string name,
        string? baseFullName = null,
        IReadOnlyList<ProjectedMethod>? methods = null) =>
        new()
        {
            Name = name,
            FullName = "Tests." + name,
            Kind = ProjectedTypeKind.Interface,
            BaseFullName = baseFullName,
            Methods = methods ?? [],
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
