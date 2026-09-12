using System.Text.Json;
using System.Text.Json.Serialization;

namespace Avalonia.Projection.Ir;

public sealed partial class ProjectionIr
{
    public const int MinimumVersion = 1;
    public const int CurrentVersion = 17;

    public int Version { get; init; } = CurrentVersion;
    public string? SourceAssembly { get; init; }
    public string? FactoryIid { get; init; }
    public int FactoryAbiVersion { get; init; } = 1;

    /// <summary>
    /// The projected solid-brush interface, present only when a member marshals as
    /// <see cref="MarshallingKind.Brush"/>.
    /// </summary>
    public string? BrushInterfaceName { get; init; }
    public string? BrushInterfaceIid { get; init; }
    public int BrushAbiVersion { get; init; } = 1;

    /// <summary>
    /// The projected gradient-brush capability interfaces (version 17), present only when a
    /// member marshals as <see cref="MarshallingKind.Brush"/>. Additive to
    /// <see cref="BrushInterfaceName"/>: the solid-brush interface, IID and vtable are unchanged.
    /// </summary>
    public string? GradientBrushInterfaceName { get; init; }
    public string? GradientBrushInterfaceIid { get; init; }
    public string? LinearGradientBrushInterfaceName { get; init; }
    public string? LinearGradientBrushInterfaceIid { get; init; }
    public string? RadialGradientBrushInterfaceName { get; init; }
    public string? RadialGradientBrushInterfaceIid { get; init; }

    /// <summary>
    /// The projected command interface, present only when a member marshals as
    /// <see cref="MarshallingKind.Command"/>.
    /// </summary>
    public string? CommandInterfaceName { get; init; }
    public string? CommandInterfaceIid { get; init; }
    public int CommandAbiVersion { get; init; } = 1;

    /// <summary>
    /// The CanExecuteChanged handler interface paired with the projected command,
    /// present only when a member marshals as <see cref="MarshallingKind.Command"/>.
    /// </summary>
    public string? CommandHandlerInterfaceName { get; init; }
    public string? CommandHandlerInterfaceIid { get; init; }

    /// <summary>
    /// The projected data-template interface, present only when a member marshals as
    /// <see cref="MarshallingKind.DataTemplate"/>.
    /// </summary>
    public string? TemplateInterfaceName { get; init; }
    public string? TemplateInterfaceIid { get; init; }

    /// <summary>
    /// The projected AutoCompleteBox filter interfaces, present only when a member
    /// marshals as <see cref="MarshallingKind.ItemFilter"/> or
    /// <see cref="MarshallingKind.TextFilter"/>.
    /// </summary>
    public string? ItemFilterInterfaceName { get; init; }
    public string? ItemFilterInterfaceIid { get; init; }
    public string? TextFilterInterfaceName { get; init; }
    public string? TextFilterInterfaceIid { get; init; }

    /// <summary>
    /// The projected notification interface, present only when a member marshals as
    /// <see cref="MarshallingKind.Notification"/>.
    /// </summary>
    public string? NotificationInterfaceName { get; init; }
    public string? NotificationInterfaceIid { get; init; }
    public string? NotificationHandlerInterfaceIid { get; init; }

    /// <summary>
    /// The projected AutoCompleteBox selector interfaces, present only when a member
    /// marshals as <see cref="MarshallingKind.ItemSelector"/> or
    /// <see cref="MarshallingKind.TextSelector"/>.
    /// </summary>
    public string? ItemSelectorInterfaceName { get; init; }
    public string? ItemSelectorInterfaceIid { get; init; }
    public string? TextSelectorInterfaceName { get; init; }
    public string? TextSelectorInterfaceIid { get; init; }

    /// <summary>
    /// The projected custom popup placement callback interface, present only when a
    /// member marshals as <see cref="MarshallingKind.PopupPlacement"/>.
    /// </summary>
    public string? PopupPlacementInterfaceName { get; init; }
    public string? PopupPlacementInterfaceIid { get; init; }

    /// <summary>
    /// The projected async populator interfaces, present only when a member marshals
    /// as <see cref="MarshallingKind.AsyncPopulator"/>.
    /// </summary>
    public string? AsyncPopulatorInterfaceName { get; init; }
    public string? AsyncPopulatorInterfaceIid { get; init; }
    public string? AsyncPopulatorCompletionInterfaceName { get; init; }
    public string? AsyncPopulatorCompletionInterfaceIid { get; init; }

    /// <summary>
    /// The projected dialog completion interface, present only when a method marshals
    /// its completion as <see cref="MarshallingKind.DialogCompletion"/>.
    /// </summary>
    public string? DialogCompletionInterfaceName { get; init; }
    public string? DialogCompletionInterfaceIid { get; init; }

    public IReadOnlyList<ProjectedType> Types { get; init; } = [];
    public IReadOnlyList<ProjectedEnum> Enums { get; init; } = [];
    public IReadOnlyList<ProjectedAttachedProperty> AttachedProperties { get; init; } = [];
    public IReadOnlyList<SkippedMember> Skipped { get; init; } = [];

    public static JsonSerializerOptions JsonOptions { get; } = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    public string ToJson()
    {
        Validate();
        return JsonSerializer.Serialize(this, JsonOptions);
    }

    public static ProjectionIr FromJson(string json)
    {
        var ir = JsonSerializer.Deserialize<ProjectionIr>(json, JsonOptions)
            ?? throw new InvalidOperationException("IR JSON deserialized to null.");
        ir.Validate();
        return ir;
    }
}
