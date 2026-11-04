using Android.App;
using Android.Content.PM;
using Android.Runtime;
using Avalonia.Host.Android;

// rustolonia-apk replaces this placeholder with the app's library name.
[assembly: MetaData(RustoloniaAndroidApplication.LibraryMetaDataName, Value = "rustolonia_app")]

namespace Rustolonia.Android.HostApp;

/// <summary>
/// The generic application of the prebuilt host APK. It starts the Rust
/// library named by the <c>net.devolutions.rustolonia.library</c> meta-data.
/// </summary>
[Application(Name = "net.devolutions.rustolonia.android.RustoloniaApplication", Label = "Rustolonia")]
public sealed class RustoloniaApplication : RustoloniaAndroidApplication
{
    public RustoloniaApplication(nint javaReference, JniHandleOwnership transfer)
        : base(javaReference, transfer)
    {
    }
}

/// <summary>The generic launcher activity of the prebuilt host APK.</summary>
[Activity(
    Name = "net.devolutions.rustolonia.android.RustoloniaActivity",
    Label = "Rustolonia",
    Theme = "@style/RustoloniaTheme",
    MainLauncher = true,
    Exported = true,
    ConfigurationChanges = ConfigChanges.Orientation | ConfigChanges.ScreenSize | ConfigChanges.UiMode
        | ConfigChanges.ScreenLayout | ConfigChanges.SmallestScreenSize | ConfigChanges.Keyboard
        | ConfigChanges.KeyboardHidden | ConfigChanges.Density)]
public sealed class RustoloniaActivity : RustoloniaMainActivity
{
}
