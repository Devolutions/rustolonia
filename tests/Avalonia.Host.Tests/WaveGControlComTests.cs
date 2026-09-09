using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveGControlComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Carousel_swipe_and_viewport_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateCarousel(out var projected));
        Assert.NotNull(projected);

        Through<IAvnCarousel>(projected, carousel =>
        {
            Assert.Equal(0, carousel.SetIsSwipeEnabled(0));
            Assert.Equal(0, carousel.SetViewportFraction(0.85));
            Assert.Equal(0, carousel.GetIsSwiping(out var swiping));
            Assert.Equal(0, swiping);
        });

        var value = Target<Carousel>(projected);
        Assert.False(value.IsSwipeEnabled);
        Assert.Equal(0.85, value.ViewportFraction);
    }

    [Fact]
    public void Content_chrome_constructs_and_carries_scalar_members()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateTransitioningContentControl(out var projectedTransition));
        Assert.Equal(0, factory.CreateLabel(out var projectedLabel));
        Assert.Equal(0, factory.CreateSeparator(out var projectedSeparator));
        Assert.Equal(0, factory.CreateGroupBox(out var projectedGroup));
        Assert.Equal(0, factory.CreateUserControl(out var projectedUser));
        Assert.Equal(0, factory.CreateLayoutTransformControl(out var projectedLayout));
        Assert.NotNull(projectedTransition);
        Assert.NotNull(projectedLabel);
        Assert.NotNull(projectedSeparator);
        Assert.NotNull(projectedGroup);
        Assert.NotNull(projectedUser);
        Assert.NotNull(projectedLayout);

        Through<IAvnTransitioningContentControl>(projectedTransition, control =>
            Assert.Equal(0, control.SetIsTransitionReversed(1)));
        Through<IAvnLayoutTransformControl>(projectedLayout, control =>
            Assert.Equal(0, control.SetUseRenderTransform(1)));

        Assert.True(Target<TransitioningContentControl>(projectedTransition).IsTransitionReversed);
        Assert.True(Target<LayoutTransformControl>(projectedLayout).UseRenderTransform);
        Assert.IsType<Label>(Target<Label>(projectedLabel));
        Assert.IsType<Separator>(Target<Separator>(projectedSeparator));
        Assert.IsType<GroupBox>(Target<GroupBox>(projectedGroup));
        Assert.IsType<UserControl>(Target<UserControl>(projectedUser));
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
