using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;

// Capture a function's placement constraints and argument/return widths for ABI reports.
sealed class FunctionAbiInfo
{
    public string Name;
    public int Bank;
    public bool HasFixedBank;
    public int PlacementOrder;
    public bool HasFixedOrder;
    public bool IsPrototype;
    public bool IsInline;
    public bool IsStackCall;
    public bool IsFastCall;
    public int ReturnSize;
    public int[] ParamSizes = new int[0];
}

// Aggregate one caller/callee relationship, including bank-crossing strategy and
// the latest observed argument-size/source details used by report generation.
sealed class CallEdgeInfo
{
    public string Caller;
    public string Callee;
    public int CallerBank;
    public int CalleeBank;
    public string Kind;
    public bool ViaThunk;
    public bool ViaFarcall;
    public int Count;
    public int[] LastActualArgSizes = new int[0];
    public int[] LastExpectedArgSizes = new int[0];
    public string LastSource;
}

// Record the selected strategy and byte size for one structure/aggregate copy.
sealed class AggregateCopyInfo
{
    public string Function;
    public string Type;
    public int SizeBytes;
    public string Strategy;
    public string Source;
}

// Describe a selected restart vector, its target and estimated net code-size saving.
sealed class RstSelectionInfo
{
    public int Vector;
    public string TargetLabel;
    public int Calls;
    public int NetBytes;
}

// Collect facts and decisions emitted by code generation; this object stores
// report data and does not itself validate or execute the generated ROM.
sealed class CodegenAnalysisReport
{
    public readonly List<FunctionAbiInfo> Functions = new List<FunctionAbiInfo>();
    public readonly List<CallEdgeInfo> Calls = new List<CallEdgeInfo>();
    public readonly List<AggregateCopyInfo> AggregateCopies = new List<AggregateCopyInfo>();
    public readonly List<RstSelectionInfo> RstSelections = new List<RstSelectionInfo>();
    public readonly List<string> AbiIssues = new List<string>();
    public int CgbRuntimeCheckCount;
    public int CgbGuardedWriteCount;
    public readonly List<string> CgbGuardedRegisters = new List<string>();
}

// Record before/after line counts and the textual diff for one optimizer pass.
sealed class OptimizerPassReport
{
    public string Name;
    public int BeforeLines;
    public int AfterLines;
    public int ChangedLines;
    public int AddedLines;
    public int RemovedLines;
    public string DiffText;
}

// Collect pass results and per-vector restart rewrite counts from an optimization run.
sealed class OptimizerAnalysisReport
{
    public readonly List<OptimizerPassReport> Passes = new List<OptimizerPassReport>();
    public readonly Dictionary<int, int> RstRewriteCountsByVector = new Dictionary<int, int>();
    public int TotalRstRewrites;
}

// Keep file offsets distinct from CPU addresses when reporting placed function sizes.
sealed class FunctionSizeInfo
{
    public string Name;
    public int StartFileOffset;
    public int EndFileOffset;
    public int SizeBytes;
    public int Bank;
    public int CpuAddress;
    public string Section;
}

// Describe a placed symbol's bank, range, kind and memory section for exported metadata.
sealed class SymbolMetadataInfo
{
    public string Name;
    public int Bank;
    public int Start;
    public int End;
    public string Kind;
    public string Region;
    public string Section;
}

// Describe a variable's address, byte size and memory region; bank is optional.
sealed class VariableMetadataInfo
{
    public string Name;
    public int Address;
    public int Size;
    public string Region;
    public int? Bank;
}

// Associate a bank/address with source file, line and optional column/symbol information.
sealed class SourceLocationMetadataInfo
{
    public int Bank;
    public int Address;
    public string Path;
    public int Line;
    public int? Column;
    public string Symbol;
    public string Section;
}

// Store final layout measurements and metadata produced by assembly for downstream reports.
sealed class AssemblerAnalysisReport
{
    public readonly List<FunctionSizeInfo> FunctionSizes = new List<FunctionSizeInfo>();
    public readonly List<SymbolMetadataInfo> Symbols = new List<SymbolMetadataInfo>();
    public readonly List<VariableMetadataInfo> Variables = new List<VariableMetadataInfo>();
    public readonly List<SourceLocationMetadataInfo> SourceLocations = new List<SourceLocationMetadataInfo>();
    public int[] BankMaxPc = new int[0];
    public int RomSizeBytes;
    public int UsedBytes;
}

static class BuildReportUtil
{
    // Hash the supplied bytes as lowercase hexadecimal; null is treated as an empty byte sequence.
    public static string ComputeSha256Hex(byte[] data)
    {
        if (data == null) data = new byte[0];
        using (var sha = SHA256.Create())
        {
            var hash = sha.ComputeHash(data);
            return string.Concat(hash.Select(b => b.ToString("x2")));
        }
    }

    // Return an empty string for a missing/empty path; otherwise read and hash the
    // file. Existing-file read failures propagate rather than becoming a fake digest.
    public static string ComputeSha256HexOfFile(string path)
    {
        if (string.IsNullOrEmpty(path) || !File.Exists(path)) return "";
        return ComputeSha256Hex(File.ReadAllBytes(path));
    }

    // Apply the legacy 16 KiB fixed/switchable-window address convention: bank zero
    // is unchanged and later offsets map into 0x4000-0x7FFF. This helper is not a
    // general mapper-aware NES address translator; negative sentinel values pass through.
    public static int CpuAddrFromFileOffset(int fileOffset)
    {
        const int bankSize = 0x4000;
        if (fileOffset < 0) return fileOffset;
        if (fileOffset < bankSize) return fileOffset;
        return bankSize + (fileOffset & (bankSize - 1));
    }

    // Format an optional integer array as a comma-separated report field; empty input yields an empty string.
    public static string JoinInts(int[] values)
    {
        if (values == null || values.Length == 0) return "";
        return string.Join(",", values.Select(x => x.ToString()));
    }
}




