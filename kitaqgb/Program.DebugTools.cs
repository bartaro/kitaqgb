using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;

static partial class Program
{
    // Keep CPU/bank coordinates separate from the half-open ROM file-offset interval.
    sealed class DebugCliFunctionRange
    {
        public string Name;
        public int Bank;
        public int CpuAddress;
        public int StartFileOffset;
        public int EndFileOffset;
        public int SizeBytes => Math.Max(0, EndFileOffset - StartFileOffset);
    }

    // Retain every display line; a negative offset marks text without a file_off annotation.
    sealed class DebugCliDisLine
    {
        public string Text;
        public int FileOffset;
    }

    // Optional address, bank and size fields preserve missing metadata instead of inventing zeroes.
    sealed class DebugCliSymbol
    {
        public string Name;
        public int? Address;
        public int? Bank;
        public int? Size;
        public string Source;
        public string Kind;
        public string Region;
    }

    // Represent one accepted map row before symbol search or approximate range reconstruction.
    sealed class DebugCliMapEntry
    {
        public string Name;
        public int CpuAddress;
        public int? Bank;
        public int Offset;
        public string Kind;
        public string Region;
    }

    // Store the inclusive source-line span found by the lightweight function scanner.
    sealed class DebugCliSourceFunction
    {
        public string Name;
        public int StartLine; // 1-based
        public int EndLine; // 1-based
    }

    // Summarize one name-matched function, including additions and removals.
    sealed class RomDiffRow
    {
        public string Name;
        public string Status;
        public int OldSize;
        public int NewSize;
        public int DeltaSize;
        public int ChangedBytes;
        public string OldHash;
        public string NewHash;
    }

    // Exclude common statement-like names from the heuristic source-function matches.
    static readonly HashSet<string> SourceControlKeywords = new HashSet<string>(StringComparer.Ordinal)
    {
        "if", "for", "while", "switch", "return", "sizeof", "static_assert", "_Static_assert"
    };

    // Handle only the kqhelp subcommand; format a numeric code even when no enum entry exists.
    static bool TryRunKqHelpCommand(string[] argsArray)
    {
        if (argsArray == null || argsArray.Length == 0) return false;
        if (!string.Equals(argsArray[0], "kqhelp", StringComparison.OrdinalIgnoreCase)) return false;

        if (argsArray.Length < 2)
        {
            Console.Error.WriteLine("error KQ0000: kqhelp requires a code (example: KQ2416)");
            Console.Error.WriteLine("usage: kitaqgb kqhelp <KQxxxx>");
            Exit(1);
            return true;
        }

        if (!TryParseKqCode(argsArray[1], out int codeValue, out string codeText))
        {
            Console.Error.WriteLine("error KQ0000: invalid diagnostic code: " + argsArray[1]);
            Exit(1);
            return true;
        }

        Console.Write(BuildKqHelpText(codeValue, codeText));
        Exit(0);
        return true;
    }

    // Search map/debug symbols by substring or an explicitly requested regular expression.
    static bool TryRunSymbolFindCommand(string[] argsArray)
    {
        if (argsArray == null || argsArray.Length == 0) return false;
        if (!string.Equals(argsArray[0], "symfind", StringComparison.OrdinalIgnoreCase)) return false;

        if (argsArray.Length < 2)
        {
            Console.Error.WriteLine("error KQ0000: symfind requires a pattern");
            Console.Error.WriteLine("usage: kitaqgb symfind <pattern> [--map=<.map>] [--dbg=<.dbg>] [--regex] [--ignore-case] [--out=<file>|-]");
            Exit(1);
            return true;
        }

        string pattern = argsArray[1] ?? "";
        string mapPath = "";
        string dbgPath = "";
        string outPath = "";
        bool useRegex = false;
        bool ignoreCase = false;

        for (int i = 2; i < argsArray.Length; i++)
        {
            string a = argsArray[i] ?? "";
            if (a.StartsWith("--map=", StringComparison.Ordinal)) mapPath = ValueAfterEquals(a);
            else if (a.StartsWith("--dbg=", StringComparison.Ordinal)) dbgPath = ValueAfterEquals(a);
            else if (a == "--regex") useRegex = true;
            else if (a == "--ignore-case") ignoreCase = true;
            else if (a.StartsWith("--out=", StringComparison.Ordinal)) outPath = ValueAfterEquals(a);
            else
            {
                Console.Error.WriteLine("error KQ0000: unknown symfind option: " + a);
                Exit(1);
                return true;
            }
        }

        // Auto-discover map and debug inputs independently; explicit paths are needed to select a matching build.
        if (string.IsNullOrWhiteSpace(mapPath))
            mapPath = TryGuessNewestFile("*.map", recursive: true);
        if (string.IsNullOrWhiteSpace(dbgPath))
            dbgPath = TryGuessNewestFile("*.dbg", recursive: true);

        // Retain both sources, including duplicate names, so the report can show their provenance.
        var symbols = new List<DebugCliSymbol>();
        if (!string.IsNullOrWhiteSpace(mapPath) && File.Exists(mapPath))
            symbols.AddRange(ParseMapSymbolsForCli(mapPath));
        if (!string.IsNullOrWhiteSpace(dbgPath) && File.Exists(dbgPath))
            symbols.AddRange(ParseDbgSymbolsForCli(dbgPath));

        if (symbols.Count == 0)
        {
            Console.Error.WriteLine("error KQ0000: no symbols loaded (specify --map and/or --dbg)");
            Exit(1);
            return true;
        }

        // Validate a requested regex before filtering; ordinary patterns are literal substrings.
        Regex re = null;
        if (useRegex)
        {
            try
            {
                var options = ignoreCase ? RegexOptions.IgnoreCase : RegexOptions.None;
                re = new Regex(pattern, options);
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine("error KQ0000: invalid regex: " + ex.Message);
                Exit(1);
                return true;
            }
        }

        bool MatchName(string name)
        {
            if (useRegex) return re.IsMatch(name ?? "");
            if (ignoreCase) return (name ?? "").IndexOf(pattern, StringComparison.OrdinalIgnoreCase) >= 0;
            return (name ?? "").IndexOf(pattern, StringComparison.Ordinal) >= 0;
        }

        // Sort by name and input format for stable output independent of enumeration order.
        var matched = symbols
            .Where(s => MatchName(s.Name))
            .OrderBy(s => s.Name, StringComparer.Ordinal)
            .ThenBy(s => s.Source, StringComparer.Ordinal)
            .ToList();

        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB symbol search");
        sb.AppendLine("pattern=" + pattern);
        sb.AppendLine("use_regex=" + (useRegex ? "1" : "0"));
        sb.AppendLine("ignore_case=" + (ignoreCase ? "1" : "0"));
        sb.AppendLine("map=" + (string.IsNullOrWhiteSpace(mapPath) ? "<none>" : mapPath));
        sb.AppendLine("dbg=" + (string.IsNullOrWhiteSpace(dbgPath) ? "<none>" : dbgPath));
        sb.AppendLine("match_count=" + matched.Count);
        sb.AppendLine();
        sb.AppendLine("# name, address, bank, size, kind, region, source");
        foreach (var m in matched)
        {
            string addr = m.Address.HasValue ? ("0x" + (m.Address.Value & 0xFFFF).ToString("X4")) : "";
            string bank = m.Bank.HasValue ? m.Bank.Value.ToString() : "";
            string size = m.Size.HasValue ? m.Size.Value.ToString() : "";
            string region = m.Region ?? "";
            sb.AppendFormat("{0}, {1}, {2}, {3}, {4}, {5}, {6}\n",
                m.Name ?? "", addr, bank, size, m.Kind ?? "", region, m.Source ?? "");
        }

        if (string.IsNullOrWhiteSpace(outPath))
            outPath = "kitaqgb_symfind.txt";

        if (outPath == "-")
            Console.Write(sb.ToString());
        else
        {
            IoUtil.WriteAllTextUtf8Robust(outPath, sb.ToString(), allowAlternatePath: true);
            Console.WriteLine("[symfind] " + outPath);
        }

        Exit(0);
        return true;
    }

    // Find the function containing the requested source line, then display its assembly range.
    // This is a function-level lookup, not an instruction-to-source-line mapping.
    static bool TryRunSourceToAsmCommand(string[] argsArray)
    {
        if (argsArray == null || argsArray.Length == 0) return false;
        if (!string.Equals(argsArray[0], "src2asm", StringComparison.OrdinalIgnoreCase)) return false;

        if (argsArray.Length < 2)
        {
            Console.Error.WriteLine("error KQ0000: src2asm requires source.c:line");
            Console.Error.WriteLine("usage: kitaqgb src2asm <source.c:line> [--disasm=<dis.s>] [--funcsizes=<.funcsizes.txt>] [--context=N] [--out=<file>|-]");
            Exit(1);
            return true;
        }

        string spec = argsArray[1] ?? "";
        string disasmPath = "";
        string funcsizesPath = "";
        string mapPath = "";
        string outPath = "";
        int context = 2;

        for (int i = 2; i < argsArray.Length; i++)
        {
            string a = argsArray[i] ?? "";
            if (a.StartsWith("--disasm=", StringComparison.Ordinal)) disasmPath = ValueAfterEquals(a);
            else if (a.StartsWith("--funcsizes=", StringComparison.Ordinal)) funcsizesPath = ValueAfterEquals(a);
            else if (a.StartsWith("--map=", StringComparison.Ordinal)) mapPath = ValueAfterEquals(a);
            else if (a.StartsWith("--context=", StringComparison.Ordinal))
            {
                if (!int.TryParse(ValueAfterEquals(a), out context) || context < 0 || context > 64)
                {
                    Console.Error.WriteLine("error KQ0000: --context must be 0..64");
                    Exit(1);
                    return true;
                }
            }
            else if (a.StartsWith("--out=", StringComparison.Ordinal)) outPath = ValueAfterEquals(a);
            else
            {
                Console.Error.WriteLine("error KQ0000: unknown src2asm option: " + a);
                Exit(1);
                return true;
            }
        }

        if (!TryParseSourceLineSpec(spec, out string sourcePath, out int line1Based))
        {
            Console.Error.WriteLine("error KQ0000: src2asm expects source.c:line");
            Exit(1);
            return true;
        }
        if (!File.Exists(sourcePath))
        {
            Console.Error.WriteLine("error KQ0000: source file not found: " + sourcePath);
            Exit(1);
            return true;
        }

        if (string.IsNullOrWhiteSpace(disasmPath))
        {
            string defaultDis = Path.Combine("debug_output", "dis.s");
            if (File.Exists(defaultDis)) disasmPath = defaultDis;
            else disasmPath = TryGuessNewestFile("dis.s", recursive: true);
        }
        if (string.IsNullOrWhiteSpace(funcsizesPath))
            funcsizesPath = TryGuessNewestFile("*.funcsizes.txt", recursive: true);
        if (string.IsNullOrWhiteSpace(mapPath))
            mapPath = TryGuessNewestFile("*.map", recursive: true);

        if (string.IsNullOrWhiteSpace(disasmPath) || !File.Exists(disasmPath))
        {
            Console.Error.WriteLine("error KQ0000: disassembly not found (use --disasm)");
            Exit(1);
            return true;
        }

        var sourceFunctions = ParseSourceFunctions(sourcePath);
        DebugCliSourceFunction target = sourceFunctions
            .FirstOrDefault(f => line1Based >= f.StartLine && line1Based <= f.EndLine);
        if (target == null)
        {
            Console.Error.WriteLine("error KQ0000: no function found at line " + line1Based);
            Exit(1);
            return true;
        }

        // Prefer explicit function extents; use map-derived approximations only when the sizes file is absent.
        List<DebugCliFunctionRange> ranges = null;
        if (!string.IsNullOrWhiteSpace(funcsizesPath) && File.Exists(funcsizesPath))
            ranges = ParseFunctionRanges(funcsizesPath);
        else if (!string.IsNullOrWhiteSpace(mapPath) && File.Exists(mapPath))
            ranges = ParseFunctionRangesFromMap(mapPath, romSizeBytes: 0);
        else
            ranges = new List<DebugCliFunctionRange>();

        // Select the first exact function name; source paths and duplicate-name scopes are not disambiguated here.
        var funcRange = ranges.FirstOrDefault(r => string.Equals(r.Name, target.Name, StringComparison.Ordinal));
        if (funcRange == null)
        {
            Console.Error.WriteLine("error KQ0000: function '" + target.Name + "' not found in funcsizes/map");
            Exit(1);
            return true;
        }

        var disLines = ParseDisassemblyLines(disasmPath);
        // Match annotated ROM offsets, excluding the half-open interval end.
        var hit = new List<int>();
        for (int i = 0; i < disLines.Count; i++)
        {
            int off = disLines[i].FileOffset;
            if (off >= 0 && off >= funcRange.StartFileOffset && off < funcRange.EndFileOffset)
                hit.Add(i);
        }

        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB source -> assembly lookup");
        sb.AppendLine("source=" + sourcePath + ":" + line1Based);
        sb.AppendLine("function=" + target.Name + " (" + target.StartLine + "-" + target.EndLine + ")");
        sb.AppendLine("range=0x" + funcRange.StartFileOffset.ToString("X5") + "..0x" + funcRange.EndFileOffset.ToString("X5"));
        sb.AppendLine("disasm=" + disasmPath);
        sb.AppendLine();

        if (hit.Count == 0)
        {
            sb.AppendLine("# no disassembly lines matched this function range");
        }
        else
        {
            // Include surrounding display lines around the full function excerpt, clamped to the listing.
            int from = Math.Max(0, hit.Min() - context);
            int to = Math.Min(disLines.Count - 1, hit.Max() + context);
            sb.AppendLine("# disassembly excerpt");
            for (int i = from; i <= to; i++)
            {
                sb.AppendLine(disLines[i].Text);
            }
        }

        if (string.IsNullOrWhiteSpace(outPath))
            outPath = "kitaqgb_src2asm.txt";

        if (outPath == "-")
            Console.Write(sb.ToString());
        else
        {
            IoUtil.WriteAllTextUtf8Robust(outPath, sb.ToString(), allowAlternatePath: true);
            Console.WriteLine("[src2asm] " + outPath);
        }

        Exit(0);
        return true;
    }

    // Compare raw byte slices by function name using sidecar sizes or approximate map ranges.
    static bool TryRunRomDiffCommand(string[] argsArray)
    {
        if (argsArray == null || argsArray.Length == 0) return false;
        if (!string.Equals(argsArray[0], "romdiff", StringComparison.OrdinalIgnoreCase)) return false;

        if (argsArray.Length < 3)
        {
            Console.Error.WriteLine("error KQ0000: romdiff requires <old.gb> <new.gb>");
            Console.Error.WriteLine("usage: kitaqgb romdiff <old.gb> <new.gb> [--old-func=<.funcsizes.txt>] [--new-func=<.funcsizes.txt>] [--old-map=<.map>] [--new-map=<.map>] [--out=<file>|-]");
            Exit(1);
            return true;
        }

        string oldRomPath = argsArray[1];
        string newRomPath = argsArray[2];
        string oldFuncPath = "";
        string newFuncPath = "";
        string oldMapPath = "";
        string newMapPath = "";
        string outPath = "";

        for (int i = 3; i < argsArray.Length; i++)
        {
            string a = argsArray[i] ?? "";
            if (a.StartsWith("--old-func=", StringComparison.Ordinal)) oldFuncPath = ValueAfterEquals(a);
            else if (a.StartsWith("--new-func=", StringComparison.Ordinal)) newFuncPath = ValueAfterEquals(a);
            else if (a.StartsWith("--old-map=", StringComparison.Ordinal)) oldMapPath = ValueAfterEquals(a);
            else if (a.StartsWith("--new-map=", StringComparison.Ordinal)) newMapPath = ValueAfterEquals(a);
            else if (a.StartsWith("--out=", StringComparison.Ordinal)) outPath = ValueAfterEquals(a);
            else
            {
                Console.Error.WriteLine("error KQ0000: unknown romdiff option: " + a);
                Exit(1);
                return true;
            }
        }

        if (!File.Exists(oldRomPath) || !File.Exists(newRomPath))
        {
            Console.Error.WriteLine("error KQ0000: ROM file not found");
            Exit(1);
            return true;
        }

        if (string.IsNullOrWhiteSpace(oldFuncPath))
            oldFuncPath = Path.ChangeExtension(oldRomPath, ".funcsizes.txt");
        if (string.IsNullOrWhiteSpace(newFuncPath))
            newFuncPath = Path.ChangeExtension(newRomPath, ".funcsizes.txt");
        if (string.IsNullOrWhiteSpace(oldMapPath))
            oldMapPath = Path.ChangeExtension(oldRomPath, ".map");
        if (string.IsNullOrWhiteSpace(newMapPath))
            newMapPath = Path.ChangeExtension(newRomPath, ".map");

        byte[] oldRom = File.ReadAllBytes(oldRomPath);
        byte[] newRom = File.ReadAllBytes(newRomPath);

        List<DebugCliFunctionRange> oldFns = File.Exists(oldFuncPath)
            ? ParseFunctionRanges(oldFuncPath)
            : ParseFunctionRangesFromMap(oldMapPath, oldRom.Length);
        List<DebugCliFunctionRange> newFns = File.Exists(newFuncPath)
            ? ParseFunctionRanges(newFuncPath)
            : ParseFunctionRangesFromMap(newMapPath, newRom.Length);

        if (oldFns.Count == 0 && newFns.Count == 0)
        {
            Console.Error.WriteLine("error KQ0000: function range info not found (funcsizes/map)");
            Exit(1);
            return true;
        }

        // Use the first range for each name; a moved but byte-identical body is still classified as same.
        var oldByName = oldFns.GroupBy(f => f.Name).ToDictionary(g => g.Key, g => g.First(), StringComparer.Ordinal);
        var newByName = newFns.GroupBy(f => f.Name).ToDictionary(g => g.Key, g => g.First(), StringComparer.Ordinal);
        var allNames = new HashSet<string>(oldByName.Keys, StringComparer.Ordinal);
        foreach (var n in newByName.Keys) allNames.Add(n);

        var rows = new List<RomDiffRow>();
        foreach (var name in allNames)
        {
            oldByName.TryGetValue(name, out DebugCliFunctionRange oldFn);
            newByName.TryGetValue(name, out DebugCliFunctionRange newFn);

            // Treat the entire reported extent of a newly named function as added bytes.
            if (oldFn == null && newFn != null)
            {
                rows.Add(new RomDiffRow
                {
                    Name = name,
                    Status = "added",
                    OldSize = 0,
                    NewSize = newFn.SizeBytes,
                    DeltaSize = newFn.SizeBytes,
                    ChangedBytes = newFn.SizeBytes,
                    OldHash = "",
                    NewHash = ComputeRangeHash(newRom, newFn.StartFileOffset, newFn.EndFileOffset)
                });
                continue;
            }
            // Treat the entire reported extent of a missing function as removed bytes.
            if (oldFn != null && newFn == null)
            {
                rows.Add(new RomDiffRow
                {
                    Name = name,
                    Status = "removed",
                    OldSize = oldFn.SizeBytes,
                    NewSize = 0,
                    DeltaSize = -oldFn.SizeBytes,
                    ChangedBytes = oldFn.SizeBytes,
                    OldHash = ComputeRangeHash(oldRom, oldFn.StartFileOffset, oldFn.EndFileOffset),
                    NewHash = ""
                });
                continue;
            }

            // Align by relative position within each slice; relocation operands are compared as ordinary bytes.
            int changed = CountChangedBytes(oldRom, oldFn.StartFileOffset, oldFn.EndFileOffset, newRom, newFn.StartFileOffset, newFn.EndFileOffset);
            string oldHash = ComputeRangeHash(oldRom, oldFn.StartFileOffset, oldFn.EndFileOffset);
            string newHash = ComputeRangeHash(newRom, newFn.StartFileOffset, newFn.EndFileOffset);
            rows.Add(new RomDiffRow
            {
                Name = name,
                Status = changed == 0 ? "same" : "changed",
                OldSize = oldFn.SizeBytes,
                NewSize = newFn.SizeBytes,
                DeltaSize = newFn.SizeBytes - oldFn.SizeBytes,
                ChangedBytes = changed,
                OldHash = oldHash,
                NewHash = newHash
            });
        }

        // Show the largest byte changes first, then size changes, with names as the final tie-breaker.
        rows = rows
            .OrderByDescending(r => r.ChangedBytes)
            .ThenByDescending(r => Math.Abs(r.DeltaSize))
            .ThenBy(r => r.Name, StringComparer.Ordinal)
            .ToList();

        int changedCount = rows.Count(r => r.Status == "changed");
        int addedCount = rows.Count(r => r.Status == "added");
        int removedCount = rows.Count(r => r.Status == "removed");
        int sameCount = rows.Count(r => r.Status == "same");

        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB ROM diff (function-level)");
        sb.AppendLine("old_rom=" + oldRomPath);
        sb.AppendLine("new_rom=" + newRomPath);
        sb.AppendLine("old_ranges=" + (File.Exists(oldFuncPath) ? oldFuncPath : oldMapPath));
        sb.AppendLine("new_ranges=" + (File.Exists(newFuncPath) ? newFuncPath : newMapPath));
        sb.AppendLine("function_count=" + rows.Count);
        sb.AppendLine("changed=" + changedCount + ", added=" + addedCount + ", removed=" + removedCount + ", same=" + sameCount);
        sb.AppendLine();
        sb.AppendLine("# name, status, old_size, new_size, delta, changed_bytes, old_hash8, new_hash8");
        foreach (var r in rows)
        {
            sb.AppendFormat("{0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}\n",
                r.Name, r.Status, r.OldSize, r.NewSize, r.DeltaSize, r.ChangedBytes,
                ShortHash(r.OldHash), ShortHash(r.NewHash));
        }

        if (string.IsNullOrWhiteSpace(outPath))
            outPath = "kitaqgb_romdiff.txt";

        if (outPath == "-")
            Console.Write(sb.ToString());
        else
        {
            IoUtil.WriteAllTextUtf8Robust(outPath, sb.ToString(), allowAlternatePath: true);
            Console.WriteLine("[romdiff] " + outPath);
        }

        Exit(0);
        return true;
    }


    // Emit aggregate timings and a trace sidecar without turning optional reporting failures into build failures.
    static void EmitTraceShortSummary(List<TraceStageStat> stats, long totalMs, List<string> sourceFilenames, string outputFilename)
    {
        try
        {
            var s = stats ?? new List<TraceStageStat>();
            var compact = string.Join(", ", s.Select(x => (x.Name ?? "") + "=" + x.ElapsedMs + "ms"));
            if (compact.Length == 0) compact = "no-stage-data";
            WriteInfoLine("[trace] total=" + totalMs + "ms, " + compact);

            var sb = new StringBuilder();
            sb.AppendLine("# KITAQGB trace short summary");
            sb.AppendLine("files=" + (sourceFilenames == null ? 0 : sourceFilenames.Count));
            sb.AppendLine("output=" + (outputFilename ?? ""));
            sb.AppendLine("total_ms=" + totalMs);
            sb.AppendLine();
            sb.AppendLine("# stage, elapsed_ms, detail");
            foreach (var st in s)
            {
                sb.AppendFormat("{0}, {1}, {2}\n", st.Name ?? "", st.ElapsedMs, st.Detail ?? "");
            }

            WriteTraceFile("trace_summary.txt", sb.ToString());
        }
        catch
        {
            // best effort
        }
    }


    // Select all functions in changed source files, then intersect their names with this assembly report.
    // The selection is file-based; it does not inspect Git hunks or compare function bodies.
    static void TryWriteChangedFunctionDisasm(string outputFilename, List<string> sourceFilenames, string gitBaseRef, string outPathOption)
    {
        try
        {
            string disPath = Path.Combine(DebugOutputPath ?? "debug_output", "dis.s");
            if (!File.Exists(disPath)) return;

            var changedSourceFiles = GetChangedSourceFiles(gitBaseRef);
            // When Git yields no usable files, fall back to the existing source files supplied for this build.
            if (changedSourceFiles.Count == 0)
            {
                foreach (var s in sourceFilenames ?? new List<string>())
                {
                    if (File.Exists(s)) changedSourceFiles.Add(Path.GetFullPath(s));
                }
            }

            var changedFunctions = new HashSet<string>(StringComparer.Ordinal);
            foreach (var f in changedSourceFiles)
            {
                foreach (var fn in ParseSourceFunctions(f))
                {
                    if (!string.IsNullOrEmpty(fn.Name)) changedFunctions.Add(fn.Name);
                }
            }

            string outPath = outPathOption;
            if (string.IsNullOrWhiteSpace(outPath))
                outPath = Path.Combine(DebugOutputPath ?? "debug_output", "dis_changed.s");

            var ranges = new List<DebugCliFunctionRange>();
            foreach (var f in Assembler.LastReport.FunctionSizes)
            {
                ranges.Add(new DebugCliFunctionRange
                {
                    Name = f.Name,
                    Bank = f.Bank,
                    CpuAddress = f.CpuAddress,
                    StartFileOffset = f.StartFileOffset,
                    EndFileOffset = f.EndFileOffset
                });
            }

            // Discard source-only names that produced no reported machine-code range.
            var targetRanges = ranges
                .Where(r => changedFunctions.Contains(r.Name))
                .OrderBy(r => r.StartFileOffset)
                .ToList();

            var sb = new StringBuilder();
            sb.AppendLine("; KITAQGB changed-function disassembly");
            sb.AppendLine("; base_ref=" + (string.IsNullOrWhiteSpace(gitBaseRef) ? "<working_tree>" : gitBaseRef));
            sb.AppendLine("; changed_source_files=" + changedSourceFiles.Count);
            sb.AppendLine("; changed_functions=" + changedFunctions.Count);
            sb.AppendLine("; emitted_functions=" + targetRanges.Count);
            sb.AppendLine();

            if (targetRanges.Count == 0)
            {
                sb.AppendLine("; no changed functions found");
                IoUtil.WriteAllTextUtf8Robust(outPath, sb.ToString(), allowAlternatePath: true);
                Console.WriteLine("[debug] changed disasm: " + outPath + " (0 function)");
                return;
            }

            var disLines = ParseDisassemblyLines(disPath);
            // Emit each selected range independently, retaining only offset-annotated instructions.
            foreach (var fr in targetRanges)
            {
                sb.AppendLine("; ------------------------------------------------------------");
                sb.AppendLine("; function " + fr.Name + "  file_off=$" + fr.StartFileOffset.ToString("X5") + "..$" + fr.EndFileOffset.ToString("X5"));
                sb.AppendLine("; ------------------------------------------------------------");
                int count = 0;
                foreach (var line in disLines)
                {
                    if (line.FileOffset < 0) continue;
                    if (line.FileOffset >= fr.StartFileOffset && line.FileOffset < fr.EndFileOffset)
                    {
                        sb.AppendLine(line.Text);
                        count++;
                    }
                }
                if (count == 0) sb.AppendLine("; (no disassembly lines found)");
                sb.AppendLine();
            }

            IoUtil.WriteAllTextUtf8Robust(outPath, sb.ToString(), allowAlternatePath: true);
            Console.WriteLine("[debug] changed disasm: " + outPath + " (" + targetRanges.Count + " function)");
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("warning KQ0000: failed to emit changed-function disasm: " + ex.Message);
        }
    }

    // Snapshot diagnostics, invocation and known inputs once; individual copy failures are best effort.
    static void TryWriteFailureReproPackage(int exitCode)
    {
        if (_reproPackageWritten) return;
        // Set the guard before any I/O so a packaging failure cannot recursively trigger another package.
        _reproPackageWritten = true;

        try
        {
            string root = ReproPackagePath;
            if (string.IsNullOrWhiteSpace(root))
                root = Path.Combine(Environment.CurrentDirectory, "kitaqgb_fail_repro_" + DateTime.UtcNow.ToString("yyyyMMdd_HHmmssfff"));

            Directory.CreateDirectory(root);

            string jsonPath = Path.Combine(root, "diagnostics.json");
            IoUtil.WriteAllTextUtf8Robust(jsonPath, BuildDiagnosticJsonText(exitCode), allowAlternatePath: true);

            var args = _originalArgs ?? new string[0];
            IoUtil.WriteAllLinesUtf8Robust(Path.Combine(root, "args.txt"), args, allowAlternatePath: true);
            IoUtil.WriteAllTextUtf8Robust(Path.Combine(root, "command.txt"),
                "kitaqgb " + string.Join(" ", args.Select(QuoteArg)),
                allowAlternatePath: true);

            var files = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            foreach (var s in ExtractSourceFiles(args.ToList()))
            {
                try
                {
                    string full = Path.GetFullPath(s);
                    if (File.Exists(full)) files.Add(full);
                }
                catch { }
            }
            // Include resolved dependencies as well as command-line source inputs, deduplicated by full path.
            foreach (var d in LastCompilationDependencies ?? Array.Empty<string>())
            {
                try
                {
                    string full = Path.GetFullPath(d);
                    if (File.Exists(full)) files.Add(full);
                }
                catch { }
            }

            // Keep copied inputs under a separate directory; recorded command paths are not rewritten for relocation.
            string inputRoot = Path.Combine(root, "inputs");
            Directory.CreateDirectory(inputRoot);
            string cwd = Environment.CurrentDirectory;
            int copied = 0;
            foreach (var src in files)
            {
                try
                {
                    string rel = MakeRelativePathSafe(cwd, src);
                    if (string.IsNullOrWhiteSpace(rel)) rel = Path.GetFileName(src);
                    // Trim leading parent traversal or a rooted path before copying; distinct inputs can still share a destination.
                    rel = SanitizeRelativePath(rel);
                    string dst = Path.Combine(inputRoot, rel);
                    Directory.CreateDirectory(Path.GetDirectoryName(dst) ?? inputRoot);
                    IoUtil.CopyFileRobust(src, dst, overwrite: true);
                    copied++;
                }
                catch { }
            }

            var readme = new StringBuilder();
            readme.AppendLine("# KITAQGB build failure repro package");
            readme.AppendLine("utc=" + DateTime.UtcNow.ToString("yyyy-MM-ddTHH:mm:ssZ"));
            readme.AppendLine("exit_code=" + exitCode);
            readme.AppendLine("copied_input_files=" + copied);
            readme.AppendLine();
            readme.AppendLine("Files:");
            readme.AppendLine("- diagnostics.json : machine-readable diagnostics");
            readme.AppendLine("- args.txt / command.txt : invocation");
            readme.AppendLine("- inputs/ : source and dependency snapshot");
            IoUtil.WriteAllTextUtf8Robust(Path.Combine(root, "README.txt"), readme.ToString(), allowAlternatePath: true);

            Console.Error.WriteLine("[repro-pack] " + root);
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("warning KQ0000: failed to write repro package: " + ex.Message);
        }
    }

    // Serialize the accumulated diagnostics with escaped string fields and numeric source locations.
    static string BuildDiagnosticJsonText(int exitCode)
    {
        var sb = new StringBuilder();
        sb.Append("{");
        sb.Append("\"exit_code\":").Append(exitCode).Append(",");
        sb.Append("\"errors\":").Append(ErrorCount).Append(",");
        sb.Append("\"warnings\":").Append(WarningCount).Append(",");
        sb.Append("\"output\":\"").Append(JsonEscape(_outputFilenameForDiag ?? "")).Append("\",");
        sb.Append("\"diagnostics\":[");
        for (int i = 0; i < Diagnostics.Count; i++)
        {
            var d = Diagnostics[i];
            if (i != 0) sb.Append(",");
            sb.Append("{");
            sb.Append("\"severity\":\"").Append(JsonEscape(d.Severity)).Append("\",");
            sb.Append("\"code\":\"").Append(JsonEscape(d.Code)).Append("\",");
            sb.Append("\"message\":\"").Append(JsonEscape(d.Message)).Append("\",");
            sb.Append("\"suggestion\":\"").Append(JsonEscape(d.Suggestion ?? "")).Append("\",");
            sb.Append("\"file\":\"").Append(JsonEscape(d.Filename)).Append("\",");
            sb.Append("\"line\":").Append(d.Line).Append(",");
            sb.Append("\"column\":").Append(d.Column);
            sb.Append("}");
        }
        sb.Append("]}");
        return sb.ToString();
    }

    // Collect existing C/header paths from Git diff and, without a base ref, untracked files.
    // An empty result also covers Git failures; the caller supplies its source-list fallback.
    static HashSet<string> GetChangedSourceFiles(string gitBaseRef)
    {
        var set = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        try
        {
            var diffArgs = new List<string>();
            diffArgs.Add("diff");
            diffArgs.Add("--name-only");
            diffArgs.Add("--diff-filter=ACMRTUXB");
            if (!string.IsNullOrWhiteSpace(gitBaseRef)) diffArgs.Add(gitBaseRef);

            foreach (var line in RunGitLines(diffArgs.ToArray()))
            {
                string p = line.Trim();
                if (p.Length == 0) continue;
                string ext = Path.GetExtension(p);
                if (!ext.Equals(".c", StringComparison.OrdinalIgnoreCase) && !ext.Equals(".h", StringComparison.OrdinalIgnoreCase)) continue;
                string full = Path.GetFullPath(p);
                if (File.Exists(full)) set.Add(full);
            }

            if (string.IsNullOrWhiteSpace(gitBaseRef))
            {
                foreach (var line in RunGitLines("ls-files", "--others", "--exclude-standard"))
                {
                    string p = line.Trim();
                    if (p.Length == 0) continue;
                    string ext = Path.GetExtension(p);
                    if (!ext.Equals(".c", StringComparison.OrdinalIgnoreCase) && !ext.Equals(".h", StringComparison.OrdinalIgnoreCase)) continue;
                    string full = Path.GetFullPath(p);
                    if (File.Exists(full)) set.Add(full);
                }
            }
        }
        catch
        {
            // ignore: fallback is handled by caller
        }
        return set;
    }

    // Invoke Git directly in the current directory and split successful stdout into nonempty path lines.
    static IEnumerable<string> RunGitLines(params string[] gitArgs)
    {
        var psi = new ProcessStartInfo("git")
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            WorkingDirectory = Environment.CurrentDirectory,
            Arguments = string.Join(" ", (gitArgs ?? Array.Empty<string>()).Select(QuoteArg))
        };
        using (var p = Process.Start(psi))
        {
            string stdout = p.StandardOutput.ReadToEnd();
            p.WaitForExit();
            if (p.ExitCode != 0) return Array.Empty<string>();
            return stdout.Split(new[] { "\r\n", "\n" }, StringSplitOptions.RemoveEmptyEntries);
        }
    }

    // Combine the registered description, numeric category and existing fix suggestion into command help.
    static string BuildKqHelpText(int codeValue, string codeText)
    {
        string enumName = "";
        if (Enum.IsDefined(typeof(ErrorCode), codeValue))
        {
            enumName = ((ErrorCode)codeValue).ToString();
        }

        string category = GetKqCategory(codeValue);
        string desc = GetKqDescription(codeValue);
        string hint = GetFixSuggestion((ErrorCode)codeValue, "");
        string strictBehavior = (codeValue >= 2400 && codeValue < 2500)
            ? "strict modeではerrorに昇格"
            : "通常ルール";

        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB diagnostic help");
        sb.AppendLine("code=" + codeText);
        sb.AppendLine("enum=" + (string.IsNullOrEmpty(enumName) ? "<unknown>" : enumName));
        sb.AppendLine("category=" + category);
        sb.AppendLine("severity_behavior=" + strictBehavior);
        sb.AppendLine();
        sb.AppendLine(desc);
        if (!string.IsNullOrWhiteSpace(hint))
        {
            sb.AppendLine();
            sb.AppendLine("suggestion: " + hint);
        }
        return sb.ToString();
    }

    // Accept a number with an optional KQ prefix and normalize it to four digits in the range 0000..9999.
    static bool TryParseKqCode(string input, out int codeValue, out string codeText)
    {
        codeValue = 0;
        codeText = "KQ0000";
        if (string.IsNullOrWhiteSpace(input)) return false;

        string s = input.Trim().ToUpperInvariant();
        if (s.StartsWith("KQ", StringComparison.Ordinal))
            s = s.Substring(2);
        if (!int.TryParse(s, out codeValue) || codeValue < 0 || codeValue > 9999) return false;

        codeText = "KQ" + codeValue.ToString("0000");
        return true;
    }

    // Classify by reserved numeric ranges; this does not require an individually registered code.
    static string GetKqCategory(int code)
    {
        if (code >= 1000 && code < 2000) return "parser/front-end";
        if (code >= 2000 && code < 2200) return "type/semantic";
        if (code >= 2200 && code < 2300) return "const/qualifier";
        if (code >= 2300 && code < 2400) return "compile-time assert";
        if (code >= 2400 && code < 2500) return "lint/warning";
        if (code >= 2500 && code < 2600) return "extern linkage";
        if (code >= 9000) return "internal";
        return "generic";
    }

    // Look up the existing diagnostic descriptions; unregistered codes receive a generic fallback.
    static string GetKqDescription(int code)
    {
        switch ((ErrorCode)code)
        {
            case ErrorCode.ParseError: return "構文を解析できません。トークンの欠落や順序不整合が疑われます。";
            case ErrorCode.ExpectedToken: return "期待された記号/トークンが見つかりません。";
            case ErrorCode.ExpectedType: return "型指定が必要な位置で型が解決できませんでした。";
            case ErrorCode.AggregateNotDefined: return "struct/union が未定義のまま参照されています。";
            case ErrorCode.IncompleteType: return "不完全型のままサイズ確定やフィールドアクセスが要求されました。";
            case ErrorCode.ConstAssign: return "constオブジェクトへの代入が検出されました。";
            case ErrorCode.ConstModify: return "const修飾の実体を変更しようとしています。";
            case ErrorCode.StaticAssertFailed: return "static_assert の条件が偽です。";
            case ErrorCode.MustCheckUnused: return "__must_check関数の戻り値が未使用です。";
            case ErrorCode.NonNullArgument: return "nonnull制約に違反する引数が渡されています。";
            case ErrorCode.RangeViolation: return "__range制約を超える値が検出されました。";
            case ErrorCode.RangeIndexOob: return "__range付きインデックスが配列境界外となる可能性があります。";
            case ErrorCode.SwitchCaseEnumMismatch: return "switch(enum)に対するcaseの列挙型が一致しません。";
            case ErrorCode.SwitchCaseNonEnumOnEnumSwitch: return "enum switchに非enum caseが含まれています。";
            case ErrorCode.EnumStrictMix: return "__enum_strictモードでenumと整数の混在が検出されました。";
            case ErrorCode.BitFlagsOp: return "__bitflags型に不適切な演算が適用されています。";
            case ErrorCode.BitFlagsMix: return "__bitflagsと通常整数の混在が検出されました。";
            case ErrorCode.SafeIndexIndex: return "__safe_indexで安全でない添字式が検出されました。";
            case ErrorCode.SafeIndexMix: return "__safe_index型と通常整数の混在が検出されました。";
            case ErrorCode.RestrictAlias: return "__restrict違反の可能性があるエイリアスが検出されました。";
            case ErrorCode.ConstDiscard: return "ポインタ変換でconst修飾が暗黙に破棄されています。";
            case ErrorCode.SwitchImplicitFallthrough: return "switch caseで暗黙fallthroughが検出されました。";
            case ErrorCode.SwitchFallthroughUsage: return "fallthrough使用位置が不正です。";
            case ErrorCode.UnreachableCode: return "到達不能コードが検出されました。";
            case ErrorCode.UnusedSymbol: return "未使用の変数/関数/シンボルです。";
            case ErrorCode.ImplicitNarrowing: return "暗黙縮小変換の可能性があります。";
            case ErrorCode.PointerArithmeticDanger: return "危険なポインタ演算の可能性があります。";
            case ErrorCode.ExternUndefined: return "extern宣言に対応する定義が見つかりません。";
            case ErrorCode.ExternTypeMismatch: return "extern宣言と定義の型が一致しません。";
            case ErrorCode.ExternNotSupported: return "externの扱いとして未対応なパターンです。";
            case ErrorCode.Internal: return "内部エラーです。";
            case ErrorCode.InternalNYI: return "未実装パスに到達しました。";
            case ErrorCode.InternalUnhandledCase: return "想定外ケースが発生しました。";
            default:
                return "このコードの詳細説明は未登録です。";
        }
    }

    // Choose by modification time beneath the current directory; discovery failures return no candidate.
    static string TryGuessNewestFile(string pattern, bool recursive)
    {
        try
        {
            string root = Environment.CurrentDirectory;
            var files = Directory.GetFiles(root, pattern, recursive ? SearchOption.AllDirectories : SearchOption.TopDirectoryOnly);
            if (files.Length == 0) return "";
            return files
                .Select(f => new FileInfo(f))
                .OrderByDescending(fi => fi.LastWriteTimeUtc)
                .Select(fi => fi.FullName)
                .FirstOrDefault() ?? "";
        }
        catch
        {
            return "";
        }
    }

    // Convert accepted map rows to searchable symbols without inferring their sizes.
    static List<DebugCliSymbol> ParseMapSymbolsForCli(string mapPath)
    {
        var list = new List<DebugCliSymbol>();
        if (string.IsNullOrWhiteSpace(mapPath) || !File.Exists(mapPath)) return list;

        foreach (var line in IoUtil.ReadAllLinesUtf8(mapPath))
        {
            if (!TryParseMapEntry(line, out DebugCliMapEntry entry)) continue;
            list.Add(new DebugCliSymbol
            {
                Name = entry.Name,
                Address = entry.CpuAddress,
                Bank = entry.Bank,
                Size = null,
                Source = "map",
                Kind = entry.Kind,
                Region = entry.Region
            });
        }
        return list;
    }

    // Read only sym records; prefer explicit bank/region fields and retain missing numeric fields as null.
    static List<DebugCliSymbol> ParseDbgSymbolsForCli(string dbgPath)
    {
        var list = new List<DebugCliSymbol>();
        if (string.IsNullOrWhiteSpace(dbgPath) || !File.Exists(dbgPath)) return list;

        foreach (var line in IoUtil.ReadAllLinesUtf8(dbgPath))
        {
            if (string.IsNullOrWhiteSpace(line)) continue;
            string t = line.Trim();
            if (!t.StartsWith("sym\t", StringComparison.Ordinal)) continue;
            var kv = ParseDbgKeyValues(t.Substring(4));

            string name = GetDbgString(kv, "name");
            int? addr = GetDbgInt(kv, "val");
            int? size = GetDbgInt(kv, "size");
            int? bank = GetDbgInt(kv, "wbank") ?? GetDbgInt(kv, "bank") ?? GetDbgInt(kv, "seg");
            string region = GetDbgString(kv, "region");
            if (string.IsNullOrWhiteSpace(name)) continue;
            if (string.IsNullOrWhiteSpace(region) && addr.HasValue) region = InferRegionNameForCli(addr.Value);

            list.Add(new DebugCliSymbol
            {
                Name = name,
                Address = addr,
                Bank = bank,
                Size = size,
                Source = "dbg",
                Kind = "sym",
                Region = region
            });
        }
        return list;
    }

    // Read compiler CSV extents, skipping malformed rows; start/end offsets determine the reported size.
    static List<DebugCliFunctionRange> ParseFunctionRanges(string funcsizesPath)
    {
        var list = new List<DebugCliFunctionRange>();
        if (string.IsNullOrWhiteSpace(funcsizesPath) || !File.Exists(funcsizesPath)) return list;

        foreach (var raw in IoUtil.ReadAllLinesUtf8(funcsizesPath))
        {
            if (string.IsNullOrWhiteSpace(raw)) continue;
            if (raw.StartsWith("#")) continue;
            var parts = raw.Split(',');
            if (parts.Length < 6) continue;

            string name = parts[0].Trim();
            if (name.Length == 0) continue;
            if (!TryParseInt(parts[1].Trim(), out int bank)) bank = 0;
            if (!TryParseInt(parts[2].Trim(), out int cpuAddr)) cpuAddr = 0;
            if (!TryParseInt(parts[3].Trim(), out int startOff)) continue;
            if (!TryParseInt(parts[4].Trim(), out int endOff)) continue;

            list.Add(new DebugCliFunctionRange
            {
                Name = name,
                Bank = bank,
                CpuAddress = cpuAddr,
                StartFileOffset = startOff,
                EndFileOffset = endOff
            });
        }
        return list;
    }

    // Approximate ranges from ROM symbols tagged S using the current 16 KiB bank conversion.
    // Explicit function-size metadata is preferable to this fallback.
    static List<DebugCliFunctionRange> ParseFunctionRangesFromMap(string mapPath, int romSizeBytes)
    {
        var list = new List<DebugCliFunctionRange>();
        if (string.IsNullOrWhiteSpace(mapPath) || !File.Exists(mapPath)) return list;

        var entries = new List<(string Name, int Bank, int Offset, string Kind, int CpuAddr)>();
        foreach (var raw in IoUtil.ReadAllLinesUtf8(mapPath))
        {
            if (!TryParseMapEntry(raw, out DebugCliMapEntry entry)) continue;
            if (!string.Equals(entry.Region, "ROM", StringComparison.OrdinalIgnoreCase)) continue;
            entries.Add((entry.Name, entry.Bank ?? 0, entry.Offset, entry.Kind, entry.CpuAddress));
        }

        var grouped = entries
            .Where(e => string.Equals(e.Kind, "S", StringComparison.OrdinalIgnoreCase))
            .OrderBy(e => e.Bank)
            .ThenBy(e => e.Offset)
            .ToList();

        for (int i = 0; i < grouped.Count; i++)
        {
            var cur = grouped[i];
            int startFile = (cur.Bank * 0x4000) + (cur.Offset & 0x3FFF);
            // Initialize a one-byte extent; a later symbol in the same bank can extend it.
            int endFile = startFile + 1;

            // Skip same-offset aliases and stop at the next strictly greater offset within this bank.
            for (int j = i + 1; j < grouped.Count; j++)
            {
                if (grouped[j].Bank != cur.Bank) break;
                int nextFile = (grouped[j].Bank * 0x4000) + (grouped[j].Offset & 0x3FFF);
                if (nextFile > startFile)
                {
                    endFile = nextFile;
                    break;
                }
            }

            if (endFile <= startFile)
            {
                int bankEnd = (cur.Bank + 1) * 0x4000;
                endFile = bankEnd;
            }
            if (romSizeBytes > 0) endFile = Math.Min(endFile, romSizeBytes);

            list.Add(new DebugCliFunctionRange
            {
                Name = cur.Name,
                Bank = cur.Bank,
                CpuAddress = cur.CpuAddr,
                StartFileOffset = startFile,
                EndFileOffset = endFile
            });
        }

        return list;
    }

    // Parse the whitespace-separated map format with an optional decimal bank and optional region field.
    static bool TryParseMapEntry(string line, out DebugCliMapEntry entry)
    {
        entry = null;
        if (string.IsNullOrWhiteSpace(line)) return false;
        string t = line.Trim();
        if (t.StartsWith(";") || t.StartsWith("#")) return false;

        var parts = t.Split(new[] { ' ', '\t' }, StringSplitOptions.RemoveEmptyEntries);
        if (parts.Length < 5) return false;
        if (!TryParseHex(parts[0], out int addr)) return false;

        int index = 1;
        int? bank = null;
        if (index < parts.Length && int.TryParse(parts[index], out int parsedBank))
        {
            bank = parsedBank;
            index++;
        }

        if (index >= parts.Length || !TryParseHex(parts[index], out int off)) return false;
        index++;
        if (index >= parts.Length) return false;
        string kind = parts[index++];
        if (index >= parts.Length) return false;

        string region;
        string name;
        if (parts.Length - index >= 2)
        {
            region = parts[index++];
            name = parts[index];
        }
        else
        {
            region = InferRegionNameForCli(addr);
            name = parts[index];
        }

        entry = new DebugCliMapEntry
        {
            Name = name,
            CpuAddress = addr,
            Bank = bank,
            Offset = off,
            Kind = kind,
            Region = region
        };
        return true;
    }

    // Fallback to the legacy Game Boy address-region table when the input supplies no region.
    static string InferRegionNameForCli(int address)
    {
        if (address >= 0xFF80 && address <= 0xFFFE) return "HRAM";
        if (address >= 0xFF00 && address <= 0xFF7F) return "IO";
        if (address >= 0xFE00 && address <= 0xFE9F) return "OAM";
        if (address >= 0xD000 && address <= 0xDFFF) return "WRAMX";
        if (address >= 0xC000 && address <= 0xCFFF) return "WRAM0";
        if (address >= 0xA000 && address <= 0xBFFF) return "SRAM";
        if (address >= 0x8000 && address <= 0x9FFF) return "VRAM";
        if (address >= 0x0000 && address <= 0x7FFF) return "ROM";
        return "";
    }

    // Extract hexadecimal file_off annotations while preserving all original lines for context display.
    static List<DebugCliDisLine> ParseDisassemblyLines(string disasmPath)
    {
        var list = new List<DebugCliDisLine>();
        if (string.IsNullOrWhiteSpace(disasmPath) || !File.Exists(disasmPath)) return list;
        var re = new Regex(@";\s*file_off=\$([0-9A-Fa-f]+)", RegexOptions.Compiled);

        foreach (var line in IoUtil.ReadAllLinesUtf8(disasmPath))
        {
            int off = -1;
            var m = re.Match(line ?? "");
            if (m.Success) TryParseHex(m.Groups[1].Value, out off);
            list.Add(new DebugCliDisLine { Text = line ?? "", FileOffset = off });
        }

        return list;
    }

    // Use a declaration regex and balanced-brace scan without preprocessing or full C parsing.
    // Only the brace scan skips comments and quoted literals; regex matches remain heuristic.
    static List<DebugCliSourceFunction> ParseSourceFunctions(string sourcePath)
    {
        var list = new List<DebugCliSourceFunction>();
        if (string.IsNullOrWhiteSpace(sourcePath) || !File.Exists(sourcePath)) return list;

        string text = IoUtil.ReadAllTextUtf8(sourcePath);
        var lineStarts = BuildLineStarts(text);
        var re = new Regex(@"(?m)^[ \t]*(?:[A-Za-z_][A-Za-z0-9_\s\*\[\]]*?)\b([A-Za-z_][A-Za-z0-9_]*)\s*\([^;{}]*\)\s*\{",
            RegexOptions.Compiled);

        foreach (Match m in re.Matches(text))
        {
            if (!m.Success) continue;
            string name = m.Groups[1].Value;
            if (string.IsNullOrEmpty(name)) continue;
            if (SourceControlKeywords.Contains(name)) continue;

            int bracePos = m.Index + m.Length - 1;
            int endBrace = FindMatchingBrace(text, bracePos);
            if (endBrace < 0) continue;

            int startLine = PositionToLine(lineStarts, m.Index);
            int endLine = PositionToLine(lineStarts, endBrace);
            list.Add(new DebugCliSourceFunction
            {
                Name = name,
                StartLine = startLine,
                EndLine = endLine
            });
        }

        return list
            .OrderBy(f => f.StartLine)
            .ThenBy(f => f.Name, StringComparer.Ordinal)
            .ToList();
    }

    // Track brace depth outside comments, strings and character literals; skip escaped quoted characters.
    static int FindMatchingBrace(string text, int openBracePos)
    {
        if (string.IsNullOrEmpty(text) || openBracePos < 0 || openBracePos >= text.Length) return -1;
        if (text[openBracePos] != '{') return -1;

        int depth = 0;
        bool inLineComment = false;
        bool inBlockComment = false;
        bool inString = false;
        bool inChar = false;

        for (int i = openBracePos; i < text.Length; i++)
        {
            char c = text[i];
            char n = (i + 1 < text.Length) ? text[i + 1] : '\0';

            if (inLineComment)
            {
                if (c == '\n') inLineComment = false;
                continue;
            }
            if (inBlockComment)
            {
                if (c == '*' && n == '/') { inBlockComment = false; i++; }
                continue;
            }
            if (inString)
            {
                if (c == '\\') { i++; continue; }
                if (c == '"') inString = false;
                continue;
            }
            if (inChar)
            {
                if (c == '\\') { i++; continue; }
                if (c == '\'') inChar = false;
                continue;
            }

            if (c == '/' && n == '/') { inLineComment = true; i++; continue; }
            if (c == '/' && n == '*') { inBlockComment = true; i++; continue; }
            if (c == '"') { inString = true; continue; }
            if (c == '\'') { inChar = true; continue; }

            if (c == '{') depth++;
            else if (c == '}')
            {
                depth--;
                if (depth == 0) return i;
            }
        }
        return -1;
    }

    // Index offsets after each newline so character positions can be converted without rescanning the source.
    static List<int> BuildLineStarts(string text)
    {
        var starts = new List<int>();
        starts.Add(0);
        if (string.IsNullOrEmpty(text)) return starts;
        for (int i = 0; i < text.Length; i++)
        {
            if (text[i] == '\n')
                starts.Add(i + 1);
        }
        return starts;
    }

    // Binary-search the last line start at or before the position and return a one-based line number.
    static int PositionToLine(List<int> lineStarts, int pos)
    {
        if (lineStarts == null || lineStarts.Count == 0) return 1;
        int lo = 0;
        int hi = lineStarts.Count - 1;
        while (lo <= hi)
        {
            int mid = lo + ((hi - lo) / 2);
            if (lineStarts[mid] <= pos) lo = mid + 1;
            else hi = mid - 1;
        }
        return Math.Max(1, hi + 1);
    }

    // Split at the final colon so Windows drive letters remain part of the path; require a positive line.
    static bool TryParseSourceLineSpec(string spec, out string sourcePath, out int line1Based)
    {
        sourcePath = "";
        line1Based = 0;
        if (string.IsNullOrWhiteSpace(spec)) return false;

        int idx = spec.LastIndexOf(':');
        if (idx <= 0 || idx >= spec.Length - 1) return false;

        string pathPart = spec.Substring(0, idx);
        string linePart = spec.Substring(idx + 1);
        if (!int.TryParse(linePart, out line1Based) || line1Based <= 0) return false;

        sourcePath = Path.GetFullPath(pathPart);
        return true;
    }

    // Parse a hexadecimal field with an optional 0x prefix; callers remove other format-specific markers.
    static bool TryParseHex(string text, out int value)
    {
        value = 0;
        if (string.IsNullOrWhiteSpace(text)) return false;
        string s = text.Trim();
        if (s.StartsWith("0x", StringComparison.OrdinalIgnoreCase))
            s = s.Substring(2);
        return int.TryParse(s, System.Globalization.NumberStyles.HexNumber, null, out value);
    }

    // Parse decimal integers or hexadecimal values prefixed with 0x or $.
    static bool TryParseInt(string text, out int value)
    {
        value = 0;
        if (string.IsNullOrWhiteSpace(text)) return false;
        string s = text.Trim();
        if (s.StartsWith("0x", StringComparison.OrdinalIgnoreCase))
            return int.TryParse(s.Substring(2), System.Globalization.NumberStyles.HexNumber, null, out value);
        if (s.StartsWith("$", StringComparison.Ordinal))
            return int.TryParse(s.Substring(1), System.Globalization.NumberStyles.HexNumber, null, out value);
        return int.TryParse(s, out value);
    }

    // Distinguish missing or malformed numeric fields from a legitimate zero.
    static int? GetDbgInt(Dictionary<string, string> kv, string key)
    {
        if (!kv.TryGetValue(key, out string raw)) return null;
        if (TryParseInt(raw, out int n)) return n;
        return null;
    }

    // Remove surrounding quotes from a stored debug value and decode escaped quotation marks.
    static string GetDbgString(Dictionary<string, string> kv, string key)
    {
        if (!kv.TryGetValue(key, out string raw)) return "";
        if (raw == null) return "";
        string t = raw.Trim();
        if (t.Length >= 2 && t[0] == '"' && t[t.Length - 1] == '"')
            t = t.Substring(1, t.Length - 2);
        return t.Replace("\\\"", "\"");
    }

    // Split comma-delimited fields while allowing commas in quoted values; later duplicate keys replace earlier ones.
    static Dictionary<string, string> ParseDbgKeyValues(string payload)
    {
        var map = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        if (string.IsNullOrWhiteSpace(payload)) return map;

        int i = 0;
        while (i < payload.Length)
        {
            while (i < payload.Length && (payload[i] == ' ' || payload[i] == ',')) i++;
            if (i >= payload.Length) break;

            int eq = payload.IndexOf('=', i);
            if (eq < 0) break;

            string key = payload.Substring(i, eq - i).Trim();
            i = eq + 1;

            string value;
            if (i < payload.Length && payload[i] == '"')
            {
                int j = i + 1;
                // Within quotes, a backslash quotes the following character rather than preserving the slash itself.
                bool esc = false;
                var sb = new StringBuilder();
                while (j < payload.Length)
                {
                    char c = payload[j++];
                    if (esc) { sb.Append(c); esc = false; continue; }
                    if (c == '\\') { esc = true; continue; }
                    if (c == '"') break;
                    sb.Append(c);
                }
                value = "\"" + sb.ToString() + "\"";
                i = j;
            }
            else
            {
                int comma = payload.IndexOf(',', i);
                if (comma < 0)
                {
                    value = payload.Substring(i).Trim();
                    i = payload.Length;
                }
                else
                {
                    value = payload.Substring(i, comma - i).Trim();
                    i = comma + 1;
                }
            }

            if (key.Length > 0) map[key] = value;
        }
        return map;
    }

    // Clamp both slices to their ROMs, compare common relative positions, and count every unmatched tail byte.
    static int CountChangedBytes(byte[] oldRom, int oldStart, int oldEnd, byte[] newRom, int newStart, int newEnd)
    {
        if (oldRom == null) oldRom = Array.Empty<byte>();
        if (newRom == null) newRom = Array.Empty<byte>();
        oldStart = Math.Max(0, Math.Min(oldStart, oldRom.Length));
        oldEnd = Math.Max(oldStart, Math.Min(oldEnd, oldRom.Length));
        newStart = Math.Max(0, Math.Min(newStart, newRom.Length));
        newEnd = Math.Max(newStart, Math.Min(newEnd, newRom.Length));

        int oldLen = oldEnd - oldStart;
        int newLen = newEnd - newStart;
        int common = Math.Min(oldLen, newLen);
        int diff = Math.Abs(oldLen - newLen);
        for (int i = 0; i < common; i++)
        {
            if (oldRom[oldStart + i] != newRom[newStart + i]) diff++;
        }
        return diff;
    }

    // Hash the clamped byte interval with SHA-256; an empty interval is represented by an empty string.
    static string ComputeRangeHash(byte[] rom, int start, int end)
    {
        if (rom == null) rom = Array.Empty<byte>();
        start = Math.Max(0, Math.Min(start, rom.Length));
        end = Math.Max(start, Math.Min(end, rom.Length));
        int len = end - start;
        if (len <= 0) return "";

        byte[] slice = new byte[len];
        Buffer.BlockCopy(rom, start, slice, 0, len);
        using (var sha = SHA256.Create())
        {
            var hash = sha.ComputeHash(slice);
            return string.Concat(hash.Select(b => b.ToString("x2")));
        }
    }

    // Display at most eight hexadecimal characters; the underlying range hash remains full length.
    static string ShortHash(string hex)
    {
        if (string.IsNullOrEmpty(hex)) return "";
        return hex.Length <= 8 ? hex : hex.Substring(0, 8);
    }

    // Use URI-relative conversion when possible; return the original path if conversion is unavailable.
    static string MakeRelativePathSafe(string baseDir, string path)
    {
        try
        {
            if (string.IsNullOrWhiteSpace(baseDir) || string.IsNullOrWhiteSpace(path)) return "";
            Uri baseUri = new Uri(AppendDirectorySeparator(baseDir));
            Uri fileUri = new Uri(path);
            if (!string.Equals(baseUri.Scheme, fileUri.Scheme, StringComparison.OrdinalIgnoreCase))
                return path;
            string rel = Uri.UnescapeDataString(baseUri.MakeRelativeUri(fileUri).ToString());
            return rel.Replace('/', Path.DirectorySeparatorChar);
        }
        catch
        {
            return path;
        }
    }

    // Mark the base URI as a directory without adding a second trailing separator.
    static string AppendDirectorySeparator(string path)
    {
        if (string.IsNullOrEmpty(path)) return path;
        if (path[path.Length - 1] == Path.DirectorySeparatorChar || path[path.Length - 1] == Path.AltDirectorySeparatorChar)
            return path;
        return path + Path.DirectorySeparatorChar;
    }

    // Normalize separators, remove leading parent components, and reduce rooted paths to a filename.
    // This is a copy-path formatting helper, not a general containment validator.
    static string SanitizeRelativePath(string rel)
    {
        if (string.IsNullOrWhiteSpace(rel)) return "";
        string t = rel.Replace('/', Path.DirectorySeparatorChar).Replace('\\', Path.DirectorySeparatorChar);
        while (t.StartsWith(".." + Path.DirectorySeparatorChar, StringComparison.Ordinal))
            t = t.Substring(3);
        if (Path.IsPathRooted(t)) t = Path.GetFileName(t);
        return t;
    }
}




