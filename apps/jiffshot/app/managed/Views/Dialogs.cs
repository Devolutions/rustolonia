using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Platform.Storage;
using Jiffshot.Editor.Generated;

namespace Jiffshot.Editor.Views;

/// <summary>
/// The editor's small modal dialogs: Resize Image, Trim, Keyboard
/// Shortcuts and About (Preferences is its own window). Each one is built as a
/// window first, so the UI self-test can render it without showing it.
/// </summary>
internal static class Dialogs
{
    /// <summary>A built dialog: its window, and whether OK closed it.</summary>
    internal sealed class Built(Window window, Func<bool> accepted)
    {
        public Window Window { get; } = window;
        public bool Accepted => accepted();

        public async Task<bool> AskAsync(Window owner)
        {
            await Window.ShowDialog(owner);
            return Accepted;
        }
    }

    /// <summary>A window with <paramref name="body"/> above OK and Cancel (or Close alone).</summary>
    private static Built Frame(Window owner, string title, Control body, string? ok = "OK")
    {
        var accepted = false;
        var window = new Window
        {
            Title = title,
            SizeToContent = SizeToContent.WidthAndHeight,
            CanResize = false,
            ShowInTaskbar = false,
            WindowStartupLocation = WindowStartupLocation.CenterOwner,
            RequestedThemeVariant = owner.ActualThemeVariant,
        };
        var buttons = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, HorizontalAlignment = HorizontalAlignment.Right, Margin = new Thickness(0, 16, 0, 0) };
        if (ok is not null)
        {
            var okButton = new Button { Content = ok, IsDefault = true, MinWidth = 80, HorizontalContentAlignment = HorizontalAlignment.Center };
            okButton.Click += (_, _) =>
            {
                accepted = true;
                window.Close();
            };
            buttons.Children.Add(okButton);
        }
        var cancel = new Button { Content = ok is null ? "Close" : "Cancel", IsCancel = true, MinWidth = 80, HorizontalContentAlignment = HorizontalAlignment.Center };
        cancel.Click += (_, _) => window.Close();
        buttons.Children.Add(cancel);
        var root = new DockPanel { Margin = new Thickness(20) };
        DockPanel.SetDock(buttons, Dock.Bottom);
        root.Children.Add(buttons);
        root.Children.Add(body);
        window.Content = root;
        return new Built(window, () => accepted);
    }

    private static TextBlock Label(string text) => new() { Text = text, VerticalAlignment = VerticalAlignment.Center, Margin = new Thickness(0, 0, 12, 0) };

    private static TextBlock Heading(string text) => new() { Text = text, FontWeight = FontWeight.SemiBold, Margin = new Thickness(0, 14, 0, 6) };

    private static TextBlock Note(string text) => new() { Text = text, FontSize = 11.5, Opacity = 0.7, TextWrapping = TextWrapping.Wrap, MaxWidth = 380, Margin = new Thickness(0, 6, 0, 0) };

    private static NumericUpDown Number(decimal value, decimal min, decimal max) =>
        new() { Value = value, Minimum = min, Maximum = max, Increment = 1, FormatString = "0", Width = 150 };

    /// <summary>A two-column grid of labels and controls.</summary>
    private static Grid Rows(params (string Label, Control Control)[] rows)
    {
        var grid = new Grid { ColumnDefinitions = new ColumnDefinitions("Auto,*"), RowSpacing = 8 };
        for (var i = 0; i < rows.Length; i++)
        {
            grid.RowDefinitions.Add(new RowDefinition(GridLength.Auto));
            var label = Label(rows[i].Label);
            Grid.SetRow(label, i);
            Grid.SetRow(rows[i].Control, i);
            Grid.SetColumn(rows[i].Control, 1);
            grid.Children.Add(label);
            grid.Children.Add(rows[i].Control);
        }
        return grid;
    }

    /// <summary>Image ▸ Resize image: pixels or percent, keeping proportions or not.</summary>
    public static (Built Dialog, Func<(int W, int H)> Size) ResizeImage(Window owner, int w, int h)
    {
        var width = Number(w, 1, 16384);
        var height = Number(h, 1, 16384);
        var percent = Number(100, 1, 1000);
        var keep = new CheckBox { Content = "Keep proportions", IsChecked = true };
        var syncing = false;
        void Sync(Action change)
        {
            if (syncing)
                return;
            syncing = true;
            change();
            syncing = false;
        }
        width.ValueChanged += (_, _) => Sync(() =>
        {
            if (keep.IsChecked == true && width.Value is { } v)
            {
                height.Value = Math.Max(1, Math.Round(v * h / w));
                percent.Value = Math.Round(v * 100 / w);
            }
        });
        height.ValueChanged += (_, _) => Sync(() =>
        {
            if (keep.IsChecked == true && height.Value is { } v)
            {
                width.Value = Math.Max(1, Math.Round(v * w / h));
                percent.Value = Math.Round(v * 100 / h);
            }
        });
        percent.ValueChanged += (_, _) => Sync(() =>
        {
            if (percent.Value is { } p)
            {
                width.Value = Math.Max(1, Math.Round(w * p / 100));
                height.Value = Math.Max(1, Math.Round(h * p / 100));
            }
        });
        var body = new StackPanel
        {
            Children =
            {
                new TextBlock { Text = $"Now {w} × {h} pixels.", Margin = new Thickness(0, 0, 0, 12) },
                Rows(("Width (px)", width), ("Height (px)", height), ("Percent", percent)),
                new Border { Height = 8 },
                keep,
                Note("Objects on the image are scaled with it."),
            },
        };
        var dialog = Frame(owner, "Resize image", body, "Resize");
        return (dialog, () => ((int)(width.Value ?? w), (int)(height.Value ?? h)));
    }

    /// <summary>Image ▸ Trim: pixels to remove from each side.</summary>
    public static (Built Dialog, Func<(int L, int T, int R, int B)> Sides) Trim(Window owner, int w, int h)
    {
        var left = Number(0, 0, w - 1);
        var top = Number(0, 0, h - 1);
        var right = Number(0, 0, w - 1);
        var bottom = Number(0, 0, h - 1);
        var body = new StackPanel
        {
            Children =
            {
                new TextBlock { Text = "Pixels to remove from each side:", Margin = new Thickness(0, 0, 0, 12) },
                Rows(("Left", left), ("Top", top), ("Right", right), ("Bottom", bottom)),
                Note("Image ▸ Auto trim removes edges of one solid color by itself."),
            },
        };
        var dialog = Frame(owner, "Trim", body, "Trim");
        return (dialog, () => ((int)(left.Value ?? 0), (int)(top.Value ?? 0), (int)(right.Value ?? 0), (int)(bottom.Value ?? 0)));
    }

    private static readonly (string Keys, string What)[] Shortcuts =
    {
        ("Ctrl+Shift+N", "New capture"), ("Ctrl+N", "New image"), ("Ctrl+O", "Open"), ("Ctrl+S", "Save"), ("Ctrl+Shift+S", "Save as"),
        ("Ctrl+E", "Export"), ("Ctrl+P", "Print"), ("Ctrl+W", "Close"), ("Ctrl+Shift+V", "New from clipboard"),
        ("Ctrl+Z / Ctrl+Y", "Undo / redo"), ("Ctrl+C / Ctrl+X / Ctrl+V", "Copy / cut / paste"), ("Ctrl+Shift+C", "Copy all"),
        ("Ctrl+A / Ctrl+D", "Select all / deselect"), ("Ctrl+Shift+I", "Invert selection"), ("Ctrl+Shift+G / U", "Group / ungroup"),
        ("Ctrl+Shift+F / B", "Bring forward / send backward"), ("Ctrl+T / Ctrl+Shift+T", "Flatten selected / all"),
        ("Ctrl+1 … Ctrl+9", "The first nine tools"), ("Ctrl++ / Ctrl+−", "Zoom in / out"), ("Ctrl+0", "Actual size"),
        ("Ctrl+Alt+0", "Fit"), ("Ctrl+G", "Pixel grid"), ("Ctrl+L", "Library view"), ("Ctrl+,", "Editor preferences"),
        ("Esc", "Deselect"), ("F1", "These shortcuts"),
    };

    /// <summary>Help ▸ Keyboard shortcuts.</summary>
    public static Built KeyboardShortcuts(Window owner)
    {
        var grid = new Grid { ColumnDefinitions = new ColumnDefinitions("Auto,*"), RowSpacing = 4 };
        for (var i = 0; i < Shortcuts.Length; i++)
        {
            grid.RowDefinitions.Add(new RowDefinition(GridLength.Auto));
            var keys = new TextBlock { Text = Shortcuts[i].Keys, FontFamily = new FontFamily("Consolas, Cascadia Mono, monospace"), Margin = new Thickness(0, 0, 18, 0) };
            var what = new TextBlock { Text = Shortcuts[i].What };
            Grid.SetRow(keys, i);
            Grid.SetRow(what, i);
            Grid.SetColumn(what, 1);
            grid.Children.Add(keys);
            grid.Children.Add(what);
        }
        return Frame(owner, "Keyboard shortcuts", grid, ok: null);
    }

    /// <summary>Help ▸ About Jiffshot.</summary>
    public static Built About(Window owner)
    {
        var version = typeof(Dialogs).Assembly.GetName().Version?.ToString(3) ?? "";
        var body = new StackPanel
        {
            Spacing = 6,
            Children =
            {
                new TextBlock { Text = "Jiffshot Editor", FontSize = 18, FontWeight = FontWeight.SemiBold },
                new TextBlock { Text = "Version " + version },
                new TextBlock { Text = "Screen capture and image editing.", Margin = new Thickness(0, 6, 0, 0) },
            },
        };
        return Frame(owner, "About Jiffshot", body, ok: null);
    }
}
