$ErrorActionPreference = 'Stop'
Push-Location (Join-Path $PSScriptRoot '..')
try {
    & MSBuild.exe './kitaqgb.csproj' /t:Build /p:Configuration=Release
    if ($LASTEXITCODE -ne 0) { throw 'Build failed' }
} finally { Pop-Location }
