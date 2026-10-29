using System;
using System.Text.Json;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Shapes;
using Avalonia.Input;
using Avalonia.Layout;
using Avalonia.Media;

namespace Jiffshot.Editor.Views;

/// <summary>
/// The toolbar under the area while recording: Screen Draw on its own, then
/// Stop, Pause, the pointer and webcam switches (each with a ▾ menu), system
/// sound and the microphone, the time, Restart, Minimize and Cancel.
/// Minimized, it keeps the time and Stop. The capture side sends its state
/// (<c>recorder.rs</c>, <c>toolbar_state</c>) and takes its actions
/// (<c>hud:ID:act:NAME</c>). It never takes the focus and is left out of
/// screen captures.
/// </summary>
internal sealed class RecordingToolbar : Window
{
    private static readonly IBrush Pill = new SolidColorBrush(Color.Parse("#1B1C1F"));
    private static readonly IBrush Grey = new SolidColorBrush(Color.Parse("#474A51"));
    private static readonly IBrush Blue = new SolidColorBrush(Color.Parse("#2F80ED"));
    private static readonly IBrush Green = new SolidColorBrush(Color.Parse("#2EA043"));
    private static readonly IBrush Live = new SolidColorBrush(Color.Parse("#FF5252"));
    private static readonly IBrush Held = new SolidColorBrush(Color.Parse("#FFC107"));

    private readonly long _id;
    private readonly PixelRect _area;
    private readonly Action<string> _run;
    private readonly ToolButton _draw, _pause, _cursor, _webcam, _system, _microphone;
    private readonly ToolButton _cursorMenu, _webcamMenu;
    private readonly TextBlock _time = new() { FontSize = 24, FontWeight = FontWeight.SemiBold, Foreground = Brushes.White, VerticalAlignment = VerticalAlignment.Center, FontFeatures = FontFeatureCollection.Parse("tnum") };
    private readonly TextBlock _miniTime = new() { FontSize = 16, FontWeight = FontWeight.SemiBold, Foreground = Brushes.White, VerticalAlignment = VerticalAlignment.Center };
    private readonly Ellipse _dot = new() { Width = 9, Height = 9, Fill = Live, VerticalAlignment = VerticalAlignment.Center };
    private readonly Ellipse _miniDot = new() { Width = 8, Height = 8, Fill = Live, VerticalAlignment = VerticalAlignment.Center };
    private readonly TextBlock _noteText = new() { FontSize = 12, Foreground = new SolidColorBrush(Color.Parse("#E8EAED")), TextWrapping = TextWrapping.Wrap, MaxWidth = 640 };
    private readonly Border _note;
    private readonly Control _full, _mini;
    private JsonElement _state;
    private bool _shownPaused;

    public RecordingToolbar(long id, PixelRect area, Action<string> run)
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
        SizeToContent = SizeToContent.WidthAndHeight;
        TransparencyLevelHint = [WindowTransparencyLevel.Transparent];
        Background = Brushes.Transparent;

        _note = new Border { Background = Pill, CornerRadius = new CornerRadius(8), Padding = new Thickness(10, 5), Margin = new Thickness(66, 6, 0, 0), HorizontalAlignment = HorizontalAlignment.Left, IsVisible = false, Child = _noteText };
        _draw = new ToolButton("\uE70F", 54, 54, Grey, () => Send("draw")) { Tip = "Screen Draw: draw on the recording (A arrow, S box, E ellipse, L line, P pen, 1 step, C color)" };
        var stop = new ToolButton(StopShape(18), 68, 54, Blue, () => Send("stop")) { Tip = "Stop and save (Shift+F10)" };
        _pause = new ToolButton(PauseShape(), 68, 54, Grey, () => Send("pause")) { Tip = "Pause (Shift+F9)" };
        _cursor = new ToolButton("\uE8B0", 42, 25, Green, () => Send("toggle:cursor"), 14) { Tip = "Record the pointer", Corners = new CornerRadius(7, 0, 0, 7) };
        _cursorMenu = new ToolButton("\uE70D", 24, 25, Green, () => CursorMenu(), 9) { Tip = "Pointer effects", Corners = new CornerRadius(0, 7, 7, 0) };
        _webcam = new ToolButton("\uE714", 42, 25, Grey, () => Send("toggle:webcam"), 14) { Tip = "Webcam", Corners = new CornerRadius(7, 0, 0, 7) };
        _webcamMenu = new ToolButton("\uE70D", 24, 25, Grey, () => WebcamMenu(), 9) { Tip = "Webcam corner and shape", Corners = new CornerRadius(0, 7, 7, 0) };
        _system = new ToolButton("\uE767", 66, 25, Green, () => Send("toggle:system"), 14) { Tip = "Record system sound" };
        _microphone = new ToolButton("\uE720", 66, 25, Green, () => Send("toggle:microphone"), 14) { Tip = "Record the microphone" };
        var restart = new ToolButton("\uE72C", 46, 54, Grey, () => Send("restart"), 18) { Tip = "Restart: throw this take away and record again" };
        var minimize = new ToolButton("\uE921", 46, 54, Grey, () => Minimize(true), 16) { Tip = "Make the toolbar small" };
        var cancel = new ToolButton("\uE711", 58, 54, Grey, () => Send("cancel"), 20) { Tip = "Cancel the recording (Esc)" };

        StackPanel Pair(params Control[] items) => new() { Orientation = Orientation.Horizontal, Spacing = 1, Children = { items[0], items[1] } };
        var switches = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 8,
            Children =
            {
                new StackPanel { Spacing = 4, Children = { Pair(_cursor, _cursorMenu), Pair(_webcam, _webcamMenu) } },
                new StackPanel { Spacing = 4, Children = { _system, _microphone } },
            },
        };
        var time = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, Margin = new Thickness(10, 0, 10, 0), Children = { _dot, _time } };
        var pill = new Border
        {
            Background = Pill,
            CornerRadius = new CornerRadius(14),
            Padding = new Thickness(8),
            Child = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8, Children = { stop, _pause, switches, time, restart, minimize, cancel } },
        };
        _full = new StackPanel
        {
            Children =
            {
                new StackPanel
                {
                    Orientation = Orientation.Horizontal,
                    Spacing = 12,
                    Children = { new Border { Background = Pill, CornerRadius = new CornerRadius(12), Padding = new Thickness(5), VerticalAlignment = VerticalAlignment.Center, Child = _draw }, pill },
                },
                _note,
            },
        };
        var expand = new ToolButton("\uE740", 36, 36, Grey, () => Minimize(false), 13) { Tip = "Show the whole toolbar" };
        var miniStop = new ToolButton(StopShape(13), 44, 36, Blue, () => Send("stop")) { Tip = "Stop and save (Shift+F10)" };
        _mini = new Border
        {
            Background = Pill,
            CornerRadius = new CornerRadius(12),
            Padding = new Thickness(6),
            IsVisible = false,
            Child = new StackPanel
            {
                Orientation = Orientation.Horizontal,
                Spacing = 8,
                Children = { miniStop, new StackPanel { Orientation = Orientation.Horizontal, Spacing = 6, Margin = new Thickness(4, 0, 4, 0), Children = { _miniDot, _miniTime } }, expand },
            },
        };
        Content = new Panel { Children = { _full, _mini } };
        Opened += (_, _) =>
        {
            HudNative.Prepare(this, clickThrough: false);
            Place();
        };
        SizeChanged += (_, _) => Place();
    }

    private void Send(string action) => _run($"hud:{_id}:act:{action}");

    /// <summary>Filled shapes for Stop, Pause and Resume (the icon font's are outlines).</summary>
    private static Control StopShape(double size) => new Rectangle { Width = size, Height = size, RadiusX = 3, RadiusY = 3, Fill = Brushes.White };

    private static Control PauseShape() => new StackPanel
    {
        Orientation = Orientation.Horizontal,
        Spacing = 6,
        HorizontalAlignment = HorizontalAlignment.Center,
        VerticalAlignment = VerticalAlignment.Center,
        Children =
        {
            new Rectangle { Width = 6, Height = 20, RadiusX = 2, RadiusY = 2, Fill = Brushes.White },
            new Rectangle { Width = 6, Height = 20, RadiusX = 2, RadiusY = 2, Fill = Brushes.White },
        },
    };

    private static Control ResumeShape() => new Path
    {
        Data = Geometry.Parse("M 0,0 L 17,10 L 0,20 Z"),
        Fill = Brushes.White,
        HorizontalAlignment = HorizontalAlignment.Center,
        VerticalAlignment = VerticalAlignment.Center,
        Margin = new Thickness(4, 0, 0, 0),
    };

    private void Minimize(bool small)
    {
        _full.IsVisible = !small;
        _mini.IsVisible = small;
    }

    /// <summary>Centered under the area, or above it, or inside its bottom edge.</summary>
    private void Place()
    {
        var screen = Screens.ScreenFromPoint(_area.Center) ?? Screens.Primary;
        if (screen is null)
            return;
        var work = screen.WorkingArea;
        var scale = screen.Scaling;
        var (w, h) = ((int)Math.Ceiling(Bounds.Width * scale), (int)Math.Ceiling(Bounds.Height * scale));
        var gap = (int)Math.Round(14 * scale);
        var x = Math.Clamp(_area.X + (_area.Width - w) / 2, work.X, Math.Max(work.X, work.Right - w));
        var below = _area.Bottom + gap;
        var above = _area.Y - gap - h;
        var y = below + h <= work.Bottom ? below : above >= work.Y ? above : _area.Bottom - h - gap;
        Position = new PixelPoint(x, y);
    }

    private bool On(string name) => _state.ValueKind == JsonValueKind.Object && _state.TryGetProperty(name, out var v) && v.ValueKind == JsonValueKind.True;

    /// <summary>The capture side's state (see <c>toolbar_state</c>).</summary>
    public void Update(JsonElement state)
    {
        _state = state.Clone();
        var ms = state.GetProperty("millis").GetInt64();
        var text = TimeSpan.FromMilliseconds(ms).ToString(@"hh\:mm\:ss");
        _time.Text = text;
        _miniTime.Text = text;
        var paused = On("paused");
        _dot.Fill = _miniDot.Fill = paused ? Held : Live;
        if (paused != _shownPaused)
        {
            _pause.Shape = paused ? ResumeShape() : PauseShape();
            _shownPaused = paused;
        }
        _pause.Tip = paused ? "Resume (Shift+F9)" : "Pause (Shift+F9)";
        _draw.IsVisible = On("draw_offered");
        _draw.Fill = On("draw") ? Blue : Grey;
        _cursor.Fill = _cursorMenu.Fill = On("cursor") ? Green : Grey;
        _webcam.Fill = _webcamMenu.Fill = On("webcam") ? Green : Grey;
        _system.Fill = On("system") ? Green : Grey;
        _system.Glyph = On("system") ? "\uE767" : "\uE74F";
        _microphone.Fill = On("microphone") ? Green : Grey;
        _microphone.Struck = !On("microphone");
        var note = state.TryGetProperty("note", out var n) && n.ValueKind == JsonValueKind.String ? n.GetString() : null;
        _noteText.Text = note;
        _note.IsVisible = !string.IsNullOrEmpty(note);
    }

    private MenuItem Check(string header, bool on, string action)
    {
        var item = new MenuItem { Header = header, ToggleType = MenuItemToggleType.CheckBox, IsChecked = on };
        item.Click += (_, _) => Send(action);
        return item;
    }

    private MenuItem Radio(string header, bool on, string action)
    {
        var item = new MenuItem { Header = header, ToggleType = MenuItemToggleType.Radio, IsChecked = on };
        item.Click += (_, _) => Send(action);
        return item;
    }

    private void CursorMenu()
    {
        var menu = new MenuFlyout { Placement = PlacementMode.Top };
        menu.Items.Add(Check("Record the pointer", On("cursor"), "toggle:cursor"));
        menu.Items.Add(Check("Highlight the pointer", On("highlight"), "toggle:highlight"));
        menu.Items.Add(Check("Show clicks", On("clicks"), "toggle:clicks"));
        menu.ShowAt(_cursorMenu);
    }

    private void WebcamMenu()
    {
        var corner = _state.TryGetProperty("webcam_corner", out var c) ? c.GetString() : "bottom-right";
        var circle = On("webcam_circle");
        var menu = new MenuFlyout { Placement = PlacementMode.Top };
        menu.Items.Add(Check("Show the webcam", On("webcam"), "toggle:webcam"));
        menu.Items.Add(new Separator());
        foreach (var (name, value) in new[] { ("Top left", "top-left"), ("Top right", "top-right"), ("Bottom left", "bottom-left"), ("Bottom right", "bottom-right") })
            menu.Items.Add(Radio(name, corner == value, "corner:" + value));
        menu.Items.Add(new Separator());
        menu.Items.Add(Radio("Circle", circle, "shape:circle"));
        menu.Items.Add(Radio("Rectangle", !circle, "shape:rectangle"));
        menu.ShowAt(_webcamMenu);
    }

    /// <summary>A rounded, flat, colored button with a glyph (Fluent's button template restyles hover and press).</summary>
    private sealed class ToolButton : Border
    {
        private readonly TextBlock _glyph;
        private readonly Panel _content;
        private readonly Line _strike = new() { Stroke = Brushes.White, StrokeThickness = 1.6, IsVisible = false, IsHitTestVisible = false };

        public ToolButton(string glyph, double width, double height, IBrush fill, Action click, double size = 17)
        {
            Width = width;
            Height = height;
            Background = fill;
            CornerRadius = new CornerRadius(10);
            Cursor = new Cursor(StandardCursorType.Hand);
            _glyph = new TextBlock { Text = glyph, FontFamily = new FontFamily("Segoe Fluent Icons, Segoe MDL2 Assets"), FontSize = size, Foreground = Brushes.White, HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center };
            var half = size * 0.55;
            _strike.StartPoint = new Point(width / 2 - half, height / 2 + half);
            _strike.EndPoint = new Point(width / 2 + half, height / 2 - half);
            _content = new Panel { Children = { _glyph, _strike } };
            Child = _content;
            PointerEntered += (_, _) => Opacity = 0.85;
            PointerExited += (_, _) => Opacity = 1;
            PointerPressed += (_, e) =>
            {
                if (e.GetCurrentPoint(this).Properties.IsLeftButtonPressed)
                {
                    Opacity = 0.7;
                    e.Handled = true;
                }
            };
            PointerReleased += (_, e) =>
            {
                Opacity = 0.85;
                if (e.InitialPressMouseButton == MouseButton.Left)
                {
                    e.Handled = true;
                    click();
                }
            };
        }

        /// <summary>A button showing a shape instead of a glyph.</summary>
        public ToolButton(Control shape, double width, double height, IBrush fill, Action click)
            : this("", width, height, fill, click)
        {
            Shape = shape;
        }

        public string Glyph
        {
            set => _glyph.Text = value;
        }

        /// <summary>Replaces the glyph with a shape.</summary>
        public Control Shape
        {
            set
            {
                _content.Children.Clear();
                _content.Children.Add(value);
            }
        }

        public IBrush Fill
        {
            set => Background = value;
        }

        public string Tip
        {
            set => ToolTip.SetTip(this, value);
        }

        public CornerRadius Corners
        {
            set => CornerRadius = value;
        }

        /// <summary>A line through the glyph (switched off).</summary>
        public bool Struck
        {
            set => _strike.IsVisible = value;
        }
    }
}
