using System;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.Linq;
using System.Reflection;
using System.Reflection.Emit;
using System.Runtime.InteropServices;

namespace Avalonia.Rust.BrowserCom;

/// <summary>Layout-compatible with <c>ComWrappers.ComInterfaceEntry</c>.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct ComInterfaceEntry
{
    public Guid IID;
    public nint Vtable;
}

/// <summary>Implemented by the shadow <c>IUnknownDerivedAttribute&lt;T, TImpl&gt;</c>.</summary>
public unsafe interface IComInterfaceDetails
{
    Guid Iid { get; }

    /// <summary>The generator's interface implementing the managed-to-native stubs as default methods.</summary>
    [DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
    Type Implementation { get; }

    void** ManagedVirtualMethodTable { get; }

    /// <summary>The declaring assembly's shadow <c>IUnmanagedVirtualMethodTableProvider</c>.</summary>
    Type VirtualMethodTableProvider { get; }
}

/// <summary>Implemented by the shadow <c>ComExposedClassAttribute&lt;T&gt;</c>.</summary>
public unsafe interface IComExposedDetails
{
    ComInterfaceEntry* GetComInterfaceEntries(out int count);
}

/// <summary>
/// Minimal COM callable wrapper (CCW) and runtime callable wrapper (RCW)
/// implementation for runtimes without <c>ComWrappers</c> (Mono browser-wasm).
/// </summary>
/// <remarks>
/// Browser-wasm is single threaded, but the implementation stays lock-guarded
/// so it is safe if threads are enabled later. RCWs are runtime-emitted
/// proxies deriving from <see cref="ComObjectBase"/> and implementing the
/// generator's implementation interfaces, whose default methods perform the
/// native calls. Because Mono has no <c>IDynamicInterfaceCastable</c>, a proxy
/// can only be cast to interfaces chosen when it was created: the requested
/// interface plus every interface of the same versioned family (for example
/// <c>IAvnRustViewModel</c>, <c>IAvnRustViewModel2</c>, ...) that the native
/// object answers through <c>QueryInterface</c>.
/// </remarks>
public static unsafe class ComRuntime
{
    private const int S_OK = 0;
    private const int E_NOINTERFACE = unchecked((int)0x80004002);
    private const int E_POINTER = unchecked((int)0x80004003);
    private static readonly Guid IID_IUnknown = new("00000000-0000-0000-C000-000000000046");
    private const string ShadowProviderName = "System.Runtime.InteropServices.Marshalling.IUnmanagedVirtualMethodTableProvider";

    // CCW block: [Header][Dispatch 0 = IUnknown][Dispatch 1..Count]. Native
    // callers receive Dispatch pointers; each points back to its header.
    [StructLayout(LayoutKind.Sequential)]
    private struct Header
    {
        public nint Handle;
        public int RefCount;
        public int Count;
        public ComInterfaceEntry* Entries;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct Dispatch
    {
        public void* Vtable;
        public Header* Owner;
    }

    private static readonly object s_gate = new();
    private static readonly Dictionary<object, nint> s_ccws = new(ReferenceEqualityComparer.Instance);
    private static readonly Dictionary<Type, IComInterfaceDetails?> s_details = new();
    private static readonly Dictionary<string, Type[]> s_families = new(StringComparer.Ordinal);
    private static readonly Dictionary<string, Func<nint, ComObjectBase>> s_proxyFactories = new(StringComparer.Ordinal);
    private static readonly Dictionary<(nint Identity, Type Type), WeakReference<ComObjectBase>> s_rcws = new();
    private static List<Type>? s_comInterfaces;
    private static int s_comInterfaceAssemblyCount;
    private static void** s_unknownVtable;
    private static ModuleBuilder? s_proxyModule;

    public static void GetIUnknownImpl(out nint fpQueryInterface, out nint fpAddRef, out nint fpRelease)
    {
        fpQueryInterface = (nint)(delegate* unmanaged<Dispatch*, Guid*, void**, int>)&CcwQueryInterface;
        fpAddRef = (nint)(delegate* unmanaged<Dispatch*, uint>)&CcwAddRef;
        fpRelease = (nint)(delegate* unmanaged<Dispatch*, uint>)&CcwRelease;
    }

    public static object GetCcwTarget(void* dispatch) =>
        GCHandle.FromIntPtr(((Dispatch*)dispatch)->Owner->Handle).Target
        ?? throw new ObjectDisposedException("COM callable wrapper");

    /// <summary>Returns an AddRef'ed <c>IUnknown</c> pointer for <paramref name="instance"/>.</summary>
    public static nint GetOrCreateComInterfaceForObject(object instance)
    {
        ArgumentNullException.ThrowIfNull(instance);
        if (instance is ComObjectBase rcw)
        {
            AddRef(rcw.Identity);
            return rcw.Identity;
        }

        lock (s_gate)
        {
            if (s_ccws.TryGetValue(instance, out var existing))
            {
                var header = (Header*)existing;
                header->RefCount++;
                return (nint)(header + 1);
            }

            ComInterfaceEntry* entries = null;
            var count = 0;
            foreach (var attribute in instance.GetType().GetCustomAttributes(inherit: true))
            {
                if (attribute is IComExposedDetails exposed)
                {
                    entries = exposed.GetComInterfaceEntries(out count);
                    break;
                }
            }

            var block = (Header*)NativeMemory.AllocZeroed((nuint)(sizeof(Header) + (count + 1) * sizeof(Dispatch)));
            block->Handle = GCHandle.ToIntPtr(GCHandle.Alloc(instance));
            block->RefCount = 1;
            block->Count = count;
            block->Entries = entries;
            var dispatches = (Dispatch*)(block + 1);
            dispatches[0] = new Dispatch { Vtable = UnknownVtable, Owner = block };
            for (var i = 0; i < count; i++)
                dispatches[i + 1] = new Dispatch { Vtable = (void*)entries[i].Vtable, Owner = block };
            s_ccws[instance] = (nint)block;
            return (nint)dispatches;
        }
    }

    /// <summary>
    /// Returns the managed object for a native COM pointer. A pointer to one
    /// of this runtime's CCWs unwraps to its managed object; any other pointer
    /// is wrapped in a proxy implementing <paramref name="interfaceType"/>.
    /// </summary>
    public static object GetOrCreateObjectForComInstance(nint pointer, Type interfaceType)
    {
        ThrowIfFailed(QueryInterface(pointer, IID_IUnknown, out var identity));
        try
        {
            if (IsCcw(identity))
                return GetCcwTarget((void*)identity);

            lock (s_gate)
            {
                var key = (identity, interfaceType);
                if (s_rcws.TryGetValue(key, out var weak) && weak.TryGetTarget(out var existing))
                    return existing;

                var rcw = CreateProxy(identity, interfaceType);
                s_rcws[key] = new WeakReference<ComObjectBase>(rcw);
                return rcw;
            }
        }
        finally
        {
            Release(identity);
        }
    }

    /// <summary>Returns an AddRef'ed pointer to <paramref name="interfaceType"/> on <paramref name="managed"/>.</summary>
    public static void* ConvertToUnmanaged(object managed, Type interfaceType)
    {
        var details = GetInterfaceDetails(interfaceType)
            ?? throw new InvalidCastException($"{interfaceType} is not a source-generated COM interface.");
        var unknown = GetOrCreateComInterfaceForObject(managed);
        try
        {
            ThrowIfFailed(QueryInterface(unknown, details.Iid, out var result));
            return (void*)result;
        }
        finally
        {
            Release(unknown);
        }
    }

    public static IComInterfaceDetails? GetInterfaceDetails(Type type)
    {
        lock (s_gate)
        {
            if (!s_details.TryGetValue(type, out var details))
            {
                foreach (var attribute in type.GetCustomAttributes(inherit: false))
                {
                    if (attribute is IComInterfaceDetails found)
                    {
                        details = found;
                        break;
                    }
                }
                s_details[type] = details;
            }
            return details;
        }
    }

    public static int QueryInterface(nint unknown, in Guid iid, out nint result)
    {
        fixed (Guid* pIid = &iid)
        fixed (nint* pResult = &result)
        {
            var queryInterface = (delegate* unmanaged<nint, Guid*, nint*, int>)(*(void***)unknown)[0];
            return queryInterface(unknown, pIid, pResult);
        }
    }

    public static uint AddRef(nint unknown) =>
        ((delegate* unmanaged<nint, uint>)(*(void***)unknown)[1])(unknown);

    public static uint Release(nint unknown) =>
        ((delegate* unmanaged<nint, uint>)(*(void***)unknown)[2])(unknown);

    internal static void ForgetRcw(nint identity, Type type)
    {
        lock (s_gate)
        {
            if (s_rcws.TryGetValue((identity, type), out var weak) && !weak.TryGetTarget(out _))
                s_rcws.Remove((identity, type));
        }
    }

    private static void** UnknownVtable
    {
        get
        {
            if (s_unknownVtable == null)
            {
                var vtable = (void**)NativeMemory.AllocZeroed((nuint)(3 * sizeof(void*)));
                GetIUnknownImpl(out var queryInterface, out var addRef, out var release);
                vtable[0] = (void*)queryInterface;
                vtable[1] = (void*)addRef;
                vtable[2] = (void*)release;
                s_unknownVtable = vtable;
            }
            return s_unknownVtable;
        }
    }

    private static bool IsCcw(nint unknown)
    {
        GetIUnknownImpl(out var queryInterface, out _, out _);
        return (nint)(*(void***)unknown)[0] == queryInterface;
    }

    [UnmanagedCallersOnly]
    private static int CcwQueryInterface(Dispatch* self, Guid* iid, void** result)
    {
        if (result == null)
            return E_POINTER;
        var header = self->Owner;
        var dispatches = (Dispatch*)(header + 1);
        if (*iid == IID_IUnknown)
        {
            *result = dispatches;
            header->RefCount++;
            return S_OK;
        }
        for (var i = 0; i < header->Count; i++)
        {
            if (header->Entries[i].IID == *iid)
            {
                *result = dispatches + 1 + i;
                header->RefCount++;
                return S_OK;
            }
        }
        *result = null;
        return E_NOINTERFACE;
    }

    [UnmanagedCallersOnly]
    private static uint CcwAddRef(Dispatch* self) => (uint)++self->Owner->RefCount;

    [UnmanagedCallersOnly]
    private static uint CcwRelease(Dispatch* self)
    {
        var header = self->Owner;
        var count = --header->RefCount;
        if (count == 0)
        {
            var handle = GCHandle.FromIntPtr(header->Handle);
            lock (s_gate)
            {
                if (handle.Target is { } target)
                    s_ccws.Remove(target);
            }
            handle.Free();
            NativeMemory.Free(header);
        }
        return (uint)count;
    }

    private static ComObjectBase CreateProxy(nint identity, Type interfaceType)
    {
        if (GetInterfaceDetails(interfaceType) is null)
            throw new InvalidCastException($"{interfaceType} is not a source-generated COM interface.");

        var supported = new List<Type>();
        foreach (var candidate in GetFamily(interfaceType))
        {
            if (candidate == interfaceType)
            {
                supported.Add(candidate);
                continue;
            }
            if (QueryInterface(identity, GetInterfaceDetails(candidate)!.Iid, out var pointer) >= 0 && pointer != 0)
            {
                Release(pointer);
                supported.Add(candidate);
            }
        }

        var key = string.Join(";", supported.Select(t => t.AssemblyQualifiedName));
        if (!s_proxyFactories.TryGetValue(key, out var factory))
        {
            factory = EmitProxy(supported);
            s_proxyFactories[key] = factory;
        }
        var proxy = factory(identity);
        proxy.Initialize(identity, interfaceType);
        return proxy;
    }

    /// <summary>
    /// The versioned capability family of <paramref name="interfaceType"/>:
    /// all source-generated COM interfaces whose names equal its name with
    /// any trailing version digits removed, optionally followed by digits.
    /// </summary>
    [UnconditionalSuppressMessage("Trimming", "IL2026", Justification = "Only interfaces kept by the trimmer can be requested.")]
    private static Type[] GetFamily(Type interfaceType)
    {
        var name = interfaceType.Name.TrimEnd('0', '1', '2', '3', '4', '5', '6', '7', '8', '9');
        var familyKey = interfaceType.Namespace + "." + name;
        if (s_families.TryGetValue(familyKey, out var family) && s_comInterfaceAssemblyCount == AppDomain.CurrentDomain.GetAssemblies().Length)
            return family;

        var assemblies = AppDomain.CurrentDomain.GetAssemblies();
        if (s_comInterfaces is null || s_comInterfaceAssemblyCount != assemblies.Length)
        {
            var interfaces = new List<Type>();
            foreach (var assembly in assemblies)
            {
                if (assembly.IsDynamic || assembly.GetType(ShadowProviderName) is null)
                    continue;
                Type?[] types;
                try
                {
                    types = assembly.GetTypes();
                }
                catch (ReflectionTypeLoadException e)
                {
                    types = e.Types;
                }
                foreach (var type in types)
                {
                    if (type is { IsInterface: true } && GetInterfaceDetails(type) is not null)
                        interfaces.Add(type);
                }
            }
            s_comInterfaces = interfaces;
            s_comInterfaceAssemblyCount = assemblies.Length;
            s_families.Clear();
        }

        family = s_comInterfaces
            .Where(t => t.Namespace == interfaceType.Namespace
                && t.Name.StartsWith(name, StringComparison.Ordinal)
                && t.Name.AsSpan(name.Length).IndexOfAnyExceptInRange('0', '9') < 0)
            .OrderBy(t => t.Name.Length)
            .ThenBy(t => t.Name, StringComparer.Ordinal)
            .ToArray();
        if (!family.Contains(interfaceType))
            family = [interfaceType, .. family];
        s_families[familyKey] = family;
        return family;
    }

    [UnconditionalSuppressMessage("Trimming", "IL2072", Justification = "Implementation types are annotated with DynamicallyAccessedMembers.All.")]
    [UnconditionalSuppressMessage("AOT", "IL3050", Justification = "Browser-wasm runs on Mono with Reflection.Emit support.")]
    private static Func<nint, ComObjectBase> EmitProxy(List<Type> interfaces)
    {
        if (s_proxyModule is null)
        {
            var assembly = AssemblyBuilder.DefineDynamicAssembly(
                new AssemblyName("Avalonia.Rust.BrowserCom.Proxies"), AssemblyBuilderAccess.Run);
            s_proxyModule = assembly.DefineDynamicModule("Avalonia.Rust.BrowserCom.Proxies");
        }

        var builder = s_proxyModule.DefineType(
            $"ComProxy{s_proxyFactories.Count}_{interfaces[0].Name}",
            TypeAttributes.Public | TypeAttributes.Sealed | TypeAttributes.Class,
            typeof(ComObjectBase));
        var providers = new HashSet<Type>();
        foreach (var type in interfaces)
        {
            var details = GetInterfaceDetails(type)!;
            builder.AddInterfaceImplementation(details.Implementation);
            if (providers.Add(details.VirtualMethodTableProvider))
                builder.AddInterfaceImplementation(details.VirtualMethodTableProvider);
        }
        builder.DefineDefaultConstructor(MethodAttributes.Public);
        var proxyType = builder.CreateType();
        return _ => (ComObjectBase)Activator.CreateInstance(proxyType)!;
    }

    private static void ThrowIfFailed(int hresult)
    {
        if (hresult < 0)
            Marshal.ThrowExceptionForHR(hresult);
    }
}

/// <summary>Base class of runtime-emitted RCW proxies.</summary>
public abstract class ComObjectBase
{
    private readonly Dictionary<Type, (nint This, nint Vtable)> _interfaces = new();
    private Type? _key;

    /// <summary>The AddRef'ed <c>IUnknown</c> identity pointer.</summary>
    public nint Identity { get; private set; }

    internal void Initialize(nint identity, Type key)
    {
        ComRuntime.AddRef(identity);
        Identity = identity;
        _key = key;
    }

    /// <summary>Returns the cached native interface pointer and vtable for <paramref name="type"/>.</summary>
    public unsafe (nint This, nint Vtable) GetInterface(Type type)
    {
        lock (_interfaces)
        {
            if (_interfaces.TryGetValue(type, out var entry))
                return entry;
            var details = ComRuntime.GetInterfaceDetails(type)
                ?? throw new InvalidCastException($"{type} is not a source-generated COM interface.");
            var hr = ComRuntime.QueryInterface(Identity, details.Iid, out var pointer);
            if (hr < 0)
                Marshal.ThrowExceptionForHR(hr);
            entry = (pointer, (nint)(*(void***)pointer));
            _interfaces[type] = entry;
            return entry;
        }
    }

    ~ComObjectBase()
    {
        if (Identity == 0)
            return;
        foreach (var (pointer, _) in _interfaces.Values)
            ComRuntime.Release(pointer);
        ComRuntime.Release(Identity);
        if (_key is not null)
            ComRuntime.ForgetRcw(Identity, _key);
    }
}
