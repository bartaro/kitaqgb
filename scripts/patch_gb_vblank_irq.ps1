param([Parameter(Mandatory=$true)][string]$RomPath,[Parameter(Mandatory=$true)][string]$MapPath,[string]$Symbol='__kq_vblank_vector',[switch]$NoHeaderFix)
$ErrorActionPreference='Stop'
$patcher=Join-Path (Join-Path $PSScriptRoot '..') 'kitaqgb-patch-vblank.exe'
$arguments=@('--rom',$RomPath,'--map',$MapPath,'--symbol',$Symbol)
if ($NoHeaderFix) { $arguments+='--no-header-fix' }
& $patcher @arguments
if ($LASTEXITCODE -ne 0) { throw 'VBlank patch failed' }
