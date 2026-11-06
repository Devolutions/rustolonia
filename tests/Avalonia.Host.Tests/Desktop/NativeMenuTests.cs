using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Host.Com;
using Avalonia.Input;
using Avalonia.Rust;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests.Desktop;

public unsafe partial class NativeMenuTests
{
    private const int Command = 0, CheckBox = 1, Separator = 2, Submenu = 3, Radio = 4;
    private const int Checked = 1, Disabled = 2, Hidden = 4;

    [Fact]
    public void Native_menu_interfaces_have_a_stable_shape()
    {
        Assert.Equal(Guid.Parse("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DAB"), typeof(IAvnNativeMenu).GUID);
        Assert.Equal(Guid.Parse("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DAC"), typeof(IAvnNativeMenuHandler).GUID);
        Assert.Equal(
            new[]
            {
                nameof(IAvnNativeMenu.ClearMenu),
                nameof(IAvnNativeMenu.AddMenuItem),
                nameof(IAvnNativeMenu.UpdateMenuItem),
            },
            typeof(IAvnNativeMenu).GetMethods().Select(m => m.Name));
    }

    [Fact]
    public void Items_build_the_avalonia_model()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var menu = Create(new AvnApplication(), null);
        Assert.Equal(HResults.S_OK, menu.AddMenuItem(0, 1, Submenu, "_File", null, 0));
        Assert.Equal(HResults.S_OK, menu.AddMenuItem(1, 2, Command, "_Open", "Ctrl+O", 0));
        Assert.Equal(HResults.S_OK, menu.AddMenuItem(1, 3, Separator, null, null, 0));
        Assert.Equal(HResults.S_OK, menu.AddMenuItem(1, 4, CheckBox, "Word wrap", null, Checked));
        Assert.Equal(HResults.S_OK, menu.AddMenuItem(1, 5, Radio, "Light", null, Disabled | Hidden));

        var file = (NativeMenuItem)Assert.Single(menu.Menu.Items);
        Assert.Equal("_File", file.Header);
        var items = file.Menu!.Items.ToList();
        Assert.Equal(4, items.Count);
        var open = (NativeMenuItem)items[0];
        Assert.Equal(new KeyGesture(Key.O, KeyModifiers.Control), open.Gesture);
        Assert.Equal(MenuItemToggleType.None, open.ToggleType);
        Assert.IsType<NativeMenuItemSeparator>(items[1]);
        var wrap = (NativeMenuItem)items[2];
        Assert.Equal(MenuItemToggleType.CheckBox, wrap.ToggleType);
        Assert.True(wrap.IsChecked);
        var light = (NativeMenuItem)items[3];
        Assert.Equal(MenuItemToggleType.Radio, light.ToggleType);
        Assert.False(light.IsEnabled);
        Assert.False(light.IsVisible);

        Assert.Equal(HResults.S_OK, menu.UpdateMenuItem(4, "Wrap", 0));
        Assert.Equal("Wrap", wrap.Header);
        Assert.False(wrap.IsChecked);
        Assert.Equal(HResults.S_OK, menu.UpdateMenuItem(1, null, 0));
        Assert.Equal("_File", file.Header);

        Assert.Equal(HResults.S_OK, menu.ClearMenu());
        Assert.Empty(menu.Menu.Items);
        Assert.Equal(HResults.S_OK, menu.AddMenuItem(0, 1, Command, "Again", null, 0));
    }

    [Theory]
    [InlineData(0, 0, Command, null)]
    [InlineData(0, -1, Command, null)]
    [InlineData(0, 2, 5, null)]
    [InlineData(9, 2, Command, null)]
    [InlineData(1, 2, Command, null)]
    [InlineData(0, 2, Submenu, "Ctrl+S")]
    [InlineData(0, 2, Separator, "Ctrl+S")]
    public void Invalid_items_are_rejected(int parent, int id, int kind, string? gesture)
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var menu = Create(new AvnApplication(), null);
        Assert.Equal(HResults.S_OK, menu.AddMenuItem(0, 1, Command, "Plain", null, 0));
        Assert.True(menu.AddMenuItem(parent, id, kind, "Bad", gesture, 0) < 0);
        Assert.True(menu.AddMenuItem(0, 1, Command, "Duplicate", null, 0) < 0);
        Assert.True(menu.AddMenuItem(0, 3, Command, "Bad gesture", "Ctrl+Nope", 0) < 0);
        Assert.True(menu.UpdateMenuItem(42, "Missing", 0) < 0);
        Assert.Single(menu.Menu.Items);
    }

    [Fact]
    public void Clicks_reach_the_handler_without_toggling()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var handler = new RecordingHandler();
        var menu = Create(new AvnApplication(), handler);
        Assert.Equal(HResults.S_OK, menu.AddMenuItem(0, 7, CheckBox, "Toggle", null, Checked));
        var item = (NativeMenuItem)Assert.Single(menu.Menu.Items);
        ((INativeMenuItemExporterEventsImplBridge)item).RaiseClicked();
        Assert.Equal(new[] { 7 }, handler.Clicks);
        Assert.True(item.IsChecked);
    }

    [Fact]
    public void Window_menu_crosses_com_and_installs_key_bindings()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var wrappers = new StrategyBasedComWrappers();
        var host = new AvnApplication();
        var handler = new RecordingHandler();
        Assert.Equal(0, new AvnControlFactory().CreateWindow(out var projected));
        var window = (Window)ProjectionRuntime.Unwrap(projected)!;

        Through<IAvnApplication>(wrappers, host, application =>
        {
            Assert.Equal(HResults.S_OK, application.CreateNativeMenu(handler, out var menu));
            Assert.NotNull(menu);
            // What the host's stub does with the pointer Rust hands back.
            var raw = ComInterfaceMarshaller<IAvnNativeMenu>.ConvertToUnmanaged(menu);
            try
            {
                Assert.Same(menu, ComInterfaceMarshaller<IAvnNativeMenu>.ConvertToManaged(raw));
            }
            finally
            {
                ComInterfaceMarshaller<IAvnNativeMenu>.Free(raw);
            }
            Assert.Equal(HResults.S_OK, menu!.AddMenuItem(0, 1, Submenu, "File", null, 0));
            Assert.Equal(HResults.S_OK, menu.AddMenuItem(1, 2, Command, "Save", "Ctrl+S", 0));
            Through<IAvnWindow>(wrappers, projected!, comWindow =>
            {
                Assert.Equal(HResults.S_OK, application.SetWindowMenu(comWindow, menu));
                Assert.NotNull(NativeMenu.GetMenu(window));
                var binding = Assert.Single(window.KeyBindings);
                Assert.Equal(new KeyGesture(Key.S, KeyModifiers.Control), binding.Gesture);
                binding.Command!.Execute(null);
                Assert.Equal(new[] { 2 }, handler.Clicks);

                Assert.Equal(HResults.S_OK, menu.UpdateMenuItem(1, null, Disabled));
                Assert.False(window.KeyBindings.Single().Command!.CanExecute(null));
                Assert.Equal(HResults.S_OK, menu.UpdateMenuItem(1, null, 0));
                Assert.Equal(HResults.S_OK, menu.AddMenuItem(1, 3, Command, "Quit", "Ctrl+Q", 0));
                Assert.Equal(2, window.KeyBindings.Count);

                Assert.Equal(HResults.S_OK, application.IsNativeMenuExported(comWindow, out var exported));
                Assert.Equal(0, exported);
                Assert.True(application.SetApplicationMenu(menu) < 0);

                Assert.Equal(HResults.S_OK, application.SetWindowMenu(comWindow, null));
                Assert.Null(NativeMenu.GetMenu(window));
                Assert.Empty(window.KeyBindings);
            });
        });
    }

    [Fact]
    public void Window_keeps_one_menu_while_menus_switch_and_windows_share_one()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var host = new AvnApplication();
        var handler = new RecordingHandler();
        var first = Create(host, handler);
        Assert.Equal(HResults.S_OK, first.AddMenuItem(0, 1, Submenu, "File", null, 0));
        Assert.Equal(HResults.S_OK, first.AddMenuItem(1, 2, Command, "Save", "Ctrl+S", 0));
        var second = Create(host, null);
        Assert.Equal(HResults.S_OK, second.AddMenuItem(0, 1, Submenu, "Edit", null, 0));
        Assert.Equal(0, new AvnControlFactory().CreateWindow(out var projectedA));
        Assert.Equal(0, new AvnControlFactory().CreateWindow(out var projectedB));
        var a = (Window)ProjectionRuntime.Unwrap(projectedA)!;
        var b = (Window)ProjectionRuntime.Unwrap(projectedB)!;

        Assert.Equal(HResults.S_OK, host.SetWindowMenu((IAvnWindow)projectedA!, first));
        Assert.Equal(HResults.S_OK, host.SetWindowMenu((IAvnWindow)projectedB!, first));
        var slot = NativeMenu.GetMenu(a)!;
        Assert.NotSame(slot, NativeMenu.GetMenu(b));
        Assert.Equal(new[] { "File" }, Headers(slot));
        Assert.Equal(new[] { "File" }, Headers(NativeMenu.GetMenu(b)!));

        Assert.Equal(HResults.S_OK, first.UpdateMenuItem(2, "Save All", 0));
        var save = (NativeMenuItem)((NativeMenuItem)NativeMenu.GetMenu(b)!.Items[0]).Menu!.Items[0];
        Assert.Equal("Save All", save.Header);
        ((INativeMenuItemExporterEventsImplBridge)save).RaiseClicked();
        Assert.Equal(new[] { 2 }, handler.Clicks);

        Assert.Equal(HResults.S_OK, host.SetWindowMenu((IAvnWindow)projectedA!, second));
        Assert.Same(slot, NativeMenu.GetMenu(a));
        Assert.Equal(new[] { "Edit" }, Headers(slot));
        Assert.Empty(a.KeyBindings);
        Assert.Single(b.KeyBindings);

        Assert.Equal(HResults.S_OK, first.ClearMenu());
        Assert.Empty(NativeMenu.GetMenu(b)!.Items);
        Assert.Equal(new[] { "Edit" }, Headers(slot));
    }

    [Fact]
    public void Closing_the_window_removes_its_bindings()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var host = new AvnApplication();
        var menu = Create(host, null);
        Assert.Equal(HResults.S_OK, menu.AddMenuItem(0, 1, Command, "Save", "Ctrl+S", 0));
        Assert.Equal(0, new AvnControlFactory().CreateWindow(out var projected));
        var window = (Window)ProjectionRuntime.Unwrap(projected)!;
        Assert.Equal(HResults.S_OK, host.SetWindowMenu((IAvnWindow)projected!, menu));
        window.Show();
        window.Close();
        Assert.Empty(window.KeyBindings);
        Assert.Equal(HResults.S_OK, menu.AddMenuItem(0, 2, Command, "Open", "Ctrl+O", 0));
        Assert.Empty(window.KeyBindings);
    }

    [Fact]
    public void Application_menu_joins_an_exported_menu_and_restores_its_default()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var application = Application.Current!;
        var exported = new NativeMenu();
        var about = new NativeMenuItem("About Avalonia");
        var quit = new NativeMenuItem("Quit");
        exported.Items.Add(about);
        exported.Items.Add(quit);
        NativeMenu.SetMenu(application, exported);

        var host = new AvnApplication();
        var menu = Create(host, null);
        Assert.Equal(HResults.S_OK, menu.AddMenuItem(0, 1, Command, "About Jiffshot", null, 0));
        Assert.Equal(HResults.S_OK, host.SetApplicationMenu(menu));
        Assert.Same(exported, NativeMenu.GetMenu(application));
        Assert.Equal(new[] { "About Jiffshot", "Quit" }, Headers(exported));

        Assert.Equal(HResults.S_OK, menu.AddMenuItem(0, 2, Command, "Settings…", "Cmd+OemComma", 0));
        Assert.Equal(new[] { "About Jiffshot", "Settings…", "Quit" }, Headers(exported));

        Assert.Equal(0, new AvnControlFactory().CreateWindow(out var projected));
        Assert.True(host.SetWindowMenu((IAvnWindow)projected!, menu) < 0);

        Assert.Equal(HResults.S_OK, host.SetApplicationMenu(null));
        Assert.Equal(new[] { "About Avalonia", "Quit" }, Headers(exported));
        Assert.Equal(new[] { "About Jiffshot", "Settings…" }, Headers(menu.Menu));
    }

    [Fact]
    public void Application_menu_is_set_when_the_platform_exported_none()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var application = Application.Current!;
        NativeMenu.SetMenu(application, null);
        var host = new AvnApplication();
        var menu = Create(host, null);
        Assert.Equal(HResults.S_OK, host.SetApplicationMenu(menu));
        Assert.Same(menu.Menu, NativeMenu.GetMenu(application));
        Assert.Equal(HResults.S_OK, host.SetApplicationMenu(null));
        Assert.Null(NativeMenu.GetMenu(application));
    }

    [Fact]
    public void Menu_bar_is_a_native_menu_bar_control()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        Assert.Equal(HResults.S_OK, new AvnApplication().CreateNativeMenuBar(out var bar));
        Assert.IsType<NativeMenuBar>(ProjectionRuntime.Unwrap(bar));
    }

    private static string?[] Headers(NativeMenu menu) =>
        menu.Items.Cast<NativeMenuItem>().Select(i => i.Header).ToArray();

    private static AvnNativeMenu Create(AvnApplication host, IAvnNativeMenuHandler? handler)
    {
        Assert.Equal(HResults.S_OK, host.CreateNativeMenu(handler, out var menu));
        return Assert.IsType<AvnNativeMenu>(menu);
    }

    private static void Through<T>(StrategyBasedComWrappers wrappers, object value, Action<T> body) where T : class
    {
        var unknown = wrappers.GetOrCreateComInterfaceForObject(value, CreateComInterfaceFlags.None);
        try
        {
            body((T)wrappers.GetOrCreateObjectForComInstance(unknown, CreateObjectFlags.None));
        }
        finally
        {
            Marshal.Release(unknown);
        }
    }

    [GeneratedComClass]
    internal sealed partial class RecordingHandler : IAvnNativeMenuHandler
    {
        public List<int> Clicks { get; } = new();

        public int OnMenuItem(int id)
        {
            Clicks.Add(id);
            return HResults.S_OK;
        }
    }
}
