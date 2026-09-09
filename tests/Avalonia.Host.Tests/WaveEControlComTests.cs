using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveEControlComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Numeric_up_down_decimals_cross_as_invariant_strings()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateNumericUpDown(out var projected));
        Assert.NotNull(projected);

        Through<IAvnNumericUpDown>(projected, upDown =>
        {
            Assert.Equal(0, upDown.SetMinimum("0"));
            Assert.Equal(0, upDown.SetMaximum("100"));
            Assert.Equal(0, upDown.SetIncrement("0.5"));
            Assert.Equal(0, upDown.SetValue("12.5"));
            Assert.Equal(0, upDown.GetValue(out var value));
            Assert.Equal("12.5", value);
            Assert.True(upDown.SetMinimum("1,5") < 0);
            Assert.Equal(0, upDown.SetValue(string.Empty));
            Assert.Equal(0, upDown.GetValue(out var cleared));
            Assert.Null(cleared);
        });

        var managed = Target<NumericUpDown>(projected);
        Assert.Null(managed.Value);
        Assert.Equal(0m, managed.Minimum);
        Assert.Equal(100m, managed.Maximum);
        Assert.Equal(0.5m, managed.Increment);
    }

    [Fact]
    public void Masked_text_box_and_selectable_text_block_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateMaskedTextBox(out var projectedMask));
        Assert.Equal(0, factory.CreateSelectableTextBlock(out var projectedText));
        Assert.NotNull(projectedMask);
        Assert.NotNull(projectedText);

        Through<IAvnMaskedTextBox>(projectedMask, box =>
        {
            Assert.Equal(0, box.SetMask("000-000"));
            Assert.Equal(0, box.SetAsciiOnly(1));
        });
        Through<IAvnSelectableTextBlock>(projectedText, block =>
        {
            Assert.Equal(0, block.SetText("hello"));
            Assert.Equal(0, block.SetSelectionStart(1));
            Assert.Equal(0, block.SetSelectionEnd(4));
            Assert.Equal(0, block.GetSelectedText(out var selected));
            Assert.Equal("ell", selected);
        });

        Assert.Equal("000-000", Target<MaskedTextBox>(projectedMask).Mask);
        Assert.Equal("ell", Target<SelectableTextBlock>(projectedText).SelectedText);
    }

    [Fact]
    public void Auto_complete_and_button_spinner_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateAutoCompleteBox(out var projectedBox));
        Assert.Equal(0, factory.CreateButtonSpinner(out var projectedSpinner));
        Assert.NotNull(projectedBox);
        Assert.NotNull(projectedSpinner);

        Through<IAvnAutoCompleteBox>(projectedBox, box =>
        {
            Assert.Equal(0, box.SetText("av"));
            Assert.Equal(0, box.SetMinimumPrefixLength(2));
            Assert.Equal(0, box.SetFilterMode((int)AutoCompleteFilterMode.StartsWith));
        });
        Through<IAvnButtonSpinner>(projectedSpinner, spinner =>
        {
            Assert.Equal(0, spinner.SetAllowSpin(0));
            Assert.Equal(0, spinner.SetButtonSpinnerLocation((int)Location.Left));
        });

        Assert.Equal("av", Target<AutoCompleteBox>(projectedBox).Text);
        Assert.Equal(Location.Left, Target<ButtonSpinner>(projectedSpinner).ButtonSpinnerLocation);
        Assert.False(Target<ButtonSpinner>(projectedSpinner).AllowSpin);
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
