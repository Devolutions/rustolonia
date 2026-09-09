using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveU12VariantComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Button_command_parameter_round_trips_scalars_and_clears()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateButton(out var projected));
        Assert.NotNull(projected);

        Through<IAvnButton>(projected, button =>
        {
            Assert.Equal(0, button.SetCommandParameter(
                AvnVariant.FromObject("save")));
            Assert.Equal(0, button.GetCommandParameter(out var text));
            Assert.Equal(AvnVariant.TagUtf16, text.Tag);
            Assert.Equal("save", (string?)text.ToObject());

            Assert.Equal(0, button.SetCommandParameter(AvnVariant.FromObject(42)));
            Assert.Equal(0, button.GetCommandParameter(out var i));
            Assert.Equal(AvnVariant.TagI32, i.Tag);
            Assert.Equal(42, (int?)i.ToObject());

            Assert.Equal(0, button.SetCommandParameter(AvnVariant.FromObject(3.5)));
            Assert.Equal(0, button.GetCommandParameter(out var f));
            Assert.Equal(AvnVariant.TagF64, f.Tag);
            Assert.Equal(3.5, (double?)f.ToObject());

            Assert.Equal(0, button.SetCommandParameter(AvnVariant.FromObject(true)));
            Assert.Equal(0, button.GetCommandParameter(out var b));
            Assert.Equal(AvnVariant.TagBool, b.Tag);
            Assert.Equal(true, (bool?)b.ToObject());

            Assert.Equal(0, button.SetCommandParameter(default));
            Assert.Equal(0, button.GetCommandParameter(out var none));
            Assert.Equal(AvnVariant.TagNone, none.Tag);
            Assert.Null(none.ToObject());
        });

        var target = Target<Button>(projected);
        // The last write cleared it back to null.
        Assert.Null(target.CommandParameter);
    }

    [Fact]
    public void Variant_conversions_cover_the_supported_set()
    {
        Assert.Null(AvnVariant.FromObject(null).ToObject());
        Assert.Equal("text", AvnVariant.FromObject("text").ToObject());
        Assert.Equal(7, AvnVariant.FromObject(7).ToObject());
        Assert.Equal(7, AvnVariant.FromObject((long)7).ToObject());
        Assert.Equal(1.25, AvnVariant.FromObject(1.25).ToObject());
        Assert.True((bool)AvnVariant.FromObject(true).ToObject()!);
        Assert.True(AvnVariant.IsSupported("x"));
        Assert.True(AvnVariant.IsSupported(1));
        Assert.True(AvnVariant.IsSupported(1.0));
        Assert.True(AvnVariant.IsSupported(true));
        Assert.True(AvnVariant.IsSupported((long)5));
        Assert.False(AvnVariant.IsSupported((object?)new object()));
    }

    private static void Through<T>(object wrapper, Action<T> body) where T : class
    {
        var unknown = s_wrappers.GetOrCreateComInterfaceForObject(wrapper, CreateComInterfaceFlags.None);
        Assert.NotEqual(0, unknown);
        try
        {
            body((T)s_wrappers.GetOrCreateObjectForComInstance(unknown, CreateObjectFlags.None));
        }
        finally
        {
            Marshal.Release(unknown);
        }
    }

    private static T Target<T>(object wrapper) where T : AvaloniaObject =>
        Assert.IsType<T>(wrapper.GetType()
            .GetProperty("_value", BindingFlags.Instance | BindingFlags.NonPublic)!
            .GetValue(wrapper));
}
