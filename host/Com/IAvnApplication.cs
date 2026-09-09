using System.Runtime.InteropServices;
using System.Runtime.InteropServices.Marshalling;
using Avalonia.Rust.Interop;

namespace Avalonia.Host.Com;

[GeneratedComInterface(StringMarshalling = StringMarshalling.Utf16)]
[Guid(AvnGuids.IAvnApplication)]
public partial interface IAvnApplication
{
    [PreserveSig]
    int Run(IAvnAppHandler? handler);

    [PreserveSig]
    int Shutdown();

    [PreserveSig]
    int GetRequestedThemeVariant(out int value);

    [PreserveSig]
    int SetRequestedThemeVariant(int value);

    [PreserveSig]
    int GetActualThemeVariant(out int value);

    [PreserveSig]
    int TryGetResource(string? key, int themeVariant, out int found, out IAvnResourceValue? value);

    [PreserveSig]
    int StartDelay(int milliseconds, IAvnAsyncCompletion? completion, out long operationId);

    [PreserveSig]
    int StartClipboardSetText(
        IAvnWindow? window,
        string? text,
        IAvnAsyncCompletion? completion,
        out long operationId);

    [PreserveSig]
    int StartClipboardGetText(
        IAvnWindow? window,
        IAvnAsyncCompletion? completion,
        out long operationId);

    [PreserveSig]
    int CancelAsyncOperation(long operationId);

    [PreserveSig]
    int CreateRustVmWindow(int viewId, IAvnRustViewModel? model, out IAvnWindow? window);

}

[GeneratedComInterface]
[Guid("6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D41")]
public partial interface IAvnApplication2
{
    /// <summary>
    /// Registers (or clears, when <paramref name="provider"/> is null) the
    /// single application-scoped Rust value-converter provider. A second call
    /// with a different, still-registered provider is rejected.
    /// </summary>
    [PreserveSig]
    int SetValueConverterProvider(IAvnRustValueConverterProvider? provider);
}
