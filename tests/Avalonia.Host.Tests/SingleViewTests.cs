using System;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Host.Com;
using Avalonia.UnitTests;
using Xunit;

namespace Avalonia.Host.Tests;

public class SingleViewTests
{
    private sealed class SingleViewLifetime : ISingleViewApplicationLifetime
    {
        public Control? MainView { get; set; }
    }

    private sealed class FailingSingleViewLifetime : ISingleViewApplicationLifetime
    {
        private Control? _mainView;
        public int FailuresRemaining { get; set; }
        public Control? MainView
        {
            get => _mainView;
            set
            {
                _mainView = value;
                if (FailuresRemaining > 0)
                {
                    FailuresRemaining--;
                    throw new InvalidOperationException("Failed to attach the view.");
                }
            }
        }
    }

    [Fact]
    public void Single_view_lifetime_accepts_and_clears_a_control()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var lifetime = new SingleViewLifetime();
        Application.Current!.ApplicationLifetime = lifetime;
        var application = new AvnApplication();
        var control = new TextBlock();
        var projected = (IAvnControl)ProjectionRuntime.Wrap(control)!;

        Assert.Equal(HResults.S_OK, application.SetMainView(projected));
        Assert.Same(control, lifetime.MainView);
        Assert.Equal(HResults.S_OK, application.SetMainView(null));
        Assert.Null(lifetime.MainView);
    }

    [Fact]
    public void Single_view_rejects_windows_and_desktop_lifetimes()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var application = new AvnApplication();
        var existing = new TextBlock();
        var lifetime = new SingleViewLifetime { MainView = existing };
        Application.Current!.ApplicationLifetime = lifetime;
        Assert.Equal(HResults.E_INVALIDARG,
            application.SetMainView((IAvnControl)ProjectionRuntime.Wrap(new Window())!));
        Assert.Same(existing, lifetime.MainView);
        Application.Current.ApplicationLifetime = new ClassicDesktopStyleApplicationLifetime();
        Assert.Equal(HResults.E_NOTIMPL, application.SetMainView(null));
    }

    [Fact]
    public void Failed_main_view_change_restores_the_previous_view()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var existing = new TextBlock();
        var lifetime = new FailingSingleViewLifetime { MainView = existing, FailuresRemaining = 1 };
        Application.Current!.ApplicationLifetime = lifetime;
        var application = new AvnApplication();
        var projected = (IAvnControl)ProjectionRuntime.Wrap(new TextBlock())!;

        Assert.True(application.SetMainView(projected) < 0);
        Assert.Same(existing, lifetime.MainView);
        Assert.Contains("Failed to attach the view.", AbiError.Take());
    }

    [Fact]
    public void Failed_main_view_change_reports_a_failed_restore()
    {
        using var app = UnitTestApplication.Start(TestServices.StyledWindow);
        var existing = new TextBlock();
        var lifetime = new FailingSingleViewLifetime { MainView = existing, FailuresRemaining = 2 };
        Application.Current!.ApplicationLifetime = lifetime;
        var application = new AvnApplication();
        var projected = (IAvnControl)ProjectionRuntime.Wrap(new TextBlock())!;

        Assert.True(application.SetMainView(projected) < 0);
        var error = AbiError.Take();
        Assert.Contains("Failed to restore the previous main view.", error);
        Assert.Contains("Failed to attach the view.", error);
    }
}
