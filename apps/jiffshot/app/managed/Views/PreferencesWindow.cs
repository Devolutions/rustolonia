using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Nodes;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Platform.Storage;

namespace Jiffshot.Editor.Views;

/// <summary>
/// Jiffshot's one Preferences dialog: Capture, Editor, Library, Share and
/// Hotkeys. The tabs and their rows come from Rust (<c>src/prefs.rs</c>,
/// the <c>Prefs</c> property) in the Capture window's row kinds (number,
/// check, choice, text, folder, note); OK sends one value per row and tab
/// as <c>prefs-apply:{"capture":[…],…}</c>.
/// </summary>
internal sealed class PreferencesWindow : Window
{
    private static PreferencesWindow? _current;
    private readonly List<(string Id, List<Func<JsonNode?>> Values)> _tabs = new();
    private readonly TabControl _tabControl = new();

    private PreferencesWindow(JsonElement root, Action<string> run)
    {
        Title = "Jiffshot Preferences";
        Icon = new WindowIcon(Avalonia.Platform.AssetLoader.Open(new Uri("avares://Consumer.Presentation/Assets/jiffshot.ico")));
        Width = 640;
        Height = 560;
        MinWidth = 520;
        MinHeight = 420;
        WindowStartupLocation = WindowStartupLocation.CenterScreen;
        var open = root.GetProperty("tab").GetString();
        var index = 0;
        foreach (var tab in root.GetProperty("tabs").EnumerateArray())
        {
            var id = tab.GetProperty("id").GetString() ?? "";
            var values = new List<Func<JsonNode?>>();
            var rows = new StackPanel { Spacing = 10, Margin = new Thickness(18, 16, 18, 16) };
            foreach (var row in tab.GetProperty("rows").EnumerateArray())
                rows.Children.Add(Row(row, values));
            _tabs.Add((id, values));
            _tabControl.Items.Add(new TabItem
            {
                Header = tab.GetProperty("title").GetString(),
                Content = new ScrollViewer { Content = rows },
            });
            if (id == open)
                _tabControl.SelectedIndex = index;
            index++;
        }
        var ok = new Button { Content = "OK", MinWidth = 90, IsDefault = true, HorizontalContentAlignment = HorizontalAlignment.Center };
        var cancel = new Button { Content = "Cancel", MinWidth = 90, IsCancel = true, HorizontalContentAlignment = HorizontalAlignment.Center };
        ok.Click += (_, _) =>
        {
            run("prefs-apply:" + Collect().ToJsonString());
            Close();
        };
        cancel.Click += (_, _) => Close();
        var buttons = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, HorizontalAlignment = HorizontalAlignment.Right, Margin = new Thickness(14), Children = { ok, cancel } };
        var layout = new DockPanel();
        DockPanel.SetDock(buttons, Dock.Bottom);
        layout.Children.Add(buttons);
        layout.Children.Add(_tabControl);
        Content = layout;
        Closed += (_, _) =>
        {
            if (ReferenceEquals(_current, this))
                _current = null;
        };
    }

    /// <summary>The values to send, by tab.</summary>
    internal JsonObject Collect()
    {
        var all = new JsonObject();
        foreach (var (id, values) in _tabs)
            all[id] = new JsonArray(values.Select(v => v()).ToArray());
        return all;
    }

    /// <summary>The tab headers (for the UI self-test).</summary>
    internal IEnumerable<string> TabTitles => _tabControl.Items.OfType<TabItem>().Select(t => t.Header?.ToString() ?? "");

    internal void SelectTab(int index) => _tabControl.SelectedIndex = index;

    private Control Row(JsonElement row, List<Func<JsonNode?>> values)
    {
        string Label() => row.TryGetProperty("label", out var l) ? l.GetString() ?? "" : "";
        Control Labeled(Control control)
        {
            var grid = new Grid { ColumnDefinitions = new ColumnDefinitions("220,*") };
            var label = new TextBlock { Text = Label(), VerticalAlignment = VerticalAlignment.Center, TextWrapping = TextWrapping.Wrap, Margin = new Thickness(0, 0, 12, 0) };
            Grid.SetColumn(control, 1);
            grid.Children.Add(label);
            grid.Children.Add(control);
            return grid;
        }
        switch (row.GetProperty("kind").GetString())
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
                    Width = 140,
                    HorizontalAlignment = HorizontalAlignment.Left,
                };
                values.Add(() => JsonValue.Create((long)(box.Value ?? 0)));
                return Labeled(box);
            }
            case "check":
            {
                var check = new CheckBox { Content = Label(), IsChecked = row.GetProperty("value").GetBoolean() };
                values.Add(() => JsonValue.Create(check.IsChecked == true));
                return check;
            }
            case "choice":
            {
                var combo = new ComboBox
                {
                    ItemsSource = row.GetProperty("choices").EnumerateArray().Select(c => c.GetString()).ToList(),
                    SelectedIndex = row.GetProperty("value").GetInt32(),
                    MinWidth = 220,
                };
                values.Add(() => JsonValue.Create(Math.Max(0, combo.SelectedIndex)));
                return Labeled(combo);
            }
            case "text":
            {
                var box = new TextBox { Text = row.GetProperty("value").GetString() };
                values.Add(() => JsonValue.Create(box.Text ?? ""));
                return Labeled(box);
            }
            case "folder":
            {
                var box = new TextBox { Text = row.GetProperty("value").GetString(), IsReadOnly = true };
                var choose = new Button { Content = "Choose…", Margin = new Thickness(8, 0, 0, 0) };
                choose.Click += async (_, _) =>
                {
                    var picked = await StorageProvider.OpenFolderPickerAsync(new FolderPickerOpenOptions { Title = Label() });
                    if (picked.FirstOrDefault()?.TryGetLocalPath() is { } local)
                        box.Text = local;
                };
                var line = new DockPanel();
                DockPanel.SetDock(choose, Dock.Right);
                line.Children.Add(choose);
                line.Children.Add(box);
                values.Add(() => JsonValue.Create(box.Text ?? ""));
                return Labeled(line);
            }
            default:
                values.Add(() => null);
                return new TextBlock
                {
                    Text = row.TryGetProperty("text", out var t) ? t.GetString() : "",
                    TextWrapping = TextWrapping.Wrap,
                    Opacity = 0.75,
                };
        }
    }

    /// <summary>Opens the dialog (or replaces the one showing) from the <c>Prefs</c> JSON.</summary>
    public static PreferencesWindow? Show(string json, Action<string> run, Window? owner)
    {
        using var doc = JsonDocument.Parse(json);
        _current?.Close();
        _current = new PreferencesWindow(doc.RootElement, run);
        if (owner is { IsVisible: true })
            _current.Show(owner);
        else
            _current.Show();
        _current.Activate();
        return _current;
    }
}
