using Android.App;
using Android.Content.PM;
using Android.Runtime;
using Avalonia.Host.Android;

namespace AndroidHello;

/// <summary>Starts Avalonia, then the Rust library <c>libandroid_hello.so</c>.</summary>
[Application(Label = "Rustolonia Hello")]
public sealed class HelloApplication : RustoloniaAndroidApplication
{
    public HelloApplication(nint javaReference, JniHandleOwnership transfer)
        : base(javaReference, transfer)
    {
    }

    protected override string RustLibraryName => "android_hello";
}

/// <summary>Presents the windows the Rust application shows.</summary>
[Activity(
    Name = "net.devolutions.rustolonia.androidhello.MainActivity",
    Label = "Rustolonia Hello",
    Theme = "@style/RustoloniaTheme",
    MainLauncher = true,
    Exported = true,
    ConfigurationChanges = ConfigChanges.Orientation | ConfigChanges.ScreenSize | ConfigChanges.UiMode)]
public sealed class MainActivity : RustoloniaMainActivity
{
}
