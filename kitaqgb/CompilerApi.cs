using System;
using System.Collections.Generic;
using System.Linq;

namespace kitaqgb
{
    public sealed class CompileRequest
    {
        public IReadOnlyList<string> SourceFiles { get; set; } = new string[0];
        public string OutputFilename { get; set; } = "out.gb";
        public IReadOnlyList<string> IncludeDirectories { get; set; } = new string[0];
        public string Profile { get; set; }
        public bool StrictDiagnostics { get; set; }
        public bool EnableDebugOutput { get; set; }
        public string DebugOutputPath { get; set; }
        public bool EnableTrace { get; set; }
        public string TraceOutputPath { get; set; }
        public IReadOnlyList<string> TraceStages { get; set; } = new string[0];
        public bool EmitDiagJson { get; set; }
        public string DiagJsonPath { get; set; }
        public bool EmitPathManifest { get; set; }
        public string PathManifestPath { get; set; }
        public bool MachineReadable { get; set; } = true;
        public bool NoBanner { get; set; } = true;
        public string StackBank { get; set; }
        public int? StackTop { get; set; }
        public int? StackReserve { get; set; }
        public IReadOnlyList<string> ExtraArgs { get; set; } = new string[0];
    }

    public sealed class DiagnosticDto
    {
        public string Severity { get; set; }
        public string Code { get; set; }
        public string Message { get; set; }
        public string Suggestion { get; set; }
        public string File { get; set; }
        public int Line { get; set; }
        public int Column { get; set; }
    }

    public sealed class CompileResult
    {
        public bool Success { get; set; }
        public int ExitCode { get; set; }
        public string OutputFilename { get; set; }
        public IReadOnlyList<DiagnosticDto> Diagnostics { get; set; } = new DiagnosticDto[0];
        public string DiagJsonPath { get; set; }
        public string DebugMetadataJsonPath { get; set; }
        public string BuildReportJsonPath { get; set; }
        public IReadOnlyDictionary<string, string> OutputArtifacts { get; set; } = new Dictionary<string, string>();
    }

    public interface IKitaqgbCompiler
    {
        CompileResult Compile(CompileRequest request);
    }

    public sealed class KitaqgbCompiler : IKitaqgbCompiler
    {
        public CompileResult Compile(CompileRequest request)
        {
            if (request == null) throw new ArgumentNullException(nameof(request));
            var args = BuildCommandLineArgs(request);
            return global::Program.InvokeCompileForApi(request, args.ToArray());
        }

        static List<string> BuildCommandLineArgs(CompileRequest request)
        {
            if (request.SourceFiles == null || request.SourceFiles.Count == 0)
                throw new ArgumentException("CompileRequest.SourceFiles must contain at least one source file.", nameof(request));

            var args = new List<string>();
            foreach (var source in request.SourceFiles.Where(x => !string.IsNullOrWhiteSpace(x)))
                args.Add(source);

            if (!string.IsNullOrWhiteSpace(request.OutputFilename))
            {
                args.Add("-o");
                args.Add(request.OutputFilename);
            }

            foreach (var includeDir in request.IncludeDirectories ?? Array.Empty<string>())
            {
                if (string.IsNullOrWhiteSpace(includeDir)) continue;
                args.Add("-I");
                args.Add(includeDir);
            }

            if (!string.IsNullOrWhiteSpace(request.Profile))
                args.Add("--profile=" + request.Profile);
            if (request.StrictDiagnostics)
                args.Add("--strict");
            if (request.EnableDebugOutput)
            {
                if (string.IsNullOrWhiteSpace(request.DebugOutputPath))
                    args.Add("--debug-out");
                else
                    args.Add("--debug-out=" + request.DebugOutputPath);
            }

            if (request.EnableTrace || (request.TraceStages != null && request.TraceStages.Count > 0) || !string.IsNullOrWhiteSpace(request.TraceOutputPath))
            {
                var stages = (request.TraceStages ?? Array.Empty<string>())
                    .Where(x => !string.IsNullOrWhiteSpace(x))
                    .ToArray();
                if (stages.Length == 0)
                    args.Add("--trace");
                else
                    args.Add("--trace=" + string.Join(",", stages));

                if (!string.IsNullOrWhiteSpace(request.TraceOutputPath))
                    args.Add("--trace-out=" + request.TraceOutputPath);
            }

            if (request.EmitDiagJson)
            {
                if (string.IsNullOrWhiteSpace(request.DiagJsonPath))
                    args.Add("--diag-json");
                else
                    args.Add("--diag-json=" + request.DiagJsonPath);
            }

            if (request.EmitPathManifest)
            {
                if (string.IsNullOrWhiteSpace(request.PathManifestPath))
                    args.Add("--emit-path-manifest=" + System.IO.Path.ChangeExtension(request.OutputFilename ?? "out.gb", ".artifacts.json"));
                else
                    args.Add("--emit-path-manifest=" + request.PathManifestPath);
            }

            if (request.MachineReadable)
                args.Add("--machine-readable");
            if (request.NoBanner)
                args.Add("--no-banner");
            if (!string.IsNullOrWhiteSpace(request.StackBank))
                args.Add("--stack-bank=" + request.StackBank);
            if (request.StackTop.HasValue)
                args.Add("--stack-top=0x" + request.StackTop.Value.ToString("X4"));
            if (request.StackReserve.HasValue)
                args.Add("--stack-reserve=" + request.StackReserve.Value);

            foreach (var extraArg in request.ExtraArgs ?? Array.Empty<string>())
            {
                if (!string.IsNullOrWhiteSpace(extraArg))
                    args.Add(extraArg);
            }

            return args;
        }
    }
}
