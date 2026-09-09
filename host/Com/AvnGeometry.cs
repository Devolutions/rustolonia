using System;
using Avalonia.Media;

namespace Avalonia.Host.Com;

/// <summary>
/// Projects <c>Path.Data</c> as the path mini-language string <see cref="Geometry.Parse"/>
/// already understands, because <see cref="Geometry"/> is a managed object graph with no ABI
/// shape of its own.
/// </summary>
internal static class AvnGeometry
{
    public static string? ToAbi(Geometry? value) => value?.ToString();

    public static Geometry? FromAbi(string? value) =>
        string.IsNullOrEmpty(value) ? null : Geometry.Parse(value);
}
