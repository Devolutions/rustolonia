using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

/// <summary>
/// A host-owned control drawing up to <see cref="AvnPixelSurface.MaxLayers"/>
/// bitmap layers over a background, in layer order. Every member must be called
/// on the UI thread.
/// </summary>
[GeneratedComInterface]
[Guid(AvnGuids.IAvnPixelSurface)]
public partial interface IAvnPixelSurface
{
    /// <summary>The control to put in a layout (a plain <c>Control</c>).</summary>
    [PreserveSig]
    int GetControl(out IAvnControl? control);

    /// <summary>
    /// Allocates layer <paramref name="layer"/> as a <paramref name="width"/> by
    /// <paramref name="height"/> bitmap, cleared to transparent, or removes it
    /// when either is 0. <paramref name="format"/> is 0 for premultiplied
    /// RGBA8888 and 1 for premultiplied BGRA8888. Reallocating at the same size
    /// and format keeps the pixels.
    /// </summary>
    [PreserveSig]
    int SetLayer(int layer, int width, int height, int format);

    /// <summary>
    /// Copies a rectangle of pixels into the layer at (<paramref name="x"/>,
    /// <paramref name="y"/>), clipped to the layer, and redraws.
    /// <paramref name="pixels"/> points at the first row,
    /// <paramref name="stride"/> bytes apart, in the layer's format.
    /// </summary>
    [PreserveSig]
    int WriteLayer(int layer, int x, int y, int width, int height, nint pixels, int stride);

    /// <summary>
    /// Where the layer is drawn, in device-independent pixels relative to the
    /// control: the rectangle, or the whole control when <paramref name="fill"/>
    /// is non-zero. <paramref name="interpolation"/> is Avalonia's
    /// <c>BitmapInterpolationMode</c> (0 unspecified, 1 none, 2 low, 3 medium,
    /// 4 high). The default is the whole control at full opacity.
    /// </summary>
    [PreserveSig]
    int PlaceLayer(int layer, double x, double y, double width, double height, int fill, int interpolation, double opacity);

    /// <summary>Fills the control under the layers with an ARGB color (0 for none).</summary>
    [PreserveSig]
    int SetBackground(uint argb);

    /// <summary>
    /// Sets the pointer shape: Avalonia's <c>StandardCursorType</c>, or -1 for
    /// the inherited cursor.
    /// </summary>
    [PreserveSig]
    int SetCursor(int cursor);

    /// <summary>
    /// The size the surface asks for in layout, in device-independent pixels;
    /// 0 asks for nothing and takes what the layout gives (the default).
    /// </summary>
    [PreserveSig]
    int SetDesiredSize(double width, double height);

    /// <summary>Moves keyboard focus to the surface.</summary>
    [PreserveSig]
    int Focus();

    /// <summary>
    /// The control's current size in device-independent pixels and its
    /// top-level's render scaling (physical pixels per DIP).
    /// </summary>
    [PreserveSig]
    int GetMetrics(out double width, out double height, out double scaling);

    /// <summary>
    /// Stops reporting input and releases the handler. The control keeps
    /// drawing its layers.
    /// </summary>
    [PreserveSig]
    int Detach();
}

/// <summary>
/// Implemented by Rust: receives a pixel surface's input. Positions are in
/// device-independent pixels relative to the control.
/// </summary>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnPixelSurfaceHandler)]
public partial interface IAvnPixelSurfaceHandler
{
    /// <summary>
    /// A pointer event. <paramref name="kind"/>: 0 pressed, 1 moved, 2 released,
    /// 3 wheel, 4 entered, 5 exited, 6 capture lost. <paramref name="button"/> is
    /// the button that changed (0 none, 1 left, 2 right, 3 middle, 4 X1, 5 X2);
    /// <paramref name="buttons"/> the ones held after the event (bit 0 left,
    /// 1 right, 2 middle, 3 X1, 4 X2). <paramref name="modifiers"/> is
    /// Avalonia's <c>KeyModifiers</c>. A press captures the pointer until its
    /// release. Moves include the coalesced intermediate points, one call each.
    /// Setting <paramref name="handled"/> stops the event from bubbling.
    /// </summary>
    [PreserveSig]
    int OnPointer(int kind, double x, double y, int button, int buttons, int modifiers, int clickCount, double deltaX, double deltaY, out int handled);

    /// <summary>
    /// A key going down (<paramref name="kind"/> 0) or up (1) while the surface
    /// has focus: Avalonia's <c>Key</c> and <c>KeyModifiers</c>.
    /// </summary>
    [PreserveSig]
    int OnKey(int kind, int key, int modifiers, out int handled);

    /// <summary>Text typed while the surface has focus.</summary>
    [PreserveSig]
    int OnTextInput(string? text, out int handled);

    /// <summary>
    /// The control's size (device-independent pixels) or render scaling
    /// changed, and when it is first attached to a window.
    /// </summary>
    [PreserveSig]
    int OnResize(double width, double height, double scaling);
}
