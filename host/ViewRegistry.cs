using System;
using Avalonia.Controls;
using Avalonia.Rust.Interop;

namespace Avalonia.Host.Generated.ViewModels;

/// <summary>
/// Handwritten fallback used when no application-owned view registry is
/// supplied. Sample and other apps replace this by setting
/// <c>AvaloniaRustViewRegistryFile</c> to their generated registry.
/// </summary>
internal static class RustViewRegistry
{
    internal static Window Create(int viewId, IAvnRustViewModel model)
    {
        ArgumentNullException.ThrowIfNull(model);
        throw new InvalidOperationException(
            $"No view is registered for view id {viewId}. Publish or compile Avalonia.Host with AvaloniaRustViewRegistryFile (application-owned generated registry) and AvaloniaRustPresentationProjects (application AXAML project).");
    }
}
