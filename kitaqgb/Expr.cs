using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

// Expr is treated as a dynamically-typed tuple of symbols, numbers, and sub-expressions.
[DebuggerDisplay("{Show(),nq}")]
class Expr
{
    private readonly object[] Args;
    public readonly FilePosition Source = FilePosition.Unknown;

    // Share an existing argument array while replacing its source position; this constructor skips tuple validation.
    Expr(object[] args, FilePosition source)
    {
        Args = args;
        Source = source;
    }

    // Check allowed non-null tuple element types and retain the supplied array.
    // This does not enforce a tag string at index zero or validate each tag's argument shape.
    Expr(object[] args)
    {
        foreach (object arg in args)
        {
            if (arg == null) throw new Exception("Null in tuple.");
            if (!(arg is int || arg is string || arg is MemoryRegion || arg is CType || arg is FieldInfo[] ||
                arg is Expr || arg is Expr[] || arg is AsmOperand || arg is int[] || arg is byte[]))
            {
                throw new Exception("Unsupported type in tuple: " + arg.GetType());
            }
        }

        Args = args;
    }

    // Construct a validated-element tuple without cloning the argument array.
    public static Expr Make(params object[] args) => new Expr(args);

    // Create an assembly tuple with the shared implicit operand.
    public static Expr MakeAsm(string mnemonic) => MakeAsm(mnemonic, AsmOperand.Implicit);

    // Store an assembly tag, mnemonic and operand for later lowering/assembly.
    public static Expr MakeAsm(string mnemonic, AsmOperand operand)
    {
        return Make(global::Tag.Asm, mnemonic, operand);
    }

    // Return a new node sharing all arguments, with a different source coordinate.
    public Expr WithSource(FilePosition newSource)
    {
        return new Expr(Args, newSource);
    }

    // Read the assumed first string element; malformed or empty tuples throw rather than returning no tag.
    public string GetTag() => (string)Args[0];

    // Convenience accessor used by newer lowering/codegen paths.
    public string Tag => (string)Args[0];

    // Expose the backing argument array directly; callers can mutate shared nodes through this reference.
    public object[] GetArgs() => Args;

    // Compare the assumed first string element only, without enforcing an argument count.
    public bool MatchTag(string tag)
    {
        return (string)Args[0] == tag;
    }

    // Accept any tuple beginning with a string and return that tag; extra arguments are allowed.
    public bool MatchAnyTag(out string tag)
    {
        if (Args.Length >= 1 && Args[0] is string)
        {
            tag = (string)Args[0];
            return true;
        }
        else
        {
            tag = null;
            return false;
        }
    }

    // Match exactly one typed argument after any string tag; reset outputs on failure.
    public bool MatchAnyTag<T1>(out string tag, out T1 var1)
    {
        if (Args.Length == 2 &&
            Args[0] is string &&
            Args[1] is T1)
        {
            tag = (string)Args[0];
            var1 = (T1)Args[1];
            return true;
        }

        tag = null;
        var1 = default(T1);
        return false;
    }

    // Match exactly two typed arguments after any string tag; reset outputs on failure.
    public bool MatchAnyTag<T1, T2>(out string tag, out T1 var1, out T2 var2)
    {
        if (Args.Length == 3 &&
            Args[0] is string &&
            Args[1] is T1 &&
            Args[2] is T2)
        {
            tag = (string)Args[0];
            var1 = (T1)Args[1];
            var2 = (T2)Args[2];
            return true;
        }

        tag = null;
        var1 = default(T1);
        var2 = default(T2);
        return false;
    }

    // Match a tag-only tuple with no payload.
    public bool Match(string tag)
    {
        if (Args.Length == 1 &&
            Args[0] is string && (string)Args[0] == tag)
        {
            return true;
        }

        return false;
    }

    // Match a single typed element without requiring a string tag.
    public bool Match<T0>(out T0 var0)
    {
        if (Args.Length == 1 &&
            Args[0] is T0)
        {
            var0 = (T0)Args[0];
            return true;
        }

        var0 = default(T0);
        return false;
    }

    // Match the exact tag and one typed payload, resetting the output on mismatch.
    public bool Match<T1>(string tag, out T1 var1)
    {
        if (Args.Length == 2 &&
            Args[0] is string && (string)Args[0] == tag &&
            Args[1] is T1)
        {
            var1 = (T1)Args[1];
            return true;
        }

        var1 = default(T1);
        return false;
    }

    // Match the exact tag and two typed payloads, resetting outputs on mismatch.
    public bool Match<T1, T2>(string tag, out T1 var1, out T2 var2)
    {
        if (Args.Length == 3 &&
            Args[0] is string && (string)Args[0] == tag &&
            Args[1] is T1 &&
            Args[2] is T2)
        {
            var1 = (T1)Args[1];
            var2 = (T2)Args[2];
            return true;
        }

        var1 = default(T1);
        var2 = default(T2);
        return false;
    }

    // Match the exact tag and three typed payloads, resetting outputs on mismatch.
    public bool Match<T1, T2, T3>(string tag, out T1 var1, out T2 var2, out T3 var3)
    {
        if (Args.Length == 4 &&
            Args[0] is string && (string)Args[0] == tag &&
            Args[1] is T1 &&
            Args[2] is T2 &&
            Args[3] is T3)
        {
            var1 = (T1)Args[1];
            var2 = (T2)Args[2];
            var3 = (T3)Args[3];
            return true;
        }

        var1 = default(T1);
        var2 = default(T2);
        var3 = default(T3);
        return false;
    }

    // Match the exact tag and four typed payloads, resetting outputs on mismatch.
    public bool Match<T1, T2, T3, T4>(string tag, out T1 var1, out T2 var2, out T3 var3, out T4 var4)
    {
        if (Args.Length == 5 &&
            Args[0] is string && (string)Args[0] == tag &&
            Args[1] is T1 &&
            Args[2] is T2 &&
            Args[3] is T3 &&
            Args[4] is T4)
        {
            var1 = (T1)Args[1];
            var2 = (T2)Args[2];
            var3 = (T3)Args[3];
            var4 = (T4)Args[4];
            return true;
        }

        var1 = default(T1);
        var2 = default(T2);
        var3 = default(T3);
        var4 = default(T4);
        return false;
    }

    // Match the exact tag and five typed payloads, resetting outputs on mismatch.
    public bool Match<T1, T2, T3, T4, T5>(string tag, out T1 var1, out T2 var2, out T3 var3, out T4 var4, out T5 var5)
    {
        if (Args.Length == 6 &&
            Args[0] is string && (string)Args[0] == tag &&
            Args[1] is T1 &&
            Args[2] is T2 &&
            Args[3] is T3 &&
            Args[4] is T4 &&
            Args[5] is T5)
        {
            var1 = (T1)Args[1];
            var2 = (T2)Args[2];
            var3 = (T3)Args[3];
            var4 = (T4)Args[4];
            var5 = (T5)Args[5];
            return true;
        }

        var1 = default(T1);
        var2 = default(T2);
        var3 = default(T3);
        var4 = default(T4);
        var5 = default(T5);
        return false;
    }
    // Match a tag followed by zero or more uniform typed elements; return a new payload array.
    public bool MatchAny<T>(string tag, out T[] vars)
    {
        if (Args.Length >= 1 &&
            Args[0] is string && (string)Args[0] == tag &&
            Args.Skip(1).All(x => x is T))
        {
            vars = Args.Skip(1).Cast<T>().ToArray();
            return true;
        }

        vars = null;
        return false;
    }

    // Match a tag and one leading value plus zero or more uniform trailing values.
    public bool MatchAny<T1, T2>(string tag, out T1 var1, out T2[] vars)
    {
        if (Args.Length >= 2 &&
            Args[0] is string && (string)Args[0] == tag &&
            Args[1] is T1 &&
            Args.Skip(2).All(x => x is T2))
        {
            var1 = (T1)Args[1];
            vars = Args.Skip(2).Cast<T2>().ToArray();
            return true;
        }

        var1 = default(T1);
        vars = null;
        return false;
    }

    // Return any string tag and a new array of its uniformly typed payload, including an empty payload.
    public bool MatchAny<T>(out string tag, out T[] vars)
    {
        if (Args.Length >= 1 &&
            Args[0] is string &&
            Args.Skip(1).All(x => x is T))
        {
            tag = (string)Args[0];
            vars = Args.Skip(1).Cast<T>().ToArray();
            return true;
        }

        tag = null;
        vars = null;
        return false;
    }

    // Render a compact diagnostic tuple rather than executable source code.
    public string Show() => ShowWithOptions(false);

    

    // --- Convenience match helpers for optional metadata extensions ---
    public bool Match(string tag, out MemoryRegion region, out CType type, out string name, out Expr range)
    {
        // (tag, region, type, name) or (tag, region, type, name, range)
        region = default; type = null; name = null; range = null;
        if (Args.Length == 4 && Args[0] is string && (string)Args[0] == tag && Args[1] is MemoryRegion && Args[2] is CType && Args[3] is string)
        {
            region = (MemoryRegion)Args[1];
            type = (CType)Args[2];
            name = (string)Args[3];
            return true;
        }
        if (Args.Length == 5 && Args[0] is string && (string)Args[0] == tag && Args[1] is MemoryRegion && Args[2] is CType && Args[3] is string && Args[4] is Expr)
        {
            region = (MemoryRegion)Args[1];
            type = (CType)Args[2];
            name = (string)Args[3];
            range = (Expr)Args[4];
            return true;
        }
        return false;
    }

    public bool Match(string tag, out CType type, out string name, out Expr range)
    {
        // (tag, type, name) or (tag, type, name, range)
        type = null; name = null; range = null;
        if (Args.Length == 3 && Args[0] is string && (string)Args[0] == tag && Args[1] is CType && Args[2] is string)
        {
            type = (CType)Args[1];
            name = (string)Args[2];
            return true;
        }
        if (Args.Length == 4 && Args[0] is string && (string)Args[0] == tag && Args[1] is CType && Args[2] is string && Args[3] is Expr)
        {
            type = (CType)Args[1];
            name = (string)Args[2];
            range = (Expr)Args[3];
            return true;
        }
        return false;
    }
// Allow indentation for compound tuple trees; small nodes remain on one line.
public string ShowMultiline() => ShowWithOptions(true);

    // Convert tuple values to a display tree, then choose compact or multiline formatting.
    string ShowWithOptions(bool multiline)
    {
        return ShowStringTree(multiline, ToStringTree());
    }

    // The return value is a "string tree": a tree where each node is a string or an array of subtrees.
    object ToStringTree()
    {
        object[] tree = new object[Args.Length];

        for (int i = 0; i < Args.Length; i++)
        {
            int? integer = Args[i] as int?;
            int[] ints = Args[i] as int[];
            byte[] bytes = Args[i] as byte[];
            string name = Args[i] as string;
            Expr subexpr = Args[i] as Expr;
            Expr[] subexprs = Args[i] as Expr[];
            MemoryRegion? region = Args[i] as MemoryRegion?;
            CType type = Args[i] as CType;
            FieldInfo[] fields = Args[i] as FieldInfo[];
            AsmOperand operand = Args[i] as AsmOperand;

            if (integer != null)
            {
                int n = integer.Value;
                tree[i] = (n < 128) ? n.ToString() : "$" + n.ToString("X");
            }
            else if (ints != null)
            {
                tree[i] = "{ " + string.Join(", ", ints.Select(FormatInt)) + " }";
            }
            // This diagnostic byte list prefixes only separators with 0x; it is not a canonical source initializer.
            else if (bytes != null)
            {
                tree[i] = "{ " + string.Join(", 0x", bytes.Select(x => x.ToString("X2"))) + " }";
            }
            else if (name != null)
            {
                tree[i] = name;
            }
            else if (subexpr != null)
            {
                tree[i] = subexpr.ToStringTree();
            }
            else if (subexprs != null)
            {
                tree[i] = subexprs.Select(x => x.ToStringTree()).ToArray();
            }
            else if (region != null)
            {
                tree[i] = region.ToString();
            }
            else if (!ReferenceEquals(type, null))
            {
                tree[i] = "<" + type.Show() + ">";
            }
            else if (fields != null)
            {
                tree[i] = "<fields>";
            }
            else if (operand != null)
            {
                tree[i] = operand.Show();
            }
            else
            {
                Program.NYI();
            }
        }

        return tree;
    }

    // Display values below 128 in decimal, including negatives; larger values use dollar-prefixed hex.
    static string FormatInt(int n) => (n < 128) ? n.ToString() : "$" + n.ToString("X");

    // Format a "string tree" into a multi-line, indented string.
    // Each node must be a string or another "string tree".
    static string ShowStringTree(bool multiline, object tree)
    {
        if (tree is string) return (string)tree;

        IEnumerable<object> subtrees = (IEnumerable<object>)tree;
        IEnumerable<string> substrings = subtrees.Select(x => ShowStringTree(multiline, x));
        bool small = subtrees.Count() <= 2 || !subtrees.Any(x => x is Array);
        if (small || !multiline)
        {
            return "(" + string.Join(" ", substrings) + ")";
        }
        else
        {
            return "(" + string.Join("\n    ", substrings.Select(s => s.Replace("\n", "\n    "))) + ")";
        }
    }
}




