using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.IO;
using System.Text;
using System.Text.Json;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Markup.Xaml;
using Avalonia.Media;
using Avalonia.Rust.Interop;
using Avalonia.Threading;
using Jiffshot.Editor.Generated;

namespace Jiffshot.Editor.Views;

/// <summary>
/// The Capture window. Rust decides what it shows (jiffshot-workflow's
/// panel module): the current tab's dropdowns and switches arrive as View
/// JSON, and menus and option panels as JSON when asked for. This class
/// only lays them out and sends back actions.
/// </summary>
public partial class CaptureWindow : Window
{
    private readonly CaptureViewModelAdapter _adapter;
    private Control? _anchor;

    public CaptureWindow()
    {
        AvaloniaXamlLoader.Load(this);
        if (Resources.TryGetResource("CwDecorations", null, out var theme) && theme is Avalonia.Styling.ControlTheme decorations)
            WindowDecorationsTheme = decorations;
        _adapter = null!;
    }

    public CaptureWindow(IAvnRustViewModel model)
        : this()
    {
        _adapter = new CaptureViewModelAdapter(model);
        DataContext = _adapter;
        Instance = this;
        _adapter.PropertyChanged += OnAdapterPropertyChanged;
        this.FindControl<Grid>("TitleBar")!.PointerPressed += (_, e) =>
        {
            if (e.GetCurrentPoint(this).Properties.IsLeftButtonPressed)
                BeginMoveDrag(e);
        };
        Closing += (_, e) =>
        {
            // Closing keeps it for next time; only Jiffshot's Exit ends it.
            if (e.CloseReason is WindowCloseReason.ApplicationShutdown or WindowCloseReason.OSShutdown
                || Environment.GetEnvironmentVariable("JIFFSHOT_UITEST") is { Length: > 0 })
                return;
            e.Cancel = true;
            Hide();
            Run("closed");
        };
        Closed += (_, _) =>
        {
            _adapter.PropertyChanged -= OnAdapterPropertyChanged;
            _adapter.Dispose();
        };
        ApplyView();
    }

    /// <summary>The window the host created (mounted hidden at startup).</summary>
    internal static CaptureWindow? Instance { get; private set; }

    /// <summary>Sends an action as the window's own controls do (for the UI self-test).</summary>
    internal void RunAction(string action) => Run(action);

    /// <summary>The view JSON (for the UI self-test's log).</summary>
    internal string ViewJson => _adapter.View;

    private void Run(string action)
    {
        _adapter.Action = action;
        _adapter.RunCommand.Execute(null);
    }

    private void OnAdapterPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        switch (e.PropertyName)
        {
            case nameof(CaptureViewModelAdapter.View):
                Dispatcher.UIThread.Post(ApplyView);
                break;
            case nameof(CaptureViewModelAdapter.Menu):
                if (Payload(_adapter.Menu) is { } menu)
                    Dispatcher.UIThread.Post(() => ShowMenu(menu));
                break;
            case nameof(CaptureViewModelAdapter.Panel):
                if (Payload(_adapter.Panel) is { } panel)
                    Dispatcher.UIThread.Post(() => ShowPanel(panel));
                break;
            case nameof(CaptureViewModelAdapter.Problem):
                if (Payload(_adapter.Problem) is { } text)
                    Dispatcher.UIThread.Post(() => ShowProblem(text));
                break;
        }
    }

    /// The JSON (or text) after the serial of a "serial\x1fpayload" value.
    private static string? Payload(string value) =>
        value.IndexOf('\u001f') is var at and >= 0 ? value[(at + 1)..] : null;

    // ---- the view ----------------------------------------------------------

    private void ApplyView()
    {
        using var json = JsonDocument.Parse(string.IsNullOrEmpty(_adapter.View) ? "{}" : _adapter.View);
        var view = json.RootElement;
        if (!view.TryGetProperty("tab", out var tabValue))
            return;
        var tab = tabValue.GetInt32();
        for (var i = 0; i < 3; i++)
            this.FindControl<Button>("Tab" + i)!.Classes.Set("on", i == tab);

        var fields = this.FindControl<StackPanel>("Fields")!;
        fields.Children.Clear();
        foreach (var field in view.GetProperty("fields").EnumerateArray())
            fields.Children.Add(FieldRow(field));

        var switches = this.FindControl<StackPanel>("Switches")!;
        switches.Children.Clear();
        var rows = view.GetProperty("switches");
        var rowHeight = rows.GetArrayLength() > 4 ? 31 : 45;
        foreach (var row in rows.EnumerateArray())
            switches.Children.Add(SwitchRow(row, rowHeight));

        this.FindControl<TextBlock>("CaptureLabel")!.Text = view.GetProperty("capture").GetString();
        this.FindControl<TextBlock>("CaptureHint")!.Text = view.GetProperty("hint").GetString();
    }

    private Control FieldRow(JsonElement field)
    {
        var id = field.GetProperty("id").GetString()!;
        var grid = new Grid { ColumnDefinitions = new ColumnDefinitions("100,*,Auto"), Height = 30 };
        grid.Children.Add(new TextBlock { Text = field.GetProperty("label").GetString(), VerticalAlignment = VerticalAlignment.Center, FontSize = 13.5 });

        var drop = new Button { Classes = { "drop" }, Width = 200 };
        var content = new DockPanel();
        var arrow = new TextBlock { Text = "\uE70D", FontFamily = new FontFamily("Segoe Fluent Icons, Segoe MDL2 Assets"), FontSize = 10, VerticalAlignment = VerticalAlignment.Center, Margin = new Thickness(6, 0, 0, 0) };
        DockPanel.SetDock(arrow, Dock.Right);
        content.Children.Add(arrow);
        content.Children.Add(new TextBlock { Text = field.GetProperty("text").GetString(), TextTrimming = TextTrimming.CharacterEllipsis, VerticalAlignment = VerticalAlignment.Center, FontSize = 13 });
        drop.Content = content;
        ToolTip.SetTip(drop, field.GetProperty("text").GetString());
        drop.Click += (_, _) => RequestMenu(id, drop);
        Grid.SetColumn(drop, 1);
        grid.Children.Add(drop);

        if (field.TryGetProperty("gear", out var gear) && gear.ValueKind == JsonValueKind.String)
        {
            var panel = gear.GetString()!;
            var button = new Button { Classes = { "flat" }, Padding = new Thickness(6, 4), Margin = new Thickness(4, 0, 0, 0) };
            button.Content = new TextBlock { Text = "\uE713", FontFamily = new FontFamily("Segoe Fluent Icons, Segoe MDL2 Assets"), FontSize = 14 };
            ToolTip.SetTip(button, "Options");
            button.Click += (_, _) => RequestPanel(panel, button);
            Grid.SetColumn(button, 2);
            grid.Children.Add(button);
        }
        return grid;
    }

    private Control SwitchRow(JsonElement row, double height)
    {
        var id = row.GetProperty("id").GetString()!;
        var grid = new Grid { ColumnDefinitions = new ColumnDefinitions("*,Auto,26"), Height = height };
        var label = new TextBlock { Text = row.GetProperty("label").GetString(), VerticalAlignment = VerticalAlignment.Center, FontSize = 13.5 };
        grid.Children.Add(label);
        var toggle = new ToggleSwitch
        {
            IsChecked = row.GetProperty("on").GetBoolean(),
            OnContent = null,
            OffContent = null,
            VerticalAlignment = VerticalAlignment.Center,
            Margin = new Thickness(8, 0, 0, 0),
        };
        toggle.Click += (_, _) => Run("switch:" + id);
        Grid.SetColumn(toggle, 1);
        grid.Children.Add(toggle);
        if (row.TryGetProperty("more", out var more) && more.ValueKind == JsonValueKind.String)
        {
            var panel = more.GetString()!;
            var button = new Button { Classes = { "flat" }, Padding = new Thickness(5, 3) };
            button.Content = new TextBlock { Text = "\uE76C", FontFamily = new FontFamily("Segoe Fluent Icons, Segoe MDL2 Assets"), FontSize = 11 };
            ToolTip.SetTip(button, "Options");
            button.Click += (_, _) => RequestPanel(panel, button);
            Grid.SetColumn(button, 2);
            grid.Children.Add(button);
        }
        return grid;
    }

    // ---- buttons -----------------------------------------------------------

    private void OnTab(object? sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: string tab })
            Run("tab:" + tab);
    }

    private void OnMenuButton(object? sender, RoutedEventArgs e)
    {
        if (sender is Button { Tag: string which } button)
            RequestMenu(which, button);
    }

    private void OnCapture(object? sender, RoutedEventArgs e) => Run("capture");

    private void OnReset(object? sender, RoutedEventArgs e) => Run("pick:reset");

    private void OnEditor(object? sender, RoutedEventArgs e) => Run("editor");

    private void RequestMenu(string which, Control anchor)
    {
        _anchor = anchor;
        Run("menu:" + which);
    }

    private void RequestPanel(string which, Control anchor)
    {
        _anchor = anchor;
        Run("panel:" + which);
    }

    protected override void OnKeyDown(KeyEventArgs e)
    {
        base.OnKeyDown(e);
        if (e.Handled)
            return;
        var ctrl = (e.KeyModifiers & KeyModifiers.Control) != 0;
        switch (e.Key)
        {
            case Key.Enter:
                Run("capture");
                break;
            case Key.Escape:
                Hide();
                Run("closed");
                break;
            case Key.D1 or Key.D2 or Key.D3 when ctrl:
                Run("tab:" + (e.Key - Key.D1));
                break;
            default:
                return;
        }
        e.Handled = true;
    }

    // ---- menus -------------------------------------------------------------

    private void ShowMenu(string json)
    {
        using var doc = JsonDocument.Parse(json);
        var flyout = new MenuFlyout { Placement = PlacementMode.BottomEdgeAlignedLeft };
        foreach (var item in doc.RootElement.EnumerateArray())
            flyout.Items.Add(MenuEntry(item));
        flyout.ShowAt(_anchor ?? this);
    }

    private object MenuEntry(JsonElement item)
    {
        if (item.GetProperty("separator").GetBoolean())
            return new Separator();
        var id = item.GetProperty("id").GetString()!;
        var label = item.GetProperty("label").GetString()!;
        var parts = label.Split('\t');
        var menuItem = new MenuItem
        {
            Header = parts[0],
            IsEnabled = item.GetProperty("enabled").GetBoolean(),
            StaysOpenOnClick = item.GetProperty("keep_open").GetBoolean(),
        };
        if (parts.Length > 1)
        {
            // A shortcut or preset hotkey, as text: not every one parses as a gesture.
            var header = new DockPanel { MinWidth = 220 };
            var keys = new TextBlock { Text = parts[1], Opacity = 0.65, Margin = new Thickness(24, 0, 0, 0) };
            DockPanel.SetDock(keys, Dock.Right);
            header.Children.Add(keys);
            header.Children.Add(new TextBlock { Text = parts[0] });
            menuItem.Header = header;
        }
        var children = item.GetProperty("items");
        if (children.GetArrayLength() > 0)
        {
            foreach (var child in children.EnumerateArray())
                menuItem.Items.Add(MenuEntry(child));
            if (item.GetProperty("checked").GetBoolean())
                menuItem.Header = parts[0] + "  ✓";
            return menuItem;
        }
        if (item.GetProperty("checked").GetBoolean() || menuItem.StaysOpenOnClick)
        {
            menuItem.ToggleType = MenuItemToggleType.CheckBox;
            menuItem.IsChecked = item.GetProperty("checked").GetBoolean();
        }
        menuItem.Click += (_, _) =>
        {
            if (menuItem.StaysOpenOnClick)
                menuItem.IsChecked = !menuItem.IsChecked;
            if (id.StartsWith("window:", StringComparison.Ordinal))
                WindowCommand(id["window:".Length..]);
            else if (id.Length > 0)
                Run("pick:" + id);
        };
        return menuItem;
    }

    /// Menu commands about this window itself.
    private void WindowCommand(string what)
    {
        switch (what)
        {
            case "topmost":
                Topmost = !Topmost;
                break;
            case "about":
                _ = Dialogs.About(this).AskAsync(this);
                break;
        }
    }

    // ---- option panels -----------------------------------------------------

    private void ShowPanel(string json)
    {
        using var doc = JsonDocument.Parse(json);
        var panel = doc.RootElement;
        var id = panel.GetProperty("id").GetString()!;
        var body = new StackPanel { Spacing = 8, Margin = new Thickness(4), MinWidth = 280 };
        body.Children.Add(new TextBlock { Text = panel.GetProperty("title").GetString(), FontWeight = FontWeight.SemiBold, FontSize = 14, Margin = new Thickness(0, 0, 0, 4) });
        var read = new List<Func<object>>();
        foreach (var row in panel.GetProperty("rows").EnumerateArray())
            body.Children.Add(PanelRow(row, read));

        var flyout = new Flyout { Content = body, Placement = PlacementMode.BottomEdgeAlignedLeft };
        var ok = panel.GetProperty("ok");
        var buttons = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, HorizontalAlignment = HorizontalAlignment.Right, Margin = new Thickness(0, 8, 0, 0) };
        if (ok.ValueKind == JsonValueKind.String)
        {
            var apply = new Button { Content = ok.GetString(), IsDefault = true, MinWidth = 72, HorizontalContentAlignment = HorizontalAlignment.Center };
            apply.Click += (_, _) =>
            {
                flyout.Hide();
                Run($"apply:{id}:{Values(read)}");
            };
            buttons.Children.Add(apply);
        }
        var close = new Button { Content = ok.ValueKind == JsonValueKind.String ? "Cancel" : "Close", MinWidth = 72, HorizontalContentAlignment = HorizontalAlignment.Center };
        close.Click += (_, _) => flyout.Hide();
        buttons.Children.Add(close);
        body.Children.Add(buttons);
        flyout.ShowAt(_anchor ?? this);
    }

    private static Control PanelRow(JsonElement row, List<Func<object>> read)
    {
        var kind = row.GetProperty("kind").GetString();
        string Label() => row.GetProperty("label").GetString() ?? "";
        Control Labeled(Control control)
        {
            var grid = new Grid { ColumnDefinitions = new ColumnDefinitions("150,*") };
            grid.Children.Add(new TextBlock { Text = Label(), VerticalAlignment = VerticalAlignment.Center });
            Grid.SetColumn(control, 1);
            grid.Children.Add(control);
            return grid;
        }
        switch (kind)
        {
            case "number":
            {
                var box = new NumericUpDown
                {
                    Value = row.GetProperty("value").GetInt64(),
                    Minimum = row.GetProperty("min").GetInt64(),
                    Maximum = row.GetProperty("max").GetInt64(),
                    Increment = 1,
                    FormatString = "0",
                    MinWidth = 130,
                };
                read.Add(() => (long)(box.Value ?? 0));
                return Labeled(box);
            }
            case "check":
            {
                var box = new CheckBox { Content = Label(), IsChecked = row.GetProperty("value").GetBoolean() };
                read.Add(() => box.IsChecked == true);
                return box;
            }
            case "choice":
            {
                var choices = new List<string>();
                foreach (var c in row.GetProperty("choices").EnumerateArray())
                    choices.Add(c.GetString() ?? "");
                var box = new ComboBox { ItemsSource = choices, SelectedIndex = row.GetProperty("value").GetInt32(), MinWidth = 130 };
                read.Add(() => (long)Math.Max(0, box.SelectedIndex));
                return Labeled(box);
            }
            case "text":
            {
                var box = new TextBox { Text = row.GetProperty("value").GetString(), MinWidth = 130 };
                read.Add(() => box.Text ?? "");
                return Labeled(box);
            }
            default:
                return new TextBlock { Text = row.GetProperty("text").GetString(), TextWrapping = TextWrapping.Wrap, MaxWidth = 380, FontSize = 12.5 };
        }
    }

    /// The panel's values as a JSON array, one per input row.
    private static string Values(List<Func<object>> read)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream))
        {
            writer.WriteStartArray();
            foreach (var value in read)
            {
                switch (value())
                {
                    case bool b:
                        writer.WriteBooleanValue(b);
                        break;
                    case long n:
                        writer.WriteNumberValue(n);
                        break;
                    case string s:
                        writer.WriteStringValue(s);
                        break;
                }
            }
            writer.WriteEndArray();
        }
        return Encoding.UTF8.GetString(stream.ToArray());
    }

    private void ShowProblem(string text)
    {
        var flyout = new Flyout
        {
            Content = new TextBlock { Text = text, TextWrapping = TextWrapping.Wrap, MaxWidth = 320, Margin = new Thickness(4) },
            Placement = PlacementMode.BottomEdgeAlignedLeft,
        };
        flyout.ShowAt(_anchor ?? this);
    }
}
