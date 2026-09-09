namespace Avalonia.Host;

internal static class HResults
{
    public const int S_OK = 0;
    public const int E_POINTER = unchecked((int)0x80004003);
    public const int E_FAIL = unchecked((int)0x80004005);
    public const int E_INVALIDARG = unchecked((int)0x80070057);
    public const int E_ABORT = unchecked((int)0x80004004);

    /// <summary>Fixture-only failure used to prove PreserveSig HRESULT passthrough.</summary>
    public const int AVN_E_FIXTURE = unchecked((int)0xA7A70001);

    /// <summary>
    /// A brush reached the ABI that is not an <c>ISolidColorBrush</c>. Only solid colour
    /// brushes are projected; gradients, drawing and visual brushes fail explicitly rather
    /// than degrading to a nearest colour.
    /// </summary>
    public const int AVN_E_NONSOLIDBRUSH = unchecked((int)0xA7A70002);
}
