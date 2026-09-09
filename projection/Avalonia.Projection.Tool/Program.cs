using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Projection.Generator;
using Avalonia.Projection.Ir;

var checkMode = args.Length > 0 && args[0] == "--check";
if (checkMode)
    args = args[1..];

if (args.Length is not (2 or 3))
{
    Console.Error.WriteLine(
        "Usage: Avalonia.Projection.Tool [--check] <ir-output> <csharp-output-directory> [native-header-output]");
    return 2;
}

var sourceTypes = typeof(AvaloniaObject).Assembly.GetExportedTypes()
    .Concat(typeof(Control).Assembly.GetExportedTypes());
var ir = ClrTypeExtractor.Extract(sourceTypes, AvaloniaProjectionProfiles.ObjectModelKernel);
var irPath = Path.GetFullPath(args[0]);
var csharpDirectory = Path.GetFullPath(args[1]);
var outputDirectories = new List<string> { csharpDirectory, Path.GetDirectoryName(irPath)! };
var expectedFiles = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);

foreach (var (name, source) in ComSourceEmitter.Emit(ir))
    OwnedOutputs.Add(expectedFiles, Path.Combine(csharpDirectory, name), source.Replace(Environment.NewLine, "\n"));

var reportPath = Path.ChangeExtension(irPath, ".gaps.txt");
var reportText = string.Join("\n", ir.Skipped.Select(s => $"{s.Owner}.{s.Member}: {s.Reason}")) + "\n";

OwnedOutputs.Add(expectedFiles, irPath, ir.ToJson().Replace(Environment.NewLine, "\n") + "\n");
OwnedOutputs.Add(expectedFiles, reportPath, reportText);

if (args.Length == 3)
{
    var headerPath = Path.GetFullPath(args[2]);
    outputDirectories.Add(Path.GetDirectoryName(headerPath)!);
    OwnedOutputs.Add(expectedFiles, headerPath, NativeHeaderEmitter.Emit(ir).Replace(Environment.NewLine, "\n"));
}

if (checkMode)
{
    var result = OwnedOutputs.Check(OwnedOutputs.ProjectionGeneratorId, expectedFiles, outputDirectories);
    if (result.Success)
    {
        Console.WriteLine($"Generation check passed for {expectedFiles.Count} output file(s).");
        return 0;
    }

    foreach (var mismatch in result.Mismatches)
        Console.Error.WriteLine(mismatch);
    return 1;
}

OwnedOutputs.Write(OwnedOutputs.ProjectionGeneratorId, expectedFiles, outputDirectories);

Console.WriteLine($"Generated {ir.Types.Count} projected types and {ir.Skipped.Count} gap entries.");
return 0;
