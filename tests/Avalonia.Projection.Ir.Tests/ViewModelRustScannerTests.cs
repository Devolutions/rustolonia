using System.Linq;
using Avalonia.Projection.Ir;
using Xunit;

namespace Avalonia.Projection.Ir.Tests;

public class ViewModelRustScannerTests
{
    [Fact]
    public void Scans_enums_models_properties_collections_commands_and_views_from_rust_source()
    {
        const string rustSource = """
            #[view_model_enum(namespace = "Sample.Generated")]
            pub enum Priority {
                Low = 0,
                Normal = 1,
                High = 2,
            }

            #[view_model(namespace = "Sample.Generated")]
            pub struct AddressViewModel {
                #[prop(writable)]
                pub street: String,
                #[prop(writable)]
                pub city: String,
            }

            #[view_model(
                namespace = "Sample.Generated",
                view = "MainWindow",
                view_type = "Sample.Views.MainWindow",
                display_path = "Name",
                theme_variant_property = "IsDark"
            )]
            pub struct MainViewModel {
                #[prop(writable, initial = "Avalonia from Rust")]
                pub name: String,
                #[prop(initial = 0)]
                pub count: i64,
                #[prop(writable, initial = true)]
                pub is_dark: bool,
                #[prop(nullable, writable)]
                pub nickname: Option<String>,
                #[prop(enum = "Priority", writable, initial = 1)]
                pub priority: Priority,
                #[prop(model = "AddressViewModel")]
                pub address: Option<AddressViewModel>,
                #[collection]
                pub items: Vec<String>,
                #[collection(model = "AddressViewModel")]
                pub addresses: Vec<AddressViewModel>,
            }

            impl MainViewModel {
                #[command]
                pub fn increment(&mut self) {}

                #[command(parameter_property = "Name")]
                pub fn rename(&mut self) {}

                #[command(async, progress, cancellable)]
                pub fn save(&mut self) {}
            }
            """;

        var ir = ViewModelRustScanner.ScanSources([rustSource]);

        var priority = Assert.Single(ir.Enums);
        Assert.Equal("Priority", priority.Name);
        Assert.Equal(3, priority.Members.Count);

        Assert.Equal(2, ir.Models.Count);
        var main = ir.Models.Single(m => m.Name == "MainViewModel");
        Assert.Equal("Name", main.DisplayPath);
        Assert.Equal("IsDark", main.ThemeVariantProperty);

        Assert.Equal(6, main.Properties.Count);
        var nameProp = main.Properties.Single(p => p.Name == "Name");
        Assert.True(nameProp.Writable);
        Assert.Equal("Avalonia from Rust", nameProp.InitialString);

        var nickProp = main.Properties.Single(p => p.Name == "Nickname");
        Assert.True(nickProp.Nullable);
        Assert.True(nickProp.Writable);

        var priorityProp = main.Properties.Single(p => p.Name == "Priority");
        Assert.Equal(ViewModelValueKind.Enum, priorityProp.Kind);
        Assert.Equal("Priority", priorityProp.EnumName);
        Assert.Equal(1L, priorityProp.InitialInteger);

        var addressProp = main.Properties.Single(p => p.Name == "Address");
        Assert.Equal(ViewModelValueKind.Model, addressProp.Kind);
        Assert.Equal("AddressViewModel", addressProp.ModelName);
        Assert.True(addressProp.Nullable);

        Assert.Equal(2, main.Collections.Count);
        Assert.Equal(ViewModelValueKind.String, main.Collections.Single(c => c.Name == "Items").ElementKind);
        Assert.Equal("AddressViewModel", main.Collections.Single(c => c.Name == "Addresses").ElementModelName);

        Assert.Equal(3, main.Commands.Count);
        var saveCmd = main.Commands.Single(c => c.Name == "Save");
        Assert.True(saveCmd.IsAsync);
        Assert.True(saveCmd.SupportsProgress);
        Assert.True(saveCmd.SupportsCancellation);

        var view = Assert.Single(ir.Views);
        Assert.Equal("MainWindow", view.Name);
        Assert.Equal("MainViewModel", view.Model);
        Assert.Equal("Sample.Views.MainWindow", view.ManagedTypeName);
    }

    [Fact]
    public void Preserves_existing_baseline_ids_when_scanning_updated_rust_source()
    {
        var baseline = new ViewModelIr
        {
            Models =
            [
                new ViewModelDefinition
                {
                    Id = 10,
                    Name = "MainViewModel",
                    ManagedNamespace = "Sample.Generated",
                    Properties =
                    [
                        new ViewModelProperty { Id = 42, Name = "Count", Kind = ViewModelValueKind.Integer },
                    ],
                },
            ],
        };

        const string rustSource = """
            #[view_model(namespace = "Sample.Generated")]
            pub struct MainViewModel {
                #[prop(initial = 5)]
                pub count: i64,
                #[prop(writable)]
                pub title: String,
            }
            """;

        var updated = ViewModelRustScanner.ScanSources([rustSource], baseline);
        var model = Assert.Single(updated.Models);
        Assert.Equal(10, model.Id);
        Assert.Equal(42, model.Properties.Single(p => p.Name == "Count").Id);
        Assert.Equal(43, model.Properties.Single(p => p.Name == "Title").Id);
    }
}
