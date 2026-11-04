using System;
using System.Diagnostics.CodeAnalysis;
using System.Runtime.InteropServices;
using System.Runtime.Versioning;

[assembly: SupportedOSPlatform("browser")]

namespace Avalonia.Host.Browser;

internal static partial class Program
{
    // Exports are called by the statically linked Rust app, not by managed
    // code, so they must be rooted against trimming for wasm-ld to resolve them.
    [DynamicDependency(DynamicallyAccessedMemberTypes.All, typeof(Exports))]
    public static int Main()
    {
        var result = rustolonia_main();
        if (result < 0)
            Console.Error.WriteLine($"rustolonia_main failed: 0x{result:X8}");
        return result;
    }

    /// <summary>
    /// Entry point of the Rust app linked from rustolonia_app.a. It loads the
    /// static host, builds its UI and calls App::run, which returns once the
    /// browser lifetime is started.
    /// </summary>
    [LibraryImport("rustolonia_app")]
    private static partial int rustolonia_main();
}
