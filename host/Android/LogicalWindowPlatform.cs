using System;
using System.Collections.Generic;
using Avalonia.Controls;
using Avalonia.Controls.Platform;
using Avalonia.Input;
using Avalonia.Input.Raw;
using Avalonia.Platform;
using Avalonia.Platform.Surfaces;
using Avalonia.Rendering.Composition;

namespace Avalonia.Host.Android;

/// <summary>
/// Android has no windowing platform: Avalonia.Android registers a stub whose
/// <c>CreateWindow</c> throws. The Rust API and the generated projection are
/// window-centric (<c>new Window()</c>, <c>Window.Show</c>), so the Android host
/// replaces that stub with logical windows. A logical window is a real
/// <see cref="Window"/> that never renders by itself; <see cref="AndroidWindowPresenter"/>
/// presents the content of the top shown window inside the activity.
/// </summary>
internal sealed class LogicalWindowingPlatform(IWindowingPlatform? fallback) : IWindowingPlatform
{
    public IWindowImpl CreateWindow() => new LogicalWindowImpl();

    public ITopLevelImpl CreateEmbeddableTopLevel() =>
        fallback?.CreateEmbeddableTopLevel() ?? throw new PlatformNotSupportedException();

    public IWindowImpl CreateEmbeddableWindow() => new LogicalWindowImpl();

    public ITrayIconImpl? CreateTrayIcon() => null;

    public void GetWindowsZOrder(ReadOnlySpan<IWindowImpl> windows, Span<long> zOrder)
    {
        for (var index = 0; index < windows.Length; index++)
            zOrder[index] = AndroidWindowPresenter.ZOrder(windows[index]);
    }
}

/// <summary>
/// The platform half of a logical window. It has no render surface (the
/// compositor waits for one forever, so nothing is drawn twice), reports the
/// activity's size and scaling, and resolves platform features (clipboard,
/// storage, launcher, insets) through the activity's real top level, so the
/// existing clipboard and picker code in the host works unchanged.
/// </summary>
internal sealed class LogicalWindowImpl : IWindowImpl
{
    private Size _clientSize = new(400, 800);
    private bool _disposed;

    private IInputRoot? _inputRoot;
    private Window? _owner;

    // Avalonia passes the window's PresentationSource, not the window, to
    // SetInputRoot; the window is the logical parent of its root visual.
    internal Window? Owner => _owner ??= _inputRoot is Rendering.IPresentationSource { RootVisual: { } root }
        ? TopLevel.GetTopLevel(root) as Window ?? ((LogicalTree.ILogical)root).LogicalParent as Window
        : null;

    public Size ClientSize => AndroidWindowPresenter.ViewportSize ?? _clientSize;
    public double DesktopScaling => RenderScaling;
    public double RenderScaling => AndroidWindowPresenter.RenderScaling;
    public IPlatformHandle? Handle => null;
    public IPlatformRenderSurface[] Surfaces { get; } = [];
    public Compositor Compositor { get; } = AvaloniaLocator.Current.GetRequiredService<Compositor>();
    public Size? FrameSize => ClientSize;
    public PixelPoint Position => default;
    public Size MaxAutoSizeHint => ClientSize;
    public WindowState WindowState { get; set; } = WindowState.Maximized;
    public bool WindowStateGetterIsUsable => true;
    public WindowTransparencyLevel TransparencyLevel => WindowTransparencyLevel.None;
    public AcrylicPlatformCompensationLevels AcrylicCompensationLevels => new(1, 1, 1);
    public bool IsClientAreaExtendedToDecorations => false;
    public bool NeedsManagedDecorations => false;
    public PlatformRequestedDrawnDecoration RequestedDrawnDecorations => default;
    public Thickness ExtendedMargins => default;
    public Thickness OffScreenMargin => default;

    public Action<RawInputEventArgs>? Input { get; set; }
    public Action<Rect>? Paint { get; set; }
    public Action<Size, WindowResizeReason>? Resized { get; set; }
    public Action<double>? ScalingChanged { get; set; }
    public Action<WindowTransparencyLevel>? TransparencyLevelChanged { get; set; }
    public Action? Closed { get; set; }
    public Action? LostFocus { get; set; }
    public Action<PixelPoint>? PositionChanged { get; set; }
    public Action? Deactivated { get; set; }
    public Action? Activated { get; set; }
    public Action<WindowState>? WindowStateChanged { get; set; }
    public Action? GotInputWhenDisabled { get; set; }
    public Func<WindowCloseReason, bool>? Closing { get; set; }
    public Action<bool>? ExtendClientAreaToDecorationsChanged { get; set; }

    public object? TryGetFeature(Type featureType) =>
        AndroidWindowPresenter.TryGetActivityFeature(featureType)
        // Window.Show needs screens even before the activity exists.
        ?? (featureType == typeof(IScreenImpl) ? LogicalScreens.Instance : null);

    public void SetInputRoot(IInputRoot inputRoot)
    {
        _inputRoot = inputRoot;
        _owner = null;
    }

    public void Show(bool activate, bool isDialog)
    {
        var window = Owner ?? throw new InvalidOperationException(
            "A logical window was shown before its Window could be resolved.");
        AndroidWindowPresenter.Push(window);
        Activated?.Invoke();
    }

    public void Hide()
    {
        if (Owner is { } window)
            AndroidWindowPresenter.Remove(window);
    }

    public void Dispose()
    {
        if (_disposed)
            return;
        _disposed = true;
        if (Owner is { } window)
            AndroidWindowPresenter.Remove(window);
        Closed?.Invoke();
    }

    public void Resize(Size clientSize, WindowResizeReason reason = WindowResizeReason.Application) =>
        _clientSize = clientSize;

    public void SetTitle(string? title) => AndroidWindowPresenter.TitleChanged(Owner);

    public Point PointToClient(PixelPoint point) => point.ToPoint(RenderScaling);
    public PixelPoint PointToScreen(Point point) => PixelPoint.FromPoint(point, RenderScaling);
    public IPopupImpl? CreatePopup() => null;
    public void Activate() => Activated?.Invoke();
    public void SetCursor(ICursorImpl? cursor) { }
    public void SetTransparencyLevelHint(IReadOnlyList<WindowTransparencyLevel> transparencyLevels) { }
    public void SetFrameThemeVariant(PlatformThemeVariant? themeVariant) { }
    public void SetTopmost(bool value) { }
    public void SetParent(IWindowImpl? parent) { }
    public void SetEnabled(bool enable) { }
    public void SetWindowDecorations(WindowDecorations enabled) { }
    public void SetIcon(IWindowIconImpl? icon) { }
    public void ShowTaskbarIcon(bool value) { }
    public void CanResize(bool value) { }
    public void SetCanMinimize(bool value) { }
    public void SetCanMaximize(bool value) { }
    public void BeginMoveDrag(PointerPressedEventArgs e) { }
    public void BeginResizeDrag(WindowEdge edge, PointerPressedEventArgs e) { }
    public void Move(PixelPoint point) { }
    public void SetMinMaxSize(Size minSize, Size maxSize) { }
    public void SetExtendClientAreaToDecorationsHint(bool extendIntoClientAreaHint) { }
    public void SetExtendClientAreaTitleBarHeightHint(double titleBarHeight) { }
}

/// <summary>
/// The device display as the only screen, for logical windows shown before
/// the activity (whose top level reports the real screens) exists.
/// </summary>
internal sealed class LogicalScreens : IScreenImpl
{
    internal static readonly LogicalScreens Instance = new();

    private readonly Screen _screen = new LogicalScreen();

    public int ScreenCount => 1;
    public IReadOnlyList<Screen> AllScreens => [_screen];
    public Action? Changed { get; set; }
    public Screen? ScreenFromWindow(IWindowBaseImpl window) => _screen;
    public Screen? ScreenFromTopLevel(ITopLevelImpl topLevel) => _screen;
    public Screen? ScreenFromPoint(PixelPoint point) => _screen;
    public Screen? ScreenFromRect(PixelRect rect) => _screen;
    public System.Threading.Tasks.Task<bool> RequestScreenDetails() =>
        System.Threading.Tasks.Task.FromResult(true);

    private sealed class LogicalScreen : PlatformScreen
    {
        public LogicalScreen() : base(new PlatformHandle(IntPtr.Zero, "DisplayId"))
        {
            var metrics = global::Android.App.Application.Context.Resources?.DisplayMetrics;
            var bounds = new PixelRect(0, 0, metrics?.WidthPixels ?? 1080, metrics?.HeightPixels ?? 1920);
            DisplayName = "Android display";
            Scaling = metrics?.Density ?? 1;
            Bounds = bounds;
            WorkingArea = bounds;
            IsPrimary = true;
        }
    }
}
