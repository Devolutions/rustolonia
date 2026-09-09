using System.Runtime.InteropServices.Marshalling;

namespace Avalonia.Host.Com;

[GeneratedComClass]
public partial class AvnActivationFactory : IAvnActivationFactory
{
    public int CreateEcho(out IAvnEcho? echo)
    {
        echo = new AvnEcho();
        return HResults.S_OK;
    }

    public int CreateApplication(out IAvnApplication? application)
    {
        application = new AvnApplication();
        return HResults.S_OK;
    }

    public int CreateControlFactory(out IAvnControlFactory? factory)
    {
        factory = new AvnControlFactory();
        return HResults.S_OK;
    }

    public int CreateDispatcher(out IAvnDispatcher? dispatcher)
    {
        dispatcher = new AvnDispatcher();
        return HResults.S_OK;
    }
}
