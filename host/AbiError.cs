using System;

namespace Avalonia.Host;

internal static class AbiError
{
    [ThreadStatic]
    private static string? s_lastError;

    internal static int Capture(Exception exception)
    {
        s_lastError = exception.ToString();
        return System.Runtime.InteropServices.Marshal.GetHRForException(exception);
    }

    internal static string? Take()
    {
        var value = s_lastError;
        s_lastError = null;
        return value;
    }
}
