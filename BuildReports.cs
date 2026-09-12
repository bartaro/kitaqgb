using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;

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

sealed class AggregateCopyInfo
{
    public string Function;
    public string Type;
    public int SizeBytes;
    public string Strategy;
    public string Source;
}

sealed class RstSelectionInfo
{
    public int Vector;
    public string TargetLabel;
    public int Calls;
    public int NetBytes;
}

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

sealed class OptimizerAnalysisReport
{
    public readonly List<OptimizerPassReport> Passes = new List<OptimizerPassReport>();
    public readonly Dictionary<int, int> RstRewriteCountsByVector = new Dictionary<int, int>();
    public int TotalRstRewrites;
}

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

sealed class VariableMetadataInfo
{
    public string Name;
    public int Address;
    public int Size;
    public string Region;
    public int? Bank;
}

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
    public static string ComputeSha256Hex(byte[] data)
    {
        if (data == null) data = new byte[0];
        using (var sha = SHA256.Create())
        {
            var hash = sha.ComputeHash(data);
            return string.Concat(hash.Select(b => b.ToString("x2")));
        }
    }

    public static string ComputeSha256HexOfFile(string path)
    {
        if (string.IsNullOrEmpty(path) || !File.Exists(path)) return "";
        return ComputeSha256Hex(File.ReadAllBytes(path));
    }

    public static int CpuAddrFromFileOffset(int fileOffset)
    {
        const int bankSize = 0x4000;
        if (fileOffset < 0) return fileOffset;
        if (fileOffset < bankSize) return fileOffset;
        return bankSize + (fileOffset & (bankSize - 1));
    }

    public static string JoinInts(int[] values)
    {
        if (values == null || values.Length == 0) return "";
        return string.Join(",", values.Select(x => x.ToString()));
    }
}




