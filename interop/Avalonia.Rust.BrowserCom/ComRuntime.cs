using System;
using System.Collections.Generic;
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

    void** ManagedVirtualMethodTable { get; }
}

/// <summary>Marks an assembly whose browser COM registrations are available to the build-time generator.</summary>
[AttributeUsage(AttributeTargets.Assembly)]
public sealed class BrowserComAssemblyAttribute(string registrationType) : Attribute
{
    public string RegistrationType { get; } = registrationType;
}

public unsafe delegate ComInterfaceEntry* ComInterfaceEntriesFactory(out int count);

/// <summary>Supplies a native instance pointer and vtable to a generated COM proxy.</summary>
public readonly unsafe struct ComVirtualMethodTableInfo(void* thisPointer, void** vtable)
{
    public void Deconstruct(out void* pointer, out void** table)
    {
        pointer = thisPointer;
        table = vtable;
    }
}

/// <summary>Implemented by generated proxy types.</summary>
public unsafe interface IComVirtualMethodTableProvider
{
    ComVirtualMethodTableInfo GetVirtualMethodTableInfoForKey(Type type)
    {
        var (pointer, vtable) = ((ComObjectBase)this).GetInterface(type);
        return new ComVirtualMethodTableInfo((void*)pointer, (void**)vtable);
    }
}

/// <summary>
/// Minimal COM callable wrapper (CCW) and runtime callable wrapper (RCW)
/// implementation for runtimes without <c>ComWrappers</c> (Mono browser-wasm).
/// </summary>
/// <remarks>
/// Browser-wasm is currently single threaded; registration and wrapper
/// caches are lock-guarded, but CCW reference counts assume one thread.
/// RCW proxy types and COM registrations are generated at build time. Because
/// Mono has no <c>IDynamicInterfaceCastable</c>, a proxy implements the
/// requested interface and the supported interfaces from its versioned family.
/// </remarks>
public static unsafe class ComRuntime
{
    private const int S_OK = 0;
    private const int E_NOINTERFACE = unchecked((int)0x80004002);
    private const int E_POINTER = unchecked((int)0x80004003);
    private static readonly Guid IID_IUnknown = new("00000000-0000-0000-C000-000000000046");
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
    private static readonly Dictionary<Type, InterfaceRegistration> s_interfaces = new();
    private static readonly Dictionary<string, List<InterfaceRegistration>> s_families = new(StringComparer.Ordinal);
    private static readonly Dictionary<string, List<FamilyRegistration>> s_proxyFactories = new(StringComparer.Ordinal);
    private static readonly List<ClassRegistration> s_classes = new();
    private static readonly Dictionary<(nint Identity, Type Type), WeakReference<ComObjectBase>> s_rcws = new();
    private static void** s_unknownVtable;

    private sealed record InterfaceRegistration(Type Type, IComInterfaceDetails Details, string Family, string Identity);
    private sealed record FamilyRegistration(Type[] Interfaces, Func<ulong, ComObjectBase> Create);
    private sealed record ClassRegistration(Func<object, bool> Matches, int Depth, ComInterfaceEntriesFactory Entries);

    /// <summary>Registers a source-generated COM interface and its versioned family.</summary>
    public static void RegisterInterface(Type type, IComInterfaceDetails details, string family, string identity)
    {
        ArgumentNullException.ThrowIfNull(type);
        ArgumentNullException.ThrowIfNull(details);
        lock (s_gate)
        {
            if (s_interfaces.ContainsKey(type))
                throw new InvalidOperationException($"Browser COM interface '{type}' was registered more than once.");
            var registration = new InterfaceRegistration(type, details, family, identity);
            s_interfaces.Add(type, registration);
            if (!s_families.TryGetValue(family, out var members))
                s_families.Add(family, members = new List<InterfaceRegistration>());
            members.Add(registration);
            members.Sort(static (left, right) => StringComparer.Ordinal.Compare(left.Identity, right.Identity));
        }
    }

    /// <summary>Registers the proxy factory for one complete interface family.</summary>
    public static void RegisterFamily(string family, Type[] interfaces, Func<ulong, ComObjectBase> factory)
    {
        lock (s_gate)
        {
            if (!s_proxyFactories.TryGetValue(family, out var factories))
                s_proxyFactories.Add(family, factories = new List<FamilyRegistration>());
            factories.Add(new FamilyRegistration(interfaces, factory));
        }
    }

    /// <summary>Registers the generated native vtables for a managed COM class.</summary>
    public static void RegisterClass<T>(ComInterfaceEntriesFactory entries, int depth) where T : class
    {
        lock (s_gate)
        {
            s_classes.Add(new ClassRegistration(static instance => instance is T, depth, entries));
            s_classes.Sort(static (left, right) => right.Depth.CompareTo(left.Depth));
        }
    }

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
            foreach (var registration in s_classes)
            {
                if (registration.Matches(instance))
                {
                    entries = registration.Entries(out count);
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
            return s_interfaces.TryGetValue(type, out var registration) ? registration.Details : null;
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
        if (!s_interfaces.TryGetValue(interfaceType, out var requested))
            throw new InvalidCastException($"{interfaceType} is not a source-generated COM interface.");

        var members = s_families[requested.Family];
        FamilyRegistration? family = null;
        if (s_proxyFactories.TryGetValue(requested.Family, out var factories))
        {
            foreach (var candidate in factories)
            {
                if (candidate.Interfaces.Length != members.Count)
                    continue;
                var match = true;
                for (var i = 0; i < members.Count; i++)
                {
                    if (candidate.Interfaces[i] != members[i].Type)
                    {
                        match = false;
                        break;
                    }
                }
                if (match)
                {
                    family = candidate;
                    break;
                }
            }
        }
        if (family is null)
            throw new InvalidOperationException($"No generated browser COM proxy covers interface family '{requested.Family}'. Rebuild the browser host with all assemblies in that family referenced.");

        ulong supported = 0;
        for (var i = 0; i < members.Count; i++)
        {
            var candidate = members[i];
            if (candidate.Type == interfaceType)
            {
                supported |= 1UL << i;
            }
            else if (QueryInterface(identity, candidate.Details.Iid, out var pointer) >= 0 && pointer != 0)
            {
                Release(pointer);
                supported |= 1UL << i;
            }
        }

        var proxy = family.Create(supported);
        proxy.Initialize(identity, interfaceType);
        return proxy;
    }

    private static void ThrowIfFailed(int hresult)
    {
        if (hresult < 0)
            Marshal.ThrowExceptionForHR(hresult);
    }
}

/// <summary>Base class of source-generated RCW proxies.</summary>
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
