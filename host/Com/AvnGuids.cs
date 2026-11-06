namespace Avalonia.Host.Com;

internal static class AvnGuids
{
    public const string IUnknown = "00000000-0000-0000-C000-000000000046";
    public const string IAvnActivationFactory = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D10";
    public const string IAvnEcho = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D11";
    public const string IAvnApplication = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D23";
    public const string IAvnAppHandler = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D13";
    public const string IAvnDispatcher = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D14";
    public const string IAvnAction = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D15";
    public const string IAvnResourceValue = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D16";
    public const string IAvnAsyncCompletion = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D17";

    // Desktop file integration.
    public const string IAvnFilePickerOptions = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D51";
    public const string IAvnStorageItem = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D52";
    public const string IAvnStorageItemList = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D53";
    public const string IAvnStorageCompletion = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D54";
    public const string IAvnFileDropHandler = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D55";
    public const string IAvnActivationHandler = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D56";

    // Clipboard payloads.
    public const string IAvnClipboardData = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D61";

    // Rust-drawn surfaces and code-first tray icons.
    public const string IAvnPixelSurface = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D91";
    public const string IAvnPixelSurfaceHandler = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D92";
    public const string IAvnShellTray = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D93";
    public const string IAvnShellTrayHandler = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D94";

    // Native-window callbacks.
    public const string IAvnNativePopupHandler = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DA3";
    public const string IAvnTerminationHandler = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DA5";
    public const string IAvnFileDragHandler = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DA7";

    // Code-first native menus.
    public const string IAvnNativeMenu = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DAB";
    public const string IAvnNativeMenuHandler = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4DAC";
}
