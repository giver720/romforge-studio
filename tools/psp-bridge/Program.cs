// SPDX-License-Identifier: GPL-3.0-or-later
// Separate command-line bridge for PSPHD, pspdecrypt and LibOrbisPkg.

using System.Diagnostics;
using System.Formats.Tar;
using System.IO.Compression;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;
using DiscUtils.Iso9660;
using LibOrbisPkg.GP4;
using LibOrbisPkg.PKG;
using LibOrbisPkg.SFO;
using SkiaSharp;

namespace RomForge.PspBridge;

internal static partial class Program
{
    private const string BridgeVersion = "1.0.1";
    private const string AssetsUrl = "https://github.com/SvenGDK/PS-Classics-fPKG-Builder/releases/download/v1/PS.Classics.fPKG.Builder.v1.Linux.x64.tar.gz";
    private const string AssetsSha256 = "3a23ceb4cf29f0dd93f02a961a1e8624a2724785a8dc29be3933247134d91707";
    private const string PspDecryptWindowsUrl = "https://github.com/John-K/pspdecrypt/releases/download/1.0/pspdecrypt-1.0-windows.zip";
    private const string PspDecryptWindowsSha256 = "b915e4ce30f4e0309c71b7044584a466be17a181f889f40bcc7f6fd82ba2d045";
    private const string PspDecryptLinuxUrl = "https://github.com/John-K/pspdecrypt/releases/download/1.0/pspdecrypt-1.0-linux.zip";
    private const string PspDecryptLinuxSha256 = "37ac1c8e9530b64061ba817173d787aaa06346d5f76cfb97290d2c855ffba149";

    private sealed class Options
    {
        public Dictionary<string, string> Values { get; } = new(StringComparer.OrdinalIgnoreCase);
        public List<string> Sets { get; } = [];
        public string Required(string key) => Values.TryGetValue(key, out var value) && !string.IsNullOrWhiteSpace(value)
            ? value : throw new ArgumentException($"Missing --{key}");
        public string? Get(string key) => Values.GetValueOrDefault(key);
        public bool Enabled(string key, bool fallback = false) => Values.TryGetValue(key, out var value)
            ? value.Equals("true", StringComparison.OrdinalIgnoreCase) : fallback;
    }

    private static int Main(string[] args)
    {
        try
        {
            if (args.Length == 1 && args[0] == "--version")
            {
                Console.WriteLine($"ROMForge PSP Bridge {BridgeVersion} · PSPHD · LibOrbisPkg");
                return 0;
            }
            if (args.Length == 0 || args[0] is "--help" or "-h")
            {
                PrintHelp();
                return args.Length == 0 ? 2 : 0;
            }
            var options = Parse(args[1..]);
            return args[0] switch
            {
                "convert" => Convert(options),
                "validate" => ValidateOnly(options),
                _ => throw new ArgumentException($"Unknown command: {args[0]}")
            };
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"ERROR: {ex.Message}");
            return 1;
        }
    }

    private static Options Parse(string[] args)
    {
        var result = new Options();
        for (var i = 0; i < args.Length; i++)
        {
            var name = args[i];
            if (!name.StartsWith("--", StringComparison.Ordinal) || i + 1 >= args.Length)
                throw new ArgumentException($"Invalid option: {name}");
            var value = args[++i];
            if (name == "--set") result.Sets.Add(value);
            else result.Values[name[2..]] = value;
        }
        return result;
    }

    private static int Convert(Options options)
    {
        var input = Path.GetFullPath(options.Required("input"));
        var output = Path.GetFullPath(options.Required("output"));
        if (!File.Exists(input) || !Path.GetExtension(input).Equals(".iso", StringComparison.OrdinalIgnoreCase))
            throw new InvalidDataException("The PSP bridge requires an ISO file.");

        var cache = Path.GetFullPath(options.Get("cache") ?? DefaultCache());
        var template = ResolveTemplate(options.Get("emulator-dir"), cache, options.Enabled("no-fetch"));
        var title = CleanTitle(options.Get("title") ?? Path.GetFileNameWithoutExtension(input));
        var detectedId = DetectTitleId(input);
        var titleId = NormalizeTitleId(options.Get("title-id") ?? detectedId
            ?? throw new InvalidDataException("No PSP title ID was found. Supply --title-id, for example ULUS12345."));
        var contentId = $"UP9000-{titleId}_00-{titleId}PSPFPKG";
        Console.WriteLine($"[ROMFORGE] PSP title: {title} · {titleId}");

        var work = Directory.CreateTempSubdirectory("romforge-pspfpkg-").FullName;
        try
        {
            var project = Path.Combine(work, "image0");
            CopyTree(template, project);
            ValidateTemplate(project);
            var data = Path.Combine(project, "data");
            Directory.CreateDirectory(data);
            var image = Path.Combine(data, "USER_L0.IMG");
            File.Copy(input, image, true);

            if (!options.Enabled("skip-eboot-decrypt"))
                DecryptEmbeddedEboot(input, image, cache);
            else
                Console.WriteLine("[WARNING] EBOOT decryption was explicitly skipped.");

            WriteConfig(project, titleId, options);
            ApplyExtras(project, titleId, options);
            PatchSfo(project, title, titleId, contentId);

            Directory.CreateDirectory(Path.GetDirectoryName(output)!);
            BuildPkg(project, contentId, output);
            ValidateOrThrow(output);
            Console.WriteLine($"[ROMFORGE] OUTPUT={output}");
            return 0;
        }
        finally
        {
            try { Directory.Delete(work, true); } catch { }
        }
    }

    private static string DefaultCache() => Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "romforge-studio", "psp-fpkg");

    private static string ResolveTemplate(string? custom, string cache, bool noFetch)
    {
        if (!string.IsNullOrWhiteSpace(custom))
        {
            var full = Path.GetFullPath(custom);
            ValidateTemplate(full);
            return full;
        }
        var shared = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "ps2fpkg", "assets", "emus", "psphd");
        if (Directory.Exists(shared)) return shared;
        var target = Path.Combine(cache, "assets", "emus", "psphd");
        if (Directory.Exists(target)) return target;
        if (noFetch) throw new DirectoryNotFoundException("PSPHD assets are missing and fetching is disabled.");

        Directory.CreateDirectory(cache);
        var archive = Path.Combine(cache, "ps-classics-v1.tar.gz");
        DownloadVerified(AssetsUrl, archive, AssetsSha256, "PSPHD emulator resources");
        var root = Path.Combine(cache, "assets");
        Directory.CreateDirectory(root);
        Console.WriteLine("[ROMFORGE] Extracting PSPHD resources");
        using var stream = File.OpenRead(archive);
        using var gzip = new GZipStream(stream, CompressionMode.Decompress);
        using var source = new TarReader(gzip);
        const string marker = "Tools/PS4/emus/psphd/";
        while (source.GetNextEntry() is { } entry)
        {
            if (entry.EntryType is not (TarEntryType.RegularFile or TarEntryType.V7RegularFile) || entry.DataStream is null) continue;
            var normalized = entry.Name.Replace('\\', '/');
            var index = normalized.IndexOf(marker, StringComparison.Ordinal);
            if (index < 0) continue;
            var relative = normalized[(index + "Tools/PS4/".Length)..];
            var destination = SafeChild(root, relative);
            Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
            using var output = File.Create(destination);
            entry.DataStream.CopyTo(output);
        }
        ValidateTemplate(target);
        return target;
    }

    private static string SafeChild(string root, string relative)
    {
        var fullRoot = Path.GetFullPath(root) + Path.DirectorySeparatorChar;
        var full = Path.GetFullPath(Path.Combine(root, relative.Replace('/', Path.DirectorySeparatorChar)));
        if (!full.StartsWith(fullRoot, StringComparison.OrdinalIgnoreCase))
            throw new InvalidDataException("Unsafe path in PSPHD archive.");
        return full;
    }

    private static void DownloadVerified(string url, string path, string expected, string label)
    {
        if (HashMatches(path, expected)) return;

        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        var temporary = $"{path}.{Environment.ProcessId}.{Guid.NewGuid():N}.download";
        Console.WriteLine($"[ROMFORGE] Downloading {label}");
        try
        {
            using (var http = new HttpClient { Timeout = TimeSpan.FromMinutes(30) })
            {
                http.DefaultRequestHeaders.UserAgent.ParseAdd("ROMForge-PSP-Bridge/1.0");
                using var response = http.GetAsync(url, HttpCompletionOption.ResponseHeadersRead).GetAwaiter().GetResult();
                response.EnsureSuccessStatusCode();
                using var source = response.Content.ReadAsStream();
                using var destination = new FileStream(
                    temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None, 1024 * 128,
                    FileOptions.SequentialScan);
                source.CopyTo(destination);
                destination.Flush(true);
            }

            // The destination stream must be closed before hashing. A using declaration
            // scoped to this whole method kept the ZIP locked on Windows during first use.
            if (!HashMatches(temporary, expected))
                throw new InvalidDataException($"SHA-256 mismatch for {label}.");

            PublishDownload(temporary, path, expected, label);
        }
        finally
        {
            try { File.Delete(temporary); } catch { }
        }
    }

    private static bool HashMatches(string path, string expected)
    {
        try { return File.Exists(path) && Hash(path) == expected; }
        catch (IOException) { return false; }
        catch (UnauthorizedAccessException) { return false; }
    }

    private static void PublishDownload(string temporary, string path, string expected, string label)
    {
        for (var attempt = 0; attempt < 40; attempt++)
        {
            if (HashMatches(path, expected)) return;
            try
            {
                File.Move(temporary, path, true);
                return;
            }
            catch (IOException) when (attempt < 39)
            {
                Thread.Sleep(250);
            }
            catch (UnauthorizedAccessException) when (attempt < 39)
            {
                Thread.Sleep(250);
            }
        }
        throw new IOException($"Could not install {label}; its cache file remains in use.");
    }

    private static string Hash(string path)
    {
        using var stream = File.OpenRead(path);
        return System.Convert.ToHexString(SHA256.HashData(stream)).ToLowerInvariant();
    }

    private static string EnsurePspDecrypt(string cache)
    {
        var tools = Path.Combine(cache, "tools", "pspdecrypt-1.0");
        var exe = Path.Combine(tools, OperatingSystem.IsWindows() ? "pspdecrypt.exe" : "pspdecrypt");
        if (!File.Exists(exe))
        {
            Directory.CreateDirectory(tools);
            var url = OperatingSystem.IsWindows() ? PspDecryptWindowsUrl : PspDecryptLinuxUrl;
            var sha = OperatingSystem.IsWindows() ? PspDecryptWindowsSha256 : PspDecryptLinuxSha256;
            var zip = Path.Combine(cache, OperatingSystem.IsWindows() ? "pspdecrypt-windows.zip" : "pspdecrypt-linux.zip");
            DownloadVerified(url, zip, sha, "pspdecrypt 1.0");
            ZipFile.ExtractToDirectory(zip, tools, true);
        }
        if (!OperatingSystem.IsWindows())
            File.SetUnixFileMode(exe, UnixFileMode.UserRead | UnixFileMode.UserWrite | UnixFileMode.UserExecute);
        return exe;
    }

    private static string? DetectTitleId(string iso)
    {
        using var stream = File.OpenRead(iso);
        using var reader = new CDReader(stream, true);
        var path = FirstIsoPath(reader, "UMD_DATA.BIN", @"\\UMD_DATA.BIN");
        if (path is null) return null;
        using var data = reader.OpenFile(path, FileMode.Open);
        var bytes = new byte[Math.Min(128, (int)data.Length)];
        _ = data.Read(bytes, 0, bytes.Length);
        return TitleIdRegex().Match(Encoding.ASCII.GetString(bytes)).Value.Replace("-", "").Replace("_", "");
    }

    private static string NormalizeTitleId(string value)
    {
        var normalized = value.Trim().ToUpperInvariant().Replace("-", "").Replace("_", "");
        if (!NormalizedTitleIdRegex().IsMatch(normalized))
            throw new InvalidDataException("PSP title ID must contain four letters and five digits, for example ULUS12345.");
        return normalized;
    }

    private static string CleanTitle(string value)
    {
        var clean = value.Trim();
        if (clean.Length == 0) throw new InvalidDataException("The title cannot be empty.");
        return clean.Length > 127 ? clean[..127] : clean;
    }

    private static string? FirstIsoPath(CDReader reader, params string[] candidates) =>
        candidates.FirstOrDefault(reader.FileExists);

    private static void DecryptEmbeddedEboot(string sourceIso, string copiedIso, string cache)
    {
        Console.WriteLine("[ROMFORGE] Decrypting the PSP EBOOT inside the disc image");
        var temp = Path.Combine(cache, "eboot-work");
        Directory.CreateDirectory(temp);
        var encrypted = Path.Combine(temp, "EBOOT.BIN");
        var decrypted = encrypted + ".dec";
        if (File.Exists(decrypted)) File.Delete(decrypted);
        using (var stream = File.OpenRead(sourceIso))
        using (var reader = new CDReader(stream, true))
        {
            var path = FirstIsoPath(reader, @"PSP_GAME\SYSDIR\EBOOT.BIN", @"\\PSP_GAME\\SYSDIR\\EBOOT.BIN")
                ?? throw new InvalidDataException("PSP_GAME/SYSDIR/EBOOT.BIN was not found in the ISO.");
            using var source = reader.OpenFile(path, FileMode.Open);
            using var output = File.Create(encrypted);
            source.CopyTo(output);
        }
        var tool = EnsurePspDecrypt(cache);
        var info = new ProcessStartInfo(tool) { UseShellExecute = false, RedirectStandardOutput = true, RedirectStandardError = true };
        info.ArgumentList.Add(encrypted);
        using var process = Process.Start(info) ?? throw new InvalidOperationException("Could not start pspdecrypt.");
        var stdout = process.StandardOutput.ReadToEnd();
        var stderr = process.StandardError.ReadToEnd();
        process.WaitForExit();
        if (process.ExitCode != 0 || !File.Exists(decrypted))
            throw new InvalidDataException($"pspdecrypt failed: {stderr} {stdout}".Trim());

        var original = File.ReadAllBytes(encrypted);
        var replacement = File.ReadAllBytes(decrypted);
        if (replacement.Length > original.Length)
            throw new InvalidDataException("The decrypted EBOOT is larger than its ISO allocation; refusing to truncate it.");
        var probeLength = Math.Min(original.Length, 512_320);
        var offset = FindPattern(copiedIso, original.AsSpan(0, probeLength));
        if (offset < 0) throw new InvalidDataException("The embedded EBOOT offset could not be located.");
        using var image = new FileStream(copiedIso, FileMode.Open, FileAccess.Write, FileShare.None);
        image.Position = offset;
        image.Write(replacement, 0, Math.Min(replacement.Length, original.Length));
        if (replacement.Length < original.Length)
            image.Write(new byte[original.Length - replacement.Length]);
    }

    private static long FindPattern(string path, ReadOnlySpan<byte> pattern)
    {
        var prefix = new int[pattern.Length];
        for (int i = 1, j = 0; i < pattern.Length; i++)
        {
            while (j > 0 && pattern[i] != pattern[j]) j = prefix[j - 1];
            if (pattern[i] == pattern[j]) j++;
            prefix[i] = j;
        }
        using var stream = File.OpenRead(path);
        var buffer = new byte[1024 * 1024];
        long consumed = 0;
        int matched = 0;
        int count;
        while ((count = stream.Read(buffer, 0, buffer.Length)) > 0)
        {
            for (var i = 0; i < count; i++)
            {
                while (matched > 0 && buffer[i] != pattern[matched]) matched = prefix[matched - 1];
                if (buffer[i] == pattern[matched]) matched++;
                if (matched == pattern.Length) return consumed + i - pattern.Length + 1;
            }
            consumed += count;
        }
        return -1;
    }

    private static void WriteConfig(string project, string titleId, Options options)
    {
        var lines = new List<string>
        {
            "--ps4-trophies=0", "--ps5-uds=0", "--trophies=0",
            "--image=\"data/USER_L0.IMG\"",
            $"--antialias={options.Get("antialias") ?? "SSAA4x"}",
            $"--multisaves={(options.Enabled("multisaves", true) ? "true" : "false")}",
            "--notrophies=true"
        };
        AddOption(lines, "xobuttonmode", options.Get("xobuttonmode"));
        AddOption(lines, "lang", options.Get("lang"));
        AddOption(lines, "loglevel", options.Get("loglevel"));
        AddOption(lines, "securesaves", options.Get("securesaves"));
        if (options.Enabled("texture-replacement"))
        {
            lines.Add("--texreplace=\"host0:texreplace\"");
            lines.Add("--replacementalpha=true");
            lines.Add("--replacementfilter=true");
        }
        foreach (var setting in options.Sets)
        {
            var split = setting.Split('=', 2);
            if (split.Length != 2 || !ConfigKeyRegex().IsMatch(split[0]) || split[1].ContainsAny('\r', '\n'))
                throw new InvalidDataException($"Invalid emulator setting: {setting}");
            lines.Add($"--{split[0]}={split[1]}");
        }
        var custom = options.Get("config");
        if (!string.IsNullOrWhiteSpace(custom))
        {
            if (!File.Exists(custom)) throw new FileNotFoundException("Custom config not found.", custom);
            lines.AddRange(File.ReadAllLines(custom));
        }
        File.WriteAllLines(Path.Combine(project, "config-title.txt"), lines, new UTF8Encoding(false));
        File.WriteAllText(Path.Combine(project, "config-region.txt"), $"--active-sku=\"{titleId}#v1.00\"{Environment.NewLine}", new UTF8Encoding(false));
    }

    private static void AddOption(List<string> lines, string name, string? value)
    {
        if (!string.IsNullOrWhiteSpace(value)) lines.Add($"--{name}={value}");
    }

    private static void ApplyExtras(string project, string titleId, Options options)
    {
        var icon = options.Get("icon");
        if (!string.IsNullOrWhiteSpace(icon)) ResizeImage(icon, Path.Combine(project, "sce_sys", "icon0.png"), 512, 512);
        var background = options.Get("background");
        if (!string.IsNullOrWhiteSpace(background)) ResizeImage(background, Path.Combine(project, "sce_sys", "pic1.png"), 1920, 1080);
        var lua = options.Get("lua");
        if (!string.IsNullOrWhiteSpace(lua))
        {
            if (!File.Exists(lua)) throw new FileNotFoundException("Lua patch not found.", lua);
            var scripts = Path.Combine(project, "scripts");
            Directory.CreateDirectory(scripts);
            File.Copy(lua, Path.Combine(scripts, $"{titleId}_patches.lua"), true);
        }
        var textures = options.Get("textures");
        if (!string.IsNullOrWhiteSpace(textures))
        {
            if (!Directory.Exists(textures)) throw new DirectoryNotFoundException("Texture folder not found.");
            CopyTree(textures, Path.Combine(project, "texreplace"));
        }
    }

    private static void ResizeImage(string source, string destination, int width, int height)
    {
        if (!File.Exists(source)) throw new FileNotFoundException("Artwork file not found.", source);
        Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
        using var image = SKBitmap.Decode(source) ?? throw new InvalidDataException("The artwork image could not be decoded.");
        using var resized = new SKBitmap(width, height, SKColorType.Rgba8888, SKAlphaType.Premul);
        using var canvas = new SKCanvas(resized);
        canvas.Clear(SKColors.Black);
        var scale = Math.Max((float)width / image.Width, (float)height / image.Height);
        var drawnWidth = image.Width * scale;
        var drawnHeight = image.Height * scale;
        var destinationRect = SKRect.Create((width - drawnWidth) / 2f, (height - drawnHeight) / 2f, drawnWidth, drawnHeight);
        using var paint = new SKPaint { FilterQuality = SKFilterQuality.High, IsAntialias = true };
        canvas.DrawBitmap(image, destinationRect, paint);
        canvas.Flush();
        using var encoded = resized.Encode(SKEncodedImageFormat.Png, 95)
            ?? throw new InvalidDataException("The artwork image could not be encoded as PNG.");
        using var output = File.Create(destination);
        encoded.SaveTo(output);
    }

    private static void PatchSfo(string project, string title, string titleId, string contentId)
    {
        var path = Path.Combine(project, "sce_sys", "param.sfo");
        using var input = File.OpenRead(path);
        var sfo = ParamSfo.FromStream(input);
        input.Close();
        sfo.SetValue("CONTENT_ID", SfoEntryType.Utf8, contentId, 48);
        sfo.SetValue("TITLE", SfoEntryType.Utf8, title, 128);
        sfo.SetValue("TITLE_ID", SfoEntryType.Utf8, titleId, 12);
        using var output = File.Create(path);
        sfo.Write(output);
    }

    private static void BuildPkg(string projectRoot, string contentId, string output)
    {
        Console.WriteLine("[ROMFORGE] Building PSP FPKG");
        var project = Gp4Project.Create(VolumeType.pkg_ps4_app);
        project.volume.Id = "PSPCLASSIC";
        project.volume.Package.ContentId = contentId;
        project.volume.Package.Passcode = "00000000000000000000000000000000";
        project.files.ImageNum = 0;
        foreach (var file in Directory.EnumerateFiles(projectRoot, "*", SearchOption.AllDirectories).OrderBy(value => value, StringComparer.Ordinal))
        {
            var target = Path.GetRelativePath(projectRoot, file).Replace('\\', '/');
            project.files.Items.Add(new Gp4File { TargetPath = target, OrigPath = Path.GetFullPath(file) });
            Dir? parent = null;
            var parts = target.Split('/');
            for (var i = 0; i < parts.Length - 1; i++) parent = project.AddDir(parent, parts[i]);
        }
        foreach (var validation in Gp4Validator.ValidateProject(project, projectRoot))
            if (validation.Type == ValidateResult.ResultType.Fatal)
                throw new InvalidDataException($"GP4 fatal: {validation.Message}");
        var properties = PkgProperties.FromGp4(project, projectRoot);
        new PkgBuilder(properties).Write(output, value => Console.WriteLine($"[LibOrbisPkg] {value}"));
        if (!File.Exists(output)) throw new InvalidDataException("No PKG was produced.");
    }

    private static int ValidateOnly(Options options)
    {
        ValidateOrThrow(Path.GetFullPath(options.Required("input")));
        return 0;
    }

    private static void ValidateOrThrow(string path)
    {
        using var stream = File.OpenRead(path);
        var package = new PkgReader(stream).ReadPkg();
        var validator = new PkgValidator(package);
        var ok = 0;
        var failures = new List<string>();
        foreach (var item in validator.Validations(stream))
        {
            if (item.Validate() == PkgValidator.ValidationResult.Ok) ok++;
            else failures.Add(item.Name);
        }
        if (failures.Count != 0) throw new InvalidDataException("PKG validation failed: " + string.Join(", ", failures));
        Console.WriteLine($"[ROMFORGE] Package accepted · {ok} checks OK");
    }

    private static void ValidateTemplate(string path)
    {
        if (!Directory.Exists(path) || !File.Exists(Path.Combine(path, "eboot.bin")) || !File.Exists(Path.Combine(path, "sce_sys", "param.sfo")))
            throw new InvalidDataException("The PSPHD emulator template is incomplete (eboot.bin or sce_sys/param.sfo is missing).");
    }

    private static void CopyTree(string source, string destination)
    {
        Directory.CreateDirectory(destination);
        foreach (var directory in Directory.EnumerateDirectories(source, "*", SearchOption.AllDirectories))
            Directory.CreateDirectory(Path.Combine(destination, Path.GetRelativePath(source, directory)));
        foreach (var file in Directory.EnumerateFiles(source, "*", SearchOption.AllDirectories))
        {
            var target = Path.Combine(destination, Path.GetRelativePath(source, file));
            Directory.CreateDirectory(Path.GetDirectoryName(target)!);
            File.Copy(file, target, true);
        }
    }

    private static void PrintHelp()
    {
        Console.WriteLine("ROMForge PSP Bridge");
        Console.WriteLine("  convert --input game.iso --output game.pkg [--title text] [--title-id ULUS12345]");
        Console.WriteLine("          [--antialias off|MSAA4x|SSAA4x] [--multisaves true|false]");
        Console.WriteLine("          [--icon image] [--background image] [--config file] [--lua file]");
        Console.WriteLine("          [--textures folder] [--set name=value] [--emulator-dir folder]");
        Console.WriteLine("  validate --input game.pkg");
    }

    [GeneratedRegex(@"[A-Z]{4}[-_]?[0-9]{5}", RegexOptions.IgnoreCase)]
    private static partial Regex TitleIdRegex();
    [GeneratedRegex(@"^[A-Z]{4}[0-9]{5}$")]
    private static partial Regex NormalizedTitleIdRegex();
    [GeneratedRegex(@"^[a-z0-9][a-z0-9-]*$", RegexOptions.IgnoreCase)]
    private static partial Regex ConfigKeyRegex();
}

internal static class CharacterExtensions
{
    public static bool ContainsAny(this string value, params char[] chars) => value.IndexOfAny(chars) >= 0;
}
