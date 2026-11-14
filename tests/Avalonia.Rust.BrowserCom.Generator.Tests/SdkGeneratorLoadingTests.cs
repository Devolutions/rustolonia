using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Reflection.Emit;
using Avalonia.Rust.BrowserCom.Generator;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.Diagnostics;
using Xunit;

namespace Avalonia.Rust.BrowserCom.Generator.Tests;

public class SdkGeneratorLoadingTests
{
    [Fact]
    public void Sdk_generators_are_isolated_from_preloaded_interop_helpers()
    {
        AssemblyBuilder.DefineDynamicAssembly(
            new AssemblyName("Microsoft.Interop.SourceGeneration") { Version = new Version(99, 0, 0, 0) },
            AssemblyBuilderAccess.Run);

        var sdkPath = typeof(SdkGeneratorLoadingTests).Assembly.GetCustomAttributes<AssemblyMetadataAttribute>()
            .Single(attribute => attribute.Key == "SdkGeneratorPath").Value!;
        var references = ((string)AppContext.GetData("TRUSTED_PLATFORM_ASSEMBLIES")!)
            .Split(Path.PathSeparator)
            .Select(path => MetadataReference.CreateFromFile(path));
        var compilation = CSharpCompilation.Create(
            "Fixture",
            new[]
            {
                CSharpSyntaxTree.ParseText("""
                    using System.Runtime.InteropServices;
                    using System.Runtime.InteropServices.Marshalling;
                    [GeneratedComInterface]
                    [Guid("1B56B7A2-F7C9-49D7-9DD9-69507F6520DB")]
                    public partial interface ICalculator
                    {
                        int Increment(int value);
                    }
                    """, cancellationToken: TestContext.Current.CancellationToken)
            },
            references,
            new CSharpCompilationOptions(OutputKind.DynamicallyLinkedLibrary, allowUnsafe: true));

        // Run twice to exercise reuse in a long-lived compiler process.
        for (var attempt = 0; attempt < 2; attempt++)
        {
            GeneratorDriver driver = CSharpGeneratorDriver.Create(
                new[] { new BrowserComGenerator() },
                optionsProvider: new SdkOptionsProvider(sdkPath));
            var result = driver.RunGenerators(compilation, TestContext.Current.CancellationToken).GetRunResult();

            Assert.Empty(result.Diagnostics.Where(diagnostic => diagnostic.Severity == DiagnosticSeverity.Error));
            var generated = Assert.Single(result.Results);
            Assert.Null(generated.Exception);
            Assert.Contains(generated.GeneratedSources, source =>
                source.HintName == "SdkCom_0_ICalculator.cs"
                && source.SourceText.ToString().Contains("__RustoloniaBrowserComImpl_", StringComparison.Ordinal));
        }
    }

    private sealed class SdkOptionsProvider(string path) : AnalyzerConfigOptionsProvider
    {
        public override AnalyzerConfigOptions GlobalOptions { get; } = new Options(
            new Dictionary<string, string> { ["build_property.BrowserComSdkGeneratorPath"] = path });
        public override AnalyzerConfigOptions GetOptions(SyntaxTree tree) => new Options(ImmutableDictionary<string, string>.Empty);
        public override AnalyzerConfigOptions GetOptions(AdditionalText textFile) => new Options(ImmutableDictionary<string, string>.Empty);
    }

    private sealed class Options(IReadOnlyDictionary<string, string> values) : AnalyzerConfigOptions
    {
        public override bool TryGetValue(string key, out string value) => values.TryGetValue(key, out value!);
    }
}
