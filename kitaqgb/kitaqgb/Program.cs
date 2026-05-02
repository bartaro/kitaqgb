using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

// Global compile-time configuration options.
public static class Config
{
    public static readonly bool ShowSourcePositionInDebugOutput = true;
}

static partial class Program
{
    public static bool EnableDebugOutput { get { return CurrentSession.EnableDebugOutput; } private set { CurrentSession.EnableDebugOutput = value; } }
    public static bool DisableDisasm { get { return CurrentSession.DisableDisasm; } private set { CurrentSession.DisableDisasm = value; } }
    public static bool EnableIncrementalCache { get { return CurrentSession.EnableIncrementalCache; } private set { CurrentSession.EnableIncrementalCache = value; } }
    public static bool EmitDiagJson { get { return CurrentSession.EmitDiagJson; } private set { CurrentSession.EmitDiagJson = value; } }
    public static string DiagJsonPath { get { return CurrentSession.DiagJsonPath; } private set { CurrentSession.DiagJsonPath = value; } }
    public static bool EmitDependenciesList { get { return CurrentSession.EmitDependenciesList; } private set { CurrentSession.EmitDependenciesList = value; } }
    public static string DependenciesListPath { get { return CurrentSession.DependenciesListPath; } private set { CurrentSession.DependenciesListPath = value; } }
    public enum DiagnosticMode { Permissive, Strict }
    public static DiagnosticMode DiagnosticsMode { get { return CurrentSession.DiagnosticsMode; } private set { CurrentSession.DiagnosticsMode = value; } }
    public static bool StrictDiagnostics => DiagnosticsMode == DiagnosticMode.Strict;
    public static List<string> IncludeDirectories => CurrentSession.IncludeDirectories;
    public static IReadOnlyList<string> LastCompilationDependencies => CurrentSession.LastCompilationDependencies;
    static List<string> _lastCompilationDependencies => CurrentSession.LastCompilationDependencies;
    static string _cacheKey { get { return CurrentSession.CacheKey; } set { CurrentSession.CacheKey = value; } }
    static string[] _originalArgs { get { return CurrentSession.OriginalArgs; } set { CurrentSession.OriginalArgs = value; } }
    static bool _autoMinimizeOnFail { get { return CurrentSession.AutoMinimizeOnFail; } set { CurrentSession.AutoMinimizeOnFail = value; } }
    static bool _autoMinimizeTriggered { get { return CurrentSession.AutoMinimizeTriggered; } set { CurrentSession.AutoMinimizeTriggered = value; } }
    static bool _reproPackageWritten { get { return CurrentSession.ReproPackageWritten; } set { CurrentSession.ReproPackageWritten = value; } }
    static string _outputFilenameForDiag { get { return CurrentSession.OutputFilenameForDiag; } set { CurrentSession.OutputFilenameForDiag = value; } }
    public static bool RstUse38 { get { return CurrentSession.RstUse38; } private set { CurrentSession.RstUse38 = value; } }
    // RST hot-call compression is OFF by default.
    // Enable explicitly with --rst-enable / --rst-on.
    public static bool RstDisable { get { return CurrentSession.RstDisable; } private set { CurrentSession.RstDisable = value; } }
    public static bool ConstScalarInRom { get { return CurrentSession.ConstScalarInRom; } private set { CurrentSession.ConstScalarInRom = value; } }
    // RST hot-call compression tuning
    public static int RstMaxCalls { get { return CurrentSession.RstMaxCalls; } private set { CurrentSession.RstMaxCalls = value; } }
    // 0 means "use all available vectors" (7 or 8 depending on --rst-use-38).
    public static int RstMaxVectors { get { return CurrentSession.RstMaxVectors; } private set { CurrentSession.RstMaxVectors = value; } }
    public static System.Collections.Generic.HashSet<string> RstExcludeLabels => CurrentSession.RstExcludeLabels;
    public static bool RstSpeedSafe { get { return CurrentSession.RstSpeedSafe; } private set { CurrentSession.RstSpeedSafe = value; } }
    // Conservative mode is ON by default. It avoids RST mapping for timing/IO-sensitive routines.
    // Use --rst-unsafe to force aggressive historical behavior.
    public static bool RstUnsafe { get { return CurrentSession.RstUnsafe; } private set { CurrentSession.RstUnsafe = value; } }
    private static bool AttachDebuggerOnError { get { return CurrentSession.AttachDebuggerOnError; } set { CurrentSession.AttachDebuggerOnError = value; } }
    static List<DiagnosticEntry> Diagnostics => CurrentSession.Diagnostics;
    static bool SuppressConsoleDiagnostics { get { return CurrentSession.SuppressConsoleDiagnostics; } set { CurrentSession.SuppressConsoleDiagnostics = value; } }

    public struct DiagnosticSnapshot
    {
        public int ErrorCount;
        public int WarningCount;
        public int DiagnosticCount;
    }

    // --- Multiple-error collection ---
    // We keep compiling until we hit MaxErrors, then abort.
    public static int ErrorCount { get { return CurrentSession.ErrorCount; } private set { CurrentSession.ErrorCount = value; } }
    public static int WarningCount { get { return CurrentSession.WarningCount; } private set { CurrentSession.WarningCount = value; } }
    public static int MaxErrors { get { return CurrentSession.MaxErrors; } private set { CurrentSession.MaxErrors = value; } }

    // Cache of source files for richer diagnostics.
    // Key: normalized filename as passed to the compiler.
    static Dictionary<string, string[]> SourceCache => CurrentSession.SourceCache;

    public static string DebugOutputPath { get { return CurrentSession.DebugOutputPath; } set { CurrentSession.DebugOutputPath = value; } }

    // --- Trace (compile pipeline dump) ---
    // --trace[=tokens,ast,ir,asm] dumps intermediate forms to TraceOutputPath.
    public static bool TraceEnabled { get { return CurrentSession.TraceEnabled; } private set { CurrentSession.TraceEnabled = value; } }
    public static string TraceOutputPath { get { return CurrentSession.TraceOutputPath; } private set { CurrentSession.TraceOutputPath = value; } }
    static HashSet<string> TraceStages => CurrentSession.TraceStages;
    // Optional: disassemble only functions changed against git base.
    static bool EmitChangedFunctionDisasm { get { return CurrentSession.EmitChangedFunctionDisasm; } set { CurrentSession.EmitChangedFunctionDisasm = value; } }
    static string ChangedFunctionDisasmBaseRef { get { return CurrentSession.ChangedFunctionDisasmBaseRef; } set { CurrentSession.ChangedFunctionDisasmBaseRef = value; } }
    static string ChangedFunctionDisasmOutPath { get { return CurrentSession.ChangedFunctionDisasmOutPath; } set { CurrentSession.ChangedFunctionDisasmOutPath = value; } }
    static Dictionary<string, int> FunctionBankOverrides => CurrentSession.FunctionBankOverrides;

    // --- Variable list output (vlist) ---
    // vlist / --vlist emits a variable table after assembly.
    public static bool EmitVarList { get { return CurrentSession.EmitVarList; } private set { CurrentSession.EmitVarList = value; } }
    public static string VarListOutputPath { get { return CurrentSession.VarListOutputPath; } private set { CurrentSession.VarListOutputPath = value; } } // if empty, use <out>.vlist.txt

    // --- Lightweight optimization level ---
    // -O0: off (baseline)
    // -O1: cheap but effective passes (peepholes, constant-control-flow pruning, etc.)
    public static int OptLevel { get { return CurrentSession.OptLevel; } private set { CurrentSession.OptLevel = value; } }

    // --- Debug-only safety checks (zero-cost in release builds) ---
    // -Zcheck : enable all safety checks
    // -Zcheck-bounds : enable only fixed-array bounds checks for a[i]
    public static bool CheckBounds { get { return CurrentSession.CheckBounds; } private set { CurrentSession.CheckBounds = value; } }
    public static bool CheckMemCopy { get { return CurrentSession.CheckMemCopy; } private set { CurrentSession.CheckMemCopy = value; } }
    public static bool CheckStack { get { return CurrentSession.CheckStack; } private set { CurrentSession.CheckStack = value; } }
    public static bool CheckBankCalls { get { return CurrentSession.CheckBankCalls; } private set { CurrentSession.CheckBankCalls = value; } }
    // Slice bounds checks are enabled only by -Zcheck (not by -Zcheck-bounds).
    public static bool CheckSliceBounds { get { return CurrentSession.CheckSliceBounds; } private set { CurrentSession.CheckSliceBounds = value; } }
    public enum StackBankMode { Fixed, WramX1 }
    public static StackBankMode StackBank { get { return CurrentSession.StackBank; } private set { CurrentSession.StackBank = value; } }
    public static int? StackTop { get { return CurrentSession.StackTop; } private set { CurrentSession.StackTop = value; } }
    public static int StackReserve { get { return CurrentSession.StackReserve; } private set { CurrentSession.StackReserve = value; } }
    public static int EffectiveStackTop { get { return CurrentSession.EffectiveStackTop; } private set { CurrentSession.EffectiveStackTop = value; } }
    public static int EffectiveStackAutoLimit { get { return CurrentSession.EffectiveStackAutoLimit; } private set { CurrentSession.EffectiveStackAutoLimit = value; } }

    // --- Calling convention / ABI ---
    public enum AbiMode { Legacy, Stack }
    public static AbiMode Abi { get { return CurrentSession.Abi; } private set { CurrentSession.Abi = value; } }
    public static bool AbiStack => Abi == AbiMode.Stack;

    // --- Analysis / report options ---
    public static bool EmitBankSimReport { get { return CurrentSession.EmitBankSimReport; } private set { CurrentSession.EmitBankSimReport = value; } }
    public static string BankSimReportPath { get { return CurrentSession.BankSimReportPath; } private set { CurrentSession.BankSimReportPath = value; } }
    public static bool EmitFarcallSuggestionReport { get { return CurrentSession.EmitFarcallSuggestionReport; } private set { CurrentSession.EmitFarcallSuggestionReport = value; } }
    public static string FarcallSuggestionReportPath { get { return CurrentSession.FarcallSuggestionReportPath; } private set { CurrentSession.FarcallSuggestionReportPath = value; } }
    public static bool EmitCrossBankCallReport { get { return CurrentSession.EmitCrossBankCallReport; } private set { CurrentSession.EmitCrossBankCallReport = value; } }
    public static string CrossBankCallReportPath { get { return CurrentSession.CrossBankCallReportPath; } private set { CurrentSession.CrossBankCallReportPath = value; } }
    public static bool EmitAbiVerifyReport { get { return CurrentSession.EmitAbiVerifyReport; } private set { CurrentSession.EmitAbiVerifyReport = value; } }
    public static string AbiVerifyReportPath { get { return CurrentSession.AbiVerifyReportPath; } private set { CurrentSession.AbiVerifyReportPath = value; } }
    public static bool EmitAbiDiffReport { get { return CurrentSession.EmitAbiDiffReport; } private set { CurrentSession.EmitAbiDiffReport = value; } }
    public static string AbiDiffReportPath { get { return CurrentSession.AbiDiffReportPath; } private set { CurrentSession.AbiDiffReportPath = value; } }
    public static bool EmitRstApplyReport { get { return CurrentSession.EmitRstApplyReport; } private set { CurrentSession.EmitRstApplyReport = value; } }
    public static string RstApplyReportPath { get { return CurrentSession.RstApplyReportPath; } private set { CurrentSession.RstApplyReportPath = value; } }
    public static bool EmitOptDiffReport { get { return CurrentSession.EmitOptDiffReport; } private set { CurrentSession.EmitOptDiffReport = value; } }
    public static string OptDiffReportPath { get { return CurrentSession.OptDiffReportPath; } private set { CurrentSession.OptDiffReportPath = value; } }
    public static bool EmitFunctionSizeReport { get { return CurrentSession.EmitFunctionSizeReport; } private set { CurrentSession.EmitFunctionSizeReport = value; } }
    public static string FunctionSizeReportPath { get { return CurrentSession.FunctionSizeReportPath; } private set { CurrentSession.FunctionSizeReportPath = value; } }
    public static bool EmitHotspotReport { get { return CurrentSession.EmitHotspotReport; } private set { CurrentSession.EmitHotspotReport = value; } }
    public static string HotspotReportPath { get { return CurrentSession.HotspotReportPath; } private set { CurrentSession.HotspotReportPath = value; } }
    public static bool EnableReproCheck { get { return CurrentSession.EnableReproCheck; } private set { CurrentSession.EnableReproCheck = value; } }
    public static string ReproCheckReportPath { get { return CurrentSession.ReproCheckReportPath; } private set { CurrentSession.ReproCheckReportPath = value; } }
    public static bool EmitCgbConsistencyReport { get { return CurrentSession.EmitCgbConsistencyReport; } private set { CurrentSession.EmitCgbConsistencyReport = value; } }
    public static string CgbConsistencyReportPath { get { return CurrentSession.CgbConsistencyReportPath; } private set { CurrentSession.CgbConsistencyReportPath = value; } }
    public static bool EmitCgbSymbolVerifyReport { get { return CurrentSession.EmitCgbSymbolVerifyReport; } private set { CurrentSession.EmitCgbSymbolVerifyReport = value; } }
    public static string CgbSymbolVerifyReportPath { get { return CurrentSession.CgbSymbolVerifyReportPath; } private set { CurrentSession.CgbSymbolVerifyReportPath = value; } }
    // Build-failure repro package (--repro-pack[=<dir>])
    static bool EmitReproPackageOnFail { get { return CurrentSession.EmitReproPackageOnFail; } set { CurrentSession.EmitReproPackageOnFail = value; } }
    static string ReproPackagePath { get { return CurrentSession.ReproPackagePath; } set { CurrentSession.ReproPackagePath = value; } }

    // --- ROM header patching options (fix31_rom_header_patch) ---
    // Priority: CLI (incl. --rom-header JSON template) > #pragma rom_* > defaults.
    static RomHeaderOptions RomHeader => CurrentSession.RomHeader; // CLI/JSON-specified
    static RomHeaderOptions RomHeaderPragma => CurrentSession.RomHeaderPragma; // #pragma rom_* specified
    static List<CgbPaletteDefinition> CgbPalettePragmas => CurrentSession.CgbPalettePragmas;

    static RomHeaderOptions BuildRequestedRomHeader()
    {
        var effective = new RomHeaderOptions();
        effective.MergeFrom(RomHeaderPragma, overwrite: true);
        effective.MergeFrom(RomHeader, overwrite: true);
        return effective;
    }

    public static bool TryGetKnownCgbRuntimeValue(out int value)
    {
        value = 0;

        var effective = BuildRequestedRomHeader();
        if (!effective.CgbFlag.HasValue) return false;

        if (effective.CgbFlag.Value == 0xC0)
        {
            value = 1;
            return true;
        }

        if (effective.CgbFlag.Value == 0x00)
        {
            value = 0;
            return true;
        }

        return false;
    }

    public static bool IsCgbOnlyTargetRequested()
    {
        return TryGetKnownCgbRuntimeValue(out int value) && value != 0;
    }

    static bool TryParseStackBankMode(string text, out StackBankMode mode)
    {
        mode = StackBankMode.WramX1;
        string normalized = (text ?? "").Trim().ToLowerInvariant();
        if (normalized == "fixed")
        {
            mode = StackBankMode.Fixed;
            return true;
        }

        if (normalized == "wramx1")
        {
            mode = StackBankMode.WramX1;
            return true;
        }

        return false;
    }

    static int GetStackWindowBottom(StackBankMode mode)
    {
        return mode == StackBankMode.Fixed ? 0xC000 : 0xD000;
    }

    static int GetStackWindowTop(StackBankMode mode)
    {
        return mode == StackBankMode.Fixed ? 0xCFFF : 0xDFFF;
    }

    static string GetStackBankCliText(StackBankMode mode)
    {
        return mode == StackBankMode.Fixed ? "fixed" : "wramx1";
    }

    static string GetStackWindowName(StackBankMode mode)
    {
        return mode == StackBankMode.Fixed ? "WRAM0" : "WRAMX bank1";
    }

    static string FormatAddress16(int value)
    {
        return string.Format("${0:X4}", value & 0xFFFF);
    }

    static bool HasReservedStackRange()
    {
        return StackReserve > 0;
    }

    static int GetReservedStackRangeBegin()
    {
        return EffectiveStackAutoLimit + 1;
    }

    static int GetReservedStackRangeEnd()
    {
        return EffectiveStackTop;
    }

    static string GetReservedStackRangeText()
    {
        if (!HasReservedStackRange()) return "none";
        return FormatAddress16(GetReservedStackRangeBegin()) + "-" + FormatAddress16(GetReservedStackRangeEnd());
    }

    static string BuildStackPolicyText()
    {
        var sb = new StringBuilder();
        sb.AppendLine("stack.bank = " + GetStackBankCliText(StackBank));
        sb.AppendLine("stack.top = " + FormatAddress16(EffectiveStackTop));
        sb.AppendLine("stack.reserve = " + StackReserve);
        sb.AppendLine("stack.auto_limit = " + FormatAddress16(EffectiveStackAutoLimit));
        sb.AppendLine("stack.reserved_range = " + GetReservedStackRangeText());
        return sb.ToString();
    }

    static void AppendStackPolicyJson(StringBuilder sb)
    {
        sb.Append("\"stack_policy\":{");
        sb.Append("\"bank\":\"").Append(JsonEscape(GetStackBankCliText(StackBank))).Append("\",");
        sb.Append("\"top\":").Append(EffectiveStackTop).Append(",");
        sb.Append("\"reserve\":").Append(StackReserve).Append(",");
        sb.Append("\"auto_limit\":").Append(EffectiveStackAutoLimit).Append(",");
        sb.Append("\"reserved_begin\":").Append(GetReservedStackRangeBegin()).Append(",");
        sb.Append("\"reserved_end\":").Append(GetReservedStackRangeEnd());
        sb.Append("}");
    }

    static void FinalizeAndValidateStackPolicy()
    {
        if (!StackTop.HasValue)
            EffectiveStackTop = GetStackWindowTop(StackBank);
        else
            EffectiveStackTop = StackTop.Value;

        int windowBottom = GetStackWindowBottom(StackBank);
        int windowTop = GetStackWindowTop(StackBank);

        if (EffectiveStackTop < 0 || EffectiveStackTop > 0xFFFF)
        {
            Error(ErrorCode.InvalidStackTop,
                "--stack-top: expected a 16-bit address (got {0})",
                EffectiveStackTop);
            EffectiveStackTop = GetStackWindowTop(StackBank);
        }

        if (EffectiveStackTop < windowBottom || EffectiveStackTop > windowTop)
        {
            Error(ErrorCode.StackTopBankMismatch,
                "--stack-top: address must be within {0} range {1}-{2} for --stack-bank={3} (got {4})",
                GetStackWindowName(StackBank),
                string.Format("{0:X4}", windowBottom),
                string.Format("{0:X4}", windowTop),
                GetStackBankCliText(StackBank),
                string.Format("{0:X4}", EffectiveStackTop & 0xFFFF));
            EffectiveStackTop = Math.Max(windowBottom, Math.Min(EffectiveStackTop, windowTop));
        }

        if (StackReserve < 0)
        {
            Error(ErrorCode.InvalidStackReserve,
                "--stack-reserve: value must be a non-negative integer (got {0})",
                StackReserve);
            StackReserve = 0;
        }

        int maxReserve = EffectiveStackTop - windowBottom;
        if (StackReserve > maxReserve)
        {
            Error(ErrorCode.InvalidStackReserve,
                "--stack-reserve: value {0} exceeds usable depth for stack top {1} in --stack-bank={2}",
                StackReserve,
                string.Format("{0:X4}", EffectiveStackTop & 0xFFFF),
                GetStackBankCliText(StackBank));
            StackReserve = Math.Max(0, maxReserve);
        }

        EffectiveStackAutoLimit = EffectiveStackTop - StackReserve;
    }

    internal sealed class CgbPaletteDefinition
    {
        public string Name;
        public int[] Colors;
        public FilePosition Position;
    }

    internal sealed class TraceStageStat
    {
        public string Name;
        public long ElapsedMs;
        public string Detail;
    }

    static void Main(string[] argsArray)
    {
        PrepareForInvocation(argsArray);

        if (TryRunKqHelpCommand(argsArray)) return;
        if (TryRunSymbolFindCommand(argsArray)) return;
        if (TryRunSourceToAsmCommand(argsArray)) return;
        if (TryRunRomDiffCommand(argsArray)) return;
        if (TryRunVibeTemplateCommand(argsArray)) return;
        if (TryRunFixHintCommand(argsArray)) return;
        if (TryRunAiIrSummaryCommand(argsArray)) return;
        if (TryRunConventionsCommand(argsArray)) return;
        if (TryRunSnippetLibraryCommand(argsArray)) return;
        if (TryRunDevServerCommand(argsArray)) return;
        if (TryRunRecipeCommand(argsArray)) return;

        if (TryRunAttrVizCommand(argsArray)) return;

        if (TryRunTestCommand(argsArray)) return;
        if (HasWatchFlag(argsArray))
        {
            RunWatchDriver(argsArray);
            return;
        }

        // --minimize: delta-debugging helper to auto-generate a minimal reproducer.
        // Runs as a driver (spawns child compiler processes) and exits.
        if (Minimizer.IsMinimizeRequested(argsArray))
        {
            Minimizer.Run(argsArray);
            return;
        }

        Queue<string> args = new Queue<string>(argsArray);

        List<string> sourceFilenames = new List<string>();
        string outputFilename = "out.gb";
        bool help = (args.Count == 0);

        while (args.Count > 0)
        {
            string arg = args.Dequeue();
            if (arg == "-?" || arg == "-h" || arg == "--help")
            {
                help = true;
            }
            else if (arg == "--no-disasm")
            {
                DisableDisasm = true;
            }
            else if (arg == "--disasm")
            {
                DisableDisasm = false;
            }
            else if (arg == "--fast-build" || arg == "--fast")
            {
                DisableDisasm = true;
                TraceEnabled = false;
            }
            else if (arg == "--cache")
            {
                EnableIncrementalCache = true;
            }
            else if (arg == "--no-cache")
            {
                EnableIncrementalCache = false;
            }
            else if (arg == "--diag-json")
            {
                EmitDiagJson = true;
                DiagJsonPath = "kitaqgb.diag.json";
            }
            else if (arg == "--machine-readable")
            {
                MachineReadableOutput = true;
            }
            else if (arg == "--no-banner")
            {
                NoBanner = true;
            }
            else if (arg == "--stack-bank")
            {
                if (args.Count == 0)
                {
                    Error("error: --stack-bank requires fixed or wramx1");
                }
                else if (!TryParseStackBankMode(args.Dequeue(), out StackBankMode stackBank))
                {
                    Error("error: --stack-bank must be fixed or wramx1");
                }
                else
                {
                    StackBank = stackBank;
                }
            }
            else if (arg.StartsWith("--stack-bank="))
            {
                if (!TryParseStackBankMode(ValueAfterEquals(arg), out StackBankMode stackBank))
                {
                    Error("error: --stack-bank must be fixed or wramx1");
                }
                else
                {
                    StackBank = stackBank;
                }
            }
            else if (arg == "--stack-top")
            {
                if (args.Count == 0)
                {
                    Error(ErrorCode.InvalidStackTop, "--stack-top: expected an address literal");
                }
                else
                {
                    string raw = args.Dequeue();
                    if (!TryParseInt(raw, out int parsedTop) || parsedTop < 0 || parsedTop > 0xFFFF)
                        Error(ErrorCode.InvalidStackTop, "--stack-top: expected a 16-bit address literal (got {0})", raw);
                    else
                        StackTop = parsedTop;
                }
            }
            else if (arg.StartsWith("--stack-top="))
            {
                string raw = ValueAfterEquals(arg);
                if (!TryParseInt(raw, out int parsedTop) || parsedTop < 0 || parsedTop > 0xFFFF)
                    Error(ErrorCode.InvalidStackTop, "--stack-top: expected a 16-bit address literal (got {0})", raw);
                else
                    StackTop = parsedTop;
            }
            else if (arg == "--stack-reserve")
            {
                if (args.Count == 0)
                {
                    Error(ErrorCode.InvalidStackReserve, "--stack-reserve: expected a non-negative integer");
                }
                else
                {
                    string raw = args.Dequeue();
                    if (!TryParseInt(raw, out int parsedReserve) || parsedReserve < 0)
                        Error(ErrorCode.InvalidStackReserve, "--stack-reserve: expected a non-negative integer (got {0})", raw);
                    else
                        StackReserve = parsedReserve;
                }
            }
            else if (arg.StartsWith("--stack-reserve="))
            {
                string raw = ValueAfterEquals(arg);
                if (!TryParseInt(raw, out int parsedReserve) || parsedReserve < 0)
                    Error(ErrorCode.InvalidStackReserve, "--stack-reserve: expected a non-negative integer (got {0})", raw);
                else
                    StackReserve = parsedReserve;
            }
            else if (arg == "--strict")
            {
                DiagnosticsMode = DiagnosticMode.Strict;
            }
            else if (arg == "--permissive")
            {
                DiagnosticsMode = DiagnosticMode.Permissive;
            }
            else if (arg.StartsWith("--diag-json="))
            {
                EmitDiagJson = true;
                DiagJsonPath = ValueAfterEquals(arg);
                if (string.IsNullOrWhiteSpace(DiagJsonPath)) DiagJsonPath = "kitaqgb.diag.json";
            }
            else if (arg == "--emit-path-manifest")
            {
                EmitPathManifest = true;
                if (args.Count > 0) PathManifestPath = args.Dequeue();
                else Error("error: --emit-path-manifest requires a file path");
            }
            else if (arg.StartsWith("--emit-path-manifest="))
            {
                EmitPathManifest = true;
                PathManifestPath = ValueAfterEquals(arg);
                if (string.IsNullOrWhiteSpace(PathManifestPath)) Error("error: --emit-path-manifest requires a file path");
            }
            else if (arg == "--deps-out")
            {
                EmitDependenciesList = true;
                DependenciesListPath = "";
            }
            else if (arg.StartsWith("--deps-out="))
            {
                EmitDependenciesList = true;
                DependenciesListPath = ValueAfterEquals(arg);
            }
            else if (arg == "--minimize-on-fail" || arg == "--auto-minimize")
            {
                _autoMinimizeOnFail = true;
            }
            else if (arg.StartsWith("--profile="))
            {
                ApplyProfilePreset(ValueAfterEquals(arg));
            }
            else if (arg == "-I")
            {
                if (args.Count > 0) AddIncludeDirectory(args.Dequeue());
                else Error("error: -I option requires a directory path");
            }
            else if (arg.StartsWith("-I") && arg.Length > 2)
            {
                AddIncludeDirectory(arg.Substring(2));
            }
            else if (arg.StartsWith("--include-dir="))
            {
                AddIncludeDirectory(ValueAfterEquals(arg));
            }
            else if (arg == "-O0")
            {
                OptLevel = 0;
            }
            else if (arg == "-O1")
            {
                OptLevel = 1;
            }
            else if (arg.StartsWith("--abi="))
            {
                string v = arg.Substring("--abi=".Length).Trim().ToLowerInvariant();
                if (v == "stack") Abi = AbiMode.Stack;
                else if  (v == "legacy" || v == "default") Abi = AbiMode.Legacy;
                else Error("error: --abi must be legacy or stack");
            }
            else if (arg == "--bank-sim")
            {
                EmitBankSimReport = true;
                BankSimReportPath = "";
            }
            else if (arg.StartsWith("--bank-sim="))
            {
                EmitBankSimReport = true;
                BankSimReportPath = ValueAfterEquals(arg);
            }
            else if (arg == "--farcall-suggest")
            {
                EmitFarcallSuggestionReport = true;
                FarcallSuggestionReportPath = "";
            }
            else if (arg.StartsWith("--farcall-suggest="))
            {
                EmitFarcallSuggestionReport = true;
                FarcallSuggestionReportPath = ValueAfterEquals(arg);
            }
            else if (arg == "--cross-bank-report")
            {
                EmitCrossBankCallReport = true;
                CrossBankCallReportPath = "";
            }
            else if (arg.StartsWith("--cross-bank-report="))
            {
                EmitCrossBankCallReport = true;
                CrossBankCallReportPath = ValueAfterEquals(arg);
            }
            else if (arg == "--abi-verify")
            {
                EmitAbiVerifyReport = true;
                AbiVerifyReportPath = "";
            }
            else if (arg.StartsWith("--abi-verify="))
            {
                EmitAbiVerifyReport = true;
                AbiVerifyReportPath = ValueAfterEquals(arg);
            }
            else if (arg == "--abi-diff-report")
            {
                EmitAbiDiffReport = true;
                AbiDiffReportPath = "";
            }
            else if (arg.StartsWith("--abi-diff-report="))
            {
                EmitAbiDiffReport = true;
                AbiDiffReportPath = ValueAfterEquals(arg);
            }
            else if (arg == "--rst-report")
            {
                EmitRstApplyReport = true;
                RstApplyReportPath = "";
            }
            else if (arg.StartsWith("--rst-report="))
            {
                EmitRstApplyReport = true;
                RstApplyReportPath = ValueAfterEquals(arg);
            }
            else if (arg == "--opt-diff")
            {
                EmitOptDiffReport = true;
                OptDiffReportPath = "";
            }
            else if (arg.StartsWith("--opt-diff="))
            {
                EmitOptDiffReport = true;
                OptDiffReportPath = ValueAfterEquals(arg);
            }
            else if (arg == "--func-size-report")
            {
                EmitFunctionSizeReport = true;
                FunctionSizeReportPath = "";
            }
            else if (arg.StartsWith("--func-size-report="))
            {
                EmitFunctionSizeReport = true;
                FunctionSizeReportPath = ValueAfterEquals(arg);
            }
            else if (arg == "--hotspot-report")
            {
                EmitHotspotReport = true;
                HotspotReportPath = "";
            }
            else if (arg.StartsWith("--hotspot-report="))
            {
                EmitHotspotReport = true;
                HotspotReportPath = ValueAfterEquals(arg);
            }
            else if (arg == "--repro-check")
            {
                EnableReproCheck = true;
                ReproCheckReportPath = "";
            }
            else if (arg.StartsWith("--repro-check="))
            {
                EnableReproCheck = true;
                ReproCheckReportPath = ValueAfterEquals(arg);
            }
            else if (arg == "--cgb-consistency")
            {
                EmitCgbConsistencyReport = true;
                CgbConsistencyReportPath = "";
            }
            else if (arg.StartsWith("--cgb-consistency="))
            {
                EmitCgbConsistencyReport = true;
                CgbConsistencyReportPath = ValueAfterEquals(arg);
            }
            else if (arg == "--verify-cgb-symbols")
            {
                EmitCgbSymbolVerifyReport = true;
                CgbSymbolVerifyReportPath = "";
            }
            else if (arg.StartsWith("--verify-cgb-symbols="))
            {
                EmitCgbSymbolVerifyReport = true;
                CgbSymbolVerifyReportPath = ValueAfterEquals(arg);
            }
else if (arg == "-Zcheck")
            {
                CheckBounds = true;
                CheckMemCopy = true;
                CheckStack = true;
                CheckBankCalls = true;
                CheckSliceBounds = true;
            }
            else if (arg == "-Zcheck-bounds" || arg == "-Zcheck_bounds")
            {
                CheckBounds = true;
            }
            else if (arg == "-Zconst-scalar-in-rom" || arg == "-Zconst_scalar_in_rom")
            {
                ConstScalarInRom = true;
            }
            else if (arg == "--debug-output" || arg == "--debug-out")
            {
                EnableDebugOutput = true;
            }
            else if (arg.StartsWith("--debug-output=") || arg.StartsWith("--debug-out="))
            {
                EnableDebugOutput = true;
                int eq = arg.IndexOf('=');
                if (eq >= 0 && eq + 1 < arg.Length)
                {
                    DebugOutputPath = arg.Substring(eq + 1);
                }
            }
            else if (arg == "vlist" || arg == "--vlist")
            {
                EmitVarList = true;
            }
            else if (arg.StartsWith("--vlist=", StringComparison.Ordinal) || arg.StartsWith("--vlist-out=", StringComparison.Ordinal))
            {
                EmitVarList = true;
                int eq = arg.IndexOf('=');
                if (eq >= 0 && eq + 1 < arg.Length)
                {
                    VarListOutputPath = arg.Substring(eq + 1);
                }
            }
            else if (arg == "--rst-disable" || arg == "--no-rst" || arg == "--rst-off")
            {
                RstDisable = true;
            }
            else if (arg == "--rst-enable" || arg == "--rst" || arg == "--rst-on")
            {
                RstDisable = false;
            }
            else if (arg == "--rst-use-38")
            {
                // Any RST tuning option implies the user intends to use RST mapping.
                RstDisable = false;
                RstUse38 = true;
            }
            else if (arg == "--rst-speed-safe")
            {
                RstDisable = false;
                RstSpeedSafe = true;
                // Speed-safe preset: by default, avoid RST mapping on very hot targets.
                // You can override with --rst-max-calls.
                if (RstMaxCalls == int.MaxValue) RstMaxCalls = 12;
                else RstMaxCalls = System.Math.Min(RstMaxCalls, 12);
            }
            else if (arg == "--rst-unsafe")
            {
                RstDisable = false;
                RstUnsafe = true;
            }
            else if (arg == "--rst-safe")
            {
                RstDisable = false;
                RstUnsafe = false;
            }
            else if (arg.StartsWith("--rst-max-calls="))
            {
                RstDisable = false;
                int eq = arg.IndexOf('=');
                if (eq >= 0 && eq + 1 < arg.Length && int.TryParse(arg.Substring(eq + 1), out int v) && v > 0)
                    RstMaxCalls = v;
                else
                    Error("error: --rst-max-calls requires a positive integer");
            }
            else if (arg.StartsWith("--rst-max-vectors="))
            {
                RstDisable = false;
                int eq = arg.IndexOf('=');
                if (eq >= 0 && eq + 1 < arg.Length && int.TryParse(arg.Substring(eq + 1), out int v) && v >= 0)
                    RstMaxVectors = v;
                else
                    Error("error: --rst-max-vectors requires a non-negative integer");
            }
            else if (arg.StartsWith("--rst-exclude="))
            {
                RstDisable = false;
                int eq = arg.IndexOf('=');
                string s = (eq >= 0 && eq + 1 < arg.Length) ? arg.Substring(eq + 1) : "";
                foreach (var part in s.Split(new char[] { ',', ';' }, System.StringSplitOptions.RemoveEmptyEntries))
                {
                    string name = part.Trim();
                    if (name.Length > 0) RstExcludeLabels.Add(name);
                }
            }
            else if (arg == "--attach")
            {
                AttachDebuggerOnError = true;
            }
            else if (arg == "--trace")
            {
                TraceEnabled = true;
                // Default: dump all stages.
                TraceStages.Clear();
                TraceStages.Add("tokens");
                TraceStages.Add("ast");
                TraceStages.Add("ir");
                TraceStages.Add("asm");
            }
            else if (arg.StartsWith("--trace="))
            {
                TraceEnabled = true;
                TraceStages.Clear();
                int eq = arg.IndexOf('=');
                string list = (eq >= 0 && eq + 1 < arg.Length) ? arg.Substring(eq + 1) : "";
                foreach (var part in list.Split(new char[] { ',', ';' }, StringSplitOptions.RemoveEmptyEntries))
                {
                    string s = part.Trim();
                    if (s.Length > 0) TraceStages.Add(s);
                }
                // If user passed an empty list, treat as "all".
                if (TraceStages.Count == 0)
                {
                    TraceStages.Add("tokens");
                    TraceStages.Add("ast");
                    TraceStages.Add("ir");
                    TraceStages.Add("asm");
                }
            }
            else if (arg.StartsWith("--trace-out="))
            {
                TraceEnabled = true;
                int eq = arg.IndexOf('=');
                if (eq >= 0 && eq + 1 < arg.Length)
                {
                    TraceOutputPath = arg.Substring(eq + 1);
                }
            }
            else if (arg == "--disasm-changed")
            {
                EmitChangedFunctionDisasm = true;
                ChangedFunctionDisasmBaseRef = "";
            }
            else if (arg.StartsWith("--disasm-changed="))
            {
                EmitChangedFunctionDisasm = true;
                ChangedFunctionDisasmBaseRef = ValueAfterEquals(arg);
            }
            else if (arg.StartsWith("--disasm-changed-out="))
            {
                EmitChangedFunctionDisasm = true;
                ChangedFunctionDisasmOutPath = ValueAfterEquals(arg);
            }
            else if (arg == "--repro-pack")
            {
                EmitReproPackageOnFail = true;
                ReproPackagePath = "";
            }
            else if (arg.StartsWith("--repro-pack="))
            {
                EmitReproPackageOnFail = true;
                ReproPackagePath = ValueAfterEquals(arg);
            }
            // --- ROM header patching (fix31_rom_header_patch) ---
            else if (arg.StartsWith("--rom-header="))
            {
                string path = ValueAfterEquals(arg);
                try
                {
                    var fromJson = RomHeaderOptions.LoadFromJson(path);
                    // JSON acts like a CLI template: later explicit CLI flags override it.
                    RomHeader.MergeFrom(fromJson, overwrite: true);
                }
                catch (Exception ex)
                {
                    Error("error: --rom-header failed to load '{0}': {1}", path, ex.Message);
                }
            }
            else if (arg.StartsWith("--rom-title="))
            {
                RomHeader.Title = ValueAfterEquals(arg);
            }
            else if (arg.StartsWith("--cgb="))
            {
                if (!RomHeader.TrySetCgb(ValueAfterEquals(arg), out string err)) Error(err);
            }
            else if (arg.StartsWith("--cart="))
            {
                if (!RomHeader.TrySetCart(ValueAfterEquals(arg), out string err)) Error(err);
            }
            else if (arg.StartsWith("--romsize="))
            {
                if (!RomHeader.TrySetRomSize(ValueAfterEquals(arg), out string err)) Error(err);
            }
            else if (arg.StartsWith("--ramsize="))
            {
                if (!RomHeader.TrySetRamSize(ValueAfterEquals(arg), out string err)) Error(err);
            }
            else if (arg.StartsWith("--sgb="))
            {
                if (!RomHeader.TrySetSgb(ValueAfterEquals(arg), out string err)) Error(err);
            }
            else if (arg.StartsWith("--dest="))
            {
                if (!RomHeader.TrySetDest(ValueAfterEquals(arg), out string err)) Error(err);
            }
            else if (arg.StartsWith("--version="))
            {
                string v = ValueAfterEquals(arg);
                if (!byte.TryParse(v, out byte bv)) Error("error: --version must be 0..255");
                RomHeader.Version = bv;
            }
            else if (arg.StartsWith("--max-errors="))
            {
                int eq = arg.IndexOf('=');
                if (eq >= 0 && eq + 1 < arg.Length && int.TryParse(arg.Substring(eq + 1), out int v) && v > 0)
                    MaxErrors = v;
                else
                    Error("error: --max-errors requires a positive integer");
            }
            else if (arg == "-o")
            {
                if (args.Count > 0) outputFilename = args.Dequeue();
                else Error("error: -o option requires a filename");
            }
            else if (arg.StartsWith("-"))
            {
                Error("error: unknown option: " + arg);
            }
            else
            {
                sourceFilenames.Add(arg);
            }
        }

        if (help)
        {
            Console.Error.WriteLine("usage: kitaqgb first.c second.c ... [-o out.gb] [-O0|-O1] [-I dir|--include-dir=dir] [--profile=dev|release|test] [--strict|--permissive] [--rom-header=header.json] [--rom-title=TITLE] [--cgb=dmg|cgb|cgb_only] [--cart=romonly|mbc1|mbc3|mbc5|...] [--romsize=32k|64k|128k|256k|512k|1m|2m|4m|8m] [--ramsize=none|2k|8k|32k|64k|128k] [--sgb=on|off] [--dest=jp|nonjp] [--version=N] [--trace[=tokens,ast,ir,asm] [--trace-out=<dir>]] [--debug-output | --debug-out[=<dir>]] [--no-disasm|--fast-build] [--disasm-changed[=<git_base>] [--disasm-changed-out=<file>]] [--cache|--no-cache] [--diag-json[=<file>]] [--emit-path-manifest=<file>] [--machine-readable] [--no-banner] [--stack-bank=fixed|wramx1] [--stack-top=ADDR] [--stack-reserve=N] [--deps-out[=<file>]] [vlist|--vlist[=<file>]] [--max-errors=N] [--abi=legacy|stack] [--bank-sim[=<file>]] [--farcall-suggest[=<file>]] [--cross-bank-report[=<file>]] [--abi-verify[=<file>]] [--abi-diff-report[=<file>]] [--rst-report[=<file>]] [--opt-diff[=<file>]] [--func-size-report[=<file>]] [--hotspot-report[=<file>]] [--repro-check[=<file>]] [--repro-pack[=<dir>]] [--cgb-consistency[=<file>]] [--verify-cgb-symbols[=<file>]] [--minimize[=<file.c>] [--minimize-out=<out.c>] [--minimize-work=<dir>] [--minimize-quick] [--minimize-trace=final|all] [--minimize-on-fail]] [--rst-enable|--rst-on] [--rst-disable|--no-rst] [--rst-use-38] [--rst-speed-safe] [--rst-safe|--rst-unsafe] [--rst-max-calls=N] [--rst-exclude=a,b] [--rst-max-vectors=N] [--watch]");
            Console.Error.WriteLine("subcommands: kitaqgb test | attrviz | src2asm | symfind | romdiff | kqhelp | template | fixhint | irsum | conventions | snippet | devserver | recipe");
            Console.Error.WriteLine("  --stack-bank: select WRAM window used for initial CPU stack");
            Console.Error.WriteLine("  --stack-top: initial SP value inside selected stack window");
            Console.Error.WriteLine("  --stack-reserve: reserve N bytes below stack top from automatic placement");
            Exit(1);
        }

        if (sourceFilenames.Count == 0)
        {
            Error("error: no source files provided");
        }

        _outputFilenameForDiag = outputFilename;
        FinalizeAndValidateStackPolicy();
        if (ErrorCount > 0) Exit(1);

        if (EnableIncrementalCache && CanUseBuildCacheForThisRun())
        {
            string restoredCacheKey;
            if (TryRestoreBuildCache(sourceFilenames, outputFilename, out restoredCacheKey))
            {
                _cacheKey = restoredCacheKey;
                WriteInfoLine("[cache] hit: " + _cacheKey);
                if (TraceEnabled)
                {
                    var cacheStats = new List<TraceStageStat>();
                    cacheStats.Add(new TraceStageStat { Name = "cache_restore", ElapsedMs = 0, Detail = _cacheKey });
                    EmitTraceShortSummary(cacheStats, 0, sourceFilenames, outputFilename);
                }
                Exit(0);
            }
        }

        var traceStageStats = new List<TraceStageStat>();
        var compileTotalSw = Stopwatch.StartNew();

        try
        {
            // 0. Tokenize (optional trace)
            if (TraceEnabled && (TraceStages.Contains("tokens") || TraceStages.Contains("all")))
            {
                var swTokens = Stopwatch.StartNew();
                Directory.CreateDirectory(TraceOutputPath);
                StringBuilder all = new StringBuilder();
                foreach (string fn in sourceFilenames)
                {
                    var toks = Tokenizer.TokenizeFile(fn);
                    all.AppendLine("==== TOKENS: " + fn + " ====");
                    all.AppendLine(ShowTokens(toks));
                    all.AppendLine();
                }
                WriteTraceFile("tokens.txt", all.ToString());
                swTokens.Stop();
                traceStageStats.Add(new TraceStageStat { Name = "tokens", ElapsedMs = swTokens.ElapsedMilliseconds, Detail = sourceFilenames.Count + " file(s)" });
            }

            // 1. Parse
            var swParse = Stopwatch.StartNew();
            Expr syntaxTree = Parser.ParseFiles(sourceFilenames);
            if (ErrorCount > 0) Exit(1);
            syntaxTree = InjectCgbPaletteDeclarations(syntaxTree);
            if (EnableDebugOutput && CgbPalettePragmas.Count > 0)
            {
                WriteDebugFile("cgb_palettes.txt", BuildCgbPaletteText());
            }
            if (EnableDebugOutput) WritePassOutputToFile("syntax_tree", syntaxTree.ShowMultiline());
            if (TraceEnabled && (TraceStages.Contains("ast") || TraceStages.Contains("all")))
            {
                WriteTraceFile("ast_simple.txt", ShowAstSimple(syntaxTree));
                WriteTraceFile("ast_full.txt", syntaxTree.ShowMultiline());
            }
            swParse.Stop();
            traceStageStats.Add(new TraceStageStat { Name = "parse", ElapsedMs = swParse.ElapsedMilliseconds, Detail = "ast built" });

            // 1.5 Lowering (introduce explicit temporaries for a few key patterns)
            var swLower = Stopwatch.StartNew();
            syntaxTree = Lowerer.Lower(syntaxTree);
            if (ErrorCount > 0) Exit(1);
            if (EnableDebugOutput) WritePassOutputToFile("syntax_tree_lowered", syntaxTree.ShowMultiline());
            if (TraceEnabled && (TraceStages.Contains("ir") || TraceStages.Contains("all")))
            {
                WriteTraceFile("ir_lowered.txt", syntaxTree.ShowMultiline());
            }
            swLower.Stop();
            traceStageStats.Add(new TraceStageStat { Name = "lower", ElapsedMs = swLower.ElapsedMilliseconds, Detail = "ir lowered" });

            // 2. CodeGen + 3. Assemble
            long totalCodegenMs = 0;
            long totalAssembleMs = 0;
            int bankRelayoutPasses = 0;
            IReadOnlyList<Expr> assembly = null;
            string actualOutputFilename = outputFilename;
            ClearFunctionBankOverrides();

            while (true)
            {
                var swCodegen = Stopwatch.StartNew();
                assembly = CodeGenerator.CompileAll(syntaxTree);
                if (ErrorCount > 0) Exit(1);
                if (EnableDebugOutput) WritePassOutputToFile("assembly_code", ShowAssembly(assembly));
                if (TraceEnabled && (TraceStages.Contains("asm") || TraceStages.Contains("all")))
                {
                    WriteTraceFile("asm.txt", ShowAssembly(assembly));
                }
                swCodegen.Stop();
                totalCodegenMs += swCodegen.ElapsedMilliseconds;

                var swAssemble = Stopwatch.StartNew();
                actualOutputFilename = Assembler.Assemble(assembly, outputFilename);
                outputFilename = actualOutputFilename;
                _outputFilenameForDiag = outputFilename;
                if (ErrorCount > 0) Exit(1);
                swAssemble.Stop();
                totalAssembleMs += swAssemble.ElapsedMilliseconds;

                var relayout = DetectFunctionBankRelocations(CodeGenerator.LastReport, Assembler.LastReport);
                if (relayout.FixedBankConflicts.Count > 0)
                {
                    Error("error: fixed-bank functions spilled to a different bank: {0}",
                        string.Join(", ", relayout.FixedBankConflicts
                            .OrderBy(x => x.Name, StringComparer.Ordinal)
                            .Select(x => x.Name + " requested b" + x.RequestedBank + " actual b" + x.ActualBank)));
                    Exit(1);
                }

                var relocations = relayout.Relocations;
                if (relocations.Count == 0)
                    break;

                bankRelayoutPasses++;
                if (bankRelayoutPasses > 4)
                {
                    Error("error: function bank layout did not stabilize after {0} passes: {1}",
                        bankRelayoutPasses,
                        string.Join(", ", relocations.OrderBy(x => x.Key, StringComparer.Ordinal)
                            .Select(x => x.Key + "->b" + x.Value)));
                    Exit(1);
                }

                ReplaceFunctionBankOverrides(relocations);
                WriteErrorLine("[bank-layout] recompiling with actual function banks: " +
                    string.Join(", ", relocations.OrderBy(x => x.Key, StringComparer.Ordinal)
                        .Select(x => x.Key + "->b" + x.Value)));
            }

            string codegenDetail = (assembly == null ? 0 : assembly.Count).ToString() + " asm node(s)";
            if (bankRelayoutPasses > 0) codegenDetail += ", relayout_passes=" + bankRelayoutPasses;
            traceStageStats.Add(new TraceStageStat { Name = "codegen", ElapsedMs = totalCodegenMs, Detail = codegenDetail });
            traceStageStats.Add(new TraceStageStat { Name = "assemble", ElapsedMs = totalAssembleMs, Detail = Path.GetFileName(outputFilename) });

            // 3.5 Patch ROM header & checksums (fix31_rom_header_patch)
            var swHeader = Stopwatch.StartNew();
            var effectiveRomHeader = new RomHeaderOptions();
            // defaults -> pragma -> cli/json
            effectiveRomHeader.MergeFrom(RomHeaderPragma, overwrite: true);
            effectiveRomHeader.MergeFrom(RomHeader, overwrite: true);

            // If the ROM is larger than 32KB, it *must* use an MBC; otherwise banked code/data is unreachable.
            // Some projects rely on #pragma rom_* in a dedicated romcfg file; however, to avoid producing
            // non-bankable ROMs by accident, we auto-fill the minimum-required fields when absent.
            bool autoFilled = false;
            try
            {
                var romBytes = File.ReadAllBytes(outputFilename);
                int romLen = romBytes.Length;

                if (!effectiveRomHeader.RomSizeCode.HasValue)
                {
                    byte? code = GuessRomSizeCodeFromLength(romLen);
                    if (code.HasValue)
                    {
                        effectiveRomHeader.RomSizeCode = code.Value;
                        autoFilled = true;
                    }
                }

                if (!effectiveRomHeader.CartType.HasValue)
                {
                    if (romLen > 32 * 1024)
                    {
                        // Safe default: MBC5 ROM-only. Projects can override via #pragma rom_cart / CLI.
                        effectiveRomHeader.CartType = 0x19;
                        autoFilled = true;
                    }
                }
            }
            catch
            {
                // ignore (header patching is best-effort)
            }

            if (effectiveRomHeader.HasAny || autoFilled)
            {
                RomHeaderPatcher.PatchFile(outputFilename, effectiveRomHeader);
                if (ErrorCount > 0) Exit(1);
            }
            swHeader.Stop();
            traceStageStats.Add(new TraceStageStat { Name = "header", ElapsedMs = swHeader.ElapsedMilliseconds, Detail = (effectiveRomHeader.HasAny || autoFilled) ? "patched" : "unchanged" });

            // 4. Disassemble
            if (EnableDebugOutput && !DisableDisasm)
            {
                var swDisasm = Stopwatch.StartNew();
                Disassembler.Disassemble(outputFilename);
                swDisasm.Stop();
                traceStageStats.Add(new TraceStageStat { Name = "disasm", ElapsedMs = swDisasm.ElapsedMilliseconds, Detail = "debug_output/dis.s" });
            }
            else if (EnableDebugOutput && DisableDisasm)
            {
                traceStageStats.Add(new TraceStageStat { Name = "disasm", ElapsedMs = 0, Detail = "skipped (--no-disasm)" });
            }

            if (EnableDebugOutput && EmitChangedFunctionDisasm && !DisableDisasm)
            {
                var swChangedDis = Stopwatch.StartNew();
                TryWriteChangedFunctionDisasm(outputFilename, sourceFilenames, ChangedFunctionDisasmBaseRef, ChangedFunctionDisasmOutPath);
                swChangedDis.Stop();
                traceStageStats.Add(new TraceStageStat { Name = "disasm_changed", ElapsedMs = swChangedDis.ElapsedMilliseconds, Detail = "filtered" });
            }
            else if (EnableDebugOutput && EmitChangedFunctionDisasm && DisableDisasm)
            {
                Warning("warning: --disasm-changed was requested but disassembly is disabled (--no-disasm)");
            }

            var swReports = Stopwatch.StartNew();
            RunAnalysisReports(sourceFilenames, assembly, outputFilename, effectiveRomHeader);
            if (ErrorCount > 0) Exit(1);
            swReports.Stop();
            traceStageStats.Add(new TraceStageStat { Name = "reports", ElapsedMs = swReports.ElapsedMilliseconds, Detail = "analysis" });

            TryWriteDependenciesList(sourceFilenames, outputFilename);

            if (EnableIncrementalCache && CanUseBuildCacheForThisRun())
            {
                var swCacheSave = Stopwatch.StartNew();
                if (string.IsNullOrEmpty(_cacheKey))
                {
                    _cacheKey = ComputeBuildCacheKey(sourceFilenames);
                }
                TrySaveBuildCache(_cacheKey, outputFilename);
                swCacheSave.Stop();
                traceStageStats.Add(new TraceStageStat { Name = "cache_save", ElapsedMs = swCacheSave.ElapsedMilliseconds, Detail = _cacheKey });
            }
        }
        catch (ControlledCompilerExit)
        {
            throw;
        }
        catch (Exception ex)
        {
            Panic(ex.Message + "\n" + ex.StackTrace);
        }

        compileTotalSw.Stop();
        if (TraceEnabled)
        {
            EmitTraceShortSummary(traceStageStats, compileTotalSw.ElapsedMilliseconds, sourceFilenames, outputFilename);
        }

        Exit(0);
    }

    public static void WriteDebugFile(string filename, string text)
    {
        if (EnableDebugOutput)
        {
            Directory.CreateDirectory(DebugOutputPath);
            RememberArtifactPath("debug_dir", DebugOutputPath);
            IoUtil.WriteAllTextUtf8Robust(Path.Combine(DebugOutputPath, filename), text, allowAlternatePath: true);
        }
    }

    static void WriteTraceFile(string filename, string text)
    {
        if (!TraceEnabled) return;
        Directory.CreateDirectory(TraceOutputPath);
        RememberArtifactPath("trace_dir", TraceOutputPath);
        IoUtil.WriteAllTextUtf8Robust(Path.Combine(TraceOutputPath, filename), text, allowAlternatePath: true);
    }

    static byte? GuessRomSizeCodeFromLength(int romLenBytes)
    {
        // GB header ROM size codes (most common sizes). We round up to the next supported size.
        int[] sizes = new int[]
        {
            32 * 1024,
            64 * 1024,
            128 * 1024,
            256 * 1024,
            512 * 1024,
            1024 * 1024,
            2 * 1024 * 1024,
            4 * 1024 * 1024,
            8 * 1024 * 1024,
        };

        byte[] codes = new byte[] { 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08 };

        for (int i = 0; i < sizes.Length; i++)
        {
            if (romLenBytes <= sizes[i]) return codes[i];
        }
        return null;
    }

    static string ValueAfterEquals(string arg)
    {
        int eq = arg.IndexOf('=');
        if (eq >= 0 && eq + 1 < arg.Length) return arg.Substring(eq + 1);
        return "";
    }

    // Called from Tokenizer when it sees #pragma rom_* directives.
    // Priority rule: CLI/JSON (RomHeader) overrides pragma.
    public static void ApplyRomHeaderPragma(FilePosition pos, string key, string value)
    {
        // Normalize legacy key names.
        string k = (key ?? "").Trim();
        if (k.Equals("rom_title", StringComparison.OrdinalIgnoreCase)) k = "rom_title";
        if (k.Equals("rom-title", StringComparison.OrdinalIgnoreCase)) k = "rom_title";

        // Option B: #pragma rom_header "header.json"
        // Loads a JSON template and applies it as pragma-level defaults.
        // Priority rule (already enforced here): CLI/JSON > pragma > defaults.
        // Within pragma level: rom_header provides defaults, and later explicit rom_* pragmas can override.
        if (k.Equals("rom_header", StringComparison.OrdinalIgnoreCase) ||
            k.Equals("romheader", StringComparison.OrdinalIgnoreCase) ||
            k.Equals("rom-header", StringComparison.OrdinalIgnoreCase))
        {
            // If the CLI already specified a rom-header template, we still allow pragma rom_header
            // to fill any remaining *unset* fields. However, any field already set by CLI is never overridden.

            string path = (value ?? "").Trim();
            if (string.IsNullOrEmpty(path))
            {
                Warning(pos, "warning: #pragma rom_header requires a path");
                return;
            }

            // Resolve relative to the current source file.
            try
            {
                if (!Path.IsPathRooted(path))
                {
                    string baseDir = "";
                    try { baseDir = Path.GetDirectoryName(pos.Filename) ?? ""; } catch { baseDir = ""; }
                    if (!string.IsNullOrEmpty(baseDir)) path = Path.Combine(baseDir, path);
                }

                var tpl = RomHeaderOptions.LoadFromJson(path);

                // Apply each field as pragma-level defaults:
                // - never override CLI (RomHeader)
                // - never override already-set pragma fields (so explicit #pragma rom_title can override)
                if (tpl.Title != null && !RomHeader.IsFieldSetByKey("title") && RomHeaderPragma.Title == null)
                    RomHeaderPragma.Title = tpl.Title;

                if (tpl.CgbFlag.HasValue && !RomHeader.IsFieldSetByKey("cgb") && !RomHeaderPragma.CgbFlag.HasValue)
                    RomHeaderPragma.CgbFlag = tpl.CgbFlag;

                if (tpl.SgbFlag.HasValue && !RomHeader.IsFieldSetByKey("sgb") && !RomHeaderPragma.SgbFlag.HasValue)
                    RomHeaderPragma.SgbFlag = tpl.SgbFlag;

                if (tpl.CartType.HasValue && !RomHeader.IsFieldSetByKey("cart") && !RomHeaderPragma.CartType.HasValue)
                    RomHeaderPragma.CartType = tpl.CartType;

                if ((tpl.RomSizeCode.HasValue || tpl.RomSizeBytes.HasValue) &&
                    !RomHeader.IsFieldSetByKey("romsize") &&
                    !(RomHeaderPragma.RomSizeCode.HasValue || RomHeaderPragma.RomSizeBytes.HasValue))
                {
                    RomHeaderPragma.RomSizeCode = tpl.RomSizeCode;
                    RomHeaderPragma.RomSizeBytes = tpl.RomSizeBytes;
                }

                if (tpl.RamSizeCode.HasValue && !RomHeader.IsFieldSetByKey("ramsize") && !RomHeaderPragma.RamSizeCode.HasValue)
                    RomHeaderPragma.RamSizeCode = tpl.RamSizeCode;

                if (tpl.DestinationCode.HasValue && !RomHeader.IsFieldSetByKey("dest") && !RomHeaderPragma.DestinationCode.HasValue)
                    RomHeaderPragma.DestinationCode = tpl.DestinationCode;

                if (tpl.Version.HasValue && !RomHeader.IsFieldSetByKey("version") && !RomHeaderPragma.Version.HasValue)
                    RomHeaderPragma.Version = tpl.Version;
            }
            catch (Exception ex)
            {
                Warning(pos, "warning: #pragma rom_header failed to load '" + path + "': " + ex.Message);
            }
            return;
        }

        // If CLI already specified this field, ignore the pragma.
        if (RomHeader.IsFieldSetByKey(k)) return;

        // Apply to pragma options.
        if (!RomHeaderPragma.TrySetByKey(k, value, out string err))
        {
            if (!string.IsNullOrEmpty(err)) Warning(pos, err);
            return;
        }
    }

    public static void ApplyCgbPalettePragma(FilePosition pos, string name, int[] colors)
    {
        string n = (name ?? "").Trim();
        if (string.IsNullOrEmpty(n))
        {
            Warning(pos, "warning: #pragma cgb_palette requires a name");
            return;
        }

        if (colors == null || colors.Length != 4)
        {
            Warning(pos, "warning: #pragma cgb_palette requires exactly 4 colors");
            return;
        }

        for (int i = 0; i < colors.Length; i++)
        {
            if (colors[i] < 0 || colors[i] > 0x7FFF)
            {
                Warning(pos, "warning: #pragma cgb_palette color out of range (0..0x7FFF): " + colors[i]);
                return;
            }
        }

        for (int i = CgbPalettePragmas.Count - 1; i >= 0; i--)
        {
            if (string.Equals(CgbPalettePragmas[i].Name, n, StringComparison.Ordinal))
            {
                CgbPalettePragmas.RemoveAt(i);
            }
        }

        CgbPalettePragmas.Add(new CgbPaletteDefinition
        {
            Name = n,
            Colors = colors.ToArray(),
            Position = pos
        });
    }

    static Expr InjectCgbPaletteDeclarations(Expr syntaxTree)
    {
        if (CgbPalettePragmas.Count == 0) return syntaxTree;

        Expr[] decls;
        if (!syntaxTree.MatchAny(Tag.Sequence, out decls)) return syntaxTree;

        var merged = new List<Expr>(decls ?? new Expr[0]);
        var usedNames = new HashSet<string>(StringComparer.Ordinal);

        foreach (var d in merged)
        {
            if (TryGetTopLevelDeclName(d, out string dn) && !string.IsNullOrEmpty(dn))
                usedNames.Add(dn);
        }

        foreach (var p in CgbPalettePragmas)
        {
            if (usedNames.Contains(p.Name))
            {
                Warning(p.Position, "warning: cgb palette name collides with existing symbol: " + p.Name);
                continue;
            }

            var type = CType.MakeArray(CType.UInt16, 4);
            type.IsConst = true;
            Expr rd = Expr.Make(Tag.ReadonlyData, type, p.Name, p.Colors.ToArray()).WithSource(p.Position);
            merged.Add(rd);
            usedNames.Add(p.Name);
        }

        var args = new object[merged.Count + 1];
        args[0] = Tag.Sequence;
        for (int i = 0; i < merged.Count; i++) args[i + 1] = merged[i];
        return Expr.Make(args).WithSource(syntaxTree.Source);
    }

    static bool TryGetTopLevelDeclName(Expr decl, out string name)
    {
        name = null;
        if (decl == null) return false;

        Expr d = decl;
        int guard = 0;
        while (guard++ < 16)
        {
            Expr inner;
            int i;
            string s;
            if (d.Match(Tag.Bank, out i, out inner) ||
                d.Match(Tag.FixedBank, out i, out inner) ||
                d.Match(Tag.FixedOrder, out i, out inner) ||
                d.Match(Tag.DeclAlign, out i, out inner) ||
                d.Match(Tag.DeclSection, out s, out inner) ||
                d.Match(Tag.Unsafe, out inner) ||
                d.Match(Tag.StackCall, out inner))
            {
                d = inner;
                continue;
            }
            break;
        }

        CType t; FieldInfo[] f; Expr body; Expr range;
        if (d.Match(Tag.ReadonlyData, out t, out name, out int[] _rd)) return true;
        if (d.Match(Tag.ReadonlyData, out t, out name, out Expr[] _rdExprs)) return true;
        if (d.Match(Tag.Constant, out t, out name, out Expr _cv)) return true;
        if (d.Match(Tag.Function, out t, out name, out f, out int _mc, out body) ||
            d.Match(Tag.Function, out t, out name, out f, out body)) return true;
        if (d.Match(Tag.InlineFunction, out t, out name, out f, out int _imc, out body) ||
            d.Match(Tag.InlineFunction, out t, out name, out f, out body)) return true;
        if (d.Match(Tag.FunctionDecl, out t, out name, out f, out int _pmc) ||
            d.Match(Tag.FunctionDecl, out t, out name, out f)) return true;
        if (d.Match(Tag.Variable, out MemoryRegion _r, out t, out name, out range) ||
            d.Match(Tag.Variable, out _r, out t, out name)) return true;
        if (d.Match(Tag.ExternVariable, out MemoryRegion _er, out t, out name, out range) ||
            d.Match(Tag.ExternVariable, out _er, out t, out name)) return true;
        return false;
    }

    static string BuildCgbPaletteText()
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB CGB palette DSL");
        sb.AppendLine("# name, c0, c1, c2, c3");
        foreach (var p in CgbPalettePragmas)
        {
            sb.AppendFormat("{0}, 0x{1:X4}, 0x{2:X4}, 0x{3:X4}, 0x{4:X4}\n",
                p.Name, p.Colors[0], p.Colors[1], p.Colors[2], p.Colors[3]);
        }
        return sb.ToString();
    }

    public static void WritePassOutputToFile(string passName, string output)
    {
        WriteDebugFile(string.Format("{0}.txt", passName), output);
    }

    public static string ShowAssemblyPublic(IReadOnlyList<Expr> assembly)
    {
        return ShowAssembly(assembly);
    }

    static string ShowAssembly(IReadOnlyList<Expr> assembly)
    {
        StringBuilder sb = new StringBuilder();
        foreach (Expr e in assembly)
        {
            string line = "";
            string mnemonic, text;
            AsmOperand operand;

            bool isTopLevel = e.MatchTag(Tag.Function);
            if (isTopLevel) sb.AppendLine();
            if (!isTopLevel && !e.MatchTag(Tag.Label)) line = "\t";

            if (Config.ShowSourcePositionInDebugOutput)
            {
                sb.AppendLine(line + "; <" + e.Source + ">");
            }

            if (e.Match(Tag.Comment, out text)) line += "; " + text;
            else if (e.Match(Tag.Label, out text)) line += text + ":";
            else if (e.Match(Tag.Function, out text)) line += string.Format("; function {0}:", text);
            else if (e.MatchTag(Tag.Function)) line += e.Show();
            else if (e.Match(Tag.Asm, out mnemonic, out operand)) line += FormatAssembly(mnemonic, operand);
            else line += e.Show();

            sb.AppendLine(line);
        }
        return sb.ToString();
    }

    static string ShowTokens(List<Token> tokens)
    {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < tokens.Count; i++)
        {
            Token t = tokens[i];
            // Position can be noisy; keep it compact but present.
            sb.Append(t.Position.ToString());
            sb.Append("\t");
            sb.Append(t.Tag.ToString());
            if (t.Tag == TokenType.INT) sb.Append("\t" + t.Int);
            else if (t.Tag == TokenType.NAME) sb.Append("\t" + t.Name);
            else if (t.Tag == TokenType.STRING) sb.Append("\t\"" + t.Name + "\"");
            else if (t.Tag == TokenType.PRAGMA_BANK) sb.Append("\tbank=" + t.Int);
            else if (t.Tag == TokenType.PRAGMA_FIXED_BANK) sb.Append("\tfixed_bank=" + t.Int);
            else if (t.Tag == TokenType.PRAGMA_FIXED_ORDER) sb.Append("\tfixed_order=" + t.Int);
            else if (t.Tag == TokenType.PRAGMA_WRAMX_BANK) sb.Append("\twramx_bank=" + t.Int);
            sb.AppendLine();
        }
        return sb.ToString();
    }

    // A compact, vibecoding-friendly AST summary.
    // Prints one line per top-level item.
    static string ShowAstSimple(Expr tree)
    {
        StringBuilder sb = new StringBuilder();
        if (tree.Match(Tag.Sequence, out Expr[] items))
        {
            sb.AppendLine("Top-level items: " + items.Length);
            foreach (var e in items)
            {
                string tag = e.GetTag();
                if (tag == Tag.Function || tag == Tag.InlineFunction)
                {
                    if (e.Match<CType, string, FieldInfo[], int, Expr>(tag, out var rt, out var name, out var fields, out var mustCheck, out var body))
                    {
                        int stmtCount = 0;
                        if (body.Match(Tag.Sequence, out Expr[] stmts)) stmtCount = stmts.Length;
                        sb.AppendLine($"{tag}\t{name}({fields.Length} args) -> {rt.Show()}\tstmts={stmtCount}\tmust_check={(mustCheck != 0)}\t@{e.Source}");
                        continue;
                    }
                }
                if (tag == Tag.FunctionDecl)
                {
                    if (e.Match<CType, string, FieldInfo[], int>(tag, out var rt, out var name, out var fields, out var mustCheck))
                    {
                        sb.AppendLine($"{tag}\t{name}({fields.Length} args) -> {rt.Show()}\tmust_check={(mustCheck != 0)}\t@{e.Source}");
                        continue;
                    }
                }
                if (tag == Tag.Variable)
                {
                    if (e.Match<CType, string, int, Expr>(tag, out var vt, out var name, out var regionTag, out var init))
                    {
                        sb.AppendLine($"{tag}\t{name}: {vt.Show()}\tregion={regionTag}\t@{e.Source}");
                        continue;
                    }
                }
                if (tag == Tag.Constant)
                {
                    if (e.Match<CType, string, Expr>(tag, out var ct, out var name, out var value))
                    {
                        sb.AppendLine($"{tag}\t{name}: {ct.Show()} = {value.Show()}\t@{e.Source}");
                        continue;
                    }
                }
                if (tag == Tag.ReadonlyData)
                {
                    if (e.Match<CType, string, int, int[]>(tag, out var dt, out var name, out var bank, out var values))
                    {
                        sb.AppendLine($"{tag}\t{name}: {dt.Show()}\tbank={bank}\tbytes={values.Length}\t@{e.Source}");
                        continue;
                    }
                }

                // Fallback
                sb.AppendLine(tag + "\t" + e.Show() + "\t@" + e.Source);
            }
        }
        else
        {
            sb.AppendLine(tree.Show());
        }
        return sb.ToString();
    }

    static string FormatAssembly(string mnemonic, AsmOperand operand)
    {
        string format = (operand.Mode == AddressMode.Implicit) ? "{0}" : "{0} {1}";
        return string.Format(format, mnemonic, operand.Show());
    }

    public static string FormatAssemblyInteger(int n)
    {
        if (n < 256) return n.ToString();
        else return string.Format("${0:X}", n);
    }

    public static void GeneralError(Severity severity, string format, params object[] args)
    {
        GeneralError(severity, Maybe.Nothing, ErrorCode.None, format, args);
    }

    [DebuggerStepThrough]
    public static void Warning(string format, params object[] args) => GeneralError(Severity.Warning, Maybe.Nothing, ErrorCode.None, format, args);
    [DebuggerStepThrough]
    public static void Warning(Maybe<FilePosition> position, string format, params object[] args) => GeneralError(Severity.Warning, position, ErrorCode.None, format, args);
    [DebuggerStepThrough]
    public static void Error(string format, params object[] args) => GeneralError(Severity.Error, Maybe.Nothing, ErrorCode.None, format, args);
    [DebuggerStepThrough]
    public static void Error(Maybe<FilePosition> position, string format, params object[] args) => GeneralError(Severity.Error, position, ErrorCode.None, format, args);
    [DebuggerStepThrough]
    public static void Panic(string format, params object[] args) => GeneralError(Severity.InternalError, Maybe.Nothing, ErrorCode.Internal, format, args);
    [DebuggerStepThrough]
    public static void Panic(Maybe<FilePosition> position, string format, params object[] args) => GeneralError(Severity.InternalError, position, ErrorCode.Internal, format, args);

    // --- New: Diagnostic codes ---
    [DebuggerStepThrough]
    public static void Warning(ErrorCode code, string format, params object[] args) => GeneralError(Severity.Warning, Maybe.Nothing, code, format, args);
    [DebuggerStepThrough]
    public static void Warning(Maybe<FilePosition> position, ErrorCode code, string format, params object[] args) => GeneralError(Severity.Warning, position, code, format, args);
    [DebuggerStepThrough]
    public static void Error(ErrorCode code, string format, params object[] args) => GeneralError(Severity.Error, Maybe.Nothing, code, format, args);
    [DebuggerStepThrough]
    public static void Error(Maybe<FilePosition> position, ErrorCode code, string format, params object[] args) => GeneralError(Severity.Error, position, code, format, args);
    [DebuggerStepThrough]
    public static void Panic(ErrorCode code, string format, params object[] args) => GeneralError(Severity.InternalError, Maybe.Nothing, code, format, args);
    [DebuggerStepThrough]
    public static void Panic(Maybe<FilePosition> position, ErrorCode code, string format, params object[] args) => GeneralError(Severity.InternalError, position, code, format, args);

    [DebuggerStepThrough]
    public static void GeneralError(Severity severity, Maybe<FilePosition> position, ErrorCode code, string format, params object[] args)
    {
        Severity effectiveSeverity = severity;
        if (severity == Severity.Warning && ShouldPromoteWarningToError(code))
        {
            effectiveSeverity = Severity.Error;
        }

        // Always show a diagnostic code (KQ0000 for uncoded diagnostics).
        string codeText = ErrorCodeText(code);

        string prefix;
        if (position.HasValue) prefix = string.Format("{0} {1} {2}: ", position.Value.ToString(), SeverityText[effectiveSeverity], codeText);
        else prefix = string.Format("{0} {1}: ", SeverityText[effectiveSeverity], codeText);

        string message = string.Format(format, args);
        string suggestion = GetFixSuggestion(code, message);
        Diagnostics.Add(new DiagnosticEntry
        {
            Severity = SeverityText[effectiveSeverity],
            Code = codeText,
            Message = message,
            Suggestion = suggestion,
            Filename = position.HasValue ? position.Value.Filename : "",
            Line = position.HasValue ? (position.Value.Line + 1) : 0,
            Column = position.HasValue ? (position.Value.Column + 1) : 0,
        });
        if (!SuppressConsoleDiagnostics && CurrentHostMode != CompilerHostMode.Api)
        {
            Console.Error.WriteLine(prefix + message);
            if (!MachineReadableOutput && severity == Severity.Warning && effectiveSeverity == Severity.Error)
            {
                Console.Error.WriteLine("  hint: strict mode treats this lint warning as an error (use --permissive to keep warnings)");
            }

            // Rich diagnostics: show the source line and caret when a file position is available.
            if (!MachineReadableOutput && position.HasValue)
            {
                TryWriteSourceContext(position.Value);
            }
            if (!MachineReadableOutput && !string.IsNullOrEmpty(suggestion))
            {
                Console.Error.WriteLine("  hint: " + suggestion);
            }
        }

        if (effectiveSeverity == Severity.Warning)
        {
            WarningCount++;
        }
        else if (effectiveSeverity == Severity.Error)
        {
            ErrorCount++;
            // Keep going to collect multiple diagnostics.
            // If errors explode, abort to avoid infinite cascades.
            if (ErrorCount >= MaxErrors)
            {
                if (CurrentHostMode != CompilerHostMode.Api)
                    Console.Error.WriteLine(string.Format("fatal KQ0000: too many errors (>{0})", MaxErrors));
                Exit(1);
            }
        }
        else if (severity == Severity.InternalError)
        {
            Exit(2);
        }
    }

    static string ErrorCodeText(ErrorCode code)
    {
        int n = (int)code;
        if (n < 0) n = 0;
        // KQ0000 ... KQ9999
        if (n > 9999) n = 9999;
        return string.Format("KQ{0:0000}", n);
    }

    static bool ShouldPromoteWarningToError(ErrorCode code)
    {
        if (!StrictDiagnostics) return false;
        int n = (int)code;
        return n >= 2400 && n < 2500;
    }

    static string GetFixSuggestion(ErrorCode code, string message)
    {
        switch (code)
        {
            case ErrorCode.ExpectedToken:
                return "Check matching tokens (; ) ] }) and ensure the previous expression/declarator is properly closed.";
            case ErrorCode.ExpectedType:
                return "A type is required here. Add a valid type specifier such as u8/u16/s8/s16/struct/enum.";
            case ErrorCode.ParseError:
                return "Check the previous line for missing brackets, commas, or semicolons.";
            case ErrorCode.ConstAssign:
            case ErrorCode.ConstModify:
                return "You are modifying a const-qualified object. Write to a non-const variable or revise qualifiers.";
            case ErrorCode.StaticAssertFailed:
                return "Revisit the static_assert condition and the constants it depends on.";
            case ErrorCode.RangeViolation:
                return "Adjust the assigned value or the __range declaration so the value stays in range.";
            case ErrorCode.RangeIndexOob:
                return "Align array length and possible index range; clamp the index if needed.";
            case ErrorCode.SwitchImplicitFallthrough:
                return "If fallthrough is intentional, add 'fallthrough;' explicitly.";
            case ErrorCode.SwitchFallthroughUsage:
                return "Use 'fallthrough;' only as the last statement in a case block.";
            case ErrorCode.UnreachableCode:
                return "Remove statements after return/goto/break/continue or split control flow.";
            case ErrorCode.UnusedSymbol:
                return "The symbol is unused. Remove it or add a real use-site.";
            case ErrorCode.ImplicitNarrowing:
                return "If narrowing is intentional, add an explicit cast.";
            case ErrorCode.PointerArithmeticDanger:
                return "Review pointer-offset type/range and add explicit casts if intentional.";
            case ErrorCode.ExternUndefined:
                return "Add a matching definition for this extern declaration.";
            case ErrorCode.ExternTypeMismatch:
                return "Make the extern declaration type match the actual definition type.";
            case ErrorCode.ExternNotSupported:
                return "For const-scalar address-taking, consider enabling -Zconst-scalar-in-rom.";
            case ErrorCode.InvalidWramXBank:
                return "Use WRAMX bank numbers in the range 1..7.";
            case ErrorCode.BankedWramRequiresCgbOnly:
                return "Build with #pragma rom_cgb cgb_only or --cgb=cgb_only before using banked WRAM/SVBK.";
            case ErrorCode.BankedWramLocalNotAllowed:
                return "Move this declaration to file scope; MVP banked WRAM is only for globals/file-statics.";
            case ErrorCode.WramXBankOverflow:
                return "Reduce the allocation size or move data into a different WRAMX bank.";
            case ErrorCode.ManualSvbkRequired:
                return "Save SVBK, switch to the symbol's bank, use the data, then restore the previous SVBK.";
            case ErrorCode.InvalidStackTop:
                return "Use a 16-bit address inside the selected stack window, for example --stack-top=0xCFFF or --stack-top=$DFFF.";
            case ErrorCode.InvalidStackReserve:
                return "Choose a reserve size that fits between the selected stack top and the bottom of the chosen WRAM window.";
            case ErrorCode.StackTopBankMismatch:
                return "Keep --stack-top inside the WRAM window selected by --stack-bank.";
            case ErrorCode.StackReservedOverlap:
                return "Move the fixed allocation or lower its address so it does not overlap the reserved stack range.";
            default:
                break;
        }

        if (!string.IsNullOrEmpty(message))
        {
            if (message.IndexOf("unknown option", StringComparison.OrdinalIgnoreCase) >= 0)
                return "Use --help to check valid option names.";
            if (message.IndexOf("division by zero", StringComparison.OrdinalIgnoreCase) >= 0)
                return "Fix the constant expression so it does not divide by zero.";
        }

        return "";
    }

    static void TryWriteSourceContext(FilePosition pos)
    {
        try
        {
            if (string.IsNullOrEmpty(pos.Filename) || pos.Filename == "<unknown>") return;

            if (!SourceCache.TryGetValue(pos.Filename, out var lines))
            {
                if (!File.Exists(pos.Filename)) return;
                lines = IoUtil.ReadAllLinesUtf8(pos.Filename);
                SourceCache[pos.Filename] = lines;
            }

            if (pos.Line < 0 || pos.Line >= lines.Length) return;

            string line = lines[pos.Line] ?? "";
            // Avoid huge output for long lines.
            const int MaxPreview = 200;
            string shown = (line.Length > MaxPreview) ? line.Substring(0, MaxPreview) + "…" : line;

            Console.Error.WriteLine("  " + shown);

            int col = pos.Column;
            if (col < 0) col = 0;
            if (col > shown.Length) col = shown.Length;

            // Expand tabs to keep caret alignment reasonable.
            string prefixText = shown.Substring(0, col);
            int visualCols = 0;
            foreach (char c in prefixText)
            {
                if (c == '\t') visualCols += 4;
                else visualCols += 1;
            }
            Console.Error.WriteLine("  " + new string(' ', visualCols) + "^");
        }
        catch
        {
            // Ignore context printing failures; main error is already printed.
        }
    }


    // Token context printing: show surrounding tokens to make parser errors easier to understand.
    // Callers provide up to a few previous-consumed tokens and the remaining input tokens.
    public static void TryWriteTokenContext(IEnumerable<Token> prevTokens, IReadOnlyList<Token> remainingTokens, int prevCount = 3, int nextCount = 3)
    {
        try
        {
            if (SuppressConsoleDiagnostics || CurrentHostMode == CompilerHostMode.Api || MachineReadableOutput) return;
            if (prevTokens == null || remainingTokens == null) return;

            // Take last prevCount tokens.
            List<Token> prev = prevTokens as List<Token>;
            if (prev == null) prev = new List<Token>(prevTokens);
            if (prev.Count > prevCount) prev = prev.GetRange(prev.Count - prevCount, prevCount);

            // Take nextCount tokens from remaining.
            int take = Math.Min(nextCount, remainingTokens.Count);
            List<Token> next = new List<Token>();
            for (int i = 0; i < take; i++) next.Add(remainingTokens[i]);

            // Build a compact preview.
            StringBuilder sb = new StringBuilder();
            sb.Append("  tokens: ");

            for (int i = 0; i < prev.Count; i++)
            {
                sb.Append(prev[i].Show());
                sb.Append(' ');
            }

            // Current token is remainingTokens[0] if present.
            if (remainingTokens.Count > 0)
            {
                sb.Append('[');
                sb.Append(remainingTokens[0].Show());
                sb.Append(']');
                sb.Append(' ');
            }

            for (int i = 1; i < next.Count; i++)
            {
                sb.Append(next[i].Show());
                sb.Append(' ');
            }

            Console.Error.WriteLine(sb.ToString().TrimEnd());
        }
        catch
        {
            // Best-effort only
        }
    }


    [DebuggerStepThrough]
    internal static void Exit(int code)
    {
        TryWriteDiagnosticJson(code);
        TryWritePathManifest(code);

        if (code != 0 && _autoMinimizeOnFail && !_autoMinimizeTriggered && !Minimizer.IsMinimizeRequested(_originalArgs))
        {
            _autoMinimizeTriggered = true;
            TryRunAutoMinimize();
        }

        if (code != 0 && EmitReproPackageOnFail)
        {
            TryWriteFailureReproPackage(code);
        }

        if (code != 0)
        {
            if (AttachDebuggerOnError) Debugger.Launch();
            if (Debugger.IsAttached) Debugger.Break();
        }
        if (CurrentHostMode == CompilerHostMode.Api)
            throw new ControlledCompilerExit(code);
        Environment.Exit(code);
    }

    public static void SetLastCompilationDependencies(IEnumerable<string> deps)
    {
        _lastCompilationDependencies.Clear();
        if (deps == null) return;
        foreach (var d in deps)
        {
            if (string.IsNullOrWhiteSpace(d)) continue;
            _lastCompilationDependencies.Add(d);
        }
    }

    static void AddIncludeDirectory(string path)
    {
        if (string.IsNullOrWhiteSpace(path)) return;
        try
        {
            string full = Path.GetFullPath(path.Trim());
            if (!IncludeDirectories.Any(x => string.Equals(x, full, StringComparison.OrdinalIgnoreCase)))
                IncludeDirectories.Add(full);
        }
        catch
        {
            Warning("warning: invalid include directory ignored: " + path);
        }
    }

    static void ApplyProfilePreset(string profileName)
    {
        string p = (profileName ?? "").Trim().ToLowerInvariant();
        if (p == "dev")
        {
            EnableDebugOutput = true;
            DisableDisasm = false;
            OptLevel = 0;
            RstDisable = true;
            EnableIncrementalCache = true;
            return;
        }
        if (p == "release")
        {
            EnableDebugOutput = false;
            DisableDisasm = true;
            OptLevel = 1;
            RstDisable = false;
            EnableIncrementalCache = true;
            return;
        }
        if (p == "test")
        {
            EnableDebugOutput = true;
            DisableDisasm = true;
            OptLevel = 0;
            RstDisable = true;
            EnableIncrementalCache = false;
            return;
        }
        Error("error: --profile must be dev|release|test");
    }

    static bool CanUseBuildCacheForThisRun()
    {
        if (TraceEnabled) return false;
        if (EmitVarList) return false;
        if (EnableDebugOutput) return false;
        if (EmitBankSimReport || EmitFarcallSuggestionReport || EmitCrossBankCallReport ||
            EmitAbiVerifyReport || EmitAbiDiffReport || EmitRstApplyReport || EmitOptDiffReport ||
            EmitFunctionSizeReport || EmitHotspotReport || EnableReproCheck ||
            EmitCgbConsistencyReport || EmitCgbSymbolVerifyReport)
            return false;
        return true;
    }

    static bool TryRestoreBuildCache(List<string> sourceFilenames, string outputFilename, out string cacheKey)
    {
        cacheKey = ComputeBuildCacheKey(sourceFilenames);
        if (string.IsNullOrEmpty(cacheKey)) return false;

        string cacheDir = Path.Combine(Environment.CurrentDirectory, ".kitaqgb_cache", cacheKey);
        string cachedGb = Path.Combine(cacheDir, "out.gb");
        if (!File.Exists(cachedGb)) return false;

        string cachedMap = Path.Combine(cacheDir, "out.map");
        string cachedDbg = Path.Combine(cacheDir, "out.dbg");
        string cachedDbc = Path.Combine(cacheDir, "out.dbc");
        string cachedBanks = Path.Combine(cacheDir, "out.banks.txt");
        string cachedFuncSizes = Path.Combine(cacheDir, "out.funcsizes.txt");
        if (!File.Exists(cachedMap) || !File.Exists(cachedDbg) || !File.Exists(cachedDbc) || !File.Exists(cachedBanks) || !File.Exists(cachedFuncSizes)) return false;

        try
        {
            IoUtil.CopyFileRobust(cachedGb, outputFilename, true);
            IoUtil.CopyFileRobust(cachedMap, Path.ChangeExtension(outputFilename, ".map"), true);
            IoUtil.CopyFileRobust(cachedDbg, Path.ChangeExtension(outputFilename, ".dbg"), true);
            IoUtil.CopyFileRobust(cachedDbc, Path.ChangeExtension(outputFilename, ".dbc"), true);
            IoUtil.CopyFileRobust(cachedBanks, Path.ChangeExtension(outputFilename, ".banks.txt"), true);
            IoUtil.CopyFileRobust(cachedFuncSizes, Path.ChangeExtension(outputFilename, ".funcsizes.txt"), true);
            if (EmitDependenciesList)
            {
                string cachedDeps = Path.Combine(cacheDir, "out.deps.txt");
                if (File.Exists(cachedDeps))
                    IoUtil.CopyFileRobust(cachedDeps, ResolveDepsOutputPath(outputFilename), true);
            }
            return true;
        }
        catch (Exception ex)
        {
            Warning("warning: cache restore skipped: " + ex.Message);
            return false;
        }
    }

    static void TrySaveBuildCache(string cacheKey, string outputFilename)
    {
        if (string.IsNullOrEmpty(cacheKey)) return;
        string cacheDir = Path.Combine(Environment.CurrentDirectory, ".kitaqgb_cache", cacheKey);
        Directory.CreateDirectory(cacheDir);
        try
        {
            IoUtil.CopyFileRobust(outputFilename, Path.Combine(cacheDir, "out.gb"), true);
            IoUtil.CopyFileRobust(Path.ChangeExtension(outputFilename, ".map"), Path.Combine(cacheDir, "out.map"), true);
            IoUtil.CopyFileRobust(Path.ChangeExtension(outputFilename, ".dbg"), Path.Combine(cacheDir, "out.dbg"), true);
            IoUtil.CopyFileRobust(Path.ChangeExtension(outputFilename, ".dbc"), Path.Combine(cacheDir, "out.dbc"), true);
            IoUtil.CopyFileRobust(Path.ChangeExtension(outputFilename, ".banks.txt"), Path.Combine(cacheDir, "out.banks.txt"), true);
            IoUtil.CopyFileRobust(Path.ChangeExtension(outputFilename, ".funcsizes.txt"), Path.Combine(cacheDir, "out.funcsizes.txt"), true);
            string depsPath = ResolveDepsOutputPath(outputFilename);
            if (File.Exists(depsPath))
                IoUtil.CopyFileRobust(depsPath, Path.Combine(cacheDir, "out.deps.txt"), true);
        }
        catch (Exception ex)
        {
            Warning("warning: cache save skipped: " + ex.Message);
        }
    }

    static string ResolveDepsOutputPath(string outputFilename)
    {
        if (!string.IsNullOrWhiteSpace(DependenciesListPath)) return DependenciesListPath;
        return Path.ChangeExtension(outputFilename, ".deps.txt");
    }

    static void TryWriteDependenciesList(List<string> sourceFilenames, string outputFilename)
    {
        if (!EmitDependenciesList) return;
        try
        {
            string path = ResolveDepsOutputPath(outputFilename);
            var deps = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            foreach (var d in LastCompilationDependencies ?? Array.Empty<string>())
            {
                if (!string.IsNullOrWhiteSpace(d)) deps.Add(Path.GetFullPath(d));
            }
            foreach (var s in sourceFilenames ?? new List<string>())
            {
                if (!string.IsNullOrWhiteSpace(s)) deps.Add(Path.GetFullPath(s));
            }

            var lines = new List<string>();
            lines.Add("# KITAQGB dependency list");
            lines.Add("# generated_utc=" + DateTime.UtcNow.ToString("O"));
            foreach (var d in deps.OrderBy(x => x, StringComparer.OrdinalIgnoreCase))
                lines.Add(d);
            path = IoUtil.WriteAllLinesUtf8Robust(path, lines, allowAlternatePath: true);
            RememberArtifactPath("deps_list", path);
        }
        catch (Exception ex)
        {
            Warning("warning: failed to write deps list: " + ex.Message);
        }
    }

    static string ComputeBuildCacheKey(List<string> sourceFilenames)
    {
        try
        {
            var files = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            foreach (var s in sourceFilenames ?? new List<string>())
            {
                string full = Path.GetFullPath(s);
                if (File.Exists(full)) files.Add(full);

                string dir = Path.GetDirectoryName(full) ?? "";
                if (!string.IsNullOrWhiteSpace(dir) && Directory.Exists(dir))
                {
                    foreach (var h in Directory.GetFiles(dir, "*.h", SearchOption.AllDirectories))
                        files.Add(h);
                }
            }
            foreach (var d in IncludeDirectories)
            {
                if (!Directory.Exists(d)) continue;
                foreach (var h in Directory.GetFiles(d, "*.h", SearchOption.AllDirectories))
                    files.Add(h);
            }

            string exePath = Process.GetCurrentProcess().MainModule.FileName;
            string exeStamp = "";
            try
            {
                var fi = new FileInfo(exePath);
                using (var sha = SHA256.Create())
                using (var fs = File.OpenRead(exePath))
                {
                    var hash = sha.ComputeHash(fs);
                    exeStamp = fi.Length + "|" + fi.LastWriteTimeUtc.Ticks + "|" +
                               string.Concat(hash.Select(b => b.ToString("x2")));
                }
            }
            catch { }

            var sb = new StringBuilder();
            sb.AppendLine("kitaqgb_cache_v2");
            sb.AppendLine(exeStamp);
            sb.AppendLine("opt=" + OptLevel);
            sb.AppendLine("abi=" + Abi);
            sb.AppendLine("diag_mode=" + DiagnosticsMode);
            sb.AppendLine("rst_disable=" + RstDisable);
            sb.AppendLine("rst_use_38=" + RstUse38);
            sb.AppendLine("rst_unsafe=" + RstUnsafe);
            sb.AppendLine("rst_speed_safe=" + RstSpeedSafe);
            sb.AppendLine("rom_title=" + (RomHeader.Title ?? ""));
            sb.AppendLine("cgb=" + (RomHeader.CgbFlag.HasValue ? RomHeader.CgbFlag.Value.ToString() : ""));
            sb.AppendLine("cart=" + (RomHeader.CartType.HasValue ? RomHeader.CartType.Value.ToString() : ""));
            sb.AppendLine("romsize=" + (RomHeader.RomSizeCode.HasValue ? RomHeader.RomSizeCode.Value.ToString() : ""));
            sb.AppendLine("ramsize=" + (RomHeader.RamSizeCode.HasValue ? RomHeader.RamSizeCode.Value.ToString() : ""));
            sb.AppendLine("stack_bank=" + GetStackBankCliText(StackBank));
            sb.AppendLine("stack_top=" + EffectiveStackTop);
            sb.AppendLine("stack_reserve=" + StackReserve);
            sb.AppendLine("stack_auto_limit=" + EffectiveStackAutoLimit);

            foreach (string f in files.OrderBy(x => x, StringComparer.OrdinalIgnoreCase))
            {
                try
                {
                    var fi = new FileInfo(f);
                    sb.AppendLine(f + "|" + fi.Length + "|" + fi.LastWriteTimeUtc.Ticks);
                }
                catch
                {
                    sb.AppendLine(f + "|missing");
                }
            }

            using (var sha = SHA256.Create())
            {
                var data = IoUtil.Utf8NoBom.GetBytes(sb.ToString());
                var hash = sha.ComputeHash(data);
                return string.Concat(hash.Select(b => b.ToString("x2")));
            }
        }
        catch
        {
            return "";
        }
    }

    sealed class BankSimulationResult
    {
        public int[] BankMaxPc = new int[0];
        public int RomSizeBytes;
        public int UsedBytes;
        public List<FunctionSizeInfo> FunctionSizes = new List<FunctionSizeInfo>();
    }

    sealed class HotspotJsonRow
    {
        public string Name;
        public int Size;
        public int Calls;
        public long Score;
    }

    static void RunAnalysisReports(List<string> sourceFilenames, IReadOnlyList<Expr> assembly, string outputFilename, RomHeaderOptions effectiveRomHeader)
    {
        var codegen = CodeGenerator.LastReport ?? new CodegenAnalysisReport();
        var opt = Optimizer.LastReport ?? new OptimizerAnalysisReport();
        var asm = Assembler.LastReport ?? new AssemblerAnalysisReport();

        if (EnableDebugOutput)
        {
            WriteDebugFile("stack_policy.txt", BuildStackPolicyText());
        }

        TryWriteRichDebugMetadata(outputFilename, sourceFilenames, codegen, asm);
        TryWriteAugmentedSourceMap(outputFilename, sourceFilenames, asm);
        TryWriteBuildReportJson(outputFilename, sourceFilenames, codegen, opt, asm, effectiveRomHeader);

        if (EmitBankSimReport)
        {
            string path = ResolveReportPath(BankSimReportPath, outputFilename, ".bank_sim.txt");
            var sim = SimulateBankLayout(assembly);
            path = IoUtil.WriteAllTextUtf8Robust(path, BuildBankSimReportText(sim), allowAlternatePath: true);
            WriteInfoLine("[report] bank simulation: " + path);
        }

        if (EmitFarcallSuggestionReport)
        {
            string path = ResolveReportPath(FarcallSuggestionReportPath, outputFilename, ".farcall_suggestions.txt");
            path = IoUtil.WriteAllTextUtf8Robust(path, BuildFarcallSuggestionText(codegen), allowAlternatePath: true);
            WriteInfoLine("[report] farcall suggestions: " + path);
        }

        if (EmitCrossBankCallReport)
        {
            string path = ResolveReportPath(CrossBankCallReportPath, outputFilename, ".cross_bank_calls.txt");
            path = IoUtil.WriteAllTextUtf8Robust(path, BuildCrossBankCallText(codegen), allowAlternatePath: true);
            WriteInfoLine("[report] cross-bank calls: " + path);
        }

        if (EmitAbiVerifyReport)
        {
            string path = ResolveReportPath(AbiVerifyReportPath, outputFilename, ".abi_verify.txt");
            var issues = new List<string>(codegen.AbiIssues ?? new List<string>());
            string text = BuildAbiVerifyText(codegen, issues);
            path = IoUtil.WriteAllTextUtf8Robust(path, text, allowAlternatePath: true);
            WriteInfoLine("[report] abi verify: " + path);
            if (issues.Count > 0)
            {
                Error("ABI verification failed ({0} issue(s)). See: {1}", issues.Count, path);
            }
        }

        if (EmitRstApplyReport)
        {
            string path = ResolveReportPath(RstApplyReportPath, outputFilename, ".rst_report.txt");
            path = IoUtil.WriteAllTextUtf8Robust(path, BuildRstApplyText(codegen, opt), allowAlternatePath: true);
            WriteInfoLine("[report] rst apply: " + path);
        }

        if (EmitOptDiffReport)
        {
            string path = ResolveReportPath(OptDiffReportPath, outputFilename, ".opt_diff.txt");
            path = IoUtil.WriteAllTextUtf8Robust(path, BuildOptimizerDiffText(opt), allowAlternatePath: true);
            WriteInfoLine("[report] optimizer diff: " + path);
        }

        if (EmitFunctionSizeReport)
        {
            string path = ResolveReportPath(FunctionSizeReportPath, outputFilename, ".funcsizes.txt");
            path = IoUtil.WriteAllTextUtf8Robust(path, BuildFunctionSizeReportText(asm), allowAlternatePath: true);
            WriteInfoLine("[report] function sizes: " + path);
        }

        if (EmitHotspotReport)
        {
            string path = ResolveReportPath(HotspotReportPath, outputFilename, ".hotspots.txt");
            path = IoUtil.WriteAllTextUtf8Robust(path, BuildHotspotReportText(codegen, asm), allowAlternatePath: true);
            WriteInfoLine("[report] hotspots: " + path);
        }

        if (EmitCgbConsistencyReport)
        {
            string path = ResolveReportPath(CgbConsistencyReportPath, outputFilename, ".cgb_consistency.txt");
            var result = AnalyzeCgbConsistency(assembly, codegen, outputFilename, effectiveRomHeader);
            path = IoUtil.WriteAllTextUtf8Robust(path, BuildCgbConsistencyText(result), allowAlternatePath: true);
            WriteInfoLine("[report] cgb consistency: " + path);
            if (!result.Pass)
            {
                Error("CGB consistency check failed ({0} issue(s)). See: {1}", result.Issues.Count, path);
            }
        }

        if (EmitCgbSymbolVerifyReport)
        {
            string path = ResolveReportPath(CgbSymbolVerifyReportPath, outputFilename, ".cgb_symbols.txt");
            var result = AnalyzeCgbSymbolVerification(assembly, Path.ChangeExtension(outputFilename, ".map"));
            path = IoUtil.WriteAllTextUtf8Robust(path, BuildCgbSymbolVerifyText(result), allowAlternatePath: true);
            WriteInfoLine("[report] cgb symbols: " + path);
            if (!result.Pass)
            {
                Error("CGB symbol verification failed ({0} missing symbol(s)). See: {1}", result.Missing.Count, path);
            }
        }

        if (EmitAbiDiffReport)
        {
            string path = ResolveReportPath(AbiDiffReportPath, outputFilename, ".abi_diff.txt");
            TryGenerateAbiDiffReport(path);
        }

        if (EnableReproCheck)
        {
            string path = ResolveReportPath(ReproCheckReportPath, outputFilename, ".repro_check.txt");
            TryRunReproCheck(outputFilename, path);
        }
    }

    static void TryWriteRichDebugMetadata(
        string outputFilename,
        List<string> sourceFilenames,
        CodegenAnalysisReport codegen,
        AssemblerAnalysisReport asm)
    {
        try
        {
            string path = Path.ChangeExtension(outputFilename, ".dbg2.json");
            path = IoUtil.WriteAllTextUtf8Robust(
                path,
                BuildRichDebugMetadataJson(outputFilename, sourceFilenames, codegen, asm),
                allowAlternatePath: true);
            RememberArtifactPath("dbg2_json", path);
            WriteInfoLine("[report] debug metadata: " + path);
        }
        catch (Exception ex)
        {
            Warning("Failed to write debug metadata JSON: " + ex.Message);
        }
    }

    static void TryWriteBuildReportJson(
        string outputFilename,
        List<string> sourceFilenames,
        CodegenAnalysisReport codegen,
        OptimizerAnalysisReport opt,
        AssemblerAnalysisReport asm,
        RomHeaderOptions effectiveRomHeader)
    {
        try
        {
            string path = Path.ChangeExtension(outputFilename, ".build_report.json");
            path = IoUtil.WriteAllTextUtf8Robust(
                path,
                BuildBuildReportJson(outputFilename, sourceFilenames, codegen, opt, asm, effectiveRomHeader),
                allowAlternatePath: true);
            RememberArtifactPath("build_report_json", path);
            WriteInfoLine("[report] build report: " + path);
        }
        catch (Exception ex)
        {
            Warning("Failed to write build report JSON: " + ex.Message);
        }
    }

    static void TryWriteAugmentedSourceMap(string outputFilename, List<string> sourceFilenames, AssemblerAnalysisReport asm)
    {
        try
        {
            string path = Path.ChangeExtension(outputFilename, ".source_map.txt");
            path = IoUtil.WriteAllTextUtf8Robust(path, BuildAugmentedSourceMapText(sourceFilenames, asm), allowAlternatePath: true);
            RememberArtifactPath("source_map", path);
        }
        catch (Exception ex)
        {
            Warning("Failed to write augmented source map: " + ex.Message);
        }
    }

    static string BuildAugmentedSourceMapText(List<string> sourceFilenames, AssemblerAnalysisReport asm)
    {
        var fallbackSourceByFunction = new Dictionary<string, Tuple<string, int>>(StringComparer.Ordinal);
        foreach (var sourcePath in sourceFilenames ?? new List<string>())
        {
            foreach (var fn in ParseSourceFunctions(sourcePath))
            {
                if (string.IsNullOrEmpty(fn.Name) || fn.StartLine <= 0) continue;
                if (!fallbackSourceByFunction.ContainsKey(fn.Name))
                    fallbackSourceByFunction[fn.Name] = Tuple.Create(sourcePath, fn.StartLine);
            }
        }

        var lines = new List<string>();
        lines.Add("# KITAQGB source map");
        lines.Add("# bank:addr path:line:column symbol=... section=...");

        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (var loc in asm.SourceLocations ?? new List<SourceLocationMetadataInfo>())
        {
            if (string.IsNullOrEmpty(loc.Path) || loc.Line <= 0) continue;
            string line = string.Format(
                "{0:X2}:{1:X4} {2}:{3}{4}{5}{6}",
                loc.Bank & 0xFFFF,
                loc.Address & 0xFFFF,
                loc.Path,
                loc.Line,
                loc.Column.HasValue ? ":" + loc.Column.Value.ToString() : "",
                string.IsNullOrEmpty(loc.Symbol) ? "" : " symbol=" + loc.Symbol,
                string.IsNullOrEmpty(loc.Section) ? "" : " section=" + loc.Section);
            if (seen.Add(line)) lines.Add(line);
        }

        var functionsWithSource = new HashSet<string>(
            (asm.SourceLocations ?? new List<SourceLocationMetadataInfo>())
                .Where(loc => !string.IsNullOrEmpty(loc.Symbol))
                .Select(loc => loc.Symbol),
            StringComparer.Ordinal);

        foreach (var func in asm.FunctionSizes ?? new List<FunctionSizeInfo>())
        {
            if (string.IsNullOrEmpty(func.Name) || functionsWithSource.Contains(func.Name)) continue;
            Tuple<string, int> sourceInfo;
            if (!fallbackSourceByFunction.TryGetValue(func.Name, out sourceInfo)) continue;

            string line = string.Format(
                "{0:X2}:{1:X4} {2}:{3} symbol={4}{5}",
                func.Bank & 0xFFFF,
                func.CpuAddress & 0xFFFF,
                sourceInfo.Item1,
                sourceInfo.Item2,
                func.Name,
                string.IsNullOrEmpty(func.Section) ? "" : " section=" + func.Section);
            if (seen.Add(line)) lines.Add(line);
        }

        return string.Join(Environment.NewLine, lines) + Environment.NewLine;
    }

    static string BuildRichDebugMetadataJson(
        string outputFilename,
        List<string> sourceFilenames,
        CodegenAnalysisReport codegen,
        AssemblerAnalysisReport asm)
    {
        var abiByName = (codegen.Functions ?? new List<FunctionAbiInfo>())
            .Where(f => !string.IsNullOrEmpty(f.Name))
            .GroupBy(f => f.Name, StringComparer.Ordinal)
            .ToDictionary(g => g.Key, g => g.First(), StringComparer.Ordinal);
        var functionByName = (asm.FunctionSizes ?? new List<FunctionSizeInfo>())
            .Where(f => !string.IsNullOrEmpty(f.Name))
            .GroupBy(f => f.Name, StringComparer.Ordinal)
            .ToDictionary(g => g.Key, g => g.First(), StringComparer.Ordinal);
        var fallbackSourceByFunction = new Dictionary<string, Tuple<string, int>>(StringComparer.Ordinal);
        foreach (var sourcePath in sourceFilenames ?? new List<string>())
        {
            foreach (var fn in ParseSourceFunctions(sourcePath))
            {
                if (string.IsNullOrEmpty(fn.Name) || fn.StartLine <= 0) continue;
                if (!fallbackSourceByFunction.ContainsKey(fn.Name))
                    fallbackSourceByFunction[fn.Name] = Tuple.Create(sourcePath, fn.StartLine);
            }
        }

        var incomingCalls = new Dictionary<string, int>(StringComparer.Ordinal);
        var outgoingCalls = new Dictionary<string, int>(StringComparer.Ordinal);
        var crossBankOutgoingCalls = new Dictionary<string, int>(StringComparer.Ordinal);
        var farCalls = new Dictionary<string, int>(StringComparer.Ordinal);
        foreach (var c in codegen.Calls ?? new List<CallEdgeInfo>())
        {
            if (!string.IsNullOrEmpty(c.Callee))
            {
                int cur = 0;
                incomingCalls.TryGetValue(c.Callee, out cur);
                incomingCalls[c.Callee] = cur + Math.Max(0, c.Count);
            }
            if (!string.IsNullOrEmpty(c.Caller))
            {
                int cur = 0;
                outgoingCalls.TryGetValue(c.Caller, out cur);
                outgoingCalls[c.Caller] = cur + Math.Max(0, c.Count);

                if (c.CallerBank >= 0 && c.CalleeBank >= 0 && c.CallerBank != c.CalleeBank)
                {
                    int cross = 0;
                    crossBankOutgoingCalls.TryGetValue(c.Caller, out cross);
                    crossBankOutgoingCalls[c.Caller] = cross + Math.Max(0, c.Count);
                }
                if (c.ViaFarcall)
                {
                    int far = 0;
                    farCalls.TryGetValue(c.Caller, out far);
                    farCalls[c.Caller] = far + Math.Max(0, c.Count);
                }
            }
        }

        var sb = new StringBuilder();
        sb.Append("{");
        sb.Append("\"schema_version\":\"1\",");
        sb.Append("\"rom\":{");
        sb.Append("\"path\":\"").Append(JsonEscape(outputFilename ?? "")).Append("\",");
        sb.Append("\"sha256\":\"").Append(JsonEscape(BuildReportUtil.ComputeSha256HexOfFile(outputFilename))).Append("\",");
        sb.Append("\"size_bytes\":").Append(asm == null ? 0 : asm.RomSizeBytes);
        sb.Append("},");
        AppendStackPolicyJson(sb);
        sb.Append(",");

        sb.Append("\"symbols\":[");
        bool first = true;
        foreach (var symbol in asm.Symbols ?? new List<SymbolMetadataInfo>())
        {
            if (!first) sb.Append(",");
            first = false;
            int symbolEnd = Math.Max(symbol.Start + 1, symbol.End);
            if (functionByName.TryGetValue(symbol.Name ?? "", out FunctionSizeInfo functionInfo))
            {
                symbolEnd = functionInfo.SizeBytes > 0
                    ? (BuildReportUtil.CpuAddrFromFileOffset(Math.Max(functionInfo.EndFileOffset - 1, functionInfo.StartFileOffset)) + 1)
                    : symbolEnd;
            }
            sb.Append("{");
            sb.Append("\"name\":\"").Append(JsonEscape(symbol.Name ?? "")).Append("\",");
            sb.Append("\"bank\":").Append(symbol.Bank).Append(",");
            sb.Append("\"start\":").Append(symbol.Start & 0xFFFF).Append(",");
            sb.Append("\"end\":").Append(symbolEnd).Append(",");
            sb.Append("\"kind\":\"").Append(JsonEscape(symbol.Kind ?? "")).Append("\",");
            sb.Append("\"region\":\"").Append(JsonEscape(symbol.Region ?? "")).Append("\"");
            if (!string.IsNullOrEmpty(symbol.Section))
                sb.Append(",\"section\":\"").Append(JsonEscape(symbol.Section)).Append("\"");
            sb.Append("}");
        }
        sb.Append("],");

        sb.Append("\"source_locations\":[");
        first = true;
        var functionsWithSource = new HashSet<string>(StringComparer.Ordinal);
        foreach (var loc in asm.SourceLocations ?? new List<SourceLocationMetadataInfo>())
        {
            if (!first) sb.Append(",");
            first = false;
            sb.Append("{");
            sb.Append("\"bank\":").Append(loc.Bank).Append(",");
            sb.Append("\"addr\":").Append(loc.Address & 0xFFFF).Append(",");
            sb.Append("\"path\":\"").Append(JsonEscape(loc.Path ?? "")).Append("\",");
            sb.Append("\"line\":").Append(loc.Line);
            if (loc.Column.HasValue) sb.Append(",\"column\":").Append(loc.Column.Value);
            if (!string.IsNullOrEmpty(loc.Symbol))
            {
                sb.Append(",\"symbol\":\"").Append(JsonEscape(loc.Symbol)).Append("\"");
                functionsWithSource.Add(loc.Symbol);
            }
            if (!string.IsNullOrEmpty(loc.Section))
                sb.Append(",\"section\":\"").Append(JsonEscape(loc.Section)).Append("\"");
            sb.Append("}");
        }
        foreach (var func in asm.FunctionSizes ?? new List<FunctionSizeInfo>())
        {
            if (string.IsNullOrEmpty(func.Name) || functionsWithSource.Contains(func.Name)) continue;
            Tuple<string, int> sourceInfo;
            if (!fallbackSourceByFunction.TryGetValue(func.Name, out sourceInfo)) continue;
            if (!first) sb.Append(",");
            first = false;
            sb.Append("{");
            sb.Append("\"bank\":").Append(func.Bank).Append(",");
            sb.Append("\"addr\":").Append(func.CpuAddress & 0xFFFF).Append(",");
            sb.Append("\"path\":\"").Append(JsonEscape(sourceInfo.Item1)).Append("\",");
            sb.Append("\"line\":").Append(sourceInfo.Item2).Append(",");
            sb.Append("\"symbol\":\"").Append(JsonEscape(func.Name)).Append("\"");
            if (!string.IsNullOrEmpty(func.Section))
                sb.Append(",\"section\":\"").Append(JsonEscape(func.Section)).Append("\"");
            sb.Append("}");
        }
        sb.Append("],");

        sb.Append("\"functions\":[");
        first = true;
        foreach (var func in asm.FunctionSizes ?? new List<FunctionSizeInfo>())
        {
            if (!first) sb.Append(",");
            first = false;
            abiByName.TryGetValue(func.Name ?? "", out FunctionAbiInfo abi);
            int start = func.CpuAddress & 0xFFFF;
            int end = func.SizeBytes > 0
                ? (BuildReportUtil.CpuAddrFromFileOffset(Math.Max(func.EndFileOffset - 1, func.StartFileOffset)) + 1)
                : start;

            sb.Append("{");
            sb.Append("\"name\":\"").Append(JsonEscape(func.Name ?? "")).Append("\",");
            sb.Append("\"bank\":").Append(func.Bank).Append(",");
            sb.Append("\"start\":").Append(start).Append(",");
            sb.Append("\"end\":").Append(end).Append(",");
            sb.Append("\"size_bytes\":").Append(Math.Max(0, func.SizeBytes));
            if (!string.IsNullOrEmpty(func.Section))
                sb.Append(",\"section\":\"").Append(JsonEscape(func.Section)).Append("\"");
            if (abi != null)
            {
                sb.Append(",\"is_stack_call\":").Append(abi.IsStackCall ? "true" : "false");
                sb.Append(",\"is_fast_call\":").Append(abi.IsFastCall ? "true" : "false");
                sb.Append(",\"has_fixed_bank\":").Append(abi.HasFixedBank ? "true" : "false");
                sb.Append(",\"has_fixed_order\":").Append(abi.HasFixedOrder ? "true" : "false");
                sb.Append(",\"return_size\":").Append(Math.Max(0, abi.ReturnSize));
                sb.Append(",\"param_sizes\":[");
                for (int i = 0; i < (abi.ParamSizes == null ? 0 : abi.ParamSizes.Length); i++)
                {
                    if (i != 0) sb.Append(",");
                    sb.Append(Math.Max(0, abi.ParamSizes[i]));
                }
                sb.Append("]");
            }
            sb.Append("}");
        }
        sb.Append("],");

        sb.Append("\"variables\":[");
        first = true;
        foreach (var variable in asm.Variables ?? new List<VariableMetadataInfo>())
        {
            if (!first) sb.Append(",");
            first = false;
            sb.Append("{");
            sb.Append("\"name\":\"").Append(JsonEscape(variable.Name ?? "")).Append("\",");
            sb.Append("\"address\":").Append(variable.Address & 0xFFFF).Append(",");
            sb.Append("\"size\":").Append(Math.Max(0, variable.Size)).Append(",");
            sb.Append("\"region\":\"").Append(JsonEscape(variable.Region ?? "")).Append("\"");
            if (variable.Bank.HasValue) sb.Append(",\"bank\":").Append(variable.Bank.Value);
            sb.Append("}");
        }
        sb.Append("],");

        sb.Append("\"static_estimates\":[");
        first = true;
        foreach (var func in asm.FunctionSizes ?? new List<FunctionSizeInfo>())
        {
            if (!first) sb.Append(",");
            first = false;
            abiByName.TryGetValue(func.Name ?? "", out FunctionAbiInfo abi);
            int incoming = 0; incomingCalls.TryGetValue(func.Name ?? "", out incoming);
            int outgoing = 0; outgoingCalls.TryGetValue(func.Name ?? "", out outgoing);
            int cross = 0; crossBankOutgoingCalls.TryGetValue(func.Name ?? "", out cross);
            int far = 0; farCalls.TryGetValue(func.Name ?? "", out far);

            sb.Append("{");
            sb.Append("\"name\":\"").Append(JsonEscape(func.Name ?? "")).Append("\",");
            sb.Append("\"bank\":").Append(func.Bank).Append(",");
            sb.Append("\"size_bytes\":").Append(Math.Max(0, func.SizeBytes)).Append(",");
            sb.Append("\"incoming_call_count\":").Append(incoming).Append(",");
            sb.Append("\"outgoing_call_count\":").Append(outgoing).Append(",");
            sb.Append("\"cross_bank_outgoing_count\":").Append(cross).Append(",");
            sb.Append("\"far_call_count\":").Append(far);
            if (abi != null)
            {
                sb.Append(",\"is_stack_call\":").Append(abi.IsStackCall ? "true" : "false");
                sb.Append(",\"is_fast_call\":").Append(abi.IsFastCall ? "true" : "false");
                sb.Append(",\"return_size\":").Append(Math.Max(0, abi.ReturnSize));
                sb.Append(",\"param_sizes\":[");
                for (int i = 0; i < (abi.ParamSizes == null ? 0 : abi.ParamSizes.Length); i++)
                {
                    if (i != 0) sb.Append(",");
                    sb.Append(Math.Max(0, abi.ParamSizes[i]));
                }
                sb.Append("]");
            }
            sb.Append("}");
        }
        sb.Append("],");

        sb.Append("\"call_edges\":[");
        first = true;
        foreach (var call in codegen.Calls ?? new List<CallEdgeInfo>())
        {
            if (!first) sb.Append(",");
            first = false;
            sb.Append("{");
            sb.Append("\"caller\":\"").Append(JsonEscape(call.Caller ?? "")).Append("\",");
            sb.Append("\"callee\":\"").Append(JsonEscape(call.Callee ?? "")).Append("\",");
            sb.Append("\"caller_bank\":").Append(call.CallerBank).Append(",");
            sb.Append("\"callee_bank\":").Append(call.CalleeBank).Append(",");
            sb.Append("\"kind\":\"").Append(JsonEscape(call.Kind ?? "")).Append("\",");
            sb.Append("\"via_thunk\":").Append(call.ViaThunk ? "true" : "false").Append(",");
            sb.Append("\"via_farcall\":").Append(call.ViaFarcall ? "true" : "false").Append(",");
            sb.Append("\"count\":").Append(Math.Max(0, call.Count));
            if (!string.IsNullOrEmpty(call.LastSource))
                sb.Append(",\"last_source\":\"").Append(JsonEscape(call.LastSource)).Append("\"");
            sb.Append("}");
        }
        sb.Append("],");

        sb.Append("\"abi_issues\":[");
        AppendJsonStringArray(sb, codegen.AbiIssues ?? new List<string>());
        sb.Append("],");

        sb.Append("\"bank_usage\":[");
        first = true;
        for (int bank = 0; bank < (asm.BankMaxPc == null ? 0 : asm.BankMaxPc.Length); bank++)
        {
            if (!first) sb.Append(",");
            first = false;
            int start = bank * 0x4000;
            int end = (bank + 1) * 0x4000;
            int maxPc = asm.BankMaxPc[bank];
            int used = Math.Max(0, Math.Min(maxPc, end) - start);
            int free = Math.Max(0, 0x4000 - used);
            sb.Append("{");
            sb.Append("\"bank\":").Append(bank).Append(",");
            sb.Append("\"used_bytes\":").Append(used).Append(",");
            sb.Append("\"free_bytes\":").Append(free);
            sb.Append("}");
        }
        sb.Append("]");

        sb.Append("}");
        return sb.ToString();
    }

    static string BuildBuildReportJson(
        string outputFilename,
        List<string> sourceFilenames,
        CodegenAnalysisReport codegen,
        OptimizerAnalysisReport opt,
        AssemblerAnalysisReport asm,
        RomHeaderOptions effectiveRomHeader)
    {
        var hotspotRows = BuildHotspotRows(codegen, asm).Take(16).ToList();
        var crossBankRows = (codegen == null || codegen.Calls == null)
            ? new List<CallEdgeInfo>()
            : codegen.Calls
                .Where(c => c.CallerBank >= 0 && c.CalleeBank >= 0 && c.CallerBank != c.CalleeBank)
                .OrderByDescending(c => c.Count)
                .ThenBy(c => c.Caller, StringComparer.Ordinal)
                .ThenBy(c => c.Callee, StringComparer.Ordinal)
                .Take(16)
                .ToList();
        var sb = new StringBuilder();
        sb.Append("{");
        sb.Append("\"schema_version\":\"1\",");
        sb.Append("\"output_rom\":\"").Append(JsonEscape(outputFilename ?? "")).Append("\",");
        sb.Append("\"output_sha256\":\"").Append(JsonEscape(BuildReportUtil.ComputeSha256HexOfFile(outputFilename))).Append("\",");
        sb.Append("\"rom_size_bytes\":").Append(asm == null ? 0 : asm.RomSizeBytes).Append(",");
        sb.Append("\"used_bytes\":").Append(asm == null ? 0 : asm.UsedBytes).Append(",");
        sb.Append("\"abi_mode\":\"").Append(JsonEscape(Abi.ToString().ToLowerInvariant())).Append("\",");
        sb.Append("\"function_count\":").Append(asm == null || asm.FunctionSizes == null ? 0 : asm.FunctionSizes.Count).Append(",");
        sb.Append("\"call_edge_count\":").Append(codegen == null || codegen.Calls == null ? 0 : codegen.Calls.Count).Append(",");
        sb.Append("\"optimizer_pass_count\":").Append(opt == null || opt.Passes == null ? 0 : opt.Passes.Count).Append(",");
        sb.Append("\"abi_issue_count\":").Append(codegen == null || codegen.AbiIssues == null ? 0 : codegen.AbiIssues.Count).Append(",");
        sb.Append("\"cross_bank_call_count\":").Append(crossBankRows.Count).Append(",");
        sb.Append("\"source_files\":[");
        AppendJsonStringArray(sb, (sourceFilenames ?? new List<string>()).Distinct(StringComparer.Ordinal));
        sb.Append("],");
        sb.Append("\"abi_issues\":[");
        AppendJsonStringArray(sb, codegen == null ? new List<string>() : (codegen.AbiIssues ?? new List<string>()));
        sb.Append("],");
        sb.Append("\"header\":{");
        sb.Append("\"title\":\"").Append(JsonEscape((effectiveRomHeader == null || effectiveRomHeader.Title == null) ? "" : effectiveRomHeader.Title)).Append("\",");
        sb.Append("\"cgb_flag\":").Append((effectiveRomHeader != null && effectiveRomHeader.CgbFlag.HasValue) ? effectiveRomHeader.CgbFlag.Value : 0).Append(",");
        sb.Append("\"cart_type\":").Append((effectiveRomHeader != null && effectiveRomHeader.CartType.HasValue) ? effectiveRomHeader.CartType.Value : 0).Append(",");
        sb.Append("\"rom_size_code\":").Append((effectiveRomHeader != null && effectiveRomHeader.RomSizeCode.HasValue) ? effectiveRomHeader.RomSizeCode.Value : 0).Append(",");
        sb.Append("\"ram_size_code\":").Append((effectiveRomHeader != null && effectiveRomHeader.RamSizeCode.HasValue) ? effectiveRomHeader.RamSizeCode.Value : 0);
        sb.Append("},");
        AppendStackPolicyJson(sb);
        sb.Append(",");
        sb.Append("\"hotspots\":[");
        bool first = true;
        foreach (var row in hotspotRows)
        {
            if (!first) sb.Append(",");
            first = false;
            sb.Append("{");
            sb.Append("\"name\":\"").Append(JsonEscape(row.Name)).Append("\",");
            sb.Append("\"incoming_calls\":").Append(row.Calls).Append(",");
            sb.Append("\"size_bytes\":").Append(row.Size).Append(",");
            sb.Append("\"score\":").Append(row.Score);
            sb.Append("}");
        }
        sb.Append("],");
        sb.Append("\"cross_bank_edges\":[");
        first = true;
        foreach (var row in crossBankRows)
        {
            if (!first) sb.Append(",");
            first = false;
            sb.Append("{");
            sb.Append("\"caller\":\"").Append(JsonEscape(row.Caller ?? "")).Append("\",");
            sb.Append("\"callee\":\"").Append(JsonEscape(row.Callee ?? "")).Append("\",");
            sb.Append("\"caller_bank\":").Append(row.CallerBank).Append(",");
            sb.Append("\"callee_bank\":").Append(row.CalleeBank).Append(",");
            sb.Append("\"count\":").Append(Math.Max(0, row.Count)).Append(",");
            sb.Append("\"kind\":\"").Append(JsonEscape(row.Kind ?? "")).Append("\",");
            sb.Append("\"via_thunk\":").Append(row.ViaThunk ? "true" : "false").Append(",");
            sb.Append("\"via_farcall\":").Append(row.ViaFarcall ? "true" : "false");
            sb.Append("}");
        }
        sb.Append("],");
        sb.Append("\"notes\":[");
        AppendJsonStringArray(sb, new[]
        {
            "dbg2.json is emitted alongside the ROM so KOKURA can load richer source-aware metadata directly.",
            "source_map.txt is emitted alongside the ROM for lightweight line-level lookup without requiring the full JSON payload.",
            "static_estimates are currently structural estimates driven by codegen/call topology and function size, not cycle-perfect timing."
        });
        sb.Append("]");
        sb.Append("}");
        return sb.ToString();
    }

    static List<HotspotJsonRow> BuildHotspotRows(CodegenAnalysisReport codegen, AssemblerAnalysisReport asm)
    {
        var sizeByFunc = (asm.FunctionSizes ?? new List<FunctionSizeInfo>())
            .GroupBy(x => x.Name, StringComparer.Ordinal)
            .ToDictionary(g => g.Key, g => g.Max(x => x.SizeBytes), StringComparer.Ordinal);

        var incomingCalls = new Dictionary<string, int>(StringComparer.Ordinal);
        foreach (var c in codegen.Calls ?? new List<CallEdgeInfo>())
        {
            if (string.IsNullOrEmpty(c.Callee) || c.Callee.StartsWith("<", StringComparison.Ordinal)) continue;
            int cur = 0;
            incomingCalls.TryGetValue(c.Callee, out cur);
            incomingCalls[c.Callee] = cur + c.Count;
        }

        var allFuncs = new HashSet<string>(sizeByFunc.Keys, StringComparer.Ordinal);
        foreach (var k in incomingCalls.Keys) allFuncs.Add(k);

        return allFuncs
            .Select(name =>
            {
                int size = 0; sizeByFunc.TryGetValue(name, out size);
                int calls = 0; incomingCalls.TryGetValue(name, out calls);
                long score = (long)size * (long)calls;
                return new HotspotJsonRow { Name = name, Size = size, Calls = calls, Score = score };
            })
            .OrderByDescending(x => x.Score)
            .ThenByDescending(x => x.Calls)
            .ThenByDescending(x => x.Size)
            .ThenBy(x => x.Name, StringComparer.Ordinal)
            .ToList();
    }

    static void AppendJsonStringArray(StringBuilder sb, IEnumerable<string> values)
    {
        bool first = true;
        foreach (var value in values ?? Array.Empty<string>())
        {
            if (!first) sb.Append(",");
            first = false;
            sb.Append("\"").Append(JsonEscape(value ?? "")).Append("\"");
        }
    }

    static string ResolveReportPath(string requestedPath, string outputFilename, string suffix)
    {
        if (!string.IsNullOrWhiteSpace(requestedPath)) return requestedPath;
        string dir = Path.GetDirectoryName(outputFilename);
        if (string.IsNullOrWhiteSpace(dir)) dir = ".";
        string stem = Path.GetFileNameWithoutExtension(outputFilename);
        return Path.Combine(dir, stem + suffix);
    }

    static BankSimulationResult SimulateBankLayout(IReadOnlyList<Expr> assembly)
    {
        const int bankSize = 0x4000;
        const int codeStart = 0x0160;
        const int minRomSize = 0x8000;
        const int maxRomSize = 0x800000;

        int pc = codeStart;
        int maxPc = pc;
        var bankMax = new Dictionary<int, int>();
        var functionSizes = new List<FunctionSizeInfo>();
        string currentFunctionName = null;
        int currentFunctionStart = 0;
        int currentFunctionBytes = 0;

        Action<int> updateBankMax = (value) =>
        {
            if (value < 0) return;
            int bank = value / bankSize;
            int cur = 0;
            bankMax.TryGetValue(bank, out cur);
            if (value > cur) bankMax[bank] = value;
            if (value > maxPc) maxPc = value;
        };

        Action finalizeFunction = () =>
        {
            if (string.IsNullOrEmpty(currentFunctionName)) return;
            functionSizes.Add(new FunctionSizeInfo
            {
                Name = currentFunctionName,
                StartFileOffset = currentFunctionStart,
                EndFileOffset = currentFunctionStart + currentFunctionBytes,
                SizeBytes = currentFunctionBytes,
                Bank = currentFunctionStart >> 14,
                CpuAddress = BuildReportUtil.CpuAddrFromFileOffset(currentFunctionStart)
            });
            currentFunctionName = null;
            currentFunctionStart = 0;
            currentFunctionBytes = 0;
        };

        updateBankMax(pc);

        foreach (Expr e in assembly ?? new List<Expr>())
        {
            if (e.Match(Tag.Function, out string fn))
            {
                finalizeFunction();
                currentFunctionName = fn;
                currentFunctionStart = pc;
                currentFunctionBytes = 0;
                continue;
            }

            if (e.Match(Tag.SkipTo, out int skip))
            {
                if (skip == 0)
                {
                    if (pc < codeStart) pc = codeStart;
                }
                else
                {
                    pc = skip;
                }
                updateBankMax(pc);
                continue;
            }

            if (e.Match(Tag.Align, out int align))
            {
                if (align > 0)
                {
                    int mask = align - 1;
                    pc = (pc + mask) & ~mask;
                    updateBankMax(pc);
                }
                continue;
            }

            if (e.Match(Tag.ReadonlyData, out string rdName, out byte[] rdBytes))
            {
                int n = rdBytes == null ? 0 : rdBytes.Length;
                pc += n;
                if (!string.IsNullOrEmpty(currentFunctionName)) currentFunctionBytes += n;
                updateBankMax(pc);
                continue;
            }

            if (e.Match(Tag.Word, out string wordLabel))
            {
                pc += 2;
                if (!string.IsNullOrEmpty(currentFunctionName)) currentFunctionBytes += 2;
                updateBankMax(pc);
                continue;
            }

            if (e.Match(Tag.Asm, out string mnemonic, out AsmOperand operand))
            {
                int n = 1 + OperandBytes(operand == null ? AddressMode.Implicit : operand.Mode);
                pc += n;
                if (!string.IsNullOrEmpty(currentFunctionName)) currentFunctionBytes += n;
                updateBankMax(pc);
                continue;
            }
        }
        finalizeFunction();

        int romSize = Math.Max(minRomSize, RoundUpInt(maxPc, bankSize));
        if (romSize > maxRomSize) romSize = maxRomSize;
        int bankCount = Math.Max(2, romSize / bankSize);
        int[] bankMaxPc = new int[bankCount];
        for (int i = 0; i < bankCount; i++)
        {
            int v = 0;
            bankMax.TryGetValue(i, out v);
            bankMaxPc[i] = v;
        }

        return new BankSimulationResult
        {
            BankMaxPc = bankMaxPc,
            RomSizeBytes = romSize,
            UsedBytes = Math.Max(codeStart, pc),
            FunctionSizes = functionSizes
        };
    }

    static int OperandBytes(AddressMode mode)
    {
        if (mode == AddressMode.Implicit) return 0;
        if (mode == AddressMode.Immediate || mode == AddressMode.HighMem || mode == AddressMode.HighMemX || mode == AddressMode.Relative) return 1;
        return 2;
    }

    static int RoundUpInt(int value, int align)
    {
        if (align <= 0) return value;
        int mask = align - 1;
        return (value + mask) & ~mask;
    }

    static string BuildBankSimReportText(BankSimulationResult sim)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB bank allocation simulator");
        sb.AppendLine("# estimated pre-assembly layout");
        sb.AppendLine("rom_size=" + sim.RomSizeBytes + " used=" + sim.UsedBytes);
        sb.AppendLine("# bank, used, free");
        const int bankSize = 0x4000;
        for (int b = 0; b < sim.BankMaxPc.Length; b++)
        {
            int start = b * bankSize;
            int end = (b + 1) * bankSize;
            int maxPc = sim.BankMaxPc[b];
            int used = Math.Max(0, Math.Min(maxPc, end) - start);
            int free = Math.Max(0, bankSize - used);
            sb.AppendFormat("{0,2}, {1,5}, {2,5}\n", b, used, free);
        }

        sb.AppendLine();
        sb.AppendLine("# estimated function sizes");
        sb.AppendLine("# name, bank, start, end, size");
        foreach (var f in sim.FunctionSizes.OrderByDescending(x => x.SizeBytes).ThenBy(x => x.Name, StringComparer.Ordinal))
        {
            sb.AppendFormat("{0}, {1}, 0x{2:X5}, 0x{3:X5}, {4}\n", f.Name, f.Bank, f.StartFileOffset, f.EndFileOffset, f.SizeBytes);
        }
        return sb.ToString();
    }

    static string BuildFarcallSuggestionText(CodegenAnalysisReport report)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB farcall suggestion report");
        var rows = (report.Calls ?? new List<CallEdgeInfo>())
            .Where(c => c.CallerBank >= 0 && c.CalleeBank >= 0 && c.CallerBank != c.CalleeBank && !c.ViaFarcall)
            .OrderByDescending(c => c.Count)
            .ThenBy(c => c.Caller, StringComparer.Ordinal)
            .ThenBy(c => c.Callee, StringComparer.Ordinal)
            .ToList();

        if (rows.Count == 0)
        {
            sb.AppendLine("no farcall candidates");
            return sb.ToString();
        }

        sb.AppendLine("# caller -> callee, count, caller_bank, callee_bank, current_kind, recommendation");
        foreach (var c in rows)
        {
            string rec = c.ViaThunk ? "consider __farcall(bank, func) for explicit cross-bank intent" : "check bank safety";
            sb.AppendFormat("{0} -> {1}, {2}, {3}, {4}, {5}, {6}\n",
                c.Caller, c.Callee, c.Count, c.CallerBank, c.CalleeBank, c.Kind, rec);
        }
        return sb.ToString();
    }

    static string BuildCrossBankCallText(CodegenAnalysisReport report)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB cross-bank call report");
        sb.AppendLine("# caller, caller_bank, callee, callee_bank, count, kind, via_thunk, via_farcall");
        var rows = (report.Calls ?? new List<CallEdgeInfo>())
            .Where(c => c.CallerBank >= 0 && c.CalleeBank >= 0 && c.CallerBank != c.CalleeBank)
            .OrderByDescending(c => c.Count)
            .ThenBy(c => c.Caller, StringComparer.Ordinal)
            .ThenBy(c => c.Callee, StringComparer.Ordinal);

        foreach (var c in rows)
        {
            sb.AppendFormat("{0}, {1}, {2}, {3}, {4}, {5}, {6}, {7}\n",
                c.Caller, c.CallerBank, c.Callee, c.CalleeBank, c.Count, c.Kind, c.ViaThunk ? 1 : 0, c.ViaFarcall ? 1 : 0);
        }
        return sb.ToString();
    }

    static string BuildAbiVerifyText(CodegenAnalysisReport report, List<string> issues)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB ABI verification report");
        sb.AppendLine("abi_mode=" + Abi.ToString().ToLowerInvariant());
        sb.AppendLine("functions=" + (report.Functions == null ? 0 : report.Functions.Count));
        sb.AppendLine("calls=" + (report.Calls == null ? 0 : report.Calls.Count));
        sb.AppendLine("issues=" + (issues == null ? 0 : issues.Count));
        sb.AppendLine();
        if (issues == null || issues.Count == 0)
        {
            sb.AppendLine("PASS");
            return sb.ToString();
        }
        sb.AppendLine("FAIL");
        foreach (var issue in issues) sb.AppendLine("- " + issue);
        return sb.ToString();
    }

    static string BuildRstApplyText(CodegenAnalysisReport codegen, OptimizerAnalysisReport opt)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB RST apply report");
        sb.AppendLine("rst_enabled=" + (!RstDisable ? "1" : "0"));
        sb.AppendLine("rst_use_38=" + (RstUse38 ? "1" : "0"));
        sb.AppendLine();
        sb.AppendLine("# selection");
        foreach (var r in (codegen.RstSelections ?? new List<RstSelectionInfo>()).OrderBy(x => x.Vector))
        {
            sb.AppendFormat("RST_{0:X2} -> {1} (calls={2}, net_bytes={3})\n", r.Vector, r.TargetLabel, r.Calls, r.NetBytes);
        }

        sb.AppendLine();
        sb.AppendLine("# optimizer rewrites");
        sb.AppendLine("total=" + opt.TotalRstRewrites);
        foreach (var kv in opt.RstRewriteCountsByVector.OrderBy(x => x.Key))
        {
            sb.AppendFormat("RST_{0:X2}: {1}\n", kv.Key, kv.Value);
        }
        return sb.ToString();
    }

    static string BuildOptimizerDiffText(OptimizerAnalysisReport report)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB optimizer pass diff report");
        foreach (var p in report.Passes ?? new List<OptimizerPassReport>())
        {
            sb.AppendLine();
            sb.AppendLine("## " + p.Name);
            sb.AppendLine("before=" + p.BeforeLines + " after=" + p.AfterLines + " changed=" + p.ChangedLines + " added=" + p.AddedLines + " removed=" + p.RemovedLines);
            sb.AppendLine(p.DiffText ?? "");
        }
        return sb.ToString();
    }

    static string BuildFunctionSizeReportText(AssemblerAnalysisReport report)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB function size report");
        sb.AppendLine("# name, bank, cpu_addr, start_file, end_file, size");
        foreach (var f in (report.FunctionSizes ?? new List<FunctionSizeInfo>()).OrderByDescending(x => x.SizeBytes).ThenBy(x => x.Name, StringComparer.Ordinal))
        {
            sb.AppendFormat("{0}, {1}, 0x{2:X4}, 0x{3:X5}, 0x{4:X5}, {5}\n",
                f.Name, f.Bank, f.CpuAddress, f.StartFileOffset, f.EndFileOffset, f.SizeBytes);
        }
        return sb.ToString();
    }

    public static DiagnosticSnapshot BeginSuppressedDiagnostics()
    {
        DiagnosticSnapshot snap = new DiagnosticSnapshot
        {
            ErrorCount = ErrorCount,
            WarningCount = WarningCount,
            DiagnosticCount = Diagnostics.Count,
        };
        SuppressConsoleDiagnostics = true;
        return snap;
    }

    public static void RestoreDiagnostics(DiagnosticSnapshot snap)
    {
        ErrorCount = snap.ErrorCount;
        WarningCount = snap.WarningCount;
        while (Diagnostics.Count > snap.DiagnosticCount) Diagnostics.RemoveAt(Diagnostics.Count - 1);
        SuppressConsoleDiagnostics = false;
    }

    static void ClearFunctionBankOverrides()
    {
        FunctionBankOverrides.Clear();
    }

    public static bool TryGetFunctionBankOverride(string name, out int bank)
    {
        if (string.IsNullOrEmpty(name))
        {
            bank = 0;
            return false;
        }
        return FunctionBankOverrides.TryGetValue(name, out bank);
    }

    static void ReplaceFunctionBankOverrides(Dictionary<string, int> overrides)
    {
        FunctionBankOverrides.Clear();
        if (overrides == null) return;
        foreach (var kv in overrides)
            FunctionBankOverrides[kv.Key] = kv.Value;
    }

    sealed class FunctionBankConflictInfo
    {
        public string Name;
        public int RequestedBank;
        public int ActualBank;
    }

    sealed class FunctionBankRelayoutResult
    {
        public readonly Dictionary<string, int> Relocations = new Dictionary<string, int>(StringComparer.Ordinal);
        public readonly List<FunctionBankConflictInfo> FixedBankConflicts = new List<FunctionBankConflictInfo>();
    }

    static FunctionBankRelayoutResult DetectFunctionBankRelocations(CodegenAnalysisReport codegen, AssemblerAnalysisReport asm)
    {
        var actualByName = (asm?.FunctionSizes ?? new List<FunctionSizeInfo>())
            .Where(f => f != null && !string.IsNullOrEmpty(f.Name))
            .GroupBy(f => f.Name, StringComparer.Ordinal)
            .ToDictionary(
                g => g.Key,
                g => g.OrderBy(f => f.StartFileOffset).First().Bank,
                StringComparer.Ordinal);

        var result = new FunctionBankRelayoutResult();
        foreach (var f in codegen?.Functions ?? new List<FunctionAbiInfo>())
        {
            if (f == null || string.IsNullOrEmpty(f.Name)) continue;
            if (f.IsPrototype || f.IsInline) continue;
            if (!actualByName.TryGetValue(f.Name, out int actualBank)) continue;
            if (actualBank == f.Bank) continue;

            if (f.HasFixedBank)
            {
                result.FixedBankConflicts.Add(new FunctionBankConflictInfo
                {
                    Name = f.Name,
                    RequestedBank = f.Bank,
                    ActualBank = actualBank
                });
                continue;
            }

            result.Relocations[f.Name] = actualBank;
        }
        return result;
    }

    static string BuildHotspotReportText(CodegenAnalysisReport codegen, AssemblerAnalysisReport asm)
    {
        var sizeByFunc = (asm.FunctionSizes ?? new List<FunctionSizeInfo>())
            .GroupBy(x => x.Name, StringComparer.Ordinal)
            .ToDictionary(g => g.Key, g => g.Max(x => x.SizeBytes), StringComparer.Ordinal);

        var incomingCalls = new Dictionary<string, int>(StringComparer.Ordinal);
        foreach (var c in codegen.Calls ?? new List<CallEdgeInfo>())
        {
            if (string.IsNullOrEmpty(c.Callee) || c.Callee.StartsWith("<", StringComparison.Ordinal)) continue;
            int cur = 0;
            incomingCalls.TryGetValue(c.Callee, out cur);
            incomingCalls[c.Callee] = cur + c.Count;
        }

        var allFuncs = new HashSet<string>(sizeByFunc.Keys, StringComparer.Ordinal);
        foreach (var k in incomingCalls.Keys) allFuncs.Add(k);

        var rows = allFuncs
            .Select(name =>
            {
                int size = 0; sizeByFunc.TryGetValue(name, out size);
                int calls = 0; incomingCalls.TryGetValue(name, out calls);
                long score = (long)size * (long)calls;
                return new { Name = name, Size = size, Calls = calls, Score = score };
            })
            .OrderByDescending(x => x.Score)
            .ThenByDescending(x => x.Calls)
            .ThenByDescending(x => x.Size)
            .ThenBy(x => x.Name, StringComparer.Ordinal)
            .ToList();

        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB hotspot estimate");
        sb.AppendLine("# score = incoming_call_count * function_size");
        sb.AppendLine("# name, calls, size, score");
        foreach (var r in rows)
        {
            sb.AppendFormat("{0}, {1}, {2}, {3}\n", r.Name, r.Calls, r.Size, r.Score);
        }
        return sb.ToString();
    }

    sealed class CgbConsistencyResult
    {
        public bool Pass;
        public byte HeaderFlag;
        public string HeaderMode;
        public int TotalCgbWrites;
        public int GuardedWrites;
        public int UnguardedWrites;
        public int RuntimeChecks;
        public readonly List<string> Registers = new List<string>();
        public readonly List<string> Issues = new List<string>();
    }

    sealed class CgbSymbolVerifyResult
    {
        public bool Pass;
        public readonly List<string> Required = new List<string>();
        public readonly List<string> Referenced = new List<string>();
        public readonly List<string> Missing = new List<string>();
        public int MapSymbolCount;
    }

    static readonly Dictionary<int, string> CgbIoOffsetToName = new Dictionary<int, string>
    {
        { 0x4D, "KEY1" },
        { 0x4F, "VBK" },
        { 0x51, "HDMA1" },
        { 0x52, "HDMA2" },
        { 0x53, "HDMA3" },
        { 0x54, "HDMA4" },
        { 0x55, "HDMA5" },
        { 0x68, "BCPS" },
        { 0x69, "BCPD" },
        { 0x6A, "OCPS" },
        { 0x6B, "OCPD" },
        { 0x70, "SVBK" },
    };

    static readonly string[] CgbRequiredSymbols = new[]
    {
        "KEY1", "VBK", "SVBK", "BCPS", "BCPD", "OCPS", "OCPD", "HDMA1", "HDMA2", "HDMA3", "HDMA4", "HDMA5"
    };

    static CgbConsistencyResult AnalyzeCgbConsistency(IReadOnlyList<Expr> assembly, CodegenAnalysisReport codegen, string outputFilename, RomHeaderOptions effectiveRomHeader)
    {
        var result = new CgbConsistencyResult();
        result.HeaderFlag = ReadRomHeaderByteSafe(outputFilename, 0x143, effectiveRomHeader != null && effectiveRomHeader.CgbFlag.HasValue ? effectiveRomHeader.CgbFlag.Value : (byte)0x00);
        result.HeaderMode = DescribeCgbHeaderFlag(result.HeaderFlag);

        var regs = new HashSet<string>(StringComparer.Ordinal);
        int writes = 0;
        string currentFunction = null;

        foreach (var e in assembly ?? new List<Expr>())
        {
            if (e.Match(Tag.Function, out string fn))
            {
                currentFunction = fn;
                continue;
            }

            string mnemonic;
            AsmOperand operand;
            if (!e.Match(Tag.Asm, out mnemonic, out operand)) continue;

            // __kq_is_cgb is a compiler-injected runtime helper.
            // Do not count helper internals as payload CGB register writes.
            if (string.Equals(currentFunction, "__kq_is_cgb", StringComparison.Ordinal)) continue;

            if (TryGetCgbWriteRegisterName(mnemonic, operand, out string regName))
            {
                writes++;
                regs.Add(regName);
            }
        }

        result.TotalCgbWrites = writes;
        result.GuardedWrites = Math.Max(0, codegen == null ? 0 : codegen.CgbGuardedWriteCount);
        result.UnguardedWrites = Math.Max(0, result.TotalCgbWrites - result.GuardedWrites);
        result.RuntimeChecks = Math.Max(0, codegen == null ? 0 : codegen.CgbRuntimeCheckCount);
        foreach (var r in regs.OrderBy(x => x, StringComparer.Ordinal)) result.Registers.Add(r);

        if (result.HeaderFlag == 0x00 && result.UnguardedWrites > 0)
        {
            result.Issues.Add("header is DMG-only (0x00) but unguarded CGB register writes were detected");
        }
        if (result.HeaderFlag == 0x80 && result.UnguardedWrites > 0)
        {
            result.Issues.Add("header is CGB-compatible (0x80) but unguarded CGB register writes were detected");
        }
        if (result.HeaderFlag == 0x80 && result.GuardedWrites > 0 && result.RuntimeChecks == 0)
        {
            result.Issues.Add("CGB-compatible build uses guarded writes but no runtime CGB check calls were emitted");
        }

        result.Pass = result.Issues.Count == 0;
        return result;
    }

    static CgbSymbolVerifyResult AnalyzeCgbSymbolVerification(IReadOnlyList<Expr> assembly, string mapPath)
    {
        var result = new CgbSymbolVerifyResult();
        var mapSymbols = ParseMapSymbolNames(mapPath);
        result.MapSymbolCount = mapSymbols.Count;

        foreach (var s in CgbRequiredSymbols)
        {
            result.Required.Add(s);
        }

        var referenced = new HashSet<string>(StringComparer.Ordinal);
        foreach (var e in assembly ?? new List<Expr>())
        {
            string mnemonic;
            AsmOperand operand;
            if (!e.Match(Tag.Asm, out mnemonic, out operand)) continue;
            if (operand == null || !operand.Base.HasValue) continue;
            string baseName = operand.Base.Value;
            if (CgbRequiredSymbols.Contains(baseName))
            {
                referenced.Add(baseName);
            }
        }

        foreach (var r in referenced.OrderBy(x => x, StringComparer.Ordinal))
        {
            result.Referenced.Add(r);
        }

        var missing = new HashSet<string>(StringComparer.Ordinal);
        foreach (var req in CgbRequiredSymbols)
        {
            if (!mapSymbols.Contains(req)) missing.Add(req);
        }
        foreach (var rf in referenced)
        {
            if (!mapSymbols.Contains(rf)) missing.Add(rf);
        }

        foreach (var m in missing.OrderBy(x => x, StringComparer.Ordinal))
        {
            result.Missing.Add(m);
        }

        result.Pass = result.Missing.Count == 0;
        return result;
    }

    static HashSet<string> ParseMapSymbolNames(string mapPath)
    {
        var set = new HashSet<string>(StringComparer.Ordinal);
        if (string.IsNullOrWhiteSpace(mapPath) || !File.Exists(mapPath)) return set;

        foreach (var line in IoUtil.ReadAllLinesUtf8(mapPath))
        {
            if (string.IsNullOrWhiteSpace(line)) continue;
            string t = line.Trim();
            if (t.StartsWith(";") || t.StartsWith("#")) continue;

            var parts = t.Split(new[] { ' ', '\t' }, StringSplitOptions.RemoveEmptyEntries);
            if (parts.Length < 5) continue;
            string name = parts[parts.Length - 1];
            if (!string.IsNullOrEmpty(name)) set.Add(name);
        }
        return set;
    }

    static string BuildCgbConsistencyText(CgbConsistencyResult result)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB CGB consistency report");
        sb.AppendLine("header_flag=0x" + result.HeaderFlag.ToString("X2"));
        sb.AppendLine("header_mode=" + result.HeaderMode);
        sb.AppendLine("total_cgb_writes=" + result.TotalCgbWrites);
        sb.AppendLine("guarded_writes=" + result.GuardedWrites);
        sb.AppendLine("unguarded_writes=" + result.UnguardedWrites);
        sb.AppendLine("runtime_checks=" + result.RuntimeChecks);
        sb.AppendLine("result=" + (result.Pass ? "PASS" : "FAIL"));
        sb.AppendLine();
        sb.AppendLine("# registers");
        foreach (var r in result.Registers) sb.AppendLine("- " + r);
        sb.AppendLine();
        sb.AppendLine("# issues");
        if (result.Issues.Count == 0) sb.AppendLine("none");
        else foreach (var i in result.Issues) sb.AppendLine("- " + i);
        return sb.ToString();
    }

    static string BuildCgbSymbolVerifyText(CgbSymbolVerifyResult result)
    {
        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB CGB symbol verification report");
        sb.AppendLine("map_symbol_count=" + result.MapSymbolCount);
        sb.AppendLine("required_symbol_count=" + result.Required.Count);
        sb.AppendLine("referenced_symbol_count=" + result.Referenced.Count);
        sb.AppendLine("missing_symbol_count=" + result.Missing.Count);
        sb.AppendLine("result=" + (result.Pass ? "PASS" : "FAIL"));
        sb.AppendLine();
        sb.AppendLine("# required");
        foreach (var x in result.Required) sb.AppendLine("- " + x);
        sb.AppendLine();
        sb.AppendLine("# referenced");
        if (result.Referenced.Count == 0) sb.AppendLine("none");
        else foreach (var x in result.Referenced) sb.AppendLine("- " + x);
        sb.AppendLine();
        sb.AppendLine("# missing");
        if (result.Missing.Count == 0) sb.AppendLine("none");
        else foreach (var x in result.Missing) sb.AppendLine("- " + x);
        return sb.ToString();
    }

    static bool TryGetCgbWriteRegisterName(string mnemonic, AsmOperand operand, out string regName)
    {
        regName = null;
        if (operand == null || string.IsNullOrEmpty(mnemonic)) return false;
        if (mnemonic != "LDH_MEM_A" && mnemonic != "LD_MEM_A") return false;

        if (operand.Base.HasValue)
        {
            string b = operand.Base.Value;
            if (CgbRequiredSymbols.Contains(b))
            {
                regName = b;
                return true;
            }
        }

        if (!operand.Base.HasValue)
        {
            if (mnemonic == "LDH_MEM_A")
            {
                int off = operand.Offset & 0xFF;
                if (CgbIoOffsetToName.TryGetValue(off, out string n))
                {
                    regName = n;
                    return true;
                }
            }
            else if (mnemonic == "LD_MEM_A")
            {
                int addr = operand.Offset & 0xFFFF;
                if ((addr & 0xFF00) == 0xFF00)
                {
                    int off = addr & 0xFF;
                    if (CgbIoOffsetToName.TryGetValue(off, out string n))
                    {
                        regName = n;
                        return true;
                    }
                }
            }
        }

        return false;
    }

    static byte ReadRomHeaderByteSafe(string romPath, int offset, byte fallback)
    {
        try
        {
            if (File.Exists(romPath))
            {
                var bytes = File.ReadAllBytes(romPath);
                if (offset >= 0 && offset < bytes.Length) return bytes[offset];
            }
        }
        catch { }
        return fallback;
    }

    static string DescribeCgbHeaderFlag(byte flag)
    {
        if (flag == 0xC0) return "cgb_only";
        if (flag == 0x80) return "cgb_compatible";
        if (flag == 0x00) return "dmg_only";
        return "unknown";
    }

    static bool IsAnalysisFlag(string arg)
    {
        if (string.IsNullOrEmpty(arg)) return false;
        return
            arg == "--bank-sim" || arg.StartsWith("--bank-sim=") ||
            arg == "--farcall-suggest" || arg.StartsWith("--farcall-suggest=") ||
            arg == "--cross-bank-report" || arg.StartsWith("--cross-bank-report=") ||
            arg == "--abi-verify" || arg.StartsWith("--abi-verify=") ||
            arg == "--abi-diff-report" || arg.StartsWith("--abi-diff-report=") ||
            arg == "--rst-report" || arg.StartsWith("--rst-report=") ||
            arg == "--opt-diff" || arg.StartsWith("--opt-diff=") ||
            arg == "--func-size-report" || arg.StartsWith("--func-size-report=") ||
            arg == "--hotspot-report" || arg.StartsWith("--hotspot-report=") ||
            arg == "--repro-check" || arg.StartsWith("--repro-check=") ||
            arg == "--cgb-consistency" || arg.StartsWith("--cgb-consistency=") ||
            arg == "--verify-cgb-symbols" || arg.StartsWith("--verify-cgb-symbols=");
    }

    static string[] BuildChildCompileArgs(string childOutput, string abiOverride)
    {
        var list = new List<string>();
        var args = _originalArgs ?? new string[0];
        for (int i = 0; i < args.Length; i++)
        {
            string a = args[i];

            if (a == "-o")
            {
                i++;
                continue;
            }
            if (a.StartsWith("--abi=")) continue;
            if (a == "--cache" || a == "--no-cache") continue;
            if (a == "--watch") continue;
            if (a == "--minimize" || a.StartsWith("--minimize")) continue;
            if (a == "--disasm-changed" || a.StartsWith("--disasm-changed=") || a.StartsWith("--disasm-changed-out=")) continue;
            if (a == "--repro-pack" || a.StartsWith("--repro-pack=")) continue;
            if (IsAnalysisFlag(a)) continue;
            list.Add(a);
        }

        list.Add("--no-cache");
        list.Add("--no-disasm");
        if (!string.IsNullOrWhiteSpace(abiOverride))
            list.Add("--abi=" + abiOverride);
        list.Add("-o");
        list.Add(childOutput);
        return list.ToArray();
    }

    static Dictionary<string, int> ParseFunctionSizesFile(string path)
    {
        var map = new Dictionary<string, int>(StringComparer.Ordinal);
        if (!File.Exists(path)) return map;
        foreach (var line in IoUtil.ReadAllLinesUtf8(path))
        {
            if (string.IsNullOrWhiteSpace(line)) continue;
            if (line.StartsWith("#")) continue;
            var parts = line.Split(',');
            if (parts.Length < 6) continue;
            string name = parts[0].Trim();
            if (name.Length == 0) continue;
            int size;
            if (int.TryParse(parts[5].Trim(), out size))
                map[name] = size;
        }
        return map;
    }

    static void TryGenerateAbiDiffReport(string reportPath)
    {
        string exe = Process.GetCurrentProcess().MainModule.FileName;
        string tempDir = Path.Combine(Path.GetDirectoryName(reportPath) ?? ".", "kitaqgb_abidiff_" + DateTime.UtcNow.ToString("yyyyMMdd_HHmmssfff"));
        Directory.CreateDirectory(tempDir);
        string legacyOut = Path.Combine(tempDir, "legacy.gb");
        string stackOut = Path.Combine(tempDir, "stack.gb");

        int codeLegacy = RunChildCompiler(exe, BuildChildCompileArgs(legacyOut, "legacy"), Environment.CurrentDirectory);
        int codeStack = RunChildCompiler(exe, BuildChildCompileArgs(stackOut, "stack"), Environment.CurrentDirectory);

        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB ABI diff report");
        sb.AppendLine("legacy_exit=" + codeLegacy);
        sb.AppendLine("stack_exit=" + codeStack);

        if (codeLegacy == 0 && File.Exists(legacyOut))
        {
            sb.AppendLine("legacy_size=" + new FileInfo(legacyOut).Length);
            sb.AppendLine("legacy_sha256=" + BuildReportUtil.ComputeSha256HexOfFile(legacyOut));
        }
        if (codeStack == 0 && File.Exists(stackOut))
        {
            sb.AppendLine("stack_size=" + new FileInfo(stackOut).Length);
            sb.AppendLine("stack_sha256=" + BuildReportUtil.ComputeSha256HexOfFile(stackOut));
        }

        string legacyFuncPath = Path.ChangeExtension(legacyOut, ".funcsizes.txt");
        string stackFuncPath = Path.ChangeExtension(stackOut, ".funcsizes.txt");
        var legacySizes = ParseFunctionSizesFile(legacyFuncPath);
        var stackSizes = ParseFunctionSizesFile(stackFuncPath);
        var all = new HashSet<string>(legacySizes.Keys, StringComparer.Ordinal);
        foreach (var k in stackSizes.Keys) all.Add(k);

        var deltas = new List<(string Name, int Legacy, int Stack, int Delta)>();
        foreach (var name in all)
        {
            int l = 0, s = 0;
            legacySizes.TryGetValue(name, out l);
            stackSizes.TryGetValue(name, out s);
            deltas.Add((name, l, s, s - l));
        }

        sb.AppendLine();
        sb.AppendLine("# function size delta (stack - legacy)");
        sb.AppendLine("# name, legacy, stack, delta");
        foreach (var d in deltas.OrderByDescending(x => Math.Abs(x.Delta)).ThenBy(x => x.Name, StringComparer.Ordinal))
        {
            sb.AppendFormat("{0}, {1}, {2}, {3}\n", d.Name, d.Legacy, d.Stack, d.Delta);
        }

        reportPath = IoUtil.WriteAllTextUtf8Robust(reportPath, sb.ToString(), allowAlternatePath: true);
        WriteInfoLine("[report] abi diff: " + reportPath);

        if (codeLegacy != 0 || codeStack != 0)
        {
            Error("ABI diff report generation failed (legacy={0}, stack={1}). See: {2}", codeLegacy, codeStack, reportPath);
        }

        try { Directory.Delete(tempDir, true); } catch { }
    }

    static void TryRunReproCheck(string outputFilename, string reportPath)
    {
        string exe = Process.GetCurrentProcess().MainModule.FileName;
        string abiName = (Abi == AbiMode.Stack) ? "stack" : "legacy";
        string tempDir = Path.Combine(Path.GetDirectoryName(reportPath) ?? ".", "kitaqgb_repro_" + DateTime.UtcNow.ToString("yyyyMMdd_HHmmssfff"));
        Directory.CreateDirectory(tempDir);
        string reproOut = Path.Combine(tempDir, "repro.gb");

        int code = RunChildCompiler(exe, BuildChildCompileArgs(reproOut, abiName), Environment.CurrentDirectory);

        string currentGbHash = BuildReportUtil.ComputeSha256HexOfFile(outputFilename);
        string reproGbHash = BuildReportUtil.ComputeSha256HexOfFile(reproOut);
        string currentMapHash = BuildReportUtil.ComputeSha256HexOfFile(Path.ChangeExtension(outputFilename, ".map"));
        string reproMapHash = BuildReportUtil.ComputeSha256HexOfFile(Path.ChangeExtension(reproOut, ".map"));

        bool sameGb = !string.IsNullOrEmpty(currentGbHash) && currentGbHash == reproGbHash;
        bool sameMap = !string.IsNullOrEmpty(currentMapHash) && currentMapHash == reproMapHash;
        bool ok = (code == 0) && sameGb && sameMap;

        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB reproducibility check");
        sb.AppendLine("exit=" + code);
        sb.AppendLine("same_gb=" + (sameGb ? "1" : "0"));
        sb.AppendLine("same_map=" + (sameMap ? "1" : "0"));
        sb.AppendLine("current_gb_sha256=" + currentGbHash);
        sb.AppendLine("repro_gb_sha256=" + reproGbHash);
        sb.AppendLine("current_map_sha256=" + currentMapHash);
        sb.AppendLine("repro_map_sha256=" + reproMapHash);
        sb.AppendLine("result=" + (ok ? "PASS" : "FAIL"));

        reportPath = IoUtil.WriteAllTextUtf8Robust(reportPath, sb.ToString(), allowAlternatePath: true);
        WriteInfoLine("[report] repro check: " + reportPath);

        if (!ok)
        {
            Error("Reproducibility check failed. See: {0}", reportPath);
        }

        try { Directory.Delete(tempDir, true); } catch { }
    }

    static bool TryRunAttrVizCommand(string[] argsArray)
    {
        if (argsArray == null || argsArray.Length == 0) return false;
        if (!string.Equals(argsArray[0], "attrviz", StringComparison.OrdinalIgnoreCase)) return false;

        if (argsArray.Length < 2)
        {
            Console.Error.WriteLine("error KQ0000: attrviz requires an input file");
            Console.Error.WriteLine("usage: kitaqgb attrviz <attr.bin> [--width=N] [--height=N] [--out=<file>|-]");
            Exit(1);
            return true;
        }

        string inputPath = argsArray[1];
        int width = 32;
        int height = 0;
        string outPath = "";

        for (int i = 2; i < argsArray.Length; i++)
        {
            string a = argsArray[i] ?? "";
            if (a.StartsWith("--width=", StringComparison.Ordinal))
            {
                if (!int.TryParse(ValueAfterEquals(a), out width) || width <= 0 || width > 1024)
                {
                    Console.Error.WriteLine("error KQ0000: --width must be 1..1024");
                    Exit(1);
                    return true;
                }
            }
            else if (a.StartsWith("--height=", StringComparison.Ordinal))
            {
                if (!int.TryParse(ValueAfterEquals(a), out height) || height < 0 || height > 1024)
                {
                    Console.Error.WriteLine("error KQ0000: --height must be 0..1024");
                    Exit(1);
                    return true;
                }
            }
            else if (a.StartsWith("--out=", StringComparison.Ordinal))
            {
                outPath = ValueAfterEquals(a);
            }
            else
            {
                Console.Error.WriteLine("error KQ0000: unknown attrviz option: " + a);
                Exit(1);
                return true;
            }
        }

        try
        {
            if (!File.Exists(inputPath))
            {
                Console.Error.WriteLine("error KQ0000: attrviz input not found: " + inputPath);
                Exit(1);
                return true;
            }

            byte[] bytes = File.ReadAllBytes(inputPath);
            if (height <= 0)
            {
                height = (bytes.Length + width - 1) / width;
            }

            string text = BuildAttrVizText(inputPath, bytes, width, height);
            if (string.IsNullOrWhiteSpace(outPath))
            {
                string stem = Path.GetFileNameWithoutExtension(inputPath);
                if (string.IsNullOrWhiteSpace(stem)) stem = "attr";
                outPath = Path.Combine(Path.GetDirectoryName(Path.GetFullPath(inputPath)) ?? ".", stem + ".attrviz.txt");
            }

            if (outPath == "-")
            {
                Console.Write(text);
            }
            else
            {
                IoUtil.WriteAllTextUtf8Robust(outPath, text, allowAlternatePath: true);
                Console.WriteLine("[attrviz] " + outPath);
            }

            Exit(0);
            return true;
        }
        catch (ControlledCompilerExit)
        {
            throw;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("error KQ0000: attrviz failed: " + ex.Message);
            Exit(1);
            return true;
        }
    }

    static string BuildAttrVizText(string inputPath, byte[] data, int width, int height)
    {
        if (data == null) data = new byte[0];
        if (width <= 0) width = 1;
        if (height < 0) height = 0;

        int n = data.Length;
        int rows = height;
        int cells = width * rows;

        int[] palCount = new int[8];
        int bank1 = 0, dmgPal1 = 0, xflip = 0, yflip = 0, pri = 0;

        for (int i = 0; i < n; i++)
        {
            byte b = data[i];
            palCount[b & 0x07]++;
            if ((b & 0x08) != 0) bank1++;
            if ((b & 0x10) != 0) dmgPal1++;
            if ((b & 0x20) != 0) xflip++;
            if ((b & 0x40) != 0) yflip++;
            if ((b & 0x80) != 0) pri++;
        }

        var sb = new StringBuilder();
        sb.AppendLine("# KITAQGB tile attribute map visualization");
        sb.AppendLine("source=" + inputPath);
        sb.AppendLine("bytes=" + n);
        sb.AppendLine("width=" + width);
        sb.AppendLine("height=" + rows);
        sb.AppendLine("cells=" + cells);
        sb.AppendLine();
        sb.AppendLine("# bit layout: b0-2=palette b3=vram_bank b4=dmg_palette b5=xflip b6=yflip b7=priority");
        sb.AppendLine();
        sb.AppendLine("# summary");
        for (int p = 0; p < palCount.Length; p++) sb.AppendLine("palette_" + p + "=" + palCount[p]);
        sb.AppendLine("vram_bank_1=" + bank1);
        sb.AppendLine("dmg_palette_1=" + dmgPal1);
        sb.AppendLine("xflip_1=" + xflip);
        sb.AppendLine("yflip_1=" + yflip);
        sb.AppendLine("priority_1=" + pri);
        sb.AppendLine();

        sb.AppendLine("# grid raw(hex)");
        for (int y = 0; y < rows; y++)
        {
            sb.Append(y.ToString("D3")).Append(": ");
            for (int x = 0; x < width; x++)
            {
                int idx = y * width + x;
                if (idx < n) sb.Append(data[idx].ToString("X2"));
                else sb.Append("--");
                if (x + 1 < width) sb.Append(' ');
            }
            sb.AppendLine();
        }

        sb.AppendLine();
        sb.AppendLine("# grid decoded");
        sb.AppendLine("# token format: p<pal><B|.><D|.><X|.><Y|.><P|.>");
        for (int y = 0; y < rows; y++)
        {
            sb.Append(y.ToString("D3")).Append(": ");
            for (int x = 0; x < width; x++)
            {
                int idx = y * width + x;
                if (idx < n)
                {
                    byte b = data[idx];
                    int pal = b & 0x07;
                    sb.Append('p').Append((char)('0' + pal));
                    sb.Append((b & 0x08) != 0 ? 'B' : '.');
                    sb.Append((b & 0x10) != 0 ? 'D' : '.');
                    sb.Append((b & 0x20) != 0 ? 'X' : '.');
                    sb.Append((b & 0x40) != 0 ? 'Y' : '.');
                    sb.Append((b & 0x80) != 0 ? 'P' : '.');
                }
                else
                {
                    sb.Append("------");
                }

                if (x + 1 < width) sb.Append(' ');
            }
            sb.AppendLine();
        }

        return sb.ToString();
    }

    static bool TryRunTestCommand(string[] argsArray)
    {
        if (argsArray == null || argsArray.Length == 0) return false;
        if (!string.Equals(argsArray[0], "test", StringComparison.OrdinalIgnoreCase)) return false;

        string root = FindRepoRoot(Environment.CurrentDirectory);
        if (string.IsNullOrEmpty(root))
        {
            Console.Error.WriteLine("error KQ0000: integration_test/scripts/run_all.ps1 not found");
            Exit(1);
            return true;
        }

        string script = Path.Combine(root, "integration_test", "scripts", "run_all.ps1");
        if (!File.Exists(script))
        {
            Console.Error.WriteLine("error KQ0000: test script not found: " + script);
            Exit(1);
            return true;
        }

        var p = new ProcessStartInfo("powershell")
        {
            UseShellExecute = false,
            WorkingDirectory = root,
            CreateNoWindow = false,
            Arguments = "-NoProfile -ExecutionPolicy Bypass -File " + QuoteArg(script),
        };
        if (argsArray.Length > 1)
        {
            for (int i = 1; i < argsArray.Length; i++)
            {
                p.Arguments += " " + QuoteArg(argsArray[i]);
            }
        }

        try
        {
            using (var proc = Process.Start(p))
            {
                proc.WaitForExit();
                Exit(proc.ExitCode);
                return true;
            }
        }
        catch (ControlledCompilerExit)
        {
            throw;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("error KQ0000: failed to run test command: " + ex.Message);
            Exit(1);
            return true;
        }
    }

    static bool HasWatchFlag(string[] args)
    {
        if (args == null) return false;
        return args.Any(a => string.Equals(a, "--watch", StringComparison.OrdinalIgnoreCase));
    }

    static void RunWatchDriver(string[] argsArray)
    {
        string exe = Process.GetCurrentProcess().MainModule.FileName;
        var childArgs = argsArray.Where(a => !string.Equals(a, "--watch", StringComparison.OrdinalIgnoreCase)).ToList();

        var sourceFiles = ExtractSourceFiles(childArgs);
        var includeDirs = ExtractIncludeDirs(childArgs);
        if (sourceFiles.Count == 0)
        {
            Console.Error.WriteLine("error KQ0000: --watch requires at least one source file");
            Exit(1);
            return;
        }

        var watchDirs = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var s in sourceFiles)
        {
            string d = Path.GetDirectoryName(Path.GetFullPath(s));
            if (!string.IsNullOrWhiteSpace(d) && Directory.Exists(d)) watchDirs.Add(d);
        }
        foreach (var d in includeDirs)
        {
            try
            {
                string full = Path.GetFullPath(d);
                if (!string.IsNullOrWhiteSpace(full) && Directory.Exists(full)) watchDirs.Add(full);
            }
            catch { }
        }

        bool shouldStop = false;
        Console.CancelKeyPress += (sender, e) =>
        {
            e.Cancel = true;
            shouldStop = true;
        };

        while (!shouldStop)
        {
            int code = RunChildCompiler(exe, childArgs.ToArray(), Environment.CurrentDirectory);
            Console.WriteLine("[watch] build exit code: " + code);
            if (shouldStop) break;

            using (var changed = new AutoResetEvent(false))
            {
                var watchers = new List<FileSystemWatcher>();
                foreach (var d in watchDirs)
                {
                    var w = new FileSystemWatcher(d)
                    {
                        IncludeSubdirectories = true,
                        NotifyFilter = NotifyFilters.LastWrite | NotifyFilters.FileName | NotifyFilters.CreationTime | NotifyFilters.Size,
                        Filter = "*.*",
                        EnableRaisingEvents = true
                    };
                    FileSystemEventHandler onChanged = (s, e) =>
                    {
                        string ext = Path.GetExtension(e.FullPath).ToLowerInvariant();
                        if (ext == ".c" || ext == ".h" || ext == ".inc")
                        {
                            changed.Set();
                        }
                    };
                    RenamedEventHandler onRenamed = (s, e) =>
                    {
                        string ext = Path.GetExtension(e.FullPath).ToLowerInvariant();
                        if (ext == ".c" || ext == ".h" || ext == ".inc")
                        {
                            changed.Set();
                        }
                    };
                    w.Changed += onChanged;
                    w.Created += onChanged;
                    w.Deleted += onChanged;
                    w.Renamed += onRenamed;
                    watchers.Add(w);
                }

                Console.WriteLine("[watch] waiting for file changes...");
                while (!shouldStop)
                {
                    if (changed.WaitOne(200))
                    {
                        Thread.Sleep(180);
                        break;
                    }
                }
                foreach (var w in watchers) w.Dispose();
            }
        }

        Exit(0);
    }

    static int RunChildCompiler(string exePath, string[] args, string workingDir)
    {
        var psi = new ProcessStartInfo(exePath)
        {
            UseShellExecute = false,
            WorkingDirectory = workingDir,
            CreateNoWindow = false,
            Arguments = string.Join(" ", args.Select(QuoteArg))
        };
        using (var p = Process.Start(psi))
        {
            p.WaitForExit();
            return p.ExitCode;
        }
    }

    static List<string> ExtractSourceFiles(List<string> args)
    {
        var files = new List<string>();
        for (int i = 0; i < args.Count; i++)
        {
            string a = args[i];
            if (a == "-o" || a == "-I")
            {
                i++;
                continue;
            }
            if (a.StartsWith("-")) continue;
            files.Add(a);
        }
        return files;
    }

    static List<string> ExtractIncludeDirs(List<string> args)
    {
        var dirs = new List<string>();
        for (int i = 0; i < args.Count; i++)
        {
            string a = args[i];
            if (a == "-I")
            {
                if (i + 1 < args.Count) dirs.Add(args[i + 1]);
                i++;
                continue;
            }
            if (a.StartsWith("-I") && a.Length > 2)
            {
                dirs.Add(a.Substring(2));
                continue;
            }
            if (a.StartsWith("--include-dir="))
            {
                dirs.Add(ValueAfterEquals(a));
                continue;
            }
        }
        return dirs;
    }

    static string FindRepoRoot(string startDir)
    {
        string dir = Path.GetFullPath(startDir);
        while (true)
        {
            if (File.Exists(Path.Combine(dir, "kitaqgb.sln"))) return dir;
            string parent = Path.GetDirectoryName(dir);
            if (string.IsNullOrEmpty(parent) || string.Equals(parent, dir, StringComparison.OrdinalIgnoreCase)) return null;
            dir = parent;
        }
    }

    static string QuoteArg(string a)
    {
        if (a == null) return "\"\"";
        if (a.Length == 0) return "\"\"";
        if (a.IndexOfAny(new[] { ' ', '\t', '\n', '"' }) < 0) return a;
        return "\"" + a.Replace("\"", "\\\"") + "\"";
    }

    static void TryRunAutoMinimize()
    {
        try
        {
            string exe = Process.GetCurrentProcess().MainModule.FileName;
            var args = new List<string>();
            foreach (var a in _originalArgs ?? new string[0])
            {
                if (string.Equals(a, "--minimize-on-fail", StringComparison.OrdinalIgnoreCase)) continue;
                if (string.Equals(a, "--auto-minimize", StringComparison.OrdinalIgnoreCase)) continue;
                args.Add(a);
            }
            if (!args.Any(a => a.StartsWith("--minimize", StringComparison.Ordinal)))
            {
                args.Add("--minimize");
            }
            if (!args.Any(a => a.StartsWith("--minimize-trace=", StringComparison.Ordinal)))
            {
                args.Add("--minimize-trace=final");
            }

            var psi = new ProcessStartInfo(exe)
            {
                UseShellExecute = false,
                CreateNoWindow = true,
                Arguments = string.Join(" ", args.Select(QuoteArg))
            };
            using (var p = Process.Start(psi))
            {
                p.WaitForExit();
            }
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("warning KQ0000: auto-minimize failed: " + ex.Message);
        }
    }

    static void TryWriteDiagnosticJson(int exitCode)
    {
        if (!EmitDiagJson) return;

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

        try
        {
            if (DiagJsonPath == "-")
            {
                Console.Error.WriteLine(sb.ToString());
            }
            else
            {
                DiagJsonPath = IoUtil.WriteAllTextUtf8Robust(DiagJsonPath, sb.ToString(), allowAlternatePath: true);
                RememberArtifactPath("diag_json", DiagJsonPath);
            }
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("warning KQ0000: failed to write diag json: " + ex.Message);
        }
    }

    static string JsonEscape(string s)
    {
        if (string.IsNullOrEmpty(s)) return "";
        var sb = new StringBuilder(s.Length + 8);
        foreach (char c in s)
        {
            switch (c)
            {
                case '\\': sb.Append("\\\\"); break;
                case '"': sb.Append("\\\""); break;
                case '\n': sb.Append("\\n"); break;
                case '\r': sb.Append("\\r"); break;
                case '\t': sb.Append("\\t"); break;
                default:
                    if (c < 32) sb.Append("\\u").Append(((int)c).ToString("x4"));
                    else sb.Append(c);
                    break;
            }
        }
        return sb.ToString();
    }

    [DebuggerStepThrough]
    public static void NYI() => GeneralError(Severity.InternalError, Maybe.Nothing, ErrorCode.InternalNYI, "not yet implemented");
    [DebuggerStepThrough]
    public static void UnhandledCase() => GeneralError(Severity.InternalError, Maybe.Nothing, ErrorCode.InternalUnhandledCase, "unhandled case");

    static readonly Dictionary<Severity, string> SeverityText = new Dictionary<Severity, string>
    {
        { Severity.Warning, "warning" },
        { Severity.Error, "error" },
        { Severity.InternalError, "internal error" },
    };

    internal sealed class DiagnosticEntry
    {
        public string Severity;
        public string Code;
        public string Message;
        public string Suggestion;
        public string Filename;
        public int Line;
        public int Column;
    }
}

// Symbolic constants used in Exprs.
static class Tag
{
    // Top-level declarations:
    public static readonly string Function = "$function";
    public static readonly string InlineFunction = "$inline_function";
    public static readonly string FunctionDecl = "$function_decl";
    public static readonly string Constant = "$constant";
    public static readonly string Variable = "$variable";
    public static readonly string ExternVariable = "$extern_variable";
    public static readonly string ReadonlyData = "$readonly_data";
    public static readonly string Bank = "$bank";
    public static readonly string FixedBank = "$fixed_bank";
    public static readonly string FixedOrder = "$fixed_order";
    public static readonly string Static = "$static";
    public static readonly string Struct = "$struct";
    public static readonly string Union = "$union";
    public static readonly string OpaqueStruct = "$opaque_struct";
    public static readonly string OpaqueUnion = "$opaque_union";

    // Compile-time assertions:
    public static readonly string StaticAssert = "$static_assert";

    // Attributes:
    public static readonly string Range = "$range";
    // Declaration placement wrappers (produced by #pragma shortcuts)
    public static readonly string DeclAlign = "$decl_align";
    public static readonly string DeclSection = "$decl_section";

    // Safety boundary markers (zero runtime cost)
    public static readonly string Unsafe = "$unsafe";

    // Calling convention marker
    public static readonly string StackCall = "$stackcall";

    // Statements:
    public static readonly string Sequence = "$sequence";
    public static readonly string If = "$if";
    public static readonly string For = "$for";
    public static readonly string DoWhile = "$do_while";
    public static readonly string Continue = "$continue";
    public static readonly string Break = "$break";
    public static readonly string Fallthrough = "$fallthrough";
    public static readonly string Return = "$return";

    // Expressions:
    public static readonly string AddressOf = "$address_of";
    public static readonly string Field = "$field";
    public static readonly string Index = "$index";
    // Slice helper: __slice(ptr,len) lowered to $slice(ptr,len)
    public static readonly string Slice = "$slice";
    public static readonly string Call = "$call";
    public static readonly string Assign = "$assign";
    public static readonly string AssignModify = "$assign_modify";
    public static readonly string Conditional = "$conditional";
    public static readonly string Add = "$add";
    public static readonly string Subtract = "$sub";
    public static readonly string Multiply = "$mul";
    public static readonly string Divide = "$div";
    public static readonly string Modulus = "$mod";
    public static readonly string Load = "$load";
    public static readonly string Store = "$store";
    public static readonly string Cast = "$cast";
    public static readonly string RawOffset = "$raw_offset";

    public static readonly string Sizeof = "$sizeof";
    public static readonly string Offsetof = "$offsetof";

    public static readonly string Equal = "$equal";
    public static readonly string NotEqual = "$not_equal";
    public static readonly string LessThan = "$less_than";
    public static readonly string LessThanOrEqual = "$less_than_or_equal";
    public static readonly string GreaterThan = "$greater_than";
    public static readonly string GreaterThanOrEqual = "$greater_than_or_equal";

    public static readonly string BitwiseNot = "$bitwise_not";
    public static readonly string BitwiseAnd = "$bitwise_and";
    public static readonly string BitwiseOr = "$bitwise_or";
    public static readonly string BitwiseXor = "$bitwise_xor";
    public static readonly string ShiftLeft = "$shift_left";
    public static readonly string ShiftRight = "$shift_right";

    public static readonly string LogicalNot = "$logical_not";
    public static readonly string LogicalOr = "$logical_or";
    public static readonly string LogicalAnd = "$logical_and";

    public static readonly string PreIncrement = "$pre_increment";
    public static readonly string PostIncrement = "$post_increment";
    public static readonly string PreDecrement = "$pre_decrement";
    public static readonly string PostDecrement = "$post_decrement";

    // Assembly directives:
    public static readonly string Asm = "$asm";
    public static readonly string Label = "$label";
    public static readonly string Jump = "$jump";
    public static readonly string Comment = "$comment";
    public static readonly string SkipTo = "$skip_to";
    public static readonly string Align = "$align";
    public static readonly string Section = "$section";
    public static readonly string Word = "$word";

    // Assembler-side directives:
    // - $rst_map <vector:int> <targetLabel:string>
    // Instructs the assembler to place a JP stub at the given RST vector (0x00..0x38)
    // so that codegen/optimizer can safely emit RST_xx instead of CALL target.
    public static readonly string RstMap = "$rst_map";

    // Leaf expressions:
    public static readonly string Empty = "$empty";
    public static readonly string Integer = "$integer";
    public static readonly string Name = "$name";

    public static readonly string Switch = "$switch";
    public static readonly string Case = "$case";
}

enum Severity
{
    Warning,
    Error,
    InternalError,
}

// Diagnostic codes (KQ0000..KQ9999). Keep the numbers stable once published.
enum ErrorCode
{
    None = 0,

    // Parser / front-end (1xxx)
    ParseError = 1000,
    ExpectedToken = 1001,
    ExpectedType = 1002,
    PreprocessorWarning = 1003,
    PreprocessorError = 1004,

    // Type / semantic (2xxx)
    AggregateNotDefined = 2100,
    IncompleteType = 2101,
    InvalidWramXBank = 2102,
    BankedWramRequiresCgbOnly = 2103,
    BankedWramLocalNotAllowed = 2104,
    WramXBankOverflow = 2105,
    InvalidStackTop = 2106,
    InvalidStackReserve = 2107,
    StackTopBankMismatch = 2108,
    StackReservedOverlap = 2109,

    // Const / qualifiers (22xx)
    ConstAssign = 2201,
    ConstModify = 2202,

    // Compile-time assertions (23xx)
    StaticAssertFailed = 2301,

    // Lints / warnings (24xx)
    MustCheckUnused = 2401,

    NonNullArgument = 2402,
    RangeViolation = 2403,

    // Lint: __range(min,max) index used with array length check
    RangeIndexOob = 2404,

    // Lint: switch(enum) case label type checks
    SwitchCaseEnumMismatch = 2405,
    SwitchCaseNonEnumOnEnumSwitch = 2406,

    // Lint: __enum_strict enum/integer mixing
    EnumStrictMix = 2407,

    // Lint: __bitflags misuse / mixing
    BitFlagsOp = 2408,
    BitFlagsMix = 2409,
    
    // Lint: __safe_index index/mixing
    SafeIndexIndex = 2410,
    SafeIndexMix = 2411,

    // Lint: __restrict aliasing (lightweight)
    RestrictAlias = 2412,

    // Lint: discarding const on pointer conversions (assignment / argument passing)
    ConstDiscard = 2413,

    // Lint: switch fallthrough annotation
    SwitchImplicitFallthrough = 2414,
    SwitchFallthroughUsage = 2415,

    // Lint: unreachable code
    UnreachableCode = 2416,

    // Lint: unused variable / function
    UnusedSymbol = 2417,

    // Lint: implicit narrowing conversion
    ImplicitNarrowing = 2418,

    // Lint: potentially dangerous pointer arithmetic
    PointerArithmeticDanger = 2419,
    ManualSvbkRequired = 2420,

    // Extern (25xx)
    ExternUndefined = 2501,
    ExternTypeMismatch = 2502,
    ExternNotSupported = 2503,

    // Internal errors (9xxx)
    Internal = 9000,
    InternalNYI = 9001,
    InternalUnhandledCase = 9002,
}




