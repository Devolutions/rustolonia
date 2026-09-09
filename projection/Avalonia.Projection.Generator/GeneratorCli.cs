namespace Avalonia.Projection.Generator;

public static class GeneratorCli
{
    public const string CheckWithNormalizeMessage = "Cannot combine --check with --normalize.";

    public static bool ContainsCheckAndNormalize(IReadOnlyList<string> args) =>
        args.Any(argument => argument == "--check") && args.Any(argument => argument == "--normalize");

    public static bool TryStripCheck(string[] args, out string[] remaining)
    {
        if (args.Length > 0 && args[0] == "--check")
        {
            remaining = args[1..];
            return true;
        }

        remaining = args;
        return false;
    }
}
