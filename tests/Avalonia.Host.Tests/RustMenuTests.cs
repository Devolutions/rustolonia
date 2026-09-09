using System;
using System.Collections.Generic;
using System.Linq;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Rust;
using Avalonia.Rust.Interop;
using Avalonia.Rust.Sample.Generated;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

/// <summary>
/// Stage 31 menus, keyboard accelerators and context menus.
/// </summary>
/// <remarks>
/// Menus are presentation built from the schema, so these drive the generated
/// factories against a real adapter and assert that every item reaches the
/// Rust-owned model through the already generated command and property surface.
/// </remarks>
public class RustMenuTests
{
    [Fact]
    public void The_application_menu_projects_the_declared_structure()
    {
        var model = new MenuModel();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        using var scope = new RustMenuScope(adapter);

        var menu = SampleViewModelMenus.CreateMain(adapter, scope);

        Assert.Equal(
            ["_File", "_Edit", "_View"],
            menu.Items.Cast<NativeMenuItem>().Select(item => item.Header));
        var file = ((NativeMenuItem)menu.Items[0]).Menu!;
        Assert.Contains(file.Items, item => item is NativeMenuItemSeparator);
        Assert.Equal("E_xit", ((NativeMenuItem)file.Items[^1]).Header);

        var view = ((NativeMenuItem)menu.Items[2]).Menu!;
        Assert.Equal(MenuItemToggleType.CheckBox, ((NativeMenuItem)view.Items[0]).ToggleType);
        Assert.Equal(3, view.Items.OfType<NativeMenuItem>().Count(item => item.ToggleType == MenuItemToggleType.Radio));
    }

    [Fact]
    public void A_menu_command_reaches_the_rust_model()
    {
        var model = new MenuModel();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        using var scope = new RustMenuScope(adapter);
        var menu = SampleViewModelMenus.CreateMain(adapter, scope);

        var edit = ((NativeMenuItem)menu.Items[1]).Menu!;
        var copy = (NativeMenuItem)edit.Items[0];
        copy.Command!.Execute(copy.CommandParameter);

        // CopySelectedRow is asynchronous, so it arrives through BeginAsync.
        Assert.Equal([16], model.AsyncCommands);
    }

    [Fact]
    public void A_declared_accelerator_installs_a_key_binding_that_invokes_the_same_command()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var model = new MenuModel();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        var window = new Window();

        using var attachment = SampleViewModelMenus.AttachMain(window, adapter);

        Assert.Same(attachment.Menu, NativeMenu.GetMenu(window));
        Assert.Equal(
            ["Ctrl+O", "Ctrl+Q", "Ctrl+C", "Ctrl+X", "Ctrl+V"],
            attachment.KeyBindings.Select(binding => binding.Gesture.ToString()));
        Assert.All(attachment.KeyBindings, binding => Assert.Contains(binding, window.KeyBindings));

        var copy = attachment.KeyBindings.Single(binding => binding.Gesture.Key == Key.C);
        copy.TryHandle(new KeyEventArgs
        {
            RoutedEvent = InputElement.KeyDownEvent,
            Key = Key.C,
            KeyModifiers = KeyModifiers.Control,
        });

        Assert.Equal([16], model.AsyncCommands);
    }

    [Fact]
    public void A_standalone_accelerator_needs_no_menu_at_all()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var model = new MenuModel();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        var window = new Window();

        using var shortcuts = SampleViewModelMenus.AttachShortcuts(window, adapter);

        var binding = Assert.Single(window.KeyBindings);
        Assert.Equal("Ctrl+R", binding.Gesture.ToString());
        binding.TryHandle(new KeyEventArgs
        {
            RoutedEvent = InputElement.KeyDownEvent,
            Key = Key.R,
            KeyModifiers = KeyModifiers.Control,
        });

        Assert.Equal([14], model.Commands);
    }

    [Fact]
    public void Detaching_removes_the_menu_and_its_key_bindings()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var model = new MenuModel();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        var window = new Window();

        var attachment = SampleViewModelMenus.AttachMain(window, adapter);
        attachment.Dispose();

        Assert.Null(NativeMenu.GetMenu(window));
        Assert.Empty(window.KeyBindings);
    }

    [Fact]
    public void A_toggle_item_mirrors_and_writes_its_declared_property()
    {
        var model = new MenuModel();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        using var scope = new RustMenuScope(adapter);
        var menu = SampleViewModelMenus.CreateMain(adapter, scope);
        var details = (NativeMenuItem)((NativeMenuItem)menu.Items[2]).Menu!.Items[0];

        Assert.True(details.IsChecked);

        details.Command!.Execute(null);

        Assert.False(adapter.ShowTraceDetails);
        Assert.False(details.IsChecked);
        Assert.Equal([false], model.BooleanWrites);
    }

    [Fact]
    public void A_radio_group_writes_its_value_and_only_one_item_stays_checked()
    {
        var model = new MenuModel();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        using var scope = new RustMenuScope(adapter);
        var menu = SampleViewModelMenus.CreateMain(adapter, scope);
        var view = ((NativeMenuItem)menu.Items[2]).Menu!;
        var radios = view.Items.OfType<NativeMenuItem>()
            .Where(item => item.ToggleType == MenuItemToggleType.Radio)
            .ToArray();

        Assert.Equal([false, true, false], radios.Select(item => item.IsChecked));

        radios[2].Command!.Execute(null);

        Assert.Equal(Priority.High, adapter.Priority);
        Assert.Equal([false, false, true], radios.Select(item => item.IsChecked));
    }

    [Fact]
    public void Rust_command_state_still_drives_a_menu_item_enabled_state()
    {
        var model = new MenuModel();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        using var scope = new RustMenuScope(adapter);
        var menu = SampleViewModelMenus.CreateMain(adapter, scope);
        var open = (NativeMenuItem)((NativeMenuItem)menu.Items[0]).Menu!.Items[0];

        Assert.True(open.IsEnabled);

        // The generated DelegateCommand is what Rust disables through
        // SetCommandEnabled; the menu command forwards its CanExecuteChanged.
        adapter.OpenFilesCommand.SetEnabled(false);

        Assert.False(open.IsEnabled);
    }

    [Fact]
    public void Disposing_the_scope_stops_every_observer()
    {
        var model = new MenuModel();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        var scope = new RustMenuScope(adapter);
        var menu = SampleViewModelMenus.CreateMain(adapter, scope);
        var details = (NativeMenuItem)((NativeMenuItem)menu.Items[2]).Menu!.Items[0];

        scope.Dispose();
        adapter.ShowTraceDetails = false;

        Assert.True(details.IsChecked);
        Assert.True(scope.IsDisposed);
    }

    [Fact]
    public void The_recent_file_submenu_follows_the_rust_owned_collection()
    {
        var model = new MenuModel();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        using var scope = new RustMenuScope(adapter);
        var menu = SampleViewModelMenus.CreateMain(adapter, scope);
        var recent = ((NativeMenuItem)((NativeMenuItem)menu.Items[0]).Menu!.Items[2]).Menu!;

        var placeholder = Assert.Single(recent.Items.Cast<NativeMenuItem>());
        Assert.Equal(SampleViewModelMenus.RecentFilesEmptyHeader, placeholder.Header);
        Assert.False(placeholder.IsEnabled);

        model.PublishRecentFiles("file:///logs/a.log", "file:///logs/b.log");

        var items = recent.Items.Cast<NativeMenuItem>().ToArray();
        Assert.Equal(["a.log", "b.log"], items.Select(item => item.Header));
        Assert.Equal("file:///logs/a.log", items[0].CommandParameter);

        items[0].Command!.Execute(items[0].CommandParameter);

        Assert.Equal([(15, "file:///logs/a.log")], model.ParameterizedCommands);
    }

    [Fact]
    public void A_recent_file_list_is_bounded_by_the_declared_capacity()
    {
        var model = new MenuModel();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        using var scope = new RustMenuScope(adapter);
        var menu = SampleViewModelMenus.CreateMain(adapter, scope);
        var recent = ((NativeMenuItem)((NativeMenuItem)menu.Items[0]).Menu!.Items[2]).Menu!;

        model.PublishRecentFiles(Enumerable
            .Range(0, SampleViewModelMenus.RecentFilesCapacity + 5)
            .Select(index => $"file:///logs/{index}.log")
            .ToArray());

        Assert.Equal(SampleViewModelMenus.RecentFilesCapacity, recent.Items.Count);
    }

    [Fact]
    public void A_recent_file_header_falls_back_to_the_whole_uri_when_it_has_no_segments()
    {
        Assert.Equal("a.log", RustMenu.RecentFileHeader("file:///logs/a.log"));
        Assert.Equal("a b.log", RustMenu.RecentFileHeader("file:///logs/a%20b.log"));
        Assert.Equal("7", RustMenu.RecentFileHeader("content://media/documents/7"));
        Assert.Equal("a.log", RustMenu.RecentFileHeader(@"C:\logs\a.log"));
        Assert.Equal("", RustMenu.RecentFileHeader(null));
    }

    [Fact]
    public void The_generated_context_menu_binds_itself_from_the_controls_data_context()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var model = new MenuModel();
        using var adapter = new SampleViewModelAdapter(model, action => action());
        var contextMenu = new SampleViewModelTraceRowsContextMenu();

        Assert.Null(contextMenu.ItemsSource);

        contextMenu.DataContext = adapter;

        var items = contextMenu.ItemsSource!.Cast<Control>().ToArray();
        Assert.Equal(4, items.Length);
        Assert.Equal("Copy row", ((MenuItem)items[0]).Header);
        Assert.IsType<Separator>(items[1]);
        Assert.Equal(MenuItemToggleType.CheckBox, ((MenuItem)items[2]).ToggleType);

        ((MenuItem)items[0]).Command!.Execute(null);
        Assert.Equal([16], model.AsyncCommands);
    }

    [Fact]
    public void Replacing_the_context_menu_data_context_rebuilds_and_releases_the_previous_binding()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var first = new MenuModel();
        using var firstAdapter = new SampleViewModelAdapter(first, action => action());
        var second = new MenuModel();
        using var secondAdapter = new SampleViewModelAdapter(second, action => action());
        var contextMenu = new SampleViewModelTraceRowsContextMenu { DataContext = firstAdapter };
        var firstToggle = (MenuItem)contextMenu.ItemsSource!.Cast<Control>().ElementAt(2);

        contextMenu.DataContext = secondAdapter;
        firstAdapter.ShowTraceDetails = false;

        Assert.True(firstToggle.IsChecked);
        var secondToggle = (MenuItem)contextMenu.ItemsSource!.Cast<Control>().ElementAt(2);
        Assert.NotSame(firstToggle, secondToggle);

        contextMenu.DataContext = null;
        Assert.Null(contextMenu.ItemsSource);
    }

    [Fact]
    public void A_gesture_is_parsed_once_and_rejected_loudly_when_malformed()
    {
        Assert.Null(RustMenu.ParseGesture(null));
        Assert.Null(RustMenu.ParseGesture("   "));
        Assert.Equal(Key.O, RustMenu.ParseGesture("Ctrl+O")!.Key);
        Assert.Throws<ArgumentException>(() => RustMenu.ParseGesture("Ctrl+NotAKey"));
    }

    /// <summary>
    /// A minimal Rust-side stand-in that records what the menu invoked. Only
    /// the members the menus touch are implemented; everything else is a
    /// no-op success so the adapter can attach.
    /// </summary>
    private sealed class MenuModel : IAvnRustViewModel
    {
        private IAvnRustVmSink? _sink;

        public List<int> Commands { get; } = [];

        public List<int> AsyncCommands { get; } = [];

        public List<(int Id, string? Value)> ParameterizedCommands { get; } = [];

        public List<bool> BooleanWrites { get; } = [];

        public void PublishRecentFiles(params string[] uris)
        {
            var sink = (IRustVmStringSnapshotSink)_sink!;
            sink.ReplaceStringSnapshot(7, uris);
        }

        public int Attach(IAvnRustVmSink? sink)
        {
            _sink = sink;
            return 0;
        }

        public int Detach()
        {
            _sink = null;
            return 0;
        }

        public int SetString(int propertyId, string? value) => 0;

        public int SetInteger(int propertyId, long value) => 0;

        public int SetBoolean(int propertyId, int value)
        {
            BooleanWrites.Add(value != 0);
            return 0;
        }

        public int SetDouble(int propertyId, double value) => 0;

        public int Execute(int commandId, string? parameter)
        {
            if (parameter is null)
                Commands.Add(commandId);
            else
                ParameterizedCommands.Add((commandId, parameter));
            return 0;
        }

        public int BeginAsync(int commandId, string? parameter)
        {
            AsyncCommands.Add(commandId);
            return 0;
        }
    }
}
