using System;
using System.Globalization;
using Avalonia.Platform;

namespace Avalonia.Host;

internal sealed record LinuxDrmSettings(string? Card, double Scaling, SurfaceOrientation Orientation)
{
    internal static LinuxDrmSettings FromEnvironment(Func<string, string?> getVariable)
    {
        var card = getVariable("RUSTOLONIA_DRM_CARD");
        if (card is not null && (string.IsNullOrWhiteSpace(card) || !card.StartsWith("/dev/dri/card", StringComparison.Ordinal)))
            throw new ArgumentException("RUSTOLONIA_DRM_CARD must name a /dev/dri/card device.");

        var scaling = 1.0;
        if (getVariable("RUSTOLONIA_DRM_SCALING") is { } value &&
            (!double.TryParse(value, NumberStyles.Float, CultureInfo.InvariantCulture, out scaling) ||
             !double.IsFinite(scaling) || scaling <= 0))
            throw new ArgumentException("RUSTOLONIA_DRM_SCALING must be a finite positive number.");

        var orientation = getVariable("RUSTOLONIA_DRM_ROTATION") switch
        {
            null or "0" => SurfaceOrientation.Rotation0,
            "90" => SurfaceOrientation.Rotation90,
            "180" => SurfaceOrientation.Rotation180,
            "270" => SurfaceOrientation.Rotation270,
            _ => throw new ArgumentException("RUSTOLONIA_DRM_ROTATION must be 0, 90, 180, or 270."),
        };
        return new LinuxDrmSettings(card, scaling, orientation);
    }
}
