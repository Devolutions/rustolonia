using System;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using Avalonia.Rust.BrowserCom;
using BrowserCom.Testing;
using Xunit;

[assembly: CollectionBehavior(DisableTestParallelization = true)]

namespace Avalonia.Rust.BrowserCom.Tests;

public unsafe class BrowserComTests
{
    private const int NoInterface = unchecked((int)0x80004002);
    private const int Calculator1 = 1;
    private const int Calculator2 = 2;
    private const int Calculator3 = 4;
    private const int Base = 8;
    private const int Derived = 16;
    private const int LocalDerived = 32;
    private const int Nested = 64;
    private const int Calculator4 = 128;
    private static readonly Guid s_unknown = new("00000000-0000-0000-C000-000000000046");
    private static readonly nint s_vtable = CreateVtable();

    [Fact]
    public void Managed_wrapper_preserves_identity_and_dispatches_cross_assembly_vtables()
    {
        var target = new Calculator();
        var first = ComRuntime.GetOrCreateComInterfaceForObject(target);
        var second = ComRuntime.GetOrCreateComInterfaceForObject(target);
        Assert.Equal(first, second);

        try
        {
            Assert.Equal(NoInterface, ComRuntime.QueryInterface(first, Guid.Empty, out var missing));
            Assert.Equal(0, missing);
            Assert.Equal(0, ComRuntime.QueryInterface(first, InterfaceIds.Calculator, out var calculator));
            try
            {
                var call = (delegate* unmanaged<nint, int, int*, int>)(*(void***)calculator)[3];
                var result = 0;
                Assert.Equal(0, call(calculator, 41, &result));
                Assert.Equal(42, result);
                Assert.Same(target, ComRuntime.GetOrCreateObjectForComInstance(calculator, typeof(ICalculator)));
            }
            finally
            {
                ComRuntime.Release(calculator);
            }

            Assert.Equal(0, ComRuntime.QueryInterface(first, InterfaceIds.Derived, out var derived));
            try
            {
                var baseCall = (delegate* unmanaged<nint, int, int*, int>)(*(void***)derived)[3];
                var derivedCall = (delegate* unmanaged<nint, int, int*, int>)(*(void***)derived)[4];
                var result = 0;
                Assert.Equal(0, baseCall(derived, 1, &result));
                Assert.Equal(11, result);
                Assert.Equal(0, derivedCall(derived, 1, &result));
                Assert.Equal(21, result);
            }
            finally
            {
                ComRuntime.Release(derived);
            }
        }
        finally
        {
            Assert.Equal(1u, ComRuntime.Release(second));
            Assert.Equal(0u, ComRuntime.Release(first));
        }
    }

    [Theory]
    [InlineData(Calculator1)]
    [InlineData(Calculator1 | Calculator2)]
    [InlineData(Calculator1 | Calculator3)]
    [InlineData(Calculator1 | Calculator2 | Calculator3)]
    public void Native_proxy_exposes_exact_supported_interface_combination(int supported)
    {
        var bridge = CreateBridge(new Calculator(), supported);
        try
        {
            var proxy = (ICalculator)ComRuntime.GetOrCreateObjectForComInstance((nint)bridge, typeof(ICalculator));
            Assert.Same(proxy, ComRuntime.GetOrCreateObjectForComInstance((nint)bridge, typeof(ICalculator)));
            Assert.Equal((supported & Calculator2) != 0, proxy is ICalculator2);
            Assert.Equal((supported & Calculator3) != 0, proxy is ICalculator3);
            Assert.False(proxy is ICalculator4);
            Assert.Equal(0, proxy.Increment(41, out var incremented));
            Assert.Equal(42, incremented);
            Assert.Equal(0, proxy.Echo("hello \u03bb", out var echoed));
            Assert.Equal("hello \u03bb", echoed);
            Assert.Equal(0, proxy.Echo(null, out echoed));
            Assert.Null(echoed);
            Assert.Equal(0, proxy.InterfaceImplementation(42, out var namedResult));
            Assert.Equal(42, namedResult);

            var other = new Calculator();
            Assert.Equal(0, proxy.RoundTrip(other, out var returned));
            Assert.Same(other, returned);

            if (proxy is ICalculator2 second)
            {
                Assert.Equal(0, second.Double(21, out var doubled));
                Assert.Equal(42, doubled);
            }
            if (proxy is ICalculator3 third)
            {
                Assert.Equal(0, third.Triple(14, out var tripled));
                Assert.Equal(42, tripled);
            }
        }
        finally
        {
            ComRuntime.Release((nint)bridge);
        }
    }

    [Theory]
    [InlineData(Calculator1 | Calculator4)]
    [InlineData(Calculator1 | Calculator2 | Calculator4)]
    [InlineData(Calculator1 | Calculator3 | Calculator4)]
    public void Native_proxy_combines_a_family_across_three_assemblies(int supported)
    {
        var bridge = CreateBridge(new CalculatorExtension(), supported);
        try
        {
            var proxy = (ICalculator)ComRuntime.GetOrCreateObjectForComInstance((nint)bridge, typeof(ICalculator));
            Assert.Equal((supported & Calculator2) != 0, proxy is ICalculator2);
            Assert.Equal((supported & Calculator3) != 0, proxy is ICalculator3);
            var fourth = Assert.IsAssignableFrom<ICalculator4>(proxy);
            Assert.Equal(0, fourth.Quadruple(10, out var value));
            Assert.Equal(40, value);
        }
        finally
        {
            ComRuntime.Release((nint)bridge);
        }
    }

    [Theory]
    [InlineData(Calculator2, false)]
    [InlineData(Calculator3, true)]
    [InlineData(Calculator2 | Calculator3, false)]
    [InlineData(Calculator2 | Calculator3, true)]
    public void Native_proxy_can_start_from_an_optional_interface(int supported, bool requestThird)
    {
        var bridge = CreateBridge(new Calculator(), supported);
        try
        {
            var requested = requestThird ? typeof(ICalculator3) : typeof(ICalculator2);
            var proxy = ComRuntime.GetOrCreateObjectForComInstance((nint)bridge, requested);
            Assert.False(proxy is ICalculator);
            Assert.Equal((supported & Calculator2) != 0, proxy is ICalculator2);
            Assert.Equal((supported & Calculator3) != 0, proxy is ICalculator3);
            if (proxy is ICalculator2 second)
            {
                Assert.Equal(0, second.Double(21, out var value));
                Assert.Equal(42, value);
            }
            if (proxy is ICalculator3 third)
            {
                Assert.Equal(0, third.Triple(14, out var value));
                Assert.Equal(42, value);
            }
        }
        finally
        {
            ComRuntime.Release((nint)bridge);
        }
    }

    [Fact]
    public void Native_proxy_dispatches_inherited_methods()
    {
        var bridge = CreateBridge(new Calculator(), Base | Derived | LocalDerived);
        try
        {
            var proxy = (ILocalDerived)ComRuntime.GetOrCreateObjectForComInstance((nint)bridge, typeof(ILocalDerived));
            Assert.Equal(0, proxy.LocalValue(12, out var local));
            Assert.Equal(42, local);
            Assert.Equal(0, proxy.DerivedValue(22, out var derived));
            Assert.Equal(42, derived);
            Assert.Equal(0, proxy.BaseValue(32, out var inherited));
            Assert.Equal(42, inherited);
        }
        finally
        {
            ComRuntime.Release((nint)bridge);
        }
    }

    [Fact]
    public void Native_proxy_supports_a_nested_public_interface()
    {
        var bridge = CreateBridge(new Calculator(), Nested);
        try
        {
            var proxy = (ContractContainer.INested)ComRuntime.GetOrCreateObjectForComInstance(
                (nint)bridge, typeof(ContractContainer.INested));
            Assert.Equal(0, proxy.NestedValue(2, out var value));
            Assert.Equal(42, value);
        }
        finally
        {
            ComRuntime.Release((nint)bridge);
        }
    }

    [Fact]
    public void Nested_private_managed_class_is_registered()
    {
        var target = NestedFixture.Create();
        var unknown = ComRuntime.GetOrCreateComInterfaceForObject(target);
        try
        {
            Assert.Equal(0, ComRuntime.QueryInterface(unknown, InterfaceIds.Calculator3, out var pointer));
            try
            {
                var call = (delegate* unmanaged<nint, int, int*, int>)(*(void***)pointer)[3];
                var result = 0;
                Assert.Equal(0, call(pointer, 14, &result));
                Assert.Equal(42, result);
            }
            finally
            {
                ComRuntime.Release(pointer);
            }
        }
        finally
        {
            Assert.Equal(0u, ComRuntime.Release(unknown));
        }
    }

    [Fact]
    public void Managed_class_nested_in_a_struct_is_registered()
    {
        var unknown = ComRuntime.GetOrCreateComInterfaceForObject(StructFixture.Create());
        try
        {
            Assert.Equal(0, ComRuntime.QueryInterface(unknown, InterfaceIds.Calculator3, out var pointer));
            ComRuntime.Release(pointer);
        }
        finally
        {
            Assert.Equal(0u, ComRuntime.Release(unknown));
        }
    }

    [Fact]
    public void Managed_wrapper_uses_the_most_derived_class_registration()
    {
        var unknown = ComRuntime.GetOrCreateComInterfaceForObject(new CalculatorDerived());
        try
        {
            Assert.Equal(0, ComRuntime.QueryInterface(unknown, InterfaceIds.Calculator2, out var derived));
            ComRuntime.Release(derived);
            Assert.Equal(0, ComRuntime.QueryInterface(unknown, InterfaceIds.Calculator3, out var inherited));
            ComRuntime.Release(inherited);
        }
        finally
        {
            Assert.Equal(0u, ComRuntime.Release(unknown));
        }
    }

    [Fact]
    public void Native_proxy_releases_identity_and_cached_interface_pointers()
    {
        var bridge = CreateBridge(new Calculator(), Calculator1 | Calculator2);
        try
        {
            var weak = UseProxy((nint)bridge);
            Assert.Equal(2, bridge->RefCount);
            for (var attempt = 0; attempt < 5 && (weak.IsAlive || bridge->RefCount != 1); attempt++)
            {
                GC.Collect();
                GC.WaitForPendingFinalizers();
                GC.Collect();
            }
            Assert.False(weak.IsAlive);
            Assert.Equal(1, bridge->RefCount);
            Assert.Equal(2u, ComRuntime.AddRef(bridge->Inner));
            Assert.Equal(1u, ComRuntime.Release(bridge->Inner));
        }
        finally
        {
            Assert.Equal(0u, ComRuntime.Release((nint)bridge));
        }
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    private static WeakReference UseProxy(nint bridge)
    {
        var proxy = (ICalculator)ComRuntime.GetOrCreateObjectForComInstance(bridge, typeof(ICalculator));
        Assert.Equal(0, proxy.Increment(41, out var result));
        Assert.Equal(42, result);
        return new WeakReference(proxy);
    }

    private static NativeBridge* CreateBridge(object target, int supported)
    {
        var bridge = (NativeBridge*)NativeMemory.AllocZeroed((nuint)sizeof(NativeBridge));
        bridge->Vtable = (void**)s_vtable;
        bridge->Inner = ComRuntime.GetOrCreateComInterfaceForObject(target);
        bridge->RefCount = 1;
        bridge->Supported = supported;
        return bridge;
    }

    private static nint CreateVtable()
    {
        var vtable = (void**)NativeMemory.AllocZeroed((nuint)(3 * sizeof(void*)));
        vtable[0] = (void*)(delegate* unmanaged<NativeBridge*, Guid*, nint*, int>)&QueryInterface;
        vtable[1] = (void*)(delegate* unmanaged<NativeBridge*, uint>)&AddRef;
        vtable[2] = (void*)(delegate* unmanaged<NativeBridge*, uint>)&Release;
        return (nint)vtable;
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

        var flag = *iid == InterfaceIds.Calculator ? Calculator1
            : *iid == InterfaceIds.Calculator2 ? Calculator2
            : *iid == InterfaceIds.Calculator3 ? Calculator3
            : *iid == InterfaceIds.Calculator4 ? Calculator4
            : *iid == InterfaceIds.Base ? Base
            : *iid == InterfaceIds.Derived ? Derived
            : *iid == InterfaceIds.LocalDerived ? LocalDerived
            : *iid == InterfaceIds.Nested ? Nested
            : 0;
        if ((self->Supported & flag) != 0)
            return ComRuntime.QueryInterface(self->Inner, *iid, out *result);
        *result = 0;
        return NoInterface;
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
        public int Supported;
    }
}
