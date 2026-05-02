using System;
using System.Collections.Generic;

// Conservative AST lowering pass.
// This pass introduces explicit temporaries (u8/u16) to make certain contexts
// "atom-only" (Name/Integer) so the existing code generator can stay simple.
// Targeted pain points for Wire3D/GBFB:
// - Call arguments (spill non-atom args)
// - Pointer dereference base ( *ptr where ptr is complex )
// - Array index expression (arr[complex])
// - Variable shift counts (x << n) => loop lowering
// Notes:
// - This does NOT attempt to make full struct-by-value member access usable.
// (The current codegen supports Field(Index(...), name) and Field(Load(...), name)
// but not Field(Name(structVar), name).)
class Lowerer
{
    // ---------- Public entry ----------

    public static Expr Lower(Expr program)
    {
        GlobalEnv genv = new GlobalEnv();
        genv.Build(program);

        Expr[] decls;
        if (!program.MatchAny(Tag.Sequence, out decls))
            return program;

        List<Expr> loweredDecls = new List<Expr>(decls.Length);

        for (int i = 0; i < decls.Length; i++)
        {
            loweredDecls.Add(LowerTopDeclaration(genv, decls[i]));
        }

        WarnUnusedGlobalSymbols(loweredDecls);
        return MakeSequence(loweredDecls).WithSource(program.Source);
    }

    static Expr LowerTopDeclaration(GlobalEnv genv, Expr decl)
    {
        Expr inner;
        int bankNo;

        if (decl.Match(Tag.Unsafe, out inner))
        {
            Expr loweredInner = LowerTopDeclaration(genv, inner);
            return Expr.Make(Tag.Unsafe, loweredInner).WithSource(decl.Source);
        }
        if (decl.Match(Tag.Static, out inner))
        {
            Expr loweredInner = LowerTopDeclaration(genv, inner);
            return Expr.Make(Tag.Static, loweredInner).WithSource(decl.Source);
        }
        if (decl.Match(Tag.Bank, out bankNo, out inner))
        {
            Expr loweredInner = LowerTopDeclaration(genv, inner);
            return Expr.Make(Tag.Bank, bankNo, loweredInner).WithSource(decl.Source);
        }
        if (decl.Match(Tag.FixedBank, out bankNo, out inner))
        {
            Expr loweredInner = LowerTopDeclaration(genv, inner);
            return Expr.Make(Tag.FixedBank, bankNo, loweredInner).WithSource(decl.Source);
        }
        if (decl.Match(Tag.FixedOrder, out bankNo, out inner))
        {
            Expr loweredInner = LowerTopDeclaration(genv, inner);
            return Expr.Make(Tag.FixedOrder, bankNo, loweredInner).WithSource(decl.Source);
        }

        CType ret;
        string fname;
        FieldInfo[] ps;
        Expr body;

        int mustCheck = 0;
        if (decl.Match(Tag.Function, out ret, out fname, out ps, out mustCheck, out body) ||
            (mustCheck = 0) == 0 && decl.Match(Tag.Function, out ret, out fname, out ps, out body))
        {
            Expr newBody = LowerFunctionBody(genv, fname, ret, ps, body);
            return Expr.Make(Tag.Function, ret, fname, ps, mustCheck, newBody).WithSource(decl.Source);
        }

        int mustCheckDecl = 0;
        if (decl.Match(Tag.FunctionDecl, out ret, out fname, out ps, out mustCheckDecl) ||
            (mustCheckDecl = 0) == 0 && decl.Match(Tag.FunctionDecl, out ret, out fname, out ps))
        {
            return decl;
        }

        int mustCheckInl = 0;
        if (decl.Match(Tag.InlineFunction, out ret, out fname, out ps, out mustCheckInl, out body) ||
            (mustCheckInl = 0) == 0 && decl.Match(Tag.InlineFunction, out ret, out fname, out ps, out body))
        {
            Expr newBody = LowerFunctionBody(genv, fname, ret, ps, body);
            return Expr.Make(Tag.InlineFunction, ret, fname, ps, mustCheckInl, newBody).WithSource(decl.Source);
        }

        return decl;
    }

    // ---------- Environment ----------

    sealed class GlobalEnv
    {
        // variables (globals): name -> type
        public readonly Dictionary<string, CType> Globals = new Dictionary<string, CType>();
        public readonly Dictionary<string, int[]> VarRanges = new Dictionary<string, int[]>();

        // constants: name -> (type, value expr)
        public readonly Dictionary<string, CType> ConstTypes = new Dictionary<string, CType>();
        public readonly Dictionary<string, Expr> ConstExprs = new Dictionary<string, Expr>();
        readonly Dictionary<string, int> ConstValues = new Dictionary<string, int>();

        // functions: name -> return type
        public readonly Dictionary<string, CType> FuncReturns = new Dictionary<string, CType>();

        // functions: name -> parameter types (used by lightweight lints)
        public readonly Dictionary<string, CType[]> FuncParamTypes = new Dictionary<string, CType[]>();

        // struct/union field types: structName -> fieldName -> fieldType
        public readonly Dictionary<string, Dictionary<string, CType>> StructFieldTypes = new Dictionary<string, Dictionary<string, CType>>();

        public void Build(Expr program)
        {
            Expr[] decls;
            if (!program.MatchAny(Tag.Sequence, out decls))
                return;

            for (int i = 0; i < decls.Length; i++)
            {
                Expr d = UnwrapDeclWrappers(decls[i]);

                // struct / union
                string sname;
                FieldInfo[] fields;
                int _packed = 0, _align = 0;
                if (d.Match(Tag.Struct, out sname, out fields, out _packed, out _align) || d.Match(Tag.Struct, out sname, out fields) ||
                    d.Match(Tag.Union, out sname, out fields, out _packed, out _align) || d.Match(Tag.Union, out sname, out fields))
                {
                    Dictionary<string, CType> fmap = new Dictionary<string, CType>();
                    for (int j = 0; j < fields.Length; j++)
                    {
                        FieldInfo f = fields[j];
                        if (!fmap.ContainsKey(f.Name)) fmap.Add(f.Name, f.Type);
                    }
                    if (!StructFieldTypes.ContainsKey(sname)) StructFieldTypes.Add(sname, fmap);
                    continue;
                }

                // global variables: (Tag.Variable, region, type, name[, rangeExpr])
                MemoryRegion region;
                CType vt;
                string vname;
                Expr vrangeExpr;
                if (d.Match(Tag.Variable, out region, out vt, out vname, out vrangeExpr) || d.Match(Tag.Variable, out region, out vt, out vname))
                {
                    if (!Globals.ContainsKey(vname)) Globals.Add(vname, vt);
                    if (vrangeExpr != null && !VarRanges.ContainsKey(vname)) VarRanges.Add(vname, EvaluateRange(vrangeExpr));
                    continue;
                }

                // extern global variable declarations: (Tag.ExternVariable, region, type, name[, rangeExpr])
                if (d.Match(Tag.ExternVariable, out region, out vt, out vname, out vrangeExpr) || d.Match(Tag.ExternVariable, out region, out vt, out vname))
                {
                    // Record type for type inference / lints. Do not allocate here.
                    if (!Globals.ContainsKey(vname)) Globals.Add(vname, vt);
                    else
                    {
                        // If a prior declaration/definition exists with an incompatible type, surface it.
                        CType prev = Globals[vname];
                        if (!ExternTypeCompatible(prev, vt))
                        {
                            Program.Error(d.Source, ErrorCode.ExternTypeMismatch,
                                "extern declaration type mismatch for '{0}' (got {1}, previously {2})",
                                vname, vt.Show(), prev.Show());
                        }
                    }
                    if (vrangeExpr != null && !VarRanges.ContainsKey(vname)) VarRanges.Add(vname, EvaluateRange(vrangeExpr));
                    continue;
                }

                // constants: (Tag.Constant, type, name, valueExpr)
                CType ct;
                string cname;
                Expr cval;
                if (d.Match(Tag.Constant, out ct, out cname, out cval))
                {
                    if (!ConstTypes.ContainsKey(cname)) ConstTypes.Add(cname, ct);
                    if (!ConstExprs.ContainsKey(cname)) ConstExprs.Add(cname, cval);
                    continue;
                }

                // functions / prototypes
                CType ret;
                string fname;
                FieldInfo[] ps;
                Expr body;
                int _mc = 0;
                int _mcDecl = 0;

                if (d.Match(Tag.FunctionDecl, out ret, out fname, out ps, out _mcDecl) ||
                    d.Match(Tag.FunctionDecl, out ret, out fname, out ps))
                {
                    StoreFuncSig(fname, ret, ps);
                    continue;
                }

                if (d.Match(Tag.Function, out ret, out fname, out ps, out _mc, out body) ||
                    d.Match(Tag.Function, out ret, out fname, out ps, out body) ||
                    d.Match(Tag.InlineFunction, out ret, out fname, out ps, out _mc, out body) ||
                    d.Match(Tag.InlineFunction, out ret, out fname, out ps, out body))
                {
                    StoreFuncSig(fname, ret, ps);
                    continue;
                }
            }

            // Known intrinsics (if not present as normal functions)
            AddIntrinsicReturn("__mul8x8_hi", CType.UInt8);
            AddIntrinsicReturn("__mul16x8", CType.UInt16);
            AddIntrinsicReturn("__mac16", CType.UInt16);
            AddIntrinsicReturn("__sdot3_q1_7", CType.UInt16);
        }

        // extern type compatibility (minimal C-like rules)
        // - exact match is OK
        // - array with unspecified dimension (e.g. extern u8 a[]) is compatible with any concrete size definition
        static bool ExternTypeCompatible(CType a, CType b)
        {
            if (a == null || b == null) return a == b;
            if (a == b) return true;

            // array compatibility: allow one side to omit dimension
            if (a.IsArray && b.IsArray)
            {
                CType ae = a.Subtype;
                CType be = b.Subtype;
                if (ae != be) return false;

                // a: array[expr]
                if (a.Tag == CTypeTag.ArrayWithDimensionExpression && a.DimensionExpression != null && a.DimensionExpression.Match(Tag.Empty)) return true;
                if (b.Tag == CTypeTag.ArrayWithDimensionExpression && b.DimensionExpression != null && b.DimensionExpression.Match(Tag.Empty)) return true;

                // both concrete arrays
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

                // both dimension-expr arrays: we only accept if expressions are structurally equal
                // (Keep it conservative; the definition will ultimately decide the size.)
                if (a.Tag == CTypeTag.ArrayWithDimensionExpression && b.Tag == CTypeTag.ArrayWithDimensionExpression)
                {
                    return a.DimensionExpression.Show() == b.DimensionExpression.Show();
                }
            }

            return false;
        }

        static bool TryEvalConstArrayDim(Expr dimExpr, out int dim)
        {
            dim = 0;
            if (dimExpr == null) return false;
            if (dimExpr.Match(Tag.Integer, out dim) && dim >= 0) return true;
            return false;
        }


        void StoreFuncSig(string name, CType ret, FieldInfo[] ps)
        {
            if (!FuncReturns.ContainsKey(name)) FuncReturns.Add(name, ret);
            if (!FuncParamTypes.ContainsKey(name))
            {
                CType[] pts = new CType[ps != null ? ps.Length : 0];
                for (int i = 0; i < pts.Length; i++) pts[i] = ps[i].Type;
                FuncParamTypes.Add(name, pts);
            }
        }

        void AddIntrinsicReturn(string name, CType ret)
        {
            if (!FuncReturns.ContainsKey(name)) FuncReturns.Add(name, ret);
            if (!FuncParamTypes.ContainsKey(name)) FuncParamTypes.Add(name, new CType[0]);
        }


        // --- __range(min,max) constant evaluation ---
        public int[] EvaluateRange(Expr rangeExpr)
        {
            if (rangeExpr == null) return null;
            Expr a, b;
            if (!rangeExpr.Match(Tag.Range, out a, out b))
            {
                Program.Error(rangeExpr.Source, ErrorCode.ParseError, "__range: malformed attribute");
                return new int[] { 0, 0 };
            }
            int min = EvalConstInt(a);
            int max = EvalConstInt(b);
            if (min > max)
            {
                Program.Error(rangeExpr.Source, ErrorCode.ParseError, "__range: min must be <= max (got {0}..{1})", min, max);
            }
            return new int[] { min, max };
        }

        int EvalConstInt(Expr expr)
        {
            if (expr == null) return 0;
            if (expr.Match(Tag.Integer, out int n)) return n;
            if (expr.Match(Tag.Name, out string nm)) return EvalConstName(nm, expr);

            // sizeof(type) only (sizeof(expr) is not supported in constant-range attributes for now)
            if (expr.MatchAnyTag(out string ttag, out object arg1) && ttag == Tag.Sizeof)
            {
                if (arg1 is CType ct) return SizeOfType(ct, expr);
                Program.Error(expr.Source, ErrorCode.ParseError, "sizeof(expr) is not supported in __range constant expressions");
                return 0;
            }

            Expr x, y, sub;
            if (expr.Match(Tag.Add, out x, out y)) return EvalConstInt(x) + EvalConstInt(y);
            if (expr.Match(Tag.Subtract, out x, out y)) return EvalConstInt(x) - EvalConstInt(y);
            if (expr.Match(Tag.Multiply, out x, out y)) return EvalConstInt(x) * EvalConstInt(y);
            if (expr.Match(Tag.Divide, out x, out y)) return EvalConstInt(x) / EvalConstInt(y);
            if (expr.Match(Tag.Modulus, out x, out y)) return EvalConstInt(x) % EvalConstInt(y);
            if (expr.Match(Tag.BitwiseAnd, out x, out y)) return EvalConstInt(x) & EvalConstInt(y);
            if (expr.Match(Tag.BitwiseOr, out x, out y)) return EvalConstInt(x) | EvalConstInt(y);
            if (expr.Match(Tag.BitwiseXor, out x, out y)) return EvalConstInt(x) ^ EvalConstInt(y);
            if (expr.Match(Tag.ShiftLeft, out x, out y)) return EvalConstInt(x) << EvalConstInt(y);
            if (expr.Match(Tag.ShiftRight, out x, out y)) return EvalConstInt(x) >> EvalConstInt(y);
            if (expr.Match(Tag.BitwiseNot, out sub)) return ~EvalConstInt(sub);

            Program.Error(expr.Source, ErrorCode.ParseError, "expected constant expression in __range, got: {0}", expr.Show());
            return 0;
        }

        // Public wrapper for constant evaluation needed by other lints (e.g., array length checks).
        // This shares the same constant-expression rules as __range / sizeof handling.
        public int EvalConstIntPublic(Expr expr)
        {
            return EvalConstInt(expr);
        }

        // Lint-friendly constant evaluation: returns false instead of emitting diagnostics.
        public bool TryEvalConstInt(Expr expr, out int value)
        {
            value = 0;
            if (expr == null) return true;
            if (expr.Match(Tag.Integer, out int n)) { value = n; return true; }
            if (expr.Match(Tag.Name, out string nm))
            {
                if (ConstValues.TryGetValue(nm, out int cv)) { value = cv; return true; }
                if (ConstExprs.TryGetValue(nm, out var ce))
                {
                    // Guard recursion.
                    if (ConstValues.ContainsKey(nm)) return false;
                    ConstValues[nm] = 0;
                    if (!TryEvalConstInt(ce, out var tmp)) { ConstValues.Remove(nm); return false; }
                    ConstValues[nm] = tmp;
                    value = tmp;
                    return true;
                }
                return false;
            }

            if (expr.Match(Tag.Cast, out CType castType, out Expr castSub))
            {
                if (!TryEvalConstInt(castSub, out int cv)) return false;
                if (castType == null) { value = cv; return true; }
                if (castType.IsPointer || castType.IsEnum) { value = cv & 0xFFFF; return true; }
                if (!castType.IsSimple) { value = cv; return true; }

                if (castType.SimpleType == CSimpleType.UInt8) { value = cv & 0xFF; return true; }
                if (castType.SimpleType == CSimpleType.Int8)
                {
                    int b = cv & 0xFF;
                    value = (b >= 0x80) ? (b - 0x100) : b;
                    return true;
                }
                if (castType.SimpleType == CSimpleType.UInt16) { value = cv & 0xFFFF; return true; }
                if (castType.SimpleType == CSimpleType.Int16)
                {
                    int w = cv & 0xFFFF;
                    value = (w >= 0x8000) ? (w - 0x10000) : w;
                    return true;
                }

                value = cv;
                return true;
            }

            // sizeof(type) only
            if (expr.MatchAnyTag(out string ttag, out object arg1) && ttag == Tag.Sizeof)
            {
                if (arg1 is CType ct)
                {
                    // Use the existing sizeof rules; this should not error for complete types.
                    value = SizeOfType(ct, expr);
                    return true;
                }
                return false;
            }

            Expr x, y, sub;
            if (expr.Match(Tag.Add, out x, out y)) { if (!TryEvalConstInt(x, out var a) || !TryEvalConstInt(y, out var b)) return false; value = a + b; return true; }
            if (expr.Match(Tag.Subtract, out x, out y)) { if (!TryEvalConstInt(x, out var a) || !TryEvalConstInt(y, out var b)) return false; value = a - b; return true; }
            if (expr.Match(Tag.Multiply, out x, out y)) { if (!TryEvalConstInt(x, out var a) || !TryEvalConstInt(y, out var b)) return false; value = a * b; return true; }
            if (expr.Match(Tag.Divide, out x, out y)) { if (!TryEvalConstInt(x, out var a) || !TryEvalConstInt(y, out var b)) return false; if (b == 0) return false; value = a / b; return true; }
            if (expr.Match(Tag.Modulus, out x, out y)) { if (!TryEvalConstInt(x, out var a) || !TryEvalConstInt(y, out var b)) return false; if (b == 0) return false; value = a % b; return true; }
            if (expr.Match(Tag.BitwiseAnd, out x, out y)) { if (!TryEvalConstInt(x, out var a) || !TryEvalConstInt(y, out var b)) return false; value = a & b; return true; }
            if (expr.Match(Tag.BitwiseOr, out x, out y)) { if (!TryEvalConstInt(x, out var a) || !TryEvalConstInt(y, out var b)) return false; value = a | b; return true; }
            if (expr.Match(Tag.BitwiseXor, out x, out y)) { if (!TryEvalConstInt(x, out var a) || !TryEvalConstInt(y, out var b)) return false; value = a ^ b; return true; }
            if (expr.Match(Tag.ShiftLeft, out x, out y)) { if (!TryEvalConstInt(x, out var a) || !TryEvalConstInt(y, out var b)) return false; value = a << b; return true; }
            if (expr.Match(Tag.ShiftRight, out x, out y)) { if (!TryEvalConstInt(x, out var a) || !TryEvalConstInt(y, out var b)) return false; value = a >> b; return true; }
            if (expr.Match(Tag.BitwiseNot, out sub)) { if (!TryEvalConstInt(sub, out var a)) return false; value = ~a; return true; }

            return false;
        }

        int EvalConstName(string nm, Expr origin)
        {
            if (ConstValues.TryGetValue(nm, out int v)) return v;
            if (!ConstExprs.TryGetValue(nm, out Expr e))
            {
                Program.Error(origin.Source, ErrorCode.ParseError, "expected constant expression, got name: {0}", nm);
                return 0;
            }
            // guard recursion
            ConstValues[nm] = 0;
            v = EvalConstInt(e);
            ConstValues[nm] = v;
            return v;
        }

        int SizeOfType(CType type, Expr origin)
        {
            if (type == null) return 1;
            type = type.WithoutConst();

            if (type.Tag == CTypeTag.Simple)
            {
                if (type.SimpleType == CSimpleType.UInt8 || type.SimpleType == CSimpleType.Int8) return 1;
                if (type.SimpleType == CSimpleType.UInt16 || type.SimpleType == CSimpleType.Int16) return 2;
                if (type.SimpleType == CSimpleType.Void) return 0;
            }
            if (type.Tag == CTypeTag.Pointer) return 2;

            if (type.Tag == CTypeTag.Array)
            {
                return SizeOfType(type.Subtype, origin) * type.Dimension;
            }
            if (type.Tag == CTypeTag.ArrayWithDimensionExpression)
            {
                int dim = EvalConstInt(type.DimensionExpression);
                return SizeOfType(type.Subtype, origin) * dim;
            }

            if (type.Tag == CTypeTag.Struct || type.Tag == CTypeTag.Union)
            {
                if (!StructFieldTypes.TryGetValue(type.Name, out var fmap) || fmap.Count == 0)
                {
                    Program.Error(origin.Source, ErrorCode.IncompleteType, "incomplete type in sizeof: {0}", type.Show());
                    return 0;
                }
                int total = 0;
                int max = 0;
                foreach (var kv in fmap)
                {
                    int sz = SizeOfType(kv.Value, origin);
                    total += sz;
                    if (sz > max) max = sz;
                }
                return (type.Tag == CTypeTag.Union) ? max : total;
            }

            if (type.Tag == CTypeTag.Function)
            {
                // function types decay to pointer size in most ABIs; treat as pointer size.
                return 2;
            }

            return 2;
        }

        public bool TryGetFieldType(CType baseType, string fieldName, out CType fieldType)
        {
            fieldType = CType.UInt8;
            if (baseType == null) return false;

            CType t = baseType;
            if (t.IsArray || t.IsPointer) t = t.Subtype;

	            if (!(t.Tag == CTypeTag.Struct || t.Tag == CTypeTag.Union)) return false;

            Dictionary<string, CType> fmap;
            if (!StructFieldTypes.TryGetValue(t.Name, out fmap)) return false;

            return fmap.TryGetValue(fieldName, out fieldType);
        }
    }

    sealed class StructLocalInfo
    {
        public readonly CType StructType;
        public readonly Dictionary<string, string> FieldToVar = new Dictionary<string, string>();

        public StructLocalInfo(CType structType)
        {
            StructType = structType;
        }
    }


    sealed class FunctionCtx
    {
        public readonly GlobalEnv Global;
        public readonly Dictionary<string, CType> Locals = new Dictionary<string, CType>();
        public readonly Dictionary<string, CType> Params = new Dictionary<string, CType>();
        public readonly Dictionary<string, int> LocalUseCounts = new Dictionary<string, int>();
        public readonly Dictionary<string, int> ParamUseCounts = new Dictionary<string, int>();
        public readonly Dictionary<string, FilePosition> LocalDeclPos = new Dictionary<string, FilePosition>();
        public readonly Dictionary<string, FilePosition> ParamDeclPos = new Dictionary<string, FilePosition>();
        public readonly Dictionary<string, int[]> LocalRanges = new Dictionary<string, int[]>();
        public readonly Dictionary<string, int[]> ParamRanges = new Dictionary<string, int[]>();
        public string FunctionName = "";
        public CType FunctionReturnType = CType.Void;
        // Flow-sensitive range overrides (for if/else refinements)
        readonly Stack<Dictionary<string, int[]>> RangeOverrideStack = new Stack<Dictionary<string, int[]>>();

        // Tracks whether we are inside a __unsafe { ... } boundary.
        // Used to suppress certain lints (e.g. restrict alias lint) inside unsafe regions.
        public int UnsafeDepth = 0;
        public bool InUnsafe { get { return UnsafeDepth > 0; } }
        public void PushUnsafe() { UnsafeDepth++; }
        public void PopUnsafe() { if (UnsafeDepth > 0) UnsafeDepth--; }


        public void PushRangeOverride(Dictionary<string, int[]> ov)
        {
            if (ov == null || ov.Count == 0) return;
            RangeOverrideStack.Push(ov);
        }
        public void PopRangeOverride(Dictionary<string, int[]> ov)
        {
            if (ov == null || ov.Count == 0) return;
            if (RangeOverrideStack.Count > 0) RangeOverrideStack.Pop();
        }
        public bool TryGetRangeOverride(string name, out int[] r)
        {
            if (RangeOverrideStack.Count == 0) { r = null; return false; }
            foreach (var d in RangeOverrideStack)
            {
                if (d != null && d.TryGetValue(name, out r)) return true;
            }
            r = null;
            return false;
        }
        public readonly TempPool Temps = new TempPool();
        public readonly Dictionary<string, StructLocalInfo> StructLocals = new Dictionary<string, StructLocalInfo>();

        readonly Stack<List<string>> tempsInStatementStack = new Stack<List<string>>();

        static CType NormalizeTempType(CType t)
        {
            if (t == null) return CType.UInt16;
            t = t.WithoutConst();
            if (t.IsArray) return CType.MakePointer(t.Subtype ?? CType.UInt8);
            if (t.IsStructOrUnion) return CType.UInt16;
            return t;
        }

        public FunctionCtx(GlobalEnv global)
        {
            Global = global;
        }

        public void BeginStatement()
        {
            tempsInStatementStack.Push(new List<string>());
        }

        public void EndStatement()
        {
            if (tempsInStatementStack.Count == 0) return;
            List<string> tempsInStatement = tempsInStatementStack.Pop();
            for (int i = tempsInStatement.Count - 1; i >= 0; i--)
                Temps.Release(tempsInStatement[i]);
        }

        public string AcquireTemp(CType t)
        {
            CType tt = NormalizeTempType(t);
            string name = Temps.Acquire(tt);
            RegisterLocal(tt, name);
            if (tempsInStatementStack.Count != 0)
                tempsInStatementStack.Peek().Add(name);
            return name;
        }

        public bool TryGetStructLocalFieldVar(string baseName, string fieldName, out string fieldVarName)
        {
            fieldVarName = null;
            StructLocalInfo info;
            if (!StructLocals.TryGetValue(baseName, out info)) return false;
            return info.FieldToVar.TryGetValue(fieldName, out fieldVarName);
        }

        public bool IsStructLocal(string name)
        {
            return StructLocals.ContainsKey(name);
        }

        public CType FindTypeOfName(string name)
        {
            CType t;
            if (Locals.TryGetValue(name, out t)) return t;
            if (Params.TryGetValue(name, out t)) return t;
            if (Global.Globals.TryGetValue(name, out t)) return t;
            if (Global.ConstTypes.TryGetValue(name, out t)) return t;
            return CType.UInt8;
        }

        public CType FindFuncReturn(string fname)
        {
            CType t;
            if (Global.FuncReturns.TryGetValue(fname, out t)) return t;
            return CType.UInt8;
        }


        public CType[] FindFuncParams(string fname)
        {
            CType[] pts;
            if (Global.FuncParamTypes.TryGetValue(fname, out pts)) return pts;
            return null;
        }

        public void RegisterLocal(CType t, string name, int[] range = null, FilePosition pos = default(FilePosition))
        {
            if (!Locals.ContainsKey(name)) Locals.Add(name, t);
            if (!LocalUseCounts.ContainsKey(name)) LocalUseCounts.Add(name, 0);
            if (range != null && !LocalRanges.ContainsKey(name)) LocalRanges.Add(name, range);
            if (pos.Filename != null && !LocalDeclPos.ContainsKey(name)) LocalDeclPos.Add(name, pos);
        }

        public void RegisterParam(CType t, string name, int[] range = null, FilePosition pos = default(FilePosition))
        {
            if (!Params.ContainsKey(name)) Params.Add(name, t);
            if (!ParamUseCounts.ContainsKey(name)) ParamUseCounts.Add(name, 0);
            if (range != null && !ParamRanges.ContainsKey(name)) ParamRanges.Add(name, range);
            if (pos.Filename != null && !ParamDeclPos.ContainsKey(name)) ParamDeclPos.Add(name, pos);
        }

        public void MarkNameUse(string name)
        {
            if (string.IsNullOrEmpty(name)) return;
            int c;
            if (LocalUseCounts.TryGetValue(name, out c)) LocalUseCounts[name] = c + 1;
            if (ParamUseCounts.TryGetValue(name, out c)) ParamUseCounts[name] = c + 1;
        }
    }

    sealed class TempPool
    {
        int nextId = 0;
        readonly Dictionary<string, Stack<string>> freeByType = new Dictionary<string, Stack<string>>();
        readonly Dictionary<string, string> typeKeyByName = new Dictionary<string, string>();
        public readonly List<Expr> Decls = new List<Expr>();

        static CType NormalizeTempType(CType t)
        {
            if (t == null) return CType.UInt16;
            t = t.WithoutConst();
            if (t.IsArray) return CType.MakePointer(t.Subtype ?? CType.UInt8);
            if (t.IsStructOrUnion) return CType.UInt16;
            return t;
        }

        static string TempTypeKey(CType t)
        {
            return (t == null) ? "<null>" : t.Show();
        }

        public string Acquire(CType t)
        {
            CType declType = NormalizeTempType(t);
            int sz = SizeOfType(declType);
            if (sz != 1) sz = 2;
            string key = TempTypeKey(declType);

            Stack<string> st;
            if (freeByType.TryGetValue(key, out st))
            {
                if (st.Count > 0) return st.Pop();
            }
            else
            {
                st = new Stack<string>();
                freeByType.Add(key, st);
            }

            string name = (sz == 1 ? "__t8_" : "__t16_") + nextId.ToString();
            nextId++;

            typeKeyByName[name] = key;
            Decls.Add(Expr.Make(Tag.Variable, declType, name));
            return name;
        }

        public void Release(string name)
        {
            string key;
            if (!typeKeyByName.TryGetValue(name, out key)) return;

            Stack<string> st;
            if (!freeByType.TryGetValue(key, out st))
            {
                st = new Stack<string>();
                freeByType.Add(key, st);
            }
            st.Push(name);
        }
    }

    
    static bool TryConstInt(Expr e, out int v)
    {
        if (e.Match(Tag.Integer, out v)) return true;
        v = 0;
        return false;
    }

    static bool TryGetRange(FunctionCtx ctx, string name, out int min, out int max)
    {
        int[] r;

        // Flow-sensitive overrides first (if/else refinements).
        if (ctx.TryGetRangeOverride(name, out r))
        {
            min = r[0];
            max = r[1];
            return true;
        }

        if (ctx.LocalRanges.TryGetValue(name, out r) || ctx.ParamRanges.TryGetValue(name, out r))
        {
            min = r[0];
            max = r[1];
            return true;
        }
        // fall back to global annotation
        if (ctx.Global.VarRanges.TryGetValue(name, out r))
        {
            min = r[0];
            max = r[1];
            return true;
        }
        min = 0; max = 0;
        return false;
    }


    static bool TryGetArrayLength(FunctionCtx ctx, CType t, out int len)
    {
        len = 0;
        if (t == null) return false;
        t = t.WithoutConst();
        if (t.Tag == CTypeTag.Array)
        {
            len = t.Dimension;
            return len >= 0;
        }
        if (t.Tag == CTypeTag.ArrayWithDimensionExpression)
        {
            // Evaluate the dimension expression without emitting diagnostics.
            return ctx.Global.TryEvalConstInt(t.DimensionExpression, out len);
        }
        return false;
    }

    static int ClampLongToInt(long v)
    {
        if (v < int.MinValue) return int.MinValue;
        if (v > int.MaxValue) return int.MaxValue;
        return (int)v;
    }

    static bool TryInferIntRange(FunctionCtx ctx, Expr e, out int min, out int max)
    {
        min = 0; max = 0;
        if (e == null) return false;

        if (e.Match(Tag.Integer, out int n))
        {
            min = n; max = n;
            return true;
        }

        if (e.Match(Tag.Name, out string nm))
        {
            if (TryGetRange(ctx, nm, out min, out max)) return true;
            if (ctx != null && ctx.Global != null && ctx.Global.TryEvalConstInt(e, out int c))
            {
                min = c; max = c;
                return true;
            }
            return false;
        }

        if (e.Match(Tag.Cast, out CType ct, out Expr sub))
        {
            if (!TryInferIntRange(ctx, sub, out min, out max)) return false;

            if (TryGetNarrowingTypeInfo(ct, out int bits, out bool signed))
            {
                long tmin, tmax;
                if (signed)
                {
                    tmin = -(1L << (bits - 1));
                    tmax = (1L << (bits - 1)) - 1;
                }
                else
                {
                    tmin = 0;
                    tmax = (1L << bits) - 1;
                }
                if (min < tmin) min = (int)tmin;
                if (max > tmax) max = (int)tmax;
            }
            return true;
        }

        if (e.Match(Tag.Conditional, out Expr _c, out Expr te, out Expr fe))
        {
            if (TryInferIntRange(ctx, te, out int tmin, out int tmax) &&
                TryInferIntRange(ctx, fe, out int fmin, out int fmax))
            {
                min = Math.Min(tmin, fmin);
                max = Math.Max(tmax, fmax);
                return true;
            }
            return false;
        }

        if (e.Match(Tag.Add, out Expr a, out Expr b))
        {
            if (!TryInferIntRange(ctx, a, out int amin, out int amax) ||
                !TryInferIntRange(ctx, b, out int bmin, out int bmax))
                return false;
            min = ClampLongToInt((long)amin + bmin);
            max = ClampLongToInt((long)amax + bmax);
            return true;
        }
        if (e.Match(Tag.Subtract, out a, out b))
        {
            if (!TryInferIntRange(ctx, a, out int amin, out int amax) ||
                !TryInferIntRange(ctx, b, out int bmin, out int bmax))
                return false;
            min = ClampLongToInt((long)amin - bmax);
            max = ClampLongToInt((long)amax - bmin);
            return true;
        }
        if (e.Match(Tag.ShiftLeft, out a, out b))
        {
            if (!TryInferIntRange(ctx, a, out int amin, out int amax) || !TryConstInt(b, out int sh) || sh < 0 || sh > 15)
                return false;
            min = ClampLongToInt((long)amin << sh);
            max = ClampLongToInt((long)amax << sh);
            return true;
        }
        if (e.Match(Tag.ShiftRight, out a, out b))
        {
            if (!TryInferIntRange(ctx, a, out int amin, out int amax) || !TryConstInt(b, out int sh) || sh < 0 || sh > 15)
                return false;
            min = amin >> sh;
            max = amax >> sh;
            return true;
        }
        if (e.Match(Tag.Modulus, out a, out b))
        {
            if (!TryInferIntRange(ctx, a, out int amin, out int amax) || !TryConstInt(b, out int mod) || mod <= 0)
                return false;
            if (amin >= 0)
            {
                min = 0;
                max = mod - 1;
                return true;
            }
            return false;
        }
        if (e.Match(Tag.BitwiseAnd, out a, out b))
        {
            bool aConst = TryConstInt(a, out int av);
            bool bConst = TryConstInt(b, out int bv);
            if (aConst && bConst)
            {
                min = av & bv;
                max = min;
                return true;
            }

            if (aConst && av >= 0 && TryInferIntRange(ctx, b, out int bmin, out int bmax) && bmin >= 0)
            {
                min = 0;
                max = Math.Min(av, bmax);
                return true;
            }
            if (bConst && bv >= 0 && TryInferIntRange(ctx, a, out int amin, out int amax) && amin >= 0)
            {
                min = 0;
                max = Math.Min(bv, amax);
                return true;
            }
            return false;
        }

        if (ctx != null && ctx.Global != null && ctx.Global.TryEvalConstInt(e, out int cst))
        {
            min = cst; max = cst;
            return true;
        }

        return false;
    }

    static bool IsSideEffectFreeForRangeEval(Expr expr)
    {
        if (expr == null) return true;

        if (expr.MatchTag(Tag.Call) ||
            expr.MatchTag(Tag.Assign) ||
            expr.MatchTag(Tag.AssignModify) ||
            expr.MatchTag(Tag.PreIncrement) ||
            expr.MatchTag(Tag.PostIncrement) ||
            expr.MatchTag(Tag.PreDecrement) ||
            expr.MatchTag(Tag.PostDecrement))
        {
            return false;
        }

        object[] args = expr.GetArgs();
        for (int i = 1; i < args.Length; i++)
        {
            if (args[i] is Expr sub)
            {
                if (!IsSideEffectFreeForRangeEval(sub)) return false;
            }
            else if (args[i] is Expr[] many)
            {
                for (int j = 0; j < many.Length; j++)
                    if (!IsSideEffectFreeForRangeEval(many[j])) return false;
            }
        }
        return true;
    }

    static bool TryEvalConditionFromRanges(FunctionCtx ctx, Expr cond, out int value01)
    {
        value01 = 0;
        if (cond == null) return false;
        if (!IsSideEffectFreeForRangeEval(cond)) return false;

        if (cond.Match(Tag.Integer, out int n))
        {
            value01 = (n != 0) ? 1 : 0;
            return true;
        }

        if (cond.Match(Tag.LogicalNot, out Expr sub))
        {
            if (!TryEvalConditionFromRanges(ctx, sub, out int v)) return false;
            value01 = (v == 0) ? 1 : 0;
            return true;
        }

        if (cond.Match(Tag.LogicalAnd, out Expr la, out Expr lb))
        {
            if (TryEvalConditionFromRanges(ctx, la, out int av))
            {
                if (av == 0) { value01 = 0; return true; }
                if (TryEvalConditionFromRanges(ctx, lb, out int bv)) { value01 = (bv != 0) ? 1 : 0; return true; }
                return false;
            }
            return false;
        }
        if (cond.Match(Tag.LogicalOr, out la, out lb))
        {
            if (TryEvalConditionFromRanges(ctx, la, out int av))
            {
                if (av != 0) { value01 = 1; return true; }
                if (TryEvalConditionFromRanges(ctx, lb, out int bv)) { value01 = (bv != 0) ? 1 : 0; return true; }
                return false;
            }
            return false;
        }

        if (cond.Match(Tag.Equal, out Expr l, out Expr r) ||
            cond.Match(Tag.NotEqual, out l, out r) ||
            cond.Match(Tag.LessThan, out l, out r) ||
            cond.Match(Tag.LessThanOrEqual, out l, out r) ||
            cond.Match(Tag.GreaterThan, out l, out r) ||
            cond.Match(Tag.GreaterThanOrEqual, out l, out r))
        {
            if (!TryInferIntRange(ctx, l, out int lmin, out int lmax) ||
                !TryInferIntRange(ctx, r, out int rmin, out int rmax))
                return false;

            string op = cond.GetTag();
            if (op == Tag.Equal)
            {
                if (lmax < rmin || rmax < lmin) { value01 = 0; return true; }
                if (lmin == lmax && rmin == rmax && lmin == rmin) { value01 = 1; return true; }
                return false;
            }
            if (op == Tag.NotEqual)
            {
                if (lmax < rmin || rmax < lmin) { value01 = 1; return true; }
                if (lmin == lmax && rmin == rmax && lmin == rmin) { value01 = 0; return true; }
                return false;
            }
            if (op == Tag.LessThan)
            {
                if (lmax < rmin) { value01 = 1; return true; }
                if (lmin >= rmax) { value01 = 0; return true; }
                return false;
            }
            if (op == Tag.LessThanOrEqual)
            {
                if (lmax <= rmin) { value01 = 1; return true; }
                if (lmin > rmax) { value01 = 0; return true; }
                return false;
            }
            if (op == Tag.GreaterThan)
            {
                if (lmin > rmax) { value01 = 1; return true; }
                if (lmax <= rmin) { value01 = 0; return true; }
                return false;
            }
            if (op == Tag.GreaterThanOrEqual)
            {
                if (lmin >= rmax) { value01 = 1; return true; }
                if (lmax < rmin) { value01 = 0; return true; }
                return false;
            }
        }

        if (TryInferIntRange(ctx, cond, out int cmin, out int cmax))
        {
            if (cmin == 0 && cmax == 0) { value01 = 0; return true; }
            if (cmin > 0 || cmax < 0) { value01 = 1; return true; }
        }

        return false;
    }

    static CType MakeSafeIndexCastTypeForRange(int maxValue)
    {
        CType t = CType.MakeSimple((maxValue <= 0xFF) ? CSimpleType.UInt8 : CSimpleType.UInt16);
        t.IsSafeIndex = true;
        return t;
    }

    static Expr ApplyIndexRangeAndLints(FunctionCtx ctx, Expr sourceExpr, Expr arrayExpr, Expr indexExpr)
    {
        Expr idx = indexExpr;
        if (idx == null) return indexExpr;

        if (TryInferIntRange(ctx, idx, out int imin, out int imax))
        {
            CType bt = InferType(ctx, arrayExpr);
            int len;
            if (TryGetArrayLength(ctx, bt, out len) && len > 0)
            {
                if (imin < 0 || imax >= len)
                {
                    if (idx.Match(Tag.Name, out string idxName) && TryGetRange(ctx, idxName, out int rmin, out int rmax))
                    {
                        Program.Warning(sourceExpr.Source, ErrorCode.RangeIndexOob,
                            "__range({0},{1}) index '{2}' may go out of bounds for array length {3}",
                            rmin, rmax, idxName, len);
                    }
                    else
                    {
                        Program.Warning(sourceExpr.Source, ErrorCode.RangeIndexOob,
                            "index range [{0},{1}] may go out of bounds for array length {2}",
                            imin, imax, len);
                    }
                }
                else if (imin >= 0 && imax < len && imax <= 0xFFFF && !HasTopLevelCast(idx))
                {
                    idx = Expr.Make(Tag.Cast, MakeSafeIndexCastTypeForRange(imax), idx).WithSource(idx.Source);
                }
            }
        }

        if (idx.Match(Tag.Name, out string sidxName))
        {
            CType it = InferType(ctx, idx);
            if (IsPlainInteger(it) && !IsSafeIndex(it))
            {
                Program.Warning(sourceExpr.Source, ErrorCode.SafeIndexIndex,
                    "array index '{0}' has type {1} which is not __safe_index; consider '__safe_index' or cast explicitly to silence",
                    sidxName, it == null ? "<unknown>" : it.Show());
            }
        }

        return idx;
    }
// ---------- Function lowering ----------



    // --- __restrict alias lint (lightweight, compile-time only) ---
    // If a callee has multiple __restrict pointer parameters, warn when two
    // actual arguments appear to share the same base (name-based heuristic).
    // Suppressed within __unsafe { ... } and when either argument is explicitly cast.
    static void RestrictAliasLint(FunctionCtx ctx, Expr func, Expr[] args, FilePosition src)
    {
        string fname;
        if (!func.Match(Tag.Name, out fname)) return;

        CType[] ps = ctx.FindFuncParams(fname);
        if (ps == null || ps.Length == 0) return;

        // Collect indices of __restrict pointer parameters that have a corresponding arg.
        List<int> rix = new List<int>();
        int n = ps.Length < args.Length ? ps.Length : args.Length;
        for (int i = 0; i < n; i++)
        {
            CType pt = ps[i];
            if (pt != null && pt.Tag == CTypeTag.Pointer && pt.IsRestrict)
                rix.Add(i);
        }
        if (rix.Count < 2) return;

        for (int a = 0; a < rix.Count; a++)
        {
            for (int b = a + 1; b < rix.Count; b++)
            {
                int i = rix[a];
                int j = rix[b];

                bool castI = false;
                bool castJ = false;
                string bi = GetRestrictBaseSym(args[i], ref castI);
                string bj = GetRestrictBaseSym(args[j], ref castJ);

                // Explicit casts suppress the lint (escape hatch).
                if (castI || castJ) continue;

                if (bi != null && bj != null && bi == bj)
                {
                    Program.Warning(src, ErrorCode.RestrictAlias,
                        "__restrict may be violated in call to {0}: arg{1} and arg{2} share base '{3}'",
                        fname, i, j, bi);
                }
            }
        }
    }

    static Expr StripRestrictCasts(Expr e, ref bool hadCast)
    {
        CType ct;
        Expr sub;
        while (e.Match(Tag.Cast, out ct, out sub))
        {
            hadCast = true;
            e = sub;
        }
        return e;
    }


    static bool IsConstScalarInRomType(CType t)
    {
        // const scalar materialization option: allow &k by storing it in ROM
        if (!Program.ConstScalarInRom) return false;
        if (!t.IsConst) return false;
        if (t.IsArray || t.IsEnum) return false;
        if (!t.IsSimple) return false;
        return t.SimpleType == CSimpleType.UInt8 || t.SimpleType == CSimpleType.Int8 ||
               t.SimpleType == CSimpleType.UInt16 || t.SimpleType == CSimpleType.Int16;
    }

    static bool IsPointerToConst(CType t)
    {
        return t.Tag == CTypeTag.Pointer && t.Subtype != null && t.Subtype.IsConst;
    }

    static bool IsPointerToNonConst(CType t)
    {
        return t.Tag == CTypeTag.Pointer && t.Subtype != null && !t.Subtype.IsConst;
    }

    static bool IsArrayOfConst(CType t)
    {
        if (t == null) return false;
        if (t.Tag == CTypeTag.Array || t.Tag == CTypeTag.ArrayWithDimensionExpression)
            return t.Subtype != null && t.Subtype.IsConst;
        return false;
    }

    static bool HasTopLevelCast(Expr e)
    {
        CType _t;
        Expr _sub;
        return e.Match(Tag.Cast, out _t, out _sub);
    }

    static CType PointerTempType(FunctionCtx ctx, Expr ptrExpr)
    {
        CType t = InferType(ctx, ptrExpr);
        if (t != null)
        {
            if (t.IsPointer) return t;
            if (t.IsArray) return CType.MakePointer(t.Subtype ?? CType.UInt8);
        }
        return CType.UInt16;
    }

    static void WarnConstDiscard(Expr origin, string context)
    {
        Program.Warning(origin.Source, ErrorCode.ConstDiscard, $"{context}: discards 'const' qualifier from pointee type");
    }

    // Returns a "base symbol" name for simple pointer expressions.
    // Examples:
    // p -> p
    // p+1 -> p
    // &buf[i] -> buf
    // &s.field -> s
    static string GetRestrictBaseSym(Expr e, ref bool hadCast)
    {
        e = StripRestrictCasts(e, ref hadCast);

        string name;
        Expr sub;
        Expr left, right;
        int c;

        if (e.Match(Tag.Name, out name)) return name;

        if (e.Match(Tag.AddressOf, out sub))
        {
            sub = StripRestrictCasts(sub, ref hadCast);
            if (sub.Match(Tag.Name, out name)) return name;
            if (sub.Match(Tag.Index, out left, out right))
            {
                if (left.Match(Tag.Name, out name)) return name;
                return GetRestrictBaseSym(left, ref hadCast);
            }
            Expr obj; string field;
            if (sub.Match(Tag.Field, out obj, out field))
            {
                return GetRestrictBaseSym(obj, ref hadCast);
            }
            return null;
        }

        // Pointer arithmetic: keep base when adding/subtracting a constant.
        if (e.Match(Tag.Add, out left, out right) || e.Match(Tag.Subtract, out left, out right))
        {
            if (right.Match(Tag.Integer, out c)) return GetRestrictBaseSym(left, ref hadCast);
            if (left.Match(Tag.Integer, out c)) return GetRestrictBaseSym(right, ref hadCast);
        }

        if (e.Match(Tag.Index, out left, out right))
        {
            if (left.Match(Tag.Name, out name)) return name;
            return GetRestrictBaseSym(left, ref hadCast);
        }

        Expr obj2; string field2;
        if (e.Match(Tag.Field, out obj2, out field2))
        {
            return GetRestrictBaseSym(obj2, ref hadCast);
        }

        return null;
    }

    static Expr LowerFunctionBody(GlobalEnv genv, string fname, CType retType, FieldInfo[] ps, Expr body)
    {
        FunctionCtx ctx = new FunctionCtx(genv);
        ctx.FunctionName = fname ?? "";
        ctx.FunctionReturnType = retType ?? CType.Void;

        // Register parameters.
        for (int i = 0; i < ps.Length; i++)
        {
            FieldInfo p = ps[i];
            ctx.RegisterParam(p.Type, p.Name, null, body.Source);
        }

        List<Expr> lowered = new List<Expr>();

        ctx.BeginStatement();
        List<Expr> outStmts = LowerStatement(ctx, body);
        ctx.EndStatement();
        for (int j = 0; j < outStmts.Count; j++) lowered.Add(outStmts[j]);

        EmitUnusedLocalAndParamWarnings(ctx);

        // Prepend new temp declarations.
        if (ctx.Temps.Decls.Count > 0)
        {
            List<Expr> all = new List<Expr>(ctx.Temps.Decls.Count + lowered.Count);
            for (int i = 0; i < ctx.Temps.Decls.Count; i++) all.Add(ctx.Temps.Decls[i]);
            for (int i = 0; i < lowered.Count; i++) all.Add(lowered[i]);
            return MakeSequence(all).WithSource(body.Source);
        }

        return MakeSequence(lowered).WithSource(body.Source);
    }

    static List<Expr> LowerStatement(FunctionCtx ctx, Expr stmt)
    {
        List<Expr> result = new List<Expr>();

        // Block / sequence
        Expr[] seq;
        if (stmt.MatchAny(Tag.Sequence, out seq))
        {
            List<Expr> flat = new List<Expr>();
            bool terminated = false;
            for (int i = 0; i < seq.Length; i++)
            {
                string _lbl;
                bool isLabel = seq[i].Match(Tag.Label, out _lbl);
                if (terminated && !isLabel)
                {
                    Program.Warning(seq[i].Source, ErrorCode.UnreachableCode,
                        "unreachable code");
                }
                if (isLabel) terminated = false;

                ctx.BeginStatement();
                List<Expr> lowered = LowerStatement(ctx, seq[i]);
                ctx.EndStatement();
                for (int j = 0; j < lowered.Count; j++) flat.Add(lowered[j]);

                if (EndsControlFlowLowered(lowered))
                    terminated = true;
            }
            result.Add(MakeSequence(flat).WithSource(stmt.Source));
            return result;
        }

        // static_assert: keep as a statement, but lower the condition expression
        Expr saCond; string saMsg;
        if (stmt.Match(Tag.StaticAssert, out saCond, out saMsg))
        {
            var dummy = new List<Expr>();
            Expr loweredCond = LowerExpr(ctx, saCond, false, dummy);
            result.Add(Expr.Make(Tag.StaticAssert, loweredCond, saMsg).WithSource(stmt.Source));
            return result;
        }

        // --- __unsafe boundary
        Expr unsafeInner;
        if (stmt.Match(Tag.Unsafe, out unsafeInner))
        {
            ctx.PushUnsafe();
            List<Expr> loweredInner = LowerStatement(ctx, unsafeInner);
            ctx.PopUnsafe();

            Expr combined = loweredInner.Count == 1 ? loweredInner[0] : MakeSequence(loweredInner).WithSource(unsafeInner.Source);
            result.Add(Expr.Make(Tag.Unsafe, combined).WithSource(stmt.Source));
            return result;
        }

        // Local declaration
        CType vt;
        string vname;
        Expr vrExpr;
        if (stmt.Match(Tag.Variable, out vt, out vname, out vrExpr) || stmt.Match(Tag.Variable, out vt, out vname))
        {
            // If this is a plain local struct (by-value) variable, desugar it into per-field locals.
            // This makes member access usable without relying on address-of.
	            if ((vt.Tag == CTypeTag.Struct || vt.Tag == CTypeTag.Union) && !vt.IsArray && !vt.IsPointer)
            {
                Dictionary<string, CType> fields;
                if (ctx.Global.StructFieldTypes.TryGetValue(vt.Name, out fields) && fields.Count > 0)
                {
                    StructLocalInfo info = new StructLocalInfo(vt);
                    foreach (KeyValuePair<string, CType> kv in fields)
                    {
                        string fieldName = kv.Key;
                        CType fieldType = kv.Value;
                        string fieldVar = vname + "__" + fieldName;
                        info.FieldToVar[fieldName] = fieldVar;
                        ctx.RegisterLocal(fieldType, fieldVar, null, stmt.Source);
                        result.Add(Expr.Make(Tag.Variable, fieldType, fieldVar).WithSource(stmt.Source));
                    }
                    ctx.StructLocals[vname] = info;
                    return result;
                }
            }

            int[] vr = (vrExpr != null) ? ctx.Global.EvaluateRange(vrExpr) : null;

            ctx.RegisterLocal(vt, vname, vr, stmt.Source);
            result.Add(stmt);
            return result;
        }

        // Return (no value)
        if (stmt.Match(Tag.Return))
        {
            result.Add(stmt);
            return result;
        }

        // Return (with value) -> allow extraction before return
        Expr sub;
        if (stmt.Match(Tag.Return, out sub))
        {
            List<Expr> prefix = new List<Expr>();
            Expr newSub = LowerExpr(ctx, sub, true, prefix);
            WarnImplicitNarrowingIfNeeded(ctx, newSub, InferType(ctx, newSub), ctx.FunctionReturnType, stmt.Source,
                "return in '" + (ctx.FunctionName ?? "") + "'");
            for (int i = 0; i < prefix.Count; i++) result.Add(prefix[i]);
            result.Add(Expr.Make(Tag.Return, newSub).WithSource(stmt.Source));
            return result;
        }

        // For / while
        Expr init, test, induct, body;
        if (stmt.Match(Tag.For, out init, out test, out induct, out body))
        {
            // init: executed once
            Expr newInit = LowerStatementAsSingle(ctx, init);

            // induct: executed each iteration (after body)
            Expr newInduct = LowerStatementAsSingle(ctx, induct);

            // body: statement
            ctx.BeginStatement();
            List<Expr> loweredBody = LowerStatement(ctx, body);
            ctx.EndStatement();
            Expr newBody = (loweredBody.Count == 1) ? loweredBody[0] : MakeSequence(loweredBody);

            // test: expression
            List<Expr> testPrefix = new List<Expr>();
            Expr newTest = test.Match(Tag.Empty) ? test : LowerExpr(ctx, test, true, testPrefix);

            // If we extracted prefix statements for the test, inject them before the first test (in init)
            // and before subsequent tests (in induct).
            if (testPrefix.Count > 0)
            {
                Expr prefixSeq = MakeSequence(testPrefix).WithSource(stmt.Source);

                newInit = ConcatStatements(newInit, prefixSeq, stmt.Source);
                newInduct = ConcatStatements(newInduct, prefixSeq, stmt.Source);
            }

            // Important: the backend's FOR uses the loop-top label as ContinueLabel, so plain `continue`
            // would otherwise skip the induct step. Rewrite continues whenever the lowered for-loop has
            // any induct work at all, including the common `i++` case and any injected test-prefix work.
            if (!newInduct.Match(Tag.Empty))
                newBody = RewriteContinueInCurrentLoop(newBody, newInduct, stmt.Source);

            // Constant-control-flow pruning:
            // for(...;0;...) { ... } => init only
            // This also covers while(0) desugared as for(;0;).
            int tconst;
            if (newTest.Match(Tag.Integer, out tconst) && tconst == 0)
            {
                if (!newInit.Match(Tag.Empty)) result.Add(newInit.WithSource(stmt.Source));
                return result;
            }

            result.Add(Expr.Make(Tag.For, newInit, newTest, newInduct, newBody).WithSource(stmt.Source));
            return result;
        }

        // If (pairs of cond/body)
        Expr[] parts;
        
        // Do-while (do {body} while (test);)
        Expr doBody;
        Expr doTest;
        if (stmt.Match(Tag.DoWhile, out doBody, out doTest))
        {
            // Body statements
            ctx.BeginStatement();
            var loweredBodyStmts = LowerStatement(ctx, doBody);
            ctx.EndStatement();

            Expr newBody;
            if (loweredBodyStmts.Count == 0) newBody = Expr.Make(Tag.Empty);
            else if (loweredBodyStmts.Count == 1) newBody = loweredBodyStmts[0];
            else newBody = MakeSequence(loweredBodyStmts);

            // Condition (extract prefix)
            var testPrefix = new List<Expr>();
            Expr newTest = LowerExpr(ctx, doTest, true, testPrefix);

            if (testPrefix.Count > 0)
            {
                Expr prefixSeq = MakeSequence(testPrefix).WithSource(stmt.Source);

                // Continue should execute prefix then jump to condition check
                newBody = RewriteContinueInCurrentLoop(newBody, prefixSeq, stmt.Source);

                // Normal end-of-body should also execute prefix before checking condition
                newBody = ConcatStatements(newBody, prefixSeq, stmt.Source);
            }

            result.Add(Expr.Make(Tag.DoWhile, newBody, newTest).WithSource(stmt.Source));
            return result;
        }

if (stmt.MatchAny(Tag.If, out parts))
        {
            Expr loweredIf = LowerIfChain(ctx, parts, stmt.Source);
            result.Add(loweredIf);
            return result;
        }

        // Switch
        Expr swTest;
        Expr[] cases;
        Expr defBody;
        if (stmt.Match(Tag.Switch, out swTest, out cases, out defBody))
        {
            List<Expr> prefix = new List<Expr>();
            Expr newTest = LowerExpr(ctx, swTest, true, prefix);

            // If switch test is an enum, be stricter about case label types.
            CType swType = InferType(ctx, swTest);
            bool swIsEnum = (swType != null && swType.IsEnum);
            bool swIsStrictEnum = swIsEnum && swType.IsEnumStrict;
            string swEnumName = swIsEnum ? swType.Name : null;

            Expr[] newCases = new Expr[cases.Length];
            for (int i = 0; i < cases.Length; i++)
            {
                Expr valExpr;
                Expr caseBody;
                if (cases[i].Match(Tag.Case, out valExpr, out caseBody))
                {
                    if (swIsEnum)
                    {
                        CType caseT = InferType(ctx, valExpr);
                        if (caseT != null && caseT.IsEnum)
                        {
                            if (caseT.Name != swEnumName)
                            {
                                if (swIsStrictEnum)
                                {
                                    Program.Error(Maybe.Just(valExpr.Source), ErrorCode.SwitchCaseEnumMismatch,
                                        "case label enum '{0}' does not match strict switch enum '{1}'", caseT.Name, swEnumName);
                                }
                                else
                                {
                                    Program.Warning(Maybe.Just(valExpr.Source), ErrorCode.SwitchCaseEnumMismatch,
                                        "case label enum '{0}' does not match switch enum '{1}'", caseT.Name, swEnumName);
                                }
                            }
                        }
                        else
                        {
                            if (swIsStrictEnum)
                            {
                                Program.Error(Maybe.Just(valExpr.Source), ErrorCode.SwitchCaseNonEnumOnEnumSwitch,
                                    "case label is not an enum value for strict switch(enum {0})", swEnumName);
                            }
                            else
                            {
                                Program.Warning(Maybe.Just(valExpr.Source), ErrorCode.SwitchCaseNonEnumOnEnumSwitch,
                                    "case label is not an enum value for switch(enum {0})", swEnumName);
                            }
                        }
                    }
                    ctx.BeginStatement();
                    List<Expr> loweredCase = LowerStatement(ctx, caseBody);
                    ctx.EndStatement();
                    Expr newCaseBody = (loweredCase.Count == 1) ? loweredCase[0] : MakeSequence(loweredCase);
                    newCases[i] = Expr.Make(Tag.Case, valExpr, newCaseBody).WithSource(cases[i].Source);
                }
                else
                {
                    newCases[i] = cases[i];
                }
            }

            ctx.BeginStatement();
            List<Expr> loweredDef = LowerStatement(ctx, defBody);
            ctx.EndStatement();
            Expr newDef = (loweredDef.Count == 1) ? loweredDef[0] : MakeSequence(loweredDef);

            for (int i = 0; i < prefix.Count; i++) result.Add(prefix[i]);
            result.Add(Expr.Make(Tag.Switch, newTest, newCases, newDef).WithSource(stmt.Source));
            return result;
        }

        // Label
        string label;
        if (stmt.Match(Tag.Label, out label))
        {
            result.Add(stmt);
            return result;
        }

        // Jump (goto)
        if (stmt.Match(Tag.Jump, out label))
        {
            result.Add(stmt);
            return result;
        }

        // Continue / break / fallthrough marker
        if (stmt.Match(Tag.Continue) || stmt.Match(Tag.Break) || stmt.Match(Tag.Fallthrough))
        {
            result.Add(stmt);
            return result;
        }

        // Raw asm statement
        string mnemonic;
        AsmOperand op;
        if (stmt.Match(Tag.Asm, out mnemonic, out op))
        {
            result.Add(stmt);
            return result;
        }

        // Default: expression statement
        {
            List<Expr> prefix = new List<Expr>();
            Expr newExpr = LowerExpr(ctx, stmt, true, prefix);
            for (int i = 0; i < prefix.Count; i++) result.Add(prefix[i]);
            result.Add(newExpr);
            return result;
        }
    }

    // Lowers a statement node and returns a single statement expression.
    // If the lowering expands into multiple statements, they are wrapped in a $sequence.
    static Expr LowerStatementAsSingle(FunctionCtx ctx, Expr stmt)
    {
        if (stmt.Match(Tag.Empty)) return stmt;

        ctx.BeginStatement();
        List<Expr> lowered = LowerStatement(ctx, stmt);
        ctx.EndStatement();

        if (lowered.Count == 1) return lowered[0];
        return MakeSequence(lowered).WithSource(stmt.Source);
    }


    static Expr ConcatStatements(Expr first, Expr second, FilePosition src)
    {
        if (first.Match(Tag.Empty)) return second;
        if (second.Match(Tag.Empty)) return first;

        List<Expr> items = new List<Expr>();
        Expr[] seq;
        if (first.MatchAny(Tag.Sequence, out seq)) items.AddRange(seq); else items.Add(first);
        if (second.MatchAny(Tag.Sequence, out seq)) items.AddRange(seq); else items.Add(second);
        return MakeSequence(items).WithSource(src);
    }

    static Expr RewriteContinueInCurrentLoop(Expr stmt, Expr inductAndPrefix, FilePosition src)
    {
        // Important: do NOT rewrite continues that belong to nested loops.
        Expr init, test, induct, body;
        if (stmt.Match(Tag.For, out init, out test, out induct, out body))
            return stmt;

        if (stmt.Match(Tag.Continue))
        {
            return ConcatStatements(inductAndPrefix, stmt, src).WithSource(stmt.Source);
        }

        Expr[] seq;
        if (stmt.MatchAny(Tag.Sequence, out seq))
        {
            List<Expr> items = new List<Expr>(seq.Length);
            for (int i = 0; i < seq.Length; i++)
                items.Add(RewriteContinueInCurrentLoop(seq[i], inductAndPrefix, src));
            return MakeSequence(items).WithSource(stmt.Source);
        }

	        // (No Tag.Block in this AST; blocks are represented as $sequence.)

        Expr[] parts;
        if (stmt.MatchAny(Tag.If, out parts))
        {
            object[] newParts = new object[parts.Length];
            for (int i = 0; i < parts.Length; i += 2)
            {
                newParts[i] = parts[i];
                newParts[i + 1] = RewriteContinueInCurrentLoop(parts[i + 1], inductAndPrefix, src);
            }
            object[] args = new object[newParts.Length + 1];
            args[0] = Tag.If;
            for (int i = 0; i < newParts.Length; i++)
                args[i + 1] = newParts[i];
            return Expr.Make(args).WithSource(stmt.Source);
        }

        Expr swTest;
        Expr[] cases;
        Expr defBody;
        if (stmt.Match(Tag.Switch, out swTest, out cases, out defBody))
        {
	            Expr[] newCases = new Expr[cases.Length];
	            for (int i = 0; i < cases.Length; i++)
	            {
	                int v;
	                Expr caseBody;
	                if (cases[i].Match(Tag.Case, out v, out caseBody))
	                {
	                    Expr newCaseBody = RewriteContinueInCurrentLoop(caseBody, inductAndPrefix, src);
	                    newCases[i] = Expr.Make(Tag.Case, v, newCaseBody).WithSource(cases[i].Source);
	                }
	                else
	                {
	                    newCases[i] = cases[i];
	                }
	            }
	            Expr newDef = RewriteContinueInCurrentLoop(defBody, inductAndPrefix, src);
	            return Expr.Make(Tag.Switch, swTest, newCases, newDef).WithSource(stmt.Source);
        }

        return stmt;
    }

	    
    // ---------- __range flow refinement (very small subset) ----------
    // Refines annotated ranges inside if/else blocks for simple comparisons like:
    // if (x > 9) { ... } else { ... }
    // Supported:
    // x <const, x <=const, x >const, x >=const, x ==const
    // LogicalAnd/LogicalOr/Not with the usual conservative rules.
    static Dictionary<string, int[]> RefineRangesFromCondition(FunctionCtx ctx, Expr cond, bool assumeTrue)
    {
        Dictionary<string, int[]> ov = new Dictionary<string, int[]>();
        AddRefinements(ctx, cond, assumeTrue, ov);
        return ov;
    }

    static void AddRefinements(FunctionCtx ctx, Expr cond, bool assumeTrue, Dictionary<string, int[]> ov)
    {
        if (cond == null) return;

        // !expr
        Expr sub;
        if (cond.Match(Tag.LogicalNot, out sub))
        {
            AddRefinements(ctx, sub, !assumeTrue, ov);
            return;
        }

        // && / ||
        Expr a, b;
        if (cond.Match(Tag.LogicalAnd, out a, out b))
        {
            if (assumeTrue)
            {
                AddRefinements(ctx, a, true, ov);
                AddRefinements(ctx, b, true, ov);
            }
            // assumeFalse is ambiguous (either side could be false) -> no refinement
            return;
        }
        if (cond.Match(Tag.LogicalOr, out a, out b))
        {
            if (!assumeTrue)
            {
                AddRefinements(ctx, a, false, ov);
                AddRefinements(ctx, b, false, ov);
            }
            // assumeTrue is ambiguous -> no refinement
            return;
        }

        // Comparisons.
        string op;
        Expr left, right;
        if (cond.Match(Tag.Equal, out left, out right) ||
            cond.Match(Tag.NotEqual, out left, out right) ||
            cond.Match(Tag.LessThan, out left, out right) ||
            cond.Match(Tag.LessThanOrEqual, out left, out right) ||
            cond.Match(Tag.GreaterThan, out left, out right) ||
            cond.Match(Tag.GreaterThanOrEqual, out left, out right))
        {
            op = cond.Tag;

            // We only refine when one side is a named variable with __range and the other side is a const int.
            string varName;
            int c;
            bool varOnLeft = false;

            if (left.Match(Tag.Name, out varName) && ctx.Global.TryEvalConstInt(right, out c))
            {
                varOnLeft = true;
            }
            else if (right.Match(Tag.Name, out varName) && ctx.Global.TryEvalConstInt(left, out c))
            {
                varOnLeft = false;
            }
            else
            {
                return;
            }

            // NotEqual can't be represented as a single interval.
            if (op == Tag.NotEqual) return;

            int curMin, curMax;
            if (!TryGetRange(ctx, varName, out curMin, out curMax)) return;

            // If we already refined this var in this override map, start from that (more precise).
            int[] existing;
            if (ov.TryGetValue(varName, out existing))
            {
                curMin = existing[0];
                curMax = existing[1];
            }

            // Normalize to "var <op> const". If var is on the right, invert the operator.
            string normOp = op;
            if (!varOnLeft)
            {
                if (op == Tag.LessThan) normOp = Tag.GreaterThan;
                else if (op == Tag.LessThanOrEqual) normOp = Tag.GreaterThanOrEqual;
                else if (op == Tag.GreaterThan) normOp = Tag.LessThan;
                else if (op == Tag.GreaterThanOrEqual) normOp = Tag.LessThanOrEqual;
            }

            // If we're refining the false branch, invert the operator logically (still interval-safe).
            if (!assumeTrue)
            {
                if (normOp == Tag.LessThan) normOp = Tag.GreaterThanOrEqual;
                else if (normOp == Tag.LessThanOrEqual) normOp = Tag.GreaterThan;
                else if (normOp == Tag.GreaterThan) normOp = Tag.LessThanOrEqual;
                else if (normOp == Tag.GreaterThanOrEqual) normOp = Tag.LessThan;
                else if (normOp == Tag.Equal)
                {
                    // var == c is false -> two intervals; skip.
                    return;
                }
            }

            int newMin = curMin;
            int newMax = curMax;

            if (normOp == Tag.GreaterThan)
            {
                newMin = Math.Max(newMin, c + 1);
            }
            else if (normOp == Tag.GreaterThanOrEqual)
            {
                newMin = Math.Max(newMin, c);
            }
            else if (normOp == Tag.LessThan)
            {
                newMax = Math.Min(newMax, c - 1);
            }
            else if (normOp == Tag.LessThanOrEqual)
            {
                newMax = Math.Min(newMax, c);
            }
            else if (normOp == Tag.Equal)
            {
                newMin = Math.Max(newMin, c);
                newMax = Math.Min(newMax, c);
            }

            if (newMin <= newMax)
            {
                ov[varName] = new int[] { newMin, newMax };
            }
            return;
        }
    }

static Expr LowerIfChain(FunctionCtx ctx, Expr[] parts, FilePosition src)
    {
        return LowerIfAt(ctx, parts, 0, src);
    }

	    static Expr LowerIfAt(FunctionCtx ctx, Expr[] parts, int idx, FilePosition src)
    {
        Expr cond = parts[idx];
        Expr body = parts[idx + 1];

        // Extract any side-effecting prefix from condition lowering.
        List<Expr> prefix = new List<Expr>();
        Expr newCond = LowerExpr(ctx, cond, true, prefix);

        // Flow-sensitive __range refinement inside then/else blocks (conservative subset).
        Dictionary<string, int[]> thenOv = RefineRangesFromCondition(ctx, cond, true);
        Dictionary<string, int[]> elseOv = RefineRangesFromCondition(ctx, cond, false);

        ctx.PushRangeOverride(thenOv);
        Expr newBody = LowerStatementAsSingle(ctx, body);
        ctx.PopRangeOverride(thenOv);

        // Constant/range-backed condition pruning (-O1 style, but always safe when proven).
        // if (0) { ... } else { ... } => else
        // if (1) { ... } else { ... } => then
        // (prefix side-effects from condition are preserved)
        int condInt;
        bool condKnown = newCond.Match(Tag.Integer, out condInt);
        if (!condKnown && prefix.Count == 0)
            condKnown = TryEvalConditionFromRanges(ctx, newCond, out condInt);
        if (condKnown)
        {
            if (condInt != 0)
            {
                if (prefix.Count == 0) return newBody;
                prefix.Add(newBody);
                return MakeSequence(prefix).WithSource(src);
            }
            else
            {
                Expr taken;
                if (idx + 2 >= parts.Length)
                {
                    taken = Expr.Make(Tag.Empty).WithSource(src);
                }
                else
                {
                    ctx.PushRangeOverride(elseOv);
                    taken = LowerIfAt(ctx, parts, idx + 2, src);
                    ctx.PopRangeOverride(elseOv);
                }

                if (prefix.Count == 0) return taken;
                prefix.Add(taken);
                return MakeSequence(prefix).WithSource(src);
            }
        }

        Expr ifStmt;
        if (idx + 2 >= parts.Length)
        {
            ifStmt = Expr.Make(Tag.If, newCond, newBody).WithSource(src);
        }
        else
        {
            ctx.PushRangeOverride(elseOv);
            Expr elseStmt = LowerIfAt(ctx, parts, idx + 2, src);
            ctx.PopRangeOverride(elseOv);

            ifStmt = Expr.Make(Tag.If, newCond, newBody, Expr.Make(Tag.Integer, 1), elseStmt).WithSource(src);
        }

        if (prefix.Count == 0) return ifStmt;
        prefix.Add(ifStmt);
        return MakeSequence(prefix).WithSource(src);
    }



    // ---------- Expression lowering ----------

    static Expr LowerExpr(FunctionCtx ctx, Expr expr, bool allowExtract, List<Expr> prefix)
    {
        // Atoms
        int n;
        string name;
        if (expr.Match(Tag.Integer, out n))
            return expr;
        if (expr.Match(Tag.Name, out name))
        {
            ctx.MarkNameUse(name);
            return expr;
        }

        // Cast
        CType castT;
        Expr sub;
        if (expr.Match(Tag.Cast, out castT, out sub))
        {
            Expr newSub = LowerExpr(ctx, sub, allowExtract, prefix);
            return Expr.Make(Tag.Cast, castT, newSub).WithSource(expr.Source);
        }

        // Ternary
        Expr cond, texpr, fexpr;
        if (expr.Match(Tag.Conditional, out cond, out texpr, out fexpr))
        {
            // cond: no prefix in-expression; but if we are in a statement context we still allowExtract.
            // We'll keep it conservative: do not extract from cond.
            List<Expr> dummy = new List<Expr>();
            Expr newCond = LowerExpr(ctx, cond, false, dummy);
            Expr newT = LowerExpr(ctx, texpr, allowExtract, prefix);
            Expr newF = LowerExpr(ctx, fexpr, allowExtract, prefix);
            return Expr.Make(Tag.Conditional, newCond, newT, newF).WithSource(expr.Source);
        }

        // Call (spill non-atom args)
        Expr callFunc;
        Expr[] callArgs;
        if (expr.MatchAny(Tag.Call, out callFunc, out callArgs))
        {
            string knownCallName;
            if (callFunc != null && callFunc.Match(Tag.Name, out knownCallName) &&
                knownCallName == "__cgb_is_cgb" &&
                callArgs != null && callArgs.Length == 0 &&
                Program.TryGetKnownCgbRuntimeValue(out int knownCgbValue))
            {
                return Expr.Make(Tag.Integer, knownCgbValue).WithSource(expr.Source);
            }

            Expr newFunc = LowerExpr(ctx, callFunc, false, prefix);

            // Optimization (post-A3): when we must spill a non-atomic argument into a temp,
            // prefer a u8 temp if the *parameter type* is u8-like.
            // This preserves semantics (implicit truncation to u8) while avoiding needless
            // u16 temporaries created by integer promotions.
            CType[] paramTypes = null;
            string __fname = null;
            if (newFunc.Match(Tag.Name, out __fname))
                paramTypes = ctx.FindFuncParams(__fname);

            Expr[] newArgs = new Expr[callArgs.Length];
            for (int i = 0; i < callArgs.Length; i++)
            {
                Expr a = LowerExpr(ctx, callArgs[i], allowExtract, prefix);
                bool preserveConstantArgIntrinsic = false;
                if (a.MatchAny(Tag.Call, out Expr argFunc, out Expr[] _argCallArgs) &&
                    argFunc.Match(Tag.Name, out string argFuncName))
                {
                    preserveConstantArgIntrinsic = (argFuncName == "__bankof" || argFuncName == "__cgb_is_cgb");
                }
                bool preserveCompileTimeConstantArg = ctx.Global.TryEvalConstInt(a, out _);
                bool preserveFarcallBankArg = (__fname == "__farcall" && i == 0);

                if (allowExtract && !preserveFarcallBankArg && !preserveConstantArgIntrinsic && !preserveCompileTimeConstantArg && !IsAtom(a))
                {
                    // If we know the callee's parameter type, use it to choose a smaller temp.
                    // Example: (u8)(row*32) passed to a u8 param should spill as u8, not u16.
                    CType spillAs = null;
                    if (paramTypes != null && i < paramTypes.Length)
                    {
                        CType pt = paramTypes[i];
                        if (IsU8Like(pt)) spillAs = CType.UInt8;
                    }
                    if (spillAs == null) spillAs = InferType(ctx, a);
                    if (SizeOfType(spillAs) <= 2)
                        a = SpillToTemp(ctx, a, spillAs, prefix);
                }
                newArgs[i] = a;
            }

            // Lint: const discard for argument passing
            // Warn when passing a const-qualified pointer (or const array decay) to a non-const pointer parameter.
            // Suppress when the argument has an explicit cast, or inside __unsafe.
            if (!ctx.InUnsafe && paramTypes != null)
            {
                for (int i = 0; i < newArgs.Length && i < paramTypes.Length; i++)
                {
                    CType pt = paramTypes[i];
                    if (pt == null) continue;
                    if (!IsPointerToNonConst(pt)) continue;
                    if (HasTopLevelCast(newArgs[i])) continue;

                    CType at = InferType(ctx, newArgs[i]);
                    if (at != null && (IsPointerToConst(at) || IsArrayOfConst(at)))
                    {
                        string fn = __fname ?? "<call>";
                        WarnConstDiscard(expr, $"argument {i} to '{fn}'");
                        break; // keep it quiet: one warning per call is enough
                    }
                }
            }

            if (paramTypes != null)
            {
                string fn = __fname ?? "<call>";
                for (int i = 0; i < newArgs.Length && i < paramTypes.Length; i++)
                {
                    FilePosition argPos = newArgs[i].Source;
                    if (string.IsNullOrEmpty(argPos.Filename) || argPos.Filename == "<unknown>")
                        argPos = expr.Source;
                    WarnImplicitNarrowingIfNeeded(ctx, newArgs[i], InferType(ctx, newArgs[i]), paramTypes[i], argPos,
                        "argument " + i + " to '" + fn + "'");
                }
            }

            // Lint: __enum_strict argument passing.
            if (paramTypes != null)
            {
                string fn = __fname ?? "<call>";
                for (int i = 0; i < newArgs.Length && i < paramTypes.Length; i++)
                {
                    CType pt = paramTypes[i];
                    if (pt == null) continue;

                    CType at = InferType(ctx, newArgs[i]);
                    if (at == null) continue;

                    if (HasTopLevelCast(newArgs[i])) continue;

                    if (IsStrictEnum(pt))
                    {
                        if (!IsSameEnum(pt, at))
                        {
                            Program.Warning(expr.Source, ErrorCode.EnumStrictMix,
                                "argument {0} to '{1}' expects __enum_strict {2}, got {3}; cast explicitly to silence",
                                i, fn, pt.Show(), at.Show());
                        }
                    }
                    else if (IsPlainInteger(pt) && IsStrictEnum(at))
                    {
                        Program.Warning(expr.Source, ErrorCode.EnumStrictMix,
                            "argument {0} to '{1}' passes __enum_strict {2} to integer parameter {3}; cast explicitly to silence",
                            i, fn, at.Show(), pt.Show());
                    }
                }
            }

            if (!ctx.InUnsafe)
                RestrictAliasLint(ctx, newFunc, newArgs, expr.Source);

            // __slice(ptr,len) helper: represent a "slice" (ptr+len) in the IR.
            // Semantics: evaluate both arguments, result type is the pointer/array (same as arg0).
            // Bounds checks are injected only with -Zcheck (see Program.CheckSliceBounds).
            // Note: keep the intrinsic name conservative to avoid collisions ("slice" is allowed only
            // when there is no declared function named "slice").
            if (newFunc.Match(Tag.Name, out string __sfname))
            {
                bool isSliceIntrinsic = (__sfname == "__slice") || (__sfname == "slice" && ctx.FindFuncParams("slice") == null);
                if (isSliceIntrinsic)
                {
                    if (newArgs.Length != 2)
                    {
                        Program.Error(expr.Source, ErrorCode.ParseError, "{0}: expected 2 arguments: __slice(ptr,len)", __sfname);
                    }
                    else
                    {
                        CType pty = InferType(ctx, newArgs[0]);
                        if (pty == null || !(pty.IsPointer || pty.IsArray))
                            Program.Error(expr.Source, ErrorCode.ExpectedType, "{0}: first argument must be a pointer or array, got: {1}", __sfname, (pty == null ? "<unknown>" : pty.Show()));

                        CType lty = InferType(ctx, newArgs[1]);
                        if (lty == null || !lty.IsInteger)
                            Program.Error(expr.Source, ErrorCode.ExpectedType, "{0}: second argument must be an integer length, got: {1}", __sfname, (lty == null ? "<unknown>" : lty.Show()));
                    }

                    return Expr.Make(Tag.Slice, newArgs[0], newArgs[1]).WithSource(expr.Source);
                }
            }

            return MakeCall(newFunc, newArgs).WithSource(expr.Source);
        }

        // Index: spill non-atom index
        Expr left, right;
        if (expr.Match(Tag.Index, out left, out right))
        {
            Expr newLeft = LowerExpr(ctx, left, allowExtract, prefix);
            Expr newRight = LowerExpr(ctx, right, allowExtract, prefix);

            if (allowExtract && !IsAtom(newRight))
            {
                CType rt = InferType(ctx, newRight);
                if (SizeOfType(rt) <= 2)
                    newRight = SpillToTemp(ctx, newRight, rt, prefix);
            }
            newRight = ApplyIndexRangeAndLints(ctx, expr, newLeft, newRight);

            return Expr.Make(Tag.Index, newLeft, newRight).WithSource(expr.Source);
        }

        // Load: spill non-atom pointer
        if (expr.Match(Tag.Load, out sub))
        {
            Expr newPtr = LowerExpr(ctx, sub, allowExtract, prefix);
            if (allowExtract && !IsAtom(newPtr))
            {
                newPtr = SpillToTemp(ctx, newPtr, PointerTempType(ctx, newPtr), prefix);
            }
            return Expr.Make(Tag.Load, newPtr).WithSource(expr.Source);
        }


        // Field: allow lowering inside Index/Load
        string field;
        if (expr.Match(Tag.Field, out sub, out field))
        {
            // Struct-by-value locals are desugared into per-field locals...
            string baseName;
            string fieldVar;
            if (sub.Match(Tag.Name, out baseName) && ctx.TryGetStructLocalFieldVar(baseName, field, out fieldVar))
            {
                ctx.MarkNameUse(fieldVar);
                return Expr.Make(Tag.Name, fieldVar).WithSource(expr.Source);
            }

            Expr newBase = LowerExpr(ctx, sub, allowExtract, prefix);

            // Make sure nested index/load bases are atomized.
            Expr idxArr, idxExpr;
            Expr loadPtr;
            if (allowExtract && newBase.Match(Tag.Index, out idxArr, out idxExpr))
            {
                List<Expr> innerPrefix = new List<Expr>();
                Expr newIdx = LowerExpr(ctx, idxExpr, allowExtract, innerPrefix);
                for (int i = 0; i < innerPrefix.Count; i++) prefix.Add(innerPrefix[i]);
                if (!IsAtom(newIdx))
                {
                    CType it = InferType(ctx, newIdx);
                    if (SizeOfType(it) <= 2) newIdx = SpillToTemp(ctx, newIdx, it, prefix);
                }
                newBase = Expr.Make(Tag.Index, idxArr, newIdx).WithSource(newBase.Source);
            }
            else if (allowExtract && newBase.Match(Tag.Load, out loadPtr))
            {
                Expr newPtr = LowerExpr(ctx, loadPtr, allowExtract, prefix);
                if (!IsAtom(newPtr)) newPtr = SpillToTemp(ctx, newPtr, PointerTempType(ctx, newPtr), prefix);
                newBase = Expr.Make(Tag.Load, newPtr).WithSource(newBase.Source);
            }

            return Expr.Make(Tag.Field, newBase, field).WithSource(expr.Source);
        }

        // Assign / AssignModify
        Expr lval, rval;
        if (expr.Match(Tag.Assign, out lval, out rval))
        {
            Expr newL = LowerLValue(ctx, lval, allowExtract, prefix);
            Expr newR = LowerExpr(ctx, rval, allowExtract, prefix);

            // Semantic: assignment to const-qualified object is not allowed.
            CType lt = InferType(ctx, newL);
            if (lt != null && lt.IsConst)
            {
                Program.Error(Maybe.Just(expr.Source), ErrorCode.ConstModify,
                    "cannot modify const-qualified object");
            }

            // Lint: __enum_strict - warn on mixing strict enums with integers or other enums.
            CType rt = InferType(ctx, newR);
            WarnImplicitNarrowingIfNeeded(ctx, newR, rt, lt, expr.Source, "assignment");

            // Lint: const discard (pointer conversions)
            // Example: u8* p = table; where table is const u8[] or const u8*.
            // Suppress when the user wrote an explicit cast, or inside __unsafe.
            if (!ctx.InUnsafe && lt != null && rt != null && IsPointerToNonConst(lt) && (IsPointerToConst(rt) || IsArrayOfConst(rt)) && !HasTopLevelCast(newR))
            {
                WarnConstDiscard(expr, "assignment");
            }

            if (IsStrictEnum(lt))
            {
                if (!IsSameEnum(lt, rt))
                {
                    Program.Warning(expr.Source, ErrorCode.EnumStrictMix,
                        "assigning value of type {0} to __enum_strict {1}; cast explicitly to silence",
                        rt == null ? "<unknown>" : rt.Show(), lt.Show());
                }
            }
            else if (IsPlainInteger(lt) && IsStrictEnum(rt))
            {
                Program.Warning(expr.Source, ErrorCode.EnumStrictMix,
                    "assigning __enum_strict {0} to integer type {1}; cast explicitly to silence",
                    rt.Show(), lt.Show());
            }

            // Lint: __bitflags - discourage mixing with plain integers.
            if (IsBitFlags(lt) && !IsBitFlags(rt))
            {
                Program.Warning(expr.Source, ErrorCode.BitFlagsMix,
                    "assigning value of type {0} to __bitflags {1}; cast explicitly to silence",
                    rt == null ? "<unknown>" : rt.Show(), lt.Show());
            }
            else if (IsPlainInteger(lt) && IsBitFlags(rt))
            {
                Program.Warning(expr.Source, ErrorCode.BitFlagsMix,
                    "assigning __bitflags {0} to integer type {1}; cast explicitly to silence",
                    rt.Show(), lt.Show());
            }

            
            // Lint: __safe_index - discourage mixing index-typed integers with plain integers.
            // Allowed without warning:
            // - assigning a constant integer literal
            // - assigning from another __safe_index
            if (IsSafeIndex(lt) && !IsSafeIndex(rt))
            {
                int __v;
                if (!TryConstInt(newR, out __v))
                {
                    Program.Warning(expr.Source, ErrorCode.SafeIndexMix,
                        "assigning value of type {0} to __safe_index {1}; cast explicitly to silence",
                        rt == null ? "<unknown>" : rt.Show(), lt.Show());
                }
            }
            else if (IsPlainInteger(lt) && IsSafeIndex(rt))
            {
                Program.Warning(expr.Source, ErrorCode.SafeIndexMix,
                    "assigning __safe_index {0} to integer type {1}; cast explicitly to silence",
                    rt.Show(), lt.Show());
            }

// Lint: __range(min,max) variable assigned an out-of-range constant.
            string __rname;
            if (newL.Match(Tag.Name, out __rname))
            {
                int __min, __max, __v;
                if (TryGetRange(ctx, __rname, out __min, out __max) && TryConstInt(newR, out __v))
                {
                    if (__v < __min || __v > __max)
                    {
                        Program.Warning(expr.Source, ErrorCode.RangeViolation,
                            "value {0} is outside declared range [{1},{2}] for '{3}'", __v, __min, __max, __rname);
                    }
                }
            }
            return Expr.Make(Tag.Assign, newL, newR).WithSource(expr.Source);
        }

        string opName;
        if (expr.Match(Tag.AssignModify, out opName, out lval, out rval))
        {
            Expr newL = LowerLValue(ctx, lval, allowExtract, prefix);
            Expr newR = LowerExpr(ctx, rval, allowExtract, prefix);

            CType lt = InferType(ctx, newL);
            if (lt != null && lt.IsConst)
            {
                Program.Error(Maybe.Just(expr.Source), ErrorCode.ConstModify,
                    "cannot modify const-qualified object");
            }

            // Lint: __enum_strict - compound assignments imply integer ops.
            CType rt = InferType(ctx, newR);
            Expr widened = Expr.Make(opName, newL, newR).WithSource(expr.Source);
            WarnImplicitNarrowingIfNeeded(ctx, newR, InferType(ctx, widened), lt, expr.Source, "compound assignment");
            if (IsStrictEnum(lt))
            {
                if (!IsSameEnum(lt, rt))
                {
                    Program.Warning(expr.Source, ErrorCode.EnumStrictMix,
                        "compound assignment on __enum_strict {0} with operand type {1}; cast explicitly to silence",
                        lt.Show(), rt == null ? "<unknown>" : rt.Show());
                }
                else
                {
                    Program.Warning(expr.Source, ErrorCode.EnumStrictMix,
                        "compound assignment on __enum_strict {0}; cast explicitly to silence",
                        lt.Show());
                }
            }
            else if (IsPlainInteger(lt) && IsStrictEnum(rt))
            {
                Program.Warning(expr.Source, ErrorCode.EnumStrictMix,
                    "compound assignment mixes __enum_strict {0} into integer type {1}; cast explicitly to silence",
                    rt.Show(), lt.Show());
            }

            // Lint: __bitflags - only allow bitwise/shift compound ops and discourage mixing.
            if (IsBitFlags(lt))
            {
                bool okOp = (opName == Tag.BitwiseAnd || opName == Tag.BitwiseOr || opName == Tag.BitwiseXor ||
                            opName == Tag.ShiftLeft || opName == Tag.ShiftRight);
                if (!okOp)
                {
                    Program.Warning(expr.Source, ErrorCode.BitFlagsOp,
                        "compound op '{0}' on __bitflags {1} is discouraged; use bitwise ops or cast explicitly",
                        opName, lt.Show());
                }
                else if (!IsBitFlags(rt))
                {
                    Program.Warning(expr.Source, ErrorCode.BitFlagsMix,
                        "mixing __bitflags {0} with operand type {1} in '{2}'; cast explicitly to silence",
                        lt.Show(), rt == null ? "<unknown>" : rt.Show(), opName);
                }
            }
            else if (IsPlainInteger(lt) && IsBitFlags(rt))
            {
                Program.Warning(expr.Source, ErrorCode.BitFlagsMix,
                    "compound assignment mixes __bitflags {0} into integer type {1}; cast explicitly to silence",
                    rt.Show(), lt.Show());
            }

            // Lint: __range(min,max) variable assigned an out-of-range constant.
            string __rname;
            if (newL.Match(Tag.Name, out __rname))
            {
                int __min, __max, __v;
                if (TryGetRange(ctx, __rname, out __min, out __max) && TryConstInt(newR, out __v))
                {
                    if (__v < __min || __v > __max)
                    {
                        Program.Warning(expr.Source, ErrorCode.RangeViolation,
                            "value {0} is outside declared range [{1},{2}] for '{3}'", __v, __min, __max, __rname);
                    }
                }
            }

            // Lint: __safe_index - compound assignments imply arithmetic on indices.
            // Allow += / -= with constant literals; otherwise require explicit casts.
            if (IsSafeIndex(lt) && !IsSafeIndex(rt))
            {
                int __v;
                if (!TryConstInt(newR, out __v))
                {
                    Program.Warning(expr.Source, ErrorCode.SafeIndexMix,
                        "compound op '{0}' on __safe_index {1} with operand type {2}; cast explicitly to silence",
                        opName, lt.Show(), rt == null ? "<unknown>" : rt.Show());
                }
            }
            else if (IsPlainInteger(lt) && IsSafeIndex(rt))
            {
                Program.Warning(expr.Source, ErrorCode.SafeIndexMix,
                    "compound assignment mixes __safe_index {0} into integer type {1}; cast explicitly to silence",
                    rt.Show(), lt.Show());
            }

            return Expr.Make(Tag.AssignModify, opName, newL, newR).WithSource(expr.Source);
        }

	        // Pre/Post inc/dec: lower sub
        if (expr.Match(Tag.PostIncrement, out sub) || expr.Match(Tag.PostDecrement, out sub) ||
            expr.Match(Tag.PreIncrement, out sub) || expr.Match(Tag.PreDecrement, out sub))
        {
            string tag = expr.GetTag();
            Expr newSub = LowerLValue(ctx, sub, allowExtract, prefix);

            CType st = InferType(ctx, newSub);
            if (st != null && st.IsConst)
            {
                Program.Error(Maybe.Just(expr.Source), ErrorCode.ConstModify,
                    "cannot modify const-qualified object");
            }

            if (IsStrictEnum(st))
            {
                Program.Warning(expr.Source, ErrorCode.EnumStrictMix,
                    "increment/decrement on __enum_strict {0}; cast explicitly to silence", st.Show());
            }
            if (IsBitFlags(st))
            {
                Program.Warning(expr.Source, ErrorCode.BitFlagsOp,
                    "increment/decrement on __bitflags {0} is discouraged; cast explicitly to silence", st.Show());
            }
            return Expr.Make(tag, newSub).WithSource(expr.Source);
        }

	        // AddressOf: operand is an lvalue
	        if (expr.Match(Tag.AddressOf, out sub))
	        {
	            Expr newSub = LowerLValue(ctx, sub, allowExtract, prefix);
	            return Expr.Make(Tag.AddressOf, newSub).WithSource(expr.Source);
	        }

        // Variable shift lowering (x << n) / (x >> n)
        if (expr.Match(Tag.ShiftLeft, out left, out right) || expr.Match(Tag.ShiftRight, out left, out right))
        {
            string shiftTag = expr.GetTag();

            Expr newLeft = LowerExpr(ctx, left, allowExtract, prefix);
            Expr newRight = LowerExpr(ctx, right, allowExtract, prefix);

            // Constant shift is fine.
            if (newRight.Match(Tag.Integer, out n))
                return Expr.Make(shiftTag, newLeft, newRight).WithSource(expr.Source);

            // Variable shift is not supported by codegen; lower to loop when possible.
            if (allowExtract)
            {
                CType lt = InferType(ctx, newLeft);
                int sz = SizeOfType(lt);

                // tmpVal = left
                string tmpVal = ctx.AcquireTemp(sz == 1 ? CType.UInt8 : CType.UInt16);
                Expr tmpValName = Expr.Make(Tag.Name, tmpVal).WithSource(expr.Source);
                prefix.Add(Expr.Make(Tag.Assign, tmpValName, newLeft).WithSource(expr.Source));

                // tmpCnt = right (count treated as u8)
                string tmpCnt = ctx.AcquireTemp(CType.UInt8);
                Expr tmpCntName = Expr.Make(Tag.Name, tmpCnt).WithSource(expr.Source);
                prefix.Add(Expr.Make(Tag.Assign, tmpCntName, newRight).WithSource(expr.Source));

                // body: tmpVal = tmpVal << 1; tmpCnt = tmpCnt - 1;
                Expr one = Expr.Make(Tag.Integer, 1).WithSource(expr.Source);

                Expr shiftOnce = Expr.Make(Tag.Assign, tmpValName,
                    Expr.Make(shiftTag, tmpValName, one)).WithSource(expr.Source);

                Expr dec = Expr.Make(Tag.Assign, tmpCntName,
                    Expr.Make(Tag.Subtract, tmpCntName, one)).WithSource(expr.Source);

                Expr loopBody = MakeSequence(new List<Expr> { shiftOnce, dec }).WithSource(expr.Source);

                Expr loop = Expr.Make(Tag.For,
                    Expr.Make(Tag.Empty),
                    tmpCntName,
                    Expr.Make(Tag.Empty),
                    loopBody).WithSource(expr.Source);

                prefix.Add(loop);

                return tmpValName;
            }

            // No extraction allowed: keep as-is (will likely error later, but we cannot inject stmts here).
            return Expr.Make(shiftTag, newLeft, newRight).WithSource(expr.Source);
        }

	        // Generic unary ops
	        if (expr.Match(Tag.BitwiseNot, out sub) || expr.Match(Tag.LogicalNot, out sub))
        {
            string tag = expr.GetTag();
            Expr newSub = LowerExpr(ctx, sub, allowExtract, prefix);
            return Expr.Make(tag, newSub).WithSource(expr.Source);
        }

	        // Generic binary ops (lower children; do not spill by default)
	        string binTag;
	        if (expr.MatchAnyTag(out binTag, out left, out right) && IsBinaryOpTag(binTag))
	        {
            Expr newL = LowerExpr(ctx, left, allowExtract, prefix);
            Expr newR = LowerExpr(ctx, right, allowExtract, prefix);

            // C1: Strength-reduce constant multiplication.
            // - x * (2^k) => x << k
            // - (2^k) * x => x << k (safe because the literal has no side effects)
            // This keeps A3 correctness (promotions stay) but avoids heavier mul paths,
            // and enables the fast shift code paths for k>=8.
            if (binTag == Tag.Multiply)
            {
                // Canonicalize: if left is a literal constant and right isn't, move const to RHS.
                if (TryConstInt(newL, out int __lc) && !TryConstInt(newR, out _))
                {
                    var __tmp = newL; newL = newR; newR = __tmp;
                }

                if (TryConstInt(newR, out int __rc))
                {
                    // Only optimize well-defined 16-bit factors (0 < c < 65536).
                    // (We intentionally avoid wraparound rules for larger literals.)
                    if (__rc == 1)
                        return newL.WithSource(expr.Source);

                    if (__rc > 0 && __rc < 65536 && IsPow2(__rc))
                    {
                        int __sh = Log2Pow2(__rc);
                        return Expr.Make(Tag.ShiftLeft, newL,
                                Expr.Make(Tag.Integer, __sh).WithSource(expr.Source))
                            .WithSource(expr.Source);
                    }
                }
            }

            // C1b: Strength-reduce division/modulus by powers of two when the dividend
            // is known non-negative. This preserves C semantics for the supported cases:
            // - unsigned values
            // - signed values proven >= 0 by range analysis or constants
            if (binTag == Tag.Divide || binTag == Tag.Modulus)
            {
                if (TryConstInt(newR, out int __divc))
                {
                    if (binTag == Tag.Divide && __divc == 1)
                        return newL.WithSource(expr.Source);

                    if (__divc > 0 && __divc < 65536 && IsPow2(__divc) && IsKnownNonNegativeExpr(ctx, newL))
                    {
                        int __sh = Log2Pow2(__divc);
                        if (binTag == Tag.Divide)
                        {
                            return Expr.Make(Tag.ShiftRight, newL,
                                    Expr.Make(Tag.Integer, __sh).WithSource(expr.Source))
                                .WithSource(expr.Source);
                        }

                        return Expr.Make(Tag.BitwiseAnd, newL,
                                Expr.Make(Tag.Integer, __divc - 1).WithSource(expr.Source))
                            .WithSource(expr.Source);
                    }
                }
            }

            // For comparisons, it helps a lot if each side is an atom, because the backend's
            // 16-bit compare path is intentionally restrictive.
	            if (allowExtract && IsComparisonTag(binTag))
            {
                if (!IsAtom(newL)) newL = SpillToTemp(ctx, newL, InferType(ctx, newL), prefix);
                if (!IsAtom(newR)) newR = SpillToTemp(ctx, newR, InferType(ctx, newR), prefix);
            }

	            
            // Lint: __enum_strict - warn on mixing strict enums with integers or other enums.
            CType lt = InferType(ctx, newL);
            CType rt = InferType(ctx, newR);
            WarnDangerousPointerArithmetic(binTag, newL, newR, lt, rt, expr.Source);
            if (IsStrictEnum(lt) || IsStrictEnum(rt))
            {
                if (!(IsSameEnum(lt, rt)))
                {
                    // Allow explicit cast to integer or to the same enum to silence.
                    Program.Warning(expr.Source, ErrorCode.EnumStrictMix,
                        "mixing __enum_strict enums with other types in '{0}': {1} vs {2}; cast explicitly to silence",
                        binTag,
                        lt == null ? "<unknown>" : lt.Show(),
                        rt == null ? "<unknown>" : rt.Show());
                }
            }

            // Lint: __bitflags - prefer bitwise operations and discourage arithmetic.
            if (IsBitFlags(lt) || IsBitFlags(rt))
            {
                bool allowed = (binTag == Tag.BitwiseAnd || binTag == Tag.BitwiseOr || binTag == Tag.BitwiseXor ||
                                binTag == Tag.ShiftLeft || binTag == Tag.ShiftRight ||
                                binTag == Tag.Equal || binTag == Tag.NotEqual);
                if (!allowed)
                {
                    Program.Warning(expr.Source, ErrorCode.BitFlagsOp,
                        "operation '{0}' on __bitflags types is discouraged; use bitwise ops or cast explicitly",
                        binTag);
                }
                else if (!(IsBitFlags(lt) && IsBitFlags(rt)))
                {
                    Program.Warning(expr.Source, ErrorCode.BitFlagsMix,
                        "mixing __bitflags with other types in '{0}': {1} vs {2}; cast explicitly to silence",
                        binTag,
                        lt == null ? "<unknown>" : lt.Show(),
                        rt == null ? "<unknown>" : rt.Show());
                }
            }


            // Lint: __safe_index - discourage mixing index-typed integers with plain integers in expressions.
            // Allow common patterns: idx +/- literal, idx compared with literal.
            if (IsSafeIndex(lt) || IsSafeIndex(rt))
            {
                bool lSafe = IsSafeIndex(lt);
                bool rSafe = IsSafeIndex(rt);
                bool lLit = newL.Match(Tag.Integer, out int __li);
                bool rLit = newR.Match(Tag.Integer, out int __ri);

                bool ok = false;
                if (IsComparisonTag(binTag))
                {
                    ok = (lSafe && (rSafe || rLit)) || (rSafe && (lSafe || lLit));
                }
                else if (binTag == Tag.Add || binTag == Tag.Subtract)
                {
                    ok = (lSafe && (rSafe || rLit)) || (rSafe && (lSafe || lLit));
                }

                if (!ok)
                {
                    Program.Warning(expr.Source, ErrorCode.SafeIndexMix,
                        "mixing __safe_index with other types in '{0}': {1} vs {2}; cast explicitly to silence",
                        binTag,
                        lt == null ? "<unknown>" : lt.Show(),
                        rt == null ? "<unknown>" : rt.Show());
                }
            }


            // C2: Ensure mixed-width comparisons are evaluated at a single, stable width.
            // If either side is 16-bit (u16 / pointer), zero-extend the other side to u16
            // so backend compare selection is consistent across 'if' and value contexts.
            if (IsComparisonTag(binTag))
            {
                int __lsz = SizeOfType(lt);
                int __rsz = SizeOfType(rt);
                if (__lsz == 2 || __rsz == 2)
                {
                    if (__lsz == 1) newL = Expr.Make(Tag.Cast, CType.UInt16, newL).WithSource(expr.Source);
                    if (__rsz == 1) newR = Expr.Make(Tag.Cast, CType.UInt16, newR).WithSource(expr.Source);
                }
            }

            return Expr.Make(binTag, newL, newR).WithSource(expr.Source);
        }

        // Default: try to recursively lower all Expr children if it's an Expr-only tag.
        // If we can't recognize the shape, keep it to avoid breaking the compiler.
        return expr;
    }

    static bool IsLValueLikeExpr(Expr e)
    {
        if (e == null) return false;
        return e.MatchTag(Tag.Name) || e.MatchTag(Tag.Load) || e.MatchTag(Tag.Index) || e.MatchTag(Tag.Field);
    }

    static Expr LowerLValue(FunctionCtx ctx, Expr lval, bool allowExtract, List<Expr> prefix)
    {
        // Name lvalue
        string name;
        if (lval.Match(Tag.Name, out name))
        {
            // Compile-time constants ($constant / enum values) have no addressable storage.
            // In ROM-materialized mode, allow taking the address of const scalar objects.
            if (ctx.Global.ConstTypes.TryGetValue(name, out CType ct) && !IsConstScalarInRomType(ct))
            {
                Program.Error(lval.Source, ErrorCode.ConstAssign, $"cannot use constant '{name}' as an lvalue or take its address");
            }
            ctx.MarkNameUse(name);
            return lval;
        }

        // *ptr lvalue
        Expr sub;
        if (lval.Match(Tag.Load, out sub))
        {
            Expr newPtr = LowerExpr(ctx, sub, allowExtract, prefix);
            if (allowExtract && !IsAtom(newPtr))
                newPtr = SpillToTemp(ctx, newPtr, PointerTempType(ctx, newPtr), prefix);
            return Expr.Make(Tag.Load, newPtr).WithSource(lval.Source);
        }

        // arr[idx] lvalue: normalize nested lvalue-like bases and atomize idx.
        Expr arr, idx;
        if (lval.Match(Tag.Index, out arr, out idx))
        {
            Expr newArr = IsLValueLikeExpr(arr)
                ? LowerLValue(ctx, arr, allowExtract, prefix)
                : LowerExpr(ctx, arr, allowExtract, prefix);
            Expr newIdx = LowerExpr(ctx, idx, allowExtract, prefix);
            if (allowExtract && !IsAtom(newIdx))
            {
                CType it = InferType(ctx, newIdx);
                if (SizeOfType(it) <= 2) newIdx = SpillToTemp(ctx, newIdx, it, prefix);
            }
            newIdx = ApplyIndexRangeAndLints(ctx, lval, newArr, newIdx);
            return Expr.Make(Tag.Index, newArr, newIdx).WithSource(lval.Source);
        }

        // Field lvalue: supported for Index(...), Load(...), and desugared struct locals
        string fieldName;
        Expr baseExpr;
        if (lval.Match(Tag.Field, out baseExpr, out fieldName))
        {
            // Desugared local struct: v.x -> v__x
            string baseName;
            string fieldVar;
            if (baseExpr.Match(Tag.Name, out baseName) && ctx.TryGetStructLocalFieldVar(baseName, fieldName, out fieldVar))
            {
                ctx.MarkNameUse(fieldVar);
                return Expr.Make(Tag.Name, fieldVar).WithSource(lval.Source);
            }

            Expr newBase = IsLValueLikeExpr(baseExpr)
                ? LowerLValue(ctx, baseExpr, allowExtract, prefix)
                : LowerExpr(ctx, baseExpr, allowExtract, prefix);

            // If base is Load(ptr), atomize ptr.
            Expr ptr;
            if (allowExtract && newBase.Match(Tag.Load, out ptr))
            {
                Expr newPtr = LowerExpr(ctx, ptr, allowExtract, prefix);
                if (!IsAtom(newPtr)) newPtr = SpillToTemp(ctx, newPtr, PointerTempType(ctx, newPtr), prefix);
                newBase = Expr.Make(Tag.Load, newPtr).WithSource(newBase.Source);
            }

            // If base is Index(arr, idx), atomize idx.
            Expr arr2, idx2;
            if (allowExtract && newBase.Match(Tag.Index, out arr2, out idx2))
            {
                Expr newIdx = LowerExpr(ctx, idx2, allowExtract, prefix);
                if (!IsAtom(newIdx))
                {
                    CType it = InferType(ctx, newIdx);
                    if (SizeOfType(it) <= 2) newIdx = SpillToTemp(ctx, newIdx, it, prefix);
                }
                newBase = Expr.Make(Tag.Index, arr2, newIdx).WithSource(newBase.Source);
            }

            return Expr.Make(Tag.Field, newBase, fieldName).WithSource(lval.Source);
        }

        return lval;
    }

    // ---------- Helpers ----------

    static bool IsInternalOrGeneratedName(string name)
    {
        if (string.IsNullOrEmpty(name)) return true;
        if (name == "_") return true;
        if (name.StartsWith("__t8_") || name.StartsWith("__t16_")) return true;
        if (name.StartsWith("__")) return true;
        return false;
    }

    static Maybe<FilePosition> ToMaybePos(FilePosition pos)
    {
        if (string.IsNullOrEmpty(pos.Filename) || pos.Filename == "<unknown>") return Maybe.Nothing;
        return Maybe.Just(pos);
    }

    static void EmitUnusedLocalAndParamWarnings(FunctionCtx ctx)
    {
        if (ctx == null) return;

        foreach (KeyValuePair<string, int> kv in ctx.LocalUseCounts)
        {
            string name = kv.Key;
            if (kv.Value != 0) continue;
            if (IsInternalOrGeneratedName(name)) continue;

            FilePosition p;
            Maybe<FilePosition> pos = ctx.LocalDeclPos.TryGetValue(name, out p) ? ToMaybePos(p) : Maybe.Nothing;
            Program.Warning(pos, ErrorCode.UnusedSymbol,
                "unused local variable '{0}' in function '{1}'",
                name, ctx.FunctionName ?? "");
        }

        foreach (KeyValuePair<string, int> kv in ctx.ParamUseCounts)
        {
            string name = kv.Key;
            if (kv.Value != 0) continue;
            if (IsInternalOrGeneratedName(name)) continue;

            FilePosition p;
            Maybe<FilePosition> pos = ctx.ParamDeclPos.TryGetValue(name, out p) ? ToMaybePos(p) : Maybe.Nothing;
            Program.Warning(pos, ErrorCode.UnusedSymbol,
                "unused parameter '{0}' in function '{1}'",
                name, ctx.FunctionName ?? "");
        }
    }

    static Expr UnwrapDeclWrappers(Expr d)
    {
        while (d != null)
        {
            Expr inner;
            int bankNo;
            if (d.Match(Tag.Unsafe, out inner))
            {
                d = inner.WithSource(d.Source);
                continue;
            }
            if (d.Match(Tag.Static, out inner))
            {
                d = inner.WithSource(d.Source);
                continue;
            }
            if (d.Match(Tag.Bank, out bankNo, out inner))
            {
                d = inner.WithSource(d.Source);
                continue;
            }
            if (d.Match(Tag.FixedBank, out bankNo, out inner))
            {
                d = inner.WithSource(d.Source);
                continue;
            }
            if (d.Match(Tag.FixedOrder, out bankNo, out inner))
            {
                d = inner.WithSource(d.Source);
                continue;
            }
            break;
        }
        return d;
    }

    static void CollectNameUses(Expr expr, HashSet<string> used)
    {
        if (expr == null || used == null) return;

        string name;
        if (expr.Match(Tag.Name, out name) && !string.IsNullOrEmpty(name))
            used.Add(name);

        object[] args = expr.GetArgs();
        for (int i = 1; i < args.Length; i++)
        {
            object a = args[i];
            Expr sub = a as Expr;
            if (sub != null)
            {
                CollectNameUses(sub, used);
                continue;
            }

            Expr[] many = a as Expr[];
            if (many != null)
            {
                for (int j = 0; j < many.Length; j++)
                    CollectNameUses(many[j], used);
            }
        }
    }

    static void WarnUnusedGlobalSymbols(List<Expr> loweredDecls)
    {
        if (loweredDecls == null || loweredDecls.Count == 0) return;

        Dictionary<string, FilePosition> globalDefs = new Dictionary<string, FilePosition>();
        Dictionary<string, FilePosition> functionDefs = new Dictionary<string, FilePosition>();
        HashSet<string> usedNames = new HashSet<string>();

        for (int i = 0; i < loweredDecls.Count; i++)
        {
            Expr d = UnwrapDeclWrappers(loweredDecls[i]);
            if (d == null) continue;

            CType ret;
            string fname;
            FieldInfo[] ps;
            Expr body;
            int mc = 0;
            if (d.Match(Tag.Function, out ret, out fname, out ps, out mc, out body) ||
                (mc = 0) == 0 && d.Match(Tag.Function, out ret, out fname, out ps, out body) ||
                d.Match(Tag.InlineFunction, out ret, out fname, out ps, out mc, out body) ||
                (mc = 0) == 0 && d.Match(Tag.InlineFunction, out ret, out fname, out ps, out body))
            {
                if (!functionDefs.ContainsKey(fname))
                    functionDefs.Add(fname, d.Source);
                CollectNameUses(body, usedNames);
                continue;
            }

            MemoryRegion region;
            CType vt;
            string vname;
            Expr rangeExpr;
            if (d.Match(Tag.Variable, out region, out vt, out vname, out rangeExpr) ||
                d.Match(Tag.Variable, out region, out vt, out vname))
            {
                if (!globalDefs.ContainsKey(vname))
                    globalDefs.Add(vname, d.Source);
                if (rangeExpr != null) CollectNameUses(rangeExpr, usedNames);
                continue;
            }

            CType ct;
            string cname;
            Expr cval;
            if (d.Match(Tag.Constant, out ct, out cname, out cval))
            {
                CollectNameUses(cval, usedNames);
                continue;
            }
        }

        foreach (KeyValuePair<string, FilePosition> kv in globalDefs)
        {
            string name = kv.Key;
            if (IsInternalOrGeneratedName(name)) continue;
            if (usedNames.Contains(name)) continue;
            Program.Warning(ToMaybePos(kv.Value), ErrorCode.UnusedSymbol,
                "unused global variable '{0}'", name);
        }

        foreach (KeyValuePair<string, FilePosition> kv in functionDefs)
        {
            string name = kv.Key;
            if (name == "main") continue;
            if (IsInternalOrGeneratedName(name)) continue;
            if (usedNames.Contains(name)) continue;
            Program.Warning(ToMaybePos(kv.Value), ErrorCode.UnusedSymbol,
                "unused function '{0}'", name);
        }
    }

    static bool EndsControlFlowLowered(List<Expr> lowered)
    {
        if (lowered == null || lowered.Count == 0) return false;
        return EndsControlFlowStmt(lowered[lowered.Count - 1]);
    }

    static bool EndsControlFlowStmt(Expr stmt)
    {
        if (stmt == null) return false;

        Expr __retExpr;
        if (stmt.Match(Tag.Return) || stmt.Match(Tag.Return, out __retExpr) ||
            stmt.Match(Tag.Jump, out string _) || stmt.Match(Tag.Break) ||
            stmt.Match(Tag.Continue) || stmt.Match(Tag.Fallthrough))
        {
            return true;
        }

        Expr unsafeInner;
        if (stmt.Match(Tag.Unsafe, out unsafeInner))
            return EndsControlFlowStmt(unsafeInner);

        Expr[] seq;
        if (stmt.MatchAny(Tag.Sequence, out seq))
        {
            if (seq.Length == 0) return false;
            return EndsControlFlowStmt(seq[seq.Length - 1]);
        }

        Expr[] parts;
        if (stmt.MatchAny(Tag.If, out parts))
        {
            if (parts.Length < 4) return false;

            int condLast;
            if (!parts[parts.Length - 2].Match(Tag.Integer, out condLast) || condLast == 0)
                return false;

            for (int i = 1; i < parts.Length; i += 2)
            {
                if (!EndsControlFlowStmt(parts[i])) return false;
            }
            return true;
        }

        return false;
    }

    static bool TryGetNarrowingTypeInfo(CType t, out int bits, out bool isSigned)
    {
        bits = 0;
        isSigned = false;
        if (t == null) return false;
        t = t.WithoutConst();

        if (t.Tag == CTypeTag.Pointer)
        {
            bits = 16;
            isSigned = false;
            return true;
        }
        if (t.Tag == CTypeTag.Enum)
        {
            bits = 16;
            isSigned = false;
            return true;
        }
        if (t.Tag != CTypeTag.Simple) return false;

        if (t.SimpleType == CSimpleType.UInt8) { bits = 8; isSigned = false; return true; }
        if (t.SimpleType == CSimpleType.Int8) { bits = 8; isSigned = true; return true; }
        if (t.SimpleType == CSimpleType.UInt16) { bits = 16; isSigned = false; return true; }
        if (t.SimpleType == CSimpleType.Int16) { bits = 16; isSigned = true; return true; }

        return false;
    }

    static bool ValueFitsInType(int value, int bits, bool isSigned)
    {
        long v = value;
        long min, max;
        if (isSigned)
        {
            min = -(1L << (bits - 1));
            max = (1L << (bits - 1)) - 1;
        }
        else
        {
            min = 0;
            max = (1L << bits) - 1;
        }
        return v >= min && v <= max;
    }

    static bool TryEvalIntForNarrowing(FunctionCtx ctx, Expr e, out int value)
    {
        if (TryConstInt(e, out value)) return true;
        if (ctx != null && ctx.Global != null && ctx.Global.TryEvalConstInt(e, out value)) return true;
        value = 0;
        return false;
    }

    static void WarnImplicitNarrowingIfNeeded(
        FunctionCtx ctx,
        Expr valueExpr,
        CType srcType,
        CType dstType,
        FilePosition source,
        string context)
    {
        if (valueExpr == null || srcType == null || dstType == null) return;
        if (HasTopLevelCast(valueExpr)) return;

        int srcBits, dstBits;
        bool srcSigned, dstSigned;
        if (!TryGetNarrowingTypeInfo(srcType, out srcBits, out srcSigned)) return;
        if (!TryGetNarrowingTypeInfo(dstType, out dstBits, out dstSigned)) return;

        bool narrowingByWidth = srcBits > dstBits;
        bool narrowingBySign = (srcBits >= dstBits) && srcSigned && !dstSigned;
        bool narrowingUnsignedToSigned = (srcBits == dstBits) && !srcSigned && dstSigned;

        if (!(narrowingByWidth || narrowingBySign || narrowingUnsignedToSigned)) return;

        int c;
        if (TryEvalIntForNarrowing(ctx, valueExpr, out c) && ValueFitsInType(c, dstBits, dstSigned))
            return;

        Program.Warning(source, ErrorCode.ImplicitNarrowing,
            "implicit narrowing in {0}: {1} -> {2}; cast explicitly to silence",
            string.IsNullOrEmpty(context) ? "conversion" : context,
            srcType.Show(), dstType.Show());
    }

    static bool IsPointerLike(CType t)
    {
        if (t == null) return false;
        return t.WithoutConst().Tag == CTypeTag.Pointer;
    }

    static void WarnDangerousPointerArithmetic(string binTag, Expr leftExpr, Expr rightExpr, CType leftType, CType rightType, FilePosition source)
    {
        if (binTag != Tag.Add && binTag != Tag.Subtract) return;

        bool leftPtr = IsPointerLike(leftType);
        bool rightPtr = IsPointerLike(rightType);
        if (!leftPtr && !rightPtr) return;

        if (leftPtr && rightPtr)
        {
            Program.Warning(source, ErrorCode.PointerArithmeticDanger,
                "pointer arithmetic between two pointers in '{0}' may be unsafe", binTag);
            return;
        }

        if (binTag == Tag.Subtract && !leftPtr && rightPtr)
        {
            Program.Warning(source, ErrorCode.PointerArithmeticDanger,
                "subtracting a pointer from an integer may be unsafe");
            return;
        }

        CType ptrType = leftPtr ? leftType : rightType;
        Expr offsetExpr = leftPtr ? rightExpr : leftExpr;
        CType offsetType = leftPtr ? rightType : leftType;

        CType ptrElem = ptrType != null ? ptrType.WithoutConst().Subtype : null;
        if (ptrElem != null && ptrElem.WithoutConst().Tag == CTypeTag.Simple && ptrElem.WithoutConst().SimpleType == CSimpleType.Void)
        {
            Program.Warning(source, ErrorCode.PointerArithmeticDanger,
                "pointer arithmetic on void* may be unsafe");
            return;
        }

        if (offsetType != null && offsetType.IsSigned && !HasTopLevelCast(offsetExpr))
        {
            Program.Warning(source, ErrorCode.PointerArithmeticDanger,
                "signed offset used in pointer arithmetic; cast explicitly if intentional");
            return;
        }

        if (TryConstInt(offsetExpr, out int c) && c < 0)
        {
            Program.Warning(source, ErrorCode.PointerArithmeticDanger,
                "negative constant offset in pointer arithmetic may be unsafe");
            return;
        }
    }

    static Expr SpillToTemp(FunctionCtx ctx, Expr rhs, CType rhsType, List<Expr> prefix)
    {
        CType t = rhsType ?? CType.UInt16;
        if (t.IsArray) t = CType.MakePointer(t.Subtype ?? CType.UInt8);
        if (t.IsStructOrUnion) t = CType.UInt16;

        string tmp = ctx.AcquireTemp(t);
        Expr tmpName = Expr.Make(Tag.Name, tmp).WithSource(rhs.Source);

        prefix.Add(Expr.Make(Tag.Assign, tmpName, rhs).WithSource(rhs.Source));
        return tmpName;
    }

    static bool IsAtom(Expr e)
    {
        int n;
        string s;
        return e.Match(Tag.Integer, out n) || e.Match(Tag.Name, out s);
    }

    static bool IsPow2(int v)
    {
        return v > 0 && (v & (v - 1)) == 0;
    }

    static int Log2Pow2(int v)
    {
        int n = 0;
        while ((v >>= 1) != 0) n++;
        return n;
    }

    static bool IsKnownNonNegativeExpr(FunctionCtx ctx, Expr e)
    {
        if (e == null) return false;

        if (TryConstInt(e, out int cv)) return cv >= 0;
        if (TryInferIntRange(ctx, e, out int min, out int _max)) return min >= 0;

        CType t = InferType(ctx, e);
        if (t == null) return false;
        t = t.WithoutConst();

        if (t.IsPointer) return true;
        if (t.IsEnum) return true;
        if (t.Tag != CTypeTag.Simple) return false;

        return t.SimpleType == CSimpleType.UInt8 || t.SimpleType == CSimpleType.UInt16;
    }

    // "u8-like" includes plain u8 and cloned/annotated integer types that are ultimately 8-bit.
    // We intentionally key off the underlying simple type, not reference equality.
    static bool IsU8Like(CType t)
    {
        return t != null && t.Tag == CTypeTag.Simple &&
               (t.SimpleType == CSimpleType.UInt8 || t.SimpleType == CSimpleType.Int8);
    }

    static bool IsComparisonTag(string tag)
    {
        return tag == Tag.Equal || tag == Tag.NotEqual ||
               tag == Tag.LessThan || tag == Tag.GreaterThan ||
               tag == Tag.LessThanOrEqual || tag == Tag.GreaterThanOrEqual;
    }

	    
    static bool IsStrictEnum(CType t) => t != null && t.IsEnum && t.IsEnumStrict;
    static bool IsSameEnum(CType a, CType b) => a != null && b != null && a.IsEnum && b.IsEnum && a.Name == b.Name;
    static bool IsPlainInteger(CType t) => t != null && t.IsInteger && !t.IsEnum;

    static bool IsBitFlags(CType t) => t != null && t.IsInteger && !t.IsEnum && t.IsBitFlags;
    static bool IsSafeIndex(CType t) => t != null && t.IsInteger && !t.IsEnum && t.IsSafeIndex;

    static bool IsBinaryOpTag(string tag)
	    {
	        if (IsComparisonTag(tag)) return true;
	        return tag == Tag.Add || tag == Tag.Subtract || tag == Tag.Multiply || tag == Tag.Divide || tag == Tag.Modulus ||
	               tag == Tag.BitwiseAnd || tag == Tag.BitwiseOr || tag == Tag.BitwiseXor ||
	               tag == Tag.LogicalAnd || tag == Tag.LogicalOr ||
	               tag == Tag.ShiftLeft || tag == Tag.ShiftRight;
	    }

    static int SizeOfType(CType t)
    {
        if (t == null) return 1;
	        if (t == CType.UInt8 || t == CType.Int8) return 1;
        if (t == CType.Void) return 0;
        // everything else in this compiler is treated as 16-bit for lowering temps
        return 2;
    }

    static CType InferType(FunctionCtx ctx, Expr expr)
    {
        int n;
        string name;
        Expr left, right, sub;
        CType t;

        if (expr.Match(Tag.Integer, out n))
            return (n < 0) ? CType.Int16 : ((n > 255) ? CType.UInt16 : CType.UInt8);

        if (expr.Match(Tag.Name, out name))
            return ctx.FindTypeOfName(name);

        if (expr.Match(Tag.Cast, out t, out sub))
            return t;

        // $slice(ptr,len) behaves like its pointer/array operand.
        if (expr.Match(Tag.Slice, out left, out right))
            return InferType(ctx, left);

        if (expr.Match(Tag.Load, out sub))
        {
            CType pt = InferType(ctx, sub);
            if (pt != null && pt.IsPointer) return pt.Subtype;
            return CType.UInt8;
        }

        // Address-of yields a pointer type (16-bit address on GB).
        if (expr.Match(Tag.AddressOf, out sub))
        {
            CType st = InferType(ctx, sub);
            if (st == null) st = CType.UInt8;
            return new CType { Tag = CTypeTag.Pointer, Subtype = st };
        }

        if (expr.Match(Tag.Index, out left, out right))
        {
            CType bt = InferType(ctx, left);
            if (bt != null && (bt.IsArray || bt.IsPointer)) return bt.Subtype;
            return CType.UInt8;
        }

        string field;
        if (expr.Match(Tag.Field, out sub, out field))
        {
            CType baseT = InferType(ctx, sub);
            CType ft;
            if (ctx.Global.TryGetFieldType(baseT, field, out ft)) return ft;
            return CType.UInt8;
        }

        Expr func;
        Expr[] args;
        if (expr.MatchAny(Tag.Call, out func, out args))
        {
            if (func.Match(Tag.Name, out name))
                return ctx.FindFuncReturn(name);
            return CType.UInt8;
        }

        // Binary ops (A3: minimal usual arithmetic conversions)
        string tag;
        if (expr.MatchAnyTag(out tag, out left, out right) && IsBinaryOpTag(tag))
        {
            if (tag == Tag.LogicalAnd || tag == Tag.LogicalOr) return CType.UInt8;
            if (IsComparisonTag(tag)) return CType.UInt8;

            CType lt = InferType(ctx, left);
            CType rt = InferType(ctx, right);

            // pointer +/- integer -> pointer (keep pointer type)
            if (tag == Tag.Add || tag == Tag.Subtract)
            {
                bool isAdd = (tag == Tag.Add);
                bool isSub = (tag == Tag.Subtract);

                if (lt != null && lt.IsPointer && rt != null && (rt.IsInteger || rt.IsEnum)) return lt;
                if (isAdd && rt != null && rt.IsPointer && lt != null && (lt.IsInteger || lt.IsEnum)) return rt;
                if (isSub && lt != null && lt.IsPointer && rt != null && rt.IsPointer) return CType.UInt16;
            }

            // Integer promotions:
            // - if any operand is signed integer, promote to s16
            // - otherwise promote to u16
            if ((lt != null && (lt.IsInteger || lt.IsEnum)) || (rt != null && (rt.IsInteger || rt.IsEnum)))
            {
                bool signed = (lt != null && lt.IsSigned) || (rt != null && rt.IsSigned);
                return signed ? CType.Int16 : CType.UInt16;
            }

            if (SizeOfType(lt) == 2 || SizeOfType(rt) == 2) return CType.UInt16;
            return CType.UInt8;
        }

        if (expr.Match(Tag.Conditional, out Expr condExpr, out Expr trueExpr, out Expr falseExpr))
        {
            CType tTrue = InferType(ctx, trueExpr);
            CType tFalse = InferType(ctx, falseExpr);
            if ((tTrue != null && tTrue.IsPointer) || (tFalse != null && tFalse.IsPointer))
                return (tTrue != null && tTrue.IsPointer) ? tTrue : tFalse;

            if ((tTrue != null && (tTrue.IsInteger || tTrue.IsEnum)) ||
                (tFalse != null && (tFalse.IsInteger || tFalse.IsEnum)))
            {
                bool signed = (tTrue != null && tTrue.IsSigned) || (tFalse != null && tFalse.IsSigned);
                return signed ? CType.Int16 : CType.UInt16;
            }

            return CType.UInt8;
        }

if (expr.Match(Tag.BitwiseNot, out sub))
	            return InferType(ctx, sub);
	        if (expr.Match(Tag.LogicalNot, out sub))
	            return CType.UInt8;

        return CType.UInt8;
    }

    static Expr MakeCall(Expr func, Expr[] args)
    {
        List<object> parts = new List<object>();
        parts.Add(Tag.Call);
        parts.Add(func);
        for (int i = 0; i < args.Length; i++) parts.Add(args[i]);
        return Expr.Make(parts.ToArray());
    }

    static Expr MakeSequence(List<Expr> stmts)
    {
	        if (stmts == null || stmts.Count == 0) return Expr.Make(Tag.Sequence).WithSource(FilePosition.Unknown);

        object[] parts = new object[1 + stmts.Count];
        parts[0] = Tag.Sequence;
        for (int i = 0; i < stmts.Count; i++) parts[i + 1] = stmts[i];
        return Expr.Make(parts);
    }
}




