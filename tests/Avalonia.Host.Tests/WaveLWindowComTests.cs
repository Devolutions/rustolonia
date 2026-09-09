using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveLWindowComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Window_chrome_scalars_and_hide_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateWindow(out var projected));
        Assert.NotNull(projected);

        Through<IAvnWindow>(projected, window =>
        {
            Assert.Equal(0, window.SetSizeToContent((int)SizeToContent.WidthAndHeight));
            Assert.Equal(0, window.SetShowInTaskbar(0));
            Assert.Equal(0, window.SetCanMinimize(0));
            Assert.Equal(0, window.SetCanMaximize(0));
            Assert.Equal(0, window.SetShowActivated(0));
            Assert.Equal(0, window.SetWindowStartupLocation((int)WindowStartupLocation.CenterScreen));
            Assert.Equal(0, window.SetWindowDecorations((int)WindowDecorations.BorderOnly));
            Assert.Equal(0, window.SetClosingBehavior((int)WindowClosingBehavior.OwnerWindowOnly));
            Assert.Equal(0, window.Hide());
        });

        var value = Target<Window>(projected);
        Assert.Equal(SizeToContent.WidthAndHeight, value.SizeToContent);
        Assert.False(value.ShowInTaskbar);
        Assert.False(value.CanMinimize);
        Assert.False(value.CanMaximize);
        Assert.False(value.ShowActivated);
        Assert.Equal(WindowStartupLocation.CenterScreen, value.WindowStartupLocation);
        Assert.Equal(WindowDecorations.BorderOnly, value.WindowDecorations);
        Assert.Equal(WindowClosingBehavior.OwnerWindowOnly, value.ClosingBehavior);
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
