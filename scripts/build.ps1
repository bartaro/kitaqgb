$ErrorActionPreference = 'Stop'
# Resolve the repository from this script so invoking it from another directory still builds the intended project.
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
# Prefer MSBuild on PATH; otherwise ask the installed Visual Studio locator for a matching Build Tools instance.
$msbuildCommand = Get-Command MSBuild.exe -ErrorAction SilentlyContinue
if ($msbuildCommand) { $msbuildPath = $msbuildCommand.Source }
else {
    $vswherePath = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (!(Test-Path -LiteralPath $vswherePath)) { throw 'Install Visual Studio Build Tools and the .NET Framework 4.8 Developer Pack.' }
    $msbuildPath = & $vswherePath -latest -requires Microsoft.Component.MSBuild -find 'MSBuild\**\Bin\MSBuild.exe' | Select-Object -First 1
    if (!$msbuildPath) { throw 'MSBuild was not found. Install Visual Studio Build Tools.' }
}
# Rebuild Release and propagate native-command failure. The project AfterBuild target copies the executable to the root.
# The finally block restores the original working directory even when the build throws.
Push-Location $repositoryRoot
try {
    & $msbuildPath './kitaqgb/kitaqgb.csproj' /t:Rebuild /p:Configuration=Release /nologo
    if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
    Write-Host ('Release executable: ' + (Join-Path $repositoryRoot 'kitaqgb.exe'))
} finally { Pop-Location }
