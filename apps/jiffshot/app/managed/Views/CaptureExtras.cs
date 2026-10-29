using System;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Styling;
using Avalonia.Threading;

namespace Jiffshot.Editor.Views;

/// <summary>
/// The Capture Widget: a thin strip at the top of the primary screen that
/// opens into capture buttons when the pointer comes near, and closes again
/// when it leaves. It never takes the focus.
/// </summary>
internal sealed class WidgetWindow : Window
{
    private static WidgetWindow? _current;
    private readonly StackPanel _buttons;
    private readonly DispatcherTimer _collapse = new() { Interval = TimeSpan.FromMilliseconds(400) };

    private WidgetWindow(Action<string> run, ThemeVariant theme)
    {
        Title = "Jiffshot Capture Widget";
        RequestedThemeVariant = theme;
        WindowDecorations = WindowDecorations.None;
        Topmost = true;
        ShowActivated = false;
        ShowInTaskbar = false;
        CanResize = false;
        SizeToContent = SizeToContent.WidthAndHeight;
        var dark = theme != ThemeVariant.Light;
        Background = new SolidColorBrush(Color.Parse(dark ? "#2B2D31" : "#FFFFFF"));
        _buttons = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 2, Margin = new Thickness(6, 4), IsVisible = false };
        foreach (var (mode, glyph, tip) in new[] { ("image", "\uE722", "Image"), ("video", "\uE714", "Video"), ("scrolling", "\uE74B", "Scrolling"), ("text", "\uE8D2", "Grab text") })
        {
            var button = new Button
            {
                Content = new TextBlock { Text = glyph, FontFamily = new FontFamily("Segoe Fluent Icons, Segoe MDL2 Assets"), FontSize = 16 },
                Background = Brushes.Transparent,
                Padding = new Thickness(10, 6),
            };
            ToolTip.SetTip(button, tip);
            button.Click += (_, _) =>
            {
                Collapse();
                run("widget:" + mode);
            };
            _buttons.Children.Add(button);
        }
        var strip = new Border { Width = 90, Height = 6, Background = new SolidColorBrush(Color.Parse("#3D8BFF")), CornerRadius = new CornerRadius(0, 0, 3, 3) };
        Content = new StackPanel { Children = { _buttons, strip } };
        PointerEntered += (_, _) =>
        {
            _collapse.Stop();
            _buttons.IsVisible = true;
        };
        PointerExited += (_, _) => _collapse.Start();
        _collapse.Tick += (_, _) => Collapse();
        SizeChanged += (_, _) => Place();
        Opened += (_, _) =>
        {
            HudNative.Prepare(this, clickThrough: false);
            Place();
        };
    }

    private void Collapse()
    {
        _collapse.Stop();
        _buttons.IsVisible = false;
    }

    /// Centered at the top of the primary screen.
    private void Place()
    {
        if (Screens.Primary is not { } screen)
            return;
        var width = (int)Math.Ceiling(Bounds.Width * screen.Scaling);
        Position = new PixelPoint(screen.Bounds.X + (screen.Bounds.Width - width) / 2, screen.Bounds.Y);
    }

    /// Shows or hides the widget.
    public static void Set(bool on, Action<string> run, ThemeVariant theme)
    {
        if (on && _current is null)
        {
            _current = new WidgetWindow(run, theme);
            _current.Show();
        }
        else if (!on && _current is { } widget)
        {
            _current = null;
            widget.Close();
        }
    }
}

/// <summary>Grab Text's result: the recognized text, editable, with Copy all.</summary>
internal sealed class GrabTextWindow : Window
{
    private static GrabTextWindow? _current;
    private readonly TextBox _text;

    private GrabTextWindow(ThemeVariant theme)
    {
        Title = "Grab Text";
        Icon = new WindowIcon(Avalonia.Platform.AssetLoader.Open(new Uri("avares://Consumer.Presentation/Assets/jiffshot.ico")));
        RequestedThemeVariant = theme;
        Width = 560;
        Height = 420;
        Topmost = true;
        WindowStartupLocation = WindowStartupLocation.CenterScreen;
        _text = new TextBox { AcceptsReturn = true, TextWrapping = TextWrapping.Wrap, FontFamily = new FontFamily("Consolas, Cascadia Mono, monospace") };
        var copy = new Button { Content = "Copy all", MinWidth = 90, HorizontalContentAlignment = HorizontalAlignment.Center };
        copy.Click += async (_, _) =>
        {
            if (Clipboard is { } clipboard)
            {
                var data = new DataTransfer();
                data.Add(DataTransferItem.CreateText(_text.Text ?? ""));
                await clipboard.SetDataAsync(data);
            }
        };
        var close = new Button { Content = "Close", MinWidth = 90, IsCancel = true, HorizontalContentAlignment = HorizontalAlignment.Center };
        close.Click += (_, _) => Close();
        var buttons = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, HorizontalAlignment = HorizontalAlignment.Right, Margin = new Thickness(0, 10, 0, 0), Children = { copy, close } };
        var root = new DockPanel { Margin = new Thickness(14) };
        DockPanel.SetDock(buttons, Dock.Bottom);
        root.Children.Add(buttons);
        root.Children.Add(_text);
        Content = root;
        Closed += (_, _) =>
        {
            if (ReferenceEquals(_current, this))
                _current = null;
        };
    }

    public static void Show(string text, ThemeVariant theme)
    {
        _current ??= new GrabTextWindow(theme);
        _current._text.Text = text;
        _current.Show();
        _current.Activate();
    }
}
