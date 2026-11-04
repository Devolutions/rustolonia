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
                (T)global::Avalonia.Rust.BrowserCom.ComRuntime.GetCcwTarget(dispatchPtr);
        }

        public static void GetIUnknownImpl(out nint fpQueryInterface, out nint fpAddRef, out nint fpRelease) =>
            global::Avalonia.Rust.BrowserCom.ComRuntime.GetIUnknownImpl(out fpQueryInterface, out fpAddRef, out fpRelease);
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
        void** ManagedVirtualMethodTable { get; }
    }

    [AttributeUsage(AttributeTargets.Interface)]
    internal sealed unsafe class IUnknownDerivedAttribute<T, TImpl>
        : Attribute, IIUnknownDerivedDetails, global::Avalonia.Rust.BrowserCom.IComInterfaceDetails
        where T : IIUnknownInterfaceType
    {
        public Guid Iid => T.Iid;
        public void** ManagedVirtualMethodTable => T.ManagedVirtualMethodTable;
    }

    internal unsafe interface IComExposedClass
    {
        static abstract ComWrappers.ComInterfaceEntry* GetComInterfaceEntries(out int count);
    }

    [AttributeUsage(AttributeTargets.Class)]
    internal sealed class ComExposedClassAttribute<T> : Attribute
        where T : IComExposedClass
    {
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
        // Generated vtable-index proxies derive from ComObjectBase; the
        // default method supplies their native instance pointer and vtable.
        VirtualMethodTableInfo GetVirtualMethodTableInfoForKey(Type type)
        {
            var (thisPointer, vtable) = ((global::Avalonia.Rust.BrowserCom.ComObjectBase)this).GetInterface(type);
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
            global::Avalonia.Rust.BrowserCom.ComRuntime.GetOrCreateComInterfaceForObject(instance);

        private sealed class DetailsStrategy : IIUnknownInterfaceDetailsStrategy
        {
            public IIUnknownDerivedDetails? GetIUnknownDerivedDetails(RuntimeTypeHandle type)
            {
                var details = global::Avalonia.Rust.BrowserCom.ComRuntime.GetInterfaceDetails(Type.GetTypeFromHandle(type)!);
                return details is null ? null : new InterfaceDetails(details);
            }
        }

        private sealed unsafe class InterfaceDetails(global::Avalonia.Rust.BrowserCom.IComInterfaceDetails details) : IIUnknownDerivedDetails
        {
            public Guid Iid => details.Iid;
            public void** ManagedVirtualMethodTable => details.ManagedVirtualMethodTable;
        }
    }

    [CustomMarshaller(typeof(CustomMarshallerAttribute.GenericPlaceholder), MarshalMode.Default, typeof(ComInterfaceMarshaller<>))]
    internal static unsafe class ComInterfaceMarshaller<T> where T : class
    {
        public static void* ConvertToUnmanaged(T? managed) =>
            managed is null ? null : global::Avalonia.Rust.BrowserCom.ComRuntime.ConvertToUnmanaged(managed, typeof(T));

        public static T? ConvertToManaged(void* unmanaged) =>
            unmanaged is null ? null : (T)global::Avalonia.Rust.BrowserCom.ComRuntime.GetOrCreateObjectForComInstance((nint)unmanaged, typeof(T));

        public static void Free(void* unmanaged)
        {
            if (unmanaged is not null)
                global::Avalonia.Rust.BrowserCom.ComRuntime.Release((nint)unmanaged);
        }
    }

    [CustomMarshaller(typeof(Exception), MarshalMode.UnmanagedToManagedOut, typeof(ExceptionAsHResultMarshaller<>))]
    internal static class ExceptionAsHResultMarshaller<T> where T : unmanaged, Numerics.INumber<T>
    {
        public static T ConvertToUnmanaged(Exception e) => T.CreateTruncating(e.HResult);
    }
}
