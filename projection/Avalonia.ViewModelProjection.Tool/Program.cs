using System;
using System.Collections.Generic;
using System.IO;
using Avalonia.Projection.Generator;
using Avalonia.Projection.Ir;

if (GeneratorCli.ContainsCheckAndNormalize(args))
{
    Console.Error.WriteLine(GeneratorCli.CheckWithNormalizeMessage);
    return 2;
}

var checkMode = GeneratorCli.TryStripCheck(args, out args);

// `--normalize <view-model-ir>` rewrites the schema file in its canonical
// serialized form. The checked-in schema is asserted to round-trip byte for
// byte, so this is how a hand-edited schema is brought back to canonical order
// after new members are added.
if (args.Length == 2 && args[0] == "--normalize")
{
    var file = Path.GetFullPath(args[1]);
    var normalized = ViewModelIr.FromJson(File.ReadAllText(file));
    File.WriteAllText(file, normalized.ToJson().Replace(Environment.NewLine, "\n") + "\n");
    Console.WriteLine($"Normalized {file}.");
    return 0;
}

var externalRust = args.Length == 6 && args[5] == "--external-rust";
if (args.Length != 5 && !externalRust)
{
    Console.Error.WriteLine(
        "Usage: Avalonia.ViewModelProjection.Tool [--check] <view-model-ir> <adapter-output-directory> <registry-output-directory> <rust-output> <contract-output> [--external-rust]");
    Console.Error.WriteLine(
        "       Avalonia.ViewModelProjection.Tool --normalize <view-model-ir>");
    return 2;
}

var ir = ViewModelIr.FromJson(File.ReadAllText(Path.GetFullPath(args[0])));
var adapterDirectory = Path.GetFullPath(args[1]);
var registryDirectory = Path.GetFullPath(args[2]);
var rustPath = Path.GetFullPath(args[3]);
var contractPath = Path.GetFullPath(args[4]);
var outputDirectories = new[] { adapterDirectory, registryDirectory, Path.GetDirectoryName(rustPath)!, Path.GetDirectoryName(contractPath)! };

var expectedFiles = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
foreach (var (name, source) in ViewModelSourceEmitter.EmitCSharp(ir))
{
    var directory = name == "RustViewRegistry.g.cs"
        ? registryDirectory
        : adapterDirectory;
    OwnedOutputs.Add(expectedFiles, Path.Combine(directory, name), source.Replace(Environment.NewLine, "\n"));
}
OwnedOutputs.Add(expectedFiles, rustPath, ViewModelSourceEmitter.EmitRust(ir, externalRust).Replace(Environment.NewLine, "\n"));
OwnedOutputs.Add(expectedFiles, contractPath, ViewModelSourceEmitter.EmitContract(ir).Replace(Environment.NewLine, "\n"));

if (checkMode)
{
    var result = OwnedOutputs.Check(OwnedOutputs.ViewModelGeneratorId, expectedFiles, outputDirectories);
    if (result.Success)
    {
        Console.WriteLine($"Generation check passed for {expectedFiles.Count} output file(s).");
        return 0;
    }

    foreach (var mismatch in result.Mismatches)
        Console.Error.WriteLine(mismatch);
    return 1;
}

OwnedOutputs.Write(OwnedOutputs.ViewModelGeneratorId, expectedFiles, outputDirectories);

Console.WriteLine(
    $"Generated {ir.Models.Count} view model(s) and {ir.Views.Count} view(s).");
return 0;
