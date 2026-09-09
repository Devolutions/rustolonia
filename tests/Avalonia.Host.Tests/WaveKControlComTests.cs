using System;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public unsafe class WaveKControlComTests
{
    private static readonly StrategyBasedComWrappers s_wrappers = new();

    [Fact]
    public void Path_icon_data_and_table_view_columns_reach_the_avalonia_object()
    {
        using var app = UnitTestApplication.Start(TestServices.MockPlatformRenderInterface);
        var factory = new AvnControlFactory();
        Assert.Equal(0, factory.CreatePathIcon(out var projectedIcon));
        Assert.Equal(0, factory.CreateTableView(out var projectedView));
        Assert.Equal(0, factory.CreateTableViewColumn(out var projectedColumn));
        Assert.NotNull(projectedIcon);
        Assert.NotNull(projectedView);
        Assert.NotNull(projectedColumn);

        Through<IAvnPathIcon>(projectedIcon, icon =>
        {
            Assert.Equal(0, icon.SetData("M0,0 L10,10"));
            Assert.Equal(0, icon.GetData(out var data));
            Assert.False(string.IsNullOrEmpty(data));
        });
        Through<IAvnTableViewColumn>(projectedColumn, column =>
        {
            Assert.Equal(0, column.SetWidth("120"));
            Assert.Equal(0, column.SetMinWidth(40));
            Assert.Equal(0, column.GetWidth(out var width));
            Assert.False(string.IsNullOrEmpty(width));
        });
        Through<IAvnTableView>(projectedView, view =>
            Assert.Equal(0, view.SetCanUserResizeColumns(1)));

        Assert.NotNull(Target<PathIcon>(projectedIcon).Data);
        Assert.True(Target<TableView>(projectedView).CanUserResizeColumns);
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
