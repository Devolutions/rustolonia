using System;
using System.Runtime.InteropServices;
using Avalonia.Rust.BrowserCom;
using BrowserCom.Testing;

internal static unsafe class Program
{
    private static readonly Guid s_unknown = new("00000000-0000-0000-C000-000000000046");

    private static int Main()
    {
        var vtable = (void**)NativeMemory.AllocZeroed((nuint)(3 * sizeof(void*)));
        vtable[0] = (void*)(delegate* unmanaged<NativeBridge*, Guid*, nint*, int>)&QueryInterface;
        vtable[1] = (void*)(delegate* unmanaged<NativeBridge*, uint>)&AddRef;
        vtable[2] = (void*)(delegate* unmanaged<NativeBridge*, uint>)&Release;
        var bridge = (NativeBridge*)NativeMemory.AllocZeroed((nuint)sizeof(NativeBridge));
        bridge->Vtable = vtable;
        bridge->Inner = ComRuntime.GetOrCreateComInterfaceForObject(new Calculator());
        bridge->RefCount = 1;
        try
        {
            var details = ComRuntime.GetInterfaceDetails(typeof(ICalculator))
                ?? throw new InvalidOperationException("The calculator interface was not registered.");
            Require(details.Iid == InterfaceIds.Calculator, $"Unexpected calculator IID: {details.Iid}.");
            var queryResult = ComRuntime.QueryInterface(bridge->Inner, details.Iid, out var calculator);
            Require(queryResult == 0 && calculator != 0, $"Managed wrapper QueryInterface failed: 0x{queryResult:X8}.");
            ComRuntime.Release(calculator);
            var proxy = (ICalculator)ComRuntime.GetOrCreateObjectForComInstance((nint)bridge, typeof(ICalculator));
            Require(proxy.Increment(41, out var value) == 0 && value == 42, "Native proxy call failed.");
            Require(proxy is ICalculator2 && proxy is not ICalculator3, "Optional interface casts failed.");
            Require(((ICalculator2)proxy).Double(21, out value) == 0 && value == 42, "Optional interface call failed.");
            Require(proxy.Echo("native AOT", out var text) == 0 && text == "native AOT", "UTF-16 marshalling failed.");

            var derived = (ILocalDerived)ComRuntime.GetOrCreateObjectForComInstance((nint)bridge, typeof(ILocalDerived));
            Require(derived.BaseValue(32, out value) == 0 && value == 42, "Inherited base call failed.");
            Require(derived.DerivedValue(22, out value) == 0 && value == 42, "Inherited derived call failed.");
            Require(derived.LocalValue(12, out value) == 0 && value == 42, "Local derived call failed.");
            Console.WriteLine("Browser COM NativeAOT smoke test passed.");
            GC.KeepAlive(proxy);
            GC.KeepAlive(derived);
            return 0;
        }
        finally
        {
            ComRuntime.Release((nint)bridge);
        }
    }

    private static void Require(bool condition, string message)
    {
        if (!condition)
            throw new InvalidOperationException(message);
    }

    [UnmanagedCallersOnly]
    private static int QueryInterface(NativeBridge* self, Guid* iid, nint* result)
    {
        if (*iid == s_unknown)
        {
            self->RefCount++;
            *result = (nint)self;
            return 0;
        }
        if (*iid == InterfaceIds.Calculator || *iid == InterfaceIds.Calculator2
            || *iid == InterfaceIds.Base || *iid == InterfaceIds.Derived || *iid == InterfaceIds.LocalDerived)
            return ComRuntime.QueryInterface(self->Inner, *iid, out *result);
        *result = 0;
        return unchecked((int)0x80004002);
    }

    [UnmanagedCallersOnly]
    private static uint AddRef(NativeBridge* self) => (uint)++self->RefCount;

    [UnmanagedCallersOnly]
    private static uint Release(NativeBridge* self)
    {
        var count = --self->RefCount;
        if (count == 0)
        {
            ComRuntime.Release(self->Inner);
            NativeMemory.Free(self->Vtable);
            NativeMemory.Free(self);
        }
        return (uint)count;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct NativeBridge
    {
        public void** Vtable;
        public nint Inner;
        public int RefCount;
    }
}
