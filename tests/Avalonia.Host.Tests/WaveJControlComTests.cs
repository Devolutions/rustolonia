using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.Layout;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveJControlComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Command_bar_and_pips_pager_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateCommandBar(out var projectedBar));
        Assert.Equal(0, factory.CreateCommandBarButton(out var projectedButton));
        Assert.Equal(0, factory.CreatePipsPager(out var projectedPips));
        Assert.Equal(0, factory.CreateThemeVariantScope(out var projectedScope));
        Assert.NotNull(projectedBar);
        Assert.NotNull(projectedButton);
        Assert.NotNull(projectedPips);
        Assert.NotNull(projectedScope);

        Through<IAvnCommandBar>(projectedBar, bar =>
        {
            Assert.Equal(0, bar.SetIsOpen(1));
            Assert.Equal(0, bar.SetIsSticky(1));
            Assert.Equal(0, bar.SetDefaultLabelPosition((int)CommandBarDefaultLabelPosition.Right));
        });
        Through<IAvnCommandBarButton>(projectedButton, button =>
        {
            Assert.Equal(0, button.SetLabel("Save"));
            Assert.Equal(0, button.SetIsCompact(1));
        });
        Through<IAvnPipsPager>(projectedPips, pager =>
        {
            Assert.Equal(0, pager.SetNumberOfPages(5));
            Assert.Equal(0, pager.SetSelectedPageIndex(2));
            Assert.Equal(0, pager.SetOrientation((int)Orientation.Horizontal));
        });

        Assert.True(Target<CommandBar>(projectedBar).IsOpen);
        Assert.Equal("Save", Target<CommandBarButton>(projectedButton).Label);
        Assert.Equal(5, Target<PipsPager>(projectedPips).NumberOfPages);
        Assert.Equal(2, Target<PipsPager>(projectedPips).SelectedPageIndex);
        Assert.IsType<ThemeVariantScope>(Target<ThemeVariantScope>(projectedScope));
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
