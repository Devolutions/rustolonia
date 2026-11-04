// Shadow types for the .NET COM source generator on browser-wasm.
//
// The COM source generator emits fully qualified references to the BCL types
// below. Mono on browser-wasm does not implement ComWrappers (every API throws
// PlatformNotSupportedException) nor IDynamicInterfaceCastable, so the BCL
// implementations cannot back the generated code. build/BrowserCom.targets
// compiles this file into each COM-generating assembly for the browser target
// framework; a source type wins over an imported one (CS0436), so the generated
// code binds to these internal shadows, which forward to
// Avalonia.Rust.BrowserCom.ComRuntime.
//
// Only the surface used by the generator output and by Avalonia.Host is
// provided. Keep it in sync with the generator version pinned by the SDK.
#nullable enable
#pragma warning disable CS0436
#pragma warning disable CS0649

using System.Diagnostics.CodeAnalysis;
using BrowserCom = Avalonia.Rust.BrowserCom;

namespace System.Runtime.InteropServices
{
    internal abstract unsafe class ComWrappers
    {
        public struct ComInterfaceEntry
        {
            public Guid IID;
            public nint Vtable;
        }

        public struct ComInterfaceDispatch
        {
            public void* Vtable;

            public static T GetInstance<T>(ComInterfaceDispatch* dispatchPtr) where T : class =>
                (T)BrowserCom.ComRuntime.GetCcwTarget(dispatchPtr);
        }

        public static void GetIUnknownImpl(out nint fpQueryInterface, out nint fpAddRef, out nint fpRelease) =>
            BrowserCom.ComRuntime.GetIUnknownImpl(out fpQueryInterface, out fpAddRef, out fpRelease);
    }
}

namespace System.Runtime.InteropServices.Marshalling
{
    internal interface IIUnknownInterfaceType
    {
        static abstract Guid Iid { get; }
        static abstract unsafe void** ManagedVirtualMethodTable { get; }
    }

    internal unsafe interface IIUnknownDerivedDetails
    {
        Guid Iid { get; }
        [DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
        Type Implementation { get; }
        void** ManagedVirtualMethodTable { get; }
    }

    [AttributeUsage(AttributeTargets.Interface)]
    internal sealed unsafe class IUnknownDerivedAttribute<T, [DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)] TImpl>
        : Attribute, IIUnknownDerivedDetails, BrowserCom.IComInterfaceDetails
        where T : IIUnknownInterfaceType
    {
        public Guid Iid => T.Iid;

        [DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
        public Type Implementation => typeof(TImpl);

        public void** ManagedVirtualMethodTable => T.ManagedVirtualMethodTable;

        public Type VirtualMethodTableProvider => typeof(IUnmanagedVirtualMethodTableProvider);
    }

    internal unsafe interface IComExposedClass
    {
        static abstract ComWrappers.ComInterfaceEntry* GetComInterfaceEntries(out int count);
    }

    [AttributeUsage(AttributeTargets.Class)]
    internal sealed unsafe class ComExposedClassAttribute<T> : Attribute, BrowserCom.IComExposedDetails
        where T : IComExposedClass
    {
        public BrowserCom.ComInterfaceEntry* GetComInterfaceEntries(out int count) =>
            (BrowserCom.ComInterfaceEntry*)T.GetComInterfaceEntries(out count);
    }

    internal readonly unsafe struct VirtualMethodTableInfo
    {
        public VirtualMethodTableInfo(void* thisPointer, void** virtualMethodTable)
        {
            ThisPointer = thisPointer;
            VirtualMethodTable = virtualMethodTable;
        }

        public void* ThisPointer { get; }
        public void** VirtualMethodTable { get; }

        public void Deconstruct(out void* thisPointer, out void** virtualMethodTable)
        {
            thisPointer = ThisPointer;
            virtualMethodTable = VirtualMethodTable;
        }
    }

    internal unsafe interface IUnmanagedVirtualMethodTableProvider
    {
        // Runtime-emitted RCW proxies derive from ComObjectBase and implement
        // this interface without bodies; the default method does the lookup.
        VirtualMethodTableInfo GetVirtualMethodTableInfoForKey(Type type)
        {
            var (thisPointer, vtable) = ((BrowserCom.ComObjectBase)this).GetInterface(type);
            return new VirtualMethodTableInfo((void*)thisPointer, (void**)vtable);
        }
    }

    internal interface IIUnknownInterfaceDetailsStrategy
    {
        IIUnknownDerivedDetails? GetIUnknownDerivedDetails(RuntimeTypeHandle type);
    }

    internal class StrategyBasedComWrappers
    {
        public static IIUnknownInterfaceDetailsStrategy DefaultIUnknownInterfaceDetailsStrategy { get; } =
            new DetailsStrategy();

        public nint GetOrCreateComInterfaceForObject(object instance, CreateComInterfaceFlags flags) =>
            BrowserCom.ComRuntime.GetOrCreateComInterfaceForObject(instance);

        private sealed class DetailsStrategy : IIUnknownInterfaceDetailsStrategy
        {
            public IIUnknownDerivedDetails? GetIUnknownDerivedDetails(RuntimeTypeHandle type) =>
                BrowserCom.ComRuntime.GetInterfaceDetails(Type.GetTypeFromHandle(type)!) as IIUnknownDerivedDetails;
        }
    }

    [CustomMarshaller(typeof(CustomMarshallerAttribute.GenericPlaceholder), MarshalMode.Default, typeof(ComInterfaceMarshaller<>))]
    internal static unsafe class ComInterfaceMarshaller<T> where T : class
    {
        public static void* ConvertToUnmanaged(T? managed) =>
            managed is null ? null : BrowserCom.ComRuntime.ConvertToUnmanaged(managed, typeof(T));

        public static T? ConvertToManaged(void* unmanaged) =>
            unmanaged is null ? null : (T)BrowserCom.ComRuntime.GetOrCreateObjectForComInstance((nint)unmanaged, typeof(T));

        public static void Free(void* unmanaged)
        {
            if (unmanaged is not null)
                BrowserCom.ComRuntime.Release((nint)unmanaged);
        }
    }

    [CustomMarshaller(typeof(Exception), MarshalMode.UnmanagedToManagedOut, typeof(ExceptionAsHResultMarshaller<>))]
    internal static class ExceptionAsHResultMarshaller<T> where T : unmanaged, Numerics.INumber<T>
    {
        public static T ConvertToUnmanaged(Exception e) => T.CreateTruncating(e.HResult);
    }
}
