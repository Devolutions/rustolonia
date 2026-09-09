using System.Text.Json.Serialization;

namespace Avalonia.Projection.Ir;

public sealed class ProjectedEvent
{
    public required string Name { get; init; }
    public required string HandlerInterfaceName { get; init; }
    public required string HandlerInterfaceIid { get; init; }
    public int HandlerInterfaceAbiVersion { get; init; } = 1;
    public required EventPayloadKind PayloadKind { get; init; }
    public string? ManagedHandlerTypeName { get; init; }

    /// <summary>
    /// For <see cref="EventPayloadKind.Args"/>: the fully-qualified name of the
    /// args interface the handler receives (e.g.
    /// <c>Avalonia.Host.Com.IAvnSelectionChangedArgs</c>). The <c>Parameters</c>
    /// list names the CLR event-args properties the interface exposes as
    /// <c>Get{Name}Count</c>/<c>Get{Name}At</c> Variant slots.
    /// </summary>
    public string? ArgsInterfaceName { get; init; }

    /// <summary>
    /// For <see cref="EventPayloadKind.Args"/>: the deterministic IID of the args
    /// interface.
    /// </summary>
    public string? ArgsInterfaceIid { get; init; }
    public IReadOnlyList<ProjectedParameter> Parameters { get; init; } = [];
}

[JsonConverter(typeof(JsonStringEnumConverter))]
public enum EventPayloadKind
{
    None,
    Fields,

    /// <summary>
    /// The handler receives a host-implemented args interface whose slots expose
    /// a collection payload as a count and per-index Variant getters.
    /// </summary>
    Args,
}
