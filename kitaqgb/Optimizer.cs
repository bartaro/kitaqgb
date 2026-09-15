using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

// Rewrite assembly IR using local patterns and record textual pass summaries.
// Pattern matching is not a whole-program register/flag liveness proof.
class Optimizer
{
    // Read and replace the report stored in the current compiler session.
    public static OptimizerAnalysisReport LastReport
    {
        get { return Program.CurrentOptimizerLastReport; }
        private set { Program.CurrentOptimizerLastReport = value; }
    }

    // Run two base peephole/unreachable passes and load reuse even at level zero;
    // level one or higher additionally enables the final O1 patterns.
    public static List<Expr> Optimize(List<Expr> sourceLines, int optLevel)
    {
        var report = new OptimizerAnalysisReport();

        // Extract RST mapping directives (targetLabel -> vector)
        var rstTargetToVector = BuildRstTargetMap(sourceLines);

        var lines = RunWithDiff("run_pass_1", sourceLines, x => RemoveUnreachable(RunPass(x, rstTargetToVector, report)), report);
        lines = RunWithDiff("run_pass_2", lines, x => RemoveUnreachable(RunPass(x, rstTargetToVector, report)), report);

        // Final pass: conservative basic-block CSE for fixed-address loads
        // (removes redundant reloads and reuses cached registers)
        lines = RunWithDiff("mini_cse", lines, ApplyMiniCsePass, report);

        if (optLevel >= 1)
        {
            lines = RunWithDiff("o1_peepholes", lines, ApplyO1Peepholes, report);
        }
        LastReport = report;
        return lines;
    }

    // Run one pass and compare diagnostic lines by position. Added/removed counts are net length differences,
    // not an edit-distance diff or measurements of encoded instruction bytes.
    static List<Expr> RunWithDiff(string passName, List<Expr> before, Func<List<Expr>, List<Expr>> pass, OptimizerAnalysisReport report)
    {
        var beforeLines = before.Select(ShowExprForDiff).ToList();
        var after = pass(before);
        var afterLines = after.Select(ShowExprForDiff).ToList();

        var pr = new OptimizerPassReport
        {
            Name = passName,
            BeforeLines = beforeLines.Count,
            AfterLines = afterLines.Count
        };

        int min = Math.Min(beforeLines.Count, afterLines.Count);
        int changed = 0;
        for (int i = 0; i < min; i++)
        {
            if (!string.Equals(beforeLines[i], afterLines[i], StringComparison.Ordinal))
                changed++;
        }
        pr.ChangedLines = changed;
        pr.AddedLines = Math.Max(0, afterLines.Count - beforeLines.Count);
        pr.RemovedLines = Math.Max(0, beforeLines.Count - afterLines.Count);
        pr.DiffText = BuildSimpleDiff(beforeLines, afterLines);

        report.Passes.Add(pr);
        return after;
    }

    // Render assembly operands or general IR nodes for reports, omitting implicit operand text.
    static string ShowExprForDiff(Expr e)
    {
        string m;
        AsmOperand o;
        if (e.Match(Tag.Asm, out m, out o))
        {
            if (o == null || o.Mode == AddressMode.Implicit) return m;
            return m + " " + o.Show();
        }
        return e.Show();
    }

    // Show at most 120 differing positions or tail entries, with no insertion alignment.
    // One changed position can emit several text lines; the limit counts entries, not output lines.
    static string BuildSimpleDiff(List<string> before, List<string> after)
    {
        const int MaxLines = 120;
        var sb = new StringBuilder();
        int min = Math.Min(before.Count, after.Count);
        int emitted = 0;

        for (int i = 0; i < min && emitted < MaxLines; i++)
        {
            if (!string.Equals(before[i], after[i], StringComparison.Ordinal))
            {
                sb.AppendLine("@@ line " + (i + 1) + " @@");
                sb.AppendLine("- " + before[i]);
                sb.AppendLine("+ " + after[i]);
                emitted++;
            }
        }

        if (emitted < MaxLines && after.Count > before.Count)
        {
            for (int i = before.Count; i < after.Count && emitted < MaxLines; i++)
            {
                sb.AppendLine("+ " + after[i]);
                emitted++;
            }
        }
        else if (emitted < MaxLines && before.Count > after.Count)
        {
            for (int i = after.Count; i < before.Count && emitted < MaxLines; i++)
            {
                sb.AppendLine("- " + before[i]);
                emitted++;
            }
        }

        if (sb.Length == 0)
        {
            sb.AppendLine("(no textual changes)");
        }
        else if (emitted >= MaxLines)
        {
            sb.AppendLine("... (truncated)");
        }
        return sb.ToString();
    }

    // -O1: local GB instruction-pattern rewrites.
    // - remove no-op moves (LD r,r)
    // - remove unconditional jump to immediate next label
    // - earlier base passes handle linear unreachable instructions separately
    // Remove register self-moves and jumps to a matching following label after skipping comment/section/alignment nodes.
    static List<Expr> ApplyO1Peepholes(List<Expr> lines)
    {
        var outLines = new List<Expr>(lines.Count);

        for (int i = 0; i < lines.Count; i++)
        {
            var cur = lines[i];
            string m; AsmOperand o;
            if (cur.Match(Tag.Asm, out m, out o))
            {
                // No-op register moves: LD A,A etc.
                if (m == "LD_A_A" || m == "LD_B_B" || m == "LD_C_C" || m == "LD_D_D" || m == "LD_E_E" || m == "LD_H_H" || m == "LD_L_L")
                    continue;

                // JP label; label: => drop JP
                if ((m == "JP" || m == "JR") && o.Mode == AddressMode.Absolute && o.Offset == 0 && o.Base.HasValue)
                {
                    // Scan forward skipping comments/sections/align directives.
                    int j = i + 1;
                    while (j < lines.Count)
                    {
                        var n = lines[j];
                        string tag;
                        if (n.MatchAnyTag(out tag))
                        {
                            if (tag == Tag.Comment || tag == Tag.Section || tag == Tag.Align) { j++; continue; }
                        }
                        break;
                    }
                    if (j < lines.Count)
                    {
                        string lab;
                        if (lines[j].Match(Tag.Label, out lab) && lab == o.Base.Value)
                        {
                            continue;
                        }
                    }
                }
            }

            outLines.Add(cur);
        }

        return outLines;
    }

    // Apply adjacent instruction patterns from left to right and copy unmatched expressions.
    // Generated replacements generally omit the original source coordinate; later passes can match their results.
    static List<Expr> RunPass(List<Expr> sourceLines, Dictionary<string, int> rstTargetToVector, OptimizerAnalysisReport report)
    {
        List<Expr> optimized = new List<Expr>();

        for (int i = 0; i < sourceLines.Count; i++)
        {
            Expr current = sourceLines[i];

            string mnemonic;
            AsmOperand operand;
            if (!current.Match(Tag.Asm, out mnemonic, out operand))
            {
                optimized.Add(current);
                continue;
            }

            // RST call shortening: CALL label -> RST_xx (call-like, 1 byte)
            // Requires an RST vector stub (JP label) emitted by the assembler via $rst_map.
            if (mnemonic == "CALL" && operand.Mode == AddressMode.Absolute && operand.Offset == 0 && operand.Base.HasValue)
            {
                int vec;
                if (rstTargetToVector.TryGetValue(operand.Base.Value, out vec))
                {
                    optimized.Add(Expr.MakeAsm($"RST_{vec:X2}"));
                    if (report != null)
                    {
                        report.TotalRstRewrites++;
                        int cur = 0;
                        report.RstRewriteCountsByVector.TryGetValue(vec, out cur);
                        report.RstRewriteCountsByVector[vec] = cur + 1;
                    }
                    continue;
                }
            }

            if (i + 2 < sourceLines.Count)
            {
                Expr next1 = sourceLines[i + 1];
                Expr next2 = sourceLines[i + 2];
                string m1, m2;
                AsmOperand o1, o2;

                if (next1.Match(Tag.Asm, out m1, out o1) && next2.Match(Tag.Asm, out m2, out o2))
                {
                    // Collapse load/modify/store to an in-memory increment or decrement.
                    // Preserve the modified A value and any live carry from immediate arithmetic.
                    if (mnemonic == "LD_A_HL" && m2 == "LD_HL_A")
                    {
                        // INC
                        if (m1 == "INC_A" && CanFoldMemoryUpdate(sourceLines, i + 3, false))
                        {
                            optimized.Add(Expr.MakeAsm("INC_HL_REF"));
                            i += 2;
                            continue;
                        }
                        // DEC
                        if (m1 == "DEC_A" && CanFoldMemoryUpdate(sourceLines, i + 3, false))
                        {
                            optimized.Add(Expr.MakeAsm("DEC_HL_REF"));
                            i += 2;
                            continue;
                        }
                        // ADD A,1
                        if (m1 == "ADD_A_IMM" && o1.Mode == AddressMode.Immediate && o1.Offset == 1 && !o1.Base.HasValue && CanFoldMemoryUpdate(sourceLines, i + 3, true))
                        {
                            optimized.Add(Expr.MakeAsm("INC_HL_REF"));
                            i += 2;
                            continue;
                        }
                        // SUB 1
                        if (m1 == "SUB_IMM" && o1.Mode == AddressMode.Immediate && o1.Offset == 1 && !o1.Base.HasValue && CanFoldMemoryUpdate(sourceLines, i + 3, true))
                        {
                            optimized.Add(Expr.MakeAsm("DEC_HL_REF"));
                            i += 2;
                            continue;
                        }
                    }

                    // LD HL, Label
                    // LD DE, Const
                    // ADD HL, DE
                    // ==> LD HL, Label+Const
                    if (mnemonic == "LD_HL_IMM" && m1 == "LD_DE_IMM" && m2 == "ADD_HL_DE")
                    {
                        bool hlIsImmediate =
                            operand.Mode == AddressMode.Immediate16 || operand.Mode == AddressMode.Immediate;
                        bool deIsImmediate =
                            o1.Mode == AddressMode.Immediate16 || o1.Mode == AddressMode.Immediate;

                        if (hlIsImmediate && deIsImmediate)
                        {
                            // Fold only the second operand's numeric offset; this rule does not check for a symbolic base or extraction modifier there.
                            int offsetToAdd = o1.Offset;
                            AsmOperand newOp = new AsmOperand(operand.Base, operand.Offset + offsetToAdd, AddressMode.Immediate16, ImmediateModifier.None);

                            optimized.Add(Expr.MakeAsm("LD_HL_IMM", newOp));
                            i += 2;
                            continue;
                        }
                    }
                }
            }

            if (i + 1 < sourceLines.Count)
            {
                Expr next = sourceLines[i + 1];
                string nextMnemonic;
                AsmOperand nextOperand;

                if (next.Match(Tag.Asm, out nextMnemonic, out nextOperand))
                {

                    // 0. CP 0 + (JP/JR) Z/NZ -> (drop CP0 if flags already set) OR A + jump
                    // Safe only when the next instruction consumes only Z.
                    if (mnemonic == "CP_IMM" && operand.Mode == AddressMode.Immediate && !operand.Base.HasValue && operand.Offset == 0)
                    {
                        if (nextMnemonic == "JP_Z" || nextMnemonic == "JP_NZ" || nextMnemonic == "JR_Z" || nextMnemonic == "JR_NZ")
                        {
                            bool prevSetsZOnA = false;
                            if (i > 0)
                            {
                                string prevMnemonic;
                                AsmOperand prevOperand;
                                if (sourceLines[i - 1].Match(Tag.Asm, out prevMnemonic, out prevOperand))
                                {
                                    prevSetsZOnA = SetsZFromA(prevMnemonic);
                                }
                            }

                            if (!prevSetsZOnA)
                                optimized.Add(Expr.MakeAsm("OR_A"));

                            optimized.Add(next);
                            i++; continue;
                        }
                    }

                    // Remove immediately balanced same-register stack pairs; intermediate stack-memory effects are not modeled.
                    if ((mnemonic == "PUSH_HL" && nextMnemonic == "POP_HL") ||
                        (mnemonic == "PUSH_BC" && nextMnemonic == "POP_BC") ||
                        (mnemonic == "PUSH_DE" && nextMnemonic == "POP_DE") ||
                        (mnemonic == "PUSH_AF" && nextMnemonic == "POP_AF"))
                    {
                        i++; continue;
                    }

                    // Replace adjacent cross-pair stack copies with byte register moves.
                    if (mnemonic == "PUSH_HL" && nextMnemonic == "POP_DE") { optimized.Add(Expr.MakeAsm("LD_D_H")); optimized.Add(Expr.MakeAsm("LD_E_L")); i++; continue; }
                    if (mnemonic == "PUSH_DE" && nextMnemonic == "POP_HL") { optimized.Add(Expr.MakeAsm("LD_H_D")); optimized.Add(Expr.MakeAsm("LD_L_E")); i++; continue; }
                    if (mnemonic == "PUSH_BC" && nextMnemonic == "POP_DE") { optimized.Add(Expr.MakeAsm("LD_D_B")); optimized.Add(Expr.MakeAsm("LD_E_C")); i++; continue; }
                    if (mnemonic == "PUSH_DE" && nextMnemonic == "POP_BC") { optimized.Add(Expr.MakeAsm("LD_B_D")); optimized.Add(Expr.MakeAsm("LD_C_E")); i++; continue; }
                    if (mnemonic == "PUSH_BC" && nextMnemonic == "POP_HL") { optimized.Add(Expr.MakeAsm("LD_H_B")); optimized.Add(Expr.MakeAsm("LD_L_C")); i++; continue; }
                    if (mnemonic == "PUSH_HL" && nextMnemonic == "POP_BC") { optimized.Add(Expr.MakeAsm("LD_B_H")); optimized.Add(Expr.MakeAsm("LD_C_L")); i++; continue; }

                    // Route an immediate directly into the following destination register; no later A-use analysis is performed here.
                    if (mnemonic == "LD_A_IMM" && operand.Mode == AddressMode.Immediate)
                    {
                        string repl =
                            nextMnemonic == "LD_B_A" ? "LD_B_IMM" :
                            nextMnemonic == "LD_C_A" ? "LD_C_IMM" :
                            nextMnemonic == "LD_D_A" ? "LD_D_IMM" :
                            nextMnemonic == "LD_E_A" ? "LD_E_IMM" :
                            nextMnemonic == "LD_H_A" ? "LD_H_IMM" :
                            nextMnemonic == "LD_L_A" ? "LD_L_IMM" : null;
                        if (repl != null)
                        {
                            optimized.Add(Expr.MakeAsm(repl, operand));
                            i++; continue;
                        }
                    }
                    // Replace small numeric DE offsets with repeated HL increments/decrements.
                    // The rule does not inspect a symbolic base or prove later DE/flag values dead.
                    if (mnemonic == "LD_DE_IMM" && nextMnemonic == "ADD_HL_DE")
                    {
                        if (operand.Mode == AddressMode.Immediate16)
                        {
                            int s16 = (short)(operand.Offset & 0xFFFF);
                            if (s16 >= 1 && s16 <= 4)
                            {
                                for (int k = 0; k < s16; k++) optimized.Add(Expr.MakeAsm("INC_HL"));
                                i++; continue;
                            }
                            if (s16 <= -1 && s16 >= -4)
                            {
                                for (int k = 0; k < -s16; k++) optimized.Add(Expr.MakeAsm("DEC_HL"));
                                i++; continue;
                            }
                        }
                    }
                    // Fuse adjacent HL memory accesses and pointer increments/decrements into auto-update forms.
                    if (mnemonic == "LD_HL_A" && nextMnemonic == "INC_HL") { optimized.Add(Expr.MakeAsm("LDI_HL_A")); i++; continue; }
                    if (mnemonic == "LD_A_HL" && nextMnemonic == "INC_HL") { optimized.Add(Expr.MakeAsm("LDI_A_HL")); i++; continue; }

                    if (mnemonic == "LD_HL_A" && nextMnemonic == "DEC_HL") { optimized.Add(Expr.MakeAsm("LDD_HL_A")); i++; continue; }
                    if (mnemonic == "LD_A_HL" && nextMnemonic == "DEC_HL") { optimized.Add(Expr.MakeAsm("LDD_A_HL")); i++; continue; }

                    // Fold a following increment/decrement into the immediate displacement while retaining its symbolic base.
                    if (mnemonic == "LD_HL_IMM" && operand.Mode == AddressMode.Immediate16 &&
                        (nextMnemonic == "INC_HL" || nextMnemonic == "DEC_HL"))
                    {
                        int delta = (nextMnemonic == "INC_HL") ? 1 : -1;
                        AsmOperand folded = new AsmOperand(operand.Base, operand.Offset + delta, AddressMode.Immediate16, ImmediateModifier.None);
                        optimized.Add(Expr.MakeAsm("LD_HL_IMM", folded));
                        i++; continue;
                    }


                    // JR Label -> Label:
                    // This legacy label check is nested inside a successful next-Asm match, so a normal Label node cannot reach it.
                    if ((mnemonic == "JR" || mnemonic == "JP") && next.MatchTag(Tag.Label))
                    {
                        string labelName = (string)next.GetArgs()[1];
                        if (operand.Base.HasValue && operand.Base.Value == labelName)
                        {
                            continue;
                        }
                    }
                }
            }


            if (mnemonic == "LD_A_A" || mnemonic == "LD_B_B" || mnemonic == "LD_C_C" ||
                mnemonic == "LD_D_D" || mnemonic == "LD_E_E" || mnemonic == "LD_H_H" || mnemonic == "LD_L_L")
            {
                continue;
            }

            optimized.Add(current);
        }

        return optimized;
    }

    // Collect label-to-vector directives with last definition winning; vector validity is checked elsewhere.
    static Dictionary<string, int> BuildRstTargetMap(IEnumerable<Expr> lines)
    {
        var map = new Dictionary<string, int>(StringComparer.Ordinal);
        foreach (var e in lines)
        {
            int vec;
            string target;
            if (e.Match(Tag.RstMap, out vec, out target))
            {
                // Latest directive wins.
                map[target] = vec;
            }
        }
        return map;
    }

    // Mini CSE within a basic block: eliminate redundant fixed-address reloads into A.
    // - Removes consecutive (or repeated within a block) LD A,[abs] / LDH A,[zp] reloads when A is known unchanged.
    // - Rewrites reloads to LD A,r when another register is known to still hold the same value.
    // Name-based invalidation covers selected control flow, stores and register writes; unlisted effects are not inferred.
    static List<Expr> ApplyMiniCsePass(List<Expr> lines)
    {
        var outLines = new List<Expr>(lines.Count);

        // reg -> memKey (only for values loaded from fixed addresses)
        var regMem = new Dictionary<string, string>(StringComparer.Ordinal);

        // Forget all cached register-to-memory associations at an invalidation point.
        Action reset = () => regMem.Clear();

        // Return a tracked memory key or null when the register's value is unknown.
        Func<string, string> get = (r) =>
        {
            string v;
            return regMem.TryGetValue(r, out v) ? v : null;
        };

        // Replace a register association, removing it when the value becomes unknown.
        Action<string, string> set = (r, v) =>
        {
            if (v == null) regMem.Remove(r);
            else regMem[r] = v;
        };

        // Find an alternate register holding the same key, preferring B/C before D/E and H/L.
        Func<string, string> findRegHolding = (memKey) =>
        {
            // Prefer common scratch registers first.
            string[] order = new[] { "B", "C", "D", "E", "H", "L" };
            for (int i = 0; i < order.Length; i++)
            {
                string r = order[i];
                string v;
                if (regMem.TryGetValue(r, out v) && v == memKey) return r;
            }
            return null;
        };

        for (int i = 0; i < lines.Count; i++)
        {
            var e = lines[i];

            string m;
            AsmOperand o;
            if (!e.Match(Tag.Asm, out m, out o))
            {
                outLines.Add(e);
                reset();
                continue;
            }

            // Basic-block boundaries / unknown side effects
            if (IsControlFlowOrCall(m) || IsMemoryWrite(m))
            {
                outLines.Add(e);
                reset();
                continue;
            }

            // Register moves we can model
            if (TryApplyRegisterMove(m, get, set))
            {
                outLines.Add(e);
                continue;
            }

            // Fixed-address load into A
            string memKey;
            if (TryGetFixedLoadKey(m, o, out memKey))
            {
                // If A already holds it, drop the reload.
                if (get("A") == memKey)
                {
                    continue;
                }

                // If another register still holds it, reload from that register instead.
                var reuse = findRegHolding(memKey);
                if (reuse != null)
                {
                    outLines.Add(Expr.MakeAsm($"LD_A_{reuse}"));
                    set("A", memKey);
                    continue;
                }

                // Keep original
                outLines.Add(e);
                set("A", memKey);
                continue;
            }

            // Model common writes/invalidation conservatively
            ApplyConservativeRegInvalidation(m, set, reset);

            outLines.Add(e);
        }

        return outLines;
    }

    // Recognize numeric absolute/high-memory loads into A and exclude FF00-FF7F.
    // Keys distinguish absolute and high-memory forms; other asynchronously changed locations are not excluded by this range check.
    static bool TryGetFixedLoadKey(string mnemonic, AsmOperand operand, out string memKey)
    {
        memKey = null;

        // NOTE: 0xFF00-0xFF7F is I/O registers which can change asynchronously.
        // We must NOT CSE those loads (e.g., LY/STAT/DIV/joypad) even within a basic block.
        if (mnemonic == "LD_A_MEM" && operand.Mode == AddressMode.Absolute && !operand.Base.HasValue)
        {
            int addr = operand.Offset & 0xFFFF;
            if (addr >= 0xFF00 && addr < 0xFF80) return false; // volatile I/O
            memKey = "ABS:" + addr.ToString();
            return true;
        }
        if (mnemonic == "LDH_A_MEM" && operand.Mode == AddressMode.HighMem && !operand.Base.HasValue)
        {
            int zp = operand.Offset & 0xFF;
            if (zp < 0x80) return false; // 0xFF00-0xFF7F volatile I/O
            memKey = "ZP:" + zp.ToString();
            return true;
        }
        return false;
    }

    // Propagate memory associations through the explicitly listed byte moves; return false for every other form.
    static bool TryApplyRegisterMove(string mnemonic, Func<string, string> get, Action<string, string> set)
    {
        // LD X,A
        if (mnemonic == "LD_B_A") { set("B", get("A")); return true; }
        if (mnemonic == "LD_C_A") { set("C", get("A")); return true; }
        if (mnemonic == "LD_D_A") { set("D", get("A")); return true; }
        if (mnemonic == "LD_E_A") { set("E", get("A")); return true; }
        if (mnemonic == "LD_H_A") { set("H", get("A")); return true; }
        if (mnemonic == "LD_L_A") { set("L", get("A")); return true; }

        // LD A,X
        if (mnemonic == "LD_A_B") { set("A", get("B")); return true; }
        if (mnemonic == "LD_A_C") { set("A", get("C")); return true; }
        if (mnemonic == "LD_A_D") { set("A", get("D")); return true; }
        if (mnemonic == "LD_A_E") { set("A", get("E")); return true; }
        if (mnemonic == "LD_A_H") { set("A", get("H")); return true; }
        if (mnemonic == "LD_A_L") { set("A", get("L")); return true; }

        // For peephole result: PUSH HL; POP DE -> LD D,H; LD E,L
        if (mnemonic == "LD_D_H") { set("D", get("H")); return true; }
        if (mnemonic == "LD_E_L") { set("E", get("L")); return true; }

        return false;
    }

    // Recognize the listed branch, return, wait and restart names plus unconditional CALL.
    // Conditional CALL names are not matched by the exact CALL check.
    // Folding leaves A untouched, and immediate ADD/SUB also differ in carry.
    // Require both differing outputs to be overwritten before any use. Unknown
    // instructions or control-flow boundaries conservatively keep the original code.
    static bool CanFoldMemoryUpdate(IReadOnlyList<Expr> lines, int start, bool carryDiffers)
    {
        bool needA = true;
        bool needCarry = carryDiffers;
        for (int i = start; i < lines.Count && i < start + 16; i++)
        {
            string m; AsmOperand operand;
            if (!lines[i].Match(Tag.Asm, out m, out operand)) return false;
            if (m == "XOR_A" || m == "POP_AF") { needA = false; needCarry = false; }
            else if (m == "LD_HL_SP_IMM") needCarry = false;
            else if ((m.StartsWith("LD_A_") && m != "LD_A_A") || m.StartsWith("LDH_A_")) needA = false;
            else if (m.StartsWith("LD_") || m.StartsWith("LDH_") || m == "LDI_HL_A" || m == "LDD_HL_A")
            {
                if (m.EndsWith("_A") && needA) return false;
            }
            else if (m == "NOP" || m == "DI" || m == "EI" ||
                     m == "INC_HL" || m == "DEC_HL" || m == "INC_BC" || m == "DEC_BC" ||
                     m == "INC_DE" || m == "DEC_DE" || m == "INC_SP" || m == "DEC_SP") { }
            else if (m.StartsWith("AND") || m.StartsWith("OR") || m.StartsWith("XOR") ||
                     m.StartsWith("ADD_A") || m.StartsWith("SUB"))
            {
                if (needA) return false;
                needCarry = false;
            }
            else return false;
            if (!needA && !needCarry) return true;
        }
        return false;
    }

    static bool IsControlFlowOrCall(string mnemonic)
    {
        return mnemonic.StartsWith("JP") || mnemonic.StartsWith("JR") ||
               mnemonic.StartsWith("RET") || mnemonic == "RETI" ||
               mnemonic == "HALT" || mnemonic == "STOP" ||
               mnemonic == "RST_00" || mnemonic.StartsWith("RST_") ||
               mnemonic == "CALL";
    }

    // Invalidate for the listed store prefixes and HL_REF spellings, including some reads.
    // This name-based filter does not describe every possible memory-writing instruction.
    static bool IsMemoryWrite(string mnemonic)
    {
        // Conservative: any explicit memory-store or (HL) write is treated as clobbering.
        if (mnemonic.StartsWith("LD_MEM_") || mnemonic.StartsWith("LDH_MEM_")) return true;
        if (mnemonic == "LD_HL_A" || mnemonic == "LDI_HL_A" || mnemonic == "LDD_HL_A") return true;
        if (mnemonic.EndsWith("HL_REF") || mnemonic.Contains("HL_REF_")) return true;
        return false;
    }

    // Clear associations for recognized register writes and stack operations.
    // An unrecognized mnemonic falls through without clearing state; this is not an exhaustive instruction effect table.
    static void ApplyConservativeRegInvalidation(string mnemonic, Action<string, string> set, Action reset)
    {
        // Instructions that can change A in unknown ways -> drop A knowledge.
        if (mnemonic == "LD_A_IMM" || mnemonic.StartsWith("LD_A_") || mnemonic.StartsWith("LDH_A_") || mnemonic == "XOR_A")
        {
            // Fixed-address loads and register moves are handled earlier.
            set("A", null);
            return;
        }

        // Arithmetic/logic on A
        if (mnemonic.StartsWith("ADD_A") || mnemonic.StartsWith("ADC_A") || mnemonic.StartsWith("SUB") ||
            mnemonic.StartsWith("SBC") || mnemonic.StartsWith("AND") || mnemonic.StartsWith("OR") ||
            mnemonic.StartsWith("XOR") || mnemonic == "INC_A" || mnemonic == "DEC_A")
        {
            // OR A preserves the value, but it's fine to keep knowledge; the other OR forms change A.
            if (mnemonic != "OR_A") set("A", null);
            return;
        }

        // Writes to other registers we track
        if (mnemonic.StartsWith("LD_B_") || mnemonic == "INC_B" || mnemonic == "DEC_B") set("B", null);
        if (mnemonic.StartsWith("LD_C_") || mnemonic == "INC_C" || mnemonic == "DEC_C") set("C", null);
        if (mnemonic.StartsWith("LD_D_") || mnemonic == "INC_D" || mnemonic == "DEC_D") set("D", null);
        if (mnemonic.StartsWith("LD_E_") || mnemonic == "INC_E" || mnemonic == "DEC_E") set("E", null);
        if (mnemonic.StartsWith("LD_H_") || mnemonic == "INC_H" || mnemonic == "DEC_H") set("H", null);
        if (mnemonic.StartsWith("LD_L_") || mnemonic == "INC_L" || mnemonic == "DEC_L") set("L", null);

        // 16-bit ops affecting HL
        if (mnemonic == "INC_HL" || mnemonic == "DEC_HL" || mnemonic.StartsWith("ADD_HL") || mnemonic.StartsWith("LD_HL"))
        {
            set("H", null);
            set("L", null);
        }

        // Stack operations -> too many side effects
        if (mnemonic.StartsWith("POP_") || mnemonic.StartsWith("PUSH_"))
        {
            reset();
        }
    }

    // Identify unconditional jumps and returns that end linear fallthrough in this IR.
    static bool IsTerminator(string m)
    {
        return m == "JP" || m == "JR" || m == "JP_HL" || m == "RET" || m == "RETI";
    }

    // Drop assembly nodes after a terminator until a label or any non-assembly node resumes the stream.
    // This local sweep does not trace branch targets or discard unreachable labeled blocks.
    static List<Expr> RemoveUnreachable(List<Expr> lines)
    {
        var outLines = new List<Expr>(lines.Count);
        bool skipping = false;

        foreach (var e in lines)
        {
            if (skipping)
            {
                if (e.MatchTag(Tag.Label))
                {
                    skipping = false;
                    outLines.Add(e);
                    continue;
                }

                if (!e.MatchTag(Tag.Asm))
                {
                    skipping = false;
                    outLines.Add(e);
                    continue;
                }

                continue;
            }

            outLines.Add(e);

            string mnem;
            AsmOperand op;
            if (e.Match(Tag.Asm, out mnem, out op))
            {
                if (IsTerminator(mnem)) skipping = true;
            }
        }

        return outLines;
    }


    // Recognize selected mnemonic spellings whose zero flag describes A for the adjacent compare-zero rewrite.
    // This check does not track subsequent uses of carry or other flags.
    static bool SetsZFromA(string mnemonic)
    {
        if (mnemonic == null) return false;

        // ALU ops that set Z based on the A result (A updated), so a following CP 0 is redundant
        if (mnemonic.StartsWith("OR_") || mnemonic.StartsWith("AND_") || mnemonic.StartsWith("XOR_") ||
            mnemonic.StartsWith("ADD_A_") || mnemonic.StartsWith("ADC_A_") ||
            mnemonic.StartsWith("SUB_") || mnemonic.StartsWith("SBC_A_"))
            return true;

        if (mnemonic == "INC_A" || mnemonic == "DEC_A")
            return true;

        return false;
    }


}




