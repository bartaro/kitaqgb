$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$msbuildCommand = Get-Command MSBuild.exe -ErrorAction SilentlyContinue
if ($msbuildCommand) { $msbuildPath = $msbuildCommand.Source }
else {
    $vswherePath = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (!(Test-Path -LiteralPath $vswherePath)) { throw 'Install Visual Studio Build Tools and the .NET Framework 4.8 Developer Pack.' }
    $msbuildPath = & $vswherePath -latest -requires Microsoft.Component.MSBuild -find 'MSBuild\**\Bin\MSBuild.exe' | Select-Object -First 1
    if (!$msbuildPath) { throw 'MSBuild was not found. Install Visual Studio Build Tools.' }
}
Push-Location $repositoryRoot
try {
    & $msbuildPath './kitaqgb/kitaqgb.csproj' /t:Rebuild /p:Configuration=Release /nologo
    if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
    Write-Host ('Release executable: ' + (Join-Path $repositoryRoot 'kitaqgb.exe'))
} finally { Pop-Location }
