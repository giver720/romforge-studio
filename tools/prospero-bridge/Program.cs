// SPDX-License-Identifier: GPL-3.0-or-later
// Separate command-line bridge for the GPL-3.0 LibProsperoPKG engine.

using LibProsperoPkg;
using LibProsperoPkg.PKG;

namespace RomForge.ProsperoBridge;

internal static class Program
{
    private const string BridgeVersion = "1.0.0";

    private static int Main(string[] args)
    {
        try
        {
            if (args.Length == 1 && args[0] == "--version")
            {
                Console.WriteLine($"ROMForge Prospero Bridge {BridgeVersion} · LibProsperoPKG {typeof(ProsperoBackupConverter).Assembly.GetName().Version}");
                return 0;
            }

            if (args.Length == 0 || args[0] is "--help" or "-h")
            {
                PrintHelp();
                return args.Length == 0 ? 2 : 0;
            }

            return args[0] switch
            {
                "convert" => ConvertBackup(ParseOptions(args[1..])),
                "validate" => ValidatePackage(ParseOptions(args[1..])),
                _ => throw new ArgumentException($"Unknown command: {args[0]}")
            };
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"ERROR: {ex.Message}");
            return 1;
        }
    }

    private static Dictionary<string, string> ParseOptions(string[] args)
    {
        var values = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        for (var index = 0; index < args.Length; index++)
        {
            string name = args[index];
            if (!name.StartsWith("--", StringComparison.Ordinal) || index + 1 >= args.Length)
                throw new ArgumentException($"Invalid option: {name}");
            values[name[2..]] = args[++index];
        }
        return values;
    }

    private static string Required(Dictionary<string, string> options, string name) =>
        options.TryGetValue(name, out string? value) && !string.IsNullOrWhiteSpace(value)
            ? value
            : throw new ArgumentException($"Missing --{name}");

    private static int ConvertBackup(Dictionary<string, string> options)
    {
        string input = Path.GetFullPath(Required(options, "input"));
        string requestedOutput = Path.GetFullPath(Required(options, "output"));
        string outputDirectory = Path.GetDirectoryName(requestedOutput)
            ?? throw new ArgumentException("Output path has no parent directory.");
        Directory.CreateDirectory(outputDirectory);

        Console.WriteLine("[ROMFORGE] Inspecting decrypted PS5 backup");
        var result = ProsperoBackupConverter.Convert(
            new ProsperoBackupConversionOptions
            {
                BackupFolder = input,
                OutputFolder = outputDirectory,
                DecryptedSubfolder = options.GetValueOrDefault("decrypted-subfolder", "decrypted"),
                UseEmbeddedRightSprx = options.GetValueOrDefault("embedded-right", "false")
                    .Equals("true", StringComparison.OrdinalIgnoreCase),
            },
            line => Console.WriteLine($"[LibProsperoPKG] {line}"));

        Console.WriteLine($"[ROMFORGE] Modules substituted: {result.SubstitutedModules.Count}");
        Console.WriteLine($"[ROMFORGE] Plaintext/fake modules: {result.PlaintextModules.Count}");
        Console.WriteLine($"[ROMFORGE] Unresolved modules: {result.UnresolvedModules.Count}");
        foreach (string warning in result.Warnings)
            Console.WriteLine($"[WARNING] {warning}");

        if (result.UnresolvedModules.Count != 0)
            throw new InvalidDataException(
                "The package contains unresolved encrypted modules: " +
                string.Join(", ", result.UnresolvedModules));
        if (!result.LaunchReadiness.IsLaunchReady)
            throw new InvalidDataException(
                "The converted application is not launch-ready: " +
                string.Join("; ", result.LaunchReadiness.Issues));

        ValidateOrThrow(result.OutputPath);

        string actualOutput = Path.GetFullPath(result.OutputPath);
        if (!actualOutput.Equals(requestedOutput, StringComparison.OrdinalIgnoreCase))
        {
            File.Move(actualOutput, requestedOutput, overwrite: true);
            actualOutput = requestedOutput;
        }

        Console.WriteLine($"[ROMFORGE] OUTPUT={actualOutput}");
        return 0;
    }

    private static int ValidatePackage(Dictionary<string, string> options)
    {
        ValidateOrThrow(Path.GetFullPath(Required(options, "input")));
        return 0;
    }

    private static void ValidateOrThrow(string path)
    {
        ProsperoAcceptanceReport report = ProsperoPkgValidator.Validate(path);
        foreach (ProsperoAcceptanceCheck check in report.Checks)
            Console.WriteLine($"[{check.Status}] {check.Name}: {check.Detail}");
        if (!report.Accepted)
            throw new InvalidDataException("LibProsperoPKG rejected the generated package.");
        Console.WriteLine($"[ROMFORGE] Package accepted · {report.Checks.Count} structural checks");
    }

    private static void PrintHelp()
    {
        Console.WriteLine("ROMForge Prospero Bridge");
        Console.WriteLine("  convert --input <PS5 backup folder> --output <file.pkg>");
        Console.WriteLine("  validate --input <file.pkg>");
    }
}
