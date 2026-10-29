using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using Jiffshot.Editor.Controls;
using Jiffshot.Editor.Generated;

namespace Jiffshot.Editor.Views;

/// <summary>
/// A scripted walk through the editor for testing without a mouse: draws
/// one object with each tool through the canvas API, edits properties
/// through the view model, and writes window screenshots, the exported
/// image and a log to the directory named by JIFFSHOT_UITEST.
/// JIFFSHOT_UITEST_IMAGE optionally names the image to annotate.
/// </summary>
internal sealed class UiSelfTest
{
    private readonly Window _window;
    private readonly PixelCanvas _canvas;
    private readonly MainViewModelAdapter _vm;
    private readonly StringBuilder _log = new();
    private string _dir = "";

    public UiSelfTest(Window window, PixelCanvas canvas, MainViewModelAdapter vm)
    {
        _window = window;
        _canvas = canvas;
        _vm = vm;
    }

    private void Run(string action)
    {
        _vm.Action = action;
        _vm.RunCommand.Execute(null);
    }

    /// <summary>Waits, and keeps the log on disk current so a hang shows where it stopped.</summary>
    private Task Frame(int ms = 120)
    {
        if (_dir.Length > 0)
            File.WriteAllText(Path.Combine(_dir, "ui-log.txt"), _log.ToString());
        return Task.Delay(ms);
    }

    private async Task Drag(string tool, double x0, double y0, double x1, double y1, int steps = 12)
    {
        Run("tool:" + tool);
        _canvas.Simulate(0, x0, y0);
        for (var i = 1; i <= steps; i++)
            _canvas.Simulate(1, x0 + (x1 - x0) * i / steps, y0 + (y1 - y0) * i / steps);
        _canvas.Simulate(2, x1, y1);
        await Frame();
        _log.AppendLine($"{tool}: selection={_vm.Selection} panel={_vm.Panel} status={_vm.Status}");
    }

    private async Task Click(string tool, double x, double y)
    {
        Run("tool:" + tool);
        _canvas.Simulate(0, x, y);
        _canvas.Simulate(2, x, y);
        await Frame();
        _log.AppendLine($"{tool} click: selection={_vm.Selection}");
    }

    /// <summary>Shows a dialog without blocking, renders it, and closes it.</summary>
    /// <summary>
    /// The selection overlay over the capture side's generated desktop (two
    /// screens, the second at 150%, two windows): hover, wheel, a Shift drag,
    /// help, the second screen, freehand, multi-area and video guides, each
    /// rendered per screen, then input-plus-render timings.
    /// </summary>
    private async Task OverlayTest()
    {
        if (!UiApi.Ready)
        {
            _log.AppendLine("overlay: no UiApi");
            return;
        }
        (OverlaySession, OverlayCanvas[]) Start(int mode)
        {
            var session = new OverlaySession(UiApi.OverlayDemo(mode));
            var canvases = session.Monitors.Select(m =>
            {
                var c = new OverlayCanvas();
                c.Load(session, m, sized: true);
                c.Measure(new Size(c.Width, c.Height));
                c.Arrange(new Rect(0, 0, c.Width, c.Height));
                return c;
            }).ToArray();
            return (session, canvases);
        }
        void Shot(OverlayCanvas c, string name)
        {
            var m = c.Monitor;
            using var bitmap = new RenderTargetBitmap(new PixelSize(m.Rect.Width, m.Rect.Height), new Vector(96 * m.Scale, 96 * m.Scale));
            bitmap.Render(c);
            var path = Path.Combine(_dir, name);
#pragma warning disable CS0618 // As in Shot.
            bitmap.Save(path);
#pragma warning restore CS0618
            _log.AppendLine("wrote " + path);
        }
        string Describe(OverlaySession s) =>
            $"lit {s.State.Lit} kind {s.State.LitKind} '{s.Title}' {s.Size} on {s.State.Monitor} at {s.State.Scale}x";

        var (session, canvases) = Start(0);
        session.Input(UiApi.Move, 400, 300, 0, 0);
        _log.AppendLine("overlay hover: " + Describe(session));
        Shot(canvases[0], "ui-20-overlay-hover.png");
        Shot(canvases[1], "ui-20b-overlay-other-screen.png");
        session.Input(UiApi.Wheel, 400, 300, 120, 0);
        _log.AppendLine("overlay wheel up: " + Describe(session));
        session.Input(UiApi.Press, 200, 200, 0, 0);
        session.Input(UiApi.Move, 500, 400, 0, 0);
        session.Input(UiApi.Move, 760, 460, 0, UiApi.ModShift);
        _log.AppendLine("overlay shift drag: " + Describe(session));
        Shot(canvases[0], "ui-21-overlay-drag.png");
        session.Input(UiApi.Release, 760, 460, 0, UiApi.ModShift);
        _log.AppendLine($"overlay released: done {session.Done}");
        UiApi.OverlayEndDemo(session.Id);

        (session, canvases) = Start(0);
        session.Input(UiApi.Key, 400, 300, 0x70, 0);
        session.Input(UiApi.Move, 1900, 500, 0, 0);
        _log.AppendLine("overlay second screen: " + Describe(session));
        Shot(canvases[1], "ui-22-overlay-help-150.png");

        // Timing: the pointer sweeping the first screen, each move redrawn.
        var times = new List<double>();
        var inputs = new List<double>();
        using (var target = new RenderTargetBitmap(new PixelSize(1280, 720), new Vector(96, 96)))
        {
            var clock = System.Diagnostics.Stopwatch.StartNew();
            for (var i = 0; i < 240; i++)
            {
                var t = i / 239.0;
                var start = clock.Elapsed.TotalMilliseconds;
                session.Input(UiApi.Move, 40 + 1200 * t, 360 + 300 * Math.Sin(t * 12), 0, 0);
                inputs.Add(clock.Elapsed.TotalMilliseconds - start);
                target.Render(canvases[0]);
                times.Add(clock.Elapsed.TotalMilliseconds - start);
            }
        }
        string Stats(List<double> v)
        {
            var s = v.OrderBy(x => x).ToList();
            return $"avg {s.Average():F2} ms, p95 {s[(int)(s.Count * 0.95)]:F2} ms, max {s[^1]:F2} ms";
        }
        _log.AppendLine($"overlay timing (240 moves, offscreen raster at 1280x720): input {Stats(inputs)}; input+render {Stats(times)}");
        session.Input(UiApi.Cancel, 0, 0, 0, 0);
        _log.AppendLine($"overlay cancelled: done {session.Done}");
        UiApi.OverlayEndDemo(session.Id);

        (session, canvases) = Start(1);
        session.Input(UiApi.Press, 640 + 200, 360, 0, 0);
        for (var i = 1; i <= 60; i++)
        {
            var a = i / 60.0 * Math.PI * 1.7;
            session.Input(UiApi.Move, 640 + 200 * Math.Cos(a), 360 + 200 * Math.Sin(a), 0, 0);
        }
        _log.AppendLine($"overlay freehand: {session.Path.Length / 2} points, " + Describe(session));
        Shot(canvases[0], "ui-23-overlay-freehand.png");
        UiApi.OverlayEndDemo(session.Id);

        (session, canvases) = Start(2);
        foreach (var (a, b) in new[] { ((180, 150), (520, 330)), ((640, 400), (1000, 620)) })
        {
            session.Input(UiApi.Press, a.Item1, a.Item2, 0, 0);
            session.Input(UiApi.Move, (a.Item1 + b.Item1) / 2, (a.Item2 + b.Item2) / 2, 0, 0);
            session.Input(UiApi.Move, b.Item1, b.Item2, 0, 0);
            session.Input(UiApi.Release, b.Item1, b.Item2, 0, 0);
        }
        session.Input(UiApi.Move, 1100, 200, 0, 0);
        _log.AppendLine($"overlay multi-area: {session.Regions.Length / 4} regions");
        Shot(canvases[0], "ui-24-overlay-multi.png");
        session.Input(UiApi.Key, 1100, 200, 0x0D, 0);
        _log.AppendLine($"overlay multi-area Enter: done {session.Done}");
        UiApi.OverlayEndDemo(session.Id);

        (session, canvases) = Start(3);
        session.Input(UiApi.Key, 640, 360, 'G', 0);
        session.Input(UiApi.Move, 640, 360, 0, 0);
        _log.AppendLine($"overlay video guides: {session.State.Has(UiApi.FlagGuides)}");
        Shot(canvases[0], "ui-25-overlay-video-guides.png");
        UiApi.OverlayEndDemo(session.Id);
        await Frame(50);
    }

    /// <summary>Renders a window that is already showing.</summary>
    private async Task ShotOfShown(Window window, string name)
    {
        await Frame(250);
        var size = new PixelSize((int)window.Bounds.Width, (int)window.Bounds.Height);
        using var bitmap = new RenderTargetBitmap(size, new Vector(96, 96));
        bitmap.Render(window);
        var path = Path.Combine(_dir, name);
#pragma warning disable CS0618 // As in Shot.
        bitmap.Save(path);
#pragma warning restore CS0618
        _log.AppendLine("wrote " + path);
    }

    private async Task ShotOf(Window dialog, string name)
    {
        dialog.Show(_window);
        await Frame(400);
        var size = new PixelSize((int)dialog.Bounds.Width, (int)dialog.Bounds.Height);
        using (var bitmap = new RenderTargetBitmap(size, new Vector(96, 96)))
        {
            bitmap.Render(dialog);
            var path = Path.Combine(_dir, name);
#pragma warning disable CS0618 // As in Shot.
            bitmap.Save(path);
#pragma warning restore CS0618
            _log.AppendLine("wrote " + path);
        }
        dialog.Close();
    }

    private async Task Shot(string name)
    {
        await Frame(250);
        var size = new PixelSize((int)_window.Bounds.Width, (int)_window.Bounds.Height);
        using var bitmap = new RenderTargetBitmap(size, new Vector(96, 96));
        bitmap.Render(_window);
        var path = Path.Combine(_dir, name);
#pragma warning disable CS0618 // The replacement overload needs encoder options this test does not care about.
        bitmap.Save(path);
#pragma warning restore CS0618
        _log.AppendLine("wrote " + path);
    }

    private readonly System.Collections.Concurrent.ConcurrentDictionary<string, int> _notifications = new();

    public async Task RunAsync(string dir)
    {
        _dir = dir;
        Directory.CreateDirectory(dir);
        // Counts notifications off the UI thread, so a notification loop that
        // starves the test still leaves evidence.
        _vm.PropertyChanged += (_, e) => _notifications.AddOrUpdate(e.PropertyName ?? "?", 1, (_, n) => n + 1);
        _canvas.Changed += flags => _notifications.AddOrUpdate("canvas.Changed", 1, (_, n) => n + 1);
        using var watchdog = new System.Threading.Timer(_ =>
        {
            var lines = new StringBuilder();
            foreach (var pair in _notifications)
                lines.AppendLine($"{pair.Key}={pair.Value}");
            File.WriteAllText(Path.Combine(dir, "ui-notifications.txt"), lines.ToString());
        }, null, 1000, 1000);
        _log.AppendLine("started");
        try
        {
            await Frame(800);
            if (Environment.GetEnvironmentVariable("JIFFSHOT_UITEST_IMAGE") is { Length: > 0 } image)
            {
                Run("open:" + image);
                await Frame(600);
            }
            Run("zoom:fit");
            var size = _canvas.DocumentSize;
            double W = size.Width, H = size.Height;
            _log.AppendLine($"document {W}x{H}, zoom {_canvas.Zoom:0.00}");

            await Drag("Arrow", W * 0.08, H * 0.12, W * 0.30, H * 0.30);
            await Drag("Shape", W * 0.35, H * 0.10, W * 0.55, H * 0.30);
            _vm.Variant = "Rounded";
            _vm.StrokeColor = "#FF1E88E5";
            await Frame();
            await Drag("Text", W * 0.60, H * 0.08, W * 0.92, H * 0.18);
            _vm.Text = "Jiffshot text";
            await Frame();
            await Drag("Callout", W * 0.06, H * 0.42, W * 0.30, H * 0.58);
            _vm.Text = "A callout with a tail";
            _vm.Fill2 = "#FF8E24AA";
            _vm.Underline = true;
            _vm.VAlign = "Middle";
            await Frame();
            await Click("Step", W * 0.40, H * 0.45);
            await Click("Step", W * 0.47, H * 0.45);
            await Click("Step", W * 0.54, H * 0.45);
            await Drag("Pen", W * 0.62, H * 0.40, W * 0.90, H * 0.55, 30);
            await Drag("Highlight", W * 0.60, H * 0.62, W * 0.90, H * 0.70);
            await Drag("Blur", W * 0.06, H * 0.68, W * 0.26, H * 0.90);
            await Drag("Magnify", W * 0.32, H * 0.66, W * 0.46, H * 0.88);
            await Click("Stamp", W * 0.52, H * 0.78);
            await Drag("Line", W * 0.60, H * 0.80, W * 0.90, H * 0.92);
            _vm.Shadow = true;
            _vm.ShadowDistance = 10;
            _vm.EndCap = "Diamond";
            await Frame();
            Run("tool:Shape");
            _vm.Variant = "Polygon";
            await Frame();
            foreach (var (px, py) in new[] { (0.92, 0.30), (0.98, 0.40), (0.90, 0.48) })
            {
                _canvas.Simulate(0, W * px, H * py);
                _canvas.Simulate(2, W * px, H * py);
            }
            Run("finish");
            await Frame();
            _log.AppendLine($"polygon: selection={_vm.Selection} {_vm.DocumentInfo}");
            _vm.Variant = "Rectangle";
            await Shot("ui-1-annotated.png");
            _vm.CycleThemeCommand.Execute(null);
            await Frame(300);
            _log.AppendLine($"theme: {_vm.Theme}");
            await Shot("ui-1b-light.png");
            _vm.CycleThemeCommand.Execute(null);
            _vm.CycleThemeCommand.Execute(null);
            await Frame(300);
            _log.AppendLine($"theme back: {_vm.Theme}");

            // Select, move and restyle the first arrow; undo and redo.
            await Drag("Select", W * 0.19, H * 0.21, W * 0.24, H * 0.26);
            _vm.StrokeWidth = 14;
            await Frame();
            _vm.UndoCommand.Execute(null);
            _vm.RedoCommand.Execute(null);
            await Frame();
            _log.AppendLine($"after edits: {_vm.DocumentInfo}; title {_vm.Title}");

            Run("export-to:" + Path.Combine(dir, "ui-export.png"));
            Run("export-to:" + Path.Combine(dir, "ui-export.pdf"));
            Run("deselect");
            await Frame();
            await Drag("Select", W, H, W + 120, H + 60);
            _log.AppendLine($"canvas drag: {_vm.DocumentInfo}");
            _vm.UndoCommand.Execute(null);
            await Frame();
            await Frame();
            Run("effect:Border");
            await Frame(300);
            Run("effect:Shadow");
            await Frame(300);
            await Shot("ui-2-effects.png");
            _log.AppendLine($"after effects: {_vm.DocumentInfo}");

            await Drag("Crop", W * 0.1, H * 0.1, W * 0.6, H * 0.6);
            await Shot("ui-3-cropped.png");
            _log.AppendLine($"after crop: {_vm.DocumentInfo}");
            _log.AppendLine("status: " + _vm.Status);

            // Image tools on a fresh copy of the image (coordinates assume
            // the synthetic screenshot from the doc crate's example).
            if (Environment.GetEnvironmentVariable("JIFFSHOT_UITEST_IMAGE") is { Length: > 0 } fresh)
            {
                Run("open:" + fresh);
                await Frame(600);
                Run("zoom:fit");
                await Drag("Text Replace", W * 0.26, H * 0.15, W * 0.84, H * 0.205);
                _log.AppendLine($"text replace: selection={_vm.Selection} text={_vm.Text}");
                _vm.Text = "Text Replace made this line editable.";
                await Frame(300);
                await Drag("Smart Move", W * 0.58, H * 0.54, W * 0.745, H * 0.88);
                _log.AppendLine($"smart move: selection={_vm.Selection} status={_vm.Status}");
                await Drag("Select", W * 0.66, H * 0.71, W * 0.66 + 230, H * 0.71 - 40);
                await Click("Fill", W * 0.02, H * 0.5);
                await Drag("Spotlight", W * 0.25, H * 0.40, W * 0.50, H * 0.50);
                await Shot("ui-4-image-tools.png");
                _vm.UndoCommand.Execute(null);
                await Drag("Cut Out", W * 0.3, H * 0.33, W * 0.3, H * 0.40);
                _log.AppendLine($"after cut out: {_vm.DocumentInfo}");
                await Shot("ui-5-cut-out.png");

                Run("open:" + fresh);
                await Frame(600);
                _vm.SmartRedactCommand.Execute(null);
                await Frame(1200);
                _log.AppendLine($"smart redact: {_vm.Status}");
                await Click("Magic Wand", W * 0.30, H * 0.38);
                _log.AppendLine($"magic wand: selection={_vm.Selection}");
                await Drag("Eraser", W * 0.05, H * 0.95, W * 0.20, H * 0.95);
                _vm.SelectAllCommand.Execute(null);
                await Frame();
                _log.AppendLine($"select all: {_vm.Selection} count={_vm.SelectionCount}");
                _vm.GroupCommand.Execute(null);
                await Frame();
                _log.AppendLine($"group: {_vm.Selection}; {_vm.DocumentInfo}");
                _vm.UngroupCommand.Execute(null);
                await Frame();
                _log.AppendLine($"ungroup: {_vm.Selection}; {_vm.DocumentInfo}");
                await Shot("ui-5b-redact-wand.png");

                Run("open:" + fresh);
                await Frame(600);
                _vm.SimplifyCommand.Execute(null);
                await Frame(1500);
                _log.AppendLine($"simplify: {_vm.Status}; {_vm.DocumentInfo}");
                await Shot("ui-6-simplified.png");
            }
            if (Environment.GetEnvironmentVariable("JIFFSHOT_UITEST_VIDEO") is { Length: > 0 } video)
            {
                Run("open:" + video);
                await Frame(800);
                _log.AppendLine($"video: mode={_vm.Mode} duration={_vm.VideoDuration:0.00} info={_vm.VideoInfo}");
                _vm.VideoPosition = _vm.VideoDuration * 0.25;
                await Frame(400);
                _vm.SetTrimStartCommand.Execute(null);
                _vm.VideoPosition = _vm.VideoDuration * 0.75;
                await Frame(400);
                _vm.SetTrimEndCommand.Execute(null);
                await Frame();
                _log.AppendLine($"trim {_vm.TrimStart:0.00}..{_vm.TrimEnd:0.00}; status {_vm.Status}");
                _vm.VideoPosition = _vm.VideoDuration * 0.3;
                await Frame(300);
                _vm.PlayPauseCommand.Execute(null);
                await Frame(800);
                _log.AppendLine($"playing={_vm.Playing} position={_vm.VideoPosition:0.00}");
                await Shot("ui-8-video.png");
                var before = _vm.VideoPosition;
                _vm.PrevFrameCommand.Execute(null);
                await Frame(300);
                _vm.NextFrameCommand.Execute(null);
                _vm.NextFrameCommand.Execute(null);
                await Frame(300);
                _log.AppendLine($"frame step: {before:0.000} -> {_vm.VideoPosition:0.000}");
                _vm.GifPreset = "High motion";
                _vm.GifFade = true;
                _vm.ToggleMuteCommand.Execute(null);
                await Frame(200);
                _log.AppendLine($"gif {_vm.GifPreset} fade={_vm.GifFade} muted={_vm.Muted}");
                _vm.ToggleMuteCommand.Execute(null);
                _vm.GifPreset = "Screen recording";
                _vm.GifFade = false;
                await Frame(200);
                await Shot("ui-8b-video-options.png");
                _vm.PlayPauseCommand.Execute(null);
                await Frame(300);
                _vm.FrameToImageCommand.Execute(null);
                await Frame(500);
                _log.AppendLine($"frame to image: mode={_vm.Mode} {_vm.DocumentInfo}");
            }
            if (_vm.Library.Count > 1)
            {
                var first = _vm.Library[0];
                first.TagText = "uitest, docs";
                first.SetTagsCommand.Execute(null);
                await Frame(300);
                _vm.Library[_vm.Library.Count - 1].TogglePinCommand.Execute(null);
                await Frame(300);
                _log.AppendLine($"library: {_vm.LibraryInfo}; first={_vm.Library[0].Label} pinned={_vm.Library[0].IsPinned} tags=[{_vm.Library[1].Tags}{_vm.Library[0].Tags}]");
                _vm.LibraryFilter = "Tag: uitest";
                await Frame(300);
                _log.AppendLine($"filter Tag: uitest -> {_vm.Library.Count} item(s); filters: {string.Join(" | ", _vm.LibraryFilters)}");
                _vm.LibraryFilter = "All";
                _vm.LibrarySearch = "zzz-nothing";
                await Frame(300);
                _log.AppendLine($"search zzz -> {_vm.Library.Count}");
                _vm.LibrarySearch = "";
                _vm.Library[0].ToggleSelectCommand.Execute(null);
                _vm.Library[1].ToggleSelectCommand.Execute(null);
                await Frame(200);
                _log.AppendLine($"picked {_vm.LibrarySelected}");
                _vm.LibraryExpanded = true;
                _vm.ThumbSize = 180;
                await Frame(600);
                await Shot("ui-9-library.png");
                // Undo the test edits: unpin, untag, unpick, collapse.
                foreach (var item in _vm.Library)
                {
                    if (item.IsPinned) item.TogglePinCommand.Execute(null);
                }
                await Frame(300);
                foreach (var item in _vm.Library)
                {
                    if (item.Tags.Length > 0) { item.TagText = ""; item.SetTagsCommand.Execute(null); }
                }
                _vm.ClearLibrarySelectionCommand.Execute(null);
                _vm.ThumbSize = 120;
                _vm.LibraryExpanded = false;
                await Frame(300);
                _log.AppendLine($"library restored: {_vm.LibraryInfo}");
            }
            if (Environment.GetEnvironmentVariable("JIFFSHOT_UITEST_IMAGE") is { Length: > 0 } dropImage)
            {
                _vm.TemplateTheme = "Ocean";
                _vm.TemplateNumbers = true;
                _vm.TemplateCaptions = true;
                _vm.TemplateFooter = "Made with Jiffshot";
                await Frame(200);
                Run("template:Two sections");
                await Frame(500);
                _log.AppendLine($"template: {_vm.DocumentInfo}; status {_vm.Status}");
                // Drop the test image on the first zone, and twice more to fill the next ones.
                Run($"drop:200,200|{dropImage}|{dropImage}");
                await Frame(500);
                _log.AppendLine($"dropped: {_vm.Status}; selection={_vm.Selection} panel={_vm.Panel}");
                Run("zone-fit:1");
                await Frame(300);
                _log.AppendLine($"zone fit={_vm.ZoneFit}");
                _vm.ZoomFitCommand.Execute(null);
                await Frame(400);
                await Shot("ui-10-template.png");
                Run("toolbar:hide:Line");
                Run("toolbar:up:Text");
                await Frame(300);
                _log.AppendLine($"toolbar: {_vm.ToolOrder}");
                await Shot("ui-11-toolbar.png");
                // The layout switches: the Effects panel, an icon-only
                // toolbar, and the side bar and tray hidden.
                _vm.ShowEffects = true;
                await Frame(300);
                await Shot("ui-12-effects-panel.png");
                _vm.ShowEffects = false;
                _vm.ToolLabels = false;
                _vm.ShowSideBar = false;
                _vm.ShowTray = false;
                await Frame(300);
                _log.AppendLine($"layout: labels={_vm.ToolLabels} side={_vm.ShowSideBar} tray={_vm.ShowTray}");
                await Shot("ui-13-compact.png");
                _vm.ToolLabels = true;
                _vm.ShowSideBar = true;
                _vm.ShowTray = true;
                await Frame(200);
                // The menus added around the generated ones, and what
                // Image's new items do (each undone again).
                if (NativeMenu.GetMenu(_window) is { } bar)
                {
                    var tops = bar.Items.OfType<NativeMenuItem>().ToList();
                    _log.AppendLine("menus: " + string.Join(" ", tops.Select(i => i.Header)));
                    foreach (var top in tops.Where(t => t.Header is "_Image" or "_Share" or "_Help" or "_View" or "_File"))
                        _log.AppendLine($"  {top.Header}: " + string.Join(" | ", top.Menu!.Items.Select(i => i is NativeMenuItem m ? m.Header : "—")));
                }
                var before = _vm.DocumentInfo;
                Run("resize:800,500");
                await Frame(300);
                _log.AppendLine($"resize: {before} -> {_vm.DocumentInfo}");
                _vm.UndoCommand.Execute(null);
                Run("trim-image:10,20,30,40");
                await Frame(300);
                _log.AppendLine($"trim: -> {_vm.DocumentInfo}");
                _vm.UndoCommand.Execute(null);
                await Frame(200);
                _vm.AutoTrimCommand.Execute(null);
                await Frame(300);
                _log.AppendLine($"auto trim: -> {_vm.DocumentInfo}; {_vm.Status}");
                _log.AppendLine($"recent files: {_vm.RecentFiles.Count}");
                var (resize, _) = Dialogs.ResizeImage(_window, 1600, 1000);
                await ShotOf(resize.Window, "ui-14-resize.png");
                // The one Preferences dialog, each tab (Cancelled: nothing is applied).
                _vm.Action = "prefs:capture";
                _vm.RunCommand.Execute(null);
                await Frame(600);
                if (Avalonia.Application.Current?.ApplicationLifetime is Avalonia.Controls.ApplicationLifetimes.IClassicDesktopStyleApplicationLifetime life
                    && life.Windows.OfType<PreferencesWindow>().FirstOrDefault() is { } prefs)
                {
                    _log.AppendLine("preferences tabs: " + string.Join(", ", prefs.TabTitles));
                    for (var t = 0; t < prefs.TabTitles.Count(); t++)
                    {
                        prefs.SelectTab(t);
                        await Frame(250);
                        await ShotOfShown(prefs, $"ui-15-preferences-{t}.png");
                    }
                    var values = prefs.Collect();
                    _log.AppendLine("preferences values: " + values.ToJsonString());
                    prefs.Close();
                    // One theme for every window: Light through the dialog's own
                    // path, then back to what it was.
                    var was = values["editor"]![0]!.GetValue<int>();
                    values["editor"]![0] = 1;
                    _vm.Action = "prefs-apply:" + values.ToJsonString();
                    _vm.RunCommand.Execute(null);
                    await Frame(400);
                    _log.AppendLine($"theme after Light: app {Avalonia.Application.Current?.ActualThemeVariant}, editor {_window.ActualThemeVariant}, capture window {CaptureWindow.Instance?.ActualThemeVariant}");
                    values["editor"]![0] = was;
                    _vm.Action = "prefs-apply:" + values.ToJsonString();
                    _vm.RunCommand.Execute(null);
                    await Frame(400);
                    _log.AppendLine($"theme restored: app {Avalonia.Application.Current?.ActualThemeVariant}, capture window {CaptureWindow.Instance?.ActualThemeVariant}");
                }
                else
                {
                    _log.AppendLine("preferences: not shown");
                }
                await ShotOf(Dialogs.KeyboardShortcuts(_window).Window, "ui-16-shortcuts.png");
                await ShotOf(ToastWindow.Build("Capture saved", "Copied to the clipboard; opened in the editor", false, _window.ActualThemeVariant), "ui-17-toast.png");
                await ShotOf(ToastWindow.Build("Recording failed", "The microphone is in use by another app", true, Avalonia.Styling.ThemeVariant.Light), "ui-17b-toast-light.png");
                // A capture panel and outline, from the capture side's JSON.
                var hudLog = new StringBuilder();
                Hud.Apply("""{"panels":[{"id":9001,"area":[200,200,640,360],"status":"● Recording 0:07 · 210 frames\nShift+F9 pause · Shift+F10 stop · Esc cancels","buttons":["Stop","Pause","Draw","Cancel"]}],"borders":[{"id":9002,"area":[200,200,640,360]}]}""", a => hudLog.Append(a).Append(' '));
                await Frame(400);
                foreach (var panel in Hud.OpenPanels)
                {
                    await ShotOfShown(panel, "ui-19-hud-panel.png");
                    _log.AppendLine($"hud panel at {panel.Position} size {panel.Bounds.Size}");
                }
                foreach (var panel in Hud.OpenPanels)
                    _log.AppendLine($"hud panel affinity 0x{ScreenProbe.Affinity(panel):X}; {ScreenProbe.At(panel.Position.X + 20, panel.Position.Y + 8, panel)}");
                foreach (var border in Hud.OpenBorders)
                {
                    var o = border.Outer;
                    _log.AppendLine($"hud border {o} at {border.Position} size {border.Bounds.Size} affinity 0x{ScreenProbe.Affinity(border):X}");
                    _log.AppendLine($"  on the frame: {ScreenProbe.At(o.X + 1, o.Y + o.Height / 2, border)}; inside: {ScreenProbe.At(o.X + o.Width / 2, o.Y + o.Height / 2, border)}");
                }
                // The recording toolbar: recording, then paused with the
                // webcam on and the microphone off, then minimized.
                const string recording = """{"id":9003,"area":[200,200,640,360],"status":"","buttons":[],"toolbar":{"millis":29000,"frames":870,"paused":false,"cursor":true,"highlight":false,"clicks":true,"webcam":false,"webcam_corner":"bottom-right","webcam_circle":true,"system":false,"microphone":true,"draw":false,"draw_offered":true,"note":null}}""";
                Hud.Apply("{\"panels\":[" + recording + "],\"borders\":[]}", a => hudLog.Append(a).Append(' '));
                await Frame(500);
                foreach (var toolbar in Hud.OpenToolbars)
                {
                    await ShotOfShown(toolbar, "ui-19b-recording-toolbar.png");
                    _log.AppendLine($"recording toolbar at {toolbar.Position} size {toolbar.Bounds.Size} affinity 0x{ScreenProbe.Affinity(toolbar):X}");
                }
                var paused = recording.Replace("\"paused\":false", "\"paused\":true").Replace("\"webcam\":false", "\"webcam\":true").Replace("\"microphone\":true", "\"microphone\":false").Replace("\"millis\":29000", "\"millis\":3725000").Replace("\"note\":null", "\"note\":\"no microphone: the device is in use\"");
                Hud.Apply("{\"panels\":[" + paused + "],\"borders\":[]}", a => hudLog.Append(a).Append(' '));
                await Frame(400);
                foreach (var toolbar in Hud.OpenToolbars)
                    await ShotOfShown(toolbar, "ui-19c-recording-toolbar-paused.png");
                Hud.Apply("""{"panels":[],"borders":[]}""", a => hudLog.Append(a).Append(' '));
                await Frame(200);
                _log.AppendLine($"hud after clear: {Hud.OpenPanels.Count} panels, {Hud.OpenBorders.Count()} borders; sent '{hudLog}'");
                await OverlayTest();
                // Screen Draw over a small area: a box, then the window rendered.
                if (UiApi.Ready)
                {
                    ScreenDrawWindow.Apply(UiApi.DrawDemo(new PixelRect(120, 120, 420, 260)));
                    await Frame(400);
                    if (ScreenDrawWindow.Current is { } draw)
                    {
                        draw.Draw(UiApi.Key, 0, 0, 'S');
                        draw.Draw(UiApi.Press, 60, 50);
                        draw.Draw(UiApi.Move, 300, 200);
                        draw.Draw(UiApi.Release, 300, 200);
                        draw.Draw(UiApi.Key, 0, 0, 'A');
                        draw.Draw(UiApi.Press, 340, 40);
                        draw.Draw(UiApi.Move, 220, 120);
                        draw.Draw(UiApi.Release, 220, 120);
                        await Frame(300);
                        _log.AppendLine($"screen draw window at {draw.Position} size {draw.Bounds.Size} affinity 0x{ScreenProbe.Affinity(draw):X}");
                        await ShotOfShown(draw, "ui-26-screen-draw.png");
                    }
                    UiApi.DrawEnd();
                    ScreenDrawWindow.Apply("");
                }
                // The Capture window: each tab, a switch, then back to Image.
                if (CaptureWindow.Instance is { } capture)
                {
                    capture.Show();
                    foreach (var tab in new[] { 0, 1, 2 })
                    {
                        capture.RunAction("tab:" + tab);
                        await Frame(300);
                        await ShotOfShown(capture, $"ui-18-capture-tab{tab}.png");
                    }
                    capture.RunAction("tab:1");
                    capture.RunAction("switch:clipboard");
                    await Frame(300);
                    _log.AppendLine("capture view: " + capture.ViewJson);
                    capture.RunAction("switch:clipboard");
                    await Frame(200);
                    // Closed for real here, so the test ends with the editor.
                    capture.Close();
                }
                else
                {
                    _log.AppendLine("capture window: not mounted");
                }
                Run("toolbar:reset");
                Run("tool:Arrow");
                Run("style-theme:Warm");
                await Frame(300);
                _log.AppendLine($"arrow quick styles themed: {_vm.QuickStyles.Count}, first stroke {(_vm.QuickStyles.Count > 0 ? _vm.QuickStyles[0].Stroke : "-")}");
                if (_vm.QuickStyles.Count > 1) _vm.QuickStyles[1].ToggleFavoriteCommand.Execute(null);
                await Frame(300);
                _log.AppendLine($"favorite first: {(_vm.QuickStyles.Count > 0 && _vm.QuickStyles[0].IsFavorite)}");
                if (_vm.QuickStyles.Count > 0 && _vm.QuickStyles[0].IsFavorite) _vm.QuickStyles[0].ToggleFavoriteCommand.Execute(null);
                Run("style-theme:reset");
                await Frame(300);
                _log.AppendLine($"toolbar reset: [{_vm.ToolOrder}], styles reset: {_vm.QuickStyles.Count}");
                _vm.TemplateTheme = "Light";
                _vm.TemplateNumbers = false;
                _vm.TemplateCaptions = false;
                _vm.TemplateFooter = "";
                await Frame(200);
            }
        }
        catch (Exception e)
        {
            _log.AppendLine("FAILED: " + e);
        }
        File.WriteAllText(Path.Combine(dir, "ui-log.txt"), _log.ToString());
        if (Environment.GetEnvironmentVariable("JIFFSHOT_UITEST_KEEP") is not { Length: > 0 })
            Dispatcher.UIThread.Post(_window.Close);
    }
}

/// <summary>What is on screen at a point, for checking the capture outline.</summary>
internal static partial class ScreenProbe
{
    /// <summary>"#RRGGBB, hits WINDOW: yes|no".</summary>
    public static string At(int x, int y, Window window)
    {
        var dc = GetDC(0);
        var color = GetPixel(dc, x, y);
        ReleaseDC(0, dc);
        var hit = WindowFromPoint(new Point { X = x, Y = y }) == (window.TryGetPlatformHandle()?.Handle ?? -1);
        return $"#{color & 0xFF:X2}{(color >> 8) & 0xFF:X2}{(color >> 16) & 0xFF:X2}, hits {window.GetType().Name}: {(hit ? "yes" : "no")}";
    }

    /// <summary>The window's display affinity (0x11: left out of captures).</summary>
    public static uint Affinity(Window window) =>
        window.TryGetPlatformHandle()?.Handle is { } hwnd && GetWindowDisplayAffinity(hwnd, out var a) ? a : 0;

    private struct Point
    {
        public int X;
        public int Y;
    }

    [System.Runtime.InteropServices.LibraryImport("user32.dll")]
    private static partial nint GetDC(nint hwnd);

    [System.Runtime.InteropServices.LibraryImport("user32.dll")]
    private static partial int ReleaseDC(nint hwnd, nint dc);

    [System.Runtime.InteropServices.LibraryImport("gdi32.dll")]
    private static partial uint GetPixel(nint dc, int x, int y);

    [System.Runtime.InteropServices.LibraryImport("user32.dll")]
    private static partial nint WindowFromPoint(Point point);

    [System.Runtime.InteropServices.LibraryImport("user32.dll")]
    [return: System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.Bool)]
    private static partial bool GetWindowDisplayAffinity(nint hwnd, out uint affinity);
}
