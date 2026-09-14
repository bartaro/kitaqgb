using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

// Scan source and process includes, macros and conditional directives before
// parsing. Each file has its own cursor/conditional stack; a compilation shares
// macro definitions, include search paths, once guards and dependency tracking.
class Tokenizer
{
    // Store an unexpanded replacement token sequence and its definition location.
    sealed class ObjectMacro
    {
        public string Name;
        public List<Token> Replacement = new List<Token>();
        public FilePosition DefinedAt;
    }

    // Map parameter names to argument indices for token substitution at each invocation.
    sealed class FunctionMacro
    {
        public string Name;
        public string[] Parameters = Array.Empty<string>();
        public Dictionary<string, int> ParamIndex = new Dictionary<string, int>(StringComparer.Ordinal);
        public List<Token> Replacement = new List<Token>();
        public FilePosition DefinedAt;
    }

    // Track enclosing activity, the selected branch and whether else has occurred.
    // AnyTaken prevents a later elif/else from selecting a second branch.
    sealed class ConditionalFrame
    {
        public bool ParentActive;
        public bool BranchActive;
        public bool AnyTaken;
        public bool SeenElse;
        public FilePosition StartPos;
    }

    string Input;
    int Next = 0;
    FilePosition InputPos;
    bool InAssembly;
    bool HasPragmaOnce;
    IncludeContext IncludeState;
    bool DisableMacroExpansion;
    readonly Stack<ConditionalFrame> ConditionalStack = new Stack<ConditionalFrame>();

    // End-of-file position for diagnostics (used by Parser to place the final EOF token).
    FilePosition EndPosition;

    const int MacroExpansionDepthLimit = 32;

    // Share preprocessing state across all explicitly supplied files and recursive
    // includes. File paths are compared without case; macro names remain case-sensitive.
    sealed class IncludeContext
    {
        public readonly HashSet<string> OnceFiles = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        public readonly HashSet<string> Dependencies = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        public readonly Stack<string> IncludeStack = new Stack<string>();
        public readonly List<string> IncludeDirs = new List<string>();
        public readonly Dictionary<string, ObjectMacro> ObjectMacros = new Dictionary<string, ObjectMacro>(StringComparer.Ordinal);
        public readonly Dictionary<string, FunctionMacro> FunctionMacros = new Dictionary<string, FunctionMacro>(StringComparer.Ordinal);
    }

    // Tokenize one source using the CLI include directories, discarding EOF/dependency metadata.
    public static List<Token> TokenizeFile(string filename)
    {
        FilePosition end;
        List<string> deps;
        return TokenizeFiles(new[] { filename }, Program.IncludeDirectories, out end, out deps);
    }

    // Overload that also returns the end-of-file position.
    // Tokenize one source and return its final cursor position for parser diagnostics.
    public static List<Token> TokenizeFile(string filename, out FilePosition endPosition)
    {
        List<string> deps;
        return TokenizeFiles(new[] { filename }, Program.IncludeDirectories, out endPosition, out deps);
    }

    // Process input files in order under one shared preprocessor context. Normalize
    // and deduplicate valid include directories, return sorted unique dependencies,
    // and report the last input file's EOF position; no inputs leave it Unknown.
    public static List<Token> TokenizeFiles(IEnumerable<string> filenames, IEnumerable<string> includeDirs, out FilePosition eofPosition, out List<string> dependencies)
    {
        eofPosition = FilePosition.Unknown;
        var outTokens = new List<Token>();
        var ctx = new IncludeContext();

        if (includeDirs != null)
        {
            foreach (var d in includeDirs)
            {
                if (string.IsNullOrWhiteSpace(d)) continue;
                try
                {
                    string p = Path.GetFullPath(d.Trim());
                    if (!ctx.IncludeDirs.Any(x => string.Equals(x, p, StringComparison.OrdinalIgnoreCase)))
                        ctx.IncludeDirs.Add(p);
                }
                catch
                {
                    // ignore invalid include dirs
                }
            }
        }

        foreach (string fn in filenames ?? Enumerable.Empty<string>())
        {
            FilePosition endPos;
            outTokens.AddRange(TokenizeFileRecursive(fn, ctx, out endPos));
            eofPosition = endPos;
        }

        dependencies = ctx.Dependencies.OrderBy(x => x, StringComparer.OrdinalIgnoreCase).ToList();
        return outTokens;
    }

    // Resolve and read one file, guarding against include cycles and completed
    // pragma-once files. Register the dependency before scanning; successful scanning
    // records its once guard and removes it from the active include stack.
    static List<Token> TokenizeFileRecursive(string filename, IncludeContext ctx, out FilePosition endPosition)
    {
        string fullPath;
        try { fullPath = Path.GetFullPath(filename); }
        catch
        {
            Program.Error("failed to resolve path: " + filename);
            endPosition = FilePosition.Unknown;
            return new List<Token>();
        }

        if (!File.Exists(fullPath))
        {
            Program.Error("source file not found: " + fullPath);
            endPosition = new FilePosition(fullPath, 0, 0);
            return new List<Token>();
        }

        if (ctx.OnceFiles.Contains(fullPath))
        {
            endPosition = new FilePosition(fullPath, 0, 0);
            return new List<Token>();
        }

        if (ctx.IncludeStack.Any(x => string.Equals(x, fullPath, StringComparison.OrdinalIgnoreCase)))
        {
            Program.Error("include cycle detected: " + fullPath);
            endPosition = new FilePosition(fullPath, 0, 0);
            return new List<Token>();
        }

        ctx.Dependencies.Add(fullPath);
        ctx.IncludeStack.Push(fullPath);

        Tokenizer tokenizer = new Tokenizer();
        tokenizer.IncludeState = ctx;
        tokenizer.Input = IoUtil.ReadAllTextUtf8(fullPath);
        tokenizer.InputPos = new FilePosition(fullPath, 0, 0);
        var toks = tokenizer.Tokenize();
        endPosition = tokenizer.EndPosition;
        if (tokenizer.HasPragmaOnce) ctx.OnceFiles.Add(fullPath);

        ctx.IncludeStack.Pop();
        return toks;
    }

    // Scan the current input, expanding macros only outside inline assembly and
    // processing only conditional directives in inactive branches. Inline assembly
    // retains newline tokens; the parser adds its own EOF token using EndPosition.
    public List<Token> Tokenize()
    {
        List<Token> tokens = new List<Token>();
        FilePosition pos, lastPos;
        pos = InputPos;
        EndPosition = InputPos;
        while (true)
        {
            TokenType tag = TokenType.INVALID;
            int tokenInt = 0;
            string tokenName = null;

            SkipSpaces();

            // Record the location of this token:
            lastPos = pos;
            pos = InputPos;

            // Handle characters in order of ASCII value:

            // (Unprintable characters shouldn't be in the file, and whitespace was already skipped.)
            // NOTE: Do NOT consume a synthetic '\0' character; keep the true EOF position.
            if (GetNextChar() == '\0')
            {
                if (ConditionalStack.Count > 0)
                {
                    var frame = ConditionalStack.Peek();
                    Program.Error(frame.StartPos, "missing #endif for conditional block");
                }
                EndPosition = InputPos;
                return tokens;
            }

            // Inside an inactive preprocessor branch, only nested #if/#elif/#else/#endif directives are processed.
            if (!InAssembly && !IsCurrentBranchActive() && GetNextChar() != '#')
            {
                SkipToNextLine();
                continue;
            }
            else if (TryRead("\n")) tag = TokenType.NEWLINE;
            else if (GetNextChar() <= ' ') tag = TokenType.INVALID;
            else if (TryRead('!'))
            {
                if (TryRead("=")) tag = TokenType.NOT_EQUAL;
                else tag = TokenType.LOGICAL_NOT;
            }
            else if (TryRead('"'))
            {
                tag = TokenType.STRING;
                tokenName = "";
                while (true)
                {
                    char c = GetNextChar();
                    if (c == '\0') Program.Error(InputPos, "unexpected end of file in string");
                    if (c == '\n') Program.Error(InputPos, "unexpected end of line in string");
                    if (c == '"')
                    {
                        FetchChar();
                        break;
                    }
                    tokenName += c;
                    FetchChar();
                }
            }
            else if (TryRead('\''))
            {
                tag = TokenType.INT;
                char c = GetNextChar();
                if (!IsValidCharacterConstant(c)) Program.Error(InputPos, "invalid character constant");
                tokenInt = c;
                FetchChar();
                c = GetNextChar();
                if (c != '\'') Program.Error(InputPos, "expected '");
                FetchChar();
            }
            else if (TryRead('#'))
            {
                if (InAssembly)
                {
                    tag = TokenType.NUMBER_SIGN;
                }
                else
                {
                    if (TryHandleConditionalDirective(pos))
                    {
                        SkipToNextLine();
                        continue;
                    }

                    if (!IsCurrentBranchActive())
                    {
                        SkipToNextLine();
                        continue;
                    }

                    // Support: #include "x.h" / #include <x.h>
                    if (TryReadInclude(out string includeSpec, out bool isAngleForm))
                    {
                        string includePath;
                        if (!TryResolveIncludePath(InputPos.Filename, includeSpec, isAngleForm, out includePath))
                        {
                            Program.Error(pos, "include file not found: " + includeSpec);
                        }
                        else
                        {
                            FilePosition includeEnd;
                            tokens.AddRange(TokenizeFileRecursive(includePath, IncludeState ?? new IncludeContext(), out includeEnd));
                        }

                        SkipToNextLine();
                        continue;
                    }

                    // Support: #define (object-like + function-like)
                    if (TryReadDefine(pos))
                    {
                        SkipToNextLine();
                        continue;
                    }

                    // Support: #undef
                    if (TryReadUndef(pos))
                    {
                        SkipToNextLine();
                        continue;
                    }

                    // Support: #warning / #error
                    if (TryReadDiagnosticDirective(pos))
                    {
                        SkipToNextLine();
                        continue;
                    }

                    // Support: #pragma once
                    if (TryReadPragmaOnce())
                    {
                        HasPragmaOnce = true;
                        SkipToNextLine();
                        continue;
                    }

                    // Support: #pragma bank N
                    int pragmaBank;
                    if (TryReadPragmaBank(out pragmaBank))
                    {
                        tag = TokenType.PRAGMA_BANK;
                        tokenInt = pragmaBank;
                        // consume rest of line (newline will be consumed by SkipSpaces on next loop)
                        SkipToNextLine();
                    }
                    else
                    {
                        bool emitPragmaToken = false;
                        int pragmaFixedBank;
                        if (TryReadPragmaFixedBank(out pragmaFixedBank))
                        {
                            tag = TokenType.PRAGMA_FIXED_BANK;
                            tokenInt = pragmaFixedBank;
                            SkipToNextLine();
                            emitPragmaToken = true;
                        }
                        else
                        {
                            int pragmaFixedOrder;
                            if (TryReadPragmaFixedOrder(out pragmaFixedOrder))
                            {
                                tag = TokenType.PRAGMA_FIXED_ORDER;
                                tokenInt = pragmaFixedOrder;
                                SkipToNextLine();
                                emitPragmaToken = true;
                            }
                        }

                        // Support: #pragma cgb_palette NAME C0 C1 C2 C3
                        if (!emitPragmaToken && TryReadPragmaCgbPalette(out string palName, out int[] palColors))
                        {
                            Program.ApplyCgbPalettePragma(pos, palName, palColors);
                            SkipToNextLine();
                            continue;
                        }

                        // Support: #pragma rom_* ... (ROM header options)
                        if (TryReadPragmaRomHeader(out string key, out string value))
                        {
                            Program.ApplyRomHeaderPragma(pos, key, value);
                            SkipToNextLine();
                            continue;
                        }

                        // Support: #pragma placement shortcuts
                        if (TryReadPragmaPlacement(out TokenType ptag, out int ival, out string sval))
                        {
                            tag = ptag;
                            tokenInt = ival;
                            if (sval != null) tokenName = sval;
                            SkipToNextLine();
                            emitPragmaToken = true;
                        }

                        if (emitPragmaToken)
                        {
                            // Emit a real token so the parser can carry the pragma state into declarations.
                            // SkipToNextLine leaves the newline for the next SkipSpaces call.
                        }
                        else
                        {
                            Program.Warning(InputPos, "preprocessor directives are ignored");
                            SkipToNextLine();
                            continue;
                        }
                    }
                }
            }
            else if (TryRead('%'))
            {
                if (TryRead('=')) tag = TokenType.PERCENT_EQUALS;
                else tag = TokenType.PERCENT;
            }
            else if (TryRead('&'))
            {
                if (TryRead('&')) tag = TokenType.LOGICAL_AND;
                else if (TryRead('=')) tag = TokenType.AMPERSAND_EQUALS;
                else tag = TokenType.AMPERSAND;
            }
            else if (TryRead('(')) tag = TokenType.LPAREN;
            else if (TryRead(')')) tag = TokenType.RPAREN;
            else if (TryRead('*'))
            {
                if (TryRead('=')) tag = TokenType.STAR_EQUALS;
                else tag = TokenType.STAR;
            }
            else if (TryRead('+'))
            {
                if (TryRead('+')) tag = TokenType.INCREMENT;
                else if (TryRead('=')) tag = TokenType.PLUS_EQUALS;
                else tag = TokenType.PLUS;
            }
            else if (TryRead(',')) tag = TokenType.COMMA;
            else if (TryRead('-'))
            {
                if (TryRead('>')) tag = TokenType.ARROW;
                else if (TryRead('-')) tag = TokenType.DECREMENT;
                else if (TryRead('=')) tag = TokenType.MINUS_EQUALS;
                else tag = TokenType.MINUS;
            }
            else if (TryRead('.')) tag = TokenType.PERIOD;
            else if (TryRead('/'))
            {
                // Skip past single-line comments:
                if (TryRead('/'))
                {
                    SkipToNextLine();
                    continue;
                }
                // Skip past block comments:
                else if (TryRead('*'))
                {
                    SkipBlockComment();
                    continue;
                }
                else
                {
                    if (TryRead('=')) tag = TokenType.SLASH_EQUALS;
                    else tag = TokenType.SLASH;
                }
            }
            else if (TryRead(':')) tag = TokenType.COLON;
            else if (TryRead(';')) tag = TokenType.SEMICOLON;
            else if (TryRead('<'))
            {
                if (TryRead('=')) tag = TokenType.LESS_THAN_OR_EQUAL;
                else if (TryRead('<'))
                {
                    if (TryRead('=')) tag = TokenType.SHIFT_LEFT_EQUALS;
                    else tag = TokenType.SHIFT_LEFT;
                }
                else tag = TokenType.LESS_THAN;
            }
            else if (TryRead('='))
            {
                if (TryRead('=')) tag = TokenType.DOUBLE_EQUAL;
                else tag = TokenType.EQUAL;
            }
            else if (TryRead('>'))
            {
                if (TryRead('=')) tag = TokenType.GREATER_THAN_OR_EQUAL;
                else if (TryRead('>'))
                {
                    if (TryRead('=')) tag = TokenType.SHIFT_RIGHT_EQUALS;
                    else tag = TokenType.SHIFT_RIGHT;
                }
                else tag = TokenType.GREATER_THAN;
            }
            else if (TryRead('?')) tag = TokenType.QUESTION_MARK;
            else if (TryRead('[')) tag = TokenType.LBRACKET;
            else if (TryRead(']')) tag = TokenType.RBRACKET;
            else if (TryRead('^'))
            {
                if (TryRead('=')) tag = TokenType.CARET_EQUALS;
                else tag = TokenType.CARET;
            }
            else if (TryRead('{')) tag = TokenType.LBRACE;
            else if (TryRead('|'))
            {
                if (TryRead('|')) tag = TokenType.LOGICAL_OR;
                else if (TryRead('=')) tag = TokenType.PIPE_EQUALS;
                else tag = TokenType.PIPE;
            }
            else if (TryRead('}')) tag = TokenType.RBRACE;
            else if (TryRead('~')) tag = TokenType.TILDE;
            else if (IsNameChar(GetNextChar()))
            {
                // Parse identifiers and numeric literals:
                StringBuilder sb = new StringBuilder();
                while (IsNameChar(GetNextChar()))
                {
                    sb.Append(GetNextChar());
                    FetchChar();
                }
                string name = sb.ToString();

                if (TryConvertInt(pos, name, out tokenInt))
                {
                    tag = TokenType.INT;
                }
                else
                {
                    tag = TokenType.NAME;
                    tokenName = name;

                    if (!DisableMacroExpansion && !InAssembly)
                    {
                        var activeMacros = new HashSet<string>(StringComparer.Ordinal);
                        if (TryExpandFunctionMacroInvocation(pos, tokenName, tokens, activeMacros, 0))
                        {
                            continue;
                        }
                        if (TryExpandObjectMacroToken(pos, tokenName, tokens, activeMacros, 0))
                        {
                            continue;
                        }
                    }
                }
            }
            else
            {
                // HACK: Show the input that could not be tokenized:
                // (Specify "{0}" as the format string in case the output contains curly braces.)
                Program.Error(pos, "{0}", "invalid token: " + new string(Input.Skip(Next).Take(15).ToArray()));
            }

            tokens.Add(new Token
            {
                Tag = tag,
                Int = tokenInt,
                Name = tokenName,
                Position = pos,
            });

            if (!InAssembly && tag == TokenType.NAME && tokenName == "__asm")
            {
                InAssembly = true;
            }

            if (InAssembly && tag == TokenType.RBRACE)
            {
                InAssembly = false;
            }
        }
    }

    // Accept the shared ASCII identifier/numeric-token alphabet, including dollar
    // for hexadecimal literals; whether a name may start with a digit is caller-specific.
    static bool IsNameChar(char c)
    {
        return (c == '_') || (c == '$') || (c >= '0' && c <= '9') || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');
    }

    // Consume whitespace while preserving assembly line breaks as NEWLINE tokens.
    void SkipSpaces()
    {
        string whitespace = InAssembly ? " \t\r" : " \t\r\n";
        while (true)
        {
            char c = GetNextChar();
            if (!whitespace.Contains(c)) return;
            FetchChar();
        }
    }

    // Consume through the last character before newline or EOF, leaving that terminator unread.
    void SkipToNextLine()
    {
        while (true)
        {
            char c = GetNextChar();
            if (c == '\n' || c == '\0') break;
            FetchChar();
        }
    }

    // Consume a non-nesting block comment after its opener, or report an unterminated comment.
    void SkipBlockComment()
    {
        // We enter here immediately after consuming the opening "/*".
        // Consume until the first closing "*/".
        while (true)
        {
            char c = GetNextChar();
            if (c == '\0') Program.Error(InputPos, "unexpected end of file in block comment");

            if (c == '*')
            {
                FetchChar();
                if (GetNextChar() == '/')
                {
                    FetchChar();
                    return;
                }
            }
            else
            {
                FetchChar();
            }
        }
    }

    // Recognize decimal, 0x/$ hexadecimal and 0b binary integers. A token not beginning
    // with a digit or dollar is a name; numeric-looking tokens with invalid digits
    // are errors. No suffix, octal convention or explicit overflow check is applied.
    static bool TryConvertInt(FilePosition pos, string original, out int integer)
    {
        string decimalDigits = "0123456789";
        string hexDigits = "0123456789ABCDEF";

        if (original.Length == 0) Program.Panic(pos, "names must not be empty");
        string literal = original.ToUpperInvariant();

        // If the token doesn't start with a digit, it isn't a number.
        integer = 0;
        char first = literal[0];
        if (!decimalDigits.Contains(first) && first != '$') return false;

        int numberBase = 10;
        if (literal.StartsWith("0X"))
        {
            literal = literal.Substring(2);
            numberBase = 16;
        }
        else if (literal.StartsWith("$"))
        {
            literal = literal.Substring(1);
            numberBase = 16;
        }
        else if (literal.StartsWith("0B"))
        {
            literal = literal.Substring(2);
            numberBase = 2;
        }

        if (literal.Length == 0)
        {
            Program.Error(pos, "number contains no digits: " + original);
        }

        string allowedDigits = hexDigits.Substring(0, numberBase);
        if (!literal.All(x => allowedDigits.Contains(x)))
        {
            Program.Error(pos, "number contains invalid characters: " + original);
        }

        integer = 0;
        foreach (char c in literal)
        {
            int index = allowedDigits.IndexOf(c);
            integer = numberBase * integer + index;
        }
        return true;
    }

    // Peek at the cursor without advancing; the NUL sentinel marks the end of input.
    char GetNextChar()
    {
        return (Next < Input.Length) ? Input[Next] : '\0';
    }

    // Advance the cursor once, then update the diagnostic position from the newly
    // exposed character. This preserves the tokenizer's existing location convention.
    void FetchChar()
    {
        if (Next < Input.Length) Next++;
        char c = GetNextChar();
        if (c == '\n')
        {
            InputPos = new FilePosition(InputPos.Filename, InputPos.Line + 1, 0);
        }
        else
        {
            InputPos = new FilePosition(InputPos.Filename, InputPos.Line, InputPos.Column + 1);
        }
    }

    // Consume one matching character through FetchChar; leave all cursor state unchanged on failure.
    bool TryRead(char c)
    {
        if (GetNextChar() == c)
        {
            FetchChar();
            return true;
        }
        else
        {
            return false;
        }
    }

    // Match a literal substring and advance only the character offset. Unlike
    // the character overload, this helper does not update the diagnostic position.
    bool TryRead(string s)
    {
        if (SafeSubstring(Input, Next, s.Length) == s)
        {
            Next += s.Length;
            return true;
        }
        return false;
    }

    // Limit a forward substring to available input; callers must provide a valid start offset.
    static string SafeSubstring(string s, int start, int length)
    {
        int maxLength = s.Length - start;
        if (length > maxLength) length = maxLength;
        return s.Substring(start, length);
    }

    // Recognize pragma once after the hash, restoring both cursors if it does not match.
    bool TryReadPragmaOnce()
    {
        int saveNext = Next;
        FilePosition savePos = InputPos;

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();
        if (!TryRead("pragma"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();
        if (!TryRead("once"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }
        return true;
    }

    // Read a quoted or angle-bracket include name on the current line. Restore
    // both cursors on malformed input; path lookup and file loading happen separately.
    bool TryReadInclude(out string includeSpec, out bool isAngleForm)
    {
        includeSpec = null;
        isAngleForm = false;

        int saveNext = Next;
        FilePosition savePos = InputPos;

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();
        if (!TryRead("include"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();

        if (TryRead('"'))
        {
            StringBuilder sb = new StringBuilder();
            while (true)
            {
                char c = GetNextChar();
                if (c == '\0' || c == '\n' || c == '\r')
                {
                    Next = saveNext;
                    InputPos = savePos;
                    return false;
                }
                if (c == '"')
                {
                    FetchChar();
                    includeSpec = sb.ToString();
                    isAngleForm = false;
                    return true;
                }
                sb.Append(c);
                FetchChar();
            }
        }

        if (TryRead('<'))
        {
            StringBuilder sb = new StringBuilder();
            while (true)
            {
                char c = GetNextChar();
                if (c == '\0' || c == '\n' || c == '\r')
                {
                    Next = saveNext;
                    InputPos = savePos;
                    return false;
                }
                if (c == '>')
                {
                    FetchChar();
                    includeSpec = sb.ToString();
                    isAngleForm = true;
                    return true;
                }
                sb.Append(c);
                FetchChar();
            }
        }

        Next = saveNext;
        InputPos = savePos;
        return false;
    }

    // Resolve an existing include. Quoted names search the including file's directory
    // first; both forms then search configured directories and finally the working
    // directory. Rooted paths bypass that search; the first existing match wins.
    bool TryResolveIncludePath(string currentFile, string includeSpec, bool isAngleForm, out string includePath)
    {
        includePath = null;
        if (string.IsNullOrWhiteSpace(includeSpec)) return false;

        // Absolute path include is accepted as-is.
        if (Path.IsPathRooted(includeSpec))
        {
            string full = Path.GetFullPath(includeSpec);
            if (File.Exists(full))
            {
                includePath = full;
                return true;
            }
            return false;
        }

        var searchDirs = new List<string>();
        if (!isAngleForm)
        {
            try
            {
                string baseDir = Path.GetDirectoryName(currentFile) ?? "";
                if (!string.IsNullOrWhiteSpace(baseDir)) searchDirs.Add(baseDir);
            }
            catch { }
        }

        if (IncludeState != null)
        {
            foreach (var d in IncludeState.IncludeDirs)
            {
                if (!string.IsNullOrWhiteSpace(d)) searchDirs.Add(d);
            }
        }

        // Last fallback: current directory.
        searchDirs.Add(Environment.CurrentDirectory);

        foreach (string d in searchDirs)
        {
            try
            {
                string full = Path.GetFullPath(Path.Combine(d, includeSpec));
                if (File.Exists(full))
                {
                    includePath = full;
                    return true;
                }
            }
            catch
            {
                // ignore invalid path fragments
            }
        }
        return false;
    }

    // Accept ASCII letters, underscore or dollar as a directive/macro identifier start.
    static bool IsNameStartChar(char c)
    {
        return (c == '_') || (c == '$') || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');
    }

    // Skip spaces and tabs only, keeping directive parsing on its current line.
    void SkipHorizontalSpaces()
    {
        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();
    }

    // Require a valid identifier start, then consume its complete name. Failure
    // before the first character leaves the input untouched and returns null.
    bool TryReadIdentifierToken(out string ident)
    {
        ident = null;
        if (!IsNameStartChar(GetNextChar())) return false;

        StringBuilder sb = new StringBuilder();
        while (IsNameChar(GetNextChar()))
        {
            sb.Append(GetNextChar());
            FetchChar();
        }
        ident = sb.ToString();
        return true;
    }

    // Read unique comma-separated parameter names after the opening parenthesis,
    // including an empty list. Failure may consume input; the directive caller owns recovery.
    bool TryReadParameterList(out string[] parameters)
    {
        var ps = new List<string>();
        SkipHorizontalSpaces();

        if (TryRead(')'))
        {
            parameters = Array.Empty<string>();
            return true;
        }

        while (true)
        {
            SkipHorizontalSpaces();
            if (!TryReadIdentifierToken(out string p))
            {
                parameters = Array.Empty<string>();
                return false;
            }
            if (ps.Contains(p))
            {
                parameters = Array.Empty<string>();
                return false;
            }
            ps.Add(p);

            SkipHorizontalSpaces();
            if (TryRead(')'))
            {
                parameters = ps.ToArray();
                return true;
            }
            if (!TryRead(','))
            {
                parameters = Array.Empty<string>();
                return false;
            }
        }
    }

    // Collect uninterpreted text up to CR, LF or EOF without consuming the terminator.
    string ReadUntilLineEndRaw()
    {
        StringBuilder sb = new StringBuilder();
        while (true)
        {
            char c = GetNextChar();
            if (c == '\0' || c == '\n' || c == '\r') break;
            sb.Append(c);
            FetchChar();
        }
        return sb.ToString();
    }

    // Tokenize a replacement or argument with automatic expansion disabled, sharing
    // the current macro/include context. A later expansion pass handles its names.
    List<Token> TokenizeMacroFragment(string fragment, FilePosition pos)
    {
        if (string.IsNullOrWhiteSpace(fragment)) return new List<Token>();

        Tokenizer t = new Tokenizer
        {
            Input = fragment,
            Next = 0,
            InputPos = pos,
            InAssembly = false,
            DisableMacroExpansion = true,
            IncludeState = IncludeState,
            EndPosition = pos,
        };

        return t.Tokenize();
    }

    // Copy token payload while assigning the requested diagnostic location, usually the invocation site.
    static Token CloneToken(Token t, FilePosition pos)
    {
        return new Token
        {
            Tag = t.Tag,
            Int = t.Int,
            Name = t.Name,
            Position = pos,
        };
    }

    // Treat unguarded input as active; otherwise use the innermost frame's combined activity.
    bool IsCurrentBranchActive()
    {
        return ConditionalStack.Count == 0 || ConditionalStack.Peek().BranchActive;
    }

    // Test either macro table without expanding the replacement text.
    bool IsMacroDefined(string name)
    {
        if (string.IsNullOrEmpty(name) || IncludeState == null) return false;
        return IncludeState.ObjectMacros.ContainsKey(name) || IncludeState.FunctionMacros.ContainsKey(name);
    }

    // Maintain if/ifdef/ifndef/elif/else/endif nesting. Evaluate expressions only
    // when the parent is active and no earlier branch was taken; diagnose unmatched
    // or repeated branch directives. Nonconditional names restore the input cursor.
    bool TryHandleConditionalDirective(FilePosition hashPos)
    {
        int saveNext = Next;
        FilePosition savePos = InputPos;

        SkipHorizontalSpaces();
        if (!TryReadIdentifierToken(out string directive))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        if (directive == "if")
        {
            bool parentActive = IsCurrentBranchActive();
            SkipHorizontalSpaces();
            string exprText = ReadUntilLineEndRaw();
            bool cond = false;
            if (parentActive)
            {
                cond = EvaluateIfDirectiveExpression(exprText, hashPos) != 0;
            }

            bool active = parentActive && cond;
            ConditionalStack.Push(new ConditionalFrame
            {
                ParentActive = parentActive,
                BranchActive = active,
                AnyTaken = active,
                SeenElse = false,
                StartPos = hashPos,
            });
            return true;
        }

        if (directive == "ifdef" || directive == "ifndef")
        {
            bool parentActive = IsCurrentBranchActive();
            SkipHorizontalSpaces();
            string macroName = ReadIdentifier();
            bool cond = false;

            if (string.IsNullOrEmpty(macroName))
            {
                Program.Warning(hashPos, "malformed #{0} ignored", directive);
            }
            else
            {
                bool defined = IsMacroDefined(macroName);
                cond = directive == "ifdef" ? defined : !defined;
            }

            bool active = parentActive && cond;
            ConditionalStack.Push(new ConditionalFrame
            {
                ParentActive = parentActive,
                BranchActive = active,
                AnyTaken = active,
                SeenElse = false,
                StartPos = hashPos,
            });
            return true;
        }

        if (directive == "elif")
        {
            if (ConditionalStack.Count == 0)
            {
                Program.Error(hashPos, "unexpected #elif");
                return true;
            }

            var frame = ConditionalStack.Peek();
            SkipHorizontalSpaces();
            string exprText = ReadUntilLineEndRaw();
            if (frame.SeenElse)
            {
                Program.Error(hashPos, "#elif after #else");
                frame.BranchActive = false;
                return true;
            }

            bool cond = false;
            if (frame.ParentActive && !frame.AnyTaken)
            {
                cond = EvaluateIfDirectiveExpression(exprText, hashPos) != 0;
            }

            bool active = frame.ParentActive && !frame.AnyTaken && cond;
            frame.BranchActive = active;
            if (active) frame.AnyTaken = true;
            return true;
        }

        if (directive == "else")
        {
            if (ConditionalStack.Count == 0)
            {
                Program.Error(hashPos, "unexpected #else");
                return true;
            }

            var frame = ConditionalStack.Peek();
            if (frame.SeenElse)
            {
                Program.Error(hashPos, "duplicate #else");
            }

            frame.SeenElse = true;
            bool active = frame.ParentActive && !frame.AnyTaken;
            frame.BranchActive = active;
            if (active) frame.AnyTaken = true;
            return true;
        }

        if (directive == "endif")
        {
            if (ConditionalStack.Count == 0)
            {
                Program.Error(hashPos, "unexpected #endif");
            }
            else
            {
                ConditionalStack.Pop();
            }
            return true;
        }

        Next = saveNext;
        InputPos = savePos;
        return false;
    }

    // Tokenize an if/elif expression and evaluate it with a fresh macro-recursion guard.
    long EvaluateIfDirectiveExpression(string exprText, FilePosition hashPos)
    {
        List<Token> tokens = TokenizeMacroFragment(exprText ?? "", hashPos);
        return EvaluateIfDirectiveExpressionTokens(tokens, hashPos, new HashSet<string>(StringComparer.Ordinal), 0);
    }

    // Evaluate the supported integer-expression subset using precedence-ordered
    // recursive descent. Expand object macros with a recursion guard; undefined
    // identifiers evaluate to zero. This is separate from the language expression parser.
    long EvaluateIfDirectiveExpressionTokens(List<Token> tokens, FilePosition hashPos, HashSet<string> activeMacros, int depth, bool evaluate = true)
    {
        if (tokens == null || tokens.Count == 0) return 0;

        if (depth > MacroExpansionDepthLimit)
        {
            Program.Warning(hashPos, "macro expansion depth exceeded ({0}) in #if expression", MacroExpansionDepthLimit);
            return 0;
        }

        int i = 0;

        // Peek at the expression token kind, returning synthetic EOF without advancing.
        TokenType PeekTag()
        {
            if (i >= tokens.Count) return TokenType.EOF;
            return tokens[i].Tag;
        }

        // Consume one expression token or return a synthetic EOF at the directive location.
        Token Take()
        {
            if (i >= tokens.Count)
            {
                return new Token { Tag = TokenType.EOF, Position = hashPos };
            }
            return tokens[i++];
        }

        // Consume a matching expression token kind; preserve the index on mismatch.
        bool TryTake(TokenType t)
        {
            if (PeekTag() != t) return false;
            i++;
            return true;
        }

        // Read a literal, parenthesized expression, defined test or object-macro value.
        // Names without an eligible object replacement contribute zero.
        long ParsePrimary()
        {
            if (TryTake(TokenType.LPAREN))
            {
                long v = ParseLogicalOr();
                if (!TryTake(TokenType.RPAREN))
                {
                    Program.Warning(hashPos, "missing ')' in #if expression");
                }
                return v;
            }

            Token t = Take();
            if (t.Tag == TokenType.INT) return t.Int;

            if (t.Tag == TokenType.NAME)
            {
                if (t.Name == "defined")
                {
                    string macroName = null;
                    if (TryTake(TokenType.LPAREN))
                    {
                        if (PeekTag() == TokenType.NAME)
                        {
                            macroName = Take().Name;
                        }
                        else
                        {
                            Program.Warning(hashPos, "malformed defined(...) in #if expression");
                        }

                        if (!TryTake(TokenType.RPAREN))
                        {
                            Program.Warning(hashPos, "missing ')' after defined(...)");
                        }
                    }
                    else if (PeekTag() == TokenType.NAME)
                    {
                        macroName = Take().Name;
                    }
                    else
                    {
                        Program.Warning(hashPos, "malformed defined usage in #if expression");
                    }

                    return IsMacroDefined(macroName) ? 1 : 0;
                }

                if (IncludeState != null &&
                    IncludeState.ObjectMacros.TryGetValue(t.Name, out ObjectMacro macro) &&
                    !activeMacros.Contains(t.Name))
                {
                    var activeNext = new HashSet<string>(activeMacros, StringComparer.Ordinal);
                    activeNext.Add(t.Name);
                    return EvaluateIfDirectiveExpressionTokens(macro.Replacement, hashPos, activeNext, depth + 1, evaluate);
                }

                return 0;
            }

            return 0;
        }

        // Bind unary plus, negation, logical not and bitwise complement before binary operators.
        long ParseUnary()
        {
            if (TryTake(TokenType.PLUS)) return ParseUnary();
            if (TryTake(TokenType.MINUS)) return -ParseUnary();
            if (TryTake(TokenType.LOGICAL_NOT)) return ParseUnary() == 0 ? 1 : 0;
            if (TryTake(TokenType.TILDE)) return ~ParseUnary();
            return ParsePrimary();
        }

        // Fold multiplication, division and remainder from left to right at unary precedence.
        long ParseMulDivMod()
        {
            long v = ParseUnary();
            while (true)
            {
                if (TryTake(TokenType.STAR))
                {
                    v = v * ParseUnary();
                }
                else if (TryTake(TokenType.SLASH))
                {
                    long r = ParseUnary();
                    if (r == 0)
                    {
                        if (evaluate) Program.Warning(hashPos, "division by zero in #if expression");
                        v = 0;
                    }
                    else
                    {
                        v = evaluate ? v / r : 0;
                    }
                }
                else if (TryTake(TokenType.PERCENT))
                {
                    long r = ParseUnary();
                    if (r == 0)
                    {
                        if (evaluate) Program.Warning(hashPos, "modulo by zero in #if expression");
                        v = 0;
                    }
                    else
                    {
                        v = evaluate ? v % r : 0;
                    }
                }
                else
                {
                    break;
                }
            }
            return v;
        }

        // Fold addition and subtraction over complete multiplicative expressions.
        long ParseAddSub()
        {
            long v = ParseMulDivMod();
            while (true)
            {
                if (TryTake(TokenType.PLUS)) v = v + ParseMulDivMod();
                else if (TryTake(TokenType.MINUS)) v = v - ParseMulDivMod();
                else break;
            }
            return v;
        }

        // Fold shifts after additive expressions, masking each shift count to 0..63.
        long ParseShift()
        {
            long v = ParseAddSub();
            while (true)
            {
                if (TryTake(TokenType.SHIFT_LEFT))
                {
                    int shift = (int)(ParseAddSub() & 63);
                    v = v << shift;
                }
                else if (TryTake(TokenType.SHIFT_RIGHT))
                {
                    int shift = (int)(ParseAddSub() & 63);
                    v = v >> shift;
                }
                else
                {
                    break;
                }
            }
            return v;
        }

        // Compare shift expressions from left to right, representing each result as zero or one.
        long ParseRelational()
        {
            long v = ParseShift();
            while (true)
            {
                if (TryTake(TokenType.LESS_THAN)) v = v < ParseShift() ? 1 : 0;
                else if (TryTake(TokenType.LESS_THAN_OR_EQUAL)) v = v <= ParseShift() ? 1 : 0;
                else if (TryTake(TokenType.GREATER_THAN)) v = v > ParseShift() ? 1 : 0;
                else if (TryTake(TokenType.GREATER_THAN_OR_EQUAL)) v = v >= ParseShift() ? 1 : 0;
                else break;
            }
            return v;
        }

        // Apply equality and inequality after relational operators, producing integer truth values.
        long ParseEquality()
        {
            long v = ParseRelational();
            while (true)
            {
                if (TryTake(TokenType.DOUBLE_EQUAL)) v = v == ParseRelational() ? 1 : 0;
                else if (TryTake(TokenType.NOT_EQUAL)) v = v != ParseRelational() ? 1 : 0;
                else break;
            }
            return v;
        }

        // Fold bitwise AND over equality expressions.
        long ParseBitAnd()
        {
            long v = ParseEquality();
            while (TryTake(TokenType.AMPERSAND))
            {
                v = v & ParseEquality();
            }
            return v;
        }

        // Fold bitwise XOR after bitwise AND.
        long ParseBitXor()
        {
            long v = ParseBitAnd();
            while (TryTake(TokenType.CARET))
            {
                v = v ^ ParseBitAnd();
            }
            return v;
        }

        // Fold bitwise OR after bitwise XOR.
        long ParseBitOr()
        {
            long v = ParseBitXor();
            while (TryTake(TokenType.PIPE))
            {
                v = v | ParseBitXor();
            }
            return v;
        }

        // Handle logical AND at the precedence level above logical OR.
        long ParseLogicalAnd()
        {
            long v = ParseBitOr();
            while (TryTake(TokenType.LOGICAL_AND))
            {
                // Always consume the right operand, even when its value cannot affect
                // the result. Suppress arithmetic diagnostics only within the unevaluated branch.
                bool previousEvaluation = evaluate;
                evaluate = evaluate && v != 0;
                long right = ParseBitOr();
                evaluate = previousEvaluation;
                v = (v != 0 && right != 0) ? 1 : 0;
            }
            return v;
        }

        // Handle logical OR as the outermost supported binary-expression level.
        long ParseLogicalOr()
        {
            long v = ParseLogicalAnd();
            while (TryTake(TokenType.LOGICAL_OR))
            {
                // Parse past a short-circuited operand so later operators and closing
                // parentheses are still consumed at the correct precedence level.
                bool previousEvaluation = evaluate;
                evaluate = evaluate && v == 0;
                long right = ParseLogicalAnd();
                evaluate = previousEvaluation;
                v = (v != 0 || right != 0) ? 1 : 0;
            }
            return v;
        }

        long result = ParseLogicalOr();
        return result;
    }

    // Store an object or function macro without expanding its replacement. Function
    // syntax requires an immediate opening parenthesis; redefining a name removes
    // its entry from the opposite macro table.
    bool TryReadDefine(FilePosition hashPos)
    {
        int saveNext = Next;
        FilePosition savePos = InputPos;

        SkipHorizontalSpaces();
        if (!TryRead("define"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        SkipHorizontalSpaces();
        if (!TryReadIdentifierToken(out string macroName))
        {
            Program.Warning(hashPos, "malformed #define ignored");
            return true;
        }

        if (IncludeState == null) IncludeState = new IncludeContext();

        // C-preprocessor rule: function-like macros require no whitespace before '('.
        if (GetNextChar() == '(')
        {
            FetchChar();
            if (!TryReadParameterList(out string[] parameters))
            {
                Program.Warning(hashPos, "malformed function-like #define ignored: " + macroName);
                return true;
            }

            SkipHorizontalSpaces();
            string replacementTextFn = ReadUntilLineEndRaw();
            List<Token> replacementFn = TokenizeMacroFragment(replacementTextFn, hashPos);

            var fnMacro = new FunctionMacro
            {
                Name = macroName,
                Parameters = parameters,
                Replacement = replacementFn,
                DefinedAt = hashPos,
            };
            for (int i = 0; i < parameters.Length; i++)
            {
                fnMacro.ParamIndex[parameters[i]] = i;
            }

            IncludeState.FunctionMacros[macroName] = fnMacro;
            IncludeState.ObjectMacros.Remove(macroName);
            return true;
        }

        SkipHorizontalSpaces();
        string replacementText = ReadUntilLineEndRaw();
        List<Token> replacement = TokenizeMacroFragment(replacementText, hashPos);
        var objMacro = new ObjectMacro
        {
            Name = macroName,
            Replacement = replacement,
            DefinedAt = hashPos,
        };

        IncludeState.ObjectMacros[macroName] = objMacro;
        IncludeState.FunctionMacros.Remove(macroName);
        return true;
    }

    // Remove a name from both macro tables; an absent name is harmless.
    // A malformed recognized directive warns, while a nonmatch restores the cursor.
    bool TryReadUndef(FilePosition hashPos)
    {
        int saveNext = Next;
        FilePosition savePos = InputPos;

        SkipHorizontalSpaces();
        if (!TryRead("undef"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        SkipHorizontalSpaces();
        if (!TryReadIdentifierToken(out string macroName))
        {
            Program.Warning(hashPos, "malformed #undef ignored");
            return true;
        }

        if (IncludeState != null)
        {
            IncludeState.ObjectMacros.Remove(macroName);
            IncludeState.FunctionMacros.Remove(macroName);
        }

        return true;
    }

    // Emit the rest of a warning/error directive as literal diagnostic text.
    // Use a fixed format string so braces in the user message are not interpreted.
    bool TryReadDiagnosticDirective(FilePosition hashPos)
    {
        int saveNext = Next;
        FilePosition savePos = InputPos;

        SkipHorizontalSpaces();
        if (!TryReadIdentifierToken(out string directive) ||
            (directive != "warning" && directive != "error"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        SkipHorizontalSpaces();
        string rawMessage = ReadUntilLineEndRaw().Trim();
        string message = string.IsNullOrEmpty(rawMessage)
            ? ("#" + directive)
            : ("#" + directive + ": " + rawMessage);

        if (directive == "warning")
        {
            Program.Warning(hashPos, ErrorCode.PreprocessorWarning, "{0}", message);
        }
        else
        {
            Program.Error(hashPos, ErrorCode.PreprocessorError, "{0}", message);
        }

        return true;
    }

    // Split raw macro arguments after the opening parenthesis, respecting nested
    // parentheses and quoted text with escapes. The closing parenthesis is consumed;
    // EOF reports failure and lets the caller restore its saved cursor.
    bool TryReadInvocationArgumentsRaw(out List<string> args)
    {
        args = new List<string>();
        StringBuilder current = new StringBuilder();
        int depth = 0;
        bool sawNonWhitespace = false;

        while (true)
        {
            char c = GetNextChar();
            if (c == '\0') return false;

            if (c == '"')
            {
                current.Append(c);
                FetchChar();
                while (true)
                {
                    char s = GetNextChar();
                    if (s == '\0') return false;
                    current.Append(s);
                    FetchChar();
                    if (s == '\\')
                    {
                        char e = GetNextChar();
                        if (e == '\0') return false;
                        current.Append(e);
                        FetchChar();
                        continue;
                    }
                    if (s == '"') break;
                }
                sawNonWhitespace = true;
                continue;
            }

            if (c == '\'')
            {
                current.Append(c);
                FetchChar();
                while (true)
                {
                    char s = GetNextChar();
                    if (s == '\0') return false;
                    current.Append(s);
                    FetchChar();
                    if (s == '\\')
                    {
                        char e = GetNextChar();
                        if (e == '\0') return false;
                        current.Append(e);
                        FetchChar();
                        continue;
                    }
                    if (s == '\'') break;
                }
                sawNonWhitespace = true;
                continue;
            }

            if (c == '(')
            {
                depth++;
                current.Append(c);
                FetchChar();
                sawNonWhitespace = true;
                continue;
            }

            if (c == ')')
            {
                if (depth == 0)
                {
                    FetchChar();
                    if (args.Count > 0 || sawNonWhitespace || current.ToString().Trim().Length != 0)
                        args.Add(current.ToString());
                    return true;
                }

                depth--;
                current.Append(c);
                FetchChar();
                sawNonWhitespace = true;
                continue;
            }

            if (c == ',' && depth == 0)
            {
                args.Add(current.ToString());
                current.Clear();
                FetchChar();
                sawNonWhitespace = false;
                continue;
            }

            current.Append(c);
            if (!char.IsWhiteSpace(c)) sawNonWhitespace = true;
            FetchChar();
        }
    }

    // Split a tokenized invocation at outer-level commas, retaining nested parentheses.
    // Return the closing-token index without modifying the input token list.
    bool TryParseInvocationFromTokens(List<Token> tokens, int lparenIndex, out List<List<Token>> args, out int endIndex)
    {
        args = new List<List<Token>>();
        endIndex = -1;

        if (lparenIndex < 0 || lparenIndex >= tokens.Count || tokens[lparenIndex].Tag != TokenType.LPAREN)
            return false;

        int depth = 0;
        List<Token> current = new List<Token>();

        for (int i = lparenIndex + 1; i < tokens.Count; i++)
        {
            Token t = tokens[i];
            if (t.Tag == TokenType.LPAREN)
            {
                depth++;
                current.Add(t);
                continue;
            }
            if (t.Tag == TokenType.RPAREN)
            {
                if (depth == 0)
                {
                    if (args.Count > 0 || current.Count > 0)
                        args.Add(current);
                    endIndex = i;
                    return true;
                }
                depth--;
                current.Add(t);
                continue;
            }
            if (t.Tag == TokenType.COMMA && depth == 0)
            {
                args.Add(current);
                current = new List<Token>();
                continue;
            }
            current.Add(t);
        }

        return false;
    }

    // Recursively substitute eligible function and object macros into a fresh list.
    // Expand arguments before substitution, suppress active macro names to stop cycles,
    // and retain unexpanded token copies when the depth limit is exceeded.
    List<Token> ExpandMacrosInTokenList(List<Token> input, FilePosition callPos, HashSet<string> active, int depth)
    {
        if (input == null || input.Count == 0) return new List<Token>();
        if (IncludeState == null ||
            (IncludeState.FunctionMacros.Count == 0 && IncludeState.ObjectMacros.Count == 0))
        {
            return input.Select(t => CloneToken(t, t.Position)).ToList();
        }
        if (depth > MacroExpansionDepthLimit)
        {
            Program.Warning(callPos, "macro expansion depth exceeded ({0}); expansion truncated", MacroExpansionDepthLimit);
            return input.Select(t => CloneToken(t, t.Position)).ToList();
        }

        List<Token> outTokens = new List<Token>();
        for (int i = 0; i < input.Count; i++)
        {
            Token t = input[i];
            if (t.Tag == TokenType.NAME &&
                IncludeState.FunctionMacros.TryGetValue(t.Name, out FunctionMacro macro) &&
                !active.Contains(macro.Name))
            {
                int lparen = i + 1;
                if (lparen < input.Count && input[lparen].Tag == TokenType.LPAREN &&
                    TryParseInvocationFromTokens(input, lparen, out List<List<Token>> argTokenLists, out int endIndex))
                {
                    if (argTokenLists.Count != macro.Parameters.Length)
                    {
                        Program.Error(t.Position, "macro '{0}' expects {1} arguments, got {2}", macro.Name, macro.Parameters.Length, argTokenLists.Count);
                    }

                    var activeNext = new HashSet<string>(active, StringComparer.Ordinal);
                    activeNext.Add(macro.Name);

                    var expandedArgs = new List<List<Token>>();
                    for (int ai = 0; ai < argTokenLists.Count; ai++)
                    {
                        expandedArgs.Add(ExpandMacrosInTokenList(argTokenLists[ai], t.Position, activeNext, depth + 1));
                    }

                    var substituted = new List<Token>();
                    foreach (Token rt in macro.Replacement)
                    {
                        if (rt.Tag == TokenType.NAME && macro.ParamIndex.TryGetValue(rt.Name, out int pidx))
                        {
                            foreach (Token at in expandedArgs[pidx])
                                substituted.Add(CloneToken(at, t.Position));
                        }
                        else
                        {
                            substituted.Add(CloneToken(rt, t.Position));
                        }
                    }

                    outTokens.AddRange(ExpandMacrosInTokenList(substituted, t.Position, activeNext, depth + 1));
                    i = endIndex;
                    continue;
                }
            }

            if (t.Tag == TokenType.NAME &&
                IncludeState.ObjectMacros.TryGetValue(t.Name, out ObjectMacro objectMacro) &&
                !active.Contains(objectMacro.Name))
            {
                var activeNext = new HashSet<string>(active, StringComparer.Ordinal);
                activeNext.Add(objectMacro.Name);
                var substituted = objectMacro.Replacement.Select(rt => CloneToken(rt, t.Position)).ToList();
                outTokens.AddRange(ExpandMacrosInTokenList(substituted, t.Position, activeNext, depth + 1));
                continue;
            }

            outTokens.Add(CloneToken(t, t.Position));
        }

        return outTokens;
    }

    // Recognize a raw function-macro invocation following its name, tokenize and
    // expand its arguments, substitute parameters and rescan the replacement.
    // A missing/malformed argument list restores the input cursor and returns false.
    bool TryExpandFunctionMacroInvocation(FilePosition callPos, string macroName, List<Token> outputTokens, HashSet<string> active, int depth)
    {
        if (IncludeState == null) return false;
        if (!IncludeState.FunctionMacros.TryGetValue(macroName, out FunctionMacro macro)) return false;
        if (active.Contains(macroName)) return false;
        if (depth > MacroExpansionDepthLimit)
        {
            Program.Warning(callPos, "macro expansion depth exceeded ({0}); expansion skipped for '{1}'", MacroExpansionDepthLimit, macroName);
            return false;
        }

        int saveNext = Next;
        FilePosition savePos = InputPos;

        SkipHorizontalSpaces();
        if (!TryRead('('))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        if (!TryReadInvocationArgumentsRaw(out List<string> argTexts))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        if (argTexts.Count != macro.Parameters.Length)
        {
            Program.Error(callPos, "macro '{0}' expects {1} arguments, got {2}", macroName, macro.Parameters.Length, argTexts.Count);
        }

        var activeNext = new HashSet<string>(active, StringComparer.Ordinal);
        activeNext.Add(macroName);

        var expandedArgs = new List<List<Token>>();
        for (int i = 0; i < argTexts.Count; i++)
        {
            List<Token> rawArg = TokenizeMacroFragment(argTexts[i], callPos);
            expandedArgs.Add(ExpandMacrosInTokenList(rawArg, callPos, activeNext, depth + 1));
        }

        var substituted = new List<Token>();
        foreach (Token rt in macro.Replacement)
        {
            if (rt.Tag == TokenType.NAME && macro.ParamIndex.TryGetValue(rt.Name, out int pidx))
            {
                foreach (Token at in expandedArgs[pidx])
                    substituted.Add(CloneToken(at, callPos));
            }
            else
            {
                substituted.Add(CloneToken(rt, callPos));
            }
        }

        outputTokens.AddRange(ExpandMacrosInTokenList(substituted, callPos, activeNext, depth + 1));
        return true;
    }

    // Expand one object macro at the call location, carrying its active-name guard
    // into recursive replacement scanning. Return false for unavailable or suppressed names.
    bool TryExpandObjectMacroToken(FilePosition callPos, string macroName, List<Token> outputTokens, HashSet<string> active, int depth)
    {
        if (IncludeState == null) return false;
        if (!IncludeState.ObjectMacros.TryGetValue(macroName, out ObjectMacro macro)) return false;
        if (active.Contains(macroName)) return false;
        if (depth > MacroExpansionDepthLimit)
        {
            Program.Warning(callPos, "macro expansion depth exceeded ({0}); expansion skipped for '{1}'", MacroExpansionDepthLimit, macroName);
            return false;
        }

        var activeNext = new HashSet<string>(active, StringComparer.Ordinal);
        activeNext.Add(macroName);
        var substituted = macro.Replacement.Select(t => CloneToken(t, callPos)).ToList();
        outputTokens.AddRange(ExpandMacrosInTokenList(substituted, callPos, activeNext, depth + 1));
        return true;
    }

    // Read a signed decimal bank directive and restore both cursors on a nonmatch.
    // Validation of the bank range belongs to the later compilation stages.
    bool TryReadPragmaBank(out int bank)
    {
        // Called immediately after reading '#'. Parses: pragma bank N
        bank = 0;

        int saveNext = Next;
        FilePosition savePos = InputPos;

        // Skip whitespace
        while (GetNextChar() == ' ' || GetNextChar() == '	') FetchChar();

        if (!TryRead("pragma"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        while (GetNextChar() == ' ' || GetNextChar() == '	') FetchChar();

        if (!TryRead("bank"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        while (GetNextChar() == ' ' || GetNextChar() == '	') FetchChar();

        // Parse integer (decimal)
        bool neg = false;
        if (TryRead('+')) { }
        else if (TryRead('-')) neg = true;

        int value = 0;
        bool any = false;
        while (char.IsDigit(GetNextChar()))
        {
            any = true;
            value = value * 10 + (GetNextChar() - '0');
            FetchChar();
        }

        if (!any)
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        if (neg) value = -value;
        bank = value;
        return true;
    }

    // Read a signed decimal fixed-bank directive, preserving negative sentinel values
    // for later interpretation; restore both cursors when recognition fails.
    bool TryReadPragmaFixedBank(out int bank)
    {
        // Called immediately after reading '#'. Parses: pragma fixed_bank N
        bank = 0;

        int saveNext = Next;
        FilePosition savePos = InputPos;

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();

        if (!TryRead("pragma"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();

        if (!TryRead("fixed_bank"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();

        bool neg = false;
        if (TryRead('+')) { }
        else if (TryRead('-')) neg = true;

        int value = 0;
        bool any = false;
        while (char.IsDigit(GetNextChar()))
        {
            any = true;
            value = value * 10 + (GetNextChar() - '0');
            FetchChar();
        }

        if (!any)
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        if (neg) value = -value;
        bank = value;
        return true;
    }

    // Read signed decimal fixed-bank placement order; leave semantic validation to the parser.
    bool TryReadPragmaFixedOrder(out int order)
    {
        // Called immediately after reading '#'. Parses: pragma fixed_order N
        order = 0;

        int saveNext = Next;
        FilePosition savePos = InputPos;

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();

        if (!TryRead("pragma"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();

        if (!TryRead("fixed_order"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();

        bool neg = false;
        if (TryRead('+')) { }
        else if (TryRead('-')) neg = true;

        int value = 0;
        bool any = false;
        while (char.IsDigit(GetNextChar()))
        {
            any = true;
            value = value * 10 + (GetNextChar() - '0');
            FetchChar();
        }

        if (!any)
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        if (neg) value = -value;
        order = value;
        return true;
    }

    // Read a named four-color palette, allowing commas or whitespace between colors.
    // Each color becomes a packed 15-bit value; restore the full directive on failure.
    bool TryReadPragmaCgbPalette(out string name, out int[] colors)
    {
        name = null;
        colors = null;

        int saveNext = Next;
        FilePosition savePos = InputPos;

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();
        if (!TryRead("pragma"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();

        string directive = ReadIdentifier();
        if (string.IsNullOrEmpty(directive))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        if (!directive.Equals("cgb_palette", StringComparison.OrdinalIgnoreCase))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();
        name = ReadIdentifier();
        if (string.IsNullOrEmpty(name))
        {
            Next = saveNext;
            InputPos = savePos;
            name = null;
            return false;
        }

        var vals = new List<int>();
        for (int i = 0; i < 4; i++)
        {
            while (GetNextChar() == ' ' || GetNextChar() == '\t' || GetNextChar() == ',') FetchChar();

            if (GetNextChar() == '\0' || GetNextChar() == '\n' || GetNextChar() == '\r')
            {
                Next = saveNext;
                InputPos = savePos;
                name = null;
                return false;
            }

            if (!TryReadPaletteColorToken(out int c))
            {
                Next = saveNext;
                InputPos = savePos;
                name = null;
                return false;
            }
            vals.Add(c);
        }

        colors = vals.ToArray();
        return true;
    }

    // Consume any run of name characters, including a leading digit. This permissive
    // helper differs from TryReadIdentifierToken and may return an empty string.
    string ReadIdentifier()
    {
        StringBuilder sb = new StringBuilder();
        while (IsNameChar(GetNextChar()))
        {
            sb.Append(GetNextChar());
            FetchChar();
        }
        return sb.ToString();
    }

    // Convert #RRGGBB to rounded five-bit RGB components, or accept an integer in
    // 0..0x7FFF. This helper can consume a failed token; its directive caller restores state.
    bool TryReadPaletteColorToken(out int color)
    {
        color = 0;

        // #RRGGBB
        if (GetNextChar() == '#')
        {
            FetchChar();
            string hex = "";
            for (int i = 0; i < 6; i++)
            {
                char c = GetNextChar();
                if (!IsHexDigit(c)) return false;
                hex += c;
                FetchChar();
            }

            int rr = HexVal(hex[0]) * 16 + HexVal(hex[1]);
            int gg = HexVal(hex[2]) * 16 + HexVal(hex[3]);
            int bb = HexVal(hex[4]) * 16 + HexVal(hex[5]);

            int r5 = (rr * 31 + 127) / 255;
            int g5 = (gg * 31 + 127) / 255;
            int b5 = (bb * 31 + 127) / 255;
            color = (b5 << 10) | (g5 << 5) | r5;
            return true;
        }

        // numeric token (e.g. 0x7FFF / $7FFF / 32767)
        StringBuilder sb = new StringBuilder();
        while (true)
        {
            char c = GetNextChar();
            if (c == '\0' || c == '\n' || c == '\r' || c == ' ' || c == '\t' || c == ',') break;
            sb.Append(c);
            FetchChar();
        }

        if (sb.Length == 0) return false;
        if (!TryConvertInt(InputPos, sb.ToString(), out int v)) return false;
        if (v < 0 || v > 0x7FFF) return false;
        color = v;
        return true;
    }

    // Read a recognized header option and one quoted or bare value. Restore
    // cursor state for unknown/malformed options so other pragma readers can try them.
    bool TryReadPragmaRomHeader(out string key, out string value)
    {
        // Called immediately after reading '#'. Parses:
        // #pragma rom_title "MYGAME"
        // #pragma cgb cgb_only
        // #pragma cart mbc5
        // #pragma romsize 256k
        // #pragma header_logo off
        // etc.
        key = null;
        value = null;

        int saveNext = Next;
        FilePosition savePos = InputPos;

        // Skip whitespace
        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();

        if (!TryRead("pragma"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();

        // Parse key name
        StringBuilder sb = new StringBuilder();
        while (IsNameChar(GetNextChar()))
        {
            sb.Append(GetNextChar());
            FetchChar();
        }

        if (sb.Length == 0)
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        key = sb.ToString();

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();

        // Parse value: string literal or bare token
        if (GetNextChar() == '"')
        {
            FetchChar();
            StringBuilder vsb = new StringBuilder();
            while (true)
            {
                char c = GetNextChar();
                if (c == '\0' || c == '\n' || c == '\r')
                {
                    // Unterminated string; treat as failure
                    Next = saveNext;
                    InputPos = savePos;
                    return false;
                }
                if (c == '"')
                {
                    FetchChar();
                    break;
                }
                vsb.Append(c);
                FetchChar();
            }
            value = vsb.ToString();
        }
        else
        {
            StringBuilder vsb = new StringBuilder();
            while (true)
            {
                char c = GetNextChar();
                if (c == '\0' || c == '\n' || c == '\r') break;
                if (c == ' ' || c == '\t') break;
                vsb.Append(c);
                FetchChar();
            }

            if (vsb.Length == 0)
            {
                Next = saveNext;
                InputPos = savePos;
                return false;
            }
            value = vsb.ToString();
        }

        // Only accept rom_* pragmas + cart/cgb/romsize/ramsize/sgb/dest/version/header_logo.
        // Others return false so the existing warning path triggers.
        string k = key.Trim();
        if (k.StartsWith("rom_", StringComparison.OrdinalIgnoreCase) ||
            k.Equals("header_logo", StringComparison.OrdinalIgnoreCase) ||
            k.Equals("headerlogo", StringComparison.OrdinalIgnoreCase) ||
            k.Equals("cart", StringComparison.OrdinalIgnoreCase) ||
            k.Equals("cgb", StringComparison.OrdinalIgnoreCase) ||
            k.Equals("romsize", StringComparison.OrdinalIgnoreCase) ||
            k.Equals("ramsize", StringComparison.OrdinalIgnoreCase) ||
            k.Equals("sgb", StringComparison.OrdinalIgnoreCase) ||
            k.Equals("dest", StringComparison.OrdinalIgnoreCase) ||
            k.Equals("version", StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        Next = saveNext;
        InputPos = savePos;
        key = null;
        value = null;
        return false;
    }

    // Placement directives converted into parser-visible tokens:
    // #pragma hram
    // #pragma wram0
    // #pragma wramx
    // #pragma align N
    // #pragma section "NAME"
    // These are converted into dedicated tokens so the parser can update its current placement state.
    // Translate region, WRAM bank, alignment and quoted section directives to
    // parser-visible placement tokens. Numeric bank/alignment values accept decimal
    // or 0x hexadecimal; address and alignment validity are checked downstream.
    bool TryReadPragmaPlacement(out TokenType tag, out int intValue, out string strValue)
    {
        tag = TokenType.INVALID;
        intValue = 0;
        strValue = null;

        int saveNext = Next;
        FilePosition savePos = InputPos;

        // Skip whitespace
        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();

        if (!TryRead("pragma"))
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();

        // Read directive keyword
        StringBuilder sb = new StringBuilder();
        while (IsNameChar(GetNextChar()))
        {
            sb.Append(GetNextChar());
            FetchChar();
        }
        string directive = sb.ToString();
        if (directive.Length == 0)
        {
            Next = saveNext;
            InputPos = savePos;
            return false;
        }

        while (GetNextChar() == ' ' || GetNextChar() == '\t') FetchChar();

        // region shortcuts
        if (directive.Equals("hram", StringComparison.OrdinalIgnoreCase))
        {
            tag = TokenType.PRAGMA_REGION;
            intValue = (int)MemoryRegionTag.HighMem;
            return true;
        }
        if (directive.Equals("wram0", StringComparison.OrdinalIgnoreCase))
        {
            tag = TokenType.PRAGMA_REGION;
            intValue = (int)MemoryRegionTag.Wram0;
            return true;
        }
        if (directive.Equals("wramx", StringComparison.OrdinalIgnoreCase) || directive.Equals("wram1", StringComparison.OrdinalIgnoreCase))
        {
            tag = TokenType.PRAGMA_REGION;
            intValue = (int)MemoryRegionTag.WramX;
            return true;
        }

        if (directive.Equals("wramx_bank", StringComparison.OrdinalIgnoreCase))
        {
            int n = 0;
            bool ok = false;
            if (GetNextChar() == '0' && (PeekAhead(1) == 'x' || PeekAhead(1) == 'X'))
            {
                FetchChar(); FetchChar();
                while (IsHexDigit(GetNextChar()))
                {
                    n = (n << 4) + HexVal(GetNextChar());
                    FetchChar();
                    ok = true;
                }
            }
            else
            {
                while (char.IsDigit(GetNextChar()))
                {
                    n = n * 10 + (GetNextChar() - '0');
                    FetchChar();
                    ok = true;
                }
            }
            if (!ok)
            {
                Next = saveNext;
                InputPos = savePos;
                return false;
            }
            tag = TokenType.PRAGMA_WRAMX_BANK;
            intValue = n;
            return true;
        }

        // alignment
        if (directive.Equals("align", StringComparison.OrdinalIgnoreCase))
        {
            // integer literal (decimal/hex)
            int n = 0;
            bool ok = false;
            if (GetNextChar() == '0' && (PeekAhead(1) == 'x' || PeekAhead(1) == 'X'))
            {
                FetchChar(); FetchChar();
                while (IsHexDigit(GetNextChar()))
                {
                    n = (n << 4) + HexVal(GetNextChar());
                    FetchChar();
                    ok = true;
                }
            }
            else
            {
                while (char.IsDigit(GetNextChar()))
                {
                    n = n * 10 + (GetNextChar() - '0');
                    FetchChar();
                    ok = true;
                }
            }
            if (!ok)
            {
                Next = saveNext;
                InputPos = savePos;
                return false;
            }
            tag = TokenType.PRAGMA_ALIGN;
            intValue = n;
            return true;
        }

        // section name
        if (directive.Equals("section", StringComparison.OrdinalIgnoreCase))
        {
            if (GetNextChar() != '"')
            {
                Next = saveNext;
                InputPos = savePos;
                return false;
            }
            FetchChar();
            StringBuilder vsb = new StringBuilder();
            while (true)
            {
                char c = GetNextChar();
                if (c == '\0' || c == '\n' || c == '\r')
                {
                    Next = saveNext;
                    InputPos = savePos;
                    return false;
                }
                if (c == '"')
                {
                    FetchChar();
                    break;
                }
                vsb.Append(c);
                FetchChar();
            }
            tag = TokenType.PRAGMA_SECTION;
            strValue = vsb.ToString();
            return true;
        }

        // Not a placement pragma.
        Next = saveNext;
        InputPos = savePos;
        return false;
    }

    // Peek relative to the input cursor, returning NUL for either out-of-range direction.
    char PeekAhead(int n)
    {
        int i = Next + n;
        if (i < 0 || i >= Input.Length) return '\0';
        return Input[i];
    }

    // Recognize only ASCII hexadecimal digits, independent of locale.
    static bool IsHexDigit(char c)
    {
        return (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F');
    }

    // Convert a previously validated ASCII hexadecimal digit to its numeric value.
    static int HexVal(char c)
    {
        if (c >= '0' && c <= '9') return c - '0';
        if (c >= 'a' && c <= 'f') return 10 + (c - 'a');
        return 10 + (c - 'A');
    }
    // Accept one non-NUL ASCII character other than a quote or line break.
    // Escape sequences are not decoded by this character-literal reader.
    static bool IsValidCharacterConstant(char c)
    {
        return c <= 127 && c != '\'' && c != '\r' && c != '\n' && c != '\0';
    }
}

[DebuggerDisplay("{Show(),nq}")]
struct Token
{
    public TokenType Tag;
    public int Int;
    public string Name;
    public FilePosition Position;

    // Render a token payload for debugger display, quoting strings and naming other token kinds.
    public string Show()
    {
        if (Tag == TokenType.INT) return Int.ToString();
        else if (Tag == TokenType.NAME) return Name;
        else if (Tag == TokenType.STRING) return string.Format("\"{0}\"", Name);
        else return Tag.ToString();
    }
}

enum TokenType
{
    INVALID,
    EOF,
    NEWLINE,

    LOGICAL_NOT,
    NOT_EQUAL,
    NUMBER_SIGN,
    PERCENT,
    AMPERSAND,
    LPAREN,
    RPAREN,
    STAR,
    PLUS,
    COMMA,
    MINUS,
    ARROW,
    PERIOD,
    SLASH,
    COLON,
    SEMICOLON,
    LESS_THAN,
    LESS_THAN_OR_EQUAL,
    EQUAL,
    DOUBLE_EQUAL,
    GREATER_THAN,
    GREATER_THAN_OR_EQUAL,
    QUESTION_MARK,
    LBRACKET,
    RBRACKET,
    CARET,
    LBRACE,
    PIPE,
    RBRACE,
    TILDE,
    INCREMENT,
    DECREMENT,
    SHIFT_LEFT,
    SHIFT_RIGHT,
    LOGICAL_OR,
    LOGICAL_AND,
    PLUS_EQUALS,
    MINUS_EQUALS,
    STAR_EQUALS,
    SLASH_EQUALS,
    PERCENT_EQUALS,
    SHIFT_LEFT_EQUALS,
    SHIFT_RIGHT_EQUALS,
    AMPERSAND_EQUALS,
    PIPE_EQUALS,
    CARET_EQUALS,

    INT,
    NAME,
    STRING,
    PRAGMA_BANK,
    PRAGMA_FIXED_BANK,
    PRAGMA_FIXED_ORDER,
    PRAGMA_REGION,
    PRAGMA_WRAMX_BANK,
    PRAGMA_ALIGN,
    PRAGMA_SECTION,
}

static class TokenInfo
{
    // Keep spelling entries in exactly the same order as TokenType for indexed diagnostics.
    public static string[] TokenNames = new string[]
    {
        "(invalid)",
        "EOF",
        "newline",

        "!",
        "!=",
        "#",
        "%",
        "&",
        "(",
        ")",
        "*",
        "+",
        ",",
        "-",
        "->",
        ".",
        "/",
        ":",
        ";",
        "<",
        "<=",
        "=",
        "==",
        ">",
        ">=",
        "?",
        "[",
        "]",
        "^",
        "{",
        "|",
        "}",
        "~",
        "++",
        "--",
        "<<",
        ">>",
        "||",
        "&&",
        "+=",
        "-=",
        "*=",
        "/=",
        "%=",
        "<<=",
        ">>=",
        "&=",
        "|=",
        "^=",

        "(int)",
        "(name)",
        "(string)",
        "(pragma_bank)",
        "(pragma_fixed_bank)",
        "(pragma_fixed_order)",
        "(pragma_region)",
        "(pragma_wramx_bank)",
        "(pragma_align)",
        "(pragma_section)",
    };
}

// Store diagnostic coordinates internally as zero-based line and column values.
struct FilePosition
{
    public readonly string Filename;
    public readonly int Line, Column;

    // Capture a source location without path normalization or coordinate conversion.
    public FilePosition(string filename, int line, int column)
    {
        Filename = filename;
        Line = line;
        Column = column;
    }

    public static readonly FilePosition Unknown = new FilePosition("<unknown>", 0, 0);

    // Format stored coordinates as one-based line and column numbers for readers.
    public override string ToString()
    {
        return string.Format("{0} (line {1}, column {2})", Filename, Line + 1, Column + 1);
    }
}




