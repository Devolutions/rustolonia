using System.Linq;
using Avalonia.Host.Com;
using Xunit;

namespace Avalonia.Host.Tests;

public class ApplicationInterfaceTests
{
    // rustolonia-sys's IAvnApplicationVtbl declares these slots in this order.
    // Reordering or inserting methods is fine (host and crates ship together),
    // but both sides have to change in the same commit.
    private static readonly string[] s_slots =
    [
        nameof(IAvnApplication.Run),
        nameof(IAvnApplication.Shutdown),
        nameof(IAvnApplication.GetRequestedThemeVariant),
        nameof(IAvnApplication.SetRequestedThemeVariant),
        nameof(IAvnApplication.GetActualThemeVariant),
        nameof(IAvnApplication.TryGetResource),
        nameof(IAvnApplication.StartDelay),
        nameof(IAvnApplication.StartClipboardSetText),
        nameof(IAvnApplication.StartClipboardGetText),
        nameof(IAvnApplication.CancelAsyncOperation),
        nameof(IAvnApplication.CreateRustVmWindow),
        nameof(IAvnApplication.SetValueConverterProvider),
        nameof(IAvnApplication.CreatePickerOptions),
        nameof(IAvnApplication.GetStorageCapabilities),
        nameof(IAvnApplication.StartOpenFilePicker),
        nameof(IAvnApplication.StartOpenFolderPicker),
        nameof(IAvnApplication.StartSaveFilePicker),
        nameof(IAvnApplication.SubscribeFileDrop),
        nameof(IAvnApplication.UnsubscribeFileDrop),
        nameof(IAvnApplication.ClearStartupArguments),
        nameof(IAvnApplication.AddStartupArgument),
        nameof(IAvnApplication.GetStartupArgumentCount),
        nameof(IAvnApplication.GetStartupArgument),
        nameof(IAvnApplication.GetActivationItems),
        nameof(IAvnApplication.AdviseActivation),
        nameof(IAvnApplication.UnadviseActivation),
        nameof(IAvnApplication.CreateClipboardData),
        nameof(IAvnApplication.GetClipboardCapabilities),
        nameof(IAvnApplication.StartClipboardWrite),
        nameof(IAvnApplication.StartClipboardClear),
        nameof(IAvnApplication.StartClipboardReadFiles),
        nameof(IAvnApplication.StartShowDialog),
        nameof(IAvnApplication.SetShutdownMode),
        nameof(IAvnApplication.CreateRustVmTray),
        nameof(IAvnApplication.GetPlatformHandle),
        nameof(IAvnApplication.SetWindowTopmost),
        nameof(IAvnApplication.SetWindowTransparent),
        nameof(IAvnApplication.SetWindowExcludedFromCapture),
        nameof(IAvnApplication.CreatePixelSurface),
        nameof(IAvnApplication.CreateTrayIcon),
        nameof(IAvnApplication.GetNativeWindowHandle),
        nameof(IAvnApplication.GetMacOSWindowNumber),
        nameof(IAvnApplication.SetMacOSWindowIgnoresMouseEvents),
        nameof(IAvnApplication.ShowWindow),
        nameof(IAvnApplication.ActivateWindow),
        nameof(IAvnApplication.SetWindowOverlayPopups),
        nameof(IAvnApplication.SetNativePopupGate),
        nameof(IAvnApplication.AcknowledgeNativePopup),
        nameof(IAvnApplication.SetTerminationHandler),
        nameof(IAvnApplication.CompleteTermination),
        nameof(IAvnApplication.SetFileDragHandler),
        nameof(IAvnApplication.CompleteFileDrag),
        nameof(IAvnApplication.CancelFileDrag),
        nameof(IAvnApplication.SetMainView),
        nameof(IAvnApplication.SetApplicationName),
        nameof(IAvnApplication.CreateNativeMenu),
        nameof(IAvnApplication.SetApplicationMenu),
        nameof(IAvnApplication.SetWindowMenu),
        nameof(IAvnApplication.IsNativeMenuExported),
        nameof(IAvnApplication.CreateNativeMenuBar),
        nameof(IAvnApplication.ApplyStyles),
        nameof(IAvnApplication.RemoveStyles),
        nameof(IAvnApplication.AddWindowKeyBinding),
        nameof(IAvnApplication.RemoveWindowKeyBinding),
    ];

    [Fact]
    public void Application_is_one_interface_in_rust_vtable_order()
    {
        Assert.Equal(s_slots, typeof(IAvnApplication).GetMethods().Select(m => m.Name));
        Assert.Equal(64, s_slots.Length);
    }
}
