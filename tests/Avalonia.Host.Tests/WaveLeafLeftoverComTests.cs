using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveLeafLeftoverComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Leaf_commands_and_percentage_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateCarousel(out var projectedCarousel));
        Assert.Equal(0, factory.CreateProgressBar(out var projectedProgress));
        Assert.Equal(0, factory.CreateRefreshContainer(out var projectedRefresh));
        Assert.NotNull(projectedCarousel);
        Assert.NotNull(projectedProgress);
        Assert.NotNull(projectedRefresh);

        Through<IAvnCarousel>(projectedCarousel, carousel =>
        {
            Assert.Equal(0, carousel.Next());
            Assert.Equal(0, carousel.Previous());
        });
        Through<IAvnProgressBar>(projectedProgress, bar =>
            Assert.Equal(0, bar.GetPercentage(out _)));
        Through<IAvnRefreshContainer>(projectedRefresh, refresh =>
            Assert.Equal(0, refresh.RequestRefresh()));

        Assert.NotNull(Target<Carousel>(projectedCarousel));
        Assert.NotNull(Target<ProgressBar>(projectedProgress));
        Assert.NotNull(Target<RefreshContainer>(projectedRefresh));
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
