using System;
using System.Runtime.InteropServices;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.Platform;

namespace Jiffshot.Editor.Controls;

/// <summary>
/// Displays the Rust-owned document raster and forwards pointer input to it.
/// Rust renders into its own premultiplied RGBA buffer; this control copies
/// only the damaged rectangle into a <see cref="WriteableBitmap"/> after each
/// call, draws it with the current zoom and pan, and draws the selection
/// handles and marquee on top in screen space.
/// </summary>
public sealed unsafe class PixelCanvas : Control
{
    private const int KindPressed = 0;
    private const int KindMoved = 1;
    private const int KindReleased = 2;

    public const int FlagChanged = 1;
    public const int FlagFull = 2;
    public const int FlagFocusText = 4;

    private const int MaxHandles = 16;
    private const int MaxOthers = 64;
    private const int HandleRotate = 12;
    private const double HandleSize = 9;

    [StructLayout(LayoutKind.Sequential)]
    private struct CanvasFrame
    {
        public byte* Pixels;
        public int Width;
        public int Height;
        public int Stride;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct CanvasDamage
    {
        public int X;
        public int Y;
        public int Width;
        public int Height;
        public int Flags;
        public int Cursor;
        public long RenderMicros;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Overlay
    {
        public int Flags;
        public int HandleCount;
        public float Rotation;
        public fixed float Selection[4];
        public fixed float Marquee[4];
        public fixed float Handles[MaxHandles * 2];
        public fixed int HandleIds[MaxHandles];
        public int OtherCount;
        public fixed float Others[MaxOthers * 4];
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct CanvasApi
    {
        public delegate* unmanaged<int, double, double, int, double, CanvasDamage*, void> Pointer;
        public delegate* unmanaged<double, double, double, int> Hover;
        public delegate* unmanaged<CanvasDamage*, void> Flush;
        public delegate* unmanaged<CanvasFrame*, void> LockFrame;
        public delegate* unmanaged<void> UnlockFrame;
        public delegate* unmanaged<Overlay*, void> GetOverlay;
        public delegate* unmanaged<double, double, CanvasDamage*, void> Nudge;
    }

    private CanvasApi* _api;
    private WriteableBitmap? _bitmap;
    private double _zoom = 1;
    private Point _offset;
    private bool _fitPending = true;
    private bool _panning;
    private bool _drawing;
    private Point _panAnchor;
    private Overlay _overlay;

    private static readonly IBrush Backdrop = new SolidColorBrush(Color.FromRgb(0x26, 0x28, 0x2C));
    private static readonly IPen SelectionPen = new Pen(new SolidColorBrush(Color.FromRgb(0x3D, 0x8B, 0xFF)), 1.2, new DashStyle(new double[] { 4, 3 }, 0));
    private static readonly IPen HandlePen = new Pen(new SolidColorBrush(Color.FromRgb(0x3D, 0x8B, 0xFF)), 1.5);
    private static readonly IPen MarqueePen = new Pen(Brushes.White, 1.5, new DashStyle(new double[] { 5, 4 }, 0));
    private static readonly IBrush Shade = new SolidColorBrush(Color.FromArgb(0x70, 0, 0, 0));

    public PixelCanvas()
    {
        ClipToBounds = true;
        Focusable = true;
    }

    /// <summary>Raised after an edit with the damage flags Rust reported.</summary>
    public event Action<int>? Changed;

    /// <summary>Raised when the user double-clicks an object (to edit its text).</summary>
    public event Action? EditRequested;

    /// <summary>Raised when the zoom changes, with the new factor.</summary>
    public event Action<double>? ZoomChanged;

    /// <summary>Raised for Escape on the canvas.</summary>
    public event Action? CancelRequested;

    public double Zoom => _zoom;

    /// <summary>The zoom from which View ▸ Pixel grid draws its lines.</summary>
    public const double GridZoom = 6;

    private bool _showPixelGrid;

    /// <summary>Lines between pixels, at <see cref="GridZoom"/> and closer.</summary>
    public bool ShowPixelGrid
    {
        get => _showPixelGrid;
        set
        {
            _showPixelGrid = value;
            InvalidateVisual();
        }
    }

    private static readonly Pen GridPen = new(new SolidColorBrush(Color.FromArgb(0x50, 0x80, 0x80, 0x80)), 1);

    /// <summary>Binds the Rust API table published as the CanvasApi property.</summary>
    public void Attach(long api)
    {
        _api = (CanvasApi*)api;
        Refresh();
    }

    /// <summary>
    /// Fetches pending damage from Rust and copies it, recreating the bitmap
    /// when the document size changed.
    /// </summary>
    public void Refresh()
    {
        if (_api == null)
            return;
        CanvasDamage damage;
        _api->Flush(&damage);
        Apply(damage, forceFull: _bitmap is null);
    }

    private void Apply(in CanvasDamage damage, bool forceFull = false)
    {
        CanvasFrame frame;
        _api->LockFrame(&frame);
        try
        {
            if (frame.Pixels == null)
                return;
            var full = forceFull || (damage.Flags & FlagFull) != 0;
            if (_bitmap is null || _bitmap.PixelSize.Width != frame.Width || _bitmap.PixelSize.Height != frame.Height)
            {
                _bitmap?.Dispose();
                _bitmap = new WriteableBitmap(new PixelSize(frame.Width, frame.Height), new Vector(96, 96), PixelFormat.Rgba8888, AlphaFormat.Premul);
                _fitPending = true;
                full = true;
            }
            if (full)
                Copy(frame, 0, 0, frame.Width, frame.Height);
            else if (damage.Width > 0 && damage.Height > 0)
                Copy(frame, damage.X, damage.Y, damage.Width, damage.Height);
        }
        finally
        {
            _api->UnlockFrame();
        }
        Overlay overlay;
        _api->GetOverlay(&overlay);
        _overlay = overlay;
        InvalidateVisual();
        if (damage.Flags != 0)
            Changed?.Invoke(damage.Flags);
    }

    private void Copy(in CanvasFrame frame, int x, int y, int width, int height)
    {
        x = Math.Max(0, x);
        y = Math.Max(0, y);
        width = Math.Min(width, frame.Width - x);
        height = Math.Min(height, frame.Height - y);
        if (width <= 0 || height <= 0)
            return;
        using var target = _bitmap!.Lock();
        var rowBytes = width * 4;
        for (var row = y; row < y + height; row++)
        {
            var source = frame.Pixels + (long)row * frame.Stride + x * 4;
            var destination = (byte*)target.Address + (long)row * target.RowBytes + x * 4;
            Buffer.MemoryCopy(source, destination, rowBytes, rowBytes);
        }
    }

    private static int Modifiers(KeyModifiers m) =>
        (m.HasFlag(KeyModifiers.Shift) ? 1 : 0) | (m.HasFlag(KeyModifiers.Control) ? 2 : 0) | (m.HasFlag(KeyModifiers.Alt) ? 4 : 0);

    private double Tolerance => 6 / _zoom;

    private void Send(int kind, PointerEventArgs e)
    {
        if (_api == null || _bitmap is null)
            return;
        var mods = Modifiers(e.KeyModifiers);
        // Coalesced moves carry the intermediate samples; feeding all of
        // them keeps freehand strokes smooth when input outpaces frames.
        var points = kind == KindMoved ? e.GetIntermediatePoints(this) : null;
        var count = points?.Count ?? 1;
        CanvasDamage total = default;
        int x0 = int.MaxValue, y0 = int.MaxValue, x1 = int.MinValue, y1 = int.MinValue;
        for (var i = 0; i < count; i++)
        {
            var p = ToDocument(points is null ? e.GetPosition(this) : points[i].Position);
            CanvasDamage damage;
            _api->Pointer(kind, p.X, p.Y, mods, Tolerance, &damage);
            total.Flags |= damage.Flags;
            total.Cursor = damage.Cursor;
            if (damage.Width <= 0)
                continue;
            x0 = Math.Min(x0, damage.X);
            y0 = Math.Min(y0, damage.Y);
            x1 = Math.Max(x1, damage.X + damage.Width);
            y1 = Math.Max(y1, damage.Y + damage.Height);
        }
        if (x1 > x0)
        {
            total.X = x0;
            total.Y = y0;
            total.Width = x1 - x0;
            total.Height = y1 - y0;
        }
        SetCursor(total.Cursor);
        Apply(total);
    }

    private void SetCursor(int code)
    {
        Cursor = new Cursor(code switch
        {
            1 => StandardCursorType.SizeAll,
            2 => StandardCursorType.TopLeftCorner,
            3 => StandardCursorType.TopRightCorner,
            4 => StandardCursorType.SizeNorthSouth,
            5 => StandardCursorType.SizeWestEast,
            6 => StandardCursorType.Cross,
            7 => StandardCursorType.Hand,
            8 => StandardCursorType.Ibeam,
            _ => StandardCursorType.Arrow,
        });
    }

    /// <summary>Feeds one pointer event in document pixels (for scripted tests).</summary>
    public void Simulate(int kind, double x, double y, int modifiers = 0)
    {
        if (_api == null)
            return;
        CanvasDamage damage;
        _api->Pointer(kind, x, y, modifiers, Tolerance, &damage);
        Apply(damage);
    }

    /// <summary>Moves the selection by whole pixels (arrow keys).</summary>
    public void Nudge(double dx, double dy)
    {
        if (_api == null)
            return;
        CanvasDamage damage;
        _api->Nudge(dx, dy, &damage);
        Apply(damage);
    }

    /// <summary>"in", "out", "fit" or "actual".</summary>
    /// Zooms to `zoom` (1 = 100%) around the middle of the view.
    public void SetZoom(double zoom)
    {
        ZoomAround(new Point(Bounds.Width / 2, Bounds.Height / 2), zoom);
        InvalidateVisual();
    }

    public void ZoomTo(string what)
    {
        var center = new Point(Bounds.Width / 2, Bounds.Height / 2);
        switch (what)
        {
            case "fit":
                FitToView();
                break;
            case "actual":
                ZoomAround(center, 1);
                break;
            case "in":
                ZoomAround(center, _zoom * 1.25);
                break;
            case "out":
                ZoomAround(center, _zoom / 1.25);
                break;
        }
        InvalidateVisual();
    }

    private void ZoomAround(Point anchor, double zoom)
    {
        var document = ToDocument(anchor);
        _zoom = Math.Clamp(zoom, 0.05, 32);
        _offset = new Point(anchor.X - document.X * _zoom, anchor.Y - document.Y * _zoom);
        _fitPending = false;
        RaiseZoom();
    }

    /// <summary>Notifies after the current pass: fitting happens during render.</summary>
    private void RaiseZoom()
    {
        var zoom = _zoom;
        Avalonia.Threading.Dispatcher.UIThread.Post(() => ZoomChanged?.Invoke(zoom));
    }

    public PixelSize DocumentSize => _bitmap?.PixelSize ?? default;

    /// A view point in document pixels.
    public Point DocumentPoint(Point view) => ToDocument(view);

    private Point ToDocument(Point view) => new((view.X - _offset.X) / _zoom, (view.Y - _offset.Y) / _zoom);

    private Point ToView(double x, double y) => new(x * _zoom + _offset.X, y * _zoom + _offset.Y);

    private void FitToView()
    {
        if (_bitmap is null || Bounds.Width <= 0 || Bounds.Height <= 0)
            return;
        var size = _bitmap.PixelSize;
        _zoom = Math.Min(1, Math.Min((Bounds.Width - 48) / size.Width, (Bounds.Height - 48) / size.Height));
        _offset = new Point(Math.Round((Bounds.Width - size.Width * _zoom) / 2), Math.Round((Bounds.Height - size.Height * _zoom) / 2));
        _fitPending = false;
        RaiseZoom();
    }

    protected override void OnSizeChanged(SizeChangedEventArgs e)
    {
        base.OnSizeChanged(e);
        _fitPending = true;
    }

    public override void Render(DrawingContext context)
    {
        var backdrop = this.TryFindResource("SwChip", ActualThemeVariant, out var themed) && themed is IBrush brush ? brush : Backdrop;
        context.FillRectangle(backdrop, new Rect(Bounds.Size));
        if (_bitmap is null)
            return;
        if (_fitPending)
            FitToView();
        var size = _bitmap.PixelSize;
        var dest = new Rect(_offset.X, _offset.Y, size.Width * _zoom, size.Height * _zoom);
        // Nearest-neighbor when zoomed in, so pixels stay crisp.
        var mode = _zoom >= 2 ? BitmapInterpolationMode.None : BitmapInterpolationMode.MediumQuality;
        using (context.PushRenderOptions(new RenderOptions { BitmapInterpolationMode = mode }))
            context.DrawImage(_bitmap, new Rect(0, 0, size.Width, size.Height), dest);
        if (_showPixelGrid && _zoom >= GridZoom)
            DrawPixelGrid(context, dest, size);
        DrawOverlay(context, dest);
    }

    /// Only the visible lines: a column or row per image pixel in view.
    private void DrawPixelGrid(DrawingContext context, Rect image, PixelSize size)
    {
        var view = image.Intersect(new Rect(Bounds.Size));
        var x0 = (int)Math.Floor((view.Left - image.Left) / _zoom);
        var x1 = (int)Math.Ceiling((view.Right - image.Left) / _zoom);
        var y0 = (int)Math.Floor((view.Top - image.Top) / _zoom);
        var y1 = (int)Math.Ceiling((view.Bottom - image.Top) / _zoom);
        for (var x = Math.Max(1, x0); x < Math.Min(size.Width, x1); x++)
        {
            var at = Math.Round(image.Left + x * _zoom) + 0.5;
            context.DrawLine(GridPen, new Point(at, view.Top), new Point(at, view.Bottom));
        }
        for (var y = Math.Max(1, y0); y < Math.Min(size.Height, y1); y++)
        {
            var at = Math.Round(image.Top + y * _zoom) + 0.5;
            context.DrawLine(GridPen, new Point(view.Left, at), new Point(view.Right, at));
        }
    }

    private void DrawOverlay(DrawingContext context, Rect image)
    {
        var o = _overlay;
        for (var i = 0; i < o.OtherCount; i++)
        {
            var r = new Rect(ToView(o.Others[i * 4], o.Others[i * 4 + 1]), ToView(o.Others[i * 4] + o.Others[i * 4 + 2], o.Others[i * 4 + 1] + o.Others[i * 4 + 3]));
            context.DrawRectangle(null, SelectionPen, r.Inflate(1));
        }
        if ((o.Flags & 6) == 6)
        {
            var m = new Rect(ToView(o.Marquee[0], o.Marquee[1]), ToView(o.Marquee[0] + o.Marquee[2], o.Marquee[1] + o.Marquee[3]));
            context.DrawRectangle(new SolidColorBrush(Color.FromArgb(0x22, 0x3D, 0x8B, 0xFF)), SelectionPen, m);
        }
        else if ((o.Flags & 2) != 0)
        {
            var m = new Rect(ToView(o.Marquee[0], o.Marquee[1]), ToView(o.Marquee[0] + o.Marquee[2], o.Marquee[1] + o.Marquee[3]));
            // Shade what a crop would remove.
            var shade = new RectangleGeometry(image);
            var combined = new CombinedGeometry(GeometryCombineMode.Exclude, shade, new RectangleGeometry(m));
            context.DrawGeometry(Shade, null, combined);
            context.DrawRectangle(null, MarqueePen, m);
            var label = new FormattedText($"{Math.Round(o.Marquee[2])} × {Math.Round(o.Marquee[3])}", System.Globalization.CultureInfo.CurrentCulture,
                FlowDirection.LeftToRight, Typeface.Default, 12, Brushes.White);
            var at = new Point(m.X, Math.Max(0, m.Y - 20));
            context.FillRectangle(new SolidColorBrush(Color.FromArgb(0xC0, 0x20, 0x20, 0x20)), new Rect(at, new Size(label.Width + 10, label.Height + 4)), 3);
            context.DrawText(label, at + new Point(5, 2));
        }
        if ((o.Flags & 8) != 0)
        {
            // Canvas edge handles (drag to resize the canvas).
            for (var i = 0; i < o.HandleCount; i++)
            {
                var p = ToView(o.Handles[i * 2], o.Handles[i * 2 + 1]);
                context.DrawRectangle(new SolidColorBrush(Color.FromRgb(0xC8, 0xCC, 0xD2)), HandlePen, new Rect(p.X - 4, p.Y - 4, 8, 8));
            }
        }
        if ((o.Flags & 1) == 0)
            return;
        var s = new Rect(ToView(o.Selection[0], o.Selection[1]), ToView(o.Selection[0] + o.Selection[2], o.Selection[1] + o.Selection[3]));
        var turn = Matrix.CreateTranslation(-s.Center.X, -s.Center.Y) * Matrix.CreateRotation(o.Rotation * Math.PI / 180) * Matrix.CreateTranslation(s.Center.X, s.Center.Y);
        using (context.PushTransform(turn))
            context.DrawRectangle(null, SelectionPen, s.Inflate(1));
        for (var i = 0; i < o.HandleCount; i++)
        {
            var p = ToView(o.Handles[i * 2], o.Handles[i * 2 + 1]);
            var r = new Rect(p.X - HandleSize / 2, p.Y - HandleSize / 2, HandleSize, HandleSize);
            if (o.HandleIds[i] == HandleRotate)
                context.DrawEllipse(Brushes.White, HandlePen, p, HandleSize / 2 + 1, HandleSize / 2 + 1);
            else if (o.HandleIds[i] >= 8)
                context.DrawEllipse(Brushes.White, HandlePen, p, HandleSize / 2, HandleSize / 2);
            else
                context.DrawRectangle(Brushes.White, HandlePen, r);
        }
    }

    protected override void OnPointerPressed(PointerPressedEventArgs e)
    {
        base.OnPointerPressed(e);
        Focus();
        var properties = e.GetCurrentPoint(this).Properties;
        if (properties.IsMiddleButtonPressed || (properties.IsLeftButtonPressed && e.KeyModifiers.HasFlag(KeyModifiers.Alt)) || properties.IsRightButtonPressed)
        {
            _panning = true;
            _panAnchor = e.GetPosition(this);
            Cursor = new Cursor(StandardCursorType.SizeAll);
        }
        else if (properties.IsLeftButtonPressed)
        {
            if (e.ClickCount == 2)
            {
                EditRequested?.Invoke();
                e.Handled = true;
                return;
            }
            _drawing = true;
            Send(KindPressed, e);
        }
        else
        {
            return;
        }
        e.Pointer.Capture(this);
        e.Handled = true;
    }

    protected override void OnPointerMoved(PointerEventArgs e)
    {
        base.OnPointerMoved(e);
        if (_panning)
        {
            var position = e.GetPosition(this);
            _offset += position - _panAnchor;
            _panAnchor = position;
            _fitPending = false;
            InvalidateVisual();
        }
        else if (_drawing)
        {
            Send(KindMoved, e);
        }
        else if (_api != null && _bitmap != null)
        {
            var p = ToDocument(e.GetPosition(this));
            SetCursor(_api->Hover(p.X, p.Y, Tolerance));
        }
    }

    protected override void OnPointerReleased(PointerReleasedEventArgs e)
    {
        base.OnPointerReleased(e);
        if (_panning)
        {
            _panning = false;
            Cursor = Cursor.Default;
        }
        else if (_drawing && e.InitialPressMouseButton == MouseButton.Left)
        {
            _drawing = false;
            Send(KindReleased, e);
        }
        e.Pointer.Capture(null);
    }

    protected override void OnPointerCaptureLost(PointerCaptureLostEventArgs e)
    {
        base.OnPointerCaptureLost(e);
        _panning = false;
    }

    protected override void OnPointerWheelChanged(PointerWheelEventArgs e)
    {
        base.OnPointerWheelChanged(e);
        ZoomAround(e.GetPosition(this), _zoom * Math.Pow(1.15, e.Delta.Y));
        InvalidateVisual();
        e.Handled = true;
    }

    protected override void OnKeyDown(KeyEventArgs e)
    {
        base.OnKeyDown(e);
        var step = e.KeyModifiers.HasFlag(KeyModifiers.Shift) ? 10 : 1;
        switch (e.Key)
        {
            case Key.Left:
                Nudge(-step, 0);
                break;
            case Key.Right:
                Nudge(step, 0);
                break;
            case Key.Up:
                Nudge(0, -step);
                break;
            case Key.Down:
                Nudge(0, step);
                break;
            case Key.Escape:
                CancelRequested?.Invoke();
                break;
            default:
                return;
        }
        e.Handled = true;
    }
}
