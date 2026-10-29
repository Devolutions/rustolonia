using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Linq;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Interactivity;
using Avalonia.Markup.Xaml;
using Avalonia.Media;
using Avalonia.Platform.Storage;
using Avalonia.Controls.Primitives;
using Avalonia.Rust.Interop;
using Avalonia.Threading;
using Jiffshot.Editor.Controls;
using Jiffshot.Editor.Generated;

namespace Jiffshot.Editor.Views;

public partial class MainWindow : Window
{
    private readonly MainViewModelAdapter _adapter;
    private readonly PixelCanvas _canvas;
    private readonly Dictionary<string, Button> _toolButtons = new();
    private IDisposable? _menu;
    private bool _syncing;
    private bool _syncingZoom;
    /// A tool picked from More, shown in place of the last tool that fits.
    private string? _extraTool;
    private int _toolCapacity = -1;
    private NativeMenu? _effectsMenu;
    private NativeMenu? _shareMenu;

    /// <summary>Tool palette: view-model tool name, icon glyph, caption.</summary>
    private static readonly (string Name, string Glyph, string Caption)[] Tools =
    {
        ("Select", "\uE7C9", "Select"),
        ("Arrow", "\uE72A", "Arrow"),
        ("Line", "\uE738", "Line"),
        ("Shape", "\uE739", "Shape"),
        ("Text", "\uE8D2", "Text"),
        ("Callout", "\uE8BD", "Callout"),
        ("Step", "①", "Step"),
        ("Pen", "\uE70F", "Pen"),
        ("Highlight", "\uE7E6", "Highlight"),
        ("Blur", "▦", "Blur"),
        ("Magnify", "\uE8A3", "Magnify"),
        ("Spotlight", "\uE706", "Spotlight"),
        ("Stamp", "\uE734", "Stamp"),
        ("Fill", "\uE790", "Fill"),
        ("Crop", "\uE7A8", "Crop"),
        ("Cut Out", "\uE8C6", "Cut Out"),
        ("Selection", "\uE8B3", "Selection"),
        ("Magic Wand", "✨", "Magic Wand"),
        ("Eraser", "\uE75C", "Eraser"),
        ("Eyedropper", "⌖", "Pick color"),
        ("Smart Move", "✥", "Smart Move"),
        ("Text Replace", "\uE8AC", "Text Replace"),
    };

    private static readonly string[] Palette =
    {
        "#FFE53935", "#FFFF9800", "#FFFFEB3B", "#FF43A047", "#FF1E88E5", "#FF1E6FD9", "#FF8E24AA",
        "#FFFFFFFF", "#FFBDBDBD", "#FF757575", "#FF22262E", "#FF000000",
    };

    /// The Effects panel's groups; effects not listed go under More.
    private static readonly (string Group, string[] Effects)[] EffectGroups =
    {
        ("Border", new[] { "Border" }),
        ("Edges", new[] { "Torn edge", "Torn bottom", "Sawtooth edge", "Wave edge", "Fade edges" }),
        ("Shadow", new[] { "Shadow" }),
        ("Perspective", new[] { "Perspective", "Tilt", "Page curl", "Reflection" }),
        ("Corners", new[] { "Rounded corners" }),
        ("Filters", new[] { "Sharpen", "Blur image", "Monochrome", "Grayscale", "Sepia", "Invert", "Vignette" }),
        ("Color adjustment", new[] { "Brighten", "Darken", "More contrast", "Less contrast", "Saturate", "Desaturate", "Hue shift", "Gamma up", "Gamma down" }),
        ("Color replacement", new[] { "Replace color" }),
        ("Size", new[] { "Scale to 1920" }),
    };

    private static readonly string[] Sections = { "zone", "tails", "variant", "backfill", "text", "font", "textopts", "textcolor", "stroke", "fill", "gradient", "width", "caps", "dash", "amount", "opacity", "rotation", "shadow" };

    public MainWindow()
    {
        InitializeComponent();
        _adapter = null!;
        _canvas = this.FindControl<PixelCanvas>("Canvas")!;
    }

    public MainWindow(IAvnRustViewModel model)
        : this()
    {
        _adapter = new MainViewModelAdapter(model);
        DataContext = _adapter;
        _menu = MainViewModelMenus.AttachMain(this, _adapter);
        BuildMenus();
        BuildTools();
        BuildBars();
        BuildSwatches("StrokeSwatches", false, c => _adapter.StrokeColor = c);
        BuildSwatches("FillSwatches", true, c => _adapter.FillColor = c);
        BuildSwatches("TextSwatches", false, c => _adapter.TextColor = c);
        BuildChoices();

        _canvas.Changed += flags =>
        {
            if ((flags & PixelCanvas.FlagChanged) != 0)
                _adapter.CanvasChangedCommand.Execute(null);
        };
        _canvas.EditRequested += () =>
        {
            Run("finish");
            FocusText();
        };
        _canvas.CancelRequested += () => Run("deselect");
        _canvas.ZoomChanged += zoom =>
        {
            this.FindControl<TextBlock>("ZoomText")!.Text = $"{zoom * 100:0}%";
            _syncingZoom = true;
            this.FindControl<Slider>("ZoomSlider")!.Value = Math.Clamp(zoom * 100, 5, 800);
            _syncingZoom = false;
        };
        _adapter.PropertyChanged += OnAdapterPropertyChanged;
        if (_adapter.CanvasApi != 0)
            _canvas.Attach(_adapter.CanvasApi);
        if (_adapter.UiApi != 0)
            UiApi.Attach(_adapter.UiApi);
        // The overlay's windows, made while idle so the first capture is quick.
        Dispatcher.UIThread.Post(() =>
        {
            if (Environment.GetEnvironmentVariable("JIFFSHOT_UITEST") is null)
                OverlayHost.Prepare(Screens.All.Select(sc => new OverlayMonitor(sc.Bounds, sc.Scaling, sc.IsPrimary)));
        }, DispatcherPriority.Background);
        ApplyPanel();
        ApplyTheme();
        ApplyLibraryLayout();
        var library = this.FindControl<ItemsControl>("LibraryList")!;
        library.AddHandler(PointerPressedEvent, OnLibraryPointerPressed, Avalonia.Interactivity.RoutingStrategies.Tunnel);
        library.AddHandler(PointerMovedEvent, OnLibraryPointerMoved, Avalonia.Interactivity.RoutingStrategies.Tunnel);
        DragDrop.SetAllowDrop(_canvas, true);
        _canvas.AddHandler(DragDrop.DragOverEvent, (_, e) => e.DragEffects = DragDropEffects.Copy);
        _canvas.AddHandler(DragDrop.DropEvent, OnCanvasDrop);
        SizeChanged += (_, _) => { if (_adapter.LibraryExpanded) ApplyLibraryLayout(); };
        ApplyTool();
        ApplyChoices();
        if (Environment.GetEnvironmentVariable("JIFFSHOT_UITEST") is { Length: > 0 } script)
            Opened += async (_, _) => await new UiSelfTest(this, _canvas, _adapter).RunAsync(script);
        else
            Closing += (_, e) =>
            {
                // Closing the editor keeps Jiffshot running in the tray; only
                // its Exit (or Windows shutting down) ends it.
                if (e.CloseReason is WindowCloseReason.ApplicationShutdown or WindowCloseReason.OSShutdown)
                    return;
                e.Cancel = true;
                Hide();
                Run("hidden");
            };
        Closed += (_, _) =>
        {
            _adapter.PropertyChanged -= OnAdapterPropertyChanged;
            _menu?.Dispose();
            _adapter.Dispose();
        };
    }

    private void InitializeComponent() => AvaloniaXamlLoader.Load(this);

    /// <summary>Executes the Run command with an action string.</summary>
    public void Run(string action)
    {
        _adapter.Action = action;
        _adapter.RunCommand.Execute(null);
    }

    /// The tools in the user's order (all tools when none is set).
    private List<(string Name, string Glyph, string Caption)> OrderedTools()
    {
        var order = _adapter.ToolOrder.Split(',', StringSplitOptions.RemoveEmptyEntries);
        if (order.Length == 0)
            return Tools.ToList();
        return order.SelectMany(name => Tools.Where(t => t.Name == name)).ToList();
    }

    /// How many tools fit beside the More button.
    private static int ToolCapacity(double width, bool labels)
    {
        var per = labels ? 64.0 : 42.0;
        return Math.Max(0, (int)((width - per) / per));
    }

    /// The toolbar across the top: the tools that fit, in the user's order,
    /// then More with the rest and the hidden ones. Right-click a tool to
    /// move or hide it, or to turn the names off.
    private void BuildTools()
    {
        var panel = this.FindControl<StackPanel>("ToolPanel")!;
        var host = this.FindControl<Panel>("ToolHost")!;
        panel.Children.Clear();
        _toolButtons.Clear();
        var labels = _adapter.ToolLabels;
        var shown = OrderedTools();
        var hidden = Tools.Where(t => shown.All(s => s.Name != t.Name)).ToList();
        _toolCapacity = ToolCapacity(host.Bounds.Width, labels);
        var fits = shown.Take(_toolCapacity).ToList();
        var more = shown.Skip(_toolCapacity).Concat(hidden).ToList();
        if (_extraTool is { } extra && fits.All(t => t.Name != extra) && _toolCapacity > 0 && more.FindIndex(t => t.Name == extra) is var i and >= 0)
        {
            var picked = more[i];
            more.RemoveAt(i);
            if (fits.Count == _toolCapacity)
            {
                more.Insert(0, fits[^1]);
                fits[^1] = picked;
            }
            else
            {
                fits.Add(picked);
            }
        }
        foreach (var t in fits)
            panel.Children.Add(ToolButton(t, labels));
        panel.Children.Add(MoreButton(more, labels));
    }

    private Button ToolButton((string Name, string Glyph, string Caption) t, bool labels)
    {
        var (name, glyph, caption) = t;
        var button = new Button { Content = ToolContent(glyph, caption), Classes = { "tool" } };
        button.Classes.Set("compact", !labels);
        ToolTip.SetTip(button, caption);
        button.Click += (_, _) => Run("tool:" + name);
        var up = new MenuItem { Header = "Move left" };
        up.Click += (_, _) => Run("toolbar:up:" + name);
        var down = new MenuItem { Header = "Move right" };
        down.Click += (_, _) => Run("toolbar:down:" + name);
        var hide = new MenuItem { Header = "Move to More" };
        hide.Click += (_, _) => Run("toolbar:hide:" + name);
        var reset = new MenuItem { Header = "Show all tools" };
        reset.Click += (_, _) => Run("toolbar:reset");
        button.ContextMenu = new ContextMenu { ItemsSource = new object[] { up, down, hide, new Separator(), LabelsItem(true), LabelsItem(false), new Separator(), reset } };
        _toolButtons[name] = button;
        return button;
    }

    private static StackPanel ToolContent(string glyph, string caption)
    {
        var content = new StackPanel { Spacing = 2 };
        content.Children.Add(new TextBlock { Text = glyph, Classes = { "icon" } });
        content.Children.Add(new TextBlock { Text = caption, Classes = { "caption" } });
        return content;
    }

    /// "Icon and text" or "Icon only", checked when it is the current one.
    private MenuItem LabelsItem(bool labels)
    {
        var item = new MenuItem { Header = labels ? "Icon and text" : "Icon only", ToggleType = MenuItemToggleType.Radio, IsChecked = _adapter.ToolLabels == labels };
        item.Click += (_, _) => _adapter.ToolLabels = labels;
        return item;
    }

    private Button MoreButton(List<(string Name, string Glyph, string Caption)> more, bool labels)
    {
        var button = new Button { Content = ToolContent("⋯", "More"), Classes = { "tool" } };
        button.Classes.Set("compact", !labels);
        ToolTip.SetTip(button, "More tools");
        button.Click += (_, _) =>
        {
            var flyout = new MenuFlyout();
            foreach (var (name, _, caption) in more)
            {
                var item = new MenuItem { Header = caption, ToggleType = MenuItemToggleType.Radio, IsChecked = name == _adapter.Tool };
                item.Click += (_, _) =>
                {
                    _extraTool = name;
                    Run("tool:" + name);
                    BuildTools();
                    ApplyTool();
                };
                flyout.Items.Add(item);
            }
            if (more.Count > 0)
                flyout.Items.Add(new Separator());
            flyout.Items.Add(LabelsItem(true));
            flyout.Items.Add(LabelsItem(false));
            var reset = new MenuItem { Header = "Show all tools in the usual order" };
            reset.Click += (_, _) => Run("toolbar:reset");
            flyout.Items.Add(reset);
            flyout.Items.Add(new MenuItem { Header = "Right-click a tool to move it or put it in More", IsEnabled = false });
            flyout.ShowAt(button);
        };
        return button;
    }

    private static MenuItem Item(string header, Action click)
    {
        var item = new MenuItem { Header = header };
        item.Click += (_, _) => click();
        return item;
    }

    private static MenuItem Item(string header, System.Windows.Input.ICommand command) => new() { Header = header, Command = command };

    /// A menu command run here rather than through the view model.
    private sealed class Act(Action run) : System.Windows.Input.ICommand
    {
        public event EventHandler? CanExecuteChanged { add { } remove { } }
        public bool CanExecute(object? parameter) => true;
        public void Execute(object? parameter) => run();
    }

    private NativeMenuItem NativeItem(string header, Action run, string? gesture = null)
    {
        var command = new Act(run);
        var item = new NativeMenuItem(header) { Command = command };
        if (gesture is not null)
        {
            item.Gesture = KeyGesture.Parse(gesture);
            KeyBindings.Add(new KeyBinding { Gesture = KeyGesture.Parse(gesture), Command = command });
        }
        return item;
    }

    private static readonly (string Name, string Hex)[] CanvasColors =
    {
        ("Transparent", "#00000000"), ("White", "#FFFFFFFF"), ("Light gray", "#FFBDBDBD"), ("Gray", "#FF757575"), ("Charcoal", "#FF22262E"),
        ("Black", "#FF000000"), ("Red", "#FFE53935"), ("Orange", "#FFFF9800"), ("Yellow", "#FFFFEB3B"), ("Green", "#FF43A047"), ("Blue", "#FF1E88E5"), ("Purple", "#FF8E24AA"),
    };

    /// The menu items that open this window's dialogs or list the effects
    /// and share destinations, added to the generated menu bar: Image ▸
    /// Trim, Resize image, Canvas color and Effects; a Share menu; View ▸
    /// Pixel grid; Edit ▸ Editor preferences; and Help.
    private void BuildMenus()
    {
        if (NativeMenu.GetMenu(this) is not { } bar)
            return;
        NativeMenu? Sub(string header) => bar.Items.OfType<NativeMenuItem>().FirstOrDefault(i => i.Header == header)?.Menu;
        if (Sub("_Image") is { } image)
        {
            // After Crop and Auto trim.
            image.Items.Insert(2, NativeItem("_Trim…", () => _ = TrimAsync()));
            image.Items.Insert(3, NativeItem("Resize _image…", () => _ = ResizeAsync(), "Ctrl+Shift+R"));
            image.Items.Add(new NativeMenuItemSeparator());
            var colors = new NativeMenu();
            foreach (var (name, hex) in CanvasColors)
                colors.Items.Add(NativeItem(name, () => _adapter.CanvasColor = hex));
            image.Items.Add(new NativeMenuItem("Canvas c_olor") { Menu = colors });
            _effectsMenu = new NativeMenu();
            image.Items.Add(new NativeMenuItem("_Effects") { Menu = _effectsMenu });
            FillEffectsMenu();
            _adapter.Effects.CollectionChanged += (_, _) => Dispatcher.UIThread.Post(FillEffectsMenu);
        }
        if (Sub("_Edit") is { } edit)
        {
            edit.Items.Add(new NativeMenuItemSeparator());
            edit.Items.Add(NativeItem("_Preferences…", () => Run("prefs:editor"), "Ctrl+OemComma"));
        }
        if (Sub("_View") is { } view)
        {
            var grid = new NativeMenuItem("Pixel _grid") { ToggleType = MenuItemToggleType.CheckBox, Gesture = KeyGesture.Parse("Ctrl+G") };
            var toggle = new Act(() =>
            {
                _canvas.ShowPixelGrid = !_canvas.ShowPixelGrid;
                grid.IsChecked = _canvas.ShowPixelGrid;
            });
            grid.Command = toggle;
            KeyBindings.Add(new KeyBinding { Gesture = KeyGesture.Parse("Ctrl+G"), Command = toggle });
            view.Items.Insert(4, grid);
        }
        _shareMenu = new NativeMenu();
        var share = new NativeMenuItem("_Share") { Menu = _shareMenu };
        var imageAt = bar.Items.OfType<NativeMenuItem>().ToList().FindIndex(i => i.Header == "_Image");
        bar.Items.Insert(imageAt + 1, share);
        FillShareMenu();
        _adapter.Shares.CollectionChanged += (_, _) => Dispatcher.UIThread.Post(FillShareMenu);
        var help = new NativeMenu();
        help.Items.Add(NativeItem("_Keyboard shortcuts", () => _ = Dialogs.KeyboardShortcuts(this).AskAsync(this), "F1"));
        help.Items.Add(new NativeMenuItemSeparator());
        help.Items.Add(NativeItem("_About Jiffshot", () => _ = Dialogs.About(this).AskAsync(this)));
        bar.Items.Add(new NativeMenuItem("_Help") { Menu = help });
    }

    private void FillEffectsMenu()
    {
        if (_effectsMenu is null)
            return;
        _effectsMenu.Items.Clear();
        foreach (var (group, effects) in GroupedEffects())
        {
            if (effects.Length == 1)
            {
                var one = effects[0];
                _effectsMenu.Items.Add(NativeItem(one, () => Run("effect:" + one)));
                continue;
            }
            var sub = new NativeMenu();
            foreach (var name in effects)
            {
                var effect = name;
                sub.Items.Add(NativeItem(name, () => Run("effect:" + effect)));
            }
            _effectsMenu.Items.Add(new NativeMenuItem(group) { Menu = sub });
        }
    }

    private void FillShareMenu()
    {
        if (_shareMenu is null)
            return;
        _shareMenu.Items.Clear();
        foreach (var name in _adapter.Shares)
        {
            var to = name;
            _shareMenu.Items.Add(NativeItem(name, () => Run("share:" + to)));
        }
        _shareMenu.Items.Add(new NativeMenuItemSeparator());
        _shareMenu.Items.Add(new NativeMenuItem("Copy all") { Command = _adapter.CopyImageCommand });
        _shareMenu.Items.Add(new NativeMenuItem("Export…") { Command = _adapter.ExportCommand });
        _shareMenu.Items.Add(new NativeMenuItem("Print…") { Command = _adapter.PrintCommand });
    }

    /// The image's size, from the status line ("1600 × 1000 px · …").
    private (int W, int H)? DocumentSize()
    {
        var m = System.Text.RegularExpressions.Regex.Match(_adapter.DocumentInfo, @"(\d+) × (\d+)");
        return m.Success ? (int.Parse(m.Groups[1].Value), int.Parse(m.Groups[2].Value)) : null;
    }

    private async Task ResizeAsync()
    {
        if (DocumentSize() is not var (w, h))
            return;
        var (dialog, size) = Dialogs.ResizeImage(this, w, h);
        if (await dialog.AskAsync(this))
            Run($"resize:{size().W},{size().H}");
    }

    private async Task TrimAsync()
    {
        if (DocumentSize() is not var (w, h))
            return;
        var (dialog, sides) = Dialogs.Trim(this, w, h);
        if (await dialog.AskAsync(this))
        {
            var (l, t, r, b) = sides();
            Run($"trim-image:{l},{t},{r},{b}");
        }
    }

    /// The Editor / Library tabs, Create and Share menus, the Effects
    /// panel and the tray toggle.
    private void BuildBars()
    {
        this.FindControl<Button>("EditorTab")!.Click += (_, _) => _adapter.LibraryExpanded = false;
        this.FindControl<Button>("LibraryTab")!.Click += (_, _) => _adapter.LibraryExpanded = true;
        var create = this.FindControl<Button>("CreateButton")!;
        create.Click += (_, _) => CreateMenu().ShowAt(create);
        var share = this.FindControl<Button>("ShareButton")!;
        share.Click += (_, _) =>
        {
            var flyout = new MenuFlyout();
            foreach (var name in _adapter.Shares)
            {
                var to = name;
                flyout.Items.Add(Item(name, () => Run("share:" + to)));
            }
            flyout.Items.Add(new Separator());
            flyout.Items.Add(Item("Export…", _adapter.ExportCommand));
            flyout.Items.Add(Item("Print…", _adapter.PrintCommand));
            flyout.ShowAt(share);
        };
        this.FindControl<Button>("EffectsClose")!.Click += (_, _) => _adapter.ShowEffects = false;
        _adapter.Effects.CollectionChanged += (_, _) => Dispatcher.UIThread.Post(BuildEffects);
        BuildEffects();
        var host = this.FindControl<Panel>("ToolHost")!;
        host.SizeChanged += (_, _) =>
        {
            if (ToolCapacity(host.Bounds.Width, _adapter.ToolLabels) != _toolCapacity)
            {
                BuildTools();
                ApplyTool();
            }
        };
        var zoom = this.FindControl<Slider>("ZoomSlider")!;
        zoom.Value = 100;
        zoom.PropertyChanged += (_, e) =>
        {
            if (e.Property == RangeBase.ValueProperty && !_syncingZoom)
                _canvas.SetZoom(zoom.Value / 100);
        };
        ApplyTray();
    }

    /// Create ▾: an image from a template (with its options), combining
    /// captures, and videos made from images.
    private MenuFlyout CreateMenu()
    {
        var flyout = new MenuFlyout();
        var templates = new MenuItem { Header = "Image from template" };
        foreach (var name in _adapter.Templates)
        {
            var t = name;
            templates.Items.Add(Item(name, () => Run("template:" + t)));
        }
        flyout.Items.Add(templates);
        var themes = new MenuItem { Header = "Template theme" };
        foreach (var name in _adapter.TemplateThemes)
        {
            var t = name;
            var item = new MenuItem { Header = name, ToggleType = MenuItemToggleType.Radio, IsChecked = name == _adapter.TemplateTheme };
            item.Click += (_, _) => _adapter.TemplateTheme = t;
            themes.Items.Add(item);
        }
        flyout.Items.Add(themes);
        var captions = new MenuItem { Header = "Captions under images", ToggleType = MenuItemToggleType.CheckBox, IsChecked = _adapter.TemplateCaptions };
        captions.Click += (_, _) => _adapter.TemplateCaptions = !_adapter.TemplateCaptions;
        flyout.Items.Add(captions);
        var numbers = new MenuItem { Header = "Number images", ToggleType = MenuItemToggleType.CheckBox, IsChecked = _adapter.TemplateNumbers };
        numbers.Click += (_, _) => _adapter.TemplateNumbers = !_adapter.TemplateNumbers;
        flyout.Items.Add(numbers);
        var footer = new TextBox { Width = 220, PlaceholderText = "Footer text (optional)", Text = _adapter.TemplateFooter };
        footer.LostFocus += (_, _) => _adapter.TemplateFooter = footer.Text ?? "";
        flyout.Items.Add(new MenuItem { Header = "Footer", Items = { new MenuItem { Header = footer, StaysOpenOnClick = true } } });
        flyout.Items.Add(new Separator());
        flyout.Items.Add(Item("Combine images…", _adapter.CombineImagesCommand));
        flyout.Items.Add(Item("Combine the picked captures", _adapter.CombineSelectedCommand));
        flyout.Items.Add(new Separator());
        flyout.Items.Add(Item("Video from images…", _adapter.VideoFromImagesCommand));
        flyout.Items.Add(Item("Narrated video from images…", _adapter.NarrateImagesCommand));
        flyout.Items.Add(Item("Combine videos…", _adapter.CombineVideosCommand));
        return flyout;
    }

    /// The effects the view model offers, in their groups (those not in a
    /// group under More); empty groups left out.
    private List<(string Group, string[] Effects)> GroupedEffects()
    {
        var names = _adapter.Effects.ToList();
        var listed = EffectGroups.SelectMany(g => g.Effects).ToHashSet();
        return EffectGroups.Select(g => (g.Group, Effects: g.Effects.Where(names.Contains).ToArray()))
            .Append((Group: "More", Effects: names.Where(n => !listed.Contains(n)).ToArray()))
            .Where(g => g.Effects.Length > 0)
            .ToList();
    }

    /// The Effects panel: each group's effects as buttons.
    private void BuildEffects()
    {
        var panel = this.FindControl<StackPanel>("EffectGroups")!;
        panel.Children.Clear();
        foreach (var (group, effects) in GroupedEffects())
        {
            panel.Children.Add(new TextBlock { Text = group.ToUpperInvariant(), Classes = { "section" } });
            var wrap = new WrapPanel();
            foreach (var name in effects)
            {
                var button = new Button { Content = name, Classes = { "chip" } };
                var effect = name;
                button.Click += (_, _) => Run("effect:" + effect);
                wrap.Children.Add(button);
            }
            panel.Children.Add(wrap);
        }
    }

    private void ApplyTray()
    {
        this.FindControl<ToggleButton>("TrayToggle")!.Content = _adapter.ShowTray ? "Hide Recent ▾" : "Show Recent ▴";
        ApplyLibraryLayout();
    }

    private void BuildSwatches(string panelName, bool withNone, Action<string> set)
    {
        var panel = this.FindControl<WrapPanel>(panelName)!;
        var colors = new List<string>(Palette);
        if (withNone)
            colors.Insert(0, "#00000000");
        foreach (var hex in colors)
        {
            var button = new Button { Classes = { "swatch" }, Background = new SolidColorBrush(Color.Parse(hex)) };
            if (hex == "#00000000")
            {
                button.Content = new TextBlock { Text = "∅", FontSize = 12, HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Center };
                ToolTip.SetTip(button, "None");
            }
            var value = hex;
            button.Click += (_, _) => set(value);
            panel.Children.Add(button);
        }
    }

    private void BuildChoices()
    {
        var caps = new[] { "None", "Arrow", "Open arrow", "Dot", "Square", "Diamond", "Taper" };
        Wire("StartCapBox", caps, v => _adapter.StartCap = v);
        Wire("EndCapBox", caps, v => _adapter.EndCap = v);
        Wire("DashBox", new[] { "Solid", "Dash", "Dot" }, v => _adapter.Dash = v);
        Wire("BackFillBox", new[] { "Auto-fill", "Transparent", "Color" }, v => _adapter.BackgroundFill = v);
        Wire("VAlignBox", new[] { "Top", "Middle", "Bottom" }, v => _adapter.VAlign = v);
        BuildSwatches("Fill2Swatches", true, c => _adapter.Fill2 = c);
        BuildSwatches("CanvasSwatches", true, c => _adapter.CanvasColor = c);
        BuildSwatches("OutlineSwatches", true, c => _adapter.OutlineColor = c);
        this.FindControl<Button>("AddStamp")!.Click += (_, _) => Run("add-stamp");
        this.FindControl<ToggleButton>("ZoneFill")!.Click += (_, _) => Run("zone-fit:0");
        this.FindControl<ToggleButton>("ZoneFit")!.Click += (_, _) => Run("zone-fit:1");
        this.FindControl<Button>("ZonePick")!.Click += (_, _) => Run("zone-pick");
        this.FindControl<Button>("TailAdd")!.Click += (_, _) => Run("tail:add");
        this.FindControl<Button>("TailRemove")!.Click += (_, _) => Run("tail:remove");
        this.FindControl<Button>("StyleReset")!.Click += (_, _) => Run("style-theme:reset");
        var styleTheme = this.FindControl<ComboBox>("StyleThemeBox")!;
        styleTheme.SelectionChanged += (_, _) =>
        {
            if (styleTheme.SelectedItem is string name)
            {
                Run("style-theme:" + name);
                styleTheme.SelectedItem = null;
            }
        };
        this.FindControl<Button>("ZoneSwap")!.Click += (_, _) => Run("zone-swap");
        this.FindControl<Button>("ZoneEmpty")!.Click += (_, _) => Run("zone-empty");
        foreach (var (button, how) in new[] { ("AlignL", "left"), ("AlignC", "center"), ("AlignR", "right"), ("AlignT", "top"), ("AlignM", "middle"), ("AlignB", "bottom") })
            this.FindControl<Button>(button)!.Click += (_, _) => Run("align:" + how);
        var variant = this.FindControl<ComboBox>("VariantBox")!;
        variant.SelectionChanged += (_, _) =>
        {
            if (!_syncing && variant.SelectedItem is string v && v != _adapter.Variant)
                _adapter.Variant = v;
        };
        var font = this.FindControl<ComboBox>("FontBox")!;
        font.SelectionChanged += (_, _) =>
        {
            if (!_syncing && font.SelectedItem is string v && v != _adapter.FontFamily)
                _adapter.FontFamily = v;
        };
        this.FindControl<Button>("AlignLeft")!.Click += (_, _) => _adapter.Align = "Left";
        this.FindControl<Button>("AlignCenter")!.Click += (_, _) => _adapter.Align = "Center";
        this.FindControl<Button>("AlignRight")!.Click += (_, _) => _adapter.Align = "Right";
        _adapter.Variants.CollectionChanged += (_, _) => Dispatcher.UIThread.Post(() => Select("VariantBox", _adapter.Variant));
        _adapter.Fonts.CollectionChanged += (_, _) => Dispatcher.UIThread.Post(() => Select("FontBox", _adapter.FontFamily));
    }

    private void Wire(string name, string[] items, Action<string> set)
    {
        var box = this.FindControl<ComboBox>(name)!;
        box.ItemsSource = items;
        box.SelectionChanged += (_, _) =>
        {
            if (!_syncing && box.SelectedItem is string v)
                set(v);
        };
    }

    private void Select(string box, string value)
    {
        _syncing = true;
        try
        {
            var combo = this.FindControl<ComboBox>(box)!;
            if (!Equals(combo.SelectedItem, value))
                combo.SelectedItem = value;
        }
        finally
        {
            _syncing = false;
        }
    }

    private void ApplyPanel()
    {
        var visible = new HashSet<string>(_adapter.Panel.Split(';', StringSplitOptions.RemoveEmptyEntries));
        foreach (var section in Sections)
        {
            if (this.FindControl<Control>("Section_" + section) is { } control)
                control.IsVisible = visible.Contains(section);
        }
        this.FindControl<Control>("QuickStylesSection")!.IsVisible = _adapter.QuickStyles.Count > 0 || visible.Count > 0;
        var title = _adapter.Selection.Length > 0 ? _adapter.Selection : _adapter.Tool;
        this.FindControl<TextBlock>("PanelTitle")!.Text = title;
        this.FindControl<Control>("ShadowOptions")!.IsVisible = visible.Contains("shadow") && _adapter.Shadow;
        this.FindControl<Control>("CanvasSection")!.IsVisible = _adapter.Selection.Length == 0;
        this.FindControl<Control>("ArrangeSection")!.IsVisible = _adapter.Selection.Length > 0;
        this.FindControl<Control>("AddStamp")!.IsVisible = _adapter.Tool == "Stamp" && _adapter.Selection.Length == 0;
    }

    /// <summary>Shows the current values in the lists that are not bound.</summary>
    private void ApplyChoices()
    {
        Select("VariantBox", _adapter.Variant);
        Select("FontBox", _adapter.FontFamily);
        Select("StartCapBox", _adapter.StartCap);
        Select("EndCapBox", _adapter.EndCap);
        Select("DashBox", _adapter.Dash);
        Select("BackFillBox", _adapter.BackgroundFill);
        Select("VAlignBox", _adapter.VAlign);
    }

    private void ApplyTool()
    {
        // A tool chosen from a menu or a shortcut shows in the bar.
        if (!_toolButtons.ContainsKey(_adapter.Tool) && _toolCapacity > 0 && _extraTool != _adapter.Tool && Tools.Any(t => t.Name == _adapter.Tool))
        {
            _extraTool = _adapter.Tool;
            BuildTools();
        }
        foreach (var (name, button) in _toolButtons)
            button.Classes.Set("active", name == _adapter.Tool);
    }

    private void ApplyAmount()
    {
        var slider = this.FindControl<Slider>("AmountSlider")!;
        var title = this.FindControl<TextBlock>("AmountTitle")!;
        title.Text = _adapter.AmountLabel.ToUpperInvariant();
        (slider.Minimum, slider.Maximum, slider.SmallChange) = _adapter.AmountLabel switch
        {
            "Tolerance" => (0.0, 100.0, 1.0),
            "Zoom" => (1.0, 6.0, 0.1),
            "Dim" => (0.0, 1.0, 0.05),
            "Smoothing" => (0.0, 10.0, 1.0),
            _ => (1.0, 60.0, 1.0),
        };
    }

    private void FocusText()
    {
        Dispatcher.UIThread.Post(() =>
        {
            var editor = this.FindControl<TextBox>("TextEditor")!;
            if (!editor.IsVisible)
                return;
            editor.Focus();
            editor.SelectAll();
        }, DispatcherPriority.Background);
    }

    /// The tray strip is one scrolling row; the library view fills most
    /// of the window and wraps.
    private void ApplyLibraryLayout()
    {
        var panel = this.FindControl<Border>("LibraryPanel")!;
        var scroll = this.FindControl<ScrollViewer>("LibraryScroll")!;
        panel.IsVisible = _adapter.ShowTray || _adapter.LibraryExpanded;
        // The Library view takes the canvas's place; zooming is for the canvas.
        _canvas.IsVisible = !_adapter.LibraryExpanded;
        this.FindControl<Control>("ZoomBar")!.IsVisible = !_adapter.LibraryExpanded;
        if (_adapter.LibraryExpanded)
        {
            panel.Height = Math.Max(300, Bounds.Height - 130);
            scroll.HorizontalScrollBarVisibility = Avalonia.Controls.Primitives.ScrollBarVisibility.Disabled;
            scroll.VerticalScrollBarVisibility = Avalonia.Controls.Primitives.ScrollBarVisibility.Auto;
        }
        else
        {
            panel.Height = 76 + _adapter.ThumbSize * 88.0 / 120.0;
            scroll.HorizontalScrollBarVisibility = Avalonia.Controls.Primitives.ScrollBarVisibility.Auto;
            scroll.VerticalScrollBarVisibility = Avalonia.Controls.Primitives.ScrollBarVisibility.Disabled;
        }
    }

    private PointerPressedEventArgs? _dragPress;
    private Point _dragStart;

    /// Dragging a tray item onto the canvas drops its image there.
    private async void OnLibraryPointerMoved(object? sender, PointerEventArgs e)
    {
        if (_dragPress is not { } press || !e.GetCurrentPoint(this).Properties.IsLeftButtonPressed)
            return;
        var at = e.GetPosition(this);
        if (Math.Abs(at.X - _dragStart.X) + Math.Abs(at.Y - _dragStart.Y) < 8)
            return;
        _dragPress = null;
        if (press.Source is not StyledElement { DataContext: LibraryItemViewModelAdapter item })
            return;
        var data = new DataTransfer();
        data.Add(DataTransferItem.CreateText("jiffshot:" + item.Path));
        await DragDrop.DoDragDropAsync(press, data, DragDropEffects.Copy);
    }

    private void OnCanvasDrop(object? sender, DragEventArgs e)
    {
        var paths = new List<string>();
        if (e.DataTransfer.TryGetText() is { } text && text.StartsWith("jiffshot:", StringComparison.Ordinal))
            paths.Add(text["jiffshot:".Length..]);
        foreach (var file in e.DataTransfer.TryGetFiles() ?? [])
        {
            if (file.TryGetLocalPath() is { } local)
                paths.Add(local);
        }
        if (paths.Count == 0)
            return;
        var p = _canvas.DocumentPoint(e.GetPosition(_canvas));
        Run(string.Create(System.Globalization.CultureInfo.InvariantCulture, $"drop:{p.X:0.#},{p.Y:0.#}|{string.Join('|', paths)}"));
        e.Handled = true;
    }

    /// Ctrl+click picks library items instead of opening them.
    private void OnLibraryPointerPressed(object? sender, PointerPressedEventArgs e)
    {
        if ((e.KeyModifiers & KeyModifiers.Control) == 0 && e.GetCurrentPoint(this).Properties.IsLeftButtonPressed)
        {
            _dragPress = e;
            _dragStart = e.GetPosition(this);
        }
        if ((e.KeyModifiers & KeyModifiers.Control) == 0 || !e.GetCurrentPoint(this).Properties.IsLeftButtonPressed)
            return;
        if (e.Source is StyledElement { DataContext: LibraryItemViewModelAdapter item })
        {
            item.ToggleSelectCommand.Execute(null);
            e.Handled = true;
        }
    }

    /// <summary>
    /// The one color theme, for every window: the editor, the Capture
    /// window, dialogs, toasts, the widget and Grab Text.
    /// </summary>
    private void ApplyTheme()
    {
        var theme = _adapter.Theme switch
        {
            "Light" => Avalonia.Styling.ThemeVariant.Light,
            "System" => Avalonia.Styling.ThemeVariant.Default,
            _ => Avalonia.Styling.ThemeVariant.Dark,
        };
        if (Application.Current is { } app)
            app.RequestedThemeVariant = theme;
        _canvas.InvalidateVisual();
    }

    private void OnAdapterPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        switch (e.PropertyName)
        {
            case nameof(MainViewModelAdapter.LibraryExpanded):
                Dispatcher.UIThread.Post(ApplyLibraryLayout);
                break;
            case nameof(MainViewModelAdapter.ThumbSize):
                Dispatcher.UIThread.Post(ApplyLibraryLayout);
                break;
            case nameof(MainViewModelAdapter.ShowTray):
                Dispatcher.UIThread.Post(ApplyTray);
                break;
            case nameof(MainViewModelAdapter.ToolLabels):
                Dispatcher.UIThread.Post(() =>
                {
                    BuildTools();
                    ApplyTool();
                });
                break;
            case nameof(MainViewModelAdapter.ToolOrder):
                Dispatcher.UIThread.Post(() =>
                {
                    BuildTools();
                    ApplyTool();
                });
                break;
            case nameof(MainViewModelAdapter.Theme):
                Dispatcher.UIThread.Post(ApplyTheme);
                break;
            case nameof(MainViewModelAdapter.CanvasApi):
                Dispatcher.UIThread.Post(() => _canvas.Attach(_adapter.CanvasApi));
                break;
            case nameof(MainViewModelAdapter.FrameId):
                Dispatcher.UIThread.Post(_canvas.Refresh);
                break;
            case nameof(MainViewModelAdapter.Panel):
            case nameof(MainViewModelAdapter.Selection):
                ApplyPanel();
                ApplyChoices();
                break;
            case nameof(MainViewModelAdapter.Tool):
                ApplyTool();
                ApplyPanel();
                break;
            case nameof(MainViewModelAdapter.AmountLabel):
                ApplyAmount();
                break;
            case nameof(MainViewModelAdapter.FocusText):
                FocusText();
                break;
            case nameof(MainViewModelAdapter.ZoomRequest):
                var verb = _adapter.ZoomRequest.Split(':')[0];
                Dispatcher.UIThread.Post(() => _canvas.ZoomTo(verb), DispatcherPriority.Background);
                break;
            case nameof(MainViewModelAdapter.Variant):
                Select("VariantBox", _adapter.Variant);
                break;
            case nameof(MainViewModelAdapter.FontFamily):
                Select("FontBox", _adapter.FontFamily);
                break;
            case nameof(MainViewModelAdapter.StartCap):
                Select("StartCapBox", _adapter.StartCap);
                break;
            case nameof(MainViewModelAdapter.EndCap):
                Select("EndCapBox", _adapter.EndCap);
                break;
            case nameof(MainViewModelAdapter.Dash):
                Select("DashBox", _adapter.Dash);
                break;
            case nameof(MainViewModelAdapter.Shadow):
                ApplyPanel();
                break;
            case nameof(MainViewModelAdapter.VAlign):
                Select("VAlignBox", _adapter.VAlign);
                break;
            case nameof(MainViewModelAdapter.Playing):
                this.FindControl<Button>("PlayButton")!.Content = _adapter.Playing ? "⏸" : "▶";
                break;
            case nameof(MainViewModelAdapter.Widget):
                Dispatcher.UIThread.Post(() => WidgetWindow.Set(_adapter.Widget, Run, ActualThemeVariant));
                break;
            case nameof(MainViewModelAdapter.GrabText):
                // "serial\x1ftext"
                if (_adapter.GrabText.IndexOf('\u001f') is var at and >= 0)
                {
                    var grabbed = _adapter.GrabText[(at + 1)..];
                    Dispatcher.UIThread.Post(() => GrabTextWindow.Show(grabbed, ActualThemeVariant));
                }
                break;
            case nameof(MainViewModelAdapter.UiApi):
                UiApi.Attach(_adapter.UiApi);
                break;
            case nameof(MainViewModelAdapter.Overlay):
                // Straight away (this is the UI thread): the overlay should
                // come up as fast as it can.
                OverlayHost.Apply(_adapter.Overlay);
                break;
            case nameof(MainViewModelAdapter.ScreenDraw):
                var draw = _adapter.ScreenDraw;
                Dispatcher.UIThread.Post(() => ScreenDrawWindow.Apply(draw));
                break;
            case nameof(MainViewModelAdapter.Hud):
                // The capture side's panels and outlines, all of them each time.
                var hud = _adapter.Hud;
                Dispatcher.UIThread.Post(() => Hud.Apply(hud, Run));
                break;
            case nameof(MainViewModelAdapter.Prefs):
                // "serial\x1fJSON": the one Preferences dialog.
                if (_adapter.Prefs.IndexOf('\u001f') is var cut and >= 0)
                {
                    var prefs = _adapter.Prefs[(cut + 1)..];
                    Dispatcher.UIThread.Post(() => PreferencesWindow.Show(prefs, Run, this));
                }
                break;
            case nameof(MainViewModelAdapter.Toast):
                // "serial\x1ftitle\x1ftext\x1fproblem", from the capture side.
                if (_adapter.Toast.Split('\u001f') is [_, var title, var text, var problem])
                    Dispatcher.UIThread.Post(() => ToastWindow.Show(title, text, problem == "1", ActualThemeVariant));
                break;
        }
    }

    protected override void OnKeyDown(KeyEventArgs e)
    {
        base.OnKeyDown(e);
        if (e.Handled)
            return;
        if (e.Key == Key.Escape)
        {
            Run("deselect");
            e.Handled = true;
        }
    }
}
