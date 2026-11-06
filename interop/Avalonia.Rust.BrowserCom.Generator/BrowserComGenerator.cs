using System;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.Loader;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;
using Microsoft.CodeAnalysis.Text;

namespace Avalonia.Rust.BrowserCom.Generator;

/// <summary>
/// Runs the SDK's COM generators at build time, makes their file-local proxy
/// interfaces available to static proxy types, and emits their registrations.
/// </summary>
[Generator(LanguageNames.CSharp)]
public sealed class BrowserComGenerator : ISourceGenerator
{
    private const string GeneratedInterfaceAttribute = "System.Runtime.InteropServices.Marshalling.GeneratedComInterfaceAttribute";
    private const string GeneratedClassAttribute = "System.Runtime.InteropServices.Marshalling.GeneratedComClassAttribute";
    private const string BrowserAssemblyAttribute = "Avalonia.Rust.BrowserCom.BrowserComAssemblyAttribute";
    private const string OriginalProvider = "global::System.Runtime.InteropServices.Marshalling.IUnmanagedVirtualMethodTableProvider";
    private const string BrowserProvider = "global::Avalonia.Rust.BrowserCom.IComVirtualMethodTableProvider";
    // A family's proxies cover every subset of its interfaces (2^n types), so
    // compile time and memory double with each member.
    private const int MaximumFamilySize = 12;
    private static readonly object s_loadGate = new();
    private static readonly DiagnosticDescriptor s_failure = new(
        "BRCOM001",
        "Browser COM generation failed",
        "{0}",
        "BrowserCom",
        DiagnosticSeverity.Error,
        isEnabledByDefault: true);

    public void Initialize(GeneratorInitializationContext context) { }

    public void Execute(GeneratorExecutionContext context)
    {
        try
        {
            ExecuteCore(context);
        }
        catch (OperationCanceledException) when (context.CancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception error)
        {
            context.ReportDiagnostic(Diagnostic.Create(s_failure, Location.None, error.ToString()));
        }
    }

    private static void ExecuteCore(GeneratorExecutionContext context)
    {
        var localInterfaces = FindAttributedTypes(context.Compilation, GeneratedInterfaceAttribute, context.CancellationToken);
        var localClasses = FindAttributedTypes(context.Compilation, GeneratedClassAttribute, context.CancellationToken);
        var interfaceHints = MakeHintMap(localInterfaces, requireAccessible: true);
        var classHints = MakeHintMap(localClasses, requireAccessible: false);
        var interfaces = localInterfaces.Select(static symbol => InterfaceInfo.CreateLocal(symbol)).ToList();
        var classes = localClasses.Select(static symbol => ClassInfo.Create(symbol)).ToList();
        var interfacesByHint = interfaces.ToDictionary(static info => HintName(info.Symbol), StringComparer.Ordinal);
        var classesByHint = classes.ToDictionary(static info => HintName(info.Symbol), StringComparer.Ordinal);
        var referencedRegistrations = FindReferencedRegistrations(context.Compilation, interfaces);
        var interfacesBySymbol = new Dictionary<INamedTypeSymbol, InterfaceInfo>(SymbolEqualityComparer.Default);
        foreach (var info in interfaces)
            interfacesBySymbol.Add(info.Symbol, info);
        var seenInterfaces = new HashSet<string>(StringComparer.Ordinal);
        var seenClasses = new HashSet<string>(StringComparer.Ordinal);

        var generators = LoadSdkGenerators(context);
        GeneratorDriver driver = CSharpGeneratorDriver.Create(
            generators,
            additionalTexts: context.AdditionalFiles,
            parseOptions: (CSharpParseOptions)context.ParseOptions,
            optionsProvider: context.AnalyzerConfigOptions);
        var result = driver.RunGenerators(context.Compilation, context.CancellationToken).GetRunResult();
        foreach (var diagnostic in result.Diagnostics)
            context.ReportDiagnostic(diagnostic);

        for (var index = 0; index < result.Results.Length; index++)
        {
            var generatorResult = result.Results[index];
            if (generatorResult.Exception is not null)
                throw new InvalidOperationException("The .NET SDK COM generator failed.", generatorResult.Exception);
            foreach (var generated in generatorResult.GeneratedSources)
            {
                var name = $"SdkCom_{index}_{generated.HintName}";
                if (index == 0 && interfacesByHint.TryGetValue(generated.HintName, out var iface))
                {
                    seenInterfaces.Add(generated.HintName);
                    var source = generated.SourceText.ToString().Replace(OriginalProvider, BrowserProvider, StringComparison.Ordinal);
                    var inheritedImplementations = iface.Symbol.Interfaces.Select(baseInterface =>
                        interfacesBySymbol.TryGetValue(baseInterface, out var baseInfo)
                            ? baseInfo.ImplementationTypeName
                            : throw new InvalidOperationException(
                                $"Browser COM base interface '{baseInterface}' for '{iface.Symbol}' is not registered."))
                        .ToImmutableArray();
                    var rewriter = new GeneratedTypeRewriter(
                        new Dictionary<string, string>(StringComparer.Ordinal)
                        {
                            ["InterfaceInformation"] = iface.InformationName,
                            ["InterfaceImplementation"] = iface.ImplementationName,
                            ["InterfaceImplementationVtable"] = iface.VtableName,
                        },
                        iface.IsPublic,
                        inheritedImplementations: inheritedImplementations,
                        hidesInheritedVtable: iface.Symbol.Interfaces.Any(baseInterface =>
                            SymbolEqualityComparer.Default.Equals(baseInterface.ContainingAssembly, iface.Symbol.ContainingAssembly)));
                    context.AddSource(name, Rewrite(source, rewriter, context));
                }
                else if (index == 1 && classesByHint.TryGetValue(generated.HintName, out var comClass))
                {
                    seenClasses.Add(generated.HintName);
                    var rewriter = new GeneratedTypeRewriter(
                        new Dictionary<string, string>(StringComparer.Ordinal)
                        {
                            ["ComClassInformation"] = comClass.InformationName,
                        },
                        publicImplementation: false,
                        comClass: comClass);
                    context.AddSource(name, Rewrite(generated.SourceText.ToString(), rewriter, context));
                }
                else
                {
                    context.AddSource(name, generated.SourceText);
                }
            }
        }

        foreach (var hint in interfaceHints.Keys)
        {
            if (!seenInterfaces.Contains(hint))
                throw new InvalidOperationException($"The SDK did not generate the expected COM interface source '{hint}'.");
        }
        foreach (var hint in classHints.Keys)
        {
            if (!seenClasses.Contains(hint))
                throw new InvalidOperationException($"The SDK did not generate the expected COM class source '{hint}'.");
        }

        context.AddSource(
            "BrowserCom.Registrations.g.cs",
            SourceText.From(GenerateRegistrations(context.Compilation, interfaces, classes, referencedRegistrations, HostOnlyFamilies(context)), Encoding.UTF8));
    }

    /// <summary>
    /// Families listed in <c>BrowserComHostOnlyFamilies</c>: implemented only by
    /// managed objects, which reach managed code again as CCWs and are unwrapped,
    /// so they never need a native-object proxy.
    /// </summary>
    private static HashSet<string> HostOnlyFamilies(GeneratorExecutionContext context)
    {
        context.AnalyzerConfigOptions.GlobalOptions.TryGetValue("build_property.BrowserComHostOnlyFamilies", out var value);
        return new HashSet<string>(
            (value ?? "").Split(new[] { ';', ',' }, StringSplitOptions.RemoveEmptyEntries)
                .Select(static family => family.Trim())
                .Where(static family => family.Length != 0),
            StringComparer.Ordinal);
    }

    private static ISourceGenerator[] LoadSdkGenerators(GeneratorExecutionContext context)
    {
        if (!context.AnalyzerConfigOptions.GlobalOptions.TryGetValue("build_property.BrowserComSdkGeneratorPath", out var path)
            || string.IsNullOrEmpty(path) || !File.Exists(path))
            throw new InvalidOperationException("The .NET SDK COM source generator path was not supplied by BrowserCom.targets.");

        lock (s_loadGate)
        {
            var loadContext = AssemblyLoadContext.GetLoadContext(typeof(BrowserComGenerator).Assembly)
                ?? throw new InvalidOperationException("Could not locate the source generator's assembly load context.");
            var sourcePath = Path.Combine(Path.GetDirectoryName(path)!, "Microsoft.Interop.SourceGeneration.dll");
            LoadAssembly(loadContext, sourcePath);
            var assembly = LoadAssembly(loadContext, path);
            return new[] { "ComInterfaceGenerator", "ComClassGenerator", "VtableIndexStubGenerator" }
                .Select(name =>
                {
                    var type = assembly.GetType("Microsoft.Interop." + name, throwOnError: true)!;
                    return ((IIncrementalGenerator)Activator.CreateInstance(type)!).AsSourceGenerator();
                })
                .ToArray();
        }
    }

    private static Assembly LoadAssembly(AssemblyLoadContext context, string path)
    {
        var name = AssemblyName.GetAssemblyName(path);
        var existing = context.Assemblies.FirstOrDefault(assembly => assembly.GetName().Name == name.Name);
        if (existing is not null)
        {
            if (existing.GetName().Version != name.Version)
                throw new InvalidOperationException($"The loaded {name.Name} version does not match the selected .NET SDK.");
            return existing;
        }
        return context.LoadFromAssemblyPath(path);
    }

    private static ImmutableArray<INamedTypeSymbol> FindAttributedTypes(
        Compilation compilation,
        string attributeName,
        CancellationToken cancellationToken)
    {
        var types = new HashSet<INamedTypeSymbol>(SymbolEqualityComparer.Default);
        foreach (var tree in compilation.SyntaxTrees)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var root = tree.GetRoot(cancellationToken);
            var model = compilation.GetSemanticModel(tree);
            foreach (var declaration in root.DescendantNodes(static node =>
                         node is not BaseMethodDeclarationSyntax
                         and not AccessorDeclarationSyntax
                         and not AnonymousFunctionExpressionSyntax)
                     .OfType<TypeDeclarationSyntax>())
            {
                if (declaration.AttributeLists.Count == 0
                    || model.GetDeclaredSymbol(declaration, cancellationToken) is not INamedTypeSymbol type)
                    continue;
                if (type.GetAttributes().Any(attribute => attribute.AttributeClass?.ToDisplayString() == attributeName))
                    types.Add(type);
            }
        }
        return types.OrderBy(static type => type.ToDisplayString(), StringComparer.Ordinal).ToImmutableArray();
    }

    private static Dictionary<string, INamedTypeSymbol> MakeHintMap(
        ImmutableArray<INamedTypeSymbol> types,
        bool requireAccessible)
    {
        var result = new Dictionary<string, INamedTypeSymbol>(StringComparer.Ordinal);
        foreach (var type in types)
        {
            for (var current = type; current is not null; current = current.ContainingType)
            {
                if (current.IsGenericType)
                    throw new InvalidOperationException($"Browser COM generation does not support generic COM types: {type}.");
                if (requireAccessible && current.DeclaredAccessibility is
                    (Accessibility.Private or Accessibility.Protected or Accessibility.ProtectedAndInternal))
                    throw new InvalidOperationException($"The generated browser COM proxy cannot access the interface '{type}'.");
            }
            if (!result.TryAdd(HintName(type), type))
                throw new InvalidOperationException($"Multiple COM types produce the hint name '{HintName(type)}'.");
        }
        return result;
    }

    private static string HintName(INamedTypeSymbol symbol) => symbol.ToDisplayString() + ".cs";

    private static SourceText Rewrite(string source, GeneratedTypeRewriter rewriter, GeneratorExecutionContext context)
    {
        var tree = CSharpSyntaxTree.ParseText(source, (CSharpParseOptions)context.ParseOptions, cancellationToken: context.CancellationToken);
        var root = tree.GetRoot(context.CancellationToken);
        var model = context.Compilation.AddSyntaxTrees(tree).GetSemanticModel(tree);
        rewriter.Bind(model, root, context.CancellationToken);
        return SourceText.From(rewriter.Visit(root)!.ToFullString(), Encoding.UTF8);
    }

    private static List<string> FindReferencedRegistrations(Compilation compilation, List<InterfaceInfo> interfaces)
    {
        var registrations = new List<string>();
        foreach (var reference in compilation.References)
        {
            if (compilation.GetAssemblyOrModuleSymbol(reference) is not IAssemblySymbol assembly)
                continue;
            var attribute = assembly.GetAttributes().FirstOrDefault(
                static attribute => attribute.AttributeClass?.ToDisplayString() == BrowserAssemblyAttribute);
            if (attribute is null)
                continue;
            if (attribute.ConstructorArguments.Length != 1
                || attribute.ConstructorArguments[0].Value is not string registration
                || !registration.StartsWith("__RustoloniaBrowserComRegistration_", StringComparison.Ordinal)
                || !SyntaxFacts.IsValidIdentifier(registration))
                throw new InvalidOperationException($"The browser COM registration in '{assembly.Name}' is invalid.");
            registrations.Add(registration);
            foreach (var type in EnumerateTypes(assembly.GlobalNamespace))
            {
                if (type.TypeKind != TypeKind.Interface || !IsPubliclyVisible(type))
                    continue;
                var details = type.GetAttributes().FirstOrDefault(static attribute =>
                    attribute.AttributeClass is { Name: "IUnknownDerivedAttribute", TypeArguments.Length: 2 } attributeType
                    && attributeType.ContainingNamespace.ToDisplayString() == "System.Runtime.InteropServices.Marshalling");
                if (details?.AttributeClass?.TypeArguments[1] is INamedTypeSymbol implementation
                    && implementation.Name.StartsWith("__RustoloniaBrowserComImpl_", StringComparison.Ordinal))
                    interfaces.Add(InterfaceInfo.CreateReference(type, implementation));
            }
        }
        registrations.Sort(StringComparer.Ordinal);
        return registrations;
    }

    private static IEnumerable<INamedTypeSymbol> EnumerateTypes(INamespaceSymbol root)
    {
        foreach (var member in root.GetMembers())
        {
            if (member is INamespaceSymbol child)
            {
                foreach (var type in EnumerateTypes(child))
                    yield return type;
            }
            else if (member is INamedTypeSymbol type)
            {
                foreach (var nested in EnumerateTypes(type))
                    yield return nested;
            }
        }
    }

    private static IEnumerable<INamedTypeSymbol> EnumerateTypes(INamedTypeSymbol type)
    {
        yield return type;
        foreach (var nested in type.GetTypeMembers())
        {
            foreach (var descendant in EnumerateTypes(nested))
                yield return descendant;
        }
    }

    private static bool IsPubliclyVisible(INamedTypeSymbol type)
    {
        for (var current = type; current is not null; current = current.ContainingType)
        {
            if (current.DeclaredAccessibility != Accessibility.Public)
                return false;
        }
        return true;
    }

    private static string GenerateRegistrations(
        Compilation compilation,
        List<InterfaceInfo> interfaces,
        List<ClassInfo> classes,
        List<string> referencedRegistrations,
        HashSet<string> hostOnlyFamilies)
    {
        var registrationName = "__RustoloniaBrowserComRegistration_" + StableId(compilation.AssemblyName ?? "");
        var builder = new StringBuilder();
        builder.AppendLine("// <auto-generated />");
        builder.AppendLine("#nullable enable");
        builder.Append("[assembly: global::Avalonia.Rust.BrowserCom.BrowserComAssemblyAttribute(")
            .Append(Literal(registrationName)).AppendLine(")]");
        builder.Append("public static unsafe class ").Append(registrationName).AppendLine();
        builder.AppendLine("{");
        builder.Append("    static ").Append(registrationName).AppendLine("()");
        builder.AppendLine("    {");
        foreach (var reference in referencedRegistrations)
            builder.Append("        global::").Append(reference).AppendLine(".Touch();");
        foreach (var info in interfaces.Where(static info => info.IsLocal))
        {
            builder.Append("        global::Avalonia.Rust.BrowserCom.ComRuntime.RegisterInterface(typeof(")
                .Append(info.TypeName).Append("), new global::System.Runtime.InteropServices.Marshalling.IUnknownDerivedAttribute<global::")
                .Append(info.InformationName).Append(", global::").Append(info.ImplementationName).Append(">(), ")
                .Append(Literal(info.Family)).Append(", ").Append(Literal(info.Identity)).AppendLine(");");
        }
        foreach (var info in classes)
        {
            builder.Append("        ").Append(info.OuterTypeName).Append(".__RustoloniaRegister_")
                .Append(info.Id).AppendLine("();");
        }

        var families = interfaces.GroupBy(static info => info.Family, StringComparer.Ordinal)
            .OrderBy(static group => group.Key, StringComparer.Ordinal)
            .Select(group => group.OrderBy(static info => info.Identity, StringComparer.Ordinal).ToArray())
            .Where(static family => family.Any(static info => info.IsLocal)
                || family.Select(static info => info.AssemblyName).Distinct(StringComparer.Ordinal).Skip(1).Any())
            .ToArray();
        foreach (var family in families)
        {
            if (family.Length > MaximumFamilySize && !hostOnlyFamilies.Contains(family[0].Family))
                throw new InvalidOperationException($"Browser COM family '{family[0].Family}' has {family.Length} interfaces; the static proxy generator supports at most {MaximumFamilySize}. List it in BrowserComHostOnlyFamilies if native code never implements it.");
            var id = StableId(family[0].Family);
            builder.Append("        global::Avalonia.Rust.BrowserCom.ComRuntime.RegisterFamily(")
                .Append(Literal(family[0].Family)).Append(", new global::System.Type[] { ");
            foreach (var info in family)
                builder.Append("typeof(").Append(info.TypeName).Append("), ");
            builder.Append("}, Create_").Append(id).AppendLine(");");
        }
        builder.AppendLine("    }");
        builder.AppendLine("    [global::System.Runtime.CompilerServices.ModuleInitializer]");
        builder.AppendLine("    internal static void Initialize() => Touch();");
        builder.AppendLine("    public static void Touch() { }");
        foreach (var family in families)
        {
            if (hostOnlyFamilies.Contains(family[0].Family))
                AppendHostOnlyFamily(builder, family);
            else
                AppendFamily(builder, family);
        }
        builder.AppendLine("}");
        return builder.ToString();
    }

    private static void AppendFamily(StringBuilder builder, InterfaceInfo[] family)
    {
        var id = StableId(family[0].Family);
        var combinations = 1UL << family.Length;
        builder.Append("    private static global::Avalonia.Rust.BrowserCom.ComObjectBase Create_")
            .Append(id).AppendLine("(ulong mask) => mask switch");
        builder.AppendLine("    {");
        for (ulong mask = 1; mask < combinations; mask++)
            builder.Append("        ").Append(mask).Append("UL => new Proxy_").Append(id).Append('_').Append(mask).AppendLine("(),");
        builder.AppendLine("        _ => throw new global::System.InvalidOperationException(\"Invalid browser COM interface set.\"),");
        builder.AppendLine("    };");
        for (ulong mask = 1; mask < combinations; mask++)
        {
            builder.Append("    private sealed class Proxy_").Append(id).Append('_').Append(mask)
                .Append(" : global::Avalonia.Rust.BrowserCom.ComObjectBase, global::Avalonia.Rust.BrowserCom.IComVirtualMethodTableProvider");
            for (var index = 0; index < family.Length; index++)
            {
                if ((mask & (1UL << index)) != 0)
                    builder.Append(", ").Append(family[index].ImplementationTypeName);
            }
            builder.AppendLine(" { }");
        }
    }

    private static void AppendHostOnlyFamily(StringBuilder builder, InterfaceInfo[] family)
    {
        var message = $"Browser COM family '{family[0].Family}' is implemented only by managed objects (BrowserComHostOnlyFamilies); native objects cannot be wrapped.";
        builder.Append("    private static global::Avalonia.Rust.BrowserCom.ComObjectBase Create_")
            .Append(StableId(family[0].Family)).Append("(ulong mask) => throw new global::System.InvalidOperationException(")
            .Append(Literal(message)).AppendLine(");");
    }

    private static string Literal(string value) => SymbolDisplay.FormatLiteral(value, quote: true);

    private static string StableId(string value)
    {
        var bytes = SHA256.HashData(Encoding.UTF8.GetBytes(value));
        return Convert.ToHexString(bytes.AsSpan(0, 8));
    }

    private sealed record InterfaceInfo(
        INamedTypeSymbol Symbol,
        string TypeName,
        string AssemblyName,
        string Identity,
        string Family,
        string InformationName,
        string ImplementationName,
        string VtableName,
        string ImplementationTypeName,
        bool IsLocal,
        bool IsPublic)
    {
        public static InterfaceInfo CreateLocal(INamedTypeSymbol symbol)
        {
            var identity = IdentityOf(symbol);
            var id = StableId(identity);
            var implementation = "__RustoloniaBrowserComImpl_" + id;
            return new InterfaceInfo(
                symbol, TypeNameOf(symbol), symbol.ContainingAssembly.Identity.Name, identity, FamilyOf(symbol),
                "__RustoloniaBrowserComInfo_" + id, implementation, "__RustoloniaBrowserComVtable_" + id,
                "global::" + implementation, true, IsPubliclyVisible(symbol));
        }

        public static InterfaceInfo CreateReference(INamedTypeSymbol symbol, INamedTypeSymbol implementation) =>
            new(symbol, TypeNameOf(symbol), symbol.ContainingAssembly.Identity.Name, IdentityOf(symbol), FamilyOf(symbol),
                "", implementation.Name, "", TypeNameOf(implementation), false, true);

        private static string IdentityOf(INamedTypeSymbol symbol) =>
            symbol.ContainingAssembly.Identity.Name + "|" + TypeNameOf(symbol);

        private static string FamilyOf(INamedTypeSymbol symbol) =>
            (symbol.ContainingNamespace.IsGlobalNamespace ? "" : symbol.ContainingNamespace.ToDisplayString())
            + "." + symbol.Name.TrimEnd('0', '1', '2', '3', '4', '5', '6', '7', '8', '9');
    }

    private sealed record ClassInfo(
        INamedTypeSymbol Symbol,
        string TypeName,
        string OuterTypeName,
        string[] TypePath,
        string InformationName,
        string Id,
        int Depth)
    {
        public static ClassInfo Create(INamedTypeSymbol symbol)
        {
            var id = StableId(symbol.ContainingAssembly.Identity.Name + "|" + TypeNameOf(symbol));
            var depth = 0;
            for (var current = symbol; current is not null; current = current.BaseType)
                depth++;
            var path = new List<INamedTypeSymbol>();
            for (var current = symbol; current is not null; current = current.ContainingType)
                path.Add(current);
            path.Reverse();
            return new ClassInfo(
                symbol, TypeNameOf(symbol), TypeNameOf(path[0]), path.Select(static type => type.Name).ToArray(),
                "__RustoloniaBrowserComClass_" + id, id, depth);
        }
    }

    private static string TypeNameOf(INamedTypeSymbol symbol) =>
        symbol.ToDisplayString(SymbolDisplayFormat.FullyQualifiedFormat);

    private sealed class GeneratedTypeRewriter(
        Dictionary<string, string> names,
        bool publicImplementation,
        ClassInfo? comClass = null,
        ImmutableArray<string> inheritedImplementations = default,
        bool hidesInheritedVtable = false) : CSharpSyntaxRewriter
    {
        private readonly Dictionary<ISymbol, string> _renamedTypes = new(SymbolEqualityComparer.Default);
        private SemanticModel? _model;

        public void Bind(SemanticModel model, SyntaxNode root, CancellationToken cancellationToken)
        {
            _model = model;
            foreach (var declaration in root.DescendantNodes().OfType<TypeDeclarationSyntax>())
            {
                if (declaration.Modifiers.Any(SyntaxKind.FileKeyword)
                    && names.TryGetValue(declaration.Identifier.ValueText, out var name)
                    && model.GetDeclaredSymbol(declaration, cancellationToken) is { } symbol)
                    _renamedTypes[symbol] = name;
            }
        }

        public override SyntaxNode? VisitInterfaceDeclaration(InterfaceDeclarationSyntax node)
        {
            var result = (InterfaceDeclarationSyntax)base.VisitInterfaceDeclaration(node)!;
            if (node.Identifier.ValueText == "InterfaceImplementation"
                && node.BaseList is not null
                && !inheritedImplementations.IsDefaultOrEmpty)
            {
                // The SDK expects dynamic casting to route inherited methods to
                // the base implementation. Static proxies inherit it explicitly.
                result = result.AddBaseListTypes(inheritedImplementations
                    .Select(static name => SyntaxFactory.SimpleBaseType(SyntaxFactory.ParseTypeName(name)))
                    .ToArray());
            }
            return AppendClassRegistration(node, result);
        }

        public override SyntaxNode? VisitIdentifierName(IdentifierNameSyntax node)
        {
            var result = (IdentifierNameSyntax)base.VisitIdentifierName(node)!;
            if (names.ContainsKey(node.Identifier.ValueText)
                && _model?.GetSymbolInfo(node).Symbol is { } symbol
                && _renamedTypes.TryGetValue(symbol, out var name))
                result = result.WithIdentifier(SyntaxFactory.Identifier(node.Identifier.LeadingTrivia, name, node.Identifier.TrailingTrivia));
            return result;
        }

        public override SyntaxNode? VisitMethodDeclaration(MethodDeclarationSyntax node)
        {
            if (!inheritedImplementations.IsDefaultOrEmpty
                && node.Parent is InterfaceDeclarationSyntax { Identifier.ValueText: "InterfaceImplementation" }
                && node.ExplicitInterfaceSpecifier is not null
                && node.ExpressionBody?.Expression is ThrowExpressionSyntax
                {
                    Expression: ObjectCreationExpressionSyntax creation
                }
                && creation.Type.ToString() == "global::System.Diagnostics.UnreachableException")
                return null;
            return base.VisitMethodDeclaration(node);
        }

        public override SyntaxNode? VisitClassDeclaration(ClassDeclarationSyntax node)
        {
            var result = (ClassDeclarationSyntax)base.VisitClassDeclaration(node)!;
            return AppendClassRegistration(node, result);
        }

        public override SyntaxNode? VisitStructDeclaration(StructDeclarationSyntax node)
        {
            var result = (StructDeclarationSyntax)base.VisitStructDeclaration(node)!;
            return AppendClassRegistration(node, result);
        }

        public override SyntaxNode? VisitRecordDeclaration(RecordDeclarationSyntax node)
        {
            var result = (RecordDeclarationSyntax)base.VisitRecordDeclaration(node)!;
            return AppendClassRegistration(node, result);
        }

        private TypeDeclarationSyntax AppendClassRegistration(TypeDeclarationSyntax node, TypeDeclarationSyntax result)
        {
            if (comClass is null)
                return result;

            var path = node.Ancestors().OfType<TypeDeclarationSyntax>().Reverse()
                .Select(static declaration => declaration.Identifier.ValueText)
                .Append(node.Identifier.ValueText).ToArray();
            if (path.Length > comClass.TypePath.Length
                || !path.SequenceEqual(comClass.TypePath.Take(path.Length), StringComparer.Ordinal))
                return result;

            var methodName = "__RustoloniaRegister_" + comClass.Id;
            string method;
            if (path.Length == comClass.TypePath.Length)
            {
                method = $"internal static unsafe void {methodName}() => " +
                    $"global::Avalonia.Rust.BrowserCom.ComRuntime.RegisterClass<{comClass.TypeName}>(" +
                    $"static (out int count) => (global::Avalonia.Rust.BrowserCom.ComInterfaceEntry*)" +
                    $"global::{comClass.InformationName}.GetComInterfaceEntries(out count), {comClass.Depth});";
            }
            else
            {
                var child = comClass.TypePath[path.Length];
                if (SyntaxFacts.GetKeywordKind(child) != SyntaxKind.None)
                    child = "@" + child;
                method = $"internal static void {methodName}() => {child}.{methodName}();";
            }
            var member = SyntaxFactory.ParseMemberDeclaration(method)
                ?? throw new InvalidOperationException($"Could not generate registration for '{comClass.Symbol}'.");
            return result.WithMembers(result.Members.Add(member));
        }

        public override SyntaxToken VisitToken(SyntaxToken token)
        {
            if (token.IsKind(SyntaxKind.FileKeyword) && token.Parent is TypeDeclarationSyntax declaration)
            {
                var visibility = publicImplementation && declaration.Identifier.ValueText == "InterfaceImplementation"
                    ? SyntaxKind.PublicKeyword
                    : SyntaxKind.InternalKeyword;
                return SyntaxFactory.Token(token.LeadingTrivia, visibility, token.TrailingTrivia);
            }
            if (token.Parent is TypeDeclarationSyntax typeDeclaration
                && token == typeDeclaration.Identifier
                && typeDeclaration.Modifiers.Any(SyntaxKind.FileKeyword)
                && names.TryGetValue(token.ValueText, out var name))
                return SyntaxFactory.Identifier(token.LeadingTrivia, name, token.TrailingTrivia);
            if (token.Parent is ConstructorDeclarationSyntax { Parent: TypeDeclarationSyntax containingType } constructor
                && token == constructor.Identifier
                && containingType.Modifiers.Any(SyntaxKind.FileKeyword)
                && names.TryGetValue(containingType.Identifier.ValueText, out var constructorType))
                return SyntaxFactory.Identifier(token.LeadingTrivia, constructorType, token.TrailingTrivia);
            return base.VisitToken(token);
        }

        public override SyntaxNode? VisitFieldDeclaration(FieldDeclarationSyntax node)
        {
            var result = (FieldDeclarationSyntax)base.VisitFieldDeclaration(node)!;
            if (node.Parent is InterfaceDeclarationSyntax { Identifier.ValueText: "InterfaceImplementation" }
                && node.Declaration.Variables.Any(static variable => variable.Identifier.ValueText == "Vtable"))
            {
                var modifiers = result.Modifiers.Select(token => token.IsKind(SyntaxKind.PublicKeyword)
                    ? SyntaxFactory.Token(token.LeadingTrivia, SyntaxKind.InternalKeyword, token.TrailingTrivia)
                    : token);
                result = result.WithModifiers(SyntaxFactory.TokenList(modifiers));
                if (hidesInheritedVtable)
                {
                    var newModifier = SyntaxFactory.Token(SyntaxKind.NewKeyword).WithTrailingTrivia(SyntaxFactory.Space);
                    result = result.WithModifiers(result.Modifiers.Insert(1, newModifier));
                }
            }
            return result;
        }
    }
}
