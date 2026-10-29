using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.Platform;
using Avalonia.Rendering.Composition;
using Avalonia.Threading;

namespace Jiffshot.Editor.Views;

/// <summary>
/// The capture side's function table (<c>ui_api.rs</c>): the overlay's and
/// Screen Draw's input in, what to draw out. Published as the <c>UiApi</c>
/// property; the host shares this process, so pointers go straight through.
/// </summary>
internal static unsafe class UiApi
{
    public const int Press = 0, Move = 1, Release = 2, Wheel = 3, Key = 4, Cancel = 5, Refresh = 6;
    public const int ModShift = 1, ModCtrl = 2;
    public const int FlagDone = 1, FlagMagnifier = 2, FlagHelp = 4, FlagGuides = 8, FlagColor = 16, FlagDragging = 32;

    [StructLayout(LayoutKind.Sequential)]
    public struct OverlayState
    {
        public int Flags;
        public int CursorX, CursorY;
        public int LitX, LitY, LitW, LitH;
        public int LitKind;
        public int MonX, MonY, MonW, MonH;
        public float Scale;
        public int ColorR, ColorG, ColorB;
        public int PathCount, RegionCount;

        public readonly PixelRect Lit => new(LitX, LitY, LitW, LitH);
        public readonly PixelRect Monitor => new(MonX, MonY, MonW, MonH);
        public readonly bool Has(int flag) => (Flags & flag) != 0;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Table
    {
        public delegate* unmanaged<ulong, int, double, double, int, int, OverlayState*, void> OverlayInput;
        public delegate* unmanaged<ulong, int, char*, int, int> OverlayText;
        public delegate* unmanaged<ulong, int, int*, int, int> OverlayPoints;
        public delegate* unmanaged<ulong, int, int, int, int, byte*, int, int> OverlayCopy;
        public delegate* unmanaged<ulong, void> OverlayShown;
        public delegate* unmanaged<ulong, int, double, double, int, int> DrawInput;
        public delegate* unmanaged<ulong, int> DrawTick;
        public delegate* unmanaged<ulong, byte*, int, int, int, int> DrawCopy;
        public delegate* unmanaged<int, char*, int, int> OverlayDemo;
        public delegate* unmanaged<ulong, void> OverlayEndDemo;
        public delegate* unmanaged<int, int, int, int, char*, int, int> DrawDemo;
        public delegate* unmanaged<void> DrawEnd;
    }

    private static Table* _table;

    public static bool Ready => _table != null;

    public static void Attach(long address) => _table = (Table*)address;

    public static OverlayState OverlayInput(ulong session, int kind, double x, double y, int value, int mods)
    {
        OverlayState state = default;
        if (_table != null)
            _table->OverlayInput(session, kind, x, y, value, mods, &state);
        return state;
    }

    public static string OverlayText(ulong session, int which)
    {
        if (_table == null)
            return "";
        var buffer = stackalloc char[1024];
        var n = _table->OverlayText(session, which, buffer, 1024);
        return new string(buffer, 0, n);
    }

    public static int[] OverlayPoints(ulong session, int which, int count)
    {
        if (_table == null || count <= 0)
            return [];
        var values = new int[count];
        fixed (int* p = values)
            count = _table->OverlayPoints(session, which, p, values.Length);
        return count == values.Length ? values : values[..count];
    }

    public static bool OverlayCopy(ulong session, PixelRect r, IntPtr dst, int stride) =>
        _table != null && _table->OverlayCopy(session, r.X, r.Y, r.Width, r.Height, (byte*)dst, stride) != 0;

    public static void OverlayShown(ulong session)
    {
        if (_table != null)
            _table->OverlayShown(session);
    }

    public static bool DrawInput(ulong session, int kind, double x, double y, int value) =>
        _table != null && _table->DrawInput(session, kind, x, y, value) != 0;

    public static bool DrawTick(ulong session) => _table != null && _table->DrawTick(session) != 0;

    public static bool DrawCopy(ulong session, IntPtr dst, int stride, int w, int h) =>
        _table != null && _table->DrawCopy(session, (byte*)dst, stride, w, h) != 0;

    public static string OverlayDemo(int mode)
    {
        if (_table == null)
            return "";
        var buffer = new char[4096];
        fixed (char* p = buffer)
            return new string(p, 0, _table->OverlayDemo(mode, p, buffer.Length));
    }

    public static void OverlayEndDemo(ulong session)
    {
        if (_table != null)
            _table->OverlayEndDemo(session);
    }

    public static string DrawDemo(PixelRect area)
    {
        if (_table == null)
            return "";
        var buffer = stackalloc char[512];
        return new string(buffer, 0, _table->DrawDemo(area.X, area.Y, area.Width, area.Height, buffer, 512));
    }

    public static void DrawEnd()
    {
        if (_table != null)
            _table->DrawEnd();
    }

    /// <summary>The Windows virtual key for an Avalonia key (0 for keys the capture side ignores).</summary>
    public static int VirtualKey(Avalonia.Input.Key key) => key switch
    {
        Avalonia.Input.Key.Escape => 0x1B,
        Avalonia.Input.Key.Left => 0x25,
        Avalonia.Input.Key.Up => 0x26,
        Avalonia.Input.Key.Right => 0x27,
        Avalonia.Input.Key.Down => 0x28,
        Avalonia.Input.Key.Enter => 0x0D,
        Avalonia.Input.Key.Space => 0x20,
        Avalonia.Input.Key.Back => 0x08,
        Avalonia.Input.Key.F1 => 0x70,
        >= Avalonia.Input.Key.A and <= Avalonia.Input.Key.Z => 'A' + (key - Avalonia.Input.Key.A),
        >= Avalonia.Input.Key.D0 and <= Avalonia.Input.Key.D9 => '0' + (key - Avalonia.Input.Key.D0),
        >= Avalonia.Input.Key.NumPad0 and <= Avalonia.Input.Key.NumPad9 => '0' + (key - Avalonia.Input.Key.NumPad0),
        _ => 0,
    };

    public static int Modifiers(KeyModifiers m) =>
        (m.HasFlag(KeyModifiers.Shift) ? ModShift : 0) | (m.HasFlag(KeyModifiers.Control) ? ModCtrl : 0);
}

/// <summary>A monitor of the overlay's desktop, in desktop pixels.</summary>
internal readonly record struct OverlayMonitor(PixelRect Rect, double Scale, bool Primary);

/// <summary>
/// One showing of the selection overlay: the capture side's session, the
/// desktop's monitors, and the latest state with its texts and points.
/// </summary>
internal sealed class OverlaySession
{
    public ulong Id { get; }
    public IReadOnlyList<OverlayMonitor> Monitors { get; }
    public UiApi.OverlayState State { get; private set; }
    public string Title { get; private set; } = "";
    public string Size { get; private set; } = "";
    public string Hint { get; private set; } = "";
    public string[] Help { get; }
    public int[] Path { get; private set; } = [];
    public int[] Regions { get; private set; } = [];
    public int[] Guides { get; }
    public bool Done => State.Has(UiApi.FlagDone);

    /// <summary>After each input, so every monitor's canvas redraws.</summary>
    public event Action? Changed;

    public OverlaySession(string json)
    {
        using var doc = JsonDocument.Parse(json);
        var root = doc.RootElement;
        Id = root.GetProperty("id").GetUInt64();
        Monitors = root.GetProperty("monitors").EnumerateArray().Select(m =>
        {
            var r = m.GetProperty("rect");
            return new OverlayMonitor(new PixelRect(r[0].GetInt32(), r[1].GetInt32(), r[2].GetInt32(), r[3].GetInt32()),
                m.GetProperty("scale").GetDouble(), m.GetProperty("primary").GetBoolean());
        }).ToList();
        Help = UiApi.OverlayText(Id, 3).Split('\n');
        Guides = UiApi.OverlayPoints(Id, 2, 16);
        var cursor = root.GetProperty("cursor");
        Input(UiApi.Refresh, cursor[0].GetInt32(), cursor[1].GetInt32(), 0, 0);
    }

    /// <summary>Sends input (desktop pixels) and takes the new state.</summary>
    public void Input(int kind, double x, double y, int value, int mods)
    {
        var before = State;
        State = UiApi.OverlayInput(Id, kind, x, y, value, mods);
        if (Done)
            return;
        Title = UiApi.OverlayText(Id, 0);
        Size = UiApi.OverlayText(Id, 1);
        if (Hint.Length == 0)
            Hint = UiApi.OverlayText(Id, 2);
        if (State.PathCount != before.PathCount)
            Path = UiApi.OverlayPoints(Id, 0, State.PathCount * 2);
        if (State.RegionCount != before.RegionCount)
            Regions = UiApi.OverlayPoints(Id, 1, State.RegionCount * 4);
        Changed?.Invoke();
    }
}

/// <summary>
/// Draws the overlay over one monitor: its slice of the frozen screenshot,
/// dimmed except for the lit area, the crosshair and, on the monitor under
/// the pointer, the label, magnifier, hints and help. Works in desktop
/// pixels and scales to the control's DIPs, so the screenshot shows 1:1.
/// </summary>
internal sealed class OverlayCanvas : Control
{
    private static readonly IBrush Accent = new SolidColorBrush(Color.Parse("#2F8CFF"));
    private static readonly IBrush Crosshair = new SolidColorBrush(Color.Parse("#FFC107"));
    private static readonly IBrush LabelBack = new SolidColorBrush(Color.Parse("#1F2024"));
    private static readonly IBrush LabelText = new SolidColorBrush(Color.Parse("#F1F3F4"));
    private static readonly IBrush LabelMuted = new SolidColorBrush(Color.Parse("#A8ADB4"));
    private static readonly IBrush Dim = new SolidColorBrush(Color.FromArgb(140, 2, 6, 16));
    private static readonly Typeface Regular = new("Segoe UI");
    private static readonly Typeface Bold = new("Segoe UI", FontStyle.Normal, FontWeight.SemiBold);
    private const int Samples = 15;

    private OverlaySession? _session;
    private OverlayMonitor _monitor;
    private WriteableBitmap? _bitmap;

    public OverlaySession? Session => _session;
    public OverlayMonitor Monitor => _monitor;

    public OverlayCanvas()
    {
        Focusable = true;
        Cursor = new Cursor(StandardCursorType.Cross);
    }

    /// <summary>
    /// Takes a session: copies this monitor's part of the screenshot. With
    /// <paramref name="sized"/> the control sizes itself to the monitor (when
    /// it is not in an overlay window, which has the monitor's size).
    /// </summary>
    public void Load(OverlaySession session, OverlayMonitor monitor, bool sized = false)
    {
        if (_session is { } old)
            old.Changed -= InvalidateVisual;
        _session = session;
        _monitor = monitor;
        Reserve(monitor);
        using (var fb = _bitmap!.Lock())
            UiApi.OverlayCopy(session.Id, monitor.Rect, fb.Address, fb.RowBytes);
        session.Changed += InvalidateVisual;
        if (sized)
        {
            Width = monitor.Rect.Width / monitor.Scale;
            Height = monitor.Rect.Height / monitor.Scale;
        }
        InvalidateVisual();
    }

    /// <summary>Allocates the screenshot for a monitor of this size, ahead of a capture.</summary>
    public void Reserve(OverlayMonitor monitor)
    {
        var size = new PixelSize(monitor.Rect.Width, monitor.Rect.Height);
        if (_bitmap is null || _bitmap.PixelSize != size)
        {
            _bitmap?.Dispose();
            _bitmap = new WriteableBitmap(size, new Vector(96, 96), PixelFormat.Bgra8888, AlphaFormat.Opaque);
        }
    }

    /// <summary>Lets go of the session (the screenshot stays allocated for the next one).</summary>
    public void Unload()
    {
        if (_session is { } old)
            old.Changed -= InvalidateVisual;
        _session = null;
    }

    // ---- input -----------------------------------------------------------------

    /// <summary>Desktop pixels per DIP: the window's, or the monitor's when not in one.</summary>
    private double Scale => TopLevel.GetTopLevel(this)?.RenderScaling ?? _monitor.Scale;

    private Point Desktop(Point p) => new(_monitor.Rect.X + p.X * Scale, _monitor.Rect.Y + p.Y * Scale);

    /// <summary>Input at a point of this control (DIPs); public for the UI self-test.</summary>
    public void Send(int kind, Point at, int value = 0, int mods = 0)
    {
        if (_session is not { } s || s.Done)
            return;
        var p = Desktop(at);
        s.Input(kind, p.X, p.Y, value, mods);
        if (s.Done)
            OverlayHost.Finish(s);
    }

    protected override void OnPointerPressed(PointerPressedEventArgs e)
    {
        var point = e.GetCurrentPoint(this);
        if (point.Properties.IsRightButtonPressed)
            Send(UiApi.Cancel, point.Position);
        else if (point.Properties.IsLeftButtonPressed)
        {
            e.Pointer.Capture(this);
            Send(UiApi.Press, point.Position, 0, UiApi.Modifiers(e.KeyModifiers));
        }
        e.Handled = true;
    }

    protected override void OnPointerMoved(PointerEventArgs e)
    {
        Send(UiApi.Move, e.GetPosition(this), 0, UiApi.Modifiers(e.KeyModifiers));
    }

    protected override void OnPointerReleased(PointerReleasedEventArgs e)
    {
        if (e.InitialPressMouseButton == MouseButton.Left)
        {
            e.Pointer.Capture(null);
            Send(UiApi.Release, e.GetPosition(this), 0, UiApi.Modifiers(e.KeyModifiers));
        }
        e.Handled = true;
    }

    protected override void OnPointerWheelChanged(PointerWheelEventArgs e)
    {
        Send(UiApi.Wheel, e.GetPosition(this), e.Delta.Y > 0 ? 120 : -120);
        e.Handled = true;
    }

    /// <summary>Keys go to the canvas of the window that has the focus.</summary>
    public void Key(KeyEventArgs e, bool down)
    {
        if (_session is not { } s || s.Done)
            return;
        var mods = UiApi.Modifiers(e.KeyModifiers);
        var vk = down ? UiApi.VirtualKey(e.Key) : 0;
        var c = s.State;
        s.Input(vk != 0 ? UiApi.Key : UiApi.Refresh, c.CursorX, c.CursorY, vk, mods);
        if (s.Done)
            OverlayHost.Finish(s);
        e.Handled = true;
    }

    // ---- drawing (desktop pixels) ------------------------------------------------

    private static Rect R(int x, int y, int w, int h) => new(x, y, Math.Max(0, w), Math.Max(0, h));

    private static void Outline(DrawingContext dc, PixelRect r, int t, IBrush brush)
    {
        dc.FillRectangle(brush, R(r.X - t, r.Y - t, r.Width + 2 * t, t));
        dc.FillRectangle(brush, R(r.X - t, r.Bottom, r.Width + 2 * t, t));
        dc.FillRectangle(brush, R(r.X - t, r.Y, t, r.Height));
        dc.FillRectangle(brush, R(r.Right, r.Y, t, r.Height));
    }

    private static FormattedText Text(string text, Typeface face, double size, IBrush brush) =>
        new(text, CultureInfo.CurrentCulture, FlowDirection.LeftToRight, face, size, brush);

    public override void Render(DrawingContext context)
    {
        if (_session is not { } s || _bitmap is null)
            return;
        var m = _monitor.Rect;
        var transform = Matrix.CreateTranslation(-m.X, -m.Y) * Matrix.CreateScale(1 / Scale, 1 / Scale);
        using var _ = context.PushTransform(transform);
        using var __ = context.PushRenderOptions(new RenderOptions { BitmapInterpolationMode = BitmapInterpolationMode.None, EdgeMode = EdgeMode.Aliased });
        var st = s.State;
        context.DrawImage(_bitmap, new Rect(0, 0, m.Width, m.Height), R(m.X, m.Y, m.Width, m.Height));

        // Dimmed, except the lit area.
        var lit = st.Lit.Intersect(m);
        if (lit.Width <= 0 || lit.Height <= 0)
            context.FillRectangle(Dim, R(m.X, m.Y, m.Width, m.Height));
        else
        {
            context.FillRectangle(Dim, R(m.X, m.Y, m.Width, lit.Y - m.Y));
            context.FillRectangle(Dim, R(m.X, lit.Bottom, m.Width, m.Bottom - lit.Bottom));
            context.FillRectangle(Dim, R(m.X, lit.Y, lit.X - m.X, lit.Height));
            context.FillRectangle(Dim, R(lit.Right, lit.Y, m.Right - lit.Right, lit.Height));
        }

        var scale = (double)st.Scale;
        int S(double v) => (int)Math.Round(v * scale);
        var (cx, cy) = (st.CursorX, st.CursorY);

        // The crosshair through the whole desktop.
        context.FillRectangle(Crosshair, R(m.X, cy, m.Width, 1));
        context.FillRectangle(Crosshair, R(cx, m.Y, 1, m.Height));

        var thick = Math.Max(2, S(2));
        for (var i = 0; i + 3 < s.Regions.Length; i += 4)
            Outline(context, new PixelRect(s.Regions[i], s.Regions[i + 1], s.Regions[i + 2], s.Regions[i + 3]), thick, Accent);
        if (s.Path.Length >= 4)
        {
            var geometry = new StreamGeometry();
            using (var g = geometry.Open())
            {
                g.BeginFigure(new Point(s.Path[0], s.Path[1]), false);
                for (var i = 2; i + 1 < s.Path.Length; i += 2)
                    g.LineTo(new Point(s.Path[i], s.Path[i + 1]));
                g.EndFigure(false);
            }
            context.DrawGeometry(null, new Pen(Accent, 3), geometry);
        }
        var font = 13 * scale;
        if (st.Has(UiApi.FlagGuides))
        {
            for (var i = 0; i + 1 < s.Guides.Length; i += 2)
            {
                var (gw, gh) = (s.Guides[i], s.Guides[i + 1]);
                var g = new PixelRect(cx - gw / 2, cy - gh / 2, gw, gh);
                Outline(context, g, 1, LabelMuted);
                context.DrawText(Text($"{gw} × {gh}", Regular, font, LabelMuted), new Point(g.X + 6, g.Y + 4));
            }
        }

        // The lit area's frame.
        var area = st.Lit;
        Outline(context, area, thick, Accent);

        // Labels, help, magnifier and hints only on the monitor under the pointer.
        if (st.Monitor != m)
            return;
        var mon = st.Monitor;
        var pad = S(6);
        var sizeText = Text(s.Size, Bold, font, LabelText);
        var lineH = (int)Math.Ceiling(sizeText.Height);
        FormattedText? titleText = s.Title.Length > 0 ? Text(s.Title, Regular, font, LabelMuted) : null;
        var titleW = titleText is null ? 0 : (int)Math.Ceiling(titleText.Width) + S(10);
        PixelRect label;
        {
            var w = titleW + (int)Math.Ceiling(sizeText.Width) + 2 * pad;
            var h = lineH + 2 * pad;
            var x = Math.Max(area.X, mon.X);
            var y = area.Y - h - S(4);
            if (y < mon.Y)
            {
                y = area.Y + S(4);
                x += S(4);
            }
            x = Math.Min(x, mon.Right - w);
            label = new PixelRect(x, y, w, h);
        }
        context.FillRectangle(LabelBack, R(label.X, label.Y, label.Width, label.Height));
        if (titleText is not null)
            context.DrawText(titleText, new Point(label.X + pad, label.Y + pad));
        context.DrawText(sizeText, new Point(label.X + pad + titleW, label.Y + pad));

        if (st.Has(UiApi.FlagHelp))
        {
            var lines = s.Help.Select(l => Text(l, Regular, font, LabelText)).ToList();
            var w = (int)Math.Ceiling(lines.Max(l => l.Width)) + 2 * pad;
            var h = lines.Count * (lineH + S(4)) + 2 * pad;
            var r = new PixelRect(mon.X + (mon.Width - w) / 2, mon.Y + (mon.Height - h) / 2, w, h);
            context.FillRectangle(LabelBack, R(r.X, r.Y, r.Width, r.Height));
            for (var i = 0; i < lines.Count; i++)
                context.DrawText(lines[i], new Point(r.X + pad, r.Y + pad + i * (lineH + S(4))));
        }

        var total = default(PixelRect);
        if (st.Has(UiApi.FlagMagnifier))
        {
            var zoom = Math.Max(6, S(8));
            var box = Samples * zoom;
            var panelH = 2 * lineH + 3 * pad;
            var gap = S(22);
            var mx = cx + gap;
            var my = cy + gap;
            if (mx + box > mon.Right)
                mx = cx - gap - box;
            if (my + box + panelH > mon.Bottom)
                my = cy - gap - box - panelH;
            total = new PixelRect(mx, my, box, box + panelH);
            var half = Samples / 2;
            // The pixels around the pointer, from this monitor's screenshot.
            context.FillRectangle(Brushes.Black, R(mx, my, box, box));
            var src = new PixelRect(cx - half - m.X, cy - half - m.Y, Samples, Samples).Intersect(new PixelRect(0, 0, m.Width, m.Height));
            if (src.Width > 0 && src.Height > 0)
            {
                var dx = mx + (src.X - (cx - half - m.X)) * zoom;
                var dy = my + (src.Y - (cy - half - m.Y)) * zoom;
                context.DrawImage(_bitmap, new Rect(src.X, src.Y, src.Width, src.Height), R(dx, dy, src.Width * zoom, src.Height * zoom));
            }
            var centre = new PixelRect(mx + half * zoom, my + half * zoom, zoom, zoom);
            Outline(context, centre, 1, Brushes.Black);
            Outline(context, new PixelRect(centre.X + 1, centre.Y + 1, centre.Width - 2, centre.Height - 2), 1, Brushes.White);
            Outline(context, new PixelRect(mx, my, box, box), 1, Accent);
            var panel = new PixelRect(mx - 1, my + box + 1, box + 2, panelH);
            context.FillRectangle(LabelBack, R(panel.X, panel.Y, panel.Width, panel.Height));
            context.DrawText(Text($"{cx}, {cy}", Regular, font, LabelText), new Point(panel.X + pad, panel.Y + pad));
            if (st.Has(UiApi.FlagColor))
            {
                var swatch = new PixelRect(panel.X + pad, panel.Y + 2 * pad + lineH + 2, lineH - 4, lineH - 4);
                context.FillRectangle(new SolidColorBrush(Color.FromRgb((byte)st.ColorR, (byte)st.ColorG, (byte)st.ColorB)), R(swatch.X, swatch.Y, swatch.Width, swatch.Height));
                context.DrawText(Text($"#{st.ColorR:X2}{st.ColorG:X2}{st.ColorB:X2}", Regular, font, LabelText), new Point(swatch.Right + pad, panel.Y + 2 * pad + lineH));
            }
        }

        // Key hints at the top of the monitor, at the bottom when they would
        // cover the pointer, the label or the magnifier.
        var hint = Text(s.Hint, Regular, font, LabelMuted);
        {
            var w = (int)Math.Ceiling(hint.Width) + 2 * pad;
            var h = lineH + 2 * pad;
            var top = new PixelRect(mon.X + (mon.Width - w) / 2, mon.Y + S(12), w, h);
            var grow = S(40);
            var busy = new PixelRect(top.X - grow, top.Y - grow, top.Width + 2 * grow, top.Height + 2 * grow);
            var r = busy.Contains(new PixelPoint(cx, cy)) || Overlaps(busy, label) || (total.Width > 0 && Overlaps(busy, total))
                ? new PixelRect(top.X, mon.Bottom - h - S(12), w, h)
                : top;
            context.FillRectangle(LabelBack, R(r.X, r.Y, r.Width, r.Height));
            context.DrawText(hint, new Point(r.X + pad, r.Y + pad));
        }
    }

    private static bool Overlaps(PixelRect a, PixelRect b) =>
        a.X < b.Right && b.X < a.Right && a.Y < b.Bottom && b.Y < a.Bottom;
}

/// <summary>A borderless, topmost window over one monitor, holding its overlay canvas.</summary>
internal sealed class OverlayWindow : Window
{
    public OverlayCanvas Canvas { get; } = new();

    public OverlayWindow()
    {
        Title = "Jiffshot selection";
        WindowDecorations = WindowDecorations.None;
        Topmost = true;
        ShowActivated = false;
        ShowInTaskbar = false;
        CanResize = false;
        Background = Brushes.Black;
        Content = Canvas;
        KeyDown += (_, e) => Canvas.Key(e, true);
        KeyUp += (_, e) => Canvas.Key(e, false);
        // Windows moves and resizes a window whose screen scale changed; put it back.
        // (After Avalonia has applied the size Windows suggests.)
        ScalingChanged += (_, _) => Dispatcher.UIThread.Post(() =>
        {
            if (_rect.Width > 0)
                HudNative.Place(this, _rect);
        });
    }

    private PixelRect _rect;

    /// <summary>
    /// Covers the monitor exactly, in its pixels. The window never gets a
    /// size in DIPs: Avalonia would re-apply it at a stale scale after a
    /// move between screens of different scales.
    /// </summary>
    public void Fit(OverlayMonitor monitor)
    {
        _rect = monitor.Rect;
        Position = monitor.Rect.Position;
        HudNative.Place(this, monitor.Rect);
    }
}

/// <summary>
/// Shows the overlay the capture side announces (the <c>Overlay</c>
/// property): a window per monitor. The windows are made ahead of time
/// (<see cref="Prepare"/>) and only hidden natively between captures, so
/// putting the overlay up is a native show once the new screenshot has
/// been drawn, not a full Avalonia show. Taken down as soon as the
/// selection is made, and again when the capture side says so.
/// </summary>
internal static class OverlayHost
{
    private static readonly Dictionary<PixelRect, OverlayWindow> Windows = new();
    private static OverlaySession? _session;

    /// <summary>Makes (or keeps) a hidden window per screen, so the first capture is quick too.</summary>
    public static void Prepare(IEnumerable<OverlayMonitor> monitors)
    {
        foreach (var monitor in monitors)
            WindowFor(monitor);
    }

    private static OverlayWindow WindowFor(OverlayMonitor monitor)
    {
        if (Windows.TryGetValue(monitor.Rect, out var window))
            return window;
        window = new OverlayWindow();
        Windows[monitor.Rect] = window;
        window.Canvas.Reserve(monitor);
        window.Fit(monitor);
        // Shown once, cloaked so nothing flashes, then hidden natively:
        // Avalonia keeps it as a live window from then on.
        HudNative.Cloak(window, true);
        window.Show();
        // Shown on its own screen, it takes that screen's scale now rather
        // than on the first capture.
        HudNative.ShowAt(window, monitor.Rect);
        Dispatcher.UIThread.Post(() =>
        {
            window.Fit(monitor);
            HudNative.Hide(window);
            HudNative.Cloak(window, false);
        }, DispatcherPriority.Background);
        return window;
    }

    /// <summary>The announcement JSON, or empty to take the overlay down.</summary>
    public static async void Apply(string json)
    {
        if (string.IsNullOrEmpty(json))
        {
            End();
            return;
        }
        if (!UiApi.Ready)
            return;
        End();
        var session = new OverlaySession(json);
        if (session.Done)
            return;
        _session = session;
        var cursor = new PixelPoint(session.State.CursorX, session.State.CursorY);
        var shown = new List<(OverlayWindow Window, OverlayMonitor Monitor)>();
        foreach (var monitor in session.Monitors)
        {
            var window = WindowFor(monitor);
            window.Canvas.Load(session, monitor);
            shown.Add((window, monitor));
        }
        // Windows of monitors that are gone.
        foreach (var gone in Windows.Keys.Where(r => session.Monitors.All(m => m.Rect != r)).ToList())
        {
            Windows[gone].Close();
            Windows.Remove(gone);
        }
        // Show once the new screenshot is drawn, so the last one never flashes.
        if (ElementComposition.GetElementVisual(shown[0].Window.Canvas)?.Compositor is { } compositor)
            await compositor.RequestCompositionBatchCommitAsync().Rendered;
        if (!ReferenceEquals(_session, session))
            return;
        OverlayWindow? focus = null;
        foreach (var (window, monitor) in shown)
        {
            HudNative.ShowAt(window, monitor.Rect);
            if (monitor.Rect.Contains(cursor))
                focus = window;
        }
        focus?.Activate();
        focus?.Canvas.Focus();
        UiApi.OverlayShown(session.Id);
    }

    /// <summary>A canvas saw the selection made: hide at once.</summary>
    public static void Finish(OverlaySession session)
    {
        if (ReferenceEquals(session, _session))
            End();
    }

    private static void End()
    {
        if (_session is null)
            return;
        _session = null;
        foreach (var window in Windows.Values)
        {
            HudNative.Hide(window);
            window.Canvas.Unload();
        }
    }
}

/// <summary>
/// Screen Draw: a see-through window over the area being recorded, showing
/// the marks the capture side renders and sending it the pointer and keys.
/// It is left out of captures; the recorder draws the marks into the video.
/// </summary>
internal sealed class ScreenDrawWindow : Window
{
    private static ScreenDrawWindow? _current;
    private readonly ulong _id;
    private readonly PixelRect _area;
    private readonly WriteableBitmap _marks;
    private readonly DispatcherTimer _fade = new() { Interval = TimeSpan.FromMilliseconds(40) };
    private double _scale = 1;

    private ScreenDrawWindow(ulong id, PixelRect area)
    {
        _id = id;
        _area = area;
        Title = "Jiffshot Screen Draw";
        WindowDecorations = WindowDecorations.None;
        Topmost = true;
        ShowInTaskbar = false;
        CanResize = false;
        TransparencyLevelHint = [WindowTransparencyLevel.Transparent];
        // Barely there, so the pointer stays on this window.
        Background = new SolidColorBrush(Color.FromArgb(1, 0, 0, 0));
        Cursor = new Cursor(StandardCursorType.Cross);
        _marks = new WriteableBitmap(new PixelSize(Math.Max(1, area.Width), Math.Max(1, area.Height)), new Vector(96, 96), PixelFormat.Bgra8888, AlphaFormat.Premul);
        Content = new Image { Source = _marks, Stretch = Stretch.Fill };
        Position = area.Position;
        Fit(1);
        Opened += (_, _) =>
        {
            HudNative.ExcludeFromCapture(this);
            Fit(RenderScaling);
            Position = _area.Position;
            Refresh();
            Activate();
            _fade.Start();
        };
        _fade.Tick += (_, _) =>
        {
            if (UiApi.DrawTick(_id))
                Refresh();
        };
        Closed += (_, _) => _fade.Stop();
        PointerPressed += (_, e) =>
        {
            var p = e.GetCurrentPoint(this);
            if (p.Properties.IsRightButtonPressed)
                Send(UiApi.Cancel, p.Position);
            else
            {
                e.Pointer.Capture(this);
                Send(UiApi.Press, p.Position);
            }
        };
        PointerMoved += (_, e) => Send(UiApi.Move, e.GetPosition(this));
        PointerReleased += (_, e) =>
        {
            e.Pointer.Capture(null);
            if (e.InitialPressMouseButton == MouseButton.Left)
                Send(UiApi.Release, e.GetPosition(this));
        };
        KeyDown += (_, e) =>
        {
            if (UiApi.VirtualKey(e.Key) is var vk and > 0)
                UiApi.DrawInput(_id, UiApi.Key, 0, 0, vk);
        };
    }

    private void Fit(double scale)
    {
        _scale = scale;
        Width = _area.Width / scale;
        Height = _area.Height / scale;
    }

    private void Send(int kind, Point p)
    {
        if (UiApi.DrawInput(_id, kind, p.X * _scale, p.Y * _scale, 0))
            Refresh();
    }

    private void Refresh()
    {
        using (var fb = _marks.Lock())
            UiApi.DrawCopy(_id, fb.Address, fb.RowBytes, _marks.PixelSize.Width, _marks.PixelSize.Height);
        (Content as Image)?.InvalidateVisual();
    }

    /// <summary>The window showing, if any (for the UI self-test).</summary>
    public static ScreenDrawWindow? Current => _current;

    /// <summary>Draws as the pointer would, in area pixels (for the UI self-test).</summary>
    public void Draw(int kind, double x, double y, int value = 0)
    {
        if (UiApi.DrawInput(_id, kind, x, y, value) || kind == UiApi.Key)
            Refresh();
    }

    /// <summary><c>{"id","area":[x,y,w,h]}</c> shows Screen Draw; empty ends it.</summary>
    public static void Apply(string json)
    {
        _current?.Close();
        _current = null;
        if (string.IsNullOrEmpty(json) || !UiApi.Ready)
            return;
        using var doc = JsonDocument.Parse(json);
        var a = doc.RootElement.GetProperty("area");
        _current = new ScreenDrawWindow(doc.RootElement.GetProperty("id").GetUInt64(),
            new PixelRect(a[0].GetInt32(), a[1].GetInt32(), a[2].GetInt32(), a[3].GetInt32()));
        _current.Show();
    }
}
