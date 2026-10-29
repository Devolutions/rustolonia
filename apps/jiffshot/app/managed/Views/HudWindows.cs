using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;

namespace Jiffshot.Editor.Views;

/// <summary>
/// The capture side's on-screen furniture: control panels next to the area
/// being captured (countdown, recording, scrolling, Step Capture…) and red
/// outlines just outside it. The capture side owns the state and sends all
/// of it as JSON (the <c>Hud</c> property); <see cref="Apply"/> makes the
/// windows match. None of them takes the focus or shows in screen captures.
/// </summary>
internal static class Hud
{
    private static readonly Dictionary<long, HudPanelWindow> Panels = new();
    private static readonly Dictionary<long, HudBorderWindow> Borders = new();
    private static readonly Dictionary<long, RecordingToolbar> Toolbars = new();

    /// <summary>The recording toolbars showing (for the UI self-test).</summary>
    public static IReadOnlyCollection<RecordingToolbar> OpenToolbars => Toolbars.Values;

    /// <summary>The panel windows showing (for the UI self-test).</summary>
    public static IReadOnlyCollection<HudPanelWindow> OpenPanels => Panels.Values;

    /// <summary>The outline windows showing (for the UI self-test).</summary>
    public static IReadOnlyCollection<HudBorderWindow> OpenBorders => Borders.Values;

    /// <summary>
    /// <c>{"panels":[{"id","area":[x,y,w,h],"status","buttons":[…]}],"borders":[{"id","area"}]}</c>,
    /// areas in desktop pixels. A button sends <c>hud:ID:INDEX</c> through <paramref name="run"/>.
    /// </summary>
    public static void Apply(string json, Action<string> run)
    {
        if (string.IsNullOrEmpty(json))
            return;
        using var doc = JsonDocument.Parse(json);
        var root = doc.RootElement;
        var live = new HashSet<long>();
        foreach (var p in root.GetProperty("panels").EnumerateArray())
        {
            var id = p.GetProperty("id").GetInt64();
            live.Add(id);
            if (p.TryGetProperty("toolbar", out var toolbarState))
            {
                if (!Toolbars.TryGetValue(id, out var toolbar))
                {
                    toolbar = new RecordingToolbar(id, AreaOf(p), run);
                    Toolbars[id] = toolbar;
                    toolbar.Update(toolbarState);
                    toolbar.Show();
                }
                else
                {
                    toolbar.Update(toolbarState);
                }
                continue;
            }
            var status = p.GetProperty("status").GetString() ?? "";
            if (!Panels.TryGetValue(id, out var panel))
            {
                var buttons = p.GetProperty("buttons").EnumerateArray().Select(b => b.GetString() ?? "").ToArray();
                panel = new HudPanelWindow(id, AreaOf(p), buttons, run);
                Panels[id] = panel;
                panel.SetStatus(status);
                panel.Show();
            }
            else
            {
                panel.SetStatus(status);
            }
        }
        foreach (var id in Toolbars.Keys.Where(id => !live.Contains(id)).ToList())
        {
            Toolbars[id].Close();
            Toolbars.Remove(id);
        }
        foreach (var id in Panels.Keys.Where(id => !live.Contains(id)).ToList())
        {
            Panels[id].Dismiss();
            Panels.Remove(id);
        }

        live.Clear();
        foreach (var b in root.GetProperty("borders").EnumerateArray())
        {
            var id = b.GetProperty("id").GetInt64();
            live.Add(id);
            if (!Borders.ContainsKey(id))
            {
                var border = new HudBorderWindow(AreaOf(b));
                Borders[id] = border;
                border.Show();
            }
        }
        foreach (var id in Borders.Keys.Where(id => !live.Contains(id)).ToList())
        {
            Borders[id].Close();
            Borders.Remove(id);
        }
    }

    private static PixelRect AreaOf(JsonElement e) =>
        e.GetProperty("area") is var a && a.GetArrayLength() == 4
            ? new PixelRect(a[0].GetInt32(), a[1].GetInt32(), a[2].GetInt32(), a[3].GetInt32())
            : default;
}

/// <summary>Window styles Avalonia has no property for (Windows only).</summary>
internal static partial class HudNative
{
    private const int GWL_EXSTYLE = -20;
    private const long WS_EX_TRANSPARENT = 0x20;
    private const long WS_EX_TOOLWINDOW = 0x80;
    private const long WS_EX_LAYERED = 0x80000;
    private const long WS_EX_NOACTIVATE = 0x08000000;
    private const uint LWA_ALPHA = 0x2;
    private const uint WDA_EXCLUDEFROMCAPTURE = 0x11;

    /// <summary>
    /// Never activated by a click, left out of screen captures (unless
    /// JIFFSHOT_HUD_CAPTURABLE is set, to check them on screen), and with
    /// <paramref name="clickThrough"/>, transparent to the pointer.
    /// </summary>
    public static void Prepare(Window window, bool clickThrough)
    {
        if (!OperatingSystem.IsWindows() || window.TryGetPlatformHandle()?.Handle is not { } hwnd || hwnd == 0)
            return;
        var style = (long)GetWindowLongPtrW(hwnd, GWL_EXSTYLE) | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW;
        if (clickThrough)
            style |= WS_EX_LAYERED | WS_EX_TRANSPARENT;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (nint)style);
        if (clickThrough)
            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
        if (Environment.GetEnvironmentVariable("JIFFSHOT_HUD_CAPTURABLE") is null)
            SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
    }

    /// <summary>Left out of screen captures only (it may take the focus).</summary>
    public static void ExcludeFromCapture(Window window)
    {
        if (OperatingSystem.IsWindows() && window.TryGetPlatformHandle()?.Handle is { } hwnd and not 0
            && Environment.GetEnvironmentVariable("JIFFSHOT_HUD_CAPTURABLE") is null)
            SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
    }

    /// <summary>
    /// Puts the window exactly on <paramref name="rect"/> (desktop pixels),
    /// topmost, without activating it: Avalonia's own sizing is in DIPs and
    /// goes wrong while a window moves between screens of different scales.
    /// </summary>
    public static void Place(Window window, PixelRect rect)
    {
        if (OperatingSystem.IsWindows() && window.TryGetPlatformHandle()?.Handle is { } hwnd and not 0)
            SetWindowPos(hwnd, -1, rect.X, rect.Y, rect.Width, rect.Height, SWP_NOACTIVATE);
    }

    /// <summary>Shows the window natively on <paramref name="rect"/>, topmost, without activating it.</summary>
    public static void ShowAt(Window window, PixelRect rect)
    {
        if (OperatingSystem.IsWindows() && window.TryGetPlatformHandle()?.Handle is { } hwnd and not 0)
            SetWindowPos(hwnd, -1, rect.X, rect.Y, rect.Width, rect.Height, SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }

    /// <summary>Hides the window natively; Avalonia still counts it as open.</summary>
    public static void Hide(Window window)
    {
        if (OperatingSystem.IsWindows() && window.TryGetPlatformHandle()?.Handle is { } hwnd and not 0)
            ShowWindow(hwnd, 0);
    }

    /// <summary>Cloaks the window: it exists and is "shown", but nothing of it appears.</summary>
    public static void Cloak(Window window, bool cloaked)
    {
        if (OperatingSystem.IsWindows() && window.TryGetPlatformHandle()?.Handle is { } hwnd and not 0)
        {
            var value = cloaked ? 1 : 0;
            DwmSetWindowAttribute(hwnd, DWMWA_CLOAK, ref value, sizeof(int));
        }
    }

    private const uint SWP_NOACTIVATE = 0x10;
    private const uint SWP_SHOWWINDOW = 0x40;
    private const int DWMWA_CLOAK = 13;

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool ShowWindow(nint hwnd, int command);

    [LibraryImport("dwmapi.dll")]
    private static partial int DwmSetWindowAttribute(nint hwnd, int attribute, ref int value, int size);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool SetWindowPos(nint hwnd, nint after, int x, int y, int w, int h, uint flags);

    [LibraryImport("user32.dll")]
    private static partial nint GetWindowLongPtrW(nint hwnd, int index);

    [LibraryImport("user32.dll")]
    private static partial nint SetWindowLongPtrW(nint hwnd, int index, nint value);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool SetLayeredWindowAttributes(nint hwnd, uint key, byte alpha, uint flags);

    [LibraryImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static partial bool SetWindowDisplayAffinity(nint hwnd, uint affinity);
}

/// <summary>
/// A capture panel: the status (live) and a row of buttons, the first one
/// the main action. It sits below the area, or above it, or inside its
/// bottom edge when neither fits on the screen.
/// </summary>
internal sealed class HudPanelWindow : Window
{
    /// <summary>Gap between the area and the panel, past the outline (DIPs).</summary>
    private const double Gap = 11;

    private readonly long _id;
    private readonly PixelRect _area;
    private readonly Action<string> _run;
    private readonly TextBlock _status;
    private bool _dismissed;

    public HudPanelWindow(long id, PixelRect area, IReadOnlyList<string> buttons, Action<string> run)
    {
        _id = id;
        _area = area;
        _run = run;
        Title = "Jiffshot";
        WindowDecorations = WindowDecorations.None;
        Topmost = true;
        ShowActivated = false;
        ShowInTaskbar = false;
        CanResize = false;
        Width = 360;
        SizeToContent = SizeToContent.Height;
        RequestedThemeVariant = Avalonia.Styling.ThemeVariant.Dark;
        Background = new SolidColorBrush(Color.Parse("#232428"));
        _status = new TextBlock
        {
            FontSize = 13.5,
            FontWeight = FontWeight.SemiBold,
            TextWrapping = TextWrapping.Wrap,
            Foreground = new SolidColorBrush(Color.Parse("#E8EAED")),
        };
        var row = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, HorizontalAlignment = HorizontalAlignment.Right };
        for (var i = 0; i < buttons.Count; i++)
        {
            var index = i;
            var button = new Button
            {
                Content = buttons[i],
                MinWidth = 76,
                Height = 30,
                Padding = new Thickness(12, 0),
                CornerRadius = new CornerRadius(6),
                HorizontalContentAlignment = HorizontalAlignment.Center,
                VerticalContentAlignment = VerticalAlignment.Center,
                Foreground = Brushes.White,
                Background = new SolidColorBrush(Color.Parse(i == 0 ? "#2F5DA8" : "#3A3C42")),
            };
            button.Click += (_, _) => _run($"hud:{_id}:{index}");
            row.Children.Add(button);
        }
        Content = new Border
        {
            BorderBrush = new SolidColorBrush(Color.Parse("#3A3C42")),
            BorderThickness = new Thickness(1),
            Padding = new Thickness(12, 9, 12, 10),
            Child = new StackPanel { Spacing = 10, Children = { _status, row } },
        };
        Opened += (_, _) =>
        {
            HudNative.Prepare(this, clickThrough: false);
            Place();
        };
        SizeChanged += (_, _) => Place();
        // Closed by anything but the capture side (which never happens in
        // normal use): the same as Esc.
        Closed += (_, _) =>
        {
            if (!_dismissed)
                _run($"hud:{_id}");
        };
    }

    public void SetStatus(string status) => _status.Text = status;

    /// <summary>Closes the panel because the capture side is done with it.</summary>
    public void Dismiss()
    {
        _dismissed = true;
        Close();
    }

    private void Place()
    {
        var screen = Screens.ScreenFromPoint(_area.Center) ?? Screens.Primary;
        if (screen is null)
            return;
        var work = screen.WorkingArea;
        var scale = screen.Scaling;
        var (w, h) = ((int)Math.Ceiling(Bounds.Width * scale), (int)Math.Ceiling(Bounds.Height * scale));
        var gap = (int)Math.Round(Gap * scale);
        var x = Math.Clamp(_area.Right - w, work.X, Math.Max(work.X, work.Right - w));
        var below = _area.Bottom + gap;
        var above = _area.Y - gap - h;
        var y = below + h <= work.Bottom ? below : above >= work.Y ? above : _area.Bottom - h - gap;
        Position = new PixelPoint(x, y);
    }
}

/// <summary>
/// A capture outline: a see-through, click-through window just larger than
/// the area, drawing a thin red frame outside it. (One window rather than
/// four strips: Windows keeps top-level windows taller than a few pixels.)
/// </summary>
internal sealed class HudBorderWindow : Window
{
    /// <summary>The outline's thickness, in desktop pixels.</summary>
    public const int Weight = 3;

    private readonly PixelRect _outer;
    private readonly Border _frame;

    public HudBorderWindow(PixelRect area)
    {
        _outer = new PixelRect(area.X - Weight, area.Y - Weight, area.Width + 2 * Weight, area.Height + 2 * Weight);
        Title = "Jiffshot";
        WindowDecorations = WindowDecorations.None;
        Topmost = true;
        ShowActivated = false;
        ShowInTaskbar = false;
        CanResize = false;
        TransparencyLevelHint = [WindowTransparencyLevel.Transparent];
        Background = Brushes.Transparent;
        _frame = new Border { BorderBrush = new SolidColorBrush(Color.Parse("#E53935"), 220 / 255.0) };
        Content = _frame;
        Position = _outer.Position;
        Fit(1.0);
        Opened += (_, _) =>
        {
            HudNative.Prepare(this, clickThrough: true);
            Fit(RenderScaling);
            Position = _outer.Position;
        };
    }

    /// <summary>The window, outline included, in desktop pixels.</summary>
    public PixelRect Outer => _outer;

    private void Fit(double scale)
    {
        Width = _outer.Width / scale;
        Height = _outer.Height / scale;
        _frame.BorderThickness = new Thickness(Weight / scale);
    }
}
