using System;
using System.Linq;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Xunit;

namespace Avalonia.Host.Tests;

/// <summary>
/// Covers <c>Grid.ColumnDefinitions</c> and <c>Grid.RowDefinitions</c>, which cross the ABI as
/// the comma-separated length list that <see cref="ColumnDefinitions"/> and
/// <see cref="RowDefinitions"/> already parse and print rather than as a projected collection of
/// definition objects. Every assertion goes through a real CCW/RCW round trip, so the conversion
/// is exercised in the generated marshalling stub rather than read off the managed wrapper.
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
