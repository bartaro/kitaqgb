using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Text;
using System.Threading.Tasks;


// --- Compatibility shim ---
// Fix for CS0411 when calling Expr.Match(Tag.Integer, out _) with a discard.
// Visual Studio cannot infer T for generic Match<T>() when the out argument is '_' (untyped).
static class ExprMatchCompatibility
{
    public static bool Match(this Expr expr, string tag, out int value)
        => expr.Match<int>(tag, out value);
}

class CodeGenerator
{
    // Distinguish explicit tile-map coordinates from window and background targets.
    enum CgbTileTargetKind
    {
        At,
        Window,
        Bg
    }

    // Retain the source declaration alongside its type and requested memory region for later resolution.
    sealed class ExternDeclInfo
    {
        public Expr Origin;
        public CType Type;
        public MemoryRegion Region;
    }
    // Keep a typed constant expression unevaluated until its dependencies can be resolved.
    sealed class PendingConstantInfo
    {
        public Expr Origin;
        public CType Type;
        public Expr ValueExpr;
    }
    // Describe a bank-switching call stub, including stack-ABI argument bytes when applicable.
    sealed class BankThunkInfo
    {
        public int Bank;
        public string Target;
        public bool IsStackCall;
        public int StackArgBytes;
    }

    // Expose the report through the active compiler session rather than a separate process-wide backing field.
    public static CodegenAnalysisReport LastReport
    {
        get { return Program.CurrentCodegenLastReport; }
        private set { Program.CurrentCodegenLastReport = value; }
    }

    // Accumulate call, copy, RST and ABI observations for the final code-generation report.
    readonly Dictionary<string, CallEdgeInfo> CallReportMap = new Dictionary<string, CallEdgeInfo>(StringComparer.Ordinal);
    readonly List<AggregateCopyInfo> AggregateCopyReport = new List<AggregateCopyInfo>();
    readonly List<RstSelectionInfo> RstSelections = new List<RstSelectionInfo>();
    readonly List<string> AbiIssues = new List<string>();
    int CgbRuntimeCheckCount = 0;
    int CgbGuardedWriteCount = 0;
    readonly HashSet<string> CgbGuardedRegisters = new HashSet<string>(StringComparer.Ordinal);
    readonly HashSet<string> StaticAssertDiagnosticDedupe = new HashSet<string>(StringComparer.Ordinal);
    readonly HashSet<string> ManualSvbkWarningDedupe = new HashSet<string>(StringComparer.Ordinal);

    // Keep nested emission transactions and inline/aggregate return destinations scoped independently.
    Stack<OutputTransaction> OutputStack = new Stack<OutputTransaction>();
    Stack<AsmOperand> InlineReturnLabels = new Stack<AsmOperand>();
    Stack<int> InlineStructReturnDestAddrs = new Stack<int>();
    Stack<int> StructReturnDestinationOverrideAddrs = new Stack<int>();

    // Tracks return-width of the most recently compiled call expression.
    // Needed for intrinsics returning u16 in HL (e.g. __readpadex), where
    // some compilation paths can mis-infer the semantic return type.
    bool LastCallReturnsHL = false;
    int LastStructReturnAddress = -1;
    
    // v9 __settile call-core injection flags
    bool UsedSetTileCore = false;
    bool UsedSetTileFastCore = false;
    bool UsedSetTileBulkCore = false;
    bool UsedSetTileBulkFastCore = false;
    bool UseScrollSplitHelpers = false;
    bool ScrollStateAllocated = false;
    bool UseCgbRuntimeDetectHelper = false;
    bool UseRuntimeAssertPanicHelper = false;

    Stack<AsmOperand> BreakLabels = new Stack<AsmOperand>();
    OutputTransaction Output => OutputStack.Peek();
    Dictionary<string, CFunctionInfo> Functions = new Dictionary<string, CFunctionInfo>();
    Dictionary<string, AggregateInfo> AggregateTypes = new Dictionary<string, AggregateInfo>();
    readonly Dictionary<string, PendingConstantInfo> PendingConstants = new Dictionary<string, PendingConstantInfo>(StringComparer.Ordinal);
    readonly HashSet<string> PendingConstantsResolving = new HashSet<string>(StringComparer.Ordinal);

    Dictionary<string, int> VariableUsageCounts = new Dictionary<string, int>();
    const int HotThreshold = 8;
    // Treat a recorded usage count of at least eight as hot for allocation decisions.
    bool IsHotVar(string name) =>
        VariableUsageCounts.TryGetValue(name, out int c) && c >= HotThreshold;


    // ====== Config ======
    static readonly bool EnableWram1Fallback = true;

    // ====== Memory Regions ======
    AllocationRegion HramRegion = new AllocationRegion("HRAM", 0xFF80, 0xFFFE);
    AllocationRegion Wram0Region = new AllocationRegion("WRAM0", 0xC000, 0xCFFF);
    AllocationRegion Wram1Region = new AllocationRegion("WRAM1", 0xD000, 0xDFFF);
    AllocationRegion OamRegion = new AllocationRegion("OAM", 0xFE00, 0xFE9F);
    readonly AllocationRegion[] WramXBankRegions = new AllocationRegion[8];

    // ====== Function-frame allocation safety (A4 ABI stability) ======
    // Locals are allocated in HRAM/WRAM (static addresses) rather than on the stack.
    // Therefore, allocations MUST NOT be reused across different functions, because
    // function calls can nest at runtime and would clobber the caller's locals.
    // Direct recursion under the legacy ABI is also unsafe for the same reason, and
    // because legacy parameters live in fixed slots as well.
    // We still keep "scope rollback" inside a function (so non-overlapping scopes can
    // reuse the same slots). To make both work, we track the maximum high-water mark
    // during a function's codegen, then commit that maximum after finishing the function.
    bool TrackFunctionFrame;
    int FuncMaxHramNext;
    int FuncMaxWram0Next;
    int FuncMaxWram1Next;

    // Share WRAMX bank 1 with the ordinary WRAM1 allocator; give banks 2-7 independent cursors over the same CPU window.
    public CodeGenerator()
    {
        WramXBankRegions[1] = Wram1Region;
        for (int bank = 2; bank <= 7; bank++)
            WramXBankRegions[bank] = new AllocationRegion(string.Format("WRAMX[{0}]", bank), 0xD000, 0xDFFF);

        ApplyStackPolicyToAllocationRegions();
    }

    // Reduce the selected work-RAM allocator limit to leave the configured stack area available.
    void ApplyStackPolicyToAllocationRegions()
    {
        if (Program.StackBank == Program.StackBankMode.Fixed)
            Wram0Region.Top = Math.Min(Wram0Region.Top, Program.EffectiveStackAutoLimit);
        else
            Wram1Region.Top = Math.Min(Wram1Region.Top, Program.EffectiveStackAutoLimit);
    }

    // Start a function high-water mark from the current HRAM, WRAM0 and WRAM1 allocation cursors.
    void BeginFunctionFrameTracking()
    {
        TrackFunctionFrame = true;
        FuncMaxHramNext = HramRegion.Next;
        FuncMaxWram0Next = Wram0Region.Next;
        FuncMaxWram1Next = Wram1Region.Next;
    }

    // End peak tracking and preserve the largest cursor reached even after lexical-scope rollback.
    void EndFunctionFrameTrackingAndCommit()
    {
        TrackFunctionFrame = false;
        // After EndScope(), region.Next is rolled back to the function entry snapshot.
        // Commit the high-water mark so subsequent functions do not reuse the same slots.
        HramRegion.Next = Math.Max(HramRegion.Next, FuncMaxHramNext);
        Wram0Region.Next = Math.Max(Wram0Region.Next, FuncMaxWram0Next);
        Wram1Region.Next = Math.Max(Wram1Region.Next, FuncMaxWram1Next);
    }

    // Update peaks only for the three shared allocator objects; other banked regions and OAM are not tracked here.
    void TrackAllocPeak(AllocationRegion region)
    {
        if (!TrackFunctionFrame) return;
        if (object.ReferenceEquals(region, HramRegion)) FuncMaxHramNext = Math.Max(FuncMaxHramNext, region.Next);
        else if (object.ReferenceEquals(region, Wram0Region)) FuncMaxWram0Next = Math.Max(FuncMaxWram0Next, region.Next);
        else if (object.ReferenceEquals(region, Wram1Region)) FuncMaxWram1Next = Math.Max(FuncMaxWram1Next, region.Next);
    }


    AsmOperand RegisterL;
    AsmOperand RegisterH;
    WideOperand RegisterHL;

    // farcall support
    AsmOperand RomBankVar;
    AsmOperand RomBankSavedVar;
    AsmOperand StructReturnPtrLoVar;
    AsmOperand StructReturnPtrHiVar;
    bool UseBankSwitchBank0Helper;
    bool UseFarMemcpyBank0Helper;
    bool UseFarCallPointerBank0Helper;
    readonly Dictionary<string, BankThunkInfo> Bank0ThunkMap = new Dictionary<string, BankThunkInfo>();
    readonly Dictionary<string, string> Bank0RuntimeFarcallThunkMap = new Dictionary<string, string>();
    // Reserve eight nested bank-thunk entries and encode their depth relative to 0x80.
    const int BankThunkStackDepth = 8;
    const int BankThunkDepthEncodedBase = 0x80;
    const int BankThunkDepthEncodedLimit = BankThunkDepthEncodedBase + BankThunkStackDepth;
    int RomBankSavedAddr = -1;
    int OamDmaStubAddr = -1;
    int StructReturnPtrAddr = -1;
    int BankThunkSpAddr = -1;
    int BankThunkBankBaseAddr = -1;
    int BankThunkRetLoBaseAddr = -1;
    int BankThunkRetHiBaseAddr = -1;
    AsmOperand CriticalDepthVar;
    AsmOperand RngStateLoVar;
    AsmOperand RngStateHiVar;
    AsmOperand ScrollBgXCurVar;
    AsmOperand ScrollBgYCurVar;
    AsmOperand ScrollWinXCurVar;
    AsmOperand ScrollWinYCurVar;
    AsmOperand ScrollBgXNextVar;
    AsmOperand ScrollBgYNextVar;
    AsmOperand ScrollWinXNextVar;
    AsmOperand ScrollWinYNextVar;
    AsmOperand ScrollDirtyVar;
    AsmOperand ScrollWinVisibleVar;
    AsmOperand ScrollSplitCountVar;
    AsmOperand ScrollSplitEnabledVar;
    AsmOperand ScrollSplitIndexVar;
    AsmOperand ScrollSplitTmpLyVar;
    AsmOperand ScrollSplitTmpScxVar;
    AsmOperand ScrollSplitTmpScyVar;
    AsmOperand ScrollSplitTmpWxVar;
    AsmOperand ScrollSplitTmpWyVar;
    AsmOperand ScrollSplitTmpFlagsVar;
    int ScrollSplitTableAddr = -1;

    // Define pending-scroll flags and the six-byte, eight-entry split table format used by the runtime helpers.
    const int ScrollDirtyBgMask = 0x01;
    const int ScrollDirtyWinMask = 0x02;
    const int ScrollSplitFlagUseBg = 0x01;
    const int ScrollSplitFlagUseWin = 0x02;
    const int ScrollSplitFlagWinShow = 0x04;
    const int ScrollSplitFlagWinHide = 0x08;
    const int ScrollSplitFlagBgColor0 = 0x10;
    const int ScrollSplitFlagMask = 0x1F;
    const int ScrollSplitMaxEntries = 8;
    const int ScrollSplitEntrySize = 6;

    string CurrentFunctionName = null;
    int CurrentFunctionBank = 1;
    CType ReturnType = null;
    int NextLabelNumber = 0;
    // stack ABI: per-function saved SP base (only when needed)
    int CurrentStackBaseAddr = -1;
    bool CurrentFunctionIsStackCall = false;

    // ====== Stack usage estimate from emitted instructions for -Zcheck ======
    bool TrackStackUsage;
    int CurStackBytes;
    int MaxStackBytes;

    // Reset the current emitted stack depth and its per-function peak.
    void BeginStackUsageTracking()
    {
        TrackStackUsage = true;
        CurStackBytes = 0;
        MaxStackBytes = 0;
    }

    // Stop recording instructions and return the peak accumulated for this function.
    int EndStackUsageTracking()
    {
        TrackStackUsage = false;
        return MaxStackBytes;
    }

    // Estimate stack growth in emission order from recognized pushes, pops, SP adjustments and call return addresses.
    // This does not analyze control-flow joins or add callee/interrupt stack requirements.
    void TrackStackInstr(string mnemonic, AsmOperand operand)
    {
        if (!TrackStackUsage) return;

        if (mnemonic == null) return;

        // PUSH/POP change SP by 2 bytes.
        if (mnemonic.StartsWith("PUSH_"))
        {
            CurStackBytes += 2;
            if (CurStackBytes > MaxStackBytes) MaxStackBytes = CurStackBytes;
            return;
        }
        if (mnemonic.StartsWith("POP_"))
        {
            CurStackBytes -= 2;
            if (CurStackBytes < 0) CurStackBytes = 0;
            return;
        }

        // add sp, e8 (relative signed).
        if (mnemonic == "ADD_SP_IMM" && operand != null && operand.Mode == AddressMode.Relative)
        {
            int e8 = operand.Offset;
            if (e8 < 0)
            {
                CurStackBytes += -e8;
                if (CurStackBytes > MaxStackBytes) MaxStackBytes = CurStackBytes;
            }
            else if (e8 > 0)
            {
                CurStackBytes -= e8;
                if (CurStackBytes < 0) CurStackBytes = 0;
            }
            return;
        }

        // CALL/RST temporarily push return address (2 bytes). Track peak only.
        if (mnemonic == "CALL" || mnemonic.StartsWith("RST_"))
        {
            int peak = CurStackBytes + 2;
            if (peak > MaxStackBytes) MaxStackBytes = peak;
            return;
        }
    }

    // Banking: whether the ROM actually uses switchable banks (>=2).
    bool BankSwitchingUsed;


    LexicalScope CurrentScope;
    LoopScope Loop = null;

    // Safety boundary (used by -Zcheck-bounds etc.)
    int UnsafeDepth = 0;
    bool InUnsafe => UnsafeDepth > 0;

    AsmOperand CheckTrapLabel = null;

	// Planned alignment for readonly data symbols (computed early, before codegen).
	// Key: symbol name (label), Value: alignment in bytes (power-of-two). 0/1 means no special alignment.
    readonly Dictionary<string, int> ReadonlyDataPlannedAlign = new Dictionary<string, int>(StringComparer.Ordinal);
    readonly Dictionary<string, int> ReadonlyDataPlannedBank = new Dictionary<string, int>(StringComparer.Ordinal);
    readonly HashSet<string> NearReadonlyDataNames = new HashSet<string>(StringComparer.Ordinal);

    static readonly bool ShowVerboseComments = true;

    // Apply a configured bank override to recognized function definitions/prototypes; retain the declaration bank otherwise.
    int ResolveTopLevelRomBank(int declaredBank, Expr decl)
    {
        if (decl == null) return declaredBank;

        string name;
        CType retType;
        FieldInfo[] fields;
        Expr body;
        int mustCheck;

        if (decl.Match(Tag.Function, out retType, out name, out fields, out mustCheck, out body) ||
            (mustCheck = 0) == 0 && decl.Match(Tag.Function, out retType, out name, out fields, out body))
        {
            if (Program.TryGetFunctionBankOverride(name, out int bank)) return bank;
            return declaredBank;
        }

        if (decl.Match(Tag.FunctionDecl, out retType, out name, out fields, out mustCheck) ||
            (mustCheck = 0) == 0 && decl.Match(Tag.FunctionDecl, out retType, out name, out fields))
        {
            if (Program.TryGetFunctionBankOverride(name, out int bank)) return bank;
            return declaredBank;
        }

        if (decl.Match(Tag.InlineFunction, out retType, out name, out fields, out mustCheck, out body) ||
            (mustCheck = 0) == 0 && decl.Match(Tag.InlineFunction, out retType, out name, out fields, out body))
        {
            if (Program.TryGetFunctionBankOverride(name, out int bank)) return bank;
            return declaredBank;
        }

        return declaredBank;
    }

    // Strip up to sixteen supported declaration wrappers while carrying the outer source position inward.
    Expr UnwrapTopLevelDeclForPlacement(Expr decl)
    {
        Expr d = decl;
        int guard = 0;
        while (d != null && guard++ < 16)
        {
            Expr inner;
            int i;
            string s;
            if (d.Match(Tag.FixedBank, out i, out inner) ||
                d.Match(Tag.FixedOrder, out i, out inner) ||
                d.Match(Tag.Bank, out i, out inner) ||
                d.Match(Tag.DeclAlign, out i, out inner) ||
                d.Match(Tag.DeclSection, out s, out inner) ||
                d.Match(Tag.Static, out inner) ||
                d.Match(Tag.Unsafe, out inner) ||
                d.Match(Tag.StackCall, out inner))
            {
                d = inner.WithSource(d.Source);
                continue;
            }
            break;
        }
        return d;
    }

    // Collect readonly symbols first, then mark those used without the explicit bank-argument pattern as near data.
    void AnalyzeReadonlyDataPlacement(Expr[] declarations)
    {
        NearReadonlyDataNames.Clear();
        if (declarations == null || declarations.Length == 0) return;

        var readonlyNames = new HashSet<string>(StringComparer.Ordinal);
        foreach (var decl in declarations)
        {
            Expr d = UnwrapTopLevelDeclForPlacement(decl);
            if (TryMatchReadonlyDataDecl(d, out CType _rdType, out string rdName, out Expr[] _rdValues) &&
                !string.IsNullOrEmpty(rdName))
            {
                readonlyNames.Add(rdName);
            }
        }

        if (readonlyNames.Count == 0) return;

        foreach (var decl in declarations)
            ScanReadonlyDataUsage(decl, readonlyNames);
    }

    // Use syntax to distinguish near references from __bankof queries and paired far-call arguments.
    // This is a placement heuristic, not whole-program pointer-flow analysis.
    void ScanReadonlyDataUsage(Expr expr, HashSet<string> readonlyNames)
    {
        if (expr == null || readonlyNames == null || readonlyNames.Count == 0) return;

        if (expr.MatchAny(Tag.Call, out Expr callTarget, out Expr[] callArgs))
        {
            string callName = null;
            callTarget?.Match(Tag.Name, out callName);

            if (callName == "__bankof")
            {
                // Explicit far-ROM use: the symbol is being handled together with its bank.
                return;
            }

            // A bare readonly argument paired with __bankof of that exact symbol in the same call is treated as explicit far use.
            var bankofSiblingNames = new HashSet<string>(StringComparer.Ordinal);
            if (callArgs != null)
            {
                foreach (var arg in callArgs)
                {
                    if (arg == null) continue;
                    if (arg.MatchAny(Tag.Call, out Expr bankTarget, out Expr[] bankArgs) &&
                        bankTarget.Match(Tag.Name, out string bankFuncName) &&
                        bankFuncName == "__bankof" &&
                        bankArgs != null &&
                        bankArgs.Length == 1 &&
                        bankArgs[0].Match(Tag.Name, out string bankSymName) &&
                        readonlyNames.Contains(bankSymName))
                    {
                        bankofSiblingNames.Add(bankSymName);
                    }
                }
            }

            ScanReadonlyDataUsage(callTarget, readonlyNames);
            if (callArgs != null)
            {
                foreach (var arg in callArgs)
                {
                    if (arg == null) continue;
                    if (arg.Match(Tag.Name, out string argName) &&
                        readonlyNames.Contains(argName) &&
                        bankofSiblingNames.Contains(argName))
                    {
                        continue;
                    }
                    ScanReadonlyDataUsage(arg, readonlyNames);
                }
            }
            return;
        }

        if (expr.Match(Tag.Name, out string name) && readonlyNames.Contains(name))
        {
            NearReadonlyDataNames.Add(name);
            return;
        }

        foreach (object arg in expr.GetArgs().Skip(1))
        {
            if (arg is Expr child)
            {
                ScanReadonlyDataUsage(child, readonlyNames);
            }
            else if (arg is Expr[] children)
            {
                foreach (Expr childExpr in children)
                    ScanReadonlyDataUsage(childExpr, readonlyNames);
            }
        }
    }

    // Generate assembly and required runtime helpers, optimize the result, then publish the collected ABI/codegen report.
    public static List<Expr> CompileAll(Expr program)
    {
        CodeGenerator converter = new CodeGenerator();
        converter.CompileProgram(program);

        var rawLines = converter.Output.Lines;

        // --- Debug-only safety traps ---
        if (Program.CheckBounds || Program.CheckMemCopy || Program.CheckStack || Program.CheckBankCalls || Program.CheckSliceBounds)
        {
            // Ensure a single, shareable trap label exists.
            if (converter.CheckTrapLabel == null)
                converter.CheckTrapLabel = converter.MakeUniqueLabel("kq_trap_check");
        }

        // --- RST call compression (hot CALL targets -> RST stubs) ---
        // 1) Count actual CALL instructions (after inlining/intrinsics expansion)
        // 2) Pick a few hot targets and emit $rst_map directives
        // 3) Optimizer will rewrite CALL -> RST_xx, Assembler will install JP stubs
        if (!Program.RstDisable)
        {
            foreach (var ent in SelectRstHotEntriesFromAssembly(rawLines))
            {
                rawLines.Insert(0, Expr.Make(Tag.RstMap, ent.Vector, ent.TargetLabel));
                converter.RstSelections.Add(new RstSelectionInfo
                {
                    Vector = ent.Vector,
                    TargetLabel = ent.TargetLabel,
                    Calls = ent.Calls,
                    NetBytes = ent.NetBytes
                });
            }
        }
        else
        {
            if (Program.EnableDebugOutput)
                Program.WriteErrorLine("[KITAQGB] RST disabled (default/--rst-disable)");
        }
        // Inject v9 __settile call-core routines if any related intrinsics were used.
        converter.AppendSetTileV9RoutinesIfUsed(rawLines);
        // Inject scroll split runtime helpers when scroll split intrinsics are used.
        converter.AppendScrollHelpersIfUsed(rawLines);
        // Inject bank0 helper/trampoline routines used by cross-bank calls and __bankswitch.
        converter.AppendBankHelpersIfUsed(rawLines);
        // Inject CGB runtime-detect helper if CGB-safe intrinsics were used.
        converter.AppendCgbHelpersIfUsed(rawLines);
        // Inject debug trap entrypoint into bank0 when -Zcheck* is enabled.
        converter.AppendCheckTrapIfUsed(rawLines);
        // Inject runtime assert panic helper when __assert() is used.
        converter.AppendRuntimeAssertHelperIfUsed(rawLines);


        var optimizedLines = Optimizer.Optimize(rawLines, Program.OptLevel);

        converter.RunAbiConsistencyChecks();
        LastReport = converter.BuildCodegenAnalysisReport();

        return optimizedLines;
    }

    // Produce deterministic function/call/copy/RST ordering and include the accumulated guard and ABI observations.
    CodegenAnalysisReport BuildCodegenAnalysisReport()
    {
        var report = new CodegenAnalysisReport();

        foreach (var kv in Functions.OrderBy(x => x.Key, StringComparer.Ordinal))
        {
            string name = kv.Key;
            CFunctionInfo fi = kv.Value;
            int[] paramSizes = new int[fi.Parameters == null ? 0 : fi.Parameters.Length];
            for (int i = 0; i < paramSizes.Length; i++)
            {
                paramSizes[i] = SizeOfTypeLoose(fi.Parameters[i].Type);
            }

            report.Functions.Add(new FunctionAbiInfo
            {
                Name = name,
                Bank = fi.RomBank,
                HasFixedBank = fi.HasFixedBank,
                PlacementOrder = fi.PlacementOrder,
                HasFixedOrder = fi.HasFixedOrder,
                IsPrototype = fi.IsPrototype,
                IsInline = fi.IsInline,
                IsStackCall = fi.IsStackCall,
                IsFastCall = fi.IsFastCall,
                ReturnSize = SizeOfTypeLoose(fi.ReturnType),
                ParamSizes = paramSizes
            });
        }

        foreach (var call in CallReportMap.Values
            .OrderBy(x => x.Caller, StringComparer.Ordinal)
            .ThenBy(x => x.Callee, StringComparer.Ordinal)
            .ThenBy(x => x.Kind, StringComparer.Ordinal))
        {
            report.Calls.Add(call);
        }

        foreach (var copy in AggregateCopyReport
            .OrderBy(x => x.Function, StringComparer.Ordinal)
            .ThenBy(x => x.Source, StringComparer.Ordinal)
            .ThenBy(x => x.Type, StringComparer.Ordinal))
        {
            report.AggregateCopies.Add(copy);
        }

        foreach (var rst in RstSelections.OrderBy(x => x.Vector))
        {
            report.RstSelections.Add(rst);
        }

        foreach (var issue in AbiIssues)
        {
            report.AbiIssues.Add(issue);
        }

        report.CgbRuntimeCheckCount = CgbRuntimeCheckCount;
        report.CgbGuardedWriteCount = CgbGuardedWriteCount;
        foreach (var r in CgbGuardedRegisters.OrderBy(x => x, StringComparer.Ordinal))
        {
            report.CgbGuardedRegisters.Add(r);
        }

        return report;
    }

    // Estimate report widths with two-byte pointers/enums and known aggregate layouts.
    // Unknown aggregate sizes and invalid or unavailable array dimensions use small fallbacks; this is not strict sizeof validation.
    int SizeOfTypeLoose(CType type)
    {
        if (type == null || type == CType.Void) return 0;
        if (type.IsPointer) return 2;
        if (type.IsEnum) return 2;
        if (type.IsSimple && (type.SimpleType == CSimpleType.UInt16 || type.SimpleType == CSimpleType.Int16)) return 2;
        if (type.IsArray)
        {
            int elem = SizeOfTypeLoose(type.Subtype);
            int dim = 1;
            if (type.Tag == CTypeTag.Array) dim = Math.Max(1, type.Dimension);
            else if (type.Tag == CTypeTag.ArrayWithDimensionExpression)
            {
                dim = 1;
                try
                {
                    dim = Math.Max(1, CalculateConstantExpression(type.DimensionExpression));
                }
                catch
                {
                    dim = 1;
                }
            }
            return Math.Max(1, elem) * dim;
        }
        if (type.IsStructOrUnion)
        {
            AggregateInfo ai;
            if (AggregateTypes.TryGetValue(type.Name, out ai) && ai.TotalSize >= 0) return ai.TotalSize;
            return 1;
        }
        return 1;
    }

    // Recognize the signed eight- and sixteen-bit scalar types.
    static bool IsSignedIntegerType(CType t)
    {
        return t != null && t.IsSimple &&
               (t.SimpleType == CSimpleType.Int8 || t.SimpleType == CSimpleType.Int16);
    }

    // Classify enums with unsigned eight- and sixteen-bit scalar types for this backend.
    static bool IsUnsignedIntegerType(CType t)
    {
        if (t == null) return false;
        if (t.IsEnum) return true;
        return t.IsSimple && (t.SimpleType == CSimpleType.UInt8 || t.SimpleType == CSimpleType.UInt16);
    }

    // Accept integer scalars and enums, excluding pointer and aggregate types.
    static bool IsIntegerLike(CType t)
    {
        return t != null && (t.IsInteger || t.IsEnum);
    }

    // Use a sixteen-bit result, signed whenever either operand is a signed integer type.
    static CType PromoteIntegerBinaryType(CType leftType, CType rightType)
    {
        bool hasSigned = IsSignedIntegerType(leftType) || IsSignedIntegerType(rightType);
        return hasSigned ? CType.Int16 : CType.UInt16;
    }

    // Keep the low eight bits without saturating.
    static int ToUInt8(int v) => v & 0xFF;
    // Keep the low sixteen bits without saturating.
    static int ToUInt16(int v) => v & 0xFFFF;
    // Interpret the low byte as a signed two's-complement value.
    static int ToInt8(int v)
    {
        int b = v & 0xFF;
        return (b >= 0x80) ? (b - 0x100) : b;
    }
    // Interpret the low word as a signed two's-complement value.
    static int ToInt16(int v)
    {
        int w = v & 0xFFFF;
        return (w >= 0x8000) ? (w - 0x10000) : w;
    }

    // Apply the target scalar width and signedness; pointers/enums use unsigned words and other types retain the input.
    static int NormalizeConstValueForType(int value, CType t)
    {
        if (t == null) return value;
        if (t.IsPointer) return ToUInt16(value);
        if (t.IsEnum) return ToUInt16(value);
        if (!t.IsSimple) return value;

        if (t.SimpleType == CSimpleType.UInt8) return ToUInt8(value);
        if (t.SimpleType == CSimpleType.Int8) return ToInt8(value);
        if (t.SimpleType == CSimpleType.UInt16) return ToUInt16(value);
        if (t.SimpleType == CSimpleType.Int16) return ToInt16(value);
        return value;
    }

    // Use signed comparison only for integer-like operands promoted to a signed word; pointer comparisons stay unsigned.
    bool ShouldUseSignedComparison(Expr left, Expr right)
    {
        if (left == null || right == null) return false;
        CType lt = TypeOf(left) ?? CType.UInt8;
        CType rt = TypeOf(right) ?? CType.UInt8;
        if (lt == null || rt == null) return false;
        if (lt.IsPointer || rt.IsPointer) return false;
        if (!IsIntegerLike(lt) || !IsIntegerLike(rt)) return false;
        CType promoted = PromoteIntegerBinaryType(lt, rt);
        return IsSignedIntegerType(promoted);
    }

    // Choose signed arithmetic from the inferred integer/enum operand types, using unsigned-byte type fallbacks.
    bool ShouldUseSignedArithmetic(Expr left, Expr right)
    {
        if (left == null || right == null) return false;
        CType lt = TypeOf(left) ?? CType.UInt8;
        CType rt = TypeOf(right) ?? CType.UInt8;
        if (lt == null || rt == null) return false;
        if (!IsIntegerLike(lt) || !IsIntegerLike(rt)) return false;
        return IsSignedIntegerType(PromoteIntegerBinaryType(lt, rt));
    }

    // Identify a repeated assertion diagnostic by source position, rendered condition and user message.
    string BuildStaticAssertDedupeKey(Expr origin, Expr condition, string message)
    {
        string src = origin == null ? "" : origin.Source.ToString();
        string cond = condition == null ? "" : condition.Show();
        string msg = message ?? "";
        return src + "|" + cond + "|" + msg;
    }

    // Report each assertion failure once, including raw and type-normalized values even if condition type inference fails.
    void ReportStaticAssertFailure(Expr origin, Expr condition, string message, int rawValue)
    {
        string key = BuildStaticAssertDedupeKey(origin, condition, message);
        if (!StaticAssertDiagnosticDedupe.Add(key)) return;

        CType condType = null;
        try { condType = TypeOf(condition); } catch { condType = null; }
        int normalized = NormalizeConstValueForType(rawValue, condType);
        string renderedExpr = condition == null ? "<null>" : condition.Show();
        string renderedType = condType == null ? "<unknown>" : condType.Show();
        string extra = string.Format("expr='{0}', value={1} (raw={2}), type={3}",
            renderedExpr, normalized, rawValue, renderedType);

        string text;
        if (string.IsNullOrEmpty(message))
            text = "static_assert failed: " + extra;
        else
            text = "static_assert failed: " + message + " [" + extra + "]";

        Error(origin, ErrorCode.StaticAssertFailed, text);
    }

    // Group calls by caller/callee, their banks, call kind and thunk/farcall routing.
    string BuildCallEdgeKey(string caller, string callee, int callerBank, int calleeBank, string kind, bool viaThunk, bool viaFarcall)
    {
        return caller + "|" + callee + "|" + callerBank + "|" + calleeBank + "|" + kind + "|" + (viaThunk ? "1" : "0") + "|" + (viaFarcall ? "1" : "0");
    }

    // Count equivalent call edges and retain copied argument widths and the source position of their most recent occurrence.
    void RecordCallEdge(Expr origin, string callee, int calleeBank, string kind, bool viaThunk, bool viaFarcall, int[] actualArgSizes, int[] expectedArgSizes)
    {
        string caller = string.IsNullOrEmpty(CurrentFunctionName) ? "<global>" : CurrentFunctionName;
        int callerBank = CurrentFunctionBank;
        string key = BuildCallEdgeKey(caller, callee ?? "<unknown>", callerBank, calleeBank, kind ?? "direct", viaThunk, viaFarcall);

        CallEdgeInfo edge;
        if (!CallReportMap.TryGetValue(key, out edge))
        {
            edge = new CallEdgeInfo
            {
                Caller = caller,
                Callee = callee ?? "<unknown>",
                CallerBank = callerBank,
                CalleeBank = calleeBank,
                Kind = kind ?? "direct",
                ViaThunk = viaThunk,
                ViaFarcall = viaFarcall,
                Count = 0
            };
            CallReportMap.Add(key, edge);
        }

        edge.Count++;
        edge.LastActualArgSizes = actualArgSizes == null ? new int[0] : actualArgSizes.ToArray();
        edge.LastExpectedArgSizes = expectedArgSizes == null ? new int[0] : expectedArgSizes.ToArray();
        edge.LastSource = origin == null ? "" : origin.Source.ToString();
    }

    // Append report issues for unsupported stack-parameter/return widths and the last argument widths recorded on each edge.
    // This method records strings rather than raising compiler errors; extra arguments with no expected width are not flagged here.
    void RunAbiConsistencyChecks()
    {
        foreach (var kv in Functions)
        {
            string name = kv.Key;
            CFunctionInfo fi = kv.Value;

            if (fi.IsStackCall && fi.Parameters != null)
            {
                for (int i = 0; i < fi.Parameters.Length; i++)
                {
                    int sz = SizeOfTypeLoose(fi.Parameters[i].Type);
                    if (sz != 1 && sz != 2)
                    {
                        AbiIssues.Add("stack ABI parameter must be 1 or 2 bytes: " + name + " param#" + i + " size=" + sz);
                    }
                }
            }

            int ret = SizeOfTypeLoose(fi.ReturnType);
            if (ret > 2)
            {
                AbiIssues.Add("return size > 2 bytes is not ABI-safe on GB: " + name + " returnSize=" + ret);
            }
        }

        foreach (var edge in CallReportMap.Values)
        {
            var actual = edge.LastActualArgSizes ?? new int[0];
            var expected = edge.LastExpectedArgSizes ?? new int[0];
            int n = Math.Max(actual.Length, expected.Length);
            for (int i = 0; i < n; i++)
            {
                int a = (i < actual.Length) ? actual[i] : -1;
                int e = (i < expected.Length) ? expected[i] : -1;
                if (a != e && e > 0)
                {
                    AbiIssues.Add("arg-size mismatch: " + edge.Caller + " -> " + edge.Callee + " arg#" + i + " actual=" + a + " expected=" + e);
                }
            }
        }
    }

    // Inject referenced tile-write helpers ahead of ordinary code, using global function symbols in fixed bank zero.
    void AppendSetTileV9RoutinesIfUsed(List<Expr> rawLines)
    {
        // Robust detection: in some compile paths the "UsedSetTile*" flags can be missed.
        // Also scan symbolic operands in the already-emitted assembly for helper references.
        bool needCore = UsedSetTileCore;
        bool needFast = UsedSetTileFastCore;
        bool needBulk = UsedSetTileBulkCore;
        bool needBulkFast = UsedSetTileBulkFastCore;

        if (!needCore || !needFast || !needBulk || !needBulkFast)
        {
            foreach (var e in rawLines)
            {
                string m; AsmOperand o;
                if (e.Match(Tag.Asm, out m, out o))
                {
                    if (o != null && o.Base.HasValue)
                    {
                        var b = o.Base.Value;
                        if (!needCore && b == "__settile_core") needCore = true;
                        if (!needFast && b == "__settile_fast_core") needFast = true;
                        if (!needBulk && b == "__settile_bulk_core") needBulk = true;
                        if (!needBulkFast && b == "__settile_bulk_fast_core") needBulkFast = true;
                    }
                }
                if (needCore && needFast && needBulk && needBulkFast) break;
            }
        }

        if (!needCore && !needFast && !needBulk && !needBulkFast) return;

        // NOTE: These helper routines must live in bank0.
        // Insert them at the beginning of the program (right after any $rst_map directives)
        // to guarantee bank0 residency even if the program doesn't explicitly emit $skip_to 0x4000.
        var injected = new List<Expr>();

        injected.Add(Expr.Make(Tag.Comment, "[KITAQGB] injected v9 __settile call-core routines"));

        // IMPORTANT:
        // These entrypoints are called from other C functions later in the program.
        // The assembler clears Tag.Label symbols at every Tag.Function boundary (local-label scope).
        // If an entrypoint is emitted as Tag.Label, it will be removed when the next C function begins,
        // and later CALL/JMP fixups will remain unresolved.
        // Emit entrypoints as Tag.Function so they remain global symbols.

        // Write one tile after coordinate checks; with the LCD on, wait for STAT mode 0/1 under DI and issue EI on return.
        // The LCD-on path does not preserve an already-disabled interrupt state.
        if (needCore)
        {
            injected.Add(Expr.Make(Tag.Function, "__settile_core"));

            // Inputs: B=x (u8), A=y (u8), C=tile (u8)
            var ret = new AsmOperand("_kitaqgb_settile_v9_core_ret", AddressMode.Absolute);
            var write_now = new AsmOperand("_kitaqgb_settile_v9_core_write_now", AddressMode.Absolute);
            var wait = new AsmOperand("_kitaqgb_settile_v9_core_wait", AddressMode.Absolute);

            // Bounds check x<32, y<32
            injected.Add(Expr.MakeAsm("LD_D_A")); // save y
            injected.Add(Expr.MakeAsm("LD_A_B"));
            injected.Add(Expr.MakeAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("JP_NC", ret));
            injected.Add(Expr.MakeAsm("LD_A_D"));
            injected.Add(Expr.MakeAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("JP_NC", ret));

            // If LCDC bit7 is 0, write immediately (no wait)
            injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem))); // LCDC
            injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("JR_Z", write_now));

            // LCD ON: make the whole compute+write atomic
            injected.Add(Expr.MakeAsm("DI"));

            // HL = 0x9800 + y*32 + x
            injected.Add(Expr.MakeAsm("LD_A_D"));
            injected.Add(Expr.MakeAsm("LD_L_A"));
            injected.Add(Expr.MakeAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));

            injected.Add(Expr.MakeAsm("LD_DE_IMM", new AsmOperand(0x9800, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("ADD_HL_DE"));

            injected.Add(Expr.MakeAsm("LD_A_B"));
            injected.Add(Expr.MakeAsm("LD_E_A"));
            injected.Add(Expr.MakeAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("ADD_HL_DE"));

            // Wait STAT bit1 cleared (mode 0/1)
            injected.Add(Expr.Make(Tag.Label, wait.Base.Value));
            injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem))); // STAT
            injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("JR_NZ", wait));

            injected.Add(Expr.MakeAsm("LD_HL_C"));
            injected.Add(Expr.MakeAsm("EI"));
            injected.Add(Expr.MakeAsm("RET"));

            // LCD OFF fast write
            injected.Add(Expr.Make(Tag.Label, write_now.Base.Value));
            injected.Add(Expr.MakeAsm("LD_A_D"));
            injected.Add(Expr.MakeAsm("LD_L_A"));
            injected.Add(Expr.MakeAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));

            injected.Add(Expr.MakeAsm("LD_DE_IMM", new AsmOperand(0x9800, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("ADD_HL_DE"));

            injected.Add(Expr.MakeAsm("LD_A_B"));
            injected.Add(Expr.MakeAsm("LD_E_A"));
            injected.Add(Expr.MakeAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("ADD_HL_DE"));

            injected.Add(Expr.MakeAsm("LD_HL_C"));
            injected.Add(Expr.MakeAsm("RET"));

            // Out-of-range return
            injected.Add(Expr.Make(Tag.Label, ret.Base.Value));
            injected.Add(Expr.MakeAsm("RET"));
        }

        // Check coordinates but omit LCD polling and interrupt changes; the caller must provide a valid VRAM access period.
        if (needFast)
        {
            injected.Add(Expr.Make(Tag.Function, "__settile_fast_core"));

            // Inputs: B=x (u8), A=y (u8), C=tile (u8)
            var ret = new AsmOperand("_kitaqgb_settile_v9_fast_ret", AddressMode.Absolute);

            // Bounds check x<32, y<32
            injected.Add(Expr.MakeAsm("LD_D_A")); // save y
            injected.Add(Expr.MakeAsm("LD_A_B"));
            injected.Add(Expr.MakeAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("JP_NC", ret));
            injected.Add(Expr.MakeAsm("LD_A_D"));
            injected.Add(Expr.MakeAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("JP_NC", ret));

            // HL = 0x9800 + y*32 + x
            injected.Add(Expr.MakeAsm("LD_L_A"));
            injected.Add(Expr.MakeAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));
            injected.Add(Expr.MakeAsm("ADD_HL_HL"));

            injected.Add(Expr.MakeAsm("LD_DE_IMM", new AsmOperand(0x9800, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("ADD_HL_DE"));

            injected.Add(Expr.MakeAsm("LD_A_B"));
            injected.Add(Expr.MakeAsm("LD_E_A"));
            injected.Add(Expr.MakeAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("ADD_HL_DE"));

            injected.Add(Expr.MakeAsm("LD_HL_C"));
            injected.Add(Expr.MakeAsm("RET"));

            // Out-of-range return
            injected.Add(Expr.Make(Tag.Label, ret.Base.Value));
            injected.Add(Expr.MakeAsm("RET"));
        }

        // Copy a nonzero byte count, polling before each LCD-on write and enabling interrupts after the transfer.
        // The caller supplies valid source/destination spans; this helper performs no address bounds check.
        if (needBulk)
        {
            injected.Add(Expr.Make(Tag.Function, "__settile_bulk_core"));

            // Inputs: HL=dest (u16), DE=src (u16), B=count (u8)
            var ret = new AsmOperand("_kitaqgb_settile_bulk_v9_ret", AddressMode.Absolute);
            var loop = new AsmOperand("_kitaqgb_settile_bulk_v9_loop", AddressMode.Absolute);
            var wait = new AsmOperand("_kitaqgb_settile_bulk_v9_wait", AddressMode.Absolute);
            var fast = new AsmOperand("_kitaqgb_settile_bulk_v9_fast", AddressMode.Absolute);
            var loop_off = new AsmOperand("_kitaqgb_settile_bulk_v9_loop_off", AddressMode.Absolute);

            // if B==0 return
            injected.Add(Expr.MakeAsm("LD_A_B"));
            injected.Add(Expr.MakeAsm("OR_A"));
            injected.Add(Expr.MakeAsm("JP_Z", ret));

            // LCDC bit7 check
            injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem))); // LCDC
            injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("JR_Z", fast));

            // LCD ON: DI across the whole transfer
            injected.Add(Expr.MakeAsm("DI"));

            injected.Add(Expr.Make(Tag.Label, loop.Base.Value));

            // Wait STAT bit1 cleared
            injected.Add(Expr.Make(Tag.Label, wait.Base.Value));
            injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem))); // STAT
            injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("JR_NZ", wait));

            injected.Add(Expr.MakeAsm("LD_A_DE"));
            injected.Add(Expr.MakeAsm("LDI_HL_A"));
            injected.Add(Expr.MakeAsm("INC_DE"));
            injected.Add(Expr.MakeAsm("DEC_B"));
            injected.Add(Expr.MakeAsm("JR_NZ", loop));

            injected.Add(Expr.MakeAsm("EI"));
            injected.Add(Expr.Make(Tag.Label, ret.Base.Value));
            injected.Add(Expr.MakeAsm("RET"));

            // LCD OFF: fast copy
            injected.Add(Expr.Make(Tag.Label, fast.Base.Value));
            injected.Add(Expr.Make(Tag.Label, loop_off.Base.Value));
            injected.Add(Expr.MakeAsm("LD_A_DE"));
            injected.Add(Expr.MakeAsm("LDI_HL_A"));
            injected.Add(Expr.MakeAsm("INC_DE"));
            injected.Add(Expr.MakeAsm("DEC_B"));
            injected.Add(Expr.MakeAsm("JR_NZ", loop_off));
            injected.Add(Expr.MakeAsm("RET"));
        }

        // Copy B bytes without polling or changing interrupt state; a zero count returns without touching memory.
        if (needBulkFast)
        {
            injected.Add(Expr.Make(Tag.Function, "__settile_bulk_fast_core"));

            // Inputs: HL=dest (u16), DE=src (u16), B=count (u8)
            var ret = new AsmOperand("_kitaqgb_settile_bulk_fast_v9_ret", AddressMode.Absolute);
            var loop = new AsmOperand("_kitaqgb_settile_bulk_fast_v9_loop", AddressMode.Absolute);

            // if B==0 return
            injected.Add(Expr.MakeAsm("LD_A_B"));
            injected.Add(Expr.MakeAsm("OR_A"));
            injected.Add(Expr.MakeAsm("JP_Z", ret));

            injected.Add(Expr.Make(Tag.Label, loop.Base.Value));
            injected.Add(Expr.MakeAsm("LD_A_DE"));
            injected.Add(Expr.MakeAsm("LDI_HL_A"));
            injected.Add(Expr.MakeAsm("INC_DE"));
            injected.Add(Expr.MakeAsm("DEC_B"));
            injected.Add(Expr.MakeAsm("JR_NZ", loop));

            injected.Add(Expr.Make(Tag.Label, ret.Base.Value));
            injected.Add(Expr.MakeAsm("RET"));
        }

        // Insert after leading $rst_map directives
        int insertAt = 0;
        while (insertAt < rawLines.Count && rawLines[insertAt].MatchTag(Tag.RstMap)) insertAt++;
        rawLines.InsertRange(insertAt, injected);
    }

    // Allocate one named scroll-state byte in WRAM bank 1 and emit its debug storage declaration.
    AsmOperand AllocateScrollByte(string name)
    {
        int address = Allocate(Wram1Region, 1);
        Emit(Tag.Variable, name, address, 1);
        return MemOp(address, name);
    }

    // Allocate shared scroll/split state once, including an eight-entry table with six bytes per entry.
    void EnsureScrollState()
    {
        if (ScrollStateAllocated) return;

        ScrollStateAllocated = true;
        ScrollBgXCurVar = AllocateScrollByte("__kq_scroll_bg_x_cur");
        ScrollBgYCurVar = AllocateScrollByte("__kq_scroll_bg_y_cur");
        ScrollWinXCurVar = AllocateScrollByte("__kq_scroll_win_x_cur");
        ScrollWinYCurVar = AllocateScrollByte("__kq_scroll_win_y_cur");
        ScrollBgXNextVar = AllocateScrollByte("__kq_scroll_bg_x_next");
        ScrollBgYNextVar = AllocateScrollByte("__kq_scroll_bg_y_next");
        ScrollWinXNextVar = AllocateScrollByte("__kq_scroll_win_x_next");
        ScrollWinYNextVar = AllocateScrollByte("__kq_scroll_win_y_next");
        ScrollDirtyVar = AllocateScrollByte("__kq_scroll_dirty");
        ScrollWinVisibleVar = AllocateScrollByte("__kq_scroll_win_visible");
        ScrollSplitCountVar = AllocateScrollByte("__kq_scroll_split_count");
        ScrollSplitEnabledVar = AllocateScrollByte("__kq_scroll_split_enabled");
        ScrollSplitIndexVar = AllocateScrollByte("__kq_scroll_split_index");
        ScrollSplitTmpLyVar = AllocateScrollByte("__kq_scroll_split_tmp_ly");
        ScrollSplitTmpScxVar = AllocateScrollByte("__kq_scroll_split_tmp_scx");
        ScrollSplitTmpScyVar = AllocateScrollByte("__kq_scroll_split_tmp_scy");
        ScrollSplitTmpWxVar = AllocateScrollByte("__kq_scroll_split_tmp_wx");
        ScrollSplitTmpWyVar = AllocateScrollByte("__kq_scroll_split_tmp_wy");
        ScrollSplitTmpFlagsVar = AllocateScrollByte("__kq_scroll_split_tmp_flags");

        int tableBytes = ScrollSplitMaxEntries * ScrollSplitEntrySize;
        ScrollSplitTableAddr = Allocate(Wram1Region, tableBytes);
        Emit(Tag.Variable, "__kq_scroll_split_table", ScrollSplitTableAddr, tableBytes);
    }

    // Emit the split-table operations and VBlank/STAT handlers when requested or referenced by symbolic assembly operands.
    void AppendScrollHelpersIfUsed(List<Expr> rawLines)
    {
        bool needScrollHelpers = UseScrollSplitHelpers;
        if (!needScrollHelpers)
        {
            foreach (var e in rawLines)
            {
                string m;
                AsmOperand o;
                if (!e.Match(Tag.Asm, out m, out o) || o == null || !o.Base.HasValue) continue;
                string target = o.Base.Value;
                if (target == "__kq_scroll_split_reset_core" ||
                    target == "__kq_scroll_split_push_core" ||
                    target == "__kq_scroll_split_commit_core")
                {
                    needScrollHelpers = true;
                    break;
                }
            }
        }

        if (!needScrollHelpers) return;

        EnsureScrollState();

        var injected = new List<Expr>();
        injected.Add(Expr.Make(Tag.Comment, "[KITAQGB] injected scroll split helpers"));

        // Reset the split count/index and disable STAT coincidence interrupts; existing table bytes are left in place.
        string resetDone = "__kq_scroll_split_reset_done";
        injected.Add(Expr.Make(Tag.Function, "__kq_scroll_split_reset_core"));
        injected.Add(Expr.MakeAsm("XOR_A"));
        injected.Add(Expr.MakeAsm("LD_MEM_A", ScrollSplitCountVar));
        injected.Add(Expr.MakeAsm("LD_MEM_A", ScrollSplitEnabledVar));
        injected.Add(Expr.MakeAsm("LD_MEM_A", ScrollSplitIndexVar));
        injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem))); // STAT
        injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(0xBF, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x41, AddressMode.HighMem)));
        injected.Add(Expr.Make(Tag.Label, resetDone));
        injected.Add(Expr.MakeAsm("RET"));

        // Append staged LY, SCX, SCY, WX, WY and masked flags; silently ignore entries beyond the eight-slot capacity.
        string pushHaveRoom = "__kq_scroll_split_push_have_room";
        injected.Add(Expr.Make(Tag.Function, "__kq_scroll_split_push_core"));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitCountVar));
        injected.Add(Expr.MakeAsm("CP_IMM", new AsmOperand(ScrollSplitMaxEntries, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("JR_C", new AsmOperand(pushHaveRoom, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("RET"));
        injected.Add(Expr.Make(Tag.Label, pushHaveRoom));
        injected.Add(Expr.MakeAsm("LD_E_A"));
        injected.Add(Expr.MakeAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LD_L_A"));
        injected.Add(Expr.MakeAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("ADD_HL_HL"));
        injected.Add(Expr.MakeAsm("ADD_HL_DE"));
        injected.Add(Expr.MakeAsm("ADD_HL_HL"));
        injected.Add(Expr.MakeAsm("LD_DE_IMM", new AsmOperand(ScrollSplitTableAddr, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("ADD_HL_DE"));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitTmpLyVar));
        injected.Add(Expr.MakeAsm("LDI_HL_A"));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitTmpScxVar));
        injected.Add(Expr.MakeAsm("LDI_HL_A"));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitTmpScyVar));
        injected.Add(Expr.MakeAsm("LDI_HL_A"));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitTmpWxVar));
        injected.Add(Expr.MakeAsm("LDI_HL_A"));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitTmpWyVar));
        injected.Add(Expr.MakeAsm("LDI_HL_A"));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitTmpFlagsVar));
        injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(ScrollSplitFlagMask, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LDI_HL_A"));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitCountVar));
        injected.Add(Expr.MakeAsm("INC_A"));
        injected.Add(Expr.MakeAsm("LD_MEM_A", ScrollSplitCountVar));
        injected.Add(Expr.MakeAsm("RET"));

        // Schedule entry A by programming LYC and STAT bit 6, or disable that bit when A is outside the current count.
        string schedDisable = "__kq_scroll_schedule_next_disable";
        string schedHaveTarget = "__kq_scroll_schedule_next_have_target";
        injected.Add(Expr.Make(Tag.Function, "__kq_scroll_schedule_next"));
        injected.Add(Expr.MakeAsm("LD_B_A"));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitCountVar));
        injected.Add(Expr.MakeAsm("CP_B"));
        injected.Add(Expr.MakeAsm("JR_C", new AsmOperand(schedDisable, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("JR_Z", new AsmOperand(schedDisable, AddressMode.Absolute)));
        injected.Add(Expr.Make(Tag.Label, schedHaveTarget));
        injected.Add(Expr.MakeAsm("LD_A_B"));
        injected.Add(Expr.MakeAsm("LD_E_A"));
        injected.Add(Expr.MakeAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LD_L_A"));
        injected.Add(Expr.MakeAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("ADD_HL_HL"));
        injected.Add(Expr.MakeAsm("ADD_HL_DE"));
        injected.Add(Expr.MakeAsm("ADD_HL_HL"));
        injected.Add(Expr.MakeAsm("LD_DE_IMM", new AsmOperand(ScrollSplitTableAddr, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("ADD_HL_DE"));
        injected.Add(Expr.MakeAsm("LD_A_HL"));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x45, AddressMode.HighMem))); // LYC
        injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem))); // STAT
        injected.Add(Expr.MakeAsm("OR_IMM", new AsmOperand(0x40, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x41, AddressMode.HighMem)));
        injected.Add(Expr.MakeAsm("RET"));
        injected.Add(Expr.Make(Tag.Label, schedDisable));
        injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem))); // STAT
        injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(0xBF, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x41, AddressMode.HighMem)));
        injected.Add(Expr.MakeAsm("RET"));

        // Apply the fields selected by an entry's flags. Show follows hide when both are set; palette color zero reuses SCX/SCY bytes.
        string applySkipBg = "__kq_scroll_apply_entry_skip_bg";
        string applySkipWin = "__kq_scroll_apply_entry_skip_win";
        string applySkipHide = "__kq_scroll_apply_entry_skip_hide";
        string applySkipBgColor0 = "__kq_scroll_apply_entry_skip_bg_color0";
        string applyDone = "__kq_scroll_apply_entry_done";
        injected.Add(Expr.Make(Tag.Function, "__kq_scroll_apply_entry"));
        injected.Add(Expr.MakeAsm("INC_HL")); // scx
        injected.Add(Expr.MakeAsm("LD_A_HL"));
        injected.Add(Expr.MakeAsm("LD_B_A"));
        injected.Add(Expr.MakeAsm("INC_HL")); // scy
        injected.Add(Expr.MakeAsm("LD_A_HL"));
        injected.Add(Expr.MakeAsm("LD_C_A"));
        injected.Add(Expr.MakeAsm("INC_HL")); // wx
        injected.Add(Expr.MakeAsm("LD_A_HL"));
        injected.Add(Expr.MakeAsm("LD_D_A"));
        injected.Add(Expr.MakeAsm("INC_HL")); // wy
        injected.Add(Expr.MakeAsm("LD_A_HL"));
        injected.Add(Expr.MakeAsm("LD_E_A"));
        injected.Add(Expr.MakeAsm("INC_HL")); // flags
        injected.Add(Expr.MakeAsm("LD_A_HL"));
        injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(ScrollSplitFlagMask, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LD_H_A"));
        injected.Add(Expr.MakeAsm("LD_A_H"));
        injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(ScrollSplitFlagUseBg, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("JR_Z", new AsmOperand(applySkipBg, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("LD_A_B"));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x43, AddressMode.HighMem))); // SCX
        injected.Add(Expr.MakeAsm("LD_A_C"));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x42, AddressMode.HighMem))); // SCY
        injected.Add(Expr.Make(Tag.Label, applySkipBg));
        injected.Add(Expr.MakeAsm("LD_A_H"));
        injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(ScrollSplitFlagUseWin, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("JR_Z", new AsmOperand(applySkipWin, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("LD_A_D"));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x4B, AddressMode.HighMem))); // WX
        injected.Add(Expr.MakeAsm("LD_A_E"));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x4A, AddressMode.HighMem))); // WY
        injected.Add(Expr.Make(Tag.Label, applySkipWin));
        injected.Add(Expr.MakeAsm("LD_A_H"));
        injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(ScrollSplitFlagWinHide, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("JR_Z", new AsmOperand(applySkipHide, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem))); // LCDC
        injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(0xDF, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x40, AddressMode.HighMem)));
        injected.Add(Expr.Make(Tag.Label, applySkipHide));
        injected.Add(Expr.MakeAsm("LD_A_H"));
        injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(ScrollSplitFlagWinShow, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("JR_Z", new AsmOperand(applySkipBgColor0, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem))); // LCDC
        injected.Add(Expr.MakeAsm("OR_IMM", new AsmOperand(0x20, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x40, AddressMode.HighMem)));
        injected.Add(Expr.Make(Tag.Label, applySkipBgColor0));
        injected.Add(Expr.MakeAsm("LD_A_H"));
        injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(ScrollSplitFlagBgColor0, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("JR_Z", new AsmOperand(applyDone, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("LD_A_IMM", new AsmOperand(0x80, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x68, AddressMode.HighMem))); // BCPS: palette 0, color 0, auto increment
        injected.Add(Expr.MakeAsm("LD_A_B"));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x69, AddressMode.HighMem))); // BCPD low
        injected.Add(Expr.MakeAsm("LD_A_C"));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x69, AddressMode.HighMem))); // BCPD high
        injected.Add(Expr.Make(Tag.Label, applyDone));
        injected.Add(Expr.MakeAsm("RET"));

        // Capture base window visibility and arm a nonempty table at index zero, enabling VBlank/STAT interrupts and issuing EI.
        string commitDisable = "__kq_scroll_split_commit_disable";
        injected.Add(Expr.Make(Tag.Function, "__kq_scroll_split_commit_core"));
        injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem))); // LCDC
        injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(0x20, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LD_MEM_A", ScrollWinVisibleVar));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitCountVar));
        injected.Add(Expr.MakeAsm("OR_A"));
        injected.Add(Expr.MakeAsm("JR_Z", new AsmOperand(commitDisable, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LD_MEM_A", ScrollSplitEnabledVar));
        injected.Add(Expr.MakeAsm("XOR_A"));
        injected.Add(Expr.MakeAsm("LD_MEM_A", ScrollSplitIndexVar));
        injected.Add(Expr.MakeAsm("CALL", new AsmOperand("__kq_scroll_schedule_next", AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("LD_A_MEM", new AsmOperand(0xFFFF, AddressMode.Absolute))); // IE
        injected.Add(Expr.MakeAsm("OR_IMM", new AsmOperand(0x03, AddressMode.Immediate))); // VBlank | STAT
        injected.Add(Expr.MakeAsm("LD_MEM_A", new AsmOperand(0xFFFF, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("EI"));
        injected.Add(Expr.MakeAsm("RET"));
        injected.Add(Expr.Make(Tag.Label, commitDisable));
        injected.Add(Expr.MakeAsm("XOR_A"));
        injected.Add(Expr.MakeAsm("LD_MEM_A", ScrollSplitEnabledVar));
        injected.Add(Expr.MakeAsm("LD_MEM_A", ScrollSplitIndexVar));
        injected.Add(Expr.MakeAsm("CALL", new AsmOperand("__kq_scroll_schedule_next", AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("RET"));

        // Save registers/SVBK, select WRAM bank 1, restore base scroll state and restart the split sequence for the frame.
        // A first entry at LY zero is applied immediately before scheduling the following entry.
        string vbDisable = "__kq_vblank_disable";
        string vbFirstNonZero = "__kq_vblank_first_nonzero";
        string vbDone = "__kq_vblank_done";
        injected.Add(Expr.Make(Tag.Function, "__kq_vblank_vector"));
        injected.Add(Expr.MakeAsm("PUSH_AF"));
        injected.Add(Expr.MakeAsm("PUSH_BC"));
        injected.Add(Expr.MakeAsm("PUSH_DE"));
        injected.Add(Expr.MakeAsm("PUSH_HL"));
        injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x70, AddressMode.HighMem))); // SVBK
        injected.Add(Expr.MakeAsm("PUSH_AF"));
        injected.Add(Expr.MakeAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x70, AddressMode.HighMem)));
        injected.Add(Expr.MakeAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate)));
        // Set the existing fixed-address frame-ready flag used by the VBlank path.
        injected.Add(Expr.MakeAsm("LD_MEM_A", new AsmOperand(0xC29C, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollBgXCurVar));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x43, AddressMode.HighMem))); // SCX
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollBgYCurVar));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x42, AddressMode.HighMem))); // SCY
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollWinXCurVar));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x4B, AddressMode.HighMem))); // WX
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollWinYCurVar));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x4A, AddressMode.HighMem))); // WY
        injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem))); // LCDC
        injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(0xDF, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LD_B_A"));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollWinVisibleVar));
        injected.Add(Expr.MakeAsm("OR_B"));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x40, AddressMode.HighMem)));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitEnabledVar));
        injected.Add(Expr.MakeAsm("OR_A"));
        injected.Add(Expr.MakeAsm("JR_Z", new AsmOperand(vbDisable, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitCountVar));
        injected.Add(Expr.MakeAsm("OR_A"));
        injected.Add(Expr.MakeAsm("JR_Z", new AsmOperand(vbDisable, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("XOR_A"));
        injected.Add(Expr.MakeAsm("LD_MEM_A", ScrollSplitIndexVar));
        injected.Add(Expr.MakeAsm("LD_A_MEM", new AsmOperand(ScrollSplitTableAddr, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("OR_A"));
        injected.Add(Expr.MakeAsm("JR_NZ", new AsmOperand(vbFirstNonZero, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("LD_HL_IMM", new AsmOperand(ScrollSplitTableAddr, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("CALL", new AsmOperand("__kq_scroll_apply_entry", AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LD_MEM_A", ScrollSplitIndexVar));
        injected.Add(Expr.MakeAsm("CALL", new AsmOperand("__kq_scroll_schedule_next", AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("JR", new AsmOperand(vbDone, AddressMode.Absolute)));
        injected.Add(Expr.Make(Tag.Label, vbFirstNonZero));
        injected.Add(Expr.MakeAsm("XOR_A"));
        injected.Add(Expr.MakeAsm("CALL", new AsmOperand("__kq_scroll_schedule_next", AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("JR", new AsmOperand(vbDone, AddressMode.Absolute)));
        injected.Add(Expr.Make(Tag.Label, vbDisable));
        injected.Add(Expr.MakeAsm("CALL", new AsmOperand("__kq_scroll_schedule_next", AddressMode.Absolute)));
        injected.Add(Expr.Make(Tag.Label, vbDone));
        injected.Add(Expr.MakeAsm("POP_AF"));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x70, AddressMode.HighMem))); // restore SVBK
        injected.Add(Expr.MakeAsm("POP_HL"));
        injected.Add(Expr.MakeAsm("POP_DE"));
        injected.Add(Expr.MakeAsm("POP_BC"));
        injected.Add(Expr.MakeAsm("POP_AF"));
        injected.Add(Expr.MakeAsm("RETI"));

        // Save registers/SVBK, apply the current entry and advance its index, or disable further coincidence interrupts at the end.
        string statDisable = "__kq_stat_disable";
        string statDone = "__kq_stat_done";
        injected.Add(Expr.Make(Tag.Function, "__kq_stat_vector"));
        injected.Add(Expr.MakeAsm("PUSH_AF"));
        injected.Add(Expr.MakeAsm("PUSH_BC"));
        injected.Add(Expr.MakeAsm("PUSH_DE"));
        injected.Add(Expr.MakeAsm("PUSH_HL"));
        injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x70, AddressMode.HighMem))); // SVBK
        injected.Add(Expr.MakeAsm("PUSH_AF"));
        injected.Add(Expr.MakeAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x70, AddressMode.HighMem)));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitEnabledVar));
        injected.Add(Expr.MakeAsm("OR_A"));
        injected.Add(Expr.MakeAsm("JR_Z", new AsmOperand(statDisable, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitIndexVar));
        injected.Add(Expr.MakeAsm("LD_B_A"));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitCountVar));
        injected.Add(Expr.MakeAsm("CP_B"));
        injected.Add(Expr.MakeAsm("JR_C", new AsmOperand(statDisable, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("JR_Z", new AsmOperand(statDisable, AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("LD_A_B"));
        injected.Add(Expr.MakeAsm("LD_E_A"));
        injected.Add(Expr.MakeAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LD_L_A"));
        injected.Add(Expr.MakeAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("ADD_HL_HL"));
        injected.Add(Expr.MakeAsm("ADD_HL_DE"));
        injected.Add(Expr.MakeAsm("ADD_HL_HL"));
        injected.Add(Expr.MakeAsm("LD_DE_IMM", new AsmOperand(ScrollSplitTableAddr, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("ADD_HL_DE"));
        injected.Add(Expr.MakeAsm("CALL", new AsmOperand("__kq_scroll_apply_entry", AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("LD_A_MEM", ScrollSplitIndexVar));
        injected.Add(Expr.MakeAsm("INC_A"));
        injected.Add(Expr.MakeAsm("LD_MEM_A", ScrollSplitIndexVar));
        injected.Add(Expr.MakeAsm("CALL", new AsmOperand("__kq_scroll_schedule_next", AddressMode.Absolute)));
        injected.Add(Expr.MakeAsm("JR", new AsmOperand(statDone, AddressMode.Absolute)));
        injected.Add(Expr.Make(Tag.Label, statDisable));
        injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem))); // STAT
        injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(0xBF, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x41, AddressMode.HighMem)));
        injected.Add(Expr.Make(Tag.Label, statDone));
        injected.Add(Expr.MakeAsm("POP_AF"));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x70, AddressMode.HighMem))); // restore SVBK
        injected.Add(Expr.MakeAsm("POP_HL"));
        injected.Add(Expr.MakeAsm("POP_DE"));
        injected.Add(Expr.MakeAsm("POP_BC"));
        injected.Add(Expr.MakeAsm("POP_AF"));
        injected.Add(Expr.MakeAsm("RETI"));

        int insertAt = 0;
        while (insertAt < rawLines.Count && rawLines[insertAt].MatchTag(Tag.RstMap)) insertAt++;
        rawLines.InsertRange(insertAt, injected);
    }

    // Direct calls are valid to common bank zero or to the caller's currently mapped bank.
    bool IsDirectCallBankSafe(int calleeBank, int callerBank)
    {
        // bank0 is fixed ROM and always directly callable.
        if (calleeBank == 0) return true;
        return calleeBank == callerBank;
    }

    // Reuse a target/bank-specific stub and reject conflicting stack-ABI metadata; the stored bank number is masked to eight bits.
    string EnsureBank0Thunk(string targetName, int targetBank, bool isStackCall, int stackArgBytes)
    {
        string thunkName = "__kq_thunk_b" + (targetBank & 0xFF) + "_" + targetName;
        if (!Bank0ThunkMap.ContainsKey(thunkName))
        {
            Bank0ThunkMap.Add(thunkName, new BankThunkInfo
            {
                Bank = targetBank & 0xFF,
                Target = targetName,
                IsStackCall = isStackCall,
                StackArgBytes = stackArgBytes
            });
        }
        else
        {
            var info = Bank0ThunkMap[thunkName];
            if (info.IsStackCall != isStackCall)
                Program.Panic("internal: thunk call-convention mismatch: " + thunkName);
            if (isStackCall && info.StackArgBytes != stackArgBytes)
                Program.Panic("internal: thunk stack-arg-size mismatch: " + thunkName);
        }
        return thunkName;
    }

    // Deduplicate runtime-selected-bank call stubs by target name.
    string EnsureBank0RuntimeFarcallThunk(string targetName)
    {
        string thunkName = "__kq_farcall_" + targetName;
        if (!Bank0RuntimeFarcallThunkMap.ContainsKey(thunkName))
            Bank0RuntimeFarcallThunkMap.Add(thunkName, targetName);
        return thunkName;
    }

    // Emit only requested bank-switch/copy/call stubs in fixed bank zero, keeping common code available across ROM switches.
    void AppendBankHelpersIfUsed(List<Expr> rawLines)
    {
        if (!UseBankSwitchBank0Helper &&
            !UseFarMemcpyBank0Helper &&
            !UseFarCallPointerBank0Helper &&
            Bank0ThunkMap.Count == 0 &&
            Bank0RuntimeFarcallThunkMap.Count == 0) return;

        var injected = new List<Expr>();
        injected.Add(Expr.Make(Tag.Comment, "[KITAQGB] injected bank0 banking helpers"));

        // Write the requested low bank byte to $2000 and update its HRAM shadow at $FF82.
        if (UseBankSwitchBank0Helper)
        {
            injected.Add(Expr.Make(Tag.Function, "__kq_bankswitch_bank0"));
            injected.Add(Expr.MakeAsm("LD_MEM_A", new AsmOperand(0x2000, AddressMode.Absolute)));
            injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x82, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("RET"));
        }

        // Switch for a BC-byte forward copy from DE to HL, then restore the bank from a shared HRAM slot.
        // This helper does not mask interrupts or nest saved banks on the CPU stack.
        if (UseFarMemcpyBank0Helper)
        {
            string loopLabel = "__kq_far_memcpy_loop";
            string doneLabel = "__kq_far_memcpy_done";

            injected.Add(Expr.Make(Tag.Function, "__kq_far_memcpy_bank0"));
            // Entry: A=requested bank, HL=dst, DE=src, BC=len
            injected.Add(Expr.MakeAsm("PUSH_AF"));
            injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x82, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(RomBankSavedAddr & 0xFF, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("POP_AF"));
            injected.Add(Expr.MakeAsm("LD_MEM_A", new AsmOperand(0x2000, AddressMode.Absolute)));
            injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x82, AddressMode.Immediate)));
            injected.Add(Expr.Make(Tag.Label, loopLabel));
            injected.Add(Expr.MakeAsm("LD_A_B"));
            injected.Add(Expr.MakeAsm("OR_C"));
            injected.Add(Expr.MakeAsm("JP_Z", new AsmOperand(doneLabel, AddressMode.Absolute)));
            injected.Add(Expr.MakeAsm("LD_A_DE"));
            injected.Add(Expr.MakeAsm("LDI_HL_A"));
            injected.Add(Expr.MakeAsm("INC_DE"));
            injected.Add(Expr.MakeAsm("DEC_BC"));
            injected.Add(Expr.MakeAsm("JP", new AsmOperand(loopLabel, AddressMode.Absolute)));
            injected.Add(Expr.Make(Tag.Label, doneLabel));
            injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(RomBankSavedAddr & 0xFF, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("LD_MEM_A", new AsmOperand(0x2000, AddressMode.Absolute)));
            injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x82, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("RET"));
        }

        // A holds the requested bank and HL the runtime callback address. Save
        // each caller bank on the CPU stack, so nested far calls do not share a
        // mutable saved-bank slot. The callback receives no arguments.
        if (UseFarCallPointerBank0Helper)
        {
            string returned = "__kq_farcall_pointer_return";
            injected.Add(Expr.Make(Tag.Function, "__kq_farcall_pointer_bank0"));
            injected.Add(Expr.MakeAsm("PUSH_AF"));
            injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x82, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("PUSH_AF"));
            injected.Add(Expr.MakeAsm("POP_BC"));
            injected.Add(Expr.MakeAsm("POP_AF"));
            injected.Add(Expr.MakeAsm("PUSH_BC"));
            injected.Add(Expr.MakeAsm("LD_MEM_A", new AsmOperand(0x2000, AddressMode.Absolute)));
            injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x82, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("LD_DE_IMM", new AsmOperand(returned, AddressMode.Immediate16)));
            injected.Add(Expr.MakeAsm("PUSH_DE"));
            injected.Add(Expr.MakeAsm("JP_HL"));
            injected.Add(Expr.Make(Tag.Label, returned));
            // Preserve scalar return registers while restoring the mapping. The
            // intrinsic is void, but this also avoids corrupting callback flags.
            injected.Add(Expr.MakeAsm("PUSH_AF"));
            injected.Add(Expr.MakeAsm("PUSH_HL"));
            injected.Add(Expr.MakeAsm("POP_DE"));
            injected.Add(Expr.MakeAsm("POP_BC"));
            injected.Add(Expr.MakeAsm("POP_AF"));
            injected.Add(Expr.MakeAsm("LD_MEM_A", new AsmOperand(0x2000, AddressMode.Absolute)));
            injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x82, AddressMode.Immediate)));
            injected.Add(Expr.MakeAsm("PUSH_BC"));
            injected.Add(Expr.MakeAsm("PUSH_DE"));
            injected.Add(Expr.MakeAsm("POP_HL"));
            injected.Add(Expr.MakeAsm("POP_AF"));
            injected.Add(Expr.MakeAsm("RET"));
        }

        // Emit runtime-bank call wrappers that save the old bank on the CPU stack and preserve returned A/HL across restoration.
        if (Bank0RuntimeFarcallThunkMap.Count > 0)
        {
            foreach (var kv in Bank0RuntimeFarcallThunkMap.OrderBy(x => x.Key))
            {
                string thunkName = kv.Key;
                string targetName = kv.Value;

                injected.Add(Expr.Make(Tag.Function, thunkName));

                // Entry A = requested bank.
                // Keep old bank on stack; preserve return A/HL.
                injected.Add(Expr.MakeAsm("PUSH_AF"));
                injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x82, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("PUSH_AF"));
                injected.Add(Expr.MakeAsm("POP_BC"));
                injected.Add(Expr.MakeAsm("POP_AF"));
                injected.Add(Expr.MakeAsm("PUSH_BC"));
                injected.Add(Expr.MakeAsm("LD_MEM_A", new AsmOperand(0x2000, AddressMode.Absolute)));
                injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x82, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("CALL", new AsmOperand(targetName, AddressMode.Absolute)));

                injected.Add(Expr.MakeAsm("PUSH_AF"));
                injected.Add(Expr.MakeAsm("PUSH_HL"));
                injected.Add(Expr.MakeAsm("POP_DE"));
                injected.Add(Expr.MakeAsm("POP_BC"));
                injected.Add(Expr.MakeAsm("POP_AF"));
                injected.Add(Expr.MakeAsm("LD_MEM_A", new AsmOperand(0x2000, AddressMode.Absolute)));
                injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x82, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("PUSH_BC"));
                injected.Add(Expr.MakeAsm("PUSH_DE"));
                injected.Add(Expr.MakeAsm("POP_HL"));
                injected.Add(Expr.MakeAsm("POP_AF"));
                injected.Add(Expr.MakeAsm("RET"));
            }
        }

        // Include the depth-failure spin loop only when at least one stack-ABI thunk needs it.
        bool hasStackCallThunk = Bank0ThunkMap.Values.Any(v => v.IsStackCall);
        if (hasStackCallThunk)
        {
            // Dedicated trap for thunk stack corruption / overflow.
            injected.Add(Expr.Make(Tag.Function, "__kq_thunk_stack_trap"));
            injected.Add(Expr.MakeAsm("JP", new AsmOperand("__kq_thunk_stack_trap", AddressMode.Absolute)));
        }

        foreach (var kv in Bank0ThunkMap.OrderBy(x => x.Value.Bank).ThenBy(x => x.Value.Target))
        {
            string thunkName = kv.Key;
            int targetBank = kv.Value.Bank;
            string targetName = kv.Value.Target;

            injected.Add(Expr.Make(Tag.Function, thunkName));

            if (!kv.Value.IsStackCall)
            {
                // Preserve incoming A (fastcall u8 argument), save old bank on CPU stack,
                // switch in target bank, then restore argument A before CALL.
                injected.Add(Expr.MakeAsm("PUSH_AF"));
                injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x82, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("PUSH_AF"));
                injected.Add(Expr.MakeAsm("LD_A_IMM", new AsmOperand(targetBank, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("LD_MEM_A", new AsmOperand(0x2000, AddressMode.Absolute)));
                injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x82, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("POP_BC"));
                injected.Add(Expr.MakeAsm("POP_AF"));
                injected.Add(Expr.MakeAsm("PUSH_BC"));
                injected.Add(Expr.MakeAsm("CALL", new AsmOperand(targetName, AddressMode.Absolute)));

                // Restore old bank while preserving return registers A/HL.
                injected.Add(Expr.MakeAsm("PUSH_AF"));
                injected.Add(Expr.MakeAsm("PUSH_HL"));
                injected.Add(Expr.MakeAsm("POP_DE"));
                injected.Add(Expr.MakeAsm("POP_BC"));
                injected.Add(Expr.MakeAsm("POP_AF"));
                injected.Add(Expr.MakeAsm("LD_MEM_A", new AsmOperand(0x2000, AddressMode.Absolute)));
                injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x82, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("PUSH_BC"));
                injected.Add(Expr.MakeAsm("PUSH_DE"));
                injected.Add(Expr.MakeAsm("POP_HL"));
                injected.Add(Expr.MakeAsm("POP_AF"));
                injected.Add(Expr.MakeAsm("RET"));
            }
            else
            {
                // stackcall-aware thunk:
                // pop caller return-address so callee sees args at SP+2,
                // and keep nested-safe bank/return stacks in HRAM buffers.
                injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(BankThunkSpAddr & 0xFF, AddressMode.Immediate)));
                // Treat an out-of-encoding-range entry depth as uninitialized, but trap at the valid full-depth value.
                injected.Add(Expr.MakeAsm("CP_IMM", new AsmOperand(BankThunkDepthEncodedBase, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("JP_C", new AsmOperand("kq_thunk_depth_init_" + thunkName, AddressMode.Absolute)));
                injected.Add(Expr.MakeAsm("CP_IMM", new AsmOperand(BankThunkDepthEncodedLimit + 1, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("JP_C", new AsmOperand("kq_thunk_depth_ok_" + thunkName, AddressMode.Absolute)));
                injected.Add(Expr.Make(Tag.Label, "kq_thunk_depth_init_" + thunkName));
                injected.Add(Expr.MakeAsm("LD_A_IMM", new AsmOperand(BankThunkDepthEncodedBase, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(BankThunkSpAddr & 0xFF, AddressMode.Immediate)));
                injected.Add(Expr.Make(Tag.Label, "kq_thunk_depth_ok_" + thunkName));
                injected.Add(Expr.MakeAsm("CP_IMM", new AsmOperand(BankThunkDepthEncodedLimit, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("JP_Z", new AsmOperand("__kq_thunk_stack_trap", AddressMode.Absolute)));
                injected.Add(Expr.MakeAsm("LD_D_A"));
                injected.Add(Expr.MakeAsm("INC_A"));
                injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(BankThunkSpAddr & 0xFF, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("LD_A_D"));
                injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(0x0F, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("LD_C_A"));
                injected.Add(Expr.MakeAsm("LD_B_IMM", new AsmOperand(0, AddressMode.Immediate)));

                // Remove the caller return address from the CPU stack and store it with the bank in the selected HRAM depth slot.
                injected.Add(Expr.MakeAsm("POP_DE"));

                injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x82, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("LD_HL_IMM", new AsmOperand(BankThunkBankBaseAddr, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("ADD_HL_BC"));
                injected.Add(Expr.MakeAsm("LD_HL_A"));

                injected.Add(Expr.MakeAsm("LD_A_E"));
                injected.Add(Expr.MakeAsm("LD_HL_IMM", new AsmOperand(BankThunkRetLoBaseAddr, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("ADD_HL_BC"));
                injected.Add(Expr.MakeAsm("LD_HL_A"));

                injected.Add(Expr.MakeAsm("LD_A_D"));
                injected.Add(Expr.MakeAsm("LD_HL_IMM", new AsmOperand(BankThunkRetHiBaseAddr, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("ADD_HL_BC"));
                injected.Add(Expr.MakeAsm("LD_HL_A"));

                injected.Add(Expr.MakeAsm("LD_A_IMM", new AsmOperand(targetBank, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("LD_MEM_A", new AsmOperand(0x2000, AddressMode.Absolute)));
                injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x82, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("CALL", new AsmOperand(targetName, AddressMode.Absolute)));

                injected.Add(Expr.MakeAsm("PUSH_AF"));
                injected.Add(Expr.MakeAsm("PUSH_HL"));

                injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(BankThunkSpAddr & 0xFF, AddressMode.Immediate)));
                // Validate the return depth before decrementing it and retrieving the matching saved bank/return address.
                injected.Add(Expr.MakeAsm("CP_IMM", new AsmOperand(BankThunkDepthEncodedBase + 1, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("JP_C", new AsmOperand("__kq_thunk_stack_trap", AddressMode.Absolute)));
                injected.Add(Expr.MakeAsm("CP_IMM", new AsmOperand(BankThunkDepthEncodedLimit + 1, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("JP_NC", new AsmOperand("__kq_thunk_stack_trap", AddressMode.Absolute)));
                injected.Add(Expr.MakeAsm("DEC_A"));
                injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(BankThunkSpAddr & 0xFF, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("AND_IMM", new AsmOperand(0x0F, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("LD_C_A"));
                injected.Add(Expr.MakeAsm("LD_B_IMM", new AsmOperand(0, AddressMode.Immediate)));

                injected.Add(Expr.MakeAsm("LD_HL_IMM", new AsmOperand(BankThunkBankBaseAddr, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("ADD_HL_BC"));
                injected.Add(Expr.MakeAsm("LD_A_HL"));
                injected.Add(Expr.MakeAsm("LD_MEM_A", new AsmOperand(0x2000, AddressMode.Absolute)));
                injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x82, AddressMode.Immediate)));

                injected.Add(Expr.MakeAsm("LD_HL_IMM", new AsmOperand(BankThunkRetLoBaseAddr, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("ADD_HL_BC"));
                injected.Add(Expr.MakeAsm("LD_A_HL"));
                injected.Add(Expr.MakeAsm("LD_E_A"));

                injected.Add(Expr.MakeAsm("LD_HL_IMM", new AsmOperand(BankThunkRetHiBaseAddr, AddressMode.Immediate)));
                injected.Add(Expr.MakeAsm("ADD_HL_BC"));
                injected.Add(Expr.MakeAsm("LD_A_HL"));
                injected.Add(Expr.MakeAsm("LD_D_A"));

                injected.Add(Expr.MakeAsm("POP_HL"));
                injected.Add(Expr.MakeAsm("POP_AF"));
                injected.Add(Expr.MakeAsm("PUSH_DE"));
                injected.Add(Expr.MakeAsm("RET"));
            }
        }

        // Keep helpers in bank0 by inserting before the first non-RST directive.
        int insertAt = 0;
        while (insertAt < rawLines.Count && rawLines[insertAt].MatchTag(Tag.RstMap)) insertAt++;
        rawLines.InsertRange(insertAt, injected);
    }

    // Emit the VBK readback probe when needed, restoring VBK and BC/DE and returning a byte Boolean in A.
    void AppendCgbHelpersIfUsed(List<Expr> rawLines)
    {
        if (!UseCgbRuntimeDetectHelper) return;

        var injected = new List<Expr>();
        injected.Add(Expr.Make(Tag.Comment, "[KITAQGB] injected CGB runtime-detect helper"));
        injected.Add(Expr.Make(Tag.Function, "__kq_is_cgb"));

        // Robust VBK toggle probe:
        //   CGB: write 0 -> FE, write 1 -> FF (bits 1..7 read back as 1)
        //   DMG / emulators without CGB VBK semantics: readback does not normalize to FE/FF
        // Save the raw register value so we can restore the exact previous state.
        AsmOperand isCgb = MakeUniqueLabel("kq_is_cgb_true");
        AsmOperand done = MakeUniqueLabel("kq_is_cgb_done");
        injected.Add(Expr.MakeAsm("PUSH_BC"));
        injected.Add(Expr.MakeAsm("PUSH_DE"));
        injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x4F, AddressMode.HighMem)));
        injected.Add(Expr.MakeAsm("LD_D_A")); // original raw VBK
        injected.Add(Expr.MakeAsm("XOR_A"));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x4F, AddressMode.HighMem)));
        injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x4F, AddressMode.HighMem)));
        injected.Add(Expr.MakeAsm("LD_B_A")); // expect FE on CGB
        injected.Add(Expr.MakeAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x4F, AddressMode.HighMem)));
        injected.Add(Expr.MakeAsm("LDH_A_MEM", new AsmOperand(0x4F, AddressMode.HighMem)));
        injected.Add(Expr.MakeAsm("LD_C_A")); // expect FF on CGB
        injected.Add(Expr.MakeAsm("LD_A_D"));
        injected.Add(Expr.MakeAsm("LDH_MEM_A", new AsmOperand(0x4F, AddressMode.HighMem))); // restore raw VBK
        injected.Add(Expr.MakeAsm("LD_A_B"));
        injected.Add(Expr.MakeAsm("CP_IMM", new AsmOperand(0xFE, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("JR_NZ", done));
        injected.Add(Expr.MakeAsm("LD_A_C"));
        injected.Add(Expr.MakeAsm("CP_IMM", new AsmOperand(0xFF, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("JR_Z", isCgb));
        injected.Add(Expr.Make(Tag.Label, done.Base.Value));
        injected.Add(Expr.MakeAsm("XOR_A"));
        injected.Add(Expr.MakeAsm("POP_DE"));
        injected.Add(Expr.MakeAsm("POP_BC"));
        injected.Add(Expr.MakeAsm("RET"));
        injected.Add(Expr.Make(Tag.Label, isCgb.Base.Value));
        injected.Add(Expr.MakeAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate)));
        injected.Add(Expr.MakeAsm("POP_DE"));
        injected.Add(Expr.MakeAsm("POP_BC"));
        injected.Add(Expr.MakeAsm("RET"));

        int insertAt = 0;
        while (insertAt < rawLines.Count && rawLines[insertAt].MatchTag(Tag.RstMap)) insertAt++;
        rawLines.InsertRange(insertAt, injected);
    }

    // Map each supported safe-write intrinsic and alias to its CGB I/O offset and report name.
    bool TryGetCgbSafeRegister(string funcName, out int ioOffset, out string regName)
    {
        ioOffset = 0;
        regName = "";
        if (string.IsNullOrEmpty(funcName)) return false;

        switch (funcName)
        {
            case "__cgb_safe_set_vbk": ioOffset = 0x4F; regName = "VBK"; return true;
            case "__cgb_safe_set_svbk": ioOffset = 0x70; regName = "SVBK"; return true;
            case "__cgb_safe_set_bgpi":
            case "__cgb_safe_set_bcps": ioOffset = 0x68; regName = "BCPS"; return true;
            case "__cgb_safe_set_bgpd":
            case "__cgb_safe_set_bcpd": ioOffset = 0x69; regName = "BCPD"; return true;
            case "__cgb_safe_set_obpi":
            case "__cgb_safe_set_ocps": ioOffset = 0x6A; regName = "OCPS"; return true;
            case "__cgb_safe_set_obpd":
            case "__cgb_safe_set_ocpd": ioOffset = 0x6B; regName = "OCPD"; return true;
            case "__cgb_safe_set_hdma1": ioOffset = 0x51; regName = "HDMA1"; return true;
            case "__cgb_safe_set_hdma2": ioOffset = 0x52; regName = "HDMA2"; return true;
            case "__cgb_safe_set_hdma3": ioOffset = 0x53; regName = "HDMA3"; return true;
            case "__cgb_safe_set_hdma4": ioOffset = 0x54; regName = "HDMA4"; return true;
            case "__cgb_safe_set_hdma5": ioOffset = 0x55; regName = "HDMA5"; return true;
            default: return false;
        }
    }

    // Read the three WRAM-bank bits and normalize bank zero to the effective bank-one selection.
    void EmitReadNormalizedSvbkIntoA()
    {
        EmitAsm("LDH_A_MEM", new AsmOperand(0x70, AddressMode.HighMem));
        EmitAsm("AND_IMM", new AsmOperand(0x07, AddressMode.Immediate));
        AsmOperand ok = MakeUniqueLabel("svbk_norm_ok");
        EmitAsm("OR_A");
        EmitAsm("JR_NZ", ok);
        EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate));
        EmitLabel(ok);
    }

    // Load a configured known result directly, otherwise request/call the runtime probe and count that emitted check.
    void EmitRuntimeCgbCheckCall()
    {
        if (Program.TryGetKnownCgbRuntimeValue(out int knownCgbValue))
        {
            EmitAsm("LD_A_IMM", new AsmOperand(knownCgbValue, AddressMode.Immediate));
            return;
        }

        UseCgbRuntimeDetectHelper = true;
        EmitAsm("CALL", new AsmOperand("__kq_is_cgb", AddressMode.Absolute));
        CgbRuntimeCheckCount++;
    }

    // Return true only when compile-time runtime-mode knowledge explicitly identifies CGB.
    bool IsKnownCgbRuntimeTrue()
    {
        return Program.TryGetKnownCgbRuntimeValue(out int knownCgbValue) && knownCgbValue != 0;
    }

    // Zero-extend A into HL and push a two-byte word.
    void EmitPushAAsWord()
    {
        EmitAsm("LD_L_A");
        EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("PUSH_HL");
    }

    // Push B zero-extended through HL without changing the live value in A.
    void EmitPushBAsWord()
    {
        EmitAsm("LD_L_B");
        EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("PUSH_HL");
    }

    // Push D zero-extended through HL without changing the live value in A.
    void EmitPushDAsWord()
    {
        EmitAsm("LD_L_D");
        EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("PUSH_HL");
    }

    // Evaluate the expression as a byte and push it zero-extended; the upper expression byte is not retained.
    void EmitPushExprAsWord(Expr expr)
    {
        CompileIntoA(expr);
        EmitPushAAsWord();
    }

    // Evaluate and push the full word result in HL.
    void EmitPushExprAsWideWord(Expr expr)
    {
        CompileIntoHL(expr);
        EmitAsm("PUSH_HL");
    }

    // Pop the requested word count into HL; nonpositive counts emit nothing.
    void EmitDiscardStackWords(int count)
    {
        for (int i = 0; i < count; ++i)
            EmitAsm("POP_HL");
    }

    // Write only the low VBK bit and record the write; a hardware-mode guard must be supplied by the surrounding path.
    void EmitSetVbkUnchecked(byte value)
    {
        EmitAsm("LD_A_IMM", new AsmOperand(value & 0x01, AddressMode.Immediate));
        EmitAsm("LDH_MEM_A", new AsmOperand(0x4F, AddressMode.HighMem));
        CgbGuardedWriteCount++;
        CgbGuardedRegisters.Add("VBK");
    }

    // Select and mark the ordinary or fast single-tile helper for later injection.
    void EmitCallSetTileCore(bool fast)
    {
        if (fast)
        {
            UsedSetTileFastCore = true;
            EmitAsm("CALL", new AsmOperand("__settile_fast_core", AddressMode.Absolute));
        }
        else
        {
            UsedSetTileCore = true;
            EmitAsm("CALL", new AsmOperand("__settile_core", AddressMode.Absolute));
        }
    }

    // Select and mark the ordinary or fast bulk-tile helper for later injection.
    void EmitCallSetTileBulkCore(bool fast)
    {
        if (fast)
        {
            UsedSetTileBulkFastCore = true;
            EmitAsm("CALL", new AsmOperand("__settile_bulk_fast_core", AddressMode.Absolute));
        }
        else
        {
            UsedSetTileBulkCore = true;
            EmitAsm("CALL", new AsmOperand("__settile_bulk_core", AddressMode.Absolute));
        }
    }

    // Select $9800 or $9C00 in DE from the requested LCDC map-selection bit.
    void EmitLoadTileMapBaseToDe(int lcdcMask, string labelPrefix)
    {
        AsmOperand base0 = MakeUniqueLabel(labelPrefix + "_base0");
        AsmOperand done = MakeUniqueLabel(labelPrefix + "_done");

        EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
        EmitAsm("AND_IMM", new AsmOperand(lcdcMask, AddressMode.Immediate));
        EmitAsm("JR_Z", base0);
        EmitAsm("LD_DE_IMM", new AsmOperand(0x9C00, AddressMode.Immediate));
        EmitAsm("JR", done);
        EmitLabel(base0);
        EmitAsm("LD_DE_IMM", new AsmOperand(0x9800, AddressMode.Immediate));
        EmitLabel(done);
    }

    // Select $9800 or $9C00 in HL from the requested LCDC map-selection bit.
    void EmitLoadTileMapBaseToHl(int lcdcMask, string labelPrefix)
    {
        AsmOperand base0 = MakeUniqueLabel(labelPrefix + "_base0");
        AsmOperand done = MakeUniqueLabel(labelPrefix + "_done");

        EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
        EmitAsm("AND_IMM", new AsmOperand(lcdcMask, AddressMode.Immediate));
        EmitAsm("JR_Z", base0);
        EmitAsm("LD_HL_IMM", new AsmOperand(0x9C00, AddressMode.Immediate16));
        EmitAsm("JR", done);
        EmitLabel(base0);
        EmitAsm("LD_HL_IMM", new AsmOperand(0x9800, AddressMode.Immediate16));
        EmitLabel(done);
    }

    // Copy BC bytes forward from DE to HL, advancing both pointers and consuming BC; zero length skips the load/store.
    void EmitRamMemcpyLoop(string labelPrefix)
    {
        AsmOperand loop = MakeUniqueLabel(labelPrefix + "_loop");
        AsmOperand done = MakeUniqueLabel(labelPrefix + "_done");

        EmitLabel(loop);
        EmitAsm("LD_A_B");
        EmitAsm("OR_C");
        EmitAsm("JR_Z", done);
        EmitAsm("LD_A_DE");
        EmitAsm("LDI_HL_A");
        EmitAsm("INC_DE");
        EmitAsm("DEC_BC");
        EmitAsm("JR", loop);
        EmitLabel(done);
    }

    // Copy BC bytes from DE to HL; safe LCD-on writes each use STAT polling under DI followed by EI.
    // The caller supplies valid spans, and the helper does not preserve an initially disabled interrupt state.
    void EmitVramMemcpyLoop(bool safe, string labelPrefix)
    {
        AsmOperand fast = MakeUniqueLabel(labelPrefix + "_fast");
        AsmOperand safeLoop = MakeUniqueLabel(labelPrefix + "_safe_loop");
        AsmOperand safeWait = MakeUniqueLabel(labelPrefix + "_safe_wait");
        AsmOperand fastLoop = MakeUniqueLabel(labelPrefix + "_fast_loop");
        AsmOperand done = MakeUniqueLabel(labelPrefix + "_done");

        EmitAsm("LD_A_B");
        EmitAsm("OR_C");
        EmitAsm("JR_Z", done);

        if (safe)
        {
            EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
            EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
            EmitAsm("JR_Z", fast);

            EmitLabel(safeLoop);
            EmitAsm("LD_A_B");
            EmitAsm("OR_C");
            EmitAsm("JR_Z", done);
            EmitAsm("DI");
            EmitLabel(safeWait);
            EmitAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem)); // STAT
            EmitAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate));
            EmitAsm("JR_NZ", safeWait);
            EmitAsm("LD_A_DE");
            EmitAsm("LDI_HL_A");
            EmitAsm("EI");
            EmitAsm("INC_DE");
            EmitAsm("DEC_BC");
            EmitAsm("JR", safeLoop);
        }

        EmitLabel(fast);
        EmitLabel(fastLoop);
        EmitAsm("LD_A_B");
        EmitAsm("OR_C");
        EmitAsm("JR_Z", done);
        EmitAsm("LD_A_DE");
        EmitAsm("LDI_HL_A");
        EmitAsm("INC_DE");
        EmitAsm("DEC_BC");
        EmitAsm("JR", fastLoop);
        EmitLabel(done);
    }

    // Unroll constant counts from zero through eight without consuming BC; optionally emit per-byte LCD-on wait sections.
    bool TryEmitVramMemcpyConstCount(bool safe, int count, string labelPrefix)
    {
        if (count < 0 || count > 8) return false;

        AsmOperand fast = default;
        AsmOperand done = default;
        if (safe && count > 0)
        {
            fast = MakeUniqueLabel(labelPrefix + "_fast");
            done = MakeUniqueLabel(labelPrefix + "_done");
            EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
            EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
            EmitAsm("JP_Z", fast);

            for (int i = 0; i < count; i++)
            {
                AsmOperand wait = MakeUniqueLabel(labelPrefix + "_safe_wait");
                EmitAsm("DI");
                EmitLabel(wait);
                EmitAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem)); // STAT
                EmitAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate));
                EmitAsm("JR_NZ", wait);
                EmitAsm("LD_A_DE");
                EmitAsm("LDI_HL_A");
                EmitAsm("EI");
                EmitAsm("INC_DE");
            }

            EmitAsm("JP", done);
            EmitLabel(fast);
        }

        for (int i = 0; i < count; i++)
        {
            EmitAsm("LD_A_DE");
            EmitAsm("LDI_HL_A");
            EmitAsm("INC_DE");
        }

        if (safe && count > 0)
            EmitLabel(done);

        return true;
    }

    // Load a masked constant word or zero-extended byte/word expression into BC for transfer loops.
    void EmitLengthExprIntoBC(Expr lenExpr)
    {
        Expr foldedLen = FoldConstants(lenExpr);
        if (foldedLen.Match(Tag.Integer, out int lenConst))
        {
            EmitAsm("LD_B_IMM", new AsmOperand((lenConst >> 8) & 0xFF, AddressMode.Immediate));
            EmitAsm("LD_C_IMM", new AsmOperand(lenConst & 0xFF, AddressMode.Immediate));
            return;
        }

        if (SizeOf(foldedLen) == 1)
        {
            CompileIntoA(foldedLen);
            EmitAsm("LD_C_A");
            EmitAsm("LD_B_IMM", new AsmOperand(0, AddressMode.Immediate));
            return;
        }

        CompileIntoHL(foldedLen);
        EmitAsm("LD_B_H");
        EmitAsm("LD_C_L");
    }

    // Use the shared tile-map address calculation for rectangle operations.
    void EmitTileMapRectAddressIntoHL(Expr baseExpr, Expr xExpr, Expr yExpr)
    {
        EmitTileMapAddressIntoHL(baseExpr, xExpr, yExpr);
    }

    // Compute base + 32*y + x using byte coordinates, specializing constant coordinates to avoid runtime arithmetic.
    // This helper does not clamp coordinates to the map dimensions.
    void EmitTileMapAddressIntoHL(Expr baseExpr, Expr xExpr, Expr yExpr)
    {
        Expr foldedBaseExpr = FoldConstants(baseExpr);
        Expr foldedXExpr = FoldConstants(xExpr);
        Expr foldedYExpr = FoldConstants(yExpr);

        if (TryGetU8Const(foldedXExpr, out int fullXConst) &&
            TryGetU8Const(foldedYExpr, out int fullYConst))
        {
            CompileIntoHL(foldedBaseExpr);
            int foldedOffset = ((fullYConst & 0xFF) << 5) + (fullXConst & 0xFF);
            if (foldedOffset != 0)
                EmitAddImm16ToHL(foldedOffset);
            return;
        }

        if (TryGetU8Const(foldedYExpr, out int yConst))
        {
            CompileIntoHL(foldedBaseExpr);

            int yOffset = (yConst & 0xFF) << 5;
            if (yOffset != 0)
                EmitAddImm16ToHL(yOffset);

            if (TryGetU8Const(foldedXExpr, out int xAfterYConst))
            {
                int xOffset = xAfterYConst & 0xFF;
                if (xOffset != 0)
                    EmitAddImm16ToHL(xOffset);
            }
            else
            {
                CompileIntoA(foldedXExpr);
                EmitAsm("LD_E_A");
                EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_DE");
            }
            return;
        }

        if (TryGetU8Const(foldedXExpr, out int xConst))
        {
            CompileIntoA(foldedYExpr);
            EmitAsm("LD_L_A");
            EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
            EmitAsm("ADD_HL_HL");
            EmitAsm("ADD_HL_HL");
            EmitAsm("ADD_HL_HL");
            EmitAsm("ADD_HL_HL");
            EmitAsm("ADD_HL_HL");

            int xOffset = xConst & 0xFF;
            if (xOffset != 0)
                EmitAddImm16ToHL(xOffset);

            EmitAsm("PUSH_HL");
            CompileIntoHL(foldedBaseExpr);
            EmitAsm("POP_DE");
            EmitAsm("ADD_HL_DE");
            return;
        }

        CompileIntoA(foldedYExpr);
        EmitAsm("LD_L_A");
        EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("ADD_HL_HL");
        EmitAsm("ADD_HL_HL");
        EmitAsm("ADD_HL_HL");
        EmitAsm("ADD_HL_HL");
        EmitAsm("ADD_HL_HL");
        EmitAsm("PUSH_HL");
        CompileIntoHL(foldedBaseExpr);
        EmitAsm("POP_DE");
        EmitAsm("ADD_HL_DE");

        CompileIntoA(foldedXExpr);
        EmitAsm("LD_E_A");
        EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("ADD_HL_DE");
    }

    // Unroll up to eight sequential source bytes into a column with destination stride 32.
    // HL remains at the last written cell; DE advances past the source bytes.
    bool TryEmitVramColumnCopyConstCount(bool safe, int count, string labelPrefix)
    {
        if (count < 0 || count > 8) return false;

        AsmOperand fast = default;
        AsmOperand done = default;
        if (safe && count > 0)
        {
            fast = MakeUniqueLabel(labelPrefix + "_fast");
            done = MakeUniqueLabel(labelPrefix + "_done");
            EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
            EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
            EmitAsm("JP_Z", fast);

            for (int i = 0; i < count; i++)
            {
                AsmOperand wait = MakeUniqueLabel(labelPrefix + "_safe_wait");
                EmitAsm("DI");
                EmitLabel(wait);
                EmitAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem)); // STAT
                EmitAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate));
                EmitAsm("JR_NZ", wait);
                EmitAsm("LD_A_DE");
                EmitAsm("LD_HL_A");
                EmitAsm("EI");
                EmitAsm("INC_DE");

                if (i + 1 >= count) continue;

                EmitAsm("LD_A_L");
                EmitAsm("ADD_A_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("LD_L_A");
                EmitAsm("LD_A_H");
                EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("LD_H_A");
            }

            EmitAsm("JP", done);
            EmitLabel(fast);
        }

        for (int i = 0; i < count; i++)
        {
            EmitAsm("LD_A_DE");
            EmitAsm("LD_HL_A");
            EmitAsm("INC_DE");

            if (i + 1 >= count) continue;

            EmitAsm("LD_A_L");
            EmitAsm("ADD_A_IMM", new AsmOperand(32, AddressMode.Immediate));
            EmitAsm("LD_L_A");
            EmitAsm("LD_A_H");
            EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
            EmitAsm("LD_H_A");
        }

        if (safe && count > 0)
            EmitLabel(done);

        return true;
    }

    // Fill BC consecutive bytes at HL with D, using optional per-byte LCD/STAT waits and consuming BC.
    void EmitVramMemsetLoop(bool safe, string labelPrefix)
    {
        AsmOperand fast = MakeUniqueLabel(labelPrefix + "_fast");
        AsmOperand safeLoop = MakeUniqueLabel(labelPrefix + "_safe_loop");
        AsmOperand safeWait = MakeUniqueLabel(labelPrefix + "_safe_wait");
        AsmOperand fastLoop = MakeUniqueLabel(labelPrefix + "_fast_loop");
        AsmOperand done = MakeUniqueLabel(labelPrefix + "_done");

        EmitAsm("LD_A_B");
        EmitAsm("OR_C");
        EmitAsm("JR_Z", done);

        if (safe)
        {
            EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
            EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
            EmitAsm("JR_Z", fast);

            EmitLabel(safeLoop);
            EmitAsm("LD_A_B");
            EmitAsm("OR_C");
            EmitAsm("JR_Z", done);
            EmitAsm("DI");
            EmitLabel(safeWait);
            EmitAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem)); // STAT
            EmitAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate));
            EmitAsm("JR_NZ", safeWait);
            EmitAsm("LD_A_D");
            EmitAsm("LDI_HL_A");
            EmitAsm("EI");
            EmitAsm("DEC_BC");
            EmitAsm("JR", safeLoop);
        }

        EmitLabel(fast);
        EmitLabel(fastLoop);
        EmitAsm("LD_A_B");
        EmitAsm("OR_C");
        EmitAsm("JR_Z", done);
        EmitAsm("LD_A_D");
        EmitAsm("LDI_HL_A");
        EmitAsm("DEC_BC");
        EmitAsm("JR", fastLoop);
        EmitLabel(done);
    }

    // Unroll up to eight fills from D, optionally waiting before each LCD-on write; zero count emits no memory access.
    bool TryEmitVramMemsetConstCount(bool safe, int count, string labelPrefix)
    {
        if (count < 0 || count > 8) return false;

        AsmOperand fast = default;
        AsmOperand done = default;
        if (safe && count > 0)
        {
            fast = MakeUniqueLabel(labelPrefix + "_fast");
            done = MakeUniqueLabel(labelPrefix + "_done");
            EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
            EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
            EmitAsm("JP_Z", fast);

            for (int i = 0; i < count; i++)
            {
                AsmOperand wait = MakeUniqueLabel(labelPrefix + "_safe_wait");
                EmitAsm("DI");
                EmitLabel(wait);
                EmitAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem)); // STAT
                EmitAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate));
                EmitAsm("JR_NZ", wait);
                EmitAsm("LD_A_D");
                EmitAsm("LDI_HL_A");
                EmitAsm("EI");
            }

            EmitAsm("JP", done);
            EmitLabel(fast);
        }

        for (int i = 0; i < count; i++)
        {
            EmitAsm("LD_A_D");
            EmitAsm("LDI_HL_A");
        }

        if (safe && count > 0)
            EmitLabel(done);

        return true;
    }

    // Store C through HL directly or after an LCD-on STAT wait; the guarded path uses DI/EI rather than saving interrupt state.
    void EmitVramStoreCToHl(bool safe, string labelPrefix)
    {
        if (!safe)
        {
            EmitAsm("LD_HL_C");
            return;
        }

        AsmOperand writeNow = MakeUniqueLabel(labelPrefix + "_write_now");
        AsmOperand wait = MakeUniqueLabel(labelPrefix + "_wait");
        AsmOperand done = MakeUniqueLabel(labelPrefix + "_done");

        EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
        EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
        EmitAsm("JR_Z", writeNow);

        EmitAsm("DI");
        EmitLabel(wait);
        EmitAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem)); // STAT
        EmitAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate));
        EmitAsm("JR_NZ", wait);
        EmitAsm("LD_HL_C");
        EmitAsm("EI");
        EmitAsm("JR", done);

        EmitLabel(writeNow);
        EmitAsm("LD_HL_C");
        EmitLabel(done);
    }

    // Request the common-bank far-copy helper and emit its call.
    void EmitCallFarMemcpyHelper()
    {
        UseFarMemcpyBank0Helper = true;
        EmitAsm("CALL", new AsmOperand("__kq_far_memcpy_bank0", AddressMode.Absolute));
    }

    // Compute a tile-map offset from B=x and D=y with optional doubled axes.
    // The doubled X path uses an eight-bit rotate, so callers must supply the supported coordinate range.
    void EmitComputeTile16CellOffsetToHlFromRegs(bool doubleX, bool doubleY)
    {
        // Inputs: B=x, D=y.
        EmitAsm("LD_A_D");
        EmitAsm("LD_L_A");
        EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("ADD_HL_HL"); // y * 2
        if (doubleY)
        {
            EmitAsm("ADD_HL_HL");
        }
        EmitAsm("ADD_HL_HL");
        EmitAsm("ADD_HL_HL");
        EmitAsm("ADD_HL_HL");
        EmitAsm("ADD_HL_HL");
        // Result so far: y * (doubleY ? 64 : 32)

        EmitAsm("LD_A_B");
        if (doubleX)
            EmitAsm("RLCA");
        EmitAsm("LD_E_A");
        EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("ADD_HL_DE");
    }

    // Write a 2-by-2 tile cell into a RAM buffer with a 32-byte tile-row stride.
    // The caller supplies valid cell coordinates and enough storage for both rows.
    void EmitTile16BufferedWriteIntrinsic(Expr bufExpr, Expr xExpr, Expr yExpr, Expr valueExpr, bool incrementQuad, string labelPrefix)
    {
        EmitPushExprAsWideWord(bufExpr);
        EmitPushExprAsWord(xExpr);
        EmitPushExprAsWord(yExpr);

        CompileIntoA(valueExpr);
        EmitAsm("LD_C_A");

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_D_A");

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_B_A");

        // Double both cell coordinates to locate the upper-left tile in the buffer.
        EmitComputeTile16CellOffsetToHlFromRegs(true, true);

        EmitAsm("POP_DE");
        EmitAsm("ADD_HL_DE");

        EmitAsm("LD_HL_C");
        EmitAsm("INC_HL");
        if (incrementQuad) EmitAsm("INC_C");
        EmitAsm("LD_HL_C");

        // After writing the upper-right tile, skip to the lower-left tile.
        // Quad mode increments the byte value in row-major order; fill mode repeats it.
        EmitAsm("LD_DE_IMM", new AsmOperand(31, AddressMode.Immediate));
        EmitAsm("ADD_HL_DE");
        if (incrementQuad) EmitAsm("INC_C");
        EmitAsm("LD_HL_C");
        EmitAsm("INC_HL");
        if (incrementQuad) EmitAsm("INC_C");
        EmitAsm("LD_HL_C");
    }

    // Emit two row copies from a 32-byte-stride buffer to the selected background map.
    // X and count are tile units, while Y is a 16-pixel cell row; count is narrowed to a byte.
    void EmitTile16FlushRowsIntrinsic(Expr bufExpr, Expr yExpr, Expr xExpr, Expr countExpr, bool cgbAttr, string labelPrefix)
    {
        Expr foldedCountExpr = FoldConstants(countExpr);

        EmitPushExprAsWideWord(bufExpr);
        EmitPushExprAsWord(yExpr);
        EmitPushExprAsWord(xExpr);
        EmitPushExprAsWord(foldedCountExpr);

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_C_A"); // count

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_B_A"); // x in tiles

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_D_A"); // y in cells

        EmitAsm("POP_HL");
        EmitAsm("PUSH_HL"); // preserve buffer base while offset math uses HL/DE

        EmitComputeTile16CellOffsetToHlFromRegs(false, true);

        EmitAsm("POP_DE"); // DE = buffer base
        EmitAsm("PUSH_HL"); // preserve the offset before DE replaces the saved Y
        EmitAsm("ADD_HL_DE"); // HL = src row0 start
        EmitAsm("LD_D_H");
        EmitAsm("LD_E_L");
        EmitAsm("POP_HL"); // reuse the original offset for the destination map
        EmitAsm("PUSH_DE"); // save src row0 start
        EmitLoadTileMapBaseToDe(0x08, labelPrefix + "_bgbase");
        EmitAsm("ADD_HL_DE"); // HL = dest row0 start

        EmitAsm("POP_DE"); // DE = src row0 start
        EmitAsm("PUSH_HL");
        EmitAsm("PUSH_DE");
        EmitAsm("LD_B_C");
        // Use the small constant-count VRAM copy when available, otherwise the shared copy core.
        if (foldedCountExpr.Match(Tag.Integer, out int tile16CountConst) &&
            TryEmitVramMemcpyConstCount(true, tile16CountConst & 0xFF, labelPrefix + "_row0_small"))
        {
        }
        else
        {
            EmitCallSetTileBulkCore(false);
        }
        EmitAsm("POP_DE"); // src row0 start
        EmitAsm("POP_HL"); // dest row0 start

        // Advance both starts by one tile row (32 bytes).
        EmitAsm("PUSH_DE");
        EmitAsm("LD_DE_IMM", new AsmOperand(32, AddressMode.Immediate));
        EmitAsm("ADD_HL_DE");
        EmitAsm("POP_DE");

        EmitAsm("PUSH_HL");
        EmitAsm("LD_H_D");
        EmitAsm("LD_L_E");
        EmitAsm("LD_DE_IMM", new AsmOperand(32, AddressMode.Immediate));
        EmitAsm("ADD_HL_DE");
        EmitAsm("LD_D_H");
        EmitAsm("LD_E_L");
        EmitAsm("POP_HL");

        EmitAsm("LD_B_C");
        // Apply the same count to the lower tile row after restoring and advancing both starts.
        if (foldedCountExpr.Match(Tag.Integer, out int tile16CountConst2) &&
            TryEmitVramMemcpyConstCount(true, tile16CountConst2 & 0xFF, labelPrefix + "_row1_small"))
        {
        }
        else
        {
            EmitCallSetTileBulkCore(false);
        }

        if (cgbAttr)
            EmitSetVbkUnchecked(0);
    }

    // Write one byte at base + 32*y + x, rejecting unsigned coordinates outside 0..31.
    // The safe LCD-on path waits for STAT mode 0/1 and enables interrupts after the write.
    void EmitTileWriteAtDeFromRegs(bool safe, string labelPrefix)
    {
        AsmOperand end = MakeUniqueLabel(labelPrefix + "_end");

        // Inputs: B=x, A=y, C=tile, DE=base.
        EmitAsm("LD_L_A");
        EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));

        EmitAsm("LD_A_B");
        EmitAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate));
        EmitAsm("JR_NC", end);

        EmitAsm("LD_A_L");
        EmitAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate));
        EmitAsm("JR_NC", end);

        EmitAsm("LD_L_A");
        EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("ADD_HL_HL");
        EmitAsm("ADD_HL_HL");
        EmitAsm("ADD_HL_HL");
        EmitAsm("ADD_HL_HL");
        EmitAsm("ADD_HL_HL");

        EmitAsm("ADD_HL_DE");

        EmitAsm("LD_A_B");
        EmitAsm("LD_E_A");
        EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("ADD_HL_DE");

        if (safe)
        {
            AsmOperand writeNow = MakeUniqueLabel(labelPrefix + "_write_now");
            AsmOperand wait = MakeUniqueLabel(labelPrefix + "_wait");
            AsmOperand done = MakeUniqueLabel(labelPrefix + "_done");

            EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
            EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
            EmitAsm("JR_Z", writeNow);

            EmitAsm("DI");
            EmitLabel(wait);
            EmitAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem)); // STAT
            EmitAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate));
            EmitAsm("JR_NZ", wait);

            EmitAsm("LD_HL_C");
            EmitAsm("EI");
            EmitAsm("JR", done);

            EmitLabel(writeNow);
            EmitAsm("LD_HL_C");
            EmitLabel(done);
        }
        else
        {
            EmitAsm("LD_HL_C");
        }

        EmitLabel(end);
    }

    // Use the supplied DE base, or select a window/background map from LCDC.
    // Save Y across map selection because reading LCDC overwrites A.
    void EmitTileWriteForTargetFromRegs(CgbTileTargetKind target, bool safe, string labelPrefix)
    {
        switch (target)
        {
            case CgbTileTargetKind.At:
                EmitTileWriteAtDeFromRegs(safe, labelPrefix + "_at");
                return;
            case CgbTileTargetKind.Window:
                EmitAsm("LD_H_A");
                EmitLoadTileMapBaseToDe(0x40, labelPrefix + "_winbase");
                EmitAsm("LD_A_H");
                EmitTileWriteAtDeFromRegs(safe, labelPrefix + "_win");
                return;
            case CgbTileTargetKind.Bg:
                EmitAsm("LD_H_A");
                EmitLoadTileMapBaseToDe(0x08, labelPrefix + "_bgbase");
                EmitAsm("LD_A_H");
                EmitTileWriteAtDeFromRegs(safe, labelPrefix + "_bg");
                return;
        }
    }

    // Emit a tile attribute write in VRAM bank 1 and return to bank 0 afterward.
    // Without a known CGB runtime, evaluate arguments first and skip the write on DMG.
    void EmitSetTileAttrIntrinsic(Expr xExpr, Expr yExpr, Expr attrExpr, bool fast)
    {
        if (IsKnownCgbRuntimeTrue())
        {
            EmitPushExprAsWord(xExpr);
            EmitPushExprAsWord(yExpr);

            CompileIntoA(attrExpr);
            EmitAsm("LD_C_A");

            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");
            EmitAsm("LD_D_A");

            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");
            EmitAsm("LD_B_A");

            EmitSetVbkUnchecked(1);
            // Selecting VBK overwrites A; restore the saved Y coordinate afterward.
            EmitAsm("LD_A_D");
            EmitCallSetTileCore(fast);
            EmitSetVbkUnchecked(0);
            return;
        }

        // Keep argument words on the stack until the runtime CGB check has finished.
        AsmOperand skip = MakeUniqueLabel("settileattr_skip");
        AsmOperand done = MakeUniqueLabel("settileattr_done");

        EmitPushExprAsWord(xExpr);
        EmitPushExprAsWord(yExpr);
        EmitPushExprAsWord(attrExpr);

        EmitRuntimeCgbCheckCall();
        EmitAsm("OR_A");
        EmitAsm("JR_Z", skip);

        EmitSetVbkUnchecked(1);

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_C_A");

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_D_A");

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_B_A");
        EmitAsm("LD_A_D");

        EmitCallSetTileCore(fast);
        EmitSetVbkUnchecked(0);
        EmitAsm("JR", done);

        EmitLabel(skip);
        EmitDiscardStackWords(3);
        EmitLabel(done);
    }

    // Evaluate and save X, Y, tile and attribute bytes before issuing either write.
    // The tile write uses the current bank (normally 0); CGB attributes use bank 1, then bank 0 is restored.
    void EmitSetTileCgbIntrinsic(Expr xExpr, Expr yExpr, Expr tileExpr, Expr attrExpr, bool fast)
    {
        if (IsKnownCgbRuntimeTrue())
        {
            EmitPushExprAsWord(xExpr);
            EmitPushExprAsWord(yExpr);
            EmitPushExprAsWord(tileExpr);
            EmitPushExprAsWord(attrExpr);

            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");
            EmitAsm("LD_D_A");

            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");
            EmitAsm("LD_C_A");

            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");

            EmitAsm("POP_HL");
            EmitAsm("LD_B_L");

            EmitPushBAsWord();
            EmitPushAAsWord();
            EmitPushDAsWord();

            EmitCallSetTileCore(fast);

            EmitSetVbkUnchecked(1);

            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");
            EmitAsm("LD_C_A");

            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");
            EmitAsm("LD_D_A");

            EmitAsm("POP_HL");
            EmitAsm("LD_B_L");
            EmitAsm("LD_A_D");

            EmitCallSetTileCore(fast);
            EmitSetVbkUnchecked(0);
            return;
        }

        // On DMG, still write the tile and discard the three saved attribute-pass arguments.
        AsmOperand skip = MakeUniqueLabel("settilecgb_skip");
        AsmOperand done = MakeUniqueLabel("settilecgb_done");

        EmitPushExprAsWord(xExpr);
        EmitPushExprAsWord(yExpr);
        EmitPushExprAsWord(tileExpr);
        EmitPushExprAsWord(attrExpr);

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_D_A");

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_C_A");

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");

        EmitAsm("POP_HL");
        EmitAsm("LD_B_L");

        EmitPushBAsWord();
        EmitPushAAsWord();
        EmitPushDAsWord();

        EmitCallSetTileCore(fast);

        EmitRuntimeCgbCheckCall();
        EmitAsm("OR_A");
        EmitAsm("JR_Z", skip);

        EmitSetVbkUnchecked(1);

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_C_A");

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_D_A");

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_B_A");
        EmitAsm("LD_A_D");

        EmitCallSetTileCore(fast);
        EmitSetVbkUnchecked(0);
        EmitAsm("JR", done);

        EmitLabel(skip);
        EmitDiscardStackWords(3);
        EmitLabel(done);
    }

    // Write one attribute to an explicit, window or background map only on CGB.
    // The explicit base is a full word; coordinate and attribute arguments are bytes.
    void EmitTileTargetAttrIntrinsic(CgbTileTargetKind target, Expr baseExpr, Expr xExpr, Expr yExpr, Expr attrExpr, bool safe, string labelPrefix)
    {
        if (IsKnownCgbRuntimeTrue())
        {
            if (target == CgbTileTargetKind.At)
                EmitPushExprAsWideWord(baseExpr);
            EmitPushExprAsWord(xExpr);
            EmitPushExprAsWord(yExpr);
            EmitPushExprAsWord(attrExpr);

            // Select the bank before loading the Y argument into A.
            EmitSetVbkUnchecked(1);

            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");
            EmitAsm("LD_C_A");

            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");

            EmitAsm("POP_HL");
            EmitAsm("LD_B_L");

            if (target == CgbTileTargetKind.At)
            {
                EmitAsm("POP_HL");
                EmitAsm("LD_D_H");
                EmitAsm("LD_E_L");
            }

            EmitTileWriteForTargetFromRegs(target, safe, labelPrefix);
            EmitSetVbkUnchecked(0);
            return;
        }

        AsmOperand skip = MakeUniqueLabel(labelPrefix + "_skip");
        AsmOperand done = MakeUniqueLabel(labelPrefix + "_done");
        int wordCount = (target == CgbTileTargetKind.At) ? 4 : 3;

        if (target == CgbTileTargetKind.At)
            EmitPushExprAsWideWord(baseExpr);
        EmitPushExprAsWord(xExpr);
        EmitPushExprAsWord(yExpr);
        EmitPushExprAsWord(attrExpr);

        EmitRuntimeCgbCheckCall();
        EmitAsm("OR_A");
        EmitAsm("JR_Z", skip);

        EmitSetVbkUnchecked(1);

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_C_A");

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");

        EmitAsm("POP_HL");
        EmitAsm("LD_B_L");

        if (target == CgbTileTargetKind.At)
        {
            EmitAsm("POP_HL");
            EmitAsm("LD_D_H");
            EmitAsm("LD_E_L");
        }

        EmitTileWriteForTargetFromRegs(target, safe, labelPrefix);
        EmitSetVbkUnchecked(0);
        EmitAsm("JR", done);

        EmitLabel(skip);
        EmitDiscardStackWords(wordCount);
        EmitLabel(done);
    }

    // Write tile and attribute passes at the same selected map coordinate.
    // Keep the second pass on the stack because map selection and VRAM writes use the working registers.
    void EmitTileTargetCgbIntrinsic(CgbTileTargetKind target, Expr baseExpr, Expr xExpr, Expr yExpr, Expr tileExpr, Expr attrExpr, bool safe, string labelPrefix)
    {
        if (IsKnownCgbRuntimeTrue())
        {
            if (target == CgbTileTargetKind.At)
                EmitPushExprAsWideWord(baseExpr);
            EmitPushExprAsWord(xExpr);
            EmitPushExprAsWord(yExpr);
            EmitPushExprAsWord(tileExpr);
            EmitPushExprAsWord(attrExpr);

            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");
            EmitAsm("LD_D_A");

            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");
            EmitAsm("LD_C_A");

            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");

            EmitAsm("POP_HL");
            EmitAsm("LD_B_L");

            if (target == CgbTileTargetKind.At)
            {
                EmitAsm("POP_HL");
                EmitAsm("PUSH_HL");
                EmitAsm("PUSH_DE"); // Save the attribute in the high byte; preserve A and the base in HL.
                EmitAsm("LD_D_H");
                EmitAsm("LD_E_L");
                EmitPushBAsWord();
                EmitPushAAsWord();
            }
            else
            {
                EmitPushBAsWord();
                EmitPushAAsWord();
                EmitPushDAsWord();
            }

            EmitTileWriteForTargetFromRegs(target, safe, labelPrefix + "_tile");

            EmitSetVbkUnchecked(1);

            if (target == CgbTileTargetKind.At)
            {
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");

                EmitAsm("POP_HL");
                EmitAsm("LD_B_L");

                EmitAsm("POP_HL");
                EmitAsm("LD_C_H");

                EmitAsm("POP_HL");
                EmitAsm("LD_D_H");
                EmitAsm("LD_E_L");
            }
            else
            {
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("LD_C_A");

                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");

                EmitAsm("POP_HL");
                EmitAsm("LD_B_L");
            }

            EmitTileWriteForTargetFromRegs(target, safe, labelPrefix + "_attr");
            EmitSetVbkUnchecked(0);
            return;
        }

        AsmOperand skip = MakeUniqueLabel(labelPrefix + "_skip");
        AsmOperand done = MakeUniqueLabel(labelPrefix + "_done");
        int wordCount = (target == CgbTileTargetKind.At) ? 4 : 3;

        if (target == CgbTileTargetKind.At)
            EmitPushExprAsWideWord(baseExpr);
        EmitPushExprAsWord(xExpr);
        EmitPushExprAsWord(yExpr);
        EmitPushExprAsWord(tileExpr);
        EmitPushExprAsWord(attrExpr);

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_D_A");

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");
        EmitAsm("LD_C_A");

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");

        EmitAsm("POP_HL");
        EmitAsm("LD_B_L");

        if (target == CgbTileTargetKind.At)
        {
            EmitAsm("POP_HL");
            EmitAsm("PUSH_HL");
            EmitAsm("PUSH_DE"); // Save the attribute in the high byte; preserve A and the base in HL.
            EmitAsm("LD_D_H");
            EmitAsm("LD_E_L");
            EmitPushBAsWord();
            EmitPushAAsWord();
        }
        else
        {
            EmitPushBAsWord();
            EmitPushAAsWord();
            EmitPushDAsWord();
        }

        EmitTileWriteForTargetFromRegs(target, safe, labelPrefix + "_tile");

        EmitRuntimeCgbCheckCall();
        EmitAsm("OR_A");
        EmitAsm("JR_Z", skip);

        EmitSetVbkUnchecked(1);

        if (target == CgbTileTargetKind.At)
        {
            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");

            EmitAsm("POP_HL");
            EmitAsm("LD_B_L");

            EmitAsm("POP_HL");
            EmitAsm("LD_C_H");

            EmitAsm("POP_HL");
            EmitAsm("LD_D_H");
            EmitAsm("LD_E_L");
        }
        else
        {
            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");
            EmitAsm("LD_C_A");

            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");

            EmitAsm("POP_HL");
            EmitAsm("LD_B_L");
        }

        EmitTileWriteForTargetFromRegs(target, safe, labelPrefix + "_attr");
        EmitSetVbkUnchecked(0);
        EmitAsm("JR", done);

        EmitLabel(skip);
        EmitDiscardStackWords(wordCount);
        EmitLabel(done);
    }

    // Copy byte-counted attribute data into bank 1, skipping the copy on DMG.
    // The fast form relies on the caller to provide a VRAM-safe transfer interval.
    void EmitSetTileAttrBulkIntrinsic(Expr destExpr, Expr srcExpr, Expr countExpr, bool fast, string labelPrefix)
    {
        Expr foldedCountExpr = FoldConstants(countExpr);

        if (IsKnownCgbRuntimeTrue())
        {
            EmitPushExprAsWideWord(destExpr);
            EmitPushExprAsWideWord(srcExpr);
            EmitPushExprAsWord(foldedCountExpr);

            EmitSetVbkUnchecked(1);

            EmitAsm("POP_HL");
            EmitAsm("LD_B_L");
            EmitAsm("POP_DE");
            EmitAsm("POP_HL");

            // Small constant counts can inline the transfer; other counts use the shared copy helper.
            if (foldedCountExpr.Match(Tag.Integer, out int attrBulkCountConst) &&
                TryEmitVramMemcpyConstCount(!fast, attrBulkCountConst & 0xFF, labelPrefix + "_small"))
            {
            }
            else
            {
                EmitCallSetTileBulkCore(fast);
            }
            EmitSetVbkUnchecked(0);
            return;
        }

        AsmOperand skip = MakeUniqueLabel(labelPrefix + "_skip");
        AsmOperand done = MakeUniqueLabel(labelPrefix + "_done");

        EmitPushExprAsWideWord(destExpr);
        EmitPushExprAsWideWord(srcExpr);
        EmitPushExprAsWord(foldedCountExpr);

        EmitRuntimeCgbCheckCall();
        EmitAsm("OR_A");
        EmitAsm("JR_Z", skip);

        EmitSetVbkUnchecked(1);

        EmitAsm("POP_HL");
        EmitAsm("LD_B_L");
        EmitAsm("POP_DE");
        EmitAsm("POP_HL");

        // The runtime-guarded path uses the same transfer selection after restoring arguments.
        if (foldedCountExpr.Match(Tag.Integer, out int attrBulkCountConst2) &&
            TryEmitVramMemcpyConstCount(!fast, attrBulkCountConst2 & 0xFF, labelPrefix + "_small"))
        {
        }
        else
        {
            EmitCallSetTileBulkCore(fast);
        }
        EmitSetVbkUnchecked(0);
        EmitAsm("JP", done);

        EmitLabel(skip);
        EmitDiscardStackWords(3);
        EmitLabel(done);
    }

    // Copy tile and attribute streams to the same address while retaining the byte count.
    // The first copy uses the current bank (normally 0); the attribute pass is conditional on CGB.
    void EmitSetTileCgbBulkIntrinsic(Expr destExpr, Expr tileSrcExpr, Expr attrSrcExpr, Expr countExpr, bool fast, string labelPrefix)
    {
        Expr foldedCountExpr = FoldConstants(countExpr);

        if (IsKnownCgbRuntimeTrue())
        {
            EmitPushExprAsWideWord(destExpr);
            EmitPushExprAsWideWord(tileSrcExpr);
            EmitPushExprAsWideWord(attrSrcExpr);
            EmitPushExprAsWord(foldedCountExpr);

            EmitAsm("POP_HL");
            EmitAsm("LD_A_L");

            EmitAsm("POP_DE");
            EmitAsm("POP_BC");
            EmitAsm("POP_HL");

            EmitAsm("PUSH_HL");
            EmitAsm("PUSH_DE");
            EmitAsm("LD_D_B");
            EmitAsm("LD_E_C");
            EmitAsm("LD_C_A");
            EmitAsm("XOR_A");
            EmitAsm("LD_B_A");
            EmitAsm("PUSH_BC");
            EmitAsm("LD_B_C");

            // Save destination, attribute source and count before the tile transfer consumes its registers.
            if (foldedCountExpr.Match(Tag.Integer, out int cgbBulkCountConst) &&
                TryEmitVramMemcpyConstCount(!fast, cgbBulkCountConst & 0xFF, labelPrefix + "_tile_small"))
            {
            }
            else
            {
                EmitCallSetTileBulkCore(fast);
            }

            EmitSetVbkUnchecked(1);

            EmitAsm("POP_BC");
            EmitAsm("POP_DE");
            EmitAsm("POP_HL");
            EmitAsm("LD_B_C");

            // Reload the saved count for the attribute copy, then return to VRAM bank 0.
            if (foldedCountExpr.Match(Tag.Integer, out int cgbBulkCountConst2) &&
                TryEmitVramMemcpyConstCount(!fast, cgbBulkCountConst2 & 0xFF, labelPrefix + "_attr_small"))
            {
            }
            else
            {
                EmitCallSetTileBulkCore(fast);
            }
            EmitSetVbkUnchecked(0);
            return;
        }

        AsmOperand skip = MakeUniqueLabel(labelPrefix + "_skip");
        AsmOperand done = MakeUniqueLabel(labelPrefix + "_done");

        EmitPushExprAsWideWord(destExpr);
        EmitPushExprAsWideWord(tileSrcExpr);
        EmitPushExprAsWideWord(attrSrcExpr);
        EmitPushExprAsWord(foldedCountExpr);

        EmitAsm("POP_HL");
        EmitAsm("LD_A_L");

        EmitAsm("POP_DE");
        EmitAsm("POP_BC");
        EmitAsm("POP_HL");

        EmitAsm("PUSH_HL");
        EmitAsm("PUSH_DE");
        EmitAsm("LD_D_B");
        EmitAsm("LD_E_C");
        EmitAsm("LD_C_A");
        EmitAsm("XOR_A");
        EmitAsm("LD_B_A");
        EmitAsm("PUSH_BC");
        EmitAsm("LD_B_C");

        // Tile data is copied even when runtime detection subsequently skips CGB attributes.
        if (foldedCountExpr.Match(Tag.Integer, out int cgbBulkCountConst3) &&
            TryEmitVramMemcpyConstCount(!fast, cgbBulkCountConst3 & 0xFF, labelPrefix + "_tile_small"))
        {
        }
        else
        {
            EmitCallSetTileBulkCore(fast);
        }

        EmitRuntimeCgbCheckCall();
        EmitAsm("OR_A");
        EmitAsm("JR_Z", skip);

        EmitSetVbkUnchecked(1);

        EmitAsm("POP_BC");
        EmitAsm("POP_DE");
        EmitAsm("POP_HL");
        EmitAsm("LD_B_C");

        // After a successful runtime check, restore the attribute source and original destination.
        if (foldedCountExpr.Match(Tag.Integer, out int cgbBulkCountConst4) &&
            TryEmitVramMemcpyConstCount(!fast, cgbBulkCountConst4 & 0xFF, labelPrefix + "_attr_small"))
        {
        }
        else
        {
            EmitCallSetTileBulkCore(fast);
        }
        EmitSetVbkUnchecked(0);
        EmitAsm("JP", done);

        EmitLabel(skip);
        EmitDiscardStackWords(3);
        EmitLabel(done);
    }

    // Inject the requested self-loop trap after RST metadata at the start of bank-zero code.
    void AppendCheckTrapIfUsed(List<Expr> rawLines)
    {
        if (CheckTrapLabel == null) return;

        var injected = new List<Expr>();
        injected.Add(Expr.Make(Tag.Comment, "[KITAQGB] injected bank0 check trap"));
        // Must be a global symbol. Tag.Label would be cleared at the next Tag.Function.
        injected.Add(Expr.Make(Tag.Function, CheckTrapLabel.Base.Value));
        injected.Add(Expr.MakeAsm("JP", CheckTrapLabel));

        int insertAt = 0;
        while (insertAt < rawLines.Count && rawLines[insertAt].MatchTag(Tag.RstMap)) insertAt++;
        rawLines.InsertRange(insertAt, injected);
    }

    // Inject a bank-zero panic helper that disables interrupts and remains in a HALT loop.
    void AppendRuntimeAssertHelperIfUsed(List<Expr> rawLines)
    {
        if (!UseRuntimeAssertPanicHelper) return;

        var injected = new List<Expr>();
        injected.Add(Expr.Make(Tag.Comment, "[KITAQGB] injected runtime assert panic helper"));
        injected.Add(Expr.Make(Tag.Function, "__kq_panic"));

        AsmOperand spin = new AsmOperand("__kq_panic_spin", AddressMode.Absolute);
        injected.Add(Expr.MakeAsm("DI"));
        injected.Add(Expr.Make(Tag.Label, spin.Base.Value));
        injected.Add(Expr.MakeAsm("HALT"));
        injected.Add(Expr.MakeAsm("JP", spin));

        int insertAt = 0;
        while (insertAt < rawLines.Count && rawLines[insertAt].MatchTag(Tag.RstMap)) insertAt++;
        rawLines.InsertRange(insertAt, injected);
    }



    // ====== RST call compression (assembler stub + optimizer rewrite) ======
    // Describe one selected restart-vector mapping and its estimated static byte savings.
    struct RstHotEntry
    {
        public int Vector;
        public string TargetLabel;
        public int Calls;
        public int NetBytes;

        // Capture the selected vector, target and static call-site count for later emission/reporting.
        public RstHotEntry(int vector, string targetLabel, int calls, int netBytes)
        {
            Vector = vector;
            TargetLabel = targetLabel;
            Calls = calls;
            NetBytes = netBytes;
        }
    }

    // Track definitions, direct timing-sensitive operations and named callees for RST filtering.
    sealed class RstTargetSafetyInfo
    {
        public bool Defined;
        public bool DirectSensitive;
        public bool Sensitive;
        public string SensitiveReason;
        public readonly HashSet<string> Calls = new HashSet<string>(StringComparer.Ordinal);
    }

    // Recognize conventional case-sensitive hardware names; this is a naming heuristic, not symbol resolution.
    static bool LooksLikeHardwareRegisterName(string sym)
    {
        if (string.IsNullOrEmpty(sym)) return false;
        string s = sym.Trim();
        if (s.Length == 0) return false;

        string[] exact =
        {
            "P1", "SB", "SC", "DIV", "TIMA", "TMA", "TAC", "IF", "IE",
            "LCDC", "STAT", "SCY", "SCX", "LY", "LYC", "DMA", "BGP", "OBP0", "OBP1",
            "WY", "WX", "VBK", "SVBK", "KEY1", "BCPS", "BCPD", "OCPS", "OCPD",
            "HDMA1", "HDMA2", "HDMA3", "HDMA4", "HDMA5"
        };
        for (int i = 0; i < exact.Length; i++)
            if (s == exact[i]) return true;

        // Audio register family
        if (s.StartsWith("NR", StringComparison.Ordinal)) return true;
        if (s.StartsWith("WAVE", StringComparison.Ordinal)) return true;
        return false;
    }

    // Classify selected interrupt, IO and mapper-access patterns conservatively for RST selection.
    // This scan does not prove the absence of timing-sensitive indirect memory access.
    static bool IsDirectTimingSensitiveInstruction(string mnemonic, AsmOperand operand)
    {
        if (mnemonic == null) return false;

        if (mnemonic == "DI" || mnemonic == "EI" || mnemonic == "HALT" || mnemonic == "STOP" || mnemonic == "RETI")
            return true;
        if (mnemonic.StartsWith("RST_", StringComparison.Ordinal))
            return true;

        // LDH implies access to FF00+offset. Treat symbolic LDH conservatively as timing-sensitive.
        if ((mnemonic == "LDH_A_MEM" || mnemonic == "LDH_MEM_A") && operand != null)
        {
            if ((operand.Mode == AddressMode.Immediate || operand.Mode == AddressMode.HighMem) && !operand.Base.HasValue)
            {
                int zp = operand.Offset & 0xFF;
                if (zp < 0x80) return true; // IO registers
            }
            else
            {
                return true;
            }
        }

        if ((mnemonic == "LD_A_MEM" || mnemonic == "LD_MEM_A") && operand != null && operand.Mode == AddressMode.Absolute)
        {
            if (!operand.Base.HasValue)
            {
                int addr = operand.Offset & 0xFFFF;
                if ((addr >= 0xFF00 && addr < 0xFF80) || addr == 0x2000 || addr == 0x0000 || addr == 0x4000 || addr == 0x6000)
                    return true;
            }
            else if (LooksLikeHardwareRegisterName(operand.Base.Value))
            {
                return true;
            }
        }

        if ((mnemonic == "LD_HL_IMM" || mnemonic == "LD_DE_IMM" || mnemonic == "LD_BC_IMM")
            && operand != null && operand.Mode == AddressMode.Immediate16 && !operand.Base.HasValue)
        {
            int addr = operand.Offset & 0xFFFF;
            if ((addr >= 0xFF00 && addr < 0xFF80) || addr == 0x2000 || addr == 0x0000 || addr == 0x4000 || addr == 0x6000)
                return true;
        }

        return false;
    }

    // Scan function bodies and propagate sensitivity through recognized direct CALL edges.
    static Dictionary<string, RstTargetSafetyInfo> BuildRstTargetSafetyMap(List<Expr> rawLines)
    {
        var map = new Dictionary<string, RstTargetSafetyInfo>(StringComparer.Ordinal);

        // Create records for referenced targets as well as definitions; Defined distinguishes the two.
        RstTargetSafetyInfo EnsureInfo(string name)
        {
            if (!map.TryGetValue(name, out var inf))
            {
                inf = new RstTargetSafetyInfo();
                map[name] = inf;
            }
            return inf;
        }

        string currentFunction = null;
        for (int i = 0; i < rawLines.Count; i++)
        {
            Expr e = rawLines[i];

            if (e.Match(Tag.Function, out string fn))
            {
                currentFunction = fn;
                EnsureInfo(fn).Defined = true;
                continue;
            }

            if (currentFunction == null) continue;

            if (e.Match(Tag.Asm, out string m, out AsmOperand o))
            {
                var info = EnsureInfo(currentFunction);

                if (IsDirectTimingSensitiveInstruction(m, o))
                {
                    info.DirectSensitive = true;
                    if (string.IsNullOrEmpty(info.SensitiveReason))
                        info.SensitiveReason = "direct IO/timing-sensitive instruction";
                }

                if (m == "CALL" && o != null && o.Mode == AddressMode.Absolute && o.Offset == 0 && o.Base.HasValue)
                {
                    info.Calls.Add(o.Base.Value);
                    EnsureInfo(o.Base.Value);
                }
            }
        }

        // Exclude entry/runtime labels and, in safe mode, ordinary user-function names.
        bool NameForcesSensitive(string name)
        {
            if (string.IsNullOrEmpty(name)) return true;
            if (name == "main") return true;
            if (name.StartsWith("__kq_", StringComparison.Ordinal)) return true;
            if (!Program.RstUnsafe && !name.StartsWith("__", StringComparison.Ordinal))
                return true;
            return false;
        }

        foreach (var kv in map)
        {
            if (NameForcesSensitive(kv.Key))
            {
                kv.Value.Sensitive = true;
                if (string.IsNullOrEmpty(kv.Value.SensitiveReason))
                    kv.Value.SensitiveReason = "reserved/runtime helper label";
            }
            else if (kv.Value.DirectSensitive)
            {
                kv.Value.Sensitive = true;
                if (string.IsNullOrEmpty(kv.Value.SensitiveReason))
                    kv.Value.SensitiveReason = "direct IO/timing-sensitive instruction";
            }
        }

        bool changed;
        do
        {
            changed = false;
            foreach (var kv in map)
            {
                var info = kv.Value;
                if (info.Sensitive) continue;

                foreach (string callee in info.Calls)
                {
                    // Propagate a conservative result when a referenced target has no safety record.
                    if (!map.TryGetValue(callee, out var cInfo))
                    {
                        info.Sensitive = true;
                        info.SensitiveReason = "calls unresolved target";
                        changed = true;
                        break;
                    }

                    if (NameForcesSensitive(callee))
                    {
                        info.Sensitive = true;
                        info.SensitiveReason = "calls reserved/runtime helper target";
                        changed = true;
                        break;
                    }

                    if (cInfo.Sensitive || cInfo.DirectSensitive)
                    {
                        info.Sensitive = true;
                        info.SensitiveReason = "calls timing-sensitive target";
                        changed = true;
                        break;
                    }
                }
            }
        } while (changed);

        return map;
    }

    // Rank eligible named CALL targets by estimated ROM-byte savings and assign restart vectors.
    // Counts are static call sites, not measured runtime invocation frequencies.
    static IEnumerable<RstHotEntry> SelectRstHotEntriesFromAssembly(List<Expr> rawLines)
    {
        // Available vectors. By default we keep 0x38 free (often used as a trap/debug RST).
        // Enable with: --rst-use-38
        int[] vectors = Program.RstUse38
            ? new int[] { 0x00, 0x08, 0x10, 0x18, 0x20, 0x28, 0x30, 0x38 }
            : new int[] { 0x00, 0x08, 0x10, 0x18, 0x20, 0x28, 0x30 };

        // Tuning knobs (prefer size by default; enable speed-safe/excludes via CLI)
        int maxCalls = Program.RstMaxCalls;
        int maxVectors = Program.RstMaxVectors > 0 ? System.Math.Min(Program.RstMaxVectors, vectors.Length) : vectors.Length;
        var exclude = Program.RstExcludeLabels;

        // Heuristics:
        // - CALL (3 bytes) -> RST_xx (1 byte) saves 2 bytes per call
        // - Each vector installs a JP stub (3 bytes)
        // - Net bytes saved = 2*calls - 3
        const int MinCalls = 2;
        const int StubCostBytes = 3;
        const int CallSaveBytes = 2;
        var safety = BuildRstTargetSafetyMap(rawLines);

        Dictionary<string, int> counts = new Dictionary<string, int>();
        for (int i = 0; i < rawLines.Count; i++)
        {
            string m; AsmOperand o;
            if (!rawLines[i].Match(Tag.Asm, out m, out o)) continue;
            if (m != "CALL") continue;
            if (o.Mode != AddressMode.Absolute) continue;
            if (!o.Base.HasValue) continue;
            if (o.Offset != 0) continue;
            string name = o.Base.Value;
            if (name == "main") continue;
            if (!counts.ContainsKey(name)) counts[name] = 0;
            counts[name]++;
        }

        // Apply size, maximum-call-count and explicit-exclusion limits before the safety filter.
        var preFiltered = counts
            .Select(kv => new { Name = kv.Key, Calls = kv.Value, Net = (CallSaveBytes * kv.Value) - StubCostBytes })
            .Where(x => x.Calls >= MinCalls && x.Net > 0)
            .Where(x => x.Calls <= maxCalls)
            .Where(x => !exclude.Contains(x.Name))
            .ToList();

        // Unsafe mode bypasses definition/sensitivity checks; safe mode requires a defined allowed target.
        bool IsSafetyAllowed(string name)
        {
            if (Program.RstUnsafe) return true;
            if (!safety.TryGetValue(name, out var sInfo)) return false;
            if (!sInfo.Defined) return false;
            return !sInfo.Sensitive && !sInfo.DirectSensitive;
        }

        // Prefer the largest estimated savings, then use call counts and names to break ties.
        var ordered = preFiltered
            .Where(x => IsSafetyAllowed(x.Name))
            .OrderByDescending(x => x.Net)
            .ThenByDescending(x => x.Calls)
            .ThenBy(x => x.Name)
            .ToList();

        // --- Build-time log: show candidate list (top N) ---
        // This helps tuning RST thresholds and verifying selection.
        const int LogTopN = 24;
        try
        {
            int top = Math.Min(LogTopN, ordered.Count);
            if (top > 0)
            {
                var sb = new System.Text.StringBuilder();
                sb.AppendLine($"[KITAQGB] RST hot-call candidates (top {top} / {ordered.Count})  MinCalls={MinCalls}  Net=2*calls-3");
                int shownMaxCalls = (maxCalls == int.MaxValue) ? -1 : maxCalls;
                string maxCallsStr = (shownMaxCalls < 0) ? "inf" : shownMaxCalls.ToString();
                sb.AppendLine($"[KITAQGB] RST filters: maxCalls={maxCallsStr}  maxVectorsCap={maxVectors}  excludeCount={exclude.Count}");
                sb.AppendLine($"[KITAQGB] RST safety-mode: {(Program.RstUnsafe ? "unsafe/aggressive" : "safe/conservative")}  candidatesBeforeSafety={preFiltered.Count}  droppedBySafety={(preFiltered.Count - ordered.Count)}");
                if (exclude.Count > 0)
                {
                    // Keep this short in stderr; full list goes to debug file.
                    var shortList = exclude.Take(12).ToArray();
                    sb.AppendLine($"[KITAQGB] RST exclude: {string.Join(",", shortList)}{(exclude.Count > 12 ? ",..." : "")}");
                }
                if (!Program.RstUnsafe && preFiltered.Count > ordered.Count)
                {
                    var dropped = preFiltered.Where(x => !ordered.Any(y => y.Name == x.Name))
                                             .Take(12)
                                             .Select(x =>
                                             {
                                                 string reason = "safety-filtered";
                                                 if (safety.TryGetValue(x.Name, out var inf) && !string.IsNullOrEmpty(inf.SensitiveReason))
                                                     reason = inf.SensitiveReason;
                                                 return $"{x.Name} ({reason})";
                                             })
                                             .ToArray();
                    if (dropped.Length > 0)
                        sb.AppendLine($"[KITAQGB] RST safety dropped: {string.Join(", ", dropped)}{(preFiltered.Count - ordered.Count > dropped.Length ? ", ..." : "")}");
                }
                sb.AppendLine(" rank | calls | netB | label");
                for (int i = 0; i < top; i++)
                {
                    var x = ordered[i];
                    sb.AppendLine(string.Format(" {0,4} | {1,5} | {2,4} | {3}", i + 1, x.Calls, x.Net, x.Name));
                }
                int sel = Math.Min(maxVectors, ordered.Count);
                if (sel > 0)
                {
                    int totalNetBytes = 0;
                    int totalStaticCalls = 0;
                    for (int i = 0; i < sel; i++) { totalNetBytes += ordered[i].Net; totalStaticCalls += ordered[i].Calls; }
                    sb.AppendLine();
                    sb.AppendLine($"[KITAQGB] RST summary: vectorsUsed={sel}/{maxVectors}  use38={(Program.RstUse38 ? 1 : 0)}  totalNetBytes={totalNetBytes}  totalStaticCallSites={totalStaticCalls}");

                    sb.AppendLine();
                    sb.AppendLine("[KITAQGB] RST selection (vector -> label):");
                    for (int i = 0; i < sel; i++)
                    {
                        sb.AppendLine(string.Format("  RST_{0:X2} -> {1}   (calls={2}, net={3})", vectors[i], ordered[i].Name, ordered[i].Calls, ordered[i].Net));
                    }
                }

                // Always write to debug_output when enabled.
                Program.WriteDebugFile("rst_candidates.txt", sb.ToString());

#if DEBUG
                // Also emit to build log in Debug builds.
                Console.Error.WriteLine(sb.ToString());
#endif
            }
        }
        catch
        {
            // Never fail the build due to logging.
        }

        int n = Math.Min(maxVectors, ordered.Count);
        for (int i = 0; i < n; i++)
        {
            yield return new RstHotEntry(vectors[i], ordered[i].Name, ordered[i].Calls, ordered[i].Net);
        }
    }

    // ====== Helpers: Addressing ======
    // Select the high-memory form for addresses at or above FF00; callers supply 16-bit addresses.
    bool IsHighMemAddr(int addr) => (addr >= 0xFF00);

    // Encode high memory by its low-byte offset, retaining absolute addressing elsewhere.
    AsmOperand MemOp(int addr, string comment = null)
    {
        AsmOperand op = IsHighMemAddr(addr)
            ? new AsmOperand(addr & 0xFF, AddressMode.HighMem)
            : new AsmOperand(addr, AddressMode.Absolute);

        if (comment != null) op = op.WithComment(comment);
        return op;
    }

    // Represent a little-endian word as two independently classified adjacent byte operands.
    WideOperand WideMemOp(int addr, string comment = null)
    {
        return new WideOperand(
            MemOp(addr, comment),
            MemOp(addr + 1, comment != null ? comment + "+1" : null)
        );
    }

    // Choose LDH for high-memory reads and the absolute byte-load form otherwise.
    void EmitLoadA(AsmOperand src)
    {
        if (src.Mode == AddressMode.HighMem) EmitAsm("LDH_A_MEM", src);
        else EmitAsm("LD_A_MEM", src);
    }

    // Choose LDH for high-memory writes and the absolute byte-store form otherwise.
    void EmitStoreA(AsmOperand dst)
    {
        if (dst.Mode == AddressMode.HighMem) EmitAsm("LDH_MEM_A", dst);
        else EmitAsm("LD_MEM_A", dst);
    }

    // ====== Stack ABI helpers (stack parameters, SP base) ======
    // Reload the saved entry stack pointer; this path uses A while assembling HL.
    void EmitLoadStackBaseIntoHL()
    {
        if (CurrentStackBaseAddr >= 0)
        {
            EmitLoadA(MemOp(CurrentStackBaseAddr, "spbase"));
            EmitAsm("LD_L_A");
            EmitLoadA(MemOp(CurrentStackBaseAddr + 1, "spbase+1"));
            EmitAsm("LD_H_A");
        }
        else
        {
            // Fallback: use current SP (safe only if SP is unchanged).
            EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
        }
    }

    // Add a word displacement through DE, replacing DE and updating the addition flags.
    void EmitAddImm16ToHL(int imm16)
    {
        EmitAsm("LD_DE_IMM", new AsmOperand(imm16, AddressMode.Immediate16));
        EmitAsm("ADD_HL_DE");
    }

    // Address a parameter relative to the saved entry stack pointer, past the two-byte return address.
    void EmitStackParamAddrIntoHL(int offsetFromArg0, string comment = null)
    {
        // arg0 starts at (SP_entry + 2)
        EmitLoadStackBaseIntoHL();
        EmitAddImm16ToHL(2 + offsetFromArg0);
        if (comment != null) EmitComment(comment);
    }

    // Resolve the parameter slot and read its byte into A.
    void EmitLoadStackParamU8IntoA(Symbol sym)
    {
        EmitStackParamAddrIntoHL(sym.Value, $"stack param {sym.Name}");
        EmitAsm("LD_A_HL");
    }

    // Read consecutive low/high bytes through DE, then return the parameter word in HL.
    void EmitLoadStackParamU16IntoHL(Symbol sym)
    {
        EmitStackParamAddrIntoHL(sym.Value, $"stack param {sym.Name}");
        EmitAsm("LD_A_HL");
        EmitAsm("LD_E_A");
        EmitAsm("INC_HL");
        EmitAsm("LD_A_HL");
        EmitAsm("LD_D_A");
        EmitAsm("LD_H_D");
        EmitAsm("LD_L_E");
    }

    // Emit a byte store after resolving the stack parameter address.
    void EmitStoreAIntoStackParam(Symbol sym)
    {
        EmitStackParamAddrIntoHL(sym.Value, $"stack param {sym.Name}");
        EmitAsm("LD_HL_A");
    }

    // Keep the source word on the machine stack while computing its destination slot.
    void EmitStoreHLIntoStackParam(Symbol sym)
    {
        // Save value
        EmitAsm("PUSH_HL");
        EmitStackParamAddrIntoHL(sym.Value, $"stack param {sym.Name}");
        EmitAsm("POP_DE");
        EmitAsm("LD_A_E");
        EmitAsm("LD_HL_A");
        EmitAsm("INC_HL");
        EmitAsm("LD_A_D");
        EmitAsm("LD_HL_A");
    }


    // ====== Helpers: scaled index (base + index * elemSize) ======
    // Recognize positive powers of two before choosing shift-only index scaling.
    static bool IsPow2(int n) => n > 0 && (n & (n - 1)) == 0;
    // Count left shifts for the positive power-of-two input selected by IsPow2.
    static int Log2Pow2(int n)
    {
        int s = 0;
        while ((1 << s) < n) s++;
        return s;
    }

    // extern type compatibility (minimal C-like rules)
    // - exact match is OK
    // - array with unspecified dimension (e.g. extern u8 a[]) is compatible with any concrete size definition
    static bool ExternTypeCompatible(CType a, CType b)
    {
        if (a == null || b == null) return a == b;
        if (a == b) return true;

        if (a.IsArray && b.IsArray)
        {
            if (a.Subtype != b.Subtype) return false;

            // allow one side to omit dimension
            if (a.Tag == CTypeTag.ArrayWithDimensionExpression && a.DimensionExpression != null && a.DimensionExpression.Match(Tag.Empty)) return true;
            if (b.Tag == CTypeTag.ArrayWithDimensionExpression && b.DimensionExpression != null && b.DimensionExpression.Match(Tag.Empty)) return true;

            // concrete arrays
            if (a.Tag == CTypeTag.Array && b.Tag == CTypeTag.Array) return a.Dimension == b.Dimension;

            // decl with constant dimension expression vs concrete definition
            if (a.Tag == CTypeTag.ArrayWithDimensionExpression && b.Tag == CTypeTag.Array)
            {
                int dim;
                if (TryEvalConstArrayDim(a.DimensionExpression, out dim)) return dim == b.Dimension;
            }
            if (b.Tag == CTypeTag.ArrayWithDimensionExpression && a.Tag == CTypeTag.Array)
            {
                int dim;
                if (TryEvalConstArrayDim(b.DimensionExpression, out dim)) return dim == a.Dimension;
            }

            if (a.Tag == CTypeTag.ArrayWithDimensionExpression && b.Tag == CTypeTag.ArrayWithDimensionExpression)
            {
                return a.DimensionExpression.Show() == b.DimensionExpression.Show();
            }
        }

        return false;
    }

    // Accept only an already-folded nonnegative integer dimension in this compatibility check.
    static bool TryEvalConstArrayDim(Expr dimExpr, out int dim)
    {
        dim = 0;
        if (dimExpr == null) return false;
        if (dimExpr.Match(Tag.Integer, out dim) && dim >= 0) return true;
        return false;
    }

    // Use the complete element size for array/pointer indexing; other expression types default to one byte.
    int GetIndexElementSize(Expr baseExpr)
    {
        CType t = TypeOf(baseExpr);
        if (t.IsArray || t.IsPointer)
        {
            return SizeOf(baseExpr, t.Subtype);
        }
        return 1;
    }

	// B3) __prg_rom u8 table[i] read optimization
	// - constant index: ld a,(table+off)
	// - aligned-to-256 table + u8 index: HL = (HIGH(table)<<8) | idx ; ld a,(hl)
	// Optimize named readonly byte-array reads, preserving the optional bounds-check emission.
	bool TryCompilePrgRomU8IndexIntoA(Expr origin, Expr left, Expr right)
	{
	    // Only for 1-byte element arrays
	    int elemSize = GetIndexElementSize(left);
	    if (elemSize != 1) return false;

	    // Left must be a named readonly data symbol
	    if (!left.Match(Tag.Name, out string baseName)) return false;
	    if (!TryFindSymbol(baseName, out Symbol sym)) return false;
	    if (sym.Tag != SymbolTag.ReadonlyData) return false;

	    CType t = sym.Type;
	    if (t == null || !t.IsArray) return false;
	    if (t.Subtype == null) return false;
	    if (SizeOf(origin, t.Subtype) != 1) return false; // u8

	    // Preserve optional bounds checks (debug aid).
	    EmitBoundsCheckIfNeeded(left, right);

	    // Constant index: direct absolute load
	    if (right.Match(Tag.Integer, out int idxConst) && idxConst >= 0)
	    {
	        int off = (idxConst * elemSize) & 0xFFFF;
		        EmitAsm("LD_A_MEM", new AsmOperand(Maybe.Just(sym.Name), off, AddressMode.Absolute, ImmediateModifier.None));
	        return true;
	    }

	    // u8 index + table aligned to 256: ultra-fast addressing
	    if (SizeOf(right) == 1)
	    {
	        int plannedAlign = 0;
	        ReadonlyDataPlannedAlign.TryGetValue(sym.Name, out plannedAlign);
	        plannedAlign = Math.Max(plannedAlign, (t.ForcedAlign > 0) ? t.ForcedAlign : 0);
	        if (plannedAlign >= 256)
	        {
	            CompileIntoA(right);
	            EmitAsm("LD_L_A");
	            var hi = new AsmOperand(sym.Name, ImmediateModifier.HighByte);
	            EmitAsm("LD_H_IMM", hi);
	            EmitAsm("LD_A_HL");
	            return true;
	        }
	    }

	    return false;
	}

    // DE = index * elemSize (16bit)
    void CompileScaledIndexToDE(Expr indexExpr, int elemSize)
    {
        indexExpr = FoldConstants(indexExpr);

        if (elemSize <= 0) elemSize = 1;
        if (elemSize == 1)
        {
            // Unit-stride byte indices are zero-extended; wider indices retain their word value.
            if (SizeOf(indexExpr) == 1)
            {
                if (indexExpr.Match(Tag.Integer, out int k))
                {
                    EmitAsm("LD_E_IMM", new AsmOperand(k & 0xFF, AddressMode.Immediate));
                    EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
                }
                else
                {
                    CompileIntoA(indexExpr);
                    EmitAsm("LD_E_A");
                    EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
                }
            }
            else
            {
                CompileIntoHL(indexExpr);
                EmitAsm("LD_D_H");
                EmitAsm("LD_E_L");
            }
            return;
        }

        if (indexExpr.Match(Tag.Integer, out int idxConst))
        {
            int off = (idxConst * elemSize) & 0xFFFF;
            EmitAsm("LD_DE_IMM", new AsmOperand(off, AddressMode.Immediate16));
            return;
        }

        // HL = index (u16)
        if (SizeOf(indexExpr) == 1)
        {
            CompileIntoA(indexExpr);
            EmitAsm("LD_L_A");
            EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
        }
        else
        {
            CompileIntoHL(indexExpr);
        }

        // HL = HL * elemSize
        if (IsPow2(elemSize))
        {
            int sh = Log2Pow2(elemSize);
            for (int i = 0; i < sh; i++) EmitAsm("ADD_HL_HL");
        }
        else
        {
            bool ok = EmitConstantMultiplication(elemSize);
            if (!ok)
            {
                if (elemSize > 255) Program.Error("Element size too large for scaled index multiply loop.");
                EmitAsm("LD_D_H"); EmitAsm("LD_E_L"); // DE = index
                EmitAsm("LD_HL_IMM", new AsmOperand(0, AddressMode.Immediate16)); // HL = 0
                EmitAsm("LD_A_IMM", new AsmOperand(elemSize & 0xFF, AddressMode.Immediate));
                AsmOperand mloop = MakeUniqueLabel("idxmul");
                EmitLabel(mloop);
                EmitAsm("ADD_HL_DE");
                EmitAsm("DEC_A");
                EmitAsm("JP_NZ", mloop);
            }
        }

        // DE = HL
        EmitAsm("LD_D_H");
        EmitAsm("LD_E_L");
    }

    // Compute lvalue address into HL for Name / *ptr / arr[idx] / field (including nested fields).
    bool TryCompileLValueAddressIntoHL(Expr lvalue, bool forStore)
    {
        if (!forStore)
        {
            CType _sretType;
            if (TryCompileStructReturnCallAddressIntoHL(lvalue, out _sretType))
                return true;
        }

        if (lvalue.Match(Tag.Name, out string name))
        {
            Symbol sym = FindSymbol(lvalue, name);
            if (forStore && sym.Tag == SymbolTag.ReadonlyData)
            {
                Error(lvalue, "Cannot assign to readonly data: " + name);
                return false;
            }

            if (sym.Tag == SymbolTag.StackParam)
            {
                EmitStackParamAddrIntoHL(sym.Value, "&stack param " + sym.Name);
                return true;
            }

            if (sym.Tag == SymbolTag.ReadonlyData)
                EmitAsm("LD_HL_IMM", new AsmOperand(sym.Name, AddressMode.Immediate16));
            else
                EmitAsm("LD_HL_IMM", new AsmOperand(sym.Value, AddressMode.Immediate16));
            return true;
        }

        if (lvalue.Match(Tag.Load, out Expr ptrExpr))
        {
            CompileIntoHL(ptrExpr);
            return true;
        }

        if (lvalue.Match(Tag.Index, out Expr arrExpr, out Expr idxExpr))
        {
            Expr baseExpr = arrExpr;
            Expr sliceLen = null;
            if (arrExpr.Match(Tag.Slice, out Expr slicePtr, out Expr lenExpr))
            {
                baseExpr = slicePtr;
                sliceLen = lenExpr;
            }

            if (forStore && baseExpr.Match(Tag.Name, out string arrName) && TryFindSymbol(arrName, out Symbol arrSym) && arrSym.Tag == SymbolTag.ReadonlyData)
            {
                Error(lvalue, "Cannot assign to readonly data: " + arrName);
                return false;
            }

            if (sliceLen != null)
            {
                if (Program.CheckSliceBounds && !InUnsafe)
                    EmitSliceBoundsCheckIfNeeded(sliceLen, idxExpr);
                else
                    CompileDiscard(sliceLen);
            }
            else
            {
                EmitBoundsCheckIfNeeded(baseExpr, idxExpr);
            }

            CompileIntoHL(baseExpr);
            EmitAsm("PUSH_HL");
            int elemSize = GetIndexElementSize(baseExpr);
            CompileScaledIndexToDE(idxExpr, elemSize);
            EmitAsm("POP_HL");
            EmitAsm("ADD_HL_DE");
            return true;
        }

        if (lvalue.Match(Tag.Field, out Expr baseField, out string fieldName))
        {
            if (!TryCompileLValueAddressIntoHL(baseField, forStore))
                return false;

            FieldInfo f = GetFieldInfo(baseField, fieldName);
            if (f == null)
                return false;

            if (f.Offset != 0)
            {
                EmitAsm("LD_DE_IMM", new AsmOperand(f.Offset, AddressMode.Immediate16));
                EmitAsm("ADD_HL_DE");
            }
            return true;
        }

        return false;
    }

    // Ignore top-level const and require matching aggregate kind/name with a completed layout.
    bool IsSameCompleteAggregateType(CType leftType, CType rightType)
    {
        if (leftType == null || rightType == null) return false;
        CType l = leftType.WithoutConst();
        CType r = rightType.WithoutConst();
        if (!l.IsStructOrUnion || !r.IsStructOrUnion) return false;
        if (l.Tag != r.Tag || l.Name != r.Name) return false;
        AggregateInfo ai;
        return AggregateTypes.TryGetValue(l.Name, out ai) && ai.TotalSize >= 0;
    }

    // Reject implicit copies touching ROM, VRAM, OAM or IO; wrapped ranges are rejected conservatively.
    bool IsSpecialImplicitCopyAddress(int address, int size, out string region)
    {
        region = null;
        int start = address & 0xFFFF;
        int end = (start + Math.Max(1, size) - 1) & 0xFFFF;

        Func<int, int, int, bool> overlaps = (lo, hi, point) => point >= lo && point <= hi;
        bool rangeWraps = end < start;
        Func<int, int, bool> rangeOverlaps = (lo, hi) =>
            rangeWraps || overlaps(lo, hi, start) || overlaps(lo, hi, end) || (start <= lo && end >= hi);

        if (rangeOverlaps(0x0000, 0x7FFF)) { region = "ROM"; return true; }
        if (rangeOverlaps(0x8000, 0x9FFF)) { region = "VRAM"; return true; }
        if (rangeOverlaps(0xFE00, 0xFE9F)) { region = "OAM"; return true; }
        if (rangeOverlaps(0xFF00, 0xFF7F)) { region = "IO"; return true; }
        return false;
    }

    // Recognize numeric addresses and addresses of named fixed storage after stripping casts.
    bool TryGetConstantAddress(Expr expr, out int address)
    {
        address = 0;
        Expr e = StripCasts(expr);
        if (e.Match(Tag.Integer, out address)) return true;
        if (e.Match(Tag.AddressOf, out Expr sub) && sub.Match(Tag.Name, out string name) && TryFindSymbol(name, out Symbol sym))
        {
            if (sym.Tag == SymbolTag.Global || sym.Tag == SymbolTag.Local || sym.Tag == SymbolTag.ReadonlyData)
            {
                address = sym.Value;
                return true;
            }
        }
        return false;
    }

    // Check recognizable storage against implicit aggregate-copy restrictions.
    // Unknown pointer values are allowed here; this is not a runtime memory-region check.
    bool IsImplicitAggregateCopyStorageAllowed(Expr lvalue, bool destination, int size, out string reason)
    {
        reason = null;
        Expr e = StripCasts(lvalue);

        if (!destination && TryGetStructReturnCallType(e, out CType _sretType, out CFunctionInfo sretInfo, out string sretName))
        {
            if (sretInfo != null)
            {
                Symbol retSym = EnsureStructReturnSlot(e, sretInfo, sretName, sretInfo.ReturnType);
                string retRegion = null;
                if (retSym == null || IsSpecialImplicitCopyAddress(retSym.Value, size, out retRegion))
                {
                    reason = "implicit struct/union copy from struct return storage in " + (retRegion ?? "unknown memory") + " is not supported";
                    return false;
                }
            }
            return true;
        }

        if (e.Match(Tag.Name, out string name))
        {
            if (!TryFindSymbol(name, out Symbol sym))
            {
                reason = "unknown aggregate object: " + name;
                return false;
            }

            if (sym.Tag == SymbolTag.ReadonlyData)
            {
                reason = "implicit struct/union copy " + (destination ? "to" : "from") + " readonly ROM data is not supported; use an explicit copy routine";
                return false;
            }
            if (sym.Tag == SymbolTag.Constant)
            {
                reason = "implicit struct/union copy " + (destination ? "to" : "from") + " a constant is not supported";
                return false;
            }
            if (sym.Tag == SymbolTag.Global && sym.WramBank > 1)
            {
                reason = "implicit struct/union copy " + (destination ? "to" : "from") + " WRAMX bank " + sym.WramBank + " is not supported; switch SVBK and copy explicitly";
                return false;
            }
            if (sym.Tag == SymbolTag.Global || sym.Tag == SymbolTag.Local)
            {
                string region;
                if (IsSpecialImplicitCopyAddress(sym.Value, size, out region))
                {
                    reason = "implicit struct/union copy " + (destination ? "to" : "from") + " " + region + " is not supported; use the explicit hardware/far copy API";
                    return false;
                }
            }
            return true;
        }

        // Apply the storage restriction recursively to the containing object for fields and indexed elements.
        if (e.Match(Tag.Field, out Expr baseExpr, out string _fieldName))
            return IsImplicitAggregateCopyStorageAllowed(baseExpr, destination, size, out reason);

        if (e.Match(Tag.Index, out Expr arrExpr, out Expr _idxExpr))
            return IsImplicitAggregateCopyStorageAllowed(arrExpr, destination, size, out reason);

        if (e.Match(Tag.Load, out Expr ptrExpr))
        {
            if (TryGetConstantAddress(ptrExpr, out int addr))
            {
                string region;
                if (IsSpecialImplicitCopyAddress(addr, size, out region))
                {
                    reason = "implicit struct/union copy " + (destination ? "to" : "from") + " " + region + " is not supported; use the explicit hardware/far copy API";
                    return false;
                }
            }
            return true;
        }

        reason = "struct/union copy requires an addressable " + (destination ? "destination" : "source");
        return false;
    }

    // Choose the legacy size-based strategy label recorded in aggregate-copy reports.
    string AggregateCopyStrategy(int size)
    {
        if (size <= 2) return "scalar";
        if (size <= 16) return "unrolled";
        if (size <= 255) return "__memcpy_small";
        return "__memcpy";
    }

    const int StructValueArgumentWarningThreshold = 16;

    // Record the chosen copy strategy with the current function, aggregate type and source location.
    void RecordAggregateCopy(Expr origin, CType type, int size, string strategy)
    {
        AggregateCopyReport.Add(new AggregateCopyInfo
        {
            Function = string.IsNullOrEmpty(CurrentFunctionName) ? "<global>" : CurrentFunctionName,
            Type = type == null ? "<unknown>" : type.WithoutConst().Show(),
            SizeBytes = size,
            Strategy = strategy,
            Source = origin == null ? "" : origin.Source.ToString()
        });
    }

    // Resolve the source before the destination, preserving its address on the stack, then emit a forward copy.
    void EmitAggregateCopy(Expr origin, Expr dst, Expr src, CType aggregateType, int size)
    {
        string strategy = AggregateCopyStrategy(size);
        RecordAggregateCopy(origin, aggregateType, size, strategy);
        EmitComment("kitaqgb.struct_copy type={0} size={1} strategy={2}",
            aggregateType == null ? "<unknown>" : aggregateType.WithoutConst().Show(),
            size,
            strategy);

        if (size <= 0) return;

        if (!TryCompileAggregateSourceAddressIntoHL(src, aggregateType))
        {
            Error(src, ErrorCode.ParseError, "struct/union copy source is not addressable");
            return;
        }
        EmitAsm("PUSH_HL");
        if (!TryCompileLValueAddressIntoHL(dst, true))
        {
            EmitAsm("POP_DE");
            Error(dst, ErrorCode.ParseError, "struct/union copy destination is not addressable");
            return;
        }
        EmitAsm("POP_DE");
        EmitCopyBytesFromDEToHL(size);
    }

    // Copy bytes forward from DE to HL using inline loads/stores; overlapping ranges are not handled as memmove.
    // The register end positions depend on the selected block/remainder path.
    void EmitCopyBytesFromDEToHL(int size)
    {
        if (size <= 0) return;

        if (size > 16 && size <= 255)
        {
            // Medium copies loop over unrolled eight- or sixteen-byte blocks, then emit the remaining bytes.
            int blockSize = size <= 64 ? 8 : 16;
            int blocks = size / blockSize;
            int rem = size % blockSize;
            if (blocks > 0)
            {
                EmitAsm("LD_B_IMM", new AsmOperand(blocks, AddressMode.Immediate));
                AsmOperand loop = MakeUniqueLabel("aggcpy_loop");
                EmitLabel(loop);
                for (int i = 0; i < blockSize; i++)
                {
                    EmitAsm("LD_A_DE");
                    EmitAsm("LDI_HL_A");
                    EmitAsm("INC_DE");
                }
                EmitAsm("DEC_B");
                EmitAsm("JP_NZ", loop);
            }
            for (int i = 0; i < rem; i++)
            {
                EmitAsm("LD_A_DE");
                EmitAsm("LDI_HL_A");
                if (i != rem - 1) EmitAsm("INC_DE");
            }
            return;
        }

        if (size > 255)
        {
            EmitAsm("LD_BC_IMM", new AsmOperand(size & 0xFFFF, AddressMode.Immediate16));
            // Large copies use a 16-bit countdown; the emitted count is the low word of the requested size.
            AsmOperand loop = MakeUniqueLabel("aggcpy16_loop");
            EmitLabel(loop);
            EmitAsm("LD_A_DE");
            EmitAsm("LDI_HL_A");
            EmitAsm("INC_DE");
            EmitAsm("DEC_BC");
            EmitAsm("LD_A_B");
            EmitAsm("OR_C");
            EmitAsm("JP_NZ", loop);
            return;
        }

        for (int i = 0; i < size; i++)
        {
            EmitAsm("LD_A_DE");
            EmitAsm("LDI_HL_A");
            if (i != size - 1) EmitAsm("INC_DE");
        }
    }

    // Prefer the primary source location, fall back to the argument location, and omit unknown positions.
    Maybe<FilePosition> BestDiagnosticPosition(Expr primary, Expr fallback)
    {
        FilePosition pos = primary == null ? FilePosition.Unknown : primary.Source;
        if (string.IsNullOrEmpty(pos.Filename) || pos.Filename == "<unknown>")
            pos = fallback == null ? FilePosition.Unknown : fallback.Source;
        if (string.IsNullOrEmpty(pos.Filename) || pos.Filename == "<unknown>")
            return Maybe.Nothing;
        return Maybe.Just(pos);
    }

    // Warn when a by-value aggregate exceeds the configured copy-warning threshold.
    void WarnStructValueArgumentIfNeeded(Expr origin, Expr argExpr, string funcName, int argIndex, CType aggregateType, int size)
    {
        if (size <= StructValueArgumentWarningThreshold) return;
        Program.Warning(BestDiagnosticPosition(origin, argExpr), ErrorCode.LargeStructCopy,
            "struct value argument copy of {0} bytes when calling {1} argument {2}; prefer pointer passing for large aggregates",
            size,
            string.IsNullOrEmpty(funcName) ? "<call>" : funcName,
            argIndex);
    }

    // When either side is an aggregate, require matching complete struct/union types before copying argument bytes.
    bool ValidateAggregateArgument(Expr callExpr, string funcName, int argIndex, Expr argExpr, CType paramType)
    {
        CType argType = TypeOf(argExpr);
        bool paramAgg = paramType != null && paramType.WithoutConst().IsStructOrUnion;
        bool argAgg = argType != null && argType.WithoutConst().IsStructOrUnion;
        if (!paramAgg && !argAgg) return true;

        if (!IsSameCompleteAggregateType(paramType, argType))
        {
            Error(callExpr, ErrorCode.ParseError,
                "incompatible struct/union value argument {0} to {1}: expected {2}, got {3}",
                argIndex,
                string.IsNullOrEmpty(funcName) ? "<call>" : funcName,
                paramType == null ? "<unknown>" : paramType.Show(),
                argType == null ? "<unknown>" : argType.Show());
            return false;
        }
        return true;
    }

    // Validate recognizable source/destination storage and copy a by-value aggregate into a fixed argument slot.
    void EmitAggregateArgumentCopyToFixedAddress(Expr callExpr, string funcName, int argIndex, Expr argExpr, CType paramType, int dstAddr, string dstName)
    {
        int size = SizeOf(callExpr, paramType);
        WarnStructValueArgumentIfNeeded(callExpr, argExpr, funcName, argIndex, paramType, size);

        string reason;
        if (!IsImplicitAggregateCopyStorageAllowed(argExpr, false, size, out reason))
        {
            Error(argExpr, ErrorCode.ParseError, reason);
            return;
        }

        string dstRegion;
        if (IsSpecialImplicitCopyAddress(dstAddr, size, out dstRegion))
        {
            Error(callExpr, ErrorCode.ParseError,
                "implicit struct/union value argument copy to {0} is not supported", dstRegion);
            return;
        }

        string strategy = AggregateCopyStrategy(size);
        RecordAggregateCopy(callExpr, paramType, size, strategy);
        EmitComment("kitaqgb.struct_value_arg callee={0} arg={1} type={2} size={3} strategy={4}",
            string.IsNullOrEmpty(funcName) ? "<call>" : funcName,
            argIndex,
            paramType == null ? "<unknown>" : paramType.WithoutConst().Show(),
            size,
            strategy);

        if (!TryCompileAggregateSourceAddressIntoHL(argExpr, paramType))
        {
            Error(argExpr, ErrorCode.ParseError, "struct/union value argument source is not addressable");
            return;
        }

        EmitAsm("PUSH_HL");
        EmitAsm("LD_HL_IMM", new AsmOperand(dstAddr, AddressMode.Immediate16));
        EmitAsm("POP_DE");
        EmitCopyBytesFromDEToHL(size);
    }

    // Copy an aggregate argument to the supplied offset from the current SP after evaluating its source address.
    void EmitAggregateArgumentCopyToStackOffset(Expr callExpr, string funcName, int argIndex, Expr argExpr, CType paramType, int stackOffset)
    {
        int size = SizeOf(callExpr, paramType);
        WarnStructValueArgumentIfNeeded(callExpr, argExpr, funcName, argIndex, paramType, size);

        string reason;
        if (!IsImplicitAggregateCopyStorageAllowed(argExpr, false, size, out reason))
        {
            Error(argExpr, ErrorCode.ParseError, reason);
            return;
        }

        string strategy = AggregateCopyStrategy(size);
        RecordAggregateCopy(callExpr, paramType, size, strategy);
        EmitComment("kitaqgb.struct_value_arg callee={0} arg={1} type={2} size={3} strategy={4} stack_off={5}",
            string.IsNullOrEmpty(funcName) ? "<call>" : funcName,
            argIndex,
            paramType == null ? "<unknown>" : paramType.WithoutConst().Show(),
            size,
            strategy,
            stackOffset);

        if (!TryCompileAggregateSourceAddressIntoHL(argExpr, paramType))
        {
            Error(argExpr, ErrorCode.ParseError, "struct/union value argument source is not addressable");
            return;
        }

        EmitAsm("LD_D_H");
        EmitAsm("LD_E_L");
        EmitAsm("LD_HL_SP_IMM", new AsmOperand(stackOffset, AddressMode.Relative));
        EmitCopyBytesFromDEToHL(size);
    }

    // Route aggregate arguments through validation/copying; store scalar bytes or little-endian words directly.
    void EmitStoreArgumentToFixedAddress(Expr callExpr, string funcName, int argIndex, Expr argExpr, CType paramType, int dstAddr, string dstName)
    {
        CType argType = TypeOf(argExpr);
        bool involvesAggregate =
            (paramType != null && paramType.WithoutConst().IsStructOrUnion) ||
            (argType != null && argType.WithoutConst().IsStructOrUnion);
        if (involvesAggregate)
        {
            if (ValidateAggregateArgument(callExpr, funcName, argIndex, argExpr, paramType))
                EmitAggregateArgumentCopyToFixedAddress(callExpr, funcName, argIndex, argExpr, paramType, dstAddr, dstName);
            return;
        }

        int size = SizeOf(argExpr, paramType);
        if (size == 1)
        {
            CompileIntoA(argExpr);
            EmitStoreA(MemOp(dstAddr, dstName));
        }
        else if (size == 2)
        {
            CompileIntoHL(argExpr);
            EmitAsm("LD_A_L"); EmitStoreA(MemOp(dstAddr, dstName));
            EmitAsm("LD_A_H"); EmitStoreA(MemOp(dstAddr + 1, dstName != null ? dstName + "+1" : null));
        }
        else
        {
            NYI(callExpr, "Argument size not supported");
        }
    }

    // Copy a type-compatible aggregate result to the active inline destination or saved caller return storage.
    void EmitAggregateReturnValue(Expr returnExpr)
    {
        CFunctionInfo info = null;
        int inlineDestAddr = -1;
        // Inline returns require a destination associated with the current inline-return context.
        if (InlineReturnLabels.Count > 0)
        {
            if (InlineStructReturnDestAddrs.Count == 0)
            {
                Error(returnExpr, ErrorCode.ParseError, "missing inline struct/union return destination");
                return;
            }
            inlineDestAddr = InlineStructReturnDestAddrs.Peek();
        }
        else if (string.IsNullOrEmpty(CurrentFunctionName) || !Functions.TryGetValue(CurrentFunctionName, out info))
        {
            Error(returnExpr, ErrorCode.ParseError, "struct/union return outside a known function");
            return;
        }

        CType valueType = TypeOf(returnExpr);
        if (!IsSameCompleteAggregateType(ReturnType, valueType))
        {
            Error(returnExpr, ErrorCode.ParseError,
                "incompatible struct/union return: expected {0}, got {1}",
                ReturnType == null ? "<unknown>" : ReturnType.Show(),
                valueType == null ? "<unknown>" : valueType.Show());
            return;
        }

        int size = SizeOf(returnExpr, ReturnType);
        string reason;
        if (!IsImplicitAggregateCopyStorageAllowed(returnExpr, false, size, out reason))
        {
            Error(returnExpr, ErrorCode.ParseError, reason);
            return;
        }

        if (inlineDestAddr < 0)
            EnsureStructReturnSlot(returnExpr, info, CurrentFunctionName, ReturnType);

        string strategy = AggregateCopyStrategy(size);
        RecordAggregateCopy(returnExpr, ReturnType, size, strategy);
        EmitComment("kitaqgb.struct_return callee={0} type={1} size={2} strategy={3}",
            InlineReturnLabels.Count > 0 ? "<inline>" : CurrentFunctionName,
            ReturnType == null ? "<unknown>" : ReturnType.WithoutConst().Show(),
            size,
            strategy);

        if (!TryCompileAggregateSourceAddressIntoHL(returnExpr, ReturnType))
        {
            Error(returnExpr, ErrorCode.ParseError, "struct/union return source is not addressable");
            return;
        }
        EmitAsm("PUSH_HL");
        if (inlineDestAddr >= 0)
            EmitAsm("LD_HL_IMM", new AsmOperand(inlineDestAddr, AddressMode.Immediate16));
        else
            EmitLoadSavedStructReturnDestIntoHL(info);
        EmitAsm("POP_DE");
        EmitCopyBytesFromDEToHL(size);
    }

    // Recognize aggregate assignment, validate both storage locations, and report unsupported copies as handled errors.
    bool TryEmitAggregateAssignment(Expr assignExpr, Expr left, Expr right)
    {
        CType leftType = TypeOf(left);
        CType rightType = TypeOf(right);
        bool leftAgg = leftType != null && leftType.WithoutConst().IsStructOrUnion;
        bool rightAgg = rightType != null && rightType.WithoutConst().IsStructOrUnion;
        if (!leftAgg && !rightAgg) return false;

        if (!IsSameCompleteAggregateType(leftType, rightType))
        {
            Error(assignExpr, ErrorCode.ParseError,
                "incompatible struct/union assignment: {0} = {1}",
                leftType == null ? "<unknown>" : leftType.Show(),
                rightType == null ? "<unknown>" : rightType.Show());
            return true;
        }

        string reason;
        int size = SizeOf(left, leftType);
        if (!IsImplicitAggregateCopyStorageAllowed(left, true, size, out reason))
        {
            Error(left, ErrorCode.ParseError, reason);
            return true;
        }
        if (!IsImplicitAggregateCopyStorageAllowed(right, false, size, out reason))
        {
            Error(right, ErrorCode.ParseError, reason);
            return true;
        }

        EmitAggregateCopy(assignExpr, left, right, leftType, size);
        return true;
    }

    // Debug-only bounds check for a[i] where `a` is a fixed-size array (known Dimension).
    // Enabled by: -Zcheck / -Zcheck-bounds
    void EmitBoundsCheckIfNeeded(Expr arrExpr, Expr indexExpr)
    {
        if (!Program.CheckBounds) return;
        if (InUnsafe) return;

        // IMPORTANT: array names decay to pointers in TypeOf(Name), so recover the true array type
        // from the symbol table when possible.
        CType t = null;
        Expr arrBase = StripCasts(arrExpr);
        if (arrBase.Match(Tag.Name, out string arrName) && TryFindSymbol(arrName, out Symbol arrSym) && arrSym.Type != null && arrSym.Type.IsArray)
        {
            t = arrSym.Type;
        }
        else
        {
            t = TypeOf(arrExpr);
        }
        if (t == null || !t.IsArray) return;
        int len = t.Dimension;
        if (len <= 0) return;

        // Constant index in range: no need to emit.
        int k;
        if (indexExpr.Match(Tag.Integer, out k))
        {
            if (k >= 0 && k < len) return;
        }

        // A safe-index type suppresses this optional fixed-array bounds check.
        CType indexType = TypeOf(indexExpr);
        if (indexType != null && indexType.IsSafeIndex) return;

        AsmOperand ok = MakeUniqueLabel("kq_bounds_ok");
        int idxSize = SizeOf(indexExpr);

        // Fast path for 8-bit indices into <=255-sized arrays.
        if (idxSize == 1 && len <= 0xFF)
        {
            CompileIntoA(indexExpr);
            EmitAsm("CP_IMM", new AsmOperand(len & 0xFF, AddressMode.Immediate));
            EmitAsm("JP_C", ok);
            EmitAsm("JP", EnsureCheckTrap());
            EmitLabel(ok);
            return;
        }

        // General path: compare HL < len as 16-bit.
        // This fixes false traps for arrays larger than 255 bytes (e.g. board[320]).
        CompileIntoHL(indexExpr);

        // Any 16-bit index is in-range if len exceeds 16-bit addressable range.
        if (len > 0xFFFF) return;

        int lenHi = (len >> 8) & 0xFF;
        int lenLo = len & 0xFF;

        EmitAsm("LD_A_H");
        EmitAsm("CP_IMM", new AsmOperand(lenHi, AddressMode.Immediate));
        EmitAsm("JP_C", ok);
        EmitAsm("JP_NZ", EnsureCheckTrap());
        EmitAsm("LD_A_L");
        EmitAsm("CP_IMM", new AsmOperand(lenLo, AddressMode.Immediate));
        EmitAsm("JP_C", ok);
        EmitAsm("JP", EnsureCheckTrap());
        EmitLabel(ok);
    }

    // Create a shared trap label lazily; bank-zero trap code is injected only if it is used.
    AsmOperand EnsureCheckTrap()
    {
        if (CheckTrapLabel == null)
            CheckTrapLabel = MakeUniqueLabel("kq_trap_check");
        return CheckTrapLabel;
    }

    // Compare HL with an unsigned word limit, jumping on decisive high/low-byte differences.
    // Equality falls through to the caller rather than jumping explicitly to okLabel.
    void EmitCheckHLGeImm16(int limit, AsmOperand okLabel, AsmOperand trapLabel)
    {
        EmitAsm("LD_DE_IMM", new AsmOperand(limit & 0xFFFF, AddressMode.Immediate16));
        EmitAsm("LD_A_H");
        EmitAsm("CP_D");
        EmitAsm("JP_C", trapLabel);
        EmitAsm("JP_NZ", okLabel);
        EmitAsm("LD_A_L");
        EmitAsm("CP_E");
        EmitAsm("JP_C", trapLabel);
    }

    // -Zcheck stack: ensure SP has headroom so that (SP - bytesNeeded) stays above the configured low-water mark.
    // bytesNeeded is a *peak* extra stack usage relative to current SP (in bytes).
    void EmitStackLowWaterCheck(int bytesNeeded)
    {
        if (!Program.CheckStack || InUnsafe) return;
        if (bytesNeeded < 0) bytesNeeded = 0;

        EnsureCheckTrap();

        // HL = SP
        EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));

        // HL = SP - bytesNeeded
        if (bytesNeeded != 0)
        {
            int neg = (0x10000 - (bytesNeeded & 0xFFFF)) & 0xFFFF;
            EmitAsm("LD_DE_IMM", new AsmOperand(neg, AddressMode.Immediate16));
            EmitAsm("ADD_HL_DE");
        }

        int limit = Program.StackReserve > 0 ? Program.EffectiveStackAutoLimit : 0xC000;
        AsmOperand ok = MakeUniqueLabel("kq_sp_ok");
        EmitCheckHLGeImm16(limit, ok, EnsureCheckTrap());
        EmitLabel(ok);
    }

    // Preserve the first fastcall argument in B or BC while the low-water check uses A, HL and DE.
    void EmitFunctionEntryStackCheck(CFunctionInfo info, Expr origin, int bytesNeeded)
    {
        if (!Program.CheckStack || InUnsafe)
        {
            EmitStackLowWaterCheck(bytesNeeded);
            return;
        }

        if (info != null && info.IsFastCall && info.ParameterSymbols != null && info.ParameterSymbols.Length > 0)
        {
            int size = SizeOf(origin, info.ParameterSymbols[0].Type);
            if (size == 1)
            {
                EmitAsm("LD_B_A");
                EmitStackLowWaterCheck(bytesNeeded);
                EmitAsm("LD_A_B");
                return;
            }
            if (size == 2)
            {
                EmitAsm("LD_B_H");
                EmitAsm("LD_C_L");
                EmitStackLowWaterCheck(bytesNeeded);
                EmitAsm("LD_H_B");
                EmitAsm("LD_L_C");
                return;
            }
        }

        EmitStackLowWaterCheck(bytesNeeded);
    }

    // Compile an expression for side effects only.
    void CompileDiscard(Expr expr)
    {
        if (expr == null) return;
        // An unused aggregate result still requires its call and return-storage handling.
        if (IsStructReturnType(TypeOf(expr)))
        {
            CompileCall(expr);
            return;
        }
        int sz = SizeOf(expr);
        if (sz <= 0) return;
        if (sz == 1) CompileIntoA(expr);
        else CompileIntoHL(expr);
    }

    // Bounds check for a "slice" (ptr+len): index < len.
    // Enabled only by -Zcheck (Program.CheckSliceBounds).
    void EmitSliceBoundsCheckIfNeeded(Expr lenExpr, Expr indexExpr)
    {
        if (!Program.CheckSliceBounds || InUnsafe) return;

        // If both are compile-time constants and proven in-range, skip.
        if (indexExpr != null && lenExpr != null &&
            indexExpr.Match(Tag.Integer, out int __idx) && lenExpr.Match(Tag.Integer, out int __len) &&
            __idx >= 0 && __idx < __len)
        {
            return;
        }

        // Load len into DE (u16)
        CompileIntoHL(lenExpr);
        EmitAsm("LD_D_H");
        EmitAsm("LD_E_L");

        // Load index into HL (u16)
        CompileIntoHL(indexExpr);

        // if (HL < DE) ok; else trap.
        AsmOperand ok = MakeUniqueLabel("kq_slice_ok");
        EmitAsm("LD_A_H");
        EmitAsm("CP_D");
        EmitAsm("JP_C", ok);
        EmitAsm("JP_NZ", EnsureCheckTrap()); // H > D
        EmitAsm("LD_A_L");
        EmitAsm("CP_E");
        EmitAsm("JP_C", ok);
        EmitAsm("JP", EnsureCheckTrap());
        EmitLabel(ok);
    }

    // Remove nested outer cast nodes to inspect the underlying expression shape.
    static Expr StripCasts(Expr e)
    {
        while (e.Match(Tag.Cast, out CType _t, out Expr sub))
            e = sub;
        return e;
    }

    // Recognize a named fixed-size array and return its full storage size in bytes.
    bool TryGetFixedArrayByteLength(Expr e, out int byteLen, out string baseName)
    {
        byteLen = 0;
        baseName = null;
        e = StripCasts(e);

        if (e.Match(Tag.Name, out string name))
        {
            if (TryFindSymbol(name, out Symbol sym) && sym.Type.IsArray && sym.Type.Dimension > 0)
            {
                baseName = name;
                byteLen = SizeOf(e, sym.Type);
                return true;
            }
        }
        return false;
    }

    // Emits a runtime check: if HL > limit then trap.
    // (Used by -Zcheck for memcpy/memset length checks.)
    void EmitU16LeqConstOrTrap(int limit)
    {
        if (limit < 0) return;
        AsmOperand ok = MakeUniqueLabel("kq_chk_ok");
        int hi = (limit >> 8) & 0xFF;
        int lo = (limit >> 0) & 0xFF;

        EmitAsm("LD_A_H");
        EmitAsm("CP_IMM", new AsmOperand(hi, AddressMode.Immediate));
        EmitAsm("JP_C", ok); // H < hi
        EmitAsm("JP_NZ", EnsureCheckTrap()); // H > hi

        EmitAsm("LD_A_L");
        EmitAsm("CP_IMM", new AsmOperand(lo, AddressMode.Immediate));
        EmitAsm("JP_C", ok); // L < lo
        EmitAsm("JP_Z", ok); // L == lo
        EmitAsm("JP", EnsureCheckTrap());
        EmitLabel(ok);
    }

    // Emits a runtime check: if A > limit then trap.
    // (Used by -Zcheck for u8-sized memcpy/memset length checks.)
    void EmitU8LeqConstOrTrapInA(int limit)
    {
        if (limit < 0 || limit >= 0xFF) return;

        AsmOperand ok = MakeUniqueLabel("kq_chk8_ok");
        EmitAsm("CP_IMM", new AsmOperand(limit & 0xFF, AddressMode.Immediate));
        EmitAsm("JP_C", ok);
        EmitAsm("JP_Z", ok);
        EmitAsm("JP", EnsureCheckTrap());
        EmitLabel(ok);
    }


    // Reserve executable HRAM before globals and function-local storage are allocated.
    static bool UsesOamDma(Expr expr)
    {
        if (expr == null) return false;
        if (expr.MatchAny(Tag.Call, out Expr target, out Expr[] arguments) &&
            target.Match(Tag.Name, out string name) && name == "__oam_dma") return true;
        foreach (object arg in expr.GetArgs().Skip(1))
        {
            if (arg is Expr child && UsesOamDma(child)) return true;
            if (arg is Expr[] children && children.Any(UsesOamDma)) return true;
        }
        return false;
    }

    // Register runtime storage and declarations before emitting each ROM bank and function body.
    // Function bodies are generated transactionally so their measured stack usage can precede them in an entry guard.
    void CompileProgram(Expr program)
    {
        OutputStack.Push(new OutputTransaction());
        CurrentScope = new LexicalScope(null);

        // Reserve the compiler register slots before user declarations consume the remaining HRAM.
        int addrL = Allocate(HramRegion, 1);
        int addrH = Allocate(HramRegion, 1);
        RegisterL = MemOp(addrL, "reg L");
        RegisterH = MemOp(addrH, "reg H");
        RegisterHL = new WideOperand(RegisterL, RegisterH);

        // Reserve HRAM variables for farcall bank tracking
        int addrRomBank = Allocate(HramRegion, 1);
        int addrRomBankSaved = Allocate(HramRegion, 1);
        StructReturnPtrAddr = Allocate(HramRegion, 2);
        RomBankVar = MemOp(addrRomBank, "__rom_bank");
        RomBankSavedVar = MemOp(addrRomBankSaved, "__rom_bank_saved");
        StructReturnPtrLoVar = MemOp(StructReturnPtrAddr, "__kq_sret_ptr");
        StructReturnPtrHiVar = MemOp(StructReturnPtrAddr + 1, "__kq_sret_ptr+1");
        RomBankSavedAddr = addrRomBankSaved;
        BankThunkSpAddr = Allocate(HramRegion, 1);
        BankThunkBankBaseAddr = Allocate(HramRegion, 8);
        BankThunkRetLoBaseAddr = Allocate(HramRegion, 8);
        BankThunkRetHiBaseAddr = Allocate(HramRegion, 8);
        int addrCriticalDepth = Allocate(HramRegion, 1);
        int addrRngLo = Allocate(HramRegion, 1);
        int addrRngHi = Allocate(HramRegion, 1);
        CriticalDepthVar = MemOp(addrCriticalDepth, "__kq_critical_depth");
        RngStateLoVar = MemOp(addrRngLo, "__kq_rng_lo");
        RngStateHiVar = MemOp(addrRngHi, "__kq_rng_hi");
        // Expose as symbols for optional user access / debugging
        Emit(Tag.Variable, "__rom_bank", addrRomBank, 1);
        Emit(Tag.Variable, "__rom_bank_saved", addrRomBankSaved, 1);
        Emit(Tag.Variable, "__kq_sret_ptr", StructReturnPtrAddr, 2);
        Emit(Tag.Variable, "__kq_thunk_sp", BankThunkSpAddr, 1);
        Emit(Tag.Variable, "__kq_thunk_bank_stack", BankThunkBankBaseAddr, 8);
        Emit(Tag.Variable, "__kq_thunk_retlo_stack", BankThunkRetLoBaseAddr, 8);
        Emit(Tag.Variable, "__kq_thunk_rethi_stack", BankThunkRetHiBaseAddr, 8);
        Emit(Tag.Variable, "__kq_critical_depth", addrCriticalDepth, 1);
        Emit(Tag.Variable, "__kq_rng_lo", addrRngLo, 1);
        Emit(Tag.Variable, "__kq_rng_hi", addrRngHi, 1);
        DeclareSymbol(program, new Symbol(SymbolTag.Global, addrRomBank, CType.UInt8, "__rom_bank"));
        DeclareSymbol(program, new Symbol(SymbolTag.Global, addrRomBankSaved, CType.UInt8, "__rom_bank_saved"));
        DeclareSymbol(program, new Symbol(SymbolTag.Global, StructReturnPtrAddr, CType.MakePointer(CType.UInt8), "__kq_sret_ptr"));
        DeclareSymbol(program, new Symbol(SymbolTag.Global, BankThunkSpAddr, CType.UInt8, "__kq_thunk_sp"));
        DeclareSymbol(program, new Symbol(SymbolTag.Global, BankThunkBankBaseAddr, CType.MakeArray(CType.UInt8, 8), "__kq_thunk_bank_stack"));
        DeclareSymbol(program, new Symbol(SymbolTag.Global, BankThunkRetLoBaseAddr, CType.MakeArray(CType.UInt8, 8), "__kq_thunk_retlo_stack"));
        DeclareSymbol(program, new Symbol(SymbolTag.Global, BankThunkRetHiBaseAddr, CType.MakeArray(CType.UInt8, 8), "__kq_thunk_rethi_stack"));
        DeclareSymbol(program, new Symbol(SymbolTag.Global, addrCriticalDepth, CType.UInt8, "__kq_critical_depth"));
        DeclareSymbol(program, new Symbol(SymbolTag.Global, addrRngLo, CType.UInt8, "__kq_rng_lo"));
        DeclareSymbol(program, new Symbol(SymbolTag.Global, addrRngHi, CType.UInt8, "__kq_rng_hi"));

        if (UsesOamDma(program))
        {
            OamDmaStubAddr = Allocate(HramRegion, 8);
            Emit(Tag.Variable, "__kq_oam_dma_stub", OamDmaStubAddr, 8);
            DeclareSymbol(program, new Symbol(SymbolTag.Global, OamDmaStubAddr,
                CType.MakeArray(CType.UInt8, 8), "__kq_oam_dma_stub"));
        }

        Expr[] declarations;
        if (!program.MatchAny(Tag.Sequence, out declarations))
            Program.Panic("The top level of the syntax tree must be a sequence.");

        AnalyzeReadonlyDataPlacement(declarations);

        // Unwrap optional wrappers:
        // ($bank N <decl>)
        // ($decl_align N <decl>)
        // ($decl_section "NAME" <decl>)
        // ...and record per-declaration metadata.
        // Preserve source positions and placement/calling-convention metadata while stripping declaration wrappers.
        var declItems = new List<(int sourceIndex, int bank, bool isFixedBank, int? fixedOrder, int align, string section, bool isUnsafe, bool isStackCall, Expr decl)>();
        int declSourceIndex = 0;
        foreach (var d0 in declarations)
        {
            int b = 1;
            bool fixedBank = false;
            int? fixedOrder = null;
            int a = 0;
            string s = null;
            bool u = false;
            bool sc = false;
            Expr d = d0;

            // Note: wrappers can nest in any order.
            bool unwrapped;
            do
            {
                unwrapped = false;
                int tmpi; string tmps; Expr inner;
                if (d.Match(Tag.FixedBank, out tmpi, out inner)) { b = tmpi; fixedBank = true; d = inner.WithSource(d.Source); unwrapped = true; continue; }
                if (d.Match(Tag.FixedOrder, out tmpi, out inner)) { fixedOrder = tmpi; d = inner.WithSource(d.Source); unwrapped = true; continue; }
                if (d.Match(Tag.Bank, out tmpi, out inner)) { b = tmpi; d = inner.WithSource(d.Source); unwrapped = true; continue; }
                if (d.Match(Tag.DeclAlign, out tmpi, out inner)) { a = Math.Max(a, tmpi); d = inner.WithSource(d.Source); unwrapped = true; continue; }
                if (d.Match(Tag.DeclSection, out tmps, out inner)) { s = tmps; d = inner.WithSource(d.Source); unwrapped = true; continue; }
                if (d.Match(Tag.Static, out inner)) { d = inner.WithSource(d.Source); unwrapped = true; continue; }
                if (d.Match(Tag.Unsafe, out inner)) { u = true; d = inner.WithSource(d.Source); unwrapped = true; continue; }
                if (d.Match(Tag.StackCall, out inner)) { sc = true; d = inner.WithSource(d.Source); unwrapped = true; continue; }
            } while (unwrapped);

            if (!fixedBank &&
                TryMatchReadonlyDataDecl(d, out CType _nearRdType, out string nearRdName, out Expr[] _nearRdValues) &&
                NearReadonlyDataNames.Contains(nearRdName))
            {
                // Plain C pointers into ROM are near pointers. Keep readonly data that is
                // used without a matching __bankof(symbol) in fixed bank 0 so callers in
                // other banks don't end up dereferencing invisible ROM.
                b = 0;
                fixedBank = true;
            }

            declItems.Add((declSourceIndex++, b, fixedBank, fixedOrder, a, s, u, sc, d));
        }

        if (declItems.Count > 0)
        {
            // Resolve automatic bank choices only after explicit and near-data placement constraints are known.
            var resolvedDeclItems = new List<(int sourceIndex, int bank, bool isFixedBank, int? fixedOrder, int align, string section, bool isUnsafe, bool isStackCall, Expr decl)>(declItems.Count);
            foreach (var it in declItems)
            {
                int bank = it.isFixedBank ? it.bank : ResolveTopLevelRomBank(it.bank, it.decl);
                resolvedDeclItems.Add((it.sourceIndex, bank, it.isFixedBank, it.fixedOrder, it.align, it.section, it.isUnsafe, it.isStackCall, it.decl));
            }
            declItems = resolvedDeclItems;
        }

        // Collect compile-time constants before resolving their expressions, permitting dependency lookup across declarations.
        PendingConstants.Clear();
        PendingConstantsResolving.Clear();
        foreach (var it in declItems)
        {
            if (it.decl.Match(Tag.Constant, out CType pendingType, out string pendingName, out Expr pendingValue))
            {
                // ROM-materialized scalar constants remain addressable objects rather than compile-time-only symbols.
                bool materializedConstScalar =
                    Program.ConstScalarInRom &&
                    pendingType.IsConst &&
                    pendingType.IsSimple &&
                    !pendingType.IsArray &&
                    !pendingType.IsEnum &&
                    (pendingType.SimpleType == CSimpleType.UInt8 || pendingType.SimpleType == CSimpleType.Int8 ||
                     pendingType.SimpleType == CSimpleType.UInt16 || pendingType.SimpleType == CSimpleType.Int16);
                if (materializedConstScalar) continue;
                if (PendingConstants.ContainsKey(pendingName))
                {
                    Error(it.decl, "Symbol redefined: " + pendingName);
                    continue;
                }
                PendingConstants.Add(pendingName, new PendingConstantInfo
                {
                    Origin = it.decl,
                    Type = pendingType,
                    ValueExpr = pendingValue,
                });
            }
        }

        foreach (var it in declItems)
        {
            Expr decl = it.decl;
            string name; CType type; Expr value; FieldInfo[] parsedFields;
            if (decl.Match(Tag.Constant, out type, out name, out value))
            {
                // Optional: materialize const scalar as ROM data so &name becomes valid.
                // Enabled by -Zconst-scalar-in-rom.
                if (Program.ConstScalarInRom &&
                    type.IsConst &&
                    type.IsSimple &&
                    !type.IsArray &&
                    !type.IsEnum &&
                    (type.SimpleType == CSimpleType.UInt8 || type.SimpleType == CSimpleType.Int8 ||
                     type.SimpleType == CSimpleType.UInt16 || type.SimpleType == CSimpleType.Int16))
                {
                    // Not a compile-time-only constant when materialized.
                }
                else
                {
                    EnsureConstantDeclared(name, decl);
                }
            }
            else if (decl.Match(Tag.OpaqueStruct, out name))
            {
                DeclareOpaqueAggregate(decl, name, AggregateLayout.Struct);
            }
            else if (decl.Match(Tag.OpaqueUnion, out name))
            {
                DeclareOpaqueAggregate(decl, name, AggregateLayout.Union);
            }
            else
            {
                int packed = 0;
                int forcedAlign = 0;
                if (decl.Match(Tag.Struct, out name, out parsedFields, out packed, out forcedAlign) ||
                    (packed = 0) == 0 && (forcedAlign = 0) == 0 && decl.Match(Tag.Struct, out name, out parsedFields))
                {
                    DefineAggregate(decl, name, parsedFields, AggregateLayout.Struct, packed != 0, forcedAlign);
                }
                else if (decl.Match(Tag.Union, out name, out parsedFields, out packed, out forcedAlign) ||
                    (packed = 0) == 0 && (forcedAlign = 0) == 0 && decl.Match(Tag.Union, out name, out parsedFields))
                {
                    DefineAggregate(decl, name, parsedFields, AggregateLayout.Union, packed != 0, forcedAlign);
                }
            }
        }

        // Track extern global variable declarations (no allocation) for end-of-build validation.
        Dictionary<string, ExternDeclInfo> externDecls = new Dictionary<string, ExternDeclInfo>();

        foreach (var it in declItems)
        {
            int declBank = it.bank;
            Expr decl = it.decl;
            string name, funcName; CType type, retType; MemoryRegion region; FieldInfo[] paramsFields; Expr body;

            if (decl.Match(Tag.InlineFunction, out retType, out funcName, out paramsFields, out body))
            {
                if (Functions.ContainsKey(funcName) && !Functions[funcName].IsPrototype) Error(decl, "function redefined: " + funcName);
                Symbol[] paramSymbols = new Symbol[paramsFields.Length];
                for (int i = 0; i < paramsFields.Length; i++)
                {
                    int size = SizeOf(decl, paramsFields[i].Type);
                    int addr = AllocatePreferred(size, ColdRegions());
                    paramSymbols[i] = new Symbol(SymbolTag.Local, addr, paramsFields[i].Type, paramsFields[i].Name);
                }

                bool isFastCall = CanUseFastCall(decl, paramsFields, desiredStack: false);

                // Retain inline bodies and their fixed parameter slots for later call-site expansion.
                CFunctionInfo inlInfo = new CFunctionInfo
                {
                    Parameters = paramsFields,
                    ParameterSymbols = paramSymbols,
                    IsFastCall = isFastCall,
                    ReturnType = retType,
                    RomBank = declBank,
                    HasFixedBank = it.isFixedBank,
                    PlacementOrder = it.fixedOrder ?? int.MaxValue,
                    HasFixedOrder = it.fixedOrder.HasValue,
                    IsPrototype = false,
                    IsInline = true,
                    Body = body
                };
                EnsureStructReturnSlot(decl, inlInfo, funcName, retType);
                Functions.Add(funcName, inlInfo);
            }



            // --- static_assert(expr, "msg") ---
            Expr saCond; string saMsg;
            if (decl.Match(Tag.StaticAssert, out saCond, out saMsg))
            {
                // Report a false static assertion only when evaluating its condition introduced no earlier error.
                int prevErr = Program.ErrorCount;
                int v = CalculateConstantExpression(saCond);
                if (Program.ErrorCount == prevErr && v == 0)
                    ReportStaticAssertFailure(decl, saCond, saMsg, v);
                continue;
            }

int mustCheckFlag;
            if (decl.Match(Tag.FunctionDecl, out retType, out funcName, out paramsFields, out mustCheckFlag) ||
                (mustCheckFlag = 0) == 0 && decl.Match(Tag.FunctionDecl, out retType, out funcName, out paramsFields))
            {
                bool desiredStack = Program.AbiStack || it.isStackCall;
                bool isFastCall = CanUseFastCall(decl, paramsFields, desiredStack);

                // Function prototype (no body)
                if (!Functions.ContainsKey(funcName))
                {
                Symbol[] paramSymbols = null;
                if (!desiredStack)
                {
                    paramSymbols = new Symbol[paramsFields.Length];
                    for (int i = 0; i < paramsFields.Length; i++)
                    {
                        int size = SizeOf(decl, paramsFields[i].Type);
                        int addr = AllocatePreferred(size, ColdRegions());
                        paramSymbols[i] = new Symbol(SymbolTag.Local, addr, paramsFields[i].Type, paramsFields[i].Name);
                    }
                }

                // Store prototype bank, ABI and result metadata without emitting a function body.
                CFunctionInfo protoInfo = new CFunctionInfo
                {
                    Parameters = paramsFields,
                    ParameterSymbols = paramSymbols,
                        IsFastCall = isFastCall,
                        IsStackCall = desiredStack,
                        ReturnType = retType,
                        RomBank = declBank,
                        HasFixedBank = it.isFixedBank,
                        PlacementOrder = it.fixedOrder ?? int.MaxValue,
                        HasFixedOrder = it.fixedOrder.HasValue,
                        IsPrototype = true,
                        MustCheck = (mustCheckFlag != 0)
                    };
                    EnsureStructReturnSlot(decl, protoInfo, funcName, retType);
                    Functions.Add(funcName, protoInfo);
                }
                else
                {
                    CFunctionInfo fi = Functions[funcName];
                    if (fi.Parameters != null && fi.Parameters.Length != paramsFields.Length)
                        Error(decl, "function prototype mismatch: " + funcName);

                    if (fi.IsStackCall != desiredStack)
                        Error(decl, "Function calling convention mismatch (__stackcall): " + funcName);

                    // Keep the result-check requirement if any compatible declaration supplied it.
                    fi.MustCheck = fi.MustCheck || (mustCheckFlag != 0);
                    if (fi.IsPrototype)
                    {
                        fi.RomBank = declBank;
                        fi.HasFixedBank = it.isFixedBank;
                        fi.PlacementOrder = it.fixedOrder ?? int.MaxValue;
                        fi.HasFixedOrder = it.fixedOrder.HasValue;
                        if (desiredStack) fi.ParameterSymbols = null;
                        fi.IsFastCall = isFastCall;
                        fi.IsStackCall = desiredStack;
                    }
                    EnsureStructReturnSlot(decl, fi, funcName, retType);
                }
                continue;
            }

            // Definitions complete or create function metadata before the bank-emission pass.
            int mustCheckDef = 0;
            if (decl.Match(Tag.Function, out retType, out funcName, out paramsFields, out mustCheckDef, out body) ||
                (mustCheckDef = 0) == 0 && decl.Match(Tag.Function, out retType, out funcName, out paramsFields, out body))
            {
                bool desiredStack = Program.AbiStack || it.isStackCall;
                bool isFastCall = CanUseFastCall(decl, paramsFields, desiredStack);

                if (Functions.ContainsKey(funcName))
                {
                    CFunctionInfo fi = Functions[funcName];
                    if (!fi.IsPrototype) Error(decl, "function redefined: " + funcName);

                    if (fi.Parameters != null && fi.Parameters.Length != paramsFields.Length)
                        Error(decl, "Function parameters changed");

                    if (fi.IsStackCall != desiredStack)
                        Error(decl, "Function calling convention mismatch (__stackcall): " + funcName);
                    fi.Parameters = paramsFields;
                    if (!desiredStack)
                    {
                        if (fi.ParameterSymbols == null || fi.ParameterSymbols.Length != paramsFields.Length)
                        {
                            Symbol[] paramSymbols = new Symbol[paramsFields.Length];
                            for (int i = 0; i < paramsFields.Length; i++)
                            {
                                int size = SizeOf(decl, paramsFields[i].Type);
                                int addr = AllocatePreferred(size, ColdRegions());
                                paramSymbols[i] = new Symbol(SymbolTag.Local, addr, paramsFields[i].Type, paramsFields[i].Name);
                            }
                            fi.ParameterSymbols = paramSymbols;
                        }
                    }
                    else
                    {
                        fi.ParameterSymbols = null;
                    }
                    fi.IsFastCall = isFastCall;
                    fi.IsStackCall = desiredStack;
                    fi.ReturnType = retType;
                    fi.RomBank = declBank;
                    fi.HasFixedBank = it.isFixedBank;
                    fi.PlacementOrder = it.fixedOrder ?? int.MaxValue;
                    fi.HasFixedOrder = it.fixedOrder.HasValue;
                    fi.IsPrototype = false;
                    fi.Body = body;
                    fi.MustCheck = fi.MustCheck || (mustCheckDef != 0);
                    EnsureStructReturnSlot(decl, fi, funcName, retType);
                }
                else
                {
                Symbol[] paramSymbols = null;
                if (!desiredStack)
                {
                    paramSymbols = new Symbol[paramsFields.Length];
                    for (int i = 0; i < paramsFields.Length; i++)
                    {
                        int size = SizeOf(decl, paramsFields[i].Type);
                        int addr = AllocatePreferred(size, ColdRegions());
                        paramSymbols[i] = new Symbol(SymbolTag.Local, addr, paramsFields[i].Type, paramsFields[i].Name);
                    }
                }

                CFunctionInfo defInfo = new CFunctionInfo
                {
                    Parameters = paramsFields,
                    ParameterSymbols = paramSymbols,
                        IsFastCall = isFastCall,
                        IsStackCall = desiredStack,
                        ReturnType = retType,
                        RomBank = declBank,
                        HasFixedBank = it.isFixedBank,
                        PlacementOrder = it.fixedOrder ?? int.MaxValue,
                        HasFixedOrder = it.fixedOrder.HasValue,
                        IsPrototype = false,
                        Body = body,
                        MustCheck = (mustCheckDef != 0)
                    };
                    EnsureStructReturnSlot(decl, defInfo, funcName, retType);
                    Functions.Add(funcName, defInfo);
                }
                continue;
            }
else if (decl.Match(Tag.Variable, out region, out type, out name, out Expr _range) || decl.Match(Tag.Variable, out region, out type, out name))
            {
                DeclareGlobal(decl, region, CalculateConstantArrayDimensions(type), name, it.align);
            }

	            // extern global variable declarations (no allocation)
	            else if (decl.Match(Tag.ExternVariable, out region, out type, out name, out Expr _erange) || decl.Match(Tag.ExternVariable, out region, out type, out name))
	            {
	                if (!externDecls.ContainsKey(name))
	                {
	                    externDecls.Add(name, new ExternDeclInfo { Origin = decl, Type = type, Region = region });
	                }
	                else
	                {
	                    ExternDeclInfo prev = externDecls[name];
	                    if (!ExternTypeCompatible(prev.Type, type))
	                    {
		                        Error(decl, ErrorCode.ExternTypeMismatch,
		                            string.Format("extern declaration type mismatch for '{0}' (got {1}, previously {2})",
		                                name, type.Show(), prev.Type.Show()));
	                    }
	                }
	            }
	            CompileReadonlyData(decl, it.align, it.section, it.bank);
        }

        // Validate extern declarations now that all globals/readonly data have been declared.
        foreach (var kv in externDecls)
        {
            string exName = kv.Key;
            ExternDeclInfo info = kv.Value;
            Symbol sym;
            if (!CurrentScope.Symbols.TryGetValue(exName, out sym))
            {
	            Error(info.Origin, ErrorCode.ExternUndefined, string.Format("extern symbol not defined: {0}", exName));
                continue;
            }

            if (sym.Tag == SymbolTag.Constant && !Program.ConstScalarInRom)
            {
	            Error(info.Origin, ErrorCode.ExternNotSupported,
	                string.Format("extern refers to an object, but '{0}' is a compile-time constant (enable -Zconst-scalar-in-rom to take its address)",
	                    exName));
                continue;
            }

            if (!ExternTypeCompatible(info.Type, sym.Type))
            {
	            Error(info.Origin, ErrorCode.ExternTypeMismatch,
	                string.Format("extern declaration does not match definition for '{0}' (declared {1}, defined {2})",
	                    exName, info.Type.Show(), sym.Type.Show()));
            }

            // Note: we intentionally do not enforce memory region qualifiers in v1.
        }

        // Record whether any declared function uses a switchable bank beyond the first ROM window.
        BankSwitchingUsed = Functions.Values.Any(fi => fi.RomBank >= 2);

        // Emit ROM content bank-by-bank (readonly data and functions), inserting $skip_to at each bank boundary.
        var banks = declItems.Select(x => x.bank).Distinct().OrderBy(x => x).ToArray();
        foreach (int bank in banks)
        {
            // Bank 0 starts at 0x0000, Bank 1 starts at 0x4000, etc.
            Emit(Tag.SkipTo, bank * 0x4000);

            var bankDeclItems = declItems
                .Where(x => x.bank == bank)
                .OrderBy(x => x.sourceIndex)
                .ToList();
            if (bankDeclItems.Any(x => x.fixedOrder.HasValue))
            {
                // Reorder only explicitly marked positions, leaving unmarked declarations in their original relative slots.
                var reordered = bankDeclItems.ToArray();
                var markedPositions = new List<int>();
                var markedItems = new List<(int sourceIndex, int bank, bool isFixedBank, int? fixedOrder, int align, string section, bool isUnsafe, bool isStackCall, Expr decl)>();

                for (int i = 0; i < bankDeclItems.Count; i++)
                {
                    if (!bankDeclItems[i].fixedOrder.HasValue) continue;
                    markedPositions.Add(i);
                    markedItems.Add(bankDeclItems[i]);
                }

                markedItems = markedItems
                    .OrderBy(x => x.fixedOrder ?? int.MaxValue)
                    .ThenBy(x => x.sourceIndex)
                    .ToList();

                for (int i = 0; i < markedPositions.Count; i++)
                {
                    reordered[markedPositions[i]] = markedItems[i];
                }

                bankDeclItems = reordered.ToList();
            }

            // Near readonly objects use plain 16-bit pointers and are therefore
            // pinned to their declared bank.  Reserve their space before movable
            // functions: otherwise a late string pool can be stranded at the end
            // of bank 0 while the relayout pass is still able to move code away.
            // Explicit fixed-order declarations keep their requested placement.
            bankDeclItems = bankDeclItems
                .OrderBy(x =>
                {
                    if (x.fixedOrder.HasValue) return 1;
                    Expr dataDecl = x.decl;
                    return TryMatchReadonlyDataDecl(dataDecl,
                        out CType _pinnedType,
                        out string pinnedName,
                        out Expr[] _pinnedValues) &&
                        NearReadonlyDataNames.Contains(pinnedName) ? 0 : 1;
                })
                .ToList();

            foreach (var it in bankDeclItems)
            {
                Expr decl = it.decl;

                // Readonly data: emit inline at the current PC (bank region)
                if (TryMatchReadonlyDataDecl(decl, out CType rdType, out string rdName, out Expr[] rdValueExprs))
                {
                    DeclareReadonlyData(decl, rdType, rdName, rdValueExprs, it.align, it.section);
                    continue;
                }

                // const scalar materialization (-Zconst-scalar-in-rom): emit as readonly data (scalar)
                if (Program.ConstScalarInRom && decl.Match(Tag.Constant, out CType ctType, out string ctName, out Expr ctExpr))
                {
                    if (ctType.IsConst && ctType.IsSimple && !ctType.IsArray && !ctType.IsEnum &&
                        (ctType.SimpleType == CSimpleType.UInt8 || ctType.SimpleType == CSimpleType.Int8 ||
                         ctType.SimpleType == CSimpleType.UInt16 || ctType.SimpleType == CSimpleType.Int16))
                    {
                        int v = CalculateConstantExpression(ctExpr);
                        DeclareReadonlyData(decl, ctType, ctName, new int[] { v }, it.align, it.section);
                        continue;
                    }
                }

                // Inline functions: just capture body; no code emission.
                int inlMustCheck = 0;
                if (decl.Match(Tag.InlineFunction, out CType inlRet, out string inlName, out FieldInfo[] inlFields, out inlMustCheck, out Expr inlBody) ||
                    (inlMustCheck = 0) == 0 && decl.Match(Tag.InlineFunction, out inlRet, out inlName, out inlFields, out inlBody))
                {
                    if (!Functions.TryGetValue(inlName, out CFunctionInfo info))
                    {
                        // If not registered (shouldn't happen), register minimal info.
                        Symbol[] paramSymbols = new Symbol[inlFields.Length];
                        for (int i = 0; i < inlFields.Length; i++)
                        {
                            int size = SizeOf(decl, inlFields[i].Type);
                            int addr = AllocatePreferred(size, HotRegions());
                            paramSymbols[i] = new Symbol(SymbolTag.Local, addr, inlFields[i].Type, inlFields[i].Name);
                        }
                        bool isFastCall = CanUseFastCall(decl, inlFields, desiredStack: false);
                        info = new CFunctionInfo
                        {
                            Parameters = inlFields,
                            ParameterSymbols = paramSymbols,
                            IsFastCall = isFastCall,
                            ReturnType = inlRet,
                            RomBank = bank,
                            HasFixedBank = it.isFixedBank,
                            MustCheck = (inlMustCheck != 0)
                        };
                        EnsureStructReturnSlot(decl, info, inlName, inlRet);
                        Functions.Add(inlName, info);
                    }
                    EnsureStructReturnSlot(decl, info, inlName, inlRet);
                    info.IsInline = true;
                    info.Body = inlBody;
                    info.MustCheck = info.MustCheck || (inlMustCheck != 0);
                    continue;
                }

                int fnMustCheck2 = 0;
                if (decl.Match(Tag.Function, out CType retType, out string name, out FieldInfo[] fields, out fnMustCheck2, out Expr body) ||
                    (fnMustCheck2 = 0) == 0 && decl.Match(Tag.Function, out retType, out name, out fields, out body))
                {
                    int _savedUnsafeDepth = UnsafeDepth;
                    if (it.isUnsafe) UnsafeDepth++;
                    if (!string.IsNullOrEmpty(it.section)) Emit(Tag.Section, it.section);
                    int fnAlign = Math.Max(1, it.align);
                    if (fnAlign > 1) Emit(Tag.Align, fnAlign);

                    // Generate function body first (in a transaction) to compute peak stack usage for -Zcheck.
                    BeginStackUsageTracking();
                    Speculate();

                    BeginFunctionFrameTracking();
                    BeginScope();
                    CurrentFunctionName = name;
                    CurrentFunctionBank = bank;
                    ReturnType = retType;
                    NextLabelNumber = 0;
                    CFunctionInfo info = Functions[name];

                    // Reset per-function stack base pointer
                    CurrentStackBaseAddr = -1;
                    CurrentFunctionIsStackCall = info.IsStackCall;

                    // Declare parameters (legacy ABI: pre-allocated HRAM locals; stack ABI: stack-resident unless written)
                    List<(Symbol sym, int offset, int size)> stackCopyList = null;
                    if (!info.IsStackCall)
                    {
                        if (info.ParameterSymbols != null)
                            foreach (Symbol sym in info.ParameterSymbols) DeclareSymbol(decl, sym);
                    }
                    else
                    {
                        // Choose local copies for parameters written by name; other stack parameters keep their incoming storage.
                        bool[] needCopy = AnalyzeStackAbiParamNeedsCopy(body, fields);
                        int[] offsets = new int[fields.Length];
                        // Lay out stack arguments consecutively and request an entry-SP snapshot only when later access needs it.
                        int curOff = 0;
                        bool needsEntrySpSnapshot = false;
                        for (int i = 0; i < fields.Length; i++)
                        {
                            offsets[i] = curOff;
                            int sz = SizeOf(decl, fields[i].Type);
                            bool aggregateParam = fields[i].Type != null && fields[i].Type.WithoutConst().IsStructOrUnion;
                            if (sz != 1 && sz != 2 && !aggregateParam)
                                Error(decl, "Stack ABI supports only 1 or 2 byte parameters");
                            curOff += sz;
                        }

                        stackCopyList = new List<(Symbol sym, int offset, int size)>();
                        for (int i = 0; i < fields.Length; i++)
                        {
                            int sz = SizeOf(decl, fields[i].Type);
                            int off = offsets[i];
                            int argOff = 2 + off;

                            if (needCopy[i] && !fields[i].Type.IsConst)
                            {
                                // Only params that are written by-name get a local copy (hot => HRAM preferred).
                                int addr = AllocatePreferred(sz, HotRegions());
                                var lsym = new Symbol(SymbolTag.Local, addr, fields[i].Type, fields[i].Name);
                                DeclareSymbol(decl, lsym);
                                stackCopyList.Add((lsym, off, sz));

                                // Without an entry-SP snapshot, initial copies must use LD HL,SP+e8.
                                // Keep the snapshot only when the parameter offset no longer fits e8.
                                if (argOff < -128 || argOff > 127)
                                    needsEntrySpSnapshot = true;
                            }
                            else
                            {
                                // Stack-resident param (read-only): accessed via saved SP base + offset.
                                var ssym = new Symbol(SymbolTag.StackParam, off, fields[i].Type, fields[i].Name);
                                DeclareSymbol(decl, ssym);
                                needsEntrySpSnapshot = true;
                            }
                        }

                        // Keep the entry-SP snapshot only when later stack-param loads need it,
                        // or when initial stack-to-local copies no longer fit LD HL,SP+e8.
                        if (needsEntrySpSnapshot)
                            CurrentStackBaseAddr = AllocatePreferred(2, HotRegions());
                    }

                    // FastCall: save register-passed argument into HRAM slot.
                    // (FastCall and StackCall are expected to be mutually exclusive, but guard anyway.)
                    if (info.IsFastCall && info.ParameterSymbols != null && info.ParameterSymbols.Length > 0)
                    {
                        Symbol paramSym = info.ParameterSymbols[0];
                        int size = SizeOf(decl, paramSym.Type);
                        int addr = paramSym.Value;
                        if (size == 1)
                        {
                            EmitStoreA(MemOp(addr, "fastcall arg"));
                        }
                        else if (size == 2)
                        {
                            EmitAsm("LD_A_L"); EmitStoreA(MemOp(addr, "fastcall arg"));
                            EmitAsm("LD_A_H"); EmitStoreA(MemOp(addr + 1, "fastcall arg+1"));
                        }
                    }

                    EmitSaveIncomingStructReturnPointer(info);

                    // Stack ABI: if needed, save SP base at function entry for stack-resident parameters.
                    // Entry layout: SP+0..1 = return address, SP+2 = arg0 ...
                    if (info.IsStackCall && CurrentStackBaseAddr >= 0)
                    {
                        EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                        EmitAsm("LD_A_L"); EmitStoreA(MemOp(CurrentStackBaseAddr, "spbase"));
                        EmitAsm("LD_A_H"); EmitStoreA(MemOp(CurrentStackBaseAddr + 1, "spbase+1"));
                    }

                    // Stack ABI: copy only the parameters that need a local slot (written by-name).
                    if (info.IsStackCall && stackCopyList != null && stackCopyList.Count > 0)
                    {
                        foreach (var item2 in stackCopyList)
                        {
                            Symbol ps = item2.sym;
                            int psz = item2.size;

                            if (CurrentStackBaseAddr >= 0)
                            {
                                EmitStackParamAddrIntoHL(item2.offset, $"stk arg:{ps.Name}");
                                if (psz > 2)
                                {
                                    EmitAsm("PUSH_HL");
                                    EmitAsm("LD_HL_IMM", new AsmOperand(ps.Value, AddressMode.Immediate16));
                                    EmitAsm("POP_DE");
                                    EmitCopyBytesFromDEToHL(psz);
                                }
                                else if (psz == 1)
                                {
                                    EmitAsm("LD_A_HL");
                                    EmitStoreA(MemOp(ps.Value, $"stk arg:{ps.Name}"));
                                }
                                else
                                {
                                    EmitAsm("LDI_A_HL");
                                    EmitStoreA(MemOp(ps.Value, $"stk arg:{ps.Name}"));
                                    EmitAsm("LD_A_HL");
                                    EmitStoreA(MemOp(ps.Value + 1, $"stk arg:{ps.Name}+1"));
                                }
                            }
                            else
                            {
                                int argOff = 2 + item2.offset;
                                if (argOff < -128 || argOff > 127)
                                    Error(decl, "Too many stack arguments (SP offset out of range for LD HL,SP+e8)");

                                if (psz > 2)
                                {
                                    EmitAsm("LD_HL_SP_IMM", new AsmOperand(argOff, AddressMode.Relative));
                                    EmitAsm("PUSH_HL");
                                    EmitAsm("LD_HL_IMM", new AsmOperand(ps.Value, AddressMode.Immediate16));
                                    EmitAsm("POP_DE");
                                    EmitCopyBytesFromDEToHL(psz);
                                }
                                else if (psz == 1)
                                {
                                    EmitAsm("LD_HL_SP_IMM", new AsmOperand(argOff, AddressMode.Relative));
                                    EmitAsm("LD_A_HL");
                                    EmitStoreA(MemOp(ps.Value, $"stk arg:{ps.Name}"));
                                }
                                else
                                {
                                    EmitAsm("LD_HL_SP_IMM", new AsmOperand(argOff, AddressMode.Relative));
                                    EmitAsm("LDI_A_HL");
                                    EmitStoreA(MemOp(ps.Value, $"stk arg:{ps.Name}"));
                                    EmitAsm("LD_A_HL");
                                    EmitStoreA(MemOp(ps.Value + 1, $"stk arg:{ps.Name}+1"));
                                }
                            }
                        }
                    }

                    VariableUsageCounts.Clear();
                    AnalyzeVariableUsage(body, 1);

                    CompileStatement(body);
                    ReturnFromFunction();
                    EndScope();
                    EndFunctionFrameTrackingAndCommit();

                    OutputTransaction fnTx = OutputStack.Pop();
                    // Use the completed body transaction to obtain the peak extra stack usage for the entry check.
                    int fnMaxStack = EndStackUsageTracking();

                    // Emit function entry and precise stack guard, then append the function body.
                    Emit(Tag.Function, name);
                    if (name == "main")
                    {
                        // Real hardware does not promise zero-filled HRAM.
                        // Initialize runtime nesting before any library call.
                        EmitAsm("XOR_A");
                        EmitStoreA(CriticalDepthVar);
                        EmitStoreA(MemOp(BankThunkSpAddr, "__kq_thunk_sp"));
                    }
                    EmitFunctionEntryStackCheck(info, decl, fnMaxStack);
                    Output.Lines.AddRange(fnTx.Lines);

                    UnsafeDepth = _savedUnsafeDepth;
                    continue;
                }

                // Other declarations do not affect ROM layout here.
            }
        }


        if (Output.SpeculationError) Program.Panic("root transaction error: {0}", Output.AbortReason);
    }

    // Round upward using a bit mask; alignments above one are expected to be powers of two.
    static int AlignUp(int v, int a)
    {
        if (a <= 1) return v;
        int m = a - 1;
        return (v + m) & ~m;
    }

    int NaturalAlignOf(Expr origin, CType type)
    {
        // GB-compatible target: natural alignments are currently 1-byte for u8 and 2-byte for u16/pointer/enum.
        // Arrays inherit element alignment. Aggregates use their defined alignment when known.
        if (type == null) return 1;
        if (type.ForcedAlign > 0) return type.ForcedAlign;
        if (type.IsArray) return NaturalAlignOf(origin, type.Subtype);
        if (type.IsPointer) return 2;
        if (type.IsEnum) return 2;
        if (type.IsSimple && (type.SimpleType == CSimpleType.UInt16 || type.SimpleType == CSimpleType.Int16)) return 2;
        if (type.IsStructOrUnion)
        {
            AggregateInfo info;
            if (AggregateTypes.TryGetValue(type.Name, out info) && info.TotalSize >= 0)
                return Math.Max(1, info.Alignment);
            return 1;
        }
        // Default
        return 1;
    }

    // Register an incomplete aggregate with unknown size unless that name already has an entry.
    void DeclareOpaqueAggregate(Expr decl, string name, AggregateLayout layout)
    {
        AggregateInfo existing;
        if (AggregateTypes.TryGetValue(name, out existing)) return;
        AggregateTypes.Add(name, new AggregateInfo(layout, -1, 1, false, Array.Empty<FieldInfo>()));
    }

    // Calculate member offsets and final padding, replacing an incomplete declaration or rejecting a duplicate definition.
    void DefineAggregate(Expr decl, string name, FieldInfo[] parsedFields, AggregateLayout layout, bool isPacked, int forcedAlign)
    {
        FieldInfo[] fields = new FieldInfo[parsedFields.Length];
        int offset = 0;
        int maxSize = 1;
        int aggAlign = 1;
        for (int i = 0; i < parsedFields.Length; i++)
        {
            // Resolve array extents before choosing each member size and packed/natural alignment.
            CType fieldType = CalculateConstantArrayDimensions(parsedFields[i].Type);
            int fieldSize = SizeOf(decl, fieldType);
            int fieldAlign = isPacked ? 1 : NaturalAlignOf(decl, fieldType);
            aggAlign = Math.Max(aggAlign, fieldAlign);

            if (layout == AggregateLayout.Struct)
            {
                offset = AlignUp(offset, fieldAlign);
                fields[i] = new FieldInfo(fieldType, parsedFields[i].Name, offset);
                offset += fieldSize;
            }
            else
            {
                // Union: all fields start at offset 0
                fields[i] = new FieldInfo(fieldType, parsedFields[i].Name, 0);
                maxSize = Math.Max(maxSize, fieldSize);
            }
            maxSize = Math.Max(maxSize, fieldSize);
        }

        // Forced alignment overrides natural aggregate alignment (and may be larger than 2).
        if (forcedAlign > 0) aggAlign = Math.Max(aggAlign, forcedAlign);
        if (layout == AggregateLayout.Union) offset = maxSize;
        int totalSize = AlignUp(offset, aggAlign);

        AggregateInfo info = new AggregateInfo(layout, totalSize, aggAlign, isPacked, fields);
        AggregateInfo existing;
        if (AggregateTypes.TryGetValue(name, out existing))
        {
            if (existing.TotalSize < 0)
            {
                // Overwrite an earlier opaque forward declaration.
                AggregateTypes[name] = info;
                return;
            }
            Error(decl, ErrorCode.ParseError, "duplicate struct/union definition: " + name);
            return;
        }
        AggregateTypes.Add(name, info);
    }

    // Use register passing for exactly one non-aggregate byte/word parameter when the stack ABI is not requested.
    bool CanUseFastCall(Expr origin, FieldInfo[] fields, bool desiredStack)
    {
        if (desiredStack || fields == null || fields.Length != 1) return false;

        CType t = fields[0].Type;
        if (t == null) return false;
        if (t.WithoutConst().IsStructOrUnion) return false;

        int size = SizeOf(origin, t);
        return size == 1 || size == 2;
    }

    // Recognize struct/union results after removing top-level const.
    bool IsStructReturnType(CType type)
    {
        return type != null && type.WithoutConst().IsStructOrUnion;
    }

    // Scalar word results use HL; aggregate results are returned through dedicated storage.
    bool ReturnsInHL(Expr origin, CType type)
    {
        return type != null && !IsStructReturnType(type) && SizeOf(origin, type) == 2;
    }

    // Build a stable hidden return-storage symbol, using anon when no function name is available.
    string StructReturnSlotName(string funcName)
    {
        string safe = string.IsNullOrEmpty(funcName) ? "anon" : funcName;
        return "__kq_sret_" + safe;
    }

    // Name the hidden per-function slot that saves an incoming aggregate-result destination.
    string StructReturnPointerSlotName(string funcName)
    {
        string safe = string.IsNullOrEmpty(funcName) ? "anon" : funcName;
        return "__kq_sret_dst_" + safe;
    }

    int StructReturnTempCounter = 0;

    // Allocate an aligned aggregate-result temporary in preferred cold storage and register its debug symbol.
    int AllocateStructReturnTemp(Expr origin, CType retType, string hint)
    {
        int size = SizeOf(origin, retType);
        int alignment = Math.Max(1, (retType != null && retType.ForcedAlign > 0) ? retType.ForcedAlign : NaturalAlignOf(origin, retType));
        int addr = AllocatePreferred(size, alignment, ColdRegions());
        MemoryRegion actualRegion = InferRegionFromCpuAddress(addr);
        string name = "__kq_sret_tmp_" + (++StructReturnTempCounter).ToString();
        if (!string.IsNullOrEmpty(hint)) name += "_" + hint;
        Emit(Tag.Variable, name, addr, size, actualRegion);
        return addr;
    }

    // Publish HL as the little-endian destination pointer used by the aggregate-return calling convention.
    void EmitStoreHLToGlobalStructReturnPointer()
    {
        EmitAsm("LD_A_L");
        EmitStoreA(StructReturnPtrLoVar);
        EmitAsm("LD_A_H");
        EmitStoreA(StructReturnPtrHiVar);
    }

    // Load a known result address and publish it for the next aggregate-return call.
    void EmitSetGlobalStructReturnPointer(int address, string comment = null)
    {
        EmitAsm("LD_HL_IMM", new AsmOperand(address, AddressMode.Immediate16));
        EmitStoreHLToGlobalStructReturnPointer();
        if (comment != null) EmitComment(comment);
    }

    // Reload the destination saved by this function; diagnose missing metadata before emitting a zero fallback.
    void EmitLoadSavedStructReturnDestIntoHL(CFunctionInfo info)
    {
        if (info == null || info.ReturnPointerSymbol == null)
        {
            Program.Error(FilePosition.Unknown, ErrorCode.ParseError, "missing struct return destination");
            EmitAsm("LD_HL_IMM", new AsmOperand(0, AddressMode.Immediate16));
            return;
        }

        int addr = info.ReturnPointerSymbol.Value;
        EmitLoadA(MemOp(addr, info.ReturnPointerSymbol.Name));
        EmitAsm("LD_L_A");
        EmitLoadA(MemOp(addr + 1, info.ReturnPointerSymbol.Name + "+1"));
        EmitAsm("LD_H_A");
    }

    // Copy the shared incoming destination into the function slot before nested calls can replace it.
    void EmitSaveIncomingStructReturnPointer(CFunctionInfo info)
    {
        if (info == null || !IsStructReturnType(info.ReturnType)) return;
        if (info.ReturnPointerSymbol == null) return;

        int addr = info.ReturnPointerSymbol.Value;
        EmitLoadA(StructReturnPtrLoVar);
        EmitStoreA(MemOp(addr, info.ReturnPointerSymbol.Name));
        EmitLoadA(StructReturnPtrHiVar);
        EmitStoreA(MemOp(addr + 1, info.ReturnPointerSymbol.Name + "+1"));
    }

    // Reuse or allocate the function result buffer, plus a hot two-byte destination slot when function metadata is available.
    Symbol EnsureStructReturnSlot(Expr origin, CFunctionInfo info, string funcName, CType retType)
    {
        if (!IsStructReturnType(retType)) return null;
        if (info != null && info.ReturnSymbol != null) return info.ReturnSymbol;

        int size = SizeOf(origin, retType);
        int alignment = Math.Max(1, (retType != null && retType.ForcedAlign > 0) ? retType.ForcedAlign : NaturalAlignOf(origin, retType));
        int addr = AllocatePreferred(size, alignment, ColdRegions());
        MemoryRegion actualRegion = InferRegionFromCpuAddress(addr);
        string slotName = StructReturnSlotName(funcName);
        Emit(Tag.Variable, slotName, addr, size, actualRegion);

        if (size > StructValueArgumentWarningThreshold)
        {
            Program.Warning(BestDiagnosticPosition(origin, null), ErrorCode.LargeStructCopy,
                "struct return value of {0} bytes from {1}; prefer an output pointer for large aggregates",
                size,
                string.IsNullOrEmpty(funcName) ? "<function>" : funcName);
        }

        Symbol sym = new Symbol(SymbolTag.Global, addr, retType, slotName, actualRegion.WramBank);
        if (info != null)
        {
            info.ReturnSymbol = sym;

            int ptrAddr = AllocatePreferred(2, 1, HotRegions());
            MemoryRegion ptrRegion = InferRegionFromCpuAddress(ptrAddr);
            string ptrName = StructReturnPointerSlotName(funcName);
            Emit(Tag.Variable, ptrName, ptrAddr, 2, ptrRegion);
            info.ReturnPointerSymbol = new Symbol(SymbolTag.Global, ptrAddr, CType.MakePointer(retType), ptrName, ptrRegion.WramBank);
        }
        return sym;
    }

    // Prefer an explicit destination override, then a named result buffer, otherwise an indirect-call temporary.
    int PrepareStructReturnDestination(Expr origin, CType retType, CFunctionInfo info, string funcName)
    {
        if (!IsStructReturnType(retType))
        {
            LastStructReturnAddress = -1;
            return -1;
        }

        int address;
        if (StructReturnDestinationOverrideAddrs.Count > 0)
        {
            address = StructReturnDestinationOverrideAddrs.Peek();
        }
        else if (info != null)
        {
            Symbol retSym = EnsureStructReturnSlot(origin, info, funcName, retType);
            address = retSym.Value;
        }
        else
        {
            address = AllocateStructReturnTemp(origin, retType, "fp");
        }

        EmitSetGlobalStructReturnPointer(address, "kitaqgb.struct_return_dest");
        LastStructReturnAddress = address;
        return address;
    }

    // Recognize a direct call whose registered return type is a struct or union.
    bool TryGetStructReturnCallInfo(Expr expr, out CFunctionInfo info, out string funcName)
    {
        info = null;
        funcName = null;
        Expr callTarget;
        Expr[] args;
        if (!expr.MatchAny(Tag.Call, out callTarget, out args)) return false;
        if (callTarget == null || !callTarget.Match(Tag.Name, out funcName)) return false;
        if (!Functions.TryGetValue(funcName, out info)) return false;
        return IsStructReturnType(info.ReturnType);
    }

    // Resolve aggregate return types from direct functions, function pointers, or expression type inference.
    bool TryGetStructReturnCallType(Expr expr, out CType retType, out CFunctionInfo info, out string funcName)
    {
        retType = null;
        info = null;
        funcName = null;

        Expr callTarget;
        Expr[] args;
        if (!expr.MatchAny(Tag.Call, out callTarget, out args)) return false;

        if (callTarget != null && callTarget.Match(Tag.Name, out funcName) && Functions.TryGetValue(funcName, out info))
        {
            retType = info.ReturnType;
            return IsStructReturnType(retType);
        }
        if (callTarget != null && callTarget.Match(Tag.Name, out string pointerName) &&
            TryFindSymbol(pointerName, out Symbol fpSym) &&
            fpSym.Type != null &&
            fpSym.Type.IsPointer &&
            fpSym.Type.Subtype != null &&
            fpSym.Type.Subtype.IsFunction)
        {
            retType = fpSym.Type.Subtype.Subtype;
            return IsStructReturnType(retType);
        }

        CType callTargetType = TypeOf(callTarget);
        if (callTargetType != null &&
            callTargetType.IsPointer &&
            callTargetType.Subtype != null &&
            callTargetType.Subtype.IsFunction)
        {
            retType = callTargetType.Subtype.Subtype;
            return IsStructReturnType(retType);
        }
        if (callTargetType != null && callTargetType.IsFunction)
        {
            retType = callTargetType.Subtype;
            return IsStructReturnType(retType);
        }

        retType = TypeOf(expr);
        return IsStructReturnType(retType);
    }

    // Emit an aggregate-producing call and return its selected result-storage address in HL.
    bool TryCompileStructReturnCallAddressIntoHL(Expr expr, out CType retType)
    {
        retType = null;
        CFunctionInfo info;
        string funcName;
        if (!TryGetStructReturnCallType(expr, out retType, out info, out funcName)) return false;
        CompileCall(expr);

        int address = LastStructReturnAddress;
        if (address < 0 && info != null)
        {
            Symbol retSym = EnsureStructReturnSlot(expr, info, funcName, info.ReturnType);
            address = retSym.Value;
        }

        if (address < 0)
        {
            Error(expr, ErrorCode.ParseError, "missing struct/union return storage for call expression");
            address = 0;
        }

        EmitAsm("LD_HL_IMM", new AsmOperand(address, AddressMode.Immediate16));
        return true;
    }

    // Accept a compatible aggregate call temporary or resolve an ordinary source lvalue address.
    bool TryCompileAggregateSourceAddressIntoHL(Expr expr, CType expectedType)
    {
        CType retType;
        if (TryCompileStructReturnCallAddressIntoHL(expr, out retType))
        {
            if (!IsSameCompleteAggregateType(expectedType, retType))
            {
                Error(expr, ErrorCode.ParseError,
                    "incompatible struct/union temporary source: expected {0}, got {1}",
                    expectedType == null ? "<unknown>" : expectedType.Show(),
                    retType == null ? "<unknown>" : retType.Show());
                return false;
            }
            return true;
        }

        return TryCompileLValueAddressIntoHL(expr, false);
    }

    // Dispatch statement forms, using specialized stores and control flow before falling back to an expression result in A.
    void CompileStatement(Expr expr)
    {
        if (ShowVerboseComments && !expr.MatchTag(Tag.Sequence) && !expr.Match(Tag.Empty))
            EmitComment("{0}", ToSourceCode(expr));

        Expr subexpr, left, right, init, test, induct, body;
        Expr[] block, parts;
        CType type;
        string name, mnemonic;
        AsmOperand operand;

        if (expr.Match(Tag.Empty)) return;

        // __unsafe marker: compile inner with safety checks suppressed.
        if (expr.Match(Tag.Unsafe, out subexpr))
        {
            UnsafeDepth++;
            CompileStatement(subexpr);
            UnsafeDepth--;
            return;
        }

        // static_assert inside function body (compile-time only)
        Expr saCond2; string saMsg2;
        if (expr.Match(Tag.StaticAssert, out saCond2, out saMsg2))
        {
            int prevErr = Program.ErrorCount;
            int v = CalculateConstantExpression(saCond2);
            if (Program.ErrorCount == prevErr && v == 0)
                ReportStaticAssertFailure(expr, saCond2, saMsg2, v);
            return;
        }

        string opName;
        if (expr.Match(Tag.AssignModify, out opName, out left, out right))
        {
            // Express compound assignment as a read-modify-write tree for the normal assignment paths.
            Expr expanded = Expr.Make(Tag.Assign, left, Expr.Make(opName, left, right));
            CompileStatement(expanded);
            return;
        }

        if (expr.MatchAny(Tag.Sequence, out block))
        {
            foreach (Expr stmt in block) CompileStatement(stmt);
            return;
        }

        if (expr.Match(Tag.Variable, out type, out name, out Expr _range) || expr.Match(Tag.Variable, out type, out name))
        {
            DeclareLocal(expr, type, name);
            return;
        }

        if (expr.Match(Tag.Asm, out mnemonic, out operand))
        {
            // Resolve known data symbols in inline assembly and select LDH addressing for HRAM operands.
            AsmOperand fixedOp = operand;
            if (operand.Base.HasValue && TryFindSymbol(operand.Base.Value, out Symbol sym) && sym.Tag != SymbolTag.ReadonlyData)
            {
                int baseVal = sym.Value;

                bool isHram = IsHighMemAddr(baseVal);
                AddressMode mode = operand.Mode;

                if (mode == AddressMode.Absolute && isHram) mode = AddressMode.HighMem;
                if (isHram) baseVal &= 0xFF;

                fixedOp = operand.ReplaceBase(baseVal).WithMode(mode).WithComment(sym.Name);

                if (isHram)
                {
                    if (mnemonic == "LD_A_MEM") mnemonic = "LDH_A_MEM";
                    else if (mnemonic == "LD_MEM_A") mnemonic = "LDH_MEM_A";
                }

            }
            Emit(Tag.Asm, mnemonic, fixedOp);
            return;
        }

        // Increment/Decrement
        if (expr.Match(Tag.PreIncrement, out subexpr) || expr.Match(Tag.PostIncrement, out subexpr) ||
            expr.Match(Tag.PreDecrement, out subexpr) || expr.Match(Tag.PostDecrement, out subexpr))
        {
            bool isInc = expr.MatchTag(Tag.PreIncrement) || expr.MatchTag(Tag.PostIncrement);

            if (SizeOf(subexpr) == 1)
            {
                if (TryGetOperand(subexpr, out operand))
                {
                    if (operand.Mode == AddressMode.HighMem) EmitAsm("LDH_A_MEM", operand);
                    else EmitAsm("LD_A_MEM", operand);

                    if (isInc) EmitAsm("INC_A"); else EmitAsm("DEC_A");

                    if (operand.Mode == AddressMode.HighMem) EmitAsm("LDH_MEM_A", operand);
                    else EmitAsm("LD_MEM_A", operand);
                    return;
                }

                // Fallback for non-addressable lvalues (e.g. arr[i]++).
                if (TryCompileLValueAddressIntoHL(subexpr, true))
                {
                    EmitAsm("LD_A_HL");
                    if (isInc) EmitAsm("INC_A"); else EmitAsm("DEC_A");
                    EmitAsm("LD_HL_A");
                    return;
                }
            }
            else if (SizeOf(subexpr) == 2)
            {
                if (TryGetWideOperand(subexpr, out WideOperand wideOp))
                {
                    if (wideOp.Low.Mode == AddressMode.HighMem) EmitAsm("LDH_A_MEM", wideOp.Low);
                    else EmitAsm("LD_A_MEM", wideOp.Low);
                    EmitAsm("LD_L_A");

                    if (wideOp.High.Mode == AddressMode.HighMem) EmitAsm("LDH_A_MEM", wideOp.High);
                    else EmitAsm("LD_A_MEM", wideOp.High);
                    EmitAsm("LD_H_A");

                    int step = 1;
                    CType targetType = TypeOf(subexpr);
                    if (targetType.IsPointer) step = SizeOf(subexpr, targetType.Subtype);

                    int adjustment = isInc ? step : -step;
                    EmitAsm("LD_DE_IMM", new AsmOperand(adjustment & 0xFFFF, AddressMode.Immediate16));
                    EmitAsm("ADD_HL_DE");

                    EmitAsm("LD_A_L");
                    if (wideOp.Low.Mode == AddressMode.HighMem) EmitAsm("LDH_MEM_A", wideOp.Low);
                    else EmitAsm("LD_MEM_A", wideOp.Low);

                    EmitAsm("LD_A_H");
                    if (wideOp.High.Mode == AddressMode.HighMem) EmitAsm("LDH_MEM_A", wideOp.High);
                    else EmitAsm("LD_MEM_A", wideOp.High);
                    return;
                }

                // Fallback for non-addressable 16-bit lvalues.
                if (TryCompileLValueAddressIntoHL(subexpr, true))
                {
                    // Preserve address while loading/storing the 16-bit value.
                    EmitAsm("PUSH_HL");
                    EmitAsm("LD_A_HL");
                    EmitAsm("LD_E_A");
                    EmitAsm("INC_HL");
                    EmitAsm("LD_A_HL");
                    EmitAsm("LD_D_A");
                    EmitAsm("LD_H_D");
                    EmitAsm("LD_L_E");

                    int step = 1;
                    CType targetType = TypeOf(subexpr);
                    if (targetType != null && targetType.IsPointer) step = SizeOf(subexpr, targetType.Subtype);

                    int adjustment = isInc ? step : -step;
                    if (adjustment == 1) EmitAsm("INC_HL");
                    else if (adjustment == -1) EmitAsm("DEC_HL");
                    else
                    {
                        EmitAsm("LD_DE_IMM", new AsmOperand(adjustment & 0xFFFF, AddressMode.Immediate16));
                        EmitAsm("ADD_HL_DE");
                    }

                    EmitAsm("LD_D_H");
                    EmitAsm("LD_E_L");
                    EmitAsm("POP_HL");
                    EmitAsm("LD_A_E");
                    EmitAsm("LD_HL_A");
                    EmitAsm("INC_HL");
                    EmitAsm("LD_A_D");
                    EmitAsm("LD_HL_A");
                    return;
                }
            }
            NYI(expr, "Complex increment/decrement not supported");
            return;
        }

        if (expr.MatchAny(Tag.If, out parts))
        {
            // Fold constant arms and share an exit label only when a preceding arm must skip later alternatives.
            AsmOperand endIf = MakeUniqueLabel("end_if");
            bool usedEndIf = false;
            for (int i = 0; i < parts.Length; i += 2)
            {
                test = FoldConstants(parts[i]); body = parts[i + 1];

                if (test.Match(Tag.Integer, out int ifConst))
                {
                    if (ifConst == 0) continue;
                    BeginScope(); CompileStatement(body); EndScope();
                    if (usedEndIf) EmitLabel(endIf);
                    return;
                }

                AsmOperand elseLabel = MakeUniqueLabel("else");
                CompileJumpIf(false, test, elseLabel);
                BeginScope(); CompileStatement(body); EndScope();
                if (i < parts.Length - 2)
                {
                    EmitAsm("JP", endIf);
                    usedEndIf = true;
                }
                EmitLabel(elseLabel);
            }
            if (usedEndIf) EmitLabel(endIf);
            return;
        }
        if (expr.Match(Tag.DoWhile, out body, out test))
        {
            test = FoldConstants(test);
            AsmOperand top = MakeUniqueLabel("do_top");
            // Continue in a do-while loop targets the condition after the body.
            AsmOperand check = MakeUniqueLabel("do_check");
            AsmOperand end = MakeUniqueLabel("do_end");
            Loop = new LoopScope { Outer = Loop, ContinueLabel = check, BreakLabel = end };
            EmitLabel(top);
            BeginScope();
            // The nearest loop or switch owns break, irrespective of nesting order.
            BreakLabels.Push(end);
            CompileStatement(body);
            BreakLabels.Pop();
            EmitLabel(check);
            CompileJumpIf(true, test, top);
            EmitLabel(end);
            EndScope();
            Loop = Loop.Outer;
            return;
        }

        if (expr.Match(Tag.For, out init, out test, out induct, out body))
        {
            if (TryCompileCountdownForLoop(init, test, induct, body))
            {
                return;
            }

            // A constant-false for condition still executes the initializer within its scope.
            Expr foldedTest = test.Match(Tag.Empty) ? test : FoldConstants(test);
            if (!foldedTest.Match(Tag.Empty) && foldedTest.Match(Tag.Integer, out int forConst) && forConst == 0)
            {
                BeginScope();
                CompileStatement(init);
                EndScope();
                return;
            }

            AsmOperand top = MakeUniqueLabel("loop_top");
            AsmOperand end = MakeUniqueLabel("loop_end");
            Loop = new LoopScope { Outer = Loop, ContinueLabel = top, BreakLabel = end };
            BeginScope(); CompileStatement(init); EmitLabel(top);
            if (!foldedTest.Match(Tag.Empty)) CompileJumpIf(false, foldedTest, end);
            BreakLabels.Push(end);
            CompileStatement(body);
            BreakLabels.Pop();
            CompileStatement(induct);
            EmitAsm("JP", top); EmitLabel(end); EndScope();
            Loop = Loop.Outer; return;
        }

        if (expr.Match(Tag.Return))
        {
            if (InlineReturnLabels.Count > 0)
            {
                EmitAsm("JP", InlineReturnLabels.Peek());
            }
            else
            {
                ReturnFromFunction();
            }
            return;
        }
        if (expr.Match(Tag.Return, out subexpr))
        {
            if (IsStructReturnType(ReturnType))
            {
                EmitAggregateReturnValue(subexpr);
            }
            else if (SizeOf(expr, ReturnType) == 2) CompileIntoHL(subexpr);
            else CompileIntoA(subexpr);
            if (InlineReturnLabels.Count > 0)
            {
                EmitAsm("JP", InlineReturnLabels.Peek());
            }
            else
            {
                ReturnFromFunction();
            }
            return;
        }

        // runtime assert: __assert(cond) / __assert(cond, code)
        if (expr.MatchAny(Tag.Call, out Expr assertCallee, out Expr[] assertArgs) &&
            assertCallee.Match(Tag.Name, out string assertName) &&
            assertName == "__assert")
        {
            CompileRuntimeAssert(expr, assertArgs);
            return;
        }

        if (expr.MatchTag(Tag.Call))
        {
            // Lint: warn if the return value of a [[nodiscard]] / __must_check function is ignored.
            if (expr.MatchAny(Tag.Call, out Expr calleeExpr, out Expr[] _args) &&
                calleeExpr.Match(Tag.Name, out string calleeName) &&
                Functions.TryGetValue(calleeName, out CFunctionInfo finfo) &&
                finfo.MustCheck &&
                !(finfo.ReturnType.IsSimple && finfo.ReturnType.SimpleType == CSimpleType.Void))
            {
                Program.Warning(expr.Source, ErrorCode.MustCheckUnused, "return value of '{0}' is ignored", calleeName);
            }

            CompileCall(expr);

            if (IsStructReturnType(TypeOf(expr)))
                return;

            // Prefer CompileCall's intrinsic return-width tracking.
            if (LastCallReturnsHL)
            {
                EmitAsm("LD_A_L");
                return;
            }
            if (SizeOf(expr) == 2) EmitAsm("LD_A_L");
            return;
        }

        if (expr.Match(Tag.Assign, out left, out right))
        {
            if (TryEmitAggregateAssignment(expr, left, right))
                return;

            // For an indexed aggregate field, form base plus scaled index plus field offset before evaluating the stored value.
            if (left.Match(Tag.Field, out Expr structExpr2, out string fieldName2))
            {
                if (structExpr2.Match(Tag.Index, out Expr arrExpr2, out Expr idxExpr2))
                {
                    EmitBoundsCheckIfNeeded(arrExpr2, idxExpr2);
                    int fSize = SizeOf(left);
                    if (fSize == 1 || fSize == 2)
                    {
                        Speculate();

                        CompileIntoHL(arrExpr2);
                        EmitAsm("PUSH_HL");

                        int elemSize2 = GetIndexElementSize(arrExpr2);
                        CompileScaledIndexToDE(idxExpr2, elemSize2); // DE = i*sizeof(elem)

                        EmitAsm("POP_HL");
                        EmitAsm("ADD_HL_DE"); // HL = base + offset

                        FieldInfo field2 = GetFieldInfo(structExpr2, fieldName2);
                        if (field2.Offset != 0)
                        {
                            EmitAsm("LD_DE_IMM", new AsmOperand(field2.Offset, AddressMode.Immediate16));
                            EmitAsm("ADD_HL_DE");
                        }

                        EmitAsm("PUSH_HL"); // address

                        if (fSize == 1)
                        {
                            CompileIntoA(right);
                            EmitAsm("POP_HL");
                            EmitAsm("LD_HL_A");
                        }
                        else
                        {
                            // u16 store (little-endian): *(u16*)addr = HL
                            CompileIntoHL(right); // value
                            EmitAsm("POP_DE"); // address -> DE
                            EmitAsm("LD_A_L"); EmitAsm("LD_DE_A");
                            EmitAsm("INC_DE");
                            EmitAsm("LD_A_H"); EmitAsm("LD_DE_A");
                        }

                        if (Commit()) return;
                    }
                }
            }

            // For a dereferenced aggregate field, keep its computed address on the stack while producing the value.
            if (left.Match(Tag.Field, out Expr structExpr, out string fieldName))
            {
                if (structExpr.Match(Tag.Load, out Expr ptrExpr))
                {
                    int fSize = SizeOf(left);
                    if (fSize == 1 || fSize == 2)
                    {
                        Speculate();
                        CompileIntoHL(ptrExpr);

                        FieldInfo field = GetFieldInfo(structExpr, fieldName);
                        if (field.Offset != 0)
                        {
                            EmitAsm("LD_DE_IMM", new AsmOperand(field.Offset, AddressMode.Immediate16));
                            EmitAsm("ADD_HL_DE");
                        }

                        EmitAsm("PUSH_HL"); // address

                        if (fSize == 1)
                        {
                            CompileIntoA(right);
                            EmitAsm("POP_HL");
                            EmitAsm("LD_HL_A");
                        }
                        else
                        {
                            // u16 store (little-endian): *(u16*)addr = HL
                            CompileIntoHL(right); // value
                            EmitAsm("POP_DE"); // address -> DE
                            EmitAsm("LD_A_L"); EmitAsm("LD_DE_A");
                            EmitAsm("INC_DE");
                            EmitAsm("LD_A_H"); EmitAsm("LD_DE_A");
                        }

                        if (Commit()) return;
                    }
                }
            }

            // struct local/global field assignment: s.x = rhs
            if (left.Match(Tag.Field, out Expr structExprName, out string fieldNameName))
            {
                if (structExprName.Match(Tag.Name, out string baseName))
                {
                    Symbol baseSym = FindSymbol(structExprName, baseName);
                    if (baseSym.Tag == SymbolTag.ReadonlyData)
                    {
                        Error(left, "Cannot assign to readonly data: " + baseName);
                        return;
                    }

                    if (baseSym.Tag == SymbolTag.Global || baseSym.Tag == SymbolTag.Local)
                    {
                        FieldInfo field = GetFieldInfo(structExprName, fieldNameName);
                        int fieldOffset = (field != null) ? field.Offset : 0;
                        int addr = baseSym.Value + fieldOffset;
                        int fSize = SizeOf(left);

                        if (fSize == 1)
                        {
                            CompileIntoA(right);
                            EmitStoreA(MemOp(addr, baseName + "." + fieldNameName));
                            return;
                        }
                        if (fSize == 2)
                        {
                            CompileIntoHL(right);
                            EmitAsm("LD_A_L");
                            EmitStoreA(MemOp(addr, baseName + "." + fieldNameName));
                            EmitAsm("LD_A_H");
                            EmitStoreA(MemOp(addr + 1, baseName + "." + fieldNameName + "+1"));
                            return;
                        }
                    }
                }
            }

            // Generic field lvalue store (supports nested fields).
            if (left.Match(Tag.Field, out Expr _anyStructExpr, out string _anyFieldName))
            {
                int fSize = SizeOf(left);
                if (fSize == 1)
                {
                    CompileIntoA(right);
                    EmitAsm("PUSH_AF");
                    if (TryCompileLValueAddressIntoHL(left, true))
                    {
                        EmitAsm("POP_AF");
                        EmitAsm("LD_HL_A");
                        return;
                    }
                    EmitAsm("POP_AF");
                }
                else if (fSize == 2)
                {
                    CompileIntoHL(right);
                    EmitAsm("PUSH_HL");
                    if (TryCompileLValueAddressIntoHL(left, true))
                    {
                        EmitAsm("POP_DE");
                        EmitAsm("LD_A_E");
                        EmitAsm("LD_HL_A");
                        EmitAsm("INC_HL");
                        EmitAsm("LD_A_D");
                        EmitAsm("LD_HL_A");
                        return;
                    }
                    EmitAsm("POP_DE");
                }
            }

            if (left.Match(Tag.Load, out Expr loadExpr))
            {
                Expr ptrExpr = loadExpr;
                if (ptrExpr.Match(Tag.Cast, out CType castType, out Expr inner)) ptrExpr = inner;

                int dSize = SizeOf(left);

                // Constant address store: *(T*)0x1234 = rhs
                if (ptrExpr.Match(Tag.Integer, out int addrVal))
                {
                    if (dSize == 1)
                    {
                        CompileIntoA(right);
                        if (addrVal >= 0xFF00)
                            EmitAsm("LDH_MEM_A", new AsmOperand(addrVal & 0xFF, AddressMode.HighMem));
                        else
                            EmitAsm("LD_MEM_A", new AsmOperand(addrVal, AddressMode.Absolute));
                        return;
                    }
                    else if (dSize == 2)
                    {
                        CompileIntoHL(right);

                        EmitAsm("LD_A_L");
                        if (addrVal >= 0xFF00)
                            EmitAsm("LDH_MEM_A", new AsmOperand(addrVal & 0xFF, AddressMode.HighMem));
                        else
                            EmitAsm("LD_MEM_A", new AsmOperand(addrVal, AddressMode.Absolute));

                        int addrHi = addrVal + 1;
                        EmitAsm("LD_A_H");
                        if (addrHi >= 0xFF00)
                            EmitAsm("LDH_MEM_A", new AsmOperand(addrHi & 0xFF, AddressMode.HighMem));
                        else
                            EmitAsm("LD_MEM_A", new AsmOperand(addrHi, AddressMode.Absolute));
                        return;
                    }
                    else
                    {
                        Error(left, "Unsupported store size: " + dSize);
                        return;
                    }
                }

                // Dynamic address store: *ptrExpr = rhs
                if (dSize == 1)
                {
                    CompileIntoA(right);
                    EmitAsm("PUSH_AF");
                    CompileIntoHL(ptrExpr);
                    EmitAsm("POP_AF");
                    EmitAsm("LD_HL_A");
                    return;
                }
                else if (dSize == 2)
                {
                    // value -> DE, address -> HL, store E then D
                    CompileIntoHL(right); // value in HL
                    EmitAsm("PUSH_HL");
                    CompileIntoHL(ptrExpr); // address in HL
                    EmitAsm("POP_DE"); // value -> DE

                    EmitAsm("LD_A_E");
                    EmitAsm("LD_HL_A");
                    EmitAsm("INC_HL");
                    EmitAsm("LD_A_D");
                    EmitAsm("LD_HL_A");
                    return;
                }
                else
                {
                    Error(left, "Unsupported store size: " + dSize);
                    return;
                }
            }

            if (left.Match(Tag.Index, out Expr arrExpr, out Expr idxExpr))
            {
                EmitBoundsCheckIfNeeded(arrExpr, idxExpr);
                // Assignment to indexed lvalue: arr[idx] = right
                int lhsSize = SizeOf(left);
                int rhsSize = SizeOf(right);
                // C assignment width must follow the lvalue element type, not RHS promotion width.
                // (e.g. u8_array[i] = (u16_expr) should store one byte without scaling index by 2)
                int elemSize = GetIndexElementSize(arrExpr);
                if (lhsSize == 1 || lhsSize == 2)
                    elemSize = lhsSize; // keep index scaling and store width coherent for scalar lvalues
                if (elemSize != 1 && elemSize != 2)
                    elemSize = 1;
                int storeSize = elemSize;

                if (storeSize == 1)
                {
                    // Compute value -> A
                    if (rhsSize == 2)
                    {
                        // Best-effort: take low byte (keeps compilation from failing)
                        CompileIntoHL(right);
                        EmitAsm("LD_A_L");
                    }
                    else
                    {
                        CompileIntoA(right);
                    }
                    EmitAsm("PUSH_AF");

                    // DE = idx * elemSize
                    CompileScaledIndexToDE(idxExpr, elemSize);
                    EmitAsm("PUSH_DE");

                    // HL = base address
                    CType baseType = TypeOf(arrExpr);
                    if (baseType.IsArray && arrExpr.Match(Tag.Name, out string arrName))
                    {
                        Symbol sym = FindSymbol(arrExpr, arrName);
                        if (sym.Tag == SymbolTag.ReadonlyData)
                            Error(left, "Cannot assign to readonly data: " + arrName);
                        EmitAsm("LD_HL_IMM", new AsmOperand(sym.Value, AddressMode.Immediate16));
                    }
                    else
                    {
                        // Pointer: evaluate base pointer value
                        CompileIntoHL(arrExpr);
                    }

                    EmitAsm("POP_DE");
                    EmitAsm("ADD_HL_DE");
                    EmitAsm("POP_AF");
                    EmitAsm("LD_HL_A");
                    return;
                }
                else if (storeSize == 2)
                {
                    // Value -> DE, address -> HL, store E then D
                    CompileIntoHL(right);
                    EmitAsm("PUSH_HL"); // value

                    // DE = idx * elemSize
                    CompileScaledIndexToDE(idxExpr, elemSize);
                    EmitAsm("PUSH_DE");

                    // HL = base address
                    CType baseType = TypeOf(arrExpr);
                    if (baseType.IsArray && arrExpr.Match(Tag.Name, out string arrName))
                    {
                        Symbol sym = FindSymbol(arrExpr, arrName);
                        if (sym.Tag == SymbolTag.ReadonlyData)
                            Error(left, "Cannot assign to readonly data: " + arrName);
                        EmitAsm("LD_HL_IMM", new AsmOperand(sym.Value, AddressMode.Immediate16));
                    }
                    else
                    {
                        CompileIntoHL(arrExpr);
                    }

                    EmitAsm("POP_DE");
                    EmitAsm("ADD_HL_DE"); // HL = addr
                    EmitAsm("POP_DE"); // DE = value

                    EmitAsm("LD_A_E");
                    EmitAsm("LD_HL_A");
                    EmitAsm("INC_HL");
                    EmitAsm("LD_A_D");
                    EmitAsm("LD_HL_A");
                    return;
                }
            }


            int size = SizeOf(left);
            if (size == 1)
            {
                if (TryGetOperand(left, out AsmOperand leftOp))
                {
                    CompileIntoA(right);
                    if (leftOp.Mode == AddressMode.HighMem) EmitAsm("LDH_MEM_A", leftOp);
                    else EmitAsm("LD_MEM_A", leftOp);
                    return;
                }
            }
            else if (size == 2)
            {
                if (TryGetOperand(left, out AsmOperand leftOp))
                {
                    CompileIntoHL(right);
                    EmitAsm("LD_A_L");
                    if (leftOp.Mode == AddressMode.HighMem) EmitAsm("LDH_MEM_A", leftOp);
                    else EmitAsm("LD_MEM_A", leftOp);
                    EmitAsm("LD_A_H");
                    AsmOperand highOp = new AsmOperand(Maybe.Nothing, leftOp.Offset + 1, leftOp.Mode, ImmediateModifier.None, leftOp.Comment + "+1");
                    if (leftOp.Base.HasValue) highOp = new AsmOperand(leftOp.Base, leftOp.Offset + 1, leftOp.Mode, ImmediateModifier.None, leftOp.Comment + "+1");
                    if (leftOp.Mode == AddressMode.HighMem) EmitAsm("LDH_MEM_A", highOp);
                    else EmitAsm("LD_MEM_A", highOp);
                    return;
                }
            }
        }

        if (expr.MatchTag(Tag.Label)) { Emit(expr); return; }

        if (expr.Match(Tag.Jump, out name))
        {
            EmitAsm("JP", new AsmOperand(name, AddressMode.Absolute));
            return;
        }

        if (expr.Match(Tag.Continue))
        {
            if (Loop == null) Error(expr, "continue outside of loop");
            EmitAsm("JP", Loop.ContinueLabel);
            return;
        }
        if (expr.Match(Tag.Break))
        {
            if (BreakLabels.Count > 0)
            {
                EmitAsm("JP", BreakLabels.Peek());
                return;
            }
            if (Loop == null) Error(expr, "break outside of loop or switch");
            EmitAsm("JP", Loop.BreakLabel);
            return;
        }
        if (expr.Match(Tag.Fallthrough))
        {
            // Marker-only statement. Control naturally continues into the next case body.
            return;
        }

        if (expr.Match(Tag.Switch, out test, out Expr[] cases, out Expr defaultStmt))
        {
            if (cases.Length == 0)
            {
                CompileIntoA(test);
                // A default-only switch still executes its body and owns its break target.
                if (!defaultStmt.Match(Tag.Empty))
                {
                    AsmOperand defaultEnd = MakeUniqueLabel("switch_end");
                    BreakLabels.Push(defaultEnd);
                    CompileStatement(defaultStmt);
                    BreakLabels.Pop();
                    EmitLabel(defaultEnd);
                }
                return;
            }

            // Validate enum case labels, then collect the byte range and source order for jump-table dispatch.
            CType switchType = TypeOf(test);
            bool switchIsEnum = (switchType != null && switchType.IsEnum);
            bool switchIsStrictEnum = switchIsEnum && switchType.IsEnumStrict;
            string switchEnumName = switchIsEnum ? switchType.Name : null;

            int minVal = int.MaxValue;
            int maxVal = int.MinValue;

            Dictionary<int, Expr> caseMap = new Dictionary<int, Expr>();
            List<int> caseOrder = new List<int>();

            foreach (Expr c in cases)
            {
                c.Match(Tag.Case, out Expr valExpr, out Expr caseBody);

                if (switchIsEnum)
                {
                    CType caseType = TypeOf(valExpr);
                    if (caseType != null && caseType.IsEnum)
                    {
                        if (caseType.Name != switchEnumName)
                        {
                            if (switchIsStrictEnum)
                            {
                                Program.Error(Maybe.Just(valExpr.Source), ErrorCode.SwitchCaseEnumMismatch,
                                    "case label enum '{0}' does not match strict switch enum '{1}'", caseType.Name, switchEnumName);
                            }
                            else
                            {
                                Program.Warning(Maybe.Just(valExpr.Source), ErrorCode.SwitchCaseEnumMismatch,
                                    "case label enum '{0}' does not match switch enum '{1}'", caseType.Name, switchEnumName);
                            }
                        }
                    }
                    else
                    {
                        if (switchIsStrictEnum)
                        {
                            Program.Error(Maybe.Just(valExpr.Source), ErrorCode.SwitchCaseNonEnumOnEnumSwitch,
                                "case label is not an enum value for strict switch(enum {0})", switchEnumName);
                        }
                        else
                        {
                            Program.Warning(Maybe.Just(valExpr.Source), ErrorCode.SwitchCaseNonEnumOnEnumSwitch,
                                "case label is not an enum value for switch(enum {0})", switchEnumName);
                        }
                    }
                }

                int val = CalculateConstantExpression(valExpr);

                if (val < 0 || val > 255) Error(c, "Switch case value out of u8 range: " + val);
                if (caseMap.ContainsKey(val)) Error(c, "duplicate case value: " + val);

                caseMap[val] = caseBody;
                caseOrder.Add(val);

                if (val < minVal) minVal = val;
                if (val > maxVal) maxVal = val;
            }

            int range = maxVal - minVal + 1;
            if (range > 256) NYI(expr, "Switch range too large for jump table (max 256)");

            AsmOperand endLabel = MakeUniqueLabel("switch_end");
            AsmOperand defaultLabel = MakeUniqueLabel("switch_default");
            AsmOperand afterTable = MakeUniqueLabel("switch_after_table");
            AsmOperand tableLabel = MakeUniqueLabel("switch_table");

            // Keep the switch exit available while compiling its case bodies and default body.
            BreakLabels.Push(endLabel);

            // A = test (u8)
            CompileIntoA(test);

            // if (A < minVal) default
            if (minVal > 0)
            {
                EmitAsm("CP_IMM", new AsmOperand(minVal & 0xFF, AddressMode.Immediate));
                EmitAsm("JP_C", defaultLabel);
                EmitAsm("SUB_IMM", new AsmOperand(minVal & 0xFF, AddressMode.Immediate)); // A -= minVal
            }

            if (range < 256)
            {
                EmitAsm("CP_IMM", new AsmOperand(range & 0xFF, AddressMode.Immediate));
                EmitAsm("JP_NC", defaultLabel);
            }

            // HL = table + (A*2)
            EmitAsm("LD_L_A");
            EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
            EmitAsm("ADD_HL_HL");

            // DE = &table (IMMEDIATE!)
            EmitAsm("LD_DE_IMM", new AsmOperand(tableLabel.Base.Value, AddressMode.Immediate16));
            EmitAsm("ADD_HL_DE");

            // DE = *(u16*)HL
            EmitAsm("LD_A_HL");
            EmitAsm("LD_E_A");
            EmitAsm("INC_HL");
            EmitAsm("LD_A_HL");
            EmitAsm("LD_D_A");

            EmitAsm("PUSH_DE");
            EmitAsm("RET");

            // ----- Build label table -----
            AsmOperand[] labels = new AsmOperand[range];
            for (int i = 0; i < range; i++) labels[i] = defaultLabel;

            foreach (var kv in caseMap)
            {
                labels[kv.Key - minVal] = MakeUniqueLabel("case_" + kv.Key);
            }

            // ----- Emit case bodies in SOURCE ORDER -----
            foreach (int v in caseOrder)
            {
                EmitLabel(labels[v - minVal]);
                CompileStatement(caseMap[v]);
            }

            // default:
            EmitLabel(defaultLabel);
            if (!defaultStmt.Match(Tag.Empty))
                CompileStatement(defaultStmt);

            EmitLabel(endLabel);
            BreakLabels.Pop();

            // Skip table data
            EmitAsm("JP", afterTable);

            // Jump table data
            EmitLabel(tableLabel);
            for (int i = 0; i < range; i++)
                Emit(Tag.Word, labels[i].Base.Value);

            EmitLabel(afterTable);
            return;
        }


        CompileIntoA(expr);
    }

    // CodeGenerator.cs

    // Emit a conditional jump with constant folding, short-circuit evaluation and byte/word comparisons.
    void CompileJumpIf(bool condition, Expr expr, AsmOperand target)
    {
        expr = FoldConstants(expr);
        if (expr.Match(Tag.Integer, out int constValue))
        {
            if ((constValue != 0) == condition)
                EmitAsm("JP", target);
            return;
        }

        Expr left, right;

        // --- logical operators with short-circuit (&&, ||, !) ---
        Expr la, lb;
        if (expr.Match(Tag.LogicalNot, out la))
        {
            CompileJumpIf(!condition, la, target);
            return;
        }
        if (expr.Match(Tag.LogicalAnd, out la, out lb))
        {
            if (condition)
            {
                // jump if (a && b) is true (short-circuit)
                AsmOperand lSkip = MakeUniqueLabel("and_sc_skip");
                CompileJumpIf(false, la, lSkip);
                CompileJumpIf(true, lb, target);
                EmitLabel(lSkip);
            }
            else
            {
                // jump if (a && b) is false
                CompileJumpIf(false, la, target);
                CompileJumpIf(false, lb, target);
            }
            return;
        }
        if (expr.Match(Tag.LogicalOr, out la, out lb))
        {
            if (condition)
            {
                // jump if (a || b) is true (short-circuit)
                CompileJumpIf(true, la, target);
                CompileJumpIf(true, lb, target);
            }
            else
            {
                // jump if (a || b) is false (both false) — preserve short-circuit
                AsmOperand lEnd = MakeUniqueLabel("or_sc_end");
                CompileJumpIf(true, la, lEnd);
                CompileJumpIf(false, lb, target);
                EmitLabel(lEnd);
            }
            return;
        }

        string tag;
        if (expr.MatchAnyTag(out tag, out left, out right) && IsComparisonTag(tag))
        {
            // Choose signed ordering before selecting the byte or word comparison path.
            bool signedCmp = ShouldUseSignedComparison(left, right);

            if (SizeOf(left) == 1 && SizeOf(right) == 1)
            {
                if ((tag == Tag.Equal || tag == Tag.NotEqual) && left.Match(Tag.Integer, out int lVal0) && ((lVal0 & 0xFF) == 0))
                {
                    CompileIntoA(right);
                    EmitAsm("OR_A");
                    EmitBranch(tag, condition, target);
                    return;
                }

                CompileIntoA(left);
                if (signedCmp)
                    EmitAsm("XOR_IMM", new AsmOperand(0x80, AddressMode.Immediate));

                if (right.Match(Tag.Integer, out int rVal))
                {
                    int imm8 = rVal & 0xFF;
                    if (signedCmp) imm8 ^= 0x80;
                    if (!signedCmp && imm8 == 0 && (tag == Tag.Equal || tag == Tag.NotEqual))
                    {
                        EmitAsm("OR_A");
                    }
                    else
                    {
                        EmitAsm("CP_IMM", new AsmOperand(imm8, AddressMode.Immediate));
                    }
                }
                else
                {
                    EmitAsm("PUSH_AF"); // save Left(A)
                    CompileIntoA(right); // A = Right
                    if (signedCmp)
                        EmitAsm("XOR_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                    EmitAsm("LD_B_A"); // B = Right
                    EmitAsm("POP_AF"); // restore Left(A)
                    EmitAsm("CP_B"); // compare Left vs Right
                }

                EmitBranch(tag, condition, target);
                return;
            }

            // --- 16bit eq/neq with constant 0: use H|L test (Z-only) ---
            if (tag == Tag.Equal || tag == Tag.NotEqual)
            {
                if (right.Match(Tag.Integer, out int rVal16) && ((rVal16 & 0xFFFF) == 0) && SizeOf(left) == 2)
                {
                    CompileIntoHL(left);
                    EmitAsm("LD_A_H");
                    EmitAsm("OR_L");
                    EmitAsm(condition ? (tag == Tag.Equal ? "JP_Z" : "JP_NZ")
                                      : (tag == Tag.Equal ? "JP_NZ" : "JP_Z"), target);
                    return;
                }
                if (left.Match(Tag.Integer, out int lVal16) && ((lVal16 & 0xFFFF) == 0) && SizeOf(right) == 2)
                {
                    CompileIntoHL(right);
                    EmitAsm("LD_A_H");
                    EmitAsm("OR_L");
                    EmitAsm(condition ? (tag == Tag.Equal ? "JP_Z" : "JP_NZ")
                                      : (tag == Tag.Equal ? "JP_NZ" : "JP_Z"), target);
                    return;
                }
            }

            Load16BitCompareOperands(left, right);

            // Signed relational compare:
            // transform to unsigned by flipping the sign bit on both operands.
            if (signedCmp && tag != Tag.Equal && tag != Tag.NotEqual)
            {
                EmitAsm("LD_A_H");
                EmitAsm("XOR_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                EmitAsm("LD_H_A");
                EmitAsm("LD_A_D");
                EmitAsm("XOR_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                EmitAsm("LD_D_A");
            }

            EmitAsm("LD_A_H");
            EmitAsm("CP_D");

            if (tag == Tag.Equal || tag == Tag.NotEqual)
            {

                EmitAsm("LD_A_H"); EmitAsm("XOR_D"); EmitAsm("LD_B_A");
                EmitAsm("LD_A_L"); EmitAsm("XOR_E"); EmitAsm("OR_B");
                EmitAsm(condition ? (tag == Tag.Equal ? "JP_Z" : "JP_NZ")
                                  : (tag == Tag.Equal ? "JP_NZ" : "JP_Z"), target);
                return;
            }
            else
            {
                AsmOperand skip = MakeUniqueLabel("cmp16_skip");
                EmitAsm("JP_NZ", skip);
                EmitAsm("LD_A_L"); EmitAsm("CP_E");
                EmitLabel(skip);

                EmitBranch(tag, condition, target);
                return;
            }
        }

        if (SizeOf(expr) == 2)
        {
            CompileIntoHL(expr);
            EmitAsm("LD_A_H");
            EmitAsm("OR_L");
            EmitAsm(condition ? "JP_NZ" : "JP_Z", target);
            return;
        }

        CompileIntoA(expr);
        EmitAsm("OR_A");
        EmitAsm(condition ? "JP_NZ" : "JP_Z", target);
    }

    bool IsComparisonTag(string tag)
    {
        return tag == Tag.Equal || tag == Tag.NotEqual ||
               tag == Tag.LessThan || tag == Tag.GreaterThan ||
               tag == Tag.LessThanOrEqual || tag == Tag.GreaterThanOrEqual;
    }

    // Translate comparison flags into jumps; strict greater-than requires both carry and zero to be clear.
    void EmitBranch(string tag, bool condition, AsmOperand target)
    {

        // Equal (Z=1)
        // Less (C=1) (A < n)

        string opcode = "";

        if (tag == Tag.Equal) opcode = condition ? "JP_Z" : "JP_NZ";
        else if (tag == Tag.NotEqual) opcode = condition ? "JP_NZ" : "JP_Z";
        else if (tag == Tag.LessThan) opcode = condition ? "JP_C" : "JP_NC"; // Carry set if A < n
        else if (tag == Tag.GreaterThanOrEqual) opcode = condition ? "JP_NC" : "JP_C";
        else if (tag == Tag.GreaterThan)
        {
            // A > n <=> n < A <=> CP n, A (Reverse)

            // A <= n is (A < n) or (A == n) -> C=1 or Z=1
            // A > n is C=0 and Z=0


            if (condition)
            {
                // if (A > B) goto target
                AsmOperand noJump = MakeUniqueLabel("no_gt");
                EmitAsm("JP_C", noJump); // A < B
                EmitAsm("JP_Z", noJump); // A == B
                EmitAsm("JP", target);
                EmitLabel(noJump);
            }
            else
            {
                // if (A <= B) goto target
                EmitAsm("JP_C", target);
                EmitAsm("JP_Z", target);
            }
            return;
        }
        else if (tag == Tag.LessThanOrEqual)
        {
            // A <= n -> C=1 or Z=1
            if (condition)
            {
                EmitAsm("JP_C", target);
                EmitAsm("JP_Z", target);
            }
            else
            {
                // A > n
                AsmOperand noJump = MakeUniqueLabel("no_le");
                EmitAsm("JP_C", noJump);
                EmitAsm("JP_Z", noJump);
                EmitAsm("JP", target);
                EmitLabel(noJump);
            }
            return;
        }

        if (opcode != "") EmitAsm(opcode, target);
    }

    
    // C2) Mixed u8/u16 comparisons: compile to boolean (A=0/1) using the same width decision
    // as conditional branches. This fixes cases like:
    // u8 b = (u16v >= 128);
    // u8 b = (ptr != 0);
    // which previously used an 8-bit CP path and compared only the low byte.
    void CompileComparisonBoolIntoA(string tag, Expr left, Expr right, FilePosition src)
    {
        AsmOperand lblTrue = MakeUniqueLabel("cmp_t");
        AsmOperand lblEnd  = MakeUniqueLabel("cmp_e");
        Expr cmp = Expr.Make(tag, left, right).WithSource(src);

        // Jump to true label if condition holds (handles 8/16-bit + short-circuit correctly).
        CompileJumpIf(true, cmp, lblTrue);

        // false
        EmitAsm("LD_A_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("JP", lblEnd);

        // true
        EmitLabel(lblTrue);
        EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate));

        EmitLabel(lblEnd);
    }

// Load the left word into HL and the right word into DE, preserving the left across RHS evaluation.
void Load16BitCompareOperands(Expr left, Expr right)
    {
        CompileIntoHL(left);
        if (right.Match(Tag.Integer, out int val))
        {
            EmitAsm("LD_DE_IMM", new AsmOperand(val, AddressMode.Immediate16));
        }
        else
        {
            EmitAsm("PUSH_HL"); CompileIntoHL(right);
            EmitAsm("PUSH_HL"); EmitAsm("POP_DE"); EmitAsm("POP_HL");
        }
    }

    // Branch past the panic helper on success; evaluate an optional failure code only when the assertion fails.
    void CompileRuntimeAssert(Expr origin, Expr[] args)
    {
        if (args == null || args.Length < 1 || args.Length > 2)
        {
            Error(origin, "__assert(cond[, code]) expects 1 or 2 arguments");
            return;
        }

        Expr cond = args[0];
        AsmOperand ok = MakeUniqueLabel("assert_ok");
        CompileJumpIf(true, cond, ok); // cond != 0 -> continue

        if (args.Length >= 2)
            CompileIntoA(args[1]);
        else
            EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate));

        UseRuntimeAssertPanicHelper = true;
        EmitAsm("JP", new AsmOperand("__kq_panic", AddressMode.Absolute));
        EmitLabel(ok);
    }

    // Dispatch recognized intrinsics before ordinary calls, tracking scalar width and aggregate-result storage for the caller.
    void CompileCall(Expr expr)
    {

        // Reset per-call return tracking.
        LastCallReturnsHL = false;
        LastStructReturnAddress = -1;

        string funcName = null;
        if (expr.MatchAny(Tag.Call, out Expr funcExpr, out Expr[] args) && funcExpr.Match(Tag.Name, out funcName))
        {
            // Normalize compatibility aliases to the implementations shared by their canonical intrinsic names.
            if (funcName == "__settile_xy") funcName = "__settilebg";
            else if (funcName == "__vram_copy" || funcName == "__vram_copy_hblank" || funcName == "__vram_copy_dma") funcName = "__vram_memcpy";
            else if (funcName == "__vram_fill" || funcName == "__fill_tilemap") funcName = "__vram_memset";
            else if (funcName == "__farmemcpy") funcName = "__far_memcpy";

            // __bankswitch(bank_u8)
            // MBC5 ROM bank select (writes to 0x2000)
            if (funcName == "__bankswitch")
            {
                if (args.Length != 1) Program.Error("__bankswitch(bank) expects 1 argument");
                CompileIntoA(args[0]);
                if (CurrentFunctionBank == 0)
                {
                    EmitAsm("LD_MEM_A", new AsmOperand(0x2000, AddressMode.Absolute));
                    EmitStoreA(RomBankVar);
                }
                else
                {
                    UseBankSwitchBank0Helper = true;
                    EmitAsm("CALL", new AsmOperand("__kq_bankswitch_bank0", AddressMode.Absolute));
                }
                return;
            }

            
            // __farcall(bank_u8, func_name)
            // Safe far call: always executes bank switch from bank0 thunk.
            if (funcName == "__farcall")
            {
                if (args.Length != 2) Program.Error("__farcall(bank, func) expects 2 arguments");
                if (!args[1].Match(Tag.Name, out string targetName)) Program.Error("__farcall(bank, func) requires 2nd argument to be a function name");

                // A named far call supplies no arguments. Reject data symbols
                // and callbacks that would otherwise read stale argument storage.
                CFunctionInfo targetInfo;
                if (!Functions.TryGetValue(targetName, out targetInfo))
                {
                    Error(expr, "__farcall requires a declared function name");
                    return;
                }
                if ((targetInfo.Parameters ?? Array.Empty<FieldInfo>()).Length != 0)
                {
                    Error(expr, "__farcall requires a callback with no parameters");
                    return;
                }
                Expr bankExpr = FoldConstants(args[0]);
                int targetBank = -1;
                if (targetInfo != null)
                {
                    targetBank = targetInfo.RomBank;
                    PrepareStructReturnDestination(expr, targetInfo.ReturnType, targetInfo, targetName);
                    if (targetInfo.ReturnType != null)
                        LastCallReturnsHL = ReturnsInHL(expr, targetInfo.ReturnType);
                }

                // A known bank permits direct dispatch when safe, otherwise request a fixed-bank thunk.
                if (TryResolveBankExprToConstU8(args[0], out int bankLiteral))
                {
                    int optimizedBank = bankLiteral & 0xFF;
                    bool farcallDirectOK = IsDirectCallBankSafe(optimizedBank, CurrentFunctionBank);
                    if (!farcallDirectOK)
                    {
                        string farcallThunkName = EnsureBank0Thunk(
                            targetName,
                            optimizedBank,
                            isStackCall: targetInfo != null && targetInfo.IsStackCall,
                            stackArgBytes: 0);
                        EmitAsm("CALL", new AsmOperand(farcallThunkName, AddressMode.Absolute));
                        RecordCallEdge(expr, targetName, optimizedBank, "farcall_bank_thunk", viaThunk: true, viaFarcall: true, actualArgSizes: new int[0], expectedArgSizes: new int[0]);
                    }
                    else
                    {
                        if (Program.CheckBankCalls && !InUnsafe && optimizedBank != 0)
                        {
                            AsmOperand ok = MakeUniqueLabel("kq_bank_ok");
                            EmitLoadA(RomBankVar);
                            EmitAsm("CP_IMM", new AsmOperand(optimizedBank & 0xFF, AddressMode.Immediate));
                            EmitAsm("JP_Z", ok);
                            EmitAsm("JP", EnsureCheckTrap());
                            EmitLabel(ok);
                        }

                        EmitAsm("CALL", new AsmOperand(targetName, AddressMode.Absolute));
                        RecordCallEdge(expr, targetName, optimizedBank, "farcall_direct", viaThunk: false, viaFarcall: true, actualArgSizes: new int[0], expectedArgSizes: new int[0]);
                    }
                    return;
                }

                // A = requested bank argument (runtime value)
                CompileIntoA(bankExpr);

                // Always dispatch via bank0 helper thunk.
                string thunkName = EnsureBank0RuntimeFarcallThunk(targetName);
                EmitAsm("CALL", new AsmOperand(thunkName, AddressMode.Absolute));
                RecordCallEdge(expr, targetName, targetBank, "farcall_intrinsic", viaThunk: true, viaFarcall: true, actualArgSizes: new int[0], expectedArgSizes: new int[0]);
                return;
            }
// __bankof(symbol)
            // Returns the ROM bank number where 'symbol' is located (resolved by assembler).
            if (funcName == "__bankof")
            {
                if (args.Length != 1) Program.Error("__bankof(symbol) expects 1 argument");
                if (!args[0].Match(Tag.Name, out string symName)) Program.Error("__bankof(symbol) requires a symbol name argument");
                if (TryGetKnownRomBankForSymbol(symName, out int knownBank))
                {
                    EmitAsm("LD_A_IMM", new AsmOperand(knownBank & 0xFF, AddressMode.Immediate));
                    return;
                }
                EmitAsm("LD_A_IMM", new AsmOperand(symName, ImmediateModifier.Bank));
                return;
            }

            // __cgb_is_cgb() -> A (0 or 1)
            if (funcName == "__cgb_is_cgb")
            {
                if (args.Length != 0) Program.Error("__cgb_is_cgb() expects 0 arguments");
                if (Program.TryGetKnownCgbRuntimeValue(out int knownCgbValue))
                {
                    EmitAsm("LD_A_IMM", new AsmOperand(knownCgbValue, AddressMode.Immediate));
                    return;
                }
                EmitRuntimeCgbCheckCall();
                return;
            }

            // Expose the normalized WRAM bank only for a color-only target.
            if (funcName == "__svbk_get")
            {
                if (args.Length != 0) Error(expr, "__svbk_get() expects 0 arguments");
                if (!Program.IsCgbOnlyTargetRequested())
                    Error(expr, ErrorCode.BankedWramRequiresCgbOnly, "__svbk_get() requires #pragma rom_cgb cgb_only or --cgb=cgb_only");
                EmitReadNormalizedSvbkIntoA();
                return;
            }

            // Select a WRAM bank and return the prior normalized bank; diagnose out-of-range constant requests.
            if (funcName == "__svbk_set")
            {
                if (args.Length != 1) Error(expr, "__svbk_set(bank_u8) expects 1 argument");
                if (!Program.IsCgbOnlyTargetRequested())
                    Error(expr, ErrorCode.BankedWramRequiresCgbOnly, "__svbk_set() requires #pragma rom_cgb cgb_only or --cgb=cgb_only");

                Expr bankExpr = FoldConstants(args[0]);
                if (bankExpr.Match(Tag.Integer, out int bankLiteral) && (bankLiteral < 1 || bankLiteral > 7))
                    Error(expr, ErrorCode.InvalidWramXBank, "__svbk_set() bank must be in range 1..7 (got {0})", bankLiteral);

                EmitReadNormalizedSvbkIntoA();
                EmitAsm("LD_B_A");
                CompileIntoA(bankExpr);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x70, AddressMode.HighMem));
                EmitAsm("LD_A_B");
                return;
            }

            // __cgb_safe_set_* (safe CGB-only register write wrappers)
            if (TryGetCgbSafeRegister(funcName, out int cgbOffset, out string cgbReg))
            {
                if (args.Length != 1) Program.Error(funcName + "(value_u8) expects 1 argument");

                if (IsKnownCgbRuntimeTrue())
                {
                    CompileIntoA(args[0]);
                    if (cgbOffset == 0x4F)
                        EmitAsm("AND_IMM", new AsmOperand(0x01, AddressMode.Immediate));
                    EmitAsm("LDH_MEM_A", new AsmOperand(cgbOffset, AddressMode.HighMem));
                    CgbGuardedWriteCount++;
                    CgbGuardedRegisters.Add(cgbReg);
                    return;
                }

                // Preserve the user value while calling runtime detector.
                CompileIntoA(args[0]);
                EmitAsm("LD_B_A");
                EmitRuntimeCgbCheckCall(); // A = 0/1
                AsmOperand skip = MakeUniqueLabel("cgbsafe_skip");
                EmitAsm("OR_A");
                EmitAsm("JR_Z", skip);
                EmitAsm("LD_A_B");
                if (cgbOffset == 0x4F)
                    EmitAsm("AND_IMM", new AsmOperand(0x01, AddressMode.Immediate));
                EmitAsm("LDH_MEM_A", new AsmOperand(cgbOffset, AddressMode.HighMem));
                EmitLabel(skip);

                CgbGuardedWriteCount++;
                CgbGuardedRegisters.Add(cgbReg);
                return;
            }

            // __wait_vblank()
            // Waits for the next VBlank start (LY crossing to >=144).
            if (funcName == "__wait_vblank")
            {
                if (args.Length != 0) Error(expr, "__wait_vblank() expects 0 arguments");

                AsmOperand wbRet = MakeUniqueLabel("waitvb_ret");
                AsmOperand wbExitVblank = MakeUniqueLabel("waitvb_exit");
                AsmOperand wbEnterVblank = MakeUniqueLabel("waitvb_enter");

                // If LCD is off, return immediately.
                EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
                EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                EmitAsm("JR_Z", wbRet);

                // while (LY >= 144) {}
                EmitLabel(wbExitVblank);
                EmitAsm("LDH_A_MEM", new AsmOperand(0x44, AddressMode.HighMem)); // LY
                EmitAsm("CP_IMM", new AsmOperand(144, AddressMode.Immediate));
                EmitAsm("JR_C", wbEnterVblank);
                EmitAsm("JR", wbExitVblank);

                // while (LY < 144) {}
                EmitLabel(wbEnterVblank);
                EmitAsm("LDH_A_MEM", new AsmOperand(0x44, AddressMode.HighMem)); // LY
                EmitAsm("CP_IMM", new AsmOperand(144, AddressMode.Immediate));
                EmitAsm("JR_C", wbEnterVblank);

                EmitLabel(wbRet);
                return;
            }

            // __wait_ly(target_u8)
            // Waits until LY == target while LCD is on.
            if (funcName == "__wait_ly")
            {
                if (args.Length != 1) Error(expr, "__wait_ly(target) expects 1 argument");

                CompileIntoA(args[0]);
                EmitAsm("LD_B_A");

                AsmOperand wlyLoop = MakeUniqueLabel("waitly_loop");
                AsmOperand wlyDone = MakeUniqueLabel("waitly_done");

                // If LCD is off, LY is not meaningful; return.
                EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
                EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                EmitAsm("JR_Z", wlyDone);

                EmitLabel(wlyLoop);
                EmitAsm("LDH_A_MEM", new AsmOperand(0x44, AddressMode.HighMem)); // LY
                EmitAsm("CP_B");
                EmitAsm("JR_NZ", wlyLoop);
                EmitLabel(wlyDone);
                return;
            }

            // Write both background coordinates immediately, synchronize current/pending state and clear its dirty bit.
            if (funcName == "__scroll_bg_set")
            {
                if (args.Length != 2) Error(expr, "__scroll_bg_set(scx, scy) expects 2 arguments");
                EnsureScrollState();

                CompileIntoA(args[0]);
                EmitPushAAsWord();
                CompileIntoA(args[1]);
                EmitAsm("LD_B_A");
                EmitAsm("POP_HL");

                EmitAsm("LD_A_L");
                EmitStoreA(ScrollBgXCurVar);
                EmitStoreA(ScrollBgXNextVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x43, AddressMode.HighMem)); // SCX

                EmitAsm("LD_A_B");
                EmitStoreA(ScrollBgYCurVar);
                EmitStoreA(ScrollBgYNextVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x42, AddressMode.HighMem)); // SCY

                EmitLoadA(ScrollDirtyVar);
                EmitAsm("AND_IMM", new AsmOperand(0xFF ^ ScrollDirtyBgMask, AddressMode.Immediate));
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Commit background X now and discard any queued Y change by resynchronizing it with current Y.
            if (funcName == "__scroll_bg_x_set")
            {
                if (args.Length != 1) Error(expr, "__scroll_bg_x_set(scx) expects 1 argument");
                EnsureScrollState();

                CompileIntoA(args[0]);
                EmitStoreA(ScrollBgXCurVar);
                EmitStoreA(ScrollBgXNextVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x43, AddressMode.HighMem)); // SCX
                EmitLoadA(ScrollBgYCurVar);
                EmitStoreA(ScrollBgYNextVar);
                EmitLoadA(ScrollDirtyVar);
                EmitAsm("AND_IMM", new AsmOperand(0xFF ^ ScrollDirtyBgMask, AddressMode.Immediate));
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Commit background Y now and discard any queued X change by resynchronizing it with current X.
            if (funcName == "__scroll_bg_y_set")
            {
                if (args.Length != 1) Error(expr, "__scroll_bg_y_set(scy) expects 1 argument");
                EnsureScrollState();

                CompileIntoA(args[0]);
                EmitStoreA(ScrollBgYCurVar);
                EmitStoreA(ScrollBgYNextVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x42, AddressMode.HighMem)); // SCY
                EmitLoadA(ScrollBgXCurVar);
                EmitStoreA(ScrollBgXNextVar);
                EmitLoadA(ScrollDirtyVar);
                EmitAsm("AND_IMM", new AsmOperand(0xFF ^ ScrollDirtyBgMask, AddressMode.Immediate));
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Read the tracked current background X, excluding any pending buffered value.
            if (funcName == "__scroll_bg_x_get")
            {
                if (args.Length != 0) Error(expr, "__scroll_bg_x_get() expects 0 arguments");
                EnsureScrollState();
                EmitLoadA(ScrollBgXCurVar);
                return;
            }

            // Read the tracked current background Y, excluding any pending buffered value.
            if (funcName == "__scroll_bg_y_get")
            {
                if (args.Length != 0) Error(expr, "__scroll_bg_y_get() expects 0 arguments");
                EnsureScrollState();
                EmitLoadA(ScrollBgYCurVar);
                return;
            }

            // Commit both window register coordinates and synchronize the current and pending pairs.
            if (funcName == "__scroll_win_set")
            {
                if (args.Length != 2) Error(expr, "__scroll_win_set(wx, wy) expects 2 arguments");
                EnsureScrollState();

                CompileIntoA(args[0]);
                EmitPushAAsWord();
                CompileIntoA(args[1]);
                EmitAsm("LD_B_A");
                EmitAsm("POP_HL");

                EmitAsm("LD_A_L");
                EmitStoreA(ScrollWinXCurVar);
                EmitStoreA(ScrollWinXNextVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x4B, AddressMode.HighMem)); // WX

                EmitAsm("LD_A_B");
                EmitStoreA(ScrollWinYCurVar);
                EmitStoreA(ScrollWinYNextVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x4A, AddressMode.HighMem)); // WY

                EmitLoadA(ScrollDirtyVar);
                EmitAsm("AND_IMM", new AsmOperand(0xFF ^ ScrollDirtyWinMask, AddressMode.Immediate));
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Commit window X and reset pending Y to its current value before clearing the window dirty bit.
            if (funcName == "__scroll_win_x_set")
            {
                if (args.Length != 1) Error(expr, "__scroll_win_x_set(wx) expects 1 argument");
                EnsureScrollState();

                CompileIntoA(args[0]);
                EmitStoreA(ScrollWinXCurVar);
                EmitStoreA(ScrollWinXNextVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x4B, AddressMode.HighMem)); // WX
                EmitLoadA(ScrollWinYCurVar);
                EmitStoreA(ScrollWinYNextVar);
                EmitLoadA(ScrollDirtyVar);
                EmitAsm("AND_IMM", new AsmOperand(0xFF ^ ScrollDirtyWinMask, AddressMode.Immediate));
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Commit window Y and reset pending X to its current value before clearing the window dirty bit.
            if (funcName == "__scroll_win_y_set")
            {
                if (args.Length != 1) Error(expr, "__scroll_win_y_set(wy) expects 1 argument");
                EnsureScrollState();

                CompileIntoA(args[0]);
                EmitStoreA(ScrollWinYCurVar);
                EmitStoreA(ScrollWinYNextVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x4A, AddressMode.HighMem)); // WY
                EmitLoadA(ScrollWinXCurVar);
                EmitStoreA(ScrollWinXNextVar);
                EmitLoadA(ScrollDirtyVar);
                EmitAsm("AND_IMM", new AsmOperand(0xFF ^ ScrollDirtyWinMask, AddressMode.Immediate));
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Return the tracked current WX register value.
            if (funcName == "__scroll_win_x_get")
            {
                if (args.Length != 0) Error(expr, "__scroll_win_x_get() expects 0 arguments");
                EnsureScrollState();
                EmitLoadA(ScrollWinXCurVar);
                return;
            }

            // Return the tracked current WY register value.
            if (funcName == "__scroll_win_y_get")
            {
                if (args.Length != 0) Error(expr, "__scroll_win_y_get() expects 0 arguments");
                EnsureScrollState();
                EmitLoadA(ScrollWinYCurVar);
                return;
            }

            // Queue both background coordinates and mark them for a later scroll flush.
            if (funcName == "__scroll_bg_set_buffered")
            {
                if (args.Length != 2) Error(expr, "__scroll_bg_set_buffered(scx, scy) expects 2 arguments");
                EnsureScrollState();

                CompileIntoA(args[0]);
                EmitPushAAsWord();
                CompileIntoA(args[1]);
                EmitAsm("LD_B_A");
                EmitAsm("POP_HL");

                EmitAsm("LD_A_L");
                EmitStoreA(ScrollBgXNextVar);
                EmitAsm("LD_A_B");
                EmitStoreA(ScrollBgYNextVar);
                EmitLoadA(ScrollDirtyVar);
                EmitAsm("OR_IMM", new AsmOperand(ScrollDirtyBgMask, AddressMode.Immediate));
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Replace pending background X while retaining the queued Y coordinate.
            if (funcName == "__scroll_bg_x_set_buffered")
            {
                if (args.Length != 1) Error(expr, "__scroll_bg_x_set_buffered(scx) expects 1 argument");
                EnsureScrollState();

                CompileIntoA(args[0]);
                EmitStoreA(ScrollBgXNextVar);
                EmitLoadA(ScrollDirtyVar);
                EmitAsm("OR_IMM", new AsmOperand(ScrollDirtyBgMask, AddressMode.Immediate));
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Replace pending background Y while retaining the queued X coordinate.
            if (funcName == "__scroll_bg_y_set_buffered")
            {
                if (args.Length != 1) Error(expr, "__scroll_bg_y_set_buffered(scy) expects 1 argument");
                EnsureScrollState();

                CompileIntoA(args[0]);
                EmitStoreA(ScrollBgYNextVar);
                EmitLoadA(ScrollDirtyVar);
                EmitAsm("OR_IMM", new AsmOperand(ScrollDirtyBgMask, AddressMode.Immediate));
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Queue both window coordinates without writing WX or WY yet.
            if (funcName == "__scroll_win_set_buffered")
            {
                if (args.Length != 2) Error(expr, "__scroll_win_set_buffered(wx, wy) expects 2 arguments");
                EnsureScrollState();

                CompileIntoA(args[0]);
                EmitPushAAsWord();
                CompileIntoA(args[1]);
                EmitAsm("LD_B_A");
                EmitAsm("POP_HL");

                EmitAsm("LD_A_L");
                EmitStoreA(ScrollWinXNextVar);
                EmitAsm("LD_A_B");
                EmitStoreA(ScrollWinYNextVar);
                EmitLoadA(ScrollDirtyVar);
                EmitAsm("OR_IMM", new AsmOperand(ScrollDirtyWinMask, AddressMode.Immediate));
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Update pending window X and mark the window pair dirty.
            if (funcName == "__scroll_win_x_set_buffered")
            {
                if (args.Length != 1) Error(expr, "__scroll_win_x_set_buffered(wx) expects 1 argument");
                EnsureScrollState();

                CompileIntoA(args[0]);
                EmitStoreA(ScrollWinXNextVar);
                EmitLoadA(ScrollDirtyVar);
                EmitAsm("OR_IMM", new AsmOperand(ScrollDirtyWinMask, AddressMode.Immediate));
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Update pending window Y and mark the window pair dirty.
            if (funcName == "__scroll_win_y_set_buffered")
            {
                if (args.Length != 1) Error(expr, "__scroll_win_y_set_buffered(wy) expects 1 argument");
                EnsureScrollState();

                CompileIntoA(args[0]);
                EmitStoreA(ScrollWinYNextVar);
                EmitLoadA(ScrollDirtyVar);
                EmitAsm("OR_IMM", new AsmOperand(ScrollDirtyWinMask, AddressMode.Immediate));
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Commit dirty background/window pairs to tracked state and hardware, then clear all scroll dirty flags.
            if (funcName == "__scroll_flush")
            {
                if (args.Length != 0) Error(expr, "__scroll_flush() expects 0 arguments");
                EnsureScrollState();

                AsmOperand flushSkipBg = MakeUniqueLabel("scroll_flush_skip_bg");
                AsmOperand flushSkipWin = MakeUniqueLabel("scroll_flush_skip_win");

                EmitLoadA(ScrollDirtyVar);
                EmitAsm("AND_IMM", new AsmOperand(ScrollDirtyBgMask, AddressMode.Immediate));
                EmitAsm("JR_Z", flushSkipBg);
                EmitLoadA(ScrollBgXNextVar);
                EmitStoreA(ScrollBgXCurVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x43, AddressMode.HighMem)); // SCX
                EmitLoadA(ScrollBgYNextVar);
                EmitStoreA(ScrollBgYCurVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x42, AddressMode.HighMem)); // SCY
                EmitLabel(flushSkipBg);

                EmitLoadA(ScrollDirtyVar);
                EmitAsm("AND_IMM", new AsmOperand(ScrollDirtyWinMask, AddressMode.Immediate));
                EmitAsm("JR_Z", flushSkipWin);
                EmitLoadA(ScrollWinXNextVar);
                EmitStoreA(ScrollWinXCurVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x4B, AddressMode.HighMem)); // WX
                EmitLoadA(ScrollWinYNextVar);
                EmitStoreA(ScrollWinYCurVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x4A, AddressMode.HighMem)); // WY
                EmitLabel(flushSkipWin);

                EmitAsm("XOR_A");
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Enable the LCDC window bit and mirror that bit in the tracked visibility state.
            if (funcName == "__scroll_win_show")
            {
                if (args.Length != 0) Error(expr, "__scroll_win_show() expects 0 arguments");
                EnsureScrollState();

                EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
                EmitAsm("OR_IMM", new AsmOperand(0x20, AddressMode.Immediate));
                EmitAsm("LDH_MEM_A", new AsmOperand(0x40, AddressMode.HighMem));
                EmitAsm("LD_A_IMM", new AsmOperand(0x20, AddressMode.Immediate));
                EmitStoreA(ScrollWinVisibleVar);
                return;
            }

            // Clear the LCDC window bit and the tracked visibility state.
            if (funcName == "__scroll_win_hide")
            {
                if (args.Length != 0) Error(expr, "__scroll_win_hide() expects 0 arguments");
                EnsureScrollState();

                EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
                EmitAsm("AND_IMM", new AsmOperand(0xDF, AddressMode.Immediate));
                EmitAsm("LDH_MEM_A", new AsmOperand(0x40, AddressMode.HighMem));
                EmitAsm("XOR_A");
                EmitStoreA(ScrollWinVisibleVar);
                return;
            }

            // Add byte-sized deltas to current background coordinates, commit immediately and discard pending changes.
            if (funcName == "__scroll_bg_add")
            {
                if (args.Length != 2) Error(expr, "__scroll_bg_add(dx, dy) expects 2 arguments");
                EnsureScrollState();

                CompileIntoA(args[0]);
                EmitPushAAsWord();
                CompileIntoA(args[1]);
                EmitAsm("LD_C_A");
                EmitAsm("POP_HL");
                EmitAsm("LD_B_L");

                EmitLoadA(ScrollBgXCurVar);
                EmitAsm("ADD_B");
                EmitStoreA(ScrollBgXCurVar);
                EmitStoreA(ScrollBgXNextVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x43, AddressMode.HighMem)); // SCX

                EmitLoadA(ScrollBgYCurVar);
                EmitAsm("ADD_C");
                EmitStoreA(ScrollBgYCurVar);
                EmitStoreA(ScrollBgYNextVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x42, AddressMode.HighMem)); // SCY

                EmitLoadA(ScrollDirtyVar);
                EmitAsm("AND_IMM", new AsmOperand(0xFF ^ ScrollDirtyBgMask, AddressMode.Immediate));
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Add byte-sized deltas to current window coordinates, then synchronize pending state and hardware.
            if (funcName == "__scroll_win_add")
            {
                if (args.Length != 2) Error(expr, "__scroll_win_add(dx, dy) expects 2 arguments");
                EnsureScrollState();

                CompileIntoA(args[0]);
                EmitPushAAsWord();
                CompileIntoA(args[1]);
                EmitAsm("LD_C_A");
                EmitAsm("POP_HL");
                EmitAsm("LD_B_L");

                EmitLoadA(ScrollWinXCurVar);
                EmitAsm("ADD_B");
                EmitStoreA(ScrollWinXCurVar);
                EmitStoreA(ScrollWinXNextVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x4B, AddressMode.HighMem)); // WX

                EmitLoadA(ScrollWinYCurVar);
                EmitAsm("ADD_C");
                EmitStoreA(ScrollWinYCurVar);
                EmitStoreA(ScrollWinYNextVar);
                EmitAsm("LDH_MEM_A", new AsmOperand(0x4A, AddressMode.HighMem)); // WY

                EmitLoadA(ScrollDirtyVar);
                EmitAsm("AND_IMM", new AsmOperand(0xFF ^ ScrollDirtyWinMask, AddressMode.Immediate));
                EmitStoreA(ScrollDirtyVar);
                return;
            }

            // Request the split-scroll helpers and dispatch the queue reset operation.
            if (funcName == "__scroll_split_reset")
            {
                if (args.Length != 0) Error(expr, "__scroll_split_reset() expects 0 arguments");
                EnsureScrollState();
                UseScrollSplitHelpers = true;
                EmitAsm("CALL", new AsmOperand("__kq_scroll_split_reset_core", AddressMode.Absolute));
                return;
            }

            // Stage one background-only split entry in shared helper arguments, with unused window fields cleared.
            if (funcName == "__scroll_split_push")
            {
                if (args.Length != 3) Error(expr, "__scroll_split_push(ly, scx, scy) expects 3 arguments");
                EnsureScrollState();
                UseScrollSplitHelpers = true;

                CompileIntoA(args[0]);
                EmitStoreA(ScrollSplitTmpLyVar);
                CompileIntoA(args[1]);
                EmitStoreA(ScrollSplitTmpScxVar);
                CompileIntoA(args[2]);
                EmitStoreA(ScrollSplitTmpScyVar);
                EmitAsm("XOR_A");
                EmitStoreA(ScrollSplitTmpWxVar);
                EmitStoreA(ScrollSplitTmpWyVar);
                EmitAsm("LD_A_IMM", new AsmOperand(ScrollSplitFlagUseBg, AddressMode.Immediate));
                EmitStoreA(ScrollSplitTmpFlagsVar);
                EmitAsm("CALL", new AsmOperand("__kq_scroll_split_push_core", AddressMode.Absolute));
                return;
            }

            // Stage a complete split entry and warn about unsupported constant flag bits before helper dispatch.
            if (funcName == "__scroll_split_push_ex")
            {
                if (args.Length != 6) Error(expr, "__scroll_split_push_ex(ly, scx, scy, wx, wy, flags) expects 6 arguments");
                EnsureScrollState();
                UseScrollSplitHelpers = true;

                if (TryGetU8Const(args[5], out int splitFlagsConst))
                {
                    int unknownBits = splitFlagsConst & ~ScrollSplitFlagMask;
                    if (unknownBits != 0)
                        Warning(args[5], string.Format("__scroll_split_push_ex() ignores unknown flag bits 0x{0:X2}", unknownBits));
                }

                CompileIntoA(args[0]);
                EmitStoreA(ScrollSplitTmpLyVar);
                CompileIntoA(args[1]);
                EmitStoreA(ScrollSplitTmpScxVar);
                CompileIntoA(args[2]);
                EmitStoreA(ScrollSplitTmpScyVar);
                CompileIntoA(args[3]);
                EmitStoreA(ScrollSplitTmpWxVar);
                CompileIntoA(args[4]);
                EmitStoreA(ScrollSplitTmpWyVar);
                CompileIntoA(args[5]);
                EmitStoreA(ScrollSplitTmpFlagsVar);
                EmitAsm("CALL", new AsmOperand("__kq_scroll_split_push_core", AddressMode.Absolute));
                return;
            }

            // Dispatch the staged split-scroll configuration to the shared commit helper.
            if (funcName == "__scroll_split_commit")
            {
                if (args.Length != 0) Error(expr, "__scroll_split_commit() expects 0 arguments");
                EnsureScrollState();
                UseScrollSplitHelpers = true;
                EmitAsm("CALL", new AsmOperand("__kq_scroll_split_commit_core", AddressMode.Absolute));
                return;
            }

            // __critical_enter() -> token_u8
            // Returns a software token for __critical_leave(token).
            if (funcName == "__critical_enter")
            {
                if (args.Length != 0) Error(expr, "__critical_enter() expects 0 arguments");

                EmitLoadA(CriticalDepthVar);
                EmitAsm("LD_B_A"); // token = old depth
                EmitAsm("INC_A");
                EmitStoreA(CriticalDepthVar);
                EmitAsm("DI");
                EmitAsm("LD_A_B");
                return;
            }

            // __critical_leave(token_u8)
            if (funcName == "__critical_leave")
            {
                if (args.Length != 1) Error(expr, "__critical_leave(token) expects 1 argument");

                CompileIntoA(args[0]);
                EmitAsm("LD_B_A"); // token

                AsmOperand clAfterDec = MakeUniqueLabel("critleave_afterdec");
                AsmOperand clDone = MakeUniqueLabel("critleave_done");

                EmitLoadA(CriticalDepthVar);
                EmitAsm("OR_A");
                EmitAsm("JR_Z", clAfterDec);
                EmitAsm("DEC_A");
                EmitStoreA(CriticalDepthVar);
                EmitLabel(clAfterDec);

                // Re-enable only for outermost token (token==0).
                EmitAsm("LD_A_B");
                EmitAsm("OR_A");
                EmitAsm("JR_NZ", clDone);
                EmitAsm("EI");
                EmitLabel(clDone);
                return;
            }

            // Copy one 160-byte, page-aligned OAM source and return after DMA completes.
            if (funcName == "__oam_dma")
            {
                if (args.Length != 1) Error(expr, "__oam_dma(src_ptr) expects 1 argument");

                CompileIntoHL(args[0]); // Evaluate once; hardware uses only the high byte.
                // Mask interrupt sources without changing IME, including calls made inside an ISR.
                EmitAsm("LD_A_MEM", new AsmOperand(0xFFFF, AddressMode.Absolute));
                EmitAsm("PUSH_AF");
                EmitAsm("XOR_A");
                EmitAsm("LD_MEM_A", new AsmOperand(0xFFFF, AddressMode.Absolute));
                // LDH [FF46],A; LD B,40; DEC B; JR NZ,-3; RET.
                // Initialize on every call, so no startup copy or readiness flag is needed.
                byte[] dmaStub = { 0xE0, 0x46, 0x06, 0x28, 0x05, 0x20, 0xFD, 0xC9 };
                for (int i = 0; i < dmaStub.Length; i++)
                {
                    EmitAsm("LD_A_IMM", new AsmOperand(dmaStub[i], AddressMode.Immediate));
                    EmitAsm("LDH_MEM_A", new AsmOperand((OamDmaStubAddr + i) & 0xFF, AddressMode.HighMem));
                }
                EmitAsm("LD_A_H");
                // CALL pushes before DMA starts; RET reads the stack only after the wait.
                EmitAsm("CALL", new AsmOperand(OamDmaStubAddr, AddressMode.Absolute));
                EmitAsm("POP_AF");
                EmitAsm("LD_MEM_A", new AsmOperand(0xFFFF, AddressMode.Absolute));
                return;
            }

            // __vram_memcpy(dst_ptr, src_ptr, len_u16)
            // Safe VRAM copy: if LCD is on, each byte write is gated by STAT mode 0/1.
            if (funcName == "__vram_memcpy")
            {
                if (args.Length != 3) Error(expr, "__vram_memcpy(dst, src, len) expects 3 arguments");

                // Evaluate args left-to-right.
                CompileIntoHL(args[0]); EmitAsm("PUSH_HL"); // dst
                CompileIntoHL(args[1]); EmitAsm("PUSH_HL"); // src
                // Fold the length for small-copy selection after saving both addresses across length evaluation.
                Expr vramCopyLen = FoldConstants(args[2]);
                EmitLengthExprIntoBC(vramCopyLen);
                EmitAsm("POP_DE"); // src
                EmitAsm("POP_HL"); // dst

                if (vramCopyLen.Match(Tag.Integer, out int vramCopyLenConst) &&
                    TryEmitVramMemcpyConstCount(true, vramCopyLenConst, "vramcpy_small"))
                    return;

                AsmOperand vcFast = MakeUniqueLabel("vramcpy_fast");
                AsmOperand vcSafeLoop = MakeUniqueLabel("vramcpy_safe_loop");
                AsmOperand vcSafeWait = MakeUniqueLabel("vramcpy_safe_wait");
                AsmOperand vcFastLoop = MakeUniqueLabel("vramcpy_fast_loop");
                AsmOperand vcDone = MakeUniqueLabel("vramcpy_done");

                EmitAsm("LD_A_B");
                EmitAsm("OR_C");
                EmitAsm("JR_Z", vcDone);

                // LCD off => plain memcpy loop.
                EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
                EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                EmitAsm("JR_Z", vcFast);

                EmitLabel(vcSafeLoop);
                EmitAsm("LD_A_B");
                EmitAsm("OR_C");
                EmitAsm("JR_Z", vcDone);
                EmitAsm("DI");
                EmitLabel(vcSafeWait);
                EmitAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem)); // STAT
                EmitAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate));
                EmitAsm("JR_NZ", vcSafeWait);
                EmitAsm("LD_A_DE");
                EmitAsm("LDI_HL_A");
                EmitAsm("EI");
                EmitAsm("INC_DE");
                EmitAsm("DEC_BC");
                EmitAsm("JR", vcSafeLoop);

                EmitLabel(vcFast);
                EmitLabel(vcFastLoop);
                EmitAsm("LD_A_B");
                EmitAsm("OR_C");
                EmitAsm("JR_Z", vcDone);
                EmitAsm("LD_A_DE");
                EmitAsm("LDI_HL_A");
                EmitAsm("INC_DE");
                EmitAsm("DEC_BC");
                EmitAsm("JR", vcFastLoop);

                EmitLabel(vcDone);
                return;
            }

            // __vram_memcpy_unsafe(dst_ptr, src_ptr, len_u16)
            // Fast path for ISR/VBlank-only contexts.
            if (funcName == "__vram_memcpy_unsafe")
            {
                if (args.Length != 3) Error(expr, "__vram_memcpy_unsafe(dst, src, len) expects 3 arguments");

                CompileIntoHL(args[0]); EmitAsm("PUSH_HL"); // dst
                CompileIntoHL(args[1]); EmitAsm("PUSH_HL"); // src
                // Use the same count specialization without emitting LCD-mode waits.
                Expr vramCopyUnsafeLen = FoldConstants(args[2]);
                EmitLengthExprIntoBC(vramCopyUnsafeLen);
                EmitAsm("POP_DE"); // src
                EmitAsm("POP_HL"); // dst

                if (vramCopyUnsafeLen.Match(Tag.Integer, out int vramCopyUnsafeLenConst) &&
                    TryEmitVramMemcpyConstCount(false, vramCopyUnsafeLenConst, "vramcpyu_small"))
                    return;

                AsmOperand vcuLoop = MakeUniqueLabel("vramcpyu_loop");
                AsmOperand vcuDone = MakeUniqueLabel("vramcpyu_done");
                EmitLabel(vcuLoop);
                EmitAsm("LD_A_B");
                EmitAsm("OR_C");
                EmitAsm("JR_Z", vcuDone);
                EmitAsm("LD_A_DE");
                EmitAsm("LDI_HL_A");
                EmitAsm("INC_DE");
                EmitAsm("DEC_BC");
                EmitAsm("JR", vcuLoop);
                EmitLabel(vcuDone);
                return;
            }

            // __vram_memset(dst_ptr, value_u8, len_u16)
            // Safe VRAM fill: if LCD is on, each byte write is gated by STAT mode 0/1.
            if (funcName == "__vram_memset")
            {
                if (args.Length != 3) Error(expr, "__vram_memset(dst, value, len) expects 3 arguments");

                CompileIntoHL(args[0]); EmitAsm("PUSH_HL"); // dst
                CompileIntoA(args[1]); EmitAsm("LD_D_A"); // value
                // Select a small constant fill when possible; the general path uses BC as count and D as value.
                Expr vramSetLen = FoldConstants(args[2]);
                EmitLengthExprIntoBC(vramSetLen);
                EmitAsm("POP_HL"); // dst

                if (vramSetLen.Match(Tag.Integer, out int vramSetLenConst) &&
                    TryEmitVramMemsetConstCount(true, vramSetLenConst, "vramset_small"))
                    return;

                AsmOperand vsFast = MakeUniqueLabel("vramset_fast");
                AsmOperand vsSafeLoop = MakeUniqueLabel("vramset_safe_loop");
                AsmOperand vsSafeWait = MakeUniqueLabel("vramset_safe_wait");
                AsmOperand vsFastLoop = MakeUniqueLabel("vramset_fast_loop");
                AsmOperand vsDone = MakeUniqueLabel("vramset_done");

                EmitAsm("LD_A_B");
                EmitAsm("OR_C");
                EmitAsm("JR_Z", vsDone);

                EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
                EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                EmitAsm("JR_Z", vsFast);

                EmitLabel(vsSafeLoop);
                EmitAsm("LD_A_B");
                EmitAsm("OR_C");
                EmitAsm("JR_Z", vsDone);
                EmitAsm("DI");
                EmitLabel(vsSafeWait);
                EmitAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem)); // STAT
                EmitAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate));
                EmitAsm("JR_NZ", vsSafeWait);
                EmitAsm("LD_A_D");
                EmitAsm("LDI_HL_A");
                EmitAsm("EI");
                EmitAsm("DEC_BC");
                EmitAsm("JR", vsSafeLoop);

                EmitLabel(vsFast);
                EmitLabel(vsFastLoop);
                EmitAsm("LD_A_B");
                EmitAsm("OR_C");
                EmitAsm("JR_Z", vsDone);
                EmitAsm("LD_A_D");
                EmitAsm("LDI_HL_A");
                EmitAsm("DEC_BC");
                EmitAsm("JR", vsFastLoop);

                EmitLabel(vsDone);
                return;
            }

            // __vram_memset_unsafe(dst_ptr, value_u8, len_u16)
            if (funcName == "__vram_memset_unsafe")
            {
                if (args.Length != 3) Error(expr, "__vram_memset_unsafe(dst, value, len) expects 3 arguments");

                CompileIntoHL(args[0]); EmitAsm("PUSH_HL"); // dst
                CompileIntoA(args[1]); EmitAsm("LD_D_A"); // value
                // Specialize a constant unsafe fill or fall back to its unchecked byte loop.
                Expr vramSetUnsafeLen = FoldConstants(args[2]);
                EmitLengthExprIntoBC(vramSetUnsafeLen);
                EmitAsm("POP_HL"); // dst

                if (vramSetUnsafeLen.Match(Tag.Integer, out int vramSetUnsafeLenConst) &&
                    TryEmitVramMemsetConstCount(false, vramSetUnsafeLenConst, "vramsetu_small"))
                    return;

                AsmOperand vsuLoop = MakeUniqueLabel("vramsetu_loop");
                AsmOperand vsuDone = MakeUniqueLabel("vramsetu_done");
                EmitLabel(vsuLoop);
                EmitAsm("LD_A_B");
                EmitAsm("OR_C");
                EmitAsm("JR_Z", vsuDone);
                EmitAsm("LD_A_D");
                EmitAsm("LDI_HL_A");
                EmitAsm("DEC_BC");
                EmitAsm("JR", vsuLoop);
                EmitLabel(vsuDone);
                return;
            }

            // __far_memcpy(dst_ptr, bank_u8, src_ptr, len_u16)
            // Copies from switchable ROM bank via a bank0 helper and restores original bank.
            if (funcName == "__far_memcpy")
            {
                if (args.Length != 4) Error(expr, "__far_memcpy(dst, bank, src, len) expects 4 arguments");

                CompileIntoHL(args[0]); EmitAsm("PUSH_HL"); // dst
                CompileIntoA(args[1]); EmitAsm("PUSH_AF"); // bank
                CompileIntoHL(args[2]); EmitAsm("PUSH_HL"); // src
                CompileIntoHL(args[3]); // len
                EmitAsm("LD_B_H");
                EmitAsm("LD_C_L");
                EmitAsm("POP_DE"); // src
                EmitAsm("POP_AF"); // bank -> A
                EmitAsm("POP_HL"); // dst

                UseFarMemcpyBank0Helper = true;
                EmitAsm("CALL", new AsmOperand("__kq_far_memcpy_bank0", AddressMode.Absolute));
                return;
            }

            // Copy a fixed RAM block after staging both pointers, using the shared BC-counted loop.
            if (funcName == "__copy16" || funcName == "__copy32")
            {
                if (args.Length != 2) Error(expr, funcName + "(dst, src) expects 2 arguments");

                int copyLen = (funcName == "__copy16") ? 16 : 32;
                CompileIntoHL(args[0]); EmitAsm("PUSH_HL"); // dst
                CompileIntoHL(args[1]); EmitAsm("PUSH_HL"); // src
                EmitAsm("LD_BC_IMM", new AsmOperand(copyLen, AddressMode.Immediate16));
                EmitAsm("POP_DE"); // src
                EmitAsm("POP_HL"); // dst
                EmitRamMemcpyLoop(funcName.TrimStart('_'));
                return;
            }

            // Stage the bank/address and use a stack-based destination for a one-byte far-memory helper call.
            if (funcName == "__farpeek8")
            {
                if (args.Length != 2) Error(expr, "__farpeek8(bank, addr) expects 2 arguments");

                CompileIntoA(args[0]); EmitPushAAsWord(); // bank
                CompileIntoHL(args[1]); EmitAsm("PUSH_HL"); // src
                EmitAsm("ADD_SP_IMM", new AsmOperand(-2, AddressMode.Relative)); // scratch
                // At SP: scratch, saved address, saved bank; each occupies two bytes.

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                EmitAsm("PUSH_HL"); // dst = &scratch

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A");

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                EmitAsm("LD_A_HL"); // bank

                EmitAsm("POP_HL"); // dst
                EmitAsm("LD_BC_IMM", new AsmOperand(1, AddressMode.Immediate16));
                EmitCallFarMemcpyHelper();

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("ADD_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                return;
            }

            // Fetch two bytes through the far-memory helper and mark the little-endian word result as returned in HL.
            if (funcName == "__farpeek16")
            {
                if (args.Length != 2) Error(expr, "__farpeek16(bank, addr) expects 2 arguments");

                CompileIntoA(args[0]); EmitPushAAsWord(); // bank
                CompileIntoHL(args[1]); EmitAsm("PUSH_HL"); // src
                EmitAsm("ADD_SP_IMM", new AsmOperand(-2, AddressMode.Relative)); // scratch
                // At SP: scratch, saved address, saved bank; each occupies two bytes.

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                EmitAsm("PUSH_HL"); // dst = &scratch

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A");

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                EmitAsm("LD_A_HL"); // bank

                EmitAsm("POP_HL"); // dst
                EmitAsm("LD_BC_IMM", new AsmOperand(2, AddressMode.Immediate16));
                EmitCallFarMemcpyHelper();

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A");
                EmitAsm("LD_H_D");
                EmitAsm("LD_L_E");
                EmitAsm("ADD_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                LastCallReturnsHL = true;
                return;
            }

            // Invoke a runtime-address, no-argument callback through fixed ROM.
            if (funcName == "__farcall_ptr")
            {
                if (args.Length != 2) Error(expr, "__farcall_ptr(bank, func) expects 2 arguments");
                CType callbackType = TypeOf(args[1]);
                if (callbackType.IsPointer) callbackType = callbackType.Subtype;
                if (callbackType != null && callbackType.IsFunction &&
                    (callbackType.ParamTypes ?? Array.Empty<CType>()).Length != 0)
                    Error(expr, "__farcall_ptr requires a callback with no parameters");
                CompileIntoA(args[0]);
                EmitAsm("PUSH_AF");
                CompileIntoHL(args[1]);
                EmitAsm("POP_AF");
                UseFarCallPointerBank0Helper = true;
                EmitAsm("CALL", new AsmOperand("__kq_farcall_pointer_bank0", AddressMode.Absolute));
                RecordCallEdge(expr, "<indirect-far>", -1, "farcall_pointer", viaThunk: true,
                    viaFarcall: true, actualArgSizes: new int[0], expectedArgSizes: new int[0]);
                return;
            }

            // Compute a tile-map address in HL and expose it as a word-valued intrinsic result.
            if (funcName == "__tile_addr")
            {
                if (args.Length != 3) Error(expr, "__tile_addr(base, x, y) expects 3 arguments");

                EmitTileMapAddressIntoHL(args[0], args[1], args[2]);
                LastCallReturnsHL = true;
                return;
            }

            // Choose constant folding, power-of-two multiplication or the general byte-multiply path for y*width+x.
            if (funcName == "__map_index")
            {
                if (args.Length != 3) Error(expr, "__map_index(x, y, width) expects 3 arguments");

                Expr xExpr = FoldConstants(args[0]);
                Expr yExpr = FoldConstants(args[1]);
                Expr widthExpr = FoldConstants(args[2]);
                // Apply the width parameter's byte conversion before choosing a shift specialization.
                if (widthExpr.Match(Tag.Integer, out int byteWidth))
                    widthExpr = Expr.Make(Tag.Integer, byteWidth & 0xFF).WithSource(widthExpr.Source);
                if (xExpr.Match(Tag.Integer, out int fullXConst) &&
                    yExpr.Match(Tag.Integer, out int fullYConst) &&
                    widthExpr.Match(Tag.Integer, out int fullWidthConst))
                {
                    // Apply the byte argument widths before calculating the word-sized constant index.
                    int fullIndexConst = (((fullYConst & 0xFF) * (fullWidthConst & 0xFF)) + (fullXConst & 0xFF)) & 0xFFFF;
                    EmitAsm("LD_HL_IMM", new AsmOperand(fullIndexConst, AddressMode.Immediate16));
                    LastCallReturnsHL = true;
                    return;
                }

                if (widthExpr.Match(Tag.Integer, out int widthConst) && widthConst > 0 && IsPow2(widthConst))
                {
                    CompileIntoA(yExpr);
                    EmitAsm("LD_L_A");
                    EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                    int sh = Log2Pow2(widthConst);
                    for (int i = 0; i < sh; i++) EmitAsm("ADD_HL_HL");

                    if (xExpr.Match(Tag.Integer, out int xConst) && xConst == 0)
                    {
                        LastCallReturnsHL = true;
                        return;
                    }

                    CompileIntoA(xExpr);
                    EmitAsm("LD_E_A");
                    EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
                    EmitAsm("ADD_HL_DE");
                    LastCallReturnsHL = true;
                    return;
                }

                CompileIntoA(args[0]); EmitPushAAsWord(); // x
                CompileIntoA(args[1]); EmitPushAAsWord(); // y
                CompileIntoA(args[2]); EmitPushAAsWord(); // width

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_C_A"); // width

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(2, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A"); // y

                EmitMul8x8ToHL_EC("__map_index");

                // Loading the saved X argument uses HL, so preserve the multiplication result first.
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("POP_HL");
                EmitAsm("ADD_HL_DE");
                EmitAsm("ADD_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                LastCallReturnsHL = true;
                return;
            }

            // Check half-open rectangle bounds using coordinate differences, with a fully constant fast path.
            if (funcName == "__xy_in_rect")
            {
                if (args.Length != 6) Error(expr, "__xy_in_rect(x, y, rx, ry, rw, rh) expects 6 arguments");

                Expr[] foldedRectArgs = args.Select(FoldConstants).ToArray();
                if (foldedRectArgs[0].Match(Tag.Integer, out int constX) &&
                    foldedRectArgs[1].Match(Tag.Integer, out int constY) &&
                    foldedRectArgs[2].Match(Tag.Integer, out int constRx) &&
                    foldedRectArgs[3].Match(Tag.Integer, out int constRy) &&
                    foldedRectArgs[4].Match(Tag.Integer, out int constRw) &&
                    foldedRectArgs[5].Match(Tag.Integer, out int constRh))
                {
                    int x = constX & 0xFF;
                    int y = constY & 0xFF;
                    int rx = constRx & 0xFF;
                    int ry = constRy & 0xFF;
                    int rw = constRw & 0xFF;
                    int rh = constRh & 0xFF;
                    bool inside = x >= rx && y >= ry && (x - rx) < rw && (y - ry) < rh;
                    EmitAsm(inside ? "LD_A_IMM" : "XOR_A", inside ? new AsmOperand(1, AddressMode.Immediate) : null);
                    return;
                }

                EmitPushExprAsWord(args[0]); // x
                EmitPushExprAsWord(args[1]); // y
                EmitPushExprAsWord(args[2]); // rx
                EmitPushExprAsWord(args[3]); // ry
                EmitPushExprAsWord(args[4]); // rw
                EmitPushExprAsWord(args[5]); // rh

                AsmOperand ret0 = MakeUniqueLabel("xyrect_ret0");
                AsmOperand done = MakeUniqueLabel("xyrect_done");

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(10, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_B_A"); // x
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_C_A"); // rx
                EmitAsm("LD_A_B");
                EmitAsm("CP_C");
                EmitAsm("JR_C", ret0);
                EmitAsm("SUB_C");
                EmitAsm("LD_B_A");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(2, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_C_A"); // rw
                EmitAsm("LD_A_B");
                EmitAsm("CP_C");
                EmitAsm("JR_NC", ret0);

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(8, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_B_A"); // y
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_C_A"); // ry
                EmitAsm("LD_A_B");
                EmitAsm("CP_C");
                EmitAsm("JR_C", ret0);
                EmitAsm("SUB_C");
                EmitAsm("LD_B_A");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_C_A"); // rh
                EmitAsm("LD_A_B");
                EmitAsm("CP_C");
                EmitAsm("JR_NC", ret0);

                EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate));
                EmitAsm("JR", done);
                EmitLabel(ret0);
                EmitAsm("XOR_A");
                EmitLabel(done);
                EmitAsm("ADD_SP_IMM", new AsmOperand(12, AddressMode.Relative));
                return;
            }

            // Return the byte-sized sum of unsigned coordinate distances, wrapping sums beyond 255.
            if (funcName == "__manhattan")
            {
                if (args.Length != 4) Error(expr, "__manhattan(x1, y1, x2, y2) expects 4 arguments");

                Expr[] foldedManhattanArgs = args.Select(FoldConstants).ToArray();
                if (foldedManhattanArgs[0].Match(Tag.Integer, out int constX1) &&
                    foldedManhattanArgs[1].Match(Tag.Integer, out int constY1) &&
                    foldedManhattanArgs[2].Match(Tag.Integer, out int constX2) &&
                    foldedManhattanArgs[3].Match(Tag.Integer, out int constY2))
                {
                    int dx = Math.Abs((constX1 & 0xFF) - (constX2 & 0xFF));
                    int dy = Math.Abs((constY1 & 0xFF) - (constY2 & 0xFF));
                    EmitAsm("LD_A_IMM", new AsmOperand((dx + dy) & 0xFF, AddressMode.Immediate));
                    return;
                }

                EmitPushExprAsWord(args[0]); // x1
                EmitPushExprAsWord(args[1]); // y1
                EmitPushExprAsWord(args[2]); // x2
                EmitPushExprAsWord(args[3]); // y2

                AsmOperand dxGe = MakeUniqueLabel("manhattan_dx_ge");
                AsmOperand dxDone = MakeUniqueLabel("manhattan_dx_done");
                AsmOperand dyGe = MakeUniqueLabel("manhattan_dy_ge");
                AsmOperand dyDone = MakeUniqueLabel("manhattan_dy_done");

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_B_A"); // x1
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(2, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_C_A"); // x2
                EmitAsm("LD_A_B");
                EmitAsm("CP_C");
                EmitAsm("JR_NC", dxGe);
                EmitAsm("LD_A_C");
                EmitAsm("SUB_B");
                EmitAsm("JR", dxDone);
                EmitLabel(dxGe);
                EmitAsm("LD_A_B");
                EmitAsm("SUB_C");
                EmitLabel(dxDone);
                EmitAsm("LD_B_A"); // dx

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_C_A"); // y1
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A"); // y2
                EmitAsm("LD_A_C");
                EmitAsm("CP_D");
                EmitAsm("JR_NC", dyGe);
                EmitAsm("LD_A_D");
                EmitAsm("SUB_C");
                EmitAsm("JR", dyDone);
                EmitLabel(dyGe);
                EmitAsm("LD_A_C");
                EmitAsm("SUB_D");
                EmitLabel(dyDone);
                EmitAsm("ADD_B");
                EmitAsm("ADD_SP_IMM", new AsmOperand(8, AddressMode.Relative));
                return;
            }

            // Split a word bit index into a byte offset and mask; GB bit tests normalize the result to zero or one.
            if (funcName == "__bit_test" || funcName == "__bit_set" || funcName == "__bit_clear" || funcName == "__bit_toggle")
            {
                if (args.Length != 2) Error(expr, funcName + "(ptr, bit_index) expects 2 arguments");

                CompileIntoHL(args[0]); EmitAsm("PUSH_HL"); // ptr
                CompileIntoHL(args[1]); EmitAsm("PUSH_HL"); // bit_index

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("AND_IMM", new AsmOperand(7, AddressMode.Immediate));
                EmitAsm("LD_B_A"); // bit number 0..7

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A");
                EmitAsm("LD_H_D");
                EmitAsm("LD_L_E");
                EmitShiftRightLogicalHL(3);
                EmitAsm("PUSH_HL"); // byte offset

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A");

                EmitAsm("POP_HL");
                EmitAsm("ADD_HL_DE"); // HL = ptr + (bit_index >> 3)

                AsmOperand maskReady = MakeUniqueLabel("bit_mask_ready");
                AsmOperand maskLoop = MakeUniqueLabel("bit_mask_loop");
                EmitAsm("LD_A_B");
                EmitAsm("OR_A");
                EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate));
                EmitAsm("JR_Z", maskReady);
                EmitLabel(maskLoop);
                EmitAsm("ADD_A");
                EmitAsm("DEC_B");
                EmitAsm("JR_NZ", maskLoop);
                EmitLabel(maskReady);
                EmitAsm("LD_B_A"); // mask

                if (funcName == "__bit_test")
                {
                    AsmOperand retZero = MakeUniqueLabel("bit_test_zero");
                    AsmOperand done = MakeUniqueLabel("bit_test_done");
                    EmitAsm("LD_A_HL");
                    EmitAsm("AND_B");
                    EmitAsm("JR_Z", retZero);
                    EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate));
                    EmitAsm("JR", done);
                    EmitLabel(retZero);
                    EmitAsm("XOR_A");
                    EmitLabel(done);
                    EmitAsm("ADD_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                    return;
                }

                if (funcName == "__bit_set")
                {
                    EmitAsm("LD_A_HL");
                    EmitAsm("OR_B");
                    EmitAsm("LD_HL_A");
                    EmitAsm("ADD_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                    return;
                }

                if (funcName == "__bit_clear")
                {
                    EmitAsm("LD_A_B");
                    EmitAsm("CPL");
                    EmitAsm("LD_B_A");
                    EmitAsm("LD_A_HL");
                    EmitAsm("AND_B");
                    EmitAsm("LD_HL_A");
                    EmitAsm("ADD_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                    return;
                }

                EmitAsm("LD_A_HL");
                EmitAsm("XOR_B");
                EmitAsm("LD_HL_A");
                EmitAsm("ADD_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                return;
            }

            // Read count/value pairs from banked memory until a zero count, filling VRAM and accumulating the output length.
            if (funcName == "__rle_decode_vram")
            {
                if (args.Length != 3) Error(expr, "__rle_decode_vram(dst, bank, src) expects 3 arguments");

                CompileIntoHL(args[0]); EmitAsm("PUSH_HL"); // dst
                CompileIntoA(args[1]); EmitPushAAsWord(); // bank
                CompileIntoHL(args[2]); EmitAsm("PUSH_HL"); // src
                EmitAsm("ADD_SP_IMM", new AsmOperand(-4, AddressMode.Relative)); // total(2) + scratch(2)

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                EmitAsm("XOR_A");
                EmitAsm("LD_HL_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_HL_A");

                AsmOperand loop = MakeUniqueLabel("rlevram_loop");
                // Keep total, scratch, source, bank and destination in stack slots across repeated far-memory calls.
                AsmOperand countDone = MakeUniqueLabel("rlevram_count_inc_done");
                AsmOperand valueDone = MakeUniqueLabel("rlevram_value_inc_done");
                AsmOperand finished = MakeUniqueLabel("rlevram_finished");

                EmitLabel(loop);

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(2, AddressMode.Relative));
                EmitAsm("PUSH_HL"); // &scratch

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A"); // src

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(8, AddressMode.Relative));
                EmitAsm("LD_A_HL"); // bank

                EmitAsm("POP_HL"); // &scratch
                EmitAsm("LD_BC_IMM", new AsmOperand(1, AddressMode.Immediate16));
                EmitCallFarMemcpyHelper();

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(2, AddressMode.Relative));
                EmitAsm("LD_A_HL"); // count
                EmitAsm("OR_A");
                EmitAsm("JR_Z", finished);
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(3, AddressMode.Relative));
                EmitAsm("LD_HL_A"); // save count in scratch+1

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("ADD_A_IMM", new AsmOperand(1, AddressMode.Immediate));
                EmitAsm("LD_HL_A");
                EmitAsm("JR_NC", countDone);
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("LD_HL_A");
                EmitLabel(countDone);

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(2, AddressMode.Relative));
                EmitAsm("PUSH_HL"); // &scratch

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A"); // src

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(8, AddressMode.Relative));
                EmitAsm("LD_A_HL"); // bank

                EmitAsm("POP_HL"); // &scratch
                EmitAsm("LD_BC_IMM", new AsmOperand(1, AddressMode.Immediate16));
                EmitCallFarMemcpyHelper();

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(2, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A"); // value

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(3, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_C_A");
                EmitAsm("LD_B_IMM", new AsmOperand(0, AddressMode.Immediate)); // count

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(8, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A");
                EmitAsm("LD_H_D");
                EmitAsm("LD_L_E"); // dst

                // Use the LCD-aware fill loop for each decoded run, then store its advanced destination pointer.
                EmitVramMemsetLoop(true, "rlevram_fill");

                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(10, AddressMode.Relative));
                EmitAsm("POP_DE");
                EmitAsm("LD_A_E");
                EmitAsm("LD_HL_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_D");
                EmitAsm("LD_HL_A"); // save dst

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(3, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_B_A"); // count
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("ADD_B");
                EmitAsm("LD_HL_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("LD_HL_A"); // total += count

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("ADD_A_IMM", new AsmOperand(1, AddressMode.Immediate));
                EmitAsm("LD_HL_A");
                EmitAsm("JR_NC", valueDone);
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("LD_HL_A");
                EmitLabel(valueDone);

                EmitAsm("JR", loop);

                EmitLabel(finished);
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A");
                EmitAsm("LD_H_D");
                EmitAsm("LD_L_E");
                EmitAsm("ADD_SP_IMM", new AsmOperand(10, AddressMode.Relative));
                LastCallReturnsHL = true;
                return;
            }

            // __sram_read8(addr_u16) -> u8
            if (funcName == "__sram_read8")
            {
                if (args.Length != 1) Error(expr, "__sram_read8(addr) expects 1 argument");

                CompileIntoHL(args[0]);
                EmitAsm("PUSH_HL");

                EmitAsm("LD_A_IMM", new AsmOperand(0x0A, AddressMode.Immediate));
                EmitAsm("LD_MEM_A", new AsmOperand(0x0000, AddressMode.Absolute));

                EmitAsm("POP_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_B_A");

                EmitAsm("LD_A_IMM", new AsmOperand(0x00, AddressMode.Immediate));
                EmitAsm("LD_MEM_A", new AsmOperand(0x0000, AddressMode.Absolute));
                EmitAsm("LD_A_B");
                return;
            }

            // __sram_write8(addr_u16, value_u8)
            if (funcName == "__sram_write8")
            {
                if (args.Length != 2) Error(expr, "__sram_write8(addr, value) expects 2 arguments");

                CompileIntoHL(args[0]);
                EmitAsm("PUSH_HL");
                CompileIntoA(args[1]);
                EmitAsm("LD_C_A");

                EmitAsm("LD_A_IMM", new AsmOperand(0x0A, AddressMode.Immediate));
                EmitAsm("LD_MEM_A", new AsmOperand(0x0000, AddressMode.Absolute));

                EmitAsm("POP_HL");
                EmitAsm("LD_A_C");
                EmitAsm("LD_HL_A");

                EmitAsm("LD_A_IMM", new AsmOperand(0x00, AddressMode.Immediate));
                EmitAsm("LD_MEM_A", new AsmOperand(0x0000, AddressMode.Absolute));
                return;
            }

            // __rng_seed(seed_u16)
            if (funcName == "__rng_seed")
            {
                if (args.Length != 1) Error(expr, "__rng_seed(seed) expects 1 argument");

                CompileIntoHL(args[0]);
                EmitAsm("LD_A_L");
                EmitStoreA(RngStateLoVar);
                EmitAsm("LD_A_H");
                EmitStoreA(RngStateHiVar);
                return;
            }

            // __rng8() -> u8
            if (funcName == "__rng8")
            {
                if (args.Length != 0) Error(expr, "__rng8() expects 0 arguments");

                EmitLoadA(RngStateLoVar);
                EmitAsm("LD_B_A");
                EmitLoadA(RngStateHiVar);
                EmitAsm("LD_C_A");

                // Zero state fallback for deterministic startup.
                AsmOperand rngSeeded = MakeUniqueLabel("rng_seeded");
                EmitAsm("LD_A_B");
                EmitAsm("OR_C");
                EmitAsm("JR_NZ", rngSeeded);
                EmitAsm("LD_A_IMM", new AsmOperand(0xA5, AddressMode.Immediate));
                EmitAsm("LD_B_A");
                EmitAsm("LD_A_IMM", new AsmOperand(0x5A, AddressMode.Immediate));
                EmitAsm("LD_C_A");
                EmitLabel(rngSeeded);

                // 16-bit mixed add/xor/rotate step.
                EmitAsm("LD_A_B");
                EmitAsm("ADD_A_IMM", new AsmOperand(0x17, AddressMode.Immediate));
                EmitAsm("LD_B_A");
                EmitAsm("LD_A_C");
                EmitAsm("ADC_IMM", new AsmOperand(0x5D, AddressMode.Immediate));
                EmitAsm("LD_C_A");
                EmitAsm("LD_A_B");
                EmitAsm("XOR_C");
                EmitAsm("LD_B_A");
                EmitAsm("LD_A_C");
                EmitAsm("RLCA");
                EmitAsm("ADD_B");
                EmitAsm("LD_C_A");

                EmitAsm("LD_A_B");
                EmitStoreA(RngStateLoVar);
                EmitAsm("LD_A_C");
                EmitStoreA(RngStateHiVar);
                EmitAsm("LD_A_C");
                return;
            }

            // 1. __memset(dest_ptr, val_u8, count_u16)
            //    __memset_small(dest_ptr, val_u8, len_u8)
            if (funcName == "__memset" || funcName == "__memset_small")
            {
                // The small API truncates its count to a byte; the general API uses a word count.
                bool useU8Count = (funcName == "__memset_small");
                if (args.Length != 3) { Error(expr, "__memset requires 3 arguments"); return; }

                // -Zcheck: fixed-array range check for memset length
                bool doCheck = Program.CheckMemCopy && !InUnsafe;
                bool hasDst = false;
                int dstBytes = 0;
                string dstName = null;
                if (doCheck)
                    hasDst = TryGetFixedArrayByteLength(args[0], out dstBytes, out dstName);

                bool isConstCountAny = args[2].Match(Tag.Integer, out int constCountAny);
                int normalizedConstCount = constCountAny;
                if (isConstCountAny && useU8Count)
                    normalizedConstCount &= 0xFF;

                if (doCheck && hasDst && isConstCountAny && normalizedConstCount >= 0 && normalizedConstCount > dstBytes)
                {
                    Program.Warning(args[2].Source, "__memset count {0} exceeds fixed array '{1}' size {2} bytes (-Zcheck)", normalizedConstCount, dstName, dstBytes);
                    EmitAsm("JP", EnsureCheckTrap());
                    return;
                }

                // Size-specialized memset for small constant counts (<=16): unrolled stores.
                if (isConstCountAny && normalizedConstCount >= 0 && normalizedConstCount <= 16)
                {
                    if (normalizedConstCount == 0) return;

                    CompileIntoHL(args[0]);
                    EmitAsm("PUSH_HL");
                    CompileIntoA(args[1]);
                    EmitAsm("POP_HL");

                    for (int k = 0; k < normalizedConstCount; k++)
                        EmitAsm("LDI_HL_A");
                    return;
                }

                // Size-specialized memset for constant counts beyond 16
                if (isConstCountAny && normalizedConstCount >= 0)
                {
                    if (normalizedConstCount == 0) return;

                    if (normalizedConstCount > 16 && normalizedConstCount <= 64)
                    {
                        int blocks8 = normalizedConstCount / 8;
                        int rem = normalizedConstCount % 8;

                        CompileIntoHL(args[0]);
                        EmitAsm("PUSH_HL");
                        CompileIntoA(args[1]);
                        EmitAsm("POP_HL");

                        if (blocks8 > 0)
                        {
                            EmitAsm("LD_B_IMM", new AsmOperand(blocks8, AddressMode.Immediate));
                            // Emit groups of eight stores followed by the constant remainder for medium fills.
                            AsmOperand loop8 = MakeUniqueLabel("memset8_loop");
                            EmitLabel(loop8);
                            for (int k = 0; k < 8; k++) EmitAsm("LDI_HL_A");
                            EmitAsm("DEC_B");
                            EmitAsm("JP_NZ", loop8);
                        }
                        for (int k = 0; k < rem; k++) EmitAsm("LDI_HL_A");
                        return;
                    }

                    if (normalizedConstCount <= 255)
                    {
                        int blocks16 = normalizedConstCount / 16;
                        int rem = normalizedConstCount % 16;

                        CompileIntoHL(args[0]);
                        EmitAsm("PUSH_HL");
                        CompileIntoA(args[1]);
                        EmitAsm("POP_HL");

                        if (blocks16 > 0)
                        {
                            EmitAsm("LD_B_IMM", new AsmOperand(blocks16, AddressMode.Immediate));
                            // Use sixteen-store groups for larger byte-range constant fills.
                            AsmOperand loop16 = MakeUniqueLabel("memset16_loop");
                            EmitLabel(loop16);
                            for (int k = 0; k < 16; k++) EmitAsm("LDI_HL_A");
                            EmitAsm("DEC_B");
                            EmitAsm("JP_NZ", loop16);
                        }
                        for (int k = 0; k < rem; k++) EmitAsm("LDI_HL_A");
                        return;
                    }
                }

                if (useU8Count)
                {
                    CompileIntoHL(args[0]);
                    EmitAsm("PUSH_HL");

                    CompileIntoA(args[1]);
                    EmitAsm("LD_D_A");

                    CompileIntoA(args[2]);
                    EmitAsm("LD_B_A");
                    if (doCheck && hasDst)
                        EmitU8LeqConstOrTrapInA(dstBytes);

                    AsmOperand doneLabel = MakeUniqueLabel("memset_small_done");
                    AsmOperand smallLoopLabel = MakeUniqueLabel("memset_small_loop");
                    EmitAsm("OR_A");
                    EmitAsm("POP_HL");
                    EmitAsm("JP_Z", doneLabel);
                    EmitAsm("LD_A_D");
                    EmitLabel(smallLoopLabel);
                    EmitAsm("LDI_HL_A");
                    EmitAsm("DEC_B");
                    EmitAsm("JP_NZ", smallLoopLabel);
                    EmitLabel(doneLabel);
                    return;
                }

                CompileIntoHL(args[0]);
                EmitAsm("PUSH_HL");

                CompileIntoHL(args[2]);
                if (doCheck && hasDst && !isConstCountAny)
                    EmitU16LeqConstOrTrap(dstBytes);
                EmitAsm("LD_B_H");
                EmitAsm("LD_C_L");

                CompileIntoA(args[1]);
                EmitAsm("LD_D_A");

                EmitAsm("POP_HL");

                AsmOperand loopLabel = MakeUniqueLabel("memset_loop");
                AsmOperand wordDoneLabel = MakeUniqueLabel("memset_done");
                // A zero word count must skip the first write as well as the loop.
                EmitAsm("LD_A_B");
                EmitAsm("OR_C");
                EmitAsm("JP_Z", wordDoneLabel);
                EmitLabel(loopLabel);

                EmitAsm("LD_A_D");
                EmitAsm("LDI_HL_A");

                EmitAsm("DEC_BC");
                EmitAsm("LD_A_B");
                EmitAsm("OR_C");
                EmitAsm("JP_NZ", loopLabel);
                EmitLabel(wordDoneLabel);

                return;
            }

            // 2. __memcpy(dst_ptr, src_ptr, count_u16)
            //    __memcpy_small(dst_ptr, src_ptr, len_u8)
            if (funcName == "__memcpy" || funcName == "__memcpy_small")
            {
                // Select byte or word count semantics before optional fixed-array bounds diagnostics.
                bool useU8Count = (funcName == "__memcpy_small");
                if (args.Length != 3) { Error(expr, "__memcpy requires 3 arguments"); return; }

                bool doCheck = Program.CheckMemCopy && !InUnsafe;
                bool hasDst = false, hasSrc = false;
                int dstBytes = 0, srcBytes = 0;
                string dstName = null, srcName = null;
                if (doCheck)
                {
                    hasDst = TryGetFixedArrayByteLength(args[0], out dstBytes, out dstName);
                    hasSrc = TryGetFixedArrayByteLength(args[1], out srcBytes, out srcName);
                }

                bool isConstCountAny = args[2].Match(Tag.Integer, out int constCountAny);
                int normalizedConstCount = constCountAny;
                if (isConstCountAny && useU8Count)
                    normalizedConstCount &= 0xFF;

                if (doCheck && isConstCountAny)
                {
                    if (hasDst && normalizedConstCount >= 0 && normalizedConstCount > dstBytes)
                    {
                        Program.Warning(args[2].Source, "__memcpy count {0} exceeds fixed dst array '{1}' size {2} bytes (-Zcheck)", normalizedConstCount, dstName, dstBytes);
                        EmitAsm("JP", EnsureCheckTrap());
                        return;
                    }
                    if (hasSrc && normalizedConstCount >= 0 && normalizedConstCount > srcBytes)
                    {
                        Program.Warning(args[2].Source, "__memcpy count {0} exceeds fixed src array '{1}' size {2} bytes (-Zcheck)", normalizedConstCount, srcName, srcBytes);
                        EmitAsm("JP", EnsureCheckTrap());
                        return;
                    }
                }

                if (isConstCountAny && normalizedConstCount >= 0 && normalizedConstCount <= 16)
                {
                    if (normalizedConstCount == 0) return;

                    CompileIntoHL(args[0]);
                    EmitAsm("PUSH_HL");
                    CompileIntoHL(args[1]);
                    EmitAsm("PUSH_HL");
                    EmitAsm("POP_DE");
                    EmitAsm("POP_HL");

                    for (int k = 0; k < normalizedConstCount; k++)
                    {
                        EmitAsm("LD_A_DE");
                        EmitAsm("LDI_HL_A");
                        EmitAsm("INC_DE");
                    }
                    return;
                }

                if (isConstCountAny && normalizedConstCount > 16 && normalizedConstCount <= 255)
                {
                    CompileIntoHL(args[0]);
                    EmitAsm("PUSH_HL");
                    CompileIntoHL(args[1]);
                    EmitAsm("PUSH_HL");
                    EmitAsm("POP_DE");
                    EmitAsm("POP_HL");

                    if (normalizedConstCount <= 64)
                    {
                        int blocks8 = normalizedConstCount / 8;
                        int rem8 = normalizedConstCount % 8;
                        if (blocks8 > 0)
                        {
                            EmitAsm("LD_B_IMM", new AsmOperand(blocks8, AddressMode.Immediate));
                            // Advance source and destination through eight-byte groups, then emit the remaining constant bytes.
                            AsmOperand loop8 = MakeUniqueLabel("memcpy8_loop");
                            EmitLabel(loop8);
                            for (int k = 0; k < 8; k++)
                            {
                                EmitAsm("LD_A_DE");
                                EmitAsm("LDI_HL_A");
                                EmitAsm("INC_DE");
                            }
                            EmitAsm("DEC_B");
                            EmitAsm("JP_NZ", loop8);
                        }
                        for (int k = 0; k < rem8; k++)
                        {
                            EmitAsm("LD_A_DE");
                            EmitAsm("LDI_HL_A");
                            EmitAsm("INC_DE");
                        }
                        return;
                    }

                    int blocks16 = normalizedConstCount / 16;
                    int rem16 = normalizedConstCount % 16;
                    if (blocks16 > 0)
                    {
                        EmitAsm("LD_B_IMM", new AsmOperand(blocks16, AddressMode.Immediate));
                        // Use sixteen-byte groups while keeping the residual copy length known at compile time.
                        AsmOperand loop16 = MakeUniqueLabel("memcpy16_loop");
                        EmitLabel(loop16);
                        for (int k = 0; k < 16; k++)
                        {
                            EmitAsm("LD_A_DE");
                            EmitAsm("LDI_HL_A");
                            EmitAsm("INC_DE");
                        }
                        EmitAsm("DEC_B");
                        EmitAsm("JP_NZ", loop16);
                    }
                    for (int k = 0; k < rem16; k++)
                    {
                        EmitAsm("LD_A_DE");
                        EmitAsm("LDI_HL_A");
                        EmitAsm("INC_DE");
                    }
                    return;
                }

                if (useU8Count)
                {
                    CompileIntoHL(args[0]);
                    EmitAsm("PUSH_HL");

                    CompileIntoHL(args[1]);
                    EmitAsm("PUSH_HL");

                    CompileIntoA(args[2]);
                    EmitAsm("LD_B_A");
                    if (doCheck)
                    {
                        if (hasDst) EmitU8LeqConstOrTrapInA(dstBytes);
                        if (hasSrc) EmitU8LeqConstOrTrapInA(srcBytes);
                    }

                    AsmOperand doneLabel = MakeUniqueLabel("memcpy_small_done");
                    AsmOperand smallLoopLabel = MakeUniqueLabel("memcpy_small_loop");
                    EmitAsm("OR_A");
                    EmitAsm("POP_DE");
                    EmitAsm("POP_HL");
                    EmitAsm("JP_Z", doneLabel);

                    EmitLabel(smallLoopLabel);
                    EmitAsm("LD_A_DE");
                    EmitAsm("LDI_HL_A");
                    EmitAsm("INC_DE");
                    EmitAsm("DEC_B");
                    EmitAsm("JP_NZ", smallLoopLabel);
                    EmitLabel(doneLabel);
                    return;
                }

                CompileIntoHL(args[0]);
                EmitAsm("PUSH_HL");

                CompileIntoHL(args[1]);
                EmitAsm("PUSH_HL");

                CompileIntoHL(args[2]);
                if (doCheck && !isConstCountAny)
                {
                    if (hasDst) EmitU16LeqConstOrTrap(dstBytes);
                    if (hasSrc) EmitU16LeqConstOrTrap(srcBytes);
                }
                EmitAsm("LD_B_H");
                EmitAsm("LD_C_L");

                EmitAsm("POP_DE");
                EmitAsm("POP_HL");

                AsmOperand loopLabel = MakeUniqueLabel("memcpy_loop");
                AsmOperand wordDoneLabel = MakeUniqueLabel("memcpy_done");
                // A zero word count must skip the first write as well as the loop.
                EmitAsm("LD_A_B");
                EmitAsm("OR_C");
                EmitAsm("JP_Z", wordDoneLabel);
                EmitLabel(loopLabel);

                EmitAsm("LD_A_DE");
                EmitAsm("LDI_HL_A");
                EmitAsm("INC_DE");

                EmitAsm("DEC_BC");
                EmitAsm("LD_A_B");
                EmitAsm("OR_C");
                EmitAsm("JP_NZ", loopLabel);
                EmitLabel(wordDoneLabel);

                return;
            }



            
            // 2.5 __settile(x_u8, y_u8, tile_u8)
            // Writes a tile into BG map 0 (0x9800).
            // v9: CALL-core intrinsic (keeps intrinsic but avoids huge inline expansion).
            if (funcName == "__settile")
            {
                if (args.Length != 3) Error(expr, "__settile requires 3 arguments");

                // Evaluate args left-to-right (matches normal call semantics)
                // Preserve x,y while evaluating tile.
                CompileIntoA(args[0]);
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // x

                CompileIntoA(args[1]);
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // y

                CompileIntoA(args[2]);
                EmitAsm("LD_C_A"); // tile

                // Restore y -> A, x -> B
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("LD_D_A"); // y temp

                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("LD_B_A"); // x

                EmitAsm("LD_A_D"); // y

                UsedSetTileCore = true;
                EmitAsm("CALL", new AsmOperand("__settile_core", AddressMode.Absolute));
                return;
            }


                        // 2.6 __settile_unsafe(x_u8, y_u8, tile_u8)
            // ISR / VBlank-only fast path.
            // v9: CALL-core intrinsic.
            if (funcName == "__settile_unsafe" || funcName == "__settile_fast")
            {
                if (args.Length != 3) Error(expr, "__settile_unsafe requires 3 arguments");

                // Evaluate args left-to-right (matches normal call semantics)
                // Preserve x,y while evaluating tile.
                CompileIntoA(args[0]);
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // x

                CompileIntoA(args[1]);
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // y

                CompileIntoA(args[2]);
                EmitAsm("LD_C_A"); // tile

                // Restore y -> A, x -> B
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("LD_D_A"); // y temp

                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("LD_B_A"); // x

                EmitAsm("LD_A_D"); // y

                UsedSetTileFastCore = true;
                EmitAsm("CALL", new AsmOperand("__settile_fast_core", AddressMode.Absolute));
                return;
            }

            // 2.6b __settile_bulk(dest_u16, src_u16, count_u8)
            // v9: bulk tile-map copy (LCD ON safe with wait+DI/EI inside core).
            if (funcName == "__settile_bulk")
            {
                if (args.Length != 3) Error(expr, "__settile_bulk requires 3 arguments");

                Expr bulkCountExpr = FoldConstants(args[2]);

                // dest
                CompileIntoHL(args[0]);
                EmitAsm("PUSH_HL");

                // src
                CompileIntoHL(args[1]);
                EmitAsm("PUSH_HL");

                // count
                CompileIntoA(bulkCountExpr);
                EmitAsm("LD_B_A");

                // pop src -> DE, dest -> HL
                EmitAsm("POP_DE");
                EmitAsm("POP_HL");

                if (bulkCountExpr.Match(Tag.Integer, out int bulkCountConst) &&
                    TryEmitVramMemcpyConstCount(true, bulkCountConst & 0xFF, "settilebulk_small"))
                    return;

                UsedSetTileBulkCore = true;
                EmitAsm("CALL", new AsmOperand("__settile_bulk_core", AddressMode.Absolute));
                return;
            }

            // 2.6c __settile_bulk_fast(dest_u16, src_u16, count_u8)
            // v9: VBlank-only fast bulk copy (no wait).
            if (funcName == "__settile_bulk_fast")
            {
                if (args.Length != 3) Error(expr, "__settile_bulk_fast requires 3 arguments");

                Expr bulkFastCountExpr = FoldConstants(args[2]);

                // dest
                CompileIntoHL(args[0]);
                EmitAsm("PUSH_HL");

                // src
                CompileIntoHL(args[1]);
                EmitAsm("PUSH_HL");

                // count
                CompileIntoA(bulkFastCountExpr);
                EmitAsm("LD_B_A");

                // pop src -> DE, dest -> HL
                EmitAsm("POP_DE");
                EmitAsm("POP_HL");

                if (bulkFastCountExpr.Match(Tag.Integer, out int bulkFastCountConst) &&
                    TryEmitVramMemcpyConstCount(false, bulkFastCountConst & 0xFF, "settilebulkf_small"))
                    return;

                UsedSetTileBulkFastCore = true;
                EmitAsm("CALL", new AsmOperand("__settile_bulk_fast_core", AddressMode.Absolute));
                return;
            }


                        // 2.7 __settileat(base_u16, x_u8, y_u8, tile_u8)
            // Writes a tile into BG/Window map base (e.g., 0x9800 or 0x9C00).
            // Safe VRAM write: if LCD is on, waits for STAT mode 0/1 before writing (DI/EI).
            // Treated as a compiler intrinsic (no C definition required).
            if (funcName == "__settileat")
            {
                if (args.Length != 4) Error(expr, "__settileat requires 4 arguments");

                AsmOperand st_end = MakeUniqueLabel("settileat_end");

                // x -> B (bounds check x < 32)
                CompileIntoA(args[1]);
                EmitAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("JR_NC", st_end);
                EmitAsm("LD_B_A");

                // y -> A (bounds check y < 32)
                CompileIntoA(args[2]);
                EmitAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("JR_NC", st_end);

                // Save y then x (so later argument evaluation can't clobber them)
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // y

                EmitAsm("LD_A_B");
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // x

                // base -> stack (as u16)
                CompileIntoHL(args[0]);
                EmitAsm("PUSH_HL"); // base

                // tile -> stack (as u16 low byte)
                CompileIntoA(args[3]);
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // tile

                // Pop tile -> C
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("LD_C_A");

                // Pop base -> DE
                EmitAsm("POP_DE");

                // Pop x -> B
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("LD_B_A");

                // Pop y -> A
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");

                // HL = y * 32
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");

                // HL += base (from DE)
                EmitAsm("ADD_HL_DE");

                // HL += x (from B) using DE as temp
                EmitAsm("LD_A_B");
                EmitAsm("LD_E_A");
                EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_DE");

                AsmOperand st_write_now = MakeUniqueLabel("settileat_write_now");
                AsmOperand st_wait = MakeUniqueLabel("settileat_wait");
                AsmOperand st_done = MakeUniqueLabel("settileat_done");

                // Check LCD ON (LCDC bit7). If off, write immediately.
                EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
                EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                EmitAsm("JR_Z", st_write_now);

                // LCD on: wait until STAT bit1 cleared (mode 0/1), with DI/EI window
                EmitAsm("DI");
                EmitLabel(st_wait);
                EmitAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem)); // STAT
                EmitAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate));
                EmitAsm("JR_NZ", st_wait);

                EmitAsm("LD_HL_C"); // [HL] = C
                EmitAsm("EI");
                EmitAsm("JR", st_done);

                EmitLabel(st_write_now);
                EmitAsm("LD_HL_C"); // [HL] = C

                EmitLabel(st_done);
                EmitLabel(st_end);
                return;
            }

            // 2.8 __settileat_unsafe(base_u16, x_u8, y_u8, tile_u8)
            // ISR / VBlank-only fast path: writes immediately (no DI/EI, no STAT wait).
            // Treated as a compiler intrinsic (no C definition required).
            if (funcName == "__settileat_unsafe")
            {
                if (args.Length != 4) Error(expr, "__settileat_unsafe requires 4 arguments");

                AsmOperand st_end = MakeUniqueLabel("settileatu_end");

                // x -> B (bounds check x < 32)
                CompileIntoA(args[1]);
                EmitAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("JR_NC", st_end);
                EmitAsm("LD_B_A");

                // y -> A (bounds check y < 32)
                CompileIntoA(args[2]);
                EmitAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("JR_NC", st_end);

                // Save y then x
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // y

                EmitAsm("LD_A_B");
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // x

                // base -> stack
                CompileIntoHL(args[0]);
                EmitAsm("PUSH_HL");

                // tile -> stack
                CompileIntoA(args[3]);
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL");

                // Pop tile -> C
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("LD_C_A");

                // Pop base -> DE
                EmitAsm("POP_DE");

                // Pop x -> B
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("LD_B_A");

                // Pop y -> A
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");

                // HL = y * 32
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");

                // HL += base (DE)
                EmitAsm("ADD_HL_DE");

                // HL += x (B) using DE temp
                EmitAsm("LD_A_B");
                EmitAsm("LD_E_A");
                EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_DE");

                // direct write
                EmitAsm("LD_HL_C");

                EmitLabel(st_end);
                return;
            }

            // 2.9 __settilewin(x_u8, y_u8, tile_u8)
            // Writes a tile into the Window tilemap (base chosen by LCDC bit6: 0 => 0x9800, 1 => 0x9C00).
            // Safe VRAM write: if LCD is on, waits for STAT mode 0/1 before writing (DI/EI).
            // Treated as a compiler intrinsic (no C definition required).
            if (funcName == "__settilewin")
            {
                if (args.Length != 3) Error(expr, "__settilewin requires 3 arguments");

                AsmOperand st_end = MakeUniqueLabel("settilewin_end");

                // x -> B (bounds check x < 32)
                CompileIntoA(args[0]);
                EmitAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("JR_NC", st_end);
                EmitAsm("LD_B_A");

                // y -> A (bounds check y < 32)
                CompileIntoA(args[1]);
                EmitAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("JR_NC", st_end);

                // Save y then x (so later argument evaluation can't clobber them)
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // y

                EmitAsm("LD_A_B");
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // x

                // tile -> C
                CompileIntoA(args[2]);
                EmitAsm("LD_C_A");

                // Choose Window tilemap base by LCDC bit6
                AsmOperand st_base0 = MakeUniqueLabel("settilewin_base0");
                AsmOperand st_base_done = MakeUniqueLabel("settilewin_basedone");

                EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
                EmitAsm("AND_IMM", new AsmOperand(0x40, AddressMode.Immediate)); // bit6
                EmitAsm("JR_Z", st_base0);
                EmitAsm("LD_DE_IMM", new AsmOperand(0x9C00, AddressMode.Immediate));
                EmitAsm("JR", st_base_done);
                EmitLabel(st_base0);
                EmitAsm("LD_DE_IMM", new AsmOperand(0x9800, AddressMode.Immediate));
                EmitLabel(st_base_done);

                // Pop x -> B
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("LD_B_A");

                // Pop y -> A
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");

                // HL = y * 32
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");

                // HL += base (from DE)
                EmitAsm("ADD_HL_DE");

                // HL += x (from B) using DE as temp
                EmitAsm("LD_A_B");
                EmitAsm("LD_E_A");
                EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_DE");

                AsmOperand st_write_now = MakeUniqueLabel("settilewin_write_now");
                AsmOperand st_wait = MakeUniqueLabel("settilewin_wait");
                AsmOperand st_done = MakeUniqueLabel("settilewin_done");

                // Check LCD ON (LCDC bit7). If off, write immediately.
                EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
                EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                EmitAsm("JR_Z", st_write_now);

                // LCD on: wait until STAT bit1 cleared (mode 0/1), with DI/EI window
                EmitAsm("DI");
                EmitLabel(st_wait);
                EmitAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem)); // STAT
                EmitAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate));
                EmitAsm("JR_NZ", st_wait);

                EmitAsm("LD_HL_C"); // [HL] = C
                EmitAsm("EI");
                EmitAsm("JR", st_done);

                EmitLabel(st_write_now);
                EmitAsm("LD_HL_C"); // [HL] = C

                EmitLabel(st_done);
                EmitLabel(st_end);
                return;
            }

            // 2.10 __settilewin_unsafe(x_u8, y_u8, tile_u8)
            // ISR / VBlank-only fast path: chooses Window base by LCDC bit6 and writes immediately.
            // Treated as a compiler intrinsic (no C definition required).
            if (funcName == "__settilewin_unsafe")
            {
                if (args.Length != 3) Error(expr, "__settilewin_unsafe requires 3 arguments");

                AsmOperand st_end = MakeUniqueLabel("settilewinu_end");

                // x -> B (bounds check x < 32)
                CompileIntoA(args[0]);
                EmitAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("JR_NC", st_end);
                EmitAsm("LD_B_A");

                // y -> A (bounds check y < 32)
                CompileIntoA(args[1]);
                EmitAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("JR_NC", st_end);

                // Save y then x
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // y

                EmitAsm("LD_A_B");
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // x

                // tile -> C
                CompileIntoA(args[2]);
                EmitAsm("LD_C_A");

                // Choose Window tilemap base by LCDC bit6
                AsmOperand st_base0 = MakeUniqueLabel("settilewinu_base0");
                AsmOperand st_base_done = MakeUniqueLabel("settilewinu_basedone");

                EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
                EmitAsm("AND_IMM", new AsmOperand(0x40, AddressMode.Immediate)); // bit6
                EmitAsm("JR_Z", st_base0);
                EmitAsm("LD_DE_IMM", new AsmOperand(0x9C00, AddressMode.Immediate));
                EmitAsm("JR", st_base_done);
                EmitLabel(st_base0);
                EmitAsm("LD_DE_IMM", new AsmOperand(0x9800, AddressMode.Immediate));
                EmitLabel(st_base_done);

                // Pop x -> B
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("LD_B_A");

                // Pop y -> A
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");

                // HL = y * 32
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");

                // HL += base (DE)
                EmitAsm("ADD_HL_DE");

                // HL += x (B) using DE temp
                EmitAsm("LD_A_B");
                EmitAsm("LD_E_A");
                EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_DE");

                // direct write
                EmitAsm("LD_HL_C");

                EmitLabel(st_end);
                return;
            }


            
            // 2.11 __settilebg(x_u8, y_u8, tile_u8)
            // Writes a tile into the BG tilemap (base chosen by LCDC bit3: 0 => 0x9800, 1 => 0x9C00).
            // Safe VRAM write: if LCD is on, waits for STAT mode 0/1 before writing (DI/EI).
            // Treated as a compiler intrinsic (no C definition required).
            if (funcName == "__settilebg")
            {
                if (args.Length != 3) Error(expr, "__settilebg requires 3 arguments");
            
                AsmOperand st_end = MakeUniqueLabel("settilebg_end");
            
                // x -> B (bounds check x < 32)
                CompileIntoA(args[0]);
                EmitAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("JR_NC", st_end);
                EmitAsm("LD_B_A");
            
                // y -> A (bounds check y < 32)
                CompileIntoA(args[1]);
                EmitAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("JR_NC", st_end);
            
                // Save y then x (so later argument evaluation can't clobber them)
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // y
            
                EmitAsm("LD_A_B");
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // x
            
                // tile -> C
                CompileIntoA(args[2]);
                EmitAsm("LD_C_A");
            
                // Choose BG tilemap base by LCDC bit3
                AsmOperand st_base0 = MakeUniqueLabel("settilebg_base0");
                AsmOperand st_base_done = MakeUniqueLabel("settilebg_basedone");
            
                EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
                EmitAsm("AND_IMM", new AsmOperand(0x08, AddressMode.Immediate)); // bit3
                EmitAsm("JR_Z", st_base0);
                EmitAsm("LD_DE_IMM", new AsmOperand(0x9C00, AddressMode.Immediate));
                EmitAsm("JR", st_base_done);
                EmitLabel(st_base0);
                EmitAsm("LD_DE_IMM", new AsmOperand(0x9800, AddressMode.Immediate));
                EmitLabel(st_base_done);
            
                // Pop x -> B
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("LD_B_A");
            
                // Pop y -> A
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
            
                // HL = y * 32
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
            
                // HL += base (from DE)
                EmitAsm("ADD_HL_DE");
            
                // HL += x (from B) using DE as temp
                EmitAsm("LD_A_B");
                EmitAsm("LD_E_A");
                EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_DE");
            
                AsmOperand st_write_now = MakeUniqueLabel("settilebg_write_now");
                AsmOperand st_wait = MakeUniqueLabel("settilebg_wait");
                AsmOperand st_done = MakeUniqueLabel("settilebg_done");
            
                // Check LCD ON (LCDC bit7). If off, write immediately.
                EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
                EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                EmitAsm("JR_Z", st_write_now);
            
                // LCD on: wait until STAT bit1 cleared (mode 0/1), with DI/EI window
                EmitAsm("DI");
                EmitLabel(st_wait);
                EmitAsm("LDH_A_MEM", new AsmOperand(0x41, AddressMode.HighMem)); // STAT
                EmitAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate));
                EmitAsm("JR_NZ", st_wait);
            
                EmitAsm("LD_HL_C"); // [HL] = C
                EmitAsm("EI");
                EmitAsm("JR", st_done);
            
                EmitLabel(st_write_now);
                EmitAsm("LD_HL_C"); // [HL] = C
            
                EmitLabel(st_done);
                EmitLabel(st_end);
                return;
            }
            
            // 2.12 __settilebg_unsafe(x_u8, y_u8, tile_u8)
            // ISR / VBlank-only fast path: chooses BG tilemap base by LCDC bit3 and writes immediately.
            // Treated as a compiler intrinsic (no C definition required).
            if (funcName == "__settilebg_unsafe")
            {
                if (args.Length != 3) Error(expr, "__settilebg_unsafe requires 3 arguments");
            
                AsmOperand st_end = MakeUniqueLabel("settilebgu_end");
            
                // x -> B (bounds check x < 32)
                CompileIntoA(args[0]);
                EmitAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("JR_NC", st_end);
                EmitAsm("LD_B_A");
            
                // y -> A (bounds check y < 32)
                CompileIntoA(args[1]);
                EmitAsm("CP_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("JR_NC", st_end);
            
                // Save y then x
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // y
            
                EmitAsm("LD_A_B");
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("PUSH_HL"); // x
            
                // tile -> C
                CompileIntoA(args[2]);
                EmitAsm("LD_C_A");
            
                // Choose BG tilemap base by LCDC bit3
                AsmOperand st_base0 = MakeUniqueLabel("settilebgu_base0");
                AsmOperand st_base_done = MakeUniqueLabel("settilebgu_basedone");
            
                EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
                EmitAsm("AND_IMM", new AsmOperand(0x08, AddressMode.Immediate)); // bit3
                EmitAsm("JR_Z", st_base0);
                EmitAsm("LD_DE_IMM", new AsmOperand(0x9C00, AddressMode.Immediate));
                EmitAsm("JR", st_base_done);
                EmitLabel(st_base0);
                EmitAsm("LD_DE_IMM", new AsmOperand(0x9800, AddressMode.Immediate));
                EmitLabel(st_base_done);
            
                // Pop x -> B
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("LD_B_A");
            
                // Pop y -> A
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
            
                // HL = y * 32
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
            
                // HL += base (DE)
                EmitAsm("ADD_HL_DE");
            
                // HL += x (B) using DE temp
                EmitAsm("LD_A_B");
                EmitAsm("LD_E_A");
                EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_DE");
            
                // direct write
                EmitAsm("LD_HL_C");
            
                EmitLabel(st_end);
                return;
            }

            // Stage the rectangle arguments and use the BG map selected by LCDC for row fills.
            if (funcName == "__settile_rect")
            {
                if (args.Length != 5) Error(expr, "__settile_rect(x, y, w, h, tile) expects 5 arguments");

                EmitPushExprAsWord(args[0]); // x
                EmitPushExprAsWord(args[1]); // y
                EmitPushExprAsWord(args[2]); // w
                EmitPushExprAsWord(args[3]); // h
                EmitPushExprAsWord(args[4]); // tile

                AsmOperand loop = MakeUniqueLabel("settilerect_loop");
                AsmOperand done = MakeUniqueLabel("settilerect_done");

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("OR_A");
                EmitAsm("JP_Z", done);
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(2, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("OR_A");
                EmitAsm("JP_Z", done);

                EmitLoadTileMapBaseToHl(0x08, "settilerect_base");
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(8, AddressMode.Relative));
                EmitAsm("LD_A_HL"); // y
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("POP_DE");
                EmitAsm("ADD_HL_DE");
                // Preserve the VRAM destination while retrieving the saved X coordinate.
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(10, AddressMode.Relative));
                EmitAsm("LD_A_HL"); // x
                EmitAsm("LD_C_A");
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("ADD_C");
                EmitAsm("LD_L_A");
                EmitAsm("LD_A_H");
                EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("LD_H_A");

                EmitLabel(loop);
                // Preserve the row destination while updating height and loading row arguments.
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("DEC_A");
                EmitAsm("LD_HL_A");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(2, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A"); // tile
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_C_A");
                EmitAsm("LD_B_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("POP_HL");
                EmitVramMemsetLoop(true, "settilerect_fill");

                // Restore the end-of-row pointer before testing height or adding the row stride.
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("OR_A");
                EmitAsm("POP_HL");
                EmitAsm("JP_Z", done);
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_C_A");
                EmitAsm("POP_HL");
                EmitAsm("LD_A_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("SUB_C");
                EmitAsm("LD_C_A");
                EmitAsm("LD_A_L");
                EmitAsm("ADD_C");
                EmitAsm("LD_L_A");
                EmitAsm("LD_A_H");
                EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("LD_H_A");
                EmitAsm("JP", loop);

                EmitLabel(done);
                EmitAsm("ADD_SP_IMM", new AsmOperand(10, AddressMode.Relative));
                return;
            }

            // Stage a source pointer and byte length for a horizontal copy into the selected BG map.
            if (funcName == "__settile_row")
            {
                if (args.Length != 4) Error(expr, "__settile_row(x, y, src, len) expects 4 arguments");

                Expr rowLen = FoldConstants(args[3]);

                EmitPushExprAsWord(args[0]); // x
                EmitPushExprAsWord(args[1]); // y
                EmitPushExprAsWideWord(args[2]); // src
                EmitPushExprAsWord(rowLen); // len

                AsmOperand done = MakeUniqueLabel("settilerow_done");

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("OR_A");
                EmitAsm("JP_Z", done);

                EmitLoadTileMapBaseToHl(0x08, "settilerow_base");
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                EmitAsm("LD_A_HL"); // y
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("POP_DE");
                EmitAsm("ADD_HL_DE");
                // Preserve the VRAM destination while retrieving the saved X coordinate.
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(8, AddressMode.Relative));
                EmitAsm("LD_A_HL"); // x
                EmitAsm("LD_C_A");
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("ADD_C");
                EmitAsm("LD_L_A");
                EmitAsm("LD_A_H");
                EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("LD_H_A");

                // Stack-address loads use HL; retain the destination while restoring DE.
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A"); // src
                EmitAsm("POP_HL");

                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(2, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_C_A");
                EmitAsm("LD_B_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("POP_HL");
                if (!(rowLen.Match(Tag.Integer, out int rowLenConst) &&
                      TryEmitVramMemcpyConstCount(true, rowLenConst & 0xFF, "settilerow_small")))
                {
                    EmitVramMemcpyLoop(true, "settilerow_copy");
                }

                EmitLabel(done);
                EmitAsm("ADD_SP_IMM", new AsmOperand(8, AddressMode.Relative));
                return;
            }

            // Stage the column arguments and select an unrolled or iterative strided copy.
            if (funcName == "__settile_col")
            {
                if (args.Length != 4) Error(expr, "__settile_col(x, y, src, len) expects 4 arguments");

                Expr colLen = FoldConstants(args[3]);

                EmitPushExprAsWord(args[0]); // x
                EmitPushExprAsWord(args[1]); // y
                EmitPushExprAsWideWord(args[2]); // src
                EmitPushExprAsWord(colLen); // len

                AsmOperand loop = MakeUniqueLabel("settilecol_loop");
                AsmOperand done = MakeUniqueLabel("settilecol_done");

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(0, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("OR_A");
                EmitAsm("JP_Z", done);

                EmitLoadTileMapBaseToHl(0x08, "settilecol_base");
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                EmitAsm("LD_A_HL"); // y
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("POP_DE");
                EmitAsm("ADD_HL_DE");
                // Preserve the VRAM destination while retrieving the saved X coordinate.
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(8, AddressMode.Relative));
                EmitAsm("LD_A_HL"); // x
                EmitAsm("LD_C_A");
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("ADD_C");
                EmitAsm("LD_L_A");
                EmitAsm("LD_A_H");
                EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("LD_H_A");

                // Stack-address loads use HL; retain the destination while restoring DE.
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A"); // src
                EmitAsm("POP_HL");

                if (colLen.Match(Tag.Integer, out int colLenConst) &&
                    TryEmitVramColumnCopyConstCount(true, colLenConst & 0xFF, "settilecol_small"))
                {
                    EmitLabel(done);
                    EmitAsm("ADD_SP_IMM", new AsmOperand(8, AddressMode.Relative));
                    return;
                }

                EmitLabel(loop);
                // The initial guard and loop-tail check ensure a nonzero remaining count.
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(2, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("DEC_A");
                EmitAsm("LD_HL_A");
                EmitAsm("POP_HL");

                EmitAsm("LD_A_DE");
                EmitAsm("LD_C_A");
                EmitVramStoreCToHl(true, "settilecol_store");
                EmitAsm("INC_DE");

                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(2, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("OR_A");
                EmitAsm("POP_HL");
                EmitAsm("JP_Z", done);
                EmitAsm("LD_A_L");
                EmitAsm("ADD_A_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("LD_L_A");
                EmitAsm("LD_A_H");
                EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("LD_H_A");
                EmitAsm("JP", loop);

                EmitLabel(done);
                EmitAsm("ADD_SP_IMM", new AsmOperand(8, AddressMode.Relative));
                return;
            }

            // Specialize single rows or columns before emitting the general explicit-map rectangle copy.
            if (funcName == "__settilemap_rect")
            {
                if (args.Length != 6) Error(expr, "__settilemap_rect(base, x, y, w, h, src) expects 6 arguments");

                Expr rectW = FoldConstants(args[3]);
                Expr rectH = FoldConstants(args[4]);
                if (rectH.Match(Tag.Integer, out int hConstRect) && ((hConstRect & 0xFF) == 1))
                {
                    EmitLengthExprIntoBC(rectW);
                    EmitAsm("PUSH_BC");
                    CompileIntoHL(args[5]);
                    EmitAsm("PUSH_HL");
                    EmitTileMapRectAddressIntoHL(args[0], args[1], args[2]);
                    EmitAsm("POP_DE");
                    EmitAsm("POP_BC");
                    if (!(rectW.Match(Tag.Integer, out int rowRectLenConst) &&
                          TryEmitVramMemcpyConstCount(true, rowRectLenConst & 0xFF, "settilemaprect_row_small")))
                    {
                        EmitVramMemcpyLoop(true, "settilemaprect_row");
                    }
                    return;
                }

                if (rectW.Match(Tag.Integer, out int wConstRect) && ((wConstRect & 0xFF) == 1))
                {
                    CompileIntoA(rectH);
                    EmitAsm("LD_B_A");
                    CompileIntoHL(args[5]);
                    EmitAsm("PUSH_HL");
                    EmitTileMapRectAddressIntoHL(args[0], args[1], args[2]);
                    EmitAsm("POP_DE");

                    if (rectH.Match(Tag.Integer, out int colRectLenConst) &&
                        TryEmitVramColumnCopyConstCount(true, colRectLenConst & 0xFF, "settilemaprect_col_small"))
                        return;

                    AsmOperand colLoop = MakeUniqueLabel("settilemaprect_col_loop");
                    AsmOperand colDone = MakeUniqueLabel("settilemaprect_col_done");
                    EmitLabel(colLoop);
                    EmitAsm("LD_A_B");
                    EmitAsm("OR_A");
                    EmitAsm("JR_Z", colDone);
                    EmitAsm("DEC_B");
                    EmitAsm("LD_A_DE");
                    EmitAsm("LD_C_A");
                    EmitVramStoreCToHl(true, "settilemaprect_col_store");
                    EmitAsm("INC_DE");
                    EmitAsm("LD_A_L");
                    EmitAsm("ADD_A_IMM", new AsmOperand(32, AddressMode.Immediate));
                    EmitAsm("LD_L_A");
                    EmitAsm("LD_A_H");
                    EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
                    EmitAsm("LD_H_A");
                    EmitAsm("JR", colLoop);
                    EmitLabel(colDone);
                    return;
                }

                EmitPushExprAsWideWord(args[0]); // base
                EmitPushExprAsWord(args[1]); // x
                EmitPushExprAsWord(args[2]); // y
                EmitPushExprAsWord(args[3]); // w
                EmitPushExprAsWord(args[4]); // h
                EmitPushExprAsWideWord(args[5]); // src

                AsmOperand loop = MakeUniqueLabel("settilemaprect_loop");
                AsmOperand done = MakeUniqueLabel("settilemaprect_done");

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("OR_A");
                EmitAsm("JP_Z", done);
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(2, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("OR_A");
                EmitAsm("JP_Z", done);

                EmitAsm("LD_HL_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                EmitAsm("LD_A_HL"); // y
                EmitAsm("LD_L_A");
                EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("ADD_HL_HL");
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(12, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A"); // base
                EmitAsm("POP_HL");
                EmitAsm("ADD_HL_DE");
                // Preserve the VRAM destination while retrieving the saved X coordinate.
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(10, AddressMode.Relative));
                EmitAsm("LD_A_HL"); // x
                EmitAsm("LD_C_A");
                EmitAsm("POP_HL");
                EmitAsm("LD_A_L");
                EmitAsm("ADD_C");
                EmitAsm("LD_L_A");
                EmitAsm("LD_A_H");
                EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("LD_H_A");

                // Stack-address loads use HL; retain the destination while restoring DE.
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(2, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A"); // src
                EmitAsm("POP_HL");

                EmitLabel(loop);
                // Preserve the row destination while updating height and loading row arguments.
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("DEC_A");
                EmitAsm("LD_HL_A");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_C_A");
                EmitAsm("LD_B_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("POP_HL");
                EmitVramMemcpyLoop(true, "settilemaprect_copy");

                // Restore the end-of-row pointer before testing height or adding the row stride.
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(4, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("OR_A");
                EmitAsm("POP_HL");
                EmitAsm("JP_Z", done);
                EmitAsm("PUSH_HL");
                EmitAsm("LD_HL_SP_IMM", new AsmOperand(6, AddressMode.Relative));
                EmitAsm("LD_A_HL");
                EmitAsm("LD_C_A");
                EmitAsm("POP_HL");
                EmitAsm("LD_A_IMM", new AsmOperand(32, AddressMode.Immediate));
                EmitAsm("SUB_C");
                EmitAsm("LD_C_A");
                EmitAsm("LD_A_L");
                EmitAsm("ADD_C");
                EmitAsm("LD_L_A");
                EmitAsm("LD_A_H");
                EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
                EmitAsm("LD_H_A");
                EmitAsm("JP", loop);

                EmitLabel(done);
                EmitAsm("ADD_SP_IMM", new AsmOperand(12, AddressMode.Relative));
                return;
            }

            // Dispatch a safe attribute-only write through the CGB-aware scalar helper.
            if (funcName == "__settileattr")
            {
                if (args.Length != 3) Error(expr, "__settileattr requires 3 arguments");
                EmitSetTileAttrIntrinsic(args[0], args[1], args[2], fast: false);
                return;
            }

            // Dispatch an attribute-only write with VRAM access timing supplied by the caller.
            if (funcName == "__settileattr_unsafe")
            {
                if (args.Length != 3) Error(expr, "__settileattr_unsafe requires 3 arguments");
                EmitSetTileAttrIntrinsic(args[0], args[1], args[2], fast: true);
                return;
            }

            // Dispatch a tile and attribute pair through the safe CGB-aware helper.
            if (funcName == "__settilecgb")
            {
                if (args.Length != 4) Error(expr, "__settilecgb requires 4 arguments");
                EmitSetTileCgbIntrinsic(args[0], args[1], args[2], args[3], fast: false);
                return;
            }

            // Dispatch a tile and attribute pair without waiting for accessible VRAM.
            if (funcName == "__settilecgb_unsafe")
            {
                if (args.Length != 4) Error(expr, "__settilecgb_unsafe requires 4 arguments");
                EmitSetTileCgbIntrinsic(args[0], args[1], args[2], args[3], fast: true);
                return;
            }

            // Pass an explicit map base to the safe attribute-target helper.
            if (funcName == "__settileatattr")
            {
                if (args.Length != 4) Error(expr, "__settileatattr requires 4 arguments");
                EmitTileTargetAttrIntrinsic(CgbTileTargetKind.At, args[0], args[1], args[2], args[3], safe: true, labelPrefix: "settileatattr");
                return;
            }

            // Pass an explicit map base to the attribute-target helper without VRAM timing waits.
            if (funcName == "__settileatattr_unsafe")
            {
                if (args.Length != 4) Error(expr, "__settileatattr_unsafe requires 4 arguments");
                EmitTileTargetAttrIntrinsic(CgbTileTargetKind.At, args[0], args[1], args[2], args[3], safe: false, labelPrefix: "settileatattru");
                return;
            }

            // Use an explicit map base for a safe tile and attribute write.
            if (funcName == "__settileatcgb")
            {
                if (args.Length != 5) Error(expr, "__settileatcgb requires 5 arguments");
                EmitTileTargetCgbIntrinsic(CgbTileTargetKind.At, args[0], args[1], args[2], args[3], args[4], safe: true, labelPrefix: "settileatcgb");
                return;
            }

            // Use an explicit map base for a tile and attribute write at caller-controlled timing.
            if (funcName == "__settileatcgb_unsafe")
            {
                if (args.Length != 5) Error(expr, "__settileatcgb_unsafe requires 5 arguments");
                EmitTileTargetCgbIntrinsic(CgbTileTargetKind.At, args[0], args[1], args[2], args[3], args[4], safe: false, labelPrefix: "settileatcgbu");
                return;
            }

            // Select the Window map when emitting a safe attribute write.
            if (funcName == "__settilewinattr")
            {
                if (args.Length != 3) Error(expr, "__settilewinattr requires 3 arguments");
                EmitTileTargetAttrIntrinsic(CgbTileTargetKind.Window, null, args[0], args[1], args[2], safe: true, labelPrefix: "settilewinattr");
                return;
            }

            // Select the Window map for an attribute write without a VRAM wait.
            if (funcName == "__settilewinattr_unsafe")
            {
                if (args.Length != 3) Error(expr, "__settilewinattr_unsafe requires 3 arguments");
                EmitTileTargetAttrIntrinsic(CgbTileTargetKind.Window, null, args[0], args[1], args[2], safe: false, labelPrefix: "settilewinattru");
                return;
            }

            // Select the Window map for a safe combined tile and attribute write.
            if (funcName == "__settilewincgb")
            {
                if (args.Length != 4) Error(expr, "__settilewincgb requires 4 arguments");
                EmitTileTargetCgbIntrinsic(CgbTileTargetKind.Window, null, args[0], args[1], args[2], args[3], safe: true, labelPrefix: "settilewincgb");
                return;
            }

            // Select the Window map for a combined write at caller-controlled timing.
            if (funcName == "__settilewincgb_unsafe")
            {
                if (args.Length != 4) Error(expr, "__settilewincgb_unsafe requires 4 arguments");
                EmitTileTargetCgbIntrinsic(CgbTileTargetKind.Window, null, args[0], args[1], args[2], args[3], safe: false, labelPrefix: "settilewincgbu");
                return;
            }

            // Select the displayed BG map when emitting a safe attribute write.
            if (funcName == "__settilebgattr")
            {
                if (args.Length != 3) Error(expr, "__settilebgattr requires 3 arguments");
                EmitTileTargetAttrIntrinsic(CgbTileTargetKind.Bg, null, args[0], args[1], args[2], safe: true, labelPrefix: "settilebgattr");
                return;
            }

            // Select the displayed BG map for an attribute write without a VRAM wait.
            if (funcName == "__settilebgattr_unsafe")
            {
                if (args.Length != 3) Error(expr, "__settilebgattr_unsafe requires 3 arguments");
                EmitTileTargetAttrIntrinsic(CgbTileTargetKind.Bg, null, args[0], args[1], args[2], safe: false, labelPrefix: "settilebgattru");
                return;
            }

            // Select the displayed BG map for a safe combined tile and attribute write.
            if (funcName == "__settilebgcgb")
            {
                if (args.Length != 4) Error(expr, "__settilebgcgb requires 4 arguments");
                EmitTileTargetCgbIntrinsic(CgbTileTargetKind.Bg, null, args[0], args[1], args[2], args[3], safe: true, labelPrefix: "settilebgcgb");
                return;
            }

            // Select the displayed BG map for a combined write at caller-controlled timing.
            if (funcName == "__settilebgcgb_unsafe")
            {
                if (args.Length != 4) Error(expr, "__settilebgcgb_unsafe requires 4 arguments");
                EmitTileTargetCgbIntrinsic(CgbTileTargetKind.Bg, null, args[0], args[1], args[2], args[3], safe: false, labelPrefix: "settilebgcgbu");
                return;
            }

            // Dispatch an attribute-buffer transfer with safe VRAM timing.
            if (funcName == "__settileattr_bulk")
            {
                if (args.Length != 3) Error(expr, "__settileattr_bulk requires 3 arguments");
                EmitSetTileAttrBulkIntrinsic(args[0], args[1], args[2], fast: false, labelPrefix: "settileattrbulk");
                return;
            }

            // Dispatch an attribute-buffer transfer whose timing is controlled by the caller.
            if (funcName == "__settileattr_bulk_fast")
            {
                if (args.Length != 3) Error(expr, "__settileattr_bulk_fast requires 3 arguments");
                EmitSetTileAttrBulkIntrinsic(args[0], args[1], args[2], fast: true, labelPrefix: "settileattrbulkf");
                return;
            }

            // Transfer tile and attribute buffers through the safe combined bulk helper.
            if (funcName == "__settilecgb_bulk")
            {
                if (args.Length != 4) Error(expr, "__settilecgb_bulk requires 4 arguments");
                EmitSetTileCgbBulkIntrinsic(args[0], args[1], args[2], args[3], fast: false, labelPrefix: "settilecgbbulk");
                return;
            }

            // Transfer tile and attribute buffers without VRAM timing waits.
            if (funcName == "__settilecgb_bulk_fast")
            {
                if (args.Length != 4) Error(expr, "__settilecgb_bulk_fast requires 4 arguments");
                EmitSetTileCgbBulkIntrinsic(args[0], args[1], args[2], args[3], fast: true, labelPrefix: "settilecgbbulkf");
                return;
            }

            // Write four consecutive tile numbers into a two-by-two cell in the RAM map buffer.
            if (funcName == "__settilebg16_buf")
            {
                if (args.Length != 4) Error(expr, "__settilebg16_buf requires 4 arguments");
                EmitTile16BufferedWriteIntrinsic(args[0], args[1], args[2], args[3], incrementQuad: true, labelPrefix: "settilebg16buf");
                return;
            }

            // Update tile and attribute RAM buffers separately, repeating the attribute across the cell.
            if (funcName == "__settilebg16cgb_buf")
            {
                if (args.Length != 6) Error(expr, "__settilebg16cgb_buf requires 6 arguments");
                EmitTile16BufferedWriteIntrinsic(args[0], args[2], args[3], args[4], incrementQuad: true, labelPrefix: "settilebg16cgbbuf_tile");
                EmitTile16BufferedWriteIntrinsic(args[1], args[2], args[3], args[5], incrementQuad: false, labelPrefix: "settilebg16cgbbuf_attr");
                return;
            }

            // Flush both tile rows belonging to the requested buffered cell row.
            if (funcName == "__settilebg16_flush")
            {
                if (args.Length != 4) Error(expr, "__settilebg16_flush requires 4 arguments");
                EmitTile16FlushRowsIntrinsic(args[0], args[1], args[2], args[3], cgbAttr: false, labelPrefix: "settilebg16flush");
                return;
            }

            // Flush the tile buffer first and copy attributes only when CGB hardware is available.
            if (funcName == "__settilebg16cgb_flush")
            {
                if (args.Length != 5) Error(expr, "__settilebg16cgb_flush requires 5 arguments");
                EmitTile16FlushRowsIntrinsic(args[0], args[2], args[3], args[4], cgbAttr: false, labelPrefix: "settilebg16cgbflush_tile");
                if (IsKnownCgbRuntimeTrue())
                {
                    EmitSetVbkUnchecked(1);
                    EmitTile16FlushRowsIntrinsic(args[1], args[2], args[3], args[4], cgbAttr: true, labelPrefix: "settilebg16cgbflush_attr");
                }
                else
                {
                    AsmOperand skip = MakeUniqueLabel("settilebg16cgbflush_skip");
                    AsmOperand done = MakeUniqueLabel("settilebg16cgbflush_done");
                    EmitRuntimeCgbCheckCall();
                    EmitAsm("OR_A");
                    // Two unrolled row copies can exceed the relative-branch range.
                    EmitAsm("JP_Z", skip);
                    EmitSetVbkUnchecked(1);
                    EmitTile16FlushRowsIntrinsic(args[1], args[2], args[3], args[4], cgbAttr: true, labelPrefix: "settilebg16cgbflush_attr");
                    EmitAsm("JR", done);
                    EmitLabel(skip);
                    EmitLabel(done);
                }
                return;
            }
            
            // 2.14 __getbgmapbase() -> u16 in HL
// Returns the BG tilemap base address selected by LCDC bit3:
// bit3=0 => 0x9800, bit3=1 => 0x9C00
// Treated as a compiler intrinsic (no C definition required).
if (funcName == "__getbgmapbase")
{
    if (args.Length != 0) Error(expr, "__getbgmapbase requires 0 arguments");

    AsmOperand gb_base0 = MakeUniqueLabel("getbgbase_base0");
    AsmOperand gb_done  = MakeUniqueLabel("getbgbase_done");

    EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
    EmitAsm("AND_IMM", new AsmOperand(0x08, AddressMode.Immediate)); // bit3 (BG tile map display select)
    EmitAsm("JR_Z", gb_base0);
    EmitAsm("LD_HL_IMM", new AsmOperand(0x9C00, AddressMode.Immediate16));
    EmitAsm("JR", gb_done);
    EmitLabel(gb_base0);
    EmitAsm("LD_HL_IMM", new AsmOperand(0x9800, AddressMode.Immediate16));
    EmitLabel(gb_done);
    LastCallReturnsHL = true;
    return;
}

// 2.15 __getwinmapbase() -> u16 in HL
// Returns the Window tilemap base address selected by LCDC bit6:
// bit6=0 => 0x9800, bit6=1 => 0x9C00
// Treated as a compiler intrinsic (no C definition required).
if (funcName == "__getwinmapbase")
{
    if (args.Length != 0) Error(expr, "__getwinmapbase requires 0 arguments");

    AsmOperand win_base0 = MakeUniqueLabel("getwinbase_base0");
    AsmOperand win_done  = MakeUniqueLabel("getwinbase_done");

    EmitAsm("LDH_A_MEM", new AsmOperand(0x40, AddressMode.HighMem)); // LCDC
    EmitAsm("AND_IMM", new AsmOperand(0x40, AddressMode.Immediate)); // bit6 (Window tile map display select)
    EmitAsm("JR_Z", win_base0);
    EmitAsm("LD_HL_IMM", new AsmOperand(0x9C00, AddressMode.Immediate16));
    EmitAsm("JR", win_done);
    EmitLabel(win_base0);
    EmitAsm("LD_HL_IMM", new AsmOperand(0x9800, AddressMode.Immediate16));
    EmitLabel(win_done);
    LastCallReturnsHL = true;
    return;
}



// 2.16 __readpad() -> u8 in A
// Returns merged key bitfield:
// low nibble (bits0..3): D-Pad (Right,Left,Up,Down)
// high nibble(bits4..7): Buttons(A,B,Select,Start)
// Each bit is 1 when pressed.
// Treated as a compiler intrinsic (no C definition required).
if (funcName == "__readpad")
{
    if (args.Length != 0) Error(expr, "__readpad requires 0 arguments");

    AsmOperand p1 = new AsmOperand(0x00, AddressMode.HighMem); // P1 / JOYP

    // Directions (P1=0x20)
    EmitAsm("LD_A_IMM", new AsmOperand(0x20, AddressMode.Immediate));
    EmitAsm("LDH_MEM_A", p1);
    EmitAsm("LDH_A_MEM", p1); // dummy reads for settle
    EmitAsm("LDH_A_MEM", p1);
    EmitAsm("CPL");
    EmitAsm("AND_IMM", new AsmOperand(0x0F, AddressMode.Immediate));
    EmitAsm("LD_C_A"); // dpad in C

    // Buttons (P1=0x10)
    EmitAsm("LD_A_IMM", new AsmOperand(0x10, AddressMode.Immediate));
    EmitAsm("LDH_MEM_A", p1);
    EmitAsm("LDH_A_MEM", p1);
    EmitAsm("LDH_A_MEM", p1);
    EmitAsm("CPL");
    EmitAsm("AND_IMM", new AsmOperand(0x0F, AddressMode.Immediate));
    // shift into upper nibble (<<4)
    EmitAsm("RLCA"); EmitAsm("RLCA"); EmitAsm("RLCA"); EmitAsm("RLCA");
    EmitAsm("OR_C");

    // Restore P1 selection (0x30) without losing return value
    EmitAsm("LD_C_A");
    EmitAsm("LD_A_IMM", new AsmOperand(0x30, AddressMode.Immediate));
    EmitAsm("LDH_MEM_A", p1);
    EmitAsm("LD_A_C");
    return;
}

// 2.17 __readpaddir() -> u8 in A (bits0..3 only)
// Returns dpad nibble (Right,Left,Up,Down) with 1 when pressed.
// Treated as a compiler intrinsic (no C definition required).
if (funcName == "__readpaddir")
{
    if (args.Length != 0) Error(expr, "__readpaddir requires 0 arguments");

    AsmOperand p1 = new AsmOperand(0x00, AddressMode.HighMem); // P1 / JOYP

    EmitAsm("LD_A_IMM", new AsmOperand(0x20, AddressMode.Immediate));
    EmitAsm("LDH_MEM_A", p1);
    EmitAsm("LDH_A_MEM", p1);
    EmitAsm("LDH_A_MEM", p1);
    EmitAsm("CPL");
    EmitAsm("AND_IMM", new AsmOperand(0x0F, AddressMode.Immediate));

    EmitAsm("LD_C_A");
    EmitAsm("LD_A_IMM", new AsmOperand(0x30, AddressMode.Immediate));
    EmitAsm("LDH_MEM_A", p1);
    EmitAsm("LD_A_C");
    return;
}

// 2.18 __readpadbtn() -> u8 in A (bits0..3 only)
// Returns buttons nibble (A,B,Select,Start) with 1 when pressed.
// Treated as a compiler intrinsic (no C definition required).
if (funcName == "__readpadbtn")
{
    if (args.Length != 0) Error(expr, "__readpadbtn requires 0 arguments");

    AsmOperand p1 = new AsmOperand(0x00, AddressMode.HighMem); // P1 / JOYP

    EmitAsm("LD_A_IMM", new AsmOperand(0x10, AddressMode.Immediate));
    EmitAsm("LDH_MEM_A", p1);
    EmitAsm("LDH_A_MEM", p1);
    EmitAsm("LDH_A_MEM", p1);
    EmitAsm("CPL");
    EmitAsm("AND_IMM", new AsmOperand(0x0F, AddressMode.Immediate));

    EmitAsm("LD_C_A");
    EmitAsm("LD_A_IMM", new AsmOperand(0x30, AddressMode.Immediate));
    EmitAsm("LDH_MEM_A", p1);
    EmitAsm("LD_A_C");
    return;
}

// 2.19 __readpadex(prev_u8) -> u16 in HL
// Returns: L = keys (same encoding as __readpad), H = trigger (newly pressed edge: keys & ~prev).
// This mirrors the common pattern:
// prev = keys; keys = __readpad(); trigger = keys & ~prev;
// Treated as a compiler intrinsic (no C definition required).
if (funcName == "__readpadex")
{
    if (args.Length != 1) Error(expr, "__readpadex requires 1 argument (prev_keys)");

    // prev -> B
    CompileIntoA(args[0]);
    EmitAsm("LD_B_A");

    AsmOperand p1 = new AsmOperand(0x00, AddressMode.HighMem); // P1 / JOYP

    // Directions (P1=0x20)
    EmitAsm("LD_A_IMM", new AsmOperand(0x20, AddressMode.Immediate));
    EmitAsm("LDH_MEM_A", p1);
    EmitAsm("LDH_A_MEM", p1);
    EmitAsm("LDH_A_MEM", p1);
    EmitAsm("CPL");
    EmitAsm("AND_IMM", new AsmOperand(0x0F, AddressMode.Immediate));
    EmitAsm("LD_C_A"); // dpad in C

    // Buttons (P1=0x10)
    EmitAsm("LD_A_IMM", new AsmOperand(0x10, AddressMode.Immediate));
    EmitAsm("LDH_MEM_A", p1);
    EmitAsm("LDH_A_MEM", p1);
    EmitAsm("LDH_A_MEM", p1);
    EmitAsm("CPL");
    EmitAsm("AND_IMM", new AsmOperand(0x0F, AddressMode.Immediate));
    EmitAsm("RLCA"); EmitAsm("RLCA"); EmitAsm("RLCA"); EmitAsm("RLCA");
    EmitAsm("OR_C");

    // L = keys
    EmitAsm("LD_L_A");

    // Restore P1=0x30
    EmitAsm("LD_A_IMM", new AsmOperand(0x30, AddressMode.Immediate));
    EmitAsm("LDH_MEM_A", p1);

    // trigger = keys & ~prev
    EmitAsm("LD_A_B");
    EmitAsm("CPL");
    EmitAsm("LD_B_A");
    EmitAsm("LD_A_L");
    EmitAsm("AND_B");
    EmitAsm("LD_H_A");
    LastCallReturnsHL = true;
    return;
}




// 2.20 __padrep_init(state_ptr, das_u8, arr_u8) -> void
// Initializes a 5-byte PadRepeatState:
// [0]=dirMask (0,0x01,0x02), [1]=dasFrames, [2]=arrFrames, [3]=dasCnt, [4]=arrCnt
// Treated as a compiler intrinsic (no C definition required).
if (funcName == "__padrep_init")
{
    if (args.Length != 3) Error(expr, "__padrep_init requires 3 arguments (state*, das, arr)");

    // Evaluate args with register safety
    CompileIntoHL(args[0]);
    EmitAsm("PUSH_HL");
    CompileIntoA(args[1]); EmitAsm("LD_B_A"); // das
    CompileIntoA(args[2]); EmitAsm("LD_C_A"); // arr
    EmitAsm("POP_HL");

    // dirMask = 0; dasCnt=0; arrCnt=0; store params
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // [0]=0
    EmitAsm("INC_HL");
    EmitAsm("LD_A_B");
    EmitAsm("LD_HL_A"); // [1]=das
    EmitAsm("INC_HL");
    EmitAsm("LD_A_C");
    EmitAsm("LD_HL_A"); // [2]=arr
    EmitAsm("INC_HL");
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // [3]=dasCnt
    EmitAsm("INC_HL");
    EmitAsm("LD_HL_A"); // [4]=arrCnt
    return;
}

// 2.21 __padrep_reset(state_ptr) -> void
// Resets runtime counters for PadRepeatState while keeping das/arr params:
// dirMask=0, dasCnt=0, arrCnt=0
// Treated as a compiler intrinsic (no C definition required).
if (funcName == "__padrep_reset")
{
    if (args.Length != 1) Error(expr, "__padrep_reset requires 1 argument (state*)");

    CompileIntoHL(args[0]);

    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // [0]=0 (dirMask)

    // [3]=0, [4]=0
    EmitAsm("PUSH_HL");
    EmitAsm("INC_HL"); EmitAsm("INC_HL"); EmitAsm("INC_HL"); // -> [3]
    EmitAsm("LD_HL_A");
    EmitAsm("INC_HL"); // -> [4]
    EmitAsm("LD_HL_A");
    EmitAsm("POP_HL");
    return;
}

// __padrep_lr(state*, keys, trigger) returns Right (1), Left (2), or zero.
// New triggers take priority, with Right selected first when both trigger bits are set.
// Without a new trigger, simultaneous Left/Right holds suppress movement without resetting counters.
// A single held direction is adopted without immediate movement when no direction is active.
// Subsequent updates use the five-byte state's initial delay and repeat interval; ARR=0 repeats each update.
if (funcName == "__padrep_lr")
{
    if (args.Length != 3) Error(expr, "__padrep_lr requires 3 arguments (state*, keys, trigger)");

    AsmOperand pr_done        = MakeUniqueLabel("padrep_done");
    AsmOperand pr_ret0        = MakeUniqueLabel("padrep_ret0");
    AsmOperand pr_check_left  = MakeUniqueLabel("padrep_chkL");
    AsmOperand pr_no_trig     = MakeUniqueLabel("padrep_notrig");
    AsmOperand pr_reset_all   = MakeUniqueLabel("padrep_reset");
    AsmOperand pr_dir0        = MakeUniqueLabel("padrep_dir0");
    AsmOperand pr_have_dir    = MakeUniqueLabel("padrep_havedir");
    AsmOperand pr_first_rep   = MakeUniqueLabel("padrep_firstrep");
    AsmOperand pr_ret_dir     = MakeUniqueLabel("padrep_retdir");
    AsmOperand pr_skip_dasinc = MakeUniqueLabel("padrep_skipdasinc");
    AsmOperand pr_skip_arrinc = MakeUniqueLabel("padrep_skiparrinc");

    // Evaluate args safely:
    // HL = state*, B = keys, C = trigger
    CompileIntoHL(args[0]);
    EmitAsm("PUSH_HL");
    CompileIntoA(args[1]); EmitAsm("LD_B_A"); // keys
    CompileIntoA(args[2]); EmitAsm("LD_C_A"); // trigger
    EmitAsm("POP_HL");

    // --- Trigger Right? (bit0) ---
    EmitAsm("LD_A_C");
    EmitAsm("AND_IMM", new AsmOperand(0x01, AddressMode.Immediate));
    EmitAsm("JP_Z", pr_check_left);

    // dirMask=0x01, dasCnt=0, arrCnt=0
    EmitAsm("LD_A_IMM", new AsmOperand(0x01, AddressMode.Immediate));
    EmitAsm("LD_HL_A"); // [0]=0x01
    EmitAsm("PUSH_HL");
    EmitAsm("INC_HL"); EmitAsm("INC_HL"); EmitAsm("INC_HL"); // -> [3]
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // dasCnt=0
    EmitAsm("INC_HL");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("POP_HL");

    EmitAsm("LD_A_IMM", new AsmOperand(0x01, AddressMode.Immediate));
    EmitAsm("JP", pr_done);

    // --- Trigger Left? (bit1) ---
    EmitLabel(pr_check_left);
    EmitAsm("LD_A_C");
    EmitAsm("AND_IMM", new AsmOperand(0x02, AddressMode.Immediate));
    EmitAsm("JP_Z", pr_no_trig);

    // dirMask=0x02, dasCnt=0, arrCnt=0
    EmitAsm("LD_A_IMM", new AsmOperand(0x02, AddressMode.Immediate));
    EmitAsm("LD_HL_A"); // [0]=0x02
    EmitAsm("PUSH_HL");
    EmitAsm("INC_HL"); EmitAsm("INC_HL"); EmitAsm("INC_HL"); // -> [3]
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // dasCnt=0
    EmitAsm("INC_HL");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("POP_HL");

    EmitAsm("LD_A_IMM", new AsmOperand(0x02, AddressMode.Immediate));
    EmitAsm("JP", pr_done);

    // --- No trigger: handle hold / repeat ---
    EmitLabel(pr_no_trig);

    // lr = keys & 0x03 -> D
    EmitAsm("LD_A_B");
    EmitAsm("AND_IMM", new AsmOperand(0x03, AddressMode.Immediate));
    EmitAsm("LD_D_A");
    EmitAsm("JP_Z", pr_reset_all); // none held

    // both held? -> return 0
    EmitAsm("CP_IMM", new AsmOperand(0x03, AddressMode.Immediate));
    EmitAsm("JP_Z", pr_ret0);

    // E = dirMask (state[0])
    EmitAsm("LD_A_HL");
    EmitAsm("LD_E_A");

    // If dirMask != 0 and that key is no longer held, clear state.
    EmitAsm("LD_A_E");
    EmitAsm("OR_A");
    EmitAsm("JP_Z", pr_dir0);
    EmitAsm("LD_A_B");
    EmitAsm("AND_E");
    EmitAsm("JP_NZ", pr_have_dir);

    // released: clear dirMask + counters (keep params)
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // dirMask=0
    EmitAsm("LD_E_A"); // E=0
    EmitAsm("PUSH_HL");
    EmitAsm("INC_HL"); EmitAsm("INC_HL"); EmitAsm("INC_HL"); // -> [3]
    EmitAsm("LD_HL_A"); // dasCnt=0
    EmitAsm("INC_HL");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("POP_HL");
    EmitAsm("JP", pr_dir0);

    // If dirMask==0, adopt current single-held direction but do not move immediately.
    EmitLabel(pr_dir0);
    EmitAsm("LD_A_E");
    EmitAsm("OR_A");
    EmitAsm("JP_NZ", pr_have_dir);

    // dirMask = lr (D), reset counters
    EmitAsm("LD_A_D");
    EmitAsm("LD_HL_A"); // [0]=dirMask
    EmitAsm("LD_E_A"); // E=dirMask
    EmitAsm("PUSH_HL");
    EmitAsm("INC_HL"); EmitAsm("INC_HL"); EmitAsm("INC_HL"); // -> [3]
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // dasCnt=0
    EmitAsm("INC_HL");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("POP_HL");

    EmitAsm("XOR_A"); // return 0 (no immediate move)
    EmitAsm("JP", pr_done);

    // --- Have a valid dirMask in E ---
    EmitLabel(pr_have_dir);

    // Load params: C=dasFrames (state[1]), D=arrFrames (state[2])
    EmitAsm("INC_HL");
    EmitAsm("LD_A_HL"); EmitAsm("LD_C_A"); // dasFrames
    EmitAsm("INC_HL");
    EmitAsm("LD_A_HL"); EmitAsm("LD_D_A"); // arrFrames
    EmitAsm("INC_HL"); // -> [3] dasCnt

    // A saturated delay counter has already fired its first repeat; proceed to ARR.
    EmitAsm("LD_A_HL");
    EmitAsm("CP_IMM", new AsmOperand(0xFF, AddressMode.Immediate));
    EmitAsm("JP_Z", pr_skip_dasinc);
    EmitAsm("INC_A");
    EmitAsm("LD_HL_A"); // store dasCnt

    // Compare dasCnt vs dasFrames
    EmitAsm("CP_C"); // A - C
    EmitAsm("JP_C", pr_ret0); // dasCnt < dasFrames
    EmitAsm("JP_Z", pr_first_rep); // dasCnt == dasFrames

    EmitLabel(pr_skip_dasinc);
    // After DAS: if arrFrames==0 => every frame
    EmitAsm("LD_A_D");
    EmitAsm("OR_A");
    EmitAsm("JP_Z", pr_ret_dir);

    // arrCnt at state[4]
    EmitAsm("INC_HL"); // -> [4] arrCnt
    EmitAsm("LD_A_HL");
    EmitAsm("CP_IMM", new AsmOperand(0xFF, AddressMode.Immediate));
    EmitAsm("JP_Z", pr_skip_arrinc);
    EmitAsm("INC_A");
    EmitLabel(pr_skip_arrinc);
    EmitAsm("LD_HL_A"); // store arrCnt
    EmitAsm("CP_D"); // arrCnt - arrFrames
    EmitAsm("JP_C", pr_ret0); // arrCnt < arrFrames

    // arrCnt reached: reset to 0 and return dir
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("JP", pr_ret_dir);

    // First repeat exactly at dasCnt==dasFrames: reset arrCnt and return dir
    EmitLabel(pr_first_rep);
    EmitAsm("INC_HL"); // -> [4]
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("JP", pr_ret_dir);

    // Return dirMask in A
    EmitLabel(pr_ret_dir);
    EmitAsm("LD_A_E");
    EmitAsm("JP", pr_done);

    // Return 0
    EmitLabel(pr_ret0);
    EmitAsm("XOR_A");
    EmitAsm("JP", pr_done);

    // Reset-all (no LR held): clear dir + counters and return 0
    EmitLabel(pr_reset_all);
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // dirMask=0
    EmitAsm("PUSH_HL");
    EmitAsm("INC_HL"); EmitAsm("INC_HL"); EmitAsm("INC_HL"); // -> [3]
    EmitAsm("LD_HL_A"); // dasCnt=0
    EmitAsm("INC_HL");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("POP_HL");
    EmitAsm("XOR_A");
    EmitAsm("JP", pr_done);

    EmitLabel(pr_done);
    return;
}




// __padrep_down(state*, keys, trigger) returns Down (8) or zero using the same five-byte state.
// A Down trigger fires immediately and resets counters; releasing Down clears the active direction.
// An already-held Down input is adopted without an immediate move when the state is inactive.
if (funcName == "__padrep_down")
{
    if (args.Length != 3) Error(expr, "__padrep_down requires 3 arguments (state*, keys, trigger)");

    AsmOperand pd_done        = MakeUniqueLabel("padrepD_done");
    AsmOperand pd_ret0        = MakeUniqueLabel("padrepD_ret0");
    AsmOperand pd_no_trig     = MakeUniqueLabel("padrepD_notrig");
    AsmOperand pd_dir0        = MakeUniqueLabel("padrepD_dir0");
    AsmOperand pd_have_dir    = MakeUniqueLabel("padrepD_havedir");
    AsmOperand pd_first_rep   = MakeUniqueLabel("padrepD_firstrep");
    AsmOperand pd_ret_dir     = MakeUniqueLabel("padrepD_retdir");
    AsmOperand pd_skip_dasinc = MakeUniqueLabel("padrepD_skipdasinc");
    AsmOperand pd_skip_arrinc = MakeUniqueLabel("padrepD_skiparrinc");
    AsmOperand pd_reset_all   = MakeUniqueLabel("padrepD_reset");

    // Evaluate args safely:
    // HL = state*, B = keys, C = trigger
    CompileIntoHL(args[0]);
    EmitAsm("PUSH_HL");
    CompileIntoA(args[1]); EmitAsm("LD_B_A"); // keys
    CompileIntoA(args[2]); EmitAsm("LD_C_A"); // trigger
    EmitAsm("POP_HL");

    // --- Trigger Down? (bit3 = 0x08) ---
    EmitAsm("LD_A_C");
    EmitAsm("AND_IMM", new AsmOperand(0x08, AddressMode.Immediate));
    EmitAsm("JP_Z", pd_no_trig);

    // dirMask=0x08, dasCnt=0, arrCnt=0
    EmitAsm("LD_A_IMM", new AsmOperand(0x08, AddressMode.Immediate));
    EmitAsm("LD_HL_A"); // [0]=0x08
    EmitAsm("PUSH_HL");
    EmitAsm("INC_HL"); EmitAsm("INC_HL"); EmitAsm("INC_HL"); // -> [3]
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // dasCnt=0
    EmitAsm("INC_HL");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("POP_HL");

    EmitAsm("LD_A_IMM", new AsmOperand(0x08, AddressMode.Immediate));
    EmitAsm("JP", pd_done);

    // --- No trigger: handle hold / repeat ---
    EmitLabel(pd_no_trig);

    // down = keys & 0x08 -> D
    EmitAsm("LD_A_B");
    EmitAsm("AND_IMM", new AsmOperand(0x08, AddressMode.Immediate));
    EmitAsm("JP_Z", pd_reset_all); // not held
    EmitAsm("LD_D_A");

    // E = dirMask (state[0])
    EmitAsm("LD_A_HL");
    EmitAsm("LD_E_A");

    // If dirMask != 0 and that key is no longer held, clear state.
    EmitAsm("LD_A_E");
    EmitAsm("OR_A");
    EmitAsm("JP_Z", pd_dir0);
    EmitAsm("LD_A_B");
    EmitAsm("AND_E");
    EmitAsm("JP_NZ", pd_have_dir);

    // released: clear dirMask + counters (keep params)
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // dirMask=0
    EmitAsm("LD_E_A"); // E=0
    EmitAsm("PUSH_HL");
    EmitAsm("INC_HL"); EmitAsm("INC_HL"); EmitAsm("INC_HL"); // -> [3]
    EmitAsm("LD_HL_A"); // dasCnt=0
    EmitAsm("INC_HL");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("POP_HL");
    EmitAsm("JP", pd_dir0);

    // If dirMask==0, adopt current held direction but do not move immediately.
    EmitLabel(pd_dir0);
    EmitAsm("LD_A_E");
    EmitAsm("OR_A");
    EmitAsm("JP_NZ", pd_have_dir);

    // dirMask = 0x08 (D), reset counters
    EmitAsm("LD_A_D");
    EmitAsm("LD_HL_A"); // [0]=dirMask
    EmitAsm("LD_E_A"); // E=dirMask
    EmitAsm("PUSH_HL");
    EmitAsm("INC_HL"); EmitAsm("INC_HL"); EmitAsm("INC_HL"); // -> [3]
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // dasCnt=0
    EmitAsm("INC_HL");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("POP_HL");

    EmitAsm("XOR_A"); // return 0 (no immediate move)
    EmitAsm("JP", pd_done);

    // --- Have a valid dirMask in E ---
    EmitLabel(pd_have_dir);

    // Load params: C=dasFrames (state[1]), D=arrFrames (state[2])
    EmitAsm("INC_HL");
    EmitAsm("LD_A_HL"); EmitAsm("LD_C_A"); // dasFrames
    EmitAsm("INC_HL");
    EmitAsm("LD_A_HL"); EmitAsm("LD_D_A"); // arrFrames
    EmitAsm("INC_HL"); // -> [3] dasCnt

    // A saturated delay counter has already fired its first repeat; proceed to ARR.
    EmitAsm("LD_A_HL");
    EmitAsm("CP_IMM", new AsmOperand(0xFF, AddressMode.Immediate));
    EmitAsm("JP_Z", pd_skip_dasinc);
    EmitAsm("INC_A");
    EmitAsm("LD_HL_A"); // store dasCnt

    // Compare dasCnt vs dasFrames
    EmitAsm("CP_C"); // A - C
    EmitAsm("JP_C", pd_ret0); // dasCnt < dasFrames
    EmitAsm("JP_Z", pd_first_rep); // dasCnt == dasFrames

    EmitLabel(pd_skip_dasinc);
    // After DAS: if arrFrames==0 => every frame
    EmitAsm("LD_A_D");
    EmitAsm("OR_A");
    EmitAsm("JP_Z", pd_ret_dir);

    // arrCnt at state[4]
    EmitAsm("INC_HL"); // -> [4] arrCnt
    EmitAsm("LD_A_HL");
    EmitAsm("CP_IMM", new AsmOperand(0xFF, AddressMode.Immediate));
    EmitAsm("JP_Z", pd_skip_arrinc);
    EmitAsm("INC_A");
    EmitLabel(pd_skip_arrinc);
    EmitAsm("LD_HL_A"); // store arrCnt
    EmitAsm("CP_D"); // arrCnt - arrFrames
    EmitAsm("JP_C", pd_ret0); // arrCnt < arrFrames

    // arrCnt reached: reset to 0 and return dir
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("JP", pd_ret_dir);

    // First repeat exactly at dasCnt==dasFrames: reset arrCnt and return dir
    EmitLabel(pd_first_rep);
    EmitAsm("INC_HL"); // -> [4]
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("JP", pd_ret_dir);

    // Return dirMask in A
    EmitLabel(pd_ret_dir);
    EmitAsm("LD_A_E");
    EmitAsm("JP", pd_done);

    // Return 0
    EmitLabel(pd_ret0);
    EmitAsm("XOR_A");
    EmitAsm("JP", pd_done);

    // Reset-all (Down not held): clear dir + counters and return 0
    EmitLabel(pd_reset_all);
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // dirMask=0
    EmitAsm("PUSH_HL");
    EmitAsm("INC_HL"); EmitAsm("INC_HL"); EmitAsm("INC_HL"); // -> [3]
    EmitAsm("LD_HL_A"); // dasCnt=0
    EmitAsm("INC_HL");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("POP_HL");
    EmitAsm("XOR_A");
    EmitAsm("JP", pd_done);

    EmitLabel(pd_done);
    return;
}



// __padrep(state*, keys, trigger, mask), also named __padrep_mask, schedules one selected action.
// A masked trigger selects its lowest set bit and fires immediately, resetting the counters.
// With no trigger, retain the active action while held; otherwise adopt the lowest masked held bit without firing.
// Use the same five-byte delay/repeat state as the directional variants; an empty held mask clears it.
if (funcName == "__padrep" || funcName == "__padrep_mask")
{
    if (args.Length != 4) Error(expr, "__padrep requires 4 arguments (state*, keys, trigger, mask)");

    AsmOperand pm_done        = MakeUniqueLabel("padrepM_done");
    AsmOperand pm_ret0        = MakeUniqueLabel("padrepM_ret0");
    AsmOperand pm_no_trig     = MakeUniqueLabel("padrepM_notrig");
    AsmOperand pm_reset_all   = MakeUniqueLabel("padrepM_reset");
    AsmOperand pm_dir0        = MakeUniqueLabel("padrepM_dir0");
    AsmOperand pm_have_dir    = MakeUniqueLabel("padrepM_havedir");
    AsmOperand pm_first_rep   = MakeUniqueLabel("padrepM_firstrep");
    AsmOperand pm_ret_dir     = MakeUniqueLabel("padrepM_retdir");
    AsmOperand pm_skip_dasinc = MakeUniqueLabel("padrepM_skipdasinc");
    AsmOperand pm_skip_arrinc = MakeUniqueLabel("padrepM_skiparrinc");

    // Evaluate args safely:
    // HL = state*, B = keys, C = trigger, E = mask
    CompileIntoHL(args[0]);
    EmitAsm("PUSH_HL");
    CompileIntoA(args[1]); EmitAsm("LD_B_A"); // keys
    CompileIntoA(args[2]); EmitAsm("LD_C_A"); // trigger
    CompileIntoA(args[3]); EmitAsm("LD_E_A"); // mask
    EmitAsm("POP_HL");

    // trigMasked = trigger & mask -> D
    EmitAsm("LD_A_C");
    EmitAsm("AND_E");
    EmitAsm("LD_D_A");
    EmitAsm("OR_A");
    EmitAsm("JP_Z", pm_no_trig);

    // select = lowbit(trigMasked) = x & -x
    EmitAsm("LD_A_D");
    EmitAsm("CPL");
    EmitAsm("INC_A");
    EmitAsm("AND_D");
    EmitAsm("LD_C_A"); // save selected in C

    // dirMask=selected, dasCnt=0, arrCnt=0
    EmitAsm("LD_A_C");
    EmitAsm("LD_HL_A"); // [0]=selected
    EmitAsm("PUSH_HL");
    EmitAsm("INC_HL"); EmitAsm("INC_HL"); EmitAsm("INC_HL"); // -> [3]
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // dasCnt=0
    EmitAsm("INC_HL");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("POP_HL");

    EmitAsm("LD_A_C");
    EmitAsm("JP", pm_done);

    // --- No trigger: handle hold / repeat ---
    EmitLabel(pm_no_trig);

    // held = keys & mask -> D
    EmitAsm("LD_A_B");
    EmitAsm("AND_E");
    EmitAsm("LD_D_A");
    EmitAsm("OR_A");
    EmitAsm("JP_Z", pm_reset_all); // none held

    // C = dirMask (state[0])
    EmitAsm("LD_A_HL");
    EmitAsm("LD_C_A");

    // If dirMask != 0 and that key is no longer held, clear state.
    EmitAsm("LD_A_C");
    EmitAsm("OR_A");
    EmitAsm("JP_Z", pm_dir0);
    EmitAsm("LD_A_C");
    EmitAsm("AND_D");
    EmitAsm("JP_NZ", pm_have_dir);

    // released: clear dirMask + counters (keep params)
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // dirMask=0
    EmitAsm("LD_C_A"); // C=0
    EmitAsm("PUSH_HL");
    EmitAsm("INC_HL"); EmitAsm("INC_HL"); EmitAsm("INC_HL"); // -> [3]
    EmitAsm("LD_HL_A"); // dasCnt=0
    EmitAsm("INC_HL");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("POP_HL");
    EmitAsm("JP", pm_dir0);

    // If dirMask==0, adopt from held (lowest set bit) but do not fire immediately.
    EmitLabel(pm_dir0);
    EmitAsm("LD_A_C");
    EmitAsm("OR_A");
    EmitAsm("JP_NZ", pm_have_dir);

    // select = lowbit(held)
    EmitAsm("LD_A_D");
    EmitAsm("CPL");
    EmitAsm("INC_A");
    EmitAsm("AND_D");
    EmitAsm("LD_C_A"); // C=selected

    EmitAsm("LD_A_C");
    EmitAsm("LD_HL_A"); // [0]=selected

    // reset counters
    EmitAsm("PUSH_HL");
    EmitAsm("INC_HL"); EmitAsm("INC_HL"); EmitAsm("INC_HL"); // -> [3]
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // dasCnt=0
    EmitAsm("INC_HL");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("POP_HL");

    EmitAsm("XOR_A"); // return 0 (no immediate move)
    EmitAsm("JP", pm_done);

    // --- Have a valid dirMask in C ---
    EmitLabel(pm_have_dir);

    // E = dirMask for return
    EmitAsm("LD_A_C");
    EmitAsm("LD_E_A");

    // Load params: C=dasFrames (state[1]), D=arrFrames (state[2])
    EmitAsm("INC_HL");
    EmitAsm("LD_A_HL"); EmitAsm("LD_C_A"); // dasFrames
    EmitAsm("INC_HL");
    EmitAsm("LD_A_HL"); EmitAsm("LD_D_A"); // arrFrames
    EmitAsm("INC_HL"); // -> [3] dasCnt

    // A saturated delay counter has already fired its first repeat; proceed to ARR.
    EmitAsm("LD_A_HL");
    EmitAsm("CP_IMM", new AsmOperand(0xFF, AddressMode.Immediate));
    EmitAsm("JP_Z", pm_skip_dasinc);
    EmitAsm("INC_A");
    EmitAsm("LD_HL_A"); // store dasCnt

    // Compare dasCnt vs dasFrames
    EmitAsm("CP_C");
    EmitAsm("JP_C", pm_ret0); // dasCnt < dasFrames
    EmitAsm("JP_Z", pm_first_rep); // dasCnt == dasFrames

    EmitLabel(pm_skip_dasinc);
    // After DAS: if arrFrames==0 => every frame
    EmitAsm("LD_A_D");
    EmitAsm("OR_A");
    EmitAsm("JP_Z", pm_ret_dir);

    // arrCnt at state[4]
    EmitAsm("INC_HL"); // -> [4]
    EmitAsm("LD_A_HL");
    EmitAsm("CP_IMM", new AsmOperand(0xFF, AddressMode.Immediate));
    EmitAsm("JP_Z", pm_skip_arrinc);
    EmitAsm("INC_A");
    EmitLabel(pm_skip_arrinc);
    EmitAsm("LD_HL_A"); // store arrCnt
    EmitAsm("CP_D");
    EmitAsm("JP_C", pm_ret0);

    // reached: reset arrCnt and return dir
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A");
    EmitAsm("JP", pm_ret_dir);

    // First repeat exactly at dasCnt==dasFrames: reset arrCnt and return dir
    EmitLabel(pm_first_rep);
    EmitAsm("INC_HL"); // -> [4]
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A");
    EmitAsm("JP", pm_ret_dir);

    // Return dirMask in A
    EmitLabel(pm_ret_dir);
    EmitAsm("LD_A_E");
    EmitAsm("JP", pm_done);

    // Return 0
    EmitLabel(pm_ret0);
    EmitAsm("XOR_A");
    EmitAsm("JP", pm_done);

    // Reset-all (none held): clear dir + counters and return 0
    EmitLabel(pm_reset_all);
    EmitAsm("XOR_A");
    EmitAsm("LD_HL_A"); // dirMask=0
    EmitAsm("PUSH_HL");
    EmitAsm("INC_HL"); EmitAsm("INC_HL"); EmitAsm("INC_HL"); // -> [3]
    EmitAsm("LD_HL_A"); // dasCnt=0
    EmitAsm("INC_HL");
    EmitAsm("LD_HL_A"); // arrCnt=0
    EmitAsm("POP_HL");
    EmitAsm("XOR_A");
    EmitAsm("JP", pm_done);

    EmitLabel(pm_done);
    return;
}



// --- Math intrinsics (GB-friendly fixed-point) ---
            // Conventions:
            // - u8 return in A
            // - u16 return in HL
            // Preserve the first byte while evaluating the second, then return the product high byte.
            if (funcName == "__mul8x8_hi")
            {
                if (args.Length != 2) Error(expr, "__mul8x8_hi requires 2 arguments");

                CompileIntoA(args[0]);
                EmitAsm("PUSH_AF");
                CompileIntoA(args[1]);
                EmitAsm("LD_C_A");
                EmitAsm("POP_AF");
                EmitAsm("LD_E_A");
                EmitMul8x8ToHL_EC("__mul8x8_hi");
                EmitAsm("LD_A_H");
                return;
            }
            // Dispatch unsigned word-by-byte multiplication with an eight-bit fractional shift.
            if (funcName == "__mul16x8")
            {
                if (args.Length != 2) Error(expr, "__mul16x8 requires 2 arguments");

                EmitDotTermToHL(args[0], args[1], signed: false, q1_7: false, labelPrefix: "__mul16x8");
                LastCallReturnsHL = true;
                return;
            }

            // Signed variants for 3D math (two's complement):
            // __smul16x8(a_s8_8, b_s0_8) -> (a*b)>>8 (s8_8)
            if (funcName == "__smul16x8")
            {
                if (args.Length != 2) Error(expr, "__smul16x8 requires 2 arguments");

                EmitDotTermToHL(args[0], args[1], signed: true, q1_7: false, labelPrefix: "__smul16x8");
                LastCallReturnsHL = true;
                return;
            }

            // Signed multiply where coefficient is Q1.7 (int8 scaled by 1/128).
            // Implemented as (a*b)>>8 then *2 (loses 1 bit of fractional precision).
            // Useful for sin/cos rotation coefficients in [-1, 1).
            if (funcName == "__smul16x8_q1_7")
            {
                if (args.Length != 2) Error(expr, "__smul16x8_q1_7 requires 2 arguments");

                EmitDotTermToHL(args[0], args[1], signed: true, q1_7: true, labelPrefix: "__smul16x8_q1_7");
                LastCallReturnsHL = true;
                return;
            }
            // Compute the shifted product first, preserve it, then add the accumulator modulo one word.
            if (funcName == "__mac16")
            {
                if (args.Length != 3) Error(expr, "__mac16 requires 3 arguments");

                // mul = (a*b)>>8
                EmitDotTermToHL(args[1], args[2], signed: false, q1_7: false, labelPrefix: "__mac16_mul");

                // acc + mul
                EmitAsm("PUSH_HL");
                CompileIntoHL(args[0]);
                EmitAsm("POP_DE");
                EmitAsm("ADD_HL_DE");
                LastCallReturnsHL = true;
                return;
            }

            // Signed MAC: acc + ((a*b)>>8)
            if (funcName == "__smac16")
            {
                if (args.Length != 3) Error(expr, "__smac16 requires 3 arguments");

                // mul = (a*b)>>8 (signed)
                EmitDotTermToHL(args[1], args[2], signed: true, q1_7: false, labelPrefix: "__smac16_mul");

                // acc + mul
                EmitAsm("PUSH_HL");
                CompileIntoHL(args[0]);
                EmitAsm("POP_DE");
                EmitAsm("ADD_HL_DE");
                LastCallReturnsHL = true;
                return;
            }

            // Signed MAC with Q1.7 coefficient (see __smul16x8_q1_7)
            if (funcName == "__smac16_q1_7")
            {
                if (args.Length != 3) Error(expr, "__smac16_q1_7 requires 3 arguments");

                // mul = (a*b)>>7 (approx via >>8 then *2)
                EmitDotTermToHL(args[1], args[2], signed: true, q1_7: true, labelPrefix: "__smac16_q1_7_mul");

                // acc + mul
                EmitAsm("PUSH_HL");
                CompileIntoHL(args[0]);
                EmitAsm("POP_DE");
                EmitAsm("ADD_HL_DE");
                LastCallReturnsHL = true;
                return;
            }

            // Accumulate three individually shifted unsigned terms, preserving each partial sum on the stack.
            if (funcName == "__dot3_q8_8")
            {
                if (args.Length != 6) Error(expr, "__dot3_q8_8 requires 6 arguments");
                LastCallReturnsHL = true;

                // x*ax
                EmitDotTermToHL(args[0], args[3], signed: false, q1_7: false, labelPrefix: "__dot3_x");
                EmitAsm("PUSH_HL");

                // y*ay
                EmitDotTermToHL(args[1], args[4], signed: false, q1_7: false, labelPrefix: "__dot3_y");
                EmitAsm("POP_DE");
                EmitAsm("ADD_HL_DE");
                EmitAsm("PUSH_HL");

                // z*az
                EmitDotTermToHL(args[2], args[5], signed: false, q1_7: false, labelPrefix: "__dot3_z");
                EmitAsm("POP_DE");
                EmitAsm("ADD_HL_DE");
                return;
            }


// 2D dot (unsigned): x*ax + y*ay
// x,y are Q8.8 (u16). ax,ay are u8 coefficients i.e. Q0.8. Returns u16 in HL (Q8.8).
if (funcName == "__dot2_q8_8")
{
    if (args.Length != 4) Error(expr, "__dot2_q8_8 requires 4 arguments");
    LastCallReturnsHL = true;

    // Fast paths when coefficients are compile-time constants.
    // Special-case: 0 / power-of-two coefficients (u8) => shift instead of mul.
    if (TryGetU8Const(args[2], out int ax0) && (ax0 == 0) && TryGetU8Const(args[3], out int ay0) && (ay0 == 0))
    {
        EmitAsm("XOR_A");
        EmitAsm("LD_H_A");
        EmitAsm("LD_L_A");
        return;
    }
    if (TryGetU8Const(args[2], out int axOnly) && (axOnly == 0))
    {
        // only y*ay
        if (!TryEmitDot2Pow2Term(args[1], args[3], signed: false, q1_7: false, out _))
        {
            CompileIntoHL(args[1]);
            EmitAsm("PUSH_HL");
            CompileIntoA(args[3]);
            EmitAsm("LD_C_A");
            EmitAsm("POP_HL");
            EmitMul16x8Shr8_HL("__dot2_y");
        }
        return;
    }
    if (TryGetU8Const(args[3], out int ayOnly) && (ayOnly == 0))
    {
        // only x*ax
        if (!TryEmitDot2Pow2Term(args[0], args[2], signed: false, q1_7: false, out _))
        {
            CompileIntoHL(args[0]);
            EmitAsm("PUSH_HL");
            CompileIntoA(args[2]);
            EmitAsm("LD_C_A");
            EmitAsm("POP_HL");
            EmitMul16x8Shr8_HL("__dot2_x");
        }
        return;
    }

    // x*ax
    if (!TryEmitDot2Pow2Term(args[0], args[2], signed: false, q1_7: false, out _))
    {
        CompileIntoHL(args[0]);
        EmitAsm("PUSH_HL");
        CompileIntoA(args[2]);
        EmitAsm("LD_C_A");
        EmitAsm("POP_HL");
        EmitMul16x8Shr8_HL("__dot2_x");
    }
    EmitAsm("PUSH_HL");

    // y*ay
    if (!TryEmitDot2Pow2Term(args[1], args[3], signed: false, q1_7: false, out _))
    {
        CompileIntoHL(args[1]);
        EmitAsm("PUSH_HL");
        CompileIntoA(args[3]);
        EmitAsm("LD_C_A");
        EmitAsm("POP_HL");
        EmitMul16x8Shr8_HL("__dot2_y");
    }
    EmitAsm("POP_DE");
    EmitAsm("ADD_HL_DE");
    return;
}

// 2D dot (signed two's complement): x*ax + y*ay
// x,y are Q8.8 (s16). ax,ay are s8 coefficients i.e. Q0.8. Returns s16 in HL (Q8.8).
if (funcName == "__sdot2_q8_8")
{
    if (args.Length != 4) Error(expr, "__sdot2_q8_8 requires 4 arguments");
    LastCallReturnsHL = true;

    // Fast paths when coefficients are compile-time constants.
    // Special-case: 0 / power-of-two coefficients (s8 two's complement) => arithmetic shift instead of mul.
    if (TryGetS8Const(args[2], out int sax0) && (sax0 == 0) && TryGetS8Const(args[3], out int say0) && (say0 == 0))
    {
        EmitAsm("XOR_A");
        EmitAsm("LD_H_A");
        EmitAsm("LD_L_A");
        return;
    }
    if (TryGetS8Const(args[2], out int saxOnly) && (saxOnly == 0))
    {
        // only y*ay
        if (!TryEmitDot2Pow2Term(args[1], args[3], signed: true, q1_7: false, out _))
        {
            CompileIntoHL(args[1]);
            EmitAsm("PUSH_HL");
            CompileIntoA(args[3]);
            EmitAsm("LD_C_A");
            EmitAsm("POP_HL");
            EmitSMul16x8Shr8_HL("__sdot2_y");
        }
        return;
    }
    if (TryGetS8Const(args[3], out int sayOnly) && (sayOnly == 0))
    {
        // only x*ax
        if (!TryEmitDot2Pow2Term(args[0], args[2], signed: true, q1_7: false, out _))
        {
            CompileIntoHL(args[0]);
            EmitAsm("PUSH_HL");
            CompileIntoA(args[2]);
            EmitAsm("LD_C_A");
            EmitAsm("POP_HL");
            EmitSMul16x8Shr8_HL("__sdot2_x");
        }
        return;
    }

    // x*ax
    if (!TryEmitDot2Pow2Term(args[0], args[2], signed: true, q1_7: false, out _))
    {
        CompileIntoHL(args[0]);
        EmitAsm("PUSH_HL");
        CompileIntoA(args[2]);
        EmitAsm("LD_C_A");
        EmitAsm("POP_HL");
        EmitSMul16x8Shr8_HL("__sdot2_x");
    }
    EmitAsm("PUSH_HL");

    // y*ay
    if (!TryEmitDot2Pow2Term(args[1], args[3], signed: true, q1_7: false, out _))
    {
        CompileIntoHL(args[1]);
        EmitAsm("PUSH_HL");
        CompileIntoA(args[3]);
        EmitAsm("LD_C_A");
        EmitAsm("POP_HL");
        EmitSMul16x8Shr8_HL("__sdot2_y");
    }
    EmitAsm("POP_DE");
    EmitAsm("ADD_HL_DE");
    return;
}

// Signed dot2 with Q1.7 coefficients (approx): x*ax + y*ay
if (funcName == "__sdot2_q1_7")
{
    if (args.Length != 4) Error(expr, "__sdot2_q1_7 requires 4 arguments");
    LastCallReturnsHL = true;

    // x*ax
    if (!TryEmitDot2Pow2Term(args[0], args[2], signed: true, q1_7: true, out _))
    {
        CompileIntoHL(args[0]);
        EmitAsm("PUSH_HL");
        CompileIntoA(args[2]);
        EmitAsm("LD_C_A");
        EmitAsm("POP_HL");
        EmitSMul16x8Shr8_HL("__sdot2_q1_7_x");
        EmitAsm("ADD_HL_HL");
    }
    EmitAsm("PUSH_HL");

    // y*ay
    if (!TryEmitDot2Pow2Term(args[1], args[3], signed: true, q1_7: true, out _))
    {
        CompileIntoHL(args[1]);
        EmitAsm("PUSH_HL");
        CompileIntoA(args[3]);
        EmitAsm("LD_C_A");
        EmitAsm("POP_HL");
        EmitSMul16x8Shr8_HL("__sdot2_q1_7_y");
        EmitAsm("ADD_HL_HL");
    }
    EmitAsm("POP_DE");
    EmitAsm("ADD_HL_DE");
    return;
}

            // Signed dot3 (two's complement): x*ax + y*ay + z*az
            if (funcName == "__sdot3_q8_8")
            {
                if (args.Length != 6) Error(expr, "__sdot3_q8_8 requires 6 arguments");
                LastCallReturnsHL = true;

                // x*ax
                EmitDotTermToHL(args[0], args[3], signed: true, q1_7: false, labelPrefix: "__sdot3_x");
                EmitAsm("PUSH_HL");

                // y*ay
                EmitDotTermToHL(args[1], args[4], signed: true, q1_7: false, labelPrefix: "__sdot3_y");
                EmitAsm("POP_DE");
                EmitAsm("ADD_HL_DE");
                EmitAsm("PUSH_HL");

                // z*az
                EmitDotTermToHL(args[2], args[5], signed: true, q1_7: false, labelPrefix: "__sdot3_z");
                EmitAsm("POP_DE");
                EmitAsm("ADD_HL_DE");
                return;
            }

            // Signed dot3 with Q1.7 coefficients (approx): x*ax + y*ay + z*az
            if (funcName == "__sdot3_q1_7")
            {
                if (args.Length != 6) Error(expr, "__sdot3_q1_7 requires 6 arguments");
                LastCallReturnsHL = true;

                // x*ax
                EmitDotTermToHL(args[0], args[3], signed: true, q1_7: true, labelPrefix: "__sdot3_q1_7_x");
                EmitAsm("PUSH_HL");

                // y*ay
                EmitDotTermToHL(args[1], args[4], signed: true, q1_7: true, labelPrefix: "__sdot3_q1_7_y");
                EmitAsm("POP_DE");
                EmitAsm("ADD_HL_DE");
                EmitAsm("PUSH_HL");

                // z*az
                EmitDotTermToHL(args[2], args[5], signed: true, q1_7: true, labelPrefix: "__sdot3_q1_7_z");
                EmitAsm("POP_DE");
                EmitAsm("ADD_HL_DE");
                return;
            }
            // --- Function / Function-pointer dispatch ---
            // If funcName is not a known function symbol, it may be a variable holding a function pointer.
            // Resolve unknown function names as typed pointer variables before reporting an undefined function.
            if (!Functions.TryGetValue(funcName, out CFunctionInfo info))
            {
                if (TryFindSymbol(funcName, out Symbol fpSym) && fpSym.Type != null && fpSym.Type.IsPointer && fpSym.Type.Subtype != null && fpSym.Type.Subtype.IsFunction)
                {
                    CType fnType = fpSym.Type.Subtype;
                    if (Program.AbiStack)
                    {
                        Error(expr, "Indirect calls are not supported with --abi=stack yet");
                    }
                    CType rt = fnType.Subtype ?? CType.UInt8;

                    // Currently we only support the fast-call subset for indirect calls:
                    // - 0 args, or 1 arg of size 1 byte (passed in A)
                    // This keeps codegen simple and avoids clobbering HL which is needed for JP_HL.
                    int expected = fnType.ParamTypes == null ? 0 : fnType.ParamTypes.Length;
                    if (args.Length != expected)
                        Error(expr, $"Wrong number of arguments (fnptr). Expected {expected}, got {args.Length}.");

                    if (expected > 1)
                        Error(expr, "Function-pointer calls currently support only 0 or 1 argument");

                    bool hasArgA = false;
                    if (expected == 1)
                    {
                        int psz = SizeOf(args[0], fnType.ParamTypes[0]);
                        if (psz != 1)
                            Error(expr, "Function-pointer calls currently support only 1-byte arguments (fastcall-A).");
                        CompileIntoA(args[0]);
                        EmitAsm("PUSH_AF");
                        hasArgA = true;
                    }

                    if (IsStructReturnType(rt))
                        PrepareStructReturnDestination(expr, rt, null, null);

                    // Load function pointer into HL
                    CompileIntoHL(funcExpr);

                    // -Zcheck: indirect calls into the banked ROM window are unsafe when switchable banks exist.
                    if (Program.CheckBankCalls && !InUnsafe && BankSwitchingUsed)
                    {
                        AsmOperand ok = MakeUniqueLabel("icall_bank0_ok");
                        EmitAsm("LD_A_H");
                        EmitAsm("CP_IMM", new AsmOperand(0x40, AddressMode.Immediate));
                        EmitAsm("JP_C", ok);
                        EmitAsm("CP_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                        EmitAsm("JP_NC", ok);
                        EmitAsm("JP", EnsureCheckTrap());
                        EmitLabel(ok);
                    }

                    if (hasArgA)
                        EmitAsm("POP_AF");

                    // Indirect call sequence: push return addr; jp (hl)
                    AsmOperand ret = MakeUniqueLabel("icall_ret");
                    EmitAsm("LD_DE_IMM", new AsmOperand(ret.Base.Value, AddressMode.Immediate16));
                    EmitAsm("PUSH_DE");
                    EmitAsm("JP_HL");
                    EmitLabel(ret);

                    // Return-width tracking
                    int[] fpActualArgSizes = args.Select(a => SizeOf(a)).ToArray();
                    int[] fpExpectedArgSizes = (fnType.ParamTypes ?? Array.Empty<CType>()).Select(t => SizeOf(expr, t)).ToArray();
                    RecordCallEdge(expr, "<indirect:" + funcName + ">", -1, "indirect", viaThunk: false, viaFarcall: false, actualArgSizes: fpActualArgSizes, expectedArgSizes: fpExpectedArgSizes);
                    LastCallReturnsHL = ReturnsInHL(expr, rt);
                    return;
                }

                Error(expr, "Undefined function: " + funcName);
                return;
            }

            if (args.Length != info.Parameters.Length)
                Error(expr, $"Wrong number of arguments. Expected {info.Parameters.Length}, got {args.Length}.");

            int[] actualArgSizes = new int[args.Length];
            int[] expectedArgSizes = new int[args.Length];
            int commonArgCount = Math.Min(args.Length, info.Parameters.Length);
            for (int i = 0; i < args.Length; i++)
            {
                actualArgSizes[i] = SizeOf(args[i]);
                expectedArgSizes[i] = (i < commonArgCount) ? SizeOf(expr, info.Parameters[i].Type) : -1;
            }

            // Reject direct self-recursion when the callee would reuse the caller's static parameter and local slots.
            if (!info.IsInline &&
                !info.IsStackCall &&
                !string.IsNullOrEmpty(CurrentFunctionName) &&
                string.Equals(funcName, CurrentFunctionName, StringComparison.Ordinal))
            {
                Error(expr,
                    "direct recursion requires __stackcall (legacy ABI reuses static parameter/local slots): " + funcName);
            }

            if (info.IsInline)
            {
                if (args.Length != info.Parameters.Length)
                    Error(expr, "Argument count mismatch for inline function");

                BeginScope();

                // Give this expansion its own return target and, for aggregate results, a fixed destination.
                AsmOperand returnLabel = MakeUniqueLabel("inline_end");
                InlineReturnLabels.Push(returnLabel);
                bool inlineStructReturn = IsStructReturnType(info.ReturnType);
                Symbol inlineRetSym = null;
                if (inlineStructReturn)
                {
                    inlineRetSym = EnsureStructReturnSlot(expr, info, funcName, info.ReturnType);
                    InlineStructReturnDestAddrs.Push(inlineRetSym.Value);
                }

                for (int i = 0; i < args.Length; i++)
                {
                    FieldInfo param = info.Parameters[i];

                    int size = SizeOf(expr, param.Type);

                    int addr = AllocatePreferred(size, HotRegions());

                    Symbol sym = new Symbol(SymbolTag.Local, addr, param.Type, param.Name);

                    if (CurrentScope.Symbols.ContainsKey(sym.Name))
                        // Reject duplicate parameter bindings within the newly opened inline scope.
                        Error(expr, "Symbol redefined in inline expansion: " + sym.Name);
                    CurrentScope.Symbols.Add(sym.Name, sym);

                    EmitStoreArgumentToFixedAddress(expr, funcName, i, args[i], param.Type, addr, $"inl arg:{sym.Name}");
                }

                // Compile the inline body with its declared return type, then restore the enclosing function's type.
                CType savedReturnType = ReturnType;
                ReturnType = info.ReturnType;
                CompileStatement(info.Body);
                ReturnType = savedReturnType;

                EmitLabel(returnLabel);
                InlineReturnLabels.Pop();
                if (inlineStructReturn)
                {
                    InlineStructReturnDestAddrs.Pop();
                    LastStructReturnAddress = inlineRetSym.Value;
                    LastCallReturnsHL = false;
                }

                EndScope();
                return;
            }

            // Stack ABI call (args on stack, caller-cleanup)
            if (info.IsStackCall)
            {
                bool stackCallReturnsHL = ReturnsInHL(expr, info.ReturnType);

                // Compute total arg bytes and offsets (arg0 is first param)
                int totalBytes = 0;
                // Lay out arguments consecutively in the caller-reserved stack block, retaining each declared width.
                int[] argOffs = new int[args.Length];
                int[] argSz = new int[args.Length];
                for (int i = 0; i < args.Length; i++)
                {
                    int sz = SizeOf(args[i], info.Parameters[i].Type);
                    bool aggregateParam = info.Parameters[i].Type != null && info.Parameters[i].Type.WithoutConst().IsStructOrUnion;
                    if (aggregateParam)
                    {
                        ValidateAggregateArgument(expr, funcName, i, args[i], info.Parameters[i].Type);
                    }
                    else if (sz != 1 && sz != 2)
                    {
                        NYI(expr, "Stack ABI supports only 1 or 2 byte arguments");
                    }
                    argOffs[i] = totalBytes;
                    argSz[i] = sz;
                    totalBytes += sz;
                }

                if (totalBytes > 0)
                {
                    // Keep the reserved argument block within the signed SP-relative instruction range.
                    if (totalBytes > 127)
                        Error(expr, "Too many stack arguments (total bytes > 127)");
                    EmitAsm("ADD_SP_IMM", new AsmOperand(-totalBytes, AddressMode.Relative));
                }

                // Evaluate args in the current (legacy) order, store into reserved stack arg block.
                for (int i = 0; i < args.Length; i++)
                {
                    int off = argOffs[i];
                    int sz = argSz[i];
                    if (off < -128 || off > 127)
                        Error(expr, "Too many stack arguments (SP offset out of range)");

                    if (info.Parameters[i].Type != null && info.Parameters[i].Type.WithoutConst().IsStructOrUnion)
                    {
                        EmitAggregateArgumentCopyToStackOffset(expr, funcName, i, args[i], info.Parameters[i].Type, off);
                    }
                    else if (sz == 1)
                    {
                        CompileIntoA(args[i]);
                        EmitAsm("LD_HL_SP_IMM", new AsmOperand(off, AddressMode.Relative));
                        EmitAsm("LD_HL_A");
                    }
                    else
                    {
                        // Value -> DE, then store low/high into stack block.
                        CompileIntoHL(args[i]);
                        EmitAsm("LD_D_H");
                        EmitAsm("LD_E_L");

                        EmitAsm("LD_A_E");
                        EmitAsm("LD_HL_SP_IMM", new AsmOperand(off, AddressMode.Relative));
                        EmitAsm("LD_HL_A");

                        EmitAsm("LD_A_D");
                        EmitAsm("LD_HL_SP_IMM", new AsmOperand(off + 1, AddressMode.Relative));
                        EmitAsm("LD_HL_A");
                    }
                }

                PrepareStructReturnDestination(expr, info.ReturnType, info, funcName);

                // Cross-bank calls must never switch MBC from banked code.
                int __calleeBank = info.RomBank;
                int __callerBank = CurrentFunctionBank;
                bool __directOK = IsDirectCallBankSafe(__calleeBank, __callerBank);

                if (!__directOK)
                {
                    string thunkName = EnsureBank0Thunk(funcName, __calleeBank, isStackCall: true, stackArgBytes: totalBytes);
                    EmitAsm("CALL", new AsmOperand(thunkName, AddressMode.Absolute));
                    RecordCallEdge(expr, funcName, __calleeBank, "bank_thunk", viaThunk: true, viaFarcall: false, actualArgSizes: actualArgSizes, expectedArgSizes: expectedArgSizes);
                    LastCallReturnsHL = stackCallReturnsHL;

                    if (totalBytes > 0)
                        EmitAsm("ADD_SP_IMM", new AsmOperand(totalBytes, AddressMode.Relative));
                    return;
                }

                // -Zcheck bank guard for direct calls into switchable banks.
	                if (Program.CheckBankCalls && !InUnsafe && __calleeBank != 0)
                {
                    AsmOperand ok = MakeUniqueLabel("kq_bank_ok");
                    EmitLoadA(RomBankVar);
	                    EmitAsm("CP_IMM", new AsmOperand(__calleeBank & 0xFF, AddressMode.Immediate));
                    EmitAsm("JP_Z", ok);
                    EmitAsm("JP", EnsureCheckTrap());
                    EmitLabel(ok);
                }

                EmitAsm("CALL", new AsmOperand(funcName, AddressMode.Absolute));
                RecordCallEdge(expr, funcName, __calleeBank, "direct", viaThunk: false, viaFarcall: false, actualArgSizes: actualArgSizes, expectedArgSizes: expectedArgSizes);
                LastCallReturnsHL = stackCallReturnsHL;
                if (totalBytes > 0)
                    EmitAsm("ADD_SP_IMM", new AsmOperand(totalBytes, AddressMode.Relative));
                return;
            }

            PrepareStructReturnDestination(expr, info.ReturnType, info, funcName);

            // Track byte fastcall arguments so optional bank checks can preserve A before the actual call.
            bool fastcallArgInA = false;
            if (info.IsFastCall)
            {
                Expr argExpr = args[0];
                Symbol paramSym = info.ParameterSymbols[0];
                int size = SizeOf(argExpr, paramSym.Type);

                if (size == 1)
                {
                    CompileIntoA(argExpr);
                    fastcallArgInA = true;
                }
                else if (size == 2)
                {
                    CompileIntoHL(argExpr);
                }
                else
                {
                    NYI(expr, "FastCall supports only 1 or 2 bytes arguments");
                }

            }
            else
            {

                for (int i = 0; i < args.Length; i++)
                {
                    Expr argExpr = args[i];
                    Symbol paramSym = info.ParameterSymbols[i];
                    int addr = paramSym.Value;
                    EmitStoreArgumentToFixedAddress(expr, funcName, i, argExpr, paramSym.Type, addr, $"arg:{paramSym.Name}");
                }
            }

            // Cross-bank calls must go through a bank0 thunk.
            int calleeBank = info.RomBank;
            int callerBank = CurrentFunctionBank;
            bool directCallReturnsHL = ReturnsInHL(expr, info.ReturnType);
            bool directOK = IsDirectCallBankSafe(calleeBank, callerBank);

            if (!directOK)
            {
                string thunkName = EnsureBank0Thunk(funcName, calleeBank, isStackCall: false, stackArgBytes: 0);
                EmitAsm("CALL", new AsmOperand(thunkName, AddressMode.Absolute));
                RecordCallEdge(expr, funcName, calleeBank, "bank_thunk", viaThunk: true, viaFarcall: false, actualArgSizes: actualArgSizes, expectedArgSizes: expectedArgSizes);
                LastCallReturnsHL = directCallReturnsHL;
                return;
            }

            // -Zcheck bank guard for direct calls into switchable banks.
            if (Program.CheckBankCalls && !InUnsafe && calleeBank != 0)
            {
                AsmOperand ok = MakeUniqueLabel("kq_bank_ok");
                if (fastcallArgInA)
                    EmitAsm("PUSH_AF");
                EmitLoadA(RomBankVar);
                EmitAsm("CP_IMM", new AsmOperand(calleeBank & 0xFF, AddressMode.Immediate));
                EmitAsm("JP_Z", ok);
                EmitAsm("JP", EnsureCheckTrap());
                EmitLabel(ok);
                if (fastcallArgInA)
                    EmitAsm("POP_AF");
            }

            EmitAsm("CALL", new AsmOperand(funcName, AddressMode.Absolute));
            RecordCallEdge(expr, funcName, calleeBank, "direct", viaThunk: false, viaFarcall: false, actualArgSizes: actualArgSizes, expectedArgSizes: expectedArgSizes);
            LastCallReturnsHL = directCallReturnsHL;
            return;
        }

        // --- Function-pointer call: fnptr(args...) ---
        // Supported subset (v1):
        // - call target is a variable of type pointer-to-function
        // - 0 arguments OR 1 argument of size 1 byte (passed in A)
        // - return type: u8 (A), u16 (HL), or void
        // NOTE: The normal (non-fastcall) calling convention in this compiler
        // uses per-function HRAM argument slots, which are not compatible with
        // indirect calls. We therefore restrict fnptr calls to the fastcall-like
        // subset above.
        if (funcName != null &&
            !Functions.ContainsKey(funcName) &&
            TryFindSymbol(funcName, out Symbol fnSym) &&
            fnSym.Type != null &&
            fnSym.Type.IsPointer &&
            fnSym.Type.Subtype != null &&
            fnSym.Type.Subtype.IsFunction)
        {
            CType fnType = fnSym.Type.Subtype;
            CType retType = fnType.Subtype ?? CType.UInt8;
            CType[] paramTypes = fnType.ParamTypes ?? Array.Empty<CType>();

            // Arg count check
            if (args.Length != paramTypes.Length)
                Error(expr, $"function pointer '{funcName}' expects {paramTypes.Length} arguments, got {args.Length}");

            // Evaluate argument(s) first, then load function pointer into HL.
            bool hasArgA = false;
            if (args.Length == 1)
            {
                int psz = SizeOf(args[0], paramTypes[0]);
                if (psz != 1)
                    Error(expr, "function pointer calls currently support only 1-byte parameter (passed in A)");
                CompileIntoA(args[0]);
                EmitAsm("PUSH_AF");
                hasArgA = true;
            }
            else if (args.Length > 1)
            {
                Error(expr, "function pointer calls currently support only 0 or 1 argument");
            }

            if (IsStructReturnType(retType))
                PrepareStructReturnDestination(expr, retType, null, null);

            // Load function address into HL
            CompileIntoHL(funcExpr);

            // -Zcheck: indirect calls into the banked ROM window are unsafe when switchable banks exist.
            if (Program.CheckBankCalls && !InUnsafe && BankSwitchingUsed)
            {
                AsmOperand ok = MakeUniqueLabel("icall_bank0_ok");
                EmitAsm("LD_A_H");
                EmitAsm("CP_IMM", new AsmOperand(0x40, AddressMode.Immediate));
                EmitAsm("JP_C", ok);
                EmitAsm("CP_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                EmitAsm("JP_NC", ok);
                EmitAsm("JP", EnsureCheckTrap());
                EmitLabel(ok);
            }

            if (hasArgA)
                EmitAsm("POP_AF");


            // Indirect call sequence: push return address; jp (hl)
            AsmOperand ret = MakeUniqueLabel("icall_ret");
            EmitAsm("LD_DE_IMM", new AsmOperand(ret.Base.Value, AddressMode.Immediate16));
            EmitAsm("PUSH_DE");
            EmitAsm("JP_HL");
            EmitLabel(ret);

            // Track return width
            int[] fpActualArgSizes = args.Select(a => SizeOf(a)).ToArray();
            int[] fpExpectedArgSizes = paramTypes.Select(t => SizeOf(expr, t)).ToArray();
            RecordCallEdge(expr, "<indirect:" + funcName + ">", -1, "indirect", viaThunk: false, viaFarcall: false, actualArgSizes: fpActualArgSizes, expectedArgSizes: fpExpectedArgSizes);
            LastCallReturnsHL = ReturnsInHL(expr, retType);
            return;
        }
        // --- Indirect call (function pointer expression) ---
        // Supports patterns like (*fp)(x) or (fp)(x) where fp is a function pointer.
        // NOTE: Because KITAQGB's default calling convention uses per-function HRAM
        // argument slots, we restrict indirect calls to the fastcall-like subset:
        // - 0 args, or
        // - 1 arg of 1 byte (passed in A)
        if (expr.Match(Tag.Call, out Expr calleeExpr, out Expr[] callArgs))
        {
            CType ct = TypeOf(calleeExpr);
            Expr calleeAddressExpr = calleeExpr;
            CType fnType = null;
            // Accept either a pointer-valued callee or a dereference of a typed function pointer.
            if (ct != null && ct.IsPointer && ct.Subtype != null && ct.Subtype.IsFunction)
            {
                fnType = ct.Subtype;
            }
            else if (ct != null && ct.IsFunction && calleeExpr.Match(Tag.Load, out Expr loadedFunctionPointer))
            {
                CType ptrType = TypeOf(loadedFunctionPointer);
                if (ptrType != null && ptrType.IsPointer && ptrType.Subtype != null && ptrType.Subtype.IsFunction)
                {
                    fnType = ptrType.Subtype;
                    calleeAddressExpr = loadedFunctionPointer;
                }
            }

            if (fnType != null)
            {
                CType retType = fnType.Subtype ?? CType.UInt8;
                CType[] paramTypes = fnType.ParamTypes ?? Array.Empty<CType>();

                if (callArgs.Length != paramTypes.Length)
                    Error(expr, $"function pointer call expects {paramTypes.Length} arguments, got {callArgs.Length}");

                bool hasArgA = false;

                if (callArgs.Length == 1)
                {
                    int psz = SizeOf(callArgs[0], paramTypes[0]);
                    if (psz != 1)
                        Error(expr, "function pointer calls currently support only 1-byte parameter (passed in A)");
                    CompileIntoA(callArgs[0]);
                    EmitAsm("PUSH_AF");
                    hasArgA = true;
                }
                else if (callArgs.Length > 1)
                {
                    Error(expr, "function pointer calls currently support only 0 or 1 argument");
                }

                if (IsStructReturnType(retType))
                    PrepareStructReturnDestination(expr, retType, null, null);

                // Load function address into HL (callee expression must evaluate to a 16-bit pointer)
                CompileIntoHL(calleeAddressExpr);

                // -Zcheck: indirect calls into the banked ROM window are unsafe when switchable banks exist.
                if (Program.CheckBankCalls && !InUnsafe && BankSwitchingUsed)
                {
                    AsmOperand ok = MakeUniqueLabel("icall_bank0_ok");
                    EmitAsm("LD_A_H");
                    EmitAsm("CP_IMM", new AsmOperand(0x40, AddressMode.Immediate));
                    EmitAsm("JP_C", ok);
                    EmitAsm("CP_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                    EmitAsm("JP_NC", ok);
                    EmitAsm("JP", EnsureCheckTrap());
                    EmitLabel(ok);
                }

                if (hasArgA)
                    EmitAsm("POP_AF");

                // Indirect call sequence: push return address; jp (hl)
                AsmOperand ret = MakeUniqueLabel("icall_ret");
                EmitAsm("LD_DE_IMM", new AsmOperand(ret.Base.Value, AddressMode.Immediate16));
                EmitAsm("PUSH_DE");
                EmitAsm("JP_HL");
                EmitLabel(ret);

                int[] fpActualArgSizes = callArgs.Select(a => SizeOf(a)).ToArray();
                int[] fpExpectedArgSizes = paramTypes.Select(t => SizeOf(expr, t)).ToArray();
                RecordCallEdge(expr, "<indirect:expr>", -1, "indirect", viaThunk: false, viaFarcall: false, actualArgSizes: fpActualArgSizes, expectedArgSizes: fpExpectedArgSizes);
                LastCallReturnsHL = ReturnsInHL(expr, retType);
                return;
            }
        }

        // Reject callee expressions outside the supported direct and typed-indirect calling conventions.
        NYI(expr, "Complex call not supported");
    }

    // Store A through an absolute operand or the GB high-memory addressing mode.
    void StoreAToOperand(AsmOperand op)
    {
        if (op.Mode == AddressMode.HighMem) EmitAsm("LDH_MEM_A", op);
        else EmitAsm("LD_MEM_A", op);
    }

    // Assignment used as an expression.
    // Policy: the expression value is always u8 (low byte of RHS).
    // This is intentionally permissive to handle AI-generated C patterns.
    void CompileAssignExprIntoA(Expr lhs, Expr rhs)
    {
        int lhsSize = SizeOf(lhs);
        int rhsSize = SizeOf(rhs);
        CType lhsType = TypeOf(lhs);
        CType rhsType = TypeOf(rhs);
        if ((lhsType != null && lhsType.WithoutConst().IsStructOrUnion) ||
            (rhsType != null && rhsType.WithoutConst().IsStructOrUnion))
        {
            Error(lhs, ErrorCode.ParseError, "struct/union assignment cannot be used as a scalar expression");
            EmitAsm("LD_A_IMM", new AsmOperand(0, AddressMode.Immediate));
            return;
        }

        // 1) Simple lvalue: local/global variable / addressable symbol operand
        if (TryGetOperand(lhs, out AsmOperand lhsOp))
        {
            if (lhsSize == 1)
            {
                CompileIntoA(rhs);
                StoreAToOperand(lhsOp);
                return;
            }

            if (lhsSize == 2)
            {
                if (rhsSize == 2)
                {
                    // store full 16-bit; return low byte
                    CompileIntoHL(rhs);

                    AsmOperand hiOp = new AsmOperand(lhsOp.Base, lhsOp.Offset + 1, lhsOp.Mode, lhsOp.Modifier, lhsOp.Comment);

                    EmitAsm("LD_A_L");
                    StoreAToOperand(lhsOp);
                    EmitAsm("LD_A_H");
                    StoreAToOperand(hiOp);
                    EmitAsm("LD_A_L"); // return value = low byte
                    return;
                }
                else
                {
                    // implicit u8 -> u16 assignment: high byte = 0
                    AsmOperand hiOp = new AsmOperand(lhsOp.Base, lhsOp.Offset + 1, lhsOp.Mode, lhsOp.Modifier, lhsOp.Comment);
                    CompileIntoA(rhs);
                    EmitAsm("PUSH_AF");
                    StoreAToOperand(lhsOp); // low
                    EmitAsm("LD_A_IMM", new AsmOperand(0, AddressMode.Immediate));
                    StoreAToOperand(hiOp); // high
                    EmitAsm("POP_AF");
                    return;
                }
            }
        }

        // 2) Pointer store: *p = rhs
        if (lhs.Match(Tag.Load, out Expr ptr))
        {
            if (lhsSize == 1)
            {
                CompileIntoHL(ptr); // HL = address
                CompileIntoA(rhs); // A = value
                EmitAsm("LD_HL_A"); // [HL] = A
                return;
            }

            if (lhsSize == 2)
            {
                CompileIntoHL(ptr);
                EmitAsm("PUSH_HL");

                // Compute RHS into DE (so HL can remain address)
                if (rhsSize == 2)
                {
                    CompileIntoHL(rhs);
                    EmitAsm("LD_A_L"); EmitAsm("LD_E_A");
                    EmitAsm("LD_A_H"); EmitAsm("LD_D_A");
                }
                else
                {
                    CompileIntoA(rhs);
                    EmitAsm("LD_E_A");
                    EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
                }

                EmitAsm("POP_HL");

                EmitAsm("LD_A_E");
                EmitAsm("LD_HL_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_D");
                EmitAsm("LD_HL_A");
                EmitAsm("LD_A_E"); // return low byte
                return;
            }
        }

        // 3) Indexed store: a[i] = rhs (supports u8/u16 element sizes)
        if (lhs.Match(Tag.Index, out Expr arr, out Expr index))
        {
            // Unwrap slice helper: __slice(ptr,len)[i]
            Expr baseArr = arr;
            Expr sliceLenIdx = null;
            if (arr.Match(Tag.Slice, out Expr slicePtr, out Expr lenExpr))
            {
                baseArr = slicePtr;
	                sliceLenIdx = lenExpr;
            }

            int elemSize = GetIndexElementSize(baseArr);
            // Use the actual indexed lvalue width when the base expression alone does not describe its element type.
            int lhsElemSize = SizeOf(lhs);
            if (lhsElemSize == 1 || lhsElemSize == 2)
                elemSize = lhsElemSize; // assignment expression must follow lvalue element width
            if (elemSize != 1 && elemSize != 2)
                elemSize = 1;

            if (sliceLenIdx != null)
            {
                // Always evaluate len for side effects; inject bounds only with -Zcheck.
                if (Program.CheckSliceBounds && !InUnsafe)
                    EmitSliceBoundsCheckIfNeeded(sliceLenIdx, index);
                else
                    CompileDiscard(sliceLenIdx);
            }
            else
            {
                // Debug-only runtime bounds check (fixed-size arrays only).
                EmitBoundsCheckIfNeeded(baseArr, index);
            }

            // compute address into HL: HL = base + scaled(index)
            CompileScaledIndexToDE(index, elemSize);
            EmitAsm("PUSH_DE");
            CompileIntoHL(baseArr);
            EmitAsm("POP_DE");
            EmitAsm("ADD_HL_DE");
            EmitAsm("PUSH_HL");

            if (elemSize == 1)
            {
                CompileIntoA(rhs);
                EmitAsm("POP_HL");
                EmitAsm("LD_HL_A");
                return;
            }

            // elemSize == 2
            if (rhsSize == 2)
            {
                CompileIntoHL(rhs);
                EmitAsm("LD_A_L"); EmitAsm("LD_E_A");
                EmitAsm("LD_A_H"); EmitAsm("LD_D_A");
            }
            else
            {
                CompileIntoA(rhs);
                EmitAsm("LD_E_A");
                EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
            }

            EmitAsm("POP_HL");
            EmitAsm("LD_A_E");
            EmitAsm("LD_HL_A");
            EmitAsm("INC_HL");
            EmitAsm("LD_A_D");
            EmitAsm("LD_HL_A");
            EmitAsm("LD_A_E");
            return;
        }

        NYI(lhs, "Assignment expression LHS not supported");
    }

    // Emit a byte-valued conditional chain with a shared exit, evaluating only the selected result branch.
    void CompileTernaryElseChainIntoA(Expr expr, AsmOperand endLabel)
    {
        Expr cond, texpr, fexpr;
        if (!expr.Match(Tag.Conditional, out cond, out texpr, out fexpr))
        {
            CompileIntoA(expr);
            return;
        }

        AsmOperand lblFalse = MakeUniqueLabel("tern_false");
        CompileJumpIf(false, cond, lblFalse);
        CompileIntoA(texpr);
        EmitAsm("JP", endLabel);
        EmitLabel(lblFalse);

        if (fexpr.Tag == Tag.Conditional)
            CompileTernaryElseChainIntoA(fexpr, endLabel);
        else
            CompileIntoA(fexpr);
    }

    // Identify signed byte expressions that require a sign-filled high byte when widened.
    bool ShouldSignExtendExprToHL(Expr expr)
    {
        CType t = TypeOf(expr);
        return t != null && t.IsSimple && t.SimpleType == CSimpleType.Int8;
    }

    // Copy A into L and fill H with zero or the original sign bit; signed extension also changes A.
    void EmitExtendAIntoHL(bool signExtend)
    {
        EmitAsm("LD_L_A");
        if (signExtend)
        {
            EmitAsm("RLCA"); // carry = sign(A)
            EmitAsm("SBC_A"); // A = 0x00 or 0xFF
            EmitAsm("LD_H_A");
        }
        else
        {
            EmitAsm("LD_H_IMM", new AsmOperand(0, AddressMode.Immediate));
        }
    }

    // Preserve L and derive H from its sign bit using carry and subtract-with-borrow.
    void EmitSignExtendLIntoH()
    {
        EmitAsm("LD_A_L");
        EmitAsm("RLCA");
        EmitAsm("SBC_A");
        EmitAsm("LD_H_A");
    }

    // Emit the low byte of an expression into A, folding constants before selecting an evaluation path.
    void CompileIntoA(Expr expr)
    {
        expr = FoldConstants(expr);

        if (expr.Match(Tag.Unsafe, out Expr unsafeSub))
        {
            UnsafeDepth++;
            CompileIntoA(unsafeSub);
            UnsafeDepth--;
            return;
        }

        // $slice(ptr,len): evaluate len (side effects), then yield ptr.
        if (expr.Match(Tag.Slice, out Expr slicePtr, out Expr sliceLen))
        {
            CompileDiscard(sliceLen);
            CompileIntoHL(slicePtr);
            EmitAsm("LD_A_L");
            return;
        }

        // sizeof(...) is a compile-time constant
        if (expr.Tag == Tag.Sizeof)
        {
            object arg = expr.GetArgs()[1];
            int sz = 0;
            if (arg is CType ct) sz = SizeOf(expr, ct);
            else if (arg is Expr ex) sz = SizeOf(ex);
            EmitAsm("LD_A_IMM", new AsmOperand(sz & 0xFF, AddressMode.Immediate));
            return;
        }

        // offsetof(type, member) is also a compile-time constant
        if (expr.Match(Tag.Offsetof, out CType offTyA, out string offPathA))
        {
            int off = CalculateOffsetOf(expr, offTyA, offPathA);
            EmitAsm("LD_A_IMM", new AsmOperand(off & 0xFF, AddressMode.Immediate));
            return;
        }

        // ?: (ternary) - u8 policy
        if (expr.Tag == Tag.Conditional)
        {
            AsmOperand lblEnd = MakeUniqueLabel("tern_end");
            CompileTernaryElseChainIntoA(expr, lblEnd);
            EmitLabel(lblEnd);
            return;
        }

        // Assignment as an expression (AI-style): (x = rhs)
        // Policy: expression value is always u8 (low byte of rhs).
        // This enables common ternary patterns like:
        // b = cond ? (c = 11) : (c = 22);
        // j = cond ? (c = foo(b)) : (c = bar(b));
        if (expr.Match(Tag.Assign, out Expr lhsAssign, out Expr rhsAssign))
        {
            CompileAssignExprIntoA(lhsAssign, rhsAssign);
            return;
        }

        Expr left, right, sub;
        int val;

        // Stack ABI: load parameter directly from stack (no local copy)
        if (expr.Match(Tag.Name, out string spName) && TryFindSymbol(spName, out Symbol spSym) && spSym.Tag == SymbolTag.StackParam)
        {
            EmitLoadStackParamU8IntoA(spSym);
            return;
        }

        if (expr.Match(Tag.Field, out Expr structExprF, out string fieldNameF))
        {
            FieldInfo f = GetFieldInfo(structExprF, fieldNameF);

            // 1) shadow_oam[i].x
            if (structExprF.Match(Tag.Index, out Expr arrF, out Expr idxF))
            {
                // Unwrap slice helper: __slice(ptr,len)[i].field
                Expr baseArrF = arrF;
                Expr sliceLenF = null;
                if (arrF.Match(Tag.Slice, out Expr slicePtrF, out Expr lenExprF))
                {
                    baseArrF = slicePtrF;
                    sliceLenF = lenExprF;
                }

                if (sliceLenF != null)
                {
                    // Always evaluate len for side effects; inject bounds only with -Zcheck.
                    if (Program.CheckSliceBounds && !InUnsafe)
                        EmitSliceBoundsCheckIfNeeded(sliceLenF, idxF);
                    else
                        CompileDiscard(sliceLenF);
                }
                else
                {
                    EmitBoundsCheckIfNeeded(baseArrF, idxF);
                }

                CompileIntoHL(baseArrF); // HL = base address
                EmitAsm("PUSH_HL");
                int elemSizeF = GetIndexElementSize(baseArrF);
                CompileScaledIndexToDE(idxF, elemSizeF); // DE = i*sizeof(elem)
                EmitAsm("POP_HL");
                EmitAsm("ADD_HL_DE"); // HL = element base
                if (f.Offset != 0)
                {
                    EmitAsm("LD_DE_IMM", new AsmOperand(f.Offset, AddressMode.Immediate16));
                    EmitAsm("ADD_HL_DE");
                }
                EmitAsm("LD_A_HL");
                return;
            }

            // 2) (*ptr).x
            if (structExprF.Match(Tag.Load, out Expr ptrF))
            {
                CompileIntoHL(ptrF); // HL = address
                if (f.Offset != 0)
                {
                    EmitAsm("LD_DE_IMM", new AsmOperand(f.Offset, AddressMode.Immediate16));
                    EmitAsm("ADD_HL_DE");
                }
                EmitAsm("LD_A_HL");
                return;
            }

            // 3) s.field (struct local/global)
            if (structExprF.Match(Tag.Name, out string sNameF))
            {
                Symbol sSym = FindSymbol(structExprF, sNameF);
                if (sSym.Tag == SymbolTag.Global || sSym.Tag == SymbolTag.Local)
                {
                    int addr = sSym.Value + ((f != null) ? f.Offset : 0);
                    EmitLoadA(MemOp(addr, sNameF + "." + fieldNameF));
                    return;
                }
            }

            // Generic fallback: nested field address -> load byte
            if (TryCompileLValueAddressIntoHL(expr, false))
            {
                EmitAsm("LD_A_HL");
                return;
            }
        }

        if (expr.Match(Tag.Index, out left, out right))
        {
            // Unwrap slice helper: __slice(ptr,len)[i]
            Expr baseExpr = left;
            Expr sliceLenIdx = null;
            if (left.Match(Tag.Slice, out Expr slicePtrIdx, out Expr lenExprIdx))
            {
                baseExpr = slicePtrIdx;
                sliceLenIdx = lenExprIdx;
            }

            if (sliceLenIdx != null)
            {
                // Always evaluate len for side effects; inject bounds only with -Zcheck.
                if (Program.CheckSliceBounds && !InUnsafe)
                    EmitSliceBoundsCheckIfNeeded(sliceLenIdx, right);
                else
                    CompileDiscard(sliceLenIdx);
            }
            else
            {
                // B3) Fast ROM-table read template for __prg_rom u8 table[i]
                // - constant index: emit direct absolute load (ld a,(table+off))
                // - if table is aligned to 256 and index is u8: use HL = (HIGH(table)<<8) | idx
                if (TryCompilePrgRomU8IndexIntoA(expr, baseExpr, right))
                    return;

                EmitBoundsCheckIfNeeded(baseExpr, right);
            }

            int elemSize = GetIndexElementSize(baseExpr);
            CompileScaledIndexToDE(right, elemSize); // DE = i*sizeof(elem)
            EmitAsm("PUSH_DE");

            // base address -> HL
            CompileIntoHL(baseExpr);
            EmitAsm("POP_DE");
            EmitAsm("ADD_HL_DE");
            EmitAsm("LD_A_HL");
            return;
        }

        if (expr.Match(Tag.Load, out sub))
        {
            CompileIntoHL(sub);
            EmitAsm("LD_A_HL");
            return;
        }

        if (expr.Match(Tag.ShiftLeft, out left, out right))
{
    if (right.Match(Tag.Integer, out int count))
    {
        int lSize = SizeOf(left);

        // 16-bit (u16) << n in an 8-bit context:
        // result is truncated to the low byte of the 16-bit shift result.
        // For n >= 8, the low byte is always 0, but we must still evaluate 'left' for side effects.
        if (lSize == 2)
        {
            CompileIntoA(left); // evaluate left (low byte is enough for truncation semantics)
            if (count >= 8)
            {
                EmitAsm("XOR_A");
                return;
            }
            for (int i = 0; i < count; i++) EmitAsm("ADD_A"); // SLA A
            return;
        }

        // 8-bit shift-left (legacy behavior)
        CompileIntoA(left);
        for (int i = 0; i < count; i++) EmitAsm("ADD_A");
        return;
    }
    NYI(expr, "Variable shift not supported");
}
        // if (expr.Match(Tag.ShiftRight, out left, out right))
        // {
        // if (right.Match(Tag.Integer, out int count))
        // {
        // CompileIntoA(left);
        // for (int i = 0; i < count; i++) { EmitAsm("OR_A"); EmitAsm("RRA"); }
        // return;
        // }
        // NYI(expr, "Variable shift not supported");
        // }

        // NOTE:
        if (expr.Match(Tag.ShiftRight, out left, out right))
        {
            if (right.Match(Tag.Integer, out int count))
            {
                int lSize = SizeOf(left);
                bool signedShift = IsSignedIntegerType(TypeOf(left));

                if (lSize == 2)
                {
                    CompileIntoHL(left);
                    if (signedShift)
                    {
                        int c = count;
                        if (c >= 16) c = 16;
                        EmitShiftRightArithmeticHL(c);
                    }
                    else
                    {
                        if (count >= 16) EmitAsm("LD_HL_IMM", new AsmOperand(0, AddressMode.Immediate16));
                        else EmitShiftRightLogicalHL(count);
                    }
                    EmitAsm("LD_A_L");
                    return;
                }

                // 8bit shift in A.
                CompileIntoA(left);
                for (int i = 0; i < count; i++)
                {
                    if (signedShift)
                    {
                        // Compare without changing A, then invert borrow to recover its original sign bit.
                        EmitAsm("CP_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                        EmitAsm("CCF");
                        EmitAsm("RRA"); // shift the unchanged byte with its sign in carry
                    }
                    else
                    {
                        EmitAsm("OR_A"); // clear carry
                        EmitAsm("RRA"); // logical shift right
                    }
                }
                return;
            }
            NYI(expr, "Variable shift not supported");
        }

        // if (expr.Match(Tag.Multiply, out left, out right))
        // {
        // if (right.Match(Tag.Integer, out int rVal))
        // {
        // if (rVal > 0 && (rVal & (rVal - 1)) == 0)
        // {
        // CompileIntoA(left);
        // while (rVal > 1) { EmitAsm("ADD_A"); rVal >>= 1; }
        // return;
        // }
        // }
        // }


        if (expr.MatchTag(Tag.Multiply) || expr.MatchTag(Tag.Divide) || expr.MatchTag(Tag.Modulus))
        {
            CompileIntoHL(expr);
            EmitAsm("LD_A_L");
            return;
        }


        if (expr.Match(Tag.Integer, out val))
        {
            EmitAsm("LD_A_IMM", new AsmOperand(val & 0xFF, AddressMode.Immediate));
            return;
        }
        if (TryGetOperand(expr, out AsmOperand op))
        {
            if (op.Mode == AddressMode.Immediate16)
            {
                EmitAsm("LD_HL_IMM", op);
                EmitAsm("LD_A_L");
                return;
            }

            if (op.Mode == AddressMode.Immediate) EmitAsm("LD_A_IMM", op);
            else if (op.Mode == AddressMode.HighMem) EmitAsm("LDH_A_MEM", op);
            else EmitAsm("LD_A_MEM", op);
            return;
        }

        if (expr.Match(Tag.BitwiseNot, out sub))
        {
            CompileIntoA(sub);
            EmitAsm("CPL");
            return;
        }

        if (expr.Match(Tag.LogicalOr, out left, out right))
        {
            // Short-circuit on either true operand and normalize the byte result to zero or one.
            AsmOperand lblTrue = MakeUniqueLabel("lor_t"), lblEnd = MakeUniqueLabel("lor_e");
            CompileJumpIf(true, left, lblTrue);
            CompileJumpIf(true, right, lblTrue);
            EmitAsm("LD_A_IMM", new AsmOperand(0, AddressMode.Immediate));
            EmitAsm("JP", lblEnd);
            EmitLabel(lblTrue);
            EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate));
            EmitLabel(lblEnd);
            return;
        }
        if (expr.Match(Tag.LogicalAnd, out left, out right))
        {
            // Skip the right operand when the left is false and normalize the result to zero or one.
            AsmOperand lblFalse = MakeUniqueLabel("land_f"), lblEnd = MakeUniqueLabel("land_e");
            CompileJumpIf(false, left, lblFalse);
            CompileJumpIf(false, right, lblFalse);
            EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate));
            EmitAsm("JP", lblEnd);
            EmitLabel(lblFalse);
            EmitAsm("LD_A_IMM", new AsmOperand(0, AddressMode.Immediate));
            EmitLabel(lblEnd);
            return;
        }
        if (expr.Match(Tag.LogicalNot, out sub))
        {
            // Invert the operand truth value through the shared conditional-branch emitter.
            AsmOperand lblTrue = MakeUniqueLabel("lnot_t"), lblEnd = MakeUniqueLabel("lnot_e");
            CompileJumpIf(false, sub, lblTrue);
            EmitAsm("LD_A_IMM", new AsmOperand(0, AddressMode.Immediate));
            EmitAsm("JP", lblEnd);
            EmitLabel(lblTrue);
            EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate));
            EmitLabel(lblEnd);
            return;
        }
        if (expr.MatchTag(Tag.Call))
        {
            CompileCall(expr);

            // If the call returns u16 in HL but this context needs an 8-bit value,
            // take the low byte. Size inference can be shaky for some intrinsics,
            // so also force known HL-returning intrinsics here.
            if (expr.Match(Tag.Call, out Expr callTarget, out Expr[] callArgs) &&
                callTarget.Match(Tag.Name, out string callName) &&
                IsIntrinsicReturningHL(callName))
            {
                EmitAsm("LD_A_L");
                return;
            }

            if (SizeOf(expr) == 2)
            {
                EmitAsm("LD_A_L");
            }
            return;
        }

        if (expr.Match(Tag.Add, out left, out right))
        {
            if (TryGetOperand(right, out AsmOperand rOp) && rOp.Mode == AddressMode.Immediate)
            {
                // Use an immediate byte addition when the right operand is already a literal operand.
                CompileIntoA(left); EmitAsm("ADD_A_IMM", rOp); return;
            }
            CompileIntoA(left);
            EmitAsm("PUSH_AF");
            CompileIntoA(right);
            EmitAsm("LD_B_A");
            EmitAsm("POP_AF");
            EmitAsm("ADD_B");
            return;
        }
        if (expr.Match(Tag.Subtract, out left, out right))
        {
            if (TryGetOperand(right, out AsmOperand rOp) && rOp.Mode == AddressMode.Immediate)
            {
                // Use an immediate byte subtraction without allocating a saved right operand.
                CompileIntoA(left); EmitAsm("SUB_IMM", rOp); return;
            }
            CompileIntoA(left);
            EmitAsm("PUSH_AF");
            CompileIntoA(right);
            EmitAsm("LD_B_A");
            EmitAsm("POP_AF");
            EmitAsm("SUB_B");
            return;
        }
        if (expr.Match(Tag.BitwiseAnd, out left, out right))
        {
            if (TryGetOperand(right, out AsmOperand rOp) && rOp.Mode == AddressMode.Immediate)
            {
                // Apply a constant low-byte AND mask directly.
                CompileIntoA(left); EmitAsm("AND_IMM", rOp); return;
            }
            CompileIntoA(left);
            EmitAsm("PUSH_AF");
            CompileIntoA(right);
            EmitAsm("LD_B_A");
            EmitAsm("POP_AF");
            EmitAsm("AND_B");
            return;
        }
        if (expr.Match(Tag.BitwiseOr, out left, out right))
        {
            if (TryGetOperand(right, out AsmOperand rOp) && rOp.Mode == AddressMode.Immediate)
            {
                // Apply a constant low-byte OR mask directly.
                CompileIntoA(left); EmitAsm("OR_IMM", rOp); return;
            }
            CompileIntoA(left);
            EmitAsm("PUSH_AF");
            CompileIntoA(right);
            EmitAsm("LD_B_A");
            EmitAsm("POP_AF");
            EmitAsm("OR_B");
            return;
        }
        if (expr.Match(Tag.BitwiseXor, out left, out right))
        {
            if (TryGetOperand(right, out AsmOperand rOp) && rOp.Mode == AddressMode.Immediate)
            {
                // Apply a constant low-byte XOR mask directly.
                CompileIntoA(left); EmitAsm("XOR_IMM", rOp); return;
            }
            CompileIntoA(left);
            EmitAsm("PUSH_AF");
            CompileIntoA(right);
            EmitAsm("LD_B_A");
            EmitAsm("POP_AF");
            EmitAsm("XOR_B");
            return;
        }

        if (expr.Match(Tag.Equal, out left, out right))
        {
            // Use shared jump-based comparator for 16-bit/pointer and signed relational cases.
            if (SizeOf(left) == 2 || SizeOf(right) == 2 || ShouldUseSignedComparison(left, right))
            { CompileComparisonBoolIntoA(Tag.Equal, left, right, expr.Source); return; }

            CompileIntoA(left);
            EmitAsm("PUSH_AF");
            CompileIntoA(right);
            EmitAsm("LD_B_A");
            EmitAsm("POP_AF");
            EmitAsm("CP_B");
            // Convert byte equality flags into a normalized boolean value.
            AsmOperand lblEqT = MakeUniqueLabel("eq_t"), lblEqE = MakeUniqueLabel("eq_e");
            EmitAsm("JP_Z", lblEqT); EmitAsm("LD_A_IMM", new AsmOperand(0, AddressMode.Immediate)); EmitAsm("JP", lblEqE);
            EmitLabel(lblEqT); EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate)); EmitLabel(lblEqE); return;
        }
        if (expr.Match(Tag.NotEqual, out left, out right))
        {
            // Use shared jump-based comparator for 16-bit/pointer and signed relational cases.
            if (SizeOf(left) == 2 || SizeOf(right) == 2 || ShouldUseSignedComparison(left, right))
            { CompileComparisonBoolIntoA(Tag.NotEqual, left, right, expr.Source); return; }

            CompileIntoA(left);
            EmitAsm("PUSH_AF");
            CompileIntoA(right);
            EmitAsm("LD_B_A");
            EmitAsm("POP_AF");
            EmitAsm("CP_B");
            // Convert byte inequality flags into a normalized boolean value.
            AsmOperand lblNeqT = MakeUniqueLabel("neq_t"), lblNeqE = MakeUniqueLabel("neq_e");
            EmitAsm("JP_NZ", lblNeqT); EmitAsm("LD_A_IMM", new AsmOperand(0, AddressMode.Immediate)); EmitAsm("JP", lblNeqE);
            EmitLabel(lblNeqT); EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate)); EmitLabel(lblNeqE); return;
        }
        if (expr.Match(Tag.LessThan, out left, out right))
        {
            // Use shared jump-based comparator for 16-bit/pointer and signed relational cases.
            if (SizeOf(left) == 2 || SizeOf(right) == 2 || ShouldUseSignedComparison(left, right))
            { CompileComparisonBoolIntoA(Tag.LessThan, left, right, expr.Source); return; }

            CompileIntoA(left);
            EmitAsm("PUSH_AF");
            CompileIntoA(right);
            EmitAsm("LD_B_A");
            EmitAsm("POP_AF");
            EmitAsm("CP_B");
            // Use borrow from the unsigned byte comparison to produce the less-than result.
            AsmOperand lblLtT = MakeUniqueLabel("lt_t"), lblLtE = MakeUniqueLabel("lt_e");
            EmitAsm("JP_C", lblLtT); EmitAsm("LD_A_IMM", new AsmOperand(0, AddressMode.Immediate)); EmitAsm("JP", lblLtE);
            EmitLabel(lblLtT); EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate)); EmitLabel(lblLtE); return;
        }
        if (expr.Match(Tag.GreaterThanOrEqual, out left, out right))
        {
            // Use shared jump-based comparator for 16-bit/pointer and signed relational cases.
            if (SizeOf(left) == 2 || SizeOf(right) == 2 || ShouldUseSignedComparison(left, right))
            { CompileComparisonBoolIntoA(Tag.GreaterThanOrEqual, left, right, expr.Source); return; }

            CompileIntoA(left);
            EmitAsm("PUSH_AF");
            CompileIntoA(right);
            EmitAsm("LD_B_A");
            EmitAsm("POP_AF");
            EmitAsm("CP_B");
            // Invert unsigned borrow to produce the greater-than-or-equal result.
            AsmOperand lblGeT = MakeUniqueLabel("ge_t"), lblGeE = MakeUniqueLabel("ge_e");
            EmitAsm("JP_NC", lblGeT); EmitAsm("LD_A_IMM", new AsmOperand(0, AddressMode.Immediate)); EmitAsm("JP", lblGeE);
            EmitLabel(lblGeT); EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate)); EmitLabel(lblGeE); return;
        }
        if (expr.Match(Tag.GreaterThan, out left, out right))
        {
            // Use shared jump-based comparator for 16-bit/pointer and signed relational cases.
            if (SizeOf(left) == 2 || SizeOf(right) == 2 || ShouldUseSignedComparison(left, right))
            { CompileComparisonBoolIntoA(Tag.GreaterThan, left, right, expr.Source); return; }

            CompileIntoA(right);
            EmitAsm("PUSH_AF");
            CompileIntoA(left);
            EmitAsm("LD_C_A");
            EmitAsm("POP_AF");
            EmitAsm("CP_C");
            // The reversed operand comparison turns unsigned borrow into greater-than.
            AsmOperand lblGtT = MakeUniqueLabel("gt_t"), lblGtE = MakeUniqueLabel("gt_e");
            EmitAsm("JP_C", lblGtT); EmitAsm("LD_A_IMM", new AsmOperand(0, AddressMode.Immediate)); EmitAsm("JP", lblGtE);
            EmitLabel(lblGtT); EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate)); EmitLabel(lblGtE); return;
        }
        if (expr.Match(Tag.LessThanOrEqual, out left, out right))
        {
            // Use shared jump-based comparator for 16-bit/pointer and signed relational cases.
            if (SizeOf(left) == 2 || SizeOf(right) == 2 || ShouldUseSignedComparison(left, right))
            { CompileComparisonBoolIntoA(Tag.LessThanOrEqual, left, right, expr.Source); return; }

            CompileIntoA(right);
            EmitAsm("PUSH_AF");
            CompileIntoA(left);
            EmitAsm("LD_C_A");
            EmitAsm("POP_AF");
            EmitAsm("CP_C");
            // The reversed operand comparison turns no borrow into less-than-or-equal.
            AsmOperand lblLeT = MakeUniqueLabel("le_t"), lblLeE = MakeUniqueLabel("le_e");
            EmitAsm("JP_NC", lblLeT); EmitAsm("LD_A_IMM", new AsmOperand(0, AddressMode.Immediate)); EmitAsm("JP", lblLeE);
            EmitLabel(lblLeT); EmitAsm("LD_A_IMM", new AsmOperand(1, AddressMode.Immediate)); EmitLabel(lblLeE); return;
        }

        if (expr.Match(Tag.PreIncrement, out sub) || expr.Match(Tag.PostIncrement, out sub) ||
            expr.Match(Tag.PreDecrement, out sub) || expr.Match(Tag.PostDecrement, out sub))
        {
            if (TryGetOperand(sub, out AsmOperand subOp))
            {
                if (subOp.Mode == AddressMode.HighMem) EmitAsm("LDH_A_MEM", subOp); else EmitAsm("LD_A_MEM", subOp);
                bool isPost = expr.MatchTag(Tag.PostIncrement) || expr.MatchTag(Tag.PostDecrement);
                bool isInc = expr.MatchTag(Tag.PreIncrement) || expr.MatchTag(Tag.PostIncrement);
                if (isPost) EmitAsm("LD_B_A");
                if (isInc) EmitAsm("INC_A"); else EmitAsm("DEC_A");
                if (subOp.Mode == AddressMode.HighMem) EmitAsm("LDH_MEM_A", subOp); else EmitAsm("LD_MEM_A", subOp);
                if (isPost) EmitAsm("LD_A_B");
                return;
            }
        }

        if (expr.Match(Tag.Cast, out CType t, out sub))
        {
            CompileIntoA(sub); return;
        }

        NYI(expr, "Expression too complex for CompileIntoA");
    }

    // Emit a word value or address into HL, widening byte loads according to their signedness.
    void CompileIntoHL(Expr expr)
    {
        expr = FoldConstants(expr);

        if (expr.Match(Tag.Unsafe, out Expr unsafeSub))
        {
            UnsafeDepth++;
            CompileIntoHL(unsafeSub);
            UnsafeDepth--;
            return;
        }

        // $slice(ptr,len): evaluate len (side effects), then yield ptr.
        if (expr.Match(Tag.Slice, out Expr slicePtr, out Expr sliceLen))
        {
            CompileDiscard(sliceLen);
            CompileIntoHL(slicePtr);
            return;
        }

        // sizeof(...) is a compile-time constant
        if (expr.Tag == Tag.Sizeof)
        {
            object arg = expr.GetArgs()[1];
            int sz = 0;
            if (arg is CType ct) sz = SizeOf(expr, ct);
            else if (arg is Expr ex) sz = SizeOf(ex);
            EmitAsm("LD_HL_IMM", new AsmOperand(sz & 0xFFFF, AddressMode.Immediate16));
            return;
        }

        // offsetof(type, member) is also a compile-time constant
        if (expr.Match(Tag.Offsetof, out CType offTyHL, out string offPathHL))
        {
            int off = CalculateOffsetOf(expr, offTyHL, offPathHL);
            EmitAsm("LD_HL_IMM", new AsmOperand(off & 0xFFFF, AddressMode.Immediate16));
            return;
        }

        // ?: (ternary)
        if (expr.Match(Tag.Conditional, out Expr cond, out Expr texpr, out Expr fexpr))
        {
            int resultSize = SizeOf(expr);
            if (resultSize == 1)
            {
                CompileIntoA(expr);
                EmitExtendAIntoHL(ShouldSignExtendExprToHL(expr));
                return;
            }

            // Merge the selected word-valued branch in HL without evaluating the other branch.
            AsmOperand lblFalse = MakeUniqueLabel("tern_hl_false");
            AsmOperand lblEnd = MakeUniqueLabel("tern_hl_end");
            CompileJumpIf(false, cond, lblFalse);
            CompileIntoHL(texpr);
            EmitAsm("JP", lblEnd);
            EmitLabel(lblFalse);
            CompileIntoHL(fexpr);
            EmitLabel(lblEnd);
            return;
        }

        Expr left, right, sub;
        int val;

        if (expr.Match(Tag.Name, out string name))
        {
            // A bare function name yields its address when no local/global symbol shadows it.
            if (!TryFindSymbol(name, out Symbol sym) && Functions.ContainsKey(name))
            {
                EmitAsm("LD_HL_IMM", new AsmOperand(name, AddressMode.Immediate16));
                return;
            }

            if (sym == null)
            {
                Error(expr, "Undefined symbol: " + name);
                return;
            }
            
            if (sym != null && sym.Type.IsArray)
            {
                if (sym.Tag == SymbolTag.ReadonlyData)
                    EmitAsm("LD_HL_IMM", new AsmOperand(sym.Name, AddressMode.Immediate16));
                else
                    EmitAsm("LD_HL_IMM", new AsmOperand(sym.Value, AddressMode.Immediate16));
                return;
            }
        }

        // Stack ABI: load parameter directly from stack (no local copy)
        if (expr.Match(Tag.Name, out string spName16) && TryFindSymbol(spName16, out Symbol spSym16) && spSym16.Tag == SymbolTag.StackParam)
        {
            int sz = SizeOf(expr);
            if (sz == 1)
            {
                EmitLoadStackParamU8IntoA(spSym16);
                EmitExtendAIntoHL(ShouldSignExtendExprToHL(expr));
            }
            else
            {
                EmitLoadStackParamU16IntoHL(spSym16);
            }
            return;
        }


        if (expr.Match(Tag.AddressOf, out sub))
        {
            // &name
            if (sub.Match(Tag.Name, out string n))
            {
                if (Functions.ContainsKey(n))
                {
                    EmitAsm("LD_HL_IMM", new AsmOperand(n, AddressMode.Immediate16));
                    return;
                }

                Symbol sym = FindSymbol(sub, n);
                    if (sym.Tag == SymbolTag.StackParam)
                    {
                        EmitStackParamAddrIntoHL(sym.Value, $"&stack param {sym.Name}");
                        return;
                    }

                if (sym.Tag == SymbolTag.ReadonlyData)
                    EmitAsm("LD_HL_IMM", new AsmOperand(sym.Name, AddressMode.Immediate16));
                else
                    EmitAsm("LD_HL_IMM", new AsmOperand(sym.Value, AddressMode.Immediate16));
                return;
            }

            // &arr[idx]
            if (sub.Match(Tag.Index, out Expr baseE, out Expr idxE))
            {
                EmitBoundsCheckIfNeeded(baseE, idxE);
                // HL = base address (array name decays to address, pointer value loads to HL)
                CompileIntoHL(baseE);
                EmitAsm("PUSH_HL");

                int elemSize = GetIndexElementSize(baseE);
                CompileScaledIndexToDE(idxE, elemSize);

                EmitAsm("POP_HL");
                EmitAsm("ADD_HL_DE");
                return;
            }

            // &s.field / &arr[i].field / &(*p).field
            if (sub.Match(Tag.Field, out Expr structE, out string fieldName))
            {
                FieldInfo f = GetFieldInfo(structE, fieldName);

                // arr[i].field
                if (structE.Match(Tag.Index, out Expr arrF, out Expr idxF))
                {
                    EmitBoundsCheckIfNeeded(arrF, idxF);
                    CompileIntoHL(arrF);
                    EmitAsm("PUSH_HL");
                    int elemSizeF = GetIndexElementSize(arrF);
                    CompileScaledIndexToDE(idxF, elemSizeF);
                    EmitAsm("POP_HL");
                    EmitAsm("ADD_HL_DE");

                    if (f != null && f.Offset != 0)
                    {
                        EmitAsm("LD_DE_IMM", new AsmOperand(f.Offset, AddressMode.Immediate16));
                        EmitAsm("ADD_HL_DE");
                    }
                    return;
                }

                // (*p).field
                if (structE.Match(Tag.Load, out Expr ptrF))
                {
                    CompileIntoHL(ptrF);
                    if (f != null && f.Offset != 0)
                    {
                        EmitAsm("LD_DE_IMM", new AsmOperand(f.Offset, AddressMode.Immediate16));
                        EmitAsm("ADD_HL_DE");
                    }
                    return;
                }

                // s.field (struct local/global)
                if (structE.Match(Tag.Name, out string sName))
                {
                    Symbol sSym = FindSymbol(structE, sName);
                    if (sSym.Tag == SymbolTag.StackParam)
                        EmitStackParamAddrIntoHL(sSym.Value, $"&stack param {sSym.Name}");
                    else if (sSym.Tag == SymbolTag.ReadonlyData)
                        EmitAsm("LD_HL_IMM", new AsmOperand(sSym.Name, AddressMode.Immediate16));
                    else
                        EmitAsm("LD_HL_IMM", new AsmOperand(sSym.Value, AddressMode.Immediate16));

                    if (f != null && f.Offset != 0)
                    {
                        EmitAsm("LD_DE_IMM", new AsmOperand(f.Offset, AddressMode.Immediate16));
                        EmitAsm("ADD_HL_DE");
                    }
                    return;
                }
            }

            // &(*p) => p
            if (sub.Match(Tag.Load, out Expr ptrExpr))
            {
                CompileIntoHL(ptrExpr);
                return;
            }

            NYI(expr, "Address-of form not supported");
        }

        if (expr.Match(Tag.Field, out Expr structExprF, out string fieldNameF))
        {
            FieldInfo f = GetFieldInfo(structExprF, fieldNameF);
            int fSize = (f != null) ? SizeOf(expr, f.Type) : 1;

            if (f != null && f.Type != null && f.Type.IsArray)
            {
                if (TryCompileLValueAddressIntoHL(expr, false))
                    return;
            }

            // 1) arr[i].field
            if (structExprF.Match(Tag.Index, out Expr arrF, out Expr idxF))
            {
                // Unwrap slice helper: __slice(ptr,len)[i].field
                Expr baseArrF = arrF;
                Expr sliceLenF = null;
                if (arrF.Match(Tag.Slice, out Expr slicePtrF, out Expr lenExprF))
                {
                    baseArrF = slicePtrF;
                    sliceLenF = lenExprF;
                }

                if (sliceLenF != null)
                {
                    // Always evaluate len for side effects; inject bounds only with -Zcheck.
                    if (Program.CheckSliceBounds && !InUnsafe)
                        EmitSliceBoundsCheckIfNeeded(sliceLenF, idxF);
                    else
                        CompileDiscard(sliceLenF);
                }
                else
                {
                    EmitBoundsCheckIfNeeded(baseArrF, idxF);
                }

                CompileIntoHL(baseArrF); // HL = base address
                EmitAsm("PUSH_HL");
                int elemSizeF = GetIndexElementSize(baseArrF);
                CompileScaledIndexToDE(idxF, elemSizeF); // DE = i*sizeof(elem)
                EmitAsm("POP_HL");
                EmitAsm("ADD_HL_DE");

                if (f != null && f.Offset != 0)
                {
                    EmitAsm("LD_DE_IMM", new AsmOperand(f.Offset, AddressMode.Immediate16));
                    EmitAsm("ADD_HL_DE");
                }

                if (fSize == 1)
                {
                    EmitAsm("LD_A_HL");
                    EmitExtendAIntoHL(f != null && IsSignedIntegerType(f.Type));
                }
                else
                {
                    // Read 16-bit value at [HL] into HL without destroying the address early
                    EmitAsm("LD_A_HL");
                    EmitAsm("LD_E_A");
                    EmitAsm("INC_HL");
                    EmitAsm("LD_A_HL");
                    EmitAsm("LD_D_A");
                    EmitAsm("LD_H_D");
                    EmitAsm("LD_L_E");
                }
                return;
            }

            // 2) (*ptr).field (ptr->field)
            if (structExprF.Match(Tag.Load, out Expr ptrF))
            {
                CompileIntoHL(ptrF); // HL = address
                if (f != null && f.Offset != 0)
                {
                    EmitAsm("LD_DE_IMM", new AsmOperand(f.Offset, AddressMode.Immediate16));
                    EmitAsm("ADD_HL_DE");
                }

                if (fSize == 1)
                {
                    EmitAsm("LD_A_HL");
                    EmitExtendAIntoHL(f != null && IsSignedIntegerType(f.Type));
                }
                else
                {
                    EmitAsm("LD_A_HL");
                    EmitAsm("LD_E_A");
                    EmitAsm("INC_HL");
                    EmitAsm("LD_A_HL");
                    EmitAsm("LD_D_A");
                    EmitAsm("LD_H_D");
                    EmitAsm("LD_L_E");
                }
                return;
            }

            // 3) s.field (struct local/global)
            if (structExprF.Match(Tag.Name, out string sName))
            {
                Symbol sSym = FindSymbol(structExprF, sName);
                if (sSym.Tag == SymbolTag.Global || sSym.Tag == SymbolTag.Local)
                {
                    EmitAsm("LD_HL_IMM", new AsmOperand(sSym.Value, AddressMode.Immediate16));
                    if (f != null && f.Offset != 0)
                    {
                        EmitAsm("LD_DE_IMM", new AsmOperand(f.Offset, AddressMode.Immediate16));
                        EmitAsm("ADD_HL_DE");
                    }

                    if (fSize == 1)
                    {
                        EmitAsm("LD_A_HL");
                        EmitExtendAIntoHL(f != null && IsSignedIntegerType(f.Type));
                    }
                    else
                    {
                        EmitAsm("LD_A_HL");
                        EmitAsm("LD_E_A");
                        EmitAsm("INC_HL");
                        EmitAsm("LD_A_HL");
                        EmitAsm("LD_D_A");
                        EmitAsm("LD_H_D");
                        EmitAsm("LD_L_E");
                    }
                    return;
                }
            }

            // Generic fallback: nested field address, then load width according to field type.
            if (TryCompileLValueAddressIntoHL(expr, false))
            {
                if (fSize == 1)
                {
                    EmitAsm("LD_A_HL");
                    EmitExtendAIntoHL(f != null && IsSignedIntegerType(f.Type));
                }
                else
                {
                    EmitAsm("LD_A_HL");
                    EmitAsm("LD_E_A");
                    EmitAsm("INC_HL");
                    EmitAsm("LD_A_HL");
                    EmitAsm("LD_D_A");
                    EmitAsm("LD_H_D");
                    EmitAsm("LD_L_E");
                }
                return;
            }
        }

        if (expr.Match(Tag.Index, out left, out right))
        {
            // Unwrap slice helper: __slice(ptr,len)[i]
            Expr baseExpr = left;
            Expr sliceLenIdx = null;
            if (left.Match(Tag.Slice, out Expr slicePtrIdx, out Expr lenExprIdx))
            {
                baseExpr = slicePtrIdx;
                sliceLenIdx = lenExprIdx;
            }

            if (sliceLenIdx != null)
            {
                // Always evaluate len for side effects; inject bounds only with -Zcheck.
                if (Program.CheckSliceBounds && !InUnsafe)
                    EmitSliceBoundsCheckIfNeeded(sliceLenIdx, right);
                else
                    CompileDiscard(sliceLenIdx);
            }
            else
            {
                EmitBoundsCheckIfNeeded(baseExpr, right);
            }

            int resultSize = SizeOf(expr);
            int elemSize = GetIndexElementSize(baseExpr);
            CompileScaledIndexToDE(right, elemSize); // DE = idx*sizeof(elem)
            EmitAsm("PUSH_DE");

            // base address -> HL
            CompileIntoHL(baseExpr);
            EmitAsm("POP_DE");
            EmitAsm("ADD_HL_DE");

            if (resultSize == 1)
            {
                // 8-bit load then extend according to signedness
                EmitAsm("LD_A_HL");
                EmitExtendAIntoHL(ShouldSignExtendExprToHL(expr));
                return;
            }

            // Read 16-bit from [HL] into DE, then move to HL
            EmitAsm("LD_A_HL");
            EmitAsm("LD_E_A");
            EmitAsm("INC_HL");
            EmitAsm("LD_A_HL");
            EmitAsm("LD_D_A");
            EmitAsm("LD_H_D");
            EmitAsm("LD_L_E");
            return;
        }

        if (expr.Match(Tag.Load, out sub))
        {
            int loadSize = SizeOf(expr);
            CompileIntoHL(sub); // HL = address
            if (loadSize == 1)
            {
                EmitAsm("LD_A_HL");
                EmitExtendAIntoHL(ShouldSignExtendExprToHL(expr));
            }
            else
            {
                EmitAsm("LD_A_HL");
                EmitAsm("LD_E_A");
                EmitAsm("INC_HL");
                EmitAsm("LD_A_HL");
                EmitAsm("LD_D_A");
                EmitAsm("LD_H_D");
                EmitAsm("LD_L_E");
            }
            return;
        }

if (expr.Match(Tag.ShiftLeft, out left, out right))
{
    if (right.Match(Tag.Integer, out int count))
    {
        CompileIntoHL(left);
        EmitShiftLeftLogicalHL(count);
        return;
    }
}



        if (expr.Match(Tag.ShiftRight, out left, out right))
        {
            if (right.Match(Tag.Integer, out int count))
            {
                int lSize = SizeOf(left);
                bool signedShift = IsSignedIntegerType(TypeOf(left));

                CompileIntoHL(left);

                if (count <= 0) return;

                if (signedShift)
                {
                    int c = count;
                    if (c >= 16) c = 16;
                    EmitShiftRightArithmeticHL(c);
                    return;
                }

                if (count >= 16)
                {
                    EmitAsm("LD_HL_IMM", new AsmOperand(0, AddressMode.Immediate16));
                    return;
                }

                EmitShiftRightLogicalHL(count);
                return;
            }
            NYI(expr, "Variable shift not supported");
        }

        if (expr.Match(Tag.Integer, out val))
        {
            EmitAsm("LD_HL_IMM", new AsmOperand(val, AddressMode.Immediate16));
            return;
        }

        if (TryGetWideOperand(expr, out WideOperand wideOp))
        {
            if (wideOp.Low.Mode == AddressMode.HighMem) EmitAsm("LDH_A_MEM", wideOp.Low);
            else EmitAsm("LD_A_MEM", wideOp.Low);
            EmitAsm("LD_L_A");

            if (wideOp.High.Mode == AddressMode.HighMem) EmitAsm("LDH_A_MEM", wideOp.High);
            else EmitAsm("LD_A_MEM", wideOp.High);
            EmitAsm("LD_H_A");
            return;
        }

        if (TryGetOperand(expr, out AsmOperand op))
        {
            if (op.Mode == AddressMode.Immediate || op.Mode == AddressMode.Immediate16)
            {
                EmitAsm("LD_HL_IMM", op);
                return;
            }

            int size = SizeOf(expr);
            if (op.Mode == AddressMode.HighMem) EmitAsm("LDH_A_MEM", op); else EmitAsm("LD_A_MEM", op);
            EmitAsm("LD_L_A");

            if (size == 1)
            {
                if (ShouldSignExtendExprToHL(expr)) EmitSignExtendLIntoH();
                else
                {
                    EmitAsm("XOR_A");
                    EmitAsm("LD_H_A");
                }
            }
            else
            {
                AsmOperand highOp = new AsmOperand(Maybe.Nothing, op.Offset + 1, op.Mode, ImmediateModifier.None);
                if (op.Mode == AddressMode.HighMem) EmitAsm("LDH_A_MEM", highOp); else EmitAsm("LD_A_MEM", highOp);
                EmitAsm("LD_H_A");
            }
            return;
        }

        if (expr.Match(Tag.Cast, out CType t, out sub))
        {
            if (SizeOf(expr, t) == 1)
            {
                CompileIntoA(sub);
                EmitExtendAIntoHL(IsSignedIntegerType(t));
                return;
            }

            if (SizeOf(sub) == 1)
            {
                CompileIntoA(sub);
                EmitExtendAIntoHL(ShouldSignExtendExprToHL(sub));
                return;
            }

            CompileIntoHL(sub);
            return;
        }

        if (expr.Match(Tag.ShiftLeft, out left, out right))
{
    if (right.Match(Tag.Integer, out int count))
    {
        CompileIntoHL(left);
        EmitShiftLeftLogicalHL(count);
        return;
    }
}

        if (expr.Match(Tag.BitwiseNot, out sub))
        {
            CompileIntoHL(sub);
            EmitAsm("LD_A_L"); EmitAsm("CPL"); EmitAsm("LD_L_A");
            EmitAsm("LD_A_H"); EmitAsm("CPL"); EmitAsm("LD_H_A");
            return;
        }

        string btag;
        if (expr.MatchAnyTag(out btag, out left, out right) &&
            (btag == Tag.BitwiseAnd || btag == Tag.BitwiseOr || btag == Tag.BitwiseXor))
        {
            // HL = left
            CompileIntoHL(left);

            if (right.Match(Tag.Integer, out int rVal))
            {
                int lo = rVal & 0xFF;
                int hi = (rVal >> 8) & 0xFF;

                EmitAsm("LD_A_L");
                if (btag == Tag.BitwiseAnd) EmitAsm("AND_IMM", new AsmOperand(lo, AddressMode.Immediate));
                else if (btag == Tag.BitwiseOr) EmitAsm("OR_IMM", new AsmOperand(lo, AddressMode.Immediate));
                else EmitAsm("XOR_IMM", new AsmOperand(lo, AddressMode.Immediate));
                EmitAsm("LD_L_A");

                EmitAsm("LD_A_H");
                if (btag == Tag.BitwiseAnd) EmitAsm("AND_IMM", new AsmOperand(hi, AddressMode.Immediate));
                else if (btag == Tag.BitwiseOr) EmitAsm("OR_IMM", new AsmOperand(hi, AddressMode.Immediate));
                else EmitAsm("XOR_IMM", new AsmOperand(hi, AddressMode.Immediate));
                EmitAsm("LD_H_A");
                return;
            }

            EmitAsm("PUSH_HL");
            CompileIntoHL(right);
            EmitAsm("LD_D_H");
            EmitAsm("LD_E_L");
            EmitAsm("POP_HL");

            EmitAsm("LD_A_L");
            if (btag == Tag.BitwiseAnd) EmitAsm("AND_E");
            else if (btag == Tag.BitwiseOr) EmitAsm("OR_E");
            else EmitAsm("XOR_E");
            EmitAsm("LD_L_A");

            EmitAsm("LD_A_H");
            if (btag == Tag.BitwiseAnd) EmitAsm("AND_D");
            else if (btag == Tag.BitwiseOr) EmitAsm("OR_D");
            else EmitAsm("XOR_D");
            EmitAsm("LD_H_A");
            return;
        }

        if (expr.Match(Tag.Add, out left, out right))
        {
            CType leftType = TypeOf(left);
            CType rightType = TypeOf(right);
            bool leftPtrInt = leftType != null && leftType.IsPointer && rightType != null && (rightType.IsInteger || rightType.IsEnum);
            // Recognize integer-plus-pointer as well as pointer-plus-integer so offsets use pointee storage size.
            bool rightPtrInt = rightType != null && rightType.IsPointer && leftType != null && (leftType.IsInteger || leftType.IsEnum);

            if (leftPtrInt || rightPtrInt)
            {
                Expr ptrExpr = leftPtrInt ? left : right;
                Expr indexExpr = leftPtrInt ? right : left;
                CType ptrType = leftPtrInt ? leftType : rightType;
                int elemSize = 1;
                if (ptrType != null && ptrType.Subtype != null) elemSize = SizeOf(expr, ptrType.Subtype);

                CompileScaledIndexToDE(indexExpr, elemSize);
                EmitAsm("PUSH_DE");
                CompileIntoHL(ptrExpr);
                EmitAsm("POP_DE");
                EmitAsm("ADD_HL_DE");
                return;
            }

            CompileIntoHL(left);
            if (right.Match(Tag.Integer, out int rVal))
            {
                EmitAsm("LD_DE_IMM", new AsmOperand(rVal, AddressMode.Immediate16));
                EmitAsm("ADD_HL_DE");
                return;
            }
            EmitAsm("PUSH_HL");
            CompileIntoHL(right);
            EmitAsm("LD_D_H"); EmitAsm("LD_E_L");
            EmitAsm("POP_HL");
            EmitAsm("ADD_HL_DE");
            return;
        }

        if (expr.Match(Tag.Subtract, out left, out right))
        {
            CType leftType = TypeOf(left);
            CType rightType = TypeOf(right);
            bool leftPtrInt = leftType != null && leftType.IsPointer && rightType != null && (rightType.IsInteger || rightType.IsEnum);

            if (leftPtrInt)
            {
                int elemSize = 1;
                if (leftType.Subtype != null) elemSize = SizeOf(expr, leftType.Subtype);
                CompileScaledIndexToDE(right, elemSize);
                EmitAsm("PUSH_DE");
                CompileIntoHL(left);
                EmitAsm("POP_DE");
                EmitAsm("LD_A_L");
                EmitAsm("SUB_E");
                EmitAsm("LD_L_A");
                EmitAsm("LD_A_H");
                EmitAsm("SBC_D");
                EmitAsm("LD_H_A");
                return;
            }

            if (right.Match(Tag.Integer, out int rVal))
            {
                // HL = left - n => HL = left + (-n)
                int negVal = -rVal;
                CompileIntoHL(left);
                EmitAsm("LD_DE_IMM", new AsmOperand(negVal & 0xFFFF, AddressMode.Immediate16));
                EmitAsm("ADD_HL_DE");
                return;
            }

            CompileIntoHL(left); // HL = left
            EmitAsm("PUSH_HL"); // Stack = left
            CompileIntoHL(right); // HL = right


            EmitAsm("LD_D_H"); // DE = Right
            EmitAsm("LD_E_L");
            EmitAsm("POP_HL"); // HL = Left

            // HL = HL - DE
            // L = L - E
            EmitAsm("LD_A_L");
            EmitAsm("SUB_E");
            EmitAsm("LD_L_A");

            // H = H - D - Carry
            EmitAsm("LD_A_H");
            EmitAsm("SBC_D"); // SBC A, D
            EmitAsm("LD_H_A");
            return;
        }




        if (expr.Match(Tag.Multiply, out left, out right))
        {
            // --- Fast path: u8 * u8 -> u16 ---
            if (SizeOf(left) == 1 && SizeOf(right) == 1)
            {
                CompileIntoA(left);
                EmitAsm("LD_E_A"); // multiplicand in E
                CompileIntoA(right);
                EmitAsm("LD_C_A"); // multiplier in C
                EmitMul8x8ToHL_EC("mul8");
                return;
            }

            // --- Constant RHS optimizations ---
            if (right.Match(Tag.Integer, out int rConstRaw))
            {
                int factor = rConstRaw & 0xFFFF;

                if (factor == 0)
                {
                    EmitAsm("LD_HL_IMM", new AsmOperand(0, AddressMode.Immediate16));
                    return;
                }
                if (factor == 1)
                {
                    CompileIntoHL(left);
                    return;
                }
                if (IsPow2(factor))
                {
                    CompileIntoHL(left);
                    int sh = Log2Pow2(factor);
                    for (int i = 0; i < sh; i++) EmitAsm("ADD_HL_HL");
                    return;
                }

                // HL = left
                CompileIntoHL(left);

                // Try the shared constant-multiplication expansion before the general word multiply loop.
                if (EmitConstantMultiplication(factor))
                    return;

                // BC = left
                EmitAsm("LD_B_H");
                EmitAsm("LD_C_L");
                // DE = factor
                EmitAsm("LD_DE_IMM", new AsmOperand(factor, AddressMode.Immediate16));
            }
            else
            {
                // --- Generic: BC=left, DE=right ---
                CompileIntoHL(left);
                EmitAsm("LD_B_H");
                EmitAsm("LD_C_L");

                CompileIntoHL(right);
                EmitAsm("PUSH_HL");
                EmitAsm("POP_DE");
            }

            // --- 16-bit shift/add multiply: HL = BC * DE (mod 65536) ---
            EmitAsm("LD_HL_IMM", new AsmOperand(0, AddressMode.Immediate16));
            EmitAsm("LD_A_IMM", new AsmOperand(16, AddressMode.Immediate));
            AsmOperand loopLabel = MakeUniqueLabel("mul16_loop");
            EmitLabel(loopLabel);
            EmitAsm("PUSH_AF");

            // result <<= 1
            EmitAsm("ADD_HL_HL");

            // shift BC <<= 1, carry = next multiplier bit (MSB-first)
            EmitAsm("PUSH_HL");
            EmitAsm("LD_H_B");
            EmitAsm("LD_L_C");
            EmitAsm("ADD_HL_HL");
            EmitAsm("LD_B_H");
            EmitAsm("LD_C_L");
            EmitAsm("POP_HL");

            AsmOperand skipAdd = MakeUniqueLabel("mul16_skip");
            EmitAsm("JP_NC", skipAdd);
            EmitAsm("ADD_HL_DE");
            EmitLabel(skipAdd);

            EmitAsm("POP_AF");
            EmitAsm("DEC_A");
            EmitAsm("JP_NZ", loopLabel);
            return;
        }


        bool isDiv = expr.Match(Tag.Divide, out left, out right);
        bool isMod = false;
        if (!isDiv) isMod = expr.Match(Tag.Modulus, out left, out right);

        if (isDiv || isMod)
        {
            // Select signed quotient/remainder handling from the operand types before preserving and evaluating the inputs.
            bool signedDiv = ShouldUseSignedArithmetic(left, right);

            // DE = dividend, BC = divisor
            CompileIntoHL(left);
            EmitAsm("PUSH_HL");

            CompileIntoHL(right);
            EmitAsm("LD_B_H");
            EmitAsm("LD_C_L");

            EmitAsm("POP_DE");

            AsmOperand divEnd = MakeUniqueLabel("div_end");
            AsmOperand divNonZero = MakeUniqueLabel("div_nonzero");

            // divisor == 0 ? => return 0 (safe behavior)
            EmitAsm("LD_A_B");
            EmitAsm("OR_C");
            EmitAsm("JP_NZ", divNonZero);

            // quotient(=DE)=0, remainder(=HL)=0
            EmitAsm("LD_DE_IMM", new AsmOperand(0, AddressMode.Immediate16));
            EmitAsm("LD_HL_IMM", new AsmOperand(0, AddressMode.Immediate16));
            EmitAsm("JP", divEnd);

            EmitLabel(divNonZero);

            if (signedDiv)
            {
                // Save sign flags:
                // stack top: signQ (dividend^divisor), below: signR (dividend)
                EmitAsm("LD_A_D");
                EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                EmitAsm("PUSH_AF"); // signR

                EmitAsm("LD_A_D");
                EmitAsm("XOR_B");
                EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                EmitAsm("PUSH_AF"); // signQ

                // abs(dividend)
                AsmOperand absDeDone = MakeUniqueLabel("div_absde_done");
                EmitAsm("LD_A_D");
                EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                EmitAsm("JP_Z", absDeDone);
                EmitNegateDE();
                EmitLabel(absDeDone);

                // abs(divisor)
                AsmOperand absBcDone = MakeUniqueLabel("div_absbc_done");
                EmitAsm("LD_A_B");
                EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
                EmitAsm("JP_Z", absBcDone);
                EmitNegateBC();
                EmitLabel(absBcDone);
            }

            EmitUnsignedDivMod16Core_DE_BC();

            if (signedDiv)
            {
                // Apply quotient sign.
                AsmOperand qSignDone = MakeUniqueLabel("div_qsign_done");
                EmitAsm("POP_AF"); // signQ
                EmitAsm("OR_A");
                EmitAsm("JP_Z", qSignDone);
                EmitNegateDE();
                EmitLabel(qSignDone);

                // Apply remainder sign (same sign as dividend).
                AsmOperand rSignDone = MakeUniqueLabel("div_rsign_done");
                EmitAsm("POP_AF"); // signR
                EmitAsm("OR_A");
                EmitAsm("JP_Z", rSignDone);
                EmitNegateHL();
                EmitLabel(rSignDone);
            }

            // Done: quotient in DE, remainder in HL
            if (isDiv)
            {
                EmitAsm("LD_H_D");
                EmitAsm("LD_L_E");
            }

            EmitLabel(divEnd);
            return;
        }



        if (expr.Match(Tag.Multiply, out left, out right))
        {
            CompileIntoHL(left);
            if (right.Match(Tag.Integer, out int rVal))
            {
                if (rVal > 0 && (rVal & (rVal - 1)) == 0) // Power of 2
                {
                    while (rVal > 1) { EmitAsm("ADD_HL_HL"); rVal >>= 1; }
                    return;
                }

                if (rVal == 3) { EmitAsm("LD_D_H"); EmitAsm("LD_E_L"); EmitAsm("ADD_HL_HL"); EmitAsm("ADD_HL_DE"); return; }
                if (rVal == 5) { EmitAsm("LD_D_H"); EmitAsm("LD_E_L"); EmitAsm("ADD_HL_HL"); EmitAsm("ADD_HL_HL"); EmitAsm("ADD_HL_DE"); return; }
                if (rVal == 6) { EmitAsm("LD_D_H"); EmitAsm("LD_E_L"); EmitAsm("ADD_HL_HL"); EmitAsm("ADD_HL_DE"); EmitAsm("ADD_HL_HL"); return; }
                if (rVal == 10) { EmitAsm("ADD_HL_HL"); EmitAsm("LD_D_H"); EmitAsm("LD_E_L"); EmitAsm("ADD_HL_HL"); EmitAsm("ADD_HL_HL"); EmitAsm("ADD_HL_DE"); return; }
            }
            NYI(expr, "Multiplication by this constant is not supported for u16 in HL");
        }

        if (expr.Match(Tag.PreIncrement, out sub) || expr.Match(Tag.PostIncrement, out sub) ||
            expr.Match(Tag.PreDecrement, out sub) || expr.Match(Tag.PostDecrement, out sub))
        {
            if (TryGetOperand(sub, out AsmOperand incOp))
            {
                if (incOp.Mode == AddressMode.HighMem) EmitAsm("LDH_A_MEM", incOp); else EmitAsm("LD_A_MEM", incOp);
                EmitAsm("LD_L_A");
                AsmOperand highOp = new AsmOperand(Maybe.Nothing, incOp.Offset + 1, incOp.Mode, ImmediateModifier.None);
                if (incOp.Mode == AddressMode.HighMem) EmitAsm("LDH_A_MEM", highOp); else EmitAsm("LD_A_MEM", highOp);
                EmitAsm("LD_H_A");

                bool isPost = expr.MatchTag(Tag.PostIncrement) || expr.MatchTag(Tag.PostDecrement);
                bool isInc = expr.MatchTag(Tag.PreIncrement) || expr.MatchTag(Tag.PostIncrement);
                if (isPost) EmitAsm("PUSH_HL");

                if (isInc) EmitAsm("INC_HL");
                else EmitAsm("DEC_HL");

                EmitAsm("LD_A_L");
                if (incOp.Mode == AddressMode.HighMem) EmitAsm("LDH_MEM_A", incOp); else EmitAsm("LD_MEM_A", incOp);
                EmitAsm("LD_A_H");
                if (incOp.Mode == AddressMode.HighMem) EmitAsm("LDH_MEM_A", highOp); else EmitAsm("LD_MEM_A", highOp);

                if (isPost) EmitAsm("POP_HL");
                return;
            }
        }

        if (expr.MatchTag(Tag.Call))
        {
            // Hardening: decide intrinsic return width before emitting.
            // Combine known intrinsic conventions with the call emitter and semantic return type before widening A.
            bool returnsHL = false;
            if (expr.Match(Tag.Call, out Expr callTarget0, out Expr[] callArgs0) &&
                callTarget0.Match(Tag.Name, out string callName0) &&
                IsIntrinsicReturningHL(callName0))
            {
                returnsHL = true;
            }

            // Emit the call (intrinsics may inline and leave result in A or HL depending on the intrinsic)
            CompileCall(expr);

            // Trust CompileCall's intrinsic return-width tracking.
            if (LastCallReturnsHL) returnsHL = true;

            if (returnsHL)
            {
                // Result is already in HL.
                return;
            }

            // Prefer semantic return type (TypeOf) for calls (handles intrinsics like __readpadex).
            CType rt = TypeOf(expr);
            if (IsStructReturnType(rt))
            {
                Error(expr, ErrorCode.ParseError, "struct/union value cannot be used as a scalar expression");
                return;
            }
            if (ReturnsInHL(expr, rt))
            {
                // Result is already in HL.
                return;
            }

            if (ReturnsInHL(expr, TypeOf(expr))) return;

            // 8-bit return in A -> extend according to return signedness
            EmitExtendAIntoHL(ShouldSignExtendExprToHL(expr));
            return;
        }

        // Reject expression forms with no supported word-emission path instead of inventing a result.
        NYI(expr, "Expression too complex for CompileIntoHL");
    }

    // --- Helper Methods ---


    // Recognize word-returning intrinsic names after removing leading underscores and normalizing case.
    bool IsIntrinsicReturningHL(string funcName)
    {
        // Intrinsics that conventionally return u16 in HL.
        if (funcName == null) return false;
        string lower = funcName.Trim().ToLowerInvariant();
        string bare = lower.TrimStart('_');

        switch (bare)
        {
            case "mul16x8":
            case "mac16":
            case "dot3_q8_8":
            case "dot2_q8_8":
            case "smul16x8":
            case "smul16x8_q1_7":
            case "smac16":
            case "smac16_q1_7":
            case "sdot3_q8_8":
            case "sdot3_q1_7":
            case "sdot2_q8_8":
            case "sdot2_q1_7":
            case "getbgmapbase":
            case "getwinmapbase":
            case "readpadex":
            case "farpeek16":
            case "tile_addr":
            case "map_index":
            case "rle_decode_vram":
                return true;
            default:
                return false;
        }
    }

    // Multiply two unsigned bytes by consuming multiplier bits while doubling the multiplicand.
    void EmitMul8x8ToHL_EC(string labelPrefix)
    {
        EmitAsm("PUSH_BC");

        // Input: E=multiplicand (u8), C=multiplier (u8). Output: HL=product (u16).
        // Preserves BC; changes A, DE, HL and flags.
        EmitAsm("LD_D_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("LD_HL_IMM", new AsmOperand(0, AddressMode.Immediate16));
        EmitAsm("LD_B_IMM", new AsmOperand(8, AddressMode.Immediate));

        AsmOperand loopLabel = MakeUniqueLabel(labelPrefix + "_loop");
        AsmOperand skipLabel = MakeUniqueLabel(labelPrefix + "_skip");
        EmitLabel(loopLabel);

        EmitAsm("LD_A_C");
        EmitAsm("RRCA");
        EmitAsm("LD_C_A");
        EmitAsm("JP_NC", skipLabel);
        EmitAsm("ADD_HL_DE");
        EmitLabel(skipLabel);

        // DE <<= 1
        EmitAsm("LD_A_E");
        EmitAsm("ADD_E");
        EmitAsm("LD_E_A");
        EmitAsm("LD_A_D");
        EmitAsm("ADC_D");
        EmitAsm("LD_D_A");

        EmitAsm("DEC_B");
        EmitAsm("JP_NZ", loopLabel);

        EmitAsm("POP_BC");
    }

    // Combine two byte products to compute the high 16 bits of the unsigned 24-bit product.
    void EmitMul16x8Shr8_HL(string labelPrefix)
    {
        // Input: HL=a (u16), C=b (u8). Output: HL=(a*b)>>8 (u16).
        // Uses: (H*b) + ((L*b)>>8)
        EmitAsm("PUSH_HL"); // save a

        // temp1 = (H*b)
        EmitAsm("POP_DE"); // DE = a
        EmitAsm("PUSH_DE"); // keep a on stack
        EmitAsm("LD_A_D"); // A = high byte
        EmitAsm("LD_E_A"); // E = multiplicand
        EmitMul8x8ToHL_EC(labelPrefix + "_hi"); // HL = H*b

        // rearrange stack: [temp1][a]
        EmitAsm("POP_DE"); // DE = a
        EmitAsm("PUSH_HL"); // push temp1
        EmitAsm("PUSH_DE"); // push a
        EmitAsm("POP_HL"); // HL = a

        // temp2 = high(L*b)
        EmitAsm("LD_A_L");
        EmitAsm("LD_E_A");
        EmitMul8x8ToHL_EC(labelPrefix + "_lo"); // HL = L*b
        EmitAsm("LD_A_H"); // A = (L*b)>>8

        // HL = temp1
        EmitAsm("POP_DE"); // DE = temp1
        EmitAsm("LD_H_D");
        EmitAsm("LD_L_E");

        // HL += A
        EmitAsm("LD_B_A");
        EmitAsm("LD_A_L");
        EmitAsm("ADD_B");
        EmitAsm("LD_L_A");
        EmitAsm("LD_A_H");
        EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("LD_H_A");
    }

    // Negate a word modulo 65536 by complementing both bytes and adding one.
    void EmitNegateHL()
    {
        // HL = -HL (two's complement)
        EmitAsm("LD_A_L");
        EmitAsm("CPL");
        EmitAsm("LD_L_A");
        EmitAsm("LD_A_H");
        EmitAsm("CPL");
        EmitAsm("LD_H_A");
        EmitAsm("INC_HL");
    }

    // Fold an expression and interpret its low byte as an unsigned constant.
    bool TryGetU8Const(Expr e, out int u8)
    {
        Expr folded = FoldConstants(e);
        if (folded.Match(Tag.Integer, out int v))
        {
            u8 = v & 0xFF;
            return true;
        }
        u8 = 0;
        return false;
    }

    // Fold an expression and sign-interpret its low byte for coefficient specialization.
    bool TryGetS8Const(Expr e, out int s8)
    {
        Expr folded = FoldConstants(e);
        if (folded.Match(Tag.Integer, out int v))
        {
            int b = v & 0xFF;
            s8 = (b >= 128) ? (b - 256) : b;
            return true;
        }
        s8 = 0;
        return false;
    }

    
// Shift a word left, specializing whole-byte displacement and zeroing counts at least 16.
void EmitShiftLeftLogicalHL(int count)
{
    if (count <= 0) return;

    // Shift >=16 yields 0 in our 16-bit model.
    if (count >= 16)
    {
        EmitAsm("LD_HL_IMM", new AsmOperand(0, AddressMode.Immediate16));
        return;
    }

    // Fast path: <<8 (HL = (L<<8), low byte becomes 0)
    if (count == 8)
    {
        EmitAsm("LD_A_L");
        EmitAsm("LD_H_A");
        EmitAsm("XOR_A");
        EmitAsm("LD_L_A");
        return;
    }

    // Fast path: 9..15 (after shifting out H, only original L contributes)
    if (count > 8)
    {
        int rem = count - 8;
        EmitAsm("LD_A_L");
        for (int i = 0; i < rem; i++) EmitAsm("ADD_A"); // SLA A
        EmitAsm("LD_H_A");
        EmitAsm("XOR_A");
        EmitAsm("LD_L_A");
        return;
    }

    // 1..7: small and fast with ADD HL,HL
    for (int i = 0; i < count; i++) EmitAsm("ADD_HL_HL");
}

// Shift a word right with zero fill, transferring carry from H into L.
void EmitShiftRightLogicalHL(int count)
    {
        if (count <= 0) return;

        // Fast path: >>8
        if (count == 8)
        {
            EmitAsm("LD_A_H");
            EmitAsm("LD_L_A");
            EmitAsm("XOR_A");
            EmitAsm("LD_H_A");
            return;
        }

        for (int i = 0; i < count; i++)
        {
            // H = H >> 1 (logical)
            EmitAsm("LD_A_H");
            EmitAsm("OR_A"); // clear carry
            EmitAsm("RRA");
            EmitAsm("LD_H_A");

            // L = (carry<<7) | (L>>1)
            EmitAsm("LD_A_L");
            EmitAsm("RRA");
            EmitAsm("LD_L_A");
        }
    }

    // Shift a word right with sign fill, reloading the unchanged high byte after extracting its sign.
    void EmitShiftRightArithmeticHL(int count)
    {
        if (count <= 0) return;

        // Fast path: >>8 (sign-extend high byte)
        if (count == 8)
        {
            EmitAsm("LD_A_H");
            EmitAsm("LD_L_A");
            EmitAsm("RLCA"); // carry = sign bit
            EmitAsm("SBC_A"); // A = A - A - carry => 0x00 or 0xFF
            EmitAsm("LD_H_A");
            return;
        }

        for (int i = 0; i < count; i++)
        {
            // Prepare carry = sign(H)
            EmitAsm("LD_A_H");
            EmitAsm("RLCA");

            // H = arithmetic shift right by 1, feeding sign via carry
            EmitAsm("LD_A_H");
            EmitAsm("RRA");
            EmitAsm("LD_H_A");

            // L = (carry<<7) | (L>>1)
            EmitAsm("LD_A_L");
            EmitAsm("RRA");
            EmitAsm("LD_L_A");
        }
    }

    // Negate DE by complementing each byte and propagating the low-byte increment carry.
    void EmitNegateDE()
    {
        EmitAsm("LD_A_E");
        EmitAsm("CPL");
        EmitAsm("ADD_A_IMM", new AsmOperand(1, AddressMode.Immediate));
        EmitAsm("LD_E_A");
        EmitAsm("LD_A_D");
        EmitAsm("CPL");
        EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("LD_D_A");
    }

    // Negate BC by complementing each byte and propagating the low-byte increment carry.
    void EmitNegateBC()
    {
        EmitAsm("LD_A_C");
        EmitAsm("CPL");
        EmitAsm("ADD_A_IMM", new AsmOperand(1, AddressMode.Immediate));
        EmitAsm("LD_C_A");
        EmitAsm("LD_A_B");
        EmitAsm("CPL");
        EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("LD_B_A");
    }

    // Perform 16 restoring-division steps; the caller must handle a zero divisor before entering.
    void EmitUnsignedDivMod16Core_DE_BC()
    {
        // Input : DE=dividend, BC=divisor(>0)
        // Output: DE=quotient, HL=remainder
        EmitAsm("LD_HL_IMM", new AsmOperand(0, AddressMode.Immediate16)); // remainder = 0
        EmitAsm("LD_A_IMM", new AsmOperand(16, AddressMode.Immediate));
        AsmOperand loop = MakeUniqueLabel("div_loop");
        EmitLabel(loop);
        EmitAsm("PUSH_AF");

        // Shift left DE (quotient builder / dividend bits), carry = next input bit
        EmitAsm("LD_A_E");
        EmitAsm("ADD_E");
        EmitAsm("LD_E_A");
        EmitAsm("LD_A_D");
        EmitAsm("ADC_D");
        EmitAsm("LD_D_A");

        // A = carry (0/1)
        EmitAsm("LD_A_IMM", new AsmOperand(0, AddressMode.Immediate));
        EmitAsm("ADC_IMM", new AsmOperand(0, AddressMode.Immediate));

        // remainder <<= 1
        EmitAsm("ADD_HL_HL");
        AsmOperand noIn = MakeUniqueLabel("div_noin");
        EmitAsm("OR_A");
        EmitAsm("JP_Z", noIn);
        EmitAsm("INC_HL");
        EmitLabel(noIn);

        // remainder -= divisor (HL -= BC)
        EmitAsm("LD_A_L");
        EmitAsm("SUB_C");
        EmitAsm("LD_L_A");
        EmitAsm("LD_A_H");
        EmitAsm("SBC_B");
        EmitAsm("LD_H_A");

        AsmOperand borrow = MakeUniqueLabel("div_borrow");
        AsmOperand after = MakeUniqueLabel("div_after");
        EmitAsm("JP_C", borrow);

        // no borrow => quotient bit = 1
        EmitAsm("LD_A_E");
        EmitAsm("OR_IMM", new AsmOperand(1, AddressMode.Immediate));
        EmitAsm("LD_E_A");
        EmitAsm("JP", after);

        EmitLabel(borrow);
        // restore remainder: HL += BC (use DE temp)
        EmitAsm("PUSH_DE");
        EmitAsm("LD_D_B");
        EmitAsm("LD_E_C");
        EmitAsm("ADD_HL_DE");
        EmitAsm("POP_DE");

        EmitLabel(after);
        EmitAsm("POP_AF");
        EmitAsm("DEC_A");
        EmitAsm("JP_NZ", loop);
    }

    // Select zero, power-of-two or near-unit coefficient expansions; return false when a full multiply is needed.
    bool TryEmitDot2Pow2Term(Expr valueExpr, Expr coefExpr, bool signed, bool q1_7, out bool isZero)
    {
        isZero = false;

        if (!signed)
        {
            if (!TryGetU8Const(coefExpr, out int cU8)) return false;
            if (cU8 == 0)
            {
                isZero = true;
                EmitAsm("XOR_A");
                EmitAsm("LD_H_A");
                EmitAsm("LD_L_A");
                return true;
            }
            if ((cU8 & (cU8 - 1)) != 0) return false; // not power-of-two
            int k = 0;
            int t = cU8;
            while (t > 1) { t >>= 1; k++; }
            int shift = (q1_7 ? (7 - k) : (8 - k));
            if (shift < 0) return false;
            CompileIntoHL(valueExpr);
            EmitShiftRightLogicalHL(shift);
            return true;
        }
        else
        {
            if (!TryGetS8Const(coefExpr, out int cS8)) return false;
            if (cS8 == 0)
            {
                isZero = true;
                EmitAsm("XOR_A");
                EmitAsm("LD_H_A");
                EmitAsm("LD_L_A");
                return true;
            }
            // Special-case: Q1.7 near +/-1.0 (0x7F / 0x81) => x*(127/128) without mul.
            // Approximate the positive coefficient with x - floor(x/128), then negate for -127.
            if (q1_7 && (cS8 == 127 || cS8 == -127))
            {
                int signNear = (cS8 < 0) ? -1 : 1;

                // HL = x
                CompileIntoHL(valueExpr);
                EmitAsm("PUSH_HL");

                // HL = x >> 7 (arith)
                EmitShiftRightArithmeticHL(7);

                // DE = x>>7
                EmitAsm("LD_D_H");
                EmitAsm("LD_E_L");

                // HL = x
                EmitAsm("POP_HL");

                // HL = x - (x>>7)
                EmitAsm("LD_A_L");
                EmitAsm("SUB_E");
                EmitAsm("LD_L_A");
                EmitAsm("LD_A_H");
                EmitAsm("SBC_D");
                EmitAsm("LD_H_A");

                if (signNear < 0) EmitNegateHL();
                return true;
            }

            int sign = (cS8 < 0) ? -1 : 1;
            int abs = (cS8 < 0) ? -cS8 : cS8;
            if ((abs & (abs - 1)) != 0) return false; // not power-of-two
            int k = 0;
            int t = abs;
            while (t > 1) { t >>= 1; k++; }
            int shift = (q1_7 ? (7 - k) : (8 - k));
            if (shift < 0) shift = 0;
            CompileIntoHL(valueExpr);
            EmitShiftRightArithmeticHL(shift);
            if (sign < 0) EmitNegateHL();
            return true;
        }
    }

    // Prefer a specialized coefficient term, otherwise preserve the word input while evaluating the byte coefficient.
    void EmitDotTermToHL(Expr valueExpr, Expr coefExpr, bool signed, bool q1_7, string labelPrefix)
    {
        if (TryEmitDot2Pow2Term(valueExpr, coefExpr, signed, q1_7, out bool _))
            return;

        CompileIntoHL(valueExpr);
        EmitAsm("PUSH_HL");
        CompileIntoA(coefExpr);
        EmitAsm("LD_C_A");
        EmitAsm("POP_HL");

        if (signed)
            EmitSMul16x8Shr8_HL(labelPrefix);
        else
            EmitMul16x8Shr8_HL(labelPrefix);

        if (q1_7)
            EmitAsm("ADD_HL_HL");
    }

    // Multiply operand magnitudes, discard eight fractional bits and restore the product sign.
    void EmitSMul16x8Shr8_HL(string labelPrefix)
    {
        // Signed: HL=a (s16), C=b (s8). Output: (a*b)/256 in HL, truncated toward zero.
        // Strategy: sign = sign(a) XOR sign(b); abs -> unsigned mul -> apply sign.

        // Save sign(a) in D
        EmitAsm("LD_A_H");
        EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
        EmitAsm("LD_D_A");

        // Save sign(b) in E
        EmitAsm("LD_A_C");
        EmitAsm("AND_IMM", new AsmOperand(0x80, AddressMode.Immediate));
        EmitAsm("LD_E_A");

        // Push sign = D XOR E (in A)
        EmitAsm("LD_A_D");
        EmitAsm("XOR_E");
        EmitAsm("PUSH_AF");

        // abs(a)
        AsmOperand skipAbsA = MakeUniqueLabel(labelPrefix + "_absA");
        EmitAsm("LD_A_D");
        EmitAsm("OR_A");
        EmitAsm("JP_Z", skipAbsA);
        EmitNegateHL();
        EmitLabel(skipAbsA);

        // abs(b)
        AsmOperand skipAbsB = MakeUniqueLabel(labelPrefix + "_absB");
        EmitAsm("LD_A_E");
        EmitAsm("OR_A");
        EmitAsm("JP_Z", skipAbsB);
        EmitAsm("LD_A_C");
        EmitAsm("CPL");
        EmitAsm("INC_A");
        EmitAsm("LD_C_A");
        EmitLabel(skipAbsB);

        // unsigned mul
        EmitMul16x8Shr8_HL(labelPrefix + "_umul");

        // apply sign
        AsmOperand skipNeg = MakeUniqueLabel(labelPrefix + "_sign");
        EmitAsm("POP_AF");
        EmitAsm("OR_A");
        EmitAsm("JP_Z", skipNeg);
        EmitNegateHL();
        EmitLabel(skipNeg);
    }

    // Use bank one for default or unavailable bank indices; otherwise return the selected WRAM allocator.
    AllocationRegion GetWramXRegion(int bank)
    {
        if (bank <= 0 || bank == 1) return Wram1Region;
        if (bank >= WramXBankRegions.Length || WramXBankRegions[bank] == null)
            return Wram1Region;
        return WramXBankRegions[bank];
    }

    // Classify known CPU ranges for allocation metadata, leaving unclassified addresses as generic RAM.
    static MemoryRegion InferRegionFromCpuAddress(int address)
    {
        if (address >= 0xFF80 && address <= 0xFFFE) return MemoryRegion.HighMem;
        if (address >= 0xFE00 && address <= 0xFE9F) return MemoryRegion.Oam;
        if (address >= 0xD000 && address <= 0xDFFF) return MemoryRegion.WramX;
        if (address >= 0xC000 && address <= 0xCFFF) return MemoryRegion.Wram0;
        if (address >= 0x0000 && address <= 0x7FFF) return MemoryRegion.ProgramRom;
        return MemoryRegion.Ram;
    }

    // Warn once per global symbol when accessing WRAM banks that require caller-managed SVBK selection.
    void MaybeWarnManualSvbkRequired(Expr origin, Symbol sym)
    {
        if (origin == null || sym == null) return;
        if (sym.Tag != SymbolTag.Global) return;
        if (sym.WramBank <= 1) return;
        if (!ManualSvbkWarningDedupe.Add(sym.Name)) return;

        Program.Warning(origin.Source, ErrorCode.ManualSvbkRequired,
            "symbol '{0}' is placed in WRAMX bank {1}; compiler does not auto-switch SVBK in MVP",
            sym.Name, sym.WramBank);
    }

    // Find a supported allocator containing the entire fixed allocation span.
    AllocationRegion FixedAllocationRegion(int address, int end)
    {
        if (address >= Wram0Region.Bottom && end <= 0xCFFF) return Wram0Region;
        if (address >= Wram1Region.Bottom && end <= 0xDFFF) return Wram1Region;
        if (address >= HramRegion.Bottom && end <= HramRegion.Top) return HramRegion;
        if (address >= OamRegion.Bottom && end <= OamRegion.Top) return OamRegion;
        return null;
    }

    // Reject overlap with earlier reservations or automatic allocations, then keep fixed spans sorted by address.
    void ReserveFixedAllocation(Expr origin, string name, int address, int size)
    {
        if (size <= 0) return;
        int end = address + size - 1;
        AllocationRegion allocationRegion = FixedAllocationRegion(address, end);
        if (allocationRegion == null) return;

        foreach (AllocationReservation reservation in allocationRegion.Reservations)
        {
            if (address <= reservation.End && end >= reservation.Begin)
            {
                Error(origin, ErrorCode.FixedAllocationOverlap,
                    "fixed allocation '{0}' at {1:X4}-{2:X4} overlaps fixed allocation '{3}' at {4:X4}-{5:X4}",
                    name, address & 0xFFFF, end & 0xFFFF,
                    reservation.Name, reservation.Begin & 0xFFFF, reservation.End & 0xFFFF);
                return;
            }
        }

        if (address < allocationRegion.Next)
        {
            Error(origin, ErrorCode.FixedAllocationOverlap,
                "fixed allocation '{0}' at {1:X4}-{2:X4} overlaps prior automatic allocation in {3} ending at {4:X4}",
                name, address & 0xFFFF, end & 0xFFFF,
                allocationRegion.Name, (allocationRegion.Next - 1) & 0xFFFF);
            return;
        }

        allocationRegion.Reservations.Add(
            new AllocationReservation(address, end, name));
        allocationRegion.Reservations.Sort(
            (left, right) => left.Begin.CompareTo(right.Begin));
    }


    // Choose aligned global storage, enforce bank/stack constraints and emit the variable placement metadata.
    Symbol DeclareGlobal(Expr origin, MemoryRegion region, CType type, string name, int pragmaAlign = 0)
    {
        int size = SizeOf(origin, type);
        int typeAlign = Math.Max(1, (type != null && type.ForcedAlign > 0) ? type.ForcedAlign : NaturalAlignOf(origin, type));
        int alignment = Math.Max(typeAlign, Math.Max(1, pragmaAlign));
        int address;
        MemoryRegion actualRegion = region;

        if (region.Tag == MemoryRegionTag.Fixed)
        {
            address = region.FixedAddress;
            ReserveFixedAllocation(origin, name, address, size);
            if (Program.StackReserve > 0)
            {
                int fixedEnd = address + Math.Max(0, size - 1);
                int reservedBegin = Program.EffectiveStackAutoLimit + 1;
                int reservedEnd = Program.EffectiveStackTop;
                if (address <= reservedEnd && fixedEnd >= reservedBegin)
                {
                    Error(origin, ErrorCode.StackReservedOverlap,
                        "fixed allocation '{0}' at {1:X4}-{2:X4} overlaps reserved stack range {3:X4}-{4:X4}",
                        name,
                        address & 0xFFFF,
                        fixedEnd & 0xFFFF,
                        reservedBegin & 0xFFFF,
                        reservedEnd & 0xFFFF);
                }
            }
        }
        else if (region.Tag == MemoryRegionTag.HighMem)
        {
            address = Allocate(HramRegion, size, alignment);
        }
        else if (region.Tag == MemoryRegionTag.Oam)
        {
            address = Allocate(OamRegion, size, alignment);
        }
        else if (region.Tag == MemoryRegionTag.Wram0)
        {
            address = Allocate(Wram0Region, size, alignment);
        }
        else if (region.Tag == MemoryRegionTag.WramX && region.WramBank > 0)
        {
            if (region.WramBank > 1 && !Program.IsCgbOnlyTargetRequested())
            {
                Error(origin, ErrorCode.BankedWramRequiresCgbOnly, "banked WRAM requires #pragma rom_cgb cgb_only or --cgb=cgb_only");
            }

            AllocationRegion bankRegion = GetWramXRegion(region.WramBank);
            int bankAddr = TryAllocate(bankRegion, size, alignment);
            if (bankAddr == -1)
            {
                Error(origin, ErrorCode.WramXBankOverflow, "WRAMX bank {0} overflow while placing '{1}' ({2} bytes)", region.WramBank, name, size);
                bankAddr = bankRegion.Bottom;
            }
            address = bankAddr;
            actualRegion = MemoryRegion.WramXBank(region.WramBank);
        }
        else if (region.Tag == MemoryRegionTag.WramX)
        {
            address = Allocate(Wram1Region, size, alignment);
            actualRegion = MemoryRegion.WramX;
        }
        else
        {
            address = AllocatePreferred(size, alignment, GlobalRegions());
            actualRegion = InferRegionFromCpuAddress(address);
        }

        Emit(Tag.Variable, name, address, size, actualRegion);
        return DeclareSymbol(origin, new Symbol(SymbolTag.Global, address, type, name, actualRegion.WramBank));
    }
    // Allocate an aligned static local slot using the hot/cold placement preference for its name.
    Symbol DeclareLocal(Expr origin, CType type, string name)
    {
        int size = SizeOf(origin, type);
        int alignment = Math.Max(1, (type != null && type.ForcedAlign > 0) ? type.ForcedAlign : NaturalAlignOf(origin, type));
        int address = AllocatePreferred(size, alignment, IsHotVar(name) ? HotRegions() : ColdRegions());
        return DeclareSymbol(origin, new Symbol(SymbolTag.Local, address, type, name));
    }
    // Install a binding in the current lexical scope and reject duplicate names in that scope.
    Symbol DeclareSymbol(Expr origin, Symbol s)
    {
        if (CurrentScope.Symbols.ContainsKey(s.Name)) Error(origin, "Symbol redefined: " + s.Name);
        CurrentScope.Symbols.Add(s.Name, s);
        return s;
    }
    // Accept literal-value and expression initializer forms, normalizing literals to source-tagged AST nodes.
    bool TryMatchReadonlyDataDecl(Expr expr, out CType type, out string name, out Expr[] valueExprs)
    {
        if (expr.Match(Tag.ReadonlyData, out type, out name, out int[] values))
        {
            valueExprs = new Expr[values.Length];
            for (int i = 0; i < values.Length; i++)
                valueExprs[i] = Expr.Make(Tag.Integer, values[i]).WithSource(expr.Source);
            return true;
        }
        if (expr.Match(Tag.ReadonlyData, out type, out name, out Expr[] exprValues))
        {
            valueExprs = exprValues ?? Array.Empty<Expr>();
            return true;
        }

        type = null;
        name = null;
        valueExprs = null;
        return false;
    }
    // Convert the legacy integer initializer array to expressions before using the shared readonly-data path.
    void DeclareReadonlyData(Expr origin, CType type, string name, int[] values, int pragmaAlign = 0, string section = null)
    {
        Expr[] exprValues = new Expr[values == null ? 0 : values.Length];
        for (int i = 0; i < exprValues.Length; i++)
            exprValues[i] = Expr.Make(Tag.Integer, values[i]).WithSource(origin.Source);
        DeclareReadonlyData(origin, type, name, exprValues, pragmaAlign, section);
    }

    // Emit section/alignment directives, encode readonly bytes and register a forward-address symbol when needed.
    void DeclareReadonlyData(Expr origin, CType type, string name, Expr[] valueExprs, int pragmaAlign = 0, string section = null)
    {
        if (!string.IsNullOrEmpty(section)) Emit(Tag.Section, section);

	        // Respect __aligned(N) / #pragma align N, plus planned fast-table alignment (B3).
	        int romAlign = ComputeReadonlyDataAlign(origin, type, pragmaAlign);
	        ReadonlyDataPlannedAlign[name] = romAlign;
	        if (romAlign > 1) Emit(Tag.Align, romAlign);

        var relocations = new List<Expr>();
        byte[] bytes = EncodeReadonlyDataInitializers(origin, type, valueExprs ?? Array.Empty<Expr>(), relocations, 0);

        if (!TryFindSymbol(name, out Symbol _existing2))
        {
            DeclareSymbol(origin, new Symbol(SymbolTag.ReadonlyData, 0, type, name));
        }
        if (relocations.Count == 0) Emit(Tag.ReadonlyData, name, bytes);
        else Emit(Tag.ReadonlyData, name, bytes, relocations.ToArray());
    }

    // Route arrays to element encoding and reject multiple initializers for a nonarray object.
    byte[] EncodeReadonlyDataInitializers(Expr origin, CType type, Expr[] valueExprs, List<Expr> relocations, int objectOffset)
    {
        if (type != null && type.IsArray)
            return EncodeReadonlyArrayInitializer(origin, type, valueExprs, relocations, objectOffset);

        int size = Math.Max(1, SizeOf(origin, type));
        if (valueExprs == null || valueExprs.Length == 0)
            return new byte[size];
        if (valueExprs.Length > 1)
            Error(origin, ErrorCode.ParseError, "too many initializers for readonly data");

        return EncodeReadonlyInitializer(origin, type, valueExprs[0], relocations, objectOffset);
    }

    // Encode nested arrays/aggregates recursively or serialize a scalar constant in little-endian byte order.
    byte[] EncodeReadonlyInitializer(Expr origin, CType type, Expr init, List<Expr> relocations, int objectOffset)
    {
        if (type == null) type = CType.UInt8;

        if (IsEmptyReadonlyInitializer(init))
            return new byte[Math.Max(1, SizeOf(origin, type))];

        if (type.IsArray)
        {
            Expr[] items;
            if (init != null && init.MatchAny(Tag.Sequence, out items))
                return EncodeReadonlyArrayInitializer(origin, type, items ?? Array.Empty<Expr>(), relocations, objectOffset);
            return EncodeReadonlyArrayInitializer(origin, type, new Expr[] { init }, relocations, objectOffset);
        }

        if (type.IsStructOrUnion)
            return EncodeReadonlyAggregateInitializer(origin, type, init, relocations, objectOffset);

        Expr[] scalarItems;
        if (init != null && init.MatchAny(Tag.Sequence, out scalarItems))
        {
            if (scalarItems == null || scalarItems.Length == 0)
                return new byte[Math.Max(1, SizeOf(origin, type))];
            if (scalarItems.Length > 1)
                Error(origin, ErrorCode.ParseError, "too many initializers for scalar readonly data");
            init = scalarItems[0];
        }

        int size = Math.Max(1, SizeOf(origin, type));
        if (size > 4)
            Error(origin, ErrorCode.ParseError, "readonly data scalar initializer supports sizes up to 4 bytes (got {0})", size);

        if (type.IsPointer && size == 2 && TryGetReadonlyPointer(init, false, out AsmOperand address))
        {
            relocations.Add(Expr.Make(Tag.Word, objectOffset, address).WithSource(init.Source));
            return new byte[size];
        }

        int value = CalculateConstantExpression(init);
        byte[] bytes = new byte[size];
        uint u = (uint)value;
        for (int i = 0; i < size; i++)
            bytes[i] = (byte)((u >> (8 * i)) & 0xFF);
        return bytes;
    }

    // Allocate the full declared array extent, zero omitted elements and copy each encoded initializer at its stride.
    byte[] EncodeReadonlyArrayInitializer(Expr origin, CType arrayType, Expr[] items, List<Expr> relocations, int objectOffset)
    {
        CType elemType = arrayType.Subtype ?? CType.UInt8;
        int elemSize = Math.Max(1, SizeOf(origin, elemType));
        int declaredCount = GetReadonlyArrayDimension(origin, arrayType);
        if (declaredCount < 0) declaredCount = 0;

        items = items ?? Array.Empty<Expr>();
        if (items.Length > declaredCount)
            Error(origin, ErrorCode.ParseError, "too many initializers for readonly array (got {0}, declared {1})", items.Length, declaredCount);

        byte[] bytes = new byte[declaredCount * elemSize];
        int n = Math.Min(items.Length, declaredCount);
        for (int i = 0; i < n; i++)
        {
            if (IsEmptyReadonlyInitializer(items[i])) continue;
            byte[] elemBytes = EncodeReadonlyInitializer(origin, elemType, items[i], relocations, objectOffset + i * elemSize);
            if (elemBytes.Length != elemSize)
                Error(origin, ErrorCode.ParseError, "readonly array initializer size mismatch for element {0}", i);
            System.Array.Copy(elemBytes, 0, bytes, i * elemSize, elemSize);
        }
        return bytes;
    }

    // Place struct fields at their layout offsets; for a union, encode only the first nonempty selected initializer.
    byte[] EncodeReadonlyAggregateInitializer(Expr origin, CType type, Expr init, List<Expr> relocations, int objectOffset)
    {
        int totalSize = Math.Max(1, SizeOf(origin, type));
        byte[] bytes = new byte[totalSize];

        Expr[] items;
        if (init == null || !init.MatchAny(Tag.Sequence, out items))
        {
            int value = CalculateConstantExpression(init);
            if (value != 0)
                Error(origin, ErrorCode.ParseError, "aggregate readonly initializer requires braces");
            return bytes;
        }

        AggregateInfo info = GetAggregateInfo(origin, type.Name);
        items = items ?? Array.Empty<Expr>();
        if (items.Length > info.Fields.Length)
            Error(origin, ErrorCode.ParseError, "too many initializers for {0}", type.Show());

        if (info.Layout == AggregateLayout.Union)
        {
            for (int i = 0; i < items.Length && i < info.Fields.Length; i++)
            {
                if (IsEmptyReadonlyInitializer(items[i])) continue;
                FieldInfo field = info.Fields[i];
                byte[] fieldBytes = EncodeReadonlyInitializer(origin, field.Type, items[i], relocations, objectOffset + field.Offset);
                int copy = Math.Min(fieldBytes.Length, totalSize);
                System.Array.Copy(fieldBytes, 0, bytes, field.Offset, copy);
                break;
            }
            return bytes;
        }

        for (int i = 0; i < items.Length && i < info.Fields.Length; i++)
        {
            if (IsEmptyReadonlyInitializer(items[i])) continue;
            FieldInfo field = info.Fields[i];
            byte[] fieldBytes = EncodeReadonlyInitializer(origin, field.Type, items[i], relocations, objectOffset + field.Offset);
            int fieldSize = Math.Max(1, SizeOf(origin, field.Type));
            if (fieldBytes.Length != fieldSize)
                Error(origin, ErrorCode.ParseError, "readonly struct initializer size mismatch for field {0}", field.Name);
            System.Array.Copy(fieldBytes, 0, bytes, field.Offset, fieldSize);
        }
        return bytes;
    }

    // Resolve only static object addresses, never the contents of a pointer variable.
    // Symbol bases are preserved so bank placement and dead stripping see the dependency.
    bool TryGetReadonlyPointer(Expr expr, bool addressOfObject, out AsmOperand address)
    {
        address = null;
        if (expr == null) return false;
        if (!addressOfObject && expr.Match(Tag.Cast, out CType castType, out Expr castValue))
            return castType.IsPointer && TryGetReadonlyPointer(castValue, false, out address);
        if (!addressOfObject && expr.Match(Tag.AddressOf, out Expr target))
            return TryGetReadonlyPointer(target, true, out address);

        if (expr.Match(Tag.Name, out string name) && TryFindSymbol(name, out Symbol symbol))
        {
            if (!addressOfObject && (symbol.Type == null || !symbol.Type.IsArray)) return false;
            if (symbol.Tag == SymbolTag.ReadonlyData)
                address = new AsmOperand(name, AddressMode.Immediate);
            else if (symbol.Tag == SymbolTag.Global)
                address = new AsmOperand(symbol.Value, AddressMode.Immediate);
            return address != null;
        }

        if (addressOfObject && expr.Match(Tag.Index, out Expr array, out Expr index) &&
            TryGetReadonlyPointer(array, false, out AsmOperand arrayAddress))
        {
            CType pointerType = TypeOf(array);
            if (!pointerType.IsPointer && !pointerType.IsArray) return false;
            int offset = CalculateConstantExpression(index) * SizeOf(expr, pointerType.Subtype);
            address = new AsmOperand(arrayAddress.Base, arrayAddress.Offset + offset,
                AddressMode.Immediate, ImmediateModifier.None);
            return true;
        }
        if (addressOfObject && expr.Match(Tag.Field, out Expr owner, out string fieldName) &&
            TryGetReadonlyPointer(owner, true, out AsmOperand ownerAddress))
        {
            FieldInfo field = GetFieldInfo(owner, fieldName);
            address = new AsmOperand(ownerAddress.Base, ownerAddress.Offset + field.Offset,
                AddressMode.Immediate, ImmediateModifier.None);
            return true;
        }
        if (!addressOfObject && (expr.Match(Tag.Add, out Expr left, out Expr right) ||
            expr.Match(Tag.Subtract, out left, out right)))
        {
            bool subtract = expr.MatchTag(Tag.Subtract);
            CType pointerType = TypeOf(left);
            if (!subtract && !pointerType.IsPointer && !pointerType.IsArray)
            {
                Expr swap = left; left = right; right = swap;
                pointerType = TypeOf(left);
            }
            if ((pointerType.IsPointer || pointerType.IsArray) &&
                TryGetReadonlyPointer(left, false, out AsmOperand baseAddress))
            {
                int offset = CalculateConstantExpression(right) * SizeOf(expr, pointerType.Subtype);
                if (subtract) offset = -offset;
                address = new AsmOperand(baseAddress.Base, baseAddress.Offset + offset,
                    AddressMode.Immediate, ImmediateModifier.None);
                return true;
            }
        }
        return false;
    }

    // Resolve either a stored array count or its constant dimension expression.
    int GetReadonlyArrayDimension(Expr origin, CType arrayType)
    {
        if (arrayType == null || !arrayType.IsArray) return 1;
        if (arrayType.Tag == CTypeTag.Array) return arrayType.Dimension;
        if (arrayType.Tag == CTypeTag.ArrayWithDimensionExpression)
            return CalculateConstantExpression(arrayType.DimensionExpression);
        return 1;
    }

    // Treat missing or explicitly empty initializer nodes as zero-filled storage.
    bool IsEmptyReadonlyInitializer(Expr expr)
    {
        return expr == null || expr.Match(Tag.Empty);
    }
	    // Combine explicit and natural alignment, optionally page-aligning an unannotated 256-byte lookup table.
	    int ComputeReadonlyDataAlign(Expr origin, CType type, int pragmaAlign)
	    {
	        int align = 0;
	        if (type != null && type.ForcedAlign != 0) align = Math.Max(align, type.ForcedAlign);
	        if (pragmaAlign != 0) align = Math.Max(align, pragmaAlign);

	        // Default: natural alignment if not overridden.
	        if (align <= 0) align = Math.Max(1, NaturalAlignOf(origin, type));

	        // B3) __prg_rom table read optimization helper:
	        // If we are optimizing (-O1) and a readonly u8[256] table did not specify alignment,
	        // we can safely align it to 256 to enable a much cheaper address calculation template.
	        // This only affects ROM layout (adds up to 255 bytes padding) and preserves semantics.
	        if (Program.OptLevel >= 1 && align < 256 && pragmaAlign == 0 && (type == null || type.ForcedAlign == 0))
	        {
	            if (type != null && type.IsArray && type.Dimension == 256)
	            {
	                CType elem = type.Subtype;
	                int elemSize = (elem != null) ? SizeOf(origin, elem) : 0;
	                if (elemSize == 1) align = 256;
	            }
	        }
	        return align;
	    }

	    // Register readonly symbols and planned bank/alignment metadata before function emission needs their addresses.
	    void CompileReadonlyData(Expr expr, int pragmaAlign, string pragmaSection, int romBank)
	    {
	        // Early-pass hook: register readonly data symbols so functions compiled later
	        // can refer to them, and record planned alignment for codegen optimizations.
	        if (TryMatchReadonlyDataDecl(expr, out CType rdType, out string rdName, out Expr[] _rdValueExprs))
	        {
	            // Declare symbol early (if not already declared)
	            if (!TryFindSymbol(rdName, out Symbol _existing))
	                DeclareSymbol(expr, new Symbol(SymbolTag.ReadonlyData, 0, rdType, rdName));

	            int plannedAlign = ComputeReadonlyDataAlign(expr, rdType, pragmaAlign);
	            ReadonlyDataPlannedAlign[rdName] = plannedAlign;
                ReadonlyDataPlannedBank[rdName] = romBank;
	        }

	        // const scalar materialization (-Zconst-scalar-in-rom)
	        if (Program.ConstScalarInRom && expr.Match(Tag.Constant, out CType ctType, out string ctName, out Expr _ctExpr))
	        {
	            if (ctType.IsConst && ctType.IsSimple && !ctType.IsArray && !ctType.IsEnum &&
	                (ctType.SimpleType == CSimpleType.UInt8 || ctType.SimpleType == CSimpleType.Int8 ||
                     ctType.SimpleType == CSimpleType.UInt16 || ctType.SimpleType == CSimpleType.Int16))
	            {
	                if (!TryFindSymbol(ctName, out Symbol _existing2))
	                    DeclareSymbol(expr, new Symbol(SymbolTag.ReadonlyData, 0, ctType, ctName));

	                int plannedAlign = ComputeReadonlyDataAlign(expr, ctType, pragmaAlign);
	                ReadonlyDataPlannedAlign[ctName] = plannedAlign;
                    ReadonlyDataPlannedBank[ctName] = romBank;
	            }
	        }
	    }
    // Use byte alignment for allocation requests without an explicit alignment.
    int TryAllocate(AllocationRegion region, int size)
    {
        return TryAllocate(region, size, 1);
    }

    // Advance past overlapping reserved spans and commit the allocator cursor only when the request fits.
    int TryAllocate(AllocationRegion region, int size, int alignment)
    {
        int addr = alignment <= 1 ? region.Next : AlignUp(region.Next, alignment);
        bool moved;
        do
        {
            moved = false;
            int end = addr + size - 1;
            foreach (AllocationReservation reservation in region.Reservations)
            {
                if (addr <= reservation.End && end >= reservation.Begin)
                {
                    addr = reservation.End + 1;
                    if (alignment > 1) addr = AlignUp(addr, alignment);
                    moved = true;
                    break;
                }
            }
        }
        while (moved);

        if (addr + size - 1 > region.Top) return -1;
        region.Next = addr + size;
        TrackAllocPeak(region);
        return addr;
    }

    // Allocate unaligned storage and report exhaustion instead of returning a usable address.
    int Allocate(AllocationRegion region, int size)
    {
        int addr = TryAllocate(region, size);
        if (addr == -1) Program.Error("Out of memory in " + region.Name);
        return addr;
    }

    // Allocate aligned storage and diagnose a region that cannot satisfy the request.
    int Allocate(AllocationRegion region, int size, int alignment)
    {
        int addr = TryAllocate(region, size, alignment);
        if (addr == -1) Program.Error("Out of memory in " + region.Name);
        return addr;
    }

    // Prefer HRAM for hot values, with permitted WRAM regions as fallbacks.
    AllocationRegion[] HotRegions()
    {
        if (EnableWram1Fallback) return new[] { HramRegion, Wram0Region, Wram1Region };
        return new[] { HramRegion, Wram0Region };
    }
    // Prefer WRAM for cold values and keep HRAM as the final fallback.
    AllocationRegion[] ColdRegions()
    {
        if (EnableWram1Fallback) return new[] { Wram0Region, Wram1Region, HramRegion };
        return new[] { Wram0Region, HramRegion };
    }
    // Restrict ordinary global placement to the enabled WRAM regions.
    AllocationRegion[] GlobalRegions()
    {
        if (EnableWram1Fallback) return new[] { Wram0Region, Wram1Region };
        return new[] { Wram0Region };
    }

    // Apply byte alignment when trying a caller-specified sequence of allocation regions.
    int AllocatePreferred(int size, params AllocationRegion[] regions) => AllocatePreferred(size, 1, regions);

    // Try each permitted region in order and diagnose failure only after every candidate is exhausted.
    int AllocatePreferred(int size, int alignment, params AllocationRegion[] regions)
    {
        foreach (var r in regions)
        {
            int a = TryAllocate(r, size, alignment);
            if (a != -1) return a;
        }
        Program.Error($"Out of memory: could not allocate {size} bytes in any preferred region.");
        return 0;
    }

    // For stack calls, restore the entry SP while preserving the return register, then emit RET.
    void ReturnFromFunction()
    {
        // Stack ABI hardening:
        // restore SP to function-entry value on every return path.
        if (CurrentFunctionIsStackCall && CurrentStackBaseAddr >= 0)
        {
            int returnSize = 0;
            if (ReturnType != null && !(ReturnType.IsSimple && ReturnType.SimpleType == CSimpleType.Void))
            {
                if (ReturnType.IsPointer || ReturnType.IsEnum ||
                    (ReturnType.IsSimple &&
                     (ReturnType.SimpleType == CSimpleType.UInt16 || ReturnType.SimpleType == CSimpleType.Int16)))
                    returnSize = 2;
                else
                    returnSize = 1;
            }

            if (returnSize == 2)
            {
                EmitAsm("LD_D_H");
                EmitAsm("LD_E_L");
                EmitLoadStackBaseIntoHL();
                EmitAsm("LD_SP_HL");
                EmitAsm("LD_H_D");
                EmitAsm("LD_L_E");
            }
            else if (returnSize == 1)
            {
                EmitAsm("LD_B_A");
                EmitLoadStackBaseIntoHL();
                EmitAsm("LD_SP_HL");
                EmitAsm("LD_A_B");
            }
            else
            {
                EmitLoadStackBaseIntoHL();
                EmitAsm("LD_SP_HL");
            }
        }

        EmitAsm("RET");
    }
    // Resolve immediate values, array addresses and scalar memory operands without treating arrays as stored pointers.
    bool TryGetOperand(Expr expr, out AsmOperand operand)
    {
        if (expr.Match(Tag.Name, out string name))
        {
            Symbol sym = FindSymbol(expr, name);
            if (sym == null) { operand = null; return false; }

            // Array variables are not pointers stored in memory.
            // When an array name is used in an expression, it decays to the base address of the array.
            // If we mistakenly treat an array like a pointer variable, we would load the first two bytes of the
            // array as an address, breaking indexing of __wram/__hram arrays and __prg_rom tables.
            if (sym.Type != null && sym.Type.IsArray)
            {
                if (sym.Tag == SymbolTag.ReadonlyData)
                    operand = new AsmOperand(sym.Name, AddressMode.Immediate16);
                else
                    operand = new AsmOperand(sym.Value, AddressMode.Immediate16);
                return true;
            }

            if (sym.Tag == SymbolTag.ReadonlyData)
            {
                // Arrays decay to pointers (immediate address); scalars are read via absolute memory reference.
                operand = sym.Type.IsArray ? new AsmOperand(sym.Name, AddressMode.Immediate16) : new AsmOperand(sym.Name, AddressMode.Absolute);
                return true;
            }

            if (sym.Tag == SymbolTag.Local || sym.Tag == SymbolTag.Global)
            {
                operand = MemOp(sym.Value);
                return true;
            }

            if (sym.Tag == SymbolTag.Constant)
            {
                operand = new AsmOperand(sym.Value, AddressMode.Immediate);
                return true;
            }

            operand = null;
            return false;
        }
        if (expr.Match(Tag.Integer, out int val))
        {
            operand = new AsmOperand(val, AddressMode.Immediate);
            return true;
        }
        operand = null; return false;
    }

    // Expose paired memory operands only for named two-byte scalar locals or globals.
    bool TryGetWideOperand(Expr expr, out WideOperand operand)
    {
        if (expr.Match(Tag.Name, out string name))
        {
            Symbol sym = FindSymbol(expr, name);
            if (sym != null)
            {
                if (sym.Tag == SymbolTag.Global || sym.Tag == SymbolTag.Local)
                {
                    if (sym.Type != null)
                    {
                        // Wide memory load/store is only valid for scalar 16-bit objects.
                        // Arrays decay to addresses and must not be read as [sym],[sym+1].
                        if (sym.Type.IsArray)
                        {
                            operand = null;
                            return false;
                        }
                        int symSize = SizeOf(expr, sym.Type);
                        if (symSize != 2)
                        {
                            operand = null;
                            return false;
                        }
                    }
                    operand = WideMemOp(sym.Value);
                    return true;
                }
            }
        }
        operand = null;
        return false;
    }

    // Resolve pending constant definitions lazily and reject dependency cycles, always clearing the active-resolution marker.
    bool EnsureConstantDeclared(string name, Expr useSite)
    {
        if (string.IsNullOrEmpty(name)) return false;
        if (TryFindSymbol(name, out Symbol existing) && existing != null && existing.Tag == SymbolTag.Constant)
            return true;

        if (!PendingConstants.TryGetValue(name, out PendingConstantInfo info))
            return false;

        if (PendingConstantsResolving.Contains(name))
        {
            Program.Error(useSite.Source, ErrorCode.ParseError, "cyclic constant definition: {0}", name);
            return false;
        }

        PendingConstantsResolving.Add(name);
        try
        {
            int number = CalculateConstantExpression(info.ValueExpr);
            if (!TryFindSymbol(name, out Symbol _existing))
            {
                DeclareSymbol(info.Origin, new Symbol(SymbolTag.Constant, number, info.Type, name));
            }
            return true;
        }
        finally
        {
            PendingConstantsResolving.Remove(name);
        }
    }

    // Carry the interpreted constant and its type together so promotion and casts remain explicit.
    struct ConstEvalResult
    {
        public int Value;
        public CType Type;
        public ConstEvalResult(int value, CType type)
        {
            Value = value;
            Type = type;
        }
    }

    // Evaluate required constants with target-width promotion, short-circuit selection and diagnostics for invalid forms.
    ConstEvalResult CalculateConstantExpressionTyped(Expr expr)
    {
        if (expr == null) return new ConstEvalResult(0, CType.UInt16);

        if (expr.Match(Tag.Integer, out int n))
        {
            CType litType = (n < 0) ? CType.Int16 : ((n > 255) ? CType.UInt16 : CType.UInt8);
            return new ConstEvalResult(n, litType);
        }

        if (expr.Match(Tag.Name, out string nm))
        {
            if (TryFindSymbol(nm, out Symbol sym) && sym != null && sym.Tag == SymbolTag.Constant)
                return new ConstEvalResult(sym.Value, sym.Type ?? CType.UInt16);
            if (EnsureConstantDeclared(nm, expr) &&
                TryFindSymbol(nm, out sym) && sym != null && sym.Tag == SymbolTag.Constant)
                return new ConstEvalResult(sym.Value, sym.Type ?? CType.UInt16);
            Program.Error(expr.Source, ErrorCode.ParseError, "expected constant expression, got name: {0}", nm);
            return new ConstEvalResult(0, CType.UInt16);
        }

        // sizeof(type) or sizeof(expr)
        if (expr.MatchAnyTag(out string ttag, out object arg1) && ttag == Tag.Sizeof)
        {
            if (arg1 is CType ct)
                return new ConstEvalResult(SizeOf(expr, ct), CType.UInt16);
            if (arg1 is Expr ex)
                return new ConstEvalResult(SizeOf(ex), CType.UInt16);
        }

        // offsetof(type, member)
        if (expr.Match(Tag.Offsetof, out CType offTy, out string offPath))
            return new ConstEvalResult(CalculateOffsetOf(expr, offTy, offPath), CType.UInt16);

        // Casts
        if (expr.Match(Tag.Cast, out CType castType, out Expr castSub))
        {
            ConstEvalResult castRes = CalculateConstantExpressionTyped(castSub);
            if (castType == null) return castRes;
            return new ConstEvalResult(NormalizeConstValueForType(castRes.Value, castType), castType);
        }

        // Ternary
        if (expr.Match(Tag.Conditional, out Expr cond, out Expr texpr, out Expr fexpr))
        {
            int cv = CalculateConstantExpressionTyped(cond).Value;
            return (cv != 0) ? CalculateConstantExpressionTyped(texpr) : CalculateConstantExpressionTyped(fexpr);
        }

        Expr a;
        Expr b;

        // Short-circuit logical operators
        if (expr.Match(Tag.LogicalAnd, out a, out b))
        {
            int av = CalculateConstantExpressionTyped(a).Value;
            if (av == 0) return new ConstEvalResult(0, CType.UInt8);
            int bv = CalculateConstantExpressionTyped(b).Value;
            return new ConstEvalResult((bv != 0) ? 1 : 0, CType.UInt8);
        }
        if (expr.Match(Tag.LogicalOr, out a, out b))
        {
            int av = CalculateConstantExpressionTyped(a).Value;
            if (av != 0) return new ConstEvalResult(1, CType.UInt8);
            int bv = CalculateConstantExpressionTyped(b).Value;
            return new ConstEvalResult((bv != 0) ? 1 : 0, CType.UInt8);
        }

        // Binary arithmetic / bitwise / comparison
        string op;
        if (expr.MatchAnyTag(out op, out a, out b))
        {
            bool isArithmetic = op == Tag.Add || op == Tag.Subtract || op == Tag.Multiply ||
                                op == Tag.Divide || op == Tag.Modulus;
            bool isBitwise = op == Tag.BitwiseAnd || op == Tag.BitwiseOr || op == Tag.BitwiseXor;
            bool isShift = op == Tag.ShiftLeft || op == Tag.ShiftRight;
            bool isCompare = op == Tag.Equal || op == Tag.NotEqual ||
                             op == Tag.LessThan || op == Tag.LessThanOrEqual ||
                             op == Tag.GreaterThan || op == Tag.GreaterThanOrEqual;

            if (isArithmetic || isBitwise || isShift || isCompare)
            {
                ConstEvalResult la = CalculateConstantExpressionTyped(a);
                ConstEvalResult rb = CalculateConstantExpressionTyped(b);

                CType promoted;
                if (isShift) promoted = PromoteIntegerBinaryType(la.Type, la.Type);
                else promoted = PromoteIntegerBinaryType(la.Type, rb.Type);

                bool signed = IsSignedIntegerType(promoted);
                int lv = signed ? ToInt16(la.Value) : ToUInt16(la.Value);
                int rv = signed ? ToInt16(rb.Value) : ToUInt16(rb.Value);

                if (isCompare)
                {
                    bool ok = false;
                    if (op == Tag.Equal) ok = (lv == rv);
                    else if (op == Tag.NotEqual) ok = (lv != rv);
                    else if (op == Tag.LessThan) ok = (lv < rv);
                    else if (op == Tag.LessThanOrEqual) ok = (lv <= rv);
                    else if (op == Tag.GreaterThan) ok = (lv > rv);
                    else if (op == Tag.GreaterThanOrEqual) ok = (lv >= rv);
                    return new ConstEvalResult(ok ? 1 : 0, CType.UInt8);
                }

                if ((op == Tag.Divide || op == Tag.Modulus) && rv == 0)
                {
                    Program.Error(expr.Source, ErrorCode.ParseError, "division by zero in constant expression");
                    return new ConstEvalResult(0, promoted);
                }

                bool leftPtrInt = (la.Type != null && la.Type.IsPointer) && (rb.Type != null && (rb.Type.IsInteger || rb.Type.IsEnum));
                bool rightPtrInt = (rb.Type != null && rb.Type.IsPointer) && (la.Type != null && (la.Type.IsInteger || la.Type.IsEnum));
                bool ptrPtrSub = (op == Tag.Subtract) && (la.Type != null && la.Type.IsPointer) && (rb.Type != null && rb.Type.IsPointer);

                int v = 0;
                if (leftPtrInt)
                {
                    int elemSize = 1;
                    if (la.Type.Subtype != null) elemSize = SizeOf(expr, la.Type.Subtype);
                    if (op == Tag.Add) v = lv + (rv * elemSize);
                    else if (op == Tag.Subtract) v = lv - (rv * elemSize);
                    return new ConstEvalResult(NormalizeConstValueForType(v, la.Type), la.Type);
                }
                if (rightPtrInt && op == Tag.Add)
                {
                    int elemSize = 1;
                    if (rb.Type.Subtype != null) elemSize = SizeOf(expr, rb.Type.Subtype);
                    v = rv + (lv * elemSize);
                    return new ConstEvalResult(NormalizeConstValueForType(v, rb.Type), rb.Type);
                }
                if (ptrPtrSub)
                {
                    v = lv - rv;
                    if (la.Type.Subtype != null)
                    {
                        int elemSize = SizeOf(expr, la.Type.Subtype);
                        if (elemSize > 1) v /= elemSize;
                    }
                    return new ConstEvalResult(NormalizeConstValueForType(v, CType.UInt16), CType.UInt16);
                }

                if (op == Tag.Add) v = lv + rv;
                else if (op == Tag.Subtract) v = lv - rv;
                else if (op == Tag.Multiply) v = lv * rv;
                else if (op == Tag.Divide) v = lv / rv;
                else if (op == Tag.Modulus) v = lv % rv;
                else if (op == Tag.BitwiseAnd) v = lv & rv;
                else if (op == Tag.BitwiseOr) v = lv | rv;
                else if (op == Tag.BitwiseXor) v = lv ^ rv;
                else if (op == Tag.ShiftLeft)
                {
                    int shift = rb.Value & 31;
                    v = lv << shift;
                }
                else if (op == Tag.ShiftRight)
                {
                    int shift = rb.Value & 31;
                    if (signed) v = lv >> shift;
                    else v = ToUInt16(lv) >> shift;
                }

                return new ConstEvalResult(NormalizeConstValueForType(v, promoted), promoted);
            }
        }

        // Unary operators
        if (expr.Match(Tag.BitwiseNot, out Expr unarySub))
        {
            ConstEvalResult sv = CalculateConstantExpressionTyped(unarySub);
            CType promoted = PromoteIntegerBinaryType(sv.Type, sv.Type);
            bool signed = IsSignedIntegerType(promoted);
            int x = signed ? ToInt16(sv.Value) : ToUInt16(sv.Value);
            return new ConstEvalResult(NormalizeConstValueForType(~x, promoted), promoted);
        }
        if (expr.Match(Tag.LogicalNot, out Expr unarySub2))
        {
            return new ConstEvalResult(CalculateConstantExpressionTyped(unarySub2).Value == 0 ? 1 : 0, CType.UInt8);
        }

        Program.Error(expr.Source, ErrorCode.ParseError, "expected constant expression, got: {0}", expr.Show());
        return new ConstEvalResult(0, CType.UInt16);
    }

    // Return the value portion of the typed constant evaluation result.
    int CalculateConstantExpression(Expr expr)
    {
        return CalculateConstantExpressionTyped(expr).Value;
    }

    // Resolve a function or planned readonly bank and expose its low byte.
    bool TryGetKnownRomBankForSymbol(string name, out int bank)
    {
        bank = -1;
        if (string.IsNullOrEmpty(name)) return false;

        if (Functions.TryGetValue(name, out CFunctionInfo funcInfo) && funcInfo.RomBank >= 0)
        {
            bank = funcInfo.RomBank & 0xFF;
            return true;
        }

        if (ReadonlyDataPlannedBank.TryGetValue(name, out int readonlyBank))
        {
            bank = readonlyBank & 0xFF;
            return true;
        }

        return false;
    }

    // Accept a folded bank byte or a bankof call on a symbol with known placement metadata.
    bool TryResolveBankExprToConstU8(Expr expr, out int bank)
    {
        bank = 0;
        Expr folded = FoldConstants(expr);
        if (folded.Match(Tag.Integer, out int foldedBank))
        {
            bank = foldedBank & 0xFF;
            return true;
        }

        if (expr.MatchAny(Tag.Call, out Expr bankTarget, out Expr[] bankArgs) &&
            bankTarget.Match(Tag.Name, out string bankFuncName) &&
            bankFuncName == "__bankof" &&
            bankArgs != null &&
            bankArgs.Length == 1 &&
            bankArgs[0].Match(Tag.Name, out string bankSymName) &&
            TryGetKnownRomBankForSymbol(bankSymName, out int knownBank))
        {
            bank = knownBank & 0xFF;
            return true;
        }

        return false;
    }

    // Attempt typed folding, including known bank/target intrinsics; unsupported or zero-divisor operations return false.
    bool TryEvaluateConstantExpressionTyped(Expr expr, out ConstEvalResult result)
    {
        result = new ConstEvalResult(0, CType.UInt16);
        if (expr == null) return true;

        if (expr.Match(Tag.Integer, out int n))
        {
            CType litType = (n < 0) ? CType.Int16 : ((n > 255) ? CType.UInt16 : CType.UInt8);
            result = new ConstEvalResult(n, litType);
            return true;
        }

        if (expr.Match(Tag.Name, out string nm))
        {
            if (TryFindSymbol(nm, out Symbol sym) && sym != null && sym.Tag == SymbolTag.Constant)
            {
                result = new ConstEvalResult(sym.Value, sym.Type ?? CType.UInt16);
                return true;
            }
            if (EnsureConstantDeclared(nm, expr) &&
                TryFindSymbol(nm, out sym) && sym != null && sym.Tag == SymbolTag.Constant)
            {
                result = new ConstEvalResult(sym.Value, sym.Type ?? CType.UInt16);
                return true;
            }
            return false;
        }

        if (expr.MatchAnyTag(out string ttag, out object arg1) && ttag == Tag.Sizeof)
        {
            if (arg1 is CType ct)
            {
                result = new ConstEvalResult(SizeOf(expr, ct), CType.UInt16);
                return true;
            }
            if (arg1 is Expr ex)
            {
                result = new ConstEvalResult(SizeOf(ex), CType.UInt16);
                return true;
            }
            return false;
        }

        if (expr.Match(Tag.Offsetof, out CType offTy, out string offPath))
        {
            result = new ConstEvalResult(CalculateOffsetOf(expr, offTy, offPath), CType.UInt16);
            return true;
        }

        if (expr.MatchAny(Tag.Call, out Expr callTarget, out Expr[] callArgs) && callTarget.Match(Tag.Name, out string foldFuncName))
        {
            if (foldFuncName == "__bankof")
            {
                if (callArgs != null && callArgs.Length == 1 &&
                    callArgs[0].Match(Tag.Name, out string bankSym) &&
                    TryGetKnownRomBankForSymbol(bankSym, out int knownBank))
                {
                    result = new ConstEvalResult(knownBank & 0xFF, CType.UInt8);
                    return true;
                }
                return false;
            }

            if (foldFuncName == "__cgb_is_cgb" &&
                callArgs != null && callArgs.Length == 0 &&
                Program.TryGetKnownCgbRuntimeValue(out int knownCgbValue))
            {
                result = new ConstEvalResult((knownCgbValue != 0) ? 1 : 0, CType.UInt8);
                return true;
            }
        }

        if (expr.Match(Tag.Cast, out CType castType, out Expr castSub))
        {
            if (!TryEvaluateConstantExpressionTyped(castSub, out ConstEvalResult castRes)) return false;
            CType finalType = castType ?? castRes.Type;
            result = new ConstEvalResult(NormalizeConstValueForType(castRes.Value, finalType), finalType);
            return true;
        }

        if (expr.Match(Tag.Conditional, out Expr cond, out Expr texpr, out Expr fexpr))
        {
            if (!TryEvaluateConstantExpressionTyped(cond, out ConstEvalResult condRes)) return false;
            return TryEvaluateConstantExpressionTyped((condRes.Value != 0) ? texpr : fexpr, out result);
        }

        Expr a;
        Expr b;
        if (expr.Match(Tag.LogicalAnd, out a, out b))
        {
            if (!TryEvaluateConstantExpressionTyped(a, out ConstEvalResult av)) return false;
            if (av.Value == 0)
            {
                result = new ConstEvalResult(0, CType.UInt8);
                return true;
            }

            if (!TryEvaluateConstantExpressionTyped(b, out ConstEvalResult bv)) return false;
            result = new ConstEvalResult((bv.Value != 0) ? 1 : 0, CType.UInt8);
            return true;
        }
        if (expr.Match(Tag.LogicalOr, out a, out b))
        {
            if (!TryEvaluateConstantExpressionTyped(a, out ConstEvalResult av)) return false;
            if (av.Value != 0)
            {
                result = new ConstEvalResult(1, CType.UInt8);
                return true;
            }

            if (!TryEvaluateConstantExpressionTyped(b, out ConstEvalResult bv)) return false;
            result = new ConstEvalResult((bv.Value != 0) ? 1 : 0, CType.UInt8);
            return true;
        }

        string op;
        if (expr.MatchAnyTag(out op, out a, out b))
        {
            bool isArithmetic = op == Tag.Add || op == Tag.Subtract || op == Tag.Multiply ||
                                op == Tag.Divide || op == Tag.Modulus;
            bool isBitwise = op == Tag.BitwiseAnd || op == Tag.BitwiseOr || op == Tag.BitwiseXor;
            bool isShift = op == Tag.ShiftLeft || op == Tag.ShiftRight;
            bool isCompare = op == Tag.Equal || op == Tag.NotEqual ||
                             op == Tag.LessThan || op == Tag.LessThanOrEqual ||
                             op == Tag.GreaterThan || op == Tag.GreaterThanOrEqual;

            if (isArithmetic || isBitwise || isShift || isCompare)
            {
                if (!TryEvaluateConstantExpressionTyped(a, out ConstEvalResult la) ||
                    !TryEvaluateConstantExpressionTyped(b, out ConstEvalResult rb))
                    return false;

                CType promoted = isShift ? PromoteIntegerBinaryType(la.Type, la.Type)
                                         : PromoteIntegerBinaryType(la.Type, rb.Type);
                bool signed = IsSignedIntegerType(promoted);
                int lv = signed ? ToInt16(la.Value) : ToUInt16(la.Value);
                int rv = signed ? ToInt16(rb.Value) : ToUInt16(rb.Value);

                if (isCompare)
                {
                    bool ok = false;
                    if (op == Tag.Equal) ok = (lv == rv);
                    else if (op == Tag.NotEqual) ok = (lv != rv);
                    else if (op == Tag.LessThan) ok = (lv < rv);
                    else if (op == Tag.LessThanOrEqual) ok = (lv <= rv);
                    else if (op == Tag.GreaterThan) ok = (lv > rv);
                    else if (op == Tag.GreaterThanOrEqual) ok = (lv >= rv);
                    result = new ConstEvalResult(ok ? 1 : 0, CType.UInt8);
                    return true;
                }

                if ((op == Tag.Divide || op == Tag.Modulus) && rv == 0) return false;

                bool leftPtrInt = (la.Type != null && la.Type.IsPointer) && (rb.Type != null && (rb.Type.IsInteger || rb.Type.IsEnum));
                bool rightPtrInt = (rb.Type != null && rb.Type.IsPointer) && (la.Type != null && (la.Type.IsInteger || la.Type.IsEnum));
                bool ptrPtrSub = (op == Tag.Subtract) && (la.Type != null && la.Type.IsPointer) && (rb.Type != null && rb.Type.IsPointer);

                int v = 0;
                if (leftPtrInt)
                {
                    int elemSize = 1;
                    if (la.Type.Subtype != null) elemSize = SizeOf(expr, la.Type.Subtype);
                    if (op == Tag.Add) v = lv + (rv * elemSize);
                    else if (op == Tag.Subtract) v = lv - (rv * elemSize);
                    result = new ConstEvalResult(NormalizeConstValueForType(v, la.Type), la.Type);
                    return true;
                }
                if (rightPtrInt && op == Tag.Add)
                {
                    int elemSize = 1;
                    if (rb.Type.Subtype != null) elemSize = SizeOf(expr, rb.Type.Subtype);
                    v = rv + (lv * elemSize);
                    result = new ConstEvalResult(NormalizeConstValueForType(v, rb.Type), rb.Type);
                    return true;
                }
                if (ptrPtrSub)
                {
                    v = lv - rv;
                    if (la.Type.Subtype != null)
                    {
                        int elemSize = SizeOf(expr, la.Type.Subtype);
                        if (elemSize > 1) v /= elemSize;
                    }
                    result = new ConstEvalResult(NormalizeConstValueForType(v, CType.UInt16), CType.UInt16);
                    return true;
                }

                if (op == Tag.Add) v = lv + rv;
                else if (op == Tag.Subtract) v = lv - rv;
                else if (op == Tag.Multiply) v = lv * rv;
                else if (op == Tag.Divide) v = lv / rv;
                else if (op == Tag.Modulus) v = lv % rv;
                else if (op == Tag.BitwiseAnd) v = lv & rv;
                else if (op == Tag.BitwiseOr) v = lv | rv;
                else if (op == Tag.BitwiseXor) v = lv ^ rv;
                else if (op == Tag.ShiftLeft) v = lv << (rb.Value & 31);
                else if (op == Tag.ShiftRight)
                {
                    int shift = rb.Value & 31;
                    v = signed ? (lv >> shift) : (ToUInt16(lv) >> shift);
                }

                result = new ConstEvalResult(NormalizeConstValueForType(v, promoted), promoted);
                return true;
            }
        }

        if (expr.Match(Tag.BitwiseNot, out Expr unarySub))
        {
            if (!TryEvaluateConstantExpressionTyped(unarySub, out ConstEvalResult sv)) return false;
            CType promoted = PromoteIntegerBinaryType(sv.Type, sv.Type);
            bool signed = IsSignedIntegerType(promoted);
            int x = signed ? ToInt16(sv.Value) : ToUInt16(sv.Value);
            result = new ConstEvalResult(NormalizeConstValueForType(~x, promoted), promoted);
            return true;
        }
        if (expr.Match(Tag.LogicalNot, out Expr unarySub2))
        {
            if (!TryEvaluateConstantExpressionTyped(unarySub2, out ConstEvalResult logicalSub)) return false;
            result = new ConstEvalResult(logicalSub.Value == 0 ? 1 : 0, CType.UInt8);
            return true;
        }

        return false;
    }

    // Sum offsets through embedded aggregate members, rejecting implicit pointer traversal between nested fields.
    int CalculateOffsetOf(Expr origin, CType type, string memberPath)
    {
        if (type == null) { Program.Error(origin.Source, ErrorCode.ParseError, "offsetof requires a type"); return 0; }

        CType t = type.WithoutConst();
        // Convenience: allow pointer-to-aggregate in offsetof, but treat it as the aggregate itself.
        if (!t.IsStructOrUnion && t.IsPointer) t = t.Subtype;

        if (t == null || !t.IsStructOrUnion)
        {
            Program.Error(origin.Source, ErrorCode.ParseError, "offsetof requires a struct/union type, got: {0}", type.Show());
            return 0;
        }

        if (string.IsNullOrEmpty(memberPath))
        {
            Program.Error(origin.Source, ErrorCode.ParseError, "offsetof requires a member name");
            return 0;
        }

        int offset = 0;
        string[] parts = memberPath.Split(new[] { '.' }, StringSplitOptions.RemoveEmptyEntries);
        CType curType = t;

        for (int i = 0; i < parts.Length; i++)
        {
            string part = parts[i];
            AggregateInfo ai = GetAggregateInfo(origin, curType.Name);
            FieldInfo fi = null;
            for (int k = 0; k < ai.Fields.Length; k++)
            {
                if (ai.Fields[k].Name == part) { fi = ai.Fields[k]; break; }
            }
            if (fi == null)
            {
                Program.Error(origin.Source, ErrorCode.ParseError, "offsetof: no such field '{0}' in {1}", part, curType.Show());
                return 0;
            }

            offset += fi.Offset;

            // Recurse into nested aggregates if needed.
            if (i != parts.Length - 1)
            {
                CType next = fi.Type;
                if (next != null && next.IsPointer)
                {
                    // Standard C offsetof uses '.' member designator; pointers do not automatically dereference.
                    Program.Error(origin.Source, ErrorCode.ParseError, "offsetof: field '{0}' is a pointer; nested member requires an embedded struct/union, not pointer", part);
                    return 0;
                }
                if (next == null || !next.IsStructOrUnion)
                {
                    Program.Error(origin.Source, ErrorCode.ParseError, "offsetof: field '{0}' is not a struct/union; cannot access nested member", part);
                    return 0;
                }
                curType = next;
            }
        }

        return offset;
    }
    // Materialize an expression-sized outer array dimension when the type carries one.
    CType CalculateConstantArrayDimensions(CType type)
    {
        if (type.Tag == CTypeTag.ArrayWithDimensionExpression)
        {
            int dim = CalculateConstantExpression(type.DimensionExpression); return CType.MakeArray(type.Subtype, dim);
        }
        return type;
    }
    // Resolve a name with diagnostics and issue any required manual-SVBK warning for the selected global.
    Symbol FindSymbol(Expr origin, string name)
    {
        TryFindSymbol(name, out Symbol s);
        if (s == null) Error(origin, "Undefined symbol: " + name);
        MaybeWarnManualSvbkRequired(origin, s);
        return s;
    }
    // Search outward from the current lexical scope so the nearest binding wins.
    bool TryFindSymbol(string name, out Symbol s)
    {
        for (LexicalScope scope = CurrentScope; scope != null; scope = scope.Outer)
            if (scope.Symbols.TryGetValue(name, out s)) return true;
        s = null; return false;
    }
    // Determine expression storage width through its inferred type.
    int SizeOf(Expr expr) => SizeOf(expr, TypeOf(expr));
    // Compute scalar, pointer, array or complete aggregate size, diagnosing unsized arrays and incomplete aggregates.
    int SizeOf(Expr origin, CType type)
    {
        if (type.IsSimple && (type.SimpleType == CSimpleType.UInt16 || type.SimpleType == CSimpleType.Int16)) return 2;
        if (type.IsPointer) return 2;
        if (type.IsFunction) return 2;
        if (type.IsEnum) return 2;
        
        if (type.IsArray)
        {
            if (type.Subtype == null) return 2;

            int elementSize = SizeOf(origin, type.Subtype);

            int dim = 0;
            if (type.Tag == CTypeTag.Array) dim = type.Dimension;
            else if (type.Tag == CTypeTag.ArrayWithDimensionExpression)
                dim = CalculateConstantExpression(type.DimensionExpression);

            if (dim <= 0)
            {
                Program.Error(origin.Source, ErrorCode.ParseError, "array dimension is not a constant positive integer: {0}", type.Show());
                dim = 1;
            }

            return elementSize * dim;
        }
        
        if (type.IsStructOrUnion)
        {
            // Opaque/incomplete aggregates cannot be sized.
            AggregateInfo ai;
            if (!AggregateTypes.TryGetValue(type.Name, out ai) || ai.TotalSize < 0)
            {
                Program.Error(origin.Source, ErrorCode.IncompleteType, "incomplete struct/union type: {0}", type.Show());
                return 1;
            }
            return ai.TotalSize;
        }
        return 1;
    }

    // Infer source-level result types, including array decay, intrinsic returns and this compiler's integer promotion rules.
    CType TypeOf(Expr expr)
    {
        if (expr == null) return CType.UInt8;
        if (expr.Match(Tag.Name, out string name))
        {
            if (TryFindSymbol(name, out Symbol sym))
            {
                if (sym.Type != null && sym.Type.IsArray)
                    return CType.MakePointer(sym.Type.Subtype);
                return sym.Type;
            }
            if (Functions.TryGetValue(name, out CFunctionInfo fnInfo))
                return CType.MakeFunction(fnInfo.ReturnType, (fnInfo.Parameters ?? Array.Empty<FieldInfo>()).Select(p => p.Type).ToArray());
            return CType.UInt8;
        }
        if (expr.Match(Tag.Integer, out int val))
            return (val < 0) ? CType.Int16 : ((val > 255) ? CType.UInt16 : CType.UInt8);
        if (expr.Tag == Tag.Sizeof) return CType.UInt16;
        if (expr.Tag == Tag.Offsetof) return CType.UInt16;

        Expr sub;
        if (expr.Match(Tag.Cast, out CType castType, out sub)) return castType;

        if (expr.Match(Tag.BitwiseNot, out sub))
        {
            CType subType = TypeOf(sub);
            if (IsIntegerLike(subType)) return PromoteIntegerBinaryType(subType, subType);
            return (SizeOf(sub) == 2) ? CType.UInt16 : CType.UInt8;
        }

        if (expr.Match(Tag.LogicalNot, out sub)) return CType.UInt8;

        if (expr.Match(Tag.Slice, out Expr slicePtr, out Expr sliceLen))
            return TypeOf(slicePtr);

        if (expr.Match(Tag.Load, out sub))
        {
            CType loadType = TypeOf(sub);
            if (loadType.IsPointer) return loadType.Subtype;
            if (loadType.IsArray) return loadType.Subtype;
            return CType.UInt8;
        }

        if (expr.Match(Tag.AddressOf, out sub))
        {
            if (sub.Match(Tag.Name, out string n) && TryFindSymbol(n, out Symbol s2) && s2.Type != null && s2.Type.IsArray)
                return CType.MakePointer(s2.Type);
            return CType.MakePointer(TypeOf(sub));
        }

        Expr left, right;
        if (expr.Match(Tag.Index, out left, out right))
        {
            CType indexType = TypeOf(left);
            if (indexType.IsArray || indexType.IsPointer) return indexType.Subtype;
            return CType.UInt8;
        }

        string fieldName;
        if (expr.Match(Tag.Field, out left, out fieldName))
        {
            FieldInfo f = GetFieldInfo(left, fieldName);
            if (f != null) return f.Type;
            return CType.UInt8;
        }

        if (expr.MatchAny(Tag.Call, out Expr callTarget, out Expr[] callArgs))
        {
            // Function pointer call: (fp)(...) / (*fp)(...)
            // If the call target is a pointer-to-function, infer return type from the signature.
            CType ctCall = TypeOf(callTarget);
            if (ctCall != null && ctCall.IsPointer && ctCall.Subtype != null && ctCall.Subtype.IsFunction)
            {
                CType fnT = ctCall.Subtype;
                if (fnT.Subtype != null) return fnT.Subtype;
                return CType.UInt8;
            }
            if (ctCall != null && ctCall.IsFunction)
            {
                if (ctCall.Subtype != null) return ctCall.Subtype;
                return CType.UInt8;
            }

            if (callTarget.Match(Tag.Name, out string funcName))
            {
                if (TryFindSymbol(funcName, out Symbol fpSym) &&
                    fpSym.Type != null &&
                    fpSym.Type.IsPointer &&
                    fpSym.Type.Subtype != null &&
                    fpSym.Type.Subtype.IsFunction)
                {
                    CType fnT = fpSym.Type.Subtype;
                    if (fnT.Subtype != null) return fnT.Subtype;
                    return CType.UInt8;
                }

                // Built-in intrinsics
                if (funcName == "__memcpy" || funcName == "__memset") return CType.Void;
                if (funcName == "__bankswitch") return CType.Void;
                if (funcName == "__bankof") return CType.UInt8;
                if (funcName == "__cgb_is_cgb") return CType.UInt8;
                if (funcName == "__wait_vblank" || funcName == "__wait_ly") return CType.Void;
                if (funcName == "__scroll_bg_set" ||
                    funcName == "__scroll_bg_x_set" ||
                    funcName == "__scroll_bg_y_set" ||
                    funcName == "__scroll_win_set" ||
                    funcName == "__scroll_win_x_set" ||
                    funcName == "__scroll_win_y_set" ||
                    funcName == "__scroll_bg_set_buffered" ||
                    funcName == "__scroll_bg_x_set_buffered" ||
                    funcName == "__scroll_bg_y_set_buffered" ||
                    funcName == "__scroll_win_set_buffered" ||
                    funcName == "__scroll_win_x_set_buffered" ||
                    funcName == "__scroll_win_y_set_buffered" ||
                    funcName == "__scroll_flush" ||
                    funcName == "__scroll_split_reset" ||
                    funcName == "__scroll_split_push" ||
                    funcName == "__scroll_split_push_ex" ||
                    funcName == "__scroll_split_commit" ||
                    funcName == "__scroll_win_show" ||
                    funcName == "__scroll_win_hide" ||
                    funcName == "__scroll_bg_add" ||
                    funcName == "__scroll_win_add") return CType.Void;
                if (funcName == "__scroll_bg_x_get" ||
                    funcName == "__scroll_bg_y_get" ||
                    funcName == "__scroll_win_x_get" ||
                    funcName == "__scroll_win_y_get") return CType.UInt8;
                if (funcName == "__critical_enter") return CType.UInt8;
                if (funcName == "__critical_leave") return CType.Void;
                if (funcName == "__oam_dma") return CType.Void;
                if (funcName == "__vram_memcpy" || funcName == "__vram_memcpy_unsafe" ||
                    funcName == "__vram_memset" || funcName == "__vram_memset_unsafe" ||
                    funcName == "__vram_copy" || funcName == "__vram_fill" ||
                    funcName == "__vram_copy_hblank" || funcName == "__vram_copy_dma")
                    return CType.Void;
                if (funcName == "__far_memcpy" || funcName == "__farmemcpy") return CType.Void;
                if (funcName == "__sram_read8") return CType.UInt8;
                if (funcName == "__sram_write8") return CType.Void;
                if (funcName == "__rng8") return CType.UInt8;
                if (funcName == "__rng_seed") return CType.Void;
                if (funcName == "__svbk_get" || funcName == "__svbk_set") return CType.UInt8;
                if (funcName == "__cgb_safe_set_vbk" ||
                    funcName == "__cgb_safe_set_svbk" ||
                    funcName == "__cgb_safe_set_bgpi" ||
                    funcName == "__cgb_safe_set_bcps" ||
                    funcName == "__cgb_safe_set_bgpd" ||
                    funcName == "__cgb_safe_set_bcpd" ||
                    funcName == "__cgb_safe_set_obpi" ||
                    funcName == "__cgb_safe_set_ocps" ||
                    funcName == "__cgb_safe_set_obpd" ||
                    funcName == "__cgb_safe_set_ocpd" ||
                    funcName == "__cgb_safe_set_hdma1" ||
                    funcName == "__cgb_safe_set_hdma2" ||
                    funcName == "__cgb_safe_set_hdma3" ||
                    funcName == "__cgb_safe_set_hdma4" ||
                    funcName == "__cgb_safe_set_hdma5") return CType.Void;
                if (funcName == "__farcall")
                {
                    if (callArgs != null && callArgs.Length == 2 && callArgs[1].Match(Tag.Name, out string farTargetName))
                    {
                        if (Functions.TryGetValue(farTargetName, out CFunctionInfo farTarget))
                            return farTarget.ReturnType;
                    }
                    return CType.UInt8;
                }
                if (funcName == "__settile" ||
                    funcName == "__settile_unsafe" ||
                    funcName == "__settile_fast" ||
                    funcName == "__settile_xy" ||
                    funcName == "__settile_rect" ||
                    funcName == "__settile_row" ||
                    funcName == "__settile_col" ||
                    funcName == "__settile_bulk" || funcName == "__settile_bulk_fast" ||
                    funcName == "__settileat" ||
                    funcName == "__settileat_unsafe" ||
                    funcName == "__settilewin" ||
                    funcName == "__settilewin_unsafe" ||
                    funcName == "__settilebg" ||
                    funcName == "__settilebg_unsafe" ||
                    funcName == "__settileattr" || funcName == "__settileattr_unsafe" ||
                    funcName == "__settilecgb" || funcName == "__settilecgb_unsafe" ||
                    funcName == "__settileatattr" || funcName == "__settileatattr_unsafe" ||
                    funcName == "__settileatcgb" || funcName == "__settileatcgb_unsafe" ||
                    funcName == "__settilewinattr" || funcName == "__settilewinattr_unsafe" ||
                    funcName == "__settilewincgb" || funcName == "__settilewincgb_unsafe" ||
                    funcName == "__settilebgattr" || funcName == "__settilebgattr_unsafe" ||
                    funcName == "__settilebgcgb" || funcName == "__settilebgcgb_unsafe" ||
                    funcName == "__settilebg16_buf" || funcName == "__settilebg16cgb_buf" ||
                    funcName == "__settilebg16_flush" || funcName == "__settilebg16cgb_flush" ||
                    funcName == "__fill_tilemap" ||
                    funcName == "__settilemap_rect" ||
                    funcName == "__settileattr_bulk" || funcName == "__settileattr_bulk_fast" ||
                    funcName == "__settilecgb_bulk" || funcName == "__settilecgb_bulk_fast") return CType.Void;

if (funcName == "__getbgmapbase" || funcName == "__getwinmapbase" ||
    funcName == "__tile_addr" || funcName == "__map_index" ||
    funcName == "__farpeek16" || funcName == "__rle_decode_vram") return CType.UInt16;
if (funcName == "__readpadex") return CType.UInt16;
                if (funcName == "__readpad" ||
                    funcName == "__readpaddir" ||
                    funcName == "__readpadbtn") return CType.UInt8;
                if (funcName == "__farpeek8" || funcName == "__xy_in_rect" ||
                    funcName == "__manhattan" || funcName == "__bit_test")
                    return CType.UInt8;
                if (funcName == "__bit_set" || funcName == "__bit_clear" ||
                    funcName == "__bit_toggle" || funcName == "__farcall_ptr" ||
                    funcName == "__memcpy_small" || funcName == "__memset_small" ||
                    funcName == "__copy16" || funcName == "__copy32")
                    return CType.Void;

                if (funcName == "__padrep_init" ||
                    funcName == "__padrep_reset") return CType.Void;
                if (funcName == "__padrep_lr") return CType.UInt8;
                if (funcName == "__padrep_down") return CType.UInt8;
                if (funcName == "__padrep" ||
                    funcName == "__padrep_mask") return CType.UInt8;
                if (funcName == "__mul8x8_hi") return CType.UInt8;
                if (funcName == "__mul16x8" || funcName == "__mac16" || funcName == "__dot3_q8_8" ||
                    funcName == "__dot2_q8_8" ||
                    funcName == "__smul16x8" || funcName == "__smul16x8_q1_7" ||
                    funcName == "__smac16" || funcName == "__smac16_q1_7" ||
                    funcName == "__sdot3_q8_8" || funcName == "__sdot3_q1_7" ||
                    funcName == "__sdot2_q8_8" || funcName == "__sdot2_q1_7")
                    return CType.UInt16;

                // GBFB convenience API (provided as runtime source; treated as void here)
                if (funcName == "gbfb_init" || funcName == "gbfb_clear" || funcName == "gbfb_begin_frame" ||
                    funcName == "gbfb_plot" || funcName == "gbfb_line" || funcName == "gbfb_triangle" ||
                    funcName == "gbfb_present")
                    return CType.Void;

                if (Functions.TryGetValue(funcName, out CFunctionInfo fi)) return fi.ReturnType;
            }
            return CType.UInt8;
        }

        
        // String literal / inline readonly data ("TEXT") support:
        // Treat ReadonlyData expression as pointer (array decays to pointer).
        if (TryMatchReadonlyDataDecl(expr, out CType roType, out string roName, out Expr[] roValues))
        {
            if (roType != null)
            {
                if (roType.IsArray) return CType.MakePointer(roType.Subtype);
                return CType.MakePointer(roType);
            }
            return CType.MakePointer(CType.UInt8);
        }

        if (expr.Match(Tag.Conditional, out Expr condExprType, out Expr trueExprType, out Expr falseExprType))
        {
            CType trueType = TypeOf(trueExprType);
            CType falseType = TypeOf(falseExprType);

            if ((trueType != null && trueType.IsPointer) || (falseType != null && falseType.IsPointer))
                return (trueType != null && trueType.IsPointer) ? trueType : falseType;

            if (IsIntegerLike(trueType) || IsIntegerLike(falseType))
                return PromoteIntegerBinaryType(trueType, falseType);

            return trueType ?? falseType ?? CType.UInt8;
        }

if (expr.Match(Tag.Add, out left, out right) ||
    expr.Match(Tag.Subtract, out left, out right) ||
    expr.Match(Tag.Multiply, out left, out right) ||
    expr.Match(Tag.Divide, out left, out right) ||
    expr.Match(Tag.Modulus, out left, out right) ||
    expr.Match(Tag.BitwiseAnd, out left, out right) ||
    expr.Match(Tag.BitwiseOr, out left, out right) ||
    expr.Match(Tag.BitwiseXor, out left, out right) ||
    expr.Match(Tag.ShiftLeft, out left, out right) ||
    expr.Match(Tag.ShiftRight, out left, out right))
{
    CType lt = TypeOf(left);
    CType rt = TypeOf(right);

    // pointer +/- integer -> pointer (keep pointer type)
    bool isAdd = expr.Tag == Tag.Add;
    bool isSub = expr.Tag == Tag.Subtract;
    if (isAdd || isSub)
    {
        if (lt != null && lt.IsPointer && rt != null && (rt.IsInteger || rt.IsEnum))
            return lt;

        if (isAdd && rt != null && rt.IsPointer && lt != null && (lt.IsInteger || lt.IsEnum))
            return rt;

        // pointer - pointer -> integer (address diff)
        if (isSub && lt != null && lt.IsPointer && rt != null && rt.IsPointer)
            return CType.UInt16;
    }

    // Shift result follows promoted left operand.
    if (expr.Tag == Tag.ShiftLeft || expr.Tag == Tag.ShiftRight)
    {
        if (IsIntegerLike(lt)) return PromoteIntegerBinaryType(lt, lt);
        return (SizeOf(left) == 2) ? CType.UInt16 : CType.UInt8;
    }

    if (IsIntegerLike(lt) || IsIntegerLike(rt))
        return PromoteIntegerBinaryType(lt, rt);

    if (SizeOf(left) == 2 || SizeOf(right) == 2) return CType.UInt16;
    return CType.UInt8;
}

        return CType.UInt8;
    }

    // Fold supported expressions recursively while preserving lvalue identity, source locations and explicit cast widths.
    Expr FoldConstants(Expr expr)
    {
        if (expr == null) return null;

        if (TryEvaluateConstantExpressionTyped(expr, out ConstEvalResult directConst))
        {
            // Keep an explicit cast around folded constants.  The value alone
            // cannot represent C's argument width (for example (u16)5 would
            // otherwise become an inferred u8 literal), which made ABI reports
            // disagree with the two-byte marshalling code.
            if (expr.Match(Tag.Cast, out CType directCastType, out Expr _directCastSub))
            {
                return Expr.Make(Tag.Cast, directCastType,
                    Expr.Make(Tag.Integer, directConst.Value).WithSource(expr.Source))
                    .WithSource(expr.Source);
            }
            return Expr.Make(Tag.Integer, directConst.Value).WithSource(expr.Source);
        }

        if (expr.Match(Tag.AddressOf, out Expr addrSub))
        {
            Expr foldedSub = addrSub;
            bool preserveAddrSub = addrSub.Match(Tag.Name, out string _addrName) ||
                                   addrSub.Match(Tag.Field, out Expr _addrFieldBase, out string _addrFieldName) ||
                                   addrSub.Match(Tag.Index, out Expr _addrIndexBase, out Expr _addrIndexExpr);
            if (!preserveAddrSub)
            {
                foldedSub = FoldConstants(addrSub);
            }

            return ReferenceEquals(foldedSub, addrSub)
                ? expr
                : Expr.Make(Tag.AddressOf, foldedSub).WithSource(expr.Source);
        }

        if (expr.Match(Tag.Unsafe, out Expr unsafeSub))
        {
            Expr foldedUnsafeSub = FoldConstants(unsafeSub);
            return ReferenceEquals(foldedUnsafeSub, unsafeSub)
                ? expr
                : Expr.Make(Tag.Unsafe, foldedUnsafeSub).WithSource(expr.Source);
        }

        if (expr.Match(Tag.Assign, out Expr assignLeft, out Expr assignRight))
        {
            Expr foldedRight = FoldConstants(assignRight);
            return ReferenceEquals(foldedRight, assignRight)
                ? expr
                : Expr.Make(Tag.Assign, assignLeft, foldedRight).WithSource(expr.Source);
        }

        if (expr.Match(Tag.Cast, out CType castType, out Expr castSub))
        {
            Expr foldedCastSub = FoldConstants(castSub);
            Expr rebuiltCast = ReferenceEquals(foldedCastSub, castSub)
                ? expr
                : Expr.Make(Tag.Cast, castType, foldedCastSub).WithSource(expr.Source);
            if (TryEvaluateConstantExpressionTyped(rebuiltCast, out ConstEvalResult castConst))
                return Expr.Make(Tag.Cast, castType,
                    Expr.Make(Tag.Integer, castConst.Value).WithSource(expr.Source))
                    .WithSource(expr.Source);
            return rebuiltCast;
        }

        if (expr.Match(Tag.Conditional, out Expr cond, out Expr texpr, out Expr fexpr))
        {
            Expr foldedCond = FoldConstants(cond);
            if (foldedCond.Match(Tag.Integer, out int condValue))
                return FoldConstants((condValue != 0) ? texpr : fexpr);

            Expr foldedTrue = FoldConstants(texpr);
            Expr foldedFalse = FoldConstants(fexpr);
            Expr rebuiltConditional =
                (ReferenceEquals(foldedCond, cond) && ReferenceEquals(foldedTrue, texpr) && ReferenceEquals(foldedFalse, fexpr))
                ? expr
                : Expr.Make(Tag.Conditional, foldedCond, foldedTrue, foldedFalse).WithSource(expr.Source);
            if (TryEvaluateConstantExpressionTyped(rebuiltConditional, out ConstEvalResult condConst))
                return Expr.Make(Tag.Integer, condConst.Value).WithSource(expr.Source);
            return rebuiltConditional;
        }

        if (expr.MatchAny(Tag.Call, out Expr callTarget, out Expr[] callArgs))
        {
            Expr foldedTarget = FoldConstants(callTarget);
            Expr[] foldedArgs = callArgs;
            bool changed = !ReferenceEquals(foldedTarget, callTarget);

            if (!(callTarget.Match(Tag.Name, out string foldedCallName) && foldedCallName == "__bankof"))
            {
                foldedArgs = new Expr[callArgs.Length];
                for (int i = 0; i < callArgs.Length; i++)
                {
                    foldedArgs[i] = FoldConstants(callArgs[i]);
                    changed |= !ReferenceEquals(foldedArgs[i], callArgs[i]);
                }
            }

            Expr rebuiltCall = expr;
            if (changed)
            {
                var callParts = new List<object>(2 + foldedArgs.Length);
                callParts.Add(Tag.Call);
                callParts.Add(foldedTarget);
                for (int i = 0; i < foldedArgs.Length; i++) callParts.Add(foldedArgs[i]);
                rebuiltCall = Expr.Make(callParts.ToArray()).WithSource(expr.Source);
            }
            if (TryEvaluateConstantExpressionTyped(rebuiltCall, out ConstEvalResult callConst))
                return Expr.Make(Tag.Integer, callConst.Value).WithSource(expr.Source);
            return rebuiltCall;
        }

        if (expr.Match(Tag.Load, out Expr loadPtr))
        {
            Expr foldedPtr = FoldConstants(loadPtr);
            return ReferenceEquals(foldedPtr, loadPtr)
                ? expr
                : Expr.Make(Tag.Load, foldedPtr).WithSource(expr.Source);
        }

        if (expr.Match(Tag.Slice, out Expr slicePtr, out Expr sliceLen))
        {
            Expr foldedPtr = FoldConstants(slicePtr);
            Expr foldedLen = FoldConstants(sliceLen);
            return (ReferenceEquals(foldedPtr, slicePtr) && ReferenceEquals(foldedLen, sliceLen))
                ? expr
                : Expr.Make(Tag.Slice, foldedPtr, foldedLen).WithSource(expr.Source);
        }

        if (expr.Match(Tag.Index, out Expr indexBase, out Expr indexExpr))
        {
            Expr foldedBase = FoldConstants(indexBase);
            Expr foldedIndex = FoldConstants(indexExpr);
            return (ReferenceEquals(foldedBase, indexBase) && ReferenceEquals(foldedIndex, indexExpr))
                ? expr
                : Expr.Make(Tag.Index, foldedBase, foldedIndex).WithSource(expr.Source);
        }

        if (expr.Match(Tag.Field, out Expr fieldBase, out string fieldName))
        {
            Expr foldedBase = FoldConstants(fieldBase);
            return ReferenceEquals(foldedBase, fieldBase)
                ? expr
                : Expr.Make(Tag.Field, foldedBase, fieldName).WithSource(expr.Source);
        }

        Expr unarySubExpr;
        if (expr.Match(Tag.BitwiseNot, out unarySubExpr) || expr.Match(Tag.LogicalNot, out unarySubExpr))
        {
            Expr foldedUnarySub = FoldConstants(unarySubExpr);
            Expr rebuiltUnary = ReferenceEquals(foldedUnarySub, unarySubExpr)
                ? expr
                : Expr.Make(expr.Tag, foldedUnarySub).WithSource(expr.Source);
            if (TryEvaluateConstantExpressionTyped(rebuiltUnary, out ConstEvalResult unaryConst))
                return Expr.Make(Tag.Integer, unaryConst.Value).WithSource(expr.Source);
            return rebuiltUnary;
        }

        Expr left, right;
        if (expr.MatchAnyTag(out string tag, out left, out right))
        {
            bool foldableBinary = tag == Tag.Add || tag == Tag.Subtract || tag == Tag.Multiply ||
                                  tag == Tag.Divide || tag == Tag.Modulus ||
                                  tag == Tag.BitwiseAnd || tag == Tag.BitwiseOr || tag == Tag.BitwiseXor ||
                                  tag == Tag.ShiftLeft || tag == Tag.ShiftRight ||
                                  tag == Tag.Equal || tag == Tag.NotEqual ||
                                  tag == Tag.LessThan || tag == Tag.LessThanOrEqual ||
                                  tag == Tag.GreaterThan || tag == Tag.GreaterThanOrEqual ||
                                  tag == Tag.LogicalAnd || tag == Tag.LogicalOr;

            if (foldableBinary)
            {
                Expr foldedLeft = FoldConstants(left);
                if (tag == Tag.LogicalAnd && foldedLeft.Match(Tag.Integer, out int andConst) && andConst == 0)
                    return Expr.Make(Tag.Integer, 0).WithSource(expr.Source);
                if (tag == Tag.LogicalOr && foldedLeft.Match(Tag.Integer, out int orConst) && orConst != 0)
                    return Expr.Make(Tag.Integer, 1).WithSource(expr.Source);

                Expr foldedRight = FoldConstants(right);
                Expr rebuiltBinary =
                    (ReferenceEquals(foldedLeft, left) && ReferenceEquals(foldedRight, right))
                    ? expr
                    : Expr.Make(tag, foldedLeft, foldedRight).WithSource(expr.Source);
                if (TryEvaluateConstantExpressionTyped(rebuiltBinary, out ConstEvalResult binaryConst))
                    return Expr.Make(Tag.Integer, binaryConst.Value).WithSource(expr.Source);
                return rebuiltBinary;
            }
        }

        return expr;
    }

    // Recognize an integer after folding and convert nonzero values to true.
    bool TryGetConstantTruthValue(Expr expr, out bool truth)
    {
        Expr folded = FoldConstants(expr);
        if (folded.Match(Tag.Integer, out int constValue))
        {
            truth = (constValue != 0);
            return true;
        }

        truth = false;
        return false;
    }
    // Produce a compact source label for simple nodes, falling back to the stripped AST tag.
    string ToSourceCode(Expr expr)
    {
        if (expr.Match(Tag.Empty)) return ""; if (expr.Match(Tag.Integer, out int n)) return n.ToString(); if (expr.Match(Tag.Name, out string s)) return s; return expr.GetTag().Replace("$", "");
    }
    // Create an absolute symbolic label with a generator-local unique suffix.
    AsmOperand MakeUniqueLabel(string p) => new AsmOperand(p + "_" + NextLabelNumber++, AddressMode.Absolute);
    // Construct an AST output line from its tag and arguments.
    void Emit(params object[] a) => Output.Lines.Add(Expr.Make(a));
    // Append an already constructed line to the active output transaction.
    void Emit(Expr e) => Output.Lines.Add(e);
    // Emit the symbolic label represented by an operand.
    void EmitLabel(AsmOperand l) => Emit(Tag.Label, l.Base.Value);
    // Add a formatted explanatory line to generated output.
    void EmitComment(string format, params object[] args) => Emit(Tag.Comment, string.Format(format, args));
    // Track stack effects before appending an instruction without an explicit operand.
    void EmitAsm(string m)
    {
        TrackStackInstr(m, null);
        Emit(Expr.MakeAsm(m));
    }

    // Track stack effects before appending an instruction with its operand.
    void EmitAsm(string m, AsmOperand o)
    {
        TrackStackInstr(m, o);
        Emit(Expr.MakeAsm(m, o));
    }
    // Open a lexical scope and save allocator cursors for temporary local storage.
    void BeginScope()
    {
        CurrentScope = new LexicalScope(CurrentScope);

        CurrentScope.SavedHramNext = HramRegion.Next;
        CurrentScope.SavedWram0Next = Wram0Region.Next;
        CurrentScope.SavedWram1Next = Wram1Region.Next;

    }
    // Restore the saved local-storage cursors and return to the outer lexical scope.
    void EndScope()
    {
        HramRegion.Next = CurrentScope.SavedHramNext;
        Wram0Region.Next = CurrentScope.SavedWram0Next;
        Wram1Region.Next = CurrentScope.SavedWram1Next;
        CurrentScope = CurrentScope.Outer;
    }
    // Forward source-located errors and warnings through the compiler diagnostic entry points.
    void Error(Expr e, string m) => Program.Error(e.Source, m);
    void Error(Expr e, ErrorCode code, string m) => Program.Error(e.Source, code, m);
    void Error(Expr e, ErrorCode code, string format, params object[] args) => Program.Error(e.Source, code, format, args);
    void Warning(Expr e, string m) => Program.Warning(e.Source, m);
    void Warning(Expr e, ErrorCode code, string m) => Program.Warning(e.Source, code, m);
    void NYI(Expr e, string m) => Error(e, "Not Implemented: " + m);
    // Begin a separate output transaction whose lines are committed only if speculation succeeds.
    void Speculate() => OutputStack.Push(new OutputTransaction());
    // Discard a failed speculative transaction or append its emitted lines to the enclosing output.
    bool Commit()
    {
        OutputTransaction transaction = OutputStack.Pop();
        if (transaction.SpeculationError) return false;
        Output.Lines.AddRange(transaction.Lines);
        return true;
    }
    // Compatibility hook: this generator does not maintain a register-reservation state here.
    void Reserve(Register r) { }
    // Compatibility hook paired with Reserve; no register state is changed.
    void Release(Register r) { }

    // Resolve aggregate metadata from an expression, allowing a pointer to an aggregate.
    AggregateInfo GetAggregateInfo(Expr structExpr)
    {
        CType type = TypeOf(structExpr);
        if (!type.IsStructOrUnion && type.IsPointer) type = type.Subtype;
        if (!type.IsStructOrUnion) Program.Error("struct or union type required");
        return AggregateTypes[type.Name];
    }
    // Look up a named aggregate and diagnose missing or incomplete definitions.
    AggregateInfo GetAggregateInfo(Expr origin, string name)
    {
        AggregateInfo info;
        if (!AggregateTypes.TryGetValue(name, out info)) Error(origin, ErrorCode.AggregateNotDefined, "struct or union not defined: " + name);
        if (info.TotalSize < 0) Error(origin, ErrorCode.IncompleteType, "incomplete struct/union type: " + name);
        return info;
    }
    // Locate a field in the resolved aggregate definition or diagnose an invalid member name.
    FieldInfo GetFieldInfo(Expr structExpr, string fieldName)
    {
        AggregateInfo info = GetAggregateInfo(structExpr);
        foreach (FieldInfo field in info.Fields) if (field.Name == fieldName) return field;
        Program.Error("Invalid field name: " + fieldName);
        return null;
    }


    // Count set bits in a positive factor for the constant-multiplication cost heuristic.
    int PopCount(int n)
    {
        int count = 0;
        while (n > 0)
        {
            if ((n & 1) == 1) count++;
            n >>= 1;
        }
        return count;
    }

    // Use a shift/add expansion for positive factors with at most four set bits, retaining the original value in DE.
    bool EmitConstantMultiplication(int factor)
    {
        if (factor <= 0) return false;

        if (PopCount(factor) > 4) return false;

        EmitAsm("LD_D_H");
        EmitAsm("LD_E_L");

        int msb = 0;
        for (int i = 0; i < 16; i++) { if ((factor & (1 << i)) != 0) msb = i; }

        int currentMult = 1;

        for (int i = msb - 1; i >= 0; i--)
        {
            EmitAsm("ADD_HL_HL");
            currentMult *= 2;

            if ((factor & (1 << i)) != 0)
            {
                EmitAsm("ADD_HL_DE");
                currentMult += 1;
            }
        }

        return true;
    }


    // --- Loop codegen micro-optimizations ---
    // Recognize a common countdown loop form and emit a tighter pattern that enables
    // `DEC ...; JP_NZ` which then becomes `DEC ...; JR NZ` after jump relaxation.
    // Form:
    // for ( ...; v != 0; v-- ) { body }
    // Constraints:
    // - v is u8
    // - induct is exactly v-- or --v
    // - test is v, v!=0, v>0, v>=1
    // - body does not modify v (conservative check)
    bool TryCompileCountdownForLoop(Expr init, Expr test, Expr induct, Expr body)
    {
        Expr decExpr;
        if (induct.Match(Tag.PreDecrement, out decExpr) == false && induct.Match(Tag.PostDecrement, out decExpr) == false)
            return false;

        if (!decExpr.Match(Tag.Name, out string varName))
            return false;

        if (!IsSimpleNonZeroTest(test, varName))
            return false;

        // Avoid transforming if body mutates the induction variable.
        if (ContainsWriteToVar(body, varName))
            return false;

        // Set up loop scope so `continue;` runs the decrement step (for-loop semantics).
        AsmOperand top = MakeUniqueLabel("loop_top");
        AsmOperand cont = MakeUniqueLabel("loop_cont");
        AsmOperand end = MakeUniqueLabel("loop_end");

        Loop = new LoopScope { Outer = Loop, ContinueLabel = cont, BreakLabel = end };
        BeginScope();
        CompileStatement(init);

		// Resolve the loop variable operand after init has run (decls happen here).
		// NOTE: Expr has no MakeName() helper in this codebase; build a Name node directly.
		Expr nameExpr = Expr.Make(Tag.Name, varName);
        if (SizeOf(nameExpr) != 1 || !TryGetOperand(nameExpr, out AsmOperand varOp))
        {
            EndScope();
            Loop = Loop.Outer;
            return false;
        }

        string ldIn = (varOp.Mode == AddressMode.HighMem) ? "LDH_A_MEM" : "LD_A_MEM";
        string ldOut = (varOp.Mode == AddressMode.HighMem) ? "LDH_MEM_A" : "LD_MEM_A";

        // Initial test (skip body when v == 0)
        EmitAsm(ldIn, varOp);
        EmitAsm("OR_A");
        EmitAsm("JP_Z", end);

        EmitLabel(top);
        BreakLabels.Push(end);
        CompileStatement(body);
        BreakLabels.Pop();

        EmitLabel(cont);
        // v-- ; if (v != 0) goto top;
        EmitAsm(ldIn, varOp);
        EmitAsm("DEC_A");
        EmitAsm(ldOut, varOp); // preserve Z flag from DEC
        EmitAsm("JP_NZ", top);

        EmitLabel(end);
        EndScope();
        Loop = Loop.Outer;
        return true;
    }

    // Recognize the limited nonzero-test forms accepted by countdown-loop specialization.
    bool IsSimpleNonZeroTest(Expr test, string varName)
    {
        if (test.Match(Tag.Empty)) return false;

        // `v` as truthy test
        if (test.Match(Tag.Name, out string n) && n == varName) return true;

        // v != 0
        if (test.Match(Tag.NotEqual, out Expr a, out Expr b))
        {
            if (IsName(a, varName) && IsInt(b, 0)) return true;
            if (IsName(b, varName) && IsInt(a, 0)) return true;
        }

        // v > 0
        if (test.Match(Tag.GreaterThan, out a, out b))
        {
            if (IsName(a, varName) && IsInt(b, 0)) return true;
        }

        // v >= 1
        if (test.Match(Tag.GreaterThanOrEqual, out a, out b))
        {
            if (IsName(a, varName) && IsInt(b, 1)) return true;
        }

        return false;
    }

    // Match one exact variable-name expression.
    bool IsName(Expr expr, string varName)
    {
        return expr.Match(Tag.Name, out string n) && n == varName;
    }

    // Match one exact integer-literal expression.
    bool IsInt(Expr expr, int value)
    {
        return expr.Match(Tag.Integer, out int v) && v == value;
    }

    // Scan syntax for direct writes and address-taking hazards before countdown-loop specialization.
    bool ContainsWriteToVar(Expr expr, string varName)
    {
        if (expr == null) return false;

        // direct assignment: v = ...
        if (expr.Match(Tag.Assign, out Expr left, out Expr right))
        {
            if (IsName(left, varName)) return true;
            if (ContainsWriteToVar(right, varName)) return true;
            return ContainsWriteToVar(left, varName);
        }

        // ++v / v++ / --v / v-- inside body
        if (expr.Match(Tag.PreIncrement, out Expr sub) || expr.Match(Tag.PostIncrement, out sub) ||
            expr.Match(Tag.PreDecrement, out sub) || expr.Match(Tag.PostDecrement, out sub))
        {
            if (IsName(sub, varName)) return true;
        }

        // For safety, treat taking address of v as a write hazard (could be modified through pointer)
        if (expr.Match(Tag.AddressOf, out sub))
        {
            if (IsName(sub, varName)) return true;
        }

        foreach (var arg in expr.GetArgs())
        {
            if (arg is Expr child)
            {
                if (ContainsWriteToVar(child, varName)) return true;
            }
            else if (arg is Expr[] children)
            {
                foreach (var c in children)
                    if (ContainsWriteToVar(c, varName)) return true;
            }
        }

        return false;
    }

    // Analyze stack ABI parameters: decide which parameters must be copied into local HRAM/WRAM slots.
    // Rule: copy only parameters that are *assigned to by name* (including +=, ++/--).
    // const parameters are never copied.
    bool[] AnalyzeStackAbiParamNeedsCopy(Expr body, FieldInfo[] parameters)
    {
        var result = new bool[parameters.Length];
        var nameToIndex = new Dictionary<string, int>(StringComparer.Ordinal);
        for (int i = 0; i < parameters.Length; i++)
            nameToIndex[parameters[i].Name] = i;

        // Mark a named nonconst parameter for a local copy, peeling explicit casts from the assignment target.
        void MarkWriteLhs(Expr lhs)
        {
            if (lhs == null) return;
            if (lhs.Match(Tag.Name, out string n) && nameToIndex.TryGetValue(n, out int idx))
            {
                // Do not copy const parameters (assignment should be rejected earlier).
                if (!parameters[idx].Type.IsConst) result[idx] = true;
                return;
            }
            // Peel casts
            if (lhs.Match(Tag.Cast, out CType _t, out Expr inner))
            {
                MarkWriteLhs(inner);
                return;
            }
        }

        // Find parameter writes through assignments and increments, then recurse into remaining child expressions.
        void Scan(Expr e)
        {
            if (e == null) return;

            if (e.Match(Tag.Assign, out Expr lhs, out Expr rhs))
            {
                MarkWriteLhs(lhs);
                Scan(lhs);
                Scan(rhs);
                return;
            }
            if (e.Match(Tag.AssignModify, out string _op, out Expr lhs2, out Expr rhs2))
            {
                MarkWriteLhs(lhs2);
                Scan(lhs2);
                Scan(rhs2);
                return;
            }
            if (e.Match(Tag.PreIncrement, out Expr sub) || e.Match(Tag.PostIncrement, out sub)
                || e.Match(Tag.PreDecrement, out sub) || e.Match(Tag.PostDecrement, out sub))
            {
                MarkWriteLhs(sub);
                Scan(sub);
                return;
            }

            // Generic recursion over child expressions
            object[] args = e.GetArgs();
            for (int i = 1; i < args.Length; i++)
            {
                if (args[i] is Expr child) Scan(child);
                else if (args[i] is Expr[] arr) { foreach (var c in arr) Scan(c); }
            }
        }

        Scan(body);
        return result;
    }


    // Accumulate weighted name occurrences, multiplying loop test, induction and body weights by four.
    void AnalyzeVariableUsage(Expr expr, int weight)
    {
        if (expr == null) return;

        if (expr.Match(Tag.Name, out string n))
        {
            if (!VariableUsageCounts.ContainsKey(n)) VariableUsageCounts[n] = 0;
            VariableUsageCounts[n] += weight;
            return;
        }

        if (expr.Match(Tag.For, out Expr init, out Expr test, out Expr induct, out Expr body))
        {
            AnalyzeVariableUsage(init, weight);
            AnalyzeVariableUsage(test, weight * 4);
            AnalyzeVariableUsage(induct, weight * 4);
            AnalyzeVariableUsage(body, weight * 4);
            return;
        }

        foreach (var arg in expr.GetArgs())
        {
            if (arg is Expr child) AnalyzeVariableUsage(child, weight);
            else if (arg is Expr[] children)
                foreach (var c in children) AnalyzeVariableUsage(c, weight);
        }
    }




}
