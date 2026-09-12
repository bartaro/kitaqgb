using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;
using System.Threading;

static partial class Program
{
    sealed class VibeSnippet
    {
        public string Id;
        public string Category;
        public string Title;
        public string Body;
    }

    sealed class VibeHistoryEntry
    {
        public DateTime TimestampUtc;
        public string WorkingDirectory;
        public string ArgsLine;
    }

    sealed class VibeDiagRecord
    {
        public string Severity;
        public string CodeText;
        public string Message;
        public string Suggestion;
        public string File;
        public int Line;
        public int Column;
    }

    sealed class VibeAsmFunctionStat
    {
        public string Name;
        public int InstructionCount;
    }

    struct VibeFileStamp
    {
        public bool Exists;
        public long Length;
        public long LastWriteTicksUtc;
    }

    static readonly Dictionary<string, VibeSnippet> _vibeSnippets = BuildVibeSnippetLibrary();
    static readonly HashSet<string> _subcommandSet = new HashSet<string>(StringComparer.OrdinalIgnoreCase)
    {
        "test", "attrviz", "src2asm", "symfind", "romdiff", "kqhelp",
        "template", "fixhint", "irsum", "conventions", "snippet", "devserver", "recipe"
    };

    static void TryAppendCommandHistory(string[] argsArray)
    {
        try
        {
            if (argsArray == null) return;

            string path = GetCommandHistoryPath();
            string dir = Path.GetDirectoryName(path);
            if (!string.IsNullOrWhiteSpace(dir)) Directory.CreateDirectory(dir);

            string ts = DateTime.UtcNow.ToString("O");
            string cwd = SanitizeHistoryField(Environment.CurrentDirectory);
            string args = BuildHistoryArgsLine(argsArray);
            string line = ts + "\t" + cwd + "\t" + args + Environment.NewLine;

            File.AppendAllText(path, line, IoUtil.Utf8NoBom);
        }
        catch
        {
            // History logging is best-effort.
        }
    }

    static string GetCommandHistoryPath()
    {
        string root = FindRepoRoot(Environment.CurrentDirectory);
        if (string.IsNullOrWhiteSpace(root)) root = Environment.CurrentDirectory;
        return Path.Combine(root, "integration_test", "reports", "COMMAND_HISTORY.log");
    }

    static string BuildHistoryArgsLine(IEnumerable<string> args)
    {
        if (args == null) return "";
        return string.Join(" ", args.Select(a => QuoteArg(a ?? "")));
    }

    static string SanitizeHistoryField(string s)
    {
        if (string.IsNullOrEmpty(s)) return "";
        return s.Replace("\r", " ").Replace("\n", " ").Replace("\t", " ");
    }

    static bool TryRunVibeTemplateCommand(string[] argsArray)
    {
        if (argsArray == null || argsArray.Length == 0) return false;
        if (!string.Equals(argsArray[0], "template", StringComparison.OrdinalIgnoreCase)) return false;

        string outPath = Path.Combine(Environment.CurrentDirectory, "quickstart.c");
        bool overwrite = false;
        bool includeBuildScript = true;
        bool includeReadme = true;
        bool cgbMode = false;

        for (int i = 1; i < argsArray.Length; i++)
        {
            string a = argsArray[i] ?? "";
            if (string.IsNullOrWhiteSpace(a)) continue;
            if (a == "--overwrite") overwrite = true;
            else if (a == "--no-build-script") includeBuildScript = false;
            else if (a == "--no-readme") includeReadme = false;
            else if (a == "--cgb") cgbMode = true;
            else if (a.StartsWith("--out=", StringComparison.Ordinal)) outPath = ValueAfterEquals(a);
            else if (!a.StartsWith("-", StringComparison.Ordinal)) outPath = a;
            else
            {
                Console.Error.WriteLine("error KQ0000: unknown template option: " + a);
                Console.Error.WriteLine("usage: kitaqgb template [out.c] [--out=<file.c>] [--cgb] [--overwrite] [--no-build-script] [--no-readme]");
                Exit(1);
                return true;
            }
        }

        if (string.IsNullOrWhiteSpace(outPath))
        {
            Console.Error.WriteLine("error KQ0000: template output path is empty");
            Exit(1);
            return true;
        }

        if (!Path.HasExtension(outPath))
            outPath += ".c";

        string fullOut = Path.GetFullPath(outPath);
        string outDir = Path.GetDirectoryName(fullOut);
        if (!string.IsNullOrWhiteSpace(outDir)) Directory.CreateDirectory(outDir);

        if (File.Exists(fullOut) && !overwrite)
        {
            Console.Error.WriteLine("error KQ0000: output exists (use --overwrite): " + fullOut);
            Exit(1);
            return true;
        }

        string stem = Path.GetFileNameWithoutExtension(fullOut);
        string source = BuildVibePrototypeSource(stem, cgbMode);
        IoUtil.WriteAllTextUtf8Robust(fullOut, source, allowAlternatePath: true);
        Console.WriteLine("[template] source: " + fullOut);

        string romOut = Path.Combine(outDir ?? ".", stem + ".gb");
        if (includeBuildScript)
        {
            string scriptPath = Path.Combine(outDir ?? ".", stem + "_build.ps1");
            IoUtil.WriteAllTextUtf8Robust(scriptPath, BuildVibePrototypeBuildScript(Path.GetFileName(fullOut), Path.GetFileName(romOut)), allowAlternatePath: true);
            Console.WriteLine("[template] build script: " + scriptPath);
        }

        if (includeReadme)
        {
            string readme = Path.Combine(outDir ?? ".", stem + "_README.md");
            var sb = new StringBuilder();
            sb.AppendLine("# Quick Prototype");
            sb.AppendLine();
            sb.AppendLine("1. Build:");
            sb.AppendLine("   `kitaqgb " + Path.GetFileName(fullOut) + " -o " + Path.GetFileName(romOut) + " --profile=dev --fast-build --cache --deps-out`");
            sb.AppendLine("2. Run on emulator/hardware and iterate.");
            sb.AppendLine("3. Add snippets with `kitaqgb snippet list` / `kitaqgb snippet get <id>`.");
            IoUtil.WriteAllTextUtf8Robust(readme, sb.ToString(), allowAlternatePath: true);
            Console.WriteLine("[template] readme: " + readme);
        }

        Console.WriteLine("[template] instant build:");
        Console.WriteLine("kitaqgb " + QuoteArg(Path.GetFileName(fullOut)) + " -o " + QuoteArg(Path.GetFileName(romOut)) + " --profile=dev --fast-build --cache --deps-out");

        Exit(0);
        return true;
    }

    static string BuildVibePrototypeSource(string stem, bool cgbMode)
    {
        var sb = new StringBuilder();
        sb.AppendLine("#pragma bank 0");
        sb.AppendLine();
        sb.AppendLine("// Auto-generated single-file prototype.");
        sb.AppendLine("// Build: kitaqgb " + stem + ".c -o " + stem + ".gb --profile=dev --fast-build --cache");
        sb.AppendLine();
        sb.AppendLine("// Hardware register declarations (keep __location in ascending order).");
        sb.AppendLine("__location(0xFF40) u8 LCDC;");
        sb.AppendLine("__location(0xFF44) u8 LY;");
        sb.AppendLine("__location(0xFF47) u8 BGP;");
        sb.AppendLine();
        sb.AppendLine("__wram u8 g_frame;");
        sb.AppendLine("__wram u8 g_flash;");
        sb.AppendLine();
        sb.AppendLine("void WaitVBlank() {");
        sb.AppendLine("    if ((LCDC & 0x80) == 0) return;");
        sb.AppendLine("    while (LY >= 144) { }");
        sb.AppendLine("    while (LY < 144) { }");
        sb.AppendLine("}");
        sb.AppendLine();
        sb.AppendLine("void main() {");
        sb.AppendLine("    BGP = 0xE4;");
        if (cgbMode)
        {
            sb.AppendLine(" // CGB-safe template: only writes DMG-compatible registers by default.");
            sb.AppendLine();
        }
        sb.AppendLine("    g_frame = 0;");
        sb.AppendLine("    g_flash = 0;");
        sb.AppendLine("    while (1) {");
        sb.AppendLine("        WaitVBlank();");
        sb.AppendLine("        g_frame++;");
        sb.AppendLine("        if ((g_frame & 31) == 0) {");
        sb.AppendLine("            g_flash ^= 1;");
        sb.AppendLine("            if (g_flash != 0) BGP = 0x1B;");
        sb.AppendLine("            else BGP = 0xE4;");
        sb.AppendLine("        }");
        sb.AppendLine("    }");
        sb.AppendLine("}");
        return sb.ToString();
    }

    static string BuildVibePrototypeBuildScript(string sourceName, string romName)
    {
        var sb = new StringBuilder();
        sb.AppendLine("param(");
        sb.AppendLine("    [switch]$Release");
        sb.AppendLine(")");
        sb.AppendLine("$ErrorActionPreference = 'Stop'");
        sb.AppendLine("$args = @('" + sourceName.Replace("'", "''") + "', '-o', '" + romName.Replace("'", "''") + "', '--cache', '--deps-out')");
        sb.AppendLine("if ($Release) {");
        sb.AppendLine("    $args += '--profile=release'");
        sb.AppendLine("} else {");
        sb.AppendLine("    $args += '--profile=dev'");
        sb.AppendLine("    $args += '--fast-build'");
        sb.AppendLine("}");
        sb.AppendLine("& kitaqgb @args");
        sb.AppendLine("exit $LASTEXITCODE");
        return sb.ToString();
    }

    static bool TryRunConventionsCommand(string[] argsArray)
    {
        if (argsArray == null || argsArray.Length == 0) return false;
        if (!string.Equals(argsArray[0], "conventions", StringComparison.OrdinalIgnoreCase)) return false;

        string outPath = "";
        bool stdout = false;
        for (int i = 1; i < argsArray.Length; i++)
        {
            string a = argsArray[i] ?? "";
            if (a == "--stdout" || a == "-") stdout = true;
            else if (a.StartsWith("--out=", StringComparison.Ordinal)) outPath = ValueAfterEquals(a);
            else
            {
                Console.Error.WriteLine("error KQ0000: unknown conventions option: " + a);
                Console.Error.WriteLine("usage: kitaqgb conventions [--out=<file>|--stdout]");
                Exit(1);
                return true;
            }
        }

        if (!stdout && string.IsNullOrWhiteSpace(outPath))
        {
            string root = FindRepoRoot(Environment.CurrentDirectory) ?? Environment.CurrentDirectory;
            outPath = Path.Combine(root, "integration_test", "reports", "PROJECT_CONVENTIONS.md");
        }

        string text = BuildProjectConventionsText();
        if (stdout)
        {
            Console.Write(text);
        }
        else
        {
            IoUtil.WriteAllTextUtf8Robust(outPath, text, allowAlternatePath: true);
            Console.WriteLine("[conventions] " + outPath);
        }

        Exit(0);
        return true;
    }

    static string BuildProjectConventionsText()
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB Project Conventions");
        sb.AppendLine();
        sb.AppendLine("generated_utc=" + DateTime.UtcNow.ToString("O"));
        sb.AppendLine();
        sb.AppendLine("## Build");
        sb.AppendLine("- Default dev build: `kitaqgb a.c b.c -o out.gb --profile=dev --fast-build --cache --deps-out`");
        sb.AppendLine("- Release build: `kitaqgb a.c b.c -o out.gb --profile=release --cache --deps-out`");
        sb.AppendLine("- Test entrypoint: `kitaqgb test`");
        sb.AppendLine();
        sb.AppendLine("## Diagnostics");
        sb.AppendLine("- Use `--diag-json` for machine-readable diagnostics.");
        sb.AppendLine("- Use `kitaqgb fixhint <diag-or-log>` for shortest fix suggestions.");
        sb.AppendLine("- Use `--strict` in CI and `--permissive` during exploration.");
        sb.AppendLine();
        sb.AppendLine("## Vibe Workflow");
        sb.AppendLine("- Start with `kitaqgb template` and keep prototype in one file.");
        sb.AppendLine("- Use `kitaqgb snippet list` and `kitaqgb snippet get <id>` to paste proven fragments.");
        sb.AppendLine("- Use `kitaqgb devserver ...` for dependency-aware rebuild loop.");
        sb.AppendLine("- Record reproducible steps with `kitaqgb recipe`.");
        sb.AppendLine();
        sb.AppendLine("## Encoding");
        sb.AppendLine("- Treat source, reports, and scripts as UTF-8.");
        sb.AppendLine("- Keep command history in `integration_test/reports/COMMAND_HISTORY.log`.");
        return sb.ToString();
    }

    static bool TryRunSnippetLibraryCommand(string[] argsArray)
    {
        if (argsArray == null || argsArray.Length == 0) return false;
        if (!string.Equals(argsArray[0], "snippet", StringComparison.OrdinalIgnoreCase)) return false;

        if (argsArray.Length < 2)
        {
            Console.Error.WriteLine("error KQ0000: snippet requires subcommand");
            Console.Error.WriteLine("usage: kitaqgb snippet list | get <id> [--out=<file>|-] | emit <category|all> [--out=<file>|-]");
            Exit(1);
            return true;
        }

        string sub = (argsArray[1] ?? "").Trim();
        if (string.Equals(sub, "list", StringComparison.OrdinalIgnoreCase))
        {
            var sb = new StringBuilder();
            sb.AppendLine("# KITAQGB snippet library");
            foreach (var s in _vibeSnippets.Values.OrderBy(v => v.Category, StringComparer.Ordinal).ThenBy(v => v.Id, StringComparer.Ordinal))
            {
                sb.AppendLine(s.Id + "  [" + s.Category + "]  " + s.Title);
            }
            Console.Write(sb.ToString());
            Exit(0);
            return true;
        }

        if (string.Equals(sub, "get", StringComparison.OrdinalIgnoreCase))
        {
            if (argsArray.Length < 3)
            {
                Console.Error.WriteLine("error KQ0000: snippet get requires <id>");
                Exit(1);
                return true;
            }

            string id = argsArray[2];
            string outPath = "-";
            for (int i = 3; i < argsArray.Length; i++)
            {
                string a = argsArray[i] ?? "";
                if (a == "--stdout" || a == "-") outPath = "-";
                else if (a.StartsWith("--out=", StringComparison.Ordinal)) outPath = ValueAfterEquals(a);
                else
                {
                    Console.Error.WriteLine("error KQ0000: unknown snippet get option: " + a);
                    Exit(1);
                    return true;
                }
            }

            if (!_vibeSnippets.TryGetValue(id, out var snip))
            {
                Console.Error.WriteLine("error KQ0000: snippet not found: " + id);
                Exit(1);
                return true;
            }

            string text = snip.Body.TrimEnd() + Environment.NewLine;
            if (outPath == "-")
            {
                Console.Write(text);
            }
            else
            {
                IoUtil.WriteAllTextUtf8Robust(outPath, text, allowAlternatePath: true);
                Console.WriteLine("[snippet] " + outPath);
            }
            Exit(0);
            return true;
        }

        if (string.Equals(sub, "emit", StringComparison.OrdinalIgnoreCase))
        {
            if (argsArray.Length < 3)
            {
                Console.Error.WriteLine("error KQ0000: snippet emit requires <category|all>");
                Exit(1);
                return true;
            }
            string category = (argsArray[2] ?? "").Trim();
            string outPath = "-";
            for (int i = 3; i < argsArray.Length; i++)
            {
                string a = argsArray[i] ?? "";
                if (a == "--stdout" || a == "-") outPath = "-";
                else if (a.StartsWith("--out=", StringComparison.Ordinal)) outPath = ValueAfterEquals(a);
                else
                {
                    Console.Error.WriteLine("error KQ0000: unknown snippet emit option: " + a);
                    Exit(1);
                    return true;
                }
            }

            IEnumerable<VibeSnippet> selected = string.Equals(category, "all", StringComparison.OrdinalIgnoreCase)
                ? _vibeSnippets.Values
                : _vibeSnippets.Values.Where(v => string.Equals(v.Category, category, StringComparison.OrdinalIgnoreCase));

            var list = selected.OrderBy(v => v.Category, StringComparer.Ordinal).ThenBy(v => v.Id, StringComparer.Ordinal).ToList();
            if (list.Count == 0)
            {
                Console.Error.WriteLine("error KQ0000: no snippets found for category: " + category);
                Exit(1);
                return true;
            }

            var sb = new StringBuilder();
            sb.AppendLine("// KITAQGB snippet bundle");
            sb.AppendLine("// category=" + category);
            sb.AppendLine();
            foreach (var s in list)
            {
                sb.AppendLine("// --- " + s.Id + " [" + s.Category + "] " + s.Title + " ---");
                sb.AppendLine(s.Body.TrimEnd());
                sb.AppendLine();
            }

            if (outPath == "-") Console.Write(sb.ToString());
            else
            {
                IoUtil.WriteAllTextUtf8Robust(outPath, sb.ToString(), allowAlternatePath: true);
                Console.WriteLine("[snippet] " + outPath);
            }

            Exit(0);
            return true;
        }

        Console.Error.WriteLine("error KQ0000: unknown snippet subcommand: " + sub);
        Exit(1);
        return true;
    }

    static Dictionary<string, VibeSnippet> BuildVibeSnippetLibrary()
    {
        var map = new Dictionary<string, VibeSnippet>(StringComparer.Ordinal);

        void Add(string id, string category, string title, string body)
        {
            map[id] = new VibeSnippet
            {
                Id = id,
                Category = category,
                Title = title,
                Body = body
            };
        }

        Add(
            "input.poll_joypad",
            "input",
            "Joypad Poll",
@"u8 PollJoypad() {
    // requires: __location(0xFF00) u8 P1;
    P1 = 0x20;
    u8 dpad = (~P1) & 0x0F;
    P1 = 0x10;
    u8 buttons = ((~P1) & 0x0F) << 4;
    return (u8)(buttons | dpad);
}");

        Add(
            "render.wait_vblank",
            "render",
            "Wait VBlank",
@"void WaitVBlank() {
    // requires:
    // __location(0xFF40) u8 LCDC;
    // __location(0xFF44) u8 LY;
    if ((LCDC & 0x80) == 0) return;
    while (LY >= 144) { }
    while (LY < 144) { }
}");

        Add(
            "render.clear_oam4",
            "render",
            "Clear 4 sprites",
@"void ClearOam4() {
    __store8(0xFE00, 0);
    __store8(0xFE01, 0);
    __store8(0xFE02, 0);
    __store8(0xFE03, 0);
}");

        Add(
            "sound.beep_ch2",
            "sound",
            "Simple CH2 beep",
@"void SfxBeepCh2() {
    // requires:
    // __location(0xFF16) u8 NR21;
    // __location(0xFF17) u8 NR22;
    // __location(0xFF18) u8 NR23;
    // __location(0xFF19) u8 NR24;
    NR21 = 0x80;
    NR22 = 0xF3;
    NR23 = 0x40;
    NR24 = 0x87;
}");

        Add(
            "sram.enable_disable",
            "sram",
            "Enable / Disable SRAM",
@"void SRAM_Enable()  { __store8(0x0000, 0x0A); }
void SRAM_Disable() { __store8(0x0000, 0x00); }");

        Add(
            "sram.save_byte",
            "sram",
            "Save one byte",
@"void SRAM_SaveByte(u16 addr, u8 value) {
    SRAM_Enable();
    __store8(addr, value);
    SRAM_Disable();
}");

        return map;
    }

    static bool TryRunFixHintCommand(string[] argsArray)
    {
        if (argsArray == null || argsArray.Length == 0) return false;
        if (!string.Equals(argsArray[0], "fixhint", StringComparison.OrdinalIgnoreCase)) return false;

        string inPath = "";
        string outPath = "-";
        int maxItems = 20;

        for (int i = 1; i < argsArray.Length; i++)
        {
            string a = argsArray[i] ?? "";
            if (a == "--stdout" || a == "-") outPath = "-";
            else if (a.StartsWith("--in=", StringComparison.Ordinal)) inPath = ValueAfterEquals(a);
            else if (a.StartsWith("--out=", StringComparison.Ordinal)) outPath = ValueAfterEquals(a);
            else if (a.StartsWith("--max=", StringComparison.Ordinal))
            {
                if (!int.TryParse(ValueAfterEquals(a), out maxItems) || maxItems < 1 || maxItems > 200)
                {
                    Console.Error.WriteLine("error KQ0000: --max must be 1..200");
                    Exit(1);
                    return true;
                }
            }
            else if (!a.StartsWith("-", StringComparison.Ordinal) && string.IsNullOrWhiteSpace(inPath))
            {
                inPath = a;
            }
            else
            {
                Console.Error.WriteLine("error KQ0000: unknown fixhint option: " + a);
                Console.Error.WriteLine("usage: kitaqgb fixhint [diag.json|stderr.log] [--in=<file>] [--out=<file>|-] [--max=N]");
                Exit(1);
                return true;
            }
        }

        if (string.IsNullOrWhiteSpace(inPath))
        {
            if (File.Exists("kitaqgb.diag.json")) inPath = "kitaqgb.diag.json";
            if (string.IsNullOrWhiteSpace(inPath))
                inPath = TryGuessNewestFile("*.diag.json", recursive: true);
            if (string.IsNullOrWhiteSpace(inPath))
                inPath = TryGuessNewestFile("*.stderr.log", recursive: true);
            if (string.IsNullOrWhiteSpace(inPath))
                inPath = TryGuessNewestFile("*.log", recursive: true);
        }

        if (string.IsNullOrWhiteSpace(inPath) || !File.Exists(inPath))
        {
            Console.Error.WriteLine("error KQ0000: no input log/diag found for fixhint");
            Exit(1);
            return true;
        }

        string text;
        try
        {
            text = IoUtil.ReadAllTextUtf8(inPath);
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("error KQ0000: failed to read input: " + ex.Message);
            Exit(1);
            return true;
        }

        List<VibeDiagRecord> records;
        if (LooksLikeDiagJson(text))
            records = ParseDiagJsonRecords(text);
        else
            records = ParseDiagnosticLogRecords(text);

        foreach (var r in records)
        {
            if (string.IsNullOrWhiteSpace(r.Suggestion))
                r.Suggestion = GetFixSuggestionFromTextCode(r.CodeText, r.Message);
            if (string.IsNullOrWhiteSpace(r.Suggestion))
                r.Suggestion = GetFallbackFixSuggestion(r.Message);
        }

        var selected = records
            .Where(r => !string.IsNullOrWhiteSpace(r.Message))
            .OrderBy(r => FixHintSeverityRank(r.Severity))
            .ThenBy(r => r.CodeText, StringComparer.Ordinal)
            .Take(maxItems)
            .ToList();

        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB shortest fix suggestions");
        sb.AppendLine("input=" + Path.GetFullPath(inPath));
        sb.AppendLine("diagnostics_found=" + records.Count);
        sb.AppendLine("suggestions_emitted=" + selected.Count);
        sb.AppendLine();

        int idx = 1;
        foreach (var r in selected)
        {
            string where = "";
            if (!string.IsNullOrWhiteSpace(r.File) && r.Line > 0)
                where = " @ " + r.File + ":" + r.Line;

            sb.Append(idx.ToString()).Append(". ");
            sb.Append("[").Append(string.IsNullOrWhiteSpace(r.Severity) ? "diag" : r.Severity);
            if (!string.IsNullOrWhiteSpace(r.CodeText)) sb.Append(" ").Append(r.CodeText);
            sb.Append("] ").Append(r.Message).Append(where).AppendLine();
            sb.Append("   shortest_fix: ").Append(r.Suggestion ?? "").AppendLine();
            sb.AppendLine();
            idx++;
        }

        if (selected.Count == 0)
        {
            sb.AppendLine("No diagnostics were parsed.");
            sb.AppendLine("Run compiler with --diag-json and re-run fixhint.");
        }

        if (outPath == "-")
            Console.Write(sb.ToString());
        else
        {
            IoUtil.WriteAllTextUtf8Robust(outPath, sb.ToString(), allowAlternatePath: true);
            Console.WriteLine("[fixhint] " + outPath);
        }

        Exit(0);
        return true;
    }

    static bool LooksLikeDiagJson(string text)
    {
        if (string.IsNullOrWhiteSpace(text)) return false;
        return text.IndexOf("\"diagnostics\"", StringComparison.OrdinalIgnoreCase) >= 0 &&
               text.IndexOf("\"exit_code\"", StringComparison.OrdinalIgnoreCase) >= 0;
    }

    static List<VibeDiagRecord> ParseDiagJsonRecords(string json)
    {
        var list = new List<VibeDiagRecord>();
        if (string.IsNullOrWhiteSpace(json)) return list;

        var re = new Regex("\\{\\s*\"severity\":\"(?<sev>(?:\\\\.|[^\"])*)\",\\s*\"code\":\"(?<code>(?:\\\\.|[^\"])*)\",\\s*\"message\":\"(?<msg>(?:\\\\.|[^\"])*)\",\\s*\"suggestion\":\"(?<sug>(?:\\\\.|[^\"])*)\",\\s*\"file\":\"(?<file>(?:\\\\.|[^\"])*)\",\\s*\"line\":(?<line>\\d+),\\s*\"column\":(?<col>\\d+)\\s*\\}", RegexOptions.Singleline);
        foreach (Match m in re.Matches(json))
        {
            if (!m.Success) continue;
            int.TryParse(m.Groups["line"].Value, out int line);
            int.TryParse(m.Groups["col"].Value, out int col);
            list.Add(new VibeDiagRecord
            {
                Severity = JsonUnescape(m.Groups["sev"].Value),
                CodeText = JsonUnescape(m.Groups["code"].Value),
                Message = JsonUnescape(m.Groups["msg"].Value),
                Suggestion = JsonUnescape(m.Groups["sug"].Value),
                File = JsonUnescape(m.Groups["file"].Value),
                Line = line,
                Column = col,
            });
        }
        return list;
    }

    static string JsonUnescape(string s)
    {
        if (string.IsNullOrEmpty(s)) return "";
        var sb = new StringBuilder(s.Length);
        for (int i = 0; i < s.Length; i++)
        {
            char c = s[i];
            if (c != '\\')
            {
                sb.Append(c);
                continue;
            }
            if (i + 1 >= s.Length)
            {
                sb.Append('\\');
                break;
            }
            char n = s[++i];
            switch (n)
            {
                case '\\': sb.Append('\\'); break;
                case '"': sb.Append('"'); break;
                case '/': sb.Append('/'); break;
                case 'b': sb.Append('\b'); break;
                case 'f': sb.Append('\f'); break;
                case 'n': sb.Append('\n'); break;
                case 'r': sb.Append('\r'); break;
                case 't': sb.Append('\t'); break;
                case 'u':
                    if (i + 4 < s.Length)
                    {
                        string hex = s.Substring(i + 1, 4);
                        if (int.TryParse(hex, System.Globalization.NumberStyles.HexNumber, System.Globalization.CultureInfo.InvariantCulture, out int code))
                        {
                            sb.Append((char)code);
                            i += 4;
                        }
                        else
                        {
                            sb.Append("\\u").Append(hex);
                            i += 4;
                        }
                    }
                    else
                    {
                        sb.Append("\\u");
                    }
                    break;
                default:
                    sb.Append(n);
                    break;
            }
        }
        return sb.ToString();
    }

    static List<VibeDiagRecord> ParseDiagnosticLogRecords(string text)
    {
        var list = new List<VibeDiagRecord>();
        if (string.IsNullOrWhiteSpace(text)) return list;

        string[] lines = text.Replace("\r\n", "\n").Replace('\r', '\n').Split('\n');
        var re = new Regex("^(?:(?<file>.+?) \\(line (?<line>\\d+), column (?<col>\\d+)\\) )?(?<sev>warning|error|fatal|internalerror) (?<code>KQ\\d{4}): (?<msg>.*)$", RegexOptions.IgnoreCase);
        foreach (string raw in lines)
        {
            string line = raw ?? "";
            var m = re.Match(line);
            if (!m.Success) continue;

            int.TryParse(m.Groups["line"].Value, out int lineNum);
            int.TryParse(m.Groups["col"].Value, out int colNum);
            list.Add(new VibeDiagRecord
            {
                Severity = (m.Groups["sev"].Value ?? "").ToLowerInvariant(),
                CodeText = m.Groups["code"].Value ?? "",
                Message = m.Groups["msg"].Value ?? "",
                Suggestion = "",
                File = m.Groups["file"].Value ?? "",
                Line = lineNum,
                Column = colNum,
            });
        }
        return list;
    }

    static string GetFixSuggestionFromTextCode(string codeText, string message)
    {
        int n = ParseKqCode(codeText);
        if (n >= 0 && n <= 9999)
        {
            var code = (ErrorCode)n;
            string suggestion = GetFixSuggestion(code, message ?? "");
            if (!string.IsNullOrWhiteSpace(suggestion)) return suggestion;
        }
        return "";
    }

    static int ParseKqCode(string codeText)
    {
        if (string.IsNullOrWhiteSpace(codeText)) return -1;
        string t = codeText.Trim().ToUpperInvariant();
        if (!t.StartsWith("KQ", StringComparison.Ordinal)) return -1;
        if (t.Length != 6) return -1;
        if (int.TryParse(t.Substring(2), out int n)) return n;
        return -1;
    }

    static string GetFallbackFixSuggestion(string message)
    {
        if (string.IsNullOrWhiteSpace(message)) return "Run again with --diag-json for structured hints.";
        string m = message.ToLowerInvariant();

        if (m.Contains("unknown option")) return "Unknown option. Check spelling with `kitaqgb --help`.";
        if (m.Contains("no source files")) return "Pass one or more `.c` source files.";
        if (m.Contains("expected")) return "Check nearby tokens like `;`, `)`, `]`, or `}`.";
        if (m.Contains("undefined") || m.Contains("not found")) return "Verify declarations/definitions and symbol scope.";
        if (m.Contains("type")) return "Review type annotations and add explicit casts where needed.";
        if (m.Contains("range")) return "Adjust the declared `__range` or clamp assigned values.";
        if (m.Contains("overflow")) return "Reduce ROM/bank pressure or increase cartridge size config.";
        return "Reduce the code to a minimal repro, then reintroduce pieces incrementally.";
    }

    static int FixHintSeverityRank(string severity)
    {
        if (string.IsNullOrWhiteSpace(severity)) return 3;
        string s = severity.ToLowerInvariant();
        if (s == "fatal") return 0;
        if (s == "error") return 1;
        if (s == "internalerror") return 1;
        if (s == "warning") return 2;
        return 3;
    }

    static bool TryRunAiIrSummaryCommand(string[] argsArray)
    {
        if (argsArray == null || argsArray.Length == 0) return false;
        if (!string.Equals(argsArray[0], "irsum", StringComparison.OrdinalIgnoreCase)) return false;

        string irPath = "";
        string asmPath = "";
        string outPath = "-";
        int topN = 12;

        for (int i = 1; i < argsArray.Length; i++)
        {
            string a = argsArray[i] ?? "";
            if (a == "--stdout" || a == "-") outPath = "-";
            else if (a.StartsWith("--ir=", StringComparison.Ordinal)) irPath = ValueAfterEquals(a);
            else if (a.StartsWith("--asm=", StringComparison.Ordinal)) asmPath = ValueAfterEquals(a);
            else if (a.StartsWith("--out=", StringComparison.Ordinal)) outPath = ValueAfterEquals(a);
            else if (a.StartsWith("--top=", StringComparison.Ordinal))
            {
                if (!int.TryParse(ValueAfterEquals(a), out topN) || topN < 1 || topN > 200)
                {
                    Console.Error.WriteLine("error KQ0000: --top must be 1..200");
                    Exit(1);
                    return true;
                }
            }
            else
            {
                Console.Error.WriteLine("error KQ0000: unknown irsum option: " + a);
                Console.Error.WriteLine("usage: kitaqgb irsum [--ir=<syntax_tree_lowered.txt>] [--asm=<assembly_code.txt>] [--top=N] [--out=<file>|-]");
                Exit(1);
                return true;
            }
        }

        if (string.IsNullOrWhiteSpace(irPath))
            irPath = TryGuessNewestFile("syntax_tree_lowered.txt", recursive: true);
        if (string.IsNullOrWhiteSpace(asmPath))
            asmPath = TryGuessNewestFile("assembly_code.txt", recursive: true);

        if ((string.IsNullOrWhiteSpace(irPath) || !File.Exists(irPath)) &&
            (string.IsNullOrWhiteSpace(asmPath) || !File.Exists(asmPath)))
        {
            Console.Error.WriteLine("error KQ0000: no IR/ASM input found for irsum");
            Exit(1);
            return true;
        }

        string irText = "";
        string asmText = "";
        try
        {
            if (!string.IsNullOrWhiteSpace(irPath) && File.Exists(irPath)) irText = IoUtil.ReadAllTextUtf8(irPath);
            if (!string.IsNullOrWhiteSpace(asmPath) && File.Exists(asmPath)) asmText = IoUtil.ReadAllTextUtf8(asmPath);
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("error KQ0000: failed to read input files: " + ex.Message);
            Exit(1);
            return true;
        }

        int irLines = CountLinesFast(irText);
        int asmLines = CountLinesFast(asmText);
        int irFunctions = CountRegex(irText, "\\(\\$function\\b");
        int irFunctionDecls = CountRegex(irText, "\\(\\$function_decl\\b");
        int irBanks = CountRegex(irText, "\\(\\$bank\\b");
        int irCalls = CountRegex(irText, "\\(\\$call\\b");
        int irVariables = CountRegex(irText, "\\(\\$variable\\b");
        int asmFunctions = 0;
        int asmInstructions = 0;
        var funcStats = ParseAsmFunctionStats(asmText, out asmFunctions, out asmInstructions)
            .OrderByDescending(f => f.InstructionCount)
            .ThenBy(f => f.Name, StringComparer.Ordinal)
            .Take(topN)
            .ToList();

        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB AI IR short summary");
        sb.AppendLine("generated_utc=" + DateTime.UtcNow.ToString("O"));
        sb.AppendLine("ir_file=" + (string.IsNullOrWhiteSpace(irPath) ? "<none>" : Path.GetFullPath(irPath)));
        sb.AppendLine("asm_file=" + (string.IsNullOrWhiteSpace(asmPath) ? "<none>" : Path.GetFullPath(asmPath)));
        sb.AppendLine();
        sb.AppendLine("counts.ir_lines=" + irLines);
        sb.AppendLine("counts.asm_lines=" + asmLines);
        sb.AppendLine("counts.ir_functions=" + irFunctions);
        sb.AppendLine("counts.ir_function_decls=" + irFunctionDecls);
        sb.AppendLine("counts.ir_banks=" + irBanks);
        sb.AppendLine("counts.ir_calls=" + irCalls);
        sb.AppendLine("counts.ir_variables=" + irVariables);
        sb.AppendLine("counts.asm_functions=" + asmFunctions);
        sb.AppendLine("counts.asm_instructions=" + asmInstructions);
        sb.AppendLine();
        sb.AppendLine("top_functions_by_asm_insn:");
        foreach (var f in funcStats)
            sb.AppendLine("- " + f.Name + " = " + f.InstructionCount);

        if (outPath == "-")
            Console.Write(sb.ToString());
        else
        {
            IoUtil.WriteAllTextUtf8Robust(outPath, sb.ToString(), allowAlternatePath: true);
            Console.WriteLine("[irsum] " + outPath);
        }

        Exit(0);
        return true;
    }

    static int CountRegex(string text, string pattern)
    {
        if (string.IsNullOrEmpty(text)) return 0;
        return Regex.Matches(text, pattern).Count;
    }

    static int CountLinesFast(string text)
    {
        if (string.IsNullOrEmpty(text)) return 0;
        int n = 1;
        foreach (char c in text)
        {
            if (c == '\n') n++;
        }
        return n;
    }

    static List<VibeAsmFunctionStat> ParseAsmFunctionStats(string asmText, out int functionCount, out int instructionCount)
    {
        functionCount = 0;
        instructionCount = 0;
        var map = new Dictionary<string, int>(StringComparer.Ordinal);
        if (string.IsNullOrEmpty(asmText)) return new List<VibeAsmFunctionStat>();

        string current = "";
        string[] lines = asmText.Replace("\r\n", "\n").Replace('\r', '\n').Split('\n');
        foreach (string raw in lines)
        {
            string line = raw == null ? "" : raw.Trim();
            if (line.StartsWith("; function ", StringComparison.OrdinalIgnoreCase))
            {
                string name = line.Substring("; function ".Length).Trim();
                if (name.EndsWith(":", StringComparison.Ordinal)) name = name.Substring(0, name.Length - 1).Trim();
                if (!map.ContainsKey(name)) map[name] = 0;
                current = name;
                continue;
            }

            if (string.IsNullOrWhiteSpace(current)) continue;
            if (string.IsNullOrWhiteSpace(line)) continue;
            if (line.StartsWith(";", StringComparison.Ordinal)) continue;
            if (line.EndsWith(":", StringComparison.Ordinal)) continue;

            map[current]++;
            instructionCount++;
        }

        functionCount = map.Count;
        return map.Select(kv => new VibeAsmFunctionStat { Name = kv.Key, InstructionCount = kv.Value }).ToList();
    }

    static bool TryRunDevServerCommand(string[] argsArray)
    {
        if (argsArray == null || argsArray.Length == 0) return false;
        if (!string.Equals(argsArray[0], "devserver", StringComparison.OrdinalIgnoreCase)) return false;

        int pollMs = 250;
        int debounceMs = 180;
        bool once = false;
        int maxBuilds = 0;
        string depsPath = "";
        var compileArgs = new List<string>();

        for (int i = 1; i < argsArray.Length; i++)
        {
            string a = argsArray[i] ?? "";
            if (string.IsNullOrWhiteSpace(a)) continue;
            if (a == "--once") once = true;
            else if (a.StartsWith("--poll-ms=", StringComparison.Ordinal))
            {
                if (!int.TryParse(ValueAfterEquals(a), out pollMs) || pollMs < 50 || pollMs > 5000)
                {
                    Console.Error.WriteLine("error KQ0000: --poll-ms must be 50..5000");
                    Exit(1);
                    return true;
                }
            }
            else if (a.StartsWith("--debounce-ms=", StringComparison.Ordinal))
            {
                if (!int.TryParse(ValueAfterEquals(a), out debounceMs) || debounceMs < 0 || debounceMs > 3000)
                {
                    Console.Error.WriteLine("error KQ0000: --debounce-ms must be 0..3000");
                    Exit(1);
                    return true;
                }
            }
            else if (a.StartsWith("--max-builds=", StringComparison.Ordinal))
            {
                if (!int.TryParse(ValueAfterEquals(a), out maxBuilds) || maxBuilds < 0 || maxBuilds > 100000)
                {
                    Console.Error.WriteLine("error KQ0000: --max-builds must be 0..100000");
                    Exit(1);
                    return true;
                }
            }
            else if (a.StartsWith("--deps=", StringComparison.Ordinal) || a.StartsWith("--deps-out=", StringComparison.Ordinal))
            {
                depsPath = ValueAfterEquals(a);
            }
            else
            {
                compileArgs.Add(a);
            }
        }

        if (compileArgs.Count == 0)
        {
            Console.Error.WriteLine("error KQ0000: devserver requires compile args");
            Console.Error.WriteLine("usage: kitaqgb devserver [--poll-ms=N] [--debounce-ms=N] [--once] [--max-builds=N] [--deps=<file>] <compile args...>");
            Exit(1);
            return true;
        }

        string root = FindRepoRoot(Environment.CurrentDirectory) ?? Environment.CurrentDirectory;
        if (string.IsNullOrWhiteSpace(depsPath))
            depsPath = Path.Combine(root, "integration_test", "reports", "devserver.deps.txt");
        depsPath = Path.GetFullPath(depsPath);

        compileArgs = PrepareDevServerCompileArgs(compileArgs, depsPath);

        bool stopRequested = false;
        Console.CancelKeyPress += (sender, e) =>
        {
            e.Cancel = true;
            stopRequested = true;
        };

        string exePath = Process.GetCurrentProcess().MainModule.FileName;
        int buildCount = 0;
        while (!stopRequested)
        {
            buildCount++;
            Console.WriteLine("[devserver] build #{0}", buildCount);
            int code = RunChildCompiler(exePath, compileArgs.ToArray(), Environment.CurrentDirectory);
            Console.WriteLine("[devserver] exit={0}", code);

            if (once) break;
            if (maxBuilds > 0 && buildCount >= maxBuilds) break;
            if (stopRequested) break;

            var deps = LoadDevServerDependencyFiles(depsPath, compileArgs);
            if (deps.Count == 0)
            {
                Console.WriteLine("[devserver] no dependencies detected, waiting by source args");
                foreach (var f in ExtractSourceFiles(compileArgs))
                {
                    try
                    {
                        string full = Path.GetFullPath(f);
                        deps.Add(full);
                    }
                    catch { }
                }
            }

            var stamps = BuildFileStampMap(deps);
            Console.WriteLine("[devserver] watching {0} file(s)", deps.Count);

            while (!stopRequested)
            {
                Thread.Sleep(pollMs);
                var changed = DetectChangedFiles(deps, stamps, maxFiles: 10);
                if (changed.Count == 0) continue;

                if (debounceMs > 0) Thread.Sleep(debounceMs);
                Console.WriteLine("[devserver] changed: " + string.Join(", ", changed.Select(Path.GetFileName)));
                break;
            }
        }

        Exit(0);
        return true;
    }

    static List<string> PrepareDevServerCompileArgs(List<string> compileArgs, string depsPath)
    {
        var args = new List<string>();
        foreach (var a in compileArgs ?? new List<string>())
        {
            if (string.Equals(a, "--watch", StringComparison.OrdinalIgnoreCase)) continue;
            args.Add(a);
        }

        bool hasFast = args.Any(a => string.Equals(a, "--fast-build", StringComparison.OrdinalIgnoreCase) ||
                                     string.Equals(a, "--fast", StringComparison.OrdinalIgnoreCase) ||
                                     string.Equals(a, "--no-disasm", StringComparison.OrdinalIgnoreCase));
        bool hasCache = args.Any(a => string.Equals(a, "--cache", StringComparison.OrdinalIgnoreCase) ||
                                      string.Equals(a, "--no-cache", StringComparison.OrdinalIgnoreCase));
        bool hasDepsOut = args.Any(a => string.Equals(a, "--deps-out", StringComparison.OrdinalIgnoreCase) ||
                                        a.StartsWith("--deps-out=", StringComparison.OrdinalIgnoreCase));

        if (!hasFast) args.Add("--fast-build");
        if (!hasCache) args.Add("--cache");
        if (!hasDepsOut) args.Add("--deps-out=" + depsPath);
        return args;
    }

    static HashSet<string> LoadDevServerDependencyFiles(string depsPath, List<string> compileArgs)
    {
        var files = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        if (!string.IsNullOrWhiteSpace(depsPath) && File.Exists(depsPath))
        {
            foreach (string raw in IoUtil.ReadAllLinesUtf8(depsPath))
            {
                string line = (raw ?? "").Trim();
                if (line.Length == 0 || line.StartsWith("#", StringComparison.Ordinal)) continue;
                try
                {
                    string full = Path.GetFullPath(line);
                    files.Add(full);
                }
                catch { }
            }
        }

        foreach (var src in ExtractSourceFiles(compileArgs))
        {
            try
            {
                files.Add(Path.GetFullPath(src));
            }
            catch { }
        }

        return files;
    }

    static Dictionary<string, VibeFileStamp> BuildFileStampMap(IEnumerable<string> files)
    {
        var map = new Dictionary<string, VibeFileStamp>(StringComparer.OrdinalIgnoreCase);
        if (files == null) return map;
        foreach (string f in files)
        {
            if (string.IsNullOrWhiteSpace(f)) continue;
            map[f] = GetFileStamp(f);
        }
        return map;
    }

    static VibeFileStamp GetFileStamp(string path)
    {
        try
        {
            var fi = new FileInfo(path);
            if (!fi.Exists)
            {
                return new VibeFileStamp { Exists = false, Length = 0, LastWriteTicksUtc = 0 };
            }
            return new VibeFileStamp
            {
                Exists = true,
                Length = fi.Length,
                LastWriteTicksUtc = fi.LastWriteTimeUtc.Ticks
            };
        }
        catch
        {
            return new VibeFileStamp { Exists = false, Length = 0, LastWriteTicksUtc = 0 };
        }
    }

    static List<string> DetectChangedFiles(IEnumerable<string> files, Dictionary<string, VibeFileStamp> lastMap, int maxFiles)
    {
        var changed = new List<string>();
        if (files == null) return changed;

        foreach (string f in files)
        {
            if (string.IsNullOrWhiteSpace(f)) continue;
            var now = GetFileStamp(f);
            if (!lastMap.TryGetValue(f, out var prev))
            {
                lastMap[f] = now;
                changed.Add(f);
            }
            else if (now.Exists != prev.Exists || now.Length != prev.Length || now.LastWriteTicksUtc != prev.LastWriteTicksUtc)
            {
                lastMap[f] = now;
                changed.Add(f);
            }

            if (changed.Count >= maxFiles) break;
        }

        return changed;
    }

    static bool TryRunRecipeCommand(string[] argsArray)
    {
        if (argsArray == null || argsArray.Length == 0) return false;
        if (!string.Equals(argsArray[0], "recipe", StringComparison.OrdinalIgnoreCase)) return false;

        string historyPath = GetCommandHistoryPath();
        string outPath = "";
        string scriptPath = "";
        bool stdout = false;
        int last = 30;

        for (int i = 1; i < argsArray.Length; i++)
        {
            string a = argsArray[i] ?? "";
            if (a == "--stdout" || a == "-") stdout = true;
            else if (a.StartsWith("--history=", StringComparison.Ordinal)) historyPath = ValueAfterEquals(a);
            else if (a.StartsWith("--out=", StringComparison.Ordinal)) outPath = ValueAfterEquals(a);
            else if (a.StartsWith("--script=", StringComparison.Ordinal)) scriptPath = ValueAfterEquals(a);
            else if (a.StartsWith("--last=", StringComparison.Ordinal))
            {
                if (!int.TryParse(ValueAfterEquals(a), out last) || last < 1 || last > 10000)
                {
                    Console.Error.WriteLine("error KQ0000: --last must be 1..10000");
                    Exit(1);
                    return true;
                }
            }
            else
            {
                Console.Error.WriteLine("error KQ0000: unknown recipe option: " + a);
                Console.Error.WriteLine("usage: kitaqgb recipe [--history=<file>] [--last=N] [--out=<file>] [--script=<file>] [--stdout]");
                Exit(1);
                return true;
            }
        }

        if (string.IsNullOrWhiteSpace(historyPath))
        {
            Console.Error.WriteLine("error KQ0000: recipe history path is empty");
            Exit(1);
            return true;
        }

        if (!File.Exists(historyPath))
        {
            Console.Error.WriteLine("error KQ0000: history file not found: " + historyPath);
            Exit(1);
            return true;
        }

        var entries = LoadHistoryEntries(historyPath).OrderBy(e => e.TimestampUtc).ToList();
        if (entries.Count == 0)
        {
            Console.Error.WriteLine("error KQ0000: history is empty: " + historyPath);
            Exit(1);
            return true;
        }

        var tail = entries.Skip(Math.Max(0, entries.Count - last)).ToList();
        var lastCompile = tail.LastOrDefault(IsCompileHistoryEntry);
        var lastTest = tail.LastOrDefault(e => StartsWithSubcommand(e.ArgsLine, "test"));
        var lastDevServer = tail.LastOrDefault(e => StartsWithSubcommand(e.ArgsLine, "devserver"));

        if (!stdout && string.IsNullOrWhiteSpace(outPath))
        {
            string root = FindRepoRoot(Environment.CurrentDirectory) ?? Environment.CurrentDirectory;
            outPath = Path.Combine(root, "integration_test", "reports", "RECIPE_LATEST.md");
        }
        if (!stdout && string.IsNullOrWhiteSpace(scriptPath))
        {
            string dir = Path.GetDirectoryName(outPath);
            if (string.IsNullOrWhiteSpace(dir)) dir = Environment.CurrentDirectory;
            scriptPath = Path.Combine(dir, "RECIPE_RUN.ps1");
        }

        string recipe = BuildRecipeMarkdown(historyPath, tail, lastCompile, lastTest, lastDevServer);
        if (stdout)
        {
            Console.Write(recipe);
        }
        else
        {
            IoUtil.WriteAllTextUtf8Robust(outPath, recipe, allowAlternatePath: true);
            Console.WriteLine("[recipe] " + outPath);
        }

        if (!stdout && !string.IsNullOrWhiteSpace(scriptPath))
        {
            string script = BuildRecipeScript(lastCompile, lastTest, lastDevServer);
            IoUtil.WriteAllTextUtf8Robust(scriptPath, script, allowAlternatePath: true);
            Console.WriteLine("[recipe] script: " + scriptPath);
        }

        Exit(0);
        return true;
    }

    static List<VibeHistoryEntry> LoadHistoryEntries(string historyPath)
    {
        var list = new List<VibeHistoryEntry>();
        foreach (string raw in IoUtil.ReadAllLinesUtf8(historyPath))
        {
            string line = raw ?? "";
            if (line.Length == 0) continue;
            string[] p = line.Split(new[] { '\t' }, 3);
            if (p.Length < 3) continue;
            if (!DateTime.TryParse(p[0], null, System.Globalization.DateTimeStyles.AdjustToUniversal | System.Globalization.DateTimeStyles.AssumeUniversal, out DateTime ts))
                continue;

            list.Add(new VibeHistoryEntry
            {
                TimestampUtc = ts.ToUniversalTime(),
                WorkingDirectory = p[1] ?? "",
                ArgsLine = p[2] ?? "",
            });
        }
        return list;
    }

    static bool IsCompileHistoryEntry(VibeHistoryEntry e)
    {
        if (e == null) return false;
        string first = FirstArgFromArgsLine(e.ArgsLine);
        if (string.IsNullOrWhiteSpace(first)) return false;
        if (_subcommandSet.Contains(first)) return false;
        return true;
    }

    static bool StartsWithSubcommand(string argsLine, string cmd)
    {
        string first = FirstArgFromArgsLine(argsLine);
        return string.Equals(first, cmd, StringComparison.OrdinalIgnoreCase);
    }

    static string FirstArgFromArgsLine(string argsLine)
    {
        if (string.IsNullOrWhiteSpace(argsLine)) return "";
        string s = argsLine.Trim();
        if (s.Length == 0) return "";
        if (s[0] == '"')
        {
            int i = 1;
            while (i < s.Length)
            {
                if (s[i] == '"' && s[i - 1] != '\\') return s.Substring(1, i - 1);
                i++;
            }
            return s.Trim('"');
        }

        int sp = s.IndexOfAny(new[] { ' ', '\t' });
        if (sp < 0) return s;
        return s.Substring(0, sp);
    }

    static string BuildRecipeMarkdown(string historyPath, List<VibeHistoryEntry> tail, VibeHistoryEntry lastCompile, VibeHistoryEntry lastTest, VibeHistoryEntry lastDevServer)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB Repro Recipe");
        sb.AppendLine();
        sb.AppendLine("generated_utc=" + DateTime.UtcNow.ToString("O"));
        sb.AppendLine("history_file=" + Path.GetFullPath(historyPath));
        sb.AppendLine("history_entries=" + tail.Count);
        sb.AppendLine();
        sb.AppendLine("## Recommended Steps");
        sb.AppendLine("1. Move to working directory.");
        sb.AppendLine("2. Re-run the latest compile command.");
        sb.AppendLine("3. Re-run test/devserver command if needed.");
        sb.AppendLine();

        if (lastCompile != null)
        {
            sb.AppendLine("### Compile");
            sb.AppendLine("- cwd: `" + lastCompile.WorkingDirectory + "`");
            sb.AppendLine("- cmd: `kitaqgb " + lastCompile.ArgsLine + "`");
            sb.AppendLine();
        }
        else
        {
            sb.AppendLine("### Compile");
            sb.AppendLine("- Not found in selected history window.");
            sb.AppendLine();
        }

        if (lastTest != null)
        {
            sb.AppendLine("### Test");
            sb.AppendLine("- cwd: `" + lastTest.WorkingDirectory + "`");
            sb.AppendLine("- cmd: `kitaqgb " + lastTest.ArgsLine + "`");
            sb.AppendLine();
        }

        if (lastDevServer != null)
        {
            sb.AppendLine("### Devserver");
            sb.AppendLine("- cwd: `" + lastDevServer.WorkingDirectory + "`");
            sb.AppendLine("- cmd: `kitaqgb " + lastDevServer.ArgsLine + "`");
            sb.AppendLine();
        }

        sb.AppendLine("## Recent Commands");
        foreach (var e in tail.Skip(Math.Max(0, tail.Count - 10)))
        {
            sb.AppendLine("- " + e.TimestampUtc.ToString("O") + " | `" + e.ArgsLine + "`");
        }

        return sb.ToString();
    }

    static string BuildRecipeScript(VibeHistoryEntry lastCompile, VibeHistoryEntry lastTest, VibeHistoryEntry lastDevServer)
    {
        string cwd = lastCompile?.WorkingDirectory ?? lastTest?.WorkingDirectory ?? lastDevServer?.WorkingDirectory ?? Environment.CurrentDirectory;
        cwd = cwd ?? Environment.CurrentDirectory;

        var sb = new StringBuilder();
        sb.AppendLine("$ErrorActionPreference = 'Stop'");
        sb.AppendLine("Set-Location '" + (cwd.Replace("'", "''")) + "'");
        sb.AppendLine("$compiler = Join-Path (Get-Location) 'kitaqgb.exe'");
        sb.AppendLine("if (!(Test-Path $compiler)) { $compiler = 'kitaqgb' }");
        sb.AppendLine();
        if (lastCompile != null)
        {
            sb.AppendLine("& $compiler " + lastCompile.ArgsLine);
            sb.AppendLine("if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }");
            sb.AppendLine();
        }
        if (lastTest != null)
        {
            sb.AppendLine("& $compiler " + lastTest.ArgsLine);
            sb.AppendLine("if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }");
            sb.AppendLine();
        }
        if (lastDevServer != null)
        {
            sb.AppendLine("& $compiler " + lastDevServer.ArgsLine);
            sb.AppendLine("if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }");
            sb.AppendLine();
        }
        sb.AppendLine("exit 0");
        return sb.ToString();
    }
}




