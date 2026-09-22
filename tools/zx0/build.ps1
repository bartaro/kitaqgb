$ErrorActionPreference = 'Stop'
$toolDirectory = $PSScriptRoot
$compilerPath = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
& $compilerPath /nologo /optimize+ /target:exe "/out:$toolDirectory\..\..\kitaqgb-zx0.exe" "$toolDirectory\KitaqZx0.cs"
if ($LASTEXITCODE -ne 0) { throw 'ZX0 asset tool build failed.' }
