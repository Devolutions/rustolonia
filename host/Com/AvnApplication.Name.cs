using System;

namespace Avalonia.Host.Com;

public partial class AvnApplication
{
#if !AVN_HOST_ANDROID && !AVN_HOST_IOS
    private string? _applicationName;
    private bool _applicationCreated;
#endif

    public int SetApplicationName(string? name)
    {
        if (name is null)
            return HResults.E_POINTER;
        if (string.IsNullOrWhiteSpace(name) || name.Contains('\0'))
            return HResults.E_INVALIDARG;
#if AVN_HOST_ANDROID || AVN_HOST_IOS
        return HResults.E_NOTIMPL;
#else
        try
        {
            if (_applicationCreated || Application.Current is not null)
                throw new InvalidOperationException("The application name must be set before Avalonia initialization.");
            _applicationName = name;
            return HResults.S_OK;
        }
        catch (Exception e)
        {
            return AbiError.Capture(e);
        }
#endif
    }

#if !AVN_HOST_ANDROID && !AVN_HOST_IOS
    internal HostApplication CreateApplication()
    {
        _applicationCreated = true;
        var application = new HostApplication();
        if (_applicationName is not null)
            application.Name = _applicationName;
        return application;
    }

    private AppBuilder CreateAppBuilder()
    {
        var builder = RustHostPlatform.Configure(AppBuilder.Configure(CreateApplication));
#if AVN_HOST_OSX
        builder.With(CreateMacOSPlatformOptions());
#endif
        return builder;
    }
#endif
}
