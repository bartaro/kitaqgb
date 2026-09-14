using System;
using System.Collections.Generic;
using System.Linq;

// Whole-program "dead strip" for KITAQGB assembly IR:
// - Remove functions outside a conservative closure of main / RST maps / keep list / code references
// - Remove readonly data blobs with no recognized reference anywhere in the code section
// Safety notes:
// - KITAQGB C subset does not support function pointers (in practice), so call-graph tracing is safe.
// - If you jump/call labels via hand-written asm in ways we can't see, add --strip-keep=label.
static class DeadStripper
{
    // Treat everything before the first ReadonlyData node as code and retain original expression objects.
    // This conservative pass does not compute strict entry-point reachability or discover encoded/indirect references.
    public static IReadOnlyList<Expr> Strip(IReadOnlyList<Expr> assembly, HashSet<string> keepLabels)
    {
        if (assembly == null) return assembly;

        // Collect function definitions + their ranges (code section only).
        var funcStartIdx = new List<int>();
        var funcNames = new List<string>();
        int firstDataIdx = -1;

        for (int i = 0; i < assembly.Count; i++)
        {
            if (firstDataIdx < 0 && assembly[i].MatchTag(Tag.ReadonlyData)) { firstDataIdx = i; break; }
        }

        int scanEnd = (firstDataIdx >= 0) ? firstDataIdx : assembly.Count;

        for (int i = 0; i < scanEnd; i++)
        {
            string fn;
            if (assembly[i].Match(Tag.Function, out fn))
            {
                funcStartIdx.Add(i);
                funcNames.Add(fn);
            }
        }

        // Nothing to strip
        if (funcStartIdx.Count == 0 && firstDataIdx < 0) return assembly;

        var definedFuncs = new HashSet<string>(funcNames, StringComparer.Ordinal);

        // RST map targets must be kept (CALL sites may have been rewritten to RST_xx).
        var rstTargets = new HashSet<string>(StringComparer.Ordinal);
        for (int i = 0; i < assembly.Count; i++)
        {
            int vec;
            string target;
            if (assembly[i].Match(Tag.RstMap, out vec, out target))
            {
                if (!string.IsNullOrEmpty(target)) rstTargets.Add(target);
            }
        }

        // Function ranges
        // Range each function through the next Function marker or the data boundary. Duplicate names overwrite the keyed range.
        var funcRanges = new Dictionary<string, (int start, int end)>(StringComparer.Ordinal);
        for (int k = 0; k < funcStartIdx.Count; k++)
        {
            int start = funcStartIdx[k];
            int end = (k + 1 < funcStartIdx.Count) ? funcStartIdx[k + 1] : scanEnd;
            funcRanges[funcNames[k]] = (start, end);
        }

        // Pre-compute outgoing refs per function (only to absolute label operands / word labels).
        var funcRefs = new Dictionary<string, HashSet<string>>(StringComparer.Ordinal);
        foreach (var fn in funcNames)
        {
            var refs = new HashSet<string>(StringComparer.Ordinal);
            var r = funcRanges[fn];
            for (int i = r.start; i < r.end; i++)
            {
                CollectLabelRefs(assembly[i], refs);
            }
            funcRefs[fn] = refs;
        }

        // Roots
        var liveFuncs = new HashSet<string>(StringComparer.Ordinal);
        var q = new Queue<string>();

        // main is entry
        EnqueueIfDefined("main", definedFuncs, liveFuncs, q);

        // keep list
        // The explicit keep set roots defined functions here; it does not directly mark readonly data as live.
        if (keepLabels != null)
        {
            foreach (var k in keepLabels)
            {
                if (string.IsNullOrEmpty(k)) continue;
                EnqueueIfDefined(k, definedFuncs, liveFuncs, q);
            }
        }

        // RST targets
        foreach (var t in rstTargets)
        {
            EnqueueIfDefined(t, definedFuncs, liveFuncs, q);
        }

        // Conservatively root references from every non-marker code expression, including
        // bodies of functions that might otherwise be dead. Only Function markers are skipped.
        for (int i = 0; i < scanEnd; i++)
        {
            if (assembly[i].MatchTag(Tag.Function)) continue;
            var tmp = new HashSet<string>(StringComparer.Ordinal);
            CollectLabelRefs(assembly[i], tmp);
            foreach (var s in tmp) EnqueueIfDefined(s, definedFuncs, liveFuncs, q);
        }

        // BFS over call graph
        while (q.Count > 0)
        {
            var fn = q.Dequeue();
            if (!funcRefs.TryGetValue(fn, out var refs)) continue;
            foreach (var r in refs)
            {
                EnqueueIfDefined(r, definedFuncs, liveFuncs, q);
            }
        }

        // Collect readonly data labels and which ones are referenced by live code.
        var dataNames = new HashSet<string>(StringComparer.Ordinal);
        for (int i = (firstDataIdx >= 0 ? firstDataIdx : assembly.Count); i < assembly.Count; i++)
        {
            string dn;
            byte[] bytes;
            if (assembly[i].Match(Tag.ReadonlyData, out dn, out bytes))
            {
                if (!string.IsNullOrEmpty(dn)) dataNames.Add(dn);
            }
        }

        var liveData = new HashSet<string>(StringComparer.Ordinal);
        foreach (var fn in liveFuncs)
        {
            if (!funcRanges.TryGetValue(fn, out var r)) continue;
            for (int i = r.start; i < r.end; i++)
            {
                CollectDataRefs(assembly[i], dataNames, liveData);
            }
        }
        // Scan every non-marker expression again, including dead function bodies; their data references are retained.
        for (int i = 0; i < scanEnd; i++)
        {
            if (assembly[i].MatchTag(Tag.Function)) continue;
            CollectDataRefs(assembly[i], dataNames, liveData);
        }

        // Build keep mask
        bool[] keep = new bool[assembly.Count];
        for (int i = 0; i < keep.Length; i++) keep[i] = true;

        // Drop dead functions (code section)
        foreach (var fn in funcNames)
        {
            if (liveFuncs.Contains(fn)) continue;
            var r = funcRanges[fn];
            for (int i = r.start; i < r.end; i++) keep[i] = false;
        }

        // Drop dead readonly data
        if (firstDataIdx >= 0)
        {
            for (int i = firstDataIdx; i < assembly.Count; i++)
            {
                string dn; byte[] bytes;
                if (assembly[i].Match(Tag.ReadonlyData, out dn, out bytes))
                {
                    if (!liveData.Contains(dn)) keep[i] = false;
                }
            }
        }

        // Emit summary (debug output only)
        try
        {
            if (Program.EnableDebugOutput)
            {
                var removedFns = funcNames.Where(n => !liveFuncs.Contains(n)).OrderBy(n => n).ToArray();
                var keptFns = liveFuncs.OrderBy(n => n).ToArray();
                var removedData = dataNames.Where(n => !liveData.Contains(n)).OrderBy(n => n).ToArray();

                var sb = new System.Text.StringBuilder();
                sb.AppendLine("[KITAQGB] dead-strip summary");
                sb.AppendLine("  kept functions : " + keptFns.Length);
                sb.AppendLine("  removed funcs  : " + removedFns.Length);
                sb.AppendLine("  kept rodata    : " + liveData.Count);
                sb.AppendLine("  removed rodata : " + removedData.Length);
                if (keepLabels != null && keepLabels.Count > 0)
                    sb.AppendLine("  keep list      : " + string.Join(",", keepLabels.OrderBy(x => x)));

                if (removedFns.Length > 0)
                {
                    sb.AppendLine();
                    sb.AppendLine("removed functions:");
                    foreach (var n in removedFns) sb.AppendLine("  - " + n);
                }
                if (removedData.Length > 0)
                {
                    sb.AppendLine();
                    sb.AppendLine("removed readonly data:");
                    foreach (var n in removedData) sb.AppendLine("  - " + n);
                }

                Program.WriteDebugFile("deadstrip_summary.txt", sb.ToString());
            }
        }
        catch
        {
            // never fail
        }

        // Preserve the relative order of retained IR nodes without cloning or rewriting them.
        var result = new List<Expr>(assembly.Count);
        for (int i = 0; i < assembly.Count; i++)
        {
            if (keep[i]) result.Add(assembly[i]);
        }
        return result;
    }

    // Queue a nonempty defined function only on its first transition into the live set.
    static void EnqueueIfDefined(string name, HashSet<string> defined, HashSet<string> live, Queue<string> q)
    {
        if (string.IsNullOrEmpty(name)) return;
        if (!defined.Contains(name)) return;
        if (live.Add(name)) q.Enqueue(name);
    }

    // Collect symbolic assembly bases in any addressing mode and string-valued Word labels.
    // Numeric addresses and other expression shapes do not contribute dependencies.
    static void CollectLabelRefs(Expr e, HashSet<string> refs)
    {
        // CALL/JP etc
        string m;
        AsmOperand o;
        if (e.Match(Tag.Asm, out m, out o))
        {
            // Treat any symbol base reference as a dependency, even when it has an offset (e.g. LABEL+12).
            // Offsets are common when indexing into lookup tables (tiles, mino shape tables, etc.).
            if (o.Base.HasValue)
            {
                string name = o.Base.Value;
                if (!string.IsNullOrEmpty(name)) refs.Add(name);
            }
            return;
        }

        // 16-bit words used for jump tables etc
        string w;
        if (e.Match(Tag.Word, out w))
        {
            if (!string.IsNullOrEmpty(w)) refs.Add(w);
            return;
        }
    }

    // Recognize the same symbolic forms but retain only names defined as readonly data.
    static void CollectDataRefs(Expr e, HashSet<string> dataNames, HashSet<string> liveData)
    {
        string m; AsmOperand o;
        if (e.Match(Tag.Asm, out m, out o))
        {
            // Keep readonly data that is referenced with an offset (LABEL+N) as well.
            if (o.Base.HasValue)
            {
                string name = o.Base.Value;
                if (dataNames.Contains(name)) liveData.Add(name);
            }
            return;
        }

        string w;
        if (e.Match(Tag.Word, out w))
        {
            if (dataNames.Contains(w)) liveData.Add(w);
            return;
        }
    }
}




