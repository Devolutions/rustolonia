using System;
using System.Runtime.InteropServices;
using Android.OS;
using Android.Runtime;
using Avalonia.Android;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Platform;

namespace Avalonia.Host.Android;

/// <summary>
/// Base <c>android.app.Application</c> for a Rustolonia APK.
/// </summary>
/// <remarks>
/// On Android the process belongs to the Android runtime, so the host starts
/// first and runs the Rust application instead of being loaded by it:
/// <list type="number">
/// <item>Avalonia is set up for Android (Skia, HarfBuzz, Fluent theme) and
/// <c>new Window()</c> is redirected to logical windows.</item>
/// <item>The application's Rust <c>cdylib</c> (<see cref="RustLibraryName"/>)
/// is loaded and its <c>rustolonia_android_main</c> export, declared with
/// <c>rustolonia::android_main!</c>, is called on the UI thread with the
/// host's <see cref="HostExportsTable"/>.</item>
/// <item>The Rust code builds its UI through the usual ABI; the windows it
/// shows are presented by the activity (<see cref="RustoloniaMainActivity"/>).</item>
/// </list>
/// An application declares a concrete subclass with <c>[Application]</c>.
/// </remarks>
public abstract class RustoloniaAndroidApplication : AvaloniaAndroidApplication<HostApplication>
{
    internal const string LogTag = "rustolonia";

    protected RustoloniaAndroidApplication(nint javaReference, JniHandleOwnership transfer)
        : base(javaReference, transfer)
    {
    }

    /// <summary>
    /// The Rust library to start, without the <c>lib</c> prefix and
    /// <c>.so</c> suffix: a crate named <c>my_app</c> builds <c>libmy_app.so</c>.
    /// </summary>
    protected abstract string RustLibraryName { get; }

    protected override AppBuilder CustomizeAppBuilder(AppBuilder builder) =>
        base.CustomizeAppBuilder(builder).AfterPlatformServicesSetup(_ =>
        {
            var stub = AvaloniaLocator.Current.GetService<IWindowingPlatform>();
            AvaloniaLocator.CurrentMutable
                .Bind<IWindowingPlatform>()
                .ToConstant(new LogicalWindowingPlatform(stub));
        });

    public override void OnCreate()
    {
        // Projected calls report managed exceptions to Rust only as HRESULTs.
        // `adb shell setprop log.tag.rustolonia DEBUG` logs each one as it is thrown.
        if (global::Android.Util.Log.IsLoggable(LogTag, global::Android.Util.LogPriority.Debug))
        {
            AppDomain.CurrentDomain.FirstChanceException += (_, e) =>
                global::Android.Util.Log.Debug(LogTag, $"first-chance exception: {e.Exception}");
        }
        base.OnCreate();
        if (Avalonia.Application.Current?.ApplicationLifetime is IActivityApplicationLifetime lifetime)
            lifetime.MainViewFactory = () => AndroidWindowPresenter.Root;
        StartRust();
    }

    private unsafe void StartRust()
    {
        var name = RustLibraryName;
        int result;
        try
        {
            // System.loadLibrary resolves the APK's native library directory
            // (including uncompressed, unextracted libraries); dlopen by
            // soname then returns that same, already loaded, library.
            Java.Lang.JavaSystem.LoadLibrary(name);
            var library = NativeLibrary.Load($"lib{name}.so");
            var entry = (delegate* unmanaged<HostExportsTable*, int>)
                NativeLibrary.GetExport(library, "rustolonia_android_main");
            var exports = HostExportsTable.Create();
            result = entry(&exports);
        }
        catch (Exception e)
        {
            result = AbiError.Capture(e);
        }

        if (result >= 0)
            return;

        var message = $"lib{name}.so failed to start (HRESULT 0x{result:X8}). " +
                      (AbiError.Take() ?? "See `adb logcat -s rustolonia` for the Rust error.");
        global::Android.Util.Log.Error(LogTag, message);
        AndroidWindowPresenter.Root.Content = new TextBlock
        {
            Text = message,
            Margin = new Thickness(24),
            TextWrapping = Media.TextWrapping.Wrap,
        };
    }
}

/// <summary>
/// Base launcher activity for a Rustolonia APK: hosts the windows the Rust
/// application shows and routes Back to closing the top window. An
/// application declares a concrete subclass with
/// <c>[Activity(MainLauncher = true, ...)]</c>.
/// </summary>
public abstract class RustoloniaMainActivity : AvaloniaMainActivity
{
    protected override void OnCreate(Bundle? savedInstanceState)
    {
        base.OnCreate(savedInstanceState);
        AndroidWindowPresenter.AttachActivity(this);
    }

    protected override void OnResume()
    {
        base.OnResume();
        AndroidWindowPresenter.AttachActivity(this);
    }

    protected override void OnDestroy()
    {
        AndroidWindowPresenter.DetachActivity(this);
        base.OnDestroy();
    }
}
