using System;
using System.Collections.Generic;
using System.Text.Json;
using Avalonia;
using Avalonia.Animation;
using Avalonia.Controls;
using Avalonia.Controls.Presenters;
using Avalonia.Controls.Primitives;
using Avalonia.Input;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Media.Immutable;
using Avalonia.Styling;
using Avalonia.Threading;

namespace Avalonia.Host.Com;

public partial class AvnApplication
{
    private readonly Dictionary<long, (IStyleHost Host, Styles Sheet)> _registeredStyles = new();
    private readonly Dictionary<long, (Window Window, KeyBinding Binding)> _windowKeyBindings = new();
    private long _nextStyleOrKeyId = 1;

    public int ApplyStyles(IAvnControl? scopeControl, string? rulesJson, out long styleHandle)
    {
        styleHandle = 0;
        if (rulesJson is null)
            return HResults.E_POINTER;

        try
        {
            Dispatcher.UIThread.VerifyAccess();
            IStyleHost styleHost;
            if (scopeControl is not null)
            {
                if (ProjectionRuntime.Unwrap(scopeControl) is not Control control)
                    return HResults.E_INVALIDARG;
                styleHost = control;
            }
            else
            {
                if (Application.Current is not { } app)
                    return HResults.E_FAIL;
                styleHost = app;
            }

            var sheet = ParseStyleSheet(rulesJson);
            styleHost.Styles.Add(sheet);
            var id = _nextStyleOrKeyId++;
            _registeredStyles[id] = (styleHost, sheet);
            styleHandle = id;
            return HResults.S_OK;
        }
        catch (JsonException e)
        {
            return AbiError.Capture(new ArgumentException(e.Message, e));
        }
        catch (FormatException e)
        {
            return AbiError.Capture(new ArgumentException(e.Message, e));
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int RemoveStyles(long styleHandle)
    {
        try
        {
            Dispatcher.UIThread.VerifyAccess();
            if (!_registeredStyles.Remove(styleHandle, out var entry))
                return HResults.E_INVALIDARG;
            entry.Host.Styles.Remove(entry.Sheet);
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int AddWindowKeyBinding(
        IAvnWindow? window,
        string? gesture,
        IAvnCommand? command,
        AvnVariant parameter,
        out long bindingId)
    {
        bindingId = 0;
        if (window is null || gesture is null || command is null)
            return HResults.E_POINTER;

        try
        {
            Dispatcher.UIThread.VerifyAccess();
            if (ProjectionRuntime.Unwrap(window) is not Window managedWindow)
                return HResults.E_INVALIDARG;
            if (string.IsNullOrWhiteSpace(gesture))
                return HResults.E_INVALIDARG;

            var parsedGesture = KeyGesture.Parse(gesture);
            var managedCommand = AvnCommand.ToCommand(command);
            if (managedCommand is null)
                return HResults.E_POINTER;

            var keyBinding = new KeyBinding
            {
                Gesture = parsedGesture,
                Command = managedCommand,
            };
            if (parameter.ToObject() is { } paramObj)
                keyBinding.CommandParameter = paramObj;
            managedWindow.KeyBindings.Add(keyBinding);
            var id = _nextStyleOrKeyId++;
            _windowKeyBindings[id] = (managedWindow, keyBinding);
            bindingId = id;
            return HResults.S_OK;
        }
        catch (FormatException e)
        {
            return AbiError.Capture(new ArgumentException(e.Message, e));
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    public int RemoveWindowKeyBinding(IAvnWindow? window, long bindingId)
    {
        if (window is null)
            return HResults.E_POINTER;

        try
        {
            Dispatcher.UIThread.VerifyAccess();
            if (ProjectionRuntime.Unwrap(window) is not Window managedWindow)
                return HResults.E_INVALIDARG;
            if (!_windowKeyBindings.TryGetValue(bindingId, out var entry) || !ReferenceEquals(entry.Window, managedWindow))
                return HResults.E_INVALIDARG;

            _windowKeyBindings.Remove(bindingId);
            managedWindow.KeyBindings.Remove(entry.Binding);
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
    }

    private static Styles ParseStyleSheet(string rulesJson)
    {
        using var doc = JsonDocument.Parse(rulesJson);
        var root = doc.RootElement;
        if (!root.TryGetProperty("rules", out var rulesEl) || rulesEl.ValueKind != JsonValueKind.Array)
            throw new FormatException("StyleSheet JSON must contain a 'rules' array.");

        var sheet = new Styles();
        foreach (var ruleEl in rulesEl.EnumerateArray())
        {
            var target = ruleEl.TryGetProperty("target", out var tEl) && tEl.ValueKind == JsonValueKind.String
                ? tEl.GetString()
                : null;
            var selector = BuildTargetSelector(target);
            var hasInteractivePseudo = false;

            if (ruleEl.TryGetProperty("classes", out var classesEl) && classesEl.ValueKind == JsonValueKind.Array)
            {
                foreach (var cls in classesEl.EnumerateArray())
                {
                    var name = cls.GetString();
                    if (!string.IsNullOrWhiteSpace(name))
                        selector = selector.Class(name);
                }
            }

            if (ruleEl.TryGetProperty("pseudos", out var pseudosEl) && pseudosEl.ValueKind == JsonValueKind.Array)
            {
                foreach (var pseudo in pseudosEl.EnumerateArray())
                {
                    var name = pseudo.GetString();
                    if (!string.IsNullOrWhiteSpace(name))
                    {
                        var normalized = name.StartsWith(':') ? name : ":" + name;
                        if (normalized is ":pointerover" or ":pressed" or ":disabled")
                            hasInteractivePseudo = true;
                        selector = selector.Class(normalized);
                    }
                }
            }

            var baseSelector = selector;
            var style = new Style(_ => baseSelector);
            List<Setter>? presenterSetters = IsButtonFamily(target) && hasInteractivePseudo ? new List<Setter>() : null;

            if (ruleEl.TryGetProperty("setters", out var settersEl) && settersEl.ValueKind == JsonValueKind.Array)
            {
                foreach (var setterEl in settersEl.EnumerateArray())
                {
                    AddStyleSetters(style, presenterSetters, target, setterEl);
                }
            }

            if (ruleEl.TryGetProperty("transitions", out var transEl) && transEl.ValueKind == JsonValueKind.Array)
            {
                var transitions = new Transitions();
                foreach (var trEl in transEl.EnumerateArray())
                {
                    var transition = ParseTransition(target, trEl);
                    if (transition is not null)
                        transitions.Add(transition);
                }
                if (transitions.Count > 0)
                    style.Setters.Add(new Setter(Animatable.TransitionsProperty, transitions));
            }

            sheet.Add(style);

            if (presenterSetters is { Count: > 0 })
            {
                var templateSelector = baseSelector.Template().OfType<ContentPresenter>().Name("PART_ContentPresenter");
                var presenterStyle = new Style(_ => templateSelector);
                foreach (var s in presenterSetters)
                    presenterStyle.Setters.Add(s);
                sheet.Add(presenterStyle);
            }
        }

        return sheet;
    }

    private static bool IsButtonFamily(string? target) =>
        target is "Button" or "RepeatButton" or "ToggleButton" or "DropDownButton";

    private static Selector BuildTargetSelector(string? target) =>
        target switch
        {
            null or "" or "Control" => Selectors.Is<Control>(null),
            "Button" => Selectors.OfType<Button>(null),
            "RepeatButton" => Selectors.OfType<RepeatButton>(null),
            "DropDownButton" => Selectors.OfType<DropDownButton>(null),
            "SplitButton" => Selectors.OfType<SplitButton>(null),
            "ToggleButton" => Selectors.OfType<ToggleButton>(null),
            "CheckBox" => Selectors.OfType<CheckBox>(null),
            "RadioButton" => Selectors.OfType<RadioButton>(null),
            "ToggleSwitch" => Selectors.OfType<ToggleSwitch>(null),
            "Border" => Selectors.OfType<Border>(null),
            "TextBlock" => Selectors.OfType<TextBlock>(null),
            "SelectableTextBlock" => Selectors.OfType<SelectableTextBlock>(null),
            "TextBox" => Selectors.OfType<TextBox>(null),
            "MaskedTextBox" => Selectors.OfType<MaskedTextBox>(null),
            "ComboBox" => Selectors.OfType<ComboBox>(null),
            "ComboBoxItem" => Selectors.OfType<ComboBoxItem>(null),
            "ListBox" => Selectors.OfType<ListBox>(null),
            "ListBoxItem" => Selectors.OfType<ListBoxItem>(null),
            "TabControl" => Selectors.OfType<TabControl>(null),
            "TabItem" => Selectors.OfType<TabItem>(null),
            "TreeView" => Selectors.OfType<TreeView>(null),
            "TreeViewItem" => Selectors.OfType<TreeViewItem>(null),
            "Expander" => Selectors.OfType<Expander>(null),
            "MenuItem" => Selectors.OfType<MenuItem>(null),
            "Slider" => Selectors.OfType<Slider>(null),
            "ProgressBar" => Selectors.OfType<ProgressBar>(null),
            "NumericUpDown" => Selectors.OfType<NumericUpDown>(null),
            "ScrollViewer" => Selectors.OfType<ScrollViewer>(null),
            "StackPanel" => Selectors.OfType<StackPanel>(null),
            "Grid" => Selectors.OfType<Grid>(null),
            "DockPanel" => Selectors.OfType<DockPanel>(null),
            "WrapPanel" => Selectors.OfType<WrapPanel>(null),
            "Panel" => Selectors.Is<Panel>(null),
            "TemplatedControl" => Selectors.Is<TemplatedControl>(null),
            "Window" => Selectors.Is<Window>(null),
            _ => throw new FormatException($"Unsupported style target type '{target}'."),
        };

    private static void AddStyleSetters(
        Style style,
        List<Setter>? presenterSetters,
        string? target,
        JsonElement setterEl)
    {
        var prop = setterEl.GetProperty("prop").GetString()
            ?? throw new FormatException("Style setter missing 'prop'.");
        var val = setterEl.GetProperty("value");

        switch (prop)
        {
            case "background":
            {
                var brush = ParsePaintJson(val);
                if (target == "Border")
                    style.Setters.Add(new Setter(Border.BackgroundProperty, brush));
                else if (target is "Panel" or "StackPanel" or "Grid" or "DockPanel" or "WrapPanel")
                    style.Setters.Add(new Setter(Panel.BackgroundProperty, brush));
                else if (target is "TextBlock" or "SelectableTextBlock")
                    style.Setters.Add(new Setter(TextBlock.BackgroundProperty, brush));
                else
                    style.Setters.Add(new Setter(TemplatedControl.BackgroundProperty, brush));

                presenterSetters?.Add(new Setter(ContentPresenter.BackgroundProperty, brush));
                break;
            }
            case "foreground":
            {
                var brush = ParsePaintJson(val);
                if (target is "TextBlock" or "SelectableTextBlock")
                    style.Setters.Add(new Setter(TextBlock.ForegroundProperty, brush));
                else
                    style.Setters.Add(new Setter(TemplatedControl.ForegroundProperty, brush));

                presenterSetters?.Add(new Setter(ContentPresenter.ForegroundProperty, brush));
                break;
            }
            case "borderBrush":
            {
                var brush = ParsePaintJson(val);
                if (target == "Border")
                    style.Setters.Add(new Setter(Border.BorderBrushProperty, brush));
                else
                    style.Setters.Add(new Setter(TemplatedControl.BorderBrushProperty, brush));

                presenterSetters?.Add(new Setter(ContentPresenter.BorderBrushProperty, brush));
                break;
            }
            case "borderThickness":
            {
                var thickness = ParseThicknessJson(val);
                if (target == "Border")
                    style.Setters.Add(new Setter(Border.BorderThicknessProperty, thickness));
                else
                    style.Setters.Add(new Setter(TemplatedControl.BorderThicknessProperty, thickness));

                presenterSetters?.Add(new Setter(ContentPresenter.BorderThicknessProperty, thickness));
                break;
            }
            case "cornerRadius":
            {
                var cr = ParseCornerRadiusJson(val);
                if (target == "Border")
                    style.Setters.Add(new Setter(Border.CornerRadiusProperty, cr));
                else
                    style.Setters.Add(new Setter(TemplatedControl.CornerRadiusProperty, cr));

                presenterSetters?.Add(new Setter(ContentPresenter.CornerRadiusProperty, cr));
                break;
            }
            case "padding":
            {
                var thickness = ParseThicknessJson(val);
                if (target == "Border")
                    style.Setters.Add(new Setter(Decorator.PaddingProperty, thickness));
                else if (target is "TextBlock" or "SelectableTextBlock")
                    style.Setters.Add(new Setter(TextBlock.PaddingProperty, thickness));
                else
                    style.Setters.Add(new Setter(TemplatedControl.PaddingProperty, thickness));
                break;
            }
            case "margin":
                style.Setters.Add(new Setter(Layoutable.MarginProperty, ParseThicknessJson(val)));
                break;
            case "opacity":
                style.Setters.Add(new Setter(Visual.OpacityProperty, val.GetDouble()));
                break;
            case "fontSize":
                if (target is "TextBlock" or "SelectableTextBlock")
                    style.Setters.Add(new Setter(TextBlock.FontSizeProperty, val.GetDouble()));
                else
                    style.Setters.Add(new Setter(TemplatedControl.FontSizeProperty, val.GetDouble()));
                break;
            case "fontWeight":
            {
                var weight = (FontWeight)val.GetInt32();
                if (target is "TextBlock" or "SelectableTextBlock")
                    style.Setters.Add(new Setter(TextBlock.FontWeightProperty, weight));
                else
                    style.Setters.Add(new Setter(TemplatedControl.FontWeightProperty, weight));
                break;
            }
            case "boxShadow":
                style.Setters.Add(new Setter(Border.BoxShadowProperty, AvnBoxShadows.FromAbi(val.GetString())));
                break;
            case "width":
                style.Setters.Add(new Setter(Layoutable.WidthProperty, val.GetDouble()));
                break;
            case "height":
                style.Setters.Add(new Setter(Layoutable.HeightProperty, val.GetDouble()));
                break;
            case "minWidth":
                style.Setters.Add(new Setter(Layoutable.MinWidthProperty, val.GetDouble()));
                break;
            case "minHeight":
                style.Setters.Add(new Setter(Layoutable.MinHeightProperty, val.GetDouble()));
                break;
            case "maxWidth":
                style.Setters.Add(new Setter(Layoutable.MaxWidthProperty, val.GetDouble()));
                break;
            case "maxHeight":
                style.Setters.Add(new Setter(Layoutable.MaxHeightProperty, val.GetDouble()));
                break;
            default:
                throw new FormatException($"Unsupported style property '{prop}'.");
        }
    }

    private static ITransition? ParseTransition(string? target, JsonElement trEl)
    {
        var prop = trEl.GetProperty("prop").GetString();
        var durationMs = trEl.GetProperty("durationMs").GetInt64();
        var duration = TimeSpan.FromMilliseconds(Math.Max(0, durationMs));

        return prop switch
        {
            "opacity" => new DoubleTransition { Property = Visual.OpacityProperty, Duration = duration },
            "width" => new DoubleTransition { Property = Layoutable.WidthProperty, Duration = duration },
            "height" => new DoubleTransition { Property = Layoutable.HeightProperty, Duration = duration },
            "background" => new BrushTransition
            {
                Property = target == "Border"
                    ? Border.BackgroundProperty
                    : target is "Panel" or "StackPanel" or "Grid" or "DockPanel" or "WrapPanel"
                        ? Panel.BackgroundProperty
                        : TemplatedControl.BackgroundProperty,
                Duration = duration,
            },
            "foreground" => new BrushTransition
            {
                Property = target is "TextBlock" or "SelectableTextBlock"
                    ? TextBlock.ForegroundProperty
                    : TemplatedControl.ForegroundProperty,
                Duration = duration,
            },
            "borderBrush" => new BrushTransition
            {
                Property = target == "Border" ? Border.BorderBrushProperty : TemplatedControl.BorderBrushProperty,
                Duration = duration,
            },
            "borderThickness" => new ThicknessTransition
            {
                Property = target == "Border" ? Border.BorderThicknessProperty : TemplatedControl.BorderThicknessProperty,
                Duration = duration,
            },
            "padding" => new ThicknessTransition
            {
                Property = target == "Border" ? Decorator.PaddingProperty : TemplatedControl.PaddingProperty,
                Duration = duration,
            },
            "margin" => new ThicknessTransition { Property = Layoutable.MarginProperty, Duration = duration },
            "cornerRadius" => new CornerRadiusTransition
            {
                Property = target == "Border" ? Border.CornerRadiusProperty : TemplatedControl.CornerRadiusProperty,
                Duration = duration,
            },
            "boxShadow" => new BoxShadowsTransition { Property = Border.BoxShadowProperty, Duration = duration },
            _ => throw new FormatException($"Unsupported transition property '{prop}'."),
        };
    }

    private static IBrush? ParsePaintJson(JsonElement el)
    {
        if (el.ValueKind == JsonValueKind.Null)
            return null;
        var kind = el.GetProperty("kind").GetString();
        return kind switch
        {
            "solid" => new ImmutableSolidColorBrush(
                Color.FromUInt32(el.GetProperty("argb").GetUInt32()),
                el.GetProperty("opacity").GetDouble()),
            "linear" => new ImmutableLinearGradientBrush(
                ParseStopsJson(el.GetProperty("stops")),
                el.GetProperty("opacity").GetDouble(),
                transform: null,
                transformOrigin: null,
                (GradientSpreadMethod)el.GetProperty("spread").GetInt32(),
                ParseRelativePointJson(el.GetProperty("start")),
                ParseRelativePointJson(el.GetProperty("end"))),
            "radial" => new ImmutableRadialGradientBrush(
                ParseStopsJson(el.GetProperty("stops")),
                el.GetProperty("opacity").GetDouble(),
                transform: null,
                transformOrigin: null,
                (GradientSpreadMethod)el.GetProperty("spread").GetInt32(),
                ParseRelativePointJson(el.GetProperty("center")),
                ParseRelativePointJson(el.GetProperty("origin")),
                ParseRelativeScalarJson(el.GetProperty("radiusX")),
                ParseRelativeScalarJson(el.GetProperty("radiusY"))),
            _ => throw new FormatException($"Unsupported paint kind '{kind}'."),
        };
    }

    private static ImmutableGradientStop[] ParseStopsJson(JsonElement stopsEl)
    {
        var list = new List<ImmutableGradientStop>();
        foreach (var stop in stopsEl.EnumerateArray())
        {
            list.Add(new ImmutableGradientStop(
                stop[0].GetDouble(),
                Color.FromUInt32(stop[1].GetUInt32())));
        }
        return list.ToArray();
    }

    private static RelativePoint ParseRelativePointJson(JsonElement pt) =>
        new(pt[0].GetDouble(), pt[1].GetDouble(), (RelativeUnit)pt[2].GetInt32());

    private static RelativeScalar ParseRelativeScalarJson(JsonElement sc) =>
        new(sc[0].GetDouble(), (RelativeUnit)sc[1].GetInt32());

    private static Thickness ParseThicknessJson(JsonElement arr) =>
        new(arr[0].GetDouble(), arr[1].GetDouble(), arr[2].GetDouble(), arr[3].GetDouble());

    private static CornerRadius ParseCornerRadiusJson(JsonElement arr) =>
        new(arr[0].GetDouble(), arr[1].GetDouble(), arr[2].GetDouble(), arr[3].GetDouble());
}
