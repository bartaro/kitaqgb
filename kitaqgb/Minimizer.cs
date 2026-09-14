using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;

// --minimize: Auto-generate a minimal reproducer for a compile error / crash.
// Implementation: classic ddmin over line deletions.
// The minimizer runs as a driver that repeatedly spawns this compiler executable
// with a candidate input file. This avoids invasive refactors to capture internal state.
// Usage:
// kitaqgb foo.c -o out.gb --minimize
// kitaqgb foo.c bar.c -o out.gb --minimize=foo.c
// kitaqgb foo.c -o out.gb --minimize --minimize-out=foo.min.c
// Notes:
// - The predicate is "still fails" and (best-effort) matches the original failure fingerprint.
// - Only the chosen target file is minimized; other source files remain unchanged.
internal static class Minimizer
{
    // Select the minimizer driver only for --minimize or --minimize=target, not its subordinate options.
    public static bool IsMinimizeRequested(string[] args)
    {
        foreach (var a in args)
        {
            if (a == "--minimize" || a.StartsWith("--minimize=", StringComparison.Ordinal)) return true;
        }
        return false;
    }

    // Reproduce the baseline failure in a child compiler, then shrink one selected source by deleting line groups.
    // Write the smallest retained candidate and optionally collect a final trace bundle.
    public static void Run(string[] originalArgs)
    {
        try
        {
            var opts = ParseOptions(originalArgs);

            if (opts.SourceFiles.Count == 0)
            {
                Console.Error.WriteLine("error: --minimize requires at least one source file");
                Program.Exit(1);
                return;
            }

            string target = opts.TargetFile ?? opts.SourceFiles[0];
            if (!opts.SourceFiles.Any(s => Path.GetFullPath(s).Equals(Path.GetFullPath(target), StringComparison.OrdinalIgnoreCase)))
            {
                Console.Error.WriteLine("error: --minimize target file not found in inputs: " + target);
                Program.Exit(1);
                return;
            }

            string originalText = IoUtil.ReadAllTextUtf8(target);
            var originalLines = SplitLinesPreserveNewlines(originalText);

            string outPath = opts.MinimizeOutPath;
            if (string.IsNullOrEmpty(outPath))
            {
                outPath = Path.Combine(Path.GetDirectoryName(target) ?? "", Path.GetFileNameWithoutExtension(target) + ".min" + Path.GetExtension(target));
            }

            string workDir = opts.WorkDir;
            Directory.CreateDirectory(workDir);

            string exePath = GetExePath();
            if (string.IsNullOrEmpty(exePath) || !File.Exists(exePath))
            {
                Console.Error.WriteLine("error: cannot resolve compiler exe path for --minimize");
                Program.Exit(1);
                return;
            }

            // Confirm the original reproduces and extract fingerprint.
            var baseRun = RunChild(exePath, BuildChildArgs(opts, target, target, workDir), workDir);
            if (baseRun.ExitCode == 0)
            {
                Console.WriteLine("[minimize] input does not fail; nothing to minimize.");
                Program.Exit(0);
                return;
            }

            string fingerprint = ExtractFingerprint(baseRun.Stdout + "\n" + baseRun.Stderr);
            Console.WriteLine("[minimize] fingerprint: " + (string.IsNullOrEmpty(fingerprint) ? "<none>" : fingerprint));

            // ddmin over line deletions.
            List<string> current = new List<string>(originalLines);
            int n = 2;

            int bestLen = current.Count;
            int iter = 0;
            // Try complements of progressively smaller line chunks; never test an empty candidate.
            // A failed reduction increases the partition count, so the result is deletion-minimal for this predicate, not globally shortest.
            while (current.Count >= 2)
            {
                iter++;
                bool reduced = false;
                int chunkSize = (int)Math.Ceiling(current.Count / (double)n);

                for (int i = 0; i < n; i++)
                {
                    int start = i * chunkSize;
                    if (start >= current.Count) break;
                    int count = Math.Min(chunkSize, current.Count - start);

                    var candidate = new List<string>(current.Count - count);
                    if (start > 0) candidate.AddRange(current.GetRange(0, start));
                    if (start + count < current.Count) candidate.AddRange(current.GetRange(start + count, current.Count - (start + count)));

                    if (candidate.Count == 0) continue;

                    if (TestCandidate(exePath, opts, target, candidate, workDir, fingerprint))
                    {
                        current = candidate;
                        reduced = true;
                        n = Math.Max(n - 1, 2);
                        if (current.Count < bestLen)
                        {
                            bestLen = current.Count;
                            Console.WriteLine($"[minimize] reduced: lines={bestLen} (iter {iter})");
                        }
                        break;
                    }
                }

                if (!reduced)
                {
                    if (n >= current.Count) break;
                    n = Math.Min(n * 2, current.Count);
                }
            }

            string minimizedText = string.Concat(current);
            IoUtil.WriteAllTextUtf8Robust(outPath, minimizedText, allowAlternatePath: true);

            // Write a small report.
            string reportPath = Path.Combine(workDir, "minimize_report.txt");
            var sb = new StringBuilder();
            sb.AppendLine("--minimize report");
            sb.AppendLine("target: " + target);
            sb.AppendLine("output: " + outPath);
            sb.AppendLine("orig_lines: " + originalLines.Count);
            sb.AppendLine("min_lines: " + current.Count);
            sb.AppendLine("fingerprint: " + fingerprint);
            IoUtil.WriteAllTextUtf8Robust(reportPath, sb.ToString(), allowAlternatePath: true);

            Console.WriteLine("[minimize] wrote: " + outPath);
Console.WriteLine("[minimize] report: " + reportPath);

// One-command bundle: if user also enabled --trace, generate a trace set for the minimized reproducer.
if (opts.TraceRequested)
{
    try
    {
        string finalDir = Path.Combine(workDir, "final");
        Directory.CreateDirectory(finalDir);

        string traceDir = Path.Combine(finalDir, "trace");
        string debugDir = Path.Combine(finalDir, "debug");

        // Run the compiler once more against the minimized output, with trace outputs isolated.
        var finalArgs = BuildChildArgsForFinal(opts, target, outPath, finalDir, traceDir, debugDir);
        var finalRun = RunChild(exePath, finalArgs, finalDir);

        IoUtil.WriteAllTextUtf8Robust(Path.Combine(finalDir, "run_stdout.txt"), finalRun.Stdout ?? "", allowAlternatePath: true);
        IoUtil.WriteAllTextUtf8Robust(Path.Combine(finalDir, "run_stderr.txt"), finalRun.Stderr ?? "", allowAlternatePath: true);
        IoUtil.WriteAllTextUtf8Robust(Path.Combine(finalDir, "run_exitcode.txt"), finalRun.ExitCode.ToString(), allowAlternatePath: true);

        // Also keep a copy of the minimized file inside the bundle for easy sharing.
        string bundledSrc = Path.Combine(finalDir, Path.GetFileName(outPath));
        try { IoUtil.CopyFileRobust(outPath, bundledSrc, true); } catch { /* ignore */ }

        // If the user provided --trace-out=..., copy the final trace set there as well (best effort).
        if (!string.IsNullOrEmpty(opts.UserTraceOut))
        {
            try
            {
                Directory.CreateDirectory(opts.UserTraceOut);
                CopyDirectory(traceDir, opts.UserTraceOut);
            }
            catch { /* ignore */ }
        }

        Console.WriteLine("[minimize] trace bundle: " + finalDir);
    }
    catch (Exception ex2)
    {
        Console.Error.WriteLine("[minimize] warning: trace bundling failed: " + ex2.Message);
    }
}

Program.Exit(0);
        }
        catch (ControlledCompilerExit)
        {
            throw;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("error: --minimize failed: " + ex.Message);
            Console.Error.WriteLine(ex.StackTrace);
            Program.Exit(1);
        }
    }

    // Keep source selection, forwarded compiler arguments and trace policy separate from candidate output paths.
    private sealed class Options
    {
        public List<string> SourceFiles = new List<string>();
        public string TargetFile;
        public string MinimizeOutPath;
        public string WorkDir = "minimize_output";
        public string OutputFilename = null; // from -o
        public List<string> BaseArgs = new List<string>();
        public bool Quick = false;
        public bool TraceRequested = false;
        public string TraceArg = null; // original --trace or --trace=...
        public string UserTraceOut = null;

        // Trace policy during minimization:
        // - final (default): do not trace intermediate candidates; only trace the final minimized case.
        // - all: trace and save reproducing intermediate candidates under workDir/candidates/.
        public string MinimizeTraceMode = "final";
        public int CandidateSerial = 0;
    }

    // Remove driver-only flags and capture output/trace destinations before forwarding ordinary compiler arguments.
    // Treat every remaining non-option argument as an input filename.
    private static Options ParseOptions(string[] args)
    {
        var opt = new Options();
        // BaseArgs: everything except --minimize* and (we will override -o).
        for (int i = 0; i < args.Length; i++)
        {
            string a = args[i];
            if (a == "-o")
            {
                // capture but do not forward
                if (i + 1 < args.Length) { opt.OutputFilename = args[i + 1]; i++; }
                continue;
            }
            // Match only the main option so --minimize-out, --minimize-work and --minimize-trace reach their own handlers.
            if (a == "--minimize" || a.StartsWith("--minimize=", StringComparison.Ordinal))
            {
                if (a.StartsWith("--minimize=", StringComparison.Ordinal)) opt.TargetFile = a.Substring(a.IndexOf('=') + 1);
                continue;
            }
            if (a.StartsWith("--minimize-out=", StringComparison.Ordinal))
            {
                opt.MinimizeOutPath = a.Substring(a.IndexOf('=') + 1);
                continue;
            }
            if (a.StartsWith("--minimize-work=", StringComparison.Ordinal))
            {
                opt.WorkDir = a.Substring(a.IndexOf('=') + 1);
                continue;
            }

            if (a.StartsWith("--minimize-trace=", StringComparison.Ordinal))
            {
                opt.MinimizeTraceMode = (a.Substring(a.IndexOf('=') + 1) ?? "").Trim().ToLowerInvariant();
                if (opt.MinimizeTraceMode != "final" && opt.MinimizeTraceMode != "all")
                {
                    Console.Error.WriteLine("error: --minimize-trace must be 'final' or 'all'");
                    Program.Exit(1);
                }
                continue;
            }

            if (a.StartsWith("--trace-out=", StringComparison.Ordinal)) { opt.UserTraceOut = a.Substring(a.IndexOf("=") + 1); continue; }
            if (a.StartsWith("--debug-out=", StringComparison.Ordinal)) { continue; }
            if (a == "--trace" || a.StartsWith("--trace=", StringComparison.Ordinal))
            {
                opt.TraceRequested = true;
                opt.TraceArg = a;
                // Important: do NOT forward --trace into BaseArgs by default.
                // Otherwise, every minimization trial run becomes extremely slow and noisy.
                // We'll only add trace to child args when MinimizeTraceMode == "all" (intermediates)
                // or when running the final bundle step.
                continue;
            }

            opt.BaseArgs.Add(a);
}

        // Gather source files from BaseArgs (non-"-" args that are existing .c/.h?)
        // Keep it simple: anything not starting with '-' is a source filename.
        foreach (var a in opt.BaseArgs)
        {
            if (!a.StartsWith("-")) opt.SourceFiles.Add(a);
        }
        return opt;
    }

    // Run a nonempty candidate and require failure plus a case-insensitive baseline fingerprint match when available.
    // The all-traces policy retains only reproducing candidate directories.
    private static bool TestCandidate(string exePath, Options opts, string originalTargetPath, List<string> candidateLines, string workDir, string fingerprint)
    {
        // Default (fast) mode: re-use a single candidate file and do not generate trace for intermediates.
        // TraceMode=all: isolate each reproducing candidate under workDir/candidates/ with its trace/debug.
        string runDir = workDir;
        if (opts.TraceRequested && opts.MinimizeTraceMode == "all")
        {
            int serial = ++opts.CandidateSerial;
            runDir = Path.Combine(workDir, "candidates", $"cand_{serial:D5}");
            Directory.CreateDirectory(runDir);
        }

        string tmpFile = Path.Combine(runDir, "candidate.c");
        IoUtil.WriteAllTextUtf8Robust(tmpFile, string.Concat(candidateLines), allowAlternatePath: true);

        var childArgs = BuildChildArgs(opts, originalTargetPath, tmpFile, runDir);
        var r = RunChild(exePath, childArgs, runDir);
        if (r.ExitCode == 0) return false;

        bool ok;
        if (opts.Quick) ok = true;
        else if (string.IsNullOrEmpty(fingerprint)) ok = true;
        else
        {
            string combined = (r.Stdout + "\n" + r.Stderr);
            ok = combined.IndexOf(fingerprint, StringComparison.OrdinalIgnoreCase) >= 0;
        }

        // In TraceMode=all, keep only reproducing candidates (ok==true). Non-reproducing runs are removed.
        if (opts.TraceRequested && opts.MinimizeTraceMode == "all")
        {
            if (ok)
            {
                try
                {
                    IoUtil.WriteAllTextUtf8Robust(Path.Combine(runDir, "run_stdout.txt"), r.Stdout ?? "", allowAlternatePath: true);
                    IoUtil.WriteAllTextUtf8Robust(Path.Combine(runDir, "run_stderr.txt"), r.Stderr ?? "", allowAlternatePath: true);
                    IoUtil.WriteAllTextUtf8Robust(Path.Combine(runDir, "run_exitcode.txt"), r.ExitCode.ToString(), allowAlternatePath: true);
                }
                catch { /* ignore */ }
            }
            else
            {
                try { Directory.Delete(runDir, true); } catch { /* ignore */ }
            }
        }

        return ok;
    }

    // Replace the selected source argument and isolate ROM/debug outputs; enable intermediate trace only for the all policy.
    private static string[] BuildChildArgs(Options opts, string originalTarget, string replacementTarget, string runDir)
    {
        // Replace the target file in the argument list.
        var list = new List<string>();
        foreach (var a in opts.BaseArgs)
        {
            if (!a.StartsWith("-") && Path.GetFullPath(a).Equals(Path.GetFullPath(originalTarget), StringComparison.OrdinalIgnoreCase))
                list.Add(replacementTarget);
            else
                list.Add(a);
        }

        // Override output filename to avoid clobbering user outputs.
        string outGb = Path.Combine(runDir, "candidate.gb");
        list.Add("-o");
        list.Add(outGb);

        // Also redirect trace/debug outputs to avoid messing with user directories.
        // If the user enabled trace/debug, keep it but isolate it.
        // Only enable --trace for intermediate candidates if MinimizeTraceMode == "all".
        if (opts.TraceRequested && opts.MinimizeTraceMode == "all")
        {
            list.Add(string.IsNullOrEmpty(opts.TraceArg) ? "--trace" : opts.TraceArg);
        }
        list.Add("--trace-out=" + Path.Combine(runDir, "trace"));
        list.Add("--debug-out=" + Path.Combine(runDir, "debug"));

        return list.ToArray();
    }

    // Capture the child process exit status and both diagnostic streams for the failure predicate and reports.
    private sealed class RunResult
    {
        public int ExitCode;
        public string Stdout;
        public string Stderr;
    }

    // Launch without a window from the caller's current directory; output paths are already encoded in the arguments.
    // Read stdout and then stderr synchronously and wait for process exit; this helper has no timeout.
    private static RunResult RunChild(string exePath, string[] args, string workDir)
    {
        var psi = new ProcessStartInfo(exePath)
        {
            WorkingDirectory = Environment.CurrentDirectory,
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true,
        };
        // .NET Framework (as used by this project) doesn't support ProcessStartInfo.ArgumentList.
        // Build a single argument string with conservative quoting.
        psi.Arguments = string.Join(" ", args.Select(QuoteArg));

        using (var p = Process.Start(psi))
        {
            string stdout = p.StandardOutput.ReadToEnd();
            string stderr = p.StandardError.ReadToEnd();
            p.WaitForExit();
            return new RunResult { ExitCode = p.ExitCode, Stdout = stdout, Stderr = stderr };
        }
    }

    // Build a minimally quoted Windows argument; this helper escapes quotes but does not implement general backslash doubling.
    private static string QuoteArg(string a)
    {
        if (a == null) return "";
        if (a.Length == 0) return "\"\"";
        // Quote if contains whitespace or quotes.
        if (a.IndexOfAny(new[] { ' ', '\t', '\n', '"' }) < 0) return a;
        // Minimal Windows quoting: wrap in quotes and escape internal quotes.
        return "\"" + a.Replace("\"", "\\\"") + "\"";
    }

    // Prefer the first KQ diagnostic code, then a System exception type, then a truncated non-banner output line.
    // The fingerprint is a best-effort textual identity, not a structured comparison of compiler failures.
    private static string ExtractFingerprint(string output)
    {
        if (string.IsNullOrEmpty(output)) return "";
        var m = Regex.Match(output, @"\bKQ\d{4}\b");
        if (m.Success) return m.Value;

        // Try exception type
        var ex = Regex.Match(output, @"\b(System\.[A-Za-z0-9_.]+Exception)\b");
        if (ex.Success) return ex.Groups[1].Value;

        // First non-empty line
        using (var sr = new StringReader(output))
        {
            string line;
            while ((line = sr.ReadLine()) != null)
            {
                line = line.Trim();
                if (line.Length == 0) continue;
                // Skip banner
                if (line.StartsWith("[kitaqgb]")) continue;
                return line.Length > 120 ? line.Substring(0, 120) : line;
            }
        }
        return "";
    }

    // Split after each LF while retaining the original terminators, including CRLF. Empty input becomes one newline.
    private static List<string> SplitLinesPreserveNewlines(string text)
    {
        // Keep original newlines so we can reconstruct with minimal formatting changes.
        var lines = new List<string>();
        if (string.IsNullOrEmpty(text)) { lines.Add("\n"); return lines; }

        int i = 0;
        while (i < text.Length)
        {
            int start = i;
            while (i < text.Length && text[i] != '\n') i++;
            if (i < text.Length && text[i] == '\n') i++;
            lines.Add(text.Substring(start, i - start));
        }
        return lines;
    }

// Compile the retained source into the final bundle with the originally requested trace selection.
private static string[] BuildChildArgsForFinal(Options opts, string originalTarget, string replacementTarget, string finalDir, string traceDir, string debugDir)
{
    var list = new List<string>();
    foreach (var a in opts.BaseArgs)
    {
        if (!a.StartsWith("-") && Path.GetFullPath(a).Equals(Path.GetFullPath(originalTarget), StringComparison.OrdinalIgnoreCase))
            list.Add(replacementTarget);
        else
            list.Add(a);
    }

    // Override output filename into the bundle folder.
    string outGb = Path.Combine(finalDir, "final.gb");
    list.Add("-o");
    list.Add(outGb);

    // Enable trace for the final run if the user requested it.
    if (opts.TraceRequested)
    {
        list.Add(string.IsNullOrEmpty(opts.TraceArg) ? "--trace" : opts.TraceArg);
    }

    // Ensure trace outputs are written into the bundle folder.
    list.Add("--trace-out=" + traceDir);
    list.Add("--debug-out=" + debugDir);

    return list.ToArray();
}

// Copy all descendant directories and files into the destination, overwriting matching files without removing extras.
private static void CopyDirectory(string srcDir, string dstDir)
{
    if (!Directory.Exists(srcDir)) return;
    foreach (var dir in Directory.GetDirectories(srcDir, "*", SearchOption.AllDirectories))
    {
        var rel = dir.Substring(srcDir.Length).TrimStart(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        Directory.CreateDirectory(Path.Combine(dstDir, rel));
    }
    foreach (var file in Directory.GetFiles(srcDir, "*", SearchOption.AllDirectories))
    {
        var rel = file.Substring(srcDir.Length).TrimStart(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        var dst = Path.Combine(dstDir, rel);
        Directory.CreateDirectory(Path.GetDirectoryName(dst));
        IoUtil.CopyFileRobust(file, dst, true);
    }
}

    // Resolve the current process executable, falling back to argv[0] only if process-module lookup throws.
    private static string GetExePath()
    {
        try
        {
            return Process.GetCurrentProcess().MainModule.FileName;
        }
        catch
        {
            try { return Environment.GetCommandLineArgs().FirstOrDefault(); } catch { return null; }
        }
    }
}




