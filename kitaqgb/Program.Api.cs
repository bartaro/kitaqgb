using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using System.Threading;

// Select invocation behavior for the command-line process or an in-process API caller.
internal enum CompilerHostMode
{
    Cli,
    Api,
}

// Carry an intended compiler exit code across the API boundary without terminating its host process.
internal sealed class ControlledCompilerExit : Exception
{
    public int ExitCode { get; private set; }

    // Retain the requested exit code and a diagnostic exception message.
    public ControlledCompilerExit(int exitCode)
        : base("Compiler exited with code " + exitCode)
    {
        ExitCode = exitCode;
    }
}

static partial class Program
{
    // Associate session state with the current asynchronous execution context.
    // This slot alone does not isolate other static compiler state or process-wide console settings.
    static readonly AsyncLocal<CompilerSession> SessionSlot = new AsyncLocal<CompilerSession>();
    static readonly CompilerSession BootstrapSession = new CompilerSession();

    // Use bootstrap defaults when no invocation session has been installed.
    static CompilerSession CurrentSession => SessionSlot.Value ?? BootstrapSession;
    static CompilerHostMode CurrentHostMode { get { return CurrentSession.HostMode; } set { CurrentSession.HostMode = value; } }
    static Dictionary<string, string> ArtifactPaths => CurrentSession.ArtifactPaths;
    // Expose the current session's phase reports and replace null assignments with empty reports.
    internal static CodegenAnalysisReport CurrentCodegenLastReport { get { return CurrentSession.CodegenLastReport; } set { CurrentSession.CodegenLastReport = value ?? new CodegenAnalysisReport(); } }
    internal static AssemblerAnalysisReport CurrentAssemblerLastReport { get { return CurrentSession.AssemblerLastReport; } set { CurrentSession.AssemblerLastReport = value ?? new AssemblerAnalysisReport(); } }
    internal static OptimizerAnalysisReport CurrentOptimizerLastReport { get { return CurrentSession.OptimizerLastReport; } set { CurrentSession.OptimizerLastReport = value ?? new OptimizerAnalysisReport(); } }

    public static bool MachineReadableOutput { get { return CurrentSession.MachineReadableOutput; } private set { CurrentSession.MachineReadableOutput = value; } }
    public static bool NoBanner { get { return CurrentSession.NoBanner; } private set { CurrentSession.NoBanner = value; } }
    public static bool EmitPathManifest { get { return CurrentSession.EmitPathManifest; } private set { CurrentSession.EmitPathManifest = value; } }
    public static string PathManifestPath { get { return CurrentSession.PathManifestPath; } private set { CurrentSession.PathManifestPath = value; } }

    // Suppress helper output in either machine-readable mode or API hosting mode.
    static bool SuppressInformationalOutput => MachineReadableOutput || CurrentHostMode == CompilerHostMode.Api;

    // Start with session defaults and retain the provided argument array without cloning it.
    static CompilerSession CreateInvocationSession(CompilerHostMode hostMode, string[] argsArray)
    {
        var session = new CompilerSession();
        session.HostMode = hostMode;
        session.OriginalArgs = argsArray ?? new string[0];
        return session;
    }

    // Reset invocation state while preserving its host mode and configure process-wide UTF-8 console output.
    // Only CLI invocations append command history; console encoding is not restored here.
    static void PrepareForInvocation(string[] argsArray)
    {
        CompilerHostMode hostMode = CurrentHostMode;
        SessionSlot.Value = CreateInvocationSession(hostMode, argsArray);
        Console.OutputEncoding = IoUtil.Utf8NoBom;
        if (hostMode == CompilerHostMode.Cli)
            TryAppendCommandHistory(argsArray ?? Array.Empty<string>());
    }

    // Record a nonblank role/path in the current session without requiring the artifact to exist yet.
    internal static void RememberArtifactPath(string key, string path)
    {
        if (string.IsNullOrWhiteSpace(key) || string.IsNullOrWhiteSpace(path)) return;
        ArtifactPaths[key] = NormalizeArtifactPath(path);
    }

    // Write nonblank informational text only when the current host permits helper output.
    internal static void WriteInfoLine(string text)
    {
        if (SuppressInformationalOutput || string.IsNullOrWhiteSpace(text)) return;
        Console.WriteLine(text);
    }

    // Apply the same host suppression to this stderr helper; structured diagnostics are collected elsewhere.
    internal static void WriteErrorLine(string text)
    {
        if (SuppressInformationalOutput || string.IsNullOrWhiteSpace(text)) return;
        Console.Error.WriteLine(text);
    }

    // Run the shared entry point with an API session, translating controlled exits into results.
    // Always restore the previous session; unrelated exceptions propagate to the caller.
    internal static kitaqgb.CompileResult InvokeCompileForApi(kitaqgb.CompileRequest request, string[] args)
    {
        if (request == null) throw new ArgumentNullException(nameof(request));

        var previousSession = SessionSlot.Value;
        SessionSlot.Value = CreateInvocationSession(CompilerHostMode.Api, args ?? new string[0]);
        try
        {
            Main(args ?? new string[0]);
            return BuildCompileResult(request, exitCode: 0);
        }
        catch (ControlledCompilerExit ex)
        {
            return BuildCompileResult(request, ex.ExitCode);
        }
        finally
        {
            SessionSlot.Value = previousSession;
        }
    }

    // Copy session diagnostics and artifact metadata into the API result. Success requires a zero exit code,
    // an existing output path and no error diagnostics; file existence alone does not establish freshness.
    static kitaqgb.CompileResult BuildCompileResult(kitaqgb.CompileRequest request, int exitCode)
    {
        CompilerSession session = CurrentSession;
        var artifacts = SnapshotArtifactPaths(session, request == null ? null : request.OutputFilename);
        string outputFilename;
        string diagJsonPath;
        string dbg2JsonPath;
        string buildReportJsonPath;
        artifacts.TryGetValue("output_rom", out outputFilename);
        artifacts.TryGetValue("diag_json", out diagJsonPath);
        artifacts.TryGetValue("dbg2_json", out dbg2JsonPath);
        artifacts.TryGetValue("build_report_json", out buildReportJsonPath);

        if (string.IsNullOrWhiteSpace(outputFilename) && request != null && !string.IsNullOrWhiteSpace(request.OutputFilename))
        {
            outputFilename = NormalizeArtifactPath(request.OutputFilename);
        }

        var diagnostics = session.Diagnostics
            .Select(d => new kitaqgb.DiagnosticDto
            {
                Severity = d.Severity,
                Code = d.Code,
                Message = d.Message,
                Suggestion = d.Suggestion,
                File = d.Filename,
                Line = d.Line,
                Column = d.Column,
            })
            .ToList();

        bool hasErrorDiagnostics = diagnostics.Any(d =>
            string.Equals(d.Severity, "error", StringComparison.OrdinalIgnoreCase) ||
            string.Equals(d.Severity, "internal error", StringComparison.OrdinalIgnoreCase));
        bool hasOutputRom = !string.IsNullOrWhiteSpace(outputFilename) && File.Exists(outputFilename);

        return new kitaqgb.CompileResult
        {
            Success = exitCode == 0 && hasOutputRom && !hasErrorDiagnostics,
            ExitCode = exitCode,
            OutputFilename = outputFilename,
            Diagnostics = diagnostics,
            DiagJsonPath = diagJsonPath,
            DebugMetadataJsonPath = dbg2JsonPath,
            BuildReportJsonPath = buildReportJsonPath,
            OutputArtifacts = new Dictionary<string, string>(artifacts, StringComparer.OrdinalIgnoreCase),
        };
    }

    // Copy remembered paths and supplement them with existing conventional output files and directories.
    // This can include artifacts from earlier runs; absent files do not remove already remembered entries.
    static Dictionary<string, string> SnapshotArtifactPaths(CompilerSession session, string requestedOutputFilename)
    {
        var artifacts = new Dictionary<string, string>(session.ArtifactPaths, StringComparer.OrdinalIgnoreCase);
        string outputFilename = null;
        if (!artifacts.TryGetValue("output_rom", out outputFilename))
        {
            outputFilename = string.IsNullOrWhiteSpace(session.OutputFilenameForDiag) ? requestedOutputFilename : session.OutputFilenameForDiag;
        }

        AddArtifactIfExists(artifacts, "output_rom", outputFilename);
        if (session.EmitDiagJson && !string.IsNullOrWhiteSpace(session.DiagJsonPath) && session.DiagJsonPath != "-")
            AddArtifactIfExists(artifacts, "diag_json", session.DiagJsonPath);

        if (!string.IsNullOrWhiteSpace(outputFilename))
        {
            AddArtifactIfExists(artifacts, "dbg2_json", Path.ChangeExtension(outputFilename, ".dbg2.json"));
            AddArtifactIfExists(artifacts, "build_report_json", Path.ChangeExtension(outputFilename, ".build_report.json"));
            AddArtifactIfExists(artifacts, "map", Path.ChangeExtension(outputFilename, ".map"));
            AddArtifactIfExists(artifacts, "dbg", Path.ChangeExtension(outputFilename, ".dbg"));
            AddArtifactIfExists(artifacts, "dbc", Path.ChangeExtension(outputFilename, ".dbc"));
            AddArtifactIfExists(artifacts, "banks_txt", Path.ChangeExtension(outputFilename, ".banks.txt"));
            AddArtifactIfExists(artifacts, "funcsizes_txt", Path.ChangeExtension(outputFilename, ".funcsizes.txt"));
            AddArtifactIfExists(artifacts, "source_map", Path.ChangeExtension(outputFilename, ".source_map.txt"));
            AddArtifactIfExists(artifacts, "deps_list", ResolveDepsOutputPath(outputFilename));
            if (session.EmitVarList)
            {
                string vlistPath = string.IsNullOrWhiteSpace(session.VarListOutputPath)
                    ? Path.ChangeExtension(outputFilename, ".vlist.txt")
                    : session.VarListOutputPath;
                AddArtifactIfExists(artifacts, "vlist", vlistPath);
            }
        }

        AddDirectoryIfExists(artifacts, "debug_dir", session.DebugOutputPath);
        AddDirectoryIfExists(artifacts, "trace_dir", session.TraceOutputPath);
        if (!string.IsNullOrWhiteSpace(session.PathManifestPath))
            AddArtifactIfExists(artifacts, "path_manifest", session.PathManifestPath);
        return artifacts;
    }

    // Normalize a nonblank file path and add or replace its role only when the file exists.
    static void AddArtifactIfExists(IDictionary<string, string> artifacts, string key, string path)
    {
        if (string.IsNullOrWhiteSpace(key) || string.IsNullOrWhiteSpace(path)) return;
        string normalized = NormalizeArtifactPath(path);
        if (File.Exists(normalized)) artifacts[key] = normalized;
    }

    // Normalize a nonblank directory path and add or replace its role only when the directory exists.
    static void AddDirectoryIfExists(IDictionary<string, string> artifacts, string key, string path)
    {
        if (string.IsNullOrWhiteSpace(key) || string.IsNullOrWhiteSpace(path)) return;
        string normalized = NormalizeArtifactPath(path);
        if (Directory.Exists(normalized)) artifacts[key] = normalized;
    }

    // Resolve relative paths against the process working directory; retain the original text if normalization fails.
    static string NormalizeArtifactPath(string path)
    {
        if (string.IsNullOrWhiteSpace(path)) return path;
        try
        {
            return Path.GetFullPath(path);
        }
        catch
        {
            return path;
        }
    }

    // Prefer the explicit manifest filename; otherwise derive it from the current diagnostic output name.
    static string ResolvePathManifestOutputPath()
    {
        if (!string.IsNullOrWhiteSpace(PathManifestPath))
            return PathManifestPath;

        string outputFilename = string.IsNullOrWhiteSpace(_outputFilenameForDiag) ? "out.gb" : _outputFilenameForDiag;
        return Path.ChangeExtension(outputFilename, ".artifacts.json");
    }

    // When enabled, write the artifact snapshot and remember the actual fallback path.
    // A write failure becomes a stderr warning even when ordinary helper output is suppressed.
    static void TryWritePathManifest(int exitCode)
    {
        if (!EmitPathManifest) return;

        try
        {
            string path = ResolvePathManifestOutputPath();
            string json = BuildPathManifestJson(SnapshotArtifactPaths(CurrentSession, _outputFilenameForDiag), exitCode);
            path = IoUtil.WriteAllTextUtf8Robust(path, json, allowAlternatePath: true);
            PathManifestPath = path;
            RememberArtifactPath("path_manifest", path);
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("warning KQ0000: failed to write path manifest: " + ex.Message);
        }
    }

    // Emit the exit code and escaped artifact paths in stable key order, excluding the manifest's own path.
    static string BuildPathManifestJson(IReadOnlyDictionary<string, string> artifacts, int exitCode)
    {
        var sb = new StringBuilder();
        sb.Append("{");
        sb.Append("\"exit_code\":").Append(exitCode);
        foreach (var pair in artifacts
            .Where(pair => !string.Equals(pair.Key, "path_manifest", StringComparison.OrdinalIgnoreCase))
            .OrderBy(pair => pair.Key, StringComparer.OrdinalIgnoreCase))
        {
            sb.Append(",\"").Append(JsonEscape(pair.Key)).Append("\":\"").Append(JsonEscape(pair.Value)).Append("\"");
        }
        sb.Append("}");
        return sb.ToString();
    }
}
