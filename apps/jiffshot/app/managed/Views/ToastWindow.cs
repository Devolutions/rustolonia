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
/// A notice from the capture side ("Capture saved", "Recording failed"…):
/// a small borderless window in the bottom-right corner of the primary
/// screen that does not take the focus and goes away after a few seconds or
/// a click. One shows at a time; a new notice replaces the last.
/// </summary>
internal sealed class ToastWindow : Window
{
    private static ToastWindow? _current;
    private readonly DispatcherTimer _timer = new() { Interval = TimeSpan.FromSeconds(5) };

    private ToastWindow(string title, string text, bool problem, ThemeVariant theme)
    {
        var dark = theme != ThemeVariant.Light;
        Title = "Jiffshot: " + title;
        RequestedThemeVariant = theme;
        WindowDecorations = WindowDecorations.None;
        Topmost = true;
        ShowActivated = false;
        ShowInTaskbar = false;
        CanResize = false;
        SizeToContent = SizeToContent.WidthAndHeight;
        // See-through outside the rounded card.
        TransparencyLevelHint = [WindowTransparencyLevel.Transparent];
        Background = Brushes.Transparent;
        var accent = problem ? Color.Parse("#E5534B") : Color.Parse("#3D8BFF");
        Content = new Border
        {
            Background = new SolidColorBrush(Color.Parse(dark ? "#2B2D31" : "#FFFFFF")),
            BorderBrush = new SolidColorBrush(Color.Parse(dark ? "#3A3C40" : "#D5D8DD")),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(8),
            Padding = new Thickness(0),
            Child = new DockPanel
            {
                Children =
                {
                    new Border { Width = 4, Background = new SolidColorBrush(accent), CornerRadius = new CornerRadius(8, 0, 0, 8), [DockPanel.DockProperty] = Dock.Left },
                    new StackPanel
                    {
                        Margin = new Thickness(14, 10, 16, 12),
                        Spacing = 3,
                        Children =
                        {
                            new TextBlock { Text = title, FontWeight = FontWeight.SemiBold, FontSize = 13.5, Foreground = new SolidColorBrush(Color.Parse(dark ? "#F2F3F5" : "#1F2328")) },
                            new TextBlock { Text = text, FontSize = 12, TextWrapping = TextWrapping.Wrap, MaxWidth = 340, Foreground = new SolidColorBrush(Color.Parse(dark ? "#C8CACE" : "#4B5563")) },
                        },
                    },
                },
            },
        };
        _timer.Tick += (_, _) => Close();
        PointerPressed += (_, _) => Close();
        Opened += (_, _) =>
        {
            HudNative.Prepare(this, clickThrough: false);
            Place();
            _timer.Start();
        };
        Closed += (_, _) =>
        {
            _timer.Stop();
            if (ReferenceEquals(_current, this))
                _current = null;
        };
    }

    /// <summary>A toast, not shown yet (the UI self-test renders one).</summary>
    public static ToastWindow Build(string title, string text, bool problem, ThemeVariant theme) => new(title, text, problem, theme);

    /// <summary>Shows a notice, replacing the one on screen.</summary>
    public static void Show(string title, string text, bool problem, ThemeVariant theme)
    {
        _current?.Close();
        var toast = new ToastWindow(title, text, problem, theme);
        _current = toast;
        toast.Show();
    }

    /// <summary>The bottom-right corner of the primary screen's work area, 16 px in.</summary>
    private void Place()
    {
        if (Screens.Primary is not { } screen)
            return;
        var scale = screen.Scaling;
        var area = screen.WorkingArea;
        var size = new PixelSize((int)Math.Ceiling(Bounds.Width * scale), (int)Math.Ceiling(Bounds.Height * scale));
        var margin = (int)Math.Round(16 * scale);
        Position = new PixelPoint(area.Right - size.Width - margin, area.Bottom - size.Height - margin);
    }
}
