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

    // Stage 29 desktop file integration. These are a separately versioned
    // capability: nothing below is ever added to an already published vtable.
    public const string IAvnApplication3 = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D50";
    public const string IAvnFilePickerOptions = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D51";
    public const string IAvnStorageItem = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D52";
    public const string IAvnStorageItemList = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D53";
    public const string IAvnStorageCompletion = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D54";
    public const string IAvnFileDropHandler = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D55";
    public const string IAvnActivationHandler = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D56";

    // Stage 31 clipboard commands. Another separately versioned capability;
    // the stage 29 storage snapshot interfaces are reused as-is for reading
    // file entries back, and nothing below is added to a published vtable.
    public const string IAvnApplication4 = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D60";
    public const string IAvnClipboardData = "6B2E8F10-4C91-4E3A-9A77-1F0C2B3A4D61";
}
