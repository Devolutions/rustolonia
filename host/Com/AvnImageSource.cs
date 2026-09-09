using System;
using System.IO;
using System.Runtime.CompilerServices;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.Platform;

namespace Avalonia.Host.Com;

/// <summary>
/// Projects <c>Avalonia.Controls.Image.Source</c> as the source string the host resolves into a
/// bitmap, because <see cref="IImage"/> is a managed interface with no ABI shape of its own.
/// </summary>
/// <remarks>
/// <para>
/// What crosses the ABI is a <b>string</b>, not an image object. Four spellings are accepted:
/// an absolute filesystem path, a path relative to the process working directory, a
/// <c>file://</c> URI, and an <c>avares://</c> or <c>resm:</c> URI resolved through Avalonia's
/// asset loader. Any other scheme — <c>http</c> included — fails with
/// <see cref="NotSupportedException"/> rather than silently doing nothing: fetching over the
/// network is the caller's job, and a later wave can add a byte-buffer entry point for it.
/// </para>
/// <para>
/// Reading is deliberately narrow. A <see cref="Bitmap"/> does not remember where it came from,
/// so this converter remembers instead: the string that produced an image is kept in a weak
/// table and handed back on read. An image the ABI never set — one that came from XAML, from a
/// style, or from managed code — reads back as <c>null</c> even though the control is drawing it.
/// </para>
/// </remarks>
internal static class AvnImageSource
{
    private static readonly ConditionalWeakTable<IImage, string> s_sources = new();

    /// <summary>
    /// The source string that produced <paramref name="value"/>, or <c>null</c> when the image
    /// did not come across this ABI.
    /// </summary>
    public static string? ToAbi(IImage? value) =>
        value is not null && s_sources.TryGetValue(value, out var source) ? source : null;

    /// <summary>Loads the image named by <paramref name="value"/>, or <c>null</c> when it is empty.</summary>
    public static IImage? FromAbi(string? value)
    {
        if (string.IsNullOrEmpty(value))
            return null;

        var image = Load(value);
        s_sources.AddOrUpdate(image, value);
        return image;
    }

    private static IImage Load(string source)
    {
        if (!Uri.TryCreate(source, UriKind.Absolute, out var uri))
            return new Bitmap(Path.GetFullPath(source));
        if (uri.IsFile)
            return new Bitmap(uri.LocalPath);
        if (uri.Scheme is "avares" or "resm")
        {
            using var stream = AssetLoader.Open(uri);
            return new Bitmap(stream);
        }

        throw new NotSupportedException(
            $"Image source '{source}' uses scheme '{uri.Scheme}', which the projection does not " +
            "resolve. Use a file path, a file:// URI, or an avares:// or resm: URI.");
    }
}
