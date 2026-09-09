using System;
using System.Linq;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Projection.Generator;
using Avalonia.Projection.Ir;
using Xunit;

namespace Avalonia.Projection.Generator.Tests;

/// <summary>
/// Pins the emitted shape of the solid brush ABI: the generated <c>IAvnBrush</c> interface and
/// its managed wrapper, the factory slot that mints one, and the native vtable declarations.
/// </summary>
public class BrushMarshallingEmitterTests
{
    private static readonly Lazy<ProjectionIr> Ir = new(() => ClrTypeExtractor.Extract(
        [
            typeof(AvaloniaObject),
            typeof(StyledElement),
            typeof(Control),
            typeof(Decorator),
            typeof(Border),
            typeof(Panel),
            typeof(TemplatedControl),
            typeof(TextBlock),
        ],
        AvaloniaProjectionProfiles.ObjectModelKernel));

    [Fact]
    public void Emits_a_read_only_brush_interface_and_its_managed_wrapper()
    {
        var source = ComSourceEmitter.EmitBrush(Ir.Value);

        Assert.Contains($"[Guid(\"{Ir.Value.BrushInterfaceIid}\")]", source, StringComparison.Ordinal);
        Assert.Contains("public partial interface IAvnBrush", source, StringComparison.Ordinal);
        Assert.Contains("int GetColor(out AvnColor value);", source, StringComparison.Ordinal);
        Assert.Contains("int GetOpacity(out double value);", source, StringComparison.Ordinal);

        // Read-only: a projected brush never mutates the managed brush it was read from.
        Assert.DoesNotContain("int SetColor(", source, StringComparison.Ordinal);
        Assert.DoesNotContain("int SetOpacity(", source, StringComparison.Ordinal);

        Assert.Contains("public sealed partial class AvnBrush : IAvnBrush", source, StringComparison.Ordinal);
        Assert.Contains(
            "if (value is global::Avalonia.Media.ISolidColorBrush solid)",
            source,
            StringComparison.Ordinal);
        Assert.Contains(
            "HResult = global::Avalonia.Host.HResults.AVN_E_NONSOLIDBRUSH,",
            source,
            StringComparison.Ordinal);
        Assert.Contains(
            "return new global::Avalonia.Media.Immutable.ImmutableSolidColorBrush(color.ToAvalonia(), opacity);",
            source,
            StringComparison.Ordinal);
    }

    [Fact]
    public void Emits_brush_properties_as_nullable_interface_pointers()
    {
        var source = ComSourceEmitter.EmitClass(
            Ir.Value,
            Ir.Value.Types.Single(type => type.Name == "IAvnBorder"));

        Assert.Contains("int GetBackground(out IAvnBrush? value);", source, StringComparison.Ordinal);
        Assert.Contains("int SetBackground(IAvnBrush? value);", source, StringComparison.Ordinal);
        Assert.Contains(
            "value = AvnBrush.FromBrush(_value.Background);",
            source,
            StringComparison.Ordinal);
        Assert.Contains(
            "_value.BorderBrush = AvnBrush.ToBrush(value);",
            source,
            StringComparison.Ordinal);
    }

    [Fact]
    public void Emits_the_factory_slot_that_mints_a_solid_brush()
    {
        var files = ComSourceEmitter.Emit(Ir.Value);

        Assert.True(files.ContainsKey("IAvnBrush.g.cs"));
        var factory = files["IAvnControlFactory.g.cs"];
        Assert.Contains(
            "int CreateSolidColorBrush(AvnColor color, double opacity, out IAvnBrush? value);",
            factory,
            StringComparison.Ordinal);
        Assert.Contains("value = new AvnBrush(color, opacity);", factory, StringComparison.Ordinal);
        Assert.Contains(
            "typeof(AvnBrush)",
            files["ProjectionAotRoots.g.cs"],
            StringComparison.Ordinal);
    }

    [Fact]
    public void Emits_the_native_brush_vtable_and_the_chrome_slots()
    {
        var header = NativeHeaderEmitter.Emit(Ir.Value);

        Assert.Contains("typedef struct IAvnBrush IAvnBrush;", header, StringComparison.Ordinal);
        Assert.Contains("#define I_AVN_BRUSH_ABI_VERSION 1", header, StringComparison.Ordinal);
        Assert.Contains(
            "*get_color)(IAvnBrush* self, AvnColor* value)",
            header,
            StringComparison.Ordinal);
        Assert.Contains(
            "*get_opacity)(IAvnBrush* self, double* value)",
            header,
            StringComparison.Ordinal);
        Assert.Contains("#define I_AVN_BRUSH_VTABLE_SLOTS 5", header, StringComparison.Ordinal);

        Assert.Contains(
            "*get_background)(IAvnBorder* self, IAvnBrush** value)",
            header,
            StringComparison.Ordinal);
        Assert.Contains(
            "*set_background)(IAvnBorder* self, IAvnBrush* value)",
            header,
            StringComparison.Ordinal);
        Assert.Contains(
            "*set_border_thickness)(IAvnBorder* self, AvnThickness value)",
            header,
            StringComparison.Ordinal);
        Assert.Contains(
            "*set_corner_radius)(IAvnBorder* self, AvnCornerRadius value)",
            header,
            StringComparison.Ordinal);
        Assert.Contains(
            "*set_background)(IAvnPanel* self, IAvnBrush* value)",
            header,
            StringComparison.Ordinal);
        Assert.Contains(
            "*set_foreground)(IAvnTemplatedControl* self, IAvnBrush* value)",
            header,
            StringComparison.Ordinal);
        Assert.Contains(
            "*set_font_size)(IAvnTemplatedControl* self, double value)",
            header,
            StringComparison.Ordinal);
        Assert.Contains(
            "*set_font_weight)(IAvnTextBlock* self, int32_t value)",
            header,
            StringComparison.Ordinal);
        Assert.Contains(
            "*set_text_alignment)(IAvnTextBlock* self, int32_t value)",
            header,
            StringComparison.Ordinal);
        Assert.Contains(
            "*create_solid_color_brush)(IAvnControlFactory* self, AvnColor color, double opacity, IAvnBrush** value)",
            header,
            StringComparison.Ordinal);

        // Widened interfaces republish at version 4; the ones whose flattened vtable did not
        // move keep the version they already published.
        Assert.Contains("#define I_AVN_BORDER_ABI_VERSION 12", header, StringComparison.Ordinal);
        Assert.Contains("#define I_AVN_PANEL_ABI_VERSION 11", header, StringComparison.Ordinal);
        Assert.Contains("#define I_AVN_TEXT_BLOCK_ABI_VERSION 14", header, StringComparison.Ordinal);
        Assert.Contains("#define I_AVN_CONTROL_ABI_VERSION 8", header, StringComparison.Ordinal);
        Assert.Contains("#define I_AVN_DECORATOR_ABI_VERSION 10", header, StringComparison.Ordinal);
        Assert.Contains(
            "#define I_AVN_AVALONIA_OBJECT_ABI_VERSION 2",
            header,
            StringComparison.Ordinal);
    }
}
