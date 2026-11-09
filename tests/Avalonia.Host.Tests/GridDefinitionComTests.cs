using System;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

/// <summary>
/// Covers <c>Grid.ColumnDefinitions</c> and <c>Grid.RowDefinitions</c>, which cross the ABI as
/// a length list with optional shared-size groups rather than as a projected collection of
/// definition objects. Every assertion goes through a real CCW/RCW round trip.
/// </summary>
public unsafe class GridDefinitionComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Definitions_round_trip_as_a_length_list_and_reach_the_avalonia_object()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateGrid(out var projected));
        Assert.NotNull(projected);
        var grid = Target<Grid>(projected);

        Through<IAvnGrid>(projected, live =>
        {
            // A fresh Grid has no tracks, which is an empty list rather than a null pointer.
            Assert.Equal(0, live.GetColumnDefinitions(out var initialColumns));
            Assert.Equal(string.Empty, initialColumns);
            Assert.Equal(0, live.GetRowDefinitions(out var initialRows));
            Assert.Equal(string.Empty, initialRows);

            Assert.Equal(0, live.SetColumnDefinitions("*,Auto,120"));
            Assert.Equal(0, live.GetColumnDefinitions(out var columns));
            // `*` is shorthand for `1*`, so the getter reports the normalised list Avalonia
            // parsed, not the exact characters that were written.
            Assert.Equal("1*,Auto,120", columns);

            Assert.Equal(0, live.SetRowDefinitions("Auto,2*,Auto"));
            Assert.Equal(0, live.GetRowDefinitions(out var rows));
            Assert.Equal("Auto,2*,Auto", rows);

            // A normalised list is a fixed point: reading and writing it again changes nothing.
            Assert.Equal(0, live.SetColumnDefinitions(columns));
            Assert.Equal(0, live.GetColumnDefinitions(out var again));
            Assert.Equal(columns, again);

            // Clearing the tracks is an empty string, not a null.
            Assert.Equal(0, live.SetRowDefinitions(string.Empty));
            Assert.Equal(0, live.GetRowDefinitions(out var cleared));
            Assert.Equal(string.Empty, cleared);
        });

        // The projection must have written through to the real Grid, not to wrapper state.
        Assert.Equal(3, grid.ColumnDefinitions.Count);
        Assert.Equal(GridUnitType.Star, grid.ColumnDefinitions[0].Width.GridUnitType);
        Assert.Equal(1d, grid.ColumnDefinitions[0].Width.Value);
        Assert.True(grid.ColumnDefinitions[1].Width.IsAuto);
        Assert.Equal(new GridLength(120), grid.ColumnDefinitions[2].Width);
        Assert.Empty(grid.RowDefinitions);
    }

    [Fact]
    public void Definitions_set_from_the_managed_side_are_readable_across_the_abi()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateGrid(out var projected));
        Assert.NotNull(projected);
        var grid = Target<Grid>(projected);
        grid.RowDefinitions = new RowDefinitions("Auto,*,32");

        Through<IAvnGrid>(projected, live =>
        {
            Assert.Equal(0, live.GetRowDefinitions(out var rows));
            Assert.Equal("Auto,1*,32", rows);
        });
    }

    [Fact]
    public void A_malformed_length_list_fails_the_call_rather_than_being_guessed_at()
    {
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateGrid(out var projected));
        Assert.NotNull(projected);

        Through<IAvnGrid>(projected, live =>
        {
            Assert.Equal(0, live.SetColumnDefinitions("Auto,80"));
            Assert.True(live.SetColumnDefinitions("Auto,not-a-length") < 0);

            // The failed write left the previous definitions in place.
            Assert.Equal(0, live.GetColumnDefinitions(out var columns));
            Assert.Equal("Auto,80", columns);
        });
    }

    [Fact]
    public void Shared_groups_round_trip_and_participate_in_layout()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateGrid(out var firstProjected));
        Assert.Equal(0, factory.CreateGrid(out var secondProjected));
        Assert.NotNull(firstProjected);
        Assert.NotNull(secondProjected);

        Through<IAvnGrid>(firstProjected, live =>
        {
            Assert.Equal(0, live.SetColumnDefinitions("Auto@SharedWidth,*"));
            Assert.Equal(0, live.SetRowDefinitions("Auto@SharedHeight"));
            Assert.Equal(0, live.GetColumnDefinitions(out var columns));
            Assert.Equal("Auto@SharedWidth,1*", columns);
            Assert.Equal(0, live.GetRowDefinitions(out var rows));
            Assert.Equal("Auto@SharedHeight", rows);
            Assert.True(live.SetColumnDefinitions("Auto@1invalid,*") < 0);
            Assert.True(live.SetColumnDefinitions("Auto@,*") < 0);
            Assert.True(live.SetColumnDefinitions("Auto@A@B,*") < 0);
            Assert.Equal(0, live.GetColumnDefinitions(out var unchanged));
            Assert.Equal(columns, unchanged);
        });
        Through<IAvnGrid>(secondProjected, live =>
            Assert.Equal(0, live.SetColumnDefinitions("Auto@SharedWidth,*")));

        var first = Target<Grid>(firstProjected);
        var second = Target<Grid>(secondProjected);
        first.Children.Add(new Border { Width = 25, Height = 10 });
        second.Children.Add(new Border { Width = 75, Height = 10 });
        var scope = new StackPanel
        {
            [Grid.IsSharedSizeScopeProperty] = true,
            Children = { first, second },
        };
        var root = new TestRoot(scope) { Width = 200, Height = 100 };
        // Shared-size validation schedules another layout after the first pass.
        root.ExecuteInitialLayoutPass();
        root.ExecuteInitialLayoutPass();
        root.ExecuteInitialLayoutPass();
        Assert.Equal(75, first.ColumnDefinitions[0].ActualWidth);
        Assert.Equal(75, second.ColumnDefinitions[0].ActualWidth);
    }

    [Fact]
    public void Layout_notifications_and_effective_visibility_follow_the_live_grid()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateGrid(out var projected));
        Assert.NotNull(projected);
        var wrapper = Assert.IsType<AvnGrid>(projected);
        var grid = Target<Grid>(projected);
        var child = new TextBox();
        grid.Children.Add(child);
        var parent = new Border { Child = grid };
        var root = new TestRoot(parent) { Width = 200, Height = 100 };
        var handler = new LayoutHandler();

        Assert.Equal(0, wrapper.AdviseLayoutUpdated(handler, out var subscription));
        root.ExecuteInitialLayoutPass();
        Assert.True(handler.Count > 0);
        Assert.Equal(0, wrapper.GetIsEffectivelyVisible(out var visible));
        Assert.Equal(1, visible);
        Assert.True(child.IsEffectivelyVisible);

        parent.IsVisible = false;
        Assert.Equal(0, wrapper.GetIsEffectivelyVisible(out visible));
        Assert.Equal(0, visible);
        Assert.False(child.IsEffectivelyVisible);
        parent.IsVisible = true;
        child.IsVisible = false;
        Assert.Equal(0, wrapper.GetIsEffectivelyVisible(out visible));
        Assert.Equal(1, visible);
        Assert.False(child.IsEffectivelyVisible);

        Assert.Equal(0, wrapper.UpdateLayout());
        var count = handler.Count;
        Assert.Equal(0, wrapper.UnadviseLayoutUpdated(subscription));
        Assert.True(wrapper.UnadviseLayoutUpdated(subscription) < 0);
        Assert.Equal(0, wrapper.UpdateLayout());
        Assert.Equal(count, handler.Count);
    }

    [Fact]
    public void Child_collection_notifications_work_without_a_layout_root()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateGrid(out var projected));
        Assert.NotNull(projected);
        var wrapper = Assert.IsType<AvnGrid>(projected);
        var grid = Target<Grid>(projected);
        var handler = new ChildrenChangedHandler();
        var child = new Border();

        Assert.Equal(0, wrapper.AdviseChildrenChanged(handler, out var subscription));
        grid.Children.Add(child);
        Assert.Equal(1, handler.Count);
        grid.Children.Remove(child);
        Assert.Equal(2, handler.Count);
        Assert.Equal(0, wrapper.UnadviseChildrenChanged(subscription));
        Assert.True(wrapper.UnadviseChildrenChanged(subscription) < 0);
        grid.Children.Add(child);
        Assert.Equal(2, handler.Count);
    }

    [Fact]
    public void Replacing_grid_definitions_after_layout_keeps_outlying_children_measurable()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreateGrid(out var projected));
        Assert.NotNull(projected);
        var wrapper = Assert.IsType<AvnGrid>(projected);
        var grid = Target<Grid>(projected);
        Assert.Equal(0, wrapper.SetColumnDefinitions("Auto,Auto,Auto"));
        Assert.Equal(0, wrapper.SetRowDefinitions("Auto,Auto,Auto"));
        grid.Children.Add(new Label
        {
            [Grid.ColumnProperty] = 2,
            [Grid.RowProperty] = 2,
        });
        var scope = new StackPanel
        {
            [Grid.IsSharedSizeScopeProperty] = true,
            Children = { grid },
        };
        var root = new TestRoot(scope) { Width = 200, Height = 100 };
        root.ExecuteInitialLayoutPass();
        Assert.Equal(0, wrapper.SetColumnDefinitions("Auto,Auto"));
        Assert.Equal(0, wrapper.SetRowDefinitions("Auto,Auto"));
        root.ExecuteInitialLayoutPass();
        Assert.True(grid.IsMeasureValid);

        grid.ColumnDefinitions.RemoveAt(1);
        Assert.False(grid.IsMeasureValid);
        root.ExecuteInitialLayoutPass();
        grid.RowDefinitions.RemoveAt(1);
        Assert.False(grid.IsMeasureValid);
        root.ExecuteInitialLayoutPass();
        Assert.True(grid.IsMeasureValid);

        Assert.Equal(0, wrapper.SetColumnDefinitions(string.Empty));
        Assert.Equal(0, wrapper.SetRowDefinitions(string.Empty));
        root.ExecuteInitialLayoutPass();
        Assert.Empty(grid.ColumnDefinitions);
        Assert.Empty(grid.RowDefinitions);
        Assert.True(grid.IsMeasureValid);
    }

    private sealed class LayoutHandler : IAvnGridLayoutUpdatedHandler
    {
        public int Count { get; private set; }

        public int Invoke()
        {
            Count++;
            return 0;
        }
    }

    private sealed class ChildrenChangedHandler : IAvnGridChildrenChangedHandler
    {
        public int Count { get; private set; }

        public int Invoke()
        {
            Count++;
            return 0;
        }
    }

    [Fact]
    public void Definitions_are_the_only_way_grid_publishes_tracks()
    {
        // Nothing mints a per-definition COM object: the whole feature is two string slots, so
        // IAvnGrid must not have grown an interface-shaped definition member.
        Assert.DoesNotContain(
            typeof(IAvnGrid).GetMethods(),
            method => method.Name.Contains("Definition", StringComparison.Ordinal) &&
                method.GetParameters().Any(parameter =>
                    parameter.ParameterType != typeof(string) &&
                    parameter.ParameterType != typeof(string).MakeByRefType()));
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
