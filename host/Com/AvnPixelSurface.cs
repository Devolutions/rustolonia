using System;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.Platform;
using Avalonia.Threading;
using Avalonia.VisualTree;

namespace Avalonia.Host.Com;

/// <summary>Stage 34: creates Rust-drawn pixel surfaces.</summary>
public partial class AvnApplication : IAvnApplication7
{
    public int CreatePixelSurface(IAvnPixelSurfaceHandler? handler, out IAvnPixelSurface? surface)
    {
        surface = null;
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            surface = new AvnPixelSurface(new RustPixelSurface { Handler = handler });
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }
}

/// <summary>The COM face of a <see cref="RustPixelSurface"/>.</summary>
[GeneratedComClass]
internal sealed partial class AvnPixelSurface(RustPixelSurface control) : IAvnPixelSurface
{
    public const int MaxLayers = 8;

    public int GetControl(out IAvnControl? value)
    {
        IAvnControl? found = null;
        var hr = Run(() => found = (IAvnControl?)ProjectionRuntime.Wrap(control));
        value = found;
        return hr;
    }

    public int SetLayer(int layer, int width, int height, int format)
    {
        if (layer is < 0 or >= MaxLayers || width < 0 || height < 0 || format is < 0 or > 1)
            return HResults.E_INVALIDARG;
        return Run(() => control.SetLayer(layer, width, height, format == 0 ? PixelFormat.Rgba8888 : PixelFormat.Bgra8888));
    }

    public int WriteLayer(int layer, int x, int y, int width, int height, nint pixels, int stride)
    {
        if (layer is < 0 or >= MaxLayers || width < 0 || height < 0 || stride < width * 4)
            return HResults.E_INVALIDARG;
        if (pixels == 0 && width > 0 && height > 0)
            return HResults.E_POINTER;
        return Run(() => control.WriteLayer(layer, x, y, width, height, pixels, stride));
    }

    public int PlaceLayer(int layer, double x, double y, double width, double height, int fill, int interpolation, double opacity)
    {
        if (layer is < 0 or >= MaxLayers || interpolation is < 0 or > 4)
            return HResults.E_INVALIDARG;
        return Run(() => control.PlaceLayer(layer, new Rect(x, y, Math.Max(0, width), Math.Max(0, height)), fill != 0,
            (BitmapInterpolationMode)interpolation, Math.Clamp(opacity, 0, 1)));
    }

    public int SetBackground(uint argb) => Run(() =>
    {
        control.Backdrop = argb == 0 ? null : new SolidColorBrush(Color.FromUInt32(argb));
        control.InvalidateVisual();
    });

    public int SetCursor(int cursor)
    {
        if (cursor != -1 && !Enum.IsDefined((StandardCursorType)cursor))
            return HResults.E_INVALIDARG;
        return Run(() => control.Cursor = cursor < 0 ? null : new Cursor((StandardCursorType)cursor));
    }

    public int SetDesiredSize(double width, double height) => Run(() =>
    {
        control.Desired = new Size(Math.Max(0, width), Math.Max(0, height));
        control.InvalidateMeasure();
    });

    public int Focus() => Run(() => control.Focus());

    public int GetMetrics(out double width, out double height, out double scaling)
    {
        Size size = default;
        double found = 1;
        var hr = Run(() =>
        {
            size = control.Bounds.Size;
            found = control.Scaling;
        });
        width = size.Width;
        height = size.Height;
        scaling = found;
        return hr;
    }

    public int Detach() => Run(() => control.Handler = null);

    private static int Run(Action action)
    {
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            action();
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }
}

/// <summary>
/// A control drawing Rust-owned bitmap layers and forwarding its pointer and
/// keyboard input to an <see cref="IAvnPixelSurfaceHandler"/>.
/// </summary>
internal sealed class RustPixelSurface : Control
{
    private sealed class Layer
    {
        public WriteableBitmap? Bitmap;
        public Rect Destination;
        public bool Fill = true;
        public BitmapInterpolationMode Interpolation;
        public double Opacity = 1;
    }

    private readonly Layer[] _layers = new Layer[AvnPixelSurface.MaxLayers];
    private TopLevel? _topLevel;

    public RustPixelSurface()
    {
        ClipToBounds = true;
        Focusable = true;
        for (var i = 0; i < _layers.Length; i++)
            _layers[i] = new Layer();
    }

    public IAvnPixelSurfaceHandler? Handler { get; set; }

    public IBrush? Backdrop { get; set; }

    public Size Desired { get; set; }

    public double Scaling => _topLevel?.RenderScaling ?? 1;

    public void SetLayer(int index, int width, int height, PixelFormat format)
    {
        var layer = _layers[index];
        if (width == 0 || height == 0)
        {
            layer.Bitmap?.Dispose();
            layer.Bitmap = null;
        }
        else if (layer.Bitmap is not { } bitmap || bitmap.PixelSize.Width != width || bitmap.PixelSize.Height != height || bitmap.Format != format)
        {
            layer.Bitmap?.Dispose();
            // New WriteableBitmap memory is zeroed: transparent in both formats.
            layer.Bitmap = new WriteableBitmap(new PixelSize(width, height), new Vector(96, 96), format, AlphaFormat.Premul);
        }
        InvalidateVisual();
    }

    public unsafe void WriteLayer(int index, int x, int y, int width, int height, nint pixels, int stride)
    {
        var bitmap = _layers[index].Bitmap ?? throw new InvalidOperationException($"Layer {index} has no bitmap; call SetLayer first.");
        var size = bitmap.PixelSize;
        var left = Math.Max(0, x);
        var top = Math.Max(0, y);
        var right = Math.Min(size.Width, x + width);
        var bottom = Math.Min(size.Height, y + height);
        if (right > left && bottom > top)
        {
            using var target = bitmap.Lock();
            var rowBytes = (right - left) * 4;
            for (var row = top; row < bottom; row++)
            {
                var source = (byte*)pixels + (long)(row - y) * stride + (long)(left - x) * 4;
                var destination = (byte*)target.Address + (long)row * target.RowBytes + (long)left * 4;
                Buffer.MemoryCopy(source, destination, rowBytes, rowBytes);
            }
        }
        InvalidateVisual();
    }

    public void PlaceLayer(int index, Rect destination, bool fill, BitmapInterpolationMode interpolation, double opacity)
    {
        var layer = _layers[index];
        layer.Destination = destination;
        layer.Fill = fill;
        layer.Interpolation = interpolation;
        layer.Opacity = opacity;
        InvalidateVisual();
    }

    protected override Size MeasureOverride(Size availableSize) => Desired;

    public override void Render(DrawingContext context)
    {
        var bounds = new Rect(Bounds.Size);
        if (Backdrop is { } backdrop)
            context.FillRectangle(backdrop, bounds);
        foreach (var layer in _layers)
        {
            if (layer.Bitmap is not { } bitmap || layer.Opacity <= 0)
                continue;
            var destination = layer.Fill ? bounds : layer.Destination;
            if (destination.Width <= 0 || destination.Height <= 0)
                continue;
            using var options = context.PushRenderOptions(new RenderOptions { BitmapInterpolationMode = layer.Interpolation });
            using var opacity = context.PushOpacity(layer.Opacity);
            context.DrawImage(bitmap, new Rect(bitmap.Size), destination);
        }
    }

    protected override void OnAttachedToVisualTree(VisualTreeAttachmentEventArgs e)
    {
        base.OnAttachedToVisualTree(e);
        _topLevel = TopLevel.GetTopLevel(this);
        if (_topLevel is not null)
            _topLevel.ScalingChanged += OnScalingChanged;
        ReportSize();
    }

    protected override void OnDetachedFromVisualTree(VisualTreeAttachmentEventArgs e)
    {
        base.OnDetachedFromVisualTree(e);
        if (_topLevel is not null)
            _topLevel.ScalingChanged -= OnScalingChanged;
        _topLevel = null;
    }

    private void OnScalingChanged(object? sender, EventArgs e) => ReportSize();

    protected override void OnSizeChanged(SizeChangedEventArgs e)
    {
        base.OnSizeChanged(e);
        ReportSize();
    }

    private void ReportSize() => Handler?.OnResize(Bounds.Width, Bounds.Height, Scaling);

    private static int Buttons(PointerPointProperties p) =>
        (p.IsLeftButtonPressed ? 1 : 0) | (p.IsRightButtonPressed ? 2 : 0) | (p.IsMiddleButtonPressed ? 4 : 0) |
        (p.IsXButton1Pressed ? 8 : 0) | (p.IsXButton2Pressed ? 16 : 0);

    private static int ButtonOf(PointerUpdateKind kind) => kind switch
    {
        PointerUpdateKind.LeftButtonPressed or PointerUpdateKind.LeftButtonReleased => 1,
        PointerUpdateKind.RightButtonPressed or PointerUpdateKind.RightButtonReleased => 2,
        PointerUpdateKind.MiddleButtonPressed or PointerUpdateKind.MiddleButtonReleased => 3,
        PointerUpdateKind.XButton1Pressed or PointerUpdateKind.XButton1Released => 4,
        PointerUpdateKind.XButton2Pressed or PointerUpdateKind.XButton2Released => 5,
        _ => 0,
    };

    private bool Send(int kind, PointerPoint point, int button, KeyModifiers modifiers, int clickCount = 0, Vector delta = default)
    {
        if (Handler is not { } handler)
            return false;
        var p = point.Position;
        var hr = handler.OnPointer(kind, p.X, p.Y, button, Buttons(point.Properties), (int)modifiers, clickCount, delta.X, delta.Y, out var handled);
        return hr >= 0 && handled != 0;
    }

    protected override void OnPointerPressed(PointerPressedEventArgs e)
    {
        base.OnPointerPressed(e);
        if (Handler is null)
            return;
        Focus();
        var point = e.GetCurrentPoint(this);
        e.Pointer.Capture(this);
        if (Send(0, point, ButtonOf(point.Properties.PointerUpdateKind), e.KeyModifiers, e.ClickCount))
            e.Handled = true;
    }

    protected override void OnPointerMoved(PointerEventArgs e)
    {
        base.OnPointerMoved(e);
        if (Handler is null)
            return;
        var handled = false;
        foreach (var point in e.GetIntermediatePoints(this))
            handled |= Send(1, point, 0, e.KeyModifiers);
        if (handled)
            e.Handled = true;
    }

    protected override void OnPointerReleased(PointerReleasedEventArgs e)
    {
        base.OnPointerReleased(e);
        if (Handler is null)
            return;
        var point = e.GetCurrentPoint(this);
        var button = e.InitialPressMouseButton switch
        {
            MouseButton.Left => 1,
            MouseButton.Right => 2,
            MouseButton.Middle => 3,
            MouseButton.XButton1 => 4,
            MouseButton.XButton2 => 5,
            _ => 0,
        };
        if (Send(2, point, button, e.KeyModifiers))
            e.Handled = true;
        if (Buttons(point.Properties) == 0 && e.Pointer.Captured == this)
            e.Pointer.Capture(null);
    }

    protected override void OnPointerWheelChanged(PointerWheelEventArgs e)
    {
        base.OnPointerWheelChanged(e);
        if (Send(3, e.GetCurrentPoint(this), 0, e.KeyModifiers, 0, e.Delta))
            e.Handled = true;
    }

    protected override void OnPointerEntered(PointerEventArgs e)
    {
        base.OnPointerEntered(e);
        Send(4, e.GetCurrentPoint(this), 0, e.KeyModifiers);
    }

    protected override void OnPointerExited(PointerEventArgs e)
    {
        base.OnPointerExited(e);
        Send(5, e.GetCurrentPoint(this), 0, e.KeyModifiers);
    }

    protected override void OnPointerCaptureLost(PointerCaptureLostEventArgs e)
    {
        base.OnPointerCaptureLost(e);
        Handler?.OnPointer(6, 0, 0, 0, 0, 0, 0, 0, 0, out _);
    }

    protected override void OnKeyDown(KeyEventArgs e)
    {
        base.OnKeyDown(e);
        if (Handler is { } handler && handler.OnKey(0, (int)e.Key, (int)e.KeyModifiers, out var handled) >= 0 && handled != 0)
            e.Handled = true;
    }

    protected override void OnKeyUp(KeyEventArgs e)
    {
        base.OnKeyUp(e);
        if (Handler is { } handler && handler.OnKey(1, (int)e.Key, (int)e.KeyModifiers, out var handled) >= 0 && handled != 0)
            e.Handled = true;
    }

    protected override void OnTextInput(TextInputEventArgs e)
    {
        base.OnTextInput(e);
        if (Handler is { } handler && !string.IsNullOrEmpty(e.Text) && handler.OnTextInput(e.Text, out var handled) >= 0 && handled != 0)
            e.Handled = true;
    }
}
