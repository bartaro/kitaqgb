using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

partial class Parser
{
    List<Token> Input;
    // Keep a short history of recently consumed tokens for diagnostics.
    List<Token> RecentTokens = new List<Token>();
    List<string> UndefinedLabels = new List<string>();
    FilePosition SourcePosition = FilePosition.Unknown;
    int NextStringID = 0;
    int CurrentPragmaBank = 1;
    int? CurrentPragmaFixedBank = null;
    int? CurrentPragmaFixedOrder = null;
    int SwitchDepth = 0;
    int NextStaticSymbolId = 0;

    // Per-file map of "original static symbol name" -> "mangled internal symbol name".
    readonly Dictionary<string, Dictionary<string, string>> FileStaticSymbols =
        new Dictionary<string, Dictionary<string, string>>(StringComparer.OrdinalIgnoreCase);

    // Lexical local scopes:
    // local source name -> resolved symbol name.
    // - normal locals map to themselves (x -> x)
    // - function-local static maps to a hidden global symbol (x -> __kq_fstatic_...)
    readonly Stack<Dictionary<string, string>> LocalNameScopes = new Stack<Dictionary<string, string>>();

    // Active function context used when lowering function-local static declarations.
    string CurrentFunctionNameForStaticLocals = null;
    List<Expr> CurrentFunctionStaticDecls = null;

    // --- Placement shortcut pragmas ---
    // Persist until overridden:
    // #pragma hram / wram0 / wramx
    // #pragma align N
    // #pragma section "NAME"
    MemoryRegion CurrentPragmaRegion = MemoryRegion.Ram;
    int CurrentPragmaWramXBank = 0;
    int CurrentPragmaAlign = 0; // 0/1 means "no extra alignment"
    string CurrentPragmaSection = null;

    // --- Attributes / lints ---
    // Support a minimal C++-style attribute for ergonomics:
    // [[nodiscard]]
    // And a keyword-like alias:
    // __must_check
    // Both map to the same internal flag on function declarations.

    Dictionary<string, string> StringLiteralNameByValue = new Dictionary<string, string>();
    List<Expr> StringLiteralDeclarations = new List<Expr>();

    // String literal pool:
    // - We do NOT emit ($readonly_data ...) as an expression.
    // - Instead, we create a top-level readonly data declaration and return ($name $stringN).
    // - Identical string contents are deduplicated to the same $stringN.
    Dictionary<string, string> StringPool = new Dictionary<string, string>();
    List<Expr> PooledStringDecls = new List<Expr>();

    // Typedef aliases (very small subset):
    // typedef <type> <name>;
    // typedef <type> <name>[N];
    // Used to keep demo / runtime code ergonomic without requiring struct/member support.
    Dictionary<string, CType> Typedefs = new Dictionary<string, CType>();
    // Enum tags (subset):
    // enum Tag { A=0, B, C };
    // typedef enum Tag { ... } Name;
    Dictionary<string, CType> EnumTags = new Dictionary<string, CType>();
    HashSet<string> StrictEnumTags = new HashSet<string>();

    // When parsing a type specifier that also defines enum constants, we enqueue
    // those constant declarations here so ParseGlobalDecl can emit them before the main decl.
    List<Expr> PendingTopDecls = new List<Expr>();

    sealed class AggregateDeclInfo
    {
        public bool IsUnion;
        public FieldInfo[] Fields = Array.Empty<FieldInfo>();
        public bool IsPacked;
        public int ForcedAlign;
    }

    // Parser-side aggregate definitions used for initializer parsing.
    Dictionary<string, AggregateDeclInfo> AggregateDecls = new Dictionary<string, AggregateDeclInfo>();

    // --- Multiple-error collection ---
    // We throw this to unwind to a recovery point after emitting a diagnostic.
    sealed class RecoverableParseException : Exception { }

    public static Expr ParseFiles(IEnumerable<string> filenames)
    {
        Parser p = new Parser(filenames);
        return p.ParseAll();
    }

    Parser(IEnumerable<string> filenames)
    {
        // Built-in signed aliases (storage-width compatible with existing u8/u16 core).
        // These are parsed as typedef-like names so they also work in sizeof(type) lookahead.
        Typedefs["s8"] = CType.Int8;
        Typedefs["int8_t"] = CType.Int8;
        Typedefs["s16"] = CType.Int16;
        Typedefs["int16_t"] = CType.Int16;

        // Lex all the input files:
        Input = new List<Token>();
        FilePosition eofPos = FilePosition.Unknown;
        List<string> deps;
        Input.AddRange(Tokenizer.TokenizeFiles(filenames, Program.IncludeDirectories, out eofPos, out deps));
        Program.SetLastCompilationDependencies(deps);
        PredeclareTypedefs();
        Input.Add(new Token
        {
            Tag = TokenType.EOF,
            // Avoid the common "line1/col1" misfire by placing EOF at the true end position.
            Position = eofPos,
        });
    }

    Parser(
        List<Token> input,
        Dictionary<string, CType> typedefs,
        Dictionary<string, CType> enumTags,
        HashSet<string> strictEnumTags,
        Dictionary<string, AggregateDeclInfo> aggregateDecls)
    {
        Input = input ?? new List<Token>();
        Typedefs = typedefs ?? new Dictionary<string, CType>();
        EnumTags = enumTags ?? new Dictionary<string, CType>();
        StrictEnumTags = strictEnumTags ?? new HashSet<string>();
        AggregateDecls = aggregateDecls ?? new Dictionary<string, AggregateDeclInfo>();
    }

    Expr Make(params object[] args)
    {
        return Make(Expr.Make(args));
    }

    Expr Make(Expr e)
    {
        return e.WithSource(SourcePosition);
    }

    Expr MakeSequence(IEnumerable<Expr> items)
    {
        List<object> list = new List<object>();
        list.Add(Tag.Sequence);
        list.AddRange(items);
        return Expr.Make(list.ToArray()).WithSource(SourcePosition);
    }

    string GetStringLiteralPoolKey(string value)
    {
        // String literals are plain CPU pointers. If we let them drift into
        // switchable banks, banked code can end up dereferencing a pointer to
        // a bank that is no longer visible after bank-layout relayout.
        // Pool them globally and pin them to fixed bank 0 instead.
        return value ?? "";
    }

    string MakeAnonymousTypeName(string prefix)
    {
        string file = SourcePosition.Filename ?? "unknown";
        var sb = new StringBuilder(file.Length);
        for (int i = 0; i < file.Length; i++)
        {
            char c = file[i];
            sb.Append(char.IsLetterOrDigit(c) ? c : '_');
        }
        return string.Format("{0}_{1}_{2}_{3}", prefix, sb, SourcePosition.Line + 1, SourcePosition.Column + 1);
    }

    void PredeclareTypedefs()
    {
        List<List<Token>> typedefDecls = ExtractTopLevelTypedefDeclarations();
        if (typedefDecls.Count == 0) return;

        HashSet<string> completed = new HashSet<string>(StringComparer.Ordinal);
        bool progressed;
        do
        {
            progressed = false;
            for (int i = 0; i < typedefDecls.Count; i++)
            {
                string key = BuildTokenSliceIdentity(typedefDecls[i]);
                if (completed.Contains(key)) continue;
                if (!TryPredeclareTypedef(typedefDecls[i])) continue;
                completed.Add(key);
                progressed = true;
            }
        } while (progressed);
    }

    List<List<Token>> ExtractTopLevelTypedefDeclarations()
    {
        var result = new List<List<Token>>();
        int topLevelBraceDepth = 0;
        for (int i = 0; i < Input.Count; i++)
        {
            Token t = Input[i];
            if (t.Tag == TokenType.LBRACE) topLevelBraceDepth++;
            else if (t.Tag == TokenType.RBRACE) topLevelBraceDepth = Math.Max(0, topLevelBraceDepth - 1);

            if (topLevelBraceDepth != 0)
                continue;

            if (!(t.Tag == TokenType.NAME && string.Equals(t.Name, "typedef", StringComparison.Ordinal)))
                continue;

            int stmtBraceDepth = 0;
            int stmtParenDepth = 0;
            int stmtBracketDepth = 0;
            var stmt = new List<Token>();
            int j = i;
            for (; j < Input.Count; j++)
            {
                Token cur = Input[j];
                stmt.Add(cur);
                if (cur.Tag == TokenType.LBRACE) stmtBraceDepth++;
                else if (cur.Tag == TokenType.RBRACE) stmtBraceDepth = Math.Max(0, stmtBraceDepth - 1);
                else if (cur.Tag == TokenType.LPAREN) stmtParenDepth++;
                else if (cur.Tag == TokenType.RPAREN) stmtParenDepth = Math.Max(0, stmtParenDepth - 1);
                else if (cur.Tag == TokenType.LBRACKET) stmtBracketDepth++;
                else if (cur.Tag == TokenType.RBRACKET) stmtBracketDepth = Math.Max(0, stmtBracketDepth - 1);

                if (cur.Tag == TokenType.SEMICOLON &&
                    stmtBraceDepth == 0 &&
                    stmtParenDepth == 0 &&
                    stmtBracketDepth == 0)
                {
                    break;
                }
            }

            if (stmt.Count != 0 && stmt[stmt.Count - 1].Tag == TokenType.SEMICOLON)
                result.Add(stmt);

            i = j;
        }
        return result;
    }

    bool TryPredeclareTypedef(List<Token> stmt)
    {
        if (stmt == null || stmt.Count == 0) return false;

        var typedefs = CloneTypeMap(Typedefs);
        var enumTags = CloneTypeMap(EnumTags);
        var strictEnumTags = new HashSet<string>(StrictEnumTags, StringComparer.Ordinal);
        var aggregateDecls = CloneAggregateDeclMap(AggregateDecls);

        var probeInput = new List<Token>(stmt.Count + 1);
        probeInput.AddRange(stmt);
        probeInput.Add(new Token
        {
            Tag = TokenType.EOF,
            Position = stmt[stmt.Count - 1].Position,
        });

        Parser probe = new Parser(probeInput, typedefs, enumTags, strictEnumTags, aggregateDecls);
        Program.DiagnosticSnapshot snap = Program.BeginSuppressedDiagnostics();
        try
        {
            probe.ParseDeclaration();
            if (!probe.TryParse(TokenType.EOF))
                throw new RecoverableParseException();

            Typedefs = typedefs;
            EnumTags = enumTags;
            StrictEnumTags = strictEnumTags;
            AggregateDecls = aggregateDecls;
            Program.RestoreDiagnostics(snap);
            return true;
        }
        catch (RecoverableParseException)
        {
            Program.RestoreDiagnostics(snap);
            return false;
        }
    }

    static string BuildTokenSliceIdentity(List<Token> stmt)
    {
        if (stmt == null || stmt.Count == 0) return "";
        Token first = stmt[0];
        Token last = stmt[stmt.Count - 1];
        return string.Format("{0}:{1}:{2}:{3}:{4}",
            first.Position.Filename ?? "",
            first.Position.Line,
            first.Position.Column,
            last.Position.Line,
            last.Position.Column);
    }

    static Dictionary<string, CType> CloneTypeMap(Dictionary<string, CType> src)
    {
        var dst = new Dictionary<string, CType>(src.Comparer);
        foreach (var kv in src) dst[kv.Key] = DeepCloneType(kv.Value);
        return dst;
    }

    static Dictionary<string, AggregateDeclInfo> CloneAggregateDeclMap(Dictionary<string, AggregateDeclInfo> src)
    {
        var dst = new Dictionary<string, AggregateDeclInfo>(src.Comparer);
        foreach (var kv in src)
        {
            var info = kv.Value;
            dst[kv.Key] = new AggregateDeclInfo
            {
                IsUnion = info.IsUnion,
                Fields = CloneFields(info.Fields),
                IsPacked = info.IsPacked,
                ForcedAlign = info.ForcedAlign,
            };
        }
        return dst;
    }

    static FieldInfo[] CloneFields(FieldInfo[] src)
    {
        if (src == null) return Array.Empty<FieldInfo>();
        FieldInfo[] dst = new FieldInfo[src.Length];
        for (int i = 0; i < src.Length; i++)
        {
            FieldInfo f = src[i];
            dst[i] = new FieldInfo(DeepCloneType(f.Type), f.Name, f.Offset);
        }
        return dst;
    }

    static CType DeepCloneType(CType src)
    {
        if (src == null) return null;
        return new CType
        {
            Tag = src.Tag,
            SimpleType = src.SimpleType,
            Name = src.Name,
            Subtype = DeepCloneType(src.Subtype),
            Dimension = src.Dimension,
            DimensionExpression = src.DimensionExpression,
            IsConst = src.IsConst,
            IsRestrict = src.IsRestrict,
            IsEnumStrict = src.IsEnumStrict,
            IsBitFlags = src.IsBitFlags,
            IsSafeIndex = src.IsSafeIndex,
            ForcedAlign = src.ForcedAlign,
            IsPackedAggregate = src.IsPackedAggregate,
            ParamTypes = src.ParamTypes == null ? null : src.ParamTypes.Select(DeepCloneType).ToArray(),
        };
    }

    Expr ParseAll()
    {
        List<Expr> declarations = new List<Expr>();
        while (!TryParse(TokenType.EOF))
        {
            // #pragma bank N
            int pb;
            if (TryParsePragmaBank(out pb))
            {
                CurrentPragmaBank = pb;
                continue;
            }

            if (TryParsePragmaFixedBank(out int pfb))
            {
                CurrentPragmaFixedBank = (pfb >= 0) ? (int?)pfb : null;
                continue;
            }

            if (TryParsePragmaFixedOrder(out int pfo))
            {
                CurrentPragmaFixedOrder = (pfo >= 0) ? (int?)pfo : null;
                continue;
            }

            // #pragma hram / wram0 / wramx
            if (TryParsePragmaRegion(out MemoryRegion r))
            {
                CurrentPragmaRegion = r;
                continue;
            }

            if (TryParsePragmaWramXBank(out int pwx))
            {
                if (ValidateWramXBankLiteral(pwx, "#pragma wramx_bank"))
                    CurrentPragmaWramXBank = pwx;
                continue;
            }

            // #pragma align N
            if (TryParsePragmaAlign(out int a))
            {
                CurrentPragmaAlign = a;
                continue;
            }

            // #pragma section "NAME"
            if (TryParsePragmaSection(out string s))
            {
                CurrentPragmaSection = s;
                continue;
            }
            try
            {
                PendingTopDecls.Clear();
                Expr d = ParseDeclaration();
                if (PendingTopDecls.Count != 0)
                {
                    for (int i = 0; i < PendingTopDecls.Count; i++) AppendTopLevelDeclaration(declarations, PendingTopDecls[i]);
                }
                AppendTopLevelDeclaration(declarations, d);
            }
            catch (RecoverableParseException)
            {
                SynchronizeTopLevel();
            }
        }

        // Append pooled string literals as top-level readonly data declarations.
        // (They were referenced earlier via ($name $stringN) expressions.)
        if (StringLiteralDeclarations.Count != 0)
        {
            declarations.AddRange(StringLiteralDeclarations);
        }
        return MakeSequence(declarations);
    }

    void AppendTopLevelDeclaration(List<Expr> outDecls, Expr decl)
    {
        if (decl == null || decl.Match(Tag.Empty)) return;

        Expr[] seq;
        if (decl.MatchAny(Tag.Sequence, out seq))
        {
            if (seq == null) return;
            for (int i = 0; i < seq.Length; i++) AppendTopLevelDeclaration(outDecls, seq[i]);
            return;
        }

        outDecls.Add(decl);
    }

    // Recovery strategy: skip tokens until we reach a reasonable top-level boundary.
    // This prevents infinite loops when we report an error but the current token cannot be consumed.
    void SynchronizeTopLevel()
    {
        while (true)
        {
            Token t = PeekToken();
            if (t.Tag == TokenType.EOF) return;
            if (t.Tag == TokenType.SEMICOLON)
            {
                ConsumeToken();
                return;
            }
            if (t.Tag == TokenType.RBRACE)
            {
                ConsumeToken();
                return;
            }
            // Allow pragma bank to be processed normally by the outer loop.
            if (t.Tag == TokenType.PRAGMA_BANK || t.Tag == TokenType.PRAGMA_FIXED_BANK || t.Tag == TokenType.PRAGMA_FIXED_ORDER || t.Tag == TokenType.PRAGMA_REGION || t.Tag == TokenType.PRAGMA_WRAMX_BANK || t.Tag == TokenType.PRAGMA_ALIGN || t.Tag == TokenType.PRAGMA_SECTION) return;
            ConsumeToken();
        }
    }

    // Recovery inside a statement block.
    // Stops at:
    // - ';' (consumed)
    // - '}' (not consumed; caller decides scope close)
    // - EOF
    // - (optional) switch case boundaries: 'case' / 'default' (not consumed)
    void SynchronizeStatementBoundary(bool stopAtSwitchCaseBoundary)
    {
        int nestedBraceDepth = 0;
        while (true)
        {
            Token t = PeekToken();
            if (t.Tag == TokenType.EOF) return;

            if (nestedBraceDepth == 0)
            {
                if (t.Tag == TokenType.RBRACE) return;
                if (stopAtSwitchCaseBoundary && t.Tag == TokenType.NAME &&
                    (t.Name == "case" || t.Name == "default"))
                {
                    return;
                }
            }

            if (t.Tag == TokenType.SEMICOLON)
            {
                ConsumeToken();
                return;
            }

            if (t.Tag == TokenType.LBRACE)
            {
                nestedBraceDepth++;
                ConsumeToken();
                continue;
            }
            if (t.Tag == TokenType.RBRACE)
            {
                if (nestedBraceDepth > 0)
                {
                    nestedBraceDepth--;
                    ConsumeToken();
                    continue;
                }
                return;
            }

            ConsumeToken();
        }
    }


    Expr WrapPragmasIfNeeded(Expr decl)
    {
        if (decl == null) return null;
        string tag = decl.GetTag();

        // bank wrapper applies only to ROM-emitted entities
        if (tag == Tag.Function || tag == Tag.InlineFunction || tag == Tag.FunctionDecl || tag == Tag.ReadonlyData)
        {
            if (CurrentPragmaFixedBank.HasValue)
                decl = Make(Tag.FixedBank, CurrentPragmaFixedBank.Value, decl);
            if (CurrentPragmaFixedOrder.HasValue)
                decl = Make(Tag.FixedOrder, CurrentPragmaFixedOrder.Value, decl);
            decl = Make(Tag.Bank, CurrentPragmaBank, decl);
        }

        // section/align are primarily for ROM-emitted entities, but also useful for globals.
        if (!string.IsNullOrEmpty(CurrentPragmaSection))
        {
            if (tag == Tag.Function || tag == Tag.InlineFunction || tag == Tag.FunctionDecl || tag == Tag.ReadonlyData || tag == Tag.Variable || tag == Tag.ExternVariable)
                decl = Make(Tag.DeclSection, CurrentPragmaSection, decl);
        }

        if (CurrentPragmaAlign > 1)
        {
            if (tag == Tag.Function || tag == Tag.InlineFunction || tag == Tag.FunctionDecl || tag == Tag.ReadonlyData || tag == Tag.Variable || tag == Tag.ExternVariable)
                decl = Make(Tag.DeclAlign, CurrentPragmaAlign, decl);
        }

        return decl;
    }

    string CurrentFilename()
    {
        if (Input != null && Input.Count > 0)
        {
            string fn = Input[0].Position.Filename;
            if (!string.IsNullOrEmpty(fn)) return fn;
        }
        return SourcePosition.Filename ?? "";
    }

    string GetOrCreateStaticSymbolName(string filename, string original)
    {
        if (string.IsNullOrEmpty(original)) return original;
        if (string.IsNullOrEmpty(filename)) filename = "<unknown>";

        if (!FileStaticSymbols.TryGetValue(filename, out Dictionary<string, string> map))
        {
            map = new Dictionary<string, string>(StringComparer.Ordinal);
            FileStaticSymbols.Add(filename, map);
        }

        if (map.TryGetValue(original, out string existing))
            return existing;

        string mangled = "__kq_static_" + (NextStaticSymbolId++).ToString() + "_" + original;
        map[original] = mangled;
        return mangled;
    }

    string GetFunctionLocalStaticSymbolName(string functionName, string original)
    {
        string fn = string.IsNullOrEmpty(functionName) ? "fn" : functionName;
        string nm = string.IsNullOrEmpty(original) ? "var" : original;
        return "__kq_fstatic_" + (NextStaticSymbolId++).ToString() + "_" + fn + "_" + nm;
    }

    bool TryResolveLocalScopedName(string name, out string resolved)
    {
        if (string.IsNullOrEmpty(name))
        {
            resolved = name;
            return false;
        }

        foreach (var scope in LocalNameScopes)
        {
            if (scope.TryGetValue(name, out resolved))
                return true;
        }

        resolved = name;
        return false;
    }

    string ResolveStaticSymbolReference(string filename, string name)
    {
        if (string.IsNullOrEmpty(name)) return name;
        if (TryResolveLocalScopedName(name, out string localResolved)) return localResolved;
        if (string.IsNullOrEmpty(filename)) filename = CurrentFilename();
        if (FileStaticSymbols.TryGetValue(filename, out Dictionary<string, string> map) &&
            map.TryGetValue(name, out string mapped))
        {
            return mapped;
        }
        return name;
    }

    void PushLocalScope()
    {
        LocalNameScopes.Push(new Dictionary<string, string>(StringComparer.Ordinal));
    }

    void PopLocalScope()
    {
        if (LocalNameScopes.Count > 0) LocalNameScopes.Pop();
    }

    void RegisterLocalName(string name)
    {
        if (LocalNameScopes.Count == 0) return;
        if (string.IsNullOrEmpty(name)) return;
        LocalNameScopes.Peek()[name] = name;
    }

    void RegisterLocalAlias(string sourceName, string resolvedName)
    {
        if (LocalNameScopes.Count == 0) return;
        if (string.IsNullOrEmpty(sourceName)) return;
        if (string.IsNullOrEmpty(resolvedName)) resolvedName = sourceName;
        LocalNameScopes.Peek()[sourceName] = resolvedName;
    }

Expr ParseDeclaration()
    {
        string declFile = CurrentFilename();

        bool isStatic = false;
        while (TryParseName("static"))
        {
            isStatic = true;
        }

        // extern (global variable declarations; functions will ignore it)
        bool isExtern = false;
        while (TryParseName("extern"))
        {
            isExtern = true;
        }
        if (isStatic && isExtern)
        {
            Program.Warning(SourcePosition, "warning: 'static' with 'extern' is treated as 'static'");
            isExtern = false;
        }

        // inline / __forceinline keyword
        bool isInline = false;
        if (TryParseName("inline") || TryParseName("__forceinline"))
        {
            isInline = true;
        }

        // __unsafe keyword (marks the following declaration as unsafe)
        bool isUnsafeDecl = false;
        while (TryParseName("__unsafe"))
        {
            isUnsafeDecl = true;
        }

        // --- Attributes on declarations ---
        // __must_check <type> <name>(...)
        // [[nodiscard]] <type> <name>(...)
        bool mustCheck = false;
        // __range(min,max) <type> <name>;
        Expr declRange = null;
        // __aligned(N) <type> <name>;
        int declAlign = 0;
        while (true)
        {
            if (TryParseName("__must_check")) { mustCheck = true; continue; }
            if (TryParseNodiscardAttribute()) { mustCheck = true; continue; }
            Expr r;
            if (TryParseRangeAttribute(out r)) { declRange = r; continue; }
            int a;
            if (TryParseAlignedAttribute(out a)) { declAlign = a; continue; }
            break;
        }



        // --- static_assert(expr, "msg") ---
        if (TryParseName("static_assert") || TryParseName("_Static_assert"))
        {
            if (isStatic)
            {
                Program.Warning(SourcePosition, "warning: 'static' has no effect on static_assert");
            }
            return ParseStaticAssert(true);
        }
        // --- typedef (subset) ---
        // Supports:
        // typedef <type> <name>;
        // typedef <type> <name>[N];
        // (Enough for the Wire3D demo aliases like Vec3/Mat3.)
        if (TryParseName("typedef"))
        {
            if (isStatic)
            {
                Program.Warning(SourcePosition, "warning: 'static' has no effect on typedef");
            }
            MemoryRegion bad;
            if (TryParseMemoryRegionQualifier(out bad))
            {
                ParserError("memory region qualifiers are not allowed on typedef");
            }

            Expr extraDecl = null;
            CType aliasType;

            // Support: typedef struct/union { ... } Name;
            // typedef struct/union Tag { ... } Name;
            // typedef struct/union Tag Name;
            bool isUnion = false;
            bool sawStructOrUnion = false;

            // Layout attributes for typedef struct/union blocks.
            bool aggPacked = false;
            int aggAlign = 0;
            while (true)
            {
                if (TryParseName("__packed")) { aggPacked = true; continue; }
                int a;
                if (TryParseAlignedAttribute(out a)) { aggAlign = a; continue; }
                break;
            }

            if (TryParseName("struct"))
            {
                isUnion = false;
                sawStructOrUnion = true;
            }
            else if (TryParseName("union"))
            {
                isUnion = true;
                sawStructOrUnion = true;
            }

            if (sawStructOrUnion)
            {
                // Optional tag name.
                string tagName = null;
                if (PeekToken().Tag == TokenType.NAME) tagName = ExpectAnyName();

                if (TryParse(TokenType.LBRACE))
                {
                    if (tagName == null)
                    {
                        tagName = MakeAnonymousTypeName("__anon");
                    }

                    List<FieldInfo> fields = new List<FieldInfo>();
                    while (!TryParse(TokenType.RBRACE))
                    {
                        CType fieldType = ExpectType();
                        string fieldName = ExpectAnyName();
                        ParseArrayDeclaration(ref fieldType);
                        fields.Add(new FieldInfo(fieldType, fieldName, 0));
                        while (TryParse(TokenType.COMMA))
                        {
                            fieldName = ExpectAnyName();
                            fields.Add(new FieldInfo(fieldType, fieldName, 0));
                        }
                        Expect(TokenType.SEMICOLON);
                    }

                    string tag = isUnion ? Tag.Union : Tag.Struct;
                    FieldInfo[] parsedFields = fields.ToArray();
                    extraDecl = Make(tag, tagName, parsedFields, aggPacked ? 1 : 0, aggAlign);
                    RegisterAggregateDecl(tagName, isUnion, parsedFields, aggPacked, aggAlign);

                    aliasType = isUnion ? CType.MakeUnion(tagName) : CType.MakeStruct(tagName);
                    aliasType.IsPackedAggregate = aggPacked;
                    aliasType.ForcedAlign = aggAlign;
                }
                else
                {
                    if (tagName == null) ParserError(ErrorCode.ExpectedType, "expected a struct/union tag name");
                    // Forward-declare an opaque struct/union so later definitions can overwrite it.
                    extraDecl = Make(isUnion ? Tag.OpaqueUnion : Tag.OpaqueStruct, tagName);
                    aliasType = isUnion ? CType.MakeUnion(tagName) : CType.MakeStruct(tagName);
                    aliasType.IsPackedAggregate = aggPacked;
                    aliasType.ForcedAlign = aggAlign;
                }
            }
            else
            {
                // Normal typedef: typedef <type> Name;
                aliasType = ExpectType();
            }

            // Support a limited but very useful subset of function-pointer typedefs:
            // typedef <ret> (*Name)(<paramtypes>);
            // e.g.
            // typedef void (*Fn)(u8);
            // typedef u8 (*Read)(u16 addr);
            // Notes:
            // - Parameter names are optional.
            // - We support multiple pointer stars: (**Name)(...)
            if (PeekToken().Tag == TokenType.LPAREN)
            {
                // Parse: ( *... Name ) ( ... )
                Expect(TokenType.LPAREN);

                int ptrDepth = 0;
                while (TryParse(TokenType.STAR)) ptrDepth++;
                if (ptrDepth == 0) ParserError(ErrorCode.ParseError, "expected '*' in function pointer typedef declarator");

                string aliasName = ExpectAnyName();
                Expect(TokenType.RPAREN);

                // Parameter list
                Expect(TokenType.LPAREN);
                List<CType> paramTypes = new List<CType>();
                if (!TryParse(TokenType.RPAREN))
                {
                    while (true)
                    {
                        CType ptype = ExpectType();

                        // Optional parameter name
                        if (PeekToken().Tag == TokenType.NAME)
                        {
                            ConsumeToken();
                        }

                        // Allow array declarator on params (very small subset)
                        ParseArrayDeclaration(ref ptype);

                        paramTypes.Add(ptype);

                        if (TryParse(TokenType.RPAREN)) break;
                        Expect(TokenType.COMMA);
                    }
                }

                // C special-case: (void) means no parameters.
                if (paramTypes.Count == 1 && paramTypes[0].IsSimple && paramTypes[0].SimpleType == CSimpleType.Void)
                {
                    paramTypes.Clear();
                }

                Expect(TokenType.SEMICOLON);

                CType fnType = CType.MakeFunction(aliasType, paramTypes.ToArray());
                CType finalType = fnType;
                for (int i = 0; i < ptrDepth; i++) finalType = CType.MakePointer(finalType);

                Typedefs[aliasName] = finalType;

                if (extraDecl != null)
                {
                    // Emit the aggregate declaration directly so top-level declaration
                    // passes (type registration/codegen) can consume it without needing
                    // to flatten nested single-item sequences.
                    return extraDecl;
                }
                return MakeSequence(new Expr[0]);
            }

            // Normal typedef: typedef <type> Name; (and optional array declarator)
            string aliasName2 = ExpectAnyName();
            ParseArrayDeclaration(ref aliasType);
            Expect(TokenType.SEMICOLON);
            Typedefs[aliasName2] = aliasType;

            // Typedef is a compile-time alias only (no codegen),
            // but we may need to emit a struct/union definition when using an anonymous body.
            if (extraDecl != null)
            {
                // Emit the aggregate declaration directly so top-level declaration
                // passes (type registration/codegen) can consume it without needing
                // to flatten nested single-item sequences.
                return extraDecl;
            }
            return MakeSequence(new Expr[0]);
        }

        if (TryParseName("constexpr"))
        {
            if (isExtern) Program.Warning(SourcePosition, "warning: 'extern' on constexpr has no effect");
            Expr cexpr = ParseConstexprDeclaration(declAlign, isUnsafeDecl, isStatic, declFile);
            return cexpr;
        }

        if (TryParseName("define"))
        {
            CType type = ExpectType();

            // Apply declaration-level __aligned(N) if present.
            if (declAlign > 0 && type.ForcedAlign == 0)
            {
                type = new CType
                {
                    Tag = type.Tag,
                    SimpleType = type.SimpleType,
                    Name = type.Name,
                    Subtype = type.Subtype,
                    Dimension = type.Dimension,
                    DimensionExpression = type.DimensionExpression,
                    ParamTypes = type.ParamTypes,
                    IsConst = type.IsConst,
                IsRestrict = type.IsRestrict,
                    IsEnumStrict = type.IsEnumStrict,
                    IsBitFlags = type.IsBitFlags,
                IsSafeIndex = type.IsSafeIndex,
                    ForcedAlign = declAlign,
                    IsPackedAggregate = type.IsPackedAggregate,
                };
            }
            // Allow declaration-level __aligned(N) to apply to the parsed type
            // (only if the type itself did not already carry a forced alignment).
            if (declAlign > 0 && type.ForcedAlign == 0)
            {
                type = new CType
                {
                    Tag = type.Tag,
                    SimpleType = type.SimpleType,
                    Name = type.Name,
                    Subtype = type.Subtype,
                    Dimension = type.Dimension,
                    DimensionExpression = type.DimensionExpression,
                    ParamTypes = type.ParamTypes,
                    IsConst = type.IsConst,
                IsRestrict = type.IsRestrict,
                    IsEnumStrict = type.IsEnumStrict,
                    IsBitFlags = type.IsBitFlags,
                IsSafeIndex = type.IsSafeIndex,
                    ForcedAlign = declAlign,
                    IsPackedAggregate = type.IsPackedAggregate,
                };
            }
                string name = ExpectAnyName();
                if (isStatic) name = GetOrCreateStaticSymbolName(declFile, name);
            Expect(TokenType.EQUAL);
            Expr value = ParseExpr();
            Expect(TokenType.SEMICOLON);
            Expr dc = Make(Tag.Constant, type, name, value);
            if (isStatic) dc = Make(Tag.Static, dc);
            return dc;
        }
        else
        {
            MemoryRegion explicitRegion;
            bool hadRegion = TryParseMemoryRegionQualifier(out explicitRegion);
            MemoryRegion region = hadRegion ? ApplyPragmaWramXBank(explicitRegion) : ApplyPragmaWramXBank(CurrentPragmaRegion);

            CType type = ExpectType();

            // Apply declaration-level __aligned(N) if present.
            if (declAlign > 0 && type.ForcedAlign == 0)
            {
                type = new CType
                {
                    Tag = type.Tag,
                    SimpleType = type.SimpleType,
                    Name = type.Name,
                    Subtype = type.Subtype,
                    Dimension = type.Dimension,
                    DimensionExpression = type.DimensionExpression,
                    ParamTypes = type.ParamTypes,
                    IsConst = type.IsConst,
                IsRestrict = type.IsRestrict,
                    IsEnumStrict = type.IsEnumStrict,
                    IsBitFlags = type.IsBitFlags,
                IsSafeIndex = type.IsSafeIndex,
                    ForcedAlign = declAlign,
                    IsPackedAggregate = type.IsPackedAggregate,
                };
            }

            // Allow tag-only enum declarations:
            // enum Tag { ... };
            // enum Tag;
            if (type.IsEnum && PeekToken().Tag == TokenType.SEMICOLON)
            {
                ConsumeToken();
                return MakeSequence(new Expr[0]);
            }

            if (type.IsStructOrUnion && TryParse(TokenType.LBRACE))
            {
                if (isStatic)
                {
                    Program.Warning(SourcePosition, "warning: 'static' has no effect on type declarations");
                }
                if (hadRegion) ParserError("memory region qualifiers are only allowed on global variables");

                List<FieldInfo> fields = new List<FieldInfo>();
                while (!TryParse(TokenType.RBRACE))
                {
                    CType fieldType = ExpectType();
                    string fieldName = ExpectAnyName();
                    ParseArrayDeclaration(ref fieldType);
                    fields.Add(new FieldInfo(fieldType, fieldName, 0));
                    while (TryParse(TokenType.COMMA))
                    {
                        fieldName = ExpectAnyName();
                        fields.Add(new FieldInfo(fieldType, fieldName, 0));
                    }
                    Expect(TokenType.SEMICOLON);
                }
                Expect(TokenType.SEMICOLON);

                string tag = (type.Tag == CTypeTag.Struct) ? Tag.Struct : Tag.Union;
                // Include layout attributes if present.
                FieldInfo[] parsedFields = fields.ToArray();
                RegisterAggregateDecl(type.Name, type.Tag == CTypeTag.Union, parsedFields, type.IsPackedAggregate, type.ForcedAlign);
                return Make(tag, type.Name, parsedFields, type.IsPackedAggregate ? 1 : 0, type.ForcedAlign);
            }
            else
            {
                string name = ExpectAnyName();
                if (isStatic) name = GetOrCreateStaticSymbolName(declFile, name);

                // __stackcall function attribute (must appear before the function name)
                bool isStackCallDecl = false;
                if (name == "__stackcall")
                {
                    isStackCallDecl = true;
                    name = ExpectAnyName();
                    if (isStatic) name = GetOrCreateStaticSymbolName(declFile, name);
                }

                if (isStackCallDecl && PeekToken().Tag != TokenType.LPAREN)
                {
                    ParserError("__stackcall is only allowed on function declarations");
                }

                if (TryParse(TokenType.LPAREN))
                {
                    if (hadRegion) ParserError("memory region qualifiers are only allowed on global variables");

                    if (isExtern)
                    {
                        // C allows extern on functions, but it has no effect here.
                        Program.Warning(SourcePosition, "warning: 'extern' on function declarations has no effect");
                    }

                    // Labels can be referenced before they are defined, but they must be declared by the end
                    // of the function in which they are referenced.
                    if (UndefinedLabels.Count > 0)
                    {
                        ParserError("label not defined: {0}", UndefinedLabels[0]);
                    }

                    List<FieldInfo> fields = new List<FieldInfo>();
                    if (!TryParse(TokenType.RPAREN))
                    {
                        while (true)
                        {
                            CType fieldType = ExpectType();
                            string fieldName = ExpectAnyName();
                            fields.Add(new FieldInfo(fieldType, fieldName, 0));
                            if (TryParse(TokenType.RPAREN)) break;
                            Expect(TokenType.COMMA);
                        }
                    }

                    // Prototype: <type> <name>(...);
                    if (TryParse(TokenType.SEMICOLON))
                    {
                        string ptag = Tag.FunctionDecl;
                        Expr d = WrapPragmasIfNeeded(Make(ptag, type, name, fields.ToArray(), mustCheck ? 1 : 0));
                        if (isStatic) d = Make(Tag.Static, d);
                        if (isStackCallDecl) d = Make(Tag.StackCall, d);
                        if (isUnsafeDecl) d = Make(Tag.Unsafe, d);
                        return d;
                    }
                    List<Expr> statements = new List<Expr>();
                    Expect(TokenType.LBRACE);

                    string prevStaticFnName = CurrentFunctionNameForStaticLocals;
                    List<Expr> prevStaticDecls = CurrentFunctionStaticDecls;
                    CurrentFunctionNameForStaticLocals = name;
                    CurrentFunctionStaticDecls = new List<Expr>();

                    try
                    {
                        PushLocalScope();
                        try
                        {
                            for (int pi = 0; pi < fields.Count; pi++)
                            {
                                RegisterLocalName(fields[pi].Name);
                            }

                            while (!TryParse(TokenType.RBRACE))
                            {
                                if (PeekToken().Tag == TokenType.EOF)
                                {
                                    // Keep recovery local to the function body so a single syntax
                                    // error does not cascade into top-level declaration failures.
                                    Program.Error(PeekToken().Position, ErrorCode.ExpectedToken, "expected }, got {0}", PeekToken().Show());
                                    break;
                                }

                                try
                                {
                                    statements.Add(ParseStatement(true));
                                }
                                catch (RecoverableParseException)
                                {
                                    SynchronizeStatementBoundary(stopAtSwitchCaseBoundary: false);
                                    if (PeekToken().Tag == TokenType.EOF) break;
                                }
                            }
                        }
                        finally
                        {
                            PopLocalScope();
                        }

                        string tag = isInline ? Tag.InlineFunction : Tag.Function;
                        Expr fnDef = WrapPragmasIfNeeded(Make(tag, type, name, fields.ToArray(), mustCheck ? 1 : 0, MakeSequence(statements)));
                        if (isStatic) fnDef = Make(Tag.Static, fnDef);
                        if (isStackCallDecl) fnDef = Make(Tag.StackCall, fnDef);
                        if (isUnsafeDecl) fnDef = Make(Tag.Unsafe, fnDef);

                        if (CurrentFunctionStaticDecls != null && CurrentFunctionStaticDecls.Count != 0)
                        {
                            List<Expr> top = new List<Expr>(CurrentFunctionStaticDecls.Count + 1);
                            top.AddRange(CurrentFunctionStaticDecls);
                            top.Add(fnDef);
                            return MakeSequence(top);
                        }

                        return fnDef;
                    }
                    finally
                    {
                        CurrentFunctionNameForStaticLocals = prevStaticFnName;
                        CurrentFunctionStaticDecls = prevStaticDecls;
                    }

                }
                else
                {
                    ParseArrayDeclaration(ref type);
                    if (region.Tag == MemoryRegionTag.WramX && region.WramBank > 1 && !Program.IsCgbOnlyTargetRequested())
                    {
                        ParserError(ErrorCode.BankedWramRequiresCgbOnly, "banked WRAM requires #pragma rom_cgb cgb_only or --cgb=cgb_only");
                    }
                    if (TryParse(TokenType.EQUAL))
                    {
                        if (isExtern)
                        {
                            // `extern` with an initializer is a definition in C; we treat it as a normal definition here.
                            // (RAM initializers are still not supported.)
                            Program.Warning(SourcePosition, "warning: 'extern' on a definition has no effect");
                        }
                        // Allow initializers for:
                        // - __prg_rom (existing behavior)
                        // - const globals (new behavior):
                        // * scalar const => compile-time constant symbol ($constant)
                        // * array const => ROM data ($readonly_data) in current bank
                        if (type.IsConst && !type.IsArray)
                        {
                            // const scalar: treat as a compile-time constant, like `define`.
                            Expr valueExpr = ParseExpr();
                            Expect(TokenType.SEMICOLON);
                            {
                                Expr d = WrapPragmasIfNeeded(Make(Tag.Constant, type, name, valueExpr));
                                if (isStatic) d = Make(Tag.Static, d);
                                if (isUnsafeDecl) d = Make(Tag.Unsafe, d);
                                return d;
                            }
                        }

                        if (region.Tag == MemoryRegionTag.ProgramRom || type.IsConst)
                        {
                            List<Expr> values = new List<Expr>();
                            if (type.IsArray)
                            {
                                // Support: const u8 s[] = "ABC"; (string literal initializer for byte arrays)
                                // We interpret it as an ASCII byte array with a terminating NUL.
                                // If the array dimension is omitted ( [] ), infer it from the string length.
                                string strInit;
                                if (TryParseString(out strInit))
                                {
                                    // Only allow string init for u8 arrays (C "char" subset).
                                    CType elem = type.Subtype;
                                    if (!(elem.IsSimple && elem.SimpleType == CSimpleType.UInt8))
                                        ParserError(ErrorCode.ExpectedType, "string initializer is only supported for u8 arrays");

                                    // Convert to ASCII bytes + NUL.
                                    int inferredCount = strInit.Length + 1;
                                    for (int i = 0; i < strInit.Length; i++)
                                    {
                                        int c = strInit[i];
                                        if (c > 127) ParserError(ErrorCode.ParseError, "strings can only contain ASCII characters");
                                        values.Add(Make(Tag.Integer, c));
                                    }
                                    values.Add(Make(Tag.Integer, 0));

                                    // If dimension is omitted ([]), infer it.
                                    if (type.Tag == CTypeTag.ArrayWithDimensionExpression && type.DimensionExpression != null && type.DimensionExpression.Match(Tag.Empty))
                                    {
                                        bool wasConst = type.IsConst;
                                        type = CType.MakeArray(elem, inferredCount);
                                        type.IsConst = wasConst;
                                    }
                                    else if (type.Tag == CTypeTag.ArrayWithDimensionExpression)
                                    {
                                        // If a constant dimension is provided, validate it.
                                        if (type.DimensionExpression != null && type.DimensionExpression.Match(Tag.Integer, out int declared))
                                        {
                                            if (declared < inferredCount)
                                                ParserError(ErrorCode.ExpectedType, "string initializer too long for array (need {0}, declared {1})", inferredCount, declared);
                                            // Convert to fixed-dimension array type for downstream size checks.
                                            bool wasConst = type.IsConst;
                                            type = CType.MakeArray(elem, declared);
                                            type.IsConst = wasConst;
                                        }
                                        // Non-constant dimension expressions are not supported here.
                                        else if (type.DimensionExpression != null && !type.DimensionExpression.Match(Tag.Empty))
                                        {
                                            ParserError(ErrorCode.ParseError, "array dimension must be a constant expression when using string initializer");
                                        }
                                    }
                                }
                                else
                                {
                                // Parse a comma separated list; a final comma is allowed, but not required.
                                // NOTE: In normal C mode, newlines are whitespace and won't be tokenized.
                                // However, be defensive and tolerate NEWLINE tokens here too (e.g. if
                                // tokenization state is affected by __asm blocks in the same file).
                                Expect(TokenType.LBRACE);
                                while (TryParse(TokenType.NEWLINE)) { }

                                if (!TryParse(TokenType.RBRACE))
                                {
                                    while (true)
                                    {
                                        while (TryParse(TokenType.NEWLINE)) { }
                                        values.Add(ParseExpr());
                                        while (TryParse(TokenType.NEWLINE)) { }

                                        if (TryParse(TokenType.COMMA))
                                        {
                                            while (TryParse(TokenType.NEWLINE)) { }
                                            // Allow trailing comma: { 1, 2, 3, }
                                            if (TryParse(TokenType.RBRACE)) break;
                                            continue;
                                        }

                                        while (TryParse(TokenType.NEWLINE)) { }
                                        Expect(TokenType.RBRACE);
                                        break;
                                    }
                                }

                                // Numeric array initializer: infer omitted length and validate fixed literal length.
                                if (type.Tag == CTypeTag.ArrayWithDimensionExpression && type.DimensionExpression != null && type.DimensionExpression.Match(Tag.Empty))
                                {
                                    if (values.Count <= 0)
                                        ParserError(ErrorCode.ParseError, "cannot infer array size from empty initializer");
                                    bool wasConst = type.IsConst;
                                    type = CType.MakeArray(type.Subtype, values.Count);
                                    type.IsConst = wasConst;
                                }
                                else if (type.Tag == CTypeTag.ArrayWithDimensionExpression && type.DimensionExpression != null && type.DimensionExpression.Match(Tag.Integer, out int declaredCount))
                                {
                                    if (declaredCount < values.Count)
                                        ParserError(ErrorCode.ExpectedType, "too many initializers for array (got {0}, declared {1})", values.Count, declaredCount);

                                    bool wasConst = type.IsConst;
                                    type = CType.MakeArray(type.Subtype, declaredCount);
                                    type.IsConst = wasConst;
                                }
                                else if (type.Tag == CTypeTag.Array && type.Dimension < values.Count)
                                {
                                    ParserError(ErrorCode.ExpectedType, "too many initializers for array (got {0}, declared {1})", values.Count, type.Dimension);
                                }
                                }
                            }
                            else
                            {
                                values.Add(ParseExpr());
                            }

                            Expect(TokenType.SEMICOLON);
                            // const arrays without explicit __prg_rom are treated as ROM data.
                            // (We do not model RAM-backed const in this compiler.)
                            Expr rd = WrapPragmasIfNeeded(Make(Tag.ReadonlyData, type, name, values.ToArray()));
                            if (isStatic) rd = Make(Tag.Static, rd);
                            return rd;
                        }
                        else
                        {
                            ParserError("global variables cannot be initialized");
                            return null;
                        }
                    }
                    else
                    {
                        if (type.IsConst && !isExtern)
                        {
                            ParserError(ErrorCode.ParseError, "const global must be initialized");
                            return null;
                        }
                        Expect(TokenType.SEMICOLON);
                        // extern global variable declaration (no allocation)
                        if (isExtern)
                        {
                            Expr ev = (declRange != null) ? Make(Tag.ExternVariable, region, type, name, declRange) : Make(Tag.ExternVariable, region, type, name);
                            Expr wrappedEv = WrapPragmasIfNeeded(ev);
                            if (isStatic) wrappedEv = Make(Tag.Static, wrappedEv);
                            return wrappedEv;
                        }
                        Expr v = (declRange != null) ? Make(Tag.Variable, region, type, name, declRange) : Make(Tag.Variable, region, type, name);
                        Expr wrappedV = WrapPragmasIfNeeded(v);
                        if (isStatic) wrappedV = Make(Tag.Static, wrappedV);
                        return wrappedV;
                    }
                }
            }
        }
    }

    bool TryParseMemoryRegionQualifier(out MemoryRegion region)
    {
        if (TryParseName("__hram"))
        {
            region = MemoryRegion.HighMem;
            return true;
        }
        else if (TryParseName("__oam"))
        {
            region = MemoryRegion.Oam;
            return true;
        }
        else if (TryParseName("__wram"))
        {
            region = MemoryRegion.Ram;
            return true;
        }
        else if (TryParseName("__wramx_bank"))
        {
            Expect(TokenType.LPAREN);
            int bank = ExpectInt();
            Expect(TokenType.RPAREN);
            if (!ValidateWramXBankLiteral(bank, "__wramx_bank"))
                bank = 1;
            region = MemoryRegion.WramXBank(bank);
            return true;
        }
        else if (TryParseName("__wramx"))
        {
            region = MemoryRegion.WramX;
            return true;
        }
        else if (TryParseName("__prg_rom"))
        {
            region = MemoryRegion.ProgramRom;
            return true;
        }
        else if (TryParseName("__location"))
        {
            Expect(TokenType.LPAREN);
            int location = ExpectInt();
            Expect(TokenType.RPAREN);
            region = MemoryRegion.Fixed(location);
            return true;
        }
        else
        {
            region = MemoryRegion.Ram;
            return false;
        }
    }

    MemoryRegion ParseMemoryRegionQualifier()
    {
        MemoryRegion region;
        if (TryParseMemoryRegionQualifier(out region)) return ApplyPragmaWramXBank(region);
        // If the user didn't specify an explicit region attribute, inherit the current pragma region.
        return ApplyPragmaWramXBank(CurrentPragmaRegion);
    }

    void ParseArrayDeclaration(ref CType type)
    {
        if (TryParse(TokenType.LBRACKET))
        {
            Expr dimension;
            if (TryParse(TokenType.RBRACKET))
            {
                dimension = Expr.Make(Tag.Empty);
            }
            else
            {
                dimension = ParseExpr();
                Expect(TokenType.RBRACKET);
            }
            CType src = type;
            type = CType.MakeArray(src, dimension);
            type.IsConst = src.IsConst;
            type.IsRestrict = src.IsRestrict;
            type.IsEnumStrict = src.IsEnumStrict;
            type.IsBitFlags = src.IsBitFlags;
            type.IsSafeIndex = src.IsSafeIndex;
            type.ForcedAlign = src.ForcedAlign;
            type.IsPackedAggregate = src.IsPackedAggregate;
        }
    }

    // --- static_assert(expr, "msg") ---
    // keyword has already been consumed.
    Expr ParseStaticAssert(bool requireSemicolon)
    {
        Expect(TokenType.LPAREN);
        Expr cond = ParseExpr();
        string msg = "";
        if (TryParse(TokenType.COMMA))
        {
            if (!TryParseString(out msg))
                ParserError(ErrorCode.ParseError, "expected a string literal message for static_assert");
        }
        Expect(TokenType.RPAREN);
        if (requireSemicolon) Expect(TokenType.SEMICOLON);
        return Make(Tag.StaticAssert, cond, msg);
    }

    CType CloneType(CType type)
    {
        if (type == null) return null;
        return new CType
        {
            Tag = type.Tag,
            SimpleType = type.SimpleType,
            Name = type.Name,
            Subtype = type.Subtype,
            Dimension = type.Dimension,
            DimensionExpression = type.DimensionExpression,
            ParamTypes = type.ParamTypes,
            IsConst = type.IsConst,
            IsRestrict = type.IsRestrict,
            IsEnumStrict = type.IsEnumStrict,
            IsBitFlags = type.IsBitFlags,
            IsSafeIndex = type.IsSafeIndex,
            ForcedAlign = type.ForcedAlign,
            IsPackedAggregate = type.IsPackedAggregate,
        };
    }

    CType ApplyDeclarationAlignIfNeeded(CType type, int declAlign)
    {
        if (type == null) return null;
        if (declAlign <= 0 || type.ForcedAlign != 0) return type;
        CType t = CloneType(type);
        t.ForcedAlign = declAlign;
        return t;
    }

    Expr ParseConstexprDeclaration(int declAlign, bool isUnsafeDecl, bool isStaticDecl, string declFile)
    {
        CType type = ExpectType();
        type = ApplyDeclarationAlignIfNeeded(type, declAlign);

        string name = ExpectAnyName();
        if (isStaticDecl) name = GetOrCreateStaticSymbolName(declFile, name);
        ParseArrayDeclaration(ref type);
        if (type.IsArray || type.IsStructOrUnion || type.IsFunction)
        {
            ParserError("constexpr currently supports only scalar and enum types");
        }

        if (!type.IsConst)
        {
            type = CloneType(type);
            type.IsConst = true;
        }

        Expect(TokenType.EQUAL);
        Expr value = ParseExpr();
        Expect(TokenType.SEMICOLON);

        Expr d = WrapPragmasIfNeeded(Make(Tag.Constant, type, name, value));
        if (isStaticDecl) d = Make(Tag.Static, d);
        if (isUnsafeDecl) d = Make(Tag.Unsafe, d);
        return d;
    }


    // If false, only allow statements that would fit in a "for" initializer.
    Expr ParseStatement(bool allowLong)
    {
        MemoryRegion __localRegion;
        if (TryParseMemoryRegionQualifier(out __localRegion))
        {
            if (__localRegion.Tag == MemoryRegionTag.WramX && __localRegion.WramBank > 0)
                ParserError(ErrorCode.BankedWramLocalNotAllowed, "banked WRAM is allowed only for globals/file-statics in MVP");
            else
                ParserError("memory region qualifiers are only allowed on global variables");
        }

        bool localStatic = false;
        while (TryParseName("static"))
        {
            localStatic = true;
        }


        // --- static_assert(expr, "msg") ---
        if (TryParseName("static_assert") || TryParseName("_Static_assert"))
        {
            if (!allowLong) Error_NotAllowedInFor();
            return ParseStaticAssert(true);
        }

        if (TryParseName("constexpr"))
        {
            ParserError("constexpr declarations are currently supported only at file scope");
        }

        // --- __range(min,max) for local declarations ---
        // Optional integer range annotation for local declarations
        Expr stmtRange = null;
        {
            Expr r;
            if (TryParseRangeAttribute(out r)) stmtRange = r;
        }

        // --- __unsafe block/statement ---
        // Syntax:
        // __unsafe { ... }
        // __unsafe <statement>;
        // This is a zero-runtime-cost marker used to confine debug-only checks.
        if (TryParseName("__unsafe"))
        {
            Expr inner = ParseStatementBlock();
            return Make(Tag.Unsafe, inner);
        }

        CType type;
        if (TryParseType(out type))
        {
            // Declare a local variable:
            return ParseRestOfLocalDeclaration(type, stmtRange, localStatic);
        }
        else if (localStatic)
        {
            ParserError("expected a local declaration after 'static'");
            return null;
        }
        else if (TryParseName("if"))
        {
            if (!allowLong) Error_NotAllowedInFor();

            List<object> parts = new List<object>();
            parts.Add(Tag.If);

            Expect(TokenType.LPAREN);
            parts.Add(ParseExpr());
            Expect(TokenType.RPAREN);
            parts.Add(ParseStatementBlock());

            // Parse additional else statements:
            while (TryParseName("else"))
            {
                if (TryParseName("if"))
                {
                    Expect(TokenType.LPAREN);
                    parts.Add(ParseExpr());
                    Expect(TokenType.RPAREN);
                    parts.Add(ParseStatementBlock());
                }
                else
                {
                    parts.Add(Make(Tag.Integer, 1));
                    parts.Add(ParseStatementBlock());

                    // An "else" that is not an "else if" means it's time to stop:
                    break;
                }
            }

            return Make(parts.ToArray());
        }
        else if (TryParseName("for"))
        {
            if (!allowLong) Error_NotAllowedInFor();

            Expect(TokenType.LPAREN);
            Expr init = ParseStatement(false);
            Expr test = ParseExpr();
            Expect(TokenType.SEMICOLON);
            Expr induct = ParseExpr();
            Expect(TokenType.RPAREN);
            Expr body = ParseStatementBlock();
            return Make(Tag.For, init, test, induct, body);
        }
        else if (TryParseName("while"))
        {
            if (!allowLong) Error_NotAllowedInFor();

            Expect(TokenType.LPAREN);
            Expr test = ParseExpr();
            Expect(TokenType.RPAREN);
            Expr body = ParseStatementBlock();
            return Make(Tag.For, Make(Tag.Empty), test, Make(Tag.Empty), body);
        }
        else if (TryParseName("do"))
        {
            if (!allowLong) Error_NotAllowedInFor();

            Expr body = ParseStatementBlock();

            if (!TryParseName("while")) ParserError("expected 'while' after do-statement body");
            Expect(TokenType.LPAREN);
            Expr test = ParseExpr();
            Expect(TokenType.RPAREN);
            Expect(TokenType.SEMICOLON);

            return Make(Tag.DoWhile, body, test);
        }

        else if (TryParseName("continue"))
        {
            if (!allowLong) Error_NotAllowedInFor();
            Expect(TokenType.SEMICOLON);
            return Make(Tag.Continue);
        }
        else if (TryParseName("break"))
        {
            if (!allowLong) Error_NotAllowedInFor();
            Expect(TokenType.SEMICOLON);
            return Make(Tag.Break);
        }
        else if (TryParseName("fallthrough") || TryParseName("__fallthrough"))
        {
            if (!allowLong) Error_NotAllowedInFor();
            if (SwitchDepth <= 0)
            {
                ParserError("'fallthrough' is only valid inside switch case blocks");
            }
            Expect(TokenType.SEMICOLON);
            return Make(Tag.Fallthrough);
        }
        else if (TryParseName("return"))
        {
            if (!allowLong) Error_NotAllowedInFor();

            if (TryParse(TokenType.SEMICOLON))
            {
                return Make(Tag.Return);
            }
            else
            {
                Expr result = ParseExpr();
                Expect(TokenType.SEMICOLON);
                return Make(Tag.Return, result);
            }
        }
        else if (TryParseName("goto"))
        {
            string label = ExpectAnyName();
            Expect(TokenType.SEMICOLON);
            return Make(Tag.Jump, label);
        }
        else if (TryParseName("__asm"))
        {
            if (!allowLong) Error_NotAllowedInFor();

            while (TryParse(TokenType.NEWLINE)) { }

            Expect(TokenType.LBRACE);
            List<Expr> parts = new List<Expr>();

            while (!TryParse(TokenType.RBRACE))
            {
                while (TryParse(TokenType.NEWLINE) || TryParse(TokenType.SEMICOLON)) { }

                if (PeekToken().Tag == TokenType.RBRACE) continue;

                string symbol = ExpectAnyName();

                if (TryParse(TokenType.COLON))
                {
                    parts.Add(Expr.Make(Tag.Label, symbol));
                }
                else if (TryParse(TokenType.NUMBER_SIGN))
                {
                    AsmOperand operand = ParseAssemblyOperand(AddressMode.Immediate);
                    parts.Add(Expr.MakeAsm(symbol, operand));
                }
                else if (TryParse(TokenType.PLUS))
                {
                    AsmOperand operand = ParseAssemblyOperand(AddressMode.Relative);
                    parts.Add(Expr.MakeAsm(symbol, operand));
                }
                else if (TryParse(TokenType.LPAREN))
                {
                    AsmOperand operand = ParseAssemblyOperand(AddressMode.Indirect);
                    if (TryParse(TokenType.RPAREN))
                    {
                        if (TryParse(TokenType.COMMA))
                        {
                            ExpectKeyword("Y");
                            operand = operand.WithMode(AddressMode.IndirectY);
                        }
                    }
                    else if (TryParse(TokenType.COMMA))
                    {
                        ExpectKeyword("X");
                        Expect(TokenType.RPAREN);
                        operand = operand.WithMode(AddressMode.IndirectX);
                    }
                    else
                    {
                        ParserError("expected (zp,X) or (zp),Y operand");
                    }
                    parts.Add(Expr.MakeAsm(symbol, operand));
                }
                else if (PeekToken().Tag == TokenType.NEWLINE ||
                         PeekToken().Tag == TokenType.SEMICOLON ||
                         PeekToken().Tag == TokenType.RBRACE)
                {
                    parts.Add(Expr.MakeAsm(symbol));
                }
                else
                {
                    AsmOperand operand = ParseAssemblyOperand(AddressMode.Absolute);
                    if (TryParse(TokenType.COMMA))
                    {
                        if (TryParseName("X")) operand = operand.WithMode(AddressMode.AbsoluteX);
                        else if (TryParseName("Y")) operand = operand.WithMode(AddressMode.AbsoluteY);
                        else ParserError("expected 'X' or 'Y'");
                    }
                    parts.Add(Expr.MakeAsm(symbol, operand));
                }
            }
            return MakeSequence(parts);
        }
        else if (Input.Count >= 2 && Input[0].Tag == TokenType.NAME && Input[1].Tag == TokenType.COLON)
        {
            string label = ExpectAnyName();
            Expect(TokenType.COLON);
            return Make(Tag.Label, label);
        }
        else if (TryParseName("switch"))
        {
            if (!allowLong) Error_NotAllowedInFor();
            return ParseSwitch();
        }
        else if (PeekToken().Tag == TokenType.LBRACE)
        {
            // Standalone block statement: { ... }
            // (Allowed anywhere in C; previously only handled as a body of if/for/while/do.)
            if (!allowLong) Error_NotAllowedInFor();
            return ParseStatementBlock();
        }
        else
        {
            // An expression-statement.
            // Support comma-separated multi-assign/expr lists like: a=1, b=2;
            // (Deliberately does not change the general expression grammar, so function-call argument
            // parsing continues to use ',' as a separator.)
            List<Expr> parts = new List<Expr>();
            parts.Add(ParseAssignExpr());
            while (TryParse(TokenType.COMMA))
            {
                parts.Add(ParseAssignExpr());
            }

            Expect(TokenType.SEMICOLON);
            if (parts.Count == 1) return parts[0];
            return MakeSequence(parts);
        }
    }

    Expr ParseRestOfLocalDeclaration(CType type, Expr range, bool isStaticLocal = false)
    {
        // Support comma-separated local declarations, e.g.
        // u8 a=1, b=2;
        // Each declarator may have its own array suffix and optional initializer.

        CType baseType = type;
        List<Expr> decls = new List<Expr>();

        while (true)
        {
            CType varType = baseType;
            string sourceName = ExpectAnyName();
            string resolvedName = sourceName;
            if (isStaticLocal)
            {
                if (CurrentFunctionStaticDecls == null || string.IsNullOrEmpty(CurrentFunctionNameForStaticLocals))
                    ParserError("function-local 'static' is only supported inside function bodies");

                resolvedName = GetFunctionLocalStaticSymbolName(CurrentFunctionNameForStaticLocals, sourceName);
                RegisterLocalAlias(sourceName, resolvedName);
            }
            else
            {
                RegisterLocalName(sourceName);
            }

            ParseArrayDeclaration(ref varType);
            List<Expr> initStatements = null;

            // Optionally, parse C-style initializer (including { ... } / .field = ...).
            if (TryParse(TokenType.EQUAL))
            {
                initStatements = new List<Expr>();
                Expr target = Expr.Make(Tag.Name, resolvedName).WithSource(SourcePosition);
                ParseLocalInitializerInto(ref varType, target, initStatements);
            }

            if (!isStaticLocal)
            {
                Expr decl = (range != null) ? Make(Tag.Variable, varType, sourceName, range) : Make(Tag.Variable, varType, sourceName);
                if (initStatements == null || initStatements.Count == 0)
                {
                    decls.Add(decl);
                }
                else
                {
                    List<Expr> seq = new List<Expr>(1 + initStatements.Count);
                    seq.Add(decl);
                    seq.AddRange(initStatements);
                    decls.Add(MakeSequence(seq));
                }
            }
            else
            {
                Expr storageDecl = Make(Tag.Static, Make(Tag.Variable, MemoryRegion.Ram, varType, resolvedName));
                if (CurrentFunctionStaticDecls != null) CurrentFunctionStaticDecls.Add(storageDecl);

                if (initStatements != null && initStatements.Count != 0)
                {
                    // C-like one-time initialization for function-local static variables.
                    string guardName = resolvedName + "__init";
                    if (CurrentFunctionStaticDecls != null)
                        CurrentFunctionStaticDecls.Add(Make(Tag.Static, Make(Tag.Variable, MemoryRegion.Ram, CType.UInt8, guardName)));

                    List<Expr> initOnce = new List<Expr>(initStatements.Count + 1);
                    initOnce.AddRange(initStatements);
                    initOnce.Add(Make(Tag.Assign, Make(Tag.Name, guardName), Make(Tag.Integer, 1)));

                    Expr cond = Make(Tag.Equal, Make(Tag.Name, guardName), Make(Tag.Integer, 0));
                    decls.Add(Make(Tag.If, cond, MakeSequence(initOnce)));
                }
            }

            if (!TryParse(TokenType.COMMA)) break;
        }

        Expect(TokenType.SEMICOLON);
        if (decls.Count == 0) return Make(Tag.Empty);
        if (decls.Count == 1) return decls[0];
        return MakeSequence(decls);
    }

    void ParseLocalInitializerInto(ref CType type, Expr target, List<Expr> outStmts)
    {
        if (type != null && type.IsArray)
        {
            ParseLocalArrayInitializerInto(ref type, target, outStmts);
            return;
        }

        if (type != null && type.IsStructOrUnion)
        {
            ParseLocalStructOrUnionInitializerInto(type, target, outStmts);
            return;
        }

        ParseLocalScalarInitializerInto(target, outStmts);
    }

    void ParseLocalScalarInitializerInto(Expr target, List<Expr> outStmts)
    {
        Expr value = ParseScalarInitializerValue();
        outStmts.Add(Make(Tag.Assign, target, value));
    }

    Expr ParseScalarInitializerValue()
    {
        if (!TryParse(TokenType.LBRACE))
            return ParseExpr();

        if (TryParse(TokenType.RBRACE))
            return Make(Tag.Integer, 0);

        Expr value = ParseScalarInitializerValue();
        while (TryParse(TokenType.COMMA))
        {
            if (TryParse(TokenType.RBRACE)) return value;
            ParserError(ErrorCode.ParseError, "too many initializers for scalar");
        }
        Expect(TokenType.RBRACE);
        return value;
    }

    void ParseLocalArrayInitializerInto(ref CType arrayType, Expr target, List<Expr> outStmts)
    {
        CType elemType = arrayType.Subtype ?? CType.UInt8;
        List<Expr> explicitInits = new List<Expr>();
        int maxIndex = -1;

        bool isUnsized;
        int declaredLength = GetDeclaredArrayLength(arrayType, out isUnsized);

        // String literal initializer for local u8 arrays.
        string s;
        if (TryParseString(out s))
        {
            if (!(elemType.IsSimple && elemType.SimpleType == CSimpleType.UInt8))
                ParserError(ErrorCode.ExpectedType, "string initializer is only supported for u8 arrays");

            for (int i = 0; i < s.Length; i++)
            {
                int c = s[i];
                if (c > 127) ParserError(ErrorCode.ParseError, "strings can only contain ASCII characters");
                Expr dst = Make(Tag.Index, target, Make(Tag.Integer, i));
                explicitInits.Add(Make(Tag.Assign, dst, Make(Tag.Integer, c)));
                maxIndex = i;
            }

            int nulIndex = s.Length;
            explicitInits.Add(Make(Tag.Assign, Make(Tag.Index, target, Make(Tag.Integer, nulIndex)), Make(Tag.Integer, 0)));
            maxIndex = nulIndex;
        }
        else
        {
            int nextIndex = 0;
            if (TryParse(TokenType.LBRACE))
            {
                if (!TryParse(TokenType.RBRACE))
                {
                    while (true)
                    {
                        Expr dst = Make(Tag.Index, target, Make(Tag.Integer, nextIndex));
                        CType itemType = elemType;
                        ParseLocalInitializerInto(ref itemType, dst, explicitInits);
                        maxIndex = Math.Max(maxIndex, nextIndex);
                        nextIndex++;

                        if (TryParse(TokenType.COMMA))
                        {
                            if (TryParse(TokenType.RBRACE)) break;
                            continue;
                        }
                        Expect(TokenType.RBRACE);
                        break;
                    }
                }
            }
            else
            {
                Expr dst = Make(Tag.Index, target, Make(Tag.Integer, 0));
                CType itemType = elemType;
                ParseLocalInitializerInto(ref itemType, dst, explicitInits);
                maxIndex = 0;
            }
        }

        int inferredLength = Math.Max(0, maxIndex + 1);
        if (isUnsized)
        {
            if (inferredLength <= 0)
                ParserError(ErrorCode.ParseError, "cannot infer array size from empty initializer");
            arrayType = BuildArrayTypeWithDimension(arrayType, inferredLength);
            declaredLength = inferredLength;
        }
        else if (declaredLength >= 0 && inferredLength > declaredLength)
        {
            ParserError(ErrorCode.ExpectedType, "too many initializers for array (got {0}, declared {1})", inferredLength, declaredLength);
        }

        if (declaredLength >= 0)
        {
            for (int i = 0; i < declaredLength; i++)
            {
                Expr dst = Make(Tag.Index, target, Make(Tag.Integer, i));
                AppendZeroInitializerForType(elemType, dst, outStmts);
            }
        }

        outStmts.AddRange(explicitInits);
    }

    void ParseLocalStructOrUnionInitializerInto(CType aggType, Expr target, List<Expr> outStmts)
    {
        if (!TryGetAggregateDecl(aggType, out AggregateDeclInfo info) || info.Fields == null || info.Fields.Length == 0)
            ParserError(ErrorCode.IncompleteType, "initializer requires a complete struct/union type: {0}", aggType.Show());

        List<Expr> explicitInits = new List<Expr>();
        int nextField = 0;
        int unionCount = 0;

        bool hasBrace = TryParse(TokenType.LBRACE);
        if (hasBrace)
        {
            if (!TryParse(TokenType.RBRACE))
            {
                while (true)
                {
                    Expr dst;
                    CType dstType;
                    int topFieldIndex;

                    if (TryParse(TokenType.PERIOD))
                    {
                        ParseDesignatedFieldPath(aggType, target, out dst, out dstType, out topFieldIndex);
                    }
                    else
                    {
                        if (info.IsUnion)
                        {
                            if (unionCount > 0) ParserError(ErrorCode.ParseError, "too many initializers for union");
                            topFieldIndex = 0;
                        }
                        else
                        {
                            topFieldIndex = nextField;
                            if (topFieldIndex >= info.Fields.Length)
                                ParserError(ErrorCode.ParseError, "too many initializers for struct");
                        }

                        FieldInfo f = info.Fields[topFieldIndex];
                        dst = Make(Tag.Field, target, f.Name);
                        dstType = f.Type;
                    }

                    CType initType = dstType;
                    ParseLocalInitializerInto(ref initType, dst, explicitInits);

                    if (info.IsUnion)
                    {
                        unionCount++;
                        if (unionCount > 1) ParserError(ErrorCode.ParseError, "too many initializers for union");
                    }
                    else
                    {
                        nextField = Math.Max(nextField, topFieldIndex + 1);
                    }

                    if (TryParse(TokenType.COMMA))
                    {
                        if (TryParse(TokenType.RBRACE)) break;
                        continue;
                    }

                    Expect(TokenType.RBRACE);
                    break;
                }
            }
        }
        else
        {
            // Brace elision: initialize first field, or designated path.
            Expr dst;
            CType dstType;
            int topFieldIndex;

            if (TryParse(TokenType.PERIOD))
            {
                ParseDesignatedFieldPath(aggType, target, out dst, out dstType, out topFieldIndex);
            }
            else
            {
                if (info.Fields.Length == 0)
                    ParserError(ErrorCode.IncompleteType, "initializer requires a complete struct/union type: {0}", aggType.Show());
                topFieldIndex = 0;
                FieldInfo f = info.Fields[0];
                dst = Make(Tag.Field, target, f.Name);
                dstType = f.Type;
            }

            CType initType = dstType;
            ParseLocalInitializerInto(ref initType, dst, explicitInits);
            if (info.IsUnion) unionCount = 1;
            else nextField = Math.Max(nextField, topFieldIndex + 1);
        }

        // C aggregate initializer semantics: unspecified members are zero-initialized.
        if (info.IsUnion)
        {
            if (info.Fields.Length > 0)
            {
                Expr first = Make(Tag.Field, target, info.Fields[0].Name);
                AppendZeroInitializerForType(info.Fields[0].Type, first, outStmts);
            }
        }
        else
        {
            for (int i = 0; i < info.Fields.Length; i++)
            {
                Expr dst = Make(Tag.Field, target, info.Fields[i].Name);
                AppendZeroInitializerForType(info.Fields[i].Type, dst, outStmts);
            }
        }

        outStmts.AddRange(explicitInits);
    }

    void ParseDesignatedFieldPath(CType rootType, Expr rootTarget, out Expr dst, out CType dstType, out int topFieldIndex)
    {
        if (!TryGetAggregateDecl(rootType, out AggregateDeclInfo info) || info.Fields == null || info.Fields.Length == 0)
            ParserError(ErrorCode.IncompleteType, "designated initializer requires a complete struct/union type: {0}", rootType.Show());

        string first = ExpectAnyName();
        if (!TryFindField(info.Fields, first, out FieldInfo field0, out topFieldIndex))
            ParserError(ErrorCode.ParseError, "unknown field in designated initializer: {0}", first);

        dst = Make(Tag.Field, rootTarget, first);
        dstType = field0.Type;

        while (TryParse(TokenType.PERIOD))
        {
            string part = ExpectAnyName();
            if (!TryGetAggregateDecl(dstType, out AggregateDeclInfo nested) || nested.Fields == null || nested.Fields.Length == 0)
                ParserError(ErrorCode.ParseError, "designated path member is not a struct/union: {0}", part);

            if (!TryFindField(nested.Fields, part, out FieldInfo nf, out int _))
                ParserError(ErrorCode.ParseError, "unknown field in designated initializer: {0}", part);

            dst = Make(Tag.Field, dst, part);
            dstType = nf.Type;
        }

        Expect(TokenType.EQUAL);
    }

    void AppendZeroInitializerForType(CType type, Expr target, List<Expr> outStmts)
    {
        if (type == null)
        {
            outStmts.Add(Make(Tag.Assign, target, Make(Tag.Integer, 0)));
            return;
        }

        if (type.IsArray)
        {
            bool isUnsized;
            int n = GetDeclaredArrayLength(type, out isUnsized);
            if (n < 0) return;

            CType elem = type.Subtype ?? CType.UInt8;
            for (int i = 0; i < n; i++)
            {
                Expr dst = Make(Tag.Index, target, Make(Tag.Integer, i));
                AppendZeroInitializerForType(elem, dst, outStmts);
            }
            return;
        }

        if (type.IsStructOrUnion && TryGetAggregateDecl(type, out AggregateDeclInfo info) && info.Fields != null && info.Fields.Length > 0)
        {
            if (info.IsUnion)
            {
                Expr first = Make(Tag.Field, target, info.Fields[0].Name);
                AppendZeroInitializerForType(info.Fields[0].Type, first, outStmts);
                return;
            }

            for (int i = 0; i < info.Fields.Length; i++)
            {
                Expr dst = Make(Tag.Field, target, info.Fields[i].Name);
                AppendZeroInitializerForType(info.Fields[i].Type, dst, outStmts);
            }
            return;
        }

        outStmts.Add(Make(Tag.Assign, target, Make(Tag.Integer, 0)));
    }

    int GetDeclaredArrayLength(CType arrayType, out bool isUnsized)
    {
        isUnsized = false;
        if (arrayType == null || !arrayType.IsArray) return -1;

        if (arrayType.Tag == CTypeTag.Array) return arrayType.Dimension;
        if (arrayType.Tag == CTypeTag.ArrayWithDimensionExpression)
        {
            if (arrayType.DimensionExpression == null || arrayType.DimensionExpression.Match(Tag.Empty))
            {
                isUnsized = true;
                return -1;
            }
            if (arrayType.DimensionExpression.Match(Tag.Integer, out int n)) return n;
        }
        return -1;
    }

    CType BuildArrayTypeWithDimension(CType src, int dim)
    {
        CType t = CType.MakeArray(src.Subtype ?? CType.UInt8, dim);
        t.IsConst = src.IsConst;
        t.IsRestrict = src.IsRestrict;
        t.IsEnumStrict = src.IsEnumStrict;
        t.IsBitFlags = src.IsBitFlags;
        t.IsSafeIndex = src.IsSafeIndex;
        t.ForcedAlign = src.ForcedAlign;
        t.IsPackedAggregate = src.IsPackedAggregate;
        return t;
    }

    void RegisterAggregateDecl(string name, bool isUnion, FieldInfo[] fields, bool isPacked, int forcedAlign)
    {
        if (string.IsNullOrEmpty(name)) return;
        AggregateDecls[name] = new AggregateDeclInfo
        {
            IsUnion = isUnion,
            Fields = fields ?? Array.Empty<FieldInfo>(),
            IsPacked = isPacked,
            ForcedAlign = forcedAlign,
        };
    }

    bool TryGetAggregateDecl(CType type, out AggregateDeclInfo info)
    {
        if (type == null || !type.IsStructOrUnion || string.IsNullOrEmpty(type.Name))
        {
            info = null;
            return false;
        }
        return AggregateDecls.TryGetValue(type.Name, out info);
    }

    bool TryFindField(FieldInfo[] fields, string name, out FieldInfo field, out int index)
    {
        for (int i = 0; i < fields.Length; i++)
        {
            if (fields[i].Name == name)
            {
                field = fields[i];
                index = i;
                return true;
            }
        }
        field = null;
        index = -1;
        return false;
    }

    AsmOperand ParseAssemblyOperand(AddressMode mode)
    {
        ImmediateModifier modifier = ImmediateModifier.None;
        if (TryParse(TokenType.LESS_THAN)) modifier = ImmediateModifier.LowByte;
        else if (TryParse(TokenType.GREATER_THAN)) modifier = ImmediateModifier.HighByte;

        bool bracketedAbsolute = false;
        if (mode == AddressMode.Absolute && TryParse(TokenType.LBRACKET))
        {
            bracketedAbsolute = true;
        }

        AsmOperand operand;
        string name;
        int number;
        if (TryParseInt(out number))
        {
            operand = new AsmOperand(Maybe.Nothing, number, mode, modifier);
        }
        else if (TryParseAnyName(out name))
        {
            number = 0;
            if (TryParse(TokenType.PLUS))
            {
                number = ExpectInt();
            }
            operand = new AsmOperand(name, number, mode, modifier);
        }
        else
        {
            ParserError("expected an operand");
            return null;
        }

        if (bracketedAbsolute)
        {
            Expect(TokenType.RBRACKET);
        }

        return operand;
    }

    void Error_NotAllowedInFor()
    {
        ParserError("complex statements are not allowed in for initializers");
    }

    Expr ParseStatementBlock()
    {
        // A block can be a single statement, or a series of statements surrounded by braces:
        if (TryParse(TokenType.LBRACE))
        {
            List<Expr> parts = new List<Expr>();
            PushLocalScope();
            try
            {
                while (!TryParse(TokenType.RBRACE))
                {
                    try
                    {
                        parts.Add(ParseStatement(true));
                    }
                    catch (RecoverableParseException)
                    {
                        SynchronizeStatementBoundary(stopAtSwitchCaseBoundary: false);
                        if (PeekToken().Tag == TokenType.EOF) break;
                    }
                }
            }
            finally
            {
                PopLocalScope();
            }
            return MakeSequence(parts);
        }
        else
        {
            return ParseStatement(true);
        }
    }

    Expr ParseExpr()
    {
        return ParseCommaExpr();
    }

    // ,
    Expr ParseCommaExpr()
    {
        return ParseAssignExpr();
    }

    // = *= /= %= += -= <<= >>= &= ^= |=
    Expr ParseAssignExpr()
    {
        Dictionary<TokenType, string> modifyAssignOperators = new Dictionary<TokenType, string>
        {
            { TokenType.PLUS_EQUALS, Tag.Add },
            { TokenType.MINUS_EQUALS, Tag.Subtract },
            { TokenType.STAR_EQUALS, Tag.Multiply },
            { TokenType.SLASH_EQUALS, Tag.Divide },
            { TokenType.PERCENT_EQUALS, Tag.Modulus },
            { TokenType.SHIFT_LEFT_EQUALS, Tag.ShiftLeft },
            { TokenType.SHIFT_RIGHT_EQUALS, Tag.ShiftRight },
            { TokenType.PIPE_EQUALS, Tag.BitwiseOr },
            { TokenType.AMPERSAND_EQUALS, Tag.BitwiseAnd },
            { TokenType.CARET_EQUALS, Tag.BitwiseXor },
        };

        Expr left = ParseConditionalExpr();
        string op;
        if (TryParse(TokenType.EQUAL))
        {
            return Make(Tag.Assign, left, ParseAssignExpr());
        }
        else if (modifyAssignOperators.TryGetValue(PeekToken().Tag, out op))
        {
            ConsumeToken();
            return Make(Tag.AssignModify, op, left, ParseAssignExpr());
        }
        else
        {
            return left;
        }
    }

    // ? :
    Expr ParseConditionalExpr()
    {
        Expr e = ParseLogicalOrExpr();
        if (TryParse(TokenType.QUESTION_MARK))
        {
            Expr trueCase = ParseExpr();
            Expect(TokenType.COLON);
            Expr falseCase = ParseConditionalExpr();
            e = Make(Tag.Conditional, e, trueCase, falseCase);
        }
        return e;
    }

    // ||
    Expr ParseLogicalOrExpr()
    {
        Expr e = ParseLogicalAndExpr();
        while (TryParse(TokenType.LOGICAL_OR))
        {
            Expr right = ParseLogicalAndExpr();
            e = Make(Tag.LogicalOr, e, right);
        }
        return e;
    }

    // &&
    Expr ParseLogicalAndExpr()
    {
        Expr e = ParseBitwiseOrExpr();
        while (TryParse(TokenType.LOGICAL_AND))
        {
            Expr right = ParseBitwiseOrExpr();
            e = Make(Tag.LogicalAnd, e, right);
        }
        return e;
    }

    // |
    Expr ParseBitwiseOrExpr()
    {
        Expr e = ParseBitwiseXorExpr();
        while (TryParse(TokenType.PIPE))
        {
            Expr right = ParseBitwiseXorExpr();
            e = Make(Tag.BitwiseOr, e, right);
        }
        return e;
    }

    // ^
    Expr ParseBitwiseXorExpr()
    {
        Expr e = ParseBitwiseAndExpr();
        while (TryParse(TokenType.CARET))
        {
            Expr right = ParseBitwiseAndExpr();
            e = Make(Tag.BitwiseXor, e, right);
        }
        return e;
    }

    // &
    Expr ParseBitwiseAndExpr()
    {
        Expr e = ParseEqualityExpr();
        while (TryParse(TokenType.AMPERSAND))
        {
            Expr right = ParseEqualityExpr();
            e = Make(Tag.BitwiseAnd, e, right);
        }
        return e;
    }

    // == !=
    Expr ParseEqualityExpr()
    {
        Dictionary<TokenType, string> operators = new Dictionary<TokenType, string>
        {
            { TokenType.DOUBLE_EQUAL, Tag.Equal },
            { TokenType.NOT_EQUAL, Tag.NotEqual },
        };

        return ParseInfixOperators(ParseCompareExpr, operators);
    }

    // < > <= >=
    Expr ParseCompareExpr()
    {
        Dictionary<TokenType, string> operators = new Dictionary<TokenType, string>
        {
            { TokenType.LESS_THAN, Tag.LessThan },
            { TokenType.LESS_THAN_OR_EQUAL, Tag.LessThanOrEqual },
            { TokenType.GREATER_THAN, Tag.GreaterThan },
            { TokenType.GREATER_THAN_OR_EQUAL, Tag.GreaterThanOrEqual },
        };

        return ParseInfixOperators(ParseShiftExpr, operators);
    }

    // << >>
    Expr ParseShiftExpr()
    {
        Dictionary<TokenType, string> operators = new Dictionary<TokenType, string>
        {
            { TokenType.SHIFT_LEFT, Tag.ShiftLeft },
            { TokenType.SHIFT_RIGHT, Tag.ShiftRight },
        };

        return ParseInfixOperators(ParseAddExpr, operators);
    }

    // + -
    Expr ParseAddExpr()
    {
        Dictionary<TokenType, string> operators = new Dictionary<TokenType, string>
        {
            { TokenType.PLUS, Tag.Add },
            { TokenType.MINUS, Tag.Subtract },
        };

        return ParseInfixOperators(ParseMultiplyExpr, operators);
    }

    // * / %
    Expr ParseMultiplyExpr()
    {
        Dictionary<TokenType, string> operators = new Dictionary<TokenType, string>
        {
            { TokenType.STAR, Tag.Multiply },
            { TokenType.SLASH, Tag.Divide },
            { TokenType.PERCENT, Tag.Modulus },
        };

        return ParseInfixOperators(ParseUnaryPrefixExpr, operators);
    }

    // Unary prefix operators
    Expr ParseUnaryPrefixExpr()
    {
        // offsetof(type, member)
        // We parse this as a compile-time expression node. Evaluation is handled in CodeGenerator constant evaluator.
        if (TryParseName("offsetof") || TryParseName("__builtin_offsetof"))
        {
            Expect(TokenType.LPAREN);
            CType ty = ExpectType();
            Expect(TokenType.COMMA);

            // member-designator: ident ('.' ident)*
            List<string> parts = new List<string>();
            Token t = PeekToken();
            Expect(TokenType.NAME);
            if (t.Name == null) ParserError(ErrorCode.ParseError, "expected member name for offsetof");
            parts.Add(t.Name ?? "");
            while (TryParse(TokenType.PERIOD))
            {
                Token t2 = PeekToken();
                Expect(TokenType.NAME);
                if (t2.Name == null) ParserError(ErrorCode.ParseError, "expected member name after '.' in offsetof");
                parts.Add(t2.Name ?? "");
            }

            Expect(TokenType.RPAREN);
            return Make(Tag.Offsetof, ty, string.Join(".", parts));
        }

        // sizeof(type) / sizeof expr
        // Supports typedef names as types.
        if (TryParseName("sizeof"))
        {
            // sizeof ( ... ) form
            if (TryParse(TokenType.LPAREN))
            {
                // Heuristic: if next token starts a type (builtin/struct/union/typedef), parse sizeof(type).
                Token t0 = PeekToken();
                bool looksType = false;
                if (t0.Tag == TokenType.NAME && t0.Name != null)
                {
                    if (t0.Name == "void" || t0.Name == "uint8_t" || t0.Name == "u8" || t0.Name == "char" || t0.Name == "bool" ||
                        t0.Name == "uint16_t" || t0.Name == "u16" || t0.Name == "struct" || t0.Name == "union" ||
                        Typedefs.ContainsKey(t0.Name))
                    {
                        looksType = true;
                    }
                }

                if (looksType)
                {
                    CType ty = ExpectType();
                    Expect(TokenType.RPAREN);
                    return Make(Tag.Sizeof, ty);
                }
                else
                {
                    Expr inner = ParseExpr();
                    Expect(TokenType.RPAREN);
                    return Make(Tag.Sizeof, inner);
                }
            }
            else
            {
                // sizeof unary-expr form
                Expr sub = ParseUnaryPrefixExpr();
                return Make(Tag.Sizeof, sub);
            }
        }

        Dictionary<TokenType, string> prefixes = new Dictionary<TokenType, string>
        {
            { TokenType.STAR, Tag.Load },
            { TokenType.AMPERSAND, Tag.AddressOf },
            { TokenType.TILDE, Tag.BitwiseNot },
            { TokenType.LOGICAL_NOT, Tag.LogicalNot },
            { TokenType.INCREMENT, Tag.PreIncrement },
            { TokenType.DECREMENT, Tag.PreDecrement },
        };

        TokenType nextToken = PeekToken().Tag;
        string op;
        if (prefixes.TryGetValue(nextToken, out op))
        {
            ConsumeToken();
            Expr e = ParseUnaryPrefixExpr();
            return Make(op, e);
        }
        else
        {
            return ParseSuffixExpr();
        }
    }

    // Suffix operators
    Expr ParseSuffixExpr()
    {
        Expr e = ParsePrimaryExpr();
        while (true)
        {
            if (TryParse(TokenType.LPAREN))
            {
                List<object> parts = new List<object>();
                parts.Add(Tag.Call);
                parts.Add(e);

                if (!TryParse(TokenType.RPAREN))
                {
                    while (true)
                    {
                        parts.Add(ParseExpr());
                        if (TryParse(TokenType.RPAREN)) break;
                        Expect(TokenType.COMMA);
                    }
                }

                e = Expr.Make(parts.ToArray());
            }
            else if (TryParse(TokenType.PERIOD))
            {
                string fieldName = ExpectAnyName();
                e = Make(Tag.Field, e, fieldName);
            }
            else if (TryParse(TokenType.ARROW))
            {
                string fieldName = ExpectAnyName();
                e = Make(Tag.Field, Make(Tag.Load, e), fieldName);
            }
            else if (TryParse(TokenType.INCREMENT))
            {
                e = Make(Tag.PostIncrement, e);
            }
            else if (TryParse(TokenType.DECREMENT))
            {
                e = Make(Tag.PostDecrement, e);
            }
            else if (TryParse(TokenType.LBRACKET))
            {
                Expr index = ParseExpr();
                Expect(TokenType.RBRACKET);
                e = Make(Tag.Index, e, index);
            }
            else
            {
                // No more suffixes.
                break;
            }
        }
        return e;
    }

    // "Primary" expressions
    Expr ParsePrimaryExpr()
    {
        int n;
        string name, s;
        if (TryParseInt(out n))
        {
            return Make(Tag.Integer, n);
        }
        else if (TryParseString(out s))
        {
            // String literal: pool into a top-level readonly data declaration and return a name reference.
            // This avoids emitting ($readonly_data ...) as an expression, which complicates codegen.
            string poolKey = GetStringLiteralPoolKey(s);
            if (!StringLiteralNameByValue.TryGetValue(poolKey, out name))
            {
                name = string.Format("$string{0}", NextStringID++);
                StringLiteralNameByValue[poolKey] = name;

                int[] values = new int[s.Length + 1];
                for (int i = 0; i < s.Length; i++)
                {
                    int c = s[i];
                    if (c > 127) ParserError("strings can only contain ASCII characters");
                    values[i] = c;
                }
                values[s.Length] = '\0';

                // Keep the declaration in insertion order for stable output.
                // String literals are plain near pointers, so keep them in
                // fixed bank 0 where every caller can safely dereference them.
                StringLiteralDeclarations.Add(
                    Make(
                        Tag.FixedBank, 0,
                        Make(
                            Tag.Bank, 0,
                            Make(Tag.ReadonlyData, CType.MakeArray(CType.UInt8, s.Length + 1), name, values))));
            }

            return Make(Tag.Name, name);
        }
        else if (TryParseAnyName(out name))
        {
            string resolved = ResolveStaticSymbolReference(SourcePosition.Filename, name);
            return Make(Tag.Name, resolved);
        }
        else if (TryParse(TokenType.LPAREN))
        {
            CType type;
            if (TryParseType(out type))
            {
                Expect(TokenType.RPAREN);
                Expr e = ParseUnaryPrefixExpr();
                return Make(Tag.Cast, type, e);
            }
            else
            {
                Expr e = ParseExpr();
                Expect(TokenType.RPAREN);
                return e;
            }
        }
        else
        {
            ParserError("expected an expression");
            return null;
        }
    }

    Expr ParseInfixOperators(Func<Expr> parseSubexpression, Dictionary<TokenType, string> operators)
    {
        Expr e = parseSubexpression();
        string op;
        while (operators.TryGetValue(PeekToken().Tag, out op))
        {
            ConsumeToken();
            Expr right = parseSubexpression();
            e = Make(op, e, right);
        }
        return e;
    }

    Token PeekToken()
    {
        return Input[0];
    }

    Token PeekToken(int offset)
    {
        int idx = offset;
        if (idx < 0) idx = 0;
        if (idx >= Input.Count) return Input[Input.Count - 1];
        return Input[idx];
    }

    bool TryParseNodiscardAttribute()
    {
        // Recognize: [[nodiscard]]
        // Tokens: '[' '[' NAME ']' ']'
        if (PeekToken().Tag == TokenType.LBRACKET &&
            PeekToken(1).Tag == TokenType.LBRACKET &&
            PeekToken(2).Tag == TokenType.NAME && PeekToken(2).Name == "nodiscard" &&
            PeekToken(3).Tag == TokenType.RBRACKET &&
            PeekToken(4).Tag == TokenType.RBRACKET)
        {
            ConsumeToken(); // [
            ConsumeToken(); // [
            ConsumeToken(); // nodiscard
            ConsumeToken(); // ]
            ConsumeToken(); // ]
            return true;
        }
        return false;
    }

    bool TryParseRangeAttribute(out Expr range)
    {
        // Recognize: __range(min,max)
        // Used as a lightweight integer range annotation for variables.
        // min/max are parsed as constant expressions (const/sizeof/ops) and evaluated later.
        if (!TryParseName("__range"))
        {
            range = null;
            return false;
        }
        Expect(TokenType.LPAREN);
        Expr minExpr = ParseExpr();
        Expect(TokenType.COMMA);
        Expr maxExpr = ParseExpr();
        Expect(TokenType.RPAREN);
        range = Expr.Make(Tag.Range, minExpr, maxExpr).WithSource(SourcePosition);
        return true;
    }

    bool TryParseAlignedAttribute(out int align)
    {
        // Recognize: __aligned(N)
        // N must be a positive power-of-two integer literal (1,2,4,8,16,...).
        if (!TryParseName("__aligned"))
        {
            align = 0;
            return false;
        }

        Expect(TokenType.LPAREN);
        int n = ExpectInt();
        Expect(TokenType.RPAREN);

        if (n <= 0 || (n & (n - 1)) != 0)
        {
            ParserError(ErrorCode.ParseError, "__aligned(N): N must be a power-of-two positive integer literal (got {0})", n);
        }
        align = n;
        return true;
    }

    void ConsumeToken()
    {
        // Record token for diagnostics before consuming.
        Token cur = Input[0];
        SourcePosition = cur.Position;

        // Keep last few consumed tokens (default 6 to cover both sides).
        RecentTokens.Add(cur);
        if (RecentTokens.Count > 6) RecentTokens.RemoveAt(0);

        // The final "end of file" token is never removed.
        if (cur.Tag != TokenType.EOF)
        {
            Input.RemoveAt(0);
        }
    }

    bool TryParse(TokenType expected)
    {
        Token token = PeekToken();
        if (token.Tag == expected)
        {
            ConsumeToken();
            return true;
        }
        else
        {
            return false;
        }
    }

    void Expect(TokenType expected)
    {
        if (!TryParse(expected))
        {
            Token got = PeekToken();
            ParserError(ErrorCode.ExpectedToken, "expected {0}, got {1}", TokenInfo.TokenNames[(int)expected], got.Show());
        }
    }

    bool TryParseInt(out int n)
    {
        Token t0 = PeekToken();
        Token t1 = PeekToken(1);

        // Accept a signed integer literal token pair ("- 123" / "+ 123")
        // in places that require immediate integer constants.
        if ((t0.Tag == TokenType.MINUS || t0.Tag == TokenType.PLUS) && t1.Tag == TokenType.INT)
        {
            long v = (long)t1.Int;
            if (t0.Tag == TokenType.MINUS) v = -v;
            if (v < int.MinValue || v > int.MaxValue)
            {
                ParserError("integer literal out of range");
            }
            ConsumeToken(); // sign
            ConsumeToken(); // int
            n = (int)v;
            return true;
        }

        if (t0.Tag == TokenType.INT)
        {
            n = t0.Int;
            ConsumeToken();
            return true;
        }

        n = 0;
        return false;

    }

    bool TryParsePragmaBank(out int bank)
    {
        bank = 0;
        if (PeekToken().Tag == TokenType.PRAGMA_BANK)
        {
            bank = PeekToken().Int;
            ConsumeToken();
            return true;
        }
        return false;
    }

    bool TryParsePragmaFixedBank(out int bank)
    {
        bank = 0;
        if (PeekToken().Tag == TokenType.PRAGMA_FIXED_BANK)
        {
            bank = PeekToken().Int;
            ConsumeToken();
            return true;
        }
        return false;
    }

    bool TryParsePragmaFixedOrder(out int order)
    {
        order = 0;
        if (PeekToken().Tag == TokenType.PRAGMA_FIXED_ORDER)
        {
            order = PeekToken().Int;
            ConsumeToken();
            return true;
        }
        return false;
    }

    bool TryParsePragmaRegion(out MemoryRegion region)
    {
        region = MemoryRegion.Ram;
        if (PeekToken().Tag == TokenType.PRAGMA_REGION)
        {
            int t = PeekToken().Int;
            ConsumeToken();
            switch ((MemoryRegionTag)t)
            {
                case MemoryRegionTag.HighMem: region = MemoryRegion.HighMem; break;
                case MemoryRegionTag.Wram0: region = MemoryRegion.Wram0; break;
                case MemoryRegionTag.WramX: region = MemoryRegion.WramX; break;
                default: region = MemoryRegion.Ram; break;
            }
            return true;
        }
        return false;
    }

    bool TryParsePragmaWramXBank(out int bank)
    {
        bank = 0;
        if (PeekToken().Tag == TokenType.PRAGMA_WRAMX_BANK)
        {
            bank = PeekToken().Int;
            ConsumeToken();
            return true;
        }
        return false;
    }

    bool TryParsePragmaAlign(out int align)
    {
        align = 0;
        if (PeekToken().Tag == TokenType.PRAGMA_ALIGN)
        {
            align = PeekToken().Int;
            ConsumeToken();
            return true;
        }
        return false;
    }

    bool TryParsePragmaSection(out string name)
    {
        name = null;
        if (PeekToken().Tag == TokenType.PRAGMA_SECTION)
        {
            name = PeekToken().Name;
            ConsumeToken();
            return true;
        }
        return false;
    }

    MemoryRegion ApplyPragmaWramXBank(MemoryRegion region)
    {
        if (region.Tag == MemoryRegionTag.WramX && region.WramBank == 0 && CurrentPragmaWramXBank >= 1 && CurrentPragmaWramXBank <= 7)
            return MemoryRegion.WramXBank(CurrentPragmaWramXBank);
        return region;
    }

    bool ValidateWramXBankLiteral(int bank, string context)
    {
        if (bank < 1 || bank > 7)
        {
            ParserError(ErrorCode.InvalidWramXBank, "{0}: WRAMX bank must be in range 1..7 (got {1})", context, bank);
            return false;
        }
        return true;
    }

    int ExpectInt()
    {
        int n;
        if (!TryParseInt(out n)) ParserError("expected an integer literal");
        return n;
    }

    bool TryParseString(out string s)
    {
        Token token = PeekToken();
        if (token.Tag == TokenType.STRING)
        {
            s = token.Name;
            ConsumeToken();
            return true;
        }
        else
        {
            s = null;
            return false;
        }
    }

    bool TryParseAnyName(out string name)
    {
        Token token = PeekToken();
        if (token.Tag == TokenType.NAME)
        {
            name = token.Name;
            ConsumeToken();
            return true;
        }
        else
        {
            name = null;
            return false;
        }
    }

    string ExpectAnyName()
    {
        string name;
        if (!TryParseAnyName(out name)) ParserError("expected an identifier");
        return name;
    }

    bool TryParseName(string name)
    {
        Token token = PeekToken();
        if (token.Tag == TokenType.NAME && token.Name == name)
        {
            ConsumeToken();
            return true;
        }
        else
        {
            return false;
        }
    }

    void ExpectKeyword(string name)
    {
        if (!TryParseName(name))
        {
            Token got = PeekToken();
            ParserError("expected '{0}', got {1}", name, got.Show());
        }
    }

    bool TryParseType(out CType type)
    {
        // C qualifier (subset): 'const' can appear before or after the base type, and
        // also after pointer stars (e.g. u8* const).
        // We model this as `CType.IsConst` on each node (base or pointer).
        // Additional type-annotation: __enum_strict can appear before an enum type to
        // enable stricter enum/integer mixing diagnostics for that enum tag.
        bool baseConst = false;
        bool enumStrict = false;
        bool baseBitFlags = false;
        bool baseSafeIndex = false;
        bool basePacked = false;
        int baseAlign = 0;
        while (true)
        {
            if (TryParseName("const")) { baseConst = true; continue; }
            if (TryParseName("__enum_strict")) { enumStrict = true; continue; }
            if (TryParseName("__bitflags")) { baseBitFlags = true; continue; }
            if (TryParseName("__safe_index")) { baseSafeIndex = true; continue; }
            if (TryParseName("__packed")) { basePacked = true; continue; }
            int a;
            if (TryParseAlignedAttribute(out a)) { baseAlign = a; continue; }
            break;
        }

        if (TryParseName("void"))
        {
            type = CType.Void;
        }
        else if (TryParseName("uint8_t") || TryParseName("u8") || TryParseName("char") || TryParseName("bool"))
        {
            type = CType.UInt8;
        }
        else if (TryParseName("uint16_t") || TryParseName("u16"))
        {
            type = CType.UInt16;
        }
        else if (TryParseName("enum"))
        {
            // enum Tag { A=0, B, C } / enum Tag / enum { ... }
            // enum class Tag { ... } / enum struct Tag { ... } (strict mode alias)
            Token k0 = PeekToken();
            if (k0.Tag == TokenType.NAME &&
                (string.Equals(k0.Name, "class", StringComparison.Ordinal) ||
                 string.Equals(k0.Name, "struct", StringComparison.Ordinal)))
            {
                ConsumeToken();
                enumStrict = true;
            }

            string tagName = null;

            Token t0 = PeekToken();
            if (t0.Tag == TokenType.NAME && t0.Name != null)
            {
                tagName = t0.Name;
                ConsumeToken();
            }

            bool hasBody = TryParse(TokenType.LBRACE);
            if (hasBody)
            {
                if (string.IsNullOrEmpty(tagName)) tagName = MakeAnonymousTypeName("__anon_enum");

                // Create/register the enum tag early so enumerators can reference its type.
                if (!EnumTags.ContainsKey(tagName)) EnumTags[tagName] = CType.MakeEnum(tagName);
                if (enumStrict) StrictEnumTags.Add(tagName);
                if (StrictEnumTags.Contains(tagName)) EnumTags[tagName].IsEnumStrict = true;

                // Track the current enumerator value as an expression, so that
                // we can support full constant expressions (const/sizeof/ops)
                // without having to evaluate immediately.
                // Start at -1 so that the first implicit enumerator becomes 0.
                Expr curExpr = Make(Tag.Integer, -1);
                while (!TryParse(TokenType.RBRACE))
                {
                    string ename = ExpectAnyName();

                    if (TryParse(TokenType.EQUAL))
                    {
                        // Explicit value: accept a full constant expression.
                        // We don't evaluate here; CodeGenerator will evaluate and error
                        // if it is not a constant expression.
                        Expr vexpr = ParseExpr();
                        curExpr = vexpr;
                    }
                    else
                    {
                        // Implicit value: previous + 1
                        curExpr = Make(Tag.Add, curExpr, Make(Tag.Integer, 1));
                    }

                    // Emit as a compile-time constant symbol.
                    PendingTopDecls.Add(WrapPragmasIfNeeded(Make(Tag.Constant, EnumTags[tagName], ename, curExpr)));

                    if (TryParse(TokenType.COMMA))
                    {
                        // Allow trailing comma.
                        if (PeekToken().Tag == TokenType.RBRACE) continue;
                    }
                }
            }
            else
            {
                if (string.IsNullOrEmpty(tagName))
                    ParserError(ErrorCode.ParseError, "expected enum tag name or enum body");
            }

            // Register and return the enum type.
            if (!EnumTags.ContainsKey(tagName))
            {
                EnumTags[tagName] = CType.MakeEnum(tagName);
            }
            if (enumStrict) StrictEnumTags.Add(tagName);
            if (StrictEnumTags.Contains(tagName)) EnumTags[tagName].IsEnumStrict = true;
            type = EnumTags[tagName];
        }

        else if (TryParseName("struct"))
        {
            string name = ExpectAnyName();
            type = CType.MakeStruct(name);
        }
        else if (TryParseName("union"))
        {
            string name = ExpectAnyName();
            type = CType.MakeUnion(name);
        }
        else
        {
            // Typedef aliases (subset)
            Token t = PeekToken();
            if (t.Tag == TokenType.NAME && t.Name != null && Typedefs.TryGetValue(t.Name, out type))
            {
                ConsumeToken();
            }
            else
            {
                type = null;
                return false;
            }
        }

        // Support: `u8 const *p` (qualifier after the base type specifier)
        while (TryParseName("const")) baseConst = true;

        // Apply aggregate layout annotations to struct/union base types.
        if (type != null && type.IsStructOrUnion)
        {
            if (basePacked) type.IsPackedAggregate = true;
            if (baseAlign > 0) type.ForcedAlign = baseAlign;
        }

        // Apply forced alignment to any base type (object-level). For non-aggregate types,
        // this affects placement only (not sizeof). For aggregates it also influences layout.
        if (type != null && baseAlign > 0 && !type.IsStructOrUnion)
        {
            // Clone to avoid mutating shared singleton types.
            type = new CType
            {
                Tag = type.Tag,
                SimpleType = type.SimpleType,
                Name = type.Name,
                Subtype = type.Subtype,
                Dimension = type.Dimension,
                DimensionExpression = type.DimensionExpression,
                ParamTypes = type.ParamTypes,
                IsConst = type.IsConst,
                IsRestrict = type.IsRestrict,
                IsEnumStrict = type.IsEnumStrict,
                IsBitFlags = type.IsBitFlags,
                IsSafeIndex = type.IsSafeIndex,
                ForcedAlign = baseAlign,
                IsPackedAggregate = type.IsPackedAggregate,
            };
        }

        // Apply __bitflags to the base type (integer-only).
        if (baseBitFlags)
        {
            if (!(type != null && type.IsInteger && !type.IsEnum))
            {
                ParserError("__bitflags can only be applied to integer scalar types (u8/u16)");
            }
            // Clone the base node to avoid mutating shared singleton types.
            type = new CType
            {
                Tag = type.Tag,
                SimpleType = type.SimpleType,
                Name = type.Name,
                Subtype = type.Subtype,
                Dimension = type.Dimension,
                DimensionExpression = type.DimensionExpression,
                ParamTypes = type.ParamTypes,
                IsConst = type.IsConst,
                IsRestrict = type.IsRestrict,
                IsEnumStrict = type.IsEnumStrict,
                IsBitFlags = true,
                IsSafeIndex = type.IsSafeIndex,
                ForcedAlign = type.ForcedAlign,
                IsPackedAggregate = type.IsPackedAggregate,
            };
        }

        // Apply __safe_index to the base type (integer-only).
        if (baseSafeIndex)
        {
            if (!(type != null && type.IsInteger && !type.IsEnum))
            {
                ParserError("__safe_index can only be applied to integer scalar types (u8/u16)");
            }
            // Clone the base node to avoid mutating shared singleton types.
            type = new CType
            {
                Tag = type.Tag,
                SimpleType = type.SimpleType,
                Name = type.Name,
                Subtype = type.Subtype,
                Dimension = type.Dimension,
                DimensionExpression = type.DimensionExpression,
                ParamTypes = type.ParamTypes,
                IsConst = type.IsConst,
                IsRestrict = type.IsRestrict,
                IsEnumStrict = type.IsEnumStrict,
                IsBitFlags = type.IsBitFlags,
                IsSafeIndex = true,
                ForcedAlign = type.ForcedAlign,
                IsPackedAggregate = type.IsPackedAggregate,
            };
        }

        if (baseConst && !type.IsConst)
        {
            // Apply qualifier to the parsed base type.
            // Note: const does not affect layout. We track it for diagnostics,
            // assignment checks, and for treating const globals as compile-time constants or ROM data.
            type = new CType
            {
                Tag = type.Tag,
                SimpleType = type.SimpleType,
                Name = type.Name,
                Subtype = type.Subtype,
                Dimension = type.Dimension,
                DimensionExpression = type.DimensionExpression,
                ParamTypes = type.ParamTypes,
                IsConst = true,
                IsRestrict = type.IsRestrict,
                IsEnumStrict = type.IsEnumStrict,
                IsBitFlags = type.IsBitFlags,
                IsSafeIndex = type.IsSafeIndex,
                ForcedAlign = type.ForcedAlign,
                IsPackedAggregate = type.IsPackedAggregate,
            };
        }

        // Apply layout attributes to the base type node.
        if (baseAlign > 0 || basePacked)
        {
            // Clone the base node to avoid mutating shared singleton types.
            type = new CType
            {
                Tag = type.Tag,
                SimpleType = type.SimpleType,
                Name = type.Name,
                Subtype = type.Subtype,
                Dimension = type.Dimension,
                DimensionExpression = type.DimensionExpression,
                ParamTypes = type.ParamTypes,
                IsConst = type.IsConst,
                IsRestrict = type.IsRestrict,
                IsEnumStrict = type.IsEnumStrict,
                IsBitFlags = type.IsBitFlags,
                IsSafeIndex = type.IsSafeIndex,
                ForcedAlign = baseAlign,
                IsPackedAggregate = basePacked,
            };
        }

        // Pointer chain. After each '*', accept optional qualifiers that qualify that pointer level.
        // Supported here:
        // - const (standard)
        // - __restrict / restrict (lint-only; stored on the pointer node)
        while (TryParse(TokenType.STAR))
        {
            bool ptrConst = false;
            bool ptrRestrict = false;

            while (true)
            {
                if (TryParseName("const")) { ptrConst = true; continue; }
                if (TryParseName("__restrict") || TryParseName("restrict")) { ptrRestrict = true; continue; }
                break;
            }

            type = new CType
            {
                Tag = CTypeTag.Pointer,
                Subtype = type,
                IsConst = ptrConst,
                IsRestrict = ptrRestrict,
                // Pointer qualifiers reset the integer/enum lint flags at this level.
                IsEnumStrict = false,
                IsBitFlags = false,
                IsSafeIndex = false,
                ForcedAlign = 0,
                IsPackedAggregate = false,
            };
        }

        return true;
    }

    CType ExpectType()
    {
        CType type;
        if (!TryParseType(out type)) ParserError(ErrorCode.ExpectedType, "expected a type");
        return type;
    }

    [DebuggerStepThrough]
    void ParserError(string format, params object[] args)
    {
        ParserError(ErrorCode.ParseError, format, args);
    }

    [DebuggerStepThrough]
    void ParserError(ErrorCode code, string format, params object[] args)
    {
        // Use the *current* token position (not the last consumed token) so errors point at the true culprit.
        // This is especially important when compiling multiple C files in a single invocation.
        Program.Error(PeekToken().Position, code, format, args);
        Program.TryWriteTokenContext(RecentTokens, Input, 3, 4);
        throw new RecoverableParseException();
    }

    Expr[] UnwrapSwitchBodyStatements(Expr body)
    {
        if (body == null || body.Match(Tag.Empty)) return Array.Empty<Expr>();
        Expr[] seq;
        if (body.MatchAny(Tag.Sequence, out seq)) return seq ?? Array.Empty<Expr>();
        return new[] { body };
    }

    bool IsSwitchTerminatingStatement(Expr stmt)
    {
        if (stmt == null || stmt.Match(Tag.Empty)) return false;
        string jumpLabel;
        Expr retValue;
        if (stmt.Match(Tag.Break) || stmt.Match(Tag.Return) || stmt.Match(Tag.Return, out retValue) || stmt.Match(Tag.Jump, out jumpLabel))
            return true;

        Expr[] seq;
        if (stmt.MatchAny(Tag.Sequence, out seq))
        {
            if (seq == null || seq.Length == 0) return false;
            return IsSwitchTerminatingStatement(seq[seq.Length - 1]);
        }

        Expr[] parts;
        if (stmt.MatchAny(Tag.If, out parts))
        {
            if (parts == null || parts.Length < 2 || (parts.Length % 2) != 0) return false;

            bool hasElse = false;
            bool allBranchesTerminate = true;
            for (int i = 0; i < parts.Length; i += 2)
            {
                Expr cond = parts[i];
                Expr branch = parts[i + 1];

                int v;
                if (i == parts.Length - 2 && cond.Match(Tag.Integer, out v) && v != 0)
                    hasElse = true;

                if (!IsSwitchTerminatingStatement(branch))
                    allBranchesTerminate = false;
            }
            return hasElse && allBranchesTerminate;
        }

        return false;
    }

    void ValidateSwitchFallthrough(List<Expr> cases, Expr defaultBody)
    {
        bool hasDefault = defaultBody != null && !defaultBody.Match(Tag.Empty);

        for (int i = 0; i < cases.Count; i++)
        {
            Expr valExpr;
            Expr body;
            if (!cases[i].Match(Tag.Case, out valExpr, out body)) continue;

            Expr[] stmts = UnwrapSwitchBodyStatements(body);
            if (stmts.Length == 0) continue;

            int fallthroughCount = 0;
            for (int j = 0; j < stmts.Length; j++)
            {
                if (stmts[j].Match(Tag.Fallthrough))
                {
                    fallthroughCount++;
                    if (j != stmts.Length - 1)
                    {
                        ParserError(ErrorCode.ParseError, "'fallthrough' must be the last statement in a case block");
                    }
                }
            }

            Expr last = stmts[stmts.Length - 1];
            bool hasTarget = (i + 1 < cases.Count) || hasDefault;

            if (fallthroughCount > 0)
            {
                if (!hasTarget)
                {
                    Program.Warning(Maybe.Just(last.Source), ErrorCode.SwitchFallthroughUsage,
                        "'fallthrough' has no target case/default");
                }
                continue;
            }

            if (!hasTarget) continue;
            if (IsSwitchTerminatingStatement(last)) continue;

            Program.Warning(Maybe.Just(last.Source), ErrorCode.SwitchImplicitFallthrough,
                "implicit fallthrough in switch case; use 'fallthrough;' to make intent explicit");
        }

        if (hasDefault)
        {
            Expr[] defStmts = UnwrapSwitchBodyStatements(defaultBody);
            for (int i = 0; i < defStmts.Length; i++)
            {
                if (defStmts[i].Match(Tag.Fallthrough))
                {
                    Program.Warning(Maybe.Just(defStmts[i].Source), ErrorCode.SwitchFallthroughUsage,
                        "'fallthrough' inside default has no target");
                }
            }
        }
    }

    // Switch statement parser.
    Expr ParseSwitch()
    {
        Expect(TokenType.LPAREN);
        Expr test = ParseExpr();
        Expect(TokenType.RPAREN);
        Expect(TokenType.LBRACE);

        List<Expr> cases = new List<Expr>();
        Expr defaultBody = null;

        SwitchDepth++;
        try
        {
            while (!TryParse(TokenType.RBRACE))
            {
                if (TryParseName("case"))
                {
                    Expr valExpr = ParseExpr();
                    Expect(TokenType.COLON);

                    // Read statements until the next case/default or end of switch.
                    List<Expr> stmts = new List<Expr>();
                    while (PeekToken().Tag != TokenType.RBRACE &&
                           PeekToken().Name != "case" &&
                           PeekToken().Name != "default")
                    {
                        try
                        {
                            stmts.Add(ParseStatement(true));
                        }
                        catch (RecoverableParseException)
                        {
                            SynchronizeStatementBoundary(stopAtSwitchCaseBoundary: true);
                            if (PeekToken().Tag == TokenType.EOF ||
                                PeekToken().Tag == TokenType.RBRACE ||
                                PeekToken().Name == "case" ||
                                PeekToken().Name == "default")
                                break;
                        }
                    }

                    cases.Add(Make(Tag.Case, valExpr, MakeSequence(stmts)));
                }
                else if (TryParseName("default"))
                {
                    Expect(TokenType.COLON);
                    List<Expr> stmts = new List<Expr>();
                    while (PeekToken().Tag != TokenType.RBRACE &&
                           PeekToken().Name != "case" &&
                           PeekToken().Name != "default")
                    {
                        try
                        {
                            stmts.Add(ParseStatement(true));
                        }
                        catch (RecoverableParseException)
                        {
                            SynchronizeStatementBoundary(stopAtSwitchCaseBoundary: true);
                            if (PeekToken().Tag == TokenType.EOF ||
                                PeekToken().Tag == TokenType.RBRACE ||
                                PeekToken().Name == "case" ||
                                PeekToken().Name == "default")
                                break;
                        }
                    }
                    defaultBody = MakeSequence(stmts);
                }
                else
                {
                    ParserError("expected 'case' or 'default' inside switch");
                }
            }
        }
        finally
        {
            SwitchDepth--;
        }

        ValidateSwitchFallthrough(cases, defaultBody);

        // Switch node: (Tag.Switch, testExpr, cases[], defaultBody)
        return Make(Tag.Switch, test, cases.ToArray(), defaultBody ?? Make(Tag.Empty));
    }

}




