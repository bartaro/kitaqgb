param(
  [Parameter(Mandatory=$true)][string]$RomPath,
  [Parameter(Mandatory=$true)][string]$MapPath,
  [string]$Symbol = '__kq_vblank_vector',
  [switch]$NoHeaderFix
)

$ErrorActionPreference = 'Stop'

$romFull = Resolve-Path $RomPath
$mapFull = Resolve-Path $MapPath

$escaped = [regex]::Escape($Symbol)
$vectorAddr = $null
foreach ($line in Get-Content -LiteralPath $mapFull) {
  if ($line -match "^\s*([0-9A-Fa-f]{4})\s+.*\s$escaped\s*$") {
    $vectorAddr = [Convert]::ToInt32($matches[1], 16)
    break
  }
}

if ($null -eq $vectorAddr) {
  throw "patch_gb_vblank_irq: $Symbol not found in map"
}
if ($vectorAddr -ge 0x4000) {
  throw ('patch_gb_vblank_irq: {0} must be in fixed bank, got 0x{1:X4}' -f $Symbol, $vectorAddr)
}

$bytes = [IO.File]::ReadAllBytes($romFull)
$signature = [byte[]](0xF5, 0xC5, 0xD5, 0xE5)
$entryAddr = $null

for ($addr = $vectorAddr; $addr -lt [Math]::Min($vectorAddr + 96, $bytes.Length - $signature.Length); $addr++) {
  $match = $true
  for ($i = 0; $i -lt $signature.Length; $i++) {
    if ($bytes[$addr + $i] -ne $signature[$i]) {
      $match = $false
      break
    }
  }
  if ($match) {
    $entryAddr = $addr
    break
  }
}

if ($null -eq $entryAddr) {
  throw ('patch_gb_vblank_irq: raw ISR body signature not found near 0x{0:X4}' -f $vectorAddr)
}

$bytes[0x40] = 0xC3
$bytes[0x41] = [byte]($entryAddr -band 0xFF)
$bytes[0x42] = [byte](($entryAddr -shr 8) -band 0xFF)

if (!$NoHeaderFix) {
  for ($i = 0x0134; $i -le 0x0143; $i++) {
    if ($bytes[$i] -eq 0xFF) { $bytes[$i] = 0x00 }
  }

  $headerChk = 0
  for ($i = 0x0134; $i -le 0x014C; $i++) {
    $headerChk = ($headerChk - $bytes[$i] - 1) -band 0xFF
  }
  $bytes[0x014D] = [byte]$headerChk

  $globalChk = 0
  for ($i = 0; $i -lt $bytes.Length; $i++) {
    if ($i -ne 0x014E -and $i -ne 0x014F) {
      $globalChk = ($globalChk + $bytes[$i]) -band 0xFFFF
    }
  }
  $bytes[0x014E] = [byte](($globalChk -shr 8) -band 0xFF)
  $bytes[0x014F] = [byte]($globalChk -band 0xFF)
}

[IO.File]::WriteAllBytes($romFull, $bytes)

if ($NoHeaderFix) {
  Write-Host ('patch_gb_vblank_irq: VBlank vector 0x0040 -> 0x{0:X4} ({1}, header unchanged)' -f $entryAddr, $Symbol)
} else {
  Write-Host ('patch_gb_vblank_irq: VBlank vector 0x0040 -> 0x{0:X4} ({1}), header checksums updated' -f $entryAddr, $Symbol)
}

