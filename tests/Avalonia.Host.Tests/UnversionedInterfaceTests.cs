using System;
using System.Linq;
using System.Runtime.InteropServices;
using Avalonia.Host.Com;
using Avalonia.Rust.Interop;
using Xunit;

namespace Avalonia.Host.Tests;

public class UnversionedInterfaceTests
{
    // The host and the Rust crates always ship together (Host::load rejects a
    // mismatched ABI header hash), so an interface is changed in place rather
    // than republished as a numbered successor such as IAvnFoo2.
    [Fact]
    public void No_com_interface_is_a_numbered_successor()
    {
        var numbered = new[] { typeof(IAvnApplication).Assembly, typeof(IAvnRustViewModel).Assembly }
            .SelectMany(assembly => assembly.GetTypes())
            .Where(type => type.IsInterface && type.IsDefined(typeof(GuidAttribute), inherit: false))
            .Select(type => type.FullName!)
            .Where(name => char.IsAsciiDigit(name[^1]))
            .Order(StringComparer.Ordinal)
            .ToArray();

        Assert.Empty(numbered);
    }
}
