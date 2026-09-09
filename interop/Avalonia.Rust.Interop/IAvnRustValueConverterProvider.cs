using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Rust.Interop;

/// <summary>
/// Resolves Rust-authored <c>IValueConverter</c> implementations by a stable
/// numeric ID. A single instance is registered for the lifetime of the
/// application (see <c>Avalonia.Rust.RustValueConverterRuntime</c>) so
/// it can be resolved from deferred DataTemplate/ControlTemplate realization,
/// not only while a window is being constructed.
/// </summary>
/// <remarks>
/// <para>
/// Scalar values are transported as a tagged tuple of primitive fields
/// (<c>kind</c> plus one field per supported representation) rather than a
/// single managed object, so the transport stays blittable and versionable.
/// The supported kinds are: <c>null</c>, UTF-16 string, 64-bit integer,
/// Boolean, double, <c>Avalonia.AvaloniaProperty.UnsetValue</c>, and
/// <c>Avalonia.Data.BindingOperations.DoNothing</c>. Arbitrary managed
/// objects are out of scope for this transport; callers must reject them
/// before crossing the ABI (see <c>Avalonia.Rust.RustValueConverter</c>).
/// </para>
/// <para>
/// Result and error strings are allocated by Rust using the host's
/// <c>avn_alloc_utf16</c> export (the same allocator backing <c>avn_free</c>)
/// so the managed caller can free them with <see cref="Marshal.FreeCoTaskMem"/>
/// without risking a cross-allocator mismatch.
/// </para>
/// </remarks>
[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D40")]
public partial interface IAvnRustValueConverterProvider
{
    /// <summary>
    /// Converts (or converts back) a tagged scalar value through a named Rust
    /// converter. Implementations must be pure: they must not read or lock
    /// any Rust view-model state, and must not block on the UI thread.
    /// </summary>
    /// <param name="converterId">Stable ID from the view-model IR.</param>
    /// <param name="direction">0 = Convert, 1 = ConvertBack.</param>
    /// <param name="valueKind">Tag for the value to convert; see <c>Avalonia.Rust.AvnValueKind</c>.</param>
    /// <param name="valueInt64">Populated when <paramref name="valueKind"/> is Int64.</param>
    /// <param name="valueDouble">Populated when <paramref name="valueKind"/> is Double.</param>
    /// <param name="valueBoolean">Populated (0/1) when <paramref name="valueKind"/> is Boolean.</param>
    /// <param name="valueString">Populated when <paramref name="valueKind"/> is String.</param>
    /// <param name="parameterKind">Tag for the converter parameter.</param>
    /// <param name="parameterInt64">Populated when <paramref name="parameterKind"/> is Int64.</param>
    /// <param name="parameterDouble">Populated when <paramref name="parameterKind"/> is Double.</param>
    /// <param name="parameterBoolean">Populated (0/1) when <paramref name="parameterKind"/> is Boolean.</param>
    /// <param name="parameterString">Populated when <paramref name="parameterKind"/> is String.</param>
    /// <param name="targetKind">Tag hint for the binding target type, or Any when unknown.</param>
    /// <param name="culture">IETF culture name (for example <c>"en-US"</c>).</param>
    /// <param name="resultKind">Tag for the produced result.</param>
    /// <param name="resultInt64">Populated when <paramref name="resultKind"/> is Int64.</param>
    /// <param name="resultDouble">Populated when <paramref name="resultKind"/> is Double.</param>
    /// <param name="resultBoolean">Populated (0/1) when <paramref name="resultKind"/> is Boolean.</param>
    /// <param name="resultString">
    /// Populated when <paramref name="resultKind"/> is String. Owned by the
    /// caller once returned; free with <see cref="Marshal.FreeCoTaskMem"/>.
    /// </param>
    /// <param name="error">
    /// Optional human-readable failure message, set only when the returned
    /// HRESULT is negative. Owned by the caller once returned; free with
    /// <see cref="Marshal.FreeCoTaskMem"/>.
    /// </param>
    [PreserveSig]
    int Convert(
        int converterId,
        int direction,
        int valueKind,
        long valueInt64,
        double valueDouble,
        int valueBoolean,
        string? valueString,
        int parameterKind,
        long parameterInt64,
        double parameterDouble,
        int parameterBoolean,
        string? parameterString,
        int targetKind,
        string? culture,
        out int resultKind,
        out long resultInt64,
        out double resultDouble,
        out int resultBoolean,
        out nint resultString,
        out nint error);
}
