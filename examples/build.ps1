param(
    [string]$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path,
    [string]$GbCompiler = '',
    [string]$FcCompiler = '',
    [string[]]$Only = @(),
    [string]$Output = (Join-Path $PSScriptRoot 'out')
)
$ErrorActionPreference = 'Stop'
if (!$GbCompiler) { $GbCompiler = Join-Path $Root 'kitaqgb.exe' }
if (!$FcCompiler) { $FcCompiler = Join-Path $Root 'kitaqfc.exe' }
$GbCompiler = [IO.Path]::GetFullPath($GbCompiler)
$FcCompiler = [IO.Path]::GetFullPath($FcCompiler)
$Output = [IO.Path]::GetFullPath($Output)
New-Item -ItemType Directory -Force $Output | Out-Null
$programs = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'manifest.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$failed = @()
# Accept either an array of sample IDs or comma-separated IDs, then reject unknown selections.
$selected = @($Only | ForEach-Object { $_ -split ',' })
$built = 0
foreach ($id in $selected) {
    if ($id -notin $programs.id) { throw ('Unknown sample: '+$id) }
}
# Use manifest order and compile library units before the sample source.
foreach ($program in $programs) {
    if ($selected.Count -and $program.id -notin $selected) { continue }
    # Default builds skip entries with a recorded issue; selecting one explicitly attempts its build.
    if (!$Only -and $program.known_issue) { Write-Host ('SKIP '+$program.id+': '+$program.known_issue); continue }
    $targetDir = Join-Path $Output $program.id
    $built++
    New-Item -ItemType Directory -Force $targetDir | Out-Null
    $isGb = $program.platform -eq 'gb'
    $compiler = if ($isGb) { $GbCompiler } else { $FcCompiler }
    $lib = Join-Path $Root $(if ($isGb) { 'lib' } else { 'lib' })
    $compileArgs = @()
    foreach ($unit in $program.libs) { $compileArgs += Join-Path $lib $unit }
    $compileArgs += Join-Path $PSScriptRoot $program.file
    $compileArgs += @('-I',$lib,'-I',$PSScriptRoot,'--no-disasm')
    $compileArgs += @('-o',(Join-Path $targetDir $(if ($isGb) { 'program.gb' } else { 'program.nes' })))
    if ($isGb) { $compileArgs += @('--profile=dev','--rst-disable','--stack-bank=fixed') }
    else { $compileArgs += @('--mapper=nrom',('--nes-chr='+(Join-Path $PSScriptRoot 'font.chr'))) }
    $compileArgs += $program.options
    # Run in a separate directory per sample so compiler sidecars do not overwrite each other.
    Push-Location $targetDir
    try {
        & $compiler @compileArgs *> build.log
        if ($LASTEXITCODE -ne 0) { $failed += $program.id; Write-Host ('FAIL '+$program.id) }
        else { Write-Host ('OK '+$program.id) }
    } finally { Pop-Location }
}
# Try every selected sample, then fail the script if any compiler returned an error.
if ($failed.Count) { throw ('Build failures: '+($failed -join ', ')) }
if ($built -eq 0) { throw 'No samples were selected' }
Write-Host ('ROMs and logs: '+$Output)
